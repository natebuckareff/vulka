use ash::vk;
use memoffset::offset_of;
use std::{mem::size_of, rc::Rc, sync::Arc};
use winit::window::Window;

use crate::gpu::{
    Buffer, CommandBuffer, CommandPool, Device, Fence, Framebuffer, GraphicsPipeline, Instance,
    PhysicalDevice, PipelineLayout, RenderPass, RenderPassConfig, Semaphore, ShaderKind,
    ShaderModule, Swapchain,
};

pub struct RenderContext {
    instance: Arc<Instance>,
    physical_device: Arc<PhysicalDevice>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    shader_modules: Vec<Arc<ShaderModule>>,
    render_pass: Arc<RenderPass>,
    graphics_pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
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
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl RenderContext {
    pub fn new(window: &Arc<Window>, max_frames_in_flight: usize) -> Self {
        let instance = Instance::new(&window);

        let required_queue_flags = vec![vk::QueueFlags::GRAPHICS];

        let required_extensions: Vec<String> = vec![
            // String::from("VK_EXT_debug_utils"),
            String::from("VK_KHR_swapchain"),
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
                let mut flags = required_queue_flags.clone();
                for (i, properties) in x.get_queue_family_properties().iter().enumerate() {
                    if x.supports_surface(i.try_into().unwrap()) {
                        supports_surface = true;
                    }
                    flags.retain(|x| !properties.queue_flags.contains(*x));
                }
                supports_surface && flags.len() == 0
            })
            .filter(|x| {
                // Filter for physical devices that support all of the required
                // extensions
                let extensions_hashset = x.extension_name_hashset();
                for x in &required_extensions {
                    if !extensions_hashset.contains(x) {
                        return false;
                    }
                }
                true
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

        let device = physical_device.get_device(&queue_family_indices, &required_extensions);

        let swapchain = {
            let inner_size = window.inner_size();
            RenderContext::_create_swapchain(&device, inner_size.width, inner_size.height, None)
        };

        let shader_compiler = shaderc::Compiler::new().unwrap();

        let shader_modules = vec![
            ShaderModule::new(
                &device,
                &shader_compiler,
                include_str!("./shaders/vertex.glsl"),
                ShaderKind::Vertex,
                "vertex.glsl",
                "main",
                None,
            ),
            ShaderModule::new(
                &device,
                &shader_compiler,
                include_str!("./shaders/fragment.glsl"),
                ShaderKind::Fragment,
                "fragment.glsl",
                "main",
                None,
            ),
        ];

        let render_pass = {
            let mut builder = RenderPass::builder();

            let color = builder
                .attachment()
                .format(*swapchain.format())
                .samples(vk::SampleCountFlags::TYPE_1)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

            let subpass = builder
                .subpass()
                .color(&color, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            let dep = builder
                .dependency()
                .src_external()
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst(&subpass)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

            builder.build(RenderPassConfig {
                device: &device,
                attachments: vec![color],
                subpasses: vec![subpass],
                dependencies: Some(vec![dep]),
            })
        };

        let pipeline_layout = PipelineLayout::new(&device, None, None);

        let graphics_queue = device.get_first_queue(vk::QueueFlags::GRAPHICS).unwrap();

        let cmd_pool = CommandPool::new(
            &device,
            graphics_queue.queue_family(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        );

        let indices: Vec<u16> = vec![0, 1, 2, 2, 3, 0];

        #[rustfmt::skip]
        let vertices = [
            Vertex { position: [-0.5, -0.5], color: [1.0, 0.0, 0.0] },
            Vertex { position: [ 0.5, -0.5], color: [0.0, 1.0, 0.0] },
            Vertex { position: [ 0.5,  0.5], color: [0.0, 0.0, 1.0] },
            Vertex { position: [-0.5,  0.5], color: [1.0, 1.0, 1.0] }
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
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, position).try_into().unwrap(),
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, color).try_into().unwrap(),
            },
        ];

        let index_buffer = {
            let buffer_size = size_of::<u16>() * indices.len();

            let mut staging_buffer =
                Buffer::new(&device, buffer_size, vk::BufferUsageFlags::TRANSFER_SRC);

            staging_buffer.allocate(
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            staging_buffer.copy_nonoverlapping(&indices);

            let mut index_buffer = Buffer::new(
                &device,
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            );

            index_buffer.allocate(vk::MemoryPropertyFlags::DEVICE_LOCAL);

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

            let mut staging_buffer =
                Buffer::new(&device, buffer_size, vk::BufferUsageFlags::TRANSFER_SRC);

            staging_buffer.allocate(
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            staging_buffer.copy_nonoverlapping(&vertices);

            let mut vertex_buffer = Buffer::new(
                &device,
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            );

            vertex_buffer.allocate(vk::MemoryPropertyFlags::DEVICE_LOCAL);

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
            &device,
            &shader_modules,
            Some(&[vertex_bindings]),
            Some(&vertex_attributes),
            &vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
            vk::PrimitiveTopology::TRIANGLE_LIST,
            false,
            None,
            None,
            &pipeline_layout,
            &render_pass,
        );

        let framebuffers = RenderContext::_create_framebuffers(&swapchain, &render_pass);

        println!("framebuffers.len() = {}", framebuffers.len());

        let mut render_context = Self {
            instance,
            physical_device,
            device,
            swapchain,
            shader_modules,
            render_pass,
            graphics_pipeline,
            framebuffers,
            indices,
            index_buffer,
            vertex_buffers,
            cmd_pool,
            render_frames: vec![],
            current_frame: 0,
        };

        render_context.render_frames.reserve(max_frames_in_flight);
        for _ in 0..max_frames_in_flight {
            render_context
                .render_frames
                .push(RenderFrame::new(&render_context));
        }

        render_context
    }

    fn _get_surface_details(
        physical_device: &Arc<PhysicalDevice>,
        width: u32,
        height: u32,
    ) -> SurfaceDetails {
        // Use MAILBOX if the device supports it, otherwise fallback to FIFO
        let present_mode = physical_device
            .get_surface_present_modes()
            .into_iter()
            .min_by_key(|x| match *x {
                vk::PresentModeKHR::MAILBOX => 0,
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
        device: &Arc<Device>,
        width: u32,
        height: u32,
        old_swapchain: Option<&Arc<Swapchain>>,
    ) -> Arc<Swapchain> {
        let physical_device = device.physical_device();

        let SurfaceDetails {
            present_mode,
            format,
            extent,
        } = RenderContext::_get_surface_details(physical_device, width, height);

        let swapchain = device.get_swapchain(
            physical_device.get_surface_ideal_image_count(),
            format.format,
            format.color_space,
            extent,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            present_mode,
            old_swapchain,
        );

        swapchain
    }

    fn _create_framebuffers(
        swapchain: &Arc<Swapchain>,
        render_pass: &Arc<RenderPass>,
    ) -> Vec<Arc<Framebuffer>> {
        let mut framebuffers = vec![];

        let swapchain_image_views = swapchain
            .images()
            .iter()
            .map(|image| {
                image.get_image_view(
                    vk::ImageViewType::TYPE_2D,
                    *swapchain.format(),
                    vk::ComponentMapping::default(),
                    vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                )
            })
            .collect::<Vec<_>>();

        for image_view in &swapchain_image_views {
            let vk::Extent2D { width, height } = *swapchain.extent();
            framebuffers.push(Framebuffer::new(
                &render_pass,
                vec![image_view.clone()],
                width,
                height,
                1,
            ));
        }

        framebuffers
    }

    pub fn recreate_swapchain(&mut self, width: u32, height: u32) {
        self.device.wait_idle();

        self.swapchain =
            RenderContext::_create_swapchain(&self.device, width, height, Some(&self.swapchain));

        self.framebuffers = RenderContext::_create_framebuffers(&self.swapchain, &self.render_pass);

        let max_frames_in_flight = self.render_frames.len();

        self.render_frames.clear();

        for _ in 0..max_frames_in_flight {
            self.render_frames.push(RenderFrame::new(&self));
        }
    }

    pub fn draw_next_frame(&mut self) {
        self.render_frames[self.current_frame].draw_frame(self);
        self.current_frame = (self.current_frame + 1) % self.render_frames.len();
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
    cmd_buf: CommandBuffer,
    image_available: Semaphore,
    render_finished: Semaphore,
    in_flight: Fence,
}

impl RenderFrame {
    pub fn new(context: &RenderContext) -> Self {
        let cmd_buf = context
            .cmd_pool
            .allocate_one(vk::CommandBufferLevel::PRIMARY);

        let image_available = Semaphore::new(&context.device);
        let render_finished = Semaphore::new(&context.device);
        let in_flight = Fence::signaled(&context.device);

        Self {
            cmd_buf,
            image_available,
            render_finished,
            in_flight,
        }
    }

    pub fn draw_frame(&self, context: &RenderContext) {
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
                    // TODO: Should recreate the swapchain
                    todo!()
                }
            }
            Err(result) => match result {
                vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    // TODO: Should recreate the swapchain
                    todo!();
                }
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
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            )]),
            &[&self.cmd_buf],
            Some(&[&self.render_finished]),
            Some(&self.in_flight),
        );

        let present_result =
            present_queue.submit_present(&[&self.render_finished], &context.swapchain, image_index);

        match present_result {
            Ok(suboptimal) => {
                if suboptimal {
                    // TODO: Should recreate the swapchain
                    todo!()
                }
            }
            Err(result) => match result {
                vk::Result::ERROR_OUT_OF_DATE_KHR => {
                    // TODO: Should recreate the swapchain
                    todo!();
                }
                vk::Result::ERROR_SURFACE_LOST_KHR => todo!(),
                vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => todo!(),
                _ => panic!("present_result = {:?}", result),
            },
        }
    }

    pub fn record_commands(&self, context: &RenderContext, image_index: u32) {
        self.cmd_buf.begin(vk::CommandBufferUsageFlags::empty());

        let extent = context.swapchain.extent();
        let framebuffer = context.framebuffers[image_index as usize].clone();

        self.cmd_buf.begin_render_pass(
            &context.render_pass,
            &framebuffer,
            vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: *extent,
            },
            Some(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }]),
            vk::SubpassContents::INLINE,
        );

        self.cmd_buf.bind_pipeline(&context.graphics_pipeline);

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

        self.cmd_buf
            .draw_indexed(context.indices.len().try_into().unwrap(), 1, 0, 0, 0);

        self.cmd_buf.end_render_pass();
        self.cmd_buf.end();
    }
}
