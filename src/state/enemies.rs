use super::common::BoundingBox;
use super::game_map::GameMap;
use crate::physics::integrate_kinematic;

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;
    fn bb_mut(&mut self) -> &mut BoundingBox;

    fn update(&mut self, map: &GameMap);

    fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb().overlaps(bb)
    }
}

// Simple walker that patrols horizontally and flips when hitting walls
pub struct Walker {
    pub bb: BoundingBox,
    pub speed: f32,
}

impl Walker {
    pub fn new(x: f32, y: f32) -> Self {
        Walker {
            bb: BoundingBox {
                x,
                y,
                w: 0.8,
                h: 0.8,
                vx: 0.02,
                vy: 0.0,
            },
            speed: 0.02,
        }
    }
}

impl Enemy for Walker {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }
    fn bb_mut(&mut self) -> &mut BoundingBox {
        &mut self.bb
    }

    fn update(&mut self, map: &GameMap) {
        // Try moving horizontally with current velocity; if blocked, flip direction
        let desired_vx = if self.bb.vx.abs() > 0.0 {
            self.bb.vx
        } else {
            self.speed
        };
        let mut probe = self.bb;
        probe.vx = desired_vx;
        let (new_bb, _on_ground) = integrate_kinematic(map, &probe);

        // If horizontal movement was blocked, new_bb.x stays same; flip vx
        if (new_bb.x - self.bb.x).abs() < 0.0001 {
            self.bb.vx = -desired_vx.signum() * self.speed;
        } else {
            self.bb = new_bb;
            self.bb.vx = desired_vx;
        }
        // Gravity handled by integrate_kinematic via vy
    }
}

// Floater oscillates horizontally in air using sine-like timer and very small gravity influence
pub struct Floater {
    pub bb: BoundingBox,
    pub base_x: f32,
    pub t: f32,
}

impl Floater {
    pub fn new(x: f32, y: f32) -> Self {
        Floater {
            bb: BoundingBox {
                x,
                y,
                w: 0.7,
                h: 0.7,
                vx: 0.0,
                vy: 0.0,
            },
            base_x: x,
            t: 0.0,
        }
    }
}

impl Enemy for Floater {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }
    fn bb_mut(&mut self) -> &mut BoundingBox {
        &mut self.bb
    }

    fn update(&mut self, _map: &GameMap) {
        // Lightweight oscillation: move x back and forth; ignore collisions for simplicity
        self.t += 0.05;
        let amplitude = 1.2;
        let speed = 0.03;
        // Triangle-like wave using abs
        let phase = ((self.t * speed) % 2.0) - 1.0; // -1..1
        let tri = (phase).abs();
        self.bb.x = self.base_x + (tri * 2.0 - 1.0) * amplitude * 0.5;
        // Gentle bobbing
        self.bb.y += (0.0025) * ((self.t * 0.25).sin() as f32);
    }
}
