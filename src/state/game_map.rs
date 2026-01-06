use crate::render::TextureIndexes;
use crate::state::enemies::{Enemy, Slime};
use crate::state::{Bat, BoundingBox};
use rand::Rng;
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::fs::DirEntry;
use std::{fs, io, path::Path};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Copy, Debug)]
pub enum BaseTile {
    NotPartOfRoom = 0,
    Empty = 1,
    Stone = 2,
    Wood = 3,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum OverlayTile {
    None = 0,
    Ladder = 1,
}

pub trait MapLike {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile);
    fn set_base(&mut self, x: i32, y: i32, tile: BaseTile);
    fn set_overlay(&mut self, x: i32, y: i32, tile: OverlayTile);

    fn is_solid_at(&self, tx: i32, ty: i32) -> bool {
        let (base, _overlay) = self.get_at(tx, ty);
        match base {
            BaseTile::NotPartOfRoom => true,
            BaseTile::Empty => false,
            BaseTile::Stone => true,
            BaseTile::Wood => true,
        }
    }

    fn is_ladder_at(&self, tx: i32, ty: i32) -> bool {
        let (_base, overlay) = self.get_at(tx, ty);
        matches!(overlay, OverlayTile::Ladder)
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub enum DoorDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RoomDoor {
    pub x: u32,
    pub y: u32,
    pub dir: DoorDir,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ObjectTemplateType {
    Bat = 0,
    Slime = 1,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectTemplate {
    x: f32,
    y: f32,
    object_type: ObjectTemplateType,
}

impl ObjectTemplate {
    fn new(x: f32, y: f32, object_type: ObjectTemplateType) -> ObjectTemplate {
        ObjectTemplate { x, y, object_type }
    }

    pub fn get_bb(&self) -> BoundingBox {
        *self.as_object().bb()
    }

    pub fn get_texture_index(&self) -> TextureIndexes {
        self.as_object().get_texture_index()
    }

    pub fn _as_object(&self, ox: f32, oy: f32) -> Box<dyn Enemy> {
        // Takes offset_x and offset_y so this can be used to get objects in both
        // relative and absolute positions (editor and game)
        match self.object_type {
            ObjectTemplateType::Bat => Box::new(Bat::new(self.x + ox, self.y + oy)),
            ObjectTemplateType::Slime => Box::new(Slime::new(self.x + ox, self.y + oy)),
        }
    }

    pub fn as_object(&self) -> Box<dyn Enemy> {
        self._as_object(0.0, 0.0)
    }

    pub fn as_object_rel(&self, room: &Room) -> Box<dyn Enemy> {
        self._as_object(room.x as f32, room.y as f32)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
    pub x: i32,
    pub y: i32,
    pub h: u32,
    pub w: u32,
    doors: Vec<RoomDoor>,
    pub object_templates: Vec<ObjectTemplate>,
}

impl Room {
    pub fn new_empty(x: i32, y: i32, w: u32, h: u32, base: BaseTile, overlay: OverlayTile) -> Room {
        // Create new base and overlay that has x * y size and is initialized to Empty and None
        let base = vec![base; (h * w) as usize];
        let overlay = vec![overlay; (h * w) as usize];

        Room {
            x,
            y,
            h,
            w,
            base,
            overlay,
            doors: Vec::new(),
            object_templates: Vec::new(),
        }
    }

    pub fn get_enemies_from_template(&self) -> Vec<Box<dyn Enemy>> {
        self.object_templates
            .iter()
            .map(|t| t.as_object_rel(self))
            .collect()
    }

    pub fn add_object_template(&mut self, x: f32, y: f32, template: ObjectTemplateType) {
        self.object_templates
            .push(ObjectTemplate::new(x, y, template))
    }

    pub fn get_doors(&self) -> &Vec<RoomDoor> {
        &self.doors
    }

    pub fn get_center(&self) -> (f32, f32) {
        (
            self.x as f32 + self.w as f32 / 2.0,
            self.y as f32 + self.h as f32 / 2.0,
        )
    }

    pub fn new_boxed(x: i32, y: i32, w: u32, h: u32) -> Room {
        let mut room = Room::new_empty(x, y, w, h, BaseTile::Empty, OverlayTile::None);

        for xx in 0..w {
            room.set_base_absolute(xx, 0, BaseTile::Wood);
            room.set_base_absolute(xx, room.h - 1, BaseTile::Wood);
        }
        for yy in 0..h {
            room.set_base_absolute(0, yy, BaseTile::Wood);
            room.set_base_absolute(room.w - 1, yy, BaseTile::Wood);
        }

        room
    }

    pub fn save_json(&self, path: impl AsRef<Path>) {
        let s = serde_json::to_string_pretty(self).unwrap();
        fs::write(path, s).unwrap();
    }

    pub fn load_json(path: impl AsRef<Path>) -> io::Result<Self> {
        let s = fs::read_to_string(path)?;
        let room =
            serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(room)
    }

    fn get_dir_entries() -> Vec<DirEntry> {
        // Read all json files from the "rooms" directory, sort by filename, load as Room
        let read_dir = match fs::read_dir("rooms") {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };

        let mut entries: Vec<fs::DirEntry> = read_dir
            .filter_map(|res| res.ok())
            .filter(|e| {
                let path = e.path();
                path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
            })
            .collect();

        entries.sort_by_key(|a| a.file_name());

        entries
    }

    pub fn next_available_file_name() -> String {
        let entries = Room::get_dir_entries();
        let mut max_index: u32 = 0;
        let mut digit_width: usize = 4; // default to 4 digits like room_0001.json

        for entry in entries {
            let file_name_os = entry.file_name();
            let file_name = file_name_os.to_string_lossy();

            // Expect pattern: room_<digits>.json
            if file_name.starts_with("room_") && file_name.ends_with(".json") {
                // slice between "room_" (5 chars) and ".json" (5 chars)
                if file_name.len() > 10 {
                    let digits_part = &file_name[5..file_name.len() - 5];
                    // remember observed width for padding
                    digit_width = digit_width.max(digits_part.len());
                    if let Ok(num) = digits_part.parse::<u32>() {
                        if num > max_index {
                            max_index = num;
                        }
                    }
                }
            }
        }

        let next_index = max_index + 1;
        // format with zero-padding to observed width
        format!(
            "rooms/room_{:0width$}.json",
            next_index,
            width = digit_width
        )
    }

    pub fn load_rooms_from_folder() -> Vec<(String, Self)> {
        let entries = Room::get_dir_entries();

        let mut rooms: Vec<(String, Self)> = Vec::new();
        for entry in entries {
            let path = entry.path();
            if let Ok(room) = Self::load_json(&path) {
                rooms.push((
                    String::from(path.file_name().unwrap().to_str().unwrap_or("<ERROR>")),
                    room,
                ));
            }
        }

        rooms
    }

    fn rel_to_abs(&self, xy: (u32, u32)) -> (i32, i32) {
        (self.x + xy.0 as i32, self.y + xy.1 as i32)
    }

    fn abs_to_rel(&self, xy: (i32, i32)) -> Option<(u32, u32)> {
        let rel_x = xy.0 - self.x;
        let rel_y = xy.1 - self.y;
        if rel_x < 0 || rel_y < 0 {
            return None;
        }
        if rel_x >= self.w as i32 || rel_y >= self.h as i32 {
            return None;
        }
        Some((rel_x as u32, rel_y as u32))
    }

    pub fn set_base_absolute(&mut self, x: u32, y: u32, tile: BaseTile) {
        self.base[(x + self.w * y) as usize] = tile;
    }

    pub fn set_overlay_absolute(&mut self, x: u32, y: u32, tile: OverlayTile) {
        self.overlay[(x + self.w * y) as usize] = tile;
    }

    pub fn get_absolute(&self, x: u32, y: u32) -> (BaseTile, OverlayTile) {
        (
            self.base[(x + self.w * y) as usize],
            self.overlay[(x + self.w * y) as usize],
        )
    }

    pub fn get_relative(&self, x: i32, y: i32) -> Option<(BaseTile, OverlayTile)> {
        if let Some(rel) = self.abs_to_rel((x, y)) {
            return Some(self.get_absolute(rel.0, rel.1));
        }
        None
    }

    pub fn resize_to_fit(&mut self, x: i32, y: i32) {
        let cols_to_add_left = (self.x - x).max(0);
        let rows_to_add_top = (self.y - y).max(0);
        let cols_to_add_right = (x - (self.x + self.w as i32) + 1).max(0);
        let rows_to_add_bottom = (y - (self.y + self.h as i32) + 1).max(0);

        println!(
            "{} {} {} {}",
            cols_to_add_left, cols_to_add_right, rows_to_add_bottom, rows_to_add_top
        );

        let new_h = self.h + rows_to_add_bottom as u32 + rows_to_add_top as u32;
        let new_w = self.w + cols_to_add_left as u32 + cols_to_add_right as u32;
        let new_size = new_h * new_w;

        let mut new_base = vec![BaseTile::NotPartOfRoom; new_size as usize];
        let mut new_overlay = vec![OverlayTile::None; new_size as usize];

        for xx in 0..self.w {
            for yy in 0..self.h {
                new_base[((xx as i32 + cols_to_add_left)
                    + new_w as i32 * (yy as i32 + rows_to_add_top))
                    as usize] = self.base[(xx + self.w * yy) as usize];
                new_overlay[((xx as i32 + cols_to_add_left)
                    + new_w as i32 * (yy as i32 + rows_to_add_top))
                    as usize] = self.overlay[(xx + self.w * yy) as usize];
            }
        }

        self.x = self.x.min(x);
        self.y = self.y.min(y);
        self.h = new_h;
        self.w = new_w;
        self.overlay = new_overlay;
        self.base = new_base;
        for door in &mut self.doors {
            door.x += cols_to_add_left as u32;
            door.y += rows_to_add_top as u32;
        }
    }

    pub fn resize_shrink(&mut self) {
        let mut cols_to_remove_left = 0;
        let mut rows_to_remove_top = 0;
        let mut cols_to_remove_right = 0;
        let mut rows_to_remove_bottom = 0;

        let mut still_doing_left = true;
        let mut still_doing_right = true;

        for xx in 0..self.w {
            for yy in 0..self.h {
                match self.get_absolute(xx, yy) {
                    (BaseTile::NotPartOfRoom, OverlayTile::None) => {}
                    (_, _) => {
                        still_doing_left = false;
                    }
                }
                match self.get_absolute(self.w - xx - 1, yy) {
                    (BaseTile::NotPartOfRoom, OverlayTile::None) => {}
                    (_, _) => {
                        still_doing_right = false;
                    }
                }
            }
            if !still_doing_left && !still_doing_right {
                break;
            }
            if still_doing_left {
                cols_to_remove_left += 1;
            }
            if still_doing_right {
                cols_to_remove_right += 1;
            }
        }

        let mut still_doing_top = true;
        let mut still_doing_bottom = true;
        for yy in 0..self.h {
            for xx in 0..self.w {
                match self.get_absolute(xx, yy) {
                    (BaseTile::NotPartOfRoom, OverlayTile::None) => {}
                    (_, _) => {
                        still_doing_top = false;
                    }
                }
                match self.get_absolute(xx, self.h - yy - 1) {
                    (BaseTile::NotPartOfRoom, OverlayTile::None) => {}
                    (_, _) => {
                        still_doing_bottom = false;
                    }
                }
            }
            if !still_doing_top && !still_doing_bottom {
                break;
            }
            if still_doing_top {
                rows_to_remove_top += 1;
            }
            if still_doing_bottom {
                rows_to_remove_bottom += 1;
            }
        }

        let new_h = self.h - rows_to_remove_bottom as u32 - rows_to_remove_top;
        let new_w = self.w - cols_to_remove_left - cols_to_remove_right as u32;
        let new_size = new_h * new_w;

        let mut new_base = vec![BaseTile::Empty; new_size as usize];
        let mut new_overlay = vec![OverlayTile::None; new_size as usize];

        for xx in 0..new_w {
            for yy in 0..new_h {
                new_base[(xx + yy * new_w) as usize] = self.base
                    [((xx + cols_to_remove_left) + self.w * (yy + rows_to_remove_top)) as usize];
                new_overlay[(xx + yy * new_w) as usize] = self.overlay
                    [((xx + cols_to_remove_left) + self.w * (yy + rows_to_remove_top)) as usize];
            }
        }

        self.x += cols_to_remove_left as i32;
        self.y += rows_to_remove_top as i32;
        self.h = new_h;
        self.w = new_w;
        self.overlay = new_overlay;
        self.base = new_base;

        for door in &mut self.doors {
            door.x = (door.x as i32 - cols_to_remove_left as i32)
                .max(0)
                .min(self.w as i32 - 1) as u32;
            door.y = (door.y as i32 - rows_to_remove_top as i32)
                .max(0)
                .min(self.h as i32 - 1) as u32;
        }
    }

    pub fn set_door(&mut self, x: i32, y: i32, dir: DoorDir) {
        println!("Set door");
        if let Some(rel_pos) = self.abs_to_rel((x, y)) {
            println!("Set doorrrr");
            self.remove_door(x, y);
            self.doors.push(RoomDoor {
                x: rel_pos.0,
                y: rel_pos.1,
                dir,
            });
        }
    }

    pub fn remove_door(&mut self, x: i32, y: i32) {
        println!("Remove door");
        if let Some(rel_pos) = self.abs_to_rel((x, y)) {
            println!("Remove doorrrr");
            self.doors
                .retain(|door| !(door.x == rel_pos.0 && door.y == rel_pos.1))
        }
    }
}

impl MapLike for Room {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        self.get_relative(tx, ty)
            .unwrap_or((BaseTile::NotPartOfRoom, OverlayTile::None))
    }

    fn set_base(&mut self, x: i32, y: i32, tile: BaseTile) {
        if let Some(rel) = self.abs_to_rel((x, y)) {
            self.set_base_absolute(rel.0, rel.1, tile);
        } else {
            // Resize first and then retry
            self.resize_to_fit(x, y);
            if let Some(rel) = self.abs_to_rel((x, y)) {
                self.set_base_absolute(rel.0, rel.1, tile);
            }
        }
        // Maybe shrink afterwards
        self.resize_shrink();
    }

    fn set_overlay(&mut self, x: i32, y: i32, tile: OverlayTile) {
        if let Some(rel) = self.abs_to_rel((x, y)) {
            self.set_overlay_absolute(rel.0, rel.1, tile);
        } else {
            // Resize first and then retry
            self.resize_to_fit(x, y);
            if let Some(rel) = self.abs_to_rel((x, y)) {
                self.set_overlay_absolute(rel.0, rel.1, tile);
            }
        }
        // Maybe shrink afterwards
        self.resize_shrink();
    }
}

pub struct GameMap {
    // pub base: Vec<Vec<BaseTile>>,
    // pub overlay: Vec<Vec<OverlayTile>>,
    pub rooms: Vec<Room>,
}

impl GameMap {
    pub fn player_start_pos(&self) -> (f32, f32) {
        let room = &self.rooms[0];
        (
            room.x as f32 + room.w as f32 * 0.5,
            room.y as f32 + room.h as f32 * 0.5,
        )
    }

    pub fn get_bounds(&self) -> (i32, i32, i32, i32) {
        let mut min_x = 10000;
        let mut min_y = 10000;
        let mut max_x = -10000;
        let mut max_y = -10000;

        for room in &self.rooms {
            min_x = room.x.min(min_x);
            min_y = room.y.min(min_y);
            max_x = (room.x + room.w as i32).max(max_x);
            max_y = (room.y + room.h as i32).max(max_y);
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn get_room_at(&self, x: f32, y: f32) -> Option<(usize, &Room)> {
        for room_index in 0..self.rooms.len() {
            let room = &self.rooms[room_index];

            if x < room.x as f32 + 0.5 || y < room.y as f32 + 0.5 {
                continue;
            }
            if x > (room.x as f32 + room.w as f32) - 0.5
                || y > (room.y as f32 + room.h as f32) - 0.5
            {
                continue;
            }

            if let Some((base, _overlay)) = room.get_relative(x as i32, y as i32) {
                if base != BaseTile::NotPartOfRoom {
                    return Some((room_index, room));
                }
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
        let first_room = room_candidates.choose(&mut rng).unwrap().1.clone();
        let rooms = vec![first_room];

        let mut game_map = GameMap { rooms };

        // Iterate this

        let mut room_count = 1;
        'room_loop: for i in 0..100 {
            println!("Iterating for adding a room {}", i);
            println!(" a) Choosing a random room to try to connect a room to");
            let random_existing_room_index = rng.random_range(0..game_map.rooms.len());
            // let random_existing_room = game_map.rooms.choose(&mut rng).unwrap();
            let random_existing_room = &game_map.rooms[random_existing_room_index];
            let random_door = random_existing_room.doors.choose(&mut rng).unwrap();
            let random_door_x = random_door.x;
            let random_door_y = random_door.y;
            let door_world_pos = random_existing_room.rel_to_abs((random_door.x, random_door.y));

            println!(" b) Choosing a random room to add");
            let mut random_new_room = room_candidates.choose(&mut rng).unwrap().1.clone();
            println!(" c) Choosing a random door");
            let door_match_candidates: Vec<RoomDoor> = random_new_room
                .doors
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
            random_new_room.x += -new_door_world_pos.0 + door_world_pos.0;
            random_new_room.y += -new_door_world_pos.1 + door_world_pos.1;

            if random_new_room.x == random_existing_room.x
                && random_new_room.y == random_existing_room.y
            {
                println!(" ERR: Room is a direct copy overlapping maybe");
                continue;
            }

            for x in 0..random_new_room.w {
                for y in 0..random_new_room.h {
                    let new_room_tile = random_new_room.get_absolute(x, y).0;

                    for room in &game_map.rooms {
                        let existing_tile = room
                            .get_at(random_new_room.x + x as i32, random_new_room.y + y as i32)
                            .0;

                        match (new_room_tile, existing_tile) {
                            (BaseTile::NotPartOfRoom, _) => {}
                            (_, BaseTile::NotPartOfRoom) => {}
                            // (BaseTile::Empty, _) => {},
                            // (_, BaseTile::Empty) => {},
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

            game_map.rooms.push(random_new_room);

            room_count += 1;

            if room_count >= 5 {
                break;
            }
        }

        // for x in 0..5 {
        //     for y in 0..5 {
        //         let mut room = room_candidates.choose(
        //             // &mut rand::rng()
        //             &mut rng
        //         ).unwrap().1.clone();
        //         room.x = x * 15;
        //         room.y = y * 15;
        //         rooms.push(room)
        //     }
        // }

        game_map
    }
}

impl MapLike for GameMap {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        for room in &self.rooms {
            match room.get_relative(tx, ty) {
                None => {}
                Some((BaseTile::NotPartOfRoom, _)) => {}
                Some(res) => return res,
            }
        }

        (BaseTile::Stone, OverlayTile::None)
    }

    fn set_base(&mut self, _x: i32, _y: i32, _tile: BaseTile) {
        todo!()
    }

    fn set_overlay(&mut self, _x: i32, _y: i32, _tile: OverlayTile) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_shrink_all() {
        let mut room = Room::new_empty(-1, -1, 10, 10, BaseTile::NotPartOfRoom, OverlayTile::None);
        room.set_base_absolute(1, 2, BaseTile::Stone);
        room.set_base_absolute(5, 5, BaseTile::Stone);
        println!(
            "Room before: ({} {}) ({} {})",
            room.x, room.y, room.w, room.h
        );
        room.resize_shrink();
        println!(
            "Room after: ({} {}) ({} {})",
            room.x, room.y, room.w, room.h
        );

        assert_eq!(room.x, 0);
        assert_eq!(room.y, 1);
        assert_eq!(room.w, 5);
        assert_eq!(room.h, 4);

        assert_eq!(room.get_absolute(0, 0).0, BaseTile::Stone);
        assert_eq!(room.get_absolute(4, 3).0, BaseTile::Stone);
    }

    #[test]
    fn test_resize_shrink_left() {
        let mut room = Room::new_empty(-2, -2, 5, 5, BaseTile::NotPartOfRoom, OverlayTile::None);
        room.set_base_absolute(2, 0, BaseTile::Stone);
        room.set_base_absolute(4, 4, BaseTile::Stone);
        println!(
            "Room before: ({} {}) ({} {})",
            room.x, room.y, room.w, room.h
        );
        room.resize_shrink();
        println!(
            "Room after: ({} {}) ({} {})",
            room.x, room.y, room.w, room.h
        );

        assert_eq!(room.x, 0);
        assert_eq!(room.y, -2);
        assert_eq!(room.w, 3);
        assert_eq!(room.h, 5);

        assert_eq!(room.get_absolute(0, 0).0, BaseTile::Stone);
        assert_eq!(room.get_absolute(2, 4).0, BaseTile::Stone);
    }

    #[test]
    fn test_resize_shrink_when_shrink_not_needed() {
        let mut room = Room::new_empty(-2, -2, 5, 5, BaseTile::NotPartOfRoom, OverlayTile::None);
        room.set_base_absolute(0, 0, BaseTile::Stone);
        room.set_base_absolute(4, 4, BaseTile::Stone);
        room.resize_shrink();

        assert_eq!(room.x, -2);
        assert_eq!(room.y, -2);
        assert_eq!(room.w, 5);
        assert_eq!(room.h, 5);

        assert_eq!(room.get_absolute(0, 0).0, BaseTile::Stone);
        assert_eq!(room.get_absolute(4, 4).0, BaseTile::Stone);
    }

    #[test]
    fn test_shrink_from_left_with_doors() {
        let mut room = Room::new_boxed(0, 0, 5, 5);
        // Set on left to expand the room and set the door
        room.set_base(-1, 1, BaseTile::Stone);
        room.set_door(-1, 1, DoorDir::Left);
        // Unset on left and shrink
        room.set_base(-1, 1, BaseTile::NotPartOfRoom);
        room.resize_shrink();

        println!("Doors: {:?}", room.doors);
        assert_eq!(
            room.doors[0],
            RoomDoor {
                x: 0,
                y: 1,
                dir: DoorDir::Left
            }
        );
    }
}
