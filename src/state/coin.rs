use super::common::BoundingBox;
use super::game_map::MapLike;
use crate::physics::integrate_kinematic;

pub struct Coin {
    pub bb: BoundingBox,
}

impl Coin {
    #[allow(dead_code)]
    pub fn new(x: f32, y: f32) -> Self {
        Coin {
            bb: BoundingBox {
                x,
                y,
                w: 0.5,
                h: 0.5,
                vx: 0.0,
                vy: 0.0,
            },
        }
    }

    pub fn update(&mut self, map: &dyn MapLike) {
        let res = integrate_kinematic(map, &self.bb, true);
        self.bb = res.new_bb;
    }

    pub fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb.overlaps(bb)
    }
}
