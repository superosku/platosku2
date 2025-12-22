#[derive(Eq, PartialEq, Clone, Copy)]
pub enum BaseTile {
    Empty = 0,
    Stone = 1,
    Wood = 2,
}

#[derive(Clone, Copy)]
pub enum OverlayTile {
    None = 0,
    Ladder = 1,
}

pub struct GameMap {
    pub base: Vec<Vec<BaseTile>>,
    pub overlay: Vec<Vec<OverlayTile>>,
}

impl GameMap {
    pub fn width_tiles(&self) -> usize {
        self.base.first().map(|r| r.len()).unwrap_or(0)
    }
    pub fn height_tiles(&self) -> usize {
        self.base.len()
    }
    pub fn width(&self) -> f32 {
        self.width_tiles() as f32
    }
    pub fn height(&self) -> f32 {
        self.height_tiles() as f32
    }

    pub fn get_at(&self, tx: i32, ty: i32) -> (BaseTile, OverlayTile) {
        if tx < 0 || ty < 0 {
            return (BaseTile::Stone, OverlayTile::None);
        }
        let x = tx as usize;
        let y = ty as usize;
        let base = self
            .base
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .unwrap_or(BaseTile::Stone);
        let overlay = self
            .overlay
            .get(y)
            .and_then(|row| row.get(x))
            .copied()
            .unwrap_or(OverlayTile::None);
        (base, overlay)
    }

    pub fn is_solid_at(&self, tx: i32, ty: i32) -> bool {
        let (base, _overlay) = self.get_at(tx, ty);
        match base {
            BaseTile::Empty => false,
            BaseTile::Stone => true,
            BaseTile::Wood => true,
        }
    }

    pub fn is_ladder_at(&self, tx: i32, ty: i32) -> bool {
        let (_base, overlay) = self.get_at(tx, ty);
        match overlay {
            OverlayTile::Ladder => true,
            _ => false,
        }
    }
}
