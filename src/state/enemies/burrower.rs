use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Health};
use crate::state::enemies::Enemy;
use crate::state::game_map::MapLike;

#[derive(PartialEq)]
enum BurrowerAnimationState {
    Wiggling,
    Burbing,
    BurrowingUp,
    BurrowingDown,
    Digging,
    Hidden,
}

impl AnimationConfig for BurrowerAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            BurrowerAnimationState::Wiggling => AnimationConfigResult::new(0, 3, 10),
            BurrowerAnimationState::Burbing => AnimationConfigResult::new_no_loop(4, 6, 10),
            BurrowerAnimationState::BurrowingUp => {
                AnimationConfigResult::new_reverse_no_loop(7, 12, 10)
            }
            BurrowerAnimationState::BurrowingDown => AnimationConfigResult::new_no_loop(7, 12, 10),
            BurrowerAnimationState::Digging => AnimationConfigResult::new(13, 16, 10),
            BurrowerAnimationState::Hidden => AnimationConfigResult::new(0, 1, 10), // Should not be drawn
        }
    }
}

pub struct Burrower {
    bb: BoundingBox,
    animation_handler: AnimationHandler<BurrowerAnimationState>,
    frames_remaining: u32,
    is_dead: bool,
}

impl Burrower {
    pub fn new(x: f32, y: f32) -> Self {
        Burrower {
            bb: BoundingBox {
                x,
                y,
                w: 8.0 / 16.0,
                h: 10.0 / 16.0,
                vx: 0.0,
                vy: 0.0,
            },
            frames_remaining: 0,
            animation_handler: AnimationHandler::new(BurrowerAnimationState::Digging),
            is_dead: false,
        }
    }
}

impl Enemy for Burrower {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, _map: &dyn MapLike) {
        if self.frames_remaining == 0 {
            match self.animation_handler.current_state() {
                BurrowerAnimationState::BurrowingUp => {
                    self.frames_remaining = 60;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Burbing);
                }
                BurrowerAnimationState::Burbing => {
                    // TODO: Throw the projectile here
                    self.frames_remaining = 240;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Wiggling);
                }
                BurrowerAnimationState::Wiggling => {
                    self.frames_remaining = 60;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::BurrowingDown);
                }
                BurrowerAnimationState::BurrowingDown => {
                    self.frames_remaining = 120;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Hidden);
                }
                BurrowerAnimationState::Hidden => {
                    // TODO: Change location here
                    self.frames_remaining = 120;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Digging);
                }
                BurrowerAnimationState::Digging => {
                    self.frames_remaining = 60;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::BurrowingUp);
                }
            }
        }

        self.frames_remaining -= 1;

        self.animation_handler.increment_frame();
    }

    fn got_stomped(&mut self) {
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden => {}
            _ => {
                self.is_dead = true;
            }
        }
    }

    fn can_be_stomped(&self) -> bool {
        !matches!(
            self.animation_handler.current_state(),
            BurrowerAnimationState::Hidden
        )
    }

    fn got_hit(&mut self) {
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden => {}
            _ => {
                self.is_dead = true;
            }
        }
    }

    fn can_be_hit(&self) -> bool {
        !matches!(
            self.animation_handler.current_state(),
            BurrowerAnimationState::Hidden
        )
    }

    fn should_remove(&self) -> bool {
        self.is_dead
    }

    fn contanct_damage(&self) -> u32 {
        0
    }

    fn get_health(&self) -> Health {
        Health { current: 1, max: 1 }
    }

    fn get_texture_index(&self) -> &str {
        "burrower"
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }

    fn goes_right(&self) -> bool {
        true
    }
}
