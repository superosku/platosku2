use crate::state::common::{BoundingBox, Health};
use crate::state::game_map::MapLike;

pub enum EnemyHitType {
    Swing,
    Stomp,
}

pub enum EnemyHitResult {
    GotHit,
    DidNotHit,
}

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;
    fn update(&mut self, map: &dyn MapLike);

    fn maybe_got_hit(&mut self, hit_type: EnemyHitType) -> EnemyHitResult;
    fn maybe_damage_player(&self) -> Option<u32>;
    fn draw(&self, renderer: &mut crate::render::Renderer);

    fn should_remove(&self) -> bool;
    fn get_health(&self) -> Health;
}
