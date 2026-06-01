//! Music playback using rodio.
//!
//! Loads theme tracks from THEME.MIX / thememd.mix, decodes WAV/AUD payloads
//! to PCM, and plays them through rodio. Supports play, stop,
//! next track, and live volume control.
//!
//! ## Track resolution
//! Track names come from thememd.ini / theme.ini or the map's [Basic] Theme= field.
//! The INI `Sound=` value resolves to the actual payload stem, which is looked
//! up first as `{stem}.wav`, then as `{stem}.aud`.
//!
//! ## Dependency rules
//! - Part of audio/ — depends on assets/ (aud_file decoder, AssetManager)
//!   and rules/ini_parser for theme metadata.
//! - Does NOT depend on render/, ui/, sidebar/, sim/.

use std::collections::HashMap;
use std::num::NonZero;
use std::path::Path;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

use crate::assets::asset_manager::AssetManager;
use crate::assets::aud_file;
use crate::audio::sfx::decode_wav;
use crate::rules::ini_parser::IniFile;

/// Engine default for the score (music) volume, applied before any user
/// settings file is read. The original game seeds its live music volume to
/// this same default when the saved `ScoreVolume` setting is absent.
pub const DEFAULT_SCORE_VOLUME: f64 = 0.4;

/// User settings filename in the RA2 install dir holding `[Audio] ScoreVolume`.
const RA2MD_INI_FILENAME: &str = "RA2MD.INI";
/// Section and key for the saved music volume in RA2MD.INI.
const AUDIO_SECTION: &str = "Audio";
const SCORE_VOLUME_KEY: &str = "ScoreVolume";

const FALLBACK_TRACKS: &[&str] = &[
    "Grinder", "Power", "Fortific", "InDeep", "Tension", "EagleHun", "Industro", "Jank",
    "200Meter", "BlowItUp", "Destroy", "Burn", "Motorize", "HM2", "Ra2-Opt", "RA2-Sco", "Drok",
    "Bully", "OptionX", "ScoreX", "BrainFre", "Deceiver", "PhatAtta", "Defend", "Tactics",
    "TranceLV",
];

/// Manages background music playback.
pub struct MusicPlayer {
    /// rodio mixer device sink — must be kept alive or all audio stops.
    _device: MixerDeviceSink,
    /// Player handle for the currently playing track, if any.
    current_player: Option<Player>,
    /// Name of the currently playing track.
    current_track: Option<String>,
    /// Playlist of track names to cycle through.
    playlist: Vec<String>,
    /// Theme alias -> actual sound stem, uppercase keys.
    aliases: HashMap<String, String>,
    /// Index into the playlist for the next track.
    playlist_index: usize,
    /// Music volume (0.0 to 1.0).
    volume: f64,
    /// When set, the resolved sound stem to re-play on finish instead of
    /// advancing the playlist (honors a theme's `Repeat=yes`, e.g. the menu
    /// [INTRO] theme which loops the entire time the shell is shown).
    looping_track: Option<String>,
    /// Resolved sound stem of the menu [INTRO] theme, from theme INI.
    menu_theme: Option<String>,
    /// Whether the menu [INTRO] theme is marked `Repeat=yes` in the INI.
    menu_theme_repeats: bool,
}

impl MusicPlayer {
    /// Create a new MusicPlayer. Returns None if audio output cannot be opened.
    pub fn new() -> Option<Self> {
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| log::error!("Failed to initialize music audio: {}", e))
            .ok()?;

        Some(Self {
            _device: device,
            current_player: None,
            current_track: None,
            playlist: FALLBACK_TRACKS.iter().map(|s| s.to_string()).collect(),
            aliases: HashMap::new(),
            playlist_index: 0,
            volume: DEFAULT_SCORE_VOLUME,
            looping_track: None,
            menu_theme: None,
            menu_theme_repeats: false,
        })
    }

    /// Play a specific track by name. Loads from the AssetManager on demand.
    /// Returns true if the track was found and playback started.
    ///
    /// This is the one-shot entry point used for gameplay tracks; it clears
    /// any active looping so a menu loop does not bleed into a started map.
    pub fn play_track(&mut self, track_name: &str, assets: &AssetManager) -> bool {
        self.looping_track = None;
        self.ensure_theme_config(assets);
        self.start_track(track_name, assets)
    }

    /// Start the menu [INTRO] theme, looping it if the INI marks it
    /// `Repeat=yes`. Idempotent: if the menu theme is already the current
    /// track, this is a no-op so it is safe to call every frame the shell is
    /// shown. Returns true if the menu theme is playing afterward.
    ///
    /// The theme name and repeat flag come from the theme INI ([INTRO]
    /// section), not a hardcoded stem.
    pub fn play_menu_theme(&mut self, assets: &AssetManager) -> bool {
        self.ensure_theme_config(assets);

        let Some(stem) = self.menu_theme.clone() else {
            return false;
        };

        // Already playing the menu theme — don't restart it.
        if self
            .current_track
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case(&stem))
        {
            return true;
        }

        let repeats = self.menu_theme_repeats;
        let started = self.start_track(&stem, assets);
        if started && repeats {
            // start_track stores the resolved stem in current_track.
            self.looping_track = self.current_track.clone();
        }
        started
    }

    /// Resolve, load, and begin playing a track. Does NOT touch the loop flag
    /// — callers manage looping. Returns true if playback started.
    fn start_track(&mut self, track_name: &str, assets: &AssetManager) -> bool {
        // Stop the current player without clearing loop state owned by caller.
        if let Some(player) = self.current_player.take() {
            player.stop();
        }
        self.current_track = None;

        let resolved_name: String = self.resolve_track_name(track_name);
        let (samples, sample_rate) = match load_track(&resolved_name, assets) {
            Some(data) => data,
            None => {
                log::warn!(
                    "Music track not found: requested='{}', resolved='{}'",
                    track_name,
                    resolved_name
                );
                return false;
            }
        };

        let channels = match NonZero::new(2u16) {
            Some(c) => c,
            None => return false,
        };
        let rate = match NonZero::new(sample_rate) {
            Some(r) => r,
            None => return false,
        };

        let source = SamplesBuffer::new(channels, rate, samples);
        let player: Player = Player::connect_new(self._device.mixer());
        player.set_volume(self.volume as f32);
        player.append(source);
        log::info!(
            "Playing music track: requested='{}', resolved='{}'",
            track_name,
            resolved_name
        );
        self.current_player = Some(player);
        self.current_track = Some(resolved_name);
        true
    }

    /// Stop the currently playing track. Also cancels any active loop so a
    /// stopped menu theme does not silently re-trigger on the next update.
    pub fn stop(&mut self) {
        self.looping_track = None;
        if let Some(player) = self.current_player.take() {
            player.stop();
        }
        self.current_track = None;
    }

    /// Play the next track in the playlist. Wraps around at the end.
    /// Returns the name of the track that started playing, or None if no track loaded.
    pub fn play_next(&mut self, assets: &AssetManager) -> Option<String> {
        self.ensure_theme_config(assets);

        if self.playlist.is_empty() {
            return None;
        }

        let len: usize = self.playlist.len();
        for attempt in 0..len {
            let idx: usize = (self.playlist_index + attempt) % len;
            let track_name: String = self.playlist[idx].clone();
            self.playlist_index = (idx + 1) % len;
            if self.play_track(&track_name, assets) {
                return Some(track_name);
            }
        }
        None
    }

    /// Check if the current track has finished and auto-advance to the next.
    /// Call this once per frame from the game loop.
    pub fn update(&mut self, assets: &AssetManager) {
        let finished: bool = match &self.current_player {
            Some(player) => player.empty(),
            None => self.current_track.is_some(),
        };

        if finished {
            self.current_player = None;
            self.current_track = None;
            // A looping track (e.g. the menu [INTRO] theme, Repeat=yes)
            // re-plays itself instead of advancing the playlist.
            if let Some(stem) = self.looping_track.clone() {
                if self.start_track(&stem, assets) {
                    return;
                }
                // Failed to restart — fall through to playlist advance.
                self.looping_track = None;
            }
            let _ = self.play_next(assets);
        }
    }

    /// Set the music volume (0.0 = silent, 1.0 = full).
    /// Applies immediately to the currently playing track.
    pub fn set_volume(&mut self, volume: f64) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(ref player) = self.current_player {
            player.set_volume(self.volume as f32);
        }
    }

    /// Get the current music volume.
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Get the name of the currently playing track, if any.
    pub fn current_track(&self) -> Option<&str> {
        self.current_track.as_deref()
    }

    /// Replace the playlist with custom track names.
    pub fn set_playlist(&mut self, tracks: Vec<String>) {
        self.playlist = tracks;
        self.playlist_index = 0;
    }

    fn resolve_track_name(&self, track_name: &str) -> String {
        self.aliases
            .get(&track_name.to_ascii_uppercase())
            .cloned()
            .unwrap_or_else(|| track_name.to_string())
    }

    fn ensure_theme_config(&mut self, assets: &AssetManager) {
        if !self.aliases.is_empty() {
            return;
        }

        let base = load_theme_ini(assets, "theme.ini");
        let md = load_theme_ini(assets, "thememd.ini");

        // Build aliases from both INIs (md values override base on conflict).
        if let Some(ref ini) = base {
            merge_theme_aliases(&mut self.aliases, ini);
        }
        if let Some(ref ini) = md {
            merge_theme_aliases(&mut self.aliases, ini);
        }

        // Merge playlists from both INIs — the original game plays RA2 and YR
        // tracks together. thememd.ini comments out RA2 entries, so we need
        // theme.ini to provide those tracks.
        let mut playlist = Vec::new();
        if let Some(ref ini) = base {
            playlist = playlist_from_theme_ini(ini, &self.aliases);
        }
        if let Some(ref ini) = md {
            for track in playlist_from_theme_ini(ini, &self.aliases) {
                if !playlist.iter().any(|t| t.eq_ignore_ascii_case(&track)) {
                    playlist.push(track);
                }
            }
        }

        if !playlist.is_empty() {
            self.playlist = playlist;
            self.playlist_index = 0;
        }

        // Resolve the menu [INTRO] theme (the shell loops this the whole time
        // the player is on the main menu). md overrides base on conflict.
        for ini in [base.as_ref(), md.as_ref()].into_iter().flatten() {
            if let Some((stem, repeats)) = menu_theme_from_ini(ini, &self.aliases) {
                self.menu_theme = Some(stem);
                self.menu_theme_repeats = repeats;
            }
        }
    }
}

/// Load a track and decode to interleaved f32 stereo samples.
/// Returns (samples, sample_rate) or None if not found / decode fails.
fn load_track(track_name: &str, assets: &AssetManager) -> Option<(Vec<f32>, u32)> {
    for filename in [format!("{}.wav", track_name), format!("{}.aud", track_name)] {
        let Some(data) = assets.get_ref(&filename) else {
            continue;
        };

        if data.len() >= 44 && &data[0..4] == b"RIFF" {
            if let Some(decoded) = decode_wav(data, &filename) {
                return Some((decoded.samples, decoded.sample_rate));
            }
        }

        let (header, samples) = match aud_file::decode_aud(data) {
            Some(decoded) => decoded,
            None => continue,
        };
        if samples.is_empty() {
            log::warn!("Track {} decoded to 0 samples", track_name);
            return None;
        }

        // Convert i16 PCM to interleaved f32 stereo.
        let stereo: Vec<f32> = if header.is_stereo() {
            samples.iter().map(|&s| s as f32 / 32768.0).collect()
        } else {
            samples
                .iter()
                .flat_map(|&s| {
                    let f = s as f32 / 32768.0;
                    [f, f]
                })
                .collect()
        };

        log::info!(
            "Decoded track {} from {}: {}Hz, {} channels, {} frames ({:.1}s)",
            track_name,
            filename,
            header.sample_rate,
            header.channels(),
            stereo.len() / 2,
            stereo.len() as f64 / 2.0 / header.sample_rate as f64,
        );

        return Some((stereo, header.sample_rate as u32));
    }

    None
}

/// Read `[Audio] ScoreVolume` from `{ra2_dir}/RA2MD.INI`, clamped to [0,1].
///
/// Returns None when the file, section, or key is absent or unparsable so the
/// caller can fall back to the engine default ([`DEFAULT_SCORE_VOLUME`]). This
/// is a loose user-settings INI in the install dir, not a MIX payload.
pub fn read_score_volume_from_ra2md(ra2_dir: &Path) -> Option<f64> {
    let bytes = std::fs::read(ra2_dir.join(RA2MD_INI_FILENAME)).ok()?;
    let ini = IniFile::from_bytes(&bytes).ok()?;
    score_volume_from_ini(&ini)
}

/// Extract `[Audio] ScoreVolume` from a parsed INI, clamped to [0,1].
fn score_volume_from_ini(ini: &IniFile) -> Option<f64> {
    let value = ini.section(AUDIO_SECTION)?.get_f32(SCORE_VOLUME_KEY)?;
    Some((value as f64).clamp(0.0, 1.0))
}

/// Format a score volume for `RA2MD.INI` exactly as the original does: six
/// decimal places (e.g. `0.600000`), clamped to [0,1].
fn format_score_volume(volume: f64) -> String {
    format!("{:.6}", volume.clamp(0.0, 1.0))
}

/// Persist `[Audio] ScoreVolume` into `{ra2_dir}/RA2MD.INI`, updating the key
/// in place and preserving every other key and section already in the file.
///
/// The original writes the user's settings on quit before tearing down; this
/// closes the read/write loop for the live music volume — the one setting the
/// engine currently both reads at boot ([`read_score_volume_from_ra2md`]) and
/// lets the player change at runtime. The `[Audio]` section (and the file
/// itself) is created when absent.
pub fn write_score_volume_to_ra2md(ra2_dir: &Path, volume: f64) -> std::io::Result<()> {
    let path = ra2_dir.join(RA2MD_INI_FILENAME);
    let existing = std::fs::read(&path).unwrap_or_default();
    let updated = crate::util::ini_writer::set_ini_value(
        &existing,
        AUDIO_SECTION,
        SCORE_VOLUME_KEY,
        &format_score_volume(volume),
    );
    std::fs::write(&path, updated)
}

fn load_theme_ini(assets: &AssetManager, name: &str) -> Option<IniFile> {
    let bytes = assets.get_ref(name)?;
    IniFile::from_bytes(bytes).ok()
}

fn merge_theme_aliases(into: &mut HashMap<String, String>, ini: &IniFile) {
    for section_name in ini.section_names() {
        let Some(section) = ini.section(section_name) else {
            continue;
        };
        let Some(sound) = section.get("Sound") else {
            continue;
        };
        if sound.is_empty() {
            continue;
        }

        let sound = sound.to_string();
        into.insert(section_name.to_ascii_uppercase(), sound.clone());
        into.insert(sound.to_ascii_uppercase(), sound);
    }
}

/// The section name of the main-menu shell theme in the theme INI.
const MENU_THEME_SECTION: &str = "INTRO";

/// Resolve the menu [INTRO] theme to (sound stem, repeats?) from a theme INI.
/// `repeats` reflects the [INTRO] `Repeat=` value (defaults to no if absent).
/// Returns None if the INI has no [INTRO] section with a resolvable sound.
fn menu_theme_from_ini(ini: &IniFile, aliases: &HashMap<String, String>) -> Option<(String, bool)> {
    let section = ini.section(MENU_THEME_SECTION)?;
    // Prefer the alias-resolved stem so casing matches what load_track uses.
    let stem = aliases
        .get(&MENU_THEME_SECTION.to_ascii_uppercase())
        .cloned()
        .or_else(|| {
            section
                .get("Sound")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        })?;
    let repeats = section.get_bool("Repeat").unwrap_or(false);
    Some((stem, repeats))
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
            // Skip non-Normal tracks (INTRO, SCORE, LOADING, CREDITS) —
            // they're menu/loading music, not gameplay playlist entries.
            // Normal defaults to yes if absent.
            if let Some(section) = ini.section(theme_name) {
                if section
                    .get("Normal")
                    .is_some_and(|v| v.eq_ignore_ascii_case("no"))
                {
                    return None;
                }
            }
            Some(sound.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors thememd.ini [INTRO]: Sound=Drok, Repeat=yes (the looping menu
    /// theme). Verifies we resolve the correct stem and honor Repeat=yes.
    #[test]
    fn menu_theme_resolves_intro_with_loop() {
        let ini =
            IniFile::from_str("[INTRO]\nName=THEME:Intro\nSound=Drok\nNormal=no\nRepeat=yes\n");
        let mut aliases = HashMap::new();
        merge_theme_aliases(&mut aliases, &ini);

        let (stem, repeats) =
            menu_theme_from_ini(&ini, &aliases).expect("INTRO theme must resolve");
        assert_eq!(stem, "Drok");
        assert!(repeats, "INTRO has Repeat=yes, must loop");
    }

    /// Repeat= absent defaults to no (non-looping), so we never force a loop on
    /// a theme that did not ask for one.
    #[test]
    fn menu_theme_repeat_defaults_to_no() {
        let ini = IniFile::from_str("[INTRO]\nSound=Drok\n");
        let aliases = HashMap::new();
        let (stem, repeats) =
            menu_theme_from_ini(&ini, &aliases).expect("INTRO theme must resolve");
        assert_eq!(stem, "Drok");
        assert!(!repeats);
    }

    /// No [INTRO] section -> no menu theme.
    #[test]
    fn menu_theme_absent_returns_none() {
        let ini = IniFile::from_str("[SCORE]\nSound=Score\n");
        let aliases = HashMap::new();
        assert!(menu_theme_from_ini(&ini, &aliases).is_none());
    }

    /// [Audio] ScoreVolume parses to its float value in range.
    #[test]
    fn score_volume_reads_audio_section() {
        let ini = IniFile::from_str("[Audio]\nScoreVolume=0.25\n");
        assert_eq!(score_volume_from_ini(&ini), Some(0.25));
    }

    /// A ScoreVolume above 1.0 clamps to 1.0 (matching the engine clamp).
    #[test]
    fn score_volume_clamps_above_one() {
        let ini = IniFile::from_str("[Audio]\nScoreVolume=1.5\n");
        assert_eq!(score_volume_from_ini(&ini), Some(1.0));
    }

    /// The persisted value matches the original's six-decimal format.
    #[test]
    fn score_volume_formats_six_decimals() {
        assert_eq!(format_score_volume(0.6), "0.600000");
        assert_eq!(format_score_volume(0.4), "0.400000");
    }

    /// Out-of-range volumes clamp to [0,1] before formatting.
    #[test]
    fn score_volume_format_clamps() {
        assert_eq!(format_score_volume(1.5), "1.000000");
        assert_eq!(format_score_volume(-0.2), "0.000000");
    }

    /// Missing section or key yields None so the caller uses the default.
    #[test]
    fn score_volume_missing_returns_none() {
        let ini = IniFile::from_str("[Options]\nFoo=bar\n");
        assert!(score_volume_from_ini(&ini).is_none());
        let empty = IniFile::from_str("[Audio]\nSoundVolume=0.7\n");
        assert!(score_volume_from_ini(&empty).is_none());
    }
}
