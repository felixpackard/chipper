use std::{path::PathBuf, sync::Arc, time};

use anyhow::Context;
use chip8::Chip8;
use clap::{command, Parser};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{self, EventLoop},
    platform::{
        pump_events::{EventLoopExtPumpEvents, PumpStatus},
        scancode::PhysicalKeyExtScancode,
    },
    window::Window,
};

const SCALE_FACTOR: u32 = 10;
const FRAME_INTERVAL: time::Duration = time::Duration::new(0, 1_000_000_000u32 / 60);

struct AppConfig {
    pub window: winit::window::WindowAttributes,
    pub args: Args,
}

impl AppConfig {
    pub fn new(args: Args) -> Self {
        Self {
            window: Window::default_attributes()
                .with_title("CHIP-8")
                .with_inner_size(LogicalSize::new(
                    (chip8::SCREEN_WIDTH as u32) * SCALE_FACTOR,
                    (chip8::SCREEN_HEIGHT as u32) * SCALE_FACTOR,
                ))
                .with_resizable(false),
            args,
        }
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

    pub fn init(&mut self, event_loop: &event_loop::ActiveEventLoop) -> anyhow::Result<()> {
        let mut chip8 = Chip8::new()
            .context("construct new chip8 instance")?
            .legacy_shift(self.config.args.legacy_shift)
            .jump_add_offset(self.config.args.jump_add_offset)
            .memory_increment_i(self.config.args.memory_increment_i);

        if let Some(path) = self.config.args.load.to_owned() {
            chip8
                .load_rom_from_file(path)
                .context("load rom from file")?;
        }

        println!("{}", chip8);

        let window = event_loop
            .create_window(self.config.window.to_owned())
            .context("create window")?;
        let window = Arc::new(window);

        let window_size = window.inner_size();
        let surface_texture =
            SurfaceTexture::new(window_size.width, window_size.height, window.clone());

        let pixels = Pixels::new(
            chip8::SCREEN_WIDTH as u32,
            chip8::SCREEN_HEIGHT as u32,
            surface_texture,
        )
        .context("create pixels instance")?;

        self.state = Some(State {
            chip8,
            window,
            pixels,
        });

        App::render(self.state.as_mut().unwrap());
        self.state.as_ref().unwrap().window.request_redraw();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        if let Err(e) = self.init(event_loop) {
            eprintln!("init failed: {:?}", e);
            std::process::exit(1);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Exiting...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let Some(state) = self.state.as_mut() else {
                    return;
                };

                state.window.pre_present_notify();
                App::render(state);
            }
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let Some(scancode) = event.physical_key.to_scancode() {
                    let Some(state) = self.state.as_mut() else {
                        return;
                    };

                    if event.state.is_pressed() {
                        if event.repeat {
                            return;
                        }
                        if let Err(e) = state.chip8.keydown(scancode) {
                            eprintln!("keydown failed: {:?}", e);
                        }
                    } else {
                        if let Err(e) = state.chip8.keyup(scancode) {
                            eprintln!("keyup failed: {:?}", e);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

impl App {
    pub fn render(state: &mut State) {
        let fb = state.chip8.fb();
        for (i, pixel) in state.pixels.frame_mut().chunks_exact_mut(4).enumerate() {
            let x = i % chip8::SCREEN_WIDTH;
            let y = i / chip8::SCREEN_WIDTH;

            let rgba = if fb[y][x] == 1 {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            };

            pixel.copy_from_slice(&rgba);
        }

        state.pixels.render().unwrap();
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "PATH", help = "Load ROM into memory", value_hint = clap::ValueHint::FilePath)]
    load: Option<PathBuf>,
    #[arg(long, help = "Toggle shift operation modes")]
    legacy_shift: bool,
    #[arg(long, help = "Toggle jump operation modes")]
    jump_add_offset: bool,
    #[arg(long, help = "Toggle memory read/write operation modes")]
    memory_increment_i: bool,
}

fn main() -> std::process::ExitCode {
    env_logger::init();

    let mut event_loop = EventLoop::new().unwrap();

    let args = Args::parse();
    let config = AppConfig::new(args);

    let mut app = App::new(config);

    loop {
        let timeout = Some(time::Duration::ZERO);
        let status = event_loop.pump_app_events(timeout, &mut app);

        if let PumpStatus::Exit(exit_code) = status {
            break std::process::ExitCode::from(exit_code as u8);
        }

        if let Some(state) = app.state.as_mut() {
            state.chip8.cycle();
            if state.chip8.is_fb_dirty() {
                state.window.clone().request_redraw();
            }
        }

        std::thread::sleep(FRAME_INTERVAL);
    }
}
