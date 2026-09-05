//! Per-match audio state (F11 `MatchAudioState`): the app-side sound event
//! queue.
//!
//! Everything here is scoped to one match and must reset when a new match
//! installs. The EVA state predicates (funds nag, low power, unit lost,
//! under-attack cadence) are sim-owned (`sim::house_eva`, the combat death
//! and damage sites); this owner only carries their events to the player.
//! Process/device-wide audio (players, registries, volumes) stays outside
//! this owner; grouping those into `AppAudioRuntime` lands with the F12
//! AppState owner reorganization.

use crate::audio::events::SoundEventQueue;

#[derive(Default)]
pub(crate) struct MatchAudioState {
    /// Sim/app sound events queued for playback this frame.
    pub(crate) sound_events: SoundEventQueue,
}

impl MatchAudioState {
    /// Reset every per-match latch and drop queued events. Called when a new
    /// match installs and when the player leaves a match for the shell.
    pub(crate) fn reset_for_new_match(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::MatchAudioState;

    /// F11: the new-match reset drops the queued sound events (previously
    /// surviving teardown undrained).
    #[test]
    fn new_match_reset_clears_queued_events() {
        let mut audio = MatchAudioState::default();
        audio
            .sound_events
            .push(crate::audio::events::GameSoundEvent::UiSound {
                sound_id: "leftover".to_string(),
            });

        audio.reset_for_new_match();

        assert!(
            audio.sound_events.drain().is_empty(),
            "queued events dropped"
        );
    }
}
