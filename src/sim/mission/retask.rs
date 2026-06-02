//! Verb-driven retasking on [`Simulation`] — the single funnel the player-command
//! sites use to drop the old mission and commit a new one.
//!
//! Behavior-preserving in Slice 6: the legacy `Option<T>` machines stay
//! authoritative; these helpers run the verified dock-reservation teardown and
//! write `MissionCom` in parallel via the verb API. The per-site *legacy* field
//! clears (`attack_target`/`order_intent`/`dock_state`/`c4_plant`/`capture_target`/
//! aircraft dock phase) stay inline at the call site — the sites cancel different
//! field subsets (e.g. ForceAttack clears only `order_intent`; ForceAttackCell
//! also clears the aircraft dock phase), so they cannot be folded into a fixed
//! teardown without diverging.

use crate::sim::mission::{verb, MissionType};
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

    /// Retask `id` onto a fresh `mission`: run the dock teardown, then commit the
    /// mission to the substrate via [`verb::assign_mission`] (clears the queued/
    /// suspended interrupt stack + resets the dispatch timer). Use for commands
    /// that start a brand-new order (Move, Stop, RepairAtDepot, EnterTransport,
    /// PlantC4, CaptureBuilding).
    pub fn assign_mission_with_teardown(
        &mut self,
        id: u64,
        mission: MissionType,
        teardown: DockTeardown,
    ) {
        self.run_dock_teardown(id, teardown);
        let now = self.binary_frame;
        if let Some(e) = self.substrate.entities.get_mut(id) {
            verb::assign_mission(&mut e.mission, mission, now);
        }
    }

    /// Like [`Simulation::assign_mission_with_teardown`] but **keeps** the
    /// queued/suspended interrupt stack and dispatch timer — only the current
    /// selector is updated. Use for follow-on combat orders (Attack, ForceAttack,
    /// ForceAttackCell, AttackMove) that should not wipe a pending restore.
    pub fn assign_mission_keep_fields(
        &mut self,
        id: u64,
        mission: MissionType,
        teardown: DockTeardown,
    ) {
        self.run_dock_teardown(id, teardown);
        if let Some(e) = self.substrate.entities.get_mut(id) {
            e.mission.current = mission;
        }
    }
}
