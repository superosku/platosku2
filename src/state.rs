use crate::physics::{integrate_kinematic, PhysicsParams};
use crate::camera::Camera;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Dir {
    Left,
    Right
}

#[derive(Clone, Copy)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

pub enum PlayerState {
    Normal,
    Hanging { dir: Dir, pos: Pos },
}

pub struct Player {
    pub pos: Pos,
    pub size: f32,
    pub speed: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub state: PlayerState,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Player {
            pos: Pos { x, y },
            size: 0.8,
            speed: 0.06,
            vy: 0.0,
            on_ground: false,
            state: PlayerState::Normal,
        }
    }

    pub fn update(&mut self, input: &InputState, map: &GameMap) {
        // If currently hanging, freeze position and handle jump/drop
        match &self.state {
            PlayerState::Hanging { pos, .. } => {
                self.pos = *pos;
                self.vy = 0.0;
                self.on_ground = false;

                // Jump to release, or drop if also holding down
                if input.jump {
                    self.state = PlayerState::Normal;
                    if input.down {
                        // Drop: start falling
                        self.vy = 0.0;
                    } else {
                        // Jump upward from hang
                        self.vy = -0.17;
                    }
                }
            },
            PlayerState::Normal => {
                // Horizontal movement (A/D or Left/Right)
                let mut dx = 0.0f32;
                if input.left { dx -= self.speed; }
                if input.right { dx += self.speed; }

                // Jump (W/Up) - only when grounded
                if input.jump && self.on_ground {
                    self.vy = -0.19;
                }

                // Try to start hanging while falling and pressing into a wall near a ledge
                if self.vy > 0.0 {
                    let pressing_left = input.left && !input.right;
                    let pressing_right = input.right && !input.left;
                    if pressing_left || pressing_right {
                        let dir: Dir = if pressing_right { Dir::Right } else { Dir::Left };
                        if let Some(hang_pos) = self.check_and_snap_hang(map, dir) {
                            self.state = PlayerState::Hanging { dir, pos: hang_pos };
                            self.vy = 0.0;
                            self.on_ground = false;
                            return; // skip physics while entering hang
                        }
                    }
                }

                let (nx, ny, nvy, on_ground) = integrate_kinematic(
                    map,
                    self.pos.x,
                    self.pos.y,
                    self.size,
                    self.vy,
                    dx,
                    &PhysicsParams::default(),
                );
                self.pos.x = nx;
                self.pos.y = ny;
                self.vy = nvy;
                self.on_ground = on_ground;
            }
        }
    }

    fn check_and_snap_hang(&self, map: &GameMap, dir: Dir) -> Option<Pos> {
        // Must be near the top edge of a tile row
        let top_frac = self.pos.y - self.pos.y.floor();
        if top_frac > 0.85 { return None; }

        let tile_y = self.pos.y.floor() as i32; // tile row at player's top
        let tile_x = self.pos.x.floor() as i32; // tile column at player's left

        // // Determine tile row at the player's top
        // let ty = self.y.floor() as i32;

        // Horizontal adjacency check and side tile to test
        let eps_side = 0.10;

        let (tile_x_check) = if dir == Dir::Right {
            let dist_to_right = (self.pos.x + self.size) - (tile_x as f32 + 1.0);
            if dist_to_right > eps_side { return None; }
            tile_x + 1
        } else {
            let dist_to_left = self.pos.x - (tile_x as f32);
            if dist_to_left > eps_side { return None; }
            tile_x - 1
        };

        // if !touching_side { return None; }

        // Ledge condition: side tile is blocked at ty, but open above (ty-1)
        let (base_here, _) = map.get_at(tile_x_check, tile_y);
        let (base_above, _) = map.get_at(tile_x_check, tile_y - 1);
        if base_here == 0 || base_above != 0 { return None; }

        // Snap Y to sit slightly below the tile top
        // let snapped_y = ty as f32 + 0.02;
        Some(
            Pos{
                x: if dir == Dir::Left {self.pos.x.floor()} else {self.pos.x.floor() + (1.0 - self.size)} ,
                y: self.pos.y.floor()
            })
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
    pub jump: bool,
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
        self.coins.retain(|c| !c.overlaps(self.player.pos.x, self.player.pos.y, self.player.size));

        // Camera follows player center
        let pcx = self.player.pos.x + self.player.size * 0.5;
        let pcy = self.player.pos.y + self.player.size * 0.5;
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


