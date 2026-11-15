use crate::sound_handler::SoundPlayer;

/// Everything an entity update may need besides the entity itself and the map.
///
/// New cross-cutting services (random numbers, event logs, ...) go here so that
/// update function signatures do not need to change again.
pub struct UpdateCtx<'a> {
    pub sound: &'a dyn SoundPlayer,
}
