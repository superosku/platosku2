use crate::sound_handler::SoundPlayer;

/// The random number generator used for all game logic.
///
/// ChaCha8 produces the same sequence on every platform for a given seed, which
/// keeps simulations reproducible (e.g. between macOS and CI on Linux).
pub type GameRng = rand_chacha::ChaCha8Rng;

/// Everything an entity update may need besides the entity itself and the map.
///
/// New cross-cutting services (random numbers, event logs, ...) go here so that
/// update function signatures do not need to change again.
pub struct UpdateCtx<'a> {
    pub sound: &'a dyn SoundPlayer,
    pub rng: &'a mut GameRng,
}
