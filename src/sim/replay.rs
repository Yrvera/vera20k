//! Native recording stream plus deterministic diagnostic-log helpers.
//!
//! Retail recordings are not save games. They contain a fixed startup header,
//! then presentation/sync records plus command batches at the command-transfer
//! rungs selected by the running session. Playback rebuilds the scenario
//! normally from the recorded header and consumes both record kinds at their
//! original scheduler rungs.
//!
//! [`ReplayLog`] remains the richer Rust-only command/hash diagnostic used by
//! parity tests. It is deliberately separate from [`NativeReplay`].

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rules::ruleset::RuleSet;
use crate::sim::command::{COMMAND_RECORD_LEN, CommandEnvelope, CommandRecord};
use crate::sim::pathfinding::PathGrid;
use crate::sim::world::{Simulation, TickLane, TriggerInputs};

/// Value written by the retail executable at the start of every recording.
pub const NATIVE_REPLAY_VERSION: u32 = 10;
/// Exact byte width of the seven-field retail recording header.
pub const NATIVE_REPLAY_HEADER_LEN: usize = 0x1d0;
/// Fixed scenario-name buffer stored in the native header.
pub const NATIVE_REPLAY_SCENARIO_NAME_LEN: usize = 0x104;
/// Raw retail options block stored at the end of the native header.
pub const NATIVE_REPLAY_OPTIONS_LEN: usize = 0xb8;

/// Recording-mode bits used by the retail main loop.
pub const REPLAY_FLAG_RECORD: u32 = 0x01;
pub const REPLAY_FLAG_PLAYBACK: u32 = 0x02;
pub const REPLAY_FLAG_AVAILABLE: u32 = 0x04;

/// Malformed native recording data.
#[derive(Debug, Error)]
pub enum NativeReplayError {
    #[error("native replay header must be {expected} bytes, got {actual}")]
    HeaderLength { expected: usize, actual: usize },
    #[error("native replay ended at byte {offset} while reading {field}")]
    Truncated { offset: usize, field: &'static str },
    #[error("native replay {field} count does not fit in i32: {count}")]
    CountOverflow { field: &'static str, count: usize },
    #[error("native replay command record is malformed: {0}")]
    Command(#[from] crate::sim::command::CommandRecordError),
}

/// Exact seven-field startup header used by retail recordings.
///
/// Three fields do not yet have a verified semantic name, so they remain
/// literal rather than acquiring an inferred meaning. The raw options block is
/// likewise preserved byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayHeader {
    pub version: u32,
    pub seed: u32,
    pub scenario_value: u32,
    scenario_name: [u8; NATIVE_REPLAY_SCENARIO_NAME_LEN],
    pub session_value: u32,
    pub special_flags: u32,
    options: [u8; NATIVE_REPLAY_OPTIONS_LEN],
}

impl NativeReplayHeader {
    /// Build a recording header for a normally initialized scenario.
    pub fn new(seed: u32, scenario_name: &str) -> Self {
        let mut header = Self {
            version: NATIVE_REPLAY_VERSION,
            seed,
            scenario_value: 0,
            scenario_name: [0; NATIVE_REPLAY_SCENARIO_NAME_LEN],
            session_value: 0,
            special_flags: 0,
            options: [0; NATIVE_REPLAY_OPTIONS_LEN],
        };
        header.set_scenario_name(scenario_name);
        header
    }

    /// Decode the fixed native header without assigning meaning to opaque data.
    pub fn decode(bytes: &[u8]) -> std::result::Result<Self, NativeReplayError> {
        if bytes.len() != NATIVE_REPLAY_HEADER_LEN {
            return Err(NativeReplayError::HeaderLength {
                expected: NATIVE_REPLAY_HEADER_LEN,
                actual: bytes.len(),
            });
        }

        let mut scenario_name = [0; NATIVE_REPLAY_SCENARIO_NAME_LEN];
        scenario_name.copy_from_slice(&bytes[0x0c..0x110]);
        let mut options = [0; NATIVE_REPLAY_OPTIONS_LEN];
        options.copy_from_slice(&bytes[0x118..0x1d0]);

        Ok(Self {
            version: u32::from_le_bytes(bytes[0x00..0x04].try_into().unwrap()),
            seed: u32::from_le_bytes(bytes[0x04..0x08].try_into().unwrap()),
            scenario_value: u32::from_le_bytes(bytes[0x08..0x0c].try_into().unwrap()),
            scenario_name,
            session_value: u32::from_le_bytes(bytes[0x110..0x114].try_into().unwrap()),
            special_flags: u32::from_le_bytes(bytes[0x114..0x118].try_into().unwrap()),
            options,
        })
    }

    /// Encode the seven native writes as one contiguous header.
    pub fn encode(&self) -> [u8; NATIVE_REPLAY_HEADER_LEN] {
        let mut bytes = [0; NATIVE_REPLAY_HEADER_LEN];
        bytes[0x00..0x04].copy_from_slice(&self.version.to_le_bytes());
        bytes[0x04..0x08].copy_from_slice(&self.seed.to_le_bytes());
        bytes[0x08..0x0c].copy_from_slice(&self.scenario_value.to_le_bytes());
        bytes[0x0c..0x110].copy_from_slice(&self.scenario_name);
        bytes[0x110..0x114].copy_from_slice(&self.session_value.to_le_bytes());
        bytes[0x114..0x118].copy_from_slice(&self.special_flags.to_le_bytes());
        bytes[0x118..0x1d0].copy_from_slice(&self.options);
        bytes
    }

    /// Scenario file name up to the first native NUL terminator.
    pub fn scenario_name(&self) -> String {
        let len = self
            .scenario_name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.scenario_name.len());
        String::from_utf8_lossy(&self.scenario_name[..len]).into_owned()
    }

    /// Replace the native C-string buffer, truncating to leave a NUL byte.
    pub fn set_scenario_name(&mut self, name: &str) {
        self.scenario_name.fill(0);
        let len = name
            .as_bytes()
            .len()
            .min(NATIVE_REPLAY_SCENARIO_NAME_LEN - 1);
        self.scenario_name[..len].copy_from_slice(&name.as_bytes()[..len]);
    }

    pub fn scenario_name_bytes(&self) -> &[u8; NATIVE_REPLAY_SCENARIO_NAME_LEN] {
        &self.scenario_name
    }

    pub fn options_bytes(&self) -> &[u8; NATIVE_REPLAY_OPTIONS_LEN] {
        &self.options
    }

    pub fn set_options_bytes(&mut self, options: [u8; NATIVE_REPLAY_OPTIONS_LEN]) {
        self.options = options;
    }
}

/// Per-frame replay data read before the active-object pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayPresentation {
    /// Opaque eight-byte scenario sync value written by the recording path.
    pub scenario_hash: [u8; 8],
    /// Additive checksum recorded from the selection vector.
    pub selection_checksum: u32,
    /// Packed selection identities in current-selection order.
    pub selected_objects: Vec<u32>,
    /// Two raw cursor/mouse dwords consumed by the playback render path.
    pub cursor: [u32; 2],
}

impl NativeReplayPresentation {
    pub fn new(scenario_hash: [u8; 8], selected_objects: Vec<u32>, cursor: [u32; 2]) -> Self {
        let selection_checksum = selection_checksum(&selected_objects);
        Self {
            scenario_hash,
            selection_checksum,
            selected_objects,
            cursor,
        }
    }

    /// Capture the two cursor words and clear their live accumulator, matching
    /// the recording path's post-write reset.
    pub fn record(
        scenario_hash: [u8; 8],
        selected_objects: Vec<u32>,
        live_cursor: &mut [u32; 2],
    ) -> Self {
        let cursor = *live_cursor;
        *live_cursor = [0; 2];
        Self::new(scenario_hash, selected_objects, cursor)
    }

    /// Whether the live selection matches the recorded additive identity sum.
    ///
    /// Retail clears selection on a mismatch, then consumes the recorded
    /// identities to restore the frame's selection.
    pub fn selection_matches(&self, live_selected_objects: &[u32]) -> bool {
        selection_checksum(live_selected_objects) == self.selection_checksum
    }
}

/// One complete native recording frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplayFrame {
    pub presentation: NativeReplayPresentation,
    /// A command-transfer record when that scheduler rung wrote one.
    ///
    /// `Some(empty)` is distinct from `None`: the former writes a zero count;
    /// the latter writes no command bytes before the next presentation record.
    pub commands: Option<Vec<CommandRecord>>,
}

impl NativeReplayFrame {
    /// Construct the frame exactly as the recorder does: only unprocessed
    /// synchronized commands stamped for the current frame are emitted without
    /// sorting.
    pub fn record<'a>(
        presentation: NativeReplayPresentation,
        current_frame: i32,
        write_command_batch: bool,
        do_list: impl IntoIterator<Item = &'a CommandRecord>,
    ) -> Self {
        let commands = write_command_batch.then(|| {
            do_list
                .into_iter()
                .filter(|record| record.frame_stamp() == current_frame && !record.is_processed())
                .cloned()
                .collect()
        });
        Self {
            presentation,
            commands,
        }
    }
}

/// Exact native recording document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReplay {
    pub header: NativeReplayHeader,
    pub frames: Vec<NativeReplayFrame>,
}

impl NativeReplay {
    pub fn new(header: NativeReplayHeader) -> Self {
        Self {
            header,
            frames: Vec::new(),
        }
    }

    pub fn encode(&self) -> std::result::Result<Vec<u8>, NativeReplayError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.header.encode());
        for frame in &self.frames {
            encode_frame(frame, &mut bytes)?;
        }
        Ok(bytes)
    }

    /// Decode using the running session's command-transfer schedule.
    ///
    /// The native stream has no tag for an omitted command batch. Its presence
    /// is known from session mode, frame, and network cadence, so the scheduler
    /// supplies that decision after each presentation record is decoded.
    pub fn decode_with_command_schedule(
        bytes: &[u8],
        mut has_command_batch: impl FnMut(usize, &NativeReplayPresentation) -> bool,
    ) -> std::result::Result<Self, NativeReplayError> {
        if bytes.len() < NATIVE_REPLAY_HEADER_LEN {
            return Err(NativeReplayError::HeaderLength {
                expected: NATIVE_REPLAY_HEADER_LEN,
                actual: bytes.len(),
            });
        }
        let header = NativeReplayHeader::decode(&bytes[..NATIVE_REPLAY_HEADER_LEN])?;
        let mut cursor = NATIVE_REPLAY_HEADER_LEN;
        let mut frames = Vec::new();
        while cursor < bytes.len() {
            let presentation = decode_presentation(bytes, &mut cursor)?;
            let commands = if has_command_batch(frames.len(), &presentation) {
                Some(decode_command_batch(bytes, &mut cursor)?)
            } else {
                None
            };
            frames.push(NativeReplayFrame {
                presentation,
                commands,
            });
        }
        Ok(Self { header, frames })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let bytes = self.encode().context("Failed to encode native replay")?;
        std::fs::write(path, bytes)
            .with_context(|| format!("Failed to write native replay: {}", path.display()))
    }

    pub fn load_with_command_schedule(
        path: &Path,
        has_command_batch: impl FnMut(usize, &NativeReplayPresentation) -> bool,
    ) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read native replay: {}", path.display()))?;
        Self::decode_with_command_schedule(&bytes, has_command_batch)
            .context("Failed to decode native replay")
    }

    /// Initialize playback through the normal scenario-loading owner.
    ///
    /// No snapshot or serialized active-object vector is accepted here. The
    /// caller must rebuild the scenario from the header, after which frames are
    /// exposed in stream order for the scheduler's presentation and command
    /// rungs.
    pub fn initialize_playback<S, E>(
        &self,
        initialize_scenario: impl FnOnce(&NativeReplayHeader) -> std::result::Result<S, E>,
    ) -> std::result::Result<NativeReplayPlayback<'_, S>, E> {
        let state = initialize_scenario(&self.header)?;
        Ok(NativeReplayPlayback {
            state,
            frames: self.frames.iter(),
            pending_tail: None,
        })
    }
}

/// A normally reinitialized scenario consuming native frames in file order.
pub struct NativeReplayPlayback<'a, S> {
    pub state: S,
    frames: std::slice::Iter<'a, NativeReplayFrame>,
    pending_tail: Option<&'a NativeReplayFrame>,
}

impl<'a, S> NativeReplayPlayback<'a, S> {
    /// Read the presentation/sync record at the pre-object scheduler rung.
    pub fn begin_frame(&mut self) -> Option<&'a NativeReplayPresentation> {
        assert!(
            self.pending_tail.is_none(),
            "native replay frame tail must be consumed before beginning another frame"
        );
        self.pending_tail = self.frames.next();
        self.pending_tail.map(|frame| &frame.presentation)
    }

    /// Read and admit this frame's command batch at the late command rung.
    pub fn finish_frame(&mut self) -> Option<&'a [CommandRecord]> {
        self.pending_tail
            .take()
            .expect("native replay frame must begin before its tail is consumed")
            .commands
            .as_deref()
    }
}

/// Native wrapping selection checksum.
pub fn selection_checksum(selected_objects: &[u32]) -> u32 {
    selected_objects
        .iter()
        .fold(0_u32, |sum, packed| sum.wrapping_add(*packed))
}

/// Pack the value written for one selected object.
///
/// A zero kind is the native null sentinel. Other kinds occupy the high byte
/// and the object's heap-pool identity (or the kind-specific raw value) is
/// truncated to 24 bits.
pub fn pack_selection_identity(kind: u8, raw_value: u32) -> u32 {
    if kind == 0 {
        u32::MAX
    } else {
        (u32::from(kind) << 24) | (raw_value & 0x00ff_ffff)
    }
}

fn encode_frame(
    frame: &NativeReplayFrame,
    bytes: &mut Vec<u8>,
) -> std::result::Result<(), NativeReplayError> {
    bytes.extend_from_slice(&frame.presentation.scenario_hash);
    let selection_count =
        i32::try_from(frame.presentation.selected_objects.len()).map_err(|_| {
            NativeReplayError::CountOverflow {
                field: "selection",
                count: frame.presentation.selected_objects.len(),
            }
        })?;
    bytes.extend_from_slice(&selection_count.to_le_bytes());
    bytes.extend_from_slice(&frame.presentation.selection_checksum.to_le_bytes());
    for selected in &frame.presentation.selected_objects {
        bytes.extend_from_slice(&selected.to_le_bytes());
    }
    for cursor_word in frame.presentation.cursor {
        bytes.extend_from_slice(&cursor_word.to_le_bytes());
    }

    if let Some(commands) = &frame.commands {
        let command_count =
            i32::try_from(commands.len()).map_err(|_| NativeReplayError::CountOverflow {
                field: "command",
                count: commands.len(),
            })?;
        bytes.extend_from_slice(&command_count.to_le_bytes());
        for command in commands {
            bytes.extend_from_slice(command.as_bytes());
        }
    }
    Ok(())
}

fn decode_presentation(
    bytes: &[u8],
    cursor: &mut usize,
) -> std::result::Result<NativeReplayPresentation, NativeReplayError> {
    let scenario_hash = take::<8>(bytes, cursor, "scenario hash")?;
    let selection_count = i32::from_le_bytes(take::<4>(bytes, cursor, "selection count")?);
    // Retail enters the selection loop only when 0 < count. Corrupt negative
    // values therefore consume no object records, exactly like zero.
    let selection_count = usize::try_from(selection_count).unwrap_or(0);
    let selection_checksum = u32::from_le_bytes(take::<4>(bytes, cursor, "selection checksum")?);
    let mut selected_objects = Vec::with_capacity(selection_count);
    for _ in 0..selection_count {
        selected_objects.push(u32::from_le_bytes(take::<4>(
            bytes,
            cursor,
            "selected object",
        )?));
    }
    let cursor_words = [
        u32::from_le_bytes(take::<4>(bytes, cursor, "cursor x")?),
        u32::from_le_bytes(take::<4>(bytes, cursor, "cursor y")?),
    ];

    Ok(NativeReplayPresentation {
        scenario_hash,
        selection_checksum,
        selected_objects,
        cursor: cursor_words,
    })
}

fn decode_command_batch(
    bytes: &[u8],
    cursor: &mut usize,
) -> std::result::Result<Vec<CommandRecord>, NativeReplayError> {
    let command_count = i32::from_le_bytes(take::<4>(bytes, cursor, "command count")?);
    // The native command loop has the same strictly-positive gate.
    let command_count = usize::try_from(command_count).unwrap_or(0);
    let mut commands = Vec::with_capacity(command_count);
    for _ in 0..command_count {
        let record = take::<COMMAND_RECORD_LEN>(bytes, cursor, "command record")?;
        commands.push(CommandRecord::admit_exact(&record)?);
    }
    Ok(commands)
}

fn take<const N: usize>(
    bytes: &[u8],
    cursor: &mut usize,
    field: &'static str,
) -> std::result::Result<[u8; N], NativeReplayError> {
    let end = cursor
        .checked_add(N)
        .filter(|end| *end <= bytes.len())
        .ok_or(NativeReplayError::Truncated {
            offset: *cursor,
            field,
        })?;
    let value = bytes[*cursor..end].try_into().unwrap();
    *cursor = end;
    Ok(value)
}

/// Header for the Rust-only deterministic diagnostic log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub version: u32,
    pub tick_hz: u32,
    pub seed: u64,
    pub map_name: String,
    pub rules_hash: u64,
}

/// One tick in the Rust-only deterministic diagnostic log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTick {
    pub tick: u64,
    pub commands: Vec<CommandEnvelope>,
    pub state_hash: u64,
}

/// Rich Rust-only command/hash log used by tests and diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayLog {
    pub header: ReplayHeader,
    pub ticks: Vec<ReplayTick>,
}

impl ReplayLog {
    pub fn new(header: ReplayHeader) -> Self {
        Self {
            header,
            ticks: Vec::new(),
        }
    }

    pub fn record_tick(&mut self, tick: u64, commands: Vec<CommandEnvelope>, state_hash: u64) {
        self.ticks.push(ReplayTick {
            tick,
            commands,
            state_hash,
        });
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .context("Failed to serialize deterministic diagnostic log to JSON")?;
        std::fs::write(path, bytes)
            .with_context(|| format!("Failed to write diagnostic log: {}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read diagnostic log: {}", path.display()))?;
        serde_json::from_slice(&bytes).context("Failed to parse deterministic diagnostic JSON")
    }
}

/// Headless runner for the Rust-only deterministic diagnostic log.
pub struct ReplayRunner;

impl ReplayRunner {
    /// Re-run diagnostic-log ticks and return the resulting hash timeline.
    pub fn run(
        sim: &mut Simulation,
        replay: &ReplayLog,
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        tick_ms: u32,
    ) -> Vec<u64> {
        Self::run_with_overlay_registry(sim, replay, rules, height_map, path_grid, None, tick_ms)
    }

    /// Re-run diagnostic-log ticks with the static overlay type registry used
    /// by the recorded map. Overlay-backed simulation authority (including
    /// miner resource queries) must see the same registry on record and replay.
    #[allow(clippy::too_many_arguments)]
    pub fn run_with_overlay_registry(
        sim: &mut Simulation,
        replay: &ReplayLog,
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
    ) -> Vec<u64> {
        Self::run_master_frame(
            sim,
            replay,
            rules,
            height_map,
            path_grid,
            overlay_registry,
            tick_ms,
            None,
        )
    }

    /// Replay through the same master-frame admission used by gameplay.
    ///
    /// Diagnostic logs own commands and hashes, while the caller owns static
    /// map trigger definitions. This keeps presentation-free replay faithful
    /// to the YR LogicClass trigger rung without serializing map data twice.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn run_master_frame(
        sim: &mut Simulation,
        replay: &ReplayLog,
        rules: Option<&RuleSet>,
        height_map: &BTreeMap<(u16, u16), u8>,
        path_grid: Option<&PathGrid>,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        tick_ms: u32,
        trigger_inputs: Option<TriggerInputs<'_>>,
    ) -> Vec<u64> {
        // The diagnostic playback must be constructed from the recorded seed.
        // A sim seeded
        // differently than the header it replays is a guaranteed silent
        // divergence.
        debug_assert_eq!(
            sim.session.seed, replay.header.seed,
            "replay playback sim must be constructed from header.seed"
        );
        // A replay recorded under a different rules set (mod, or a map with
        // value overrides) desyncs silently. Surface it. `rules_hash == 0` is
        // the "not stamped" sentinel (test/headless paths), so skip those.
        if let Some(rules) = rules {
            let current = rules.source_ini_hash();
            if replay.header.rules_hash != 0 && replay.header.rules_hash != current {
                log::warn!(
                    "replay rules_hash mismatch (header {:#018x} vs current \
                     {:#018x}) — recorded under a different rules set; \
                     playback will desync",
                    replay.header.rules_hash,
                    current
                );
            }
        }
        let mut hashes: Vec<u64> = Vec::with_capacity(replay.ticks.len());
        for entry in &replay.ticks {
            let result = sim.advance_master_frame(
                &entry.commands,
                rules,
                height_map,
                path_grid,
                overlay_registry,
                tick_ms,
                TickLane::Ordinary,
                None,
                trigger_inputs,
            );
            // Replay has no app layer to consume presentation-only trigger effects.
            let _ = sim.drain_trigger_effects();
            hashes.push(result.state_hash);
        }
        hashes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::command::Command;

    #[test]
    fn native_header_uses_exact_seven_field_layout() {
        let mut header = NativeReplayHeader::new(0x1234_5678, "maps\\test.map");
        header.scenario_value = 0xaabb_ccdd;
        header.session_value = 0x0102_0304;
        header.special_flags = 0x5566_7788;
        header.set_options_bytes([0x5a; NATIVE_REPLAY_OPTIONS_LEN]);

        let bytes = header.encode();
        assert_eq!(bytes.len(), 0x1d0);
        assert_eq!(&bytes[0x00..0x04], &10_u32.to_le_bytes());
        assert_eq!(&bytes[0x04..0x08], &0x1234_5678_u32.to_le_bytes());
        assert_eq!(&bytes[0x08..0x0c], &0xaabb_ccdd_u32.to_le_bytes());
        assert_eq!(&bytes[0x0c..0x19], b"maps\\test.map");
        assert_eq!(bytes[0x19], 0);
        assert!(bytes[0x1a..0x110].iter().all(|byte| *byte == 0));
        assert_eq!(&bytes[0x110..0x114], &0x0102_0304_u32.to_le_bytes());
        assert_eq!(&bytes[0x114..0x118], &0x5566_7788_u32.to_le_bytes());
        assert!(bytes[0x118..0x1d0].iter().all(|byte| *byte == 0x5a));

        assert_eq!(NativeReplayHeader::decode(&bytes).unwrap(), header);
        assert_eq!(header.scenario_name(), "maps\\test.map");
    }

    #[test]
    fn native_frame_layout_and_unknown_command_bytes_round_trip() {
        let mut unknown = [0x7b; COMMAND_RECORD_LEN];
        unknown[0] = 0xfe;
        unknown[1] = 0xa4;
        unknown[2] = 3;
        unknown[3..7].copy_from_slice(&17_i32.to_le_bytes());
        let command = CommandRecord::decode_exact(&unknown).unwrap();

        let presentation = NativeReplayPresentation::new(
            [1, 2, 3, 4, 5, 6, 7, 8],
            vec![0x3400_0001, 0x3400_0002],
            [0x1122_3344, 0x5566_7788],
        );
        let frame = NativeReplayFrame::record(presentation, 17, true, [&command]);
        let replay = NativeReplay {
            header: NativeReplayHeader::new(9, "x.map"),
            frames: vec![frame],
        };

        let bytes = replay.encode().unwrap();
        assert_eq!(&bytes[0x1d0..0x1d8], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&bytes[0x1d8..0x1dc], &2_i32.to_le_bytes());
        assert_eq!(&bytes[0x1dc..0x1e0], &0x6800_0003_u32.to_le_bytes());
        assert_eq!(&bytes[0x1e0..0x1e4], &0x3400_0001_u32.to_le_bytes());
        assert_eq!(&bytes[0x1e4..0x1e8], &0x3400_0002_u32.to_le_bytes());
        assert_eq!(&bytes[0x1e8..0x1ec], &0x1122_3344_u32.to_le_bytes());
        assert_eq!(&bytes[0x1ec..0x1f0], &0x5566_7788_u32.to_le_bytes());
        assert_eq!(&bytes[0x1f0..0x1f4], &1_i32.to_le_bytes());
        assert_eq!(&bytes[0x1f4..0x263], &unknown);
        assert_eq!(
            NativeReplay::decode_with_command_schedule(&bytes, |_, _| true).unwrap(),
            replay
        );
    }

    #[test]
    fn native_recording_filters_due_unprocessed_commands_in_queue_order() {
        let due_a = CommandRecord::encode(4, 0, 7, &[1]).unwrap();
        let future = CommandRecord::encode(5, 0, 8, &[2]).unwrap();
        let mut processed = CommandRecord::encode(6, 0, 7, &[3]).unwrap();
        processed.mark_processed();
        let due_b = CommandRecord::encode(7, 0, 7, &[4]).unwrap();

        let frame = NativeReplayFrame::record(
            NativeReplayPresentation::new([0; 8], Vec::new(), [0; 2]),
            7,
            true,
            [&due_a, &future, &processed, &due_b],
        );

        assert_eq!(frame.commands, Some(vec![due_a, due_b]));
    }

    #[test]
    fn native_selection_checksum_wraps_and_playback_starts_from_header() {
        assert_eq!(selection_checksum(&[u32::MAX, 2]), 1);
        assert_eq!(pack_selection_identity(0, 123), u32::MAX);
        assert_eq!(pack_selection_identity(0x34, 0xff12_3456), 0x3412_3456);

        let mut live_cursor = [44, 55];
        let captured = NativeReplayPresentation::record([0; 8], vec![1], &mut live_cursor);
        assert_eq!(captured.cursor, [44, 55]);
        assert_eq!(live_cursor, [0, 0]);

        let replay = NativeReplay {
            header: NativeReplayHeader::new(0xdead_beef, "arena.map"),
            frames: vec![
                NativeReplayFrame::record(
                    NativeReplayPresentation::new([0; 8], vec![1], [0; 2]),
                    0,
                    true,
                    std::iter::empty::<&CommandRecord>(),
                ),
                NativeReplayFrame::record(
                    NativeReplayPresentation::new([0; 8], vec![2], [0; 2]),
                    1,
                    true,
                    std::iter::empty::<&CommandRecord>(),
                ),
            ],
        };
        let mut playback = replay
            .initialize_playback(|header| {
                Ok::<_, ()>((header.seed, header.scenario_name(), Vec::<u32>::new()))
            })
            .unwrap();
        assert_eq!(playback.state.0, 0xdead_beef);
        assert_eq!(playback.state.1, "arena.map");
        while let Some(presentation) = playback.begin_frame() {
            playback.state.2.push(presentation.selected_objects[0]);
            assert!(playback.finish_frame().unwrap().is_empty());
        }
        assert_eq!(playback.state.2, vec![1, 2]);
    }

    #[test]
    fn native_decode_rejects_a_truncated_command_record() {
        let replay = NativeReplay {
            header: NativeReplayHeader::new(1, "x.map"),
            frames: vec![NativeReplayFrame::record(
                NativeReplayPresentation::new([0; 8], Vec::new(), [0; 2]),
                0,
                true,
                [&CommandRecord::encode(1, 0, 0, &[]).unwrap()],
            )],
        };
        let mut bytes = replay.encode().unwrap();
        bytes.pop();
        assert!(matches!(
            NativeReplay::decode_with_command_schedule(&bytes, |_, _| true),
            Err(NativeReplayError::Truncated {
                field: "command record",
                ..
            })
        ));
    }

    #[test]
    fn native_decode_treats_negative_selection_and_command_counts_as_empty() {
        let replay = NativeReplay {
            header: NativeReplayHeader::new(1, "x.map"),
            frames: vec![NativeReplayFrame::record(
                NativeReplayPresentation::new([0; 8], Vec::new(), [0; 2]),
                0,
                true,
                std::iter::empty::<&CommandRecord>(),
            )],
        };
        let mut bytes = replay.encode().unwrap();
        let selection_count_offset = NATIVE_REPLAY_HEADER_LEN + 8;
        bytes[selection_count_offset..selection_count_offset + 4]
            .copy_from_slice(&(-1_i32).to_le_bytes());
        let command_count_offset = NATIVE_REPLAY_HEADER_LEN + 8 + 4 + 4 + 8;
        bytes[command_count_offset..command_count_offset + 4]
            .copy_from_slice(&(-2_i32).to_le_bytes());

        assert_eq!(
            NativeReplay::decode_with_command_schedule(&bytes, |_, _| true).unwrap(),
            replay
        );
    }

    #[test]
    fn omitted_command_rung_writes_no_count_and_decodes_from_session_schedule() {
        let replay = NativeReplay {
            header: NativeReplayHeader::new(1, "x.map"),
            frames: vec![
                NativeReplayFrame::record(
                    NativeReplayPresentation::new([0x11; 8], Vec::new(), [0; 2]),
                    0,
                    false,
                    std::iter::empty::<&CommandRecord>(),
                ),
                NativeReplayFrame::record(
                    NativeReplayPresentation::new([0x22; 8], Vec::new(), [0; 2]),
                    1,
                    true,
                    std::iter::empty::<&CommandRecord>(),
                ),
            ],
        };

        let bytes = replay.encode().unwrap();
        let first_presentation_len = 8 + 4 + 4 + 8;
        assert_eq!(
            &bytes[0x1d0 + first_presentation_len..][..8],
            &[0x22; 8],
            "an omitted command rung has no zero-count placeholder"
        );
        assert_eq!(
            NativeReplay::decode_with_command_schedule(&bytes, |frame, _| frame != 0).unwrap(),
            replay
        );
    }

    #[test]
    fn diagnostic_log_json_roundtrip() {
        let mut log = ReplayLog::new(ReplayHeader {
            version: 1,
            tick_hz: 30,
            seed: 42,
            map_name: "test".to_string(),
            rules_hash: 123,
        });
        log.record_tick(
            1,
            vec![CommandEnvelope::new(
                crate::sim::intern::test_intern("Americans"),
                1,
                Command::SetRally {
                    owner: crate::sim::intern::test_intern("Americans"),
                    rx: 10,
                    ry: 11,
                    producer_ids: vec![1, 2],
                },
            )],
            999,
        );
        let json = serde_json::to_string(&log).expect("serialize");
        let parsed: ReplayLog = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.header.tick_hz, 30);
        assert_eq!(parsed.ticks.len(), 1);
        assert_eq!(parsed.ticks[0].tick, 1);
        assert_eq!(parsed.ticks[0].state_hash, 999);
    }

    #[test]
    fn gsi_04_07_wall_sell_diagnostic_replay_roundtrips_without_native_version_bump() {
        let owner = crate::sim::intern::test_intern("Receiver");
        let mut log = ReplayLog::new(ReplayHeader {
            version: 1,
            tick_hz: 15,
            seed: 7,
            map_name: "wall.map".to_string(),
            rules_hash: 9,
        });
        log.record_tick(
            4,
            vec![CommandEnvelope::new(
                owner,
                4,
                Command::SellWallAtCell { x: 0, y: 12 },
            )],
            11,
        );
        let json = serde_json::to_string(&log).unwrap();
        let decoded: ReplayLog = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.header.version, 1);
        assert_eq!(
            decoded.ticks[0].commands[0].payload,
            Command::SellWallAtCell { x: 0, y: 12 }
        );
        assert_eq!(NATIVE_REPLAY_VERSION, 10);
    }
}
