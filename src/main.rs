#[allow(dead_code)]
mod gpu;

use ash::vk;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event;
use winit::event::VirtualKeyCode;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new();

    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_title("vulka")
            .with_resizable(true)
            .with_decorations(true)
            .build(&event_loop)
            .expect("failed to create window"),
    );

    let instance = gpu::Instance::new(&window);

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

    // Use MAILBOX if the device supports it, otherwise fallback to FIFO
    let surface_present_mode = physical_device
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
    let surface_format = physical_device
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

    let surface_extent = physical_device.get_surface_current_extent_clamped(window.inner_size());

    println!("surface_present_mode = {:?}", surface_present_mode);
    println!("surface_present_format = {:?}", surface_format);
    println!("surface_extent = {:?}", surface_extent);

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

    let swapchain = device.get_swapchain(
        physical_device.get_surface_ideal_image_count(),
        surface_format.format,
        surface_format.color_space,
        surface_extent,
        vk::ImageUsageFlags::COLOR_ATTACHMENT,
        surface_present_mode,
        None,
    );

    let swapchain_images = swapchain.images();

    let swapchain_image_views = swapchain_images
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

    println!(
        "swapchain_image_views.len() = {}",
        swapchain_image_views.len()
    );

    let compiler = shaderc::Compiler::new().unwrap();

    let shader_modules = vec![
        gpu::ShaderModule::new(
            &device,
            &compiler,
            include_str!("./shaders/vertex.glsl"),
            gpu::ShaderKind::Vertex,
            "vertex.glsl",
            "main",
            None,
        ),
        gpu::ShaderModule::new(
            &device,
            &compiler,
            include_str!("./shaders/fragment.glsl"),
            gpu::ShaderKind::Fragment,
            "fragment.glsl",
            "main",
            None,
        ),
    ];

    let render_pass = {
        let mut builder = gpu::RenderPass::builder();

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

        builder.build(gpu::RenderPassConfig {
            device: &device,
            attachments: vec![color],
            subpasses: vec![subpass],
            dependencies: Some(vec![dep]),
        })
    };

    let pipeline_layout = gpu::PipelineLayout::new(&device, None, None);

    let graphics_pipeline = gpu::GraphicsPipeline::new(
        &device,
        &shader_modules,
        &vec![vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR],
        vk::PrimitiveTopology::TRIANGLE_LIST,
        false,
        None,
        None,
        &pipeline_layout,
        &render_pass,
    );

    let mut framebuffers = vec![];

    for image_view in &swapchain_image_views {
        let vk::Extent2D { width, height } = *swapchain.extent();
        framebuffers.push(gpu::Framebuffer::new(
            &render_pass,
            &vec![image_view.clone()],
            width,
            height,
            1,
        ));
    }

    println!("framebuffers.len() = {}", framebuffers.len());

    // TODO ...

    let graphics_queue = device.get_first_queue(vk::QueueFlags::GRAPHICS).unwrap();

    let cmd_pool = gpu::CommandBufferPool::new(
        &device,
        graphics_queue.queue_family(),
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    );

    let mut cmd_buf = cmd_pool.allocate_one(vk::CommandBufferLevel::PRIMARY);

    let image_available = gpu::Semaphore::new(&device);
    let render_finished = gpu::Semaphore::new(&device);
    let in_flight = gpu::Fence::signaled(&device);

    event_loop.run(move |event, _, control_flow| match event {
        event::Event::WindowEvent { event, .. } => match event {
            event::WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            event::WindowEvent::KeyboardInput { input, .. } => {
                if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                    *control_flow = ControlFlow::Exit;
                }
            }
            event::WindowEvent::Resized(inner_size) => {
                println!("{:?}", inner_size);
            }
            _ => {}
        },
        event::Event::MainEventsCleared => draw_frame(
            &device,
            &render_pass,
            &mut cmd_buf,
            &in_flight,
            &image_available,
            &render_finished,
            &swapchain,
            &framebuffers,
            &graphics_pipeline,
        ),
        event::Event::LoopDestroyed => {
            device.wait_idle();
        }
        _ => {}
    })
}

fn record_command_buffer(
    cmd_buf: &mut gpu::CommandBuffer,
    render_pass: &Arc<gpu::RenderPass>,
    image_index: u32,
    framebuffers: &Vec<Arc<gpu::Framebuffer>>,
    swapchain: &Arc<gpu::SwapChain>,
    pipeline: &Arc<gpu::GraphicsPipeline>,
) {
    cmd_buf.begin(vk::CommandBufferUsageFlags::empty());

    let extent = swapchain.extent();
    let framebuffer = framebuffers[image_index as usize].clone();

    cmd_buf.begin_render_pass(
        render_pass,
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

    cmd_buf.bind_pipeline(pipeline);

    cmd_buf.set_viewport(
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

    cmd_buf.set_scissor(
        0,
        &[vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: *extent,
        }],
    );

    cmd_buf.draw(3, 1, 0, 0);
    cmd_buf.end_render_pass();
    cmd_buf.end();
}

fn draw_frame(
    device: &Arc<gpu::Device>,
    render_pass: &Arc<gpu::RenderPass>,
    cmd_buf: &mut gpu::CommandBuffer,
    in_flight_fence: &gpu::Fence,
    image_available_sema: &gpu::Semaphore,
    render_finished_sema: &gpu::Semaphore,
    swapchain: &Arc<gpu::SwapChain>,
    framebuffers: &Vec<Arc<gpu::Framebuffer>>,
    pipeline: &Arc<gpu::GraphicsPipeline>,
) {
    let fences = &[in_flight_fence];
    device.wait_for_fences(fences, true, None);
    device.reset_fences(fences);

    let acquire_result = swapchain.acquire_next_image(None, Some(&image_available_sema), None);
    let image_index: u32;

    match acquire_result {
        Ok((acquired_index, suboptimal)) => {
            image_index = acquired_index;

            if suboptimal {
                todo!()
            }
        }
        Err(result) => match result {
            vk::Result::ERROR_OUT_OF_DATE_KHR => todo!(),
            vk::Result::ERROR_SURFACE_LOST_KHR => todo!(),
            vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => todo!(),
            _ => panic!("acquire_result = {:?}", result),
        },
    }

    cmd_buf.reset();

    record_command_buffer(
        cmd_buf,
        render_pass,
        image_index,
        framebuffers,
        swapchain,
        pipeline,
    );

    let graphics_queue = device.get_first_queue(vk::QueueFlags::GRAPHICS).unwrap();
    let present_queue = device.get_first_present_queue().unwrap();

    graphics_queue.submit(
        &[(
            image_available_sema,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        )],
        &[cmd_buf],
        Some(&[render_finished_sema]),
        Some(&in_flight_fence),
    );

    let present_result =
        present_queue.submit_present(&[render_finished_sema], &swapchain, image_index);

    match present_result {
        Ok(suboptimal) => {
            if suboptimal {
                todo!()
            }
        }
        Err(result) => match result {
            vk::Result::ERROR_SURFACE_LOST_KHR => todo!(),
            vk::Result::ERROR_FULL_SCREEN_EXCLUSIVE_MODE_LOST_EXT => todo!(),
            _ => panic!("present_result = {:?}", result),
        },
    }
}
