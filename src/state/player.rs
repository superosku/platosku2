use super::common::{BoundingBox, Dir, Health, Pos};
use super::game_map::MapLike;
use super::game_state::InputState;
use crate::physics::{check_and_snap_hang, integrate_kinematic};
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};

pub enum PlayerState {
    Normal,
    Swinging { total_frames: u32, frames_left: u32 },
    Hanging { pos: Pos },
    OnLadder,
    Dead,
}

pub struct SwingState {
    pub pivot: Pos,
    pub end: Pos,
    pub angle_rad: f32,
    pub length: f32,
}

pub struct Player {
    pub bb: BoundingBox,
    pub health: Health,
    pub immunity_frames: u32,
    pub on_ground: bool,
    pub safe_edge_frames: u32,
    pub state: PlayerState,
    pub max_speed: f32,
    pub max_jump_frames: u32,
    pub dir: Dir,
    animation_handler: AnimationHandler<PlayerAnimationState>,
}

#[derive(PartialEq)]
enum PlayerAnimationState {
    Walking,
    Standing,
    JumpingSide,
    JumpingDown,
    Laddering,
    Hanging,
    Dying,
}

impl AnimationConfig for PlayerAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            PlayerAnimationState::Walking => AnimationConfigResult::new(0, 7, 8),
            PlayerAnimationState::Standing => AnimationConfigResult::new(8, 9, 80),
            PlayerAnimationState::JumpingSide => AnimationConfigResult::new(10, 10, 15),
            PlayerAnimationState::JumpingDown => AnimationConfigResult::new(11, 11, 15),
            PlayerAnimationState::Laddering => AnimationConfigResult::new(12, 15, 10),
            PlayerAnimationState::Hanging => AnimationConfigResult::new(16, 16, 5),
            PlayerAnimationState::Dying => AnimationConfigResult::new_no_loop(17, 20, 10),
        }
    }
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Player {
            bb: BoundingBox {
                x,
                y,
                w: 9.0 / 16.0,
                h: 13.0 / 16.0,
                vx: 0.0,
                vy: 0.0,
            },
            health: Health { current: 4, max: 4 },
            immunity_frames: 0,
            on_ground: false,
            safe_edge_frames: 0,
            state: PlayerState::Normal,
            max_speed: 0.06,
            max_jump_frames: 0,
            dir: Dir::Right,
            animation_handler: AnimationHandler::new(PlayerAnimationState::Standing),
        }
    }

    pub fn can_be_hit(&self) -> bool {
        self.immunity_frames == 0
    }

    pub fn got_hit(&mut self, damage: u32) {
        self.health.current = 0.max(self.health.current as i32 - damage as i32) as u32;

        // If no health set to dead
        if self.health.current == 0 {
            self.state = PlayerState::Dead;
        }
        // If we took damage set the immunity frmaes
        if damage > 0 {
            self.immunity_frames = 60;
        }
    }

    pub fn maybe_stomp(&mut self, other_bb: &BoundingBox) -> bool {
        if self.bb.vy > 0.0 && (self.bb.y + self.bb.h) - other_bb.y < self.bb.vy * 2.0 {
            self.bb.vy = -0.12;
            return true;
        }
        false
    }

    pub fn get_swing_info(&self) -> Option<SwingState> {
        match self.state {
            PlayerState::Swinging {
                total_frames,
                frames_left,
            } => {
                // Swing moves half a circle in total
                let total_rads = std::f32::consts::PI;

                let start_angle = match self.dir {
                    Dir::Left => std::f32::consts::PI * 0.5 - 0.3,
                    Dir::Right => std::f32::consts::PI * 0.5 + 0.3,
                };

                let fraction = match self.dir {
                    Dir::Left => frames_left as f32 / total_frames as f32,
                    Dir::Right => (total_frames - frames_left) as f32 / total_frames as f32,
                };

                let angle_rad = start_angle + fraction * total_rads;

                let dir_move = match self.dir {
                    Dir::Left => -0.1,
                    Dir::Right => 0.1,
                };

                let pivot_x = self.bb.x + self.bb.w / 2.0 + dir_move;
                let pivot_y = self.bb.y + self.bb.h / 2.0 + 0.05;

                let length = 0.8;

                // Drawing uses angles weirdly. Adding 1/4 circle to the angle here as hacky fix
                let end_x = pivot_x + (angle_rad + std::f32::consts::PI / 2.0).cos() * length;
                let end_y = pivot_y + (angle_rad + std::f32::consts::PI / 2.0).sin() * length;

                Some(SwingState {
                    angle_rad,
                    pivot: Pos::new(pivot_x, pivot_y),
                    end: Pos::new(end_x, end_y),
                    length,
                })
            }
            _ => None,
        }
    }

    pub fn _handle_normal(&mut self, input: &InputState, map: &dyn MapLike) {
        let pressing_left = input.left && !input.right;
        let pressing_right = input.right && !input.left;

        if pressing_left {
            if self.bb.vx > -self.max_speed {
                self.bb.vx -= 0.01;
            }
            match self.state {
                PlayerState::Swinging { .. } => {}
                _ => {
                    self.dir = Dir::Left;
                }
            }
        } else if pressing_right {
            if self.bb.vx < self.max_speed {
                self.bb.vx += 0.01;
            }
            match self.state {
                PlayerState::Swinging { .. } => {}
                _ => {
                    self.dir = Dir::Right;
                }
            }
        } else {  // Ground and air friction
            if !(-0.002..=0.002).contains(&self.bb.vx) {
                self.bb.vx = self.bb.vx - self.bb.vx * 0.2;
            } else {
                self.bb.vx = 0.0;
            }
        }
        
        if self.on_ground {
            self.safe_edge_frames = 4;
            self.max_jump_frames = 10;
        } else if self.safe_edge_frames > 0 {
            self.safe_edge_frames -= 1;
        }

        if input.jump && (self.on_ground || self.safe_edge_frames > 0) {
            self.safe_edge_frames = 0;
            self.bb.vy = -0.125;
        } else if input.jump && self.max_jump_frames > 0 {
            self.max_jump_frames -= 1;
            self.bb.vy = -0.125;
        } else {
            self.max_jump_frames = 0; // if no input.jump reset to 0
        }

        if input.swing {
            if let PlayerState::Normal = self.state {
                self.state = PlayerState::Swinging {
                    total_frames: 20,
                    frames_left: 20,
                };
            }
        }

        let res = integrate_kinematic(map, &self.bb, true);
        let new_bb = res.new_bb;
        let on_ground = res.on_bottom;

        let could_ladder = map.is_ladder_at(
            (new_bb.x + new_bb.w * 0.5).floor() as i32,
            (new_bb.y + new_bb.h * 0.5).floor() as i32,
        );
        if could_ladder && (input.up || input.down) && !(input.down && self.on_ground) {
            self.state = PlayerState::OnLadder;
            let middle_tx = (new_bb.x + new_bb.w * 0.5).floor() as i32;
            self.bb.x = (middle_tx as f32 + 0.5) - self.bb.w * 0.5;
            self.bb.vx = 0.0;

            return;
        }

        if self.bb.vy > 0.0 && (pressing_left || pressing_right) {
            let dir: Dir = if pressing_right {
                Dir::Right
            } else {
                Dir::Left
            };
            if let Some(hang_pos) = check_and_snap_hang(&self.bb, &new_bb, map, dir) {
                self.state = PlayerState::Hanging { pos: hang_pos };
                self.dir = dir;
                self.bb.vy = 0.0;
                self.on_ground = false;
                return;
            }
        }

        self.bb = new_bb;
        self.on_ground = on_ground;

        if self.on_ground {
            if pressing_right || pressing_left {
                self.animation_handler
                    .set_state(PlayerAnimationState::Walking);
            } else {
                self.animation_handler
                    .set_state(PlayerAnimationState::Standing);
            }
        } else if pressing_right || pressing_left {
            self.animation_handler
                .set_state(PlayerAnimationState::JumpingSide);
        } else {
            self.animation_handler
                .set_state(PlayerAnimationState::JumpingDown);
        }
    }

    pub fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }

    pub fn update(&mut self, input: &InputState, map: &dyn MapLike) {
        let mut increment_frame = true;
        self.immunity_frames = self.immunity_frames.saturating_sub(1);

        match &self.state {
            PlayerState::Hanging { pos, .. } => {
                self.bb.x = pos.x;
                self.bb.y = pos.y;
                self.bb.vy = 0.0;
                self.bb.vx = 0.0;
                self.on_ground = false;

                if input.jump {
                    self.state = PlayerState::Normal;
                    if input.down {
                        self.bb.vy = 0.0;
                    } else {
                        self.bb.vy = -0.12;
                    }
                }
                self.animation_handler
                    .set_state(PlayerAnimationState::Hanging);
            }
            PlayerState::Swinging {
                total_frames,
                frames_left,
            } => {
                if *frames_left > 0 {
                    self.state = PlayerState::Swinging {
                        total_frames: *total_frames,
                        frames_left: frames_left - 1,
                    };
                } else {
                    self.state = PlayerState::Normal;
                }

                self._handle_normal(input, map);
            }
            PlayerState::Dead => {
                let res = integrate_kinematic(map, &self.bb, true);
                self.bb = res.new_bb;
                self.on_ground = res.on_bottom;

                self.animation_handler
                    .set_state(PlayerAnimationState::Dying)
            }

            PlayerState::Normal => {
                self._handle_normal(input, map);
            }
            PlayerState::OnLadder => {
                self.animation_handler
                    .set_state(PlayerAnimationState::Laddering);
                if input.jump && !input.up {
                    self.state = PlayerState::Normal;
                    self.bb.vy = -0.125;
                    self.bb.y = self.bb.y + 0.05;
                    return;
                }

                let middle_tx = (self.bb.x + self.bb.w * 0.5).floor() as i32;
                let head_ty = (self.bb.y).floor() as i32;
                let ladder_at_head = map.is_ladder_at(middle_tx, head_ty);
                let ladder_at_below = map.is_ladder_at(middle_tx, head_ty + 1);

                if input.up && !input.down {
                    if ladder_at_head {
                        self.bb.vy = -self.max_speed;
                    } else {
                        self.bb.vy = 0.0;
                        return;
                    }
                } else if input.down && !input.up {
                    self.bb.vy = self.max_speed;
                    let feet_y = self.bb.y + self.bb.h;
                    if map.is_solid_at_tile(middle_tx, feet_y.floor() as i32)
                        || (!ladder_at_head && !ladder_at_below)
                    {
                        self.state = PlayerState::Normal;
                        self.on_ground = true;
                        self.bb.vy = 0.0;
                        return;
                    }
                } else {
                    self.bb.vy = 0.0;
                    increment_frame = false;
                }
                let new_y = self.bb.y + self.bb.vy;

                self.bb.y = new_y;
            }
        }

        if increment_frame {
            self.animation_handler.increment_frame();
        }
    }
}
