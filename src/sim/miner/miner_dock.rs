//! Refinery dock contact and queue management.
//!
//! Only one miner may occupy a refinery dock at a time. Additional miners
//! queue up and retry in FIFO order after the dock contact is released.
//! State lives in `ProductionState.dock_reservations` (shared across entities).
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
    /// The harvester must wait and retry later.
    Waiting,
}

/// Tracks the verified refinery radio/contact protocol for harvesters.
///
/// `contacts` mirrors the refinery Contacts[] list populated by HELLO.
/// `waiting_retry_queue` is Rust-side deterministic retry ordering for miners
/// that received a negative reply. `contact_entered` mirrors the +0x418-like
/// radio flag set by the 0x18/0x19 enter/leave handshake, separate from any
/// conditional +0x2E4 reciprocal building/unit link. `on_pad` is only physical
/// pad occupancy for stock refinery unload/release bookkeeping.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RefineryDockContacts {
    pub contacts: BTreeMap<u64, Vec<u64>>,
    pub waiting_retry_queue: BTreeMap<u64, VecDeque<u64>>,
    #[serde(default)]
    pub contact_entered: BTreeMap<u64, u64>,
    pub on_pad: BTreeMap<u64, u64>,
}

impl RefineryDockContacts {
    /// Send HELLO to a refinery. Accepted miners enter Contacts[]; rejected
    /// miners wait in deterministic FIFO retry order.
    pub fn hello_or_wait(
        &mut self,
        refinery_sid: u64,
        miner_sid: u64,
        capacity: usize,
    ) -> ContactAdmission {
        let capacity = capacity.max(1);
        if self.has_contact(refinery_sid, miner_sid) {
            self.remove_waiter(refinery_sid, miner_sid);
            return ContactAdmission::Accepted;
        }

        let contacts_len = self.contacts.get(&refinery_sid).map_or(0, Vec::len);
        let queue = self.waiting_retry_queue.entry(refinery_sid).or_default();
        if contacts_len >= capacity {
            if !queue.contains(&miner_sid) {
                queue.push_back(miner_sid);
            }
            return ContactAdmission::Waiting;
        }

        if let Some(front) = queue.front().copied() {
            if front != miner_sid {
                if !queue.contains(&miner_sid) {
                    queue.push_back(miner_sid);
                }
                return ContactAdmission::Waiting;
            }
            queue.pop_front();
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

    pub fn is_waiting(&self, refinery_sid: u64, miner_sid: u64) -> bool {
        self.waiting_retry_queue
            .get(&refinery_sid)
            .is_some_and(|queue| queue.contains(&miner_sid))
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
        self.remove_waiter(refinery_sid, miner_sid);
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
        self.waiting_retry_queue.retain(|ref_sid, queue| {
            if !alive.contains(ref_sid) {
                return false;
            }
            queue.retain(|sid| alive.contains(sid));
            !queue.is_empty()
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
    pub fn release(&mut self, refinery_sid: u64) -> Option<u64> {
        let released = self
            .contacts
            .get(&refinery_sid)
            .and_then(|contacts| contacts.first().copied());
        if let Some(miner_sid) = released {
            self.release_on_pad(refinery_sid, miner_sid);
            self.release_contact(refinery_sid, miner_sid);
        }
        self.waiting_retry_queue
            .get(&refinery_sid)
            .and_then(|queue| queue.front().copied())
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

    fn remove_waiter(&mut self, refinery_sid: u64, miner_sid: u64) {
        if let Some(queue) = self.waiting_retry_queue.get_mut(&refinery_sid) {
            queue.retain(|&sid| sid != miner_sid);
        }
        self.waiting_retry_queue
            .retain(|_, queue| !queue.is_empty());
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
        assert_eq!(contacts.hello_or_wait(100, 2, 1), ContactAdmission::Waiting);

        contacts.release_contact(100, 1);

        assert!(!contacts.has_contact(100, 1));
        assert!(!contacts.has_contact_entered(100, 1));
        assert!(
            contacts.is_waiting(100, 2),
            "front waiter must remain queued until its own retry"
        );
        assert!(
            !contacts.has_contact(100, 2),
            "release_contact must not promote a waiter into Contacts[]"
        );
        assert!(
            contacts.is_on_pad(100, 1),
            "release_contact does not clear physical pad occupancy"
        );

        contacts.release_on_pad(100, 1);
        assert!(!contacts.is_on_pad(100, 1));
        assert!(contacts.is_waiting(100, 2));
        assert!(!contacts.has_contact(100, 2));
    }
}
