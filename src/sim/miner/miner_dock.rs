//! Refinery dock contact management.
//!
//! A refinery admits up to `NumberOfDocks` miners into its `Contacts[]` list
//! (capacity-1 for a stock refinery). The `NumberOfDocks` capacity source is
//! VERA-internal: natively the slot count `RadioClass+0xE8` is written only by
//! the `RadioClass` ctor (0x0065A764, = 1) and destructor, never from
//! `NumberOfDocks=` (gamemd equivalent UNCHECKED beyond `+0xE8 == 1` at
//! construction; stock refineries are `NumberOfDocks=1`, so it is inert).
//! gamemd stores **no** wait-queue: a denied
//! miner re-probes on demand and whichever re-probing miner wins a freed slot
//! docks next (V3). State lives in `ProductionState.dock_reservations` and is a
//! transitional mirror of the radio-bus `Contacts`/`dock_entered_with` state,
//! retired in a later slice.
//!
//! ## Dependency rules
//! - Part of sim/ -- no dependencies outside sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet};

/// Result of a refinery HELLO/contact attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactAdmission {
    /// The harvester is present in the refinery Contacts[] list.
    Accepted,
    /// The refinery is saturated. The harvester re-probes on a later tick; there
    /// is no stored wait-queue (V3 — gamemd keeps none).
    Waiting,
}

/// Tracks the refinery radio/contact protocol for harvesters.
///
/// `contacts` mirrors the refinery Contacts[] list populated by HELLO.
/// `contact_entered` mirrors the +0x418-like radio flag set by the 0x18/0x19
/// enter/leave handshake, separate from any conditional +0x2E4 reciprocal
/// building/unit link. `on_pad` is only physical pad occupancy for stock
/// refinery unload/release bookkeeping. There is deliberately **no** wait-queue:
/// a denied miner re-probes and whichever re-probing miner wins a freed slot
/// docks next (V3 — gamemd stores no FIFO).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RefineryDockContacts {
    pub contacts: BTreeMap<u64, Vec<u64>>,
    #[serde(default)]
    pub contact_entered: BTreeMap<u64, u64>,
    pub on_pad: BTreeMap<u64, u64>,
}

impl RefineryDockContacts {
    /// Send HELLO to a refinery. Accepted miners enter Contacts[]; a saturated
    /// refinery replies `Waiting` with no enqueue — the miner re-probes later.
    /// Idempotent: an already-present miner re-confirms `Accepted`.
    pub fn hello_or_wait(
        &mut self,
        refinery_sid: u64,
        miner_sid: u64,
        capacity: usize,
    ) -> ContactAdmission {
        let capacity = capacity.max(1);
        if self.has_contact(refinery_sid, miner_sid) {
            return ContactAdmission::Accepted;
        }

        let contacts_len = self.contacts.get(&refinery_sid).map_or(0, Vec::len);
        if contacts_len >= capacity {
            return ContactAdmission::Waiting;
        }

        self.contacts
            .entry(refinery_sid)
            .or_default()
            .push(miner_sid);
        ContactAdmission::Accepted
    }

    pub fn has_contact(&self, refinery_sid: u64, miner_sid: u64) -> bool {
        self.contacts
            .get(&refinery_sid)
            .is_some_and(|contacts| contacts.contains(&miner_sid))
    }

    /// Read-only contact-slot probe: `RadioClass` helper `FUN_0065ADF0`
    /// (gamemd.exe), which walks `Contacts[0..+0xE8)` at `+0xE4` and answers
    /// true when a slot holds null or already holds the caller. The refinery
    /// scanner `FUN_004DEE80` and `BuildingClass::Receive_Radio @ 0x0043C2D0`
    /// case 0xF both consult it before any HELLO is sent, so selection must
    /// ask without mutating. Capacity floors at 1 like `hello_or_wait`; the
    /// `NumberOfDocks`-derived capacity callers pass is VERA-internal (native
    /// `+0xE8` is fixed at 1 by the `RadioClass` ctor — see the module doc).
    pub fn would_admit(&self, refinery_sid: u64, miner_sid: u64, capacity: usize) -> bool {
        self.has_contact(refinery_sid, miner_sid)
            || self.contacts.get(&refinery_sid).map_or(0, Vec::len) < capacity.max(1)
    }

    pub fn link_on_pad(&mut self, refinery_sid: u64, miner_sid: u64) {
        self.on_pad.insert(refinery_sid, miner_sid);
    }

    pub fn mark_contact_entered(&mut self, refinery_sid: u64, miner_sid: u64) {
        self.contact_entered.insert(refinery_sid, miner_sid);
    }

    pub fn clear_contact_entered(&mut self, refinery_sid: u64, miner_sid: u64) {
        if self.contact_entered.get(&refinery_sid) == Some(&miner_sid) {
            self.contact_entered.remove(&refinery_sid);
        }
    }

    pub fn has_contact_entered(&self, refinery_sid: u64, miner_sid: u64) -> bool {
        self.contact_entered.get(&refinery_sid) == Some(&miner_sid)
    }

    pub fn release_on_pad(&mut self, refinery_sid: u64, miner_sid: u64) {
        if self.on_pad.get(&refinery_sid) == Some(&miner_sid) {
            self.on_pad.remove(&refinery_sid);
        }
    }

    pub fn is_on_pad(&self, refinery_sid: u64, miner_sid: u64) -> bool {
        self.on_pad.get(&refinery_sid) == Some(&miner_sid)
    }

    pub fn pad_occupied(&self, refinery_sid: u64) -> bool {
        self.on_pad.contains_key(&refinery_sid)
    }

    pub fn release_contact(&mut self, refinery_sid: u64, miner_sid: u64) {
        if let Some(contacts) = self.contacts.get_mut(&refinery_sid) {
            contacts.retain(|&sid| sid != miner_sid);
        }
        self.contacts.retain(|_, contacts| !contacts.is_empty());
        self.clear_contact_entered(refinery_sid, miner_sid);
    }

    pub fn cancel_miner(&mut self, refinery_sid: u64, miner_sid: u64) {
        self.release_on_pad(refinery_sid, miner_sid);
        self.release_contact(refinery_sid, miner_sid);
    }

    pub fn cleanup_dead(&mut self, alive: &BTreeSet<u64>) {
        self.contacts.retain(|ref_sid, contacts| {
            if !alive.contains(ref_sid) {
                return false;
            }
            contacts.retain(|sid| alive.contains(sid));
            !contacts.is_empty()
        });
        self.contact_entered
            .retain(|ref_sid, miner_sid| alive.contains(ref_sid) && alive.contains(miner_sid));
        self.on_pad
            .retain(|ref_sid, miner_sid| alive.contains(ref_sid) && alive.contains(miner_sid));
    }

    /// Compatibility helper for older miner tests: HELLO with one dock.
    pub fn try_reserve(&mut self, refinery_sid: u64, miner_sid: u64) -> bool {
        self.hello_or_wait(refinery_sid, miner_sid, 1) == ContactAdmission::Accepted
    }

    /// Compatibility helper for older miner tests: release contact and pad link.
    /// Returns `None` — there is no FIFO promotion (V3); the next docker is
    /// whichever waiting miner re-probes and wins the freed slot.
    pub fn release(&mut self, refinery_sid: u64) -> Option<u64> {
        let released = self
            .contacts
            .get(&refinery_sid)
            .and_then(|contacts| contacts.first().copied());
        if let Some(miner_sid) = released {
            self.release_on_pad(refinery_sid, miner_sid);
            self.release_contact(refinery_sid, miner_sid);
        }
        None
    }

    /// Compatibility helper for older miner tests: cancel miner at refinery.
    pub fn cancel(&mut self, refinery_sid: u64, miner_sid: u64) {
        self.cancel_miner(refinery_sid, miner_sid);
    }

    /// True when the refinery has an active contact or on-pad miner.
    pub fn is_occupied(&self, refinery_sid: u64) -> bool {
        self.contacts
            .get(&refinery_sid)
            .is_some_and(|contacts| !contacts.is_empty())
            || self.contact_entered.contains_key(&refinery_sid)
            || self.on_pad.contains_key(&refinery_sid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_contact_does_not_promote_waiter() {
        let mut contacts = RefineryDockContacts::default();
        assert_eq!(
            contacts.hello_or_wait(100, 1, 1),
            ContactAdmission::Accepted
        );
        contacts.mark_contact_entered(100, 1);
        contacts.link_on_pad(100, 1);
        // Saturated refinery denies the second miner with no stored queue (V3).
        assert_eq!(contacts.hello_or_wait(100, 2, 1), ContactAdmission::Waiting);

        contacts.release_contact(100, 1);

        assert!(!contacts.has_contact(100, 1));
        assert!(!contacts.has_contact_entered(100, 1));
        assert!(
            !contacts.has_contact(100, 2),
            "release_contact must not promote anyone into Contacts[] — there is no FIFO"
        );
        assert!(
            contacts.is_on_pad(100, 1),
            "release_contact does not clear physical pad occupancy"
        );

        // The slot is free; miner 2 docks only by re-probing (winning on demand).
        assert_eq!(
            contacts.hello_or_wait(100, 2, 1),
            ContactAdmission::Accepted
        );
        assert!(contacts.has_contact(100, 2));
    }

    #[test]
    fn would_admit_free_slot_tracked_and_saturated() {
        let mut c = RefineryDockContacts::default();
        assert!(c.would_admit(1, 10, 1), "empty refinery admits");
        assert_eq!(c.hello_or_wait(1, 11, 1), ContactAdmission::Accepted);
        assert!(
            !c.would_admit(1, 10, 1),
            "saturated refinery rejects a stranger"
        );
        assert!(c.would_admit(1, 11, 1), "already-tracked miner is admitted");
        assert!(c.would_admit(1, 10, 2), "capacity 2 leaves a free slot");
        assert!(c.would_admit(2, 10, 0), "capacity floors at 1");
    }
}
