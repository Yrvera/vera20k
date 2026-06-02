//! Mission verb API — the pure dispatch surface over [`MissionCom`].
//!
//! These are the Rust-native equivalents of the native mission queue/commence/
//! override/restore operations. Every verb is a pure function of
//! `(MissionCom, arg, now)` — no clock, no RNG, no float — so the scheduler is
//! trivially deterministic and unit-testable in isolation. The `Simulation`
//! wrappers (`mission::retask`) layer the legacy dock teardown on top.
//!
//! Slice 6 scope: the verbs write `MissionCom` in parallel with the still-
//! authoritative `Option<T>` machines. The interrupt guards (Selling /
//! Deliberate) and the `ready_to_commence` per-category hook are encoded from
//! the verified predicate *structure*; the exact excluded-mission set and the
//! busy byte-flag semantics carry V1 STILL-UNCHECKED residue and are traced
//! before any *live* commence path relies on the gate.

use crate::map::entities::EntityCategory;

use super::{MissionCom, MissionType};

/// The effective current mission. Falls back to the queued follow-up when no
/// mission is committed (so a freshly-queued order reads as "what's next").
#[inline]
pub fn get_current_mission(com: &MissionCom) -> MissionType {
    if com.current != MissionType::None {
        com.current
    } else {
        com.queued.unwrap_or(MissionType::None)
    }
}

/// True when a committed (non-idle) mission is active.
///
/// V1 STILL-UNCHECKED: the native busy predicate also consults per-subclass
/// byte flags + the locomotor idle latch. Slice 6 uses the minimal honest
/// definition (idle ⇔ no committed mission); the richer predicate is folded in
/// when a live commence path needs it.
#[inline]
pub fn is_busy(com: &MissionCom) -> bool {
    com.current != MissionType::None
}

/// Whether a transition from `current` to `target` is blocked by an interrupt
/// guard. Two hardcoded native guards:
///   * `Selling` blocks **all** transitions — the sell is irreversible.
///   * `Deliberate` (the guard-protected interrupt-wait) ignores a `Guard`
///     interrupt only; other targets pass.
#[inline]
pub fn is_transition_blocked(current: MissionType, target: MissionType) -> bool {
    match current {
        MissionType::Selling => true,
        MissionType::Deliberate => target == MissionType::Guard,
        _ => false,
    }
}

/// Force a fresh current mission: clears the queued/suspended interrupt stack
/// and resets the dispatch timer. Bypasses the interrupt guards (force-promote,
/// the `assign_mission` contract — only `queue_mission(commence)` consults the
/// `ready_to_commence` hook).
#[inline]
pub fn assign_mission(com: &mut MissionCom, mission: MissionType, now: u32) {
    com.current = mission;
    com.queued = None;
    com.suspended = None;
    com.substate = 0;
    com.timer.reset(now);
}

/// Queue a follow-up mission to commence after the current one. Respects the
/// interrupt guard; returns `false` (no-op) when blocked.
#[inline]
pub fn queue_mission(com: &mut MissionCom, mission: MissionType) -> bool {
    if is_transition_blocked(com.current, mission) {
        return false;
    }
    com.queued = Some(mission);
    true
}

/// Promote the queued mission to current, resetting the dispatch timer.
/// Returns `false` when nothing is queued.
#[inline]
pub fn commence_queued(com: &mut MissionCom, now: u32) -> bool {
    match com.queued.take() {
        Some(m) => {
            com.current = m;
            com.substate = 0;
            com.timer.reset(now);
            true
        }
        None => false,
    }
}

/// Suspend the current mission and switch to `mission`, saving the prior intent
/// for [`restore_mission`]. If a mission was queued, the override discards the
/// current mission and saves the *queued* one (it is the pending intent);
/// otherwise it saves the current mission. Respects the interrupt guard.
#[inline]
pub fn override_mission(com: &mut MissionCom, mission: MissionType, now: u32) -> bool {
    if is_transition_blocked(com.current, mission) {
        return false;
    }
    com.suspended = match com.queued.take() {
        Some(queued) => Some(queued),
        None => Some(com.current),
    };
    com.current = mission;
    com.substate = 0;
    com.timer.reset(now);
    true
}

/// Restore a suspended mission to current, resetting the dispatch timer.
/// Returns `false` when nothing is suspended.
#[inline]
pub fn restore_mission(com: &mut MissionCom, now: u32) -> bool {
    match com.suspended.take() {
        Some(m) => {
            com.current = m;
            com.substate = 0;
            com.timer.reset(now);
            true
        }
        None => false,
    }
}

/// The four leaf entity categories that override the native commence predicate.
/// (`Structure` maps to `Building`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyCategory {
    Building,
    Unit,
    Infantry,
    Aircraft,
}

impl From<EntityCategory> for ReadyCategory {
    fn from(cat: EntityCategory) -> Self {
        match cat {
            EntityCategory::Structure => ReadyCategory::Building,
            EntityCategory::Unit => ReadyCategory::Unit,
            EntityCategory::Infantry => ReadyCategory::Infantry,
            EntityCategory::Aircraft => ReadyCategory::Aircraft,
        }
    }
}

/// Inputs the per-category commence predicate reads. Slice 6 carries only the
/// verified-structural fields; the full per-subclass busy byte-flags are V1
/// STILL-UNCHECKED and added when a live commence path needs them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadySnapshot {
    pub category: ReadyCategory,
    /// The locomotor is actively driving (native locomotor `+0x80` idle latch
    /// is clear). A moving vehicle is not yet ready to commence a new mission.
    pub is_driving: bool,
}

/// The per-`ReadyCategory` commence gate. The native base predicate is
/// `return 1`; each leaf type overrides it. Slice 6 encodes the verified
/// *structure* — base always-ready, a driving vehicle not-ready — and is
/// exercised live only by the dock slices' `Queue_Mission` reserve path.
#[inline]
pub fn ready_to_commence(snap: &ReadySnapshot) -> bool {
    match snap.category {
        ReadyCategory::Unit => !snap.is_driving,
        // Building / Infantry / Aircraft: base predicate until the per-type
        // excluded-mission set is traced (V1 STILL-UNCHECKED).
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::mission::MissionTimer;

    fn com_with(current: MissionType) -> MissionCom {
        MissionCom {
            current,
            ..MissionCom::idle()
        }
    }

    #[test]
    fn slice6_assign_forces_clears_and_resets_timer() {
        let mut com = com_with(MissionType::Guard);
        com.queued = Some(MissionType::Attack);
        com.suspended = Some(MissionType::Move);
        com.timer = MissionTimer::armed(5, 99);
        assign_mission(&mut com, MissionType::Harvest, 42);
        assert_eq!(com.current, MissionType::Harvest);
        assert_eq!(com.queued, None, "assign clears the queued follow-up");
        assert_eq!(com.suspended, None, "assign clears the suspended stack");
        // reset(now) == defer(now, 0): armed at `now`, zero duration → due next.
        assert_eq!(com.timer.start_frame, 42);
        assert_eq!(com.timer.duration, 0);
    }

    #[test]
    fn slice6_assign_force_promotes_past_selling_guard() {
        // assign bypasses the interrupt guard (force-promote contract).
        let mut com = com_with(MissionType::Selling);
        assign_mission(&mut com, MissionType::Move, 1);
        assert_eq!(com.current, MissionType::Move);
    }

    #[test]
    fn slice6_selling_blocks_all_transitions() {
        assert!(is_transition_blocked(MissionType::Selling, MissionType::Move));
        assert!(is_transition_blocked(MissionType::Selling, MissionType::Guard));
        assert!(is_transition_blocked(MissionType::Selling, MissionType::Attack));
    }

    #[test]
    fn slice6_deliberate_blocks_only_guard_target() {
        assert!(is_transition_blocked(
            MissionType::Deliberate,
            MissionType::Guard
        ));
        assert!(!is_transition_blocked(
            MissionType::Deliberate,
            MissionType::Attack
        ));
        assert!(!is_transition_blocked(
            MissionType::Deliberate,
            MissionType::Move
        ));
    }

    #[test]
    fn slice6_queue_and_override_respect_guard() {
        let mut com = com_with(MissionType::Deliberate);
        assert!(!queue_mission(&mut com, MissionType::Guard), "guard blocked");
        assert_eq!(com.queued, None);
        assert!(!override_mission(&mut com, MissionType::Guard, 0), "guard blocked");
        assert_eq!(com.current, MissionType::Deliberate);
        // A non-Guard target passes the Deliberate guard.
        assert!(queue_mission(&mut com, MissionType::Attack));
        assert_eq!(com.queued, Some(MissionType::Attack));
    }

    #[test]
    fn slice6_override_without_queued_saves_current_then_restore() {
        let mut com = com_with(MissionType::Guard);
        assert!(override_mission(&mut com, MissionType::Attack, 7));
        assert_eq!(com.current, MissionType::Attack);
        assert_eq!(com.suspended, Some(MissionType::Guard), "saved current");
        assert!(restore_mission(&mut com, 9));
        assert_eq!(com.current, MissionType::Guard, "restored");
        assert_eq!(com.suspended, None);
        assert!(!restore_mission(&mut com, 9), "nothing left to restore");
    }

    #[test]
    fn slice6_override_with_queued_discards_current_saves_queued() {
        let mut com = com_with(MissionType::Guard);
        com.queued = Some(MissionType::Harvest);
        assert!(override_mission(&mut com, MissionType::Attack, 3));
        assert_eq!(com.current, MissionType::Attack);
        assert_eq!(
            com.suspended,
            Some(MissionType::Harvest),
            "saved the queued intent, discarded current"
        );
        assert_eq!(com.queued, None);
    }

    #[test]
    fn slice6_commence_queued_promotes_or_noops() {
        let mut com = com_with(MissionType::None);
        assert!(!commence_queued(&mut com, 0), "nothing queued");
        com.queued = Some(MissionType::Move);
        assert!(commence_queued(&mut com, 5));
        assert_eq!(com.current, MissionType::Move);
        assert_eq!(com.queued, None);
        assert_eq!(com.timer.start_frame, 5);
    }

    #[test]
    fn slice6_get_current_falls_back_to_queued() {
        let mut com = com_with(MissionType::None);
        assert_eq!(get_current_mission(&com), MissionType::None);
        com.queued = Some(MissionType::Attack);
        assert_eq!(
            get_current_mission(&com),
            MissionType::Attack,
            "idle current falls back to the queued mission"
        );
        com.current = MissionType::Move;
        assert_eq!(
            get_current_mission(&com),
            MissionType::Move,
            "a committed current wins over queued"
        );
    }

    #[test]
    fn slice6_is_busy_tracks_committed_mission() {
        assert!(!is_busy(&com_with(MissionType::None)));
        assert!(is_busy(&com_with(MissionType::Attack)));
    }

    #[test]
    fn slice6_ready_to_commence_base_true_unit_not_while_driving() {
        // Base predicate (Building / Infantry / Aircraft): always ready.
        for cat in [
            ReadyCategory::Building,
            ReadyCategory::Infantry,
            ReadyCategory::Aircraft,
        ] {
            assert!(ready_to_commence(&ReadySnapshot {
                category: cat,
                is_driving: true,
            }));
        }
        // Unit: not ready while driving, ready when idle.
        assert!(!ready_to_commence(&ReadySnapshot {
            category: ReadyCategory::Unit,
            is_driving: true,
        }));
        assert!(ready_to_commence(&ReadySnapshot {
            category: ReadyCategory::Unit,
            is_driving: false,
        }));
    }
}
