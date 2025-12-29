use miniquad::*;
use state::OverlayTile;

mod camera;
mod physics;
mod state;
use crate::state::{Bat, Coin, Enemy, GameMap, GameState, InputState, Player};
mod render;
use crate::render::Renderer;
use crate::state::enemies::Slime;
use egui_miniquad as egui_mq;

struct Stage {
    egui_mq: egui_mq::EguiMq,
    state: GameState,
    renderer: Renderer,
    last_time: f64,
    last_time_ups: f64,
    updates: u32,
    frames: u32,
    accumulator: f64,
    time_spent_drawing: f64,
    time_spent_updating: f64,
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        // Simple unit quad at origin (0..1, 0..1)
        let mut renderer = Renderer::new();

        let map = GameMap::new_random();

        // Start player near the top-left open area
        let player = Player::new(2.0, 2.0);

        let mut state = GameState {
            screen_w: width as f32,
            screen_h: height as f32,
            player,
            map: Box::new(map),
            input: InputState::default(),
            coins: vec![
                Coin::new(4.0, 1.0),
                Coin::new(6.0, 1.5),
                Coin::new(10.0, 1.0),
            ],
            enemies: vec![
                Box::new(Bat::new(8.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(12.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(4.0, 2.5)) as Box<dyn Enemy>,
                Box::new(Slime::new(5.0, 5.5)) as Box<dyn Enemy>,
                Box::new(Slime::new(9.0, 4.0)) as Box<dyn Enemy>,
                Box::new(Slime::new(10.0, 4.0)) as Box<dyn Enemy>,
            ],
            camera: camera::Camera::new(0.0, 0.0, 2.0),
        };

        // Initialize camera to player center
        let pcx = state.player.bb.x + state.player.bb.w * 0.5;
        let pcy = state.player.bb.y + state.player.bb.h * 0.5;
        state.camera.follow(pcx, pcy);

        Stage {
            egui_mq: egui_mq::EguiMq::new(&mut *renderer.ctx),
            state,
            renderer,
            last_time: date::now(),
            updates: 0,
            frames: 0,
            accumulator: 0.0,
            last_time_ups: date::now(),
            time_spent_drawing: 0.0,
            time_spent_updating: 0.0,
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        let update_start = date::now();
        let mut frame_time = update_start - self.last_time;
        self.last_time = update_start;

        if frame_time > 1.0 / 10.0 {
            frame_time = 1.0 / 10.0;
        }

        self.accumulator += frame_time;

        let dt = 1.0 / 60.0;

        while self.accumulator >= dt {
            self.state.update(); // HERE is the actual game call
            self.state.input.jump = false;
            self.updates += 1;
            self.accumulator -= dt;
        }

        let elapsed = update_start - self.last_time_ups;
        let update_total = date::now() - update_start;
        self.time_spent_updating += update_total;

        if elapsed >= 1.0 {
            let fps = self.frames as f64 / elapsed;
            let ups = self.updates as f64 / elapsed;
            let ratio_of_time_updating = self.time_spent_updating / elapsed;
            let ratio_of_time_drawing = self.time_spent_drawing / elapsed;
            self.time_spent_updating = 0.0;
            self.time_spent_drawing = 0.0;
            println!(
                "FPS: {:.2}, UPS: {:.2}, updat: {:.2} draw: {:.2}",
                fps, ups, ratio_of_time_updating, ratio_of_time_drawing
            );
            self.frames = 0;
            self.updates = 0;
            self.last_time_ups = update_start;
        }
    }

    fn draw(&mut self) {
        // Game
        let draw_start = date::now();

        self.renderer.draw(&self.state);
        self.frames += 1;
        let draw_total = date::now() - draw_start;
        self.time_spent_drawing += draw_total;

        // GUI
        self.egui_mq
            .run(&mut *self.renderer.ctx, |_mq_ctx, egui_ctx| {
                egui::Window::new("egui ❤ miniquad").show(egui_ctx, |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                    ui.checkbox(&mut true, "Show egui demo windows");
                });
            });

        self.egui_mq.draw(&mut *self.renderer.ctx);

        self.renderer.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.state.on_resize(width, height);
        self.renderer.resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::Left => self.state.input.left = true,
            KeyCode::Right => self.state.input.right = true,
            KeyCode::Up => self.state.input.up = true,
            KeyCode::X => self.state.input.swing = true,
            KeyCode::Z => self.state.input.jump = true,
            KeyCode::Down => self.state.input.down = true,
            _ => {}
        }
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        match keycode {
            KeyCode::Left => self.state.input.left = false,
            KeyCode::Right => self.state.input.right = false,
            KeyCode::Up => self.state.input.up = false,
            KeyCode::X => self.state.input.swing = false,
            KeyCode::Z => self.state.input.jump = false,
            KeyCode::Down => self.state.input.down = false,
            _ => {}
        }
        self.egui_mq.key_up_event(keycode, keymods);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.state.camera.zoom_scroll(dy);
        self.egui_mq.mouse_wheel_event(dx, dy);
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

    fn char_event(&mut self, character: char, _keymods: KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }
}

fn main() {
    miniquad::start(
        conf::Conf {
            window_title: String::from("Miniquad Dual-Grid Tilemap"),
            high_dpi: false,
            window_width: 800,
            window_height: 600,
            ..Default::default()
        },
        || {
            let (w, h) = window::screen_size();
            Box::new(Stage::new(w as i32, h as i32))
        },
    );
}
