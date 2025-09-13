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
            // Negative velocity goes upwards (y grows downwards on screen)
            self.vy = -10.0;
            self.on_ground = false;
        }

        // Apply gravity
        let gravity = 0.5f32;
        let terminal_velocity = 12.0f32;
        self.vy = (self.vy + gravity).min(terminal_velocity);

        // Move horizontally first, resolve collisions on X
        let attempted_x = self.x + dx;
        if !collides_with_map(map, attempted_x, self.y, self.size) {
            self.x = attempted_x;
        }

        // Then move vertically with velocity, resolve collisions on Y
        let attempted_y = self.y + self.vy;
        if !collides_with_map(map, self.x, attempted_y, self.size) {
            self.y = attempted_y;
            self.on_ground = false;
        } else {
            // Collision while moving vertically: place the player flush against the blocking tiles
            let epsilon = 0.001f32;
            let tile_size = map.tile_size;
            let left_tx = (self.x / tile_size).floor() as i32;
            let right_tx = ((self.x + self.size - epsilon) / tile_size).floor() as i32;

            if self.vy > 0.0 {
                // Falling: snap to the top of the first blocking tile below
                let bottom_ty_attempted = ((self.y + self.size + self.vy - epsilon) / tile_size).floor() as i32;

                let mut landed = false;
                for tx in left_tx..=right_tx {
                    let (base, _overlay) = map.get_at(tx, bottom_ty_attempted);
                    if base != 0 {
                        let tile_top = bottom_ty_attempted as f32 * tile_size;
                        self.y = tile_top - self.size;
                        landed = true;
                        break;
                    }
                }
                if !landed {
                    // Possibly hit the map bottom boundary; clamp
                    self.y = (map.height_px() - self.size).max(0.0);
                }
                self.vy = 0.0;
                self.on_ground = true;
            } else if self.vy < 0.0 {
                // Moving up: snap to the bottom of the first blocking tile above
                let top_ty_attempted = ((self.y + self.vy) / tile_size).floor() as i32;

                let mut hit_ceiling = false;
                for tx in left_tx..=right_tx {
                    let (base, _overlay) = map.get_at(tx, top_ty_attempted);
                    if base != 0 {
                        let tile_bottom = (top_ty_attempted + 1) as f32 * tile_size;
                        self.y = tile_bottom;
                        hit_ceiling = true;
                        break;
                    }
                }
                if !hit_ceiling {
                    // Possibly hit the map top boundary; clamp
                    self.y = 0.0;
                }
                self.vy = 0.0;
            }
        }

        // Keep player within map pixel bounds as a final clamp
        self.x = self.x.clamp(0.0, (map.width_px() - self.size).max(0.0));
        self.y = self.y.clamp(0.0, (map.height_px() - self.size).max(0.0));
    }
}

pub struct GameMap {
    pub tile_size: f32,
    pub base: Vec<Vec<u8>>,    // base terrain layer
    pub overlay: Vec<Vec<u8>>, // overlay/decorations layer
}

impl GameMap {
    pub fn width_tiles(&self) -> usize { self.base.first().map(|r| r.len()).unwrap_or(0) }
    pub fn height_tiles(&self) -> usize { self.base.len() }
    pub fn width_px(&self) -> f32 { self.width_tiles() as f32 * self.tile_size }
    pub fn height_px(&self) -> f32 { self.height_tiles() as f32 * self.tile_size }

    pub fn get_at(&self, tx: i32, ty: i32) -> (u8, u8) {
        if tx < 0 || ty < 0 { return (0, 0); }
        let x = tx as usize;
        let y = ty as usize;
        let base = self.base.get(y).and_then(|row| row.get(x)).copied().unwrap_or(0);
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
}

impl GameState {
    pub fn update(&mut self) {
        self.player.update(&self.input, &self.map);
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
}

fn collides_with_map(map: &GameMap, x: f32, y: f32, size: f32) -> bool {
    // Treat outside of map bounds as blocking
    if x < 0.0 || y < 0.0 { return true; }
    if x + size > map.width_px() || y + size > map.height_px() { return true; }

    let epsilon = 0.001f32;
    let left_tx = (x / map.tile_size).floor() as i32;
    let right_tx = ((x + size - epsilon) / map.tile_size).floor() as i32;
    let top_ty = (y / map.tile_size).floor() as i32;
    let bottom_ty = ((y + size - epsilon) / map.tile_size).floor() as i32;

    for ty in top_ty..=bottom_ty {
        for tx in left_tx..=right_tx {
            let (base, _overlay) = map.get_at(tx, ty);
            if base != 0 { return true; }
        }
    }
    false
}


