use crate::physics::{integrate_kinematic, PhysicsParams};
use crate::camera::Camera;

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub speed: f32,
    pub vy: f32,
    pub on_ground: bool,
}

impl Player {
    pub fn update(&mut self, input: &InputState, map: &GameMap) {
        // Horizontal movement (A/D or Left/Right)
        let mut dx = 0.0f32;
        if input.left { dx -= self.speed; }
        if input.right { dx += self.speed; }

        // Jump (W/Up) - only when grounded
        if input.up && self.on_ground {
            self.vy = -0.24;
        }

        let (nx, ny, nvy, on_ground) = integrate_kinematic(
            map,
            self.x,
            self.y,
            self.size,
            self.vy,
            dx,
            &PhysicsParams::default(),
        );
        self.x = nx;
        self.y = ny;
        self.vy = nvy;
        self.on_ground = on_ground;
    }
}

pub struct GameMap {
    pub base: Vec<Vec<u8>>,    // base terrain layer
    pub overlay: Vec<Vec<u8>>, // overlay/decorations layer
}

impl GameMap {
    pub fn width_tiles(&self) -> usize { self.base.first().map(|r| r.len()).unwrap_or(0) }
    pub fn height_tiles(&self) -> usize { self.base.len() }
    pub fn width(&self) -> f32 { self.width_tiles() as f32 }
    pub fn height(&self) -> f32 { self.height_tiles() as f32 }

    pub fn get_at(&self, tx: i32, ty: i32) -> (u8, u8) {
        // Outside the map is blocking for base layer; overlay remains empty
        if tx < 0 || ty < 0 { return (1, 0); }
        let x = tx as usize;
        let y = ty as usize;
        let base = self.base.get(y).and_then(|row| row.get(x)).copied().unwrap_or(1);
        let overlay = self.overlay.get(y).and_then(|row| row.get(x)).copied().unwrap_or(0);
        (base, overlay)
    }
}

#[derive(Default)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

pub struct GameState {
    pub screen_w: f32,
    pub screen_h: f32,
    pub player: Player,
    pub map: GameMap,
    pub input: InputState,
    pub coins: Vec<Coin>,
    pub camera: Camera,
}

impl GameState {
    pub fn update(&mut self) {
        self.player.update(&self.input, &self.map);
        // Update coins physics
        for coin in &mut self.coins {
            coin.update(&self.map);
        }
        // Collect coins on overlap with player (AABB)
        self.coins.retain(|c| !c.overlaps(self.player.x, self.player.y, self.player.size));

        // Camera follows player center
        let pcx = self.player.x + self.player.size * 0.5;
        let pcy = self.player.y + self.player.size * 0.5;
        self.camera.follow(pcx, pcy);
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
}

pub struct Coin {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub vy: f32,
}

impl Coin {
    pub fn update(&mut self, map: &GameMap) {
        let (nx, ny, nvy, _on_ground) = integrate_kinematic(
            map,
            self.x,
            self.y,
            self.size,
            self.vy,
            0.0,
            &PhysicsParams::default(),
        );
        self.x = nx;
        self.y = ny;
        self.vy = nvy;
    }

    pub fn overlaps(&self, ox: f32, oy: f32, os: f32) -> bool {
        !(self.x + self.size <= ox ||
          ox + os <= self.x ||
          self.y + self.size <= oy ||
          oy + os <= self.y)
    }
}


