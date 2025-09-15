use miniquad::*;
mod state;
mod physics;
mod camera;
use crate::state::{GameMap, GameState, InputState, Player, Coin};
mod render;
use crate::render::Renderer;

struct Stage {
    state: GameState,
    renderer: Renderer,
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        // Simple unit quad at origin (0..1, 0..1)
        let renderer = Renderer::new();

        // Small demo tilemaps (dual-grid): base terrain and overlay
        let base_grid = vec![
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            vec![1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1],
            vec![1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1],
            vec![1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        ];

        let mut overlay_grid = vec![vec![0u8; 16]; 8];
        // simple decorations in overlay
        overlay_grid[2][3] = 2;
        overlay_grid[2][4] = 2;
        overlay_grid[2][5] = 2;
        overlay_grid[4][8] = 3;

        let map = GameMap {
            base: base_grid,
            overlay: overlay_grid,
        };

        // Start player near the top-left open area
        let player = Player {
            x: 2.0,
            y: 2.0,
            size: 0.8,
            speed: 0.1,
            vy: 0.0,
            on_ground: false,
        };

        let mut state = GameState {
            screen_w: width as f32,
            screen_h: height as f32,
            player,
            map,
            input: InputState::default(),
            coins: vec![
                Coin { x: 32.0 * 4.0, y: 32.0 * 1.0, size: 14.0, vy: 0.0 },
                Coin { x: 32.0 * 6.0, y: 32.0 * 1.5, size: 14.0, vy: 0.0 },
                Coin { x: 32.0 * 10.0, y: 32.0 * 1.0, size: 14.0, vy: 0.0 },
            ],
            camera: camera::Camera::new(0.0, 0.0, 1.0),
        };

        // Initialize camera to player center
        let pcx = state.player.x + state.player.size * 0.5;
        let pcy = state.player.y + state.player.size * 0.5;
        state.camera.follow(pcx, pcy);

        Stage { state, renderer }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        self.state.update();
    }

    fn draw(&mut self) {
        self.renderer.draw(&self.state);
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.state.on_resize(width, height);
        self.renderer.resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::A | KeyCode::Left => self.state.input.left = true,
            KeyCode::D | KeyCode::Right => self.state.input.right = true,
            KeyCode::W | KeyCode::Up => self.state.input.up = true,
            KeyCode::S | KeyCode::Down => self.state.input.down = true,
            _ => {}
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        match keycode {
            KeyCode::A | KeyCode::Left => self.state.input.left = false,
            KeyCode::D | KeyCode::Right => self.state.input.right = false,
            KeyCode::W | KeyCode::Up => self.state.input.up = false,
            KeyCode::S | KeyCode::Down => self.state.input.down = false,
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
