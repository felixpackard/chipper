use std::{path::PathBuf, sync::Arc};

use chip8::Chip8;
use clap::{command, Parser};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler, dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop,
    window::Window,
};

const SCALE_FACTOR: u32 = 10;

#[derive(Default)]
struct AppConfig {
    pub window: winit::window::WindowAttributes,
    pub load: Option<PathBuf>,
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
            load: None,
        }
    }

    pub fn load(mut self, path: Option<PathBuf>) -> Self {
        self.load = path;
        self
    }
}

struct State {
    pub(crate) chip8: Chip8,
    pub(crate) window: Arc<Window>,
    pub(crate) pixels: Pixels<'static>,
}

struct App {
    config: AppConfig,
    state: Option<State>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            state: None,
        }
    }

    pub fn init(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut chip8 = Chip8::new().unwrap();

        if let Some(path) = self.config.load.to_owned() {
            chip8.load_rom_from_file(path).unwrap();
        }

        println!("{}", chip8);

        let window = event_loop
            .create_window(self.config.window.to_owned())
            .unwrap();
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
        self.init(event_loop);
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "PATH", help = "Load ROM into memory", value_hint = clap::ValueHint::FilePath)]
    load: Option<PathBuf>,
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let args = Args::parse();
    let config = AppConfig::new().load(args.load);

    let mut state = App::new(config);

    event_loop.run_app(&mut state).unwrap();
}
