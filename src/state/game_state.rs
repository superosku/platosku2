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

pub trait GameState {
    fn update(&mut self, input: &InputState);
    fn update_camera(&mut self, camera: &mut Camera, zoom_show_all: bool);
    fn player(&self) -> &Player;
    fn player_mut(&mut self) -> &mut Player;
    fn map_mut(&mut self) -> &mut dyn MapLike;
    fn map(&self) -> &dyn MapLike;
}

pub struct Editor {
    player: Player,
    pub room: Room,
}

impl Editor {
    pub fn new() -> Editor {
        let room = Room::new_boxed(0, 0, 5, 5);
        let player = Player::new(2.0, 2.0);

        Editor { player, room }
    }
}

impl GameState for Editor {
    fn update(&mut self, input: &InputState) {
        self.player.update(input, &self.room);
    }

    fn update_camera(&mut self, camera: &mut Camera, _zoom_show_all: bool) {
        let room_pos = self.room.get_pos();
        let camera_x = room_pos.0 as f32 + self.room.w as f32 * 0.5 - 3.0;
        let camera_y = room_pos.1 as f32 + self.room.h as f32 * 0.5;

        let camera_zoom = camera
            .zoom_to_fit_horizontal_tiles(self.room.w + 10)
            .min(camera.zoom_to_fit_vertical_tiles(self.room.h + 4));

        camera.slowly_follow(camera_x, camera_y, camera_zoom);
    }

    fn player(&self) -> &Player {
        &self.player
    }

    fn player_mut(&mut self) -> &mut Player {
        &mut self.player
    }

    fn map(&self) -> &dyn MapLike {
        &self.room
    }

    fn map_mut(&mut self) -> &mut dyn MapLike {
        &mut self.room
    }
}

pub struct Game {
    pub player: Player,
    pub map: GameMap,
    pub coins: Vec<Coin>,
    pub enemies: Vec<Box<dyn Enemy>>,

    cur_room_index: Option<usize>,
    prev_room_index: Option<usize>,
    prev_room_show_frames: u32,
}

impl Game {
    pub fn new() -> Game {
        let map = GameMap::new_random();
        let pos = map.player_start_pos();
        let player = Player::new(pos.0, pos.1);
        let enemies = map.get_enemies_from_templates();

        Game {
            player,
            map,
            coins: vec![],
            enemies,
            cur_room_index: None,
            prev_room_index: None,
            prev_room_show_frames: 0,
        }
    }

    pub fn get_rooms_for_display(&self) -> (Option<&Room>, Option<&Room>, f32) {
        let cur_room = self.cur_room_index.map(|index| &self.map.rooms[index]);
        let mut prev_room = self.prev_room_index.map(|index| &self.map.rooms[index]);
        if self.prev_room_show_frames == 0 {
            prev_room = None
        }
        (
            cur_room,
            prev_room,
            self.prev_room_show_frames as f32 / 30.0,
        )
    }
}

impl GameState for Game {
    fn update(&mut self, input: &InputState) {
        self.player.update(input, &self.map);
        for coin in &mut self.coins {
            coin.update(&self.map);
        }
        self.coins.retain(|c| !c.overlaps(&self.player.bb));

        for enemy in &mut self.enemies {
            enemy.update(&self.map);

            if enemy.bb().overlaps(&self.player.bb) {
                if enemy.can_be_stomped() && self.player.maybe_stomp(enemy.bb()) {
                    enemy.got_stomped();
                } else if self.player.can_be_hit() {
                    self.player.got_hit(enemy.contanct_damage());
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

        // Store the current and previous room as well as how many frames the previous has
        // been the previous. This is used for centering the camera and displaying the "black"
        // around the current room (/ rooms).
        if let Some((room_index, _room)) = self.map.get_room_at(
            self.player.bb.x + self.player.bb.w * 0.5,
            self.player.bb.y + self.player.bb.h * 0.5,
        ) {
            if self.cur_room_index != Some(room_index) {
                self.prev_room_index = self.cur_room_index;
                self.cur_room_index = Some(room_index);
                self.prev_room_show_frames = 30;
            }
        }
        if self.prev_room_show_frames > 0 {
            self.prev_room_show_frames -= 1;
        }
    }

    fn update_camera(&mut self, camera: &mut Camera, zoom_show_all: bool) {
        if zoom_show_all {
            let (x, y, w, h) = self.map.get_bounds();

            let camera_x = x as f32 + w as f32 * 0.5;
            let camera_y = y as f32 + h as f32 * 0.5;

            let camera_zoom = camera
                .zoom_to_fit_horizontal_tiles(w as u32)
                .min(camera.zoom_to_fit_vertical_tiles(h as u32));

            camera.slowly_follow(camera_x, camera_y, camera_zoom);
        } else {
            let rooms = self.get_rooms_for_display();
            if let Some(room) = rooms.0 {
                let room_pos = room.get_pos();
                let camera_x = room_pos.0 as f32 + room.w as f32 * 0.5;
                let camera_y = room_pos.1 as f32 + room.h as f32 * 0.5;

                let camera_zoom = camera
                    .zoom_to_fit_horizontal_tiles(room.w)
                    .min(camera.zoom_to_fit_vertical_tiles(room.h));

                camera.slowly_follow(camera_x, camera_y, camera_zoom);
            }
        }
    }

    fn player(&self) -> &Player {
        &self.player
    }

    fn player_mut(&mut self) -> &mut Player {
        &mut self.player
    }

    fn map(&self) -> &dyn MapLike {
        &self.map
    }

    fn map_mut(&mut self) -> &mut dyn MapLike {
        &mut self.map
    }
}
