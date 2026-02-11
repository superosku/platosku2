use super::game_map::{GameMap, MapLike, Room};
use super::player::{Player, PlayerUpdateResult};
use crate::camera::Camera;
use crate::sound_handler::{Sound, SoundHandler};
use crate::state::BoundingBox;
use crate::state::enemies::Enemy;
use crate::state::enemies::common::{EnemyHitResult, EnemyHitType};
use crate::state::item::{Item, ItemInteractionResult};
use rand::Rng;

#[derive(Default, Debug)]
pub struct InputState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,

    pub swing_pressed: bool,
    pub jump_pressed: bool,
    pub swing_held: bool,
    pub jump_held: bool,
}

pub trait GameState {
    fn update(&mut self, input: &InputState, sound_handler: &SoundHandler);
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
    fn update(&mut self, input: &InputState, sound_handler: &SoundHandler) {
        self.player.update(input, &self.room, sound_handler);
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
    pub items: Vec<Item>,
    pub enemies: Vec<Box<dyn Enemy>>,

    cur_room_index: Option<usize>,
    prev_room_index: Option<usize>,
    prev_room_show_frames: i32,
    room_change_position: (i32, i32),
}

const ROOM_TRANSITION_FRAMES: i32 = 30;

impl Game {
    pub fn new() -> Game {
        let map = GameMap::new_random();
        let pos = map.player_start_pos();
        let player = Player::new(pos.0, pos.1);
        let enemies = map.get_enemies_from_templates();

        // Add some random items to the map
        let mut items = vec![];
        for _ in 0..10000 {
            let (min_x, min_y, width, height) = map.get_bounds();
            let mut rng = rand::rng();
            let x = rng.random_range(min_x..min_x + width);
            let y = rng.random_range(min_y..min_y + height);

            if !map.is_solid_at_tile(x, y) {
                items.push(Item::new_random(x as f32 + 0.5, y as f32 + 0.5));
            }
            if items.len() > 50 {
                break;
            }
        }

        Game {
            player,
            map,
            items,
            enemies,
            cur_room_index: None,
            prev_room_index: None,
            prev_room_show_frames: 0,
            room_change_position: (0, 0),
        }
    }

    pub fn get_rooms_for_display(&self) -> (Option<&Room>, Option<&Room>, f32, (i32, i32)) {
        let cur_room = self.cur_room_index.map(|index| &self.map.rooms[index]);
        let mut prev_room = self.prev_room_index.map(|index| &self.map.rooms[index]);
        if self.prev_room_show_frames == 0 {
            prev_room = None
        }
        (
            cur_room,
            prev_room,
            self.prev_room_show_frames as f32 / ROOM_TRANSITION_FRAMES as f32,
            self.room_change_position,
        )
    }
}

impl GameState for Game {
    fn update(&mut self, input: &InputState, sound_handler: &SoundHandler) {
        let door_bbs: Vec<BoundingBox> = self
            .map
            .doors
            .iter()
            .filter(|door| !door.is_open())
            .map(|door| door.bb())
            .collect();

        // If player is inside one of the doors, we should move the player frame by frame towards
        // being out of the door and towards the current room
        if door_bbs.iter().any(|bb| self.player.bb.overlaps(bb)) {
            println!("Player overlaps a door");
            let mut push_dir: Option<(i32, i32)> = None;
            'outer: for step in 0..20 {
                for dir in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
                    let (x, y) = dir;
                    let mut user_bb = self.player.bb;
                    user_bb.x += step as f32 * 0.1 * x as f32;
                    user_bb.y += step as f32 * 0.1 * y as f32;

                    // This is a spot where the user would not overlap anything
                    if !self
                        .map
                        .overlaps_solid(user_bb.x, user_bb.y, user_bb.w, user_bb.h)
                    {
                        // This place has a room in it
                        if let Some((room_index, _)) = self
                            .map
                            .get_room_at(user_bb.x + user_bb.w * 0.5, user_bb.y + user_bb.h * 0.5)
                        {
                            // The player is at a room
                            if let Some(cur_room_index) = self.cur_room_index {
                                // The room is the same where the player is supposedly at
                                if cur_room_index == room_index {
                                    if dir.1 == -1 {
                                        // If going up lets add a little bit of "jump" to the player
                                        self.player.bb.vy = -0.1;
                                    }

                                    push_dir = Some(dir);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
            if let Some(push_dir) = push_dir {
                if push_dir.1 != 0 {
                    self.player.bb.y += self.player.bb.vy;
                    println!("Pushing player down form a door")
                } else {
                    self.player.bb.x += push_dir.0 as f32 * 0.05;
                    self.player.bb.y += push_dir.1 as f32 * 0.05;
                }
                // self.player.bb.vx = 0.0;
                // self.player.bb.vy = 0.0;
            } else {
                println!("No push dir found");
            }
        } else {
            let update_results = self.player.update(input, &self.map, sound_handler);
            for result in update_results {
                match result {
                    PlayerUpdateResult::AddItem { item } => {
                        self.items.push(item);
                    }
                    PlayerUpdateResult::PickUpItem => {
                        if let Some(item_match_index) = self
                            .items
                            .iter()
                            .position(|item| item.overlaps(&self.player.bb))
                        {
                            let item_match = self.items.remove(item_match_index);
                            self.player.set_item(item_match)
                        }
                    }
                }
            }
        }

        let mut new_items = Vec::new();
        self.items.retain_mut(|item| {
            let mut keep_item = true;
            item.update(&self.map);

            let mut handle_item_results = |results: Vec<ItemInteractionResult>| {
                for result in results {
                    match result {
                        ItemInteractionResult::RemoveItem => {
                            keep_item = false;
                        }
                        ItemInteractionResult::IncreaseScore => {}
                        ItemInteractionResult::SpawnItem { item } => {
                            new_items.push(item);
                        }
                    }
                }
            };

            if item.overlaps(&self.player.bb) {
                let results = item.handle_player_touch(sound_handler);
                handle_item_results(results);
            }

            if let Some(swing_info) = self.player.get_swing_info()
                && item.overlaps_line(&swing_info.pivot, &swing_info.end)
            {
                let results = item.handle_being_swung(sound_handler);
                handle_item_results(results);
            }

            keep_item
        });
        self.items.extend(new_items);

        for enemy in &mut self.enemies {
            enemy.update(&self.map);

            if enemy.bb().overlaps(&self.player.bb) {
                let mut should_hit_player = false;
                if self.player.maybe_stomp(enemy.bb()) {
                    match enemy.maybe_got_hit(EnemyHitType::Stomp) {
                        EnemyHitResult::DidNotHit => {
                            should_hit_player = true;
                        }
                        EnemyHitResult::GotHit => {
                            sound_handler.play(Sound::Clink);
                        }
                    }
                } else {
                    should_hit_player = true;
                }
                if should_hit_player && let Some(contact_damage) = enemy.maybe_damage_player() {
                    self.player.got_hit(contact_damage);
                }
            }

            if let Some(swing_info) = self.player.get_swing_info()
                // && enemy.can_be_hit()
                && enemy.bb().overlaps_line(&swing_info.pivot, &swing_info.end)
            {
                match enemy.maybe_got_hit(EnemyHitType::Swing) {
                    EnemyHitResult::DidNotHit => {}
                    EnemyHitResult::GotHit => {
                        sound_handler.play(Sound::Clink);
                        // TODO: Maybe play a different sound here than what the stomp plays?
                    }
                }
            }
        }
        // Filter the enemies that are dead by enemy.is_dead() value
        self.enemies.retain(|e| !e.should_remove());

        // Store the current and previous room as well as how many frames the previous has
        // been the previous. This is used for centering the camera and displaying the "black"
        // around the current room (/ rooms).

        // TODO: Use bb.get_center() here
        let player_center_x = self.player.bb.x + self.player.bb.w * 0.5;
        let player_center_y = self.player.bb.y + self.player.bb.h * 0.5;
        if let Some((room_index, _room)) = self.map.get_room_at(player_center_x, player_center_y)
            && self.cur_room_index != Some(room_index)
        {
            self.prev_room_index = self.cur_room_index;
            self.cur_room_index = Some(room_index);
            self.prev_room_show_frames = ROOM_TRANSITION_FRAMES;
            self.room_change_position = (
                player_center_x.floor() as i32,
                player_center_y.floor() as i32,
            );

            // Set the door closed here if the player is moving up and the door
            // type is up down. This helps in going to a room above
            for door in &mut self.map.doors {
                if door.goes_up_down
                    && self.player.bb.vy < 0.0
                    && self.player.bb.overlaps(&door.bb())
                {
                    door.set_closed_for_frames(120)
                }
            }
        }
        self.prev_room_show_frames = 0.max(self.prev_room_show_frames - 1);

        // Handle doors
        if let Some(cur_room_index) = self.cur_room_index {
            let mut list_of_bools: Vec<bool> = self
                .enemies
                .iter()
                .map(|enemy| {
                    let center = enemy.bb().center();
                    let enemy_room = self.map.get_room_at(center.x, center.y);
                    if let Some((index, _)) = enemy_room
                        && index == cur_room_index
                    {
                        return true;
                    };
                    false
                })
                .collect();
            list_of_bools.retain(|b| *b);
            let _room_has_enemies = !list_of_bools.is_empty();

            for door in &mut self.map.doors {
                // door.update(!room_has_enemies);
                // TODO: TEMP DOORS ALWAYS OPEN
                door.update(true);
            }
        } else {
            for door in &mut self.map.doors {
                door.update(true)
            }
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
