#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Dir {
    Left,
    Right,
}

#[derive(Clone, Copy)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy)]
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
}
