use std::{path::PathBuf, str::FromStr, time::Duration};

use anyhow::Context;
use chip8::{Chip8, FrameBuffer, Key};
use gpui::{
    actions, canvas, div, fill, point, prelude::*, px, size, App, Application, Bounds, FocusHandle,
    KeyBinding, KeyDownEvent, KeyUpEvent, Menu, MenuItem, Pixels, Window, WindowBounds,
    WindowOptions,
};

const SCALE_FACTOR: f32 = 16.;
const FRAME_INTERVAL: Duration = Duration::new(0, 1_000_000_000u32 / 60);

actions!(chipper, [Quit, CloseWindow]);

struct Chipper {
    focus_handle: FocusHandle,
    chip8: Chip8,
}

impl Chipper {
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        _cx: &mut gpui::Context<Self>,
    ) {
        // TODO: Unfortunately there doesn't seem to be a way to use scancodes in gpui right now,
        // so we're just using the key label
        self.chip8
            .keydown(Key::from_label(event.keystroke.key.as_str()))
            .context("Failed to handle key down event")
            .unwrap();
    }

    fn key_up(&mut self, event: &KeyUpEvent, _window: &mut Window, _cx: &mut gpui::Context<Self>) {
        // TODO: Unfortunately there doesn't seem to be a way to use scancodes in gpui right now,
        // so we're just using the key label
        self.chip8
            .keyup(Key::from_label(event.keystroke.key.as_str()))
            .context("Failed to handle key up event")
            .unwrap();
    }
}

impl Render for Chipper {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let fb = self.chip8.fb();

        let paint_framebuffer =
            move |bounds: Bounds<Pixels>, fb: FrameBuffer, window: &mut Window, _: &mut App| {
                let start_y = bounds.origin.y.0;
                let height = bounds.size.height.0;
                let start_x = bounds.origin.x.0;
                let width = bounds.size.width.0;

                let pixel_height = height / 32.0;
                let pixel_width = width / 64.0;

                for y in 0..32 {
                    for x in 0..64 {
                        if fb[y][x] == 1 {
                            let rect = Bounds::new(
                                point(
                                    px(start_x + x as f32 * pixel_width),
                                    px(start_y + y as f32 * pixel_height),
                                ),
                                size(px(pixel_width), px(pixel_height)),
                            );
                            window.paint_quad(fill(rect, gpui::white()));
                        }
                    }
                }
            };

        div()
            .on_action(|_: &Quit, _, app| {
                app.quit();
            })
            .on_action(|_: &CloseWindow, window, _| {
                window.remove_window();
            })
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .track_focus(&self.focus_handle)
            .size_full()
            .child(canvas(move |_, _, _| fb, paint_framebuffer).size_full())
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.activate(true);

        cx.on_action(quit);

        cx.set_menus(vec![Menu {
            name: "chipper".into(),
            items: vec![MenuItem::action("Quit", Quit)],
        }]);

        let bounds = Bounds::centered(
            None,
            size(
                px(chip8::SCREEN_WIDTH as f32 * SCALE_FACTOR),
                px(chip8::SCREEN_HEIGHT as f32 * SCALE_FACTOR),
            ),
            cx,
        );

        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
        ]);

        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let mut chip8 = Chip8::new()
                        .context("Failed to create new Chip8 instance")
                        .unwrap();
                    chip8
                        .load_rom_from_file(
                            PathBuf::from_str("../roms/programs/Keypad Test [Hap, 2006].ch8")
                                .unwrap(),
                        )
                        .context("Failed to load ROM from file")
                        .unwrap();

                    cx.new(|cx| {
                        let focus_handle = cx.focus_handle();
                        focus_handle.focus(window);
                        Chipper {
                            focus_handle,
                            chip8,
                        }
                    })
                },
            )
            .context("Failed to open the window")
            .unwrap();

        cx.spawn(move |mut cx| async move {
            loop {
                cx.update_window(window.into(), |root_view, _, cx| {
                    if let Ok(chipper_view) = root_view.downcast::<Chipper>() {
                        chipper_view.update(cx, |chipper, cx| {
                            chipper.chip8.cycle();
                            cx.notify();
                        });
                    }
                })
                .ok();

                gpui::Timer::after(FRAME_INTERVAL).await;
            }
        })
        .detach();
    })
}

fn quit(_: &Quit, cx: &mut App) {
    println!("Gracefully quitting the application...");
    cx.quit();
}
