use super::common::{ BoundingBox, Health };
use super::game_map::MapLike;
use crate::physics::integrate_kinematic;
use crate::render::TextureIndexes;
use crate::state::Dir;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::enemies::SlimeState::Idle;
use rand::prelude::*;

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;

    fn update(&mut self, map: &dyn MapLike);
    fn got_stomped(&mut self);
    fn can_be_stomped(&self) -> bool;
    fn got_hit(&mut self);
    fn can_be_hit(&self) -> bool;
    fn should_remove(&self) -> bool;
    fn contanct_damage(&self) -> u32;

    fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb().overlaps(bb)
    }

    fn get_health(&self) -> Health;

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
            SlimeAnimationState::Jumping => AnimationConfigResult::new_no_loop(2, 5, 40),
        }
    }
}

enum SlimeState {
    Idle { frames_remaining: u32 },
    Jumping { frames_remaining: u32 },
    Immune { frames_remaining: u32 },
}

pub struct Slime {
    pub bb: BoundingBox,
    dir: Dir,
    animation_handler: AnimationHandler<SlimeAnimationState>,
    state: SlimeState,
    health: Health,
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
            health: Health { current: 2, max: 2 }
        }
    }
}

impl Enemy for Slime {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &dyn MapLike) {
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
                    self.dir = *[Dir::Left, Dir::Right].choose(&mut rand::rng()).unwrap();
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
            SlimeState::Immune { frames_remaining } => {
				self.animation_handler.set_state(SlimeAnimationState::Idle);
				if frames_remaining == 0 {
					self.state = SlimeState::Idle {
						frames_remaining: idling_frames,
					}
				} else {
					self.state = SlimeState::Immune {
						frames_remaining: frames_remaining - 1,
					}
                }
            }
        }

        self.animation_handler.increment_frame();
    }

    fn got_stomped(&mut self) {
        self.state = SlimeState::Immune { frames_remaining: 10 };
        self.health.current -= 1;
    }

    fn can_be_stomped(&self) -> bool {
        !matches!(self.state, SlimeState::Immune { .. })
    }

    fn got_hit(&mut self) {
        if !matches!(self.state, SlimeState::Immune { .. }) {
            self.state = SlimeState::Immune { frames_remaining: 10 };
            self.health.current -= 1;
        }
    }

    fn can_be_hit(&self) -> bool {
        !matches!(self.state, SlimeState::Immune { .. })
    }

    fn should_remove(&self) -> bool {
        self.health.current <= 0
    }

    fn contanct_damage(&self) -> u32 {
        if matches!(self.state, SlimeState::Jumping { .. }) {
            2
        } else {
            1
        }
    }

    fn get_health(&self) -> Health {
		self.health
	}

    fn get_texture_index(&self) -> TextureIndexes {
        TextureIndexes::Slime
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
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
    Falling,
}

impl AnimationConfig for BatAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            BatAnimationState::Flying => AnimationConfigResult::new(0, 3, 8),
            BatAnimationState::Standing => AnimationConfigResult::new(4, 5, 80),
            BatAnimationState::Falling => AnimationConfigResult::new(6, 6, 80),
        }
    }
}

enum BatState {
    Flying { dir_rad: f32 },
    Standing,
    Falling { frames_remaining: i32 },
}

pub struct Bat {
    bb: BoundingBox,
    state: BatState,
    animation_handler: AnimationHandler<BatAnimationState>,
    health: Health,
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
            state: BatState::Flying {
                dir_rad: rng.random_range(0.0..std::f32::consts::PI * 2.0),
            },
            animation_handler: AnimationHandler::new(BatAnimationState::Standing),
            health: Health { current: 3, max: 3 },
        }
    }
}

impl Enemy for Bat {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &dyn MapLike) {
        let mut new_state: Option<BatState> = None;

        match &mut self.state {
            BatState::Flying { dir_rad } => {
                self.bb.vx = dir_rad.cos() * 0.01;
                self.bb.vy = dir_rad.sin() * 0.01;

                let res = integrate_kinematic(map, &self.bb, false);

                if !res.on_left && !res.on_right && !res.on_top && !res.on_bottom {
                    self.bb = res.new_bb;
                }

                let mut new_dir_rad = *dir_rad;
                if res.on_left | res.on_right {
                    new_dir_rad = (new_dir_rad.sin()).atan2(-new_dir_rad.cos());
                }
                if res.on_bottom | res.on_top {
                    new_dir_rad = (-new_dir_rad.sin()).atan2(new_dir_rad.cos());
                }

                if res.on_bottom {
                    self.state = BatState::Standing;
                } else {
                    *dir_rad = new_dir_rad;
                    // self.state = BatState::Flying {
                    //     dir_rad: new_dir_rad,
                    // };
                }

                self.animation_handler.set_state(BatAnimationState::Flying);
            }
            BatState::Standing => {
                let mut rng = rand::rng();

                if rng.random_range(0..300) == 0 {
                    let dir_rad =
                        rng.random_range(std::f32::consts::PI * 1.25..std::f32::consts::PI * 1.75);
                    self.state = BatState::Flying { dir_rad }
                }

                self.animation_handler
                    .set_state(BatAnimationState::Standing);
            }
            BatState::Falling { frames_remaining } => {
                self.animation_handler.set_state(BatAnimationState::Falling);
                let orig_vy = self.bb.vy;
                let res = integrate_kinematic(map, &self.bb, true);
                self.bb = res.new_bb;

                if res.on_bottom {
                    self.bb.vy = -orig_vy * 0.8;

                    if *frames_remaining <= 0 {
                        new_state = Some(BatState::Standing);
                    }
                }

                *frames_remaining -= 1;
            }
        }

        if let Some(state) = new_state {
            self.state = state;
        }

        self.animation_handler.increment_frame();
    }

    fn got_stomped(&mut self) {
        match self.state {
            BatState::Falling { .. } => {}
            _ => {
                self.state = BatState::Falling {
                    frames_remaining: 120,
                };
                self.health.current -= 1;
            }
        }
    }

    fn can_be_stomped(&self) -> bool {
        !matches!(self.state, BatState::Falling { .. })
    }

    fn got_hit(&mut self) {
        self.got_stomped();
    }

    fn can_be_hit(&self) -> bool {
        self.can_be_stomped()
    }

    fn should_remove(&self) -> bool {
        self.health.current <= 0
    }

    fn contanct_damage(&self) -> u32 {
        if self.can_be_hit() { 1 } else { 0 }
    }

    fn get_health(&self) -> Health {
		self.health
	}

    fn get_texture_index(&self) -> TextureIndexes {
        TextureIndexes::Bat
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }

    fn goes_right(&self) -> bool {
        true
    }
}
