use crate::state::common::{BoundingBox, Health};
use crate::state::game_map::MapLike;
use crate::state::item::Item;

pub enum EnemyHitType {
    Swing,
    Stomp,
}

pub enum EnemyHitResult {
    GotHit,
    DidNotHit,
}

pub enum EnemyUpdateResult {
    // Spawn an item that will be thrown towards the player (with gravity and such)
    SpawnItemThrowTowardsPlayer { item: Item },
}

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;
    fn update(&mut self, map: &dyn MapLike) -> Vec<EnemyUpdateResult>;

    fn maybe_got_hit(&mut self, hit_type: EnemyHitType) -> EnemyHitResult;
    fn maybe_damage_player(&self) -> Option<u32>;
    fn draw(&self, renderer: &mut crate::render::Renderer);

    fn should_remove(&self) -> bool;
    fn get_health(&self) -> Health;
}
