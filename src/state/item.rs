use crate::physics::integrate_kinematic;
use crate::render::Renderer;
use crate::sound_handler::{Sound, SoundHandler};
use crate::state::game_map::MapLike;
use crate::state::{BoundingBox, Pos};
use rand::Rng;
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

pub enum ItemInteractionResult {
    RemoveItem,
    IncreaseScore, // TODO: Add amount to increase by
    SpawnItem { item: Item },
}

impl Item {
    pub fn new_with_velocity(
        center_x: f32,
        center_y: f32,
        vx: f32,
        vy: f32,
        item_type: ItemType,
    ) -> Self {
        let (width_px, height_px) = match item_type {
            ItemType::Coin => (5, 5),
            ItemType::SmallStone => (4, 4),
            ItemType::LargeStone => (8, 8),
            ItemType::Box => (8, 10),
        };

        let width = width_px as f32 / 16.0;
        let height = height_px as f32 / 16.0;

        Item {
            bb: BoundingBox {
                x: center_x - width / 2.0,
                y: center_y - height / 2.0,
                w: width,
                h: height,
                vx,
                vy,
            },
            item_type,
        }
    }

    pub fn new(center_x: f32, center_y: f32, item_type: ItemType) -> Self {
        Self::new_with_velocity(center_x, center_y, 0.0, 0.0, item_type)
    }

    pub fn draw_fake_xy(&self, renderer: &mut Renderer, x: f32, y: f32) {
        renderer.draw_from_texture_atlas(
            match self.item_type {
                ItemType::Coin => "coin",
                ItemType::SmallStone => "small_stone",
                ItemType::LargeStone => "large_stone",
                ItemType::Box => "box",
            },
            0,
            false,
            x,
            y,
            self.bb.w,
            self.bb.h,
            1.0,
        );
    }

    pub fn set_xyv(&mut self, x: f32, y: f32, vx: f32, vy: f32) {
        self.bb.x = x;
        self.bb.y = y;
        self.bb.vx = vx;
        self.bb.vy = vy;
    }

    pub fn overlaps(&self, bb: &BoundingBox) -> bool {
        self.bb.overlaps(bb)
    }

    pub fn overlaps_line(&self, a: &Pos, b: &Pos) -> bool {
        self.bb.overlaps_line(a, b)
    }

    pub fn draw(&self, renderer: &mut Renderer) {
        self.draw_fake_xy(renderer, self.bb.x, self.bb.y);
    }

    pub fn new_random(center_x: f32, center_y: f32) -> Self {
        let mut rng = rand::rng();

        let item_types = [
            ItemType::Coin,
            ItemType::SmallStone,
            ItemType::LargeStone,
            ItemType::Box,
        ];

        let random_type = item_types.choose(&mut rng).unwrap();

        Item::new(center_x, center_y, *random_type)
    }

    pub fn update(&mut self, map: &dyn MapLike) {
        let res = integrate_kinematic(map, &self.bb, true);
        self.bb = res.new_bb;

        if res.on_left || res.on_right {
            self.bb.vx = 0.0; // On right/left hits stop x movenment
        }
        if res.on_top || res.on_bottom {
            self.bb.vy = -self.bb.vy * 0.95;
            if self.bb.vy.abs() < 0.001 {
                // Bouncing stops if almost stopped
                self.bb.vy = 0.0;
            }
            self.bb.vx *= 0.95; // Slows down on bottom
            if self.bb.vx.abs() < 0.001 {
                self.bb.vx = 0.0;
            }
        }
    }

    pub fn handle_player_touch(
        &mut self,
        sound_handler: &SoundHandler,
    ) -> Vec<ItemInteractionResult> {
        match self.item_type {
            ItemType::Coin => {
                sound_handler.play(Sound::CollectCoin);
                vec![
                    ItemInteractionResult::RemoveItem,
                    ItemInteractionResult::IncreaseScore,
                ]
            }
            _ => vec![],
        }
    }

    pub fn handle_being_swung(
        &mut self,
        sound_handler: &SoundHandler,
    ) -> Vec<ItemInteractionResult> {
        match self.item_type {
            ItemType::Box => {
                let mut results = vec![ItemInteractionResult::RemoveItem];
                let mut rng = rand::rng();
                for _ in 0..rng.random_range(1..5) {
                    let vy = rng.random_range(-0.05..0.05);
                    let vx = rng.random_range(-0.05..0.05);
                    results.push(ItemInteractionResult::SpawnItem {
                        item: Item::new_with_velocity(
                            self.bb.x + self.bb.w * 0.5,
                            self.bb.y + self.bb.h * 0.5,
                            vx,
                            vy,
                            ItemType::Coin,
                        ),
                    })
                }
                sound_handler.play(Sound::Clink);
                results
            }
            _ => vec![],
        }
    }
}
