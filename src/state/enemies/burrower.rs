use crate::render::TILE_SIZE;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Health};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType};
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
            BurrowerAnimationState::Burbing => AnimationConfigResult::new_no_loop(4, 6, 8),
            BurrowerAnimationState::BurrowingUp => {
                AnimationConfigResult::new_reverse_no_loop(7, 12, 6)
            }
            BurrowerAnimationState::BurrowingDown => AnimationConfigResult::new_no_loop(7, 12, 6),
            BurrowerAnimationState::Digging => AnimationConfigResult::new(13, 16, 6),
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
                    self.frames_remaining = 180;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Wiggling);
                }
                BurrowerAnimationState::Wiggling => {
                    self.frames_remaining = 30;
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
                    self.frames_remaining = 90;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Digging);
                }
                BurrowerAnimationState::Digging => {
                    self.frames_remaining = 30;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::BurrowingUp);
                }
            }
        }

        self.frames_remaining -= 1;

        self.animation_handler.increment_frame();
    }

    fn should_remove(&self) -> bool {
        self.is_dead
    }

    fn get_health(&self) -> Health {
        Health { current: 1, max: 1 }
    }

    fn maybe_got_hit(&mut self, _hit_type: EnemyHitType) -> EnemyHitResult {
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden | BurrowerAnimationState::Digging => {
                EnemyHitResult::DidNotHit
            }
            _ => {
                self.is_dead = true;
                EnemyHitResult::GotHit
            }
        }
    }

    fn maybe_damage_player(&self) -> Option<u32> {
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden | BurrowerAnimationState::Digging => None,
            _ => Some(1),
        }
    }

    fn draw(&self, renderer: &mut crate::render::Renderer) {
        if matches!(
            self.animation_handler.current_state(),
            BurrowerAnimationState::Hidden
        ) {
            return;
        }

        let bb = self.bb();
        renderer.draw_from_texture_atlas(
            "burrower",
            self.animation_handler.get_atlas_index(),
            true,
            bb.x - 1.0 / TILE_SIZE,
            bb.y - 1.0 / TILE_SIZE,
            bb.w + 2.0 / TILE_SIZE,
            bb.h + 2.0 / TILE_SIZE,
            1.0,
        );
    }
}
