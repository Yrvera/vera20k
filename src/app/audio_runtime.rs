//! Process-wide audio runtime (F12 `AppAudioRuntime`): output players and
//! sound/EVA registries that live for the whole process.
//!
//! Per-match audio state (event queue, EVA latches) lives in
//! `app::match_audio::MatchAudioState`; this owner survives matches. The
//! registries are reloaded on each map load today (redundant but harmless —
//! they consume no map-specific input); that behavior is unchanged here.

use crate::assets::asset_manager::AssetManager;
use crate::audio::music::MusicPlayer;
use crate::audio::sfx::SfxPlayer;
use crate::audio::theme::{
    MusicOutputState, PreparedTrack, ThemeAction, ThemeGates, ThemeRuntime,
};
use crate::rules::sound_ini::{EvaRegistry, SoundRegistry};

pub(crate) const fn derive_launcher_audio_available(
    audio_requested: bool,
    music_output_ready: bool,
    sfx_output_ready: bool,
) -> bool {
    audio_requested && (music_output_ready || sfx_output_ready)
}

trait ThemeMusicOutput {
    fn set_theme_scale(&mut self, scale: f64);
    fn stop(&mut self);
    fn submit(&mut self, prepared: PreparedTrack) -> bool;
}

impl ThemeMusicOutput for MusicPlayer {
    fn set_theme_scale(&mut self, scale: f64) {
        MusicPlayer::set_theme_scale(self, scale);
    }

    fn stop(&mut self) {
        MusicPlayer::stop(self);
    }

    fn submit(&mut self, prepared: PreparedTrack) -> bool {
        MusicPlayer::submit(self, prepared)
    }
}

fn apply_theme_action_to_output(
    mut output: Option<&mut impl ThemeMusicOutput>,
    action: ThemeAction,
) {
    if let Some(scale) = action.theme_scale
        && let Some(output) = output.as_deref_mut()
    {
        output.set_theme_scale(scale);
    }
    if action.stop_output
        && let Some(output) = output.as_deref_mut()
    {
        output.stop();
    }
    if let Some(prepared) = action.start
        && let Some(output) = output.as_deref_mut()
        && !output.submit(prepared)
    {
        // gamemd ignores PlayFile's return and still writes logical active.
        log::warn!("Physical music submission failed after Theme admission");
    }
}

pub(crate) struct AppAudioRuntime {
    /// Always-present device-independent Theme owner.
    pub(crate) theme: ThemeRuntime,
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
    /// Rust-native substitute for native's one shared DirectSound-device gate.
    /// Frozen after both process-start output constructor attempts.
    pub(crate) launcher_audio_available: bool,
    /// Native has a startup-suppression gate. Current Rust has no non-default
    /// route, so production initializes this false and keeps one explicit seam.
    pub(crate) theme_startup_suppressed: bool,
    /// EVA announcement registry from eva.ini / evamd.ini.
    /// Maps EVA event names to per-faction audio.bag sound IDs.
    pub(crate) eva_registry: EvaRegistry,
}

impl AppAudioRuntime {
    fn theme_gates(&self) -> ThemeGates {
        ThemeGates {
            launcher_audio_available: self.launcher_audio_available,
            startup_suppressed: self.theme_startup_suppressed,
        }
    }

    fn music_output_state(&self) -> MusicOutputState {
        self.music_player
            .as_ref()
            .map_or(MusicOutputState::Unavailable, MusicPlayer::state)
    }

    fn apply_theme_action(&mut self, action: ThemeAction) {
        apply_theme_action_to_output(self.music_player.as_mut(), action);
    }

    pub(crate) fn initialize_theme(&mut self, assets: &AssetManager) {
        self.theme.initialize_catalog(assets);
    }

    pub(crate) fn maintain_main_menu_theme(&mut self, assets: &AssetManager, wall_ms: u64) {
        // Let Theme AI consume a real physical completion first. A subsequent
        // direct INTRO maintenance call then takes the native same-track no-op
        // when AI already restarted the repeating menu theme.
        self.update_theme(assets, wall_ms);
        let gates = self.theme_gates();
        let physical = self.music_output_state();
        let action = self.theme.play_menu_theme(assets, gates, physical);
        self.apply_theme_action(action);
    }

    pub(crate) fn update_theme(&mut self, assets: &AssetManager, wall_ms: u64) {
        let gates = self.theme_gates();
        let physical = self.music_output_state();
        if physical == MusicOutputState::Finished
            && let Some(output) = self.music_player.as_mut()
        {
            output.discard_finished();
        }
        let action = self.theme.update(assets, gates, physical, wall_ms);
        self.apply_theme_action(action);
    }

    pub(crate) fn play_theme(&mut self, track: &str, assets: &AssetManager) -> bool {
        let gates = self.theme_gates();
        let physical = self.music_output_state();
        let action = self.theme.play_track(track, assets, gates, physical);
        let logical_started = action.start.is_some();
        self.apply_theme_action(action);
        logical_started
    }

    pub(crate) fn request_scenario_theme(
        &mut self,
        requested_section: Option<&str>,
        assets: &AssetManager,
        wall_ms: u64,
    ) {
        let gates = self.theme_gates();
        let physical = self.music_output_state();
        let action =
            self.theme
                .request_scenario_theme(requested_section, assets, gates, physical, wall_ms);
        self.apply_theme_action(action);
    }

    pub(crate) fn cancel_scenario_theme_request(&mut self) {
        let action = self.theme.cancel_scenario_theme_request();
        self.apply_theme_action(action);
    }

    pub(crate) fn stop_theme(&mut self) {
        let action = self.theme.stop(self.theme_gates());
        self.apply_theme_action(action);
    }

    pub(crate) fn queue_then_stop_score_zero(&mut self) {
        let gates = self.theme_gates();
        let physical = self.music_output_state();
        let action = self.theme.queue_then_stop_score_zero(gates, physical);
        self.apply_theme_action(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum OutputCall {
        Scale(f64),
        Stop,
        Submit(String),
    }

    struct FakeOutput {
        calls: Vec<OutputCall>,
        submit_succeeds: bool,
    }

    impl ThemeMusicOutput for FakeOutput {
        fn set_theme_scale(&mut self, scale: f64) {
            self.calls.push(OutputCall::Scale(scale));
        }

        fn stop(&mut self) {
            self.calls.push(OutputCall::Stop);
        }

        fn submit(&mut self, prepared: PreparedTrack) -> bool {
            self.calls.push(OutputCall::Submit(prepared.stem));
            self.submit_succeeds
        }
    }

    #[test]
    fn launcher_audio_gate_uses_one_process_start_predicate() {
        assert!(!derive_launcher_audio_available(false, false, false));
        assert!(!derive_launcher_audio_available(false, true, true));
        assert!(!derive_launcher_audio_available(true, false, false));
        assert!(derive_launcher_audio_available(true, true, false));
        assert!(derive_launcher_audio_available(true, false, true));
        assert!(derive_launcher_audio_available(true, true, true));
    }

    #[test]
    fn post_admission_output_failure_cannot_rewrite_theme_state() {
        let theme = ThemeRuntime::default();
        let before = format!("{:?}", theme);
        let action = ThemeAction {
            stop_output: true,
            theme_scale: Some(0.25),
            start: Some(PreparedTrack {
                stem: "Drok".into(),
                samples: vec![0.0, 0.0],
                sample_rate: 22_050,
            }),
        };
        let mut output = FakeOutput {
            calls: Vec::new(),
            submit_succeeds: false,
        };

        apply_theme_action_to_output(Some(&mut output), action);

        assert_eq!(
            output.calls,
            vec![
                OutputCall::Scale(0.25),
                OutputCall::Stop,
                OutputCall::Submit("Drok".into()),
            ]
        );
        assert_eq!(format!("{:?}", theme), before);
    }
}
