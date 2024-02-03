extern crate ash;

use ash::vk;
use glam::{f32::Mat4, Vec2, Vec3};
use image::EncodableLayout;
use memoffset::offset_of;
use std::{borrow::BorrowMut, mem::size_of, rc::Rc, sync::Arc, time::Instant};
use winit::{dpi::PhysicalSize, window::Window};

use crate::gpu::{
    Buffer, CommandBuffer, CommandPool, DescriptorPool, DescriptorSet, DescriptorSetLayout, Device,
    Fence, GraphicsPipeline, HasRawAshHandle, HasRawVkHandle, Image, ImageView, Instance,
    PhysicalDevice, PipelineLayout, Sampler, Semaphore, ShaderKind, ShaderModule, Swapchain,
};

pub struct RenderContext {
    start_time: Instant,
    frame_count: u64,
    window: Arc<Window>,
    instance: Arc<Instance>,
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    allocator: Arc<vma::Allocator>,
    swapchain: Swapchain,
    shader_modules: Vec<Arc<ShaderModule>>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    draw_images: Vec<Arc<Image>>,
    pipeline_layout: Arc<PipelineLayout>,
    descriptor_pool: DescriptorPool,
    descriptor_sets: Box<[DescriptorSet]>,
    uniform_buffers: Vec<Buffer>,
    texture_image: Arc<Image>,
    texture_image_view: Arc<ImageView>,
    sampler: Arc<Sampler>,
    indices: Vec<u16>,
    index_buffer: Buffer,
    vertex_buffers: Vec<Buffer>,
    cmd_pool: Rc<CommandPool>,
    render_frames: Vec<RenderFrame>,
    current_frame: usize,
}

struct SurfaceDetails {
    present_mode: vk::PresentModeKHR,
    format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
}

#[repr(C)]
struct Uniform {
    model: Mat4,
    view: Mat4,
    proj: Mat4,
}

#[repr(C)]
struct Vertex {
    position: Vec3,
    color: Vec3,
    tex_coord: Vec2,
}

impl RenderContext {
    pub fn new(window: Arc<Window>, max_frames_in_flight: usize) -> Self {
        let instance = Instance::new(&window);

        let required_queue_flags = &[vk::QueueFlags::GRAPHICS];

        let required_extensions: &[&[u8]] = &[
            // b"VK_EXT_debug_utils\0",
            b"VK_KHR_swapchain\0",
            b"VK_KHR_dynamic_rendering\0",
            b"VK_KHR_synchronization2\0",
        ];

        // Find the first physical device that supports the swapchain extension
        // and is preferably a discrete GPU
        let physical_device = instance
            .get_physical_devices()
            .into_iter()
            .min_by_key(|x| match x.device_type() {
                // Rank each physical device by the type, with preference for discrete
                vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                _ => 4,
            })
            .filter(|x| {
                // Filter for physical devices that support all of the required
                // queue flags and the window surface
                let mut supports_surface = false;
                let mut flags = Vec::from(required_queue_flags);
                for (i, properties) in x.get_queue_family_properties().iter().enumerate() {
                    if x.supports_surface(i.try_into().unwrap()) {
                        supports_surface = true;
                    }
                    flags.retain(|x| !properties.queue_flags.contains(*x));
                }
                supports_surface && flags.len() == 0
            })
            .filter(|x| {
                let features = x.device_features();
                features.sampler_anisotropy == vk::TRUE
            })
            .filter(|x| {
                // Filter for physical devices that support all of the required
                // extensions
                let extensions_hashset = x.extension_name_hashset();
                required_extensions
                    .iter()
                    .all(|ext| extensions_hashset.contains(ext))
            })
            .unwrap();

        println!("physical_device.name = {}", physical_device.device_name());

        // Select queue family indices for logical device creation
        let mut queue_family_indices = vec![];
        for (i, x) in physical_device
            .get_queue_family_properties()
            .iter()
            .enumerate()
        {
            println!(
                "queue_family[{}] = (queue_count = {}, {:?})",
                i, x.queue_count, x.queue_flags
            );

            let mut enable = false;

            if x.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                enable = true;
            }

            if x.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                enable = true;
            }

            if enable {
                let queue_family_index: u32 = i.try_into().unwrap();
                queue_family_indices.push(queue_family_index);
            }
        }

        let device = physical_device.get_device(&queue_family_indices, required_extensions);

        let allocator = unsafe {
            let info = vma::AllocatorCreateInfo::new(
                instance.get_ash_handle(),
                device.get_ash_handle(),
                physical_device.get_vk_handle(),
            );
            Arc::new(vma::Allocator::new(info).expect("failed to create vma allocator"))
        };

        let swapchain = {
            let inner_size = window.inner_size();
            RenderContext::_create_swapchain(
                device.clone(),
                inner_size.width,
                inner_size.height,
                None,
            )
        };

        let shader_compiler = shaderc::Compiler::new().unwrap();

        let shader_modules = vec![
            ShaderModule::new(
                device.clone(),
                &shader_compiler,
                include_str!("./shaders/vertex.glsl"),
                ShaderKind::Vertex,
                "vertex.glsl",
                "main",
                None,
            ),
            ShaderModule::new(
                device.clone(),
                &shader_compiler,
                include_str!("./shaders/fragment.glsl"),
                ShaderKind::Fragment,
                "fragment.glsl",
                "main",
                None,
            ),
        ];

        let draw_image_format = vk::Format::R16G16B16A16_SFLOAT;

        let descriptor_set_layout = {
            let mut builder = DescriptorSetLayout::builder();

            let uniform_binding = builder
                .binding()
                .descriptor(1, vk::DescriptorType::UNIFORM_BUFFER)
                .stage(vk::ShaderStageFlags::VERTEX);

            let sampler_binding = builder
                .binding()
                .descriptor(1, vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage(vk::ShaderStageFlags::FRAGMENT);

            builder.build(
                device.clone(),
                vk::DescriptorSetLayoutCreateFlags::empty(),
                &[uniform_binding, sampler_binding],
            )
        };

        let pipeline_layout =
            PipelineLayout::new(device.clone(), &[descriptor_set_layout.clone()], &[]);

        let uniform_buffers = {
            let buffer_size = size_of::<Uniform>();
            let mut uniform_buffers = vec![];

            for _ in 0..max_frames_in_flight {
                let uniform_buffer = Buffer::new(
                    device.clone(),
                    allocator.clone(),
                    buffer_size,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    vma::MemoryUsage::AutoPreferHost,
                    vma::AllocationCreateFlags::MAPPED
                        | vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
                );

                uniform_buffers.push(uniform_buffer);
            }

            uniform_buffers
        };

        let graphics_queue = device.get_first_queue(vk::QueueFlags::GRAPHICS).unwrap();

        let cmd_pool = CommandPool::new(
            device.clone(),
            graphics_queue.queue_family(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        );

        let texture_image: Arc<Image>;
        let texture_image_view: Arc<ImageView>;
        let sampler: Arc<Sampler>;

        {
            let image_path = "./checker-map.png";
            let image_buffer = image::open(image_path).unwrap().to_rgba8();
            let image_bytes = image_buffer.as_bytes();

            let staging_buffer = Buffer::new(
                device.clone(),
                allocator.clone(),
                image_bytes.len(),
                vk::BufferUsageFlags::TRANSFER_SRC,
                vma::MemoryUsage::AutoPreferHost,
                vma::AllocationCreateFlags::MAPPED
                    | vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            );

            staging_buffer.copy_nonoverlapping(image_bytes);

            texture_image = Image::new(
                device.clone(),
                allocator.clone(),
                vk::ImageType::TYPE_2D,
                vk::Format::R8G8B8A8_SRGB,
                vk::Extent3D {
                    width: image_buffer.width(),
                    height: image_buffer.height(),
                    depth: 1,
                },
                1,
                1,
                vk::SampleCountFlags::TYPE_1,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                vma::MemoryUsage::AutoPreferDevice,
                vma::AllocationCreateFlags::empty(),
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            );

            let cmds = cmd_pool.allocate_one(vk::CommandBufferLevel::PRIMARY);

            cmds.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            cmds.transition_image(
                &texture_image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );
            cmds.copy_buffer_to_image(&staging_buffer, &texture_image);
            cmds.transition_image(
                &texture_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            );
            cmds.end();

            graphics_queue.submit(None, &[&cmds], None, None);
            graphics_queue.wait_idle();

            texture_image_view = texture_image.get_default_view(vk::ImageAspectFlags::COLOR);
            sampler = Sampler::new(device.clone());
        };

        let descriptor_pool = DescriptorPool::new(
            device.clone(),
            vk::DescriptorPoolCreateFlags::empty(),
            max_frames_in_flight as u32,
            &[
                (
                    vk::DescriptorType::UNIFORM_BUFFER,
                    max_frames_in_flight.try_into().unwrap(),
                ),
                (
                    vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    max_frames_in_flight.try_into().unwrap(),
                ),
            ],
        );

        let descriptor_sets = {
            let mut layouts = vec![];
            for _ in 0..max_frames_in_flight {
                layouts.push(&*descriptor_set_layout);
            }
            descriptor_pool.allocate(&layouts)
        };

        for (i, uniform_buffer) in uniform_buffers.iter().enumerate() {
            descriptor_sets[i].write_buffer(
                uniform_buffer,
                0,
                size_of::<Uniform>().try_into().unwrap(),
                0,
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
            );

            descriptor_sets[i].write_image(
                &sampler,
                &texture_image_view,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                1,
                0,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            )
        }

        #[rustfmt::skip]
        let indices: Vec<u16> = vec![
            0, 2, 1, 2, 0, 3, // z = -0.5
            4, 5, 6, 6, 7, 4, // z =  0.5
            0, 1, 4, 5, 4, 1, // y = -0.5
            2, 3, 6, 7, 6, 3, // y =  0.5
            3, 0, 4, 4, 7, 3, // x = -0.5
            1, 2, 5, 5, 2, 6, // x =  0.5
        ];

        #[rustfmt::skip]
        let vertices = [
            /* 0 */ Vertex { position: Vec3::new(-0.5, -0.5, -0.5), color: Vec3::new(1.0, 0.0, 0.0), tex_coord: Vec2::new(0.0, 1.0) },
            /* 1 */ Vertex { position: Vec3::new( 0.5, -0.5, -0.5), color: Vec3::new(0.0, 1.0, 0.0), tex_coord: Vec2::new(0.0, 0.0) },
            /* 2 */ Vertex { position: Vec3::new( 0.5,  0.5, -0.5), color: Vec3::new(0.0, 0.0, 1.0), tex_coord: Vec2::new(1.0, 0.0) },
            /* 3 */ Vertex { position: Vec3::new(-0.5,  0.5, -0.5), color: Vec3::new(1.0, 1.0, 1.0), tex_coord: Vec2::new(1.0, 1.0) },
            /* 4 */ Vertex { position: Vec3::new(-0.5, -0.5,  0.5), color: Vec3::new(1.0, 0.0, 0.0), tex_coord: Vec2::new(1.0, 0.0) },
            /* 5 */ Vertex { position: Vec3::new( 0.5, -0.5,  0.5), color: Vec3::new(0.0, 1.0, 0.0), tex_coord: Vec2::new(0.0, 0.0) },
            /* 6 */ Vertex { position: Vec3::new( 0.5,  0.5,  0.5), color: Vec3::new(0.0, 0.0, 1.0), tex_coord: Vec2::new(0.0, 1.0) },
            /* 7 */ Vertex { position: Vec3::new(-0.5,  0.5,  0.5), color: Vec3::new(1.0, 1.0, 1.0), tex_coord: Vec2::new(1.0, 1.0) },
        ];

        let vertex_bindings = vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>().try_into().unwrap(),
            input_rate: vk::VertexInputRate::VERTEX,
        };

        let vertex_attributes = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, position).try_into().unwrap(),
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color).try_into().unwrap(),
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, tex_coord).try_into().unwrap(),
            },
        ];

        let index_buffer = {
            let buffer_size = size_of::<u16>() * indices.len();

            let staging_buffer = Buffer::new(
                device.clone(),
                allocator.clone(),
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vma::MemoryUsage::AutoPreferHost,
                vma::AllocationCreateFlags::MAPPED
                    | vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            );

            staging_buffer.copy_nonoverlapping(&indices);

            let index_buffer = Buffer::new(
                device.clone(),
                allocator.clone(),
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
                vma::MemoryUsage::AutoPreferDevice,
                vma::AllocationCreateFlags::empty(),
            );

            let xfer_cmd_buf = cmd_pool.allocate_one(vk::CommandBufferLevel::PRIMARY);
            xfer_cmd_buf.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            xfer_cmd_buf.copy_buffer(
                &staging_buffer,
                &index_buffer,
                &[vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: buffer_size.try_into().unwrap(),
                }],
            );
            xfer_cmd_buf.end();

            graphics_queue.submit(None, &[&xfer_cmd_buf], None, None);
            graphics_queue.wait_idle();

            index_buffer
        };

        let vertex_buffers = {
            let buffer_size = size_of::<Vertex>() * vertices.len();

            let staging_buffer = Buffer::new(
                device.clone(),
                allocator.clone(),
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                vma::MemoryUsage::AutoPreferHost,
                vma::AllocationCreateFlags::MAPPED
                    | vma::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE,
            );

            staging_buffer.copy_nonoverlapping(&vertices);

            let vertex_buffer = Buffer::new(
                device.clone(),
                allocator.clone(),
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
                vma::MemoryUsage::AutoPreferDevice,
                vma::AllocationCreateFlags::empty(),
            );

            let xfer_cmd_buf = cmd_pool.allocate_one(vk::CommandBufferLevel::PRIMARY);
            xfer_cmd_buf.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            xfer_cmd_buf.copy_buffer(
                &staging_buffer,
                &vertex_buffer,
                &[vk::BufferCopy {
                    src_offset: 0,
                    dst_offset: 0,
                    size: buffer_size.try_into().unwrap(),
                }],
            );
            xfer_cmd_buf.end();

            graphics_queue.submit(None, &[&xfer_cmd_buf], None, None);
            graphics_queue.wait_idle();

            vec![vertex_buffer]
        };

        let graphics_pipeline = GraphicsPipeline::new(
            device.clone(),
            &shader_modules,
            Some(&[vertex_bindings]),
            Some(&vertex_attributes),
            &vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
            vk::PrimitiveTopology::TRIANGLE_LIST,
            false,
            None,
            None,
            &pipeline_layout,
            &[draw_image_format],
            vk::Format::UNDEFINED,
            vk::Format::UNDEFINED,
        );

        let draw_images = RenderContext::_create_draw_images(
            &device,
            &allocator,
            max_frames_in_flight,
            vk::Extent3D {
                width: swapchain.extent().width,
                height: swapchain.extent().height,
                depth: 1,
            },
        );

        let mut render_context = Self {
            start_time: std::time::Instant::now(),
            frame_count: 0,
            window,
            instance,
            physical_device,
            device,
            allocator,
            swapchain,
            shader_modules,
            graphics_pipeline,
            draw_images,
            pipeline_layout,
            descriptor_pool,
            descriptor_sets,
            uniform_buffers,
            texture_image,
            texture_image_view,
            sampler,
            indices,
            index_buffer,
            vertex_buffers,
            cmd_pool,
            render_frames: vec![],
            current_frame: 0,
        };

        render_context.render_frames.reserve(max_frames_in_flight);
        for i in 0..max_frames_in_flight {
            render_context
                .render_frames
                .push(RenderFrame::new(i, &render_context));
        }

        render_context
    }

    fn _get_surface_details(
        physical_device: &Arc<PhysicalDevice>,
        width: u32,
        height: u32,
    ) -> SurfaceDetails {
        let present_mode = physical_device
            .get_surface_present_modes()
            .into_iter()
            .min_by_key(|x| match *x {
                // vk::PresentModeKHR::MAILBOX => 0, // uncapped
                vk::PresentModeKHR::FIFO_RELAXED => 0, // caps framerate
                vk::PresentModeKHR::FIFO => 1,
                _ => 2,
            })
            .unwrap();

        // Chose the swapchain surface format to use, preferring B8G8R8A8_SRGB
        // with a SRGB_NONLINEAR color space, and otherwise taking the first
        // option
        let format = physical_device
            .get_surface_formats()
            .into_iter()
            .enumerate()
            .min_by_key(|(index, x)| {
                if x.format == vk::Format::B8G8R8A8_SRGB {
                    if x.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                        return 0;
                    }
                }
                index + 1
            })
            .map(|(_, x)| x)
            .unwrap();

        let extent = physical_device.get_surface_current_extent_clamped(width, height);

        println!("present_mode = {:?}", present_mode);
        println!("present_format = {:?}", format);
        println!("extent = {:?}", extent);

        SurfaceDetails {
            present_mode,
            format,
            extent,
        }
    }

    fn _create_swapchain(
        device: Arc<Device>,
        width: u32,
        height: u32,
        old_swapchain: Option<&Swapchain>,
    ) -> Swapchain {
        let physical_device = device.physical_device();
        let min_image_count = physical_device.get_surface_ideal_image_count();

        let SurfaceDetails {
            present_mode,
            format,
            extent,
        } = RenderContext::_get_surface_details(physical_device, width, height);

        let swapchain = device.get_swapchain(
            min_image_count,
            format.format,
            format.color_space,
            extent,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
            present_mode,
            old_swapchain,
        );

        swapchain
    }

    fn _create_draw_images(
        device: &Arc<Device>,
        allocator: &Arc<vma::Allocator>,
        max_frames_in_flight: usize,
        extent: vk::Extent3D,
    ) -> Vec<Arc<Image>> {
        let mut draw_images = vec![];
        for _ in 0..max_frames_in_flight {
            draw_images.push(Image::new(
                device.clone(),
                allocator.clone(),
                vk::ImageType::TYPE_2D,
                vk::Format::R16G16B16A16_SFLOAT,
                extent,
                1,
                1,
                vk::SampleCountFlags::TYPE_1,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::TRANSFER_DST // why dst? shouldn't be srconly?
                        | vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                vma::MemoryUsage::AutoPreferDevice,
                vma::AllocationCreateFlags::empty(),
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            ));
        }
        draw_images
    }

    pub fn recreate_swapchain(&mut self, width: u32, height: u32) {
        self.device.wait_idle();

        self.swapchain = RenderContext::_create_swapchain(
            self.device.clone(),
            width,
            height,
            Some(&self.swapchain),
        );

        let max_frames_in_flight = self.render_frames.len();

        self.draw_images = RenderContext::_create_draw_images(
            &self.device,
            &self.allocator,
            max_frames_in_flight,
            vk::Extent3D {
                width,
                height,
                depth: 1,
            },
        );

        self.render_frames.clear();

        for i in 0..max_frames_in_flight {
            self.render_frames.push(RenderFrame::new(i, &self));
        }
    }

    pub fn draw_next_frame(&mut self) {
        let success = self.render_frames[self.current_frame].draw_frame(self);

        if success {
            self.current_frame = (self.current_frame + 1) % self.render_frames.len();
            self.frame_count += 1;
            self.window.request_redraw();
        } else {
            let PhysicalSize { width, height } = self.window.inner_size();
            self.recreate_swapchain(width, height)
        }
    }
}

impl Drop for RenderContext {
    fn drop(&mut self) {
        // Wait for GPU to finish all pending work before dropping the render
        // context. This gives command buffers time to finish before we drop any
        // resources they may be referencing
        self.device.wait_idle();
    }
}

struct RenderFrame {
    index: usize,
    cmd_buf: CommandBuffer,
    image_available: Semaphore,
    render_finished: Semaphore,
    in_flight: Fence,
}

impl RenderFrame {
    pub fn new(index: usize, context: &RenderContext) -> Self {
        let cmd_buf = context
            .cmd_pool
            .allocate_one(vk::CommandBufferLevel::PRIMARY);

        let image_available = Semaphore::new(context.device.clone());
        let render_finished = Semaphore::new(context.device.clone());
        let in_flight = Fence::signaled(context.device.clone());

        Self {
            index,
            cmd_buf,
            image_available,
            render_finished,
            in_flight,
        }
    }

    pub fn update_uniform_buffer(&self, context: &RenderContext) {
        let time = context.start_time.elapsed().as_secs_f32();

        let aspect_ratio = {
            let extent = context.swapchain.extent();
            extent.width as f32 / extent.height as f32
        };

        let model = {
            let x = Mat4::from_rotation_x(time * 90_f32.to_radians());
            let y = Mat4::from_rotation_y(time * 90_f32.to_radians());
            x * y
        };

        let view = Mat4::look_at_rh(
            Vec3::new(2.0, 2.0, 2.0),
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 1.0),
        );

        let proj = {
            let mut m = Mat4::perspective_rh(45_f32.to_radians(), aspect_ratio, 0.1, 10.0);
            let col = m.col_mut(1).borrow_mut();
            col.y *= -1.0;
            m
        };

        let ubo = Uniform { model, view, proj };
        let buffer = &context.uniform_buffers[self.index];

        buffer.copy_nonoverlapping(&[ubo]);
    }

    pub fn draw_frame(&self, context: &RenderContext) -> bool {
        self.update_uniform_buffer(context);

        let fences = &[&self.in_flight];
        context.device.wait_for_fences(fences, true, None);

        let acquire_result =
            context
                .swapchain
                .acquire_next_image(None, Some(&self.image_available), None);

        let image_index: u32;

        match acquire_result {
            Ok((acquired_index, suboptimal)) => {
                image_index = acquired_index;

                if suboptimal {
                    return false;
                }
            }
            Err(result) => match result {
                vk::Result::NOT_READY => todo!(),
                vk::Result::TIMEOUT => todo!(),
                vk::Result::ERROR_OUT_OF_DATE_KHR => return false,
                vk::Result::ERROR_SURFACE_LOST_KHR => todo!(),
                vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => todo!(),
                _ => panic!("acquire_result = {:?}", result),
            },
        }

        context.device.reset_fences(fences);
        self.cmd_buf.reset();

        self.record_commands(context, image_index);

        let graphics_queue = context
            .device
            .get_first_queue(vk::QueueFlags::GRAPHICS)
            .unwrap();

        let present_queue = context.device.get_first_present_queue().unwrap();

        graphics_queue.submit(
            Some(&[(
                &self.image_available,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            )]),
            &[&self.cmd_buf],
            Some(&[(&self.render_finished, vk::PipelineStageFlags2::ALL_GRAPHICS)]),
            Some(&self.in_flight),
        );

        let present_result =
            present_queue.submit_present(&[&self.render_finished], &context.swapchain, image_index);

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    return false;
                }
            }
            Err(result) => match result {
                vk::Result::ERROR_OUT_OF_DATE_KHR => return false,
                vk::Result::ERROR_SURFACE_LOST_KHR => todo!(),
                vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => todo!(),
                _ => panic!("present_result = {:?}", result),
            },
        }

        true
    }

    pub fn record_commands(&self, context: &RenderContext, image_index: u32) {
        self.cmd_buf
            .begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        let extent = context.swapchain.extent();

        let draw_image = &context.draw_images[self.index];
        let draw_image_view = draw_image.get_default_view(vk::ImageAspectFlags::COLOR);
        let swapchain_image = &context.swapchain.images()[image_index as usize];

        self.cmd_buf.transition_image(
            &draw_image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::GENERAL,
        );

        let time =
            0.5 * f32::cos(std::f32::consts::PI + context.start_time.elapsed().as_secs_f32()) + 0.5;

        let clear_value = vk::ClearColorValue {
            float32: [time, 0.0, 0.0, 0.0],
        };

        let clear_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: vk::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: vk::REMAINING_ARRAY_LAYERS,
        };

        self.cmd_buf
            .clear_color_image(&draw_image, clear_value, &[clear_range]);

        self.cmd_buf.transition_image(
            &draw_image,
            vk::ImageLayout::GENERAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let color_attachment = unsafe {
            vk::RenderingAttachmentInfo {
                s_type: vk::StructureType::RENDERING_ATTACHMENT_INFO,
                p_next: std::ptr::null(),
                image_view: draw_image_view.get_vk_handle(),
                image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                resolve_mode: vk::ResolveModeFlags::NONE,
                resolve_image_view: vk::ImageView::null(),
                resolve_image_layout: vk::ImageLayout::UNDEFINED,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::STORE,
                clear_value: vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 0.0],
                    },
                },
            }
        };

        self.cmd_buf.begin_rendering(
            vk::RenderingFlags::empty(),
            vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: extent.width,
                    height: extent.height,
                },
            },
            1,
            0,
            Some(&[color_attachment]),
            None,
            None,
        );

        println!("swapchain_image[{}] = {:p}", image_index, unsafe {
            swapchain_image.get_vk_handle()
        });

        self.cmd_buf
            .bind_pipeline(context.graphics_pipeline.as_ref());

        self.cmd_buf.set_viewport(
            0,
            &[vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            }],
        );

        self.cmd_buf.set_scissor(
            0,
            &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: *extent,
            }],
        );

        let mut vertex_buffers = vec![];
        for x in &context.vertex_buffers {
            vertex_buffers.push((x, 0u64));
        }

        self.cmd_buf
            .bind_index_buffer(&context.index_buffer, 0, vk::IndexType::UINT16);

        self.cmd_buf.bind_vertex_buffers(0, &vertex_buffers);

        self.cmd_buf.bind_descriptor_sets(
            vk::PipelineBindPoint::GRAPHICS,
            &context.pipeline_layout,
            0,
            &[&context.descriptor_sets[self.index]],
        );

        self.cmd_buf
            .draw_indexed(context.indices.len().try_into().unwrap(), 1, 0, 0, 0);

        self.cmd_buf.end_rendering();

        self.cmd_buf.transition_image(
            &draw_image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        );

        self.cmd_buf.transition_image(
            &swapchain_image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        self.copy_image_to_image(&self.cmd_buf, &draw_image, &swapchain_image);

        self.cmd_buf.transition_image(
            &swapchain_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        self.cmd_buf.end();
    }

    pub fn copy_image_to_image(&self, cmd: &CommandBuffer, src: &Image, dst: &Image) {
        let src_size = src.extent();
        let dst_size = dst.extent();

        let blit_region = vk::ImageBlit2 {
            s_type: vk::StructureType::IMAGE_BLIT_2,
            p_next: std::ptr::null(),
            src_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: src_size.width.try_into().unwrap(),
                    y: src_size.height.try_into().unwrap(),
                    z: 1,
                },
            ],
            dst_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: dst_size.width.try_into().unwrap(),
                    y: dst_size.height.try_into().unwrap(),
                    z: 1,
                },
            ],
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        let blit_info = unsafe {
            vk::BlitImageInfo2 {
                s_type: vk::StructureType::BLIT_IMAGE_INFO_2,
                p_next: std::ptr::null(),
                src_image: src.get_vk_handle(),
                src_image_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                dst_image: dst.get_vk_handle(),
                dst_image_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                filter: vk::Filter::LINEAR,
                p_regions: &blit_region,
                region_count: 1,
            }
        };

        cmd.blit_image(&blit_info)
    }
}
