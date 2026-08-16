//! Process-wide audio runtime (F12 `AppAudioRuntime`): output players and
//! sound/EVA registries that live for the whole process.
//!
//! Per-match audio state (event queue, EVA latches) lives in
//! `app::match_audio::MatchAudioState`; this owner survives matches. The
//! registries are reloaded on each map load today (redundant but harmless —
//! they consume no map-specific input); that behavior is unchanged here.

use crate::audio::music::MusicPlayer;
use crate::audio::sfx::SfxPlayer;
use crate::rules::sound_ini::{EvaRegistry, SoundRegistry};

pub(crate) struct AppAudioRuntime {
    /// Background music player (rodio). `None` when audio output is disabled
    /// or initialization failed.
    pub(crate) music_player: Option<MusicPlayer>,
    /// Sound effect player (rodio) — one-shot SFX (weapons, voices, UI).
    pub(crate) sfx_player: Option<SfxPlayer>,
    /// sound.ini / soundmd.ini registry mapping IDs to .wav filenames.
    pub(crate) sound_registry: SoundRegistry,
    /// audio.idx/bag indices for bag-based sound lookup (voices, EVA).
    /// Searched in order (YR audiomd first, then base audio).
    pub(crate) audio_indices: Vec<crate::assets::audio_bag::AudioIndex>,
    /// The process-start audio decision persisted for later scenario reloads.
    pub(crate) audio_indices_enabled: bool,
    /// EVA announcement registry from eva.ini / evamd.ini.
    /// Maps EVA event names to per-faction audio.bag sound IDs.
    pub(crate) eva_registry: EvaRegistry,
}
