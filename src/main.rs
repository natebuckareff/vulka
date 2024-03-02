#[allow(dead_code)]
mod gpu;
mod input;
mod render_context;

use gilrs::Gilrs;
use input::InputManager;
use input::{MouseControl, RawGamepadEvent, RawMouseEvent};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::event::{self, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::WindowBuilder;

fn main() {
    let start_time = Instant::now();
    let event_loop = EventLoop::new().expect("failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let window = Arc::new(
        WindowBuilder::new()
            .with_inner_size(LogicalSize::new(1024, 768))
            .with_title("vulka")
            .with_resizable(true)
            .with_decorations(true)
            .build(&event_loop)
            .expect("failed to create window"),
    );

    let mut render_context = render_context::RenderContext::new(window.clone(), 2);

    let mut gilrs = Gilrs::new().unwrap();
    let mut kbd_manager = InputManager::new(start_time);
    let mut mouse_manager = InputManager::new(start_time);
    let mut gamepad_manager = InputManager::new(start_time);

    kbd_manager.set_action(PhysicalKey::Code(KeyCode::Space), (), None);
    mouse_manager.set_action(MouseControl::Button(MouseButton::Left), (), None);
    gamepad_manager.set_wildcard_action((), None);

    event_loop
        .run(move |event, target| match event {
            event::Event::WindowEvent { event, .. } => match event {
                event::WindowEvent::CloseRequested => target.exit(),
                event::WindowEvent::KeyboardInput {
                    device_id, event, ..
                } => {
                    let raw = input::RawKeyboardEvent { device_id, event };
                    kbd_manager.update(&raw);
                    for i in 0..kbd_manager.get_input_event_count() {
                        println!("{:?}", kbd_manager.get_nth_last_input_event(i));
                    }
                    kbd_manager.flush_input_events();

                    if raw.event.logical_key == Key::Named(NamedKey::Escape) {
                        target.exit()
                    }
                }
                event::WindowEvent::MouseInput { .. } => {
                    let raw = RawMouseEvent::from_window_event(event);
                    mouse_manager.update(&raw);
                    for i in 0..mouse_manager.get_input_event_count() {
                        println!("{:?}", mouse_manager.get_nth_last_input_event(i));
                    }
                    mouse_manager.flush_input_events();
                }
                event::WindowEvent::MouseWheel { .. } => {
                    let raw = RawMouseEvent::from_window_event(event);
                    mouse_manager.update(&raw);
                    for i in 0..mouse_manager.get_input_event_count() {
                        println!("{:?}", mouse_manager.get_nth_last_input_event(i));
                    }
                    mouse_manager.flush_input_events();
                }
                event::WindowEvent::CursorMoved { .. } => {
                    let raw = RawMouseEvent::from_window_event(event);
                    mouse_manager.update(&raw);
                    for i in 0..mouse_manager.get_input_event_count() {
                        println!("{:?}", mouse_manager.get_nth_last_input_event(i));
                    }
                    mouse_manager.flush_input_events();
                }
                event::WindowEvent::Resized(inner_size) => {
                    render_context.recreate_swapchain(inner_size.width, inner_size.height);
                }
                event::WindowEvent::RedrawRequested => {
                    while let Some(event) = gilrs.next_event() {
                        let raw = RawGamepadEvent::from_gilrs_event(event);
                        gamepad_manager.update(&raw);
                        for i in 0..gamepad_manager.get_input_event_count() {
                            println!("{:?}", gamepad_manager.get_nth_last_input_event(i));
                        }
                        gamepad_manager.flush_input_events();
                    }
                    render_context.draw_next_frame();
                }
                _ => {}
            },
            _ => {}
        })
        .expect("event loop failed")
}
