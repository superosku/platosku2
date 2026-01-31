use crate::physics::integrate_kinematic;
use crate::render::Renderer;
use crate::state::BoundingBox;
use crate::state::game_map::MapLike;
use rand::seq::IndexedRandom;

#[derive(Copy, Clone)]
pub enum ItemType {
    Coin,
    SmallStone,
    LargeStone,
    // Sack,
    Box,
    // Vase,
    // Arrow,
}

pub struct Item {
    bb: BoundingBox,
    item_type: ItemType,
}

impl Item {
    pub fn new(center_x: f32, center_y: f32, item_type: ItemType) -> Self {
        let (width_px, height_px) = match item_type {
            ItemType::Coin => (4, 4),
            ItemType::SmallStone => (3, 3),
            ItemType::LargeStone => (8, 8),
            ItemType::Box => (6, 8),
        };

        let width = width_px as f32 / 16.0;
        let height = height_px as f32 / 16.0;

        Item {
            bb: BoundingBox {
                x: center_x - width / 2.0,
                y: center_y - height / 2.0,
                w: width,
                h: height,
                vx: 0.0,
                vy: 0.0,
            },
            item_type,
        }
    }

    pub fn draw(&self, renderer: &mut Renderer) {
        renderer.draw_from_texture_atlas(
            match self.item_type {
                ItemType::Coin => "coin",
                ItemType::SmallStone => "small_stone",
                ItemType::LargeStone => "large_stone",
                ItemType::Box => "box",
            },
            0,
            false,
            self.bb.x,
            self.bb.y,
            self.bb.w,
            self.bb.h,
            1.0,
        );
    }

    pub fn new_random(center_x: f32, center_y: f32) -> Self {
        let mut rng = rand::rng();

        let item_types = [ItemType::Coin,
            ItemType::SmallStone,
            ItemType::LargeStone,
            ItemType::Box];

        let random_type = item_types.choose(&mut rng).unwrap();

        Item::new(center_x, center_y, *random_type)
    }

    pub fn update(&mut self, map: &dyn MapLike) {
        let res = integrate_kinematic(map, &self.bb, true);
        self.bb = res.new_bb;
    }
}
