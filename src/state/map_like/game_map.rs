use super::common::{BaseTile, DoorDir, MapLike, OverlayInfo, OverlayTile, RoomDoor};
use super::room::Room;
use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::common::BoundingBox;
use crate::state::enemies::Enemy;
use rand::Rng;
use rand::seq::IndexedRandom;

#[derive(PartialEq)]
enum DoorAnimationState {
    ClosedUpDown,
    OpenUpDown,
    ClosedSide,
    OpenSide,
}

impl AnimationConfig for DoorAnimationState {
    fn get_config(&self) -> AnimationConfigResult {
        match self {
            DoorAnimationState::OpenSide => AnimationConfigResult::new_no_loop(0, 7 - 1, 4),
            DoorAnimationState::ClosedSide => AnimationConfigResult::new_no_loop(7, 7 * 2 - 1, 4),
            DoorAnimationState::ClosedUpDown => {
                AnimationConfigResult::new_no_loop(7 * 2, 7 * 3 - 1, 4)
            }
            DoorAnimationState::OpenUpDown => {
                AnimationConfigResult::new_no_loop(7 * 3, 7 * 4 - 1, 4)
            }
        }
    }
}

pub struct MapDoor {
    pub x: i32,
    pub y: i32,
    pub goes_up_down: bool,
    open: bool,
    closed_frames: i32,
    animation_handler: AnimationHandler<DoorAnimationState>,
}

impl MapDoor {
    pub fn is_open(&self) -> bool {
        self.open && self.closed_frames == 0
    }

    pub fn new(x: i32, y: i32, goes_up_down: bool) -> MapDoor {
        MapDoor {
            x,
            y,
            goes_up_down,
            open: false,
            closed_frames: 0,
            animation_handler: AnimationHandler::new(if goes_up_down {
                DoorAnimationState::OpenUpDown
            } else {
                DoorAnimationState::OpenSide
            }),
        }
    }

    pub fn update(&mut self, is_open: bool) {
        self.open = is_open;
        self.closed_frames = 0.max(self.closed_frames - 1);
        match (self.goes_up_down, self.is_open()) {
            (true, true) => self
                .animation_handler
                .set_state(DoorAnimationState::OpenUpDown),
            (false, true) => self
                .animation_handler
                .set_state(DoorAnimationState::OpenSide),
            (true, false) => self
                .animation_handler
                .set_state(DoorAnimationState::ClosedUpDown),
            (false, false) => self
                .animation_handler
                .set_state(DoorAnimationState::ClosedSide),
        }
        self.animation_handler.increment_frame();
    }

    pub fn set_closed_for_frames(&mut self, n: u32) {
        self.closed_frames = n as i32
    }

    pub fn bb(&self) -> BoundingBox {
        match self.goes_up_down {
            true => BoundingBox {
                x: self.x as f32,
                y: self.y as f32 + 3.0 / 16.0,
                w: 1.0,
                h: 1.0 - 6.0 / 16.0,
                vy: 0.0,
                vx: 0.0,
            },
            false => BoundingBox {
                x: self.x as f32 + 3.0 / 16.0,
                y: self.y as f32,
                w: 1.0 - 6.0 / 16.0,
                h: 1.0,
                vy: 0.0,
                vx: 0.0,
            },
        }
    }

    pub fn get_atlas_index(&self) -> u32 {
        self.animation_handler.get_atlas_index()
    }
}

pub struct GameMap {
    pub rooms: Vec<Room>,
    pub doors: Vec<MapDoor>,

    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
    all_overlays: Vec<OverlayInfo>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl GameMap {
    pub fn player_start_pos(&self) -> (f32, f32) {
        let room = &self.rooms[0];

        let start_pos = room.get_start_pos().unwrap();

        (start_pos.0 as f32, start_pos.1 as f32)
    }

    pub fn get_bounds(&self) -> (i32, i32, i32, i32) {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for room in &self.rooms {
            let (rx, ry) = room.get_pos();
            min_x = rx.min(min_x);
            min_y = ry.min(min_y);
            max_x = (rx + room.w as i32).max(max_x);
            max_y = (ry + room.h as i32).max(max_y);
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn get_room_at(&self, x: f32, y: f32) -> Option<(usize, &Room)> {
        for room_index in 0..self.rooms.len() {
            let room = &self.rooms[room_index];

            let cent = room.get_relative(x.floor() as i32, y.floor() as i32);
            let a = room.get_relative((x - 0.5).floor() as i32, y.floor() as i32);
            let b = room.get_relative((x + 0.5).floor() as i32, y.floor() as i32);
            let c = room.get_relative(x.floor() as i32, (y - 0.5).floor() as i32);
            let d = room.get_relative(x.floor() as i32, (y + 0.5).floor() as i32);
            let is_in_this_room = match (cent, a, b, c, d) {
                (None, _, _, _, _) => false,
                (_, None, _, _, _) => false,
                (_, _, None, _, _) => false,
                (_, _, _, None, _) => false,
                (_, _, _, _, None) => false,
                (Some((BaseTile::NotPartOfRoom, _)), _, _, _, _) => false,
                (_, Some((BaseTile::NotPartOfRoom, _)), _, _, _) => false,
                (_, _, Some((BaseTile::NotPartOfRoom, _)), _, _) => false,
                (_, _, _, Some((BaseTile::NotPartOfRoom, _)), _) => false,
                (_, _, _, _, Some((BaseTile::NotPartOfRoom, _))) => false,
                (_, _, _, _, _) => true,
            };

            if is_in_this_room {
                return Some((room_index, room));
            }
        }

        None
    }

    pub fn get_enemies_from_templates(&self) -> Vec<Box<dyn Enemy>> {
        let mut enemies = Vec::new();

        for room in &self.rooms {
            let mut room_enemies = room.get_enemies_from_template();
            enemies.append(&mut room_enemies)
        }

        enemies
    }

    pub fn new_random() -> GameMap {
        let room_candidates = Room::load_rooms_from_folder();
        let mut rng = rand::rng();

        let first_room_candidates: Vec<Room> = room_candidates
            .iter()
            .map(|(_, room)| room.clone())
            .filter(|room| room.get_start_pos().is_some())
            .collect();

        let non_first_room_candidates: Vec<Room> = room_candidates
            .iter()
            .map(|(_, room)| room.clone())
            .filter(|room| room.get_start_pos().is_none())
            .collect();

        let first_room = first_room_candidates[0].clone();

        println!("First room: {:?}", first_room.get_start_pos());

        assert!(
            first_room.get_start_pos().is_some(),
            "First room must have a start pos"
        );

        let rooms = vec![first_room.clone()];

        let mut game_map = GameMap {
            rooms,
            doors: Vec::new(),
            all_overlays: Vec::new(),
            x: 0,
            y: 0,
            h: 0,
            w: 0,
            overlay: Vec::new(),
            base: Vec::new(),
        };

        let mut room_count = 1;
        'room_loop: for i in 0..1000 {
            println!("Iterating for adding a room {}", i);
            println!(" a) Choosing a random room to try to connect a room to");
            let random_existing_room_index = rng.random_range(0..game_map.rooms.len());
            let random_existing_room = &game_map.rooms[random_existing_room_index];
            if random_existing_room.get_doors().is_empty() {
                continue;
            }
            let random_door = random_existing_room.get_doors().choose(&mut rng).unwrap();
            let random_door_x = random_door.x;
            let random_door_y = random_door.y;
            let door_world_pos = random_existing_room.rel_to_abs((random_door.x, random_door.y));

            println!(" b) Choosing a random room to add");
            let mut random_new_room = non_first_room_candidates.choose(&mut rng).unwrap().clone();
            println!(" c) Choosing a random door");
            let door_match_candidates: Vec<RoomDoor> = random_new_room
                .get_doors()
                .iter()
                .filter(|door| match door.dir {
                    DoorDir::Down => random_door.dir == DoorDir::Up,
                    DoorDir::Up => random_door.dir == DoorDir::Down,
                    DoorDir::Left => random_door.dir == DoorDir::Right,
                    DoorDir::Right => random_door.dir == DoorDir::Left,
                })
                .cloned()
                .collect();

            if door_match_candidates.is_empty() {
                println!(" ERR: Could not find door match from random room");
                continue;
            }

            println!(" d) Checking if room overlaps with any other ones");
            let random_door_where_trying_to_connect =
                door_match_candidates.choose(&mut rng).unwrap();
            let new_door_world_pos = random_new_room.rel_to_abs((
                random_door_where_trying_to_connect.x,
                random_door_where_trying_to_connect.y,
            ));

            let door_goes_up_down = match random_door_where_trying_to_connect.dir {
                DoorDir::Down => true,
                DoorDir::Up => true,
                DoorDir::Left => false,
                DoorDir::Right => false,
            };

            let random_new_room_new_x =
                random_new_room.get_pos().0 + (-new_door_world_pos.0 + door_world_pos.0);
            let random_new_room_new_y =
                random_new_room.get_pos().1 + (-new_door_world_pos.1 + door_world_pos.1);
            random_new_room.set_pos((random_new_room_new_x, random_new_room_new_y));

            if random_new_room.get_pos().0 == random_existing_room.get_pos().0
                && random_new_room.get_pos().1 == random_existing_room.get_pos().1
            {
                println!(" ERR: Room is a direct copy overlapping maybe");
                continue;
            }

            for x in 0..random_new_room.w {
                for y in 0..random_new_room.h {
                    let new_room_tile = random_new_room.get_absolute(x, y).0;
                    let (new_room_x, new_room_y) = random_new_room.get_pos();

                    for room in &game_map.rooms {
                        let existing_tile =
                            room.get_at(new_room_x + x as i32, new_room_y + y as i32).0;

                        match (new_room_tile, existing_tile) {
                            (BaseTile::NotPartOfRoom, _) => {}
                            (_, BaseTile::NotPartOfRoom) => {}
                            (tile1, tile2) => {
                                if tile1 != tile2 {
                                    println!(
                                        " ERR: Room overlaps with other one {:?} {:?} ({} {})",
                                        tile1, tile2, x, y
                                    );
                                    continue 'room_loop;
                                }
                            }
                        }
                    }
                }
            }

            println!(" e) Adding room and clearing doors");
            random_new_room.set_base_absolute(
                random_door_where_trying_to_connect.x,
                random_door_where_trying_to_connect.y,
                BaseTile::Empty,
            );

            game_map.rooms[random_existing_room_index].set_base_absolute(
                random_door_x,
                random_door_y,
                BaseTile::Empty,
            );

            random_new_room.update_overlays_cache();

            game_map.rooms.push(random_new_room);
            game_map.doors.push(MapDoor::new(
                door_world_pos.0,
                door_world_pos.1,
                door_goes_up_down,
            ));

            room_count += 1;

            if room_count >= 10 {
                break;
            }
        }

        let (x, y, w, h) = game_map.get_bounds();

        game_map.x = x;
        game_map.y = y;
        game_map.w = w as u32;
        game_map.h = h as u32;

        game_map.overlay.resize((w * h) as usize, OverlayTile::None);
        game_map.base.resize((w * h) as usize, BaseTile::Stone);

        for xx in 0..w {
            for yy in 0..h {
                let (base, overlay) = game_map.get_at_from_room(x + xx, y + yy);

                game_map.overlay[(xx + yy * w) as usize] = overlay;
                game_map.base[(xx + yy * w) as usize] = base;
            }
        }

        game_map.all_overlays = game_map
            .rooms
            .iter()
            .flat_map(|room| room.get_overlays().clone())
            .collect();

        game_map
    }

    fn get_at_from_room(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        for room in &self.rooms {
            match room.get_relative(tx, ty) {
                None => {}
                Some((BaseTile::NotPartOfRoom, _)) => {}
                Some(res) => return res,
            }
        }

        (BaseTile::Stone, OverlayTile::None)
    }
}

impl MapLike for GameMap {
    fn overlaps_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        if self._overlaps_solid_tile(x, y, w, h) {
            return true;
        }
        for door in &self.doors {
            if !door.is_open()
                && door.bb().overlaps(&BoundingBox {
                    x,
                    y,
                    w,
                    h,
                    vx: 0.0,
                    vy: 0.0,
                })
            {
                return true;
            }
        }
        false
    }

    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        if tx < self.x
            || ty < self.y
            || tx >= self.x + self.w as i32
            || ty >= self.y + self.h as i32
        {
            return (BaseTile::Stone, OverlayTile::None);
        }
        let index = ((tx - self.x) + (ty - self.y) * self.w as i32) as usize;
        (self.base[index], self.overlay[index])
    }

    fn get_overlays(&self) -> &Vec<OverlayInfo> {
        &self.all_overlays
    }

    fn set_base(&mut self, _x: i32, _y: i32, _tile: BaseTile) {
        todo!()
    }

    fn set_overlay(&mut self, _x: i32, _y: i32, _tile: OverlayTile) {
        todo!()
    }
}
