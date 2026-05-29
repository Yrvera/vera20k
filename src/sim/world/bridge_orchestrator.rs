//! Bridge damage orchestrator — 4-path dispatcher + cascade consumers.
//!
//! Per-tick entry that drains `BridgeDamageEvent`s emitted by combat, runs
//! each event through the 4-path dispatcher (HighSM → LowSM → LowDirect →
//! HighDirect, in fixed order), applies the per-path BridgeStrength RNG
//! gate, runs the IonCannon retry loop on state-machine paths only, then
//! applies the BlowUpBridge cascade: ground-occupant kill, bridge-deck
//! DropIn, debris spawn, rim refresh, trigger broadcast, zone rebuild.
//! `notify_bridge_span_collapse` is an intentional no-op on skirmish
//! (TriggerEvent 31 is bound only by campaign / map triggers).
//!
//! ## Dependency rules
//! Same as sim/world: depends on sim/bridge_state, sim/rng, rules/, map/;
//! never render / ui / audio / net.

use std::collections::{BTreeMap, BTreeSet};

use crate::map::bridge_facts::{
    BRIDGE_FLAG_ANCHOR_SELF, BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_DIRECTION_ZERO,
    BRIDGE_FLAG_STRUCTURAL,
};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::{
    Axis, BridgeCellRole, BridgeDamageContext, BridgeDamageEvent, BridgeRuntimeState, DamageState,
    DispatchPath, StateOutcome,
};
use crate::sim::world::Simulation;
use crate::sim::{intern::InternedId, rng::SimRng};
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::CELL_CENTER_LEPTON;

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
/// Returns `true` if any event in the batch produced a `StateOutcome::Collapsed`
/// — i.e. at least one bridge cell transitioned to `DamageState::Destroyed`.
/// Callers use this to signal `TickResult.bridge_state_changed` so the app
/// rebuilds the PathGrid before next tick's movement runs.
///
/// Cascade side-effects (kill / DropIn / debris / rim / zone) run unconditionally
/// when matching outcomes are present in this batch — they don't depend on
/// the return value.
pub(crate) fn apply_bridge_damage_events(
    sim: &mut Simulation,
    rules: &RuleSet,
    events: &[BridgeDamageEvent],
) -> bool {
    if events.is_empty() {
        return false;
    }

    // Outer gate + read bridge_strength up front (immutable borrow scope).
    let bridge_strength = match sim.bridge_state.as_ref() {
        Some(bs) if bs.is_destroyable() => bs.bridge_strength(),
        _ => return false,
    };

    // Run dispatch loop with split borrows: bridge_state &mut, terrain &,
    // rng &mut. Outcomes are collected for the cascade phase below.
    let outcomes: Vec<StateOutcome> = run_dispatch_loop(sim, events, bridge_strength);

    // Aggregate destroyed cells + the subset receiving BlowUpBridge from
    // the dispatcher's outcomes. BTreeSet keeps deterministic order for
    // the cascade walk.
    let mut destroyed_set: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut blow_up_cells: Vec<(u16, u16)> = Vec::new();
    for outcome in &outcomes {
        if let StateOutcome::Collapsed {
            destroyed_cells,
            set_bridge_direction,
            ..
        } = outcome
        {
            destroyed_set.extend(destroyed_cells.iter().copied());
            for (cell, _slot, action) in &set_bridge_direction.actions {
                if matches!(action, crate::sim::bridge_specs::CellAction::BlowUpBridge) {
                    blow_up_cells.push(*cell);
                    destroyed_set.insert(*cell);
                }
            }
        }
    }

    // BlowUpBridge fallout is per write-cell: ground occupants die with
    // C4Warhead semantics, bridge-deck occupants DropIn, then that cell emits
    // debris. Keeping the effects inside this helper preserves the binary's
    // per-cell fallout order instead of batching kills, drops, and debris.
    let c4_inf_death = c4_inf_death(rules, sim);
    for &(rx, ry) in &blow_up_cells {
        blow_up_bridge_cell_fallout(sim, rules, rx, ry, c4_inf_death);
    }

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

    // state_changed = "at least one cell collapsed this batch". The destroyed_set
    // is built from StateOutcome::Collapsed outcomes earlier in this function;
    // if it's non-empty, real work happened.
    !destroyed_set.is_empty()
}

/// Bridge-collapse dispatch from a `BridgeRepairHut` death event (C4 timer
/// expired, demo-truck explosion). Chooses low/high from hut-local evidence,
/// finds an overlay entry directly or through a bounded bridge/ramp fallback,
/// then runs the direct-overlay collapse sweep and the same BlowUpBridge
/// cascade as `apply_bridge_damage_events`.
///
/// Returns `true` if any bridge cell transitioned (caller ORs into
/// `bridge_state_changed` so the app rebuilds the PathGrid).
///
/// Caller ensures the hut itself is not damaged — the hut survives the
/// collapse, mirroring the original game's `BridgeRepairHut` death branch.
pub(crate) fn dispatch_bridge_collapse_from_hut(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool {
    let scan: Vec<(u16, u16)> = hut_destroy_5x5_scan(hut_center).collect();
    let family = choose_hut_bridge_family(sim, &scan);

    let fallback_plan = build_hut_fallback_plan(sim, hut_center);

    // gamemd's hut-death dispatch runs a BOUNDED 4-step walker, NOT a
    // full-span collapse. Per `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`
    // §4: `CollapseBridge_*_*` measures both axial extents from the seed,
    // shifts the start cell toward the shorter side, then walks at most 4
    // axial cells in the longer direction calling `DestroyBridge_*` per
    // cell. Each per-cell call writes a 3-cell axial overlay range plus
    // perpendicular ApplyBridgeDestruction (3×3 grid). Net cell coverage
    // ~18 cells for a 3-wide bridge (3 perp × 6 axial after overlap).
    let mut outcomes: Vec<StateOutcome> = Vec::new();
    let mut fallback_zones_dirty = false;
    let mut fallback_adjacent_dirty_anchor = None;
    {
        let Some(terrain) = sim.resolved_terrain.as_ref() else {
            return false;
        };
        let Some(bs) = sim.bridge_state.as_mut() else {
            return false;
        };
        let mut presentation = BridgePresentationContext {
            // bridge collapse/repair — scenario stream. Direct field (NOT bridge_rng()):
            // sits inside a live sim.bridge_state borrow + co-borrows world_effects etc.
            rng: &mut sim.scenario_rng,
            world_effects: &mut sim.world_effects,
            bridge_explosions: &sim.bridge_explosions,
            effect_frame_counts: &sim.effect_frame_counts,
            bridge_anim_sounds: &sim.bridge_anim_sounds,
        };

        // Look for a seed cell whose overlay is already in the destroy-band.
        // The no-overlay fallback below is a separate flag/ramp walk, not
        // another traced overlay search.
        let seed_axis = find_destroy_overlay_seed(bs, &scan, family);

        if let Some((seed_rx, seed_ry, axis)) = seed_axis {
            outcomes.extend(run_hut_collapse_bounded(
                bs,
                terrain,
                &mut presentation,
                family,
                axis,
                seed_rx,
                seed_ry,
            ));
        } else {
            let fallback = run_hut_fallback_plan(bs, terrain, fallback_plan);
            outcomes.extend(fallback.outcomes);
            fallback_zones_dirty = fallback.zones_dirty;
            fallback_adjacent_dirty_anchor = fallback.adjacent_dirty_anchor;
        }
    }

    apply_hut_bridge_execution(
        sim,
        rules,
        &outcomes,
        fallback_zones_dirty,
        fallback_adjacent_dirty_anchor,
    )
}

/// gamemd's `BridgeRepairHut` death path scans the 5x5 hut-local window
/// X-major: `x = -2..=2`, then `y = -2..=2`. The engineer repair path uses
/// the shared Y-major helper, so hut collapse keeps its own ordering here.
fn hut_destroy_5x5_scan(center: (u16, u16)) -> impl Iterator<Item = (u16, u16)> {
    let (cx, cy) = (center.0 as i32, center.1 as i32);
    (-2..=2i32).flat_map(move |dx| {
        (-2..=2i32).filter_map(move |dy| {
            let nx = cx + dx;
            let ny = cy + dy;
            if nx < 0 || ny < 0 || nx > u16::MAX as i32 || ny > u16::MAX as i32 {
                None
            } else {
                Some((nx as u16, ny as u16))
            }
        })
    })
}

/// Find the first cell in `scan` whose overlay maps to a physical collapse
/// sweep axis for this bridge family.
///
/// This is intentionally separate from `BridgeRuntimeState::*_destroy_overlay_axis`:
/// those helpers classify the per-cell walker/write family, while the hut
/// `CollapseBridge_*` entry walks along the bridge's physical span. The binary
/// dispatcher proves these are opposite for the bridge overlay subranges:
/// `0xCD`/`0x4A` families route to `CollapseBridge_EW_*`, which steps in X.
fn find_destroy_overlay_seed(
    bridge_state: &BridgeRuntimeState,
    scan: &[(u16, u16)],
    family: HutBridgeFamily,
) -> Option<(u16, u16, Axis)> {
    scan.iter().copied().find_map(|(rx, ry)| {
        let overlay = bridge_state.cell(rx, ry).map(|c| c.overlay_byte)?;
        let axis = physical_span_axis_for_destroy_overlay(family, overlay)?;
        let seed = canonicalize_hut_destroy_seed(bridge_state, family, (rx, ry), axis)?;
        Some((seed.0, seed.1, axis))
    })
}

fn canonicalize_hut_destroy_seed(
    bridge_state: &BridgeRuntimeState,
    family: HutBridgeFamily,
    matched: (u16, u16),
    physical_axis: Axis,
) -> Option<(u16, u16)> {
    // `DestroyBridgeFromCell_*` recenters the first hut-scan hit onto the
    // bridge lane before calling the bounded walker. The probes are
    // perpendicular to the physical span: EW walkers probe Y, NS walkers probe X.
    let probe_axis = match physical_axis {
        Axis::EW => Axis::NS,
        Axis::NS => Axis::EW,
    };
    let back_one = step_axis(matched, probe_axis, -1);
    let back_two = step_axis(matched, probe_axis, -2);
    let back_one_in_band = back_one
        .and_then(|(rx, ry)| bridge_state.cell(rx, ry).map(|c| c.overlay_byte))
        .is_some_and(|overlay| in_bridge_band(family, overlay));
    if !back_one_in_band {
        return step_axis(matched, probe_axis, 1);
    }
    let back_two_in_band = back_two
        .and_then(|(rx, ry)| bridge_state.cell(rx, ry).map(|c| c.overlay_byte))
        .is_some_and(|overlay| in_bridge_band(family, overlay));
    if back_two_in_band {
        back_one
    } else {
        Some(matched)
    }
}

fn apply_hut_bridge_execution(
    sim: &mut Simulation,
    rules: &RuleSet,
    outcomes: &[StateOutcome],
    extra_zones_dirty: bool,
    extra_adjacent_dirty_anchor: Option<(u16, u16)>,
) -> bool {
    if outcomes.is_empty() && !extra_zones_dirty && extra_adjacent_dirty_anchor.is_none() {
        return false;
    }

    let mut destroyed_set: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut blow_up_cells: Vec<(u16, u16)> = Vec::new();
    let mut rim_cells: BTreeSet<(u16, u16)> = BTreeSet::new();
    let mut any_zones_dirty = false;
    for outcome in outcomes {
        if let StateOutcome::Collapsed {
            destroyed_cells,
            set_bridge_direction,
            adjacent_bridges_dirty,
            zones_dirty,
            ..
        } = outcome
        {
            destroyed_set.extend(destroyed_cells.iter().copied());
            for (cell, _slot, action) in &set_bridge_direction.actions {
                if matches!(action, crate::sim::bridge_specs::CellAction::BlowUpBridge) {
                    blow_up_cells.push(*cell);
                    destroyed_set.insert(*cell);
                }
            }
            rim_cells.extend(adjacent_bridges_dirty.iter().copied());
            any_zones_dirty |= *zones_dirty;
        }
    }
    if let Some(anchor) = extra_adjacent_dirty_anchor {
        rim_cells.insert(anchor);
    }
    any_zones_dirty |= extra_zones_dirty;

    let c4_inf_death = c4_inf_death(rules, sim);
    for &(rx, ry) in &blow_up_cells {
        blow_up_bridge_cell_fallout(sim, rules, rx, ry, c4_inf_death);
    }
    update_adjacent_bridges(sim, &rim_cells);
    notify_bridge_span_collapse(sim, &destroyed_set);
    refresh_bridge_zones_if_dirty(sim, any_zones_dirty);

    !destroyed_set.is_empty() || extra_zones_dirty || extra_adjacent_dirty_anchor.is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HutBridgeFamily {
    Low,
    High,
}

struct BridgePresentationContext<'a> {
    rng: &'a mut SimRng,
    world_effects: &'a mut Vec<crate::sim::components::WorldEffect>,
    bridge_explosions: &'a [InternedId],
    effect_frame_counts: &'a BTreeMap<InternedId, u16>,
    bridge_anim_sounds: &'a BTreeMap<InternedId, InternedId>,
}

const HUT_FALLBACK_DIRS: [(i16, i16); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
const HUT_FALLBACK_STARTER_MASK: u32 = BRIDGE_FLAG_STRUCTURAL | BRIDGE_FLAG_DESTROYED_OR_RAMP;
// Hard cap of the bounded walker: gamemd's `CollapseBridge_*_*` uses
// `local_2c = 4`. See `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` §4.
const MAX_HUT_SWEEP_STEPS: usize = 4;
const MAX_HUT_ATTEMPTS_PER_STEP: usize = 3;
const NORMALIZED_RNG_MAX_INCLUSIVE: u32 = 0x7FFF_FFFE;
const NORMALIZED_RNG_DENOMINATOR: u64 = 0x8000_0000;
const BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE: u32 = 2_040_109_466;
const BRIDGE_METALLIC_GATE_EXCLUSIVE: u32 = 0x4000_0000;
const BRIDGE_JITTER_SPAN_LEPTONS: u64 = 50;
const BRIDGE_JITTER_HALF_LEPTONS: i32 = 25;
const BRIDGE_EFFECT_FRAME_MS: u32 = 67;
// Safety cap on the extent-measurement walk (Phase 1 of the bounded
// walker). gamemd has no explicit cap — the off-bridge band check
// terminates the walk — but a runaway count would only happen if the
// overlay band check were buggy. 64 cells is well beyond any realistic
// YR bridge length.
const MAX_EXTENT_PROBE: usize = 64;
const MAX_HUT_ENDPOINT_PROBE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HutFallbackStarter {
    pos: (u16, u16),
    flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HutFallbackPlan {
    NoAcceptedStarter,
    MissingAnchor,
    PureBridgeheadTooLong,
    RampWalk {
        starter: HutFallbackStarter,
        anchor: (u16, u16),
    },
}

#[derive(Debug, Default)]
struct HutFallbackExecution {
    outcomes: Vec<StateOutcome>,
    zones_dirty: bool,
    adjacent_dirty_anchor: Option<(u16, u16)>,
}

fn choose_hut_bridge_family(sim: &Simulation, scan: &[(u16, u16)]) -> HutBridgeFamily {
    if scan
        .iter()
        .any(|&(rx, ry)| is_low_hut_scan_evidence(sim, rx, ry))
    {
        HutBridgeFamily::Low
    } else {
        HutBridgeFamily::High
    }
}

fn is_low_hut_scan_evidence(sim: &Simulation, rx: u16, ry: u16) -> bool {
    bridge_overlay_at(sim, rx, ry).is_some_and(BridgeRuntimeState::is_low_destroy_overlay)
        || sim
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(rx, ry))
            .is_some_and(|cell| cell.is_wood_bridge_repair_tile)
}

fn bridge_overlay_at(sim: &Simulation, rx: u16, ry: u16) -> Option<u8> {
    sim.bridge_state
        .as_ref()
        .and_then(|bs| bs.cell(rx, ry))
        .map(|cell| cell.overlay_byte)
        .or_else(|| {
            sim.resolved_terrain
                .as_ref()
                .and_then(|terrain| terrain.cell(rx, ry))
                .and_then(|cell| cell.bridge_layer.as_ref())
                .map(|layer| layer.overlay_id)
        })
}

fn build_hut_fallback_plan(sim: &Simulation, hut_center: (u16, u16)) -> HutFallbackPlan {
    let Some(starter) = find_hut_fallback_starter(sim, hut_center) else {
        return HutFallbackPlan::NoAcceptedStarter;
    };
    resolve_hut_fallback_anchor(sim, starter)
}

fn find_hut_fallback_starter(
    sim: &Simulation,
    hut_center: (u16, u16),
) -> Option<HutFallbackStarter> {
    let hut_flags = hut_fallback_flags(sim, hut_center);
    if hut_flags & HUT_FALLBACK_STARTER_MASK != 0 {
        return Some(HutFallbackStarter {
            pos: hut_center,
            flags: hut_flags,
        });
    }

    for &(dx, dy) in &HUT_FALLBACK_DIRS {
        for distance in 1..=3i16 {
            let rx = hut_center.0 as i32 + dx as i32 * distance as i32;
            let ry = hut_center.1 as i32 + dy as i32 * distance as i32;
            if rx < 0 || ry < 0 || rx > u16::MAX as i32 || ry > u16::MAX as i32 {
                continue;
            }
            let pos = (rx as u16, ry as u16);
            let flags = hut_fallback_flags(sim, pos);
            if flags & HUT_FALLBACK_STARTER_MASK != 0 {
                return Some(HutFallbackStarter { pos, flags });
            }
        }
    }

    None
}

fn resolve_hut_fallback_anchor(sim: &Simulation, starter: HutFallbackStarter) -> HutFallbackPlan {
    if starter.flags & BRIDGE_FLAG_STRUCTURAL != 0 {
        if starter.flags & BRIDGE_FLAG_ANCHOR_SELF != 0 {
            return HutFallbackPlan::RampWalk {
                starter,
                anchor: starter.pos,
            };
        }
        let anchor = sim
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(starter.pos.0, starter.pos.1))
            .and_then(|cell| cell.bridge_facts.anchor)
            .map(|relation| relation.anchor)
            .or_else(|| {
                sim.bridge_state
                    .as_ref()
                    .and_then(|bs| bs.cell(starter.pos.0, starter.pos.1))
                    .and_then(|cell| cell.anchor_span_id)
                    .and_then(|span_id| sim.bridge_state.as_ref()?.anchor_span(span_id))
                    .map(|span| span.anchor)
            });
        return anchor.map_or(HutFallbackPlan::MissingAnchor, |anchor| {
            HutFallbackPlan::RampWalk { starter, anchor }
        });
    }

    if starter.flags & BRIDGE_FLAG_DESTROYED_OR_RAMP != 0 {
        return resolve_pure_bridgehead_anchor(sim, starter);
    }

    HutFallbackPlan::NoAcceptedStarter
}

fn resolve_pure_bridgehead_anchor(
    sim: &Simulation,
    starter: HutFallbackStarter,
) -> HutFallbackPlan {
    let (scan_dir, opposite_dir) = if starter.flags & BRIDGE_FLAG_DIRECTION_ZERO != 0 {
        (4usize, 0usize)
    } else {
        (2usize, 6usize)
    };
    let mut cur = starter.pos;
    let mut continuations = 0usize;
    loop {
        let Some(next) = step_hut_dir(cur, scan_dir) else {
            return HutFallbackPlan::MissingAnchor;
        };
        let flags = hut_fallback_flags(sim, next);
        if flags & BRIDGE_FLAG_DESTROYED_OR_RAMP == 0 {
            let Some(anchor) =
                step_hut_dir(next, opposite_dir).and_then(|p| step_hut_dir(p, opposite_dir))
            else {
                return HutFallbackPlan::MissingAnchor;
            };
            return HutFallbackPlan::RampWalk { starter, anchor };
        }
        continuations += 1;
        if continuations >= 4 {
            return HutFallbackPlan::PureBridgeheadTooLong;
        }
        cur = next;
    }
}

fn hut_fallback_flags(sim: &Simulation, pos: (u16, u16)) -> u32 {
    sim.resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(pos.0, pos.1))
        .map(|cell| cell.bridge_flags())
        .unwrap_or(0)
}

fn run_hut_fallback_plan(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &ResolvedTerrainGrid,
    plan: HutFallbackPlan,
) -> HutFallbackExecution {
    let HutFallbackPlan::RampWalk { starter, anchor } = plan else {
        return HutFallbackExecution::default();
    };

    let forward_dir = if starter.flags & BRIDGE_FLAG_DIRECTION_ZERO != 0 {
        6usize
    } else {
        0usize
    };
    let Some(ramp_cell) = find_hut_fallback_ramp_cell(terrain, anchor, forward_dir) else {
        return HutFallbackExecution {
            zones_dirty: true,
            ..Default::default()
        };
    };

    let mut execution = HutFallbackExecution::default();
    execution
        .outcomes
        .extend(apply_hut_damage_retries(bridge_state, terrain, ramp_cell));

    let reverse_dir = (forward_dir + 4) & 7;
    let Some(endpoint) = find_hut_fallback_endpoint_cell(terrain, ramp_cell, reverse_dir) else {
        execution.zones_dirty = true;
        execution.adjacent_dirty_anchor = Some(anchor);
        return execution;
    };

    if hut_endpoint_needs_beyond_damage(terrain, endpoint) {
        if let Some(target) = step_hut_dir(endpoint, forward_dir) {
            execution
                .outcomes
                .extend(apply_hut_damage_retries(bridge_state, terrain, target));
        }
    }
    execution.zones_dirty = true;
    execution.adjacent_dirty_anchor = Some(anchor);
    execution
}

fn find_hut_fallback_ramp_cell(
    terrain: &ResolvedTerrainGrid,
    anchor: (u16, u16),
    forward_dir: usize,
) -> Option<(u16, u16)> {
    let mut cur = anchor;
    for _ in 0..MAX_EXTENT_PROBE {
        let cell = terrain.cell(cur.0, cur.1)?;
        if cell.bridge_facts.ramp_tile.is_some() {
            return Some(cur);
        }
        cur = step_hut_dir(cur, forward_dir)?;
    }
    None
}

fn find_hut_fallback_endpoint_cell(
    terrain: &ResolvedTerrainGrid,
    ramp_cell: (u16, u16),
    reverse_dir: usize,
) -> Option<(u16, u16)> {
    let mut cur = ramp_cell;
    for _ in 0..MAX_HUT_ENDPOINT_PROBE {
        cur = step_hut_dir(cur, reverse_dir)?;
        let cell = terrain.cell(cur.0, cur.1)?;
        if cell.bridge_facts.ramp_tile.is_some() || cell.bridge_facts.anchor.is_some() {
            return Some(cur);
        }
    }
    None
}

fn hut_endpoint_needs_beyond_damage(terrain: &ResolvedTerrainGrid, endpoint: (u16, u16)) -> bool {
    terrain
        .cell(endpoint.0, endpoint.1)
        .and_then(|cell| cell.bridge_facts.ramp_tile)
        .is_none_or(|tile| tile.relative_tile_index != u16::MAX - 1)
}

fn apply_hut_damage_retries(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &ResolvedTerrainGrid,
    target: (u16, u16),
) -> Vec<StateOutcome> {
    if bridge_state.cell(target.0, target.1).is_none() {
        return Vec::new();
    }

    let mut outcomes = Vec::new();
    for _ in 0..MAX_HUT_ATTEMPTS_PER_STEP {
        let outcome = apply_hut_damage_to_cell(bridge_state, terrain, target.0, target.1);
        let success = outcome.apply_damage_success();
        if outcome.has_effect() {
            outcomes.push(outcome);
        }
        if success {
            break;
        }
    }
    outcomes
}

fn step_hut_dir(pos: (u16, u16), direction: usize) -> Option<(u16, u16)> {
    let (dx, dy) = HUT_FALLBACK_DIRS[direction & 7];
    let rx = pos.0 as i32 + dx as i32;
    let ry = pos.1 as i32 + dy as i32;
    if rx < 0 || ry < 0 || rx > u16::MAX as i32 || ry > u16::MAX as i32 {
        return None;
    }
    Some((rx as u16, ry as u16))
}

fn physical_span_axis_for_destroy_overlay(family: HutBridgeFamily, overlay: u8) -> Option<Axis> {
    let walker_axis = match family {
        HutBridgeFamily::Low => BridgeRuntimeState::low_destroy_overlay_axis(overlay),
        HutBridgeFamily::High => BridgeRuntimeState::high_destroy_overlay_axis(overlay),
    }?;
    match walker_axis {
        Axis::NS => Some(Axis::EW),
        Axis::EW => Some(Axis::NS),
    }
}

fn step_axis(pos: (u16, u16), axis: Axis, dir: i16) -> Option<(u16, u16)> {
    let (rx, ry) = pos;
    match axis {
        Axis::EW => {
            let next = rx as i32 + dir as i32;
            (0..=u16::MAX as i32)
                .contains(&next)
                .then_some((next as u16, ry))
        }
        Axis::NS => {
            let next = ry as i32 + dir as i32;
            (0..=u16::MAX as i32)
                .contains(&next)
                .then_some((rx, next as u16))
        }
    }
}

/// Bounded 4-iteration collapse walker — mirror of gamemd's
/// `MapClass::CollapseBridge_{NS,EW}_{High,Low}` at
/// `0x00575BA0` / `0x00575870` / `0x00575540` / `0x00575220`.
///
/// Per `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` §4:
///
/// 1. **Extent measurement.** Walk both axial directions from `seed`
///    counting cells still inside the bridge overlay band
///    (`[0xCD..=0xE8]` high / `[0x4A..=0x65]` low). Counts → `back` and
///    `fwd`.
/// 2. **Direction + start.** Step direction is `-1` if `fwd < back`,
///    else `+1` (walk toward the longer-extent side). Start cell is
///    `seed - (back - fwd) / 2` using signed integer division — biases
///    the starting position toward the shorter side so the 4-step walk
///    can cover the maximum bridge length.
/// 3. **4-iteration walker.** For each of `MAX_HUT_SWEEP_STEPS` (= 4)
///    axial steps, call the per-cell primitive `destroy_bridge_high/low`
///    up to `MAX_HUT_ATTEMPTS_PER_STEP` (= 3) retries. Each primitive
///    call writes a 3-cell axial overlay range and triggers
///    `ApplyBridgeDestruction_*` on the X±1 perpendicular columns,
///    producing a 3×3 destruction footprint per call. Step `cur` along
///    the chosen axial direction after each iteration, break early when
///    the next cell leaves the bridge band.
///
/// Net coverage for a 3-wide bridge: ~3 perp × 6 axial = ~18 cells per
/// invocation (axial 3-cell windows overlap by 2 across iterations).
/// For the 1-wide bridges in test fixtures, ~4 axial cells.
fn run_hut_collapse_bounded(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    presentation: &mut BridgePresentationContext<'_>,
    family: HutBridgeFamily,
    axis: Axis,
    seed_rx: u16,
    seed_ry: u16,
) -> Vec<StateOutcome> {
    // Phase 1: extent measurement in both axial directions.
    let seed = (seed_rx, seed_ry);
    let back = measure_extent(bridge_state, family, seed, axis, -1);
    let fwd = measure_extent(bridge_state, family, seed, axis, 1);

    // Phase 2: pick step direction + biased start cell. Signed integer
    // division (round toward zero) matches gamemd's Asm `idiv` semantics.
    let step: i16 = if fwd < back { -1 } else { 1 };
    let bias: i32 = (back as i32 - fwd as i32) / 2;
    // start = seed - bias (gamemd: `uVar9 - (iVar11 - iVar10) / 2`).
    let Some(mut cur) = step_axis_by(seed, axis, -bias) else {
        return Vec::new();
    };

    // Phase 3: 4-iteration walker.
    let mut outcomes: Vec<StateOutcome> = Vec::new();
    for _ in 0..MAX_HUT_SWEEP_STEPS {
        spawn_hut_walker_pre_destroy_effects(
            bridge_state,
            terrain,
            presentation,
            family,
            axis,
            cur,
        );
        // Inner retry: a healthy cell takes 2 calls to reach Destroyed
        // (Healthy → Damaged → Destroyed). gamemd's `iVar10 < 3` retry
        // loop covers this.
        for _ in 0..MAX_HUT_ATTEMPTS_PER_STEP {
            let outcome = call_destroy_per_family(bridge_state, terrain, family, cur);
            match outcome {
                StateOutcome::NoChange => {}
                StateOutcome::Absorbed => {
                    outcomes.push(StateOutcome::Absorbed);
                    // Retry: the cell took a state step but is not yet
                    // collapsed — another call may push it to Destroyed.
                }
                collapsed @ StateOutcome::Collapsed { .. } => {
                    let success = collapsed.apply_damage_success();
                    outcomes.push(collapsed);
                    if success {
                        break;
                    }
                }
            }
        }

        // Step along the chosen axial direction. Break if the step
        // would leave the map.
        let Some(next) = step_axis(cur, axis, step) else {
            break;
        };
        // Break if the next cell is outside the bridge overlay band.
        // gamemd's check is identical: `cellclass.overlay < 0xCD ||
        // cellclass.overlay > 0xE8` for high (and `0x4A`/`0x65` for low).
        let Some(overlay) = bridge_state.cell(next.0, next.1).map(|c| c.overlay_byte) else {
            break;
        };
        if !in_bridge_band(family, overlay) {
            break;
        }
        cur = next;
    }

    outcomes
}

fn spawn_hut_walker_pre_destroy_effects(
    bridge_state: &BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    presentation: &mut BridgePresentationContext<'_>,
    family: HutBridgeFamily,
    physical_axis: Axis,
    center: (u16, u16),
) {
    if presentation.bridge_explosions.is_empty() {
        return;
    }
    let Some(overlay) = bridge_state
        .cell(center.0, center.1)
        .map(|c| c.overlay_byte)
    else {
        return;
    };
    if overlay == hut_walker_terminal_cap(family, physical_axis) {
        return;
    }
    let perpendicular = match physical_axis {
        Axis::EW => Axis::NS,
        Axis::NS => Axis::EW,
    };
    for delta in [-1, 0, 1] {
        if let Some((rx, ry)) = step_axis(center, perpendicular, delta) {
            let z = terrain
                .cell(rx, ry)
                .map(|c| c.bridge_deck_level_if_any().unwrap_or(c.level))
                .unwrap_or(0);
            spawn_bridge_explosion_effect(presentation, rx, ry, z);
        }
    }
}

fn hut_walker_terminal_cap(family: HutBridgeFamily, physical_axis: Axis) -> u8 {
    match (family, physical_axis) {
        (HutBridgeFamily::High, Axis::EW) => 0xE7,
        (HutBridgeFamily::High, Axis::NS) => 0xE8,
        (HutBridgeFamily::Low, Axis::EW) => 0x64,
        (HutBridgeFamily::Low, Axis::NS) => 0x65,
    }
}

/// Count cells in the bridge overlay band along `axis` in direction
/// `dir` from `seed`. Stops at the first off-band cell, off-map step, or
/// `MAX_EXTENT_PROBE` iterations (safety cap).
fn measure_extent(
    bridge_state: &BridgeRuntimeState,
    family: HutBridgeFamily,
    seed: (u16, u16),
    axis: Axis,
    dir: i16,
) -> u32 {
    let mut count: u32 = 0;
    let mut cur = seed;
    for _ in 0..MAX_EXTENT_PROBE {
        let Some(next) = step_axis(cur, axis, dir) else {
            break;
        };
        let Some(overlay) = bridge_state.cell(next.0, next.1).map(|c| c.overlay_byte) else {
            break;
        };
        if !in_bridge_band(family, overlay) {
            break;
        }
        count += 1;
        cur = next;
    }
    count
}

fn in_bridge_band(family: HutBridgeFamily, overlay: u8) -> bool {
    match family {
        HutBridgeFamily::High => (0xCD..=0xE8).contains(&overlay),
        HutBridgeFamily::Low => (0x4A..=0x65).contains(&overlay),
    }
}

fn call_destroy_per_family(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    family: HutBridgeFamily,
    cell: (u16, u16),
) -> StateOutcome {
    match family {
        HutBridgeFamily::High => bridge_state.destroy_bridge_high(cell.0, cell.1, terrain),
        HutBridgeFamily::Low => bridge_state.destroy_bridge_low(cell.0, cell.1, terrain),
    }
}

/// Multi-cell axial step. Saturates to u16 bounds — out-of-map → None.
fn step_axis_by(pos: (u16, u16), axis: Axis, delta: i32) -> Option<(u16, u16)> {
    let (rx, ry) = pos;
    match axis {
        Axis::EW => {
            let next = rx as i32 + delta;
            (0..=u16::MAX as i32)
                .contains(&next)
                .then_some((next as u16, ry))
        }
        Axis::NS => {
            let next = ry as i32 + delta;
            (0..=u16::MAX as i32)
                .contains(&next)
                .then_some((rx, next as u16))
        }
    }
}

fn apply_hut_damage_to_cell(
    bridge_state: &mut BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
) -> StateOutcome {
    let Some(cell) = bridge_state.cell(rx, ry).copied() else {
        return StateOutcome::NoChange;
    };

    if (0x4A..=0x63).contains(&cell.overlay_byte) {
        return bridge_state.destroy_bridge_low(rx, ry, terrain);
    }
    if (0xCD..=0xE6).contains(&cell.overlay_byte) {
        return bridge_state.destroy_bridge_high(rx, ry, terrain);
    }

    let is_high = !hut_cell_is_low_bridge(bridge_state, terrain, rx, ry);
    match cell.role {
        BridgeCellRole::Bridgehead => {
            bridge_state.bridgehead_advance_state(rx, ry, is_high, terrain)
        }
        BridgeCellRole::Anchor | BridgeCellRole::Body | BridgeCellRole::Tail => {
            bridge_state.body_cell_advance_state(rx, ry, is_high, terrain)
        }
    }
}

fn hut_cell_is_low_bridge(
    bridge_state: &BridgeRuntimeState,
    terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
) -> bool {
    bridge_state
        .cell(rx, ry)
        .is_some_and(|cell| BridgeRuntimeState::is_low_destroy_overlay(cell.overlay_byte))
        || terrain.cell(rx, ry).is_some_and(|cell| {
            cell.is_wood_bridge_repair_tile
                || cell.bridge_layer.as_ref().is_some_and(|layer| {
                    BridgeRuntimeState::is_low_destroy_overlay(layer.overlay_id)
                })
                || cell
                    .bridge_facts
                    .overlay_id
                    .is_some_and(BridgeRuntimeState::is_low_destroy_overlay)
        })
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
fn c4_inf_death(rules: &RuleSet, sim: &Simulation) -> u8 {
    let c4_id = rules.c4_warhead_id();
    let name = sim.interner.resolve(c4_id);
    rules.warhead(name).map(|wh| wh.inf_death).unwrap_or(1)
}

fn blow_up_bridge_cell_fallout(
    sim: &mut Simulation,
    rules: &RuleSet,
    rx: u16,
    ry: u16,
    c4_inf_death: u8,
) {
    kill_ground_occupants_at(sim, rx, ry, c4_inf_death);
    drop_in_bridge_deck_entities(sim, rx, ry);
    let mut one_cell = BTreeSet::new();
    one_cell.insert((rx, ry));
    spawn_bridge_debris(sim, rules, &one_cell);
}

fn kill_ground_occupants_at(sim: &mut Simulation, rx: u16, ry: u16, c4_inf_death: u8) {
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

/// Per-cell debris spawn. Mirror of binary `BlowUpBridge` step 4. RNG draw
/// order is parity-critical for lockstep; the binary draws in this exact
/// sequence per cell that passes the outer gate:
/// 1. Outer normalized 95% gate.
/// 2. Two normalized jitter draws, converted to in-cell offsets.
/// 3. MetallicDebris normalized 50% gate.
/// 4. Optional MetallicDebris slot when the gate passed and debris exists.
/// 5. BridgeExplosion delay in 1..=5 frames.
/// 6. BridgeExplosion slot.
///
/// Replaces the wrong-shape legacy `Simulation::spawn_bridge_explosions`,
/// which drew 1 immediate BridgeExplosion + a 50% delayed BridgeExplosion
/// — visible every collapse.
fn spawn_bridge_debris(sim: &mut Simulation, _rules: &RuleSet, cells: &BTreeSet<(u16, u16)>) {
    use crate::sim::components::WorldEffect;

    let explosion_count = sim.bridge_explosions.len() as u32;
    let metallic_count = sim.metallic_debris.len() as u32;

    if explosion_count == 0 {
        return;
    }

    for &(rx, ry) in cells {
        // Step 1: outer 95% gate.
        let outer_draw = sim
            .bridge_rng()
            .next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
        if outer_draw >= BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE {
            continue;
        }

        // Step 2: two normalized jitter draws become the in-cell offsets.
        let (sub_x, sub_y) = bridge_jittered_subcells(sim.bridge_rng());

        let deck_level = sim
            .resolved_terrain
            .as_ref()
            .and_then(|t| t.cell(rx, ry))
            .map(|c| c.bridge_deck_level_if_any().unwrap_or(c.level))
            .unwrap_or(0);

        // Step 3: MetallicDebris 50% gate.
        let metallic_draw = sim
            .bridge_rng()
            .next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
        let metallic_pass = metallic_draw < BRIDGE_METALLIC_GATE_EXCLUSIVE;
        // Step 4: MetallicDebris slot pick + spawn (no delay). Slot draw
        // only happens when all three gates pass — short-circuit matches
        // the binary's call order.
        if metallic_pass && metallic_count > 0 {
            let idx = sim.bridge_rng().next_range_u32(metallic_count) as usize;
            let anim_id = sim.metallic_debris[idx];
            let frames = sim.effect_frame_counts.get(&anim_id).copied().unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                anim_spawn: None,
                shp_name: anim_id,
                rx,
                ry,
                sub_x,
                sub_y,
                z: deck_level,
                frame: 0,
                total_frames: frames,
                rate_ms: BRIDGE_EFFECT_FRAME_MS,
                elapsed_ms: 0,
                translucent: true,
                delay_ms: 0,
                start_sound_id: None,
                start_sound_emitted: false,
            });
        }

        // Step 5 + 6: always BridgeExplosion, delayed 1-5 frames.
        if explosion_count > 0 {
            let delay_frames = sim.bridge_rng().next_range_u32_inclusive(1, 5);
            let idx = sim.bridge_rng().next_range_u32(explosion_count) as usize;
            let anim_id = sim.bridge_explosions[idx];
            let frames = sim.effect_frame_counts.get(&anim_id).copied().unwrap_or(20);
            sim.world_effects.push(WorldEffect {
                anim_spawn: None,
                shp_name: anim_id,
                rx,
                ry,
                sub_x,
                sub_y,
                z: deck_level,
                frame: 0,
                total_frames: frames,
                rate_ms: BRIDGE_EFFECT_FRAME_MS,
                elapsed_ms: 0,
                translucent: true,
                delay_ms: delay_frames * BRIDGE_EFFECT_FRAME_MS,
                start_sound_id: sim.bridge_anim_sounds.get(&anim_id).copied(),
                start_sound_emitted: false,
            });
        }
    }
}

fn spawn_bridge_explosion_effect(
    presentation: &mut BridgePresentationContext<'_>,
    rx: u16,
    ry: u16,
    z: u8,
) {
    if presentation.bridge_explosions.is_empty() {
        return;
    }
    let (sub_x, sub_y) = bridge_jittered_subcells(presentation.rng);
    let delay_frames = presentation.rng.next_range_u32_inclusive(1, 5);
    let idx = presentation
        .rng
        .next_range_u32(presentation.bridge_explosions.len() as u32) as usize;
    let anim_id = presentation.bridge_explosions[idx];
    let frames = presentation
        .effect_frame_counts
        .get(&anim_id)
        .copied()
        .unwrap_or(20);
    presentation
        .world_effects
        .push(crate::sim::components::WorldEffect {
            anim_spawn: None,
            shp_name: anim_id,
            rx,
            ry,
            sub_x,
            sub_y,
            z,
            frame: 0,
            total_frames: frames,
            rate_ms: BRIDGE_EFFECT_FRAME_MS,
            elapsed_ms: 0,
            translucent: true,
            delay_ms: delay_frames * BRIDGE_EFFECT_FRAME_MS,
            start_sound_id: presentation.bridge_anim_sounds.get(&anim_id).copied(),
            start_sound_emitted: false,
        });
}

fn bridge_jittered_subcells(rng: &mut SimRng) -> (SimFixed, SimFixed) {
    let x_draw = rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
    let y_draw = rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
    (
        bridge_jittered_subcell(x_draw),
        bridge_jittered_subcell(y_draw),
    )
}

fn bridge_jittered_subcell(draw: u32) -> SimFixed {
    let offset = ((u64::from(draw) * BRIDGE_JITTER_SPAN_LEPTONS) / NORMALIZED_RNG_DENOMINATOR)
        as i32
        - BRIDGE_JITTER_HALF_LEPTONS;
    CELL_CENTER_LEPTON + SimFixed::from_num(offset)
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
    use crate::sim::occupancy::CellListInsertion;

    let ground_level = sim
        .resolved_terrain
        .as_ref()
        .and_then(|t| t.cell(rx, ry))
        .map(|c| c.level)
        .unwrap_or(0);

    let to_snap: Vec<u64> = sim
        .entities
        .iter_sorted()
        .filter(|(_, e)| e.position.rx == rx && e.position.ry == ry && e.is_on_bridge_layer())
        .map(|(id, _)| id)
        .collect();

    for id in to_snap {
        let mut relayer = None;
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
            relayer = Some((
                entity.position.rx,
                entity.position.ry,
                entity.sub_cell,
                CellListInsertion::from_category(entity.category),
            ));
        }
        if let Some((rx, ry, sub_cell, insertion)) = relayer {
            sim.occupancy.move_entity(
                rx,
                ry,
                rx,
                ry,
                id,
                MovementLayer::Ground,
                sub_cell,
                insertion,
            );
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
    // bridge collapse — scenario stream. Direct field (NOT bridge_rng()): held with a
    // live sim.bridge_state borrow; the disjoint-field split above is required.
    let rng = &mut sim.scenario_rng;

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
                                bridge_state
                                    .bridgehead_advance_state(event.rx, event.ry, true, terrain)
                            }
                            _ => bridge_state
                                .body_cell_advance_state(event.rx, event.ry, true, terrain),
                        }
                    }
                    DispatchPath::LowStateMachine => {
                        match bridge_state.cell(event.rx, event.ry).map(|c| c.role) {
                            Some(crate::sim::bridge_state::BridgeCellRole::Bridgehead) => {
                                bridge_state
                                    .bridgehead_advance_state(event.rx, event.ry, false, terrain)
                            }
                            _ => bridge_state
                                .body_cell_advance_state(event.rx, event.ry, false, terrain),
                        }
                    }
                    DispatchPath::HighDirect => {
                        bridge_state.destroy_bridge_high(event.rx, event.ry, terrain)
                    }
                    DispatchPath::LowDirect => {
                        bridge_state.destroy_bridge_low(event.rx, event.ry, terrain)
                    }
                };
                let success = outcome.apply_damage_success();
                if outcome.has_effect() {
                    outcomes.push(outcome);
                }
                if success {
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
    use crate::sim::components::{BridgeOccupancy, Health};
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::test_intern;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::sim::occupancy::CellListInsertion;
    use crate::util::fixed_math::{SimFixed, SIM_ZERO};

    fn seed_bridge_cell(overlay_byte: u8) -> crate::sim::bridge_state::BridgeRuntimeCell {
        crate::sim::bridge_state::BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: None,
            role: BridgeCellRole::Body,
            anchor_span_id: None,
            overlay_byte,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        }
    }

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
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: false,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
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
                    allows_tiberium: false,
                    has_ramp: false,
                    canonical_ramp: None,
                    ground_walk_blocked: is_bridge,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    base_land_type: 0,
                    base_yr_cell_land_type: 0,
                    base_terrain_class: Default::default(),
                    base_speed_costs: Default::default(),
                    build_blocked: is_bridge,
                    has_bridge_deck: is_bridge,
                    bridge_walkable: is_bridge,
                    bridge_transition: is_bridge,
                    bridge_deck_level: if is_bridge { deck_level } else { 0 },
                    bridge_layer: None,
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(6, 6, cells)
    }

    /// Build a Drive locomotor on the Bridge layer (mimics `high=true` spawn).
    fn drive_loco_on_bridge() -> LocomotorState {
        LocomotorState {
            kind: LocomotorKind::Drive,
            primary_kind: Some(LocomotorKind::Drive),
            piggyback: None,
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
        sim.occupancy.add(
            5,
            5,
            id,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

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
        let cell = sim.occupancy.get(5, 5).expect("occupancy retained");
        assert_eq!(cell.count_on(MovementLayer::Ground), 1);
        assert_eq!(cell.count_on(MovementLayer::Bridge), 0);
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

    #[test]
    fn hut_destroy_overlay_seed_uses_physical_span_axis_not_walker_family() {
        use crate::sim::bridge_state::BridgeRuntimeState;

        let mut high_ew_range = BridgeRuntimeState::default();
        high_ew_range.test_seed_cell(5, 4, seed_bridge_cell(0xCD));
        high_ew_range.test_seed_cell(5, 5, seed_bridge_cell(0xCD));
        assert_eq!(
            find_destroy_overlay_seed(&high_ew_range, &[(5, 5)], HutBridgeFamily::High),
            Some((5, 5, Axis::EW)),
            "0xCD high range dispatches to CollapseBridge_EW_High, so the hut sweep must step X"
        );

        let mut high_ns_range = BridgeRuntimeState::default();
        high_ns_range.test_seed_cell(4, 5, seed_bridge_cell(0xD6));
        high_ns_range.test_seed_cell(5, 5, seed_bridge_cell(0xD6));
        assert_eq!(
            find_destroy_overlay_seed(&high_ns_range, &[(5, 5)], HutBridgeFamily::High),
            Some((5, 5, Axis::NS)),
            "0xD6 high range dispatches to CollapseBridge_NS_High, so the hut sweep must step Y"
        );

        let mut low_ew_range = BridgeRuntimeState::default();
        low_ew_range.test_seed_cell(5, 4, seed_bridge_cell(0x4A));
        low_ew_range.test_seed_cell(5, 5, seed_bridge_cell(0x4A));
        assert_eq!(
            find_destroy_overlay_seed(&low_ew_range, &[(5, 5)], HutBridgeFamily::Low),
            Some((5, 5, Axis::EW)),
            "0x4A low range dispatches to CollapseBridge_EW_Low, so the hut sweep must step X"
        );

        let mut low_ns_range = BridgeRuntimeState::default();
        low_ns_range.test_seed_cell(4, 5, seed_bridge_cell(0x53));
        low_ns_range.test_seed_cell(5, 5, seed_bridge_cell(0x53));
        assert_eq!(
            find_destroy_overlay_seed(&low_ns_range, &[(5, 5)], HutBridgeFamily::Low),
            Some((5, 5, Axis::NS)),
            "0x53 low range dispatches to CollapseBridge_NS_Low, so the hut sweep must step Y"
        );
    }

    #[test]
    fn cabhut_seed_canonicalization_shifts_edge_hit_forward() {
        use crate::sim::bridge_state::BridgeRuntimeState;

        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(5, 5, seed_bridge_cell(0xCD));

        assert_eq!(
            find_destroy_overlay_seed(&state, &[(5, 5)], HutBridgeFamily::High),
            Some((5, 6, Axis::EW)),
            "when no back lane is in the bridge band, DestroyBridgeFromCell shifts one cell forward"
        );
    }

    #[test]
    fn cabhut_seed_canonicalization_keeps_middle_hit() {
        use crate::sim::bridge_state::BridgeRuntimeState;

        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(5, 4, seed_bridge_cell(0xCD));
        state.test_seed_cell(5, 5, seed_bridge_cell(0xCD));

        assert_eq!(
            find_destroy_overlay_seed(&state, &[(5, 5)], HutBridgeFamily::High),
            Some((5, 5, Axis::EW)),
            "one in-band back lane and one off-band second back lane keeps the matched cell"
        );
    }

    #[test]
    fn cabhut_seed_canonicalization_shifts_two_cells_in_backward() {
        use crate::sim::bridge_state::BridgeRuntimeState;

        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(5, 3, seed_bridge_cell(0xCD));
        state.test_seed_cell(5, 4, seed_bridge_cell(0xCD));
        state.test_seed_cell(5, 5, seed_bridge_cell(0xCD));

        assert_eq!(
            find_destroy_overlay_seed(&state, &[(5, 5)], HutBridgeFamily::High),
            Some((5, 4, Axis::EW)),
            "two in-band back probes shift the canonical seed one cell backward"
        );
    }

    #[test]
    fn hut_destroy_scan_uses_gamemd_x_major_order() {
        let cells: Vec<(u16, u16)> = hut_destroy_5x5_scan((10, 10)).collect();
        assert_eq!(cells.len(), 25);
        assert_eq!(
            &cells[..6],
            &[(8, 8), (8, 9), (8, 10), (8, 11), (8, 12), (9, 8)],
            "hut death scan must walk each X column before advancing X"
        );

        let edge_cells: Vec<(u16, u16)> = hut_destroy_5x5_scan((0, 0)).collect();
        assert_eq!(edge_cells.len(), 9);
        assert_eq!(
            &edge_cells[..3],
            &[(0, 0), (0, 1), (0, 2)],
            "off-map negative cells are skipped while preserving X-major order"
        );
    }

    #[test]
    fn hut_destroy_overlay_seed_prefers_x_major_first_match() {
        use crate::sim::bridge_state::BridgeRuntimeState;

        let scan: Vec<(u16, u16)> = hut_destroy_5x5_scan((10, 10)).collect();
        let mut state = BridgeRuntimeState::default();
        state.test_seed_cell(9, 8, seed_bridge_cell(0xCD));
        state.test_seed_cell(8, 12, seed_bridge_cell(0xCD));

        assert_eq!(
            find_destroy_overlay_seed(&state, &scan, HutBridgeFamily::High),
            Some((8, 13, Axis::EW)),
            "Y-major scan would find (9,8) first; gamemd hut death finds the earlier X column"
        );
    }

    /// Task 12 - RNG draw-order parity: per cell, `spawn_bridge_debris`
    /// MUST consume RNG draws in the exact binary order:
    /// outer-95% → jitter×2 → metallic-50% → optional metallic-slot →
    /// explosion-delay → explosion-slot. Wrong order desyncs lockstep.
    #[test]
    fn debris_consumes_correct_rng_count_per_cell() {
        let mut sim = Simulation::new();
        let seed = 0xDEAD_BEEF_u64;
        sim.reseed_both(seed);
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
        let outer = predicted.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
        if outer < BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE {
            let _jx = predicted.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
            let _jy = predicted.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
            let metallic_draw = predicted.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
            // Metallic slot draw is gated on the 50% predicate and the
            // presence of MetallicDebris entries. BridgeVoxelMax is not part
            // of standard BlowUpBridge debris gating.
            if metallic_draw < BRIDGE_METALLIC_GATE_EXCLUSIVE {
                let _slot = predicted.next_range_u32(1);
            }
            let _delay = predicted.next_range_u32_inclusive(1, 5);
            let _exp_slot = predicted.next_range_u32(2);
        }

        spawn_bridge_debris(&mut sim, &rules, &cells);

        assert_eq!(
            sim.scenario_rng.state(),
            predicted.state(),
            "RNG draw order/count diverged from binary parity sequence"
        );
    }

    /// Fixture sanity check: this seed fails the verified metallic gate, so
    /// no MetallicDebris should spawn even though BridgeVoxelMax is zero.
    #[test]
    fn bridge_debris_no_metallic_when_gate_fails_even_with_voxel_zero() {
        let mut sim = Simulation::new();
        let seed = 0xDEAD_BEEF_u64;
        sim.reseed_both(seed);
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        sim.bridge_explosions.push(test_intern("BRIDGEEXP1"));
        sim.metallic_debris.push(test_intern("METALDEB1"));
        let rules = rules_with_voxel_max(0);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));
        spawn_bridge_debris(&mut sim, &rules, &cells);

        // No MetallicDebris effect must spawn when the verified 50% gate fails.
        let metallic_id = test_intern("METALDEB1");
        assert!(
            !sim.world_effects
                .iter()
                .any(|fx| fx.shp_name == metallic_id),
            "metallic gate failure must suppress MetallicDebris spawn"
        );
    }

    #[test]
    fn bridge_debris_ignores_bridge_voxel_max_when_metallic_gate_passes() {
        let seed = (1u64..10_000)
            .find(|seed| {
                let mut rng = crate::sim::rng::SimRng::new(*seed);
                let outer = rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
                if outer >= BRIDGE_DEBRIS_OUTER_GATE_EXCLUSIVE {
                    return false;
                }
                let _ = rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
                let _ = rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE);
                rng.next_range_u32_inclusive(0, NORMALIZED_RNG_MAX_INCLUSIVE)
                    < BRIDGE_METALLIC_GATE_EXCLUSIVE
            })
            .expect("fixture seed with metallic pass");

        let mut sim = Simulation::new();
        sim.reseed_both(seed);
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        sim.bridge_explosions.push(test_intern("BRIDGEEXP1"));
        sim.metallic_debris.push(test_intern("METALDEB1"));
        let rules = rules_with_voxel_max(0);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));
        spawn_bridge_debris(&mut sim, &rules, &cells);

        let metallic_id = test_intern("METALDEB1");
        assert!(
            sim.world_effects
                .iter()
                .any(|fx| fx.shp_name == metallic_id),
            "BridgeVoxelMax=0 must not suppress standard BlowUpBridge metallic debris"
        );
    }

    /// Debris helper short-circuits when the required BridgeExplosion list is
    /// empty. MetallicDebris alone does not enable BlowUpBridge presentation.
    #[test]
    fn bridge_debris_requires_bridge_explosion_list() {
        let mut sim = Simulation::new();
        sim.reseed_both(7);
        let baseline_state = sim.scenario_rng.state();
        sim.resolved_terrain = Some(water_below_bridge_terrain(3));
        sim.metallic_debris.push(test_intern("METALDEB1"));
        let rules = rules_with_voxel_max(3);

        let mut cells = BTreeSet::new();
        cells.insert((5, 5));
        cells.insert((4, 5));
        spawn_bridge_debris(&mut sim, &rules, &cells);

        assert_eq!(
            sim.scenario_rng.state(),
            baseline_state,
            "no RNG draws when BridgeExplosion metadata is absent"
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
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
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
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
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
        loco.layer = MovementLayer::Bridge;
        entity.locomotor = Some(loco);
        sim.entities.insert(entity);
        sim.occupancy.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        drop_in_bridge_deck_entities(&mut sim, 5, 5);

        // Ground entity untouched — still alive, still ground layer.
        let e = sim.entities.get(1).expect("ground entity untouched");
        assert_eq!(e.health.current, 256);
        assert!(!e.on_bridge);
        assert_eq!(e.locomotor.as_ref().unwrap().layer, MovementLayer::Bridge);
        let cell = sim.occupancy.get(5, 5).expect("ground occupancy");
        assert_eq!(cell.count_on(MovementLayer::Ground), 1);
        assert_eq!(cell.count_on(MovementLayer::Bridge), 0);
    }
}
