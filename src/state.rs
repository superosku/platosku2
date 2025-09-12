pub struct Player {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub speed: f32,
}

impl Player {
    pub fn update(&mut self, input: &InputState, bounds: (f32, f32)) {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        if input.left { dx -= self.speed; }
        if input.right { dx += self.speed; }
        if input.up { dy -= self.speed; }
        if input.down { dy += self.speed; }

        let (max_w, max_h) = bounds;
        self.x = (self.x + dx).clamp(0.0, (max_w - self.size).max(0.0));
        self.y = (self.y + dy).clamp(0.0, (max_h - self.size).max(0.0));
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
        self.player.update(&self.input, (self.screen_w, self.screen_h));
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
}


