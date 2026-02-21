use crate::sound_handler::{Sound, SoundHandler};
use crate::state::common::{BoundingBox, Health};
use crate::state::game_map::GameMap;
use crate::state::item::Item;

pub enum EnemyHitType {
    Swing,
    Stomp,
    Projectile,
}

pub enum EnemyHitResult {
    GotHit,
    DidNotHit,
}

pub enum EnemyUpdateResult {
    // Spawn an item that will be thrown towards the player (with gravity and such)
    SpawnItemThrowTowardsPlayer { item: Item },
    SpawnItemCastedTowardsPlayer { item: Item },
}

pub trait Enemy {
    fn bb(&self) -> &BoundingBox;
    fn update(&mut self, map: &GameMap, player_bb: &BoundingBox) -> Vec<EnemyUpdateResult>;

    fn maybe_got_hit(&mut self, hit_type: EnemyHitType) -> EnemyHitResult;
    fn maybe_damage_player(&self) -> Option<u32>;
    fn draw(&self, renderer: &mut crate::render::Renderer);

    fn should_remove(&self) -> bool;
    fn get_health(&self) -> Health;

    fn should_render_health_bar(&self) -> bool {
        true
    }
    fn maybe_got_hit_with_sound(
        &mut self,
        hit_type: EnemyHitType,
        sound_handler: &SoundHandler,
    ) -> EnemyHitResult {
        match self.maybe_got_hit(hit_type) {
            EnemyHitResult::DidNotHit => EnemyHitResult::DidNotHit,
            EnemyHitResult::GotHit => {
                sound_handler.play(Sound::EnemyHit);
                EnemyHitResult::GotHit
            }
        }
    }
}
