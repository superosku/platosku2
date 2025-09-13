use miniquad::*;
mod state;
use crate::state::{GameMap, GameState, InputState, Player};
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
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 1, 1, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        ];

        let mut overlay_grid = vec![vec![0u8; 16]; 8];
        // simple decorations in overlay
        overlay_grid[2][3] = 2;
        overlay_grid[2][4] = 2;
        overlay_grid[2][5] = 2;
        overlay_grid[4][8] = 3;

        let map = GameMap {
            tile_size: 32.0,
            base: base_grid,
            overlay: overlay_grid,
        };

        // Start player near the top-left open area
        let player = Player {
            x: 32.0 * 2.0,
            y: 32.0 * 2.0,
            size: 24.0,
            speed: 3.0,
            vy: 0.0,
            on_ground: false,
        };

        let state = GameState {
            screen_w: width as f32,
            screen_h: height as f32,
            player,
            map,
            input: InputState::default(),
        };

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
