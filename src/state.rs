use crate::physics::{integrate_kinematic, check_and_snap_hang};
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

pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub vy: f32,
}

impl BoundingBox {
    pub fn iterate(&self) -> BoundingBox {
        BoundingBox {
            x: self.x + self.vx,
            y: self.y + self.vy,
            w: self.w,
            h: self.h,
            vx: self.vx,
            vy: self.vy,
        }
    }

    pub fn overlaps(&self, other: &BoundingBox) -> bool {
        !(self.x + self.w <= other.x ||
          other.x + other.w <= self.x ||
          self.y + self.h <= other.y ||
          other.y + other.h <= self.y)
    }
}

pub enum PlayerState {
    Normal,
    Hanging { dir: Dir, pos: Pos },
}

pub struct Player {
    pub bb: BoundingBox,
    pub on_ground: bool,
    pub state: PlayerState,
    pub speed: f32, // horizontal speed per frame
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Player {
            bb: BoundingBox { x, y, w: 0.6, h: 0.8, vx: 0.0, vy: 0.0 },
            on_ground: false,
            state: PlayerState::Normal,
            speed: 0.04,
        }
    }

    pub fn update(&mut self, input: &InputState, map: &GameMap) {
        // If currently hanging, freeze position and handle jump/drop
        match &self.state {
            PlayerState::Hanging { pos, .. } => {
                self.bb.x = pos.x;
                self.bb.y = pos.y;
                self.bb.vy = 0.0;
                self.on_ground = false;

                // Jump to release, or drop if also holding down
                if input.jump {
                    self.state = PlayerState::Normal;
                    if input.down {
                        // Drop: start falling
                        self.bb.vy = 0.0;
                    } else {
                        // Jump upward from hang
                        self.bb.vy = -0.17;
                    }
                }
            },
            PlayerState::Normal => {
                // Horizontal movement (A/D or Left/Right)
                // let mut dx = 0.0f32;
                if input.left { self.bb.vx = -self.speed; }
                else if input.right { self.bb.vx = self.speed; }
                else {self.bb.vx = 0.0;}

                // Jump (W/Up) - only when grounded
                if input.jump && self.on_ground {
                    self.bb.vy = -0.19;
                }

                let (new_bb, on_ground) = integrate_kinematic(
                    map,
                    &self.bb,
                );

                // Try to start hanging while falling and pressing into a wall near a ledge
                if self.bb.vy > 0.0 {
                    let pressing_left = input.left && !input.right;
                    let pressing_right = input.right && !input.left;
                    if pressing_left || pressing_right {
                        let dir: Dir = if pressing_right { Dir::Right } else { Dir::Left };
                        if let Some(hang_pos) = check_and_snap_hang(&self.bb, &new_bb, map, dir) {
                            self.state = PlayerState::Hanging { dir, pos: hang_pos };
                            self.bb.vy = 0.0;
                            self.on_ground = false;
                            return; // skip physics while entering hang
                        }
                    }
                }

                self.bb = new_bb;
                self.on_ground = on_ground;
            }
        }
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

    pub fn is_solid_at(&self, tx: i32, ty: i32) -> bool {
        let (base, _overlay) = self.get_at(tx, ty);
        base != 0
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
        self.coins.retain(|c| !c.overlaps(&self.player.bb));

        // Camera follows player center
        let pcx = self.player.bb.x + self.player.bb.w * 0.5;
        let pcy = self.player.bb.y + self.player.bb.h * 0.5;
        self.camera.follow(pcx, pcy);
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
}

pub struct Coin {
    pub bb: BoundingBox,
}

impl Coin {
    pub fn new(x: f32, y: f32) -> Self {
        Coin {
            bb: BoundingBox { x, y, w: 0.5, h: 0.5, vx: 0.0, vy: 0.0 },
        }
    }

    pub fn update(&mut self, map: &GameMap) {
        let (new_bb, _on_ground) = integrate_kinematic(
            map,
            &self.bb,
        );
        self.bb = new_bb;
    }

    pub fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb.overlaps(bb)
    }
}


