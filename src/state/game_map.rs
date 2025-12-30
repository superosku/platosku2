use serde::{Deserialize, Serialize};
use std::{fs, io, path::Path};

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Copy)]
pub enum BaseTile {
    Empty = 0,
    Stone = 1,
    Wood = 2,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
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

#[derive(Serialize, Deserialize)]
pub struct Room {
    base: Vec<BaseTile>,
    overlay: Vec<OverlayTile>,
    x: i32,
    y: i32,
    h: u32,
    w: u32,
}

impl Room {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Room {
        // Create new base and overlay that has x * y size and is initialized to Empty and None
        let base = vec![BaseTile::Empty; (h * w) as usize];
        let overlay = vec![OverlayTile::None; (h * w) as usize];

        let mut room = Room {
            x,
            y,
            h,
            w,
            base,
            overlay,
        };

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

        let mut new_base = vec![BaseTile::Empty; new_size as usize];
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
    }
}

impl MapLike for Room {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        self.get_relative(tx, ty)
            .unwrap_or((BaseTile::Empty, OverlayTile::None))
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
                rooms.push(Room::new(x * 6 + y - 8, y * 4 - 4, 7, 5))
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
