//! Integration tests for the engineer-bridge-repair flow + the C4-on-CABHUT
//! collapse path. Engineer entry repairs the bridge and consumes the
//! engineer; C4 on the hut leaves the hut at full HP and collapses the
//! bridge segment via the BridgeRepairHut branch in
//! `apply_c4_damage_to_building`.

use super::*;
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_state::{
    AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState, Direction,
};
use crate::sim::components::Health;
use crate::sim::game_entity::GameEntity;
use std::collections::BTreeMap;

/// Minimal 20x20 flat terrain so the repair path's `(bs, terrain)` gate
/// succeeds. has_damaged_data=false → the embedded flood-fill clear is a
/// no-op, leaving the repair test focused on damage-state transitions.
fn dummy_resolved_terrain() -> ResolvedTerrainGrid {
    let mut cells = Vec::with_capacity(20 * 20);
    for ry in 0..20u16 {
        for rx in 0..20u16 {
            cells.push(ResolvedTerrainCell {
                rx,
                ry,
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
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
            });
        }
    }
    ResolvedTerrainGrid::from_cells(20, 20, cells)
}

fn bridge_repair_test_rules() -> RuleSet {
    let ini: IniFile = IniFile::from_str(
        "[InfantryTypes]\n0=ENGI\n1=GHOST\n\n\
         [VehicleTypes]\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n0=CABHUT\n\n\
         [ENGI]\nStrength=75\nArmor=none\nSpeed=4\nPrimary=none\nEngineer=yes\n\n\
         [GHOST]\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=none\nC4=yes\n\n\
         [CABHUT]\nStrength=200\nArmor=concrete\nFoundation=1x1\nBridgeRepairHut=yes\n\n\
         [AudioVisual]\nRepairBridgeSound=BridgeRepaired\n\n\
         [CombatDamage]\nC4Warhead=SA\n\n\
         [SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    RuleSet::from_ini(&ini).expect("bridge-repair test rules should parse")
}

fn build_sim() -> (Simulation, RuleSet, BTreeMap<(u16, u16), u8>) {
    let mut sim = Simulation::new();
    let mut rules = bridge_repair_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);
    sim.resolved_terrain = Some(dummy_resolved_terrain());
    (sim, rules, BTreeMap::new())
}

fn spawn_engineer(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("ENGI");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 75,
            max: 75,
        },
        ty,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

fn spawn_seal(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GHOST");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 125,
            max: 125,
        },
        ty,
        EntityCategory::Infantry,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

fn spawn_cabhut(sim: &mut Simulation, rx: u16, ry: u16) -> u64 {
    let owner = sim.interner.intern("Soviets");
    let ty = sim.interner.intern("CABHUT");
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner,
        Health {
            current: 200,
            max: 200,
        },
        ty,
        EntityCategory::Structure,
        0,
        5,
        false,
    );
    sim.entities.insert(e);
    id
}

const BRIDGE_CELLS: &[(u16, u16)] = &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)];

fn seed_destroyed_bridge(sim: &mut Simulation) {
    seed_bridge_with_state(sim, DamageState::Destroyed);
}

fn seed_bridge_with_state(sim: &mut Simulation, state: DamageState) {
    let mut bs = BridgeRuntimeState::default();
    let span = AnchorSpan {
        id: 1,
        anchor: (10, 10),
        cells: [
            Some((10, 10)),
            Some((10, 11)),
            Some((10, 12)),
            Some((10, 13)),
            Some((10, 9)),
            None,
        ],
        axis: Axis::NS,
        direction: Direction::S,
        damage_state: state,
        bridge_group_id: 1,
    };
    bs.test_seed_anchor_span(span);
    for &(rx, ry) in BRIDGE_CELLS {
        let role = if (rx, ry) == (10, 10) {
            BridgeCellRole::Anchor
        } else {
            BridgeCellRole::Body
        };
        bs.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: state,
                axis: Some(Axis::NS),
                role,
                anchor_span_id: Some(1),
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
    }
    sim.bridge_state = Some(bs);
}

fn step(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) -> TickResult {
    let due = sim.take_due_commands();
    sim.advance_tick(&due, Some(rules), heights, None, None, 67)
}

#[test]
fn engineer_enters_cabhut_repairs_bridge() {
    let (mut sim, rules, heights) = build_sim();
    // Engineer adjacent (Chebyshev-1) to a CABHUT at (9, 10). Bridge cells
    // sit at (10, 9..=13) — the engineer's 5×5 scan covers all of them.
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    let result = step(&mut sim, &rules, &heights);

    assert!(
        result.bridge_state_changed,
        "TickResult.bridge_state_changed must be set on repair"
    );
    assert!(
        sim.entities.get(engineer).is_none(),
        "engineer must be despawned after repair"
    );

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in BRIDGE_CELLS {
        match bs.cell(rx, ry).unwrap().damage_state {
            DamageState::Healthy { variant } => assert!(
                variant <= 3,
                "cell ({rx},{ry}) variant={variant} — must be 0..=3 (healthy)"
            ),
            other => panic!("cell ({rx},{ry}) = {other:?} (expected Healthy)"),
        }
        assert!(bs.is_bridge_walkable(rx, ry));
    }
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "BridgeRepaired sound event must be emitted"
    );
}

#[test]
fn engineer_at_intact_cabhut_emits_sound_no_mutation() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });

    let result = step(&mut sim, &rules, &heights);

    assert!(
        sim.entities.get(engineer).is_none(),
        "engineer still consumed even when bridge is intact"
    );
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "sound event always fires on trigger"
    );
    assert!(
        !result.bridge_state_changed,
        "intact bridge: no zone rebuild signal"
    );
}

#[test]
fn two_engineers_both_repair_same_tick() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer_a = spawn_engineer(&mut sim, 10, 10);
    let engineer_b = spawn_engineer(&mut sim, 10, 11);
    sim.entities.get_mut(engineer_a).unwrap().capture_target = Some(cabhut);
    sim.entities.get_mut(engineer_b).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer_a).is_none());
    assert!(sim.entities.get(engineer_b).is_none());
    let repair_events = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. }))
        .count();
    assert_eq!(repair_events, 2, "both engineers emit a sound event");

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in BRIDGE_CELLS {
        assert!(matches!(
            bs.cell(rx, ry).unwrap().damage_state,
            DamageState::Healthy { .. }
        ));
    }
}

#[test]
fn engineer_far_from_bridge_at_cabhut_no_mutation() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    // Empty bridge state — scan finds nothing.
    sim.bridge_state = Some(BridgeRuntimeState::default());

    let result = step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer).is_none());
    assert!(
        sim.sound_events
            .iter()
            .any(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. })),
        "sound emitted; no bridge to mutate"
    );
    assert!(!result.bridge_state_changed);
}

/// SEAL with `c4_plant` set, adjacent to a healthy CABHUT, must:
///   - claim the plant on its first tick (adjacency-1),
///   - leave the hut at full HP across the entire C4Delay window,
///   - on timer expiry, route through the BridgeRepairHut branch in
///     `apply_c4_damage_to_building` so the bridge collapses while the
///     hut survives,
///   - propagate `bridge_state_changed` to TickResult so the app rebuilds
///     PathGrid.
///
/// Cascading-subsystem coverage (ground-occupant kill on BlowUpBridge
/// cells, deck-tank drop, zone_grid rebuild) is intentionally NOT
/// asserted here — those are owned by the bridge cascade tests proper.
/// This test only asserts the C4-on-CABHUT integration points.
#[test]
fn c4_on_cabhut_collapses_bridge_and_hut_survives() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let cabhut_max_hp = sim.entities.get(cabhut).unwrap().health.current;
    let seal = spawn_seal(&mut sim, 10, 10); // Chebyshev-1 adjacent
    sim.entities.get_mut(seal).unwrap().c4_plant =
        Some(crate::sim::components::C4PlantState {
            target_building_id: cabhut,
        });
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });

    // First tick: tick_c4_plants Phase 1 sees adjacency and claims.
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities
            .get(cabhut)
            .and_then(|b| b.pending_c4_detonation)
            .is_some(),
        "plant must be claimed once SEAL is adjacent to CABHUT"
    );
    let plant_start = sim
        .entities
        .get(cabhut)
        .unwrap()
        .pending_c4_detonation
        .unwrap()
        .plant_start_tick;

    // Throughout the C4Delay window: hut HP must stay at max — the
    // BridgeRepairHut branch never damages the hut, even before the timer
    // fires. Bridge stays Healthy until detonation, then flips.
    let delay = rules.c4_delay_ticks as u64;
    let mut bridge_state_changed_seen = false;
    for _ in 0..(delay + 1) {
        let result = step(&mut sim, &rules, &heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
        // Hut HP invariant — hold across every tick of the window.
        let cur = sim.entities.get(cabhut).unwrap().health.current;
        assert_eq!(
            cur, cabhut_max_hp,
            "hut HP must stay at max during C4Delay (plant_start={plant_start}, sim.tick={})",
            sim.tick
        );
    }

    // After detonation: hut alive, bridge segment Destroyed,
    // bridge_state_changed propagated at least once.
    let hut = sim
        .entities
        .get(cabhut)
        .expect("hut entity must survive the explosion");
    assert_eq!(
        hut.health.current, cabhut_max_hp,
        "hut HP unchanged: BridgeRepairHut branch must skip damage"
    );
    assert!(!hut.dying, "hut must not be marked dying");

    // Anchor cell must reach Destroyed via the body_cell_advance_state
    // walker invoked by `dispatch_bridge_collapse_from_hut`. Body cells
    // along the span propagate via the cascade machinery owned by the
    // bridge_orchestrator tests — this test does not re-assert that here.
    let bs = sim.bridge_state.as_ref().unwrap();
    let anchor = bs.cell(10, 10).unwrap();
    assert!(
        matches!(anchor.damage_state, DamageState::Destroyed),
        "anchor cell (10,10) must be Destroyed after C4 cascade, got {:?}",
        anchor.damage_state
    );

    assert!(
        bridge_state_changed_seen,
        "TickResult.bridge_state_changed must fire at least once so the app rebuilds PathGrid"
    );
}

// ---- G4 damaged-variant lifecycle integration tests ------------------------

/// 20×20 terrain with has_damaged_data=true and a common final_tile_index on
/// every cell. Lets the damaged-variant flood-fill propagate freely across
/// any bridge cells defined in the test BridgeRuntimeState.
fn damaged_data_resolved_terrain(tile_id: i32) -> ResolvedTerrainGrid {
    let mut cells = Vec::with_capacity(20 * 20);
    for ry in 0..20u16 {
        for rx in 0..20u16 {
            cells.push(ResolvedTerrainCell {
                rx,
                ry,
                source_tile_index: tile_id,
                source_sub_tile: 0,
                final_tile_index: tile_id,
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
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: true,
            });
        }
    }
    ResolvedTerrainGrid::from_cells(20, 20, cells)
}

/// Seed a single NS-anchor body cell at `pos` with the given state. Uses span
/// id derived from the coord so callers can place multiple independent
/// anchors without collisions.
fn seed_isolated_anchor(
    bs: &mut BridgeRuntimeState,
    pos: (u16, u16),
    span_id: u16,
    state: DamageState,
    damaged_variant: bool,
) {
    let span = AnchorSpan {
        id: span_id,
        anchor: pos,
        cells: [Some(pos), None, None, None, None, None],
        axis: Axis::NS,
        direction: Direction::S,
        damage_state: state,
        bridge_group_id: span_id,
    };
    bs.test_seed_anchor_span(span);
    bs.test_seed_cell(
        pos.0,
        pos.1,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(span_id),
            damage_state: state,
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(span_id),
            overlay_byte: 0,
            damaged_variant,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
}

#[test]
fn g4_damage_path_sets_damaged_variant_at_perpendicular_target() {
    let mut bs = BridgeRuntimeState::default();
    // Seed anchor at (10, 10) and a perpendicular target anchor at (11, 10)
    // (one east — the DamageA perpendicular direction for an NS bridge).
    seed_isolated_anchor(&mut bs, (10, 10), 1, DamageState::Healthy { variant: 0 }, false);
    seed_isolated_anchor(&mut bs, (11, 10), 2, DamageState::Healthy { variant: 0 }, false);
    let terrain = damaged_data_resolved_terrain(42);

    let _ = bs.body_cell_advance_state(10, 10, true, &terrain);

    assert!(
        bs.cell(11, 10).unwrap().damaged_variant,
        "perpendicular target must acquire damaged_variant after DamageA write"
    );
    assert!(
        bs.cell(10, 10).unwrap().damaged_variant,
        "same-tile_id seed neighbor must acquire damaged_variant via flood-fill propagation"
    );
}

#[test]
fn g4_collapse_path_keeps_damaged_variant_set() {
    let mut bs = BridgeRuntimeState::default();
    // Pre-damaged anchor + perpendicular target, both already flagged
    // damaged_variant=true. The collapse step must NOT clear the bit.
    seed_isolated_anchor(&mut bs, (10, 10), 1, DamageState::Damaged, true);
    seed_isolated_anchor(&mut bs, (11, 10), 2, DamageState::Healthy { variant: 0 }, true);
    let terrain = damaged_data_resolved_terrain(42);

    let _ = bs.body_cell_advance_state(10, 10, true, &terrain);

    assert!(
        bs.cell(10, 10).unwrap().damaged_variant,
        "collapse must preserve damaged_variant on seed cell (state=true from collapse callers)"
    );
    assert!(
        bs.cell(11, 10).unwrap().damaged_variant,
        "collapse must preserve damaged_variant on perpendicular target"
    );
}

#[test]
fn g4_repair_clears_damaged_variant_on_repaired_cells() {
    let (mut sim, rules, heights) = build_sim();
    // Replace dummy terrain with one that allows the flood-fill clear to
    // actually fire (has_damaged_data=true).
    sim.resolved_terrain = Some(damaged_data_resolved_terrain(42));
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);
    // Pre-flag every bridge cell as damaged-variant.
    {
        let bs = sim.bridge_state.as_mut().unwrap();
        for &(rx, ry) in BRIDGE_CELLS {
            bs.cell_mut(rx, ry).unwrap().damaged_variant = true;
        }
    }

    step(&mut sim, &rules, &heights);

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in BRIDGE_CELLS {
        assert!(
            !bs.cell(rx, ry).unwrap().damaged_variant,
            "cell ({rx},{ry}) damaged_variant must be cleared after engineer-CABHUT repair"
        );
    }
}

#[test]
fn g4_repair_flood_fill_propagates_clear_to_same_tile_id_bridge_neighbor() {
    let (mut sim, rules, heights) = build_sim();
    sim.resolved_terrain = Some(damaged_data_resolved_terrain(42));
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    // Add an off-span bridge cell at (10, 14): same tile_id as BRIDGE_CELLS,
    // adjacent to (10, 13). NOT a member of anchor_span 1, so it is NOT
    // visited by body_cell_repair_state's per-cell walk. It can only get
    // cleared via flood-fill propagation from a same-tile_id neighbor.
    {
        let bs = sim.bridge_state.as_mut().unwrap();
        bs.test_seed_cell(
            10,
            14,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Destroyed,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: None,
                overlay_byte: 0,
                damaged_variant: true,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
        for &(rx, ry) in BRIDGE_CELLS {
            bs.cell_mut(rx, ry).unwrap().damaged_variant = true;
        }
    }

    step(&mut sim, &rules, &heights);

    let bs = sim.bridge_state.as_ref().unwrap();
    assert!(
        !bs.cell(10, 14).unwrap().damaged_variant,
        "off-span neighbor with matching tile_id must clear via flood-fill propagation"
    );
}

/// Build a small NS-axis bridge with a bridgehead at (2, 4) (h=8) and an
/// anchor at (2, 2) (h=4). Used by the bridgehead-direct-damage integration
/// test. Resolved-terrain dims: 5x5.
fn build_ns_bridge_with_bridgehead_for_dispatch()
-> (crate::map::resolved_terrain::ResolvedTerrainGrid, BridgeRuntimeState) {
    use crate::map::resolved_terrain::ResolvedTerrainCell;
    use crate::sim::bridge_state::BridgeheadAnchorClass;
    let mut cells = Vec::with_capacity(25);
    for ry in 0..5u16 {
        for rx in 0..5u16 {
            let template_height: u8 = if rx == 2 {
                match ry {
                    4 => 8,
                    3 => 6,
                    2 => 4,
                    _ => 0,
                }
            } else {
                0
            };
            cells.push(ResolvedTerrainCell {
                rx,
                ry,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                // level must be >= 4 so the HighStateMachine path matches.
                // Z-gate accepts impact_z within [level-1, level+1].
                level: 4,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                slope_type: 0,
                template_height,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs: SpeedCostProfile::default(),
                is_water: false,
                is_cliff_like: false,
                is_cliff_redraw: false,
                variant: 0,
                is_rough: false,
                is_road: false,
                accepts_smudge: false,
                has_ramp: false,
                canonical_ramp: None,
                ground_walk_blocked: false,
                terrain_object_blocks: false,
                overlay_blocks: false,
                zone_type: 0,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                build_blocked: false,
                has_bridge_deck: true,
                bridge_walkable: true,
                bridge_transition: false,
                bridge_deck_level: 4,
                bridge_layer: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: false,
            });
        }
    }
    let resolved = crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells(5, 5, cells);

    // Build bridge state: bridgehead at (2, 4), anchor at (2, 2), and two
    // perpendicular Anchor neighbors at (1, 2) / (3, 2). Overlay 0x18 keeps
    // these cells out of the raw-body HighDirect range and routes the
    // dispatcher to the HighStateMachine path.
    //
    // Initial construction via `from_resolved_terrain` sets the global
    // `bridge_destroyable_flag = true` (required by the orchestrator's
    // outer gate); then `test_seed_cell` overrides per-cell state.
    let mut bs = BridgeRuntimeState::from_resolved_terrain(&resolved, true, 1500);
    bs.test_seed_cell(
        2,
        4,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0x18,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    bs.test_seed_cell(
        2,
        2,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x20,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    bs.test_seed_cell(
        3,
        2,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x21,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    bs.test_seed_cell(
        1,
        2,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0x22,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
        },
    );
    (resolved, bs)
}

/// Integration test: firing repeated IonCannon damage at a bridgehead
/// must not collapse the bridge. The orchestrator routes through
/// `bridgehead_advance_state`, which writes Damaged to the anchor's
/// tile-class field without touching the bridgehead's own damage_state.
#[test]
fn ramp_fire_does_not_collapse_high_bridge() {
    use crate::sim::bridge_state::{BridgeDamageEvent, BridgeheadAnchorClass};
    let mut sim = Simulation::new();
    let (resolved, bs) = build_ns_bridge_with_bridgehead_for_dispatch();
    sim.resolved_terrain = Some(resolved);
    sim.bridge_state = Some(bs);

    let mut rules = bridge_repair_test_rules();
    rules.resolve_bridge_warheads(&mut sim.interner);

    let pre_bridgehead = *sim.bridge_state.as_ref().unwrap().cell(2, 4).unwrap();

    for _ in 0..10 {
        let state_changed =
            crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
                &mut sim,
                &rules,
                &[BridgeDamageEvent {
                    rx: 2,
                    ry: 4,
                    damage: 999,
                    warhead_ref: crate::sim::intern::InternedId::default(),
                    is_ion_cannon: true,
                    impact_z: 4,
                }],
            );
        // No collapse → no path-grid refresh signal.
        assert!(
            !state_changed,
            "bridgehead direct damage must not signal state_changed (no collapse)",
        );
    }

    let bs = sim.bridge_state.as_ref().unwrap();
    // Bridgehead's own damage_state untouched.
    let post_bridgehead = *bs.cell(2, 4).unwrap();
    assert_eq!(
        post_bridgehead.damage_state, pre_bridgehead.damage_state,
        "bridgehead damage_state must not change on direct fire",
    );
    // Anchor's bridgehead_anchor_class = Damaged (idempotent across hits).
    assert_eq!(
        bs.cell(2, 2).unwrap().bridgehead_anchor_class,
        BridgeheadAnchorClass::Damaged,
        "anchor tile-class must transition to Damaged on first hit",
    );
    // Neither bridgehead nor anchor entered Destroyed.
    for cell in [bs.cell(2, 4).unwrap(), bs.cell(2, 2).unwrap()] {
        assert!(
            !matches!(cell.damage_state, DamageState::Destroyed),
            "no Destroyed cell from sustained bridgehead direct fire",
        );
    }
}
