use crate::state::animation_handler::{AnimationConfig, AnimationConfigResult, AnimationHandler};
use crate::state::enemies::{Enemy, Slime, Worm};
use crate::state::{Bat, BoundingBox, Pos};
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
    Platform = 2,
    LadderPlatform = 3,
}

pub trait MapLike {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile);
    fn set_base(&mut self, x: i32, y: i32, tile: BaseTile);
    fn set_overlay(&mut self, x: i32, y: i32, tile: OverlayTile);
    fn get_ladders(&self) -> Vec<Pos>;
    fn get_platforms(&self) -> Vec<Pos>;

    fn is_ladder_at(&self, tx: i32, ty: i32) -> bool {
        matches!(
            self.get_at(tx, ty),
            (_, OverlayTile::Ladder) | (_, OverlayTile::LadderPlatform)
        )
    }
    fn is_platform_at(&self, tx: i32, ty: i32) -> bool {
        matches!(
            self.get_at(tx, ty),
            (_, OverlayTile::Platform) | (_, OverlayTile::LadderPlatform)
        )
    }
    fn overlaps_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool;

    fn is_solid_at_tile(&self, tx: i32, ty: i32) -> bool {
        let (base, _overlay) = self.get_at(tx, ty);
        match base {
            BaseTile::NotPartOfRoom => true,
            BaseTile::Empty => false,
            BaseTile::Stone => true,
            BaseTile::Wood => true,
        }
    }
    fn _overlaps_solid_tile(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self._is_solid_at_f_tile(x, y)
            || self._is_solid_at_f_tile(x + w, y)
            || self._is_solid_at_f_tile(x + w, y + h)
            || self._is_solid_at_f_tile(x, y + h)
    }

    fn _is_solid_at_f_tile(&self, tx: f32, ty: f32) -> bool {
        self.is_solid_at_tile(tx.floor() as i32, ty.floor() as i32)
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
    Worm = 2,
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

    pub fn get_texture_index(&self) -> &str {
        match self.object_type {
            ObjectTemplateType::Bat => "bat",
            ObjectTemplateType::Slime => "slime",
            ObjectTemplateType::Worm => "worm",
        }
    }

    pub fn as_object(&self) -> Box<dyn Enemy> {
        match self.object_type {
            ObjectTemplateType::Bat => Box::new(Bat::new(self.x, self.y)),
            ObjectTemplateType::Slime => Box::new(Slime::new(self.x, self.y)),
            ObjectTemplateType::Worm => Box::new(Worm::new(self.x, self.y)),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Room {
    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
    x: i32,
    y: i32,
    pub h: u32,
    pub w: u32,
    doors: Vec<RoomDoor>,
    pub object_templates: Vec<ObjectTemplate>,
}

impl Room {
    pub fn get_pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, pos: (i32, i32)) {
        let diff_x = self.x - pos.0;
        let diff_y = self.y - pos.1;

        for t in &mut self.object_templates {
            t.x -= diff_x as f32;
            t.y -= diff_y as f32;
        }

        self.x = pos.0;
        self.y = pos.1;
    }

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
            .map(|t| t.as_object())
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
                    if let Ok(num) = digits_part.parse::<u32>()
                        && num > max_index
                    {
                        max_index = num;
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
    fn overlaps_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self._overlaps_solid_tile(x, y, w, h)
    }

    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        self.get_relative(tx, ty)
            .unwrap_or((BaseTile::NotPartOfRoom, OverlayTile::None))
    }

    fn get_ladders(&self) -> Vec<Pos> {
        let mut all_pos = Vec::new();

        for x in 0..self.w {
            for y in 0..self.h {
                if matches!(
                    self.get_absolute(x, y),
                    (_, OverlayTile::Ladder) | (_, OverlayTile::LadderPlatform)
                ) {
                    all_pos.push(Pos::new(x as f32 + self.x as f32, y as f32 + self.y as f32));
                }
            }
        }

        all_pos
    }

    fn get_platforms(&self) -> Vec<Pos> {
        let mut all_pos = Vec::new();

        for x in 0..self.w {
            for y in 0..self.h {
                if matches!(
                    self.get_absolute(x, y),
                    (_, OverlayTile::Platform) | (_, OverlayTile::LadderPlatform)
                ) {
                    all_pos.push(Pos::new(x as f32 + self.x as f32, y as f32 + self.y as f32));
                }
            }
        }

        all_pos
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

// Slime bounces around
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
    closed_frames: i32, // Extra way to set the door open temporarily for n frames
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
    // Rooms contains the room information
    pub rooms: Vec<Room>,
    pub doors: Vec<MapDoor>,

    // These are used for when returning the game data for optimization reasons
    ladder_pos: Vec<Pos>,
    platform_pos: Vec<Pos>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
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
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for room in &self.rooms {
            min_x = room.x.min(min_x);
            min_y = room.y.min(min_y);
            max_x = (room.x + room.w as i32).max(max_x);
            max_y = (room.y + room.h as i32).max(max_y);
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    pub fn get_room_at(&self, x: f32, y: f32) -> Option<(usize, &Room)> {
        // TODO: Do not use this for enemies in every frame. Rather store the room info in the
        // TODO: enemy on creation
        for room_index in 0..self.rooms.len() {
            let room = &self.rooms[room_index];

            // Must be room at the center and 0.5 away from the center in every direction
            // (Since rooms overlap by 1 tile, the in room check must use 0.5 narrower
            // room than what it acutally is.
            let cent = room.get_relative(x.floor() as i32, y.floor() as i32);
            let a = room.get_relative((x - 0.5).floor() as i32, y.floor() as i32);
            let b = room.get_relative((x + 0.5).floor() as i32, y.floor() as i32);
            let c = room.get_relative(x.floor() as i32, (y - 0.5).floor() as i32);
            let d = room.get_relative(x.floor() as i32, (y + 0.5).floor() as i32);
            let is_in_this_room = match (cent, a, b, c, d) {
                // If one of these is not part of the room, then it is not in this room...
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
        let first_room = room_candidates.choose(&mut rng).unwrap().1.clone();
        let rooms = vec![first_room];

        let mut game_map = GameMap {
            rooms,
            doors: Vec::new(),
            ladder_pos: Vec::new(),
            platform_pos: Vec::new(),
            x: 0,
            y: 0,
            h: 0,
            w: 0,
            overlay: Vec::new(),
            base: Vec::new(),
        };

        // Iterate this

        let mut room_count = 1;
        'room_loop: for i in 0..1000 {
            println!("Iterating for adding a room {}", i);
            println!(" a) Choosing a random room to try to connect a room to");
            let random_existing_room_index = rng.random_range(0..game_map.rooms.len());
            // let random_existing_room = game_map.rooms.choose(&mut rng).unwrap();
            let random_existing_room = &game_map.rooms[random_existing_room_index];
            if random_existing_room.doors.is_empty() {
                continue;
            }
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

            let door_goes_up_down = match random_door_where_trying_to_connect.dir {
                DoorDir::Down => true,
                DoorDir::Up => true,
                DoorDir::Left => false,
                DoorDir::Right => false,
            };

            // random_new_room.x += -new_door_world_pos.0 + door_world_pos.0;
            // random_new_room.y += -new_door_world_pos.1 + door_world_pos.1;

            let random_new_room_new_x =
                random_new_room.x + (-new_door_world_pos.0 + door_world_pos.0);
            let random_new_room_new_y =
                random_new_room.y + (-new_door_world_pos.1 + door_world_pos.1);
            random_new_room.set_pos((random_new_room_new_x, random_new_room_new_y));

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

        for room in &game_map.rooms {
            let mut room_ladders = room.get_ladders();
            game_map.ladder_pos.append(&mut room_ladders);

            let mut room_platforms = room.get_platforms();
            game_map.platform_pos.append(&mut room_platforms);
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

    fn get_ladders(&self) -> Vec<Pos> {
        self.ladder_pos.clone()
    }

    fn get_platforms(&self) -> Vec<Pos> {
        self.platform_pos.clone()
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
