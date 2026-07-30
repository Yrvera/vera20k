//! Deterministic lockstep command staging and dispatch.
//!
//! Local commands retain their issue frame until transfer. Single-player
//! transfer admits that frame unchanged; network transfer overwrites every
//! staged record with the send frame plus the negotiated ahead window.

use std::collections::VecDeque;
use std::mem::size_of;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::sim::command::{Command, CommandEnvelope, CommandRecord, CommandRecordError};
use crate::sim::intern::InternedId;

const MEGAMISSION_EVENT_OPCODE: u8 = 0x04;
pub const FRAMEINFO_EVENT_OPCODE: u8 = 0x1c;
pub const CHECKSUM_HISTORY_LEN: usize = 0x100;

const FRAMEINFO_CHECKSUM_PAYLOAD_OFFSET: usize = 0;
const FRAMEINFO_TIMING_WORD_PAYLOAD_OFFSET: usize = 4;
const FRAMEINFO_DELAY_PAYLOAD_OFFSET: usize = 6;

/// The two verified network send-frame policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSendPolicy {
    /// Send every frame and stamp `current_frame + max_ahead`.
    EveryFrame,
    /// Mode 2 sends only on `frame_send_rate` boundaries and rounds the
    /// execute frame up to a boundary as well.
    FrameSendRate2 { frame_send_rate: NonZeroU32 },
}

impl NetworkSendPolicy {
    /// Whether the network path transfers its local queue on this frame.
    pub fn should_send(self, current_frame: u32) -> bool {
        match self {
            Self::EveryFrame => true,
            Self::FrameSendRate2 { frame_send_rate } => {
                let rate = frame_send_rate.get();
                let boundary = current_frame
                    .wrapping_add(rate.wrapping_sub(1))
                    .wrapping_div(rate)
                    .wrapping_mul(rate);
                current_frame == boundary
            }
        }
    }

    /// Frame written into each record at network transfer time.
    pub fn execute_frame(self, current_frame: u32, max_ahead: u32) -> u32 {
        match self {
            Self::EveryFrame => current_frame.wrapping_add(max_ahead),
            Self::FrameSendRate2 { frame_send_rate } => {
                let rate = frame_send_rate.get();
                rate.wrapping_add(current_frame)
                    .wrapping_sub(1)
                    .wrapping_add(max_ahead)
                    .wrapping_div(rate)
                    .wrapping_mul(rate)
            }
        }
    }
}

/// Payload carried by the leading FRAMEINFO record of a multiplayer packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub house_id: i8,
    pub event_frame: i32,
    pub checksum: u32,
    pub timing_word: u16,
    pub delay: u8,
}

impl FrameInfo {
    pub fn encode(self) -> CommandRecord {
        assert!(
            self.house_id >= 0,
            "outgoing FRAMEINFO requires a registered house id"
        );
        let mut record = CommandRecord::encode(
            FRAMEINFO_EVENT_OPCODE,
            i32::from(self.house_id),
            self.event_frame,
            &[],
        )
        .expect("FRAMEINFO has a fixed in-record payload");
        self.write_to(&mut record);
        record
    }

    /// Overwrite only the verified FRAMEINFO fields. Flags and every unknown
    /// payload byte remain untouched.
    pub fn write_to(self, record: &mut CommandRecord) {
        assert!(
            self.house_id >= 0,
            "outgoing FRAMEINFO requires a registered house id"
        );
        record.set_issue_header(
            FRAMEINFO_EVENT_OPCODE,
            i32::from(self.house_id),
            self.event_frame,
        );
        let payload = record.payload_mut();
        payload[FRAMEINFO_CHECKSUM_PAYLOAD_OFFSET
            ..FRAMEINFO_CHECKSUM_PAYLOAD_OFFSET + size_of::<u32>()]
            .copy_from_slice(&self.checksum.to_le_bytes());
        payload[FRAMEINFO_TIMING_WORD_PAYLOAD_OFFSET
            ..FRAMEINFO_TIMING_WORD_PAYLOAD_OFFSET + size_of::<u16>()]
            .copy_from_slice(&self.timing_word.to_le_bytes());
        payload[FRAMEINFO_DELAY_PAYLOAD_OFFSET] = self.delay;
    }

    pub fn decode(record: &CommandRecord) -> Option<Self> {
        if record.opcode() != FRAMEINFO_EVENT_OPCODE {
            return None;
        }
        let payload = record.payload();
        Some(Self {
            house_id: record.house_id(),
            event_frame: record.frame_stamp(),
            checksum: u32::from_le_bytes(
                payload[FRAMEINFO_CHECKSUM_PAYLOAD_OFFSET
                    ..FRAMEINFO_CHECKSUM_PAYLOAD_OFFSET + size_of::<u32>()]
                    .try_into()
                    .expect("FRAMEINFO checksum is four bytes"),
            ),
            timing_word: u16::from_le_bytes(
                payload[FRAMEINFO_TIMING_WORD_PAYLOAD_OFFSET
                    ..FRAMEINFO_TIMING_WORD_PAYLOAD_OFFSET + size_of::<u16>()]
                    .try_into()
                    .expect("FRAMEINFO timing word is two bytes"),
            ),
            delay: payload[FRAMEINFO_DELAY_PAYLOAD_OFFSET],
        })
    }

    #[inline]
    pub fn history_index(self) -> usize {
        ((self.event_frame as u32).wrapping_sub(u32::from(self.delay)) & 0xff) as usize
    }
}

/// The active 256-frame checksum ring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiplayerChecksumHistory {
    values: [u32; CHECKSUM_HISTORY_LEN],
}

impl MultiplayerChecksumHistory {
    pub const fn new() -> Self {
        Self {
            values: [0; CHECKSUM_HISTORY_LEN],
        }
    }

    #[inline]
    pub fn record(&mut self, frame: u32, checksum: u32) {
        self.values[(frame & 0xff) as usize] = checksum;
    }

    #[inline]
    pub fn get(&self, frame: u32) -> u32 {
        self.values[(frame & 0xff) as usize]
    }

    fn compare(&self, frame_info: FrameInfo) -> Result<(), ChecksumMismatch> {
        let history_index = frame_info.history_index();
        let local_checksum = self.values[history_index];
        if local_checksum == frame_info.checksum {
            Ok(())
        } else {
            Err(ChecksumMismatch {
                house_id: frame_info.house_id,
                event_frame: frame_info.event_frame,
                delay: frame_info.delay,
                history_index: history_index as u8,
                local_checksum,
                remote_checksum: frame_info.checksum,
            })
        }
    }
}

impl Default for MultiplayerChecksumHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Exact diagnostic facts available when one due FRAMEINFO comparison fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "multiplayer checksum mismatch for house {house_id} at frame {event_frame}: \
     local {local_checksum:#010x}, remote {remote_checksum:#010x}, \
     history[{history_index}] with delay {delay}"
)]
pub struct ChecksumMismatch {
    pub house_id: i8,
    pub event_frame: i32,
    pub delay: u8,
    pub history_index: u8,
    pub local_checksum: u32,
    pub remote_checksum: u32,
}

/// One fixed command record plus its optional decoded Rust command.
///
/// The record is authoritative for lockstep house/frame ordering. Known
/// commands retain the existing `CommandEnvelope` so dispatch can feed the
/// current simulation without losing unknown record bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizedCommand {
    record: CommandRecord,
    high_level: Option<CommandEnvelope>,
}

impl SynchronizedCommand {
    pub fn opaque(record: CommandRecord) -> Self {
        Self {
            record,
            high_level: None,
        }
    }

    pub fn known(record: CommandRecord, mut high_level: CommandEnvelope) -> Self {
        high_level.execute_tick = u64::from(record.frame_stamp() as u32);
        Self {
            record,
            high_level: Some(high_level),
        }
    }

    #[inline]
    pub fn record(&self) -> &CommandRecord {
        &self.record
    }

    #[inline]
    pub fn high_level(&self) -> Option<&CommandEnvelope> {
        self.high_level.as_ref()
    }

    pub fn into_parts(self) -> (CommandRecord, Option<CommandEnvelope>) {
        (self.record, self.high_level)
    }

    fn stamp_for_network(&mut self, house_id: i8, execute_frame: u32) {
        self.record.set_house_id(house_id);
        self.record.set_frame_stamp(execute_frame as i32);
        self.record.clear_processed();
        if let Some(high_level) = &mut self.high_level {
            high_level.execute_tick = u64::from(execute_frame);
        }
    }
}

/// Result counters from one native-order DoList scan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DispatchSummary {
    /// Records forwarded to the command executor.
    pub executed: usize,
    /// Due timing records consumed without command execution.
    pub timing_consumed: usize,
    /// Due-on-this-frame timing records compared with local history.
    pub frame_info_compared: usize,
    /// Executed non-timing records whose frame was already past.
    pub late_executed: usize,
    /// Contiguous processed/expired records retired from the DoList head.
    pub retired: usize,
}

/// Local issue queue and synchronized received-command list.
///
/// `VecDeque` preserves the two native FIFO scans without retaining the
/// original fixed storage caps, which are resource limits rather than
/// lockstep rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynchronizedCommandQueue {
    local: VecDeque<SynchronizedCommand>,
    do_list: VecDeque<SynchronizedCommand>,
}

impl SynchronizedCommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn local_len(&self) -> usize {
        self.local.len()
    }

    #[inline]
    pub fn do_list_len(&self) -> usize {
        self.do_list.len()
    }

    pub fn local_records(&self) -> impl Iterator<Item = &SynchronizedCommand> {
        self.local.iter()
    }

    pub fn synchronized_records(&self) -> impl Iterator<Item = &SynchronizedCommand> {
        self.do_list.iter()
    }

    /// Append one locally issued command in issue order.
    pub fn issue(&mut self, command: SynchronizedCommand) {
        self.local.push_back(command);
    }

    /// Admit one received/replay command in arrival order.
    pub fn admit(&mut self, mut command: SynchronizedCommand) {
        command.record.clear_processed();
        self.do_list.push_back(command);
    }

    /// Admit an unknown fixed-width record without interpreting its payload.
    pub fn admit_bytes(&mut self, bytes: &[u8]) -> Result<(), CommandRecordError> {
        let record = CommandRecord::admit_exact(bytes)?;
        self.admit(SynchronizedCommand::opaque(record));
        Ok(())
    }

    /// Single-player transfer: preserve the issue-frame and house bytes.
    pub fn transfer_single_player(&mut self) -> usize {
        let moved = self.local.len();
        while let Some(mut command) = self.local.pop_front() {
            command.record.clear_processed();
            self.do_list.push_back(command);
        }
        moved
    }

    /// Network transfer: emit the packet-leading FRAMEINFO record, overwrite
    /// command house/frame fields at send time, mirror each command into the
    /// local DoList, and return the records for transport.
    ///
    /// Packet byte limits and command compression belong to the transport
    /// layer; this method drains the commands selected for the current send.
    pub fn transfer_network(
        &mut self,
        current_frame: u32,
        max_ahead: u32,
        local_house_id: i8,
        policy: NetworkSendPolicy,
        current_checksum: u32,
        timing_word: u16,
    ) -> Vec<CommandRecord> {
        if !policy.should_send(current_frame) {
            return Vec::new();
        }

        let execute_frame = policy.execute_frame(current_frame, max_ahead);
        let mut outgoing = Vec::with_capacity(self.local.len() + 1);
        outgoing.push(
            FrameInfo {
                house_id: local_house_id,
                event_frame: execute_frame as i32,
                checksum: current_checksum,
                timing_word,
                delay: max_ahead as u8,
            }
            .encode(),
        );
        while let Some(mut command) = self.local.pop_front() {
            command.stamp_for_network(local_house_id, execute_frame);
            outgoing.push(command.record.clone());
            self.do_list.push_back(command);
        }
        outgoing
    }

    /// Dispatch all due records in session house-registration order.
    ///
    /// Each house scans the DoList from its current head in arrival order.
    /// The callback runs before bit 0 is marked, matching the command-execute
    /// then acknowledge order. `house_order` is the serialized session order;
    /// its indices are the signed-byte house ids carried by records.
    pub fn dispatch_due<F>(
        &mut self,
        current_frame: i32,
        house_order: &[InternedId],
        execute: F,
    ) -> DispatchSummary
    where
        F: FnMut(InternedId, &SynchronizedCommand, bool),
    {
        self.dispatch_due_inner(current_frame, house_order, None, execute)
            .expect("checksum comparison is disabled")
    }

    /// Multiplayer variant of [`Self::dispatch_due`]. A FRAMEINFO record is
    /// compared only when its event frame equals the current frame; late
    /// FRAMEINFO records are acknowledged without comparison.
    pub fn dispatch_due_with_checksums<F>(
        &mut self,
        current_frame: i32,
        house_order: &[InternedId],
        checksum_history: &MultiplayerChecksumHistory,
        execute: F,
    ) -> Result<DispatchSummary, ChecksumMismatch>
    where
        F: FnMut(InternedId, &SynchronizedCommand, bool),
    {
        self.dispatch_due_inner(current_frame, house_order, Some(checksum_history), execute)
    }

    fn dispatch_due_inner<F>(
        &mut self,
        current_frame: i32,
        house_order: &[InternedId],
        checksum_history: Option<&MultiplayerChecksumHistory>,
        mut execute: F,
    ) -> Result<DispatchSummary, ChecksumMismatch>
    where
        F: FnMut(InternedId, &SynchronizedCommand, bool),
    {
        let mut summary = DispatchSummary::default();
        let scan_len = self.do_list.len();

        for (house_index, owner) in house_order.iter().copied().enumerate() {
            let Ok(house_id) = i8::try_from(house_index) else {
                break;
            };
            let mut staged_megamissions = Vec::new();
            for record_index in 0..scan_len {
                let Some(command) = self.do_list.get_mut(record_index) else {
                    break;
                };
                if command.record.house_id() != house_id
                    || command.record.frame_stamp() > current_frame
                    || command.record.is_processed()
                {
                    continue;
                }

                let is_late = command.record.frame_stamp() < current_frame;
                match command.record.opcode() {
                    FRAMEINFO_EVENT_OPCODE => {
                        if command.record.frame_stamp() == current_frame {
                            if let Some(history) = checksum_history {
                                let frame_info = FrameInfo::decode(&command.record)
                                    .expect("opcode was checked above");
                                history.compare(frame_info)?;
                                summary.frame_info_compared += 1;
                            }
                        }
                        summary.timing_consumed += 1;
                    }
                    MEGAMISSION_EVENT_OPCODE => {
                        staged_megamissions.push((command.clone(), is_late));
                    }
                    _ => {
                        execute(owner, command, is_late);
                        summary.executed += 1;
                        summary.late_executed += usize::from(is_late);
                    }
                }
                command.record.mark_processed();
            }
            for (command, is_late) in staged_megamissions {
                execute(owner, &command, is_late);
                summary.executed += 1;
                summary.late_executed += usize::from(is_late);
            }
        }

        while self.do_list.front().is_some_and(|command| {
            command.record.is_processed() || command.record.frame_stamp() < current_frame
        }) {
            self.do_list.pop_front();
            summary.retired += 1;
        }
        Ok(summary)
    }
}

/// Builds the local issue record and keeps the existing decoded command beside
/// it. Network delay is deliberately not applied here; transfer owns stamping.
#[derive(Debug, Clone, Copy, Default)]
pub struct LockstepScheduler;

impl LockstepScheduler {
    pub fn new() -> Self {
        Self
    }

    pub fn schedule(
        &self,
        issue_frame: u32,
        owner: InternedId,
        payload: Command,
    ) -> CommandEnvelope {
        CommandEnvelope::new(owner, u64::from(issue_frame), payload)
    }

    pub fn issue(
        &self,
        issue_frame: u32,
        house_id: i32,
        opcode: u8,
        encoded_payload: &[u8],
        owner: InternedId,
        payload: Command,
    ) -> Result<SynchronizedCommand, CommandRecordError> {
        let record = CommandRecord::encode(opcode, house_id, issue_frame as i32, encoded_payload)?;
        Ok(SynchronizedCommand::known(
            record,
            self.schedule(issue_frame, owner, payload),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::{
        FrameInfo, LockstepScheduler, MultiplayerChecksumHistory, NetworkSendPolicy,
        SynchronizedCommand, SynchronizedCommandQueue,
    };
    use crate::sim::command::{COMMAND_RECORD_LEN, Command, CommandEnvelope, CommandRecord};
    use crate::sim::intern::InternedId;

    fn opaque(opcode: u8, house: i32, frame: i32, payload: &[u8]) -> SynchronizedCommand {
        SynchronizedCommand::opaque(CommandRecord::encode(opcode, house, frame, payload).unwrap())
    }

    #[test]
    fn synchronized_queue_roundtrip_preserves_bytes_order_and_dispatch_state() {
        let owner = InternedId::from_index(3);
        let mut queue = SynchronizedCommandQueue::new();

        let mut opaque_bytes = [0xa5; COMMAND_RECORD_LEN];
        opaque_bytes[0] = 0xfe;
        opaque_bytes[1] = 0x80;
        opaque_bytes[2] = 2;
        opaque_bytes[3..7].copy_from_slice(&17_i32.to_le_bytes());
        queue.issue(SynchronizedCommand::opaque(
            CommandRecord::decode_exact(&opaque_bytes).unwrap(),
        ));
        queue.issue(SynchronizedCommand::known(
            CommandRecord::encode(0x15, 0, 18, &[7, 8]).unwrap(),
            CommandEnvelope::new(owner, 99, Command::Stop { entity_id: 42 }),
        ));

        queue.admit(opaque(0x15, 0, 12, &[1]));
        queue.admit(opaque(0x15, 0, 10, &[2]));
        queue.dispatch_due(10, &[owner], |_, _, _| {});

        let encoded = bincode::serialize(&queue).unwrap();
        let restored: SynchronizedCommandQueue = bincode::deserialize(&encoded).unwrap();

        assert_eq!(restored.local_len(), 2);
        assert_eq!(restored.do_list_len(), 2);
        assert_eq!(
            restored.local_records().next().unwrap().record().as_bytes(),
            &opaque_bytes
        );
        let known = restored.local_records().nth(1).unwrap();
        assert_eq!(&known.record().payload()[..2], &[7, 8]);
        assert_eq!(
            known.high_level().unwrap().payload,
            Command::Stop { entity_id: 42 }
        );
        assert_eq!(
            restored
                .synchronized_records()
                .map(|command| command.record().is_processed())
                .collect::<Vec<_>>(),
            vec![false, true]
        );
    }

    #[test]
    fn single_player_preserves_issue_frame_and_dispatches_house_then_fifo_order() {
        let owner_a = InternedId::from_index(9);
        let owner_b = InternedId::from_index(2);
        let scheduler = LockstepScheduler::new();
        let mut queue = SynchronizedCommandQueue::new();

        queue.issue(
            scheduler
                .issue(
                    10,
                    1,
                    0x15,
                    &[0xb1],
                    owner_b,
                    Command::Stop { entity_id: 11 },
                )
                .unwrap(),
        );
        queue.issue(
            scheduler
                .issue(
                    10,
                    0,
                    0x15,
                    &[0xa1],
                    owner_a,
                    Command::Stop { entity_id: 21 },
                )
                .unwrap(),
        );
        queue.issue(
            scheduler
                .issue(
                    10,
                    1,
                    0x15,
                    &[0xb2],
                    owner_b,
                    Command::Stop { entity_id: 12 },
                )
                .unwrap(),
        );

        assert_eq!(queue.transfer_single_player(), 3);
        let mut executed = Vec::new();
        let summary = queue.dispatch_due(10, &[owner_a, owner_b], |owner, command, late| {
            let entity_id = match &command.high_level().unwrap().payload {
                Command::Stop { entity_id } => *entity_id,
                other => panic!("unexpected command: {other:?}"),
            };
            executed.push((owner, entity_id, command.record().frame_stamp(), late));
        });

        assert_eq!(
            executed,
            vec![
                (owner_a, 21, 10, false),
                (owner_b, 11, 10, false),
                (owner_b, 12, 10, false),
            ]
        );
        assert_eq!(summary.executed, 3);
        assert_eq!(summary.retired, 3);
        assert_eq!(queue.do_list_len(), 0);
    }

    #[test]
    fn network_stamps_all_staged_commands_from_the_send_frame() {
        let owner = InternedId::from_index(4);
        let scheduler = LockstepScheduler::new();
        let mut queue = SynchronizedCommandQueue::new();
        for issue_frame in [100, 101] {
            queue.issue(
                scheduler
                    .issue(
                        issue_frame,
                        0,
                        0x15,
                        &[issue_frame as u8],
                        owner,
                        Command::Stop {
                            entity_id: u64::from(issue_frame),
                        },
                    )
                    .unwrap(),
            );
        }

        let outgoing = queue.transfer_network(
            110,
            15,
            2,
            NetworkSendPolicy::EveryFrame,
            0x1122_3344,
            0xabcd,
        );
        assert_eq!(outgoing.len(), 3);
        assert_eq!(
            FrameInfo::decode(&outgoing[0]),
            Some(FrameInfo {
                house_id: 2,
                event_frame: 125,
                checksum: 0x1122_3344,
                timing_word: 0xabcd,
                delay: 15,
            })
        );
        assert!(
            outgoing
                .iter()
                .skip(1)
                .all(|record| record.frame_stamp() == 125 && record.house_id() == 2)
        );
        assert_eq!(
            queue
                .synchronized_records()
                .map(|command| command.high_level().unwrap().execute_tick)
                .collect::<Vec<_>>(),
            vec![125, 125]
        );
    }

    #[test]
    fn network_send_boundary_emits_frame_info_without_local_commands() {
        let mut queue = SynchronizedCommandQueue::new();

        let outgoing =
            queue.transfer_network(20, 7, 1, NetworkSendPolicy::EveryFrame, 0xaabb_ccdd, 9);

        assert_eq!(outgoing.len(), 1);
        assert_eq!(
            FrameInfo::decode(&outgoing[0]),
            Some(FrameInfo {
                house_id: 1,
                event_frame: 27,
                checksum: 0xaabb_ccdd,
                timing_word: 9,
                delay: 7,
            })
        );
    }

    #[test]
    fn frame_send_rate_two_waits_and_uses_unsigned_boundary_rounding() {
        let policy = NetworkSendPolicy::FrameSendRate2 {
            frame_send_rate: NonZeroU32::new(5).unwrap(),
        };
        let mut queue = SynchronizedCommandQueue::new();
        queue.issue(opaque(0x15, 0, 11, &[7]));

        assert!(!policy.should_send(11));
        assert!(
            queue
                .transfer_network(11, 7, 0, policy, 0x55aa, 3)
                .is_empty()
        );
        assert_eq!(queue.local_len(), 1);

        assert!(policy.should_send(15));
        let outgoing = queue.transfer_network(15, 7, 0, policy, 0x55aa, 3);
        assert_eq!(outgoing[0].opcode(), super::FRAMEINFO_EVENT_OPCODE);
        assert_eq!(outgoing[0].frame_stamp(), 25);
        assert_eq!(outgoing[1].frame_stamp(), 25);
        assert_eq!(policy.execute_frame(15, 0), 15);
        assert_eq!(
            NetworkSendPolicy::EveryFrame.execute_frame(u32::MAX - 2, 5),
            2
        );
    }

    #[test]
    fn due_scan_executes_late_unknown_bytes_consumes_timing_and_retains_future() {
        let owner = InternedId::from_index(7);
        let mut queue = SynchronizedCommandQueue::new();

        let mut late_bytes = [0x7b; COMMAND_RECORD_LEN];
        late_bytes[0] = 0xfe;
        late_bytes[1] = 0x80;
        late_bytes[2] = 0;
        late_bytes[3..7].copy_from_slice(&9_i32.to_le_bytes());
        queue.admit_bytes(&late_bytes).unwrap();
        queue.admit(opaque(0x1c, 0, 10, &[1, 2]));
        queue.admit(opaque(0x15, 0, 11, &[3, 4]));

        let mut observed = Vec::new();
        let summary = queue.dispatch_due(10, &[owner], |registered_owner, command, late| {
            observed.push((
                registered_owner,
                command.record().clone().into_bytes(),
                late,
                command.record().is_processed(),
            ));
        });

        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].0, owner);
        assert_eq!(observed[0].1[0], 0xfe);
        assert_eq!(observed[0].1[1], 0x80);
        assert!(observed[0].1[7..].iter().all(|byte| *byte == 0x7b));
        assert!(observed[0].2);
        assert!(!observed[0].3, "bit 0 is marked after execution");
        assert_eq!(summary.executed, 1);
        assert_eq!(summary.late_executed, 1);
        assert_eq!(summary.timing_consumed, 1);
        assert_eq!(summary.retired, 2);
        assert_eq!(queue.do_list_len(), 1);
        assert_eq!(
            queue
                .synchronized_records()
                .next()
                .unwrap()
                .record()
                .frame_stamp(),
            11
        );
    }

    #[test]
    fn processed_records_behind_a_future_head_stay_marked_until_head_can_retire() {
        let owner = InternedId::from_index(1);
        let mut queue = SynchronizedCommandQueue::new();
        queue.admit(opaque(0x15, 0, 12, &[]));
        queue.admit(opaque(0x15, 0, 10, &[]));

        let summary = queue.dispatch_due(10, &[owner], |_, _, _| {});
        assert_eq!(summary.executed, 1);
        assert_eq!(summary.retired, 0);
        assert!(
            !queue
                .synchronized_records()
                .next()
                .unwrap()
                .record()
                .is_processed()
        );
        assert!(
            queue
                .synchronized_records()
                .nth(1)
                .unwrap()
                .record()
                .is_processed()
        );
    }

    #[test]
    fn megamissions_stage_until_after_same_house_non_megamission_commands() {
        let owner = InternedId::from_index(5);
        let mut queue = SynchronizedCommandQueue::new();
        queue.admit(opaque(0x04, 0, 10, &[1]));
        queue.admit(opaque(0x15, 0, 10, &[2]));
        queue.admit(opaque(0x04, 0, 10, &[3]));

        let mut order = Vec::new();
        let summary = queue.dispatch_due(10, &[owner], |_, command, _| {
            order.push((command.record().opcode(), command.record().payload()[0]));
            assert!(
                !command.record().is_processed(),
                "the staged copy retains its pre-acknowledgement flags"
            );
        });

        assert_eq!(order, vec![(0x15, 2), (0x04, 1), (0x04, 3)]);
        assert_eq!(summary.executed, 3);
        assert_eq!(summary.retired, 3);
    }

    #[test]
    fn known_command_wrapper_keeps_existing_high_level_payload() {
        let owner = InternedId::from_index(3);
        let envelope = CommandEnvelope::new(owner, 999, Command::Stop { entity_id: 42 });
        let command =
            SynchronizedCommand::known(CommandRecord::encode(0x15, 0, 8, &[]).unwrap(), envelope);

        assert_eq!(command.high_level().unwrap().owner, owner);
        assert_eq!(command.high_level().unwrap().execute_tick, 8);
        assert_eq!(
            command.high_level().unwrap().payload,
            Command::Stop { entity_id: 42 }
        );
    }

    #[test]
    fn frame_info_overwrites_only_verified_fields() {
        let original = [0xa4; COMMAND_RECORD_LEN];
        let mut record = CommandRecord::decode_exact(&original).unwrap();
        FrameInfo {
            house_id: 3,
            event_frame: -9,
            checksum: 0x1234_5678,
            timing_word: 0xabcd,
            delay: 0xef,
        }
        .write_to(&mut record);

        let bytes = record.as_bytes();
        assert_eq!(bytes[0], super::FRAMEINFO_EVENT_OPCODE);
        assert_eq!(bytes[1], 0xa4, "flags are not normalized");
        assert_eq!(bytes[2], 3);
        assert_eq!(&bytes[3..7], &(-9_i32).to_le_bytes());
        assert_eq!(&bytes[7..11], &0x1234_5678_u32.to_le_bytes());
        assert_eq!(&bytes[11..13], &0xabcd_u16.to_le_bytes());
        assert_eq!(bytes[13], 0xef);
        assert!(bytes[14..].iter().all(|&byte| byte == 0xa4));
    }

    #[test]
    fn frame_info_compares_delayed_history_with_wrapping_index() {
        let owner = InternedId::from_index(1);
        let mut history = MultiplayerChecksumHistory::new();
        history.record(u32::MAX, 0xdead_beef);
        let mut queue = SynchronizedCommandQueue::new();
        queue.admit(SynchronizedCommand::opaque(
            FrameInfo {
                house_id: 0,
                event_frame: 1,
                checksum: 0xdead_beef,
                timing_word: 0,
                delay: 2,
            }
            .encode(),
        ));

        let summary = queue
            .dispatch_due_with_checksums(1, &[owner], &history, |_, _, _| {
                panic!("FRAMEINFO must not reach command execution")
            })
            .unwrap();

        assert_eq!(summary.timing_consumed, 1);
        assert_eq!(summary.frame_info_compared, 1);
        assert_eq!(summary.retired, 1);
    }

    #[test]
    fn checksum_mismatch_stops_before_acknowledging_frame_info() {
        let owner = InternedId::from_index(1);
        let mut history = MultiplayerChecksumHistory::new();
        history.record(40, 0x1111_1111);
        let mut queue = SynchronizedCommandQueue::new();
        queue.admit(SynchronizedCommand::opaque(
            FrameInfo {
                house_id: 0,
                event_frame: 45,
                checksum: 0x2222_2222,
                timing_word: 0,
                delay: 5,
            }
            .encode(),
        ));

        let mismatch = queue
            .dispatch_due_with_checksums(45, &[owner], &history, |_, _, _| {})
            .unwrap_err();

        assert_eq!(mismatch.history_index, 40);
        assert_eq!(mismatch.local_checksum, 0x1111_1111);
        assert_eq!(mismatch.remote_checksum, 0x2222_2222);
        assert_eq!(queue.do_list_len(), 1);
        assert!(
            !queue
                .synchronized_records()
                .next()
                .unwrap()
                .record()
                .is_processed()
        );
    }

    #[test]
    fn late_frame_info_is_consumed_without_comparison() {
        let owner = InternedId::from_index(1);
        let history = MultiplayerChecksumHistory::new();
        let mut queue = SynchronizedCommandQueue::new();
        queue.admit(SynchronizedCommand::opaque(
            FrameInfo {
                house_id: 0,
                event_frame: 9,
                checksum: 0xffff_ffff,
                timing_word: 0,
                delay: 0,
            }
            .encode(),
        ));

        let summary = queue
            .dispatch_due_with_checksums(10, &[owner], &history, |_, _, _| {})
            .unwrap();
        assert_eq!(summary.timing_consumed, 1);
        assert_eq!(summary.frame_info_compared, 0);
        assert_eq!(summary.retired, 1);
    }
}
