use crate::physics::{integrate_kinematic, check_and_snap_hang};
use super::game_map::GameMap;
use super::game_state::InputState;
use super::common::{Dir, Pos, BoundingBox};

pub enum PlayerState {
    Normal,
    Hanging { dir: Dir, pos: Pos },
    OnLadder,
}

pub struct Player {
    pub bb: BoundingBox,
    pub on_ground: bool,
    pub state: PlayerState,
    pub speed: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Player {
            bb: BoundingBox { x, y, w: 0.6, h: 0.8, vx: 0.0, vy: 0.0 },
            on_ground: false,
            state: PlayerState::Normal,
            speed: 0.04,
        }
    }

    pub fn update(&mut self, input: &InputState, map: &GameMap) {
        match &self.state {
            PlayerState::Hanging { pos, .. } => {
                self.bb.x = pos.x;
                self.bb.y = pos.y;
                self.bb.vy = 0.0;
                self.on_ground = false;

                if input.jump {
                    self.state = PlayerState::Normal;
                    if input.down {
                        self.bb.vy = 0.0;
                    } else {
                        self.bb.vy = -0.17;
                    }
                }
            },
            PlayerState::Normal => {
                if input.left { self.bb.vx = -self.speed; }
                else if input.right { self.bb.vx = self.speed; }
                else {self.bb.vx = 0.0;}

                if input.jump && self.on_ground {
                    self.bb.vy = -0.19;
                }

                let (new_bb, on_ground) = integrate_kinematic(
                    map,
                    &self.bb,
                );

                let could_ladder = map.is_ladder_at(
                    (new_bb.x + new_bb.w * 0.5).floor() as i32,
                    (new_bb.y + new_bb.h * 0.5).floor() as i32,
                );
                if could_ladder && (input.up || input.down) && !(input.down && self.on_ground) {
                    self.state = PlayerState::OnLadder;
                    let middle_tx = (new_bb.x + new_bb.w * 0.5).floor() as i32;
                    self.bb.x = (middle_tx as f32 + 0.5) - self.bb.w * 0.5;

                    return;
                }

                if self.bb.vy > 0.0 {
                    let pressing_left = input.left && !input.right;
                    let pressing_right = input.right && !input.left;
                    if pressing_left || pressing_right {
                        let dir: Dir = if pressing_right { Dir::Right } else { Dir::Left };
                        if let Some(hang_pos) = check_and_snap_hang(&self.bb, &new_bb, map, dir) {
                            self.state = PlayerState::Hanging { dir, pos: hang_pos };
                            self.bb.vy = 0.0;
                            self.on_ground = false;
                            return;
                        }
                    }
                }

                self.bb = new_bb;
                self.on_ground = on_ground;
            },
            PlayerState::OnLadder => {
                if input.jump {
                    self.state = PlayerState::Normal;
                    self.bb.vy = -0.19;
                    return;
                }

                let middle_tx = (self.bb.x + self.bb.w * 0.5).floor() as i32;
                let head_ty = (self.bb.y).floor() as i32;
                let ladder_at_head = map.is_ladder_at(middle_tx, head_ty);
                let ladder_at_below = map.is_ladder_at(middle_tx, head_ty + 1);

                if input.up && !input.down {
                    if ladder_at_head {
                        self.bb.vy = -self.speed;
                    } else {
                        self.bb.vy = 0.0;
                        return;
                    }
                } else if input.down && !input.up {
                    self.bb.vy = self.speed;
                    let feet_y = self.bb.y + self.bb.h;
                    if map.is_solid_at(middle_tx, feet_y.floor() as i32) || (!ladder_at_head && !ladder_at_below) {
                        self.state = PlayerState::Normal;
                        self.on_ground = true;
                        self.bb.vy = 0.0;
                        return;
                    }
                } else {
                    self.bb.vy = 0.0;
                }
                let new_y = self.bb.y + self.bb.vy;

                self.bb.y = new_y;
            },
        }
    }
}


