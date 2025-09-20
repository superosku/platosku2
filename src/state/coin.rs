use crate::physics::integrate_kinematic;
use super::game_map::GameMap;
use super::common::BoundingBox;

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


