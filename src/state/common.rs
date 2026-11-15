use crate::state::update_ctx::GameRng;
use rand::Rng;

/// Art pixels per tile. Sprites are drawn on a 16 px grid, and some game logic
/// uses it for pixel-sized offsets.
pub const TILE_SIZE: f32 = 16.0;

/// Returns a random unit vector `(x, y)` whose direction is uniformly distributed
/// among the directions for which `accept(x, y)` is true.
///
/// Uses rejection sampling in the unit disc, so it only needs basic arithmetic and
/// `sqrt`, which IEEE 754 specifies exactly. The result is therefore identical on
/// every platform (unlike `sin`/`cos`). Falls back to straight up if nothing is
/// accepted.
pub fn random_unit_vector(rng: &mut GameRng, accept: impl Fn(f32, f32) -> bool) -> (f32, f32) {
    for _ in 0..1000 {
        let x = rng.random_range(-1.0f32..1.0);
        let y = rng.random_range(-1.0f32..1.0);
        let len_sq = x * x + y * y;
        if !(1e-6..=1.0).contains(&len_sq) {
            continue;
        }
        let len = len_sq.sqrt();
        let (ux, uy) = (x / len, y / len);
        if accept(ux, uy) {
            return (ux, uy);
        }
    }
    (0.0, -1.0)
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Dir {
    Left,
    Right,
}

impl Dir {
    pub fn goes_right(&self) -> bool {
        matches!(self, Dir::Right)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Health { current: max, max }
    }

    pub fn decrease(&mut self) {
        self.current = self.current.saturating_sub(1);
    }

    pub fn decrease_by(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }

    pub fn is_empty(&self) -> bool {
        self.current == 0
    }

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
    pub fn get_center(&self) -> Pos {
        Pos {
            x: self.x + self.w * 0.5,
            y: self.y + self.h * 0.5,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn random_unit_vector_is_unit_length_and_respects_accept() {
        let mut rng = GameRng::seed_from_u64(7);
        let mut saw_left = false;
        let mut saw_right = false;
        for _ in 0..1000 {
            let (x, y) = random_unit_vector(&mut rng, |x, y| -y >= x.abs());
            assert!(((x * x + y * y) - 1.0).abs() < 1e-5);
            assert!(-y >= x.abs() - 1e-6, "outside the upward cone: ({x}, {y})");
            saw_left |= x < -0.5;
            saw_right |= x > 0.5;
        }
        assert!(saw_left && saw_right, "should cover the whole cone");
    }
}
