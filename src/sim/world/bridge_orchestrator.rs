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

use std::collections::BTreeSet;

use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{
    BridgeCellRole, BridgeDamageContext, BridgeDamageEvent, DamageState, DispatchPath, StateOutcome,
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
    // rng &mut. Outcomes are collected for the cascade phase below.
    let outcomes: Vec<StateOutcome> = run_dispatch_loop(sim, events, bridge_strength);

    // Aggregate destroyed cells + the subset receiving BlowUpBridge from
    // the dispatcher's outcomes. BTreeSet keeps deterministic order for
    // the cascade walk.
    let mut destroyed_set: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut blow_up_cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    for outcome in &outcomes {
        if let StateOutcome::Collapsed {
            destroyed_cells,
            set_bridge_direction,
            ..
        } = outcome
        {
            destroyed_set.extend(destroyed_cells.iter().copied());
            for (cell, _slot, action) in &set_bridge_direction.actions {
                if matches!(
                    action,
                    crate::sim::bridge_specs::CellAction::BlowUpBridge
                ) {
                    blow_up_cells.insert(*cell);
                    destroyed_set.insert(*cell);
                }
            }
        }
    }

    // Cascade Step 1: ground-occupant kill. Per HIGH §11.4 step 1,
    // BlowUpBridge force-kills ground-layer entities at each destroyed
    // cell with C4Warhead semantics. Bridge-deck entities are handled by
    // Step 2 (DropIn) and never go through this kill path.
    //
    // Resolve C4Warhead's `InfDeath=` value once so the kill loop can
    // switch infantry to the matching death sequence (e.g., explosive
    // Die3 instead of default Die1). The inner block scopes the
    // immutable `sim.interner` borrow so the kill loop can hold `&mut sim`.
    let c4_inf_death: u8 = {
        let c4_id = rules.c4_warhead_id();
        let name = sim.interner.resolve(c4_id);
        rules.warhead(name).map(|wh| wh.inf_death).unwrap_or(1)
    };
    for &(rx, ry) in &blow_up_cells {
        kill_ground_occupants_at(sim, rx, ry, c4_inf_death);
    }

    // Cascade Step 2: bridge-deck DropIn. Per HIGH §12.7 / §12.9, deck
    // entities snap to ground level, clear OnBridge, and SURVIVE — even
    // when the destination cell is unwalkable (water below). Vanilla
    // never despawns or kills deck entities on collapse.
    for &(rx, ry) in &destroyed_set {
        drop_in_bridge_deck_entities(sim, rx, ry);
    }

    // Cascade Step 3: debris spawn (HIGH §11.4 step 4). Per-cell mix of
    // 50% MetallicDebris (no delay) + 1 always BridgeExplosion (delay
    // 1-5 frames). RNG draw order is parity-critical for lockstep.
    spawn_bridge_debris(sim, rules, &destroyed_set);

    // Aggregate rim cells + zones-dirty flag from the dispatcher's
    // outcomes so the trailing cascade hooks see them in one pass.
    let mut rim_cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut any_zones_dirty = false;
    for outcome in &outcomes {
        if let StateOutcome::Collapsed {
            adjacent_bridges_dirty,
            zones_dirty,
            ..
        } = outcome
        {
            rim_cells.extend(adjacent_bridges_dirty.iter().copied());
            any_zones_dirty |= *zones_dirty;
        }
    }

    // Cascade Step 4: rim refresh (HIGH §11.9). Stub today — see helper.
    update_adjacent_bridges(sim, &rim_cells);

    // Cascade Step 5: TriggerEvent 31 broadcast (HIGH §11.3). No-op on
    // skirmish; hook stub for future campaign / map-trigger support.
    notify_bridge_span_collapse(sim, &destroyed_set);

    // Cascade Step 6: zone graph rebuild (HIGH §12.8). Triggered when
    // any final-stage walker cell flagged the bridge endpoint records
    // dirty.
    refresh_bridge_zones_if_dirty(sim, any_zones_dirty);

    despawned_ids
}

/// Kill ground-layer entities at `(rx, ry)`. Mirrors the binary's
/// `BlowUpBridge` ground-occupant pass: walk every entity at the cell
/// that is NOT on the bridge layer and force-kill via C4Warhead semantics
/// (`damage = 0, force_kill = 1` in the binary; we set health = 0 and
/// flag `dying` for the next combat tick to handle death effects).
///
/// Bridge-deck entities go through `drop_in_bridge_deck_entities`
/// (Task 11) and survive — vanilla never drowns or kills them on
/// collapse (HIGH §12.7, §12.9).
///
/// `c4_inf_death` is the C4Warhead's `InfDeath=` byte; for entities with
/// an animation, the kill loop switches the death sequence to match (so
/// infantry play the C4-selected explosive death anim rather than the
/// default Die1). Mirrors the combat-side path in
/// `compute_dying_entities_combat_effects`.
fn kill_ground_occupants_at(
    sim: &mut Simulation,
    rx: u16,
    ry: u16,
    c4_inf_death: u8,
) {
    use crate::sim::animation::death_sequence_for_inf_death;
    let death_seq = death_sequence_for_inf_death(c4_inf_death);
    let victims: Vec<u64> = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            e.position.rx == rx
                && e.position.ry == ry
                && !e.is_on_bridge_layer()
                && e.health.current > 0
        })
        .map(|(id, _)| id)
        .collect();
    for id in victims {
        if let Some(entity) = sim.entities.get_mut(id) {
            entity.health.current = 0;
            entity.dying = true;
            entity.attack_target = None;
            entity.movement_target = None;
            entity.selected = false;
            if let Some(ref mut anim) = entity.animation {
                anim.switch_to(death_seq);
            }
        }
    }
}

/// Rim refresh. For each just-collapsed rim cell, walk along the bridge
/// in the direction of an adjacent bridge head (Bridgehead role or already
/// Destroyed) and reset orphaned stub cells whose anchor span has gone
/// away. A reset cell becomes:
///   - `overlay_byte = 0xFF` (sentinel: no overlay / -1)
///   - `damage_state = Healthy { variant: 0 }`
///   - `bridge_group_id = None`
///   - `deck_present = false`
///
/// Walk-length cap = 30 cells per RE doc §7.2 to bound the worst-case
/// linear-bridge length.
fn update_adjacent_bridges(sim: &mut Simulation, rim_cells: &BTreeSet<(u16, u16)>) {
    let Some(bridge_state) = sim.bridge_state.as_mut() else {
        return;
    };

    const WALK_LIMIT: usize = 30;
    const DIRECTIONS: [(i32, i32); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];

    for &(rx, ry) in rim_cells {
        // Phase A: find adjacent bridge-head candidate among 8 neighbors.
        let mut head_dir: Option<(i32, i32)> = None;
        for &(dx, dy) in &DIRECTIONS {
            let nx = rx as i32 + dx;
            let ny = ry as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let Some(neigh) = bridge_state.cell(nx as u16, ny as u16) else {
                continue;
            };
            let is_head_candidate = matches!(neigh.role, BridgeCellRole::Bridgehead)
                || matches!(neigh.damage_state, DamageState::Destroyed);
            if is_head_candidate {
                head_dir = Some((dx, dy));
                break;
            }
        }
        let Some((dx, dy)) = head_dir else { continue };

        // Phase B: walk along the bridge from (rx, ry) toward the head and
        // reset dangling stubs whose anchor span no longer exists.
        let mut walk_x = rx as i32;
        let mut walk_y = ry as i32;
        for _ in 0..WALK_LIMIT {
            walk_x += dx;
            walk_y += dy;
            if walk_x < 0 || walk_y < 0 {
                break;
            }
            let Some(cell) = bridge_state.cell(walk_x as u16, walk_y as u16) else {
                break;
            };
            if !cell.deck_present {
                break;
            }
            // Just-walker-destroyed cells render their own destroyed-bridge
            // overlay tile (0xE8 etc) — skip them so we don't blank that sprite.
            if matches!(cell.damage_state, DamageState::Destroyed) {
                continue;
            }
            let stub_now = cell
                .anchor_span_id
                .map(|sid| !bridge_state.anchor_spans().contains_key(&sid))
                .unwrap_or(false);
            if !stub_now {
                continue;
            }
            if let Some(c) = bridge_state.cell_mut(walk_x as u16, walk_y as u16) {
                c.overlay_byte = 0xFF;
                c.damage_state = DamageState::Healthy { variant: 0 };
                c.bridge_group_id = None;
                c.deck_present = false;
            }
        }
    }
}

/// TriggerEvent 31 broadcast. Mirror of binary
/// `MapClass::RepairBridgeSegment @ 0x00575EE0` (binary name is
/// misleading — the function actually fires `TriggerEvent 31` on bridge
/// span collapse; HIGH §11.3 + §12.6).
///
/// No-op on skirmish maps — RA2 skirmish has no triggers bound to
/// event 31. Wired as a hook so future campaign and map-trigger
/// support can drop in without changing the orchestrator's cascade
/// order.
fn notify_bridge_span_collapse(sim: &mut Simulation, cells: &BTreeSet<(u16, u16)>) {
    let _ = (sim, cells);
}

/// Zone graph refresh. Per HIGH §12.8: walker emits `zones_dirty=true`
/// only when a final-stage cell flips a `BridgeEndpointRecord.active`
/// flag, mirroring the binary's `InvalidateBridgeZones` →
/// `UpdateBridgeZonesHelper` chain. When set:
///   1. Recompute every endpoint record's `active` flag from current
///      cell damage state — first destroyed cell in a group flips its
///      endpoint pair to `active = false`. Replaces the side-effect of
///      the legacy single-shot `apply_damage`.
///   2. Rebuild the path grid from the post-collapse bridge state.
///   3. Rerun `Simulation::rebuild_zone_grid` so cross-bridge
///      passability reflects the new connectivity.
fn refresh_bridge_zones_if_dirty(sim: &mut Simulation, any_zones_dirty: bool) {
    if !any_zones_dirty {
        return;
    }
    if let Some(bs) = sim.bridge_state.as_mut() {
        bs.refresh_endpoint_active_flags();
    }
    let Some(terrain) = sim.resolved_terrain.as_ref() else {
        return;
    };
    let path_grid = crate::sim::pathfinding::PathGrid::from_resolved_terrain_with_bridges(
        terrain,
        sim.bridge_state.as_ref(),
    );
    sim.rebuild_zone_grid(&path_grid);
}

/// Per-cell debris spawn. Mirror of binary `BlowUpBridge` step 4
/// (HIGH §11.4). RNG draw order is parity-critical for lockstep — the
/// binary draws in this exact sequence per cell that passes the outer
/// gate:
/// 1. Outer 95% gate — `next_range_u32(20)` (skip cell on `== 0`).
/// 2. Two jitter draws — `next_range_u32(0xFFFF)` × 2. The values are
///    discarded; the binary uses them for in-cell pixel offsets that we
///    don't yet model, but the draws MUST be consumed for RNG-order
///    parity.
/// 3. MetallicDebris 50% gate — `next_range_u32(2)`.
/// 4. Optional MetallicDebris slot — `next_range_u32(metallic_count)`.
///    Only drawn when (50% gate passed) AND (`voxel_max > 0`) AND
///    (`metallic_count > 0`). When any of those are false, no slot
///    draw happens — the binary short-circuits.
/// 5. BridgeExplosion delay — `next_range_u32_inclusive(1, 5)`.
/// 6. BridgeExplosion slot — `next_range_u32(explosion_count)`.
///
/// Replaces the wrong-shape legacy `Simulation::spawn_bridge_explosions`,
/// which drew 1 immediate BridgeExplosion + a 50% delayed BridgeExplosion
/// — visible every collapse.
fn spawn_bridge_debris(
    sim: &mut Simulation,
    rules: &RuleSet,
    cells: &BTreeSet<(u16, u16)>,
) {
    use crate::sim::components::WorldEffect;

    let explosion_count = sim.bridge_explosions.len() as u32;
    let metallic_count = sim.metallic_debris.len() as u32;
    let voxel_max = rules.bridge_rules.voxel_max as u32;

    if explosion_count == 0 && metallic_count == 0 {
        return;
    }

    for &(rx, ry) in cells {
        // Step 1: outer 95% gate.
        if sim.rng.next_range_u32(20) == 0 {
            continue;
        }

        // Step 2: two jitter draws — values discarded but consumed for
        // RNG-order parity with the binary.
        let _jitter_x = sim.rng.next_range_u32(0xFFFF);
        let _jitter_y = sim.rng.next_range_u32(0xFFFF);

        let deck_level = sim
            .resolved_terrain
            .as_ref()
            .and_then(|t| t.cell(rx, ry))
            .map(|c| c.bridge_deck_level_if_any().unwrap_or(c.level))
            .unwrap_or(0);

        // Step 3: MetallicDebris 50% gate.
        let metallic_pass = sim.rng.next_range_u32(2) == 0;
        // Step 4: MetallicDebris slot pick + spawn (no delay). Slot draw
        // only happens when all three gates pass — short-circuit matches
        // the binary's call order.
        if metallic_pass && voxel_max > 0 && metallic_count > 0 {
            let idx = sim.rng.next_range_u32(metallic_count) as usize;
            let anim_id = sim.metallic_debris[idx];
            let frames = sim
                .effect_frame_counts
                .get(&anim_id)
                .copied()
                .unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                shp_name: anim_id,
                rx,
                ry,
                z: deck_level,
                frame: 0,
                total_frames: frames,
                rate_ms: 67,
                elapsed_ms: 0,
                translucent: true,
                delay_ms: 0,
            });
        }

        // Step 5 + 6: always BridgeExplosion, delayed 1-5 frames.
        if explosion_count > 0 {
            let delay_frames = sim.rng.next_range_u32_inclusive(1, 5);
            let idx = sim.rng.next_range_u32(explosion_count) as usize;
            let anim_id = sim.bridge_explosions[idx];
            let frames = sim
                .effect_frame_counts
                .get(&anim_id)
                .copied()
                .unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                shp_name: anim_id,
                rx,
                ry,
                z: deck_level,
                frame: 0,
                total_frames: frames,
                rate_ms: 67,
                elapsed_ms: 0,
                translucent: true,
                delay_ms: delay_frames * 67,
            });
        }
    }
}

/// Snap bridge-deck entities at `(rx, ry)` to ground level. Mirror of
/// the binary's `BlowUpBridge` step 2 (HIGH §11.4 + §12.7): walks the
/// deck entity list and calls `DropIn` on each.
///
/// Per HIGH §12.7 / §12.9: NO damage, NO despawn — units survive
/// stranded even when the destination is unwalkable (water below).
/// Vanilla has no drown mechanism. This is the parity correction
/// against the legacy `resolve_bridge_state_changes`, which despawned
/// deck entities over unwalkable ground.
fn drop_in_bridge_deck_entities(sim: &mut Simulation, rx: u16, ry: u16) {
    use crate::sim::movement::locomotor::{GroundMovePhase, MovementLayer};

    let ground_level = sim
        .resolved_terrain
        .as_ref()
        .and_then(|t| t.cell(rx, ry))
        .map(|c| c.level)
        .unwrap_or(0);

    let to_snap: Vec<u64> = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| {
            e.position.rx == rx && e.position.ry == ry && e.is_on_bridge_layer()
        })
        .map(|(id, _)| id)
        .collect();

    for id in to_snap {
        if let Some(entity) = sim.entities.get_mut(id) {
            entity.bridge_occupancy = None;
            entity.on_bridge = false;
            entity.position.z = ground_level;
            entity.position.refresh_screen_coords();
            entity.movement_target = None;
            if let Some(ref mut loco) = entity.locomotor {
                loco.layer = MovementLayer::Ground;
                loco.phase = GroundMovePhase::Idle;
            }
        }
    }
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
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::{BridgeOccupancy, Health, Position};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::test_intern;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::{SIM_ZERO, SimFixed};

    /// Build a single-cell terrain grid where (5,5) is a bridge deck at
    /// `deck_level`, ground level=0, water below (`is_water=true`,
    /// `ground_walk_blocked=true`). Used to verify DropIn lets deck units
    /// survive even with no walkable ground.
    fn water_below_bridge_terrain(deck_level: u8) -> ResolvedTerrainGrid {
        let mut cells = Vec::new();
        for y in 0..=5u16 {
            for x in 0..=5u16 {
                let is_bridge = x == 5 && y == 5;
                cells.push(ResolvedTerrainCell {
                    rx: x,
                    ry: y,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    slope_type: 0,
                    template_height: 0,
                    render_offset_x: 0,
                    render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs: SpeedCostProfile::default(),
                    is_water: is_bridge,
                    is_cliff_like: false,
                    is_cliff_redraw: false,
                    variant: 0,
                    is_rough: false,
                    is_road: false,
                    accepts_smudge: false,
                    has_ramp: false,
                    canonical_ramp: None,
                    ground_walk_blocked: is_bridge,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    build_blocked: is_bridge,
                    has_bridge_deck: is_bridge,
                    bridge_walkable: is_bridge,
                    bridge_transition: is_bridge,
                    bridge_deck_level: if is_bridge { deck_level } else { 0 },
                    bridge_layer: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                });
            }
        }
        ResolvedTerrainGrid::from_cells(6, 6, cells)
    }

    /// Build a Drive locomotor on the Bridge layer (mimics `high=true` spawn).
    fn drive_loco_on_bridge() -> LocomotorState {
        LocomotorState {
            kind: LocomotorKind::Drive,
            layer: MovementLayer::Bridge,
            phase: GroundMovePhase::Cruising,
            air_phase: AirMovePhase::Landed,
            speed_multiplier: SimFixed::from_num(1),
            speed_fraction: SimFixed::from_num(1),
            fly_current_speed: SIM_ZERO,
            altitude: SIM_ZERO,
            target_altitude: SIM_ZERO,
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_wobbles: 0.0,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 4,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            rot: 0,
            override_state: None,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
        }
    }

    /// Insert a vehicle on the bridge deck at (5,5) with deck_level=3.
    fn spawn_deck_unit(sim: &mut Simulation) -> u64 {
        let mut entity = GameEntity::new(
            1,
            5,
            5,
            3,
            64,
            test_intern("Americans"),
            Health {
                current: 256,
                max: 256,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.on_bridge = true;
        entity.bridge_occupancy = Some(BridgeOccupancy { deck_level: 3 });
        entity.locomotor = Some(drive_loco_on_bridge());
        // Give it a short fake movement target so we can verify it gets
        // halted on collapse.
        entity.movement_target = Some(crate::sim::components::MovementTarget::default());
        sim.entities.insert(entity);
        1
    }

    /// Task 11 — DropIn correction: bridge-deck entities snap to ground
    /// level + survive even when the destination is unwalkable (water
    /// below). The legacy `resolve_bridge_state_changes` despawned in
    /// this case; vanilla never does (HIGH §12.7 / §12.9).
    #[test]
    fn drop_in_snaps_deck_entity_to_ground_over_water_no_despawn() {
        let mut sim = Simulation::new();
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        let id = spawn_deck_unit(&mut sim);

        drop_in_bridge_deck_entities(&mut sim, 5, 5);

        let e = sim
            .entities
            .get(id)
            .expect("deck entity must SURVIVE collapse over water");
        assert_eq!(e.position.z, 0, "snapped to ground level");
        assert!(!e.on_bridge, "OnBridge cleared by DropIn");
        assert!(e.bridge_occupancy.is_none(), "bridge_occupancy cleared");
        assert!(e.movement_target.is_none(), "movement halted on collapse");
        assert_eq!(e.health.current, 256, "DropIn never harms — no damage");
        let loco = e.locomotor.as_ref().expect("locomotor");
        assert_eq!(
            loco.layer,
            MovementLayer::Ground,
            "layer flipped Bridge → Ground"
        );
        assert_eq!(loco.phase, GroundMovePhase::Idle, "phase reset to Idle");
    }

    /// Build a minimal RuleSet whose `bridge_rules.voxel_max` matches the
    /// argument. Used by Task 12 debris tests to toggle the voxel-max gate.
    fn rules_with_voxel_max(voxel_max: u32) -> crate::rules::ruleset::RuleSet {
        let body = format!(
            "[InfantryTypes]\n\
             [VehicleTypes]\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [General]\n\
             BridgeVoxelMax={}\n",
            voxel_max
        );
        let ini = crate::rules::ini_parser::IniFile::from_str(&body);
        crate::rules::ruleset::RuleSet::from_ini(&ini).expect("rules parse")
    }

    /// Task 12 — RNG draw-order parity: per cell, `spawn_bridge_debris`
    /// MUST consume RNG draws in the exact binary order:
    /// outer-95% → jitter×2 → metallic-50% → optional metallic-slot →
    /// explosion-delay → explosion-slot. Wrong order desyncs lockstep.
    #[test]
    fn debris_consumes_correct_rng_count_per_cell() {
        let mut sim = Simulation::new();
        let seed = 0xDEAD_BEEF_u64;
        sim.rng = crate::sim::rng::SimRng::new(seed);
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        sim.bridge_explosions
            .extend([test_intern("BRIDGEEXP1"), test_intern("BRIDGEEXP2")]);
        sim.metallic_debris.push(test_intern("METALDEB1"));
        let rules = rules_with_voxel_max(3);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));

        // Predict the exact draw sequence on a parallel RNG. The helper
        // MUST match this sequence step-for-step to maintain lockstep.
        let mut predicted = crate::sim::rng::SimRng::new(seed);
        let outer = predicted.next_range_u32(20);
        if outer != 0 {
            let _jx = predicted.next_range_u32(0xFFFF);
            let _jy = predicted.next_range_u32(0xFFFF);
            let metallic_pass = predicted.next_range_u32(2) == 0;
            // Metallic slot draw is gated on (pass) AND (voxel_max > 0)
            // AND (metallic_count > 0). With our setup all three hold
            // when `metallic_pass` is true.
            if metallic_pass {
                let _slot = predicted.next_range_u32(1);
            }
            let _delay = predicted.next_range_u32_inclusive(1, 5);
            let _exp_slot = predicted.next_range_u32(2);
        }

        spawn_bridge_debris(&mut sim, &rules, &cells);

        assert_eq!(
            sim.rng.state(),
            predicted.state(),
            "RNG draw order/count diverged from binary parity sequence"
        );
    }

    /// Task 12 — voxel_max=0 short-circuits the MetallicDebris slot draw,
    /// even if the 50% gate passes. Per HIGH §11.4, the binary skips the
    /// slot pick when `BridgeVoxelMax==0`. The 50% gate ITSELF still
    /// fires (so the draw count differs from a no-pass case).
    #[test]
    fn debris_skipped_when_voxel_max_zero() {
        let mut sim = Simulation::new();
        let seed = 0xDEAD_BEEF_u64;
        sim.rng = crate::sim::rng::SimRng::new(seed);
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        sim.bridge_explosions.push(test_intern("BRIDGEEXP1"));
        sim.metallic_debris.push(test_intern("METALDEB1"));
        let rules = rules_with_voxel_max(0);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));
        spawn_bridge_debris(&mut sim, &rules, &cells);

        // No MetallicDebris effect must have spawned, regardless of which
        // way the 50% gate fell — voxel_max=0 short-circuits the slot.
        let metallic_id = test_intern("METALDEB1");
        assert!(
            !sim.world_effects
                .iter()
                .any(|fx| fx.shp_name == metallic_id),
            "voxel_max=0 must suppress all MetallicDebris spawns"
        );
    }

    /// Task 12 — debris helper short-circuits when both lists are empty.
    /// No RNG should be consumed (no outer gate, no jitter, no slot).
    #[test]
    fn debris_no_op_when_no_lists() {
        let mut sim = Simulation::new();
        sim.rng = crate::sim::rng::SimRng::new(7);
        let baseline_state = sim.rng.state();
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        let rules = rules_with_voxel_max(3);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));
        cells.insert((4, 5));
        spawn_bridge_debris(&mut sim, &rules, &cells);

        assert_eq!(
            sim.rng.state(),
            baseline_state,
            "no RNG draws when both debris lists are empty"
        );
        assert!(sim.world_effects.is_empty());
    }

    /// Task 11 — DropIn must NOT touch entities that aren't on the bridge
    /// Rim refresh resets dangling stub cells whose anchor span has gone
    /// away. Layout: anchor span 1 owns (4,2)+(5,2)+(6,2). After collapse,
    /// drop the span entry from the registry, mark (5,2) Destroyed (the
    /// "head" candidate), and call `update_adjacent_bridges` with rim cell
    /// (4,2). Expected: (4,2)→(5,2) walks east, (5,2) is the head so the
    /// loop continues past it; once it sees an orphan-anchor cell, the
    /// reset fires.
    #[test]
    fn rim_refresh_clears_dangling_stub_cells() {
        use crate::sim::bridge_state::{BridgeRuntimeCell, BridgeRuntimeState};
        let mut sim = Simulation::new();
        let mut bs = BridgeRuntimeState::default();
        // (5,2): destroyed head (acts as direction beacon).
        bs.test_seed_cell(
            5,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 4,
                bridge_group_id: Some(1),
                damage_state: DamageState::Destroyed,
                axis: Some(crate::sim::bridge_state::Axis::EW),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(99),
                overlay_byte: 0xE8,
                damaged_variant: false,
            },
        );
        // (6,2): dangling stub — anchor_span_id=99 but no AnchorSpan entry.
        bs.test_seed_cell(
            6,
            2,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 4,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(crate::sim::bridge_state::Axis::EW),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(99),
                overlay_byte: 0xDC,
                damaged_variant: false,
            },
        );
        sim.bridge_state = Some(bs);

        let mut rim: BTreeSet<(u16, u16)> = BTreeSet::new();
        rim.insert((4, 2));
        update_adjacent_bridges(&mut sim, &rim);

        let stub = sim.bridge_state.as_ref().unwrap().cell(6, 2).unwrap();
        assert_eq!(stub.overlay_byte, 0xFF, "stub overlay reset to NONE");
        assert!(matches!(
            stub.damage_state,
            DamageState::Healthy { variant: 0 }
        ));
        assert!(stub.bridge_group_id.is_none());
        assert!(!stub.deck_present);
    }

    /// layer at the destroyed cell. Ground-layer entities are handled by
    /// `kill_ground_occupants_at` (Step 1), not DropIn.
    #[test]
    fn drop_in_ignores_ground_layer_entities_at_destroyed_cell() {
        let mut sim = Simulation::new();
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        let mut entity = GameEntity::new(
            1,
            5,
            5,
            0,
            64,
            test_intern("Americans"),
            Health {
                current: 256,
                max: 256,
            },
            test_intern("MTNK"),
            crate::map::entities::EntityCategory::Unit,
            0,
            5,
            true,
        );
        entity.on_bridge = false; // ground-layer occupant
        let mut loco = drive_loco_on_bridge();
        loco.layer = MovementLayer::Ground;
        entity.locomotor = Some(loco);
        sim.entities.insert(entity);

        drop_in_bridge_deck_entities(&mut sim, 5, 5);

        // Ground entity untouched — still alive, still ground layer.
        let e = sim.entities.get(1).expect("ground entity untouched");
        assert_eq!(e.health.current, 256);
        assert!(!e.on_bridge);
        assert_eq!(
            e.locomotor.as_ref().unwrap().layer,
            MovementLayer::Ground
        );
    }
}
