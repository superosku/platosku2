use super::common::BoundingBox;
use super::game_map::GameMap;
use crate::physics::integrate_kinematic;
use crate::render::TextureIndexes;
use crate::state::Dir;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::enemies::SlimeState::Idle;
use rand::prelude::*;

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;

    fn update(&mut self, map: &GameMap);

    fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb().overlaps(bb)
    }

    fn get_texture_index(&self) -> TextureIndexes;
    fn get_atlas_index(&self) -> u32;
    fn goes_right(&self) -> bool;
}

// Slime bounces around
#[derive(PartialEq)]
enum SlimeAnimationState {
    Idle,
    Jumping,
}

impl AnimationConfig for SlimeAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            SlimeAnimationState::Idle => AnimationConfigResult::new(0, 1, 40),
            SlimeAnimationState::Jumping => AnimationConfigResult::new(2, 8, 40),
        }
    }
}

enum SlimeState {
    Idle { frames_remaining: u32 },
    Jumping { frames_remaining: u32 },
}

pub struct Slime {
    pub bb: BoundingBox,
    dir: Dir,
    animation_handler: AnimationHandler<SlimeAnimationState>,
    state: SlimeState,
}

impl Slime {
    pub fn new(x: f32, y: f32) -> Self {
        Slime {
            bb: BoundingBox {
                x,
                y,
                w: 10.0 / 16.0,
                h: 10.0 / 16.0,
                vx: 0.02,
                vy: 0.0,
            },
            dir: Dir::Right,
            animation_handler: AnimationHandler::new(SlimeAnimationState::Idle),
            state: Idle {
                frames_remaining: 100,
            },
        }
    }
}

impl Enemy for Slime {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &GameMap) {
        let result = integrate_kinematic(map, &self.bb, true);
        self.bb = result.new_bb;

        let jump_total_frames = 8 * 40;
        let jump_before_jump = 4 * 30;
        let idling_frames = 60 * 5;

        match self.state {
            SlimeState::Idle { frames_remaining } => {
                self.animation_handler.set_state(SlimeAnimationState::Idle);
                if frames_remaining == 0 {
                    self.state = SlimeState::Jumping {
                        frames_remaining: jump_total_frames,
                    };
                    let mut rng = rand::rng();
                    self.dir = *vec![Dir::Left, Dir::Right]
                        .choose(&mut rand::rng())
                        .unwrap();
                } else {
                    self.state = SlimeState::Idle {
                        frames_remaining: frames_remaining - 1,
                    }
                }
            }
            SlimeState::Jumping { frames_remaining } => {
                self.animation_handler
                    .set_state(SlimeAnimationState::Jumping);
                if frames_remaining == jump_total_frames - jump_before_jump {
                    self.bb.vy = -0.2;
                }
                if frames_remaining <= jump_total_frames - jump_before_jump {
                    self.bb.vx = 0.06
                        * match self.dir {
                            Dir::Right => 1.0,
                            Dir::Left => -1.0,
                        };
                }

                if frames_remaining == 0
                    || (result.on_bottom
                        && frames_remaining < jump_total_frames - jump_before_jump - 1)
                {
                    self.state = SlimeState::Idle {
                        frames_remaining: idling_frames,
                    }
                } else {
                    self.state = SlimeState::Jumping {
                        frames_remaining: frames_remaining - 1,
                    }
                }
            }
        }

        self.animation_handler.increment_frame();

        // // Try moving horizontally with current velocity; if blocked, flip direction
        // let desired_vx = if self.bb.vx.abs() > 0.0 {
        //     self.bb.vx
        // } else {
        //     0.01
        // };
        //
        // let mut probe = self.bb;
        // probe.vx = desired_vx;
        //
        // let result = integrate_kinematic(map, &probe, true);
        //
        // // If horizontal movement was blocked, new_bb.x stays same; flip vx
        // if (result.new_bb.x - self.bb.x).abs() < 0.0001 {
        //     self.bb.vx = -desired_vx.signum() * 0.01;
        // } else {
        //     self.bb = result.new_bb;
        //     self.bb.vx = desired_vx;
        // }
        // // Gravity handled by integrate_kinematic via vy
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }

    fn get_texture_index(&self) -> TextureIndexes {
        TextureIndexes::Slime
    }

    fn goes_right(&self) -> bool {
        match self.dir {
            Dir::Right => true,
            Dir::Left => false,
        }
    }
}

// Bat flies around
#[derive(PartialEq)]
enum BatAnimationState {
    Flying,
    Standing,
}

impl AnimationConfig for BatAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            BatAnimationState::Flying => AnimationConfigResult::new(0, 3, 8),
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

    fn update(&mut self, map: &GameMap) {
        self.bb.vx = self.dir_rad.cos() * 0.01;
        self.bb.vy = self.dir_rad.sin() * 0.01;

        if self.is_grounded {
            let mut rng = rand::rng();

            if rng.random_range(0..300) == 0 {
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

    fn get_texture_index(&self) -> TextureIndexes {
        TextureIndexes::Bat
    }

    fn goes_right(&self) -> bool {
        true
    }
}
