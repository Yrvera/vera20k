//! Deterministic simulation command model.
//!
//! All gameplay inputs are translated into explicit commands that can be
//! scheduled by tick, logged, replayed, and sent over lockstep transport.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::sim::intern::InternedId;
use crate::sim::production::ProductionCategory;

/// Fixed byte width of one synchronized command record.
pub const COMMAND_RECORD_LEN: usize = 0x6f;
/// Byte width available to opcode-specific command data.
pub const COMMAND_RECORD_PAYLOAD_LEN: usize = COMMAND_RECORD_LEN - COMMAND_RECORD_PAYLOAD_OFFSET;

const COMMAND_RECORD_OPCODE_OFFSET: usize = 0;
const COMMAND_RECORD_FLAGS_OFFSET: usize = 1;
const COMMAND_RECORD_HOUSE_OFFSET: usize = 2;
const COMMAND_RECORD_FRAME_OFFSET: usize = 3;
const COMMAND_RECORD_PAYLOAD_OFFSET: usize = 7;
const COMMAND_RECORD_PROCESSED_FLAG: u8 = 0x01;

/// A malformed fixed-width synchronized command record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CommandRecordError {
    #[error("command record must be exactly {expected} bytes, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("command payload can contain at most {max} bytes, got {actual}")]
    PayloadTooLong { max: usize, actual: usize },
}

/// Native-width synchronized command envelope.
///
/// The header is seven bytes: opcode, flags, signed house id, and a signed
/// little-endian frame stamp. The remaining 104 bytes are an opcode-specific
/// payload. Keeping the payload opaque here lets the admission boundary
/// preserve every byte without coupling transport to command dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRecord {
    bytes: [u8; COMMAND_RECORD_LEN],
}

impl Serialize for CommandRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;

        let mut record = serializer.serialize_tuple(COMMAND_RECORD_LEN)?;
        for byte in &self.bytes {
            record.serialize_element(byte)?;
        }
        record.end()
    }
}

impl<'de> Deserialize<'de> for CommandRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct CommandRecordVisitor;

        impl<'de> serde::de::Visitor<'de> for CommandRecordVisitor {
            type Value = CommandRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    formatter,
                    "an exact {COMMAND_RECORD_LEN}-byte command record"
                )
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut bytes = [0; COMMAND_RECORD_LEN];
                for (index, byte) in bytes.iter_mut().enumerate() {
                    *byte = sequence
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(index, &self))?;
                }
                Ok(CommandRecord { bytes })
            }
        }

        deserializer.deserialize_tuple(COMMAND_RECORD_LEN, CommandRecordVisitor)
    }
}

impl CommandRecord {
    /// Encode a freshly issued command with deterministic zeroed padding.
    ///
    /// A negative house id produces the native null-event header: opcode zero
    /// and house `-1`. The frame stamp is still written and the payload is not
    /// admitted.
    pub fn encode(
        opcode: u8,
        house_id: i32,
        frame_stamp: i32,
        payload: &[u8],
    ) -> Result<Self, CommandRecordError> {
        if payload.len() > COMMAND_RECORD_PAYLOAD_LEN {
            return Err(CommandRecordError::PayloadTooLong {
                max: COMMAND_RECORD_PAYLOAD_LEN,
                actual: payload.len(),
            });
        }

        let mut record = Self {
            bytes: [0; COMMAND_RECORD_LEN],
        };
        record.set_issue_header(opcode, house_id, frame_stamp);
        if house_id >= 0 {
            record.payload_mut()[..payload.len()].copy_from_slice(payload);
        }
        Ok(record)
    }

    /// Decode one record without applying queue-admission mutations.
    pub fn decode_exact(bytes: &[u8]) -> Result<Self, CommandRecordError> {
        let bytes = <[u8; COMMAND_RECORD_LEN]>::try_from(bytes).map_err(|_| {
            CommandRecordError::InvalidLength {
                expected: COMMAND_RECORD_LEN,
                actual: bytes.len(),
            }
        })?;
        Ok(Self { bytes })
    }

    /// Admit one wire/replay record into the synchronized command queue.
    ///
    /// Admission is intentionally structural: any opcode, house byte, frame,
    /// and payload are preserved. Only the processed marker is cleared.
    pub fn admit_exact(bytes: &[u8]) -> Result<Self, CommandRecordError> {
        let mut record = Self::decode_exact(bytes)?;
        record.clear_processed();
        Ok(record)
    }

    #[inline]
    pub fn opcode(&self) -> u8 {
        self.bytes[COMMAND_RECORD_OPCODE_OFFSET]
    }

    #[inline]
    pub fn flags(&self) -> u8 {
        self.bytes[COMMAND_RECORD_FLAGS_OFFSET]
    }

    #[inline]
    pub fn house_id(&self) -> i8 {
        self.bytes[COMMAND_RECORD_HOUSE_OFFSET] as i8
    }

    #[inline]
    pub fn frame_stamp(&self) -> i32 {
        i32::from_le_bytes(
            self.bytes[COMMAND_RECORD_FRAME_OFFSET..COMMAND_RECORD_PAYLOAD_OFFSET]
                .try_into()
                .expect("frame stamp is four bytes"),
        )
    }

    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.bytes[COMMAND_RECORD_PAYLOAD_OFFSET..]
    }

    #[inline]
    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[COMMAND_RECORD_PAYLOAD_OFFSET..]
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8; COMMAND_RECORD_LEN] {
        &self.bytes
    }

    #[inline]
    pub fn into_bytes(self) -> [u8; COMMAND_RECORD_LEN] {
        self.bytes
    }

    /// Apply the header writes made when a local command is issued.
    ///
    /// Flags and payload remain untouched, including for a null event.
    pub fn set_issue_header(&mut self, opcode: u8, house_id: i32, frame_stamp: i32) {
        if house_id < 0 {
            self.bytes[COMMAND_RECORD_OPCODE_OFFSET] = 0;
            self.bytes[COMMAND_RECORD_HOUSE_OFFSET] = u8::MAX;
        } else {
            self.bytes[COMMAND_RECORD_OPCODE_OFFSET] = opcode;
            self.bytes[COMMAND_RECORD_HOUSE_OFFSET] = house_id as u8;
        }
        self.set_frame_stamp(frame_stamp);
    }

    #[inline]
    pub fn set_frame_stamp(&mut self, frame_stamp: i32) {
        self.bytes[COMMAND_RECORD_FRAME_OFFSET..COMMAND_RECORD_PAYLOAD_OFFSET]
            .copy_from_slice(&frame_stamp.to_le_bytes());
    }

    #[inline]
    pub fn set_house_id(&mut self, house_id: i8) {
        self.bytes[COMMAND_RECORD_HOUSE_OFFSET] = house_id as u8;
    }

    #[inline]
    pub fn is_processed(&self) -> bool {
        self.flags() & COMMAND_RECORD_PROCESSED_FLAG != 0
    }

    #[inline]
    pub fn mark_processed(&mut self) {
        self.bytes[COMMAND_RECORD_FLAGS_OFFSET] |= COMMAND_RECORD_PROCESSED_FLAG;
    }

    #[inline]
    pub fn clear_processed(&mut self) {
        self.bytes[COMMAND_RECORD_FLAGS_OFFSET] &= !COMMAND_RECORD_PROCESSED_FLAG;
    }
}

impl AsRef<[u8]> for CommandRecord {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl TryFrom<&[u8]> for CommandRecord {
    type Error = CommandRecordError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::decode_exact(bytes)
    }
}

/// Queueing behavior for build/production commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueMode {
    /// Replace existing queued path/intent.
    Replace,
    /// Append to existing queue/waypoint chain.
    Append,
}

/// One gameplay command payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Selection intent (kept for replay/debug parity; not authoritative sim state yet).
    Select {
        entity_ids: Vec<u64>,
        additive: bool,
    },
    /// Move one entity to a target cell.
    /// Speed is resolved at dispatch time from rules.ini (ObjectType.speed)
    /// multiplied by the entity's locomotor speed_multiplier.
    Move {
        entity_id: u64,
        target_rx: u16,
        target_ry: u16,
        queue: bool,
        /// When multiple units are ordered together, they share a group_id.
        /// The movement system syncs their speed to the slowest member.
        group_id: Option<u32>,
    },
    /// Stop movement/combat intent on one entity.
    Stop { entity_id: u64 },
    /// Attack an explicit target.
    Attack { attacker_id: u64, target_id: u64 },
    /// Force-attack a target (ignores friendship — Ctrl+click).
    ForceAttack { attacker_id: u64, target_id: u64 },
    /// Attack-move toward a cell (logic can retarget along path).
    AttackMove {
        entity_id: u64,
        target_rx: u16,
        target_ry: u16,
        queue: bool,
    },
    /// Guard a target entity or area (target optional for area guard).
    Guard {
        entity_id: u64,
        target_id: Option<u64>,
    },
    /// Deploy a mobile construction vehicle into its construction yard.
    DeployMcv { entity_id: u64 },
    /// Undeploy a structure back into its mobile unit (e.g. ConYard → MCV).
    /// Reads UndeploysInto from rules.ini to determine the spawned unit type.
    UndeployBuilding { entity_id: u64 },
    /// Set production rally point for owner and the explicit selected producers.
    SetRally {
        owner: InternedId,
        rx: u16,
        ry: u16,
        producer_ids: Vec<u64>,
    },
    /// Enqueue a production item.
    QueueProduction {
        owner: InternedId,
        type_id: InternedId,
        mode: QueueMode,
    },
    /// Pause/resume the active production item for one owner/category queue.
    TogglePauseProduction {
        owner: InternedId,
        category: ProductionCategory,
    },
    /// Cycle the active producer facility for one owner/category.
    CycleProducerFocus {
        owner: InternedId,
        category: ProductionCategory,
    },
    /// Place one completed building that is waiting for placement.
    PlaceReadyBuilding {
        owner: InternedId,
        type_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// Cancel the last queued production item for owner.
    CancelLastProduction { owner: InternedId },
    /// Cancel one queued item of a specific type (right-click cameo).
    CancelProductionByType {
        owner: InternedId,
        type_id: InternedId,
    },
    /// Sell a building, refunding a percentage of its cost and despawning it.
    SellBuilding { entity_id: u64 },
    /// Toggle repair mode on a building (spend credits to heal over time).
    ToggleRepair { entity_id: u64 },
    /// Force a miner to return to its refinery (right-click on own refinery or 'D' key).
    /// Chrono Miners teleport; War Miners drive back.
    MinerReturn {
        entity_id: u64,
        /// Explicit refinery clicked by the player. Keyboard/generic forced
        /// return leaves this empty and lets the miner choose a refinery.
        target_refinery_id: Option<u64>,
    },
    /// Send a unit to a repair depot for repairs.
    /// The unit pathfinds to the depot, docks, and auto-repairs until full HP or out of credits.
    RepairAtDepot { entity_id: u64, depot_id: u64 },
    /// Order an infantry/vehicle to enter a friendly transport or garrisonable building.
    /// The passenger pathfinds to the transport's cell and boards on arrival.
    EnterTransport {
        passenger_id: u64,
        transport_id: u64,
    },
    /// Order a bunkerable vehicle into a friendly `Bunker=yes` building (tank
    /// bunker). The unit pathfinds to the bunker cell; the install machine takes
    /// over on arrival (turn → slide onto the cell → turn south → hide). Gating
    /// happens in `world_commands`: own-owner, the bunker idle/empty, and the
    /// unit `Bunkerable=yes` with a primary weapon.
    EnterBunker { unit_id: u64, bunker_id: u64 },
    /// Eject the occupant of a friendly occupied tank bunker. Targets the bunker
    /// (the hidden occupant is not selectable in this slice); the occupant
    /// reappears on a passable cell SW of the bunker and drives out.
    EjectBunker { bunker_id: u64 },
    /// Order a transport to unload all passengers to adjacent cells (one per tick).
    UnloadPassengers { transport_id: u64 },
    /// Direct a harvester to go harvest a specific ore cell.
    /// The miner will path to the cell, then enter Harvest state on arrival.
    HarvestCell {
        entity_id: u64,
        target_rx: u16,
        target_ry: u16,
    },
    /// Order an engineer to capture an enemy building.
    /// Engineer walks to the building, instantly transfers ownership on arrival,
    /// and is consumed.
    CaptureBuilding {
        engineer_id: u64,
        target_building_id: u64,
    },
    /// Order a C4-capable infantry (SEAL / Tanya / Psi-Corp Trooper) to plant
    /// on an enemy building. The unit walks to the building's cell; on arrival
    /// the building's `pending_c4_detonation` is set; after `C4Delay` ticks the
    /// building takes full-HP damage with C4Warhead and dies. The attacker
    /// survives and scatters one cell. Gating happens in `world_commands` —
    /// attacker must have `C4=yes`, target must be a `CanC4=yes` building, not
    /// invisible-in-game, not iron-curtained, not in fog.
    PlantC4 {
        attacker_id: u64,
        target_building_id: u64,
    },
    /// Fire a superweapon at a target cell.
    /// The sim validates that the owner has a ready instance of the specified SW type
    /// and dispatches to the appropriate launch handler.
    LaunchSuperWeapon {
        sw_type_id: InternedId,
        target_rx: u16,
        target_ry: u16,
    },
    /// Toggle an infantry unit's deploy-fire state.
    ///
    /// Three transitions:
    /// - `None → Deploying` (start deploy animation)
    /// - `Deployed → Undeploying` (start undeploy animation)
    /// - mid-transition (Deploying / Undeploying) → no-op (matches gamemd)
    ///
    /// Silently no-op if the entity's type is not `DeployFire=yes`.
    ToggleInfantryDeploy { entity_id: u64 },
    /// Force-attack on a ground cell (Ctrl + left-click on empty terrain).
    ///
    /// Bypasses friendship check and entity-targeting — fires the attacker's
    /// weapon at the cell's center. Unarmed units must NOT receive this
    /// command; the order-resolution layer routes them to `Move` instead.
    /// Defensive sim-side check in `issue_attack_cell_command` warn-logs and
    /// no-ops if a stray `ForceAttackCell` reaches an unarmed unit.
    ForceAttackCell {
        attacker_id: u64,
        target_rx: u16,
        target_ry: u16,
    },
}

/// Command with deterministic execution metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub owner: InternedId,
    pub execute_tick: u64,
    pub payload: Command,
}

impl CommandEnvelope {
    pub fn new(owner: InternedId, execute_tick: u64, payload: Command) -> Self {
        Self {
            owner,
            execute_tick,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        COMMAND_RECORD_LEN, COMMAND_RECORD_PAYLOAD_LEN, CommandRecord, CommandRecordError,
    };

    #[test]
    fn command_record_encoding_uses_the_fixed_native_layout() {
        let record = CommandRecord::encode(0x15, 3, 0x1234_5678, &[0x10, 0x20, 0x30]).unwrap();
        let bytes = record.as_bytes();

        assert_eq!(bytes.len(), 111);
        assert_eq!(bytes[0], 0x15);
        assert_eq!(bytes[1], 0);
        assert_eq!(bytes[2], 3);
        assert_eq!(&bytes[3..7], &[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(&bytes[7..10], &[0x10, 0x20, 0x30]);
        assert!(bytes[10..].iter().all(|byte| *byte == 0));

        let decoded = CommandRecord::decode_exact(bytes).unwrap();
        assert_eq!(decoded, record);
        assert_eq!(decoded.opcode(), 0x15);
        assert_eq!(decoded.house_id(), 3);
        assert_eq!(decoded.frame_stamp(), 0x1234_5678);
    }

    #[test]
    fn command_record_bincode_roundtrip_preserves_all_fixed_bytes() {
        let mut bytes = [0; COMMAND_RECORD_LEN];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let record = CommandRecord::decode_exact(&bytes).unwrap();

        let encoded = bincode::serialize(&record).unwrap();
        assert_eq!(encoded.as_slice(), bytes.as_slice());
        assert_eq!(
            bincode::deserialize::<CommandRecord>(&encoded).unwrap(),
            record
        );
    }

    #[test]
    fn negative_house_id_nulls_only_the_issued_header() {
        let mut bytes = [0xcc; COMMAND_RECORD_LEN];
        bytes[1] = 0x5a;
        let mut record = CommandRecord::decode_exact(&bytes).unwrap();

        record.set_issue_header(0x15, -1, 41);

        assert_eq!(record.opcode(), 0);
        assert_eq!(record.flags(), 0x5a);
        assert_eq!(record.house_id(), -1);
        assert_eq!(record.frame_stamp(), 41);
        assert!(record.payload().iter().all(|byte| *byte == 0xcc));

        let encoded = CommandRecord::encode(0x15, -1, 41, &[1, 2, 3]).unwrap();
        assert!(encoded.payload().iter().all(|byte| *byte == 0));
    }

    #[test]
    fn admission_preserves_opaque_bytes_and_clears_only_processed_bit() {
        let mut bytes = [0x7b; COMMAND_RECORD_LEN];
        bytes[0] = 0xff;
        bytes[1] = 0xa5;
        bytes[2] = 0xfe;
        bytes[3..7].copy_from_slice(&(-2_i32).to_le_bytes());

        let decoded = CommandRecord::decode_exact(&bytes).unwrap();
        assert_eq!(decoded.flags(), 0xa5);

        let admitted = CommandRecord::admit_exact(&bytes).unwrap();
        assert_eq!(admitted.opcode(), 0xff);
        assert_eq!(admitted.flags(), 0xa4);
        assert_eq!(admitted.house_id(), -2);
        assert_eq!(admitted.frame_stamp(), -2);
        assert_eq!(admitted.payload(), &bytes[7..]);
    }

    #[test]
    fn processed_marker_mutation_preserves_other_flag_bits() {
        let mut bytes = [0; COMMAND_RECORD_LEN];
        bytes[1] = 0xa4;
        let mut record = CommandRecord::decode_exact(&bytes).unwrap();

        assert!(!record.is_processed());
        record.mark_processed();
        assert!(record.is_processed());
        assert_eq!(record.flags(), 0xa5);
        record.clear_processed();
        assert_eq!(record.flags(), 0xa4);
    }

    #[test]
    fn record_and_payload_widths_are_enforced_exactly() {
        assert_eq!(
            CommandRecord::decode_exact(&[0; COMMAND_RECORD_LEN - 1]),
            Err(CommandRecordError::InvalidLength {
                expected: COMMAND_RECORD_LEN,
                actual: COMMAND_RECORD_LEN - 1,
            })
        );
        assert_eq!(
            CommandRecord::decode_exact(&[0; COMMAND_RECORD_LEN + 1]),
            Err(CommandRecordError::InvalidLength {
                expected: COMMAND_RECORD_LEN,
                actual: COMMAND_RECORD_LEN + 1,
            })
        );
        assert_eq!(
            CommandRecord::encode(4, 0, 0, &[0; COMMAND_RECORD_PAYLOAD_LEN + 1]),
            Err(CommandRecordError::PayloadTooLong {
                max: COMMAND_RECORD_PAYLOAD_LEN,
                actual: COMMAND_RECORD_PAYLOAD_LEN + 1,
            })
        );
    }
}
