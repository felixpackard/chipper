use std::sync::Arc;

use chip8::Chip8;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler, dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop,
    window::Window,
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
                    (chip8::SCREEN_WIDTH as u32) * SCALE_FACTOR,
                    (chip8::SCREEN_HEIGHT as u32) * SCALE_FACTOR,
                ))
                .with_resizable(false),
        }
    }
}

struct State {
    pub(crate) chip8: Chip8,
    pub(crate) window: Arc<Window>,
    pub(crate) pixels: Pixels<'static>,
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl App {
    pub fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, config: AppConfig) {
        let chip8 = Chip8::new().unwrap();

        let window = event_loop.create_window(config.window).unwrap();
        let window = Arc::new(window);

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());

        let pixels = Pixels::new(
            chip8::SCREEN_WIDTH as u32,
            chip8::SCREEN_HEIGHT as u32,
            surface_texture,
        )
        .unwrap();

        self.state = Some(State {
            chip8,
            window,
            pixels,
        });

        self.render();
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
            WindowEvent::CloseRequested => {
                println!("Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => (),
        }
    }
}

impl App {
    pub fn render(&mut self) {
        if let Some(state) = &mut self.state {
            let fb = state.chip8.fb();
            for (i, pixel) in state.pixels.frame_mut().chunks_exact_mut(4).enumerate() {
                let byte = fb[i / 8];
                let bit = i % 8;
                let on = (byte & (1 << (7 - bit))) != 0;

                let rgba = if on {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };

                pixel.copy_from_slice(&rgba);
            }

            state.pixels.render().unwrap();
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
