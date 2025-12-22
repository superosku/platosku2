use super::common::BoundingBox;
use super::game_map::GameMap;
use crate::physics::integrate_kinematic;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use rand::prelude::*;

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;
    fn bb_mut(&mut self) -> &mut BoundingBox;

    fn update(&mut self, map: &GameMap);

    fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb().overlaps(bb)
    }

    fn get_atlas_index(&self) -> u32;
}

// // Simple walker that patrols horizontally and flips when hitting walls
// pub struct Walker {
//     pub bb: BoundingBox,
//     pub speed: f32,
// }
//
// impl Walker {
//     pub fn new(x: f32, y: f32) -> Self {
//         Walker {
//             bb: BoundingBox {
//                 x,
//                 y,
//                 w: 0.8,
//                 h: 0.8,
//                 vx: 0.02,
//                 vy: 0.0,
//             },
//             speed: 0.02,
//         }
//     }
// }
//
// impl Enemy for Walker {
//     fn bb(&self) -> &BoundingBox {
//         &self.bb
//     }
//     fn bb_mut(&mut self) -> &mut BoundingBox {
//         &mut self.bb
//     }
//
//     fn update(&mut self, map: &GameMap) {
//         // Try moving horizontally with current velocity; if blocked, flip direction
//         let desired_vx = if self.bb.vx.abs() > 0.0 {
//             self.bb.vx
//         } else {
//             self.speed
//         };
//         let mut probe = self.bb;
//         probe.vx = desired_vx;
//         let (new_bb, _on_ground) = integrate_kinematic(map, &probe);
//
//         // If horizontal movement was blocked, new_bb.x stays same; flip vx
//         if (new_bb.x - self.bb.x).abs() < 0.0001 {
//             self.bb.vx = -desired_vx.signum() * self.speed;
//         } else {
//             self.bb = new_bb;
//             self.bb.vx = desired_vx;
//         }
//         // Gravity handled by integrate_kinematic via vy
//     }
// }

// Bat flies around

#[derive(PartialEq)]
enum BatAnimationState {
    Flying,
    Standing,
}

impl AnimationConfig for BatAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            BatAnimationState::Flying => AnimationConfigResult::new(0, 3, 15),
            BatAnimationState::Standing => AnimationConfigResult::new(4, 4, 5),
        }
    }
}

pub struct Bat {
    pub bb: BoundingBox,
    // pub base_x: f32,
    // pub t: f32,
    dir_rad: f32,
    is_grounded: bool,
    pub animation_handler: AnimationHandler<BatAnimationState>,
}

impl Bat {
    pub fn new(x: f32, y: f32) -> Self {
        let mut rng = rand::rng();

        Bat {
            bb: BoundingBox {
                x,
                y,
                w: 14.0 / 16.0,
                h: 8.0 / 16.0,
                vx: 0.0,
                vy: 0.0,
            },
            is_grounded: false,
            dir_rad: rng.random_range(0.0..std::f32::consts::PI * 2.0),
            animation_handler: AnimationHandler::new(BatAnimationState::Standing),
        }
    }
}

impl Enemy for Bat {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }
    fn bb_mut(&mut self) -> &mut BoundingBox {
        &mut self.bb
    }

    fn update(&mut self, map: &GameMap) {
        self.bb.vx = self.dir_rad.cos() * 0.01;
        self.bb.vy = self.dir_rad.sin() * 0.01;

        if self.is_grounded {
            let mut rng = rand::rng();

            if rng.random_range(0..30) == 0 {
                self.is_grounded = false;
                // When leaving ground go up upwards left 45 degree or right 45 degree
                self.dir_rad =
                    rng.random_range(std::f32::consts::PI * 1.25..std::f32::consts::PI * 1.75)
            }
        } else {
            let res = integrate_kinematic(map, &self.bb, false);

            if !res.on_left && !res.on_right && !res.on_top && !res.on_bottom {
                self.bb = res.new_bb;
            }

            if res.on_left | res.on_right {
                self.dir_rad = (self.dir_rad.sin()).atan2(-self.dir_rad.cos());
            }
            if res.on_bottom | res.on_top {
                self.dir_rad = (-self.dir_rad.sin()).atan2(self.dir_rad.cos());
            }

            if res.on_bottom {
                self.is_grounded = true;
            }
        }

        if self.is_grounded {
            self.animation_handler
                .set_state(BatAnimationState::Standing);
        } else {
            self.animation_handler.set_state(BatAnimationState::Flying);
        }

        self.animation_handler.increment_frame();
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }
}
