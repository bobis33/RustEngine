use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop};

mod system_info;
mod window;

use system_info::SystemInfo;
use window::winit_app;

fn main() {
    SystemInfo::new().print();
    entry(EventLoop::new().unwrap());
}

pub(crate) fn entry(event_loop: EventLoop<()>) {
    let app = winit_app::WinitAppBuilder::with_init(
        |elwt| {
            winit_app::make_window(elwt, |attrs| {
                attrs.with_title("VEngine RS").with_inner_size(
                    winit::dpi::PhysicalSize { width: 1920, height: 1080 },
                )
            })
        },
        |_elwt, _window| (),
    )
    .with_event_handler(|_window, _surface, window_id, event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            WindowEvent::CloseRequested if _window.id() == window_id => {
                elwt.exit();
            }
            WindowEvent::KeyboardInput { event, .. }    => {
                if let winit::event::KeyEvent {
                    state: winit::event::ElementState::Pressed,
                    logical_key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                    ..
                } = event
                {
                    elwt.exit();
                }
            }
            _ => {}
        }
    });

    winit_app::run_app(event_loop, app);
}
