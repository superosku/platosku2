use crate::state::common::{BoundingBox, Health};
use crate::state::game_map::MapLike;

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;

    fn update(&mut self, map: &dyn MapLike);
    fn got_stomped(&mut self);
    fn can_be_stomped(&self) -> bool;
    fn got_hit(&mut self);
    fn can_be_hit(&self) -> bool;
    fn should_remove(&self) -> bool;
    fn contanct_damage(&self) -> u32;
    fn get_health(&self) -> Health;
    fn get_texture_index(&self) -> &str;
    fn get_atlas_index(&self) -> u32;
    fn goes_right(&self) -> bool;
}
