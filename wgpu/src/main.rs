use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::LogicalSize, event_loop::EventLoop, window::Window,
};

const SCALE_FACTOR: u32 = 10;

#[derive(Default)]
struct AppConfig {
    pub window: winit::window::WindowAttributes,
}

impl AppConfig {
    pub fn new() -> Self {
        Self {
            window: Window::default_attributes()
                .with_title("CHIP-8")
                .with_inner_size(LogicalSize::new(
                    chip8::SCREEN_WIDTH * SCALE_FACTOR,
                    chip8::SCREEN_HEIGHT * SCALE_FACTOR,
                )),
        }
    }
}

struct State {
    pub(crate) window: Arc<Window>,
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl App {
    pub fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, config: AppConfig) {
        let window = event_loop.create_window(config.window).unwrap();
        let window = Arc::new(window);
        self.state = Some(State { window });
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let config = AppConfig::new();
        self.init(event_loop, config);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            winit::event::WindowEvent::CloseRequested => {
                println!("Exiting...");
                event_loop.exit();
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut state = App::default();
    event_loop.run_app(&mut state).unwrap();
}
