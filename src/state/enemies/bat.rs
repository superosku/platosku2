use crate::physics::integrate_kinematic;
use crate::render::TILE_SIZE;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Health};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType};
use crate::state::game_map::MapLike;
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

enum BatState {
    Flying { dir_rad: f32 },
    Standing,
    Falling { frames_remaining: i32 },
}

pub struct Bat {
    bb: BoundingBox,
    health: Health,
    state: BatState,
    animation_handler: AnimationHandler<BatAnimationState>,
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
            health: Health { current: 3, max: 3 },
            state: BatState::Flying {
                dir_rad: rng.random_range(0.0..std::f32::consts::PI * 2.0),
            },
            animation_handler: AnimationHandler::new(BatAnimationState::Standing),
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
            self.health.current -= 1; // TODO: Can this overflow?

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

    fn draw(&self, renderer: &mut crate::render::Renderer) {
        let bb = self.bb();
        renderer.draw_from_texture_atlas(
            "bat",
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
