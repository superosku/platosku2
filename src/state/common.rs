#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Dir {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct Health {
    pub current: u32,
	pub max: u32,
}

impl Health {
    pub fn ratio(&self) -> f32 {
		self.current as f32 / self.max as f32
	}
}

#[derive(Clone, Copy, Debug)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    pub fn new(x: f32, y: f32) -> Pos {
        Pos { x, y }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub vy: f32,
}

impl BoundingBox {
    pub fn overlaps(&self, other: &BoundingBox) -> bool {
        !(self.x + self.w <= other.x
            || other.x + other.w <= self.x
            || self.y + self.h <= other.y
            || other.y + other.h <= self.y)
    }

    pub fn point_inside(&self, pos: &Pos) -> bool {
        pos.x >= self.x && pos.x <= self.x + self.w && pos.y >= self.y && pos.y <= self.y + self.h
    }

    pub fn center(&self) -> Pos {
        Pos {
            x: self.x + self.w * 0.5,
            y: self.y + self.h * 0.5,
        }
    }
}
