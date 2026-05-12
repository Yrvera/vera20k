//! Wide-amplitude self-destruct — stub for Task 8b.

use crate::sim::game_entity::GameEntity;

/// Callback invoked when a rocking entity's body angle exceeds ±π.
/// Implemented in Task 8b.
pub trait SelfDestructHook {
    fn fire(&mut self, entity: &mut GameEntity);
}

/// No-op hook for tests and for the period before combat-side damage lands.
pub struct NoopSelfDestruct;

impl SelfDestructHook for NoopSelfDestruct {
    fn fire(&mut self, _entity: &mut GameEntity) {}
}

/// Stub — implemented in Task 8b.
pub fn check_and_fire(_entity: &mut GameEntity, _hook: &mut dyn SelfDestructHook) {
    // Implemented in Task 8b.
}
