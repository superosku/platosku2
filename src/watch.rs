//! A window that replays recorded simulations (see [`crate::sim::Recording`]),
//! e.g. the gameplay tests: `cargo test --test gameplay -- --watch [filter]`.
//!
//! Keys: Space pause, `.` step one tick, `+`/`-` speed, R restart, N/→ next,
//! P/← previous, Esc quit.

use crate::camera::Camera;
use crate::render::Renderer;
use crate::sim::{Recording, Sim};
use crate::state::GameState;
use egui_miniquad as egui_mq;
use miniquad::*;

/// One test (or other named run) to watch.
pub struct WatchItem {
    pub name: String,
    /// The failure message, if the test failed.
    pub failure: Option<String>,
    /// The recorded sims of the test, usually exactly one.
    pub recordings: Vec<Recording>,
}

/// One recording to replay, with its title.
struct Clip {
    title: String,
    failure: Option<String>,
    recording: Recording,
}

const MIN_SPEED: f32 = 0.25;
const MAX_SPEED: f32 = 16.0;
/// Seconds to stay on the last frame of a passed test before moving on.
const END_PAUSE_SECONDS: f64 = 0.1;
/// Upper bound of ticks per frame, so a slow frame doesn't skip ahead too much.
const MAX_TICKS_PER_FRAME: u32 = 64;

/// Opens a window and replays the items one after another at `speed` times
/// normal game speed. Returns when the window is closed.
pub fn watch(items: Vec<WatchItem>, speed: f32) {
    let mut clips = Vec::new();
    for item in items {
        let count = item.recordings.len();
        if count == 0 {
            println!(
                "watch: {} has nothing to replay (no recorded sim)",
                item.name
            );
        }
        for (i, recording) in item.recordings.into_iter().enumerate() {
            let is_last = i + 1 == count;
            clips.push(Clip {
                title: if count > 1 {
                    format!("{} ({}/{})", item.name, i + 1, count)
                } else {
                    item.name.clone()
                },
                // A failure happens at the end of the last sim of the test.
                failure: if is_last { item.failure.clone() } else { None },
                recording,
            });
        }
    }
    if clips.is_empty() {
        println!("watch: nothing to show");
        return;
    }

    miniquad::start(
        conf::Conf {
            window_title: String::from("platosku2 watch"),
            high_dpi: false,
            window_width: 800,
            window_height: 600,
            ..Default::default()
        },
        move || {
            let (w, h) = window::screen_size();
            let mut renderer = Renderer::new();
            let egui_mq = egui_mq::EguiMq::new(&mut *renderer.ctx);
            let mut viewer = Viewer {
                sim: Sim::new(clips[0].recording.setup.clone().build()),
                clips,
                clip_index: 0,
                tick: 0,
                renderer,
                egui_mq,
                camera: Camera::new(0.0, 0.0, 2.0, w, h),
                camera_settled: false,
                speed: speed.clamp(MIN_SPEED, MAX_SPEED),
                paused: false,
                tick_accumulator: 0.0,
                last_time: date::now(),
                end_reached_at: None,
                replay_mismatch: false,
            };
            viewer.load_clip(0);
            Box::new(viewer)
        },
    );
}

struct Viewer {
    clips: Vec<Clip>,
    clip_index: usize,
    sim: Sim,
    /// Ticks replayed of the current clip.
    tick: usize,
    renderer: Renderer,
    egui_mq: egui_mq::EguiMq,
    camera: Camera,
    camera_settled: bool,
    speed: f32,
    paused: bool,
    tick_accumulator: f64,
    last_time: f64,
    /// When the last tick of the clip was reached (for the pause before the next).
    end_reached_at: Option<f64>,
    /// The replay ended somewhere else than the recorded run (should not happen).
    replay_mismatch: bool,
}

impl Viewer {
    fn clip(&self) -> &Clip {
        &self.clips[self.clip_index]
    }

    fn at_end(&self) -> bool {
        self.tick >= self.clip().recording.inputs.len()
    }

    fn load_clip(&mut self, index: usize) {
        self.clip_index = index;
        self.sim = Sim::new(self.clip().recording.setup.clone().build());
        self.tick = 0;
        self.camera = Camera::new(0.0, 0.0, 2.0, self.camera.screen_w, self.camera.screen_h);
        self.camera_settled = false;
        self.tick_accumulator = 0.0;
        self.end_reached_at = None;
        self.replay_mismatch = false;
        // Play the first tick right away. Before it the game has no current room,
        // so a frame drawn at tick 0 is all dark (and the camera isn't in place
        // yet), which flickers between tests.
        self.step();
    }

    fn step(&mut self) {
        if self.at_end() {
            return;
        }
        let input = self.clip().recording.inputs[self.tick].clone();
        self.sim.set_input(input).run(1);
        self.tick += 1;

        // Same as the game loop: update, then camera.
        if !self.camera_settled {
            // The camera eases towards the current room, which is known only after
            // the first tick. Settle it right away (doesn't touch the game state).
            for _ in 0..300 {
                self.sim.game.update_camera(&mut self.camera, false);
            }
            self.camera_settled = true;
        }
        self.sim.game.update_camera(&mut self.camera, false);

        if self.at_end() {
            let p = &self.sim.game.player.bb;
            self.replay_mismatch = (p.x, p.y) != self.clip().recording.final_player_pos;
            self.end_reached_at = Some(date::now());
        }
    }

    fn next_clip(&mut self) {
        if self.clip_index + 1 < self.clips.len() {
            self.load_clip(self.clip_index + 1);
        }
    }

    fn previous_clip(&mut self) {
        if self.clip_index > 0 {
            self.load_clip(self.clip_index - 1);
        }
    }

    fn status_text(&self) -> (String, egui::Color32) {
        if self.replay_mismatch {
            return (
                "Replay did not match the recorded run (nondeterminism?)".into(),
                egui::Color32::from_rgb(255, 170, 0),
            );
        }
        match (&self.clip().failure, self.at_end()) {
            (Some(msg), true) => (
                format!("FAILED: {msg}"),
                egui::Color32::from_rgb(255, 90, 90),
            ),
            (Some(_), false) => (
                "running (fails at the end)".into(),
                egui::Color32::from_rgb(255, 170, 120),
            ),
            (None, true) => ("passed".into(), egui::Color32::from_rgb(120, 220, 120)),
            (None, false) => ("running".into(), egui::Color32::LIGHT_GRAY),
        }
    }
}

impl EventHandler for Viewer {
    fn update(&mut self) {
        let now = date::now();
        let dt = (now - self.last_time).min(0.1);
        self.last_time = now;

        if !self.paused && !self.at_end() {
            self.tick_accumulator += dt * 60.0 * self.speed as f64;
            let mut ticks = 0;
            while self.tick_accumulator >= 1.0 && !self.at_end() && ticks < MAX_TICKS_PER_FRAME {
                self.step();
                self.tick_accumulator -= 1.0;
                ticks += 1;
            }
        }
        if self.at_end() {
            self.tick_accumulator = 0.0;
        }

        // Passed tests move on automatically after a short pause; failed ones wait.
        if let Some(end_time) = self.end_reached_at
            && !self.paused
            && self.clip().failure.is_none()
            && now - end_time > END_PAUSE_SECONDS
            && self.clip_index + 1 < self.clips.len()
        {
            self.next_clip();
        }
    }

    fn draw(&mut self) {
        self.renderer
            .draw(&mut self.sim.game, &self.camera, true, false);

        let title = self.clip().title.clone();
        let (status, status_color) = self.status_text();
        let clip_info = format!("test {}/{}", self.clip_index + 1, self.clips.len());
        let tick_info = format!(
            "tick {}/{}   speed {}x{}",
            self.tick,
            self.clip().recording.inputs.len(),
            self.speed,
            if self.paused { "   PAUSED" } else { "" }
        );
        self.egui_mq
            .run(&mut *self.renderer.ctx, |_mq_ctx, egui_ctx| {
                egui::Window::new("Watch")
                    .default_pos(egui::pos2(10.0, 10.0))
                    .show(egui_ctx, |ui| {
                        ui.label(egui::RichText::new(title.as_str()).strong());
                        ui.label(clip_info.as_str());
                        ui.label(tick_info.as_str());
                        ui.label(egui::RichText::new(status.as_str()).color(status_color));
                        ui.separator();
                        ui.small("Space pause   . step   +/- speed   R restart");
                        ui.small("N/→ next   P/← previous   Esc quit");
                    });
            });
        self.egui_mq.draw(&mut *self.renderer.ctx);
        self.renderer.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.camera.on_resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, repeat: bool) {
        match keycode {
            KeyCode::Space if !repeat => self.paused = !self.paused,
            KeyCode::Period => {
                self.paused = true;
                self.step();
            }
            KeyCode::Equal | KeyCode::KpAdd => self.speed = (self.speed * 2.0).min(MAX_SPEED),
            KeyCode::Minus | KeyCode::KpSubtract => self.speed = (self.speed / 2.0).max(MIN_SPEED),
            KeyCode::R => self.load_clip(self.clip_index),
            KeyCode::N | KeyCode::Right => self.next_clip(),
            KeyCode::P | KeyCode::Left => self.previous_clip(),
            KeyCode::Escape => window::order_quit(),
            _ => {}
        }
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        self.egui_mq.key_up_event(keycode, keymods);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);
    }

    fn mouse_button_down_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_down_event(mb, x, y);
    }

    fn mouse_button_up_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        self.egui_mq.mouse_button_up_event(mb, x, y);
    }
}
