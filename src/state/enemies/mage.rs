use crate::physics::integrate_kinematic;
use crate::render::TILE_SIZE;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Dir, Health};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType, EnemyUpdateResult};
use crate::state::game_map::GameMap;
use crate::state::item::{Item, ItemType};
use rand::prelude::IndexedRandom;

// Mage wonders and shoots bolts
#[derive(PartialEq)]
enum MageAnimationState {
	Idle,
    Walking,
	Casting,
    Reading,
    FlippingPage,
}

impl AnimationConfig for MageAnimationState {
	fn get_config(&self) -> AnimationConfigResult {
		match self {
			MageAnimationState::Idle => AnimationConfigResult::new(0, 1, 60),
			MageAnimationState::Walking => AnimationConfigResult::new(2, 5, 10),
			MageAnimationState::Casting => AnimationConfigResult::new_no_loop(6, 8, 10),
            MageAnimationState::Reading => AnimationConfigResult::new_no_loop(9, 9, 20),
            MageAnimationState::FlippingPage => AnimationConfigResult::new_no_loop(10, 14, 5),
		}
	}
}

pub struct Mage {
	bb: BoundingBox,
    animation_handler: AnimationHandler<MageAnimationState>,
    frames_remaining: u32,
	health: Health,
    immunity_frames: u32,
	dir: Dir,
    alert: bool,
    reading_cycle: u32,
}

impl Mage {
    pub fn new(x: f32, y: f32) -> Self {
		Mage {
			bb: BoundingBox {
				x,
				y,
				w: 12.0 / 16.0,
				h: 14.0 / 16.0,
				vx: 0.0,
				vy: 0.0,
			},
            frames_remaining: 50,
			animation_handler: AnimationHandler::new(MageAnimationState::Idle),
            health: Health::new(3),
            immunity_frames: 0,
            dir: Dir::Right,
            alert: true,
            reading_cycle: 0,
		}
	}
}

impl Enemy for Mage {
    fn bb(&self) -> &BoundingBox {
		&self.bb
	}

	fn update(&mut self, map: &GameMap, player_bb: &BoundingBox) -> Vec<EnemyUpdateResult> {
        let res = integrate_kinematic(map, &self.bb, true);
        self.bb = res.new_bb;
        let mut update_results = Vec::new();
        self.immunity_frames = self.immunity_frames.saturating_sub(1);
        
        if self.bb.in_range(player_bb, 4.0) 
            && self.alert
            && *self.animation_handler.current_state() != MageAnimationState::Casting
        {
            self.frames_remaining = 30;
            self.animation_handler
                .set_state(MageAnimationState::Casting);
            self.bb.vx = 0.0;
            self.dir = if player_bb.x > self.bb.x {
                Dir::Left
            } else {
                Dir::Right
            };
        }
        
        let vx = match self.animation_handler.current_state() {
            MageAnimationState::Walking => match self.dir {
                Dir::Left => 0.01,
                Dir::Right => -0.01,
            },
            _ => 0.0,
        };

        self.bb.vx = vx;

        if self.frames_remaining == 0 {
            match self.animation_handler.current_state() {
                MageAnimationState::Idle => {
					self.frames_remaining = 80 + rand::random::<u32>() % 80;
					self.dir = *[Dir::Left, Dir::Right].choose(&mut rand::rng()).unwrap();
                    self.alert = true;
                    self.reading_cycle = 3;
                    self.animation_handler
                        .set_state(MageAnimationState::Walking);
				}
                MageAnimationState::Walking => {
                    let start_reading = rand::random::<f32>() < 0.25;

                    if start_reading {
                        self.frames_remaining = 120;
                        self.alert = false;
                        self.animation_handler
                            .set_state(MageAnimationState::Reading);
                    } else {
                        self.frames_remaining = 80 + rand::random::<u32>() % 80;
                        self.alert = true;
                        self.animation_handler
							.set_state(MageAnimationState::Idle);
                    }
                }
				MageAnimationState::Reading => {
                     if self.reading_cycle == 0 {
                        self.frames_remaining = 80 + rand::random::<u32>() % 80;
						self.alert = true;
						self.animation_handler
							.set_state(MageAnimationState::Idle);
                     } else {
                        self.reading_cycle = self.reading_cycle.saturating_sub(1);
					    self.frames_remaining = 25;
                        self.alert = false;
                        self.animation_handler
                            .set_state(MageAnimationState::FlippingPage);
                     }
                }
                MageAnimationState::FlippingPage => {
                    self.frames_remaining = 120;
                    self.alert = false;
					self.animation_handler
						.set_state(MageAnimationState::Reading);
				}
                MageAnimationState::Casting => {
                    self.alert = true;
					self.frames_remaining = 50;
                    let projectile = Item::new(
                        self.bb.x + match self.dir {
                            Dir::Right => 0.0,
                            Dir::Left => self.bb.w,
                        },
                        self.bb.y, 
                        ItemType::MageProjectile,
                    );

                    update_results
                        .push(EnemyUpdateResult::SpawnItemCastedTowardsPlayer { item: projectile });

                    self.animation_handler
                        .set_state(MageAnimationState::Idle);
                }
            }
        }
        
        self.frames_remaining -= 1;

        self.animation_handler.increment_frame();

        update_results
    }

    fn should_remove(&self) -> bool {
        self.health.current == 0
    }

    fn get_health(&self) -> Health {
        self.health
    }

    fn maybe_got_hit(&mut self, _hit_type: EnemyHitType) -> EnemyHitResult {
        if self.immunity_frames == 0 {
            self.health.decrease();

            self.immunity_frames = 30;
            self.animation_handler
                    .set_state(MageAnimationState::Idle);
            EnemyHitResult::GotHit
        } else {
            EnemyHitResult::DidNotHit
        }
    }

    fn maybe_damage_player(&self) -> Option<u32> {
        Some(1)
    }

    fn draw(&self, renderer: &mut crate::render::Renderer) {
        let bb = self.bb();
        renderer.draw_from_texture_atlas(
            "mage",
            self.animation_handler.get_atlas_index(),
            self.dir.goes_right(),
            bb.x - 1.0 / TILE_SIZE,
            bb.y - 1.0 / TILE_SIZE,
            bb.w + 2.0 / TILE_SIZE,
            bb.h + 2.0 / TILE_SIZE,
            1.0,
        );
    }
}
