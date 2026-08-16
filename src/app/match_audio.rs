//! Per-match audio state (F11 `MatchAudioState`): the app-side sound event
//! queue and the EVA edge-detection latches.
//!
//! Everything here is scoped to one match and must reset when a new match
//! installs — most visibly the under-attack suppression window, which is
//! tick-indexed: carried into a new match whose tick counter restarts at
//! zero, it silenced the under-attack EVA line for the first ~30 seconds.
//! Process/device-wide audio (players, registries, volumes) stays outside
//! this owner; grouping those into `AppAudioRuntime` lands with the F12
//! AppState owner reorganization.

use crate::audio::events::SoundEventQueue;

#[derive(Default)]
pub(crate) struct MatchAudioState {
    /// Sim/app sound events queued for playback this frame.
    pub(crate) sound_events: SoundEventQueue,
    /// EVA edge-detection: the local house was in low power last frame.
    pub(crate) eva_low_power_active: bool,
    /// EVA edge-detection: a local factory was in an underfunded stall last frame.
    pub(crate) eva_funds_stalled: bool,
    /// EVA edge-detection: local mobile entities whose death has already been
    /// announced (pruned as the corpses despawn).
    pub(crate) eva_announced_dying: std::collections::HashSet<u64>,
    /// Sim tick until which the under-attack EVA voice is suppressed. The
    /// native per-house attack-voice repeat delay is UNVERIFIED-pending-trace;
    /// ~30 s is a conservative interim so sustained fire doesn't spam the line.
    pub(crate) eva_under_attack_block_until_tick: u64,
}

impl MatchAudioState {
    /// Reset every per-match latch and drop queued events. Called when a new
    /// match installs and when the player leaves a match for the shell.
    pub(crate) fn reset_for_new_match(&mut self) {
        *self = Self::default();
    }
}
