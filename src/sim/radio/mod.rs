//! Radio contact RPC vocabulary — the message/response opcodes and payload
//! exchanged over the synchronous contact bus.
//!
//! Defines the message/response vocabulary and the `Contacts` slot store; the
//! `transmit()` bus and the per-category `receive_radio()` handlers land in
//! later slices. Opcodes equal the original radio protocol's wire values so
//! dispatch stays a direct discriminant match. Pure enums + integer slots — no
//! float, no RNG. sim/ only — never render/ui/sidebar/audio/net.
use serde::{Deserialize, Serialize};

pub mod contacts;
pub mod receive;
pub use contacts::Contacts;
pub use receive::{
    receive_radio, refinery_accepted_cell, REFINERY_ACCEPTED_DX, REFINERY_ACCEPTED_DY,
};

use crate::map::entities::EntityCategory;
use crate::sim::world::Simulation;

/// Synchronous radio RPC (§5.2.1). Centralizes the HELLO/BREAK sender-side
/// contact bookkeeping (already-linked ⇒ ROGER without re-dispatch; on ROGER
/// the sender records the contact, self-evicting its own slot 0 when full); BREAK
/// nulls every sender slot to the target before forwarding. Every other opcode
/// dispatches straight to the receiver's [`receive_radio`]. The receiver only
/// ever sees an RTTI-filtered (Techno) sender.
pub fn transmit(
    sim: &mut Simulation,
    sender_sid: u64,
    target_sid: u64,
    msg: RadioMessage,
    payload: RadioPayload,
) -> RadioResponse {
    let filtered = filtered_techno_sender(sim, sender_sid);
    match msg {
        RadioMessage::Hello => transmit_hello(sim, sender_sid, target_sid, filtered),
        RadioMessage::Break => {
            transmit_break(sim, sender_sid, target_sid, filtered);
            RadioResponse::None
        }
        _ => receive_radio(sim, target_sid, filtered, msg, payload),
    }
}

/// RTTI sender filter (§5.2.2): the receiver only sees Unit/Aircraft/Building/
/// Infantry senders. Every `GameEntity` is a Techno, so this currently only
/// drops a vanished sender — kept explicit for the non-Techno cases a later
/// slice may introduce.
fn filtered_techno_sender(sim: &Simulation, sender_sid: u64) -> Option<u64> {
    match sim.substrate.entities.get(sender_sid)?.category {
        EntityCategory::Unit
        | EntityCategory::Infantry
        | EntityCategory::Structure
        | EntityCategory::Aircraft => Some(sender_sid),
    }
}

/// HELLO sender side (§5.2.4): already linked ⇒ ROGER without re-dispatch; else
/// dispatch to the receiver and, on ROGER, record the contact (slot-0 self-evict
/// when the sender's own array is full).
fn transmit_hello(
    sim: &mut Simulation,
    sender_sid: u64,
    target_sid: u64,
    filtered: Option<u64>,
) -> RadioResponse {
    if sim
        .substrate
        .entities
        .get(sender_sid)
        .is_some_and(|s| s.radio_contacts.contains(target_sid))
    {
        return RadioResponse::Roger;
    }
    let response = receive_radio(
        sim,
        target_sid,
        filtered,
        RadioMessage::Hello,
        RadioPayload::default(),
    );
    if response == RadioResponse::Roger {
        if let Some(sender) = sim.substrate.entities.get_mut(sender_sid) {
            // A non-building sender holds capacity 1; the refinery FSM always
            // BREAKs a prior dock before HELLOing a new one, so the self-evict
            // path is dormant here (the evicted partner's BREAK cascade is a
            // later-slice refinement, tracked with the broadcast-BREAK work).
            let _ = sender.radio_contacts.insert_evicting(target_sid);
        }
    }
    response
}

/// BREAK sender side (§5.2.5): null EVERY sender slot matching the target, then
/// forward BREAK so the receiver runs its teardown.
fn transmit_break(
    sim: &mut Simulation,
    sender_sid: u64,
    target_sid: u64,
    filtered: Option<u64>,
) {
    if let Some(sender) = sim.substrate.entities.get_mut(sender_sid) {
        while sender.radio_contacts.remove(target_sid).is_some() {}
    }
    receive_radio(
        sim,
        target_sid,
        filtered,
        RadioMessage::Break,
        RadioPayload::default(),
    );
}

/// A radio message sent from one entity to another. Discriminant = wire opcode.
///
/// Only codes that are sent in stock YR are modelled. Codes marked
/// `name inferred` are behaviour-named (not confirmed wire-string literals).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RadioMessage {
    Hello = 0x02,
    Break = 0x03,
    DockingComplete = 0x07,
    RequestClearance = 0x08,
    DockApproach = 0x0B,       // name inferred
    DockArrived = 0x0C,
    AnimStop = 0x0D,
    CanDock = 0x0E,
    CanEnter = 0x0F,
    IsUnitLinked = 0x11,       // name inferred
    MoveToCell = 0x12,
    NeedToMove = 0x13,
    DockNow = 0x15,
    TimingSync = 0x16,         // name inferred
    EnterDock = 0x18,
    LeaveDock = 0x19,
    SecondaryLockSet = 0x1A,   // name inferred
    SecondaryLockClear = 0x1B, // name inferred
    RepairTick = 0x1C,
    HelipadReserveAck = 0x1D,  // name inferred
    DeploySetNav = 0x1E,       // name inferred
    LinkPassenger = 0x1F,
    IsRepairing = 0x22,
    IsOccupied = 0x23,
}
// Deliberately omitted: 0x10 RESERVE_DOCK (a mission-queue verb argument, not a
// wire message) and 0x24 WANT_RIDE (dormant in stock YR).

impl RadioMessage {
    /// The wire opcode byte.
    #[inline]
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// The reply returned by a `receive_radio` handler. Discriminant = wire opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RadioResponse {
    None = 0,
    Roger = 1,
    Negatory = 0x0A,
    CellAccepted = 0x14,
    Queued = 0x17,
    InsufficientFunds = 0x20,
    RepairComplete = 0x21,
}

impl RadioResponse {
    /// The wire opcode byte.
    #[inline]
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// Optional data carried alongside a radio message (e.g. the CAN_DOCK accepted
/// cell or the MOVE_TO_CELL goal).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RadioPayload {
    /// Target cell `(x, y)`, when the message carries one.
    pub cell: Option<(u16, u16)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_codes_match_wire_opcodes() {
        assert_eq!(RadioMessage::Hello.code(), 0x02);
        assert_eq!(RadioMessage::Break.code(), 0x03);
        assert_eq!(RadioMessage::CanDock.code(), 0x0E);
        assert_eq!(RadioMessage::DockNow.code(), 0x15);
        assert_eq!(RadioMessage::IsOccupied.code(), 0x23);
    }

    #[test]
    fn response_codes_match_wire_opcodes() {
        assert_eq!(RadioResponse::None.code(), 0);
        assert_eq!(RadioResponse::Roger.code(), 1);
        assert_eq!(RadioResponse::Negatory.code(), 0x0A);
        assert_eq!(RadioResponse::CellAccepted.code(), 0x14);
        assert_eq!(RadioResponse::Queued.code(), 0x17);
        assert_eq!(RadioResponse::InsufficientFunds.code(), 0x20);
        assert_eq!(RadioResponse::RepairComplete.code(), 0x21);
    }

    #[test]
    fn payload_defaults_to_no_cell() {
        assert_eq!(RadioPayload::default().cell, None);
    }
}
