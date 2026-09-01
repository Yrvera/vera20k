//! Device-independent ThemeClass state and track preparation.
//!
//! Active Yuri's Revenge keeps Theme data and the logical active/retained/
//! pending slots alive even when its optional stream player is absent.  This
//! module mirrors that ownership: rodio remains in `music`, while aliases,
//! playlist selection, sentinels, scenario queueing, and lifecycle transitions
//! remain testable without an audio device.

use std::collections::{HashMap, HashSet};

use crate::assets::asset_manager::AssetManager;
use crate::assets::aud_file;
use crate::audio::sfx::decode_wav;
use crate::rules::ini_parser::IniFile;

const SCENARIO_THEME_FADE_MS: u64 = 1_000;
const MENU_THEME_SECTION: &str = "INTRO";

const FALLBACK_TRACKS: &[&str] = &[
    "Grinder", "Power", "Fortific", "InDeep", "Tension", "EagleHun", "Industro", "Jank",
    "200Meter", "BlowItUp", "Destroy", "Burn", "Motorize", "HM2", "Ra2-Opt", "RA2-Sco", "Drok",
    "Bully", "OptionX", "ScoreX", "BrainFre", "Deceiver", "PhatAtta", "Defend", "Tactics",
    "TranceLV",
];

/// The physical stream observation supplied to Theme's AI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MusicOutputState {
    /// No music stream object exists. This is never a completion signal.
    Unavailable,
    /// A stream object exists but no source is active.
    Idle,
    /// The current source is still playing.
    Playing,
    /// The current source reached its physical end.
    Finished,
}

/// Theme's literal pending sentinels plus a resolved track identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ThemeRequest {
    /// Native `-1`.
    None,
    /// Native `-2`.
    Auto,
    /// Native `-3`.
    Hold,
    Track(String),
}

impl Default for ThemeRequest {
    fn default() -> Self {
        Self::None
    }
}

/// Resolved Start_Scenario request. `Auto` is native index `-1` at the
/// scenario boundary and becomes Theme's `-2` automatic pending request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScenarioThemeRequest {
    Auto,
    Specific(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ThemeSlots {
    pub(crate) active: Option<String>,
    pub(crate) retained: Option<String>,
    pub(crate) pending: ThemeRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemeGates {
    pub(crate) launcher_audio_available: bool,
    pub(crate) startup_suppressed: bool,
}

/// Device-independent PCM payload ready for optional physical submission.
#[derive(Debug)]
pub(crate) struct PreparedTrack {
    pub(crate) stem: String,
    pub(crate) samples: Vec<f32>,
    pub(crate) sample_rate: u32,
}

#[derive(Debug, Default)]
pub(crate) struct ThemeAction {
    pub(crate) stop_output: bool,
    pub(crate) start: Option<PreparedTrack>,
    pub(crate) theme_scale: Option<f64>,
}

impl ThemeAction {
    fn then(mut self, next: Self) -> Self {
        self.stop_output |= next.stop_output;
        if next.start.is_some() {
            self.start = next.start;
        }
        if next.theme_scale.is_some() {
            self.theme_scale = next.theme_scale;
        }
        self
    }
}

#[derive(Debug, Clone, Default)]
struct ScenarioFade {
    started_at_ms: Option<u64>,
    completion_due: bool,
    owns_pending: bool,
    internal_scale: f64,
}

impl ScenarioFade {
    fn reset(&mut self) {
        self.started_at_ms = None;
        self.completion_due = false;
        self.owns_pending = false;
        self.internal_scale = 1.0;
    }
}

/// Process-lifetime logical Theme owner.
#[derive(Debug)]
pub(crate) struct ThemeRuntime {
    catalog_loaded: bool,
    aliases: HashMap<String, String>,
    scenario_theme_sections: HashMap<String, String>,
    playlist: Vec<String>,
    playlist_index: usize,
    repeating_tracks: HashSet<String>,
    global_repeat: bool,
    menu_theme: Option<String>,
    slots: ThemeSlots,
    scenario_fade: ScenarioFade,
}

impl Default for ThemeRuntime {
    fn default() -> Self {
        Self {
            catalog_loaded: false,
            aliases: HashMap::new(),
            scenario_theme_sections: HashMap::new(),
            playlist: FALLBACK_TRACKS
                .iter()
                .map(|track| (*track).to_string())
                .collect(),
            playlist_index: 0,
            repeating_tracks: HashSet::new(),
            global_repeat: false,
            menu_theme: None,
            slots: ThemeSlots::default(),
            scenario_fade: ScenarioFade {
                internal_scale: 1.0,
                ..Default::default()
            },
        }
    }
}

/// gamemd-derived: the device-independent Theme lifecycle is owned by
/// `ThemeClass` constructor `0x00720960`, AI `0x007209D0`, Next
/// `0x00720A80`, Queue `0x00720B20`, Play `0x00720BB0`, and Stop
/// `0x00720EA0`. These bodies operate on logical `[active, retained, pending]`
/// identities independently of the optional physical StreamPlayer.
impl ThemeRuntime {
    /// Load Theme configuration once, independently of physical output.
    pub(crate) fn initialize_catalog(&mut self, assets: &AssetManager) {
        if self.catalog_loaded {
            return;
        }

        let base = load_theme_ini(assets, "theme.ini");
        let md = load_theme_ini(assets, "thememd.ini");
        for ini in [base.as_ref(), md.as_ref()].into_iter().flatten() {
            merge_theme_aliases(&mut self.aliases, ini);
            merge_theme_section_stems(&mut self.scenario_theme_sections, ini);
            merge_repeating_tracks(&mut self.repeating_tracks, ini, &self.aliases);
        }

        let mut playlist = Vec::new();
        if let Some(ini) = base.as_ref() {
            playlist = playlist_from_theme_ini(ini, &self.aliases);
        }
        if let Some(ini) = md.as_ref() {
            for track in playlist_from_theme_ini(ini, &self.aliases) {
                if !playlist
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&track))
                {
                    playlist.push(track);
                }
            }
        }
        if !playlist.is_empty() {
            self.playlist = playlist;
            self.playlist_index = 0;
        }

        for ini in [base.as_ref(), md.as_ref()].into_iter().flatten() {
            if let Some((stem, repeats)) = menu_theme_from_ini(ini, &self.aliases) {
                if repeats {
                    self.repeating_tracks.insert(stem.to_ascii_uppercase());
                }
                self.menu_theme = Some(stem);
            }
        }
        self.catalog_loaded = true;
    }

    fn admitted(&self, gates: ThemeGates) -> bool {
        self.catalog_loaded && gates.launcher_audio_available && !gates.startup_suppressed
    }

    fn resolve_track_name(&self, track_name: &str) -> String {
        self.aliases
            .get(&track_name.to_ascii_uppercase())
            .cloned()
            .unwrap_or_else(|| track_name.to_string())
    }

    fn track_repeats(&self, stem: &str) -> bool {
        self.global_repeat || self.repeating_tracks.contains(&stem.to_ascii_uppercase())
    }

    pub(crate) fn play_track(
        &mut self,
        track_name: &str,
        assets: &AssetManager,
        gates: ThemeGates,
        physical: MusicOutputState,
    ) -> ThemeAction {
        self.initialize_catalog(assets);
        let mut prepare = |stem: &str| prepare_track(stem, assets);
        self.play_request_with(
            ThemeRequest::Track(track_name.to_string()),
            gates,
            physical,
            &mut prepare,
        )
    }

    pub(crate) fn play_menu_theme(
        &mut self,
        assets: &AssetManager,
        gates: ThemeGates,
        physical: MusicOutputState,
    ) -> ThemeAction {
        self.initialize_catalog(assets);
        let Some(stem) = self.menu_theme.clone() else {
            return ThemeAction::default();
        };
        let mut prepare = |resolved: &str| prepare_track(resolved, assets);
        self.play_request_with(ThemeRequest::Track(stem), gates, physical, &mut prepare)
    }

    /// Resolve and queue Start_Scenario's Theme in one authority call.
    pub(crate) fn request_scenario_theme(
        &mut self,
        requested_section: Option<&str>,
        assets: &AssetManager,
        gates: ThemeGates,
        physical: MusicOutputState,
        wall_ms: u64,
    ) -> ThemeAction {
        self.initialize_catalog(assets);
        let request =
            resolve_scenario_theme_section(requested_section, &self.scenario_theme_sections);
        let request = match request {
            ScenarioThemeRequest::Auto => ThemeRequest::Auto,
            ScenarioThemeRequest::Specific(stem) => ThemeRequest::Track(stem),
        };
        let admitted = self.admitted(gates);
        let action = self.queue_request(request, gates, physical, wall_ms);
        if admitted {
            self.scenario_fade.owns_pending = true;
        }
        action
    }

    pub(crate) fn queue_request(
        &mut self,
        request: ThemeRequest,
        gates: ThemeGates,
        physical: MusicOutputState,
        wall_ms: u64,
    ) -> ThemeAction {
        if !self.admitted(gates) {
            return ThemeAction::default();
        }

        let avoids_fade = matches!(request, ThemeRequest::None | ThemeRequest::Auto)
            || matches!(
                &request,
                ThemeRequest::Track(track)
                    if self
                        .slots
                        .retained
                        .as_deref()
                        .is_some_and(|retained| retained.eq_ignore_ascii_case(track))
            );
        self.slots.pending = request;
        self.scenario_fade.reset();

        if !avoids_fade && physical == MusicOutputState::Playing {
            self.scenario_fade.started_at_ms = Some(wall_ms);
        } else if physical != MusicOutputState::Unavailable
            && physical != MusicOutputState::Playing
            && !matches!(self.slots.pending, ThemeRequest::None | ThemeRequest::Hold)
        {
            self.scenario_fade.completion_due = true;
        }

        ThemeAction {
            theme_scale: Some(1.0),
            ..Default::default()
        }
    }

    /// Launcher ScoreVolume zero performs the native Queue(active else
    /// retained) followed immediately by Stop(false).
    pub(crate) fn queue_then_stop_score_zero(
        &mut self,
        gates: ThemeGates,
        physical: MusicOutputState,
    ) -> ThemeAction {
        if !self.admitted(gates) {
            return ThemeAction::default();
        }
        let request = self
            .slots
            .active
            .clone()
            .or_else(|| self.slots.retained.clone())
            .map(ThemeRequest::Track)
            .unwrap_or(ThemeRequest::None);
        let queued = self.queue_request(request, gates, physical, 0);
        queued.then(self.stop(gates))
    }

    pub(crate) fn stop(&mut self, gates: ThemeGates) -> ThemeAction {
        if !self.admitted(gates) || self.slots.active.is_none() {
            return ThemeAction::default();
        }
        self.slots = ThemeSlots::default();
        self.scenario_fade.reset();
        ThemeAction {
            stop_output: true,
            theme_scale: Some(1.0),
            ..Default::default()
        }
    }

    /// Cancel only Start_Scenario's queued/fade ownership during scenario
    /// reset. Active and retained identities remain owned by Theme.
    pub(crate) fn cancel_scenario_theme_request(&mut self) -> ThemeAction {
        if self.scenario_fade.owns_pending {
            self.slots.pending = ThemeRequest::None;
        }
        self.scenario_fade.reset();
        ThemeAction {
            theme_scale: Some(1.0),
            ..Default::default()
        }
    }

    pub(crate) fn update(
        &mut self,
        assets: &AssetManager,
        gates: ThemeGates,
        physical: MusicOutputState,
        wall_ms: u64,
    ) -> ThemeAction {
        self.initialize_catalog(assets);
        let mut prepare = |stem: &str| prepare_track(stem, assets);
        self.update_with(gates, physical, wall_ms, &mut prepare)
    }

    fn update_with(
        &mut self,
        gates: ThemeGates,
        physical: MusicOutputState,
        wall_ms: u64,
        prepare: &mut impl FnMut(&str) -> Option<PreparedTrack>,
    ) -> ThemeAction {
        if !self.admitted(gates) {
            return ThemeAction::default();
        }

        let mut action = ThemeAction::default();
        let mut completion = physical == MusicOutputState::Finished;

        if let Some(started_at_ms) = self.scenario_fade.started_at_ms {
            if physical == MusicOutputState::Playing {
                let elapsed = wall_ms.saturating_sub(started_at_ms);
                if elapsed < SCENARIO_THEME_FADE_MS {
                    self.scenario_fade.internal_scale =
                        1.0 - elapsed as f64 / SCENARIO_THEME_FADE_MS as f64;
                    action.theme_scale = Some(self.scenario_fade.internal_scale);
                    return action;
                }
                action.stop_output = true;
            }
            self.scenario_fade.started_at_ms = None;
            self.scenario_fade.internal_scale = 1.0;
            action.theme_scale = Some(1.0);
            completion = true;
        }

        completion |= self.scenario_fade.completion_due;
        self.scenario_fade.completion_due = false;
        if !completion {
            return action;
        }

        let pending = self.slots.pending.clone();
        self.scenario_fade.owns_pending = false;
        let requested = match pending {
            ThemeRequest::None | ThemeRequest::Hold => return action,
            ThemeRequest::Track(track) => Some(track),
            ThemeRequest::Auto => self.next_song(),
        };

        if let Some(track) = requested {
            action = action.then(self.play_request_with(
                ThemeRequest::Track(track),
                gates,
                MusicOutputState::Idle,
                prepare,
            ));
        }
        // ThemeClass::AI unconditionally restores Auto after attempting Play.
        self.slots.pending = ThemeRequest::Auto;
        action
    }

    fn next_song(&mut self) -> Option<String> {
        if let Some(retained) = self.slots.retained.as_deref()
            && self.track_repeats(retained)
        {
            return Some(retained.to_string());
        }
        let len = self.playlist.len();
        if len == 0 {
            return None;
        }
        let track = self.playlist[self.playlist_index % len].clone();
        self.playlist_index = (self.playlist_index + 1) % len;
        Some(track)
    }

    fn play_request_with(
        &mut self,
        request: ThemeRequest,
        gates: ThemeGates,
        physical: MusicOutputState,
        prepare: &mut impl FnMut(&str) -> Option<PreparedTrack>,
    ) -> ThemeAction {
        if !self.admitted(gates) {
            return ThemeAction::default();
        }

        match request {
            ThemeRequest::Auto => {
                self.slots.pending = ThemeRequest::Auto;
                return ThemeAction::default();
            }
            ThemeRequest::None | ThemeRequest::Hold => {
                if self.slots.active.is_none() {
                    return ThemeAction::default();
                }
                self.slots = ThemeSlots::default();
                self.scenario_fade.reset();
                return ThemeAction {
                    stop_output: true,
                    theme_scale: Some(1.0),
                    ..Default::default()
                };
            }
            ThemeRequest::Track(requested) => {
                let resolved = self.resolve_track_name(&requested);
                if physical == MusicOutputState::Playing
                    && self
                        .slots
                        .retained
                        .as_deref()
                        .is_some_and(|retained| retained.eq_ignore_ascii_case(&resolved))
                {
                    return ThemeAction::default();
                }

                let repeat = self.track_repeats(&resolved);
                let prior_active = self.slots.active.is_some();
                let pending_base = if prior_active {
                    ThemeRequest::None
                } else {
                    self.slots.pending.clone()
                };
                let mut action = ThemeAction::default();
                if prior_active {
                    self.slots = ThemeSlots::default();
                    action.stop_output = true;
                }
                self.scenario_fade.reset();
                action.theme_scale = Some(1.0);
                self.slots.retained = Some(resolved.clone());

                let Some(prepared) = prepare(&resolved) else {
                    self.slots.active = None;
                    self.slots.pending = if repeat {
                        ThemeRequest::Track(resolved.clone())
                    } else if physical != MusicOutputState::Unavailable
                        && matches!(pending_base, ThemeRequest::None)
                    {
                        // Native's stream-object resource/acquisition failure
                        // retries when the replacement base is `-1`.
                        ThemeRequest::Track(resolved.clone())
                    } else {
                        pending_base
                    };
                    return action;
                };

                self.slots.active = Some(resolved.clone());
                self.slots.retained = Some(resolved.clone());
                self.slots.pending = if repeat {
                    ThemeRequest::Track(resolved.clone())
                } else {
                    pending_base
                };
                if let Some(next_index) = playlist_index_after_specific(&self.playlist, &resolved) {
                    self.playlist_index = next_index;
                }
                action.start = Some(prepared);
                action
            }
        }
    }
}

/// Load and decode a track before optional physical submission.
fn prepare_track(track_name: &str, assets: &AssetManager) -> Option<PreparedTrack> {
    for filename in [format!("{track_name}.wav"), format!("{track_name}.aud")] {
        let Some(data) = assets.get_ref(&filename) else {
            continue;
        };

        if data.len() >= 44
            && &data[0..4] == b"RIFF"
            && let Some(decoded) = decode_wav(data, &filename)
            && decoded.sample_rate != 0
            && !decoded.samples.is_empty()
        {
            return Some(PreparedTrack {
                stem: track_name.to_string(),
                samples: decoded.samples,
                sample_rate: decoded.sample_rate,
            });
        }

        let Some((header, samples)) = aud_file::decode_aud(data) else {
            continue;
        };
        if samples.is_empty() || header.sample_rate == 0 {
            log::warn!("Track {track_name} decoded to 0 samples");
            return None;
        }
        let stereo = if header.is_stereo() {
            samples
                .iter()
                .map(|&sample| sample as f32 / 32768.0)
                .collect()
        } else {
            samples
                .iter()
                .flat_map(|&sample| {
                    let value = sample as f32 / 32768.0;
                    [value, value]
                })
                .collect()
        };
        return Some(PreparedTrack {
            stem: track_name.to_string(),
            samples: stereo,
            sample_rate: u32::from(header.sample_rate),
        });
    }
    log::warn!("Music track not found: resolved='{track_name}'");
    None
}

fn load_theme_ini(assets: &AssetManager, name: &str) -> Option<IniFile> {
    let bytes = assets.get_ref(name)?;
    IniFile::from_bytes(bytes).ok()
}

fn merge_theme_aliases(into: &mut HashMap<String, String>, ini: &IniFile) {
    for section_name in ini.section_names() {
        let Some(sound) = ini
            .section(section_name)
            .and_then(|section| section.get("Sound"))
            .filter(|sound| !sound.is_empty())
        else {
            continue;
        };
        let sound = sound.to_string();
        into.insert(section_name.to_ascii_uppercase(), sound.clone());
        into.insert(sound.to_ascii_uppercase(), sound);
    }
}

fn merge_theme_section_stems(into: &mut HashMap<String, String>, ini: &IniFile) {
    let Some(themes) = ini.section("Themes") else {
        return;
    };
    for section_name in themes.get_values() {
        let Some(sound) = ini
            .section(section_name)
            .and_then(|section| section.get("Sound"))
            .filter(|sound| !sound.is_empty())
        else {
            continue;
        };
        into.insert(section_name.to_ascii_uppercase(), sound.to_string());
    }
}

fn merge_repeating_tracks(
    into: &mut HashSet<String>,
    ini: &IniFile,
    aliases: &HashMap<String, String>,
) {
    for section_name in ini.section_names() {
        let Some(section) = ini.section(section_name) else {
            continue;
        };
        if !section.get_bool("Repeat").unwrap_or(false) {
            continue;
        }
        if let Some(stem) = aliases.get(&section_name.to_ascii_uppercase()) {
            into.insert(stem.to_ascii_uppercase());
        }
    }
}

fn resolve_scenario_theme_section(
    requested_section: Option<&str>,
    sections: &HashMap<String, String>,
) -> ScenarioThemeRequest {
    let Some(requested) = requested_section
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("No theme"))
    else {
        return ScenarioThemeRequest::Auto;
    };
    sections
        .get(&requested.to_ascii_uppercase())
        .cloned()
        .map(ScenarioThemeRequest::Specific)
        .unwrap_or(ScenarioThemeRequest::Auto)
}

fn playlist_index_after_specific(playlist: &[String], stem: &str) -> Option<usize> {
    let index = playlist
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(stem))?;
    Some((index + 1) % playlist.len())
}

fn menu_theme_from_ini(ini: &IniFile, aliases: &HashMap<String, String>) -> Option<(String, bool)> {
    let section = ini.section(MENU_THEME_SECTION)?;
    let stem = aliases
        .get(&MENU_THEME_SECTION.to_ascii_uppercase())
        .cloned()
        .or_else(|| {
            section
                .get("Sound")
                .filter(|sound| !sound.is_empty())
                .map(str::to_string)
        })?;
    Some((stem, section.get_bool("Repeat").unwrap_or(false)))
}

fn playlist_from_theme_ini(ini: &IniFile, aliases: &HashMap<String, String>) -> Vec<String> {
    let Some(themes) = ini.section("Themes") else {
        return Vec::new();
    };
    themes
        .get_values()
        .into_iter()
        .filter(|value| !value.is_empty())
        .filter_map(|theme_name| {
            let sound = aliases.get(&theme_name.to_ascii_uppercase())?;
            if ini
                .section(theme_name)
                .and_then(|section| section.get("Normal"))
                .is_some_and(|value| value.eq_ignore_ascii_case("no"))
            {
                return None;
            }
            Some(sound.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gates(enabled: bool) -> ThemeGates {
        ThemeGates {
            launcher_audio_available: enabled,
            startup_suppressed: false,
        }
    }

    fn runtime() -> ThemeRuntime {
        ThemeRuntime {
            catalog_loaded: true,
            aliases: [("INTRO".to_string(), "Drok".to_string())]
                .into_iter()
                .collect(),
            scenario_theme_sections: [("FORTIFICATION".to_string(), "Fortific".to_string())]
                .into_iter()
                .collect(),
            playlist: ["Grinder", "Power", "Fortific", "InDeep"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            repeating_tracks: ["DROK".to_string()].into_iter().collect(),
            menu_theme: Some("Drok".to_string()),
            ..Default::default()
        }
    }

    fn prepared(stem: &str) -> Option<PreparedTrack> {
        Some(PreparedTrack {
            stem: stem.to_string(),
            samples: vec![0.0, 0.0],
            sample_rate: 22_050,
        })
    }

    #[test]
    fn constructor_and_failed_common_gates_preserve_all_slots() {
        let mut theme = runtime();
        assert_eq!(theme.slots, ThemeSlots::default());
        theme.slots = ThemeSlots {
            active: Some("A".into()),
            retained: Some("R".into()),
            pending: ThemeRequest::Hold,
        };
        let before = theme.slots.clone();
        let action = theme.queue_request(
            ThemeRequest::Track("Q".into()),
            gates(false),
            MusicOutputState::Playing,
            10,
        );
        assert_eq!(theme.slots, before);
        assert!(!action.stop_output && action.start.is_none());

        let suppressed = ThemeGates {
            launcher_audio_available: true,
            startup_suppressed: true,
        };
        theme.stop(suppressed);
        assert_eq!(theme.slots, before);
    }

    #[test]
    fn queue_sentinels_and_same_retained_track_preserve_native_fade_rules() {
        let mut theme = runtime();
        theme.slots.active = Some("A".into());
        theme.slots.retained = Some("R".into());
        for request in [ThemeRequest::None, ThemeRequest::Auto] {
            theme.queue_request(request.clone(), gates(true), MusicOutputState::Playing, 100);
            assert_eq!(theme.slots.pending, request);
            assert!(theme.scenario_fade.started_at_ms.is_none());
        }
        theme.queue_request(
            ThemeRequest::Track("R".into()),
            gates(true),
            MusicOutputState::Playing,
            200,
        );
        assert!(theme.scenario_fade.started_at_ms.is_none());
        theme.queue_request(
            ThemeRequest::Hold,
            gates(true),
            MusicOutputState::Playing,
            300,
        );
        assert_eq!(theme.scenario_fade.started_at_ms, Some(300));
    }

    #[test]
    fn play_success_failure_repeat_and_same_track_rows_are_literal() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: Some("Old".into()),
            retained: Some("Old".into()),
            pending: ThemeRequest::Hold,
        };
        let mut ok = prepared;
        let action = theme.play_request_with(
            ThemeRequest::Track("Power".into()),
            gates(true),
            MusicOutputState::Playing,
            &mut ok,
        );
        assert!(action.stop_output && action.start.is_some());
        assert_eq!(
            theme.slots,
            ThemeSlots {
                active: Some("Power".into()),
                retained: Some("Power".into()),
                pending: ThemeRequest::None,
            }
        );

        let before = theme.slots.clone();
        let action = theme.play_request_with(
            ThemeRequest::Track("Power".into()),
            gates(true),
            MusicOutputState::Playing,
            &mut ok,
        );
        assert_eq!(theme.slots, before);
        assert!(!action.stop_output && action.start.is_none());

        theme.slots = ThemeSlots {
            active: None,
            retained: Some("Old".into()),
            pending: ThemeRequest::Hold,
        };
        let mut missing = |_stem: &str| None;
        theme.play_request_with(
            ThemeRequest::Track("Power".into()),
            gates(true),
            MusicOutputState::Unavailable,
            &mut missing,
        );
        assert_eq!(theme.slots.active, None);
        assert_eq!(theme.slots.retained.as_deref(), Some("Power"));
        assert_eq!(theme.slots.pending, ThemeRequest::Hold);

        theme.slots.pending = ThemeRequest::None;
        theme.play_request_with(
            ThemeRequest::Track("Power".into()),
            gates(true),
            MusicOutputState::Idle,
            &mut missing,
        );
        assert_eq!(theme.slots.pending, ThemeRequest::Track("Power".into()));

        theme.play_request_with(
            ThemeRequest::Track("INTRO".into()),
            gates(true),
            MusicOutputState::Unavailable,
            &mut ok,
        );
        assert_eq!(
            theme.slots,
            ThemeSlots {
                active: Some("Drok".into()),
                retained: Some("Drok".into()),
                pending: ThemeRequest::Track("Drok".into()),
            }
        );
    }

    #[test]
    fn replacing_active_preplay_failures_use_stream_object_specific_pending_rows() {
        for (physical, expected_pending) in [
            (MusicOutputState::Unavailable, ThemeRequest::None),
            (
                MusicOutputState::Playing,
                ThemeRequest::Track("Power".to_string()),
            ),
        ] {
            let mut theme = runtime();
            theme.slots = ThemeSlots {
                active: Some("Old".into()),
                retained: Some("Old".into()),
                pending: ThemeRequest::Hold,
            };
            let mut missing = |_stem: &str| None;
            let action = theme.play_request_with(
                ThemeRequest::Track("Power".into()),
                gates(true),
                physical,
                &mut missing,
            );
            assert!(action.stop_output);
            assert_eq!(theme.slots.active, None);
            assert_eq!(theme.slots.retained.as_deref(), Some("Power"));
            assert_eq!(theme.slots.pending, expected_pending);
        }
    }

    #[test]
    fn stop_and_direct_sentinel_rows_preserve_active_absent_state() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: None,
            retained: Some("R".into()),
            pending: ThemeRequest::Hold,
        };
        let before = theme.slots.clone();
        assert!(!theme.stop(gates(true)).stop_output);
        assert_eq!(theme.slots, before);

        let mut ok = prepared;
        theme.play_request_with(
            ThemeRequest::None,
            gates(true),
            MusicOutputState::Idle,
            &mut ok,
        );
        assert_eq!(theme.slots, before);
        theme.slots.active = Some("A".into());
        assert!(
            theme
                .play_request_with(
                    ThemeRequest::Hold,
                    gates(true),
                    MusicOutputState::Playing,
                    &mut ok,
                )
                .stop_output
        );
        assert_eq!(theme.slots, ThemeSlots::default());
    }

    #[test]
    fn completion_auto_specific_hold_and_unavailable_are_distinct() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: Some("Drok".into()),
            retained: Some("Drok".into()),
            pending: ThemeRequest::Track("Drok".into()),
        };
        let mut ok = prepared;
        let action = theme.update_with(gates(true), MusicOutputState::Finished, 1_000, &mut ok);
        assert!(action.start.is_some());
        assert_eq!(theme.slots.active.as_deref(), Some("Drok"));
        assert_eq!(theme.slots.retained.as_deref(), Some("Drok"));
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);

        let action = theme.update_with(gates(true), MusicOutputState::Finished, 1_500, &mut ok);
        assert!(action.start.is_some(), "Auto repeats retained INTRO");
        assert_eq!(theme.slots.active.as_deref(), Some("Drok"));
        assert_eq!(theme.slots.retained.as_deref(), Some("Drok"));
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);

        let before = theme.slots.clone();
        let action = theme.update_with(gates(true), MusicOutputState::Unavailable, 2_000, &mut ok);
        assert!(action.start.is_none());
        assert_eq!(theme.slots, before, "Unavailable is not Finished");

        theme.slots.pending = ThemeRequest::Hold;
        let before = theme.slots.clone();
        theme.update_with(gates(true), MusicOutputState::Finished, 3_000, &mut ok);
        assert_eq!(theme.slots, before);

        theme.slots.pending = ThemeRequest::None;
        let before = theme.slots.clone();
        theme.update_with(gates(true), MusicOutputState::Finished, 4_000, &mut ok);
        assert_eq!(theme.slots, before, "None completion preserves stale A/R");
    }

    #[test]
    fn ai_auto_success_and_failure_restore_auto_after_attempt() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: Some("Power".into()),
            retained: Some("Power".into()),
            pending: ThemeRequest::Auto,
        };
        theme.playlist_index = 2;
        let mut ok = prepared;
        theme.update_with(gates(true), MusicOutputState::Finished, 100, &mut ok);
        assert_eq!(theme.slots.active.as_deref(), Some("Fortific"));
        assert_eq!(theme.slots.retained.as_deref(), Some("Fortific"));
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);

        theme.slots.pending = ThemeRequest::Auto;
        let mut missing = |_stem: &str| None;
        theme.update_with(gates(true), MusicOutputState::Finished, 200, &mut missing);
        assert_eq!(theme.slots.active, None);
        assert_eq!(theme.slots.retained.as_deref(), Some("InDeep"));
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);
    }

    #[test]
    fn score_zero_is_queue_then_stop_not_unconditional_reset() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: Some("A".into()),
            retained: Some("R".into()),
            pending: ThemeRequest::Hold,
        };
        let action = theme.queue_then_stop_score_zero(gates(true), MusicOutputState::Playing);
        assert!(action.stop_output);
        assert_eq!(theme.slots, ThemeSlots::default());

        theme.slots = ThemeSlots {
            active: None,
            retained: Some("R".into()),
            pending: ThemeRequest::Hold,
        };
        let action = theme.queue_then_stop_score_zero(gates(true), MusicOutputState::Idle);
        assert!(!action.stop_output);
        assert_eq!(theme.slots.active, None);
        assert_eq!(theme.slots.retained.as_deref(), Some("R"));
        assert_eq!(theme.slots.pending, ThemeRequest::Track("R".into()));
    }

    #[test]
    fn scenario_specific_fades_then_ai_starts_and_restores_auto() {
        let mut theme = runtime();
        theme.slots = ThemeSlots {
            active: Some("Drok".into()),
            retained: Some("Drok".into()),
            pending: ThemeRequest::Track("Drok".into()),
        };
        let action = theme.queue_request(
            ThemeRequest::Track("Fortific".into()),
            gates(true),
            MusicOutputState::Playing,
            100,
        );
        assert_eq!(action.theme_scale, Some(1.0));
        let mut ok = prepared;
        assert_eq!(
            theme
                .update_with(gates(true), MusicOutputState::Playing, 600, &mut ok)
                .theme_scale,
            Some(0.5)
        );
        let action = theme.update_with(gates(true), MusicOutputState::Playing, 1_100, &mut ok);
        assert!(action.stop_output && action.start.is_some());
        assert_eq!(theme.slots.active.as_deref(), Some("Fortific"));
        assert_eq!(theme.slots.retained.as_deref(), Some("Fortific"));
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);
    }

    #[test]
    fn scenario_cancel_clears_only_scenario_owned_pending() {
        let mut theme = runtime();
        theme.slots.pending = ThemeRequest::Auto;
        theme.cancel_scenario_theme_request();
        assert_eq!(theme.slots.pending, ThemeRequest::Auto);

        theme.queue_request(
            ThemeRequest::Track("Fortific".into()),
            gates(true),
            MusicOutputState::Playing,
            10,
        );
        theme.scenario_fade.owns_pending = true;
        theme.cancel_scenario_theme_request();
        assert_eq!(theme.slots.pending, ThemeRequest::None);
    }

    #[test]
    fn resolver_uses_section_keys_and_playlist_specific_advances() {
        let ini = IniFile::from_str(
            "[Themes]\n0=Fortification\n1=Power\n\
             [Fortification]\nSound=Fortific\nNormal=yes\n\
             [Power]\nSound=Power\nNormal=yes\n",
        );
        let mut sections = HashMap::new();
        merge_theme_section_stems(&mut sections, &ini);
        for requested in [None, Some("No theme"), Some("Missing"), Some("Fortific")] {
            assert_eq!(
                resolve_scenario_theme_section(requested, &sections),
                ScenarioThemeRequest::Auto
            );
        }
        assert_eq!(
            resolve_scenario_theme_section(Some("fOrTiFiCaTiOn"), &sections),
            ScenarioThemeRequest::Specific("Fortific".into())
        );
        let playlist = ["Grinder", "Power", "Fortific", "InDeep"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            playlist[playlist_index_after_specific(&playlist, "fOrTiFiC").unwrap()],
            "InDeep"
        );
    }

    #[test]
    fn menu_theme_metadata_honors_repeat_and_absence() {
        let ini = IniFile::from_str("[INTRO]\nSound=Drok\nNormal=no\nRepeat=yes\n");
        let mut aliases = HashMap::new();
        merge_theme_aliases(&mut aliases, &ini);
        assert_eq!(
            menu_theme_from_ini(&ini, &aliases),
            Some(("Drok".into(), true))
        );
        let absent = IniFile::from_str("[SCORE]\nSound=Score\n");
        assert_eq!(menu_theme_from_ini(&absent, &HashMap::new()), None);
    }
}
