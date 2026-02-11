use crate::physics::integrate_kinematic;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Dir, Health};
use crate::state::enemies::Enemy;
use crate::state::game_map::MapLike;
use rand::prelude::IndexedRandom;

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

pub enum SlimeState {
    Idle { frames_remaining: u32 },
    Jumping { frames_remaining: u32 },
}

pub struct Slime {
    pub bb: BoundingBox,
    health: Health,
    immunity_frames: u32,
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
            health: Health { current: 2, max: 2 },
            immunity_frames: 0,
            dir: Dir::Right,
            animation_handler: AnimationHandler::new(SlimeAnimationState::Idle),
            state: SlimeState::Idle {
                frames_remaining: 100,
            },
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
        self.immunity_frames = self.immunity_frames.saturating_sub(1);

        let jump_total_frames = 8 * 40;
        let jump_before_jump = 4 * 30;
        let idling_frames = 60 * 5;

        match self.state {
            SlimeState::Idle { frames_remaining } => {
                self.animation_handler.set_state(SlimeAnimationState::Idle);
                self.bb.vx = 0.0;
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
        }

        self.animation_handler.increment_frame();
    }

    fn got_stomped(&mut self) {
        self.immunity_frames = 10;
        self.state = SlimeState::Idle {
            frames_remaining: 50,
        };
        self.health.current -= 1;
    }

    fn can_be_stomped(&self) -> bool {
        self.immunity_frames == 0
    }

    fn got_hit(&mut self) {
        if self.immunity_frames == 0 {
            self.immunity_frames = 10;
            self.state = SlimeState::Idle {
                frames_remaining: 50,
            };
            self.health.current -= 1;
        }
    }

    fn can_be_hit(&self) -> bool {
        self.immunity_frames == 0
    }

    fn should_remove(&self) -> bool {
        self.health.current == 0
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

    fn get_texture_index(&self) -> &str {
        "slime"
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
