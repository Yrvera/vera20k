//! Per-object mission dispatch router — the Rust-native stand-in for the common
//! mission-dispatch switch (`match mission`), at Unit handler-family granularity.
//!
//! THIS SLICE the router is a read-only classifier: it maps a `MissionType` to the
//! coarse handler *family* a Unit's behaviour uses, so the per-object AI shell can route
//! each live Unit without executing or moving any handler body. The full per-handler slot
//! identity (every distinct dispatched-handler) is deferred to the all-category slice.
//!
//! Depends on `mission` (MissionType) only — `sim/` only, never render/ui/sidebar/audio/net.
//! No `dyn`/vtable — data, not trait objects.

use super::MissionType;

/// The coarse Unit handler family a mission routes to. NOT a 1:1 of every distinct
/// dispatched handler — only the families a Unit's behaviour actually uses, plus the two
/// inert buckets. The full per-handler slot table is the all-category slice's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchSlot {
    /// Idle / Sleep family (also the `QMove` default and the `None` idle sentinel).
    Sleep,
    Move,
    Attack,
    /// Guard family — `Guard` and `Sticky` share it.
    Guard,
    /// Dock/enter family.
    Enter,
    /// Harvest family (reachable only for miners, which the live host skips this slice).
    Harvest,
    /// `AttackMove`: an assign-side selector that is NEVER a committed current mission —
    /// the assign side prevents it (and `derived_mission` never yields it for a Unit). The
    /// dispatch switch has NO special skip for it: if 29 ever reached dispatch the binary
    /// would route it via `default` to the Sleep handler WITH a timer rewrite (same as
    /// QMove). `Skip` therefore models the should-never-reach-here invariant — it does NOT
    /// mirror a binary "skip". Kept as a distinct, defensive variant so the unreachability
    /// is asserted, not silently folded into `Sleep`.
    Skip,
    /// Any mission with no Unit handler family this slice (Capture/Eaten/AreaGuard/Return/
    /// Stop/Ambush/Hunt/Unload/Sabotage/Construction/Selling/Repair/Rescue/Missile/Harmless/
    /// Open/Patrol/Paradrop/Deliberate/Spyplane/Retreat) — represented but inert for Units.
    OtherInert,
}

/// Route a mission to its Unit handler family. Total over all 32 dispatched missions plus
/// the `None` idle sentinel; pure; no panics. The reachable-Unit set
/// `{Move, Attack, Enter, Harvest, Guard, None}` maps to live families; everything else is
/// `Skip` (AttackMove) or `OtherInert`.
#[inline]
pub fn unit_dispatch_family(mission: MissionType) -> DispatchSlot {
    use MissionType as M;
    match mission {
        M::Move => DispatchSlot::Move,
        M::Attack => DispatchSlot::Attack,
        M::Enter => DispatchSlot::Enter,
        M::Harvest => DispatchSlot::Harvest,
        // Guard family: Guard + Sticky share the slot.
        M::Guard | M::Sticky => DispatchSlot::Guard,
        // Sleep family: explicit Sleep, the QMove default, and the idle sentinel.
        M::Sleep | M::QMove | M::None => DispatchSlot::Sleep,
        // AttackMove is never a committed current mission (assign-side prevents it); the
        // dispatcher has no skip for it. `Skip` models the should-never-reach-here case.
        M::AttackMove => DispatchSlot::Skip,
        // Everything else has no Unit handler family this slice.
        _ => DispatchSlot::OtherInert,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::mission::MissionType;

    #[test]
    fn reachable_unit_missions_route_to_live_families() {
        assert_eq!(unit_dispatch_family(MissionType::Move), DispatchSlot::Move);
        assert_eq!(unit_dispatch_family(MissionType::Attack), DispatchSlot::Attack);
        assert_eq!(unit_dispatch_family(MissionType::Enter), DispatchSlot::Enter);
        assert_eq!(unit_dispatch_family(MissionType::Harvest), DispatchSlot::Harvest);
        assert_eq!(unit_dispatch_family(MissionType::Guard), DispatchSlot::Guard);
        assert_eq!(unit_dispatch_family(MissionType::None), DispatchSlot::Sleep);
        for slot in [
            unit_dispatch_family(MissionType::Move),
            unit_dispatch_family(MissionType::Attack),
            unit_dispatch_family(MissionType::Enter),
            unit_dispatch_family(MissionType::Guard),
            unit_dispatch_family(MissionType::None),
        ] {
            assert!(
                !matches!(slot, DispatchSlot::Skip | DispatchSlot::OtherInert),
                "non-miner reachable Unit missions must route to a live family"
            );
        }
    }

    #[test]
    fn documented_groupings_and_specials() {
        // Sticky shares the Guard family.
        assert_eq!(
            unit_dispatch_family(MissionType::Sticky),
            unit_dispatch_family(MissionType::Guard)
        );
        // QMove defaults to the Sleep family.
        assert_eq!(
            unit_dispatch_family(MissionType::QMove),
            unit_dispatch_family(MissionType::Sleep)
        );
        // AttackMove is the defensive skip bucket — never a committed Unit mission.
        assert_eq!(unit_dispatch_family(MissionType::AttackMove), DispatchSlot::Skip);
        // A representative TS-legacy / non-Unit mission is inert.
        assert_eq!(unit_dispatch_family(MissionType::Ambush), DispatchSlot::OtherInert);
        assert_eq!(unit_dispatch_family(MissionType::Capture), DispatchSlot::OtherInert);
    }

    #[test]
    fn router_is_total_over_all_missions() {
        // Every dispatched id (0..=31) plus None routes without panic.
        for m in MissionType::all() {
            let _ = unit_dispatch_family(m);
        }
        let _ = unit_dispatch_family(MissionType::None);
    }
}
