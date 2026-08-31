//! Sound effect (SFX) playback using rodio.
//!
//! Plays short one-shot sounds triggered by game events: weapon fire, unit
//! voice responses, building placement, death explosions. Uses the SoundRegistry
//! (from sound.ini) to resolve sound IDs to .wav/.aud filenames, then loads
//! and plays them through rodio.
//!
//! ## Design
//! - Fire-and-forget: each sound is played via a Player and tracked only to
//!   cap the max number of concurrent sounds (prevents audio overload).
//! - Random selection: when a sound entry has multiple .wav files, one is
//!   chosen at random for variety.
//! - Volume scaling: each sound's volume from sound.ini is applied on playback.
//!
//! ## Dependency rules
//! - Part of audio/ — depends on assets/ (AssetManager for .wav/.aud loading),
//!   rules/sound_ini (SoundRegistry for ID→filename mapping).
//! - Does NOT depend on render/, ui/, sidebar/, sim/.

use std::collections::{BTreeMap, VecDeque};
use std::num::NonZero;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

use crate::assets::asset_manager::AssetManager;
use crate::assets::aud_file;
use crate::rules::sound_ini::SoundRegistry;

/// Maximum concurrent SFX sounds — matches original engine's 16 DirectSound buffers.
/// RESIDUAL (GSI-15.03/15.04) — there is no channel pool, and the parts of
/// these two rows that matter for parity are entangled with the device.
/// Eviction is plain FIFO over this queue, so the `Priority=` tier decoded in
/// `rules/sound_ini.rs` is ignored and a `CRITICAL` cue loses to an older
/// `LOWEST` one; `Limit=` is unenforced (17 stock sounds cap at one instance);
/// there is no loop-handle mechanism, so `Control=` loop and ambient variants
/// cannot persist; interruption stops the old player outright with no fade; and
/// finished handles are reaped only inside `play_decoded`, so an idle frame
/// never cleans up.
///
/// Pass 2 established the cadence: `SoundSystem::UpdateTick @ 0x004041D0` is
/// pumped from `AudioSystem::Pump @ 0x00406F70` off the message/service loop —
/// NOT the sim tick — so the mixer's update rate is not frame-locked and must
/// not be modelled as if it were.
/// - Trigger: any moment more than a handful of sounds compete, and every
///   ambient or looping cue.
/// - Player effect: important cues get dropped for unimportant older ones,
///   ambient beds never sustain, and interruptions click instead of crossing.
/// - Frequency: continuous in any busy engagement.
/// - Downstream risk: **not reachable from `cargo test -p vera20k --lib`.**
///   `SfxPlayer::new` returns `None` without an audio device, so every path
///   below it is unverifiable here and none of it may be claimed verified. The
///   first slice is a device-free arbiter — a slot table, per-`SoundKey`
///   instance counts and lowest-priority-wins eviction with an age tie-break,
///   taking a request and returning admit-with-eviction or reject — with all
///   rodio work left in this file. That arbiter is testable from `--lib`, and it
///   is what `Limit=` and `Control=INTERRUPT` need before either can land.
const MAX_CONCURRENT_SFX: usize = 16;

/// Range multiplier — converts VocClass Range value (cells) to pixels.
/// In the original engine: `max_distance = Range * 0x3C` where 0x3C = 60.
const RANGE_MULTIPLIER: f32 = 60.0;

/// Default audible range in cells when sound has no explicit Range.
/// The original engine's [Defaults] section uses Range=10.
pub const DEFAULT_RANGE_CELLS: u16 = 10;

/// Minimum volume cutoff — sounds below this are culled entirely (not played).
/// Matches original engine behavior (approximately 5%).
const MIN_VOLUME_CUTOFF: f32 = 0.05;

/// Calculate spatial audio volume based on screen distance from viewport center.
///
/// Algorithm:
/// 1. Compute screen-space distance from viewport center
/// 2. Subtract half viewport (on-screen = full volume)
/// 3. Double Y for isometric compensation
/// 4. Linear falloff from 1.0 at viewport edge to 0.0 at max range
///
/// `range_cells` — audible range from sound.ini Range= key (default 10).
/// `min_volume_pct` — MinVolume= floor (0-100), volume never drops below this.
/// RESIDUAL (GSI-15.02) — pass 2 transcribed the native function, so these are
/// now three DRIFTs against a known target rather than three approximations.
/// `VocClass::CalcVolumeAndPan @ 0x00750AC0` computes
/// `volume = (maxRange - max(distX, 2 * distY)) / maxRange` with
/// `maxRange = Range * 60`, both distances truncated by `ftol` before `abs`,
/// measured against the TACTICAL VIEW rect (`0x00886FA8`/`0x00886FAC`) — not the
/// window. The `max(dx, 2 * dy)` metric below is therefore structurally right.
/// The differences:
/// - **The `MinVolume=` floor is unconditional here.** Native applies it only
///   for `Type=GLOBAL` (flag `0x10`, 52 stock entries; `[Defaults]` is not
///   GLOBAL), and skips the half-viewport subtraction only for `Type=LOCAL`
///   (flag `0x40`). With stock `[Defaults] MinVolume=50` VERA puts a 50% floor
///   under every registry-resolved sound, so the audibility cutoff is
///   unreachable and distant sounds hold at half volume.
/// - **There is no pan.** Native returns
///   `ftol(clamp(offsetX, +/-fullW) * 8192 / fullW + 8192)`, i.e. `0..16384`
///   with 8192 centre, and it is NOT negated — an existing research note reading
///   an `FCHS` as a pan negation is wrong; that instruction negates the width.
/// - **The listener rect may be the window rather than the tactical view.**
/// - Trigger: every positional sound.
/// - Player effect: no stereo image, and distant sounds that should fade out
///   hold at half volume.
/// - Frequency: continuous.
/// - Downstream risk: the floor and the metric are landable without the
///   arbiter, once `Type=` is parsed (see `rules/sound_ini.rs`); only wiring pan
///   into a stereo gain pair needs the device path, which `--lib` cannot reach.
pub fn calc_spatial_volume(
    sound_screen_x: f32,
    sound_screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    camera_x: f32,
    camera_y: f32,
    range_cells: u16,
    min_volume_pct: u8,
) -> f32 {
    let center_x = camera_x + viewport_w * 0.5;
    let center_y = camera_y + viewport_h * 0.5;

    // Absolute distance from screen center.
    let mut dx = (sound_screen_x - center_x).abs();
    let mut dy = (sound_screen_y - center_y).abs();

    // Subtract half viewport — sounds on screen have zero distance.
    dx = (dx - viewport_w * 0.5).max(0.0);
    dy = (dy - viewport_h * 0.5).max(0.0);

    // Double Y for isometric compensation (Y axis is visually compressed).
    dy *= 2.0;

    // Use the larger axis as effective distance.
    let dist = dx.max(dy);

    // Max audible distance = Range (cells) * 60 (pixels per cell equivalent).
    let max_range = range_cells.max(1) as f32 * RANGE_MULTIPLIER;
    if dist >= max_range {
        return 0.0;
    }

    let mut vol = (max_range - dist) / max_range;

    // Apply MinVolume floor — volume never drops below this.
    let min_vol = min_volume_pct as f32 / 100.0;
    if vol < min_vol {
        vol = min_vol;
    }

    if vol < MIN_VOLUME_CUTOFF { 0.0 } else { vol }
}

const FADE_MS: u32 = 3;

/// Decoded audio ready for rodio playback.
/// Holds interleaved f32 stereo samples, sample rate, and channel count.
pub(crate) struct DecodedAudio {
    /// Interleaved stereo f32 samples (L, R, L, R, ...).
    pub(crate) samples: Vec<f32>,
    pub(crate) sample_rate: u32,
    /// Always 2 (stereo) — we upmix mono sources for consistency.
    pub(crate) channels: u16,
}

struct QueuedVoice {
    sound_id: String,
    decoded: DecodedAudio,
    /// Sound-entry gain only. The current Voice master is applied when this
    /// cue reaches the dedicated slot, not when it enters the queue.
    base_gain: f32,
}

/// User-controlled master channel for one secondary output.
///
/// gamemd-derived: `OptionsClass::SetDefaults @ 0x005FA350` and
/// `OptionsClass::Read @ 0x005FA620` retain independent SoundVolume and
/// VoiceVolume settings; ordinary/animation effects use Sound while unit and
/// EVA speech use Voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SfxChannel {
    Sound,
    Voice,
}

/// Master-independent gain retained beside one live secondary output.
///
/// The base gain includes sound-entry and spatial factors, but not the user
/// master. User, lifecycle, and foreground factors are recomposed from it so
/// changing or reopening a gate never has to infer the original value from a
/// muted Player.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SfxOutputGain {
    base_gain: f32,
    channel: SfxChannel,
}

impl SfxOutputGain {
    fn new(base_gain: f32, channel: SfxChannel) -> Self {
        Self { base_gain, channel }
    }

    fn effective(
        self,
        sound_volume: f32,
        voice_volume: f32,
        lifecycle_scale: f32,
        focus_output_scale: f32,
    ) -> f32 {
        let master = match self.channel {
            SfxChannel::Sound => sound_volume,
            SfxChannel::Voice => voice_volume,
        };
        self.base_gain * master * lifecycle_scale * focus_output_scale
    }
}

struct LiveSfxOutput {
    player: Player,
    gain: SfxOutputGain,
}

impl LiveSfxOutput {
    fn new(player: Player, base_gain: f32, channel: SfxChannel) -> Self {
        Self {
            player,
            gain: SfxOutputGain::new(base_gain, channel),
        }
    }

    fn apply_scales(
        &self,
        sound_volume: f32,
        voice_volume: f32,
        lifecycle_scale: f32,
        focus_output_scale: f32,
    ) {
        self.player.set_volume(self.gain.effective(
            sound_volume,
            voice_volume,
            lifecycle_scale,
            focus_output_scale,
        ));
    }
}

/// Manages sound effect playback with separate SFX pool and voice slot.
///
/// Matches the original engine's architecture:
/// - 16-channel SFX pool for weapons, explosions, ambient
/// - 1 dedicated voice slot for unit responses (cuts off previous)
pub struct SfxPlayer {
    /// rodio mixer device sink — must be kept alive or all audio stops.
    _device: MixerDeviceSink,
    /// Active SFX players — oldest first. Capped at MAX_CONCURRENT_SFX.
    active: VecDeque<LiveSfxOutput>,
    /// Active sound handle owned by each authoritative animation ID.
    animation_active: BTreeMap<u64, LiveSfxOutput>,
    /// Dedicated voice player — unit responses cut off the previous voice.
    /// Separate from SFX pool so voices never compete with weapon sounds.
    voice_player: Option<LiveSfxOutput>,
    /// Queued EVA/voice announcements waiting for the dedicated voice slot.
    queued_voice: VecDeque<QueuedVoice>,
    /// Sound id currently occupying the dedicated voice slot, when known.
    current_voice_id: Option<String>,
    /// Ordinary and animation SFX master volume (0.0 to 1.0).
    sound_volume: f64,
    /// Unit and EVA voice master volume (0.0 to 1.0).
    voice_volume: f64,
    /// Temporary app-lifecycle multiplier over every live SFX/voice output.
    output_scale: f32,
    /// Foreground-owned primary-output gate. Secondary Players stay running so
    /// their playback cursors continue while global output is suppressed.
    focus_output_scale: f32,
    /// Simple counter used as seed for pseudo-random sound selection.
    /// Not cryptographic — just needs variety.
    random_counter: u32,
}

impl SfxPlayer {
    /// Create a new SfxPlayer. Returns None if audio output cannot be opened.
    pub fn new() -> Option<Self> {
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| log::error!("Failed to initialize SFX audio: {}", e))
            .ok()?;

        Some(Self {
            _device: device,
            active: VecDeque::new(),
            animation_active: BTreeMap::new(),
            voice_player: None,
            queued_voice: VecDeque::new(),
            current_voice_id: None,
            sound_volume: 0.7,
            voice_volume: 0.7,
            output_scale: 1.0,
            focus_output_scale: 1.0,
            random_counter: 0,
        })
    }

    /// Play a sound by its sound.ini ID (e.g., "VGCannon1") or audio.bag name.
    ///
    /// Resolution order:
    /// 1. Look up `sound_id` in the SoundRegistry (sound.ini sections)
    /// 2. If found, pick a filename and load via audio bags then MIX assets
    /// 3. If NOT found in registry, try `sound_id` directly as an audio.bag name
    ///    (for EVA sounds and other bag-only entries)
    ///
    /// Returns true if the sound was successfully started.
    pub fn play_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        // Try SoundRegistry first (sound.ini-based sounds).
        if let Some(entry) = registry.get(sound_id) {
            if !entry.sounds.is_empty() {
                self.random_counter = self.random_counter.wrapping_add(1);
                let idx: usize = (self.random_counter as usize) % entry.sounds.len();
                let filename: &str = &entry.sounds[idx];

                if let Some(decoded) = load_sfx(filename, assets, audio_indices) {
                    let base_gain = entry.volume as f32 / 100.0;
                    return self.play_decoded(decoded, base_gain);
                }
            }
        }

        // Fallback: try sound_id directly as an audio.bag entry name
        // (EVA announcements and other sounds not in sound.ini).
        if let Some(decoded) = load_sfx(sound_id, assets, audio_indices) {
            return self.play_decoded(decoded, 1.0);
        }

        log::trace!("SFX: could not resolve '{}'", sound_id);
        false
    }

    /// Play a sound with an additional spatial volume multiplier.
    ///
    /// Used for positional sounds where volume is scaled by distance from camera.
    /// The spatial factor (0.0–1.0) is multiplied with the per-sound and master volumes.
    pub fn play_sound_with_volume(
        &mut self,
        sound_id: &str,
        spatial_volume: f32,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        if let Some(entry) = registry.get(sound_id) {
            if !entry.sounds.is_empty() {
                self.random_counter = self.random_counter.wrapping_add(1);
                let idx = (self.random_counter as usize) % entry.sounds.len();
                let filename = &entry.sounds[idx];

                if let Some(decoded) = load_sfx(filename, assets, audio_indices) {
                    let base_gain = entry.volume as f32 / 100.0 * spatial_volume;
                    return self.play_decoded(decoded, base_gain);
                }
            }
        }

        if let Some(decoded) = load_sfx(sound_id, assets, audio_indices) {
            return self.play_decoded(decoded, spatial_volume);
        }

        false
    }

    /// Play only a named `sound(md).ini` event and never reinterpret its ID as
    /// an audio-bag filename. `RulesClass::ReadAudioVisual @ 0x006691E0`
    /// resolves `[AudioVisual] CloakSound` through `VocClass::FindByName @
    /// 0x007514D0`; a failed lookup preserves the invalid constructor index,
    /// so `StartUncloaking @ 0x007036C0` produces no audible fallback.
    pub fn play_registered_sound_with_volume(
        &mut self,
        sound_id: &str,
        spatial_volume: f32,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        let Some((filename, entry_volume)) =
            select_registered_sound(sound_id, registry, &mut self.random_counter)
        else {
            return false;
        };
        let Some(decoded) = load_sfx(filename, assets, audio_indices) else {
            return false;
        };
        let base_gain = entry_volume as f32 * spatial_volume;
        self.play_decoded(decoded, base_gain)
    }

    /// Start or replace the sound owned by one animation object.
    pub fn play_animation_sound_with_volume(
        &mut self,
        anim_id: u64,
        sound_id: &str,
        spatial_volume: f32,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.stop_animation_sound(anim_id);
        let decoded_and_gain = if let Some(entry) = registry.get(sound_id) {
            if entry.sounds.is_empty() {
                None
            } else {
                self.random_counter = self.random_counter.wrapping_add(1);
                let filename = &entry.sounds[(self.random_counter as usize) % entry.sounds.len()];
                load_sfx(filename, assets, audio_indices).map(|decoded| {
                    let base_gain = entry.volume as f32 / 100.0 * spatial_volume;
                    (decoded, base_gain)
                })
            }
        } else {
            load_sfx(sound_id, assets, audio_indices).map(|decoded| (decoded, spatial_volume))
        };
        let Some((decoded, base_gain)) = decoded_and_gain else {
            return false;
        };
        let Some(channels) = NonZero::new(decoded.channels) else {
            return false;
        };
        let Some(sample_rate) = NonZero::new(decoded.sample_rate) else {
            return false;
        };
        let source = SamplesBuffer::new(channels, sample_rate, decoded.samples);
        let player = Player::connect_new(self._device.mixer());
        let output = LiveSfxOutput::new(player, base_gain, SfxChannel::Sound);
        output.apply_scales(
            self.sound_volume as f32,
            self.voice_volume as f32,
            self.output_scale,
            self.focus_output_scale,
        );
        output.player.append(source);
        self.animation_active.insert(anim_id, output);
        true
    }

    /// Release only the handle owned by `anim_id`. Idempotent.
    pub fn stop_animation_sound(&mut self, anim_id: u64) {
        if let Some(output) = self.animation_active.remove(&anim_id) {
            output.player.stop();
        }
    }

    /// Play a sound as a unit voice response (VoiceSelect, VoiceMove, VoiceAttack).
    ///
    /// Uses a dedicated voice slot that cuts off the previous voice — unit responses
    /// don't stack, matching the original engine's behavior.
    pub fn play_voice_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.advance_voice_queue();

        // Resolve through registry first, then fallback to bag name.
        let (decoded, entry_volume) = if let Some(entry) = registry.get(sound_id) {
            if entry.sounds.is_empty() {
                return false;
            }
            self.random_counter = self.random_counter.wrapping_add(1);
            let idx = (self.random_counter as usize) % entry.sounds.len();
            let filename = &entry.sounds[idx];
            match load_sfx(filename, assets, audio_indices) {
                Some(d) => (d, entry.volume as f64 / 100.0),
                None => return false,
            }
        } else {
            match load_sfx(sound_id, assets, audio_indices) {
                Some(d) => (d, 1.0),
                None => return false,
            }
        };

        self.play_voice(decoded, entry_volume as f32, Some(sound_id.to_string()))
    }

    /// Queue an EVA-style announcement without interrupting the current voice.
    ///
    /// This is the narrow app-facing bridge for evamd.ini `Type=QUEUE` cues.
    /// Full native priority tiers and inter-announcement delay remain a later
    /// VoxClass parity surface.
    pub fn queue_eva_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.advance_voice_queue();

        if self.current_voice_id.as_deref() == Some(sound_id)
            || self
                .queued_voice
                .iter()
                .any(|queued| queued.sound_id == sound_id)
        {
            return true;
        }

        let (decoded, entry_volume) =
            match self.resolve_voice_audio(sound_id, registry, assets, audio_indices) {
                Some(resolved) => resolved,
                None => return false,
            };
        self.queued_voice.push_back(QueuedVoice {
            sound_id: sound_id.to_string(),
            decoded,
            base_gain: entry_volume as f32,
        });
        self.advance_voice_queue();
        true
    }

    /// Play a STANDARD EVA cue only if the voice system is currently idle.
    ///
    /// Native STANDARD entries are fire-and-forget; when voice playback or a
    /// queued announcement is active they are not retained for later playback.
    pub fn play_standard_eva_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.advance_voice_queue();
        if self
            .voice_player
            .as_ref()
            .is_some_and(|output| !output.player.empty())
            || !self.queued_voice.is_empty()
        {
            return false;
        }

        let (decoded, entry_volume) =
            match self.resolve_voice_audio(sound_id, registry, assets, audio_indices) {
                Some(resolved) => resolved,
                None => return false,
            };
        self.play_voice(decoded, entry_volume as f32, Some(sound_id.to_string()))
    }

    /// Replace only the dedicated EVA/voice channel with an INTERRUPT cue.
    ///
    /// gamemd `VoxClass__QueueVoice @ 0x00752480`, type 2, discards queued
    /// voice nodes and stops the current voice before starting the new cue.
    /// Ordinary and animation SFX are deliberately untouched.
    pub fn interrupt_eva_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.queued_voice.clear();
        if let Some(output) = self.voice_player.take() {
            output.player.stop();
        }
        self.current_voice_id = None;

        let (decoded, entry_volume) =
            match self.resolve_voice_audio(sound_id, registry, assets, audio_indices) {
                Some(resolved) => resolved,
                None => return false,
            };
        self.play_voice(decoded, entry_volume as f32, Some(sound_id.to_string()))
    }

    fn resolve_voice_audio(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> Option<(DecodedAudio, f64)> {
        if let Some(entry) = registry.get(sound_id) {
            if entry.sounds.is_empty() {
                return None;
            }
            self.random_counter = self.random_counter.wrapping_add(1);
            let idx = (self.random_counter as usize) % entry.sounds.len();
            let filename = &entry.sounds[idx];
            return load_sfx(filename, assets, audio_indices)
                .map(|decoded| (decoded, entry.volume as f64 / 100.0));
        }

        load_sfx(sound_id, assets, audio_indices).map(|decoded| (decoded, 1.0))
    }

    /// Starts the next queued EVA cue if the dedicated voice slot is idle.
    pub fn advance_voice_queue(&mut self) {
        if self
            .voice_player
            .as_ref()
            .is_some_and(|output| !output.player.empty())
        {
            return;
        }
        self.voice_player = None;
        self.current_voice_id = None;

        let Some(queued) = self.queued_voice.pop_front() else {
            return;
        };
        self.play_voice(queued.decoded, queued.base_gain, Some(queued.sound_id));
    }

    /// Play decoded audio on the dedicated voice slot, cutting off any current voice.
    fn play_voice(
        &mut self,
        mut decoded: DecodedAudio,
        base_gain: f32,
        sound_id: Option<String>,
    ) -> bool {
        // Cut off previous voice immediately.
        if let Some(old) = self.voice_player.take() {
            old.player.stop();
        }
        self.current_voice_id = None;

        apply_fade(
            &mut decoded.samples,
            decoded.sample_rate,
            decoded.channels,
            FADE_MS,
        );

        let channels = match NonZero::new(decoded.channels) {
            Some(c) => c,
            None => return false,
        };
        let sample_rate = match NonZero::new(decoded.sample_rate) {
            Some(r) => r,
            None => return false,
        };

        let source = SamplesBuffer::new(channels, sample_rate, decoded.samples);
        let player: Player = Player::connect_new(self._device.mixer());
        let output = LiveSfxOutput::new(player, base_gain, SfxChannel::Voice);
        output.apply_scales(
            self.sound_volume as f32,
            self.voice_volume as f32,
            self.output_scale,
            self.focus_output_scale,
        );
        output.player.append(source);
        self.voice_player = Some(output);
        self.current_voice_id = sound_id;
        true
    }

    /// Play already-decoded audio on the SFX pool at the given master-independent gain.
    fn play_decoded(&mut self, mut decoded: DecodedAudio, base_gain: f32) -> bool {
        apply_fade(
            &mut decoded.samples,
            decoded.sample_rate,
            decoded.channels,
            FADE_MS,
        );

        // Evict finished sounds and enforce concurrency limit.
        self.cleanup_finished();
        if self.active.len() >= MAX_CONCURRENT_SFX {
            // Stop and evict oldest sound.
            if let Some(old) = self.active.pop_front() {
                old.player.stop();
            }
        }

        // NonZero is required by rodio 0.22 SamplesBuffer API.
        let channels = match NonZero::new(decoded.channels) {
            Some(c) => c,
            None => return false,
        };
        let sample_rate = match NonZero::new(decoded.sample_rate) {
            Some(r) => r,
            None => return false,
        };

        let source = SamplesBuffer::new(channels, sample_rate, decoded.samples);
        let player: Player = Player::connect_new(self._device.mixer());
        let output = LiveSfxOutput::new(player, base_gain, SfxChannel::Sound);
        output.apply_scales(
            self.sound_volume as f32,
            self.voice_volume as f32,
            self.output_scale,
            self.focus_output_scale,
        );
        output.player.append(source);
        self.active.push_back(output);
        true
    }

    /// Remove handles for sounds that have finished playing.
    fn cleanup_finished(&mut self) {
        self.active.retain(|output| !output.player.empty());
        self.animation_active
            .retain(|_, output| !output.player.empty());
        self.advance_voice_queue();
    }

    /// Compatibility setter: apply one master to both SFX and voice channels.
    pub fn set_volume(&mut self, volume: f64) {
        let volume = volume.clamp(0.0, 1.0);
        self.sound_volume = volume;
        self.voice_volume = volume;
        self.apply_live_output_scales();
    }

    /// Set the ordinary and animation SFX master volume.
    pub fn set_sound_volume(&mut self, volume: f64) {
        self.sound_volume = volume.clamp(0.0, 1.0);
        self.apply_live_output_scales();
    }

    /// Set the unit and EVA voice master volume.
    pub fn set_voice_volume(&mut self, volume: f64) {
        self.voice_volume = volume.clamp(0.0, 1.0);
        self.apply_live_output_scales();
    }

    /// Apply a temporary multiplier to all live outputs without changing the
    /// saved SFX setting or foreground gate.
    pub fn set_output_scale(&mut self, scale: f64) {
        self.output_scale = scale.clamp(0.0, 1.0) as f32;
        self.apply_live_output_scales();
    }

    /// Gate global SFX/voice output on the application-activation edge.
    ///
    /// gamemd-derived: the active `WM_ACTIVATEAPP` changed edge at `0x007778AC`
    /// reaches primary-buffer Stop through `FUN_00407020 @
    /// 0x00407020` -> `FUN_0040A940 @ 0x0040A940`, and primary-buffer restore /
    /// looping Play through `FUN_00407040 @ 0x00407040` -> `FUN_0040A950 @
    /// 0x0040A950`. It does not pause the secondary buffers modelled here.
    pub fn set_focus_output_active(&mut self, active: bool) {
        self.focus_output_scale = if active { 1.0 } else { 0.0 };
        self.apply_live_output_scales();
    }

    fn apply_live_output_scales(&self) {
        for output in &self.active {
            output.apply_scales(
                self.sound_volume as f32,
                self.voice_volume as f32,
                self.output_scale,
                self.focus_output_scale,
            );
        }
        for output in self.animation_active.values() {
            output.apply_scales(
                self.sound_volume as f32,
                self.voice_volume as f32,
                self.output_scale,
                self.focus_output_scale,
            );
        }
        if let Some(output) = self.voice_player.as_ref() {
            output.apply_scales(
                self.sound_volume as f32,
                self.voice_volume as f32,
                self.output_scale,
                self.focus_output_scale,
            );
        }
    }

    /// Pump the dedicated voice queue once and report whether any voice work
    /// remains, mirroring the poll performed inside native exit wait loops.
    pub fn pump_and_check_voices(&mut self) -> bool {
        self.advance_voice_queue();
        self.voices_active()
    }

    /// Hard-stop every SFX/voice source and discard queued announcements.
    pub fn stop_all(&mut self) {
        for output in self.active.drain(..) {
            output.player.stop();
        }
        for (_, output) in std::mem::take(&mut self.animation_active) {
            output.player.stop();
        }
        if let Some(output) = self.voice_player.take() {
            output.player.stop();
        }
        self.queued_voice.clear();
        self.current_voice_id = None;
    }

    /// Get the current SFX master volume.
    pub fn volume(&self) -> f64 {
        self.sound_volume
    }

    /// Get the current unit and EVA voice master volume.
    pub fn voice_volume(&self) -> f64 {
        self.voice_volume
    }

    /// Number of currently active (playing) sound effects.
    pub fn active_count(&self) -> usize {
        self.active.len() + self.animation_active.len()
    }

    /// Number of queued EVA/voice announcements waiting on the voice slot.
    pub fn queued_voice_count(&self) -> usize {
        self.queued_voice.len()
    }

    /// Whether any EVA/voice line is still playing or waiting in the voice queue.
    /// Non-blocking (rodio `Player::empty()` is a poll). Used by the quit cascade
    /// to wait for trailing voices before tearing down.
    pub fn voices_active(&self) -> bool {
        self.voice_player
            .as_ref()
            .is_some_and(|output| !output.player.empty())
            || !self.queued_voice.is_empty()
    }
}

fn select_registered_sound<'a>(
    sound_id: &str,
    registry: &'a SoundRegistry,
    random_counter: &mut u32,
) -> Option<(&'a str, f64)> {
    let entry = registry.get(sound_id)?;
    if entry.sounds.is_empty() {
        return None;
    }
    *random_counter = random_counter.wrapping_add(1);
    let filename = &entry.sounds[(*random_counter as usize) % entry.sounds.len()];
    Some((filename, entry.volume as f64 / 100.0))
}

/// Load a sound effect file and decode it to interleaved f32 stereo samples.
///
/// Resolution order:
/// 1. Try audio.bag indices (most voice/EVA sounds live here)
/// 2. Try MIX asset lookup by exact name
/// 3. Try MIX asset lookup with .wav extension appended
///
/// Supports .wav (raw PCM), .aud (IMA ADPCM), and audio.bag formats.
fn load_sfx(
    filename: &str,
    assets: &AssetManager,
    audio_indices: &[crate::assets::audio_bag::AudioIndex],
) -> Option<DecodedAudio> {
    // Try audio.bag indices first (voices, EVA announcements).
    for index in audio_indices {
        if let Some((entry, data)) = index.get(filename) {
            if let Some(bag_audio) = crate::assets::audio_bag::decode_bag_audio(entry, data) {
                // Convert i16 → f32 stereo.
                let stereo = upmix_i16_to_f32_stereo(&bag_audio.samples_i16, bag_audio.channels);
                return Some(DecodedAudio {
                    samples: stereo,
                    sample_rate: bag_audio.sample_rate,
                    channels: 2,
                });
            }
        }
    }

    // Try MIX asset lookup (exact name, then with .wav extension).
    let exact_name = format!("{}.wav", filename);
    let data: &[u8] = assets
        .get_ref(filename)
        .or_else(|| assets.get_ref(&exact_name))?;

    // Try WAV first (most SFX are .wav).
    if data.len() >= 44 && &data[0..4] == b"RIFF" {
        return decode_wav(data, filename);
    }

    // Fall back to .aud format.
    let (header, samples) = aud_file::decode_aud(data)?;
    if samples.is_empty() {
        return None;
    }

    // AUD is always mono — upmix to stereo for rodio.
    let stereo = upmix_i16_to_f32_stereo(&samples, 1);
    Some(DecodedAudio {
        samples: stereo,
        sample_rate: header.sample_rate as u32,
        channels: 2,
    })
}

/// Convert i16 PCM samples to interleaved f32 stereo.
/// Mono input is duplicated to both channels.
fn upmix_i16_to_f32_stereo(samples: &[i16], channels: u16) -> Vec<f32> {
    if channels >= 2 {
        // Already stereo (or more) — just convert to f32.
        samples.iter().map(|&s| s as f32 / 32768.0).collect()
    } else {
        // Mono → stereo: duplicate each sample.
        samples
            .iter()
            .flat_map(|&s| {
                let f = s as f32 / 32768.0;
                [f, f]
            })
            .collect()
    }
}

/// Apply a short linear fade-in and fade-out to interleaved samples.
///
/// Prevents audible click/pop artifacts from abrupt sample transitions.
/// The fade duration is typically 2-5ms — imperceptible but eliminates clicks.
fn apply_fade(samples: &mut [f32], sample_rate: u32, channels: u16, fade_ms: u32) {
    if samples.is_empty() || fade_ms == 0 || sample_rate == 0 {
        return;
    }
    let ch = channels.max(1) as usize;
    // Number of *frames* to fade (one frame = all channels).
    let fade_frames = (sample_rate as usize * fade_ms as usize / 1000).max(1);
    let total_frames = samples.len() / ch;
    // Don't fade if the sound is shorter than 2× fade duration.
    if total_frames < fade_frames * 2 {
        return;
    }

    // Fade in: ramp from 0.0 to 1.0 over the first fade_frames.
    for frame in 0..fade_frames {
        let scale = frame as f32 / fade_frames as f32;
        for c in 0..ch {
            samples[frame * ch + c] *= scale;
        }
    }

    // Fade out: ramp from 1.0 to 0.0 over the last fade_frames.
    let fade_out_start = total_frames - fade_frames;
    for frame in 0..fade_frames {
        let scale = 1.0 - (frame as f32 / fade_frames as f32);
        let idx = (fade_out_start + frame) * ch;
        for c in 0..ch {
            samples[idx + c] *= scale;
        }
    }
}

/// Decode a WAV file into interleaved f32 stereo samples.
///
/// Supports uncompressed PCM (format tag 1) with 8-bit or 16-bit samples,
/// and IMA ADPCM (format tag 0x11) used by RA2 EVA announcements.
/// Mono or stereo. This covers all RA2 sound effects and EVA voices.
pub(crate) fn decode_wav(data: &[u8], filename: &str) -> Option<DecodedAudio> {
    if data.len() < 44 {
        return None;
    }

    // Verify RIFF/WAVE header.
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        log::trace!("WAV: invalid header for {}", filename);
        return None;
    }

    // Find "fmt " and "data" chunks.
    let mut offset: usize = 12;
    let mut fmt_found: bool = false;
    let mut channels: u16 = 1;
    let mut sample_rate: u32 = 22050;
    let mut bits_per_sample: u16 = 16;
    let mut format_tag: u16 = 1;
    let mut block_align: u16 = 0;

    while offset + 8 <= data.len() {
        let chunk_id: &[u8] = &data[offset..offset + 4];
        let chunk_size: u32 = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        if chunk_id == b"fmt " && offset + 8 + chunk_size as usize <= data.len() {
            let fmt: &[u8] = &data[offset + 8..];
            format_tag = u16::from_le_bytes([fmt[0], fmt[1]]);
            channels = u16::from_le_bytes([fmt[2], fmt[3]]);
            sample_rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
            block_align = u16::from_le_bytes([fmt[12], fmt[13]]);
            bits_per_sample = u16::from_le_bytes([fmt[14], fmt[15]]);
            fmt_found = true;
        }

        if chunk_id == b"data" && fmt_found {
            let pcm_start: usize = offset + 8;
            let pcm_end: usize = (pcm_start + chunk_size as usize).min(data.len());
            let pcm: &[u8] = &data[pcm_start..pcm_end];

            let samples: Vec<f32> = match format_tag {
                1 => decode_pcm(pcm, channels, bits_per_sample),
                0x11 => decode_ima_adpcm(pcm, channels, block_align),
                _ => {
                    log::trace!(
                        "WAV: unsupported format tag {} for {}",
                        format_tag,
                        filename
                    );
                    return None;
                }
            };
            if samples.is_empty() {
                return None;
            }

            // Always output stereo — upmix mono if needed.
            let stereo: Vec<f32> = if channels == 1 {
                samples.iter().flat_map(|&s| [s, s]).collect()
            } else {
                samples
            };

            return Some(DecodedAudio {
                samples: stereo,
                sample_rate,
                channels: 2,
            });
        }

        // Advance to next chunk (chunks are word-aligned).
        offset += 8 + ((chunk_size as usize + 1) & !1);
    }

    log::trace!("WAV: no data chunk found for {}", filename);
    None
}

/// IMA ADPCM step size table — standard IMA/DVI specification.
const IMA_STEP_TABLE: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// IMA ADPCM index adjustment table for each nibble value.
const IMA_INDEX_TABLE: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// Decode IMA ADPCM WAV data into interleaved f32 samples.
///
/// Each block starts with a 4-byte header per channel: i16 predictor + u8 step index + u8 pad.
/// Remaining bytes contain packed 4-bit nibbles decoded with the standard IMA algorithm.
fn decode_ima_adpcm(data: &[u8], channels: u16, block_align: u16) -> Vec<f32> {
    let ch = channels as usize;
    let block_size = block_align as usize;
    if block_size == 0 || ch == 0 {
        return Vec::new();
    }
    let header_size = 4 * ch;
    if block_size < header_size {
        return Vec::new();
    }

    // Samples per block: header gives 1 sample per channel, then 2 nibbles per byte.
    let data_bytes_per_block = block_size - header_size;
    let samples_per_block = 1 + data_bytes_per_block * 2 / ch;
    let num_blocks = (data.len() + block_size - 1) / block_size;
    let mut output: Vec<f32> = Vec::with_capacity(num_blocks * samples_per_block * ch);

    for block in data.chunks(block_size) {
        if block.len() < header_size {
            break;
        }

        // Read per-channel header: initial predictor and step index.
        let mut predictor = [0i32; 2];
        let mut step_index = [0i32; 2];
        for c in 0..ch {
            let base = c * 4;
            predictor[c] = i16::from_le_bytes([block[base], block[base + 1]]) as i32;
            step_index[c] = block[base + 2] as i32;
            step_index[c] = step_index[c].clamp(0, 88);
            // First sample from header.
            if ch == 1 {
                output.push(predictor[c] as f32 / 32768.0);
            }
        }
        // For stereo, interleave the initial samples.
        if ch == 2 {
            output.push(predictor[0] as f32 / 32768.0);
            output.push(predictor[1] as f32 / 32768.0);
        }

        // Decode nibbles from the data portion.
        let payload = &block[header_size..];
        if ch == 1 {
            // Mono: straightforward sequential nibbles.
            for &byte in payload {
                for shift in [0u8, 4] {
                    let nibble = ((byte >> shift) & 0x0F) as usize;
                    let step = IMA_STEP_TABLE[step_index[0] as usize];
                    let mut diff = step >> 3;
                    if nibble & 4 != 0 {
                        diff += step;
                    }
                    if nibble & 2 != 0 {
                        diff += step >> 1;
                    }
                    if nibble & 1 != 0 {
                        diff += step >> 2;
                    }
                    if nibble & 8 != 0 {
                        predictor[0] -= diff;
                    } else {
                        predictor[0] += diff;
                    }
                    predictor[0] = predictor[0].clamp(-32768, 32767);
                    step_index[0] += IMA_INDEX_TABLE[nibble];
                    step_index[0] = step_index[0].clamp(0, 88);
                    output.push(predictor[0] as f32 / 32768.0);
                }
            }
        } else {
            // Stereo: nibbles are interleaved in 8-nibble (4-byte) chunks per channel.
            // Layout: 4 bytes for ch0 (8 nibbles), 4 bytes for ch1 (8 nibbles), repeat.
            let mut samples_buf: Vec<[f32; 2]> = Vec::new();
            let mut pos = 0;
            while pos + 8 <= payload.len() {
                for c in 0..2 {
                    for b in 0..4 {
                        let byte = payload[pos + c * 4 + b];
                        for shift in [0u8, 4] {
                            let nibble = ((byte >> shift) & 0x0F) as usize;
                            let step = IMA_STEP_TABLE[step_index[c] as usize];
                            let mut diff = step >> 3;
                            if nibble & 4 != 0 {
                                diff += step;
                            }
                            if nibble & 2 != 0 {
                                diff += step >> 1;
                            }
                            if nibble & 1 != 0 {
                                diff += step >> 2;
                            }
                            if nibble & 8 != 0 {
                                predictor[c] -= diff;
                            } else {
                                predictor[c] += diff;
                            }
                            predictor[c] = predictor[c].clamp(-32768, 32767);
                            step_index[c] += IMA_INDEX_TABLE[nibble];
                            step_index[c] = step_index[c].clamp(0, 88);
                            let sample = predictor[c] as f32 / 32768.0;
                            let sample_idx = b * 2 + shift as usize / 4;
                            if c == 0 {
                                samples_buf.push([sample, 0.0]);
                            } else if sample_idx < samples_buf.len() {
                                let last = samples_buf.len();
                                samples_buf[last - 8 + sample_idx][1] = sample;
                            }
                        }
                    }
                }
                pos += 8;
                for pair in samples_buf.drain(..) {
                    output.push(pair[0]);
                    output.push(pair[1]);
                }
            }
        }
    }

    output
}

/// Convert raw PCM bytes to f32 samples. Output channel count matches input.
fn decode_pcm(pcm: &[u8], channels: u16, bits_per_sample: u16) -> Vec<f32> {
    match (bits_per_sample, channels) {
        (16, _) => pcm
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect(),
        (8, _) => pcm.iter().map(|&b| (b as f32 - 128.0) / 128.0).collect(),
        _ => {
            log::trace!("WAV: unsupported {}bit {}ch PCM", bits_per_sample, channels);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn cloak_sound_registered_resolution_rejects_raw_sample_and_invalid_names() {
        let registry = SoundRegistry::from_ini(&IniFile::from_str(
            "[NavalUnitEmerge]\nSounds=vnavupa\nVolume=55\n",
        ));
        let mut counter = 0;

        assert_eq!(
            select_registered_sound("NavalUnitEmerge", &registry, &mut counter),
            Some(("vnavupa", 0.55)),
            "the retail Voc/event identity resolves to its registered sample"
        );
        assert_eq!(counter, 1);
        for invalid in ["", "MissingCloakEvent", "vnavupa"] {
            assert_eq!(
                select_registered_sound(invalid, &registry, &mut counter),
                None,
                "an invalid Voc identity must not become a raw-bag fallback: {invalid}"
            );
        }
        assert_eq!(
            counter, 1,
            "rejected identities do not advance sound selection"
        );
    }

    #[test]
    fn gsi_01_02_focus_gate_restores_distinct_live_sfx_gains_absolutely() {
        let quiet = SfxOutputGain::new(0.2, SfxChannel::Sound);
        let loud = SfxOutputGain::new(0.75, SfxChannel::Sound);

        assert_eq!(quiet.effective(1.0, 0.4, 0.6, 0.0), 0.0);
        assert_eq!(loud.effective(1.0, 0.4, 0.6, 0.0), 0.0);
        assert!((quiet.effective(1.0, 0.4, 0.6, 1.0) - 0.12).abs() < f32::EPSILON);
        assert!((loud.effective(1.0, 0.4, 0.6, 1.0) - 0.45).abs() < f32::EPSILON);
    }

    #[test]
    fn gsi_01_02_sfx_armed_while_inactive_retains_gain_for_restore() {
        // Construction while inactive still creates/starts the Player in the
        // production path; only this independently retained output gain is 0.
        let armed_while_inactive = SfxOutputGain::new(0.35, SfxChannel::Sound);

        assert_eq!(armed_while_inactive.effective(1.0, 0.4, 1.0, 0.0), 0.0);
        assert!((armed_while_inactive.effective(1.0, 0.4, 1.0, 1.0) - 0.35).abs() < f32::EPSILON);
    }

    #[test]
    fn options_profile_sound_and_voice_masters_are_independent() {
        let sound = SfxOutputGain::new(0.5, SfxChannel::Sound);
        let voice = SfxOutputGain::new(0.5, SfxChannel::Voice);

        assert!((sound.effective(0.2, 0.8, 1.0, 1.0) - 0.1).abs() < f32::EPSILON);
        assert!((voice.effective(0.2, 0.8, 1.0, 1.0) - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn options_profile_master_changes_recompose_from_base_gain() {
        let queued_voice = SfxOutputGain::new(0.8, SfxChannel::Voice);

        let before = queued_voice.effective(0.7, 0.4, 1.0, 1.0);
        let changed = queued_voice.effective(0.7, 0.25, 1.0, 1.0);
        let restored = queued_voice.effective(0.7, 0.4, 1.0, 1.0);

        assert!((before - 0.32).abs() < f32::EPSILON);
        assert!((changed - 0.2).abs() < f32::EPSILON);
        assert_eq!(
            restored, before,
            "recomposition must not compound old masters"
        );
    }

    /// An idle voice slot with an empty queue reports no active voices. Skips
    /// gracefully when no audio device is available (CI).
    #[test]
    fn voices_active_false_when_idle() {
        let Some(player) = SfxPlayer::new() else {
            return;
        };
        assert!(!player.voices_active());
    }

    fn build_test_wav(sample_rate: u32, bits: u16, channels: u16, samples: &[u8]) -> Vec<u8> {
        let data_size: u32 = samples.len() as u32;
        let fmt_size: u32 = 16;
        let byte_rate: u32 = sample_rate * channels as u32 * bits as u32 / 8;
        let block_align: u16 = channels * bits / 8;
        let riff_size: u32 = 4 + (8 + fmt_size) + (8 + data_size);

        let mut wav: Vec<u8> = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&riff_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&fmt_size.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        wav.extend_from_slice(samples);
        wav
    }

    #[test]
    fn test_decode_wav_16bit_mono() {
        // 4 samples of 16-bit mono silence — upmixed to 8 stereo f32 values.
        let pcm: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0];
        let wav = build_test_wav(22050, 16, 1, &pcm);
        let decoded = decode_wav(&wav, "test.wav").expect("should decode");
        assert_eq!(decoded.sample_rate, 22050);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 8); // 4 mono → 4 stereo pairs
    }

    #[test]
    fn test_decode_wav_8bit_mono() {
        let pcm: Vec<u8> = vec![128, 128, 128, 128];
        let wav = build_test_wav(11025, 8, 1, &pcm);
        let decoded = decode_wav(&wav, "test.wav").expect("should decode");
        assert_eq!(decoded.sample_rate, 11025);
        for s in &decoded.samples {
            assert!(s.abs() < 0.01);
        }
    }

    #[test]
    fn test_decode_wav_16bit_stereo() {
        let pcm: Vec<u8> = vec![0xE8, 0x03, 0x18, 0xFC, 0x00, 0x00, 0x00, 0x00];
        let wav = build_test_wav(44100, 16, 2, &pcm);
        let decoded = decode_wav(&wav, "test.wav").expect("should decode");
        assert_eq!(decoded.samples.len(), 4); // 2 stereo frames, already 2ch
    }

    #[test]
    fn test_decode_wav_too_short() {
        assert!(decode_wav(&[0u8; 10], "short.wav").is_none());
    }

    #[test]
    fn test_decode_pcm_empty() {
        let samples = decode_pcm(&[], 1, 16);
        assert!(samples.is_empty());
    }
}
