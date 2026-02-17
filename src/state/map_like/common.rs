use crate::state::common::BoundingBox;
use crate::state::enemies::{Bat, Burrower, Enemy, Slime, Worm};
use serde::{Deserialize, Serialize};

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
    StartDoor = 4,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct OverlayInfo {
    pub tile: OverlayTile,
    pub x: i32,
    pub y: i32,
}

pub trait MapLike {
    fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile);
    fn set_base(&mut self, x: i32, y: i32, tile: BaseTile);
    fn set_overlay(&mut self, x: i32, y: i32, tile: OverlayTile);
    fn get_overlays(&self) -> &Vec<OverlayInfo>;
    fn overlaps_solid(&self, x: f32, y: f32, w: f32, h: f32) -> bool;

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
    Burrower = 3,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ObjectTemplate {
    x: f32,
    y: f32,
    object_type: ObjectTemplateType,
}

impl ObjectTemplate {
    pub fn new(x: f32, y: f32, object_type: ObjectTemplateType) -> ObjectTemplate {
        ObjectTemplate { x, y, object_type }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    pub fn get_bb(&self) -> BoundingBox {
        *self.as_object().bb()
    }

    pub fn get_texture_index(&self) -> &str {
        match self.object_type {
            ObjectTemplateType::Bat => "bat",
            ObjectTemplateType::Slime => "slime",
            ObjectTemplateType::Worm => "worm",
            ObjectTemplateType::Burrower => "burrower",
        }
    }

    pub fn as_object(&self) -> Box<dyn Enemy> {
        match self.object_type {
            ObjectTemplateType::Bat => Box::new(Bat::new(self.x, self.y)),
            ObjectTemplateType::Slime => Box::new(Slime::new(self.x, self.y)),
            ObjectTemplateType::Worm => Box::new(Worm::new(self.x, self.y)),
            ObjectTemplateType::Burrower => Box::new(Burrower::new(self.x, self.y)),
        }
    }
}
