use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Serialize, Deserialize, Clone)]
pub enum DoorDir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RoomDoor {
    pub x: u32,
    pub y: u32,
    pub dir: DoorDir,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
    pub x: i32,
    pub y: i32,
    h: u32,
    w: u32,
    doors: Vec<RoomDoor>,
}

impl Room {
    pub fn new_empty(x: i32, y: i32, w: u32, h: u32) -> Room {
        // Create new base and overlay that has x * y size and is initialized to Empty and None
        let base = vec![BaseTile::Empty; (h * w) as usize];
        let overlay = vec![OverlayTile::None; (h * w) as usize];

        Room {
            x,
            y,
            h,
            w,
            base,
            overlay,
            doors: Vec::new(),
        }
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
        let mut room = Room::new_empty(x, y, w, h);

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

        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

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

        let new_h = self.h - rows_to_remove_bottom as u32 - rows_to_remove_top as u32;
        let new_w = self.w - cols_to_remove_left as u32 - cols_to_remove_right as u32;
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

        self.x = self.x + cols_to_remove_left as i32;
        self.y = self.y + rows_to_remove_top as i32;
        self.h = new_h;
        self.w = new_w;
        self.overlay = new_overlay;
        self.base = new_base;

        for door in &mut self.doors {
            door.x = (door.x as i32 - cols_to_remove_right as i32).max(0) as u32;
            door.y = (door.y as i32 - rows_to_remove_top as i32).max(0) as u32;
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
    }
}

pub struct GameMap {
    // pub base: Vec<Vec<BaseTile>>,
    // pub overlay: Vec<Vec<OverlayTile>>,
    rooms: Vec<Room>,
}

impl GameMap {
    pub fn new_random() -> GameMap {
        let mut rooms = Vec::new();

        for x in 0..5 {
            for y in 0..5 {
                rooms.push(Room::new_boxed(x * 6 + y - 8, y * 4 - 4, 7, 5))
            }
        }

        GameMap { rooms }
    }
}

impl MapLike for GameMap {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        for room in &self.rooms {
            if let Some(result) = room.get_relative(tx, ty) {
                return result;
            }
        }

        (BaseTile::Stone, OverlayTile::None)
    }

    fn set_base(&mut self, x: i32, y: i32, tile: BaseTile) {
        todo!()
    }

    fn set_overlay(&mut self, x: i32, y: i32, tile: OverlayTile) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_shrink_all() {
        let mut room = Room::new_empty(-1, -1, 10, 10);
        room.set_base_absolute(1, 2, BaseTile::Stone);
        room.set_base_absolute(5, 5, BaseTile::Stone);
        room.resize_shrink();

        assert_eq!(room.x, 0);
        assert_eq!(room.y, 1);
        assert_eq!(room.w, 5);
        assert_eq!(room.h, 4);

        assert_eq!(room.get_absolute(0, 0).0, BaseTile::Stone);
        assert_eq!(room.get_absolute(4, 3).0, BaseTile::Stone);
    }

    #[test]
    fn test_resize_shrink_left() {
        let mut room = Room::new_empty(-2, -2, 5, 5);
        room.set_base_absolute(2, 0, BaseTile::Stone);
        room.set_base_absolute(4, 4, BaseTile::Stone);
        room.resize_shrink();

        assert_eq!(room.x, 0);
        assert_eq!(room.y, -2);
        assert_eq!(room.w, 3);
        assert_eq!(room.h, 5);

        assert_eq!(room.get_absolute(0, 0).0, BaseTile::Stone);
        assert_eq!(room.get_absolute(2, 4).0, BaseTile::Stone);
    }

    #[test]
    fn test_resize_shrink_when_shrink_not_needed() {
        let mut room = Room::new_empty(-2, -2, 5, 5);
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
}
