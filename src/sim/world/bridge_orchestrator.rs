//! Bridge damage orchestrator — 4-path dispatcher + cascade consumers.
//!
//! Per-tick entry that drains `BridgeDamageEvent`s emitted by combat, runs
//! each event through the 4-path dispatcher (HighSM → LowSM → LowDirect →
//! HighDirect, in fixed order), applies the per-path BridgeStrength RNG
//! gate, runs the IonCannon retry loop on state-machine paths only, and
//! (in later tasks) applies the BlowUpBridge cascade: ground-occupant
//! kill, bridge-deck DropIn, debris spawn, rim refresh, trigger broadcast,
//! zone rebuild.
//!
//! ## Dependency rules
//! Same as sim/world: depends on sim/bridge_state, sim/rng, rules/, map/;
//! never render / ui / audio / net.
//!
//! ## Status
//! Task 9: scaffolding + dispatcher loop only — cascade consumers stubbed.
//! The orchestrator is NOT wired into the world tick yet; the legacy
//! `Simulation::apply_bridge_damage_events` + `resolve_bridge_state_changes`
//! still drive bridge damage. The atomic switchover lands in Task 14.

use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{
    BridgeDamageContext, BridgeDamageEvent, DispatchPath, StateOutcome,
};
use crate::sim::world::Simulation;

/// Drain a batch of `BridgeDamageEvent`s through the 4-path dispatcher.
///
/// Per-event behavior:
/// 1. Outer gate: if `SpecialFlags::DestroyableBridges` is clear, bail
///    early — bridges are immune.
/// 2. For each event, evaluate paths in fixed order
///    `HighSM → LowSM → LowDirect → HighDirect`.
/// 3. For each matching path, run the per-path RNG gate against
///    BridgeStrength (`damage > rand(1..=BridgeStrength)`). IonCannon
///    bypasses the gate.
/// 4. State-machine paths get up to 3 retries when the warhead is
///    IonCannon (4 attempts total). Direct-overlay paths are single-shot.
/// 5. The first path that produces a non-`NoChange` outcome is the
///    winner; subsequent paths skip for that event.
///
/// Returns the list of entity IDs despawned by the cascade. Per the
/// DropIn correction (Task 11), this list is typically empty — bridge-
/// deck entities survive stranded rather than despawning.
///
/// **Task 9: cascade consumers are stubbed.** Outcomes are collected but
/// no kill / DropIn / debris / rim / zone work happens yet — those wire
/// in Tasks 10-13. Callers should not yet use the return value.
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> Vec<u64> {
    let despawned_ids: Vec<u64> = Vec::new();
    if events.is_empty() {
        return despawned_ids;
    }

    // Outer gate + read bridge_strength up front (immutable borrow scope).
    let bridge_strength = match sim.bridge_state.as_ref() {
        Some(bs) if bs.is_destroyable() => bs.bridge_strength(),
        _ => return despawned_ids,
    };

    // Run dispatch loop with split borrows: bridge_state &mut, terrain &,
    // rng &mut. Outcomes are collected for the cascade phase (Tasks 10-13).
    let _outcomes: Vec<StateOutcome> = run_dispatch_loop(sim, events, bridge_strength);

    // TODO(Task 10-13): drain `_outcomes` and apply cascade
    // (kill ground occupants, DropIn deck entities, spawn debris,
    // rim refresh, trigger 31, zone rebuild).
    let _ = rules; // RuleSet is used by Task 10's C4Warhead force-kill.

    despawned_ids
}

/// Inner dispatch loop. Owns the split borrow of `Simulation` so the
/// dispatcher can read terrain immutably while mutating bridge_state +
/// rng. Returns a `StateOutcome` per event whose path matched and whose
/// driver did real work.
fn run_dispatch_loop(
    sim: &mut Simulation,
    events: &[BridgeDamageEvent],
    bridge_strength: u16,
) -> Vec<StateOutcome> {
    let mut outcomes = Vec::with_capacity(events.len());

    // Split-borrow projections so the dispatcher can hold &mut
    // bridge_state + & terrain + &mut rng simultaneously.
    let Some(terrain) = sim.resolved_terrain.as_ref() else {
        return outcomes;
    };
    // SAFETY of split: we only project `&` to `resolved_terrain` (no
    // mutation downstream), `&mut` to `bridge_state`, `&mut` to `rng` —
    // disjoint fields of `Simulation`. The compiler accepts this when
    // each projection is a direct field access through `sim`.
    let terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid = terrain;
    let bridge_state = match sim.bridge_state.as_mut() {
        Some(bs) => bs,
        None => return outcomes,
    };
    let rng = &mut sim.rng;

    for event in events {
        let ctx = BridgeDamageContext {
            damage: event.damage,
            warhead_ref: event.warhead_ref,
            is_ion_cannon: event.is_ion_cannon,
            bridge_strength,
            impact_z: event.impact_z,
        };

        // 4 paths in fixed order — RNG draw order is parity-critical.
        for path in [
            DispatchPath::HighStateMachine,
            DispatchPath::LowStateMachine,
            DispatchPath::LowDirect,
            DispatchPath::HighDirect,
        ] {
            if !bridge_state.path_matches_cell(path, event.rx, event.ry, &ctx, terrain) {
                continue;
            }

            // Per-path BridgeStrength RNG gate. IonCannon bypasses.
            if !ctx.is_ion_cannon {
                let roll = rng.next_range_u32_inclusive(1, ctx.bridge_strength as u32);
                if !((roll as u16) < ctx.damage) {
                    // Gate failed — try next path.
                    continue;
                }
            }

            // Retry: state-machine paths get up to 3 retries on IonCannon
            // (4 attempts total). Direct-overlay paths are single-shot
            // regardless of warhead.
            let max_attempts = if ctx.is_ion_cannon && path.is_state_machine() {
                4
            } else {
                1
            };
            for _attempt in 0..max_attempts {
                let outcome = match path {
                    DispatchPath::HighStateMachine => {
                        match bridge_state.cell(event.rx, event.ry).map(|c| c.role) {
                            Some(crate::sim::bridge_state::BridgeCellRole::Bridgehead) => {
                                bridge_state.bridgehead_advance_state(
                                    event.rx, event.ry, true, terrain,
                                )
                            }
                            _ => bridge_state.body_cell_advance_state(
                                event.rx, event.ry, true,
                            ),
                        }
                    }
                    DispatchPath::LowStateMachine => {
                        match bridge_state.cell(event.rx, event.ry).map(|c| c.role) {
                            Some(crate::sim::bridge_state::BridgeCellRole::Bridgehead) => {
                                bridge_state.bridgehead_advance_state(
                                    event.rx, event.ry, false, terrain,
                                )
                            }
                            _ => bridge_state.body_cell_advance_state(
                                event.rx, event.ry, false,
                            ),
                        }
                    }
                    DispatchPath::HighDirect => {
                        bridge_state.destroy_bridge_high(event.rx, event.ry, terrain)
                    }
                    DispatchPath::LowDirect => {
                        bridge_state.destroy_bridge_low(event.rx, event.ry, terrain)
                    }
                };
                if !matches!(outcome, StateOutcome::NoChange) {
                    outcomes.push(outcome);
                    break;
                }
            }
            // First matching path that did real work wins; stop scanning
            // remaining paths for this event.
            break;
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    // Cascade-consumer tests (ground kill, DropIn, debris, rim, zones)
    // land alongside Tasks 10-13. The dispatcher loop itself is
    // exercised end-to-end via the world_tests fixtures migrated in
    // Task 15 once the orchestrator is wired in (Task 14).
}
