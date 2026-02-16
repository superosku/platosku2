use crate::physics::EPS;
use crate::render::TILE_SIZE;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Health};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType, EnemyUpdateResult};
use crate::state::game_map::{GameMap, MapLike};
use crate::state::item::{Item, ItemType};
use rand::Rng;

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
    health: Health,
    immunity_frames: u32,
}

const BURROWING_DOWN_FRAMES: u32 = 30;

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
            frames_remaining: 180,
            animation_handler: AnimationHandler::new(BurrowerAnimationState::Digging),
            health: Health::new(2),
            immunity_frames: 0,
        }
    }
}

fn find_random_close_floor_pos_at_room_here(
    current_pos: &BoundingBox,
    map: &GameMap,
) -> Option<BoundingBox> {
    let center = current_pos.get_center();

    let current_room = map.get_room_at(center.x, center.y)?;

    let current_x = center.x.floor() as i32;
    let current_y = center.y.floor() as i32;

    for _ in 0..50 {
        let mut rng = rand::rng();
        let x_diff = rng.random_range(-10..10);
        let y_diff = rng.random_range(-10..10);

        let new_x = current_x + x_diff;
        let new_y = current_y + y_diff;

        if let Some(new_room) = map.get_room_at(new_x as f32 + 0.5, new_y as f32 + 0.5) {
            if new_room.0 != current_room.0 {
                continue;
            }
        } else {
            continue;
        }

        if map.is_solid_at_tile(new_x, new_y)
            || map.is_solid_at_tile(new_x, new_y - 1)
            || !map.is_solid_at_tile(new_x, new_y + 1)
        {
            continue;
        }

        let mut new_bounding_box = *current_pos;
        new_bounding_box.x =
            new_x as f32 + EPS + rng.random_range(0.0..1.0 - new_bounding_box.w - 2.0 * EPS);
        new_bounding_box.y = new_y as f32 - EPS + (1.0 - new_bounding_box.h);

        return Some(new_bounding_box);
    }

    None
}

impl Enemy for Burrower {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &GameMap) -> Vec<EnemyUpdateResult> {
        let mut update_results = Vec::new();
        self.immunity_frames = self.immunity_frames.saturating_sub(1);

        if self.frames_remaining == 0 {
            match self.animation_handler.current_state() {
                BurrowerAnimationState::BurrowingUp => {
                    self.frames_remaining = 30;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Burbing);
                }
                BurrowerAnimationState::Burbing => {
                    let projectile = Item::new(
                        self.bb.x + self.bb.w / 2.0,
                        self.bb.y + self.bb.w / 2.0, // Use the w (not h) here to center at "head circle"
                        ItemType::GreenProjectile,
                    );

                    update_results
                        .push(EnemyUpdateResult::SpawnItemThrowTowardsPlayer { item: projectile });

                    self.frames_remaining = 180;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Wiggling);
                }
                BurrowerAnimationState::Wiggling => {
                    self.frames_remaining = BURROWING_DOWN_FRAMES;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::BurrowingDown);
                }
                BurrowerAnimationState::BurrowingDown => {
                    self.frames_remaining = 120;
                    self.animation_handler
                        .set_state(BurrowerAnimationState::Hidden);
                }
                BurrowerAnimationState::Hidden => {
                    // Randomize the position of the burrower
                    if let Some(new_bb) = find_random_close_floor_pos_at_room_here(&self.bb, map) {
                        self.bb = new_bb;
                    }

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

        update_results
    }

    fn should_remove(&self) -> bool {
        self.health.is_empty()
    }

    fn get_health(&self) -> Health {
        self.health
    }

    fn maybe_got_hit(&mut self, _hit_type: EnemyHitType) -> EnemyHitResult {
        if self.immunity_frames > 0 {
            return EnemyHitResult::DidNotHit;
        }
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden | BurrowerAnimationState::Digging => {
                EnemyHitResult::DidNotHit
            }
            _ => {
                self.frames_remaining = BURROWING_DOWN_FRAMES;
                self.animation_handler
                    .set_state(BurrowerAnimationState::BurrowingDown);
                self.health.decrease();
                self.immunity_frames = 90;
                EnemyHitResult::GotHit
            }
        }
    }

    fn maybe_damage_player(&self) -> Option<u32> {
        match self.animation_handler.current_state() {
            BurrowerAnimationState::Hidden
            | BurrowerAnimationState::Digging
            | BurrowerAnimationState::BurrowingDown
            | BurrowerAnimationState::BurrowingUp => None,
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

    fn should_render_health_bar(&self) -> bool {
        !matches!(
            self.animation_handler.current_state(),
            BurrowerAnimationState::Hidden | BurrowerAnimationState::Digging
        )
    }
}
