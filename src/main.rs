#[allow(dead_code)]
mod gpu;
mod render_context;

use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event;
use winit::keyboard::{Key, NamedKey};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");

    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_title("vulka")
            .with_resizable(true)
            .with_decorations(true)
            .build(&event_loop)
            .expect("failed to create window"),
    );

    let mut render_context = render_context::RenderContext::new(&window, 2);

    event_loop.run(move |event, target| match event {
        event::Event::WindowEvent { event, .. } => match event {
            event::WindowEvent::CloseRequested => {
                target.exit()
            }
            event::WindowEvent::KeyboardInput { event, .. } => {
                if event.logical_key == Key::Named(NamedKey::Escape) {
                    target.exit()
                }
            }
            event::WindowEvent::Resized(inner_size) => {
                render_context.recreate_swapchain(inner_size.width, inner_size.height);
            }
            _ => {}
        },
        event::Event::AboutToWait => {
            render_context.draw_next_frame();
        }
        _ => {}
    }).expect("event loop failed")
}
