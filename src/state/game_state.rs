use super::coin::Coin;
use super::enemies::Enemy;
use super::game_map::{GameMap, MapLike, Room};
use super::player::Player;
use crate::camera::Camera;

#[derive(Default)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub jump: bool,
    pub down: bool,
    pub swing: bool,
}

pub struct GameState {
    pub screen_w: f32,
    pub screen_h: f32,
    pub player: Player,
    pub map: Room, // Box<dyn MapLike>,
    pub input: InputState,
    pub coins: Vec<Coin>,
    pub enemies: Vec<Box<dyn Enemy>>,
    pub camera: Camera,
}

impl GameState {
    pub fn update(&mut self) {
        self.player.update(&self.input, &self.map);
        for coin in &mut self.coins {
            coin.update(&self.map);
        }
        self.coins.retain(|c| !c.overlaps(&self.player.bb));

        for enemy in &mut self.enemies {
            enemy.update(&self.map);

            if enemy.can_be_stomped() && enemy.bb().overlaps(&self.player.bb) {
                let did_stomp = self.player.maybe_stomp(enemy.bb());
                if did_stomp {
                    enemy.got_stomped();
                }
            }

            if let Some(swing_info) = self.player.get_swing_info() {
                if enemy.can_be_hit() && enemy.bb().point_inside(&swing_info.end) {
                    enemy.got_hit()
                }
            }
        }
        // Filter the enemies that are dead by enemy.is_dead() value
        self.enemies.retain(|e| !e.should_remove());

        let pcx = self.player.bb.x + self.player.bb.w * 0.5;
        let pcy = self.player.bb.y + self.player.bb.h * 0.5;
        self.camera.follow(pcx, pcy);
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
}
