use miniquad::*;
use state::OverlayTile;

mod camera;
mod physics;
mod state;
use crate::state::{Bat, Coin, Enemy, GameMap, GameState, InputState, Player};
mod render;
use crate::render::Renderer;

struct Stage {
    state: GameState,
    renderer: Renderer,
    last_time: f64,
    last_time_ups: f64,
    updates: u32,
    frames: u32,
    accumulator: f64,
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        // Simple unit quad at origin (0..1, 0..1)
        let renderer = Renderer::new();

        // Small demo tilemaps (dual-grid): base terrain and overlay
        // Map this so that 1 is BaseTile::Solid, 0 is BaseTile::Empty
        let base_grid = [
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            vec![1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 2, 0, 1],
            vec![1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 2, 2, 0, 0, 0, 2, 2, 0, 1],
            vec![1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 2, 2, 0, 1],
            vec![1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        ]
        .iter()
        .map(|row| {
            row.iter()
                .map(|&v| match v {
                    0 => state::BaseTile::Empty,
                    1 => state::BaseTile::Stone,
                    2 => state::BaseTile::Wood,
                    _ => state::BaseTile::Empty,
                })
                .collect()
        })
        .collect();

        let mut overlay_grid = vec![vec![OverlayTile::None; 16]; 8];
        // simple decorations in overlay
        overlay_grid[6][9] = OverlayTile::Ladder;
        overlay_grid[5][9] = OverlayTile::Ladder;
        overlay_grid[4][9] = OverlayTile::Ladder;
        overlay_grid[3][9] = OverlayTile::Ladder;

        // overlay_grid[5][15] = OverlayTile::Ladder;
        overlay_grid[4][15] = OverlayTile::Ladder;
        overlay_grid[3][15] = OverlayTile::Ladder;

        let map = GameMap {
            base: base_grid,
            overlay: overlay_grid,
        };

        // Start player near the top-left open area
        let player = Player::new(2.0, 2.0);

        let mut state = GameState {
            screen_w: width as f32,
            screen_h: height as f32,
            player,
            map,
            input: InputState::default(),
            coins: vec![
                Coin::new(4.0, 1.0),
                Coin::new(6.0, 1.5),
                Coin::new(10.0, 1.0),
            ],
            enemies: vec![
                Box::new(Bat::new(8.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(12.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(5.0, 2.5)) as Box<dyn Enemy>,
            ],
            camera: camera::Camera::new(0.0, 0.0, 2.0),
        };

        // Initialize camera to player center
        let pcx = state.player.bb.x + state.player.bb.w * 0.5;
        let pcy = state.player.bb.y + state.player.bb.h * 0.5;
        state.camera.follow(pcx, pcy);

        Stage {
            state,
            renderer,
            last_time: date::now(),
            updates: 0,
            frames: 0,
            accumulator: 0.0,
            last_time_ups: date::now(),
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        let now = date::now();
        let mut frame_time = now - self.last_time;
        self.last_time = now;

        if frame_time > 1.0 / 10.0 {
            frame_time = 1.0 / 10.0;
        }

        self.accumulator += frame_time;

        let DT = 1.0 / 60.0;

        while self.accumulator >= DT {
            self.state.update();
            self.state.input.jump = false;
            self.updates += 1;
            self.accumulator -= DT;
        }

        let elapsed = now - self.last_time_ups;
        if elapsed >= 1.0 {
            let fps = self.frames as f64 / elapsed as f64;
            let ups = self.updates as f64 / elapsed as f64;
            println!("FPS: {:.2}, UPS: {:.2}", fps, ups);
            self.frames = 0;
            self.updates = 0;
            self.last_time_ups = now;
        }
    }

    fn draw(&mut self) {
        self.renderer.draw(&self.state);
        self.frames += 1;
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.state.on_resize(width, height);
        self.renderer.resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::Left => self.state.input.left = true,
            KeyCode::Right => self.state.input.right = true,
            KeyCode::Up => self.state.input.up = true,
            KeyCode::Z => self.state.input.swing = true,
            KeyCode::X => self.state.input.jump = true,
            KeyCode::Down => self.state.input.down = true,
            _ => {}
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        match keycode {
            KeyCode::Left => self.state.input.left = false,
            KeyCode::Right => self.state.input.right = false,
            KeyCode::Up => self.state.input.up = false,
            KeyCode::Z => self.state.input.swing = false,
            KeyCode::X => self.state.input.jump = false,
            KeyCode::Down => self.state.input.down = false,
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.state.camera.zoom_scroll(y);
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
