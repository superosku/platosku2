use crate::physics::integrate_kinematic;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::{BoundingBox, Dir, Health};
use crate::state::enemies::Enemy;
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

    fn update(&mut self, map: &dyn MapLike) {
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
    }

    fn got_stomped(&mut self) {
        self.is_dead = true;
    }

    fn can_be_stomped(&self) -> bool {
        true
    }

    fn got_hit(&mut self) {
        self.got_stomped();
    }

    fn can_be_hit(&self) -> bool {
        self.can_be_stomped()
    }

    fn should_remove(&self) -> bool {
        self.is_dead
    }

    fn contanct_damage(&self) -> u32 {
        1
    }

    fn get_health(&self) -> Health {
        Health { current: 1, max: 1 }
    }

    fn get_texture_index(&self) -> &str {
        "worm"
    }

    fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }

    fn goes_right(&self) -> bool {
        match self.dir {
            Dir::Right => false,
            Dir::Left => true,
        }
    }
}
