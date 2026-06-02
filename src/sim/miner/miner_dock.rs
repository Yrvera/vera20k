//! Refinery dock contact management.
//!
//! A refinery admits up to `NumberOfDocks` miners into its `Contacts[]` list
//! (capacity-1 for a stock refinery). gamemd stores **no** wait-queue: a denied
//! miner re-probes on demand and whichever re-probing miner wins a freed slot
//! docks next (V3). State lives in `ProductionState.dock_reservations` and is a
//! transitional mirror of the radio-bus `Contacts`/`dock_entered_with` state,
//! retired in a later slice.
//!
//! ## Dependency rules
//! - Part of sim/ -- no dependencies outside sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

/// Tracks which refinery docks are occupied and who is waiting.
///
/// Keyed by refinery StableEntityId. Each refinery has at most one occupant
/// (the miner currently unloading) and a FIFO queue of waiting miners.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DockReservations {
    /// Maps refinery StableEntityId -> currently docked miner StableEntityId.
    pub occupied: BTreeMap<u64, u64>,
    /// Maps refinery StableEntityId -> ordered queue of waiting miner StableEntityIds.
    pub queues: BTreeMap<u64, VecDeque<u64>>,
}

impl DockReservations {
    /// Try to reserve the dock at `refinery_sid` for `miner_sid`.
    ///
    /// Returns `true` if the miner now occupies the dock (immediately granted).
    /// Returns `false` if the dock is occupied — the miner is enqueued instead.
    pub fn try_reserve(&mut self, refinery_sid: u64, miner_sid: u64) -> bool {
        if let Some(&occupant) = self.occupied.get(&refinery_sid) {
            if occupant == miner_sid {
                return true; // already occupying
            }
            // Dock busy — enqueue if not already queued.
            let queue = self.queues.entry(refinery_sid).or_default();
            if !queue.contains(&miner_sid) {
                queue.push_back(miner_sid);
            }
            return false;
        }
        // Dock free — grant immediately.
        self.occupied.insert(refinery_sid, miner_sid);
        true
    }

    /// Release the dock at `refinery_sid`. Returns the next miner promoted
    /// from the queue (if any), which should transition to Dock/Unload.
    pub fn release(&mut self, refinery_sid: u64) -> Option<u64> {
        self.occupied.remove(&refinery_sid);
        let next = self
            .queues
            .get_mut(&refinery_sid)
            .and_then(|q| q.pop_front());
        if let Some(next_miner) = next {
            self.occupied.insert(refinery_sid, next_miner);
            Some(next_miner)
        } else {
            None
        }
    }

    /// Cancel a miner's reservation or queue position at a specific refinery.
    pub fn cancel(&mut self, refinery_sid: u64, miner_sid: u64) {
        if self.occupied.get(&refinery_sid) == Some(&miner_sid) {
            self.occupied.remove(&refinery_sid);
            // Promote next in queue.
            if let Some(next) = self
                .queues
                .get_mut(&refinery_sid)
                .and_then(|q| q.pop_front())
            {
                self.occupied.insert(refinery_sid, next);
            }
        } else if let Some(queue) = self.queues.get_mut(&refinery_sid) {
            queue.retain(|&sid| sid != miner_sid);
        }
    }

    /// Whether the dock at `refinery_sid` is currently occupied.
    pub fn is_occupied(&self, refinery_sid: u64) -> bool {
        self.occupied.contains_key(&refinery_sid)
    }

    /// Remove any references to dead entities (miners or refineries).
    ///
    /// Call at the start of each tick with the set of all alive StableEntityIds
    /// to prevent stale reservations from blocking docks forever.
    pub fn cleanup_dead(&mut self, alive: &BTreeSet<u64>) {
        // Remove dead refineries entirely.
        self.occupied.retain(|ref_sid, _| alive.contains(ref_sid));
        self.queues.retain(|ref_sid, _| alive.contains(ref_sid));

        // Remove dead miners from occupant slots and promote next.
        let dead_occupants: Vec<u64> = self
            .occupied
            .iter()
            .filter(|(_, miner_sid)| !alive.contains(miner_sid))
            .map(|(&ref_sid, _)| ref_sid)
            .collect();
        for ref_sid in dead_occupants {
            self.occupied.remove(&ref_sid);
            if let Some(next) = self.queues.get_mut(&ref_sid).and_then(|q| q.pop_front()) {
                if alive.contains(&next) {
                    self.occupied.insert(ref_sid, next);
                }
            }
        }

        // Remove dead miners from queues.
        for queue in self.queues.values_mut() {
            queue.retain(|sid| alive.contains(sid));
        }
        // Clean up empty queue entries.
        self.queues.retain(|_, q| !q.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_free_dock() {
        let mut docks = DockReservations::default();
        assert!(docks.try_reserve(100, 1));
        assert!(docks.is_occupied(100));
    }

    #[test]
    fn second_miner_queues() {
        let mut docks = DockReservations::default();
        assert!(docks.try_reserve(100, 1));
        assert!(!docks.try_reserve(100, 2));
        assert_eq!(docks.queues[&100].len(), 1);
    }

    #[test]
    fn release_promotes_next() {
        let mut docks = DockReservations::default();
        docks.try_reserve(100, 1);
        docks.try_reserve(100, 2);
        docks.try_reserve(100, 3);
        let promoted = docks.release(100);
        assert_eq!(promoted, Some(2));
        assert_eq!(docks.occupied[&100], 2);
    }

    #[test]
    fn cancel_occupant_promotes() {
        let mut docks = DockReservations::default();
        docks.try_reserve(100, 1);
        docks.try_reserve(100, 2);
        docks.cancel(100, 1);
        assert_eq!(docks.occupied.get(&100), Some(&2));
    }

    #[test]
    fn cleanup_removes_dead() {
        let mut docks = DockReservations::default();
        docks.try_reserve(100, 1);
        docks.try_reserve(100, 2);
        let alive: BTreeSet<u64> = [100, 2].into_iter().collect();
        docks.cleanup_dead(&alive);
        // Miner 1 is dead, miner 2 should be promoted.
        assert_eq!(docks.occupied.get(&100), Some(&2));
    }

    #[test]
    fn idempotent_reserve() {
        let mut docks = DockReservations::default();
        assert!(docks.try_reserve(100, 1));
        assert!(docks.try_reserve(100, 1)); // already occupying
    }

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
        assert_eq!(contacts.hello_or_wait(100, 2, 1), ContactAdmission::Accepted);
        assert!(contacts.has_contact(100, 2));
    }
}
