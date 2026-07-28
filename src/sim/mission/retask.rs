//! Verb-driven retasking on [`Simulation`] — the single funnel the player-command
//! sites use to queue a fresh mission.
//!
//! The mission write is the native event-execute shape: synchronized command
//! execution queues the selected mission with `commence_now = 0`
//! (`EventClass` execute, Queue callsite `0x004C73B9=dynamic/0`); the
//! per-object AI host promotes it via Ready→Commence on its next update. The
//! legacy `Option<T>` machines stay the behavior drivers; the per-site field
//! clears (`attack_target`/`order_intent`/`dock_state`/`c4_plant`/
//! `capture_target`/aircraft dock phase) stay inline at the call site — the
//! sites cancel different field subsets, so they cannot be folded into a fixed
//! teardown without diverging.

use crate::sim::mission::authority::EntityReadyInputProvider;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::world::Simulation;

/// Which dock-reservation teardown a retasking command performs.
///
/// This governs **only** the three reservation helpers — it is the one part of
/// the per-command teardown that is a closed, enumerable set. The variant for
/// each site is the exact subset that site cancels today:
///
/// | site | variant | cancels |
/// |---|---|---|
/// | Move | `All` | depot + aircraft RTB/wait + docked-idle |
/// | Stop, RepairAtDepot | `Depot` | depot reservation only |
/// | Attack | `AircraftOnly` | aircraft RTB/wait + docked-idle (NOT depot) |
/// | ForceAttack, ForceAttackCell, AttackMove | `IdleOnly` | docked-idle only |
/// | EnterTransport, PlantC4, CaptureBuilding | `None` | nothing |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockTeardown {
    /// Depot + aircraft RTB/wait + docked-idle (Move).
    All,
    /// Depot reservation only (Stop, RepairAtDepot).
    Depot,
    /// Aircraft RTB/wait + docked-idle, but NOT the depot reservation (Attack).
    AircraftOnly,
    /// Docked-idle helipad release only (ForceAttack, ForceAttackCell, AttackMove).
    IdleOnly,
    /// No dock reservation touched (EnterTransport, PlantC4, CaptureBuilding).
    None,
}

impl Simulation {
    /// Run the dock-reservation subset selected by `teardown`. Each branch calls
    /// the exact reservation helpers the corresponding command sites call today.
    fn run_dock_teardown(&mut self, id: u64, teardown: DockTeardown) {
        match teardown {
            DockTeardown::All => {
                self.cancel_depot_dock(id);
                self.cancel_aircraft_dock(id);
                self.release_docked_idle(id);
            }
            DockTeardown::Depot => {
                self.cancel_depot_dock(id);
            }
            DockTeardown::AircraftOnly => {
                self.cancel_aircraft_dock(id);
                self.release_docked_idle(id);
            }
            DockTeardown::IdleOnly => {
                self.release_docked_idle(id);
            }
            DockTeardown::None => {}
        }
    }

    /// Retask `id` onto a fresh `mission`: run the dock teardown, then queue
    /// the mission through the exact authority with `commence_now = 0` — the
    /// native event-execute shape (Queue `0x004C73B9=dynamic/0`). Promotion to
    /// `current` happens at the per-object AI host's Ready→Commence. Used by
    /// every player command site (Move, Stop, Attack, ForceAttack,
    /// ForceAttackCell, AttackMove, RepairAtDepot, EnterTransport, PlantC4,
    /// CaptureBuilding).
    pub fn queue_mission_with_teardown(
        &mut self,
        id: u64,
        mission: MissionType,
        teardown: DockTeardown,
    ) {
        self.run_dock_teardown(id, teardown);
        let now = self.session.binary_frame;
        // A missing receiver performs no write (same as the native event path
        // skipping a dead target); Queue's own guards decide the rest.
        let _ = self.mission_queue_exact(
            id,
            MissionId::from_known(mission),
            0,
            now,
            &EntityReadyInputProvider,
        );
    }
}
