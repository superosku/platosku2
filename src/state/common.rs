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
    pub fn overlaps_line(&self, a: &Pos, b: &Pos) -> bool {
        if self.point_inside(a) || self.point_inside(b) {
            return true;
        }

        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let mut t0 = 0.0f32;
        let mut t1 = 1.0f32;

        fn clip(p: f32, q: f32, t0: &mut f32, t1: &mut f32) -> bool {
            // p == 0 => line is parallel to this boundary
            if p.abs() < f32::EPSILON {
                // If q < 0, it's outside the boundary
                return q >= 0.0;
            }
            let r = q / p;
            if p < 0.0 {
                // entering
                if r > *t1 {
                    return false;
                }
                if r > *t0 {
                    *t0 = r;
                }
            } else {
                // leaving
                if r < *t0 {
                    return false;
                }
                if r < *t1 {
                    *t1 = r;
                }
            }
            true
        }

        let max_x = self.x + self.w;
        let min_x = self.x;
        let max_y = self.y + self.h;
        let min_y = self.y;

        // Left:   x >= min_x  =>  -dx * t <= a.x - min_x
        if !clip(-dx, a.x - min_x, &mut t0, &mut t1) {
            return false;
        }
        // Right:  x <= max_x  =>   dx * t <= max_x - a.x
        if !clip(dx, max_x - a.x, &mut t0, &mut t1) {
            return false;
        }
        // Top:    y >= min_y
        if !clip(-dy, a.y - min_y, &mut t0, &mut t1) {
            return false;
        }
        // Bottom: y <= max_y
        if !clip(dy, max_y - a.y, &mut t0, &mut t1) {
            return false;
        }

        // If we still have a valid interval, the segment hits the box.
        t0 <= t1
    }

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
