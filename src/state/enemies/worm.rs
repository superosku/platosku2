use crate::physics::integrate_kinematic;
use crate::render::TILE_SIZE;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Dir, Health};
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType, EnemyUpdateResult};
use crate::state::game_map::MapLike;

// Worm moves back and fort
#[derive(PartialEq)]
enum WormAnimationState {
    Moving,
}

impl AnimationConfig for WormAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            WormAnimationState::Moving => AnimationConfigResult::new(0, 5, 10),
        }
    }
}

pub struct Worm {
    bb: BoundingBox,
    animation_handler: AnimationHandler<WormAnimationState>,
    dir: Dir,
    is_dead: bool,
}

impl Worm {
    pub fn new(x: f32, y: f32) -> Self {
        Worm {
            bb: BoundingBox {
                x,
                y,
                w: 14.0 / 16.0,
                h: 6.0 / 16.0,
                vx: 0.0,
                vy: 0.0,
            },
            animation_handler: AnimationHandler::new(WormAnimationState::Moving),
            dir: Dir::Left,
            is_dead: false,
        }
    }
}

impl Enemy for Worm {
    fn bb(&self) -> &BoundingBox {
        &self.bb
    }

    fn update(&mut self, map: &dyn MapLike) -> Vec<EnemyUpdateResult> {
        match self.dir {
            Dir::Left => {
                self.bb.vx = -0.01;
            }
            Dir::Right => {
                self.bb.vx = 0.01;
            }
        }

        let res = integrate_kinematic(map, &self.bb, true);
        self.bb = res.new_bb;

        if res.on_right {
            self.dir = Dir::Left
        };
        if res.on_left {
            self.dir = Dir::Right
        };

        self.animation_handler.set_state(WormAnimationState::Moving);

        self.animation_handler.increment_frame();

        vec![]
    }

    fn should_remove(&self) -> bool {
        self.is_dead
    }

    fn get_health(&self) -> Health {
        Health { current: 1, max: 1 }
    }

    fn maybe_got_hit(&mut self, _hit_type: EnemyHitType) -> EnemyHitResult {
        self.is_dead = true;
        EnemyHitResult::GotHit
    }

    fn maybe_damage_player(&self) -> Option<u32> {
        Some(1)
    }

    fn draw(&self, renderer: &mut crate::render::Renderer) {
        let bb = self.bb();
        renderer.draw_from_texture_atlas(
            "worm",
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
