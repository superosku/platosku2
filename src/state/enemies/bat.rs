use crate::physics::integrate_kinematic;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Health, random_unit_vector};
use crate::state::draw::{DrawList, Sprite};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType, EnemyUpdateResult};
use crate::state::map_like::GameMap;
use crate::state::update_ctx::{GameRng, UpdateCtx};
use rand::Rng;

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

const BAT_SPEED: f32 = 0.01;

enum BatState {
    /// Flying in direction `(dx, dy)`, which is a unit vector.
    Flying {
        dx: f32,
        dy: f32,
    },
    Standing,
    Falling {
        frames_remaining: i32,
    },
}

pub struct Bat {
    bb: BoundingBox,
    health: Health,
    state: BatState,
    animation_handler: AnimationHandler<BatAnimationState>,
}

impl Bat {
    pub fn new(x: f32, y: f32, rng: &mut GameRng) -> Self {
        let (dx, dy) = random_unit_vector(rng, |_, _| true);
        Bat {
            bb: BoundingBox {
                x,
                y,
                w: 14.0 / 16.0,
                h: 8.0 / 16.0,
                vx: 0.0,
                vy: 0.0,
            },
            health: Health::new(3),
            state: BatState::Flying { dx, dy },
            animation_handler: AnimationHandler::new(BatAnimationState::Standing),
        }
    }
}

impl Enemy for Bat {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &GameMap, ctx: &mut UpdateCtx) -> Vec<EnemyUpdateResult> {
        let mut new_state: Option<BatState> = None;

        match &mut self.state {
            BatState::Flying { dx, dy } => {
                self.bb.vx = *dx * BAT_SPEED;
                self.bb.vy = *dy * BAT_SPEED;

                let res = integrate_kinematic(map, &self.bb, false);

                if !res.on_left && !res.on_right && !res.on_top && !res.on_bottom {
                    self.bb = res.new_bb;
                }

                if res.on_bottom {
                    self.state = BatState::Standing;
                } else {
                    // Bounce off walls and the ceiling
                    if res.on_left | res.on_right {
                        *dx = -*dx;
                    }
                    if res.on_top {
                        *dy = -*dy;
                    }
                }

                self.animation_handler.set_state(BatAnimationState::Flying);
            }
            BatState::Standing => {
                if ctx.rng.random_range(0..300) == 0 {
                    // Take off upwards, at most 45 degrees from straight up
                    let (dx, dy) = random_unit_vector(ctx.rng, |x, y| -y >= x.abs());
                    self.state = BatState::Flying { dx, dy }
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

        vec![]
    }

    fn should_remove(&self) -> bool {
        self.health.current == 0
    }

    fn get_health(&self) -> Health {
        self.health
    }

    fn maybe_got_hit(&mut self, _hit_type: EnemyHitType) -> EnemyHitResult {
        if matches!(self.state, BatState::Falling { .. }) {
            EnemyHitResult::DidNotHit
        } else {
            self.state = BatState::Falling {
                frames_remaining: 120,
            };
            self.health.decrease();

            EnemyHitResult::GotHit
        }
    }

    fn maybe_damage_player(&self) -> Option<u32> {
        if matches!(self.state, BatState::Falling { .. }) {
            None
        } else {
            Some(1)
        }
    }

    fn draw(&self, out: &mut DrawList) {
        out.sprite(
            Sprite::around("bat", self.animation_handler.get_atlas_index(), self.bb())
                .flipped(true),
        );
    }
}
