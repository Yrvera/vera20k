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
//! - Sample selection, pitch/volume shift, positional volume, stereo pan and
//!   the DirectSound loudness curve are reproduced from `gamemd.exe`
//!   (`VocClass`, `SoundEvent`, `DSoundBuffer`); see the provenance comments
//!   on each helper. Everything below `play_decoded` is device plumbing.
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
use crate::rules::sound_ini::{SoundEntry, SoundRegistry, VOLUME_SCALE, control, sound_type};

/// Maximum concurrent SFX sounds — matches original engine's 16 DirectSound buffers.
/// RESIDUAL (GSI-15.03/15.04) — there is no channel pool, and the parts of
/// these two rows that matter for parity are entangled with the device.
/// Eviction is plain FIFO over this queue, so the `Priority=` tier decoded in
/// `rules/sound_ini.rs` is ignored and a `CRITICAL` cue loses to an older
/// `LOWEST` one; `Limit=` is parsed but unenforced (17 stock sounds cap at one
/// instance — native counts live instances per event at `AudioEventClass+0x44`
/// against `+0x48` in `SoundSystem::UpdateTick @ 0x004041D0` and stops the
/// oldest); `Control=INTERRUPT` (`0x10`) has no owner; `Control=LOOP`/`Loop=`
/// (`SoundEvent::SelectNextSample @ 0x00404BB0`, `AdvancePlaylist @
/// 0x004047B0`) play only their first pass; `Delay=`/`PREDELAY`/`AMBIENT`
/// pre-delays (`SoundEvent::UpdateState @ 0x004055C0` state 2, threshold
/// `0x21` ms) are drawn from the RNG but not waited for; and finished handles
/// are reaped only inside `play_decoded`, so an idle frame never cleans up.
///
/// Pass 2 established the cadence: `SoundSystem::UpdateTick @ 0x004041D0` is
/// pumped from `AudioSystem::Pump @ 0x00406F70` off the message/service loop —
/// NOT the sim tick — so the mixer's update rate is not frame-locked and must
/// not be modelled as if it were.
/// - Trigger: any moment more than a handful of sounds compete, and every
///   ambient, looping or pre-delayed cue.
/// - Player effect: important cues get dropped for unimportant older ones,
///   ambient beds never sustain, interruptions click instead of crossing, and
///   pre-delayed cues start early.
/// - Frequency: continuous in any busy engagement.
/// - Downstream risk: **not reachable from `cargo test -p vera20k --lib`.**
///   `SfxPlayer::new` returns `None` without an audio device, so every path
///   below `play_decoded` is unverifiable here and none of it may be claimed
///   verified. The first slice is a device-free arbiter — a slot table,
///   per-event instance counts and lowest-priority-wins eviction
///   (`DSoundChannel::FindLowestPriority @ 0x00404E20`) — with all rodio work
///   left in this file.
const MAX_CONCURRENT_SFX: usize = 16;

/// `VocClass::CalcVolumeAndPan @ 0x00750AC0` (`0x00750B0F..0x00750B17`):
/// `maxRange = Range * 0x3C` pixels.
const RANGE_MULTIPLIER: i32 = 0x3C;

/// Pan scale: `0..=0x4000` with `0x2000` centre (`0x00750D10..0x00750D24`,
/// constant `0x007F68E8` = 8192.0f).
pub const PAN_CENTRE: i32 = 0x2000;
pub const PAN_SCALE: i32 = 0x4000;

/// Audibility cutoff: volumes below the double at `0x007E8AE8` (0.05) are
/// silent (`0x00750CBD..0x00750CCC`).
const MIN_VOLUME_CUTOFF: f64 = 0.05;

/// Truncating `Math::ftol @ 0x007C5F00` (control word `0x0E7F`: round toward
/// zero, 53-bit precision), applied to a double intermediate.
///
/// RESIDUAL (UNCHECKED) — out-of-`i32` inputs differ: `FISTP` stores the x87
/// indefinite `0x80000000` for both signs, while Rust's `as i32` saturates to
/// `i32::MIN` *or* `i32::MAX`. Trigger: a client point more than 2^31 pixels
/// from the view centre. Player effect: none — the largest YR map is under
/// 2^16 pixels across, so only a hand-built [`SpatialListener`] reaches it.
/// Frequency: never in play. Downstream risk: none.
fn ftol(value: f64) -> i32 {
    value.trunc() as i32
}

/// Native integer absolute value: `CDQ; XOR EAX,EDX; SUB EAX,EDX`
/// (`0x00750BCF..0x00750BD2` and `0x00750BED..0x00750BF0`). The idiom **wraps**
/// — `i32::MIN` comes back as `i32::MIN` — where Rust's `i32::abs` panics in a
/// debug build, so the transcription must use the wrapping form.
fn native_abs(value: i32) -> i32 {
    value.wrapping_abs()
}

/// The listener: the tactical view rect and its top-left in the world-pixel
/// frame the sound positions use.
///
/// gamemd-derived: `CalcVolumeAndPan` reads the width/height globals
/// `0x00886FA8`/`0x00886FAC` (written by `Set_View_Dimensions`) and projects
/// the sound through `TacticalClass::CoordsToClient2 @ 0x006D2140`, which
/// scales leptons by the fixed native 60/30-pixel tile (`iVar3 = (x*0x3c)/2 +
/// (y*-0x3c)/2`, `>> 8`) and subtracts the view origin (`this+0xB0/+0xB4`).
/// That fixed scale is VERA's world-pixel frame — `map::terrain::TILE_WIDTH`
/// 60, `TILE_HEIGHT` 30 — so at `zoom == 1.0` `client_point` is the native
/// client point exactly.
///
/// **Zoom is VERA-internal; gamemd has no zoom.** `tactical_width`/`_height`
/// are the *device* pixels of the tactical viewport, and VERA's projection is
/// `device = (world - camera) * zoom`, so the viewport spans
/// `device / zoom` world pixels ([`SpatialListener::view_extent`]). Every
/// operand handed to [`calc_volume_and_pan`] is therefore expressed in the
/// world-pixel frame: the client point, the view extent, and `Range * 60`
/// alike. Scaling the *client point* into device pixels instead would be the
/// same algebra for the falloff shape and the pan, but it would silently
/// redefine `Range=` — a cell count in `sound(md).ini` — as a device-pixel
/// budget, so a `Range=10` cue would carry 2.5 cells at 4x zoom. Keeping the
/// world frame makes zoom behave like a native resolution change, which is
/// the only zoom-like thing gamemd actually does: a bigger view rect covers
/// more world, while `Range` stays a fixed cell distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialListener {
    /// Tactical viewport width in device pixels (native `0x00886FA8`).
    pub tactical_width: i32,
    /// Tactical viewport height in device pixels (native `0x00886FAC`).
    pub tactical_height: i32,
    /// World-pixel position of the viewport's top-left corner.
    pub origin_x: f32,
    /// World-pixel position of the viewport's top-left corner.
    pub origin_y: f32,
    /// VERA-internal camera zoom (`app::input::camera`, 0.25..=4.0, default
    /// 1.0). `1.0` reproduces gamemd bit for bit.
    pub zoom: f32,
}

impl SpatialListener {
    /// Client (view-relative) pixel of a world-pixel position; the native
    /// client point is integer, so the fractional VERA camera is truncated.
    pub fn client_point(&self, screen_x: f32, screen_y: f32) -> (i32, i32) {
        (
            ftol(f64::from(screen_x) - f64::from(self.origin_x)),
            ftol(f64::from(screen_y) - f64::from(self.origin_y)),
        )
    }

    /// The tactical view size in the same world-pixel frame as
    /// [`Self::client_point`]. At `zoom == 1.0` this is the native
    /// `(float)[0x00886FA8]` / `(float)[0x00886FAC]` cast unchanged (dividing
    /// by exactly 1.0 is exact in IEEE-754).
    ///
    /// VERA-internal, gamemd equivalent UNCHECKED: the `max(EPSILON)` floor.
    /// `zoom_level` is clamped to `MIN_ZOOM` 0.25 in `app::input::camera`, so
    /// nothing in the app can reach it; it only stops a hand-built listener
    /// from dividing by zero. Same guard the visible-bounds code already uses
    /// (`presentation::instances::helpers`).
    pub fn view_extent(&self) -> (f32, f32) {
        let zoom = self.zoom.max(f32::EPSILON);
        (
            self.tactical_width as f32 / zoom,
            self.tactical_height as f32 / zoom,
        )
    }
}

/// The registry facts `CalcVolumeAndPan` reads from the event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialSource {
    pub range_cells: i32,
    pub type_flags: u32,
    /// `MinVolume` as the stored fraction (`AudioEventClass+0x54`).
    pub min_volume: f32,
}

impl SpatialSource {
    pub fn from_entry(entry: &SoundEntry) -> Self {
        Self {
            range_cells: entry.range,
            type_flags: entry.type_flags,
            min_volume: entry.min_volume,
        }
    }

    /// The `[Defaults]` facts, for raw audio-bag names that have no event.
    /// Native never plays those positionally (an invalid `VocClass` index
    /// plays nothing); this is the VERA-internal fallback's listener model.
    pub fn from_registry_defaults(registry: &SoundRegistry) -> Self {
        let defaults = registry.defaults();
        Self {
            range_cells: defaults.range,
            type_flags: defaults.type_flags,
            min_volume: defaults.min_volume_fraction(),
        }
    }
}

/// Positional result: the spatial volume `0..=1` and the pan `0..=0x4000`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpatialGain {
    pub volume: f32,
    pub pan: i32,
}

impl SpatialGain {
    /// Non-positional playback: full volume, centred. This is what
    /// `TechnoClass` voices and UI cues use (volume 1.0, pan `0x2000`).
    pub const CENTRED_FULL: Self = Self {
        volume: 1.0,
        pan: PAN_CENTRE,
    };

    /// Native linear volume `min(ftol(volume * 0x4000), 0x4000)`
    /// (`VocClass::PlayAt @ 0x007509E0`, `0x00750A55..0x00750A6F`).
    pub fn volume_linear(&self) -> i32 {
        ftol(f64::from(self.volume) * f64::from(VOLUME_SCALE)).min(VOLUME_SCALE)
    }
}

/// Positional volume and pan for one sound at one client point.
///
/// gamemd-derived: `VocClass::CalcVolumeAndPan @ 0x00750AC0`, transcribed
/// from `disassemble_function` — every rounding point below names its
/// instruction range.
/// - `halfW = W * 0.5f`, `halfH = H * 0.5f`, `fullW = halfW + halfW`
///   (`0x00750AD6..0x00750B06`), `maxRange = Range * 0x3C` as float.
/// - `Type=SHROUD` (`0x800`): a sound in a cell whose shroud flags
///   (`CellClass+0x12C & 0x18`) are both clear returns 0 (`0x00750B2D..
///   0x00750BAA`). The cell test is the caller's: `shrouded`.
///   RESIDUAL — the sentinel-cell early-out ahead of it is not modelled.
///   `0x00750B51 CMP CX,[0x00b1d310]` / `0x00750B6C CMP AX,[0x00b1d312]`
///   return 0.0f when the sound's cell equals that `CellStruct` pair, i.e.
///   *before* the `CellClass` lookup at `0x00750B8F`. `get_xrefs_to` on both
///   words shows one read (this site) and one write — the three-instruction
///   setter at `0x007502B0` (`XOR EAX,EAX; MOV [0x00b1d310],AX; MOV
///   [0x00b1d312],AX`), reached only through the vtable slot at `0x0081562C`
///   — and the image bytes are already `00 00 00 00`, so the pair is the null
///   cell `(0,0)` for the whole process lifetime. Trigger: a `Type=SHROUD`
///   cue whose object sits on map cell `(0,0)`. Player effect: that one cue is
///   silent. Frequency: never in ordinary play — cell `(0,0)` is outside every
///   map's playable rectangle, and the 11 stock `SHROUD` cues are super-weapon
///   ready/open sounds that VERA does not emit yet. Downstream risk: none; it
///   is an early-out, so modelling it can only add silence.
/// - `offsetX = clientX - halfW` kept as float (`FST [ESP+0x10]`); the
///   distances are `|ftol(clientX - halfW)|` and `|ftol(clientY - halfH)|`
///   (`0x00750BBE..0x00750BF6`), the abs taken by the wrapping `CDQ; XOR; SUB`
///   idiom ([`native_abs`]).
/// - Unless `Type=LOCAL` (`0x40`): subtract the half view from each and clamp
///   at 0 (`0x00750BFA..0x00750C37`). Then `distY *= 2` (`0x00750C3D`).
/// - `volume = (maxRange - max(distX, distY)) / maxRange` only when both are
///   below `maxRange` and `maxRange > 0`, else 0 (`0x00750C43..0x00750C99`).
///   The `max` is `distY` when `distX <= distY`.
/// - `Type=GLOBAL` (`0x10`): `volume = max(volume, MinVolume)`
///   (`0x00750C9B..0x00750CBD`).
/// - `volume < 0.05` (double compare) returns 0 with no pan (`0x00750CBD`).
/// - `pan = ftol(clamp(offsetX, -fullW, fullW) * 8192 / fullW + 8192)`
///   (`0x00750CDE..0x00750D24`); the `FCHS` builds `-fullW`, it does not
///   negate the offset.
///
/// `tactical_width`/`tactical_height` and `client_x`/`client_y` must be in the
/// same pixel frame — see [`SpatialListener::view_extent`], which is what puts
/// them there under VERA's zoom. Native passes the integer view globals here;
/// an integral `f32` reproduces the `(float)` cast exactly.
pub fn calc_volume_and_pan(
    client_x: i32,
    client_y: i32,
    tactical_width: f32,
    tactical_height: f32,
    source: SpatialSource,
    shrouded: bool,
) -> Option<SpatialGain> {
    let half_w: f32 = tactical_width * 0.5;
    let half_h: f32 = tactical_height * 0.5;
    let full_w: f32 = half_w + half_w;
    // `LEA EAX,[EAX+EAX*2]; LEA EAX,[EAX+EAX*4]; SHL EAX,2` at
    // `0x00750B0F..0x00750B17` — a 32-bit ×60 that wraps rather than trapping.
    // `AudioEventClass::SetRange @ 0x004065E0` stores `Range=` as a full int,
    // so `wrapping_mul` is the transcription of the native overflow, not a
    // guard: it is what keeps a `Range=` past 35_791_394 on the native
    // truncation instead of panicking in a debug build.
    let max_range: f32 = (source.range_cells.wrapping_mul(RANGE_MULTIPLIER)) as f32;

    if source.type_flags & sound_type::SHROUD != 0 && shrouded {
        return None;
    }

    let offset_x: f32 = (f64::from(client_x) - f64::from(half_w)) as f32;
    let mut dist_x: f32 = native_abs(ftol(f64::from(client_x) - f64::from(half_w))) as f32;
    let mut dist_y: f32 = native_abs(ftol(f64::from(client_y) - f64::from(half_h))) as f32;

    if source.type_flags & sound_type::LOCAL == 0 {
        dist_x -= half_w;
        dist_y -= half_h;
        if dist_x < 0.0 {
            dist_x = 0.0;
        }
        if dist_y < 0.0 {
            dist_y = 0.0;
        }
    }
    dist_y += dist_y;

    let mut volume: f32 = 0.0;
    if dist_x < max_range && dist_y < max_range && 0.0 < max_range {
        let dist = if dist_x <= dist_y { dist_y } else { dist_x };
        volume = (max_range - dist) / max_range;
    }
    if source.type_flags & sound_type::GLOBAL != 0 && volume < source.min_volume {
        volume = source.min_volume;
    }
    if f64::from(volume) < MIN_VOLUME_CUTOFF {
        return None;
    }

    let clamped = if offset_x < -full_w {
        -full_w
    } else if offset_x > full_w {
        full_w
    } else {
        offset_x
    };
    let pan = ftol(
        f64::from(clamped) * f64::from(PAN_CENTRE) / f64::from(full_w) + f64::from(PAN_CENTRE),
    );
    Some(SpatialGain { volume, pan })
}

/// [`calc_volume_and_pan`] for a world-pixel position against a listener.
///
/// Both operands come out of the listener in the world-pixel frame, so the
/// falloff distance and the pan stay in one unit at every zoom.
pub fn spatial_gain(
    source: SpatialSource,
    screen_x: f32,
    screen_y: f32,
    listener: &SpatialListener,
    shrouded: bool,
) -> Option<SpatialGain> {
    let (client_x, client_y) = listener.client_point(screen_x, screen_y);
    let (view_w, view_h) = listener.view_extent();
    calc_volume_and_pan(client_x, client_y, view_w, view_h, source, shrouded)
}

/// DirectSound attenuation table, hundredths of a decibel, indexed by the
/// linear volume in percent. Machine-derived: `read_memory 0x00816380` (101
/// dwords, `-10000` at 0 through `0` at 100); the entries are
/// `round(1000 * log2(i / 100))`, i.e. 10 dB per halving, floored at -100 dB.
/// Applied by the `DSoundBuffer` apply routine `FUN_0040A6D0` as
/// `SetVolume(table[(volume >> 16) * 25 >> 12])` and as the per-side pan
/// attenuation.
const DSOUND_ATTENUATION_TABLE: [i16; 101] = [
    -10000, -6644, -5644, -5059, -4644, -4322, -4059, -3837, -3644, -3474, -3322, -3184, -3059,
    -2943, -2837, -2737, -2644, -2556, -2474, -2396, -2322, -2252, -2184, -2120, -2059, -2000,
    -1943, -1889, -1837, -1786, -1737, -1690, -1644, -1599, -1556, -1515, -1474, -1434, -1396,
    -1358, -1322, -1286, -1252, -1218, -1184, -1152, -1120, -1089, -1059, -1029, -1000, -971, -943,
    -916, -889, -862, -837, -811, -786, -761, -737, -713, -690, -667, -644, -621, -599, -578, -556,
    -535, -515, -494, -474, -454, -434, -415, -396, -377, -358, -340, -322, -304, -286, -269, -252,
    -234, -218, -201, -184, -168, -152, -136, -120, -105, -89, -74, -59, -44, -29, -14, 0,
];

/// Amplitude of one DirectSound attenuation (hundredths of a dB).
/// `DSBVOLUME_MIN` (-10000) is DirectSound's documented silence, so the
/// table's zero entry maps to exactly 0 rather than the -100 dB it names.
fn attenuation_amplitude(hundredths_db: i16) -> f32 {
    if hundredths_db <= DSOUND_ATTENUATION_TABLE[0] {
        return 0.0;
    }
    10f64.powf(f64::from(hundredths_db) / 2000.0) as f32
}

/// Product of two native linear volumes: `DSoundBuffer::CombineInterps
/// FUN_004010C0` (`0x004010C7..0x004010D6`), `(a * b) >> 14`.
///
/// **VERA-internal, gamemd equivalent UNCHECKED: both `.clamp`s.** The native
/// volume path is `SHR EDX,0x10; SHR EAX,0x10; IMUL EDX,EAX; SHR EDX,0xe`
/// (`disassemble_function 0x004010C0`, read this session) — no bound at all;
/// the two operands are merely the top halves of 32-bit fixed-point fields, so
/// native accepts anything up to `0xFFFF` on each side and can return well
/// past `0x4000`. (The *pan* path at `0x00401116..0x0040114C` does clamp to
/// `0..0x4000`; the volume path does not, so this is not borrowed from there.)
/// Trigger: an operand outside `0..=0x4000`, which no VERA producer can make —
/// `SoundEntry::volume_linear` comes out of the native `[0, 1]` clamp times
/// 16384, and [`PlayShifts::volume_linear`] is `0x4000 - ((vshift << 14)/100)`
/// with `vshift` already held to `0..=100` by native `SetVShift @ 0x00406620`.
/// Player effect: none reachable. Frequency: never. Downstream risk: the guard
/// is what keeps [`native_volume_amplitude`]'s fixed 101-entry table index in
/// bounds, where Rust would panic and gamemd would read past the table.
pub fn combine_linear(a: i32, b: i32) -> i32 {
    (a.clamp(0, VOLUME_SCALE) * b.clamp(0, VOLUME_SCALE)) >> 14
}

/// Amplitude the DirectSound layer produces for one combined linear volume:
/// `FUN_0040A6D0` indexes the table with `volume * 25 >> 12` (0..=100).
///
/// VERA-internal, gamemd equivalent UNCHECKED: the `.clamp`. `FUN_0040A6D0`
/// computes `(v >> 16) * 25 >> 12` and indexes `DAT_00816380` with it
/// unchecked, so an out-of-range volume reads past the 101-entry table in
/// gamemd; in Rust that is a panic, which is not a behaviour worth
/// reproducing. Trigger and frequency: as [`combine_linear`] — unreachable
/// from any VERA producer.
pub fn native_volume_amplitude(linear: i32) -> f32 {
    let index = (linear.clamp(0, VOLUME_SCALE) * 25) >> 12;
    attenuation_amplitude(DSOUND_ATTENUATION_TABLE[index as usize])
}

/// Left/right channel amplitudes for one pan (`0..=0x4000`).
///
/// gamemd-derived: `FUN_0040A6D0` (`0x0040A6F4..`) maps the combined pan to
/// `p = (pan * 25 >> 11) - 100` and calls `SetPan(table[100 - |p|])` for a
/// left pan (negative DirectSound pan attenuates the right channel) and
/// `SetPan(-table[100 - p])` for a right pan (attenuates the left channel).
///
/// VERA-internal, gamemd equivalent UNCHECKED: the `.clamp`, for the same
/// reason as [`native_volume_amplitude`] — native indexes the table
/// unchecked. `CalcVolumeAndPan` produces the pan as
/// `ftol(clamp(offsetX, -fullW, fullW) * 8192 / fullW + 8192)`, which is
/// already `0..=0x4000`, so nothing in VERA can reach the guard.
pub fn pan_channel_gains(pan: i32) -> (f32, f32) {
    let p = ((pan.clamp(0, PAN_SCALE) * 25) >> 11) - 100;
    let attenuated = attenuation_amplitude(DSOUND_ATTENUATION_TABLE[(100 - p.abs()) as usize]);
    if p < 0 {
        (1.0, attenuated)
    } else if p > 0 {
        (attenuated, 1.0)
    } else {
        (1.0, 1.0)
    }
}

/// Apply per-channel pan gains to interleaved stereo samples.
fn apply_pan(samples: &mut [f32], pan: i32) {
    let (left, right) = pan_channel_gains(pan);
    if left == 1.0 && right == 1.0 {
        return;
    }
    for frame in samples.chunks_exact_mut(2) {
        frame[0] *= left;
        frame[1] *= right;
    }
}

/// The audio RNG contract: `Random::RandomRanged @ 0x0065C7E0` on the
/// non-scenario `g_MainRng @ 0x00886B88` (seeded from the system clock in
/// `Init_Random_Number_System @ 0x0052FC20`, never synchronised). Inclusive
/// bounds; equal bounds return without drawing.
pub trait SampleRng {
    fn ranged(&mut self, low: i32, high: i32) -> i32;
}

/// Presentation-side RNG for sample choice, pitch and volume shift. The
/// native generator is clock-seeded, so only its draw contract matters; this
/// is SplitMix64 with rejection sampling for an unbiased inclusive range.
#[derive(Debug, Clone)]
pub struct SfxRng {
    state: u64,
}

impl SfxRng {
    pub fn seeded(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);
        Self::seeded(nanos)
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl SampleRng for SfxRng {
    fn ranged(&mut self, low: i32, high: i32) -> i32 {
        if low == high {
            return low;
        }
        let (low, high) = if high < low { (high, low) } else { (low, high) };
        let span = (i64::from(high) - i64::from(low) + 1) as u64;
        let zone = u64::MAX - (u64::MAX % span);
        loop {
            let draw = self.next_u64();
            if draw < zone {
                return (i64::from(low) + (draw % span) as i64) as i32;
            }
        }
    }
}

/// Per-play randomised facts drawn before the samples are chosen.
///
/// gamemd-derived: `SoundEvent::UpdateState @ 0x004055C0` state 0
/// (`0x0040567F..0x004056A7`): `fshift = 100 + RandomRanged(FShift.min,
/// FShift.max)` and `vshift = RandomRanged(0, VShift)`; then, when
/// `Control & (PREDELAY|AMBIENT)`, `RandomRanged(AMBIENT ? 0x21 : Delay.min,
/// Delay.max)` for the pre-delay (`0x00405729..0x00405743`). The pre-delay
/// itself is a channel-arbiter (A2) residual; its draw is kept so the RNG
/// sequence matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayShifts {
    /// Frequency multiplier in percent (`SoundEvent+0x14C`).
    pub frequency_pct: i32,
    /// Volume reduction in percent (`SoundEvent+0x150`).
    pub volume_shift_pct: i32,
}

impl PlayShifts {
    pub fn draw(entry: &SoundEntry, rng: &mut impl SampleRng) -> Self {
        let frequency_pct = rng.ranged(entry.fshift.0, entry.fshift.1) + 100;
        let volume_shift_pct = rng.ranged(0, entry.vshift);
        if entry.control & (control::PREDELAY | control::AMBIENT) != 0 {
            let min = if entry.control & control::AMBIENT != 0 {
                0x21
            } else {
                entry.delay_ms.0
            };
            let _predelay_ms = rng.ranged(min, entry.delay_ms.1);
        }
        Self {
            frequency_pct,
            volume_shift_pct,
        }
    }

    /// Buffer volume after the shift: `SoundEvent::StartPlayback @
    /// 0x004054A0` (`0x004054EB..0x00405519`), `0x4000 - ((vshift << 14) /
    /// 100)` when `vshift > 0`.
    pub fn volume_linear(&self) -> i32 {
        if self.volume_shift_pct > 0 {
            VOLUME_SCALE - ((self.volume_shift_pct << 14) / 100)
        } else {
            VOLUME_SCALE
        }
    }

    /// Playback rate: `FUN_00401190` returns `(pct * rate) / 100`
    /// (`SHR ECX,0x10; IMUL ECX,EDX;` then the `0x51EB851F` magic divide by
    /// 100, truncating toward zero).
    ///
    /// VERA-internal, gamemd equivalent UNCHECKED: the `.max(1)`. The native
    /// body has no floor — it hands the raw quotient to the DirectSound
    /// buffer's `SetFrequency`, which rejects 0 at the device layer. VERA
    /// instead feeds the rate to its own resampler, where 0 is not a
    /// well-defined input. Trigger: `frequency_pct <= 0`, i.e. a `FShift=`
    /// whose low bound is at or below -100; the widest stock `FShift=` is
    /// `-15 15`. Player effect: none reachable. Frequency: never on retail
    /// data. Downstream risk: none.
    pub fn shifted_sample_rate(&self, sample_rate: u32) -> u32 {
        ((i64::from(self.frequency_pct) * i64::from(sample_rate)) / 100).max(1) as u32
    }
}

/// Sample indices, in play order, for one pass of an event.
///
/// gamemd-derived: `SoundEvent::LoadSamples @ 0x004048B0` for events whose
/// `Delay.min < 0x21` (`0x004048FB..0x00404ACD`, all 588 stock `Control=`
/// entries except the 60 with a longer pre-delay, which reach the same
/// first-pass result through `SelectNextSample @ 0x00404BB0`):
/// 1. `Attack > 0`: load `samples[RandomRanged(0, Attack - 1)]`.
/// 2. Without `ALL`: `RANDOM` loads `samples[RandomRanged(Attack, count -
///    Decay - 1)]`, otherwise `samples[Attack]` (the first body sample —
///    never a round-robin). With `ALL`: every body sample in order.
/// 3. `Decay > 0`: load `samples[RandomRanged(count - Decay, count - 1)]`.
/// Then `PreparePlayout @ 0x00404700` / `AdvancePlaylist @ 0x004047B0` play
/// the loaded buffers: the attack buffer first when `Control=ATTACK`, the
/// decay buffer last when `Control=DECAY`, and the rest in `RandomRanged(0,
/// remaining - 1)` pick-and-remove order for `RANDOM` or in load order
/// otherwise.
///
/// **VERA-internal, gamemd equivalent UNCHECKED: every bounds guard below.**
/// Native indexes the fixed 32-slot sample array at `AudioEventClass+0xB4`
/// with no check at all, and `AudioEventClass::SetControlFlags @ 0x00406570`
/// (read this session) normalises the attack/decay counts against the
/// `Control=` flags *only* — never against how many names `Sounds=` actually
/// listed. So `Attack=5` on a two-sample entry, or `Sounds=` omitted
/// entirely, reads past the loaded pointers in gamemd; the observable result
/// is whatever that garbage pointer does, which VERA cannot reproduce and
/// must not pretend to. Trigger: only a hand-edited `sound(md).ini` whose
/// `Attack=`/`Decay=` exceed its own `Sounds=` list — no stock entry does.
/// Player effect: VERA plays a valid sample (or nothing) where gamemd would
/// misbehave. Frequency: never on retail data. Downstream risk: none.
pub fn select_playout(entry: &SoundEntry, rng: &mut impl SampleRng) -> Vec<usize> {
    let count = entry.sounds.len() as i32;
    if count == 0 {
        return Vec::new();
    }
    let body = entry.body_range();
    let (body_start, body_end) = (body.start as i32, body.end as i32);
    let mut attack = None;
    let mut decay = None;
    let mut middle: Vec<usize> = Vec::new();

    if entry.attack > 0 {
        // `.clamp`: VERA-internal, see the note above.
        attack = Some(rng.ranged(0, entry.attack - 1).clamp(0, count - 1) as usize);
    }
    if entry.control & control::ALL == 0 {
        let index = if entry.control & control::RANDOM != 0 {
            rng.ranged(body_start, body_end - 1)
        } else {
            body_start
        };
        // `.contains`: VERA-internal, see the note above. An empty body range
        // (`Attack + Decay >= count`) puts `body_start` at `count`.
        if (0..count).contains(&index) {
            middle.push(index as usize);
        }
    } else {
        middle.extend(body.clone());
    }
    if entry.decay > 0 {
        // `.clamp`: VERA-internal, see the note above.
        decay = Some(
            rng.ranged(count - entry.decay, count - 1)
                .clamp(0, count - 1) as usize,
        );
    }

    let mut order = Vec::with_capacity(middle.len() + 2);
    // Native keeps the attack buffer first only under the ATTACK control flag,
    // and the decay buffer last only under DECAY; without the flag the count is
    // zero (see `SoundEntry::attack`), so both agree.
    order.extend(attack);
    if entry.control & control::RANDOM != 0 {
        while !middle.is_empty() {
            let pick = rng.ranged(0, middle.len() as i32 - 1) as usize;
            // `.min`: VERA-internal, see the note above — `SampleRng` is a
            // public trait, so an out-of-contract impl must not panic here.
            order.push(middle.remove(pick.min(middle.len() - 1)));
        }
    } else {
        order.append(&mut middle);
    }
    order.extend(decay);
    order
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

impl DecodedAudio {
    /// Append another clip; the native playlist chains buffers back to back.
    /// A rate mismatch keeps the first clip (RESIDUAL: native streams each
    /// buffer at its own rate; no stock attack/decay set mixes rates).
    fn append(&mut self, mut other: DecodedAudio) {
        if other.sample_rate != self.sample_rate || other.channels != self.channels {
            log::warn!(
                "SFX: dropped chained sample with mismatched format ({} Hz vs {} Hz)",
                other.sample_rate,
                self.sample_rate
            );
            return;
        }
        self.samples.append(&mut other.samples);
    }
}

/// A resolved playback request: the decoded audio, the event's linear volume
/// (entry `Volume=` combined with the per-play `VShift=` reduction) and the
/// spatial gain.
struct ResolvedPlayback {
    decoded: DecodedAudio,
    event_linear: i32,
}

struct QueuedVoice {
    sound_id: String,
    decoded: DecodedAudio,
    /// Sound-entry linear volume only. The current Voice master is applied
    /// when this cue reaches the dedicated slot, not when it enters the queue.
    base_linear: i32,
}

impl QueuedVoice {
    fn new(sound_id: String, decoded: DecodedAudio, base_linear: i32) -> Self {
        Self {
            sound_id,
            decoded,
            base_linear,
        }
    }

    fn prepare_for_dequeue(self, scales: SfxOutputScales) -> (String, PreparedSfxOutput) {
        let output = prepare_direct_voice_output(self.decoded, self.base_linear, scales);
        (self.sound_id, output)
    }
}

/// User-controlled master channel for one secondary output.
///
/// gamemd-derived: `OptionsClass::SetDefaults @ 0x005FA350` and
/// `OptionsClass__ReadFromINI @ 0x005FA620` retain independent SoundVolume and
/// VoiceVolume settings; ordinary/animation effects use Sound while unit and
/// EVA speech use Voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SfxChannel {
    Sound,
    Voice,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SfxOutputScales {
    sound_volume: f32,
    voice_volume: f32,
    lifecycle_scale: f32,
    focus_output_scale: f32,
}

/// Master-independent gain retained beside one live secondary output.
///
/// `base_linear` is the native linear volume (`0..=0x4000`) of everything
/// below the user master: spatial volume, entry volume and the per-play
/// `VShift=` reduction. The master is chained into the same linear product
/// before the DirectSound curve — `DSoundBuffer::CombineInterps FUN_004010C0`
/// multiplies the buffer, event and group interps (`FUN_00402220`; the sound
/// group at `DAT_0087E758`, `SoundEvent::UpdateState 0x0040571E`) and only
/// then `FUN_0040A6D0` converts to decibels — so `effective` recomposes the
/// product each time rather than scaling an amplitude. The VERA-internal
/// lifecycle and foreground gates multiply the amplitude afterwards.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SfxOutputGain {
    base_linear: i32,
    channel: SfxChannel,
}

impl SfxOutputGain {
    fn new(base_linear: i32, channel: SfxChannel) -> Self {
        Self {
            base_linear,
            channel,
        }
    }

    fn effective(self, scales: SfxOutputScales) -> f32 {
        let master = match self.channel {
            SfxChannel::Sound => scales.sound_volume,
            SfxChannel::Voice => scales.voice_volume,
        };
        let master_linear = ftol(f64::from(master.clamp(0.0, 1.0)) * f64::from(VOLUME_SCALE));
        native_volume_amplitude(combine_linear(self.base_linear, master_linear))
            * scales.lifecycle_scale
            * scales.focus_output_scale
    }
}

/// Device-independent construction shared by tests and the rodio startup path.
/// `initial_volume` is the exact volume applied before the decoded source is
/// appended; `gain` remains master-independent for later live recomposition.
struct PreparedSfxOutput {
    decoded: DecodedAudio,
    gain: SfxOutputGain,
    initial_volume: f32,
}

impl PreparedSfxOutput {
    fn new(decoded: DecodedAudio, gain: SfxOutputGain, scales: SfxOutputScales) -> Self {
        Self {
            decoded,
            gain,
            initial_volume: gain.effective(scales),
        }
    }
}

fn prepare_normal_sfx_output(
    decoded: DecodedAudio,
    base_linear: i32,
    scales: SfxOutputScales,
) -> PreparedSfxOutput {
    PreparedSfxOutput::new(
        decoded,
        SfxOutputGain::new(base_linear, SfxChannel::Sound),
        scales,
    )
}

fn prepare_direct_voice_output(
    decoded: DecodedAudio,
    base_linear: i32,
    scales: SfxOutputScales,
) -> PreparedSfxOutput {
    PreparedSfxOutput::new(
        decoded,
        SfxOutputGain::new(base_linear, SfxChannel::Voice),
        scales,
    )
}

struct LiveSfxOutput {
    player: Player,
    gain: SfxOutputGain,
}

impl LiveSfxOutput {
    fn new(player: Player, gain: SfxOutputGain, initial_volume: f32) -> Self {
        player.set_volume(initial_volume);
        Self { player, gain }
    }

    fn apply_scales(&self, scales: SfxOutputScales) {
        self.player.set_volume(self.gain.effective(scales));
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
    /// Presentation-side RNG standing in for `g_MainRng @ 0x00886B88`.
    rng: SfxRng,
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
            rng: SfxRng::from_clock(),
        })
    }

    fn output_scales(&self) -> SfxOutputScales {
        SfxOutputScales {
            sound_volume: self.sound_volume as f32,
            voice_volume: self.voice_volume as f32,
            lifecycle_scale: self.output_scale,
            focus_output_scale: self.focus_output_scale,
        }
    }

    /// Resolve a registry event to decoded audio: draw the per-play shifts,
    /// pick the sample sequence, load and chain it, apply the pitch shift.
    fn resolve_entry(
        &mut self,
        entry: &SoundEntry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> Option<ResolvedPlayback> {
        resolve_entry_playback(entry, &mut self.rng, |name| {
            load_sfx(name, assets, audio_indices)
        })
    }

    /// Resolve a sound id through the registry, else as a raw audio-bag name
    /// (EVA lines and other bag-only entries) at full linear volume.
    fn resolve_any(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> Option<ResolvedPlayback> {
        if let Some(entry) = registry.get(sound_id) {
            return self.resolve_entry(entry, assets, audio_indices);
        }
        load_sfx(sound_id, assets, audio_indices).map(|decoded| ResolvedPlayback {
            decoded,
            event_linear: VOLUME_SCALE,
        })
    }

    /// Play a sound by its sound.ini ID (e.g., "VGCannon1") or audio.bag name,
    /// non-positionally (full volume, centred).
    ///
    /// Resolution order:
    /// 1. Look up `sound_id` in the SoundRegistry (sound.ini sections)
    /// 2. If found, pick the samples and load via audio bags then MIX assets
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
        self.play_sound_spatial(
            sound_id,
            SpatialGain::CENTRED_FULL,
            registry,
            assets,
            audio_indices,
        )
    }

    /// Play a sound with a plain volume multiplier and no pan — the launcher
    /// beep path, which scales a non-positional cue.
    pub fn play_sound_with_volume(
        &mut self,
        sound_id: &str,
        volume: f32,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.play_sound_spatial(
            sound_id,
            SpatialGain {
                volume,
                pan: PAN_CENTRE,
            },
            registry,
            assets,
            audio_indices,
        )
    }

    /// Play a sound at a positional gain from [`spatial_gain`].
    pub fn play_sound_spatial(
        &mut self,
        sound_id: &str,
        gain: SpatialGain,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            log::trace!("SFX: could not resolve '{}'", sound_id);
            return false;
        };
        let base_linear = combine_linear(gain.volume_linear(), resolved.event_linear);
        self.play_decoded(resolved.decoded, base_linear, gain.pan)
    }

    /// Play only a named `sound(md).ini` event and never reinterpret its ID as
    /// an audio-bag filename. `RulesClass::ReadAudioVisual @ 0x006691E0`
    /// resolves `[AudioVisual] CloakSound` through `VocClass::FindByName @
    /// 0x007514D0`; a failed lookup preserves the invalid constructor index,
    /// so `StartUncloaking @ 0x007036C0` produces no audible fallback.
    pub fn play_registered_sound_spatial(
        &mut self,
        sound_id: &str,
        gain: SpatialGain,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        let Some(entry) = registered_entry(sound_id, registry) else {
            return false;
        };
        let Some(resolved) = self.resolve_entry(entry, assets, audio_indices) else {
            return false;
        };
        let base_linear = combine_linear(gain.volume_linear(), resolved.event_linear);
        self.play_decoded(resolved.decoded, base_linear, gain.pan)
    }

    /// Start or replace the sound owned by one animation object.
    pub fn play_animation_sound_spatial(
        &mut self,
        anim_id: u64,
        sound_id: &str,
        gain: SpatialGain,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.stop_animation_sound(anim_id);
        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            return false;
        };
        let base_linear = combine_linear(gain.volume_linear(), resolved.event_linear);
        let mut decoded = resolved.decoded;
        apply_pan(&mut decoded.samples, gain.pan);
        let prepared = prepare_normal_sfx_output(decoded, base_linear, self.output_scales());
        let PreparedSfxOutput {
            decoded,
            gain,
            initial_volume,
        } = prepared;
        let Some(channels) = NonZero::new(decoded.channels) else {
            return false;
        };
        let Some(sample_rate) = NonZero::new(decoded.sample_rate) else {
            return false;
        };
        let source = SamplesBuffer::new(channels, sample_rate, decoded.samples);
        let player = Player::connect_new(self._device.mixer());
        let output = LiveSfxOutput::new(player, gain, initial_volume);
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
    /// don't stack, matching the original engine's behavior. Voices are
    /// non-positional: `TechnoClass::AI_Update @ 0x006F9EBB` plays them at
    /// volume 1.0 and pan `0x2000`.
    pub fn play_voice_sound(
        &mut self,
        sound_id: &str,
        registry: &SoundRegistry,
        assets: &AssetManager,
        audio_indices: &[crate::assets::audio_bag::AudioIndex],
    ) -> bool {
        self.advance_voice_queue();
        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            return false;
        };
        self.play_voice(
            resolved.decoded,
            resolved.event_linear,
            Some(sound_id.to_string()),
        )
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

        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            return false;
        };
        self.queued_voice.push_back(QueuedVoice::new(
            sound_id.to_string(),
            resolved.decoded,
            resolved.event_linear,
        ));
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

        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            return false;
        };
        self.play_voice(
            resolved.decoded,
            resolved.event_linear,
            Some(sound_id.to_string()),
        )
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

        let Some(resolved) = self.resolve_any(sound_id, registry, assets, audio_indices) else {
            return false;
        };
        self.play_voice(
            resolved.decoded,
            resolved.event_linear,
            Some(sound_id.to_string()),
        )
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
        let (sound_id, prepared) = queued.prepare_for_dequeue(self.output_scales());
        self.play_prepared_voice(prepared, Some(sound_id));
    }

    /// Play decoded audio on the dedicated voice slot, cutting off any current voice.
    fn play_voice(
        &mut self,
        decoded: DecodedAudio,
        base_linear: i32,
        sound_id: Option<String>,
    ) -> bool {
        let prepared = prepare_direct_voice_output(decoded, base_linear, self.output_scales());
        self.play_prepared_voice(prepared, sound_id)
    }

    fn play_prepared_voice(
        &mut self,
        prepared: PreparedSfxOutput,
        sound_id: Option<String>,
    ) -> bool {
        // Cut off previous voice immediately.
        if let Some(old) = self.voice_player.take() {
            old.player.stop();
        }
        self.current_voice_id = None;

        let PreparedSfxOutput {
            mut decoded,
            gain,
            initial_volume,
        } = prepared;

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
        let output = LiveSfxOutput::new(player, gain, initial_volume);
        output.player.append(source);
        self.voice_player = Some(output);
        self.current_voice_id = sound_id;
        true
    }

    /// Play already-decoded audio on the SFX pool at the given master-independent
    /// linear volume and pan.
    fn play_decoded(&mut self, mut decoded: DecodedAudio, base_linear: i32, pan: i32) -> bool {
        apply_pan(&mut decoded.samples, pan);
        let prepared = prepare_normal_sfx_output(decoded, base_linear, self.output_scales());
        let PreparedSfxOutput {
            mut decoded,
            gain,
            initial_volume,
        } = prepared;
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
        let output = LiveSfxOutput::new(player, gain, initial_volume);
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
        let scales = self.output_scales();
        for output in &self.active {
            output.apply_scales(scales);
        }
        for output in self.animation_active.values() {
            output.apply_scales(scales);
        }
        if let Some(output) = self.voice_player.as_ref() {
            output.apply_scales(scales);
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

/// The registry entry for a Voc identity, or `None` for an unknown name — a
/// raw sample name is not an event.
fn registered_entry<'a>(sound_id: &str, registry: &'a SoundRegistry) -> Option<&'a SoundEntry> {
    registry.get(sound_id)
}

/// Device-free core of one play request: draw the shifts, select the
/// samples, load and chain them, apply the pitch shift, and combine the entry
/// volume with the `VShift=` reduction into the event's linear volume.
fn resolve_entry_playback(
    entry: &SoundEntry,
    rng: &mut impl SampleRng,
    mut load: impl FnMut(&str) -> Option<DecodedAudio>,
) -> Option<ResolvedPlayback> {
    if entry.sounds.is_empty() {
        return None;
    }
    let shifts = PlayShifts::draw(entry, rng);
    let order = select_playout(entry, rng);
    let mut decoded: Option<DecodedAudio> = None;
    for index in order {
        let Some(name) = entry.sounds.get(index) else {
            continue;
        };
        let Some(clip) = load(name) else {
            continue;
        };
        match decoded.as_mut() {
            Some(chain) => chain.append(clip),
            None => decoded = Some(clip),
        }
    }
    let mut decoded = decoded?;
    decoded.sample_rate = shifts.shifted_sample_rate(decoded.sample_rate);
    Some(ResolvedPlayback {
        decoded,
        event_linear: combine_linear(entry.volume_linear, shifts.volume_linear()),
    })
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

    /// A scripted RNG: hands out the listed draws in order and records every
    /// request so a test can pin the native draw sequence.
    struct ScriptedRng {
        draws: Vec<i32>,
        requests: Vec<(i32, i32)>,
    }

    impl ScriptedRng {
        fn new(draws: &[i32]) -> Self {
            Self {
                draws: draws.iter().rev().copied().collect(),
                requests: Vec::new(),
            }
        }
    }

    impl SampleRng for ScriptedRng {
        fn ranged(&mut self, low: i32, high: i32) -> i32 {
            if low == high {
                return low;
            }
            self.requests.push((low, high));
            self.draws.pop().unwrap_or(low)
        }
    }

    /// `[SoundList]` is what registers an id (`VocClass::ReadSoundListINI @
    /// 0x007510D0`), so the fixture names `[T]` there the way the stock file
    /// names every sound.
    fn entry(body: &str) -> SoundEntry {
        let registry =
            SoundRegistry::from_ini(&IniFile::from_str(&format!("[SoundList]\n1=T\n{body}")));
        registry.get("T").expect("entry").clone()
    }

    const LISTENER_W: f32 = 1024.0;
    const LISTENER_H: f32 = 600.0;

    fn source(range_cells: i32, type_flags: u32, min_volume: f32) -> SpatialSource {
        SpatialSource {
            range_cells,
            type_flags,
            min_volume,
        }
    }

    fn gain(client_x: i32, client_y: i32, src: SpatialSource) -> Option<SpatialGain> {
        calc_volume_and_pan(client_x, client_y, LISTENER_W, LISTENER_H, src, false)
    }

    /// Vectors walked through the `0x00750AC0` transcription: a 1024x600
    /// tactical view (halfW 512, halfH 300, fullW 1024) and Range 10
    /// (maxRange 600).
    #[test]
    fn calc_volume_and_pan_screen_type_matches_the_native_transcription() {
        let screen = source(10, sound_type::SCREEN, 0.5);
        // View centre: on screen is full volume, centred pan.
        assert_eq!(
            gain(512, 300, screen),
            Some(SpatialGain {
                volume: 1.0,
                pan: PAN_CENTRE
            })
        );
        // Anywhere on screen is still full volume; pan follows the offset.
        let edge = gain(1023, 599, screen).unwrap();
        assert_eq!(edge.volume, 1.0);
        assert_eq!(edge.pan, ftol(511.0 * 8192.0 / 1024.0 + 8192.0));
        // 300 px right of the view edge: half volume, pan 8192 + 812 * 8.
        assert_eq!(
            gain(1324, 300, screen),
            Some(SpatialGain {
                volume: 0.5,
                pan: 14688
            })
        );
        // Y distances count double: 100 px below the view edge is 200.
        assert_eq!(gain(512, 700, screen).unwrap().volume, 400.0 / 600.0);
        // The larger axis wins: 200 px right (200) vs 150 px below (300).
        assert_eq!(gain(1224, 750, screen).unwrap().volume, 300.0 / 600.0);
        // Below the 0.05 cutoff is silent; just above is not.
        assert_eq!(gain(1600, 300, screen), None);
        assert_eq!(gain(1584, 300, screen).unwrap().volume, 40.0 / 600.0);
        // At and beyond maxRange the volume is exactly 0 (silent).
        assert_eq!(gain(1624, 300, screen), None);
        assert_eq!(gain(-1000, 300, screen), None);
        // A zero range is never audible.
        assert_eq!(gain(512, 300, source(0, sound_type::SCREEN, 0.0)), None);
    }

    #[test]
    fn calc_volume_and_pan_local_global_and_shroud_gates() {
        // LOCAL skips the half-view subtraction: 100 px from centre is 100.
        let local = source(10, sound_type::LOCAL, 0.5);
        assert_eq!(gain(612, 300, local).unwrap().volume, 500.0 / 600.0);
        // LOCAL with a large range reaches the pan clamps at both ends.
        let far = source(100, sound_type::LOCAL, 0.0);
        assert_eq!(gain(-1500, 300, far).unwrap().pan, 0);
        assert_eq!(gain(3000, 300, far).unwrap().pan, PAN_SCALE);
        // GLOBAL raises a 10/600 volume to the MinVolume floor; SCREEN alone
        // falls below the cutoff instead.
        let global = source(10, sound_type::SCREEN | sound_type::GLOBAL, 0.5);
        assert_eq!(gain(1614, 300, global).unwrap().volume, 0.5);
        assert_eq!(gain(1614, 300, source(10, sound_type::SCREEN, 0.5)), None);
        // GLOBAL keeps a louder computed volume.
        assert_eq!(gain(1324, 300, global).unwrap().volume, 0.5);
        assert_eq!(gain(1224, 300, global).unwrap().volume, 400.0 / 600.0);
        // SHROUD silences a shrouded cell only when the flag is set.
        let shroud = source(10, sound_type::SCREEN | sound_type::SHROUD, 0.5);
        assert_eq!(
            calc_volume_and_pan(512, 300, LISTENER_W, LISTENER_H, shroud, true),
            None
        );
        assert!(calc_volume_and_pan(512, 300, LISTENER_W, LISTENER_H, shroud, false).is_some());
        let unshrouded = source(10, sound_type::SCREEN | sound_type::UNSHROUD, 0.5);
        assert!(calc_volume_and_pan(512, 300, LISTENER_W, LISTENER_H, unshrouded, true).is_some());
    }

    /// Odd view widths put the centre on a half pixel; the distances are
    /// `ftol`-truncated before `abs`, the pan offset is not.
    #[test]
    fn calc_volume_and_pan_truncates_distances_like_ftol() {
        let src = source(10, sound_type::LOCAL, 0.0);
        // clientX - 511.5 = 0.5 -> ftol 0 -> full volume; pan keeps the 0.5.
        let g = calc_volume_and_pan(512, 300, 1023.0, LISTENER_H, src, false).unwrap();
        assert_eq!(g.volume, 1.0);
        assert_eq!(g.pan, ftol(0.5 * 8192.0 / 1023.0 + 8192.0));
        // clientX - 511.5 = -0.5 -> ftol 0 as well (truncation toward zero).
        let g = calc_volume_and_pan(511, 300, 1023.0, LISTENER_H, src, false).unwrap();
        assert_eq!(g.volume, 1.0);
        assert_eq!(g.pan, ftol(-0.5 * 8192.0 / 1023.0 + 8192.0));
    }

    fn zoom_listener(zoom: f32) -> SpatialListener {
        SpatialListener {
            tactical_width: LISTENER_W as i32,
            tactical_height: LISTENER_H as i32,
            origin_x: 0.0,
            origin_y: 0.0,
            zoom,
        }
    }

    /// The listener rect and the sound positions must stay in one frame at
    /// every zoom. VERA projects `device = (world - camera) * zoom`, so a
    /// 1024x600 device viewport covers `1024/zoom` x `600/zoom` world pixels;
    /// `SpatialListener::view_extent` is what puts the rect in the world frame
    /// the positions already use.
    ///
    /// gamemd has no zoom, so nothing here is a native golden. What the
    /// vectors pin is that zoom behaves like the one zoom-shaped thing gamemd
    /// *does* have — a resolution change, which alters how much world the view
    /// rect covers while `VocClass::CalcVolumeAndPan @ 0x00750AC0` keeps
    /// `Range * 0x3C` as a fixed cell distance.
    #[test]
    fn spatial_gain_measures_one_frame_at_every_zoom() {
        let src = source(10, sound_type::SCREEN, 0.5);
        // At zoom 1.0 the world frame *is* the device frame, so the listener
        // must reproduce the raw-device-size call bit for bit.
        for x in [0.0f32, 256.0, 512.0, 1324.0, 1584.0] {
            assert_eq!(
                spatial_gain(src, x, 300.0, &zoom_listener(1.0), false),
                calc_volume_and_pan(x as i32, 300, LISTENER_W, LISTENER_H, src, false),
                "zoom 1.0 must equal the native device-pixel call at x={x}"
            );
        }

        // `Range=10` is 600 world pixels — 10 cells — at every zoom. A sound
        // exactly 300 world pixels past the right edge of the visible area is
        // half volume whether the view covers 1024, 512 or 2048 world pixels.
        for (zoom, view_w, centre_y) in [
            (1.0f32, 1024.0f32, 300.0f32),
            (2.0, 512.0, 150.0),
            (0.5, 2048.0, 600.0),
        ] {
            let gain = spatial_gain(src, view_w + 300.0, centre_y, &zoom_listener(zoom), false)
                .unwrap_or_else(|| panic!("audible at zoom {zoom}"));
            assert_eq!(gain.volume, 0.5, "zoom {zoom} falloff");
        }

        // Pan is the position *within* the view, so a sound a quarter of the
        // way across the visible area sits at the same 6144 at every zoom —
        // and stays inside the view, hence full volume.
        for (zoom, view_w, centre_y) in [
            (1.0f32, 1024.0f32, 300.0f32),
            (2.0, 512.0, 150.0),
            (0.5, 2048.0, 600.0),
        ] {
            let gain = spatial_gain(src, view_w * 0.25, centre_y, &zoom_listener(zoom), false)
                .unwrap_or_else(|| panic!("audible at zoom {zoom}"));
            assert_eq!(
                (gain.volume, gain.pan),
                (1.0, 6144),
                "zoom {zoom} quarter-width pan"
            );
        }

        // The regression the frame mismatch hid: one fixed world position must
        // move with the zoom. Ignoring zoom made all three of these 0.5/14688.
        let fixed = (1324.0f32, 300.0f32);
        assert_eq!(
            spatial_gain(src, fixed.0, fixed.1, &zoom_listener(1.0), false),
            Some(SpatialGain {
                volume: 0.5,
                pan: 14688
            })
        );
        // Zoomed 2x the view covers 512x300 world px, so the same point is
        // 812 px outside it — past `Range * 60` and silent.
        assert_eq!(
            spatial_gain(src, fixed.0, fixed.1, &zoom_listener(2.0), false),
            None
        );
        // Zoomed out to 0.5x the view covers 2048x1200 world px, so the point
        // is on screen: full volume, panned by its offset from the centre.
        assert_eq!(
            spatial_gain(src, fixed.0, fixed.1, &zoom_listener(0.5), false),
            Some(SpatialGain {
                volume: 1.0,
                pan: 9392
            })
        );

        // A degenerate zoom must neither divide by zero (the VERA-internal
        // EPSILON floor) nor panic: the resulting half-view saturates the
        // `ftol` cast to `i32::MIN`, which `native_abs` wraps the way
        // `CDQ; XOR; SUB` does instead of trapping like `i32::abs`.
        assert!(spatial_gain(src, 0.0, 0.0, &zoom_listener(0.0), false).is_some());
    }

    #[test]
    fn spatial_listener_projects_world_pixels_to_client_points() {
        let listener = SpatialListener {
            tactical_width: LISTENER_W as i32,
            tactical_height: LISTENER_H as i32,
            origin_x: 1000.25,
            origin_y: 2000.0,
            zoom: 1.0,
        };
        assert_eq!(listener.client_point(1512.25, 2300.0), (512, 300));
        assert_eq!(listener.client_point(999.75, 1999.0), (0, -1));
        let src = source(10, sound_type::SCREEN, 0.5);
        assert_eq!(
            spatial_gain(src, 1512.25, 2300.0, &listener, false),
            Some(SpatialGain {
                volume: 1.0,
                pan: PAN_CENTRE
            })
        );
    }

    #[test]
    fn spatial_gain_volume_linear_is_ftol_capped_at_the_scale() {
        assert_eq!(SpatialGain::CENTRED_FULL.volume_linear(), VOLUME_SCALE);
        let half = SpatialGain {
            volume: 0.5,
            pan: PAN_CENTRE,
        };
        assert_eq!(half.volume_linear(), 8192);
        let odd = SpatialGain {
            volume: 400.0 / 600.0,
            pan: PAN_CENTRE,
        };
        assert_eq!(
            odd.volume_linear(),
            ftol(f64::from(400.0f32 / 600.0) * 16384.0)
        );
    }

    /// The `0x00816380` table (machine-read) against `round(1000 * log2(i /
    /// 100))` and the index arithmetic of `FUN_0040A6D0`.
    #[test]
    fn dsound_attenuation_table_and_index_follow_the_binary() {
        for (i, &entry) in DSOUND_ATTENUATION_TABLE.iter().enumerate().skip(1) {
            let expected = (1000.0 * (i as f64 / 100.0).log2()).round() as i16;
            assert_eq!(entry, expected, "table[{i}]");
        }
        assert_eq!(DSOUND_ATTENUATION_TABLE[0], -10000);
        assert_eq!(DSOUND_ATTENUATION_TABLE[100], 0);
        assert_eq!(native_volume_amplitude(VOLUME_SCALE), 1.0);
        assert!((native_volume_amplitude(8192) - 10f32.powf(-0.5)).abs() < 1e-6);
        assert_eq!(native_volume_amplitude(0), 0.0);
        // 16383 * 25 >> 12 = 99, so anything short of full scale loses 0.14 dB.
        assert!((native_volume_amplitude(16383) - 10f32.powf(-14.0 / 2000.0)).abs() < 1e-6);
        assert_eq!(combine_linear(13107, VOLUME_SCALE), 13107);
        assert_eq!(combine_linear(8192, 8192), 4096);
        assert_eq!(combine_linear(VOLUME_SCALE, VOLUME_SCALE), VOLUME_SCALE);
    }

    #[test]
    fn pan_channel_gains_attenuate_the_far_side_through_the_table() {
        assert_eq!(pan_channel_gains(PAN_CENTRE), (1.0, 1.0));
        // Full right: p = 100, the left channel takes table[0].
        assert_eq!(pan_channel_gains(PAN_SCALE), (0.0, 1.0));
        assert_eq!(pan_channel_gains(0), (1.0, 0.0));
        // 12288 -> 150 - 100 = 50 -> left attenuated by table[50] = -10 dB.
        let (left, right) = pan_channel_gains(12288);
        assert!((left - 10f32.powf(-0.5)).abs() < 1e-6);
        assert_eq!(right, 1.0);
        let (left, right) = pan_channel_gains(4096);
        assert_eq!(left, 1.0);
        assert!((right - 10f32.powf(-0.5)).abs() < 1e-6);
        let mut samples = vec![1.0, 1.0, 0.5, 0.5];
        apply_pan(&mut samples, 12288);
        assert!((samples[0] - 10f32.powf(-0.5)).abs() < 1e-6);
        assert_eq!(samples[1], 1.0);
        assert_eq!(samples[3], 0.5);
    }

    #[test]
    fn play_shifts_draw_fshift_vshift_then_predelay_in_native_order() {
        let e = entry(
            "[T]\nSounds=a\nFShift= -10 10\nVShift=20\nControl= random predelay\nDelay=0 400\n",
        );
        let mut rng = ScriptedRng::new(&[7, 13, 250]);
        let shifts = PlayShifts::draw(&e, &mut rng);
        assert_eq!(rng.requests, vec![(-10, 10), (0, 20), (0, 400)]);
        assert_eq!(shifts.frequency_pct, 107);
        assert_eq!(shifts.volume_shift_pct, 13);
        // 0x4000 - (13 << 14) / 100 = 16384 - 2129.
        assert_eq!(shifts.volume_linear(), 16384 - 2129);
        assert_eq!(shifts.shifted_sample_rate(22050), 22050 * 107 / 100);

        // AMBIENT pre-delays draw from 0x21 regardless of Delay.min.
        let ambient = entry("[T]\nSounds=a\nControl= loop ambient\nDelay=5000 8000\n");
        let mut rng = ScriptedRng::new(&[6000]);
        let shifts = PlayShifts::draw(&ambient, &mut rng);
        assert_eq!(rng.requests, vec![(0x21, 8000)]);
        assert_eq!(shifts.frequency_pct, 100);
        assert_eq!(shifts.volume_linear(), VOLUME_SCALE);
        assert_eq!(shifts.shifted_sample_rate(22050), 22050);

        // No shifts, no pre-delay control: nothing is drawn.
        let plain = entry("[T]\nSounds=a\nDelay=0 400\n");
        let mut rng = ScriptedRng::new(&[]);
        PlayShifts::draw(&plain, &mut rng);
        assert!(rng.requests.is_empty());
    }

    /// `Control=random` over three samples: one `RandomRanged(0, 2)` draw and
    /// the equal-bounds pick that follows draws nothing.
    #[test]
    fn select_playout_random_picks_one_body_sample() {
        let e = entry("[T]\nSounds=a b c\nControl=random\n");
        let mut rng = ScriptedRng::new(&[1]);
        assert_eq!(select_playout(&e, &mut rng), vec![1]);
        assert_eq!(rng.requests, vec![(0, 2)]);
    }

    /// Without `random` the first body sample always plays — there is no
    /// round-robin — and `all` plays the whole body in order.
    #[test]
    fn select_playout_sequential_forms_play_first_or_all() {
        let first = entry("[T]\nSounds=a b c\n");
        let mut rng = ScriptedRng::new(&[]);
        assert_eq!(select_playout(&first, &mut rng), vec![0]);
        assert_eq!(select_playout(&first, &mut rng), vec![0]);
        assert!(rng.requests.is_empty());

        let all = entry("[T]\nSounds=a b c\nControl=all\n");
        assert_eq!(select_playout(&all, &mut rng), vec![0, 1, 2]);
        assert!(rng.requests.is_empty());
    }

    /// `random all`: the body is loaded whole and played in pick-and-remove
    /// order, `RandomRanged(0, remaining - 1)` each step.
    #[test]
    fn select_playout_random_all_shuffles_by_pick_and_remove() {
        let e = entry("[T]\nSounds=a b c\nControl= random all\n");
        let mut rng = ScriptedRng::new(&[2, 0]);
        assert_eq!(select_playout(&e, &mut rng), vec![2, 0, 1]);
        assert_eq!(rng.requests, vec![(0, 2), (0, 1)]);
    }

    /// Attack/decay envelopes: a random attack sample first, the body, and a
    /// random decay sample last, with the native draw order.
    #[test]
    fn select_playout_attack_body_decay_order_and_draws() {
        let e = entry(
            "[T]\nSounds=a1 a2 a3 b1 b2 d1 d2 d3\nControl= random all attack decay\nAttack=3\nDecay=3\n",
        );
        let mut rng = ScriptedRng::new(&[1, 6, 1]);
        assert_eq!(select_playout(&e, &mut rng), vec![1, 4, 3, 6]);
        assert_eq!(rng.requests, vec![(0, 2), (5, 7), (0, 1)]);

        let single =
            entry("[T]\nSounds=a b1 b2 d\nControl= random attack decay\nAttack=1\nDecay=1\n");
        // Attack 1 and Decay 1 are equal-bounds draws: only the body draws.
        let mut rng = ScriptedRng::new(&[2]);
        assert_eq!(select_playout(&single, &mut rng), vec![0, 2, 3]);
        assert_eq!(rng.requests, vec![(1, 2)]);
    }

    fn clip(value: f32, frames: usize, sample_rate: u32) -> DecodedAudio {
        DecodedAudio {
            samples: vec![value; frames * 2],
            sample_rate,
            channels: 2,
        }
    }

    #[test]
    fn resolve_entry_playback_chains_samples_and_applies_the_shifts() {
        let e = entry("[T]\nSounds=a b c\nVolume=80\nControl= all\nFShift=10 10\nVShift=50\n");
        let mut rng = ScriptedRng::new(&[50]);
        let resolved = resolve_entry_playback(&e, &mut rng, |name| match name {
            "a" => Some(clip(0.1, 2, 22050)),
            "b" => None,
            "c" => Some(clip(0.3, 1, 22050)),
            _ => unreachable!(),
        })
        .expect("resolved");
        assert_eq!(resolved.decoded.samples, vec![0.1, 0.1, 0.1, 0.1, 0.3, 0.3]);
        assert_eq!(resolved.decoded.sample_rate, 22050 * 110 / 100);
        // Volume=80 -> 13107; VShift draw 50 -> 16384 - 8192; combined >> 14.
        assert_eq!(resolved.event_linear, combine_linear(13107, 8192));
        assert_eq!(rng.requests, vec![(0, 50)]);

        // A rate mismatch keeps the first clip only.
        let mut rng = ScriptedRng::new(&[0]);
        let resolved = resolve_entry_playback(&e, &mut rng, |name| match name {
            "a" => Some(clip(0.1, 1, 22050)),
            _ => Some(clip(0.2, 1, 11025)),
        })
        .expect("resolved");
        assert_eq!(resolved.decoded.samples, vec![0.1, 0.1]);

        // Nothing loadable resolves to nothing.
        assert!(resolve_entry_playback(&e, &mut rng, |_| None).is_none());
    }

    #[test]
    fn cloak_sound_registered_resolution_rejects_raw_sample_and_invalid_names() {
        let registry = SoundRegistry::from_ini(&IniFile::from_str(
            "[SoundList]\n1=NavalUnitEmerge\n[NavalUnitEmerge]\nSounds=vnavupa\nVolume=55\n",
        ));
        let entry = registered_entry("NavalUnitEmerge", &registry)
            .expect("the retail Voc/event identity resolves to its registered entry");
        assert_eq!(entry.sounds, vec!["vnavupa"]);
        assert_eq!(entry.volume_linear, 9011); // ftol(0.55f * 16384)
        for invalid in ["", "MissingCloakEvent", "vnavupa"] {
            assert!(
                registered_entry(invalid, &registry).is_none(),
                "an invalid Voc identity must not become a raw-bag fallback: {invalid}"
            );
        }
    }

    #[test]
    fn sfx_rng_honours_inclusive_bounds_and_equal_bounds() {
        let mut rng = SfxRng::seeded(42);
        assert_eq!(rng.ranged(5, 5), 5);
        for _ in 0..1000 {
            let draw = rng.ranged(0, 2);
            assert!((0..=2).contains(&draw));
            let reversed = rng.ranged(3, -3);
            assert!((-3..=3).contains(&reversed));
        }
    }

    fn test_decoded_audio() -> DecodedAudio {
        DecodedAudio {
            samples: vec![0.0, 0.0],
            sample_rate: 22_050,
            channels: 2,
        }
    }

    fn test_output_scales(
        sound_volume: f32,
        voice_volume: f32,
        lifecycle_scale: f32,
        focus_output_scale: f32,
    ) -> SfxOutputScales {
        SfxOutputScales {
            sound_volume,
            voice_volume,
            lifecycle_scale,
            focus_output_scale,
        }
    }

    #[test]
    fn gsi_01_02_focus_gate_restores_distinct_live_sfx_gains_absolutely() {
        let inactive = test_output_scales(1.0, 0.4, 0.6, 0.0);
        let quiet = prepare_normal_sfx_output(test_decoded_audio(), 3277, inactive);
        let loud = prepare_normal_sfx_output(test_decoded_audio(), 12288, inactive);

        assert_eq!(quiet.initial_volume, 0.0);
        assert_eq!(loud.initial_volume, 0.0);
        let active = test_output_scales(1.0, 0.4, 0.6, 1.0);
        assert!(
            (quiet.gain.effective(active) - native_volume_amplitude(3277) * 0.6).abs()
                < f32::EPSILON
        );
        assert!(
            (loud.gain.effective(active) - native_volume_amplitude(12288) * 0.6).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn gsi_01_02_sfx_armed_while_inactive_retains_gain_for_restore() {
        // Construction while inactive still creates/starts the Player in the
        // production path; only this independently retained output gain is 0.
        let armed_while_inactive = prepare_normal_sfx_output(
            test_decoded_audio(),
            5734,
            test_output_scales(1.0, 0.4, 1.0, 0.0),
        );

        assert_eq!(armed_while_inactive.initial_volume, 0.0);
        assert!(
            (armed_while_inactive
                .gain
                .effective(test_output_scales(1.0, 0.4, 1.0, 1.0))
                - native_volume_amplitude(5734))
            .abs()
                < f32::EPSILON
        );
    }

    /// The user master is chained into the linear product before the
    /// DirectSound curve, so half master is -10 dB, not half amplitude.
    #[test]
    fn options_profile_production_routes_sound_and_direct_voice_independently() {
        let half = native_volume_amplitude(8192);
        for (sound_volume, voice_volume, expected_sound, expected_voice) in
            [(0.0, 1.0, 0.0, half), (1.0, 0.0, half, 0.0)]
        {
            let scales = test_output_scales(sound_volume, voice_volume, 1.0, 1.0);
            let sound = prepare_normal_sfx_output(test_decoded_audio(), 8192, scales);
            let direct_voice = prepare_direct_voice_output(test_decoded_audio(), 8192, scales);

            assert_eq!(sound.initial_volume, expected_sound);
            assert_eq!(direct_voice.initial_volume, expected_voice);
        }
        let full = prepare_normal_sfx_output(
            test_decoded_audio(),
            VOLUME_SCALE,
            test_output_scales(0.5, 1.0, 1.0, 1.0),
        );
        assert_eq!(full.initial_volume, half);
    }

    #[test]
    fn options_profile_queued_eva_uses_current_voice_master_at_dequeue() {
        // QueuedVoice retains no master snapshot. The scales supplied by the
        // production dequeue seam alone decide the startup volume.
        let queued_for_voice_enabled_dequeue =
            QueuedVoice::new("eva-a".to_string(), test_decoded_audio(), 13107);
        let (sound_id, started_with_voice_enabled) = queued_for_voice_enabled_dequeue
            .prepare_for_dequeue(test_output_scales(0.0, 1.0, 1.0, 1.0));
        assert_eq!(sound_id, "eva-a");
        assert!(
            (started_with_voice_enabled.initial_volume - native_volume_amplitude(13107)).abs()
                < f32::EPSILON
        );

        let queued_for_voice_muted_dequeue =
            QueuedVoice::new("eva-b".to_string(), test_decoded_audio(), 13107);
        let (sound_id, started_with_voice_muted) = queued_for_voice_muted_dequeue
            .prepare_for_dequeue(test_output_scales(1.0, 0.0, 1.0, 1.0));
        assert_eq!(sound_id, "eva-b");
        assert_eq!(started_with_voice_muted.initial_volume, 0.0);
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
