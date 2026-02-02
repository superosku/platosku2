use super::common::{BoundingBox, Dir, Health, Pos};
use super::game_map::MapLike;
use super::game_state::InputState;
use crate::camera::Camera;
use crate::physics::{check_and_snap_hang, check_and_snap_platforms, integrate_kinematic};
use crate::render::Renderer;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::item::{Item, ItemType};

pub enum PlayerState {
    Normal,
    // TODO: Swinging should be an optional part of Player struct so swinging can happen while on ladder etc.
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
    on_ground: bool,
    safe_edge_frames: u32,
    state: PlayerState,
    max_speed: f32,
    max_jump_frames: u32,
    pub dir: Dir,
    animation_handler: AnimationHandler<PlayerAnimationState>,
    item: Option<Item>,
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
    Crouching,
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
            PlayerAnimationState::Crouching => AnimationConfigResult::new(22, 22, 10),
        }
    }
}

pub enum PlayerUpdateResult {
    AddItem { item: Item },
    PickUpItem,
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
            // item: None,
            item: Some(Item::new(0.0, 0.0, ItemType::Box)),
        }
    }

    pub fn set_item(&mut self, item: Item) {
        self.item = Some(item);
    }

    pub fn draw(&self, renderer: &mut Renderer, camera: &Camera) {
        let px = self.bb.x;
        let py = self.bb.y;
        let pw = self.bb.w;
        let ph = self.bb.h;

        let alpha = if !self.can_be_hit() {
            ((self.immunity_frames / 10) % 2) as f32
        } else {
            1.0
        };

        renderer.draw_from_texture_atlas(
            "character",
            self.animation_handler.get_atlas_index(),
            match self.dir {
                Dir::Left => true,
                Dir::Right => false,
            },
            px - 1.0 / crate::render::TILE_SIZE,
            py - 1.0 / crate::render::TILE_SIZE,
            pw + 2.0 / crate::render::TILE_SIZE,
            ph + 2.0 / crate::render::TILE_SIZE,
            alpha,
        );

        if let Some(item) = &self.item {
            let crouch_offset_y =
                if self.animation_handler.current_state() == &PlayerAnimationState::Crouching {
                    2.0 / 16.0
                } else {
                    0.0
                };
            item.draw_fake_xy(renderer, self.bb.x, self.bb.y + crouch_offset_y);
        }

        // Draw the sword as the last step
        if let Some(swing_info) = self.get_swing_info() {
            renderer.draw_rect_rotated(
                camera,
                swing_info.pivot.x - 0.05,
                swing_info.pivot.y - 0.15,
                0.1,
                swing_info.length + 0.15,
                swing_info.pivot.x,
                swing_info.pivot.y,
                swing_info.angle_rad,
                [0.5, 0.5, 0.5, 1.0],
            );

            renderer.draw_rect(
                camera,
                swing_info.end.x - 0.05,
                swing_info.end.y - 0.05,
                0.1,
                0.1,
                [1.0, 0.5, 0.5, 1.0],
            )
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

    pub fn _handle_normal(
        &mut self,
        input: &InputState,
        map: &dyn MapLike,
    ) -> Vec<PlayerUpdateResult> {
        let mut update_results = vec![];

        let pressing_left = input.left && !input.right;
        let pressing_right = input.right && !input.left;

        if pressing_left {
            self.bb.vx = (-self.max_speed).max(self.bb.vx - 0.02);

            match self.state {
                PlayerState::Swinging { .. } => {}
                _ => {
                    self.dir = Dir::Left;
                }
            }
        } else if pressing_right {
            self.bb.vx = self.max_speed.min(self.bb.vx + 0.02);

            match self.state {
                PlayerState::Swinging { .. } => {}
                _ => {
                    self.dir = Dir::Right;
                }
            }
        } else {
            // Ground and air friction
            if !(-0.002..=0.002).contains(&self.bb.vx) {
                self.bb.vx = self.bb.vx - self.bb.vx * 0.3;
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

        // 1. Want to jump 2. Not trying to go down ledge 3. Can jump
        if input.jump_pressed && !input.down && (self.on_ground || self.safe_edge_frames > 0) {
            self.safe_edge_frames = 0;
            self.bb.vy = -0.125;
        } else if input.jump_held && !input.down && self.max_jump_frames > 0 {
            self.max_jump_frames -= 1;
            self.bb.vy = -0.125;
        } else {
            self.max_jump_frames = 0; // if no input.jump reset to 0
        }

        // Want to swing / throw / leave item / pick up item
        if input.swing_pressed
            && let PlayerState::Normal = self.state
        {
            let some_item: Option<Item> = self.item.take();
            if let Some(mut item) = some_item {
                if input.down {
                    // Drop item
                    item.set_xyv(self.bb.x, self.bb.y, 0.0, 0.0);
                } else {
                    // Throw item
                    item.set_xyv(
                        self.bb.x,
                        self.bb.y,
                        self.bb.vx
                            + match self.dir {
                                Dir::Left => -0.1,
                                Dir::Right => 0.1,
                            },
                        self.bb.vy + if input.up { -0.2 } else { -0.1 },
                    );
                }
                update_results.push(PlayerUpdateResult::AddItem { item });
            } else if input.down {
                // Pick up
                update_results.push(PlayerUpdateResult::PickUpItem);
            } else {
                // Swing
                self.state = PlayerState::Swinging {
                    total_frames: 20,
                    frames_left: 20,
                };
            }
        }

        let res = integrate_kinematic(map, &self.bb, true);
        let mut new_bb = res.new_bb;
        let mut on_ground = res.on_bottom;

        let could_ladder = map.is_ladder_at(
            (new_bb.x + new_bb.w * 0.5).floor() as i32,
            (new_bb.y + new_bb.h * 0.5).floor() as i32,
        );

        if could_ladder && (input.up || input.down) && !(input.down && res.on_bottom) {
            self.state = PlayerState::OnLadder;
            let middle_tx = (new_bb.x + new_bb.w * 0.5).floor() as i32;
            self.bb.x = (middle_tx as f32 + 0.5) - self.bb.w * 0.5;
            self.bb.vx = 0.0;

            return update_results;
        }

        if !(input.down && input.jump_pressed) {
            on_ground |= check_and_snap_platforms(&self.bb, &mut new_bb, map);
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
                return update_results;
            }
        }

        self.bb = new_bb;
        self.on_ground = on_ground;

        if self.on_ground {
            if pressing_right || pressing_left {
                self.animation_handler
                    .set_state(PlayerAnimationState::Walking);
            } else if input.down {
                self.animation_handler
                    .set_state(PlayerAnimationState::Crouching);
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

        update_results
    }

    pub fn update(&mut self, input: &InputState, map: &dyn MapLike) -> Vec<PlayerUpdateResult> {
        let mut update_results = vec![];

        let mut increment_frame = true;
        self.immunity_frames = self.immunity_frames.saturating_sub(1);

        match &self.state {
            PlayerState::Hanging { pos, .. } => {
                self.bb.x = pos.x;
                self.bb.y = pos.y;
                self.bb.vy = 0.0;
                self.bb.vx = 0.0;
                self.on_ground = false;

                if input.jump_pressed {
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

                update_results.append(&mut self._handle_normal(input, map));
            }
            PlayerState::Dead => {
                let res = integrate_kinematic(map, &self.bb, true);
                self.bb = res.new_bb;
                self.on_ground = res.on_bottom;

                self.animation_handler
                    .set_state(PlayerAnimationState::Dying)
            }

            PlayerState::Normal => {
                update_results.append(&mut self._handle_normal(input, map));
            }
            PlayerState::OnLadder => {
                self.animation_handler
                    .set_state(PlayerAnimationState::Laddering);
                if input.jump_pressed && !input.up {
                    self.state = PlayerState::Normal;
                    self.bb.vy = -0.125;
                    self.bb.y += 0.05;
                    return update_results;
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
                        return update_results;
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
                        self.bb.y = feet_y.floor() - self.bb.h - 0.0001;
                        return update_results;
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

        update_results
    }
}
