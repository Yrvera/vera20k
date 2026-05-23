//! Integration tests for the engineer-bridge-repair flow + the C4-on-CABHUT
//! collapse path. Engineer entry repairs the bridge and consumes the
//! engineer; C4 on the hut leaves the hut at full HP and collapses the
//! bridge segment via the BridgeRepairHut branch in
//! `apply_c4_damage_to_building`.

use super::*;
use crate::map::bridge_facts::{
    BRIDGE_FLAG_ANCHOR_SELF, BRIDGE_FLAG_DESTROYED_OR_RAMP, BRIDGE_FLAG_DIRECTION_ZERO,
    BRIDGE_FLAG_STRUCTURAL, BridgeAnchorRelation, BridgeRampKind, BridgeRampTile,
    BridgeStampFamily, BridgeStampSlot,
};
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::bridge_state::{
    AnchorSpan, Axis, BridgeCellRole, BridgeRuntimeCell, BridgeRuntimeState, DamageState, Direction,
};
use crate::sim::command::Command;
use crate::sim::components::{Health, PendingC4Detonation};
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
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
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
const ENGINEER_REPAIR_STRIP_CELLS: &[(u16, u16)] = &[(10, 9), (10, 10), (10, 11)];

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
    let overlay_byte = match state {
        DamageState::Destroyed => 0xE7,
        DamageState::Damaged | DamageState::PartialCollapseA | DamageState::PartialCollapseB => {
            0xD1
        }
        DamageState::Healthy { .. } => 0xCD,
    };
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
                overlay_byte,
                damaged_variant: false,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
    }
    sim.bridge_state = Some(bs);
}

fn seed_low_bridge_with_state(sim: &mut Simulation, state: DamageState) {
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
    let overlay_byte = match state {
        DamageState::Destroyed => 0x64,
        DamageState::Damaged | DamageState::PartialCollapseA | DamageState::PartialCollapseB => {
            0x50
        }
        DamageState::Healthy { .. } => 0x4A,
    };
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
                overlay_byte,
                damaged_variant: false,
                bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
            },
        );
    }
    sim.bridge_state = Some(bs);
}

fn seed_hut_fallback_bridgehead_layout(sim: &mut Simulation) {
    let mut bs = BridgeRuntimeState::default();
    let span = AnchorSpan {
        id: 1,
        anchor: (13, 10),
        cells: [Some((13, 10)), None, None, None, None, None],
        axis: Axis::EW,
        direction: Direction::E,
        damage_state: DamageState::Damaged,
        bridge_group_id: 1,
    };
    bs.test_seed_anchor_span(span);
    bs.test_seed_cell(
        12,
        10,
        BridgeRuntimeCell {
            deck_present: false,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: None,
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::EW),
            role: BridgeCellRole::Bridgehead,
            anchor_span_id: None,
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    bs.test_seed_cell(
        13,
        10,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    sim.bridge_state = Some(bs);
    let terrain = sim
        .resolved_terrain
        .as_mut()
        .expect("bridge fallback tests require resolved terrain");
    let starter = terrain.cell_mut(12, 10).unwrap();
    starter.bridge_facts.raw_flags = BRIDGE_FLAG_STRUCTURAL;
    starter.bridge_facts.anchor = Some(BridgeAnchorRelation {
        anchor: (13, 10),
        slot: BridgeStampSlot::Forward1,
        family: BridgeStampFamily::Nesw,
        direction: 6,
    });
    let anchor = terrain.cell_mut(13, 10).unwrap();
    anchor.bridge_facts.raw_flags = 0;
    anchor.bridge_facts.ramp_tile = Some(BridgeRampTile {
        kind: BridgeRampKind::Middle1,
        relative_tile_index: 7,
        height_byte: 4,
    });
}

fn seed_hut_pure_bridgehead_fallback_layout(sim: &mut Simulation) {
    let mut bs = BridgeRuntimeState::default();
    let span = AnchorSpan {
        id: 1,
        anchor: (11, 10),
        cells: [Some((11, 10)), None, None, None, None, None],
        axis: Axis::EW,
        direction: Direction::E,
        damage_state: DamageState::Damaged,
        bridge_group_id: 1,
    };
    bs.test_seed_anchor_span(span);
    bs.test_seed_cell(
        11,
        10,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(1),
            damage_state: DamageState::Damaged,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
    sim.bridge_state = Some(bs);

    let terrain = sim
        .resolved_terrain
        .as_mut()
        .expect("bridge fallback tests require resolved terrain");
    terrain.cell_mut(12, 10).unwrap().bridge_facts.raw_flags = BRIDGE_FLAG_DESTROYED_OR_RAMP;
    let anchor = terrain.cell_mut(11, 10).unwrap();
    anchor.bridge_facts.raw_flags = 0;
    anchor.bridge_facts.ramp_tile = Some(BridgeRampTile {
        kind: BridgeRampKind::Middle1,
        relative_tile_index: 7,
        height_byte: 4,
    });
}

fn seed_terminal_overlay_with_fallback_trap(sim: &mut Simulation, overlay_byte: u8) {
    seed_hut_fallback_bridgehead_layout(sim);
    sim.bridge_state.as_mut().unwrap().test_seed_cell(
        10,
        10,
        BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 4,
            bridge_group_id: Some(2),
            damage_state: DamageState::Destroyed,
            axis: Some(Axis::EW),
            role: BridgeCellRole::Body,
            anchor_span_id: None,
            overlay_byte,
            damaged_variant: false,
            bridgehead_anchor_class: crate::sim::bridge_state::BridgeheadAnchorClass::Variant0,
        },
    );
}

fn seed_stock_high_cabhut_no_overlay_fallback_fixture(sim: &mut Simulation) {
    // Derived from stock high-bridge CABHUT/no-overlay placements such as
    // loose:Barrel.mmx and multimd.mix:bridgegap.map.
    seed_hut_fallback_bridgehead_layout(sim);
}

fn seed_stock_low_cabhut_no_overlay_fallback_fixture(sim: &mut Simulation) {
    // Derived from stock low-bridge CABHUT/no-overlay placements such as
    // loose:Carville.mmx, loose:Hills.mmx, and multimd.mix:xcarville.map.
    seed_hut_pure_bridgehead_fallback_layout(sim);
}

fn seed_stock_no_starter_cabhut_no_overlay_fixture(sim: &mut Simulation) {
    // Derived from stock CABHUT/no-overlay placements with no nearby 0x100/0x400
    // fallback starter, including MULTI.MIX:mp24t2.map and multimd.mix:xnorest.map.
    seed_hut_fallback_bridgehead_layout(sim);
    let terrain = sim
        .resolved_terrain
        .as_mut()
        .expect("bridge fallback tests require resolved terrain");
    let starter = terrain.cell_mut(12, 10).unwrap();
    starter.bridge_facts.raw_flags = 0;
    starter.bridge_facts.anchor = None;
}

fn step(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) -> TickResult {
    let due = sim.take_due_commands();
    sim.advance_tick(&due, Some(rules), heights, None, None, 67)
}

fn advance_pending_c4_to_detonation(
    sim: &mut Simulation,
    rules: &RuleSet,
    heights: &BTreeMap<(u16, u16), u8>,
) -> bool {
    let mut bridge_state_changed_seen = false;
    for _ in 0..(rules.c4_delay_ticks as u64 + 1) {
        let result = step(sim, rules, heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
    }
    bridge_state_changed_seen
}

fn advance_until_c4_claim(
    sim: &mut Simulation,
    rules: &RuleSet,
    heights: &BTreeMap<(u16, u16), u8>,
    target_id: u64,
) -> u64 {
    // SEAL/Tanya at Speed=4 covers ~10 lep/tick (gamemd-faithful), so a
    // one-cell enter (256 leptons) takes ~26 ticks; 32 leaves headroom.
    for _ in 0..32 {
        step(sim, rules, heights);
        if let Some(pending) = sim
            .entities
            .get(target_id)
            .and_then(|b| b.pending_c4_detonation)
        {
            return pending.plant_start_tick;
        }
    }
    panic!("C4 plant was not claimed after entering the target building cell");
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
    for &(rx, ry) in ENGINEER_REPAIR_STRIP_CELLS {
        let cell = bs.cell(rx, ry).unwrap();
        assert!(
            (0xCD..=0xD0).contains(&cell.overlay_byte),
            "cell ({rx},{ry}) overlay={:#04X} must be repaired high-bridge healthy overlay",
            cell.overlay_byte
        );
        assert!(
            matches!(cell.damage_state, DamageState::Destroyed),
            "cell ({rx},{ry}) keeps stale destroyed damage byte like gamemd"
        );
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
fn consecutive_engineers_second_bridge_repair_waits_for_next_tick() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer_a = spawn_engineer(&mut sim, 10, 10);
    let engineer_b = spawn_engineer(&mut sim, 10, 11);
    sim.entities.get_mut(engineer_a).unwrap().capture_target = Some(cabhut);
    sim.entities.get_mut(engineer_b).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer_a).is_none());
    assert!(
        sim.entities.get(engineer_b).is_some(),
        "live LogicClass vector iteration skips the immediate successor after engineer A removes itself"
    );
    let repair_events = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. }))
        .count();
    assert_eq!(repair_events, 1, "only the first engineer emits this tick");
    assert_eq!(sim.radar_events.len(), 1);
    assert_eq!(
        sim.radar_events.iter().next().map(|e| e.event_type),
        Some(crate::sim::radar::RadarEventType::BridgeRepaired)
    );
    assert!(
        !sim.radar_terrain_dirty_cells.is_empty(),
        "destroyed-anchor repair cells must propagate to minimap terrain dirty state"
    );
    assert_eq!(sim.radar_terrain_dirty_generation, 1);

    let bs = sim.bridge_state.as_ref().unwrap();
    for &(rx, ry) in ENGINEER_REPAIR_STRIP_CELLS {
        let cell = bs.cell(rx, ry).unwrap();
        assert!((0xCD..=0xD0).contains(&cell.overlay_byte));
        assert!(bs.is_bridge_walkable(rx, ry));
    }

    step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer_b).is_none());
    let repair_events = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::BridgeRepaired { .. }))
        .count();
    assert_eq!(
        repair_events, 2,
        "the skipped immediate successor still triggers on the next scheduler pass"
    );
}

#[test]
fn nonconsecutive_engineers_both_repair_same_tick_with_radar_eva_gate() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer_a = spawn_engineer(&mut sim, 10, 10);
    let blocker = spawn_seal(&mut sim, 8, 8);
    let engineer_b = spawn_engineer(&mut sim, 10, 11);
    sim.entities.get_mut(engineer_a).unwrap().capture_target = Some(cabhut);
    sim.entities.get_mut(engineer_b).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    step(&mut sim, &rules, &heights);

    assert!(sim.entities.get(engineer_a).is_none());
    assert!(sim.entities.get(blocker).is_some());
    assert!(sim.entities.get(engineer_b).is_none());

    let bridge_repaired: Vec<_> = sim
        .sound_events
        .iter()
        .filter_map(|event| match event {
            SimSoundEvent::BridgeRepaired { eva_allowed, .. } => Some(*eva_allowed),
            _ => None,
        })
        .collect();
    assert_eq!(
        bridge_repaired,
        vec![true, false],
        "two nonconsecutive engineers emit, but only the enqueued radar event gates EVA"
    );
    assert_eq!(
        sim.radar_events.len(),
        1,
        "BridgeRepaired radar events dedup at the CABHUT cell"
    );
}

#[test]
fn capture_building_command_accepts_noncapturable_bridge_repair_hut() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let engineer = spawn_engineer(&mut sim, 10, 10);

    let accepted = sim.apply_command(
        "Americans",
        &Command::CaptureBuilding {
            engineer_id: engineer,
            target_building_id: cabhut,
        },
        Some(&rules),
        None,
        &heights,
    );

    assert!(accepted);
    assert_eq!(
        sim.entities.get(engineer).and_then(|e| e.capture_target),
        Some(cabhut)
    );
}

#[test]
fn engineer_bridge_repair_scan_uses_hut_cell_not_engineer_cell() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 8, 10);
    let engineer = spawn_engineer(&mut sim, 7, 10);
    sim.entities.get_mut(engineer).unwrap().capture_target = Some(cabhut);
    seed_destroyed_bridge(&mut sim);

    let result = step(&mut sim, &rules, &heights);

    assert!(
        result.bridge_state_changed,
        "hut-cell 5x5 scan should reach bridge cells that engineer-cell scan misses"
    );
    let bs = sim.bridge_state.as_ref().unwrap();
    assert!((0xCD..=0xD0).contains(&bs.cell(10, 9).unwrap().overlay_byte));
    assert!(bs.is_bridge_walkable(10, 9));
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
///   - not claim from the adjacent cell,
///   - claim after entering the CABHUT cell,
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
    sim.entities.get_mut(seal).unwrap().c4_plant = Some(crate::sim::components::C4PlantState {
        target_building_id: cabhut,
    });
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });

    // First tick: adjacency only issues the one-cell enter move. It must not
    // claim the marker until the SEAL's current cell resolves to the CABHUT.
    step(&mut sim, &rules, &heights);
    assert!(
        sim.entities
            .get(cabhut)
            .and_then(|b| b.pending_c4_detonation)
            .is_none(),
        "adjacent SEAL must not claim C4 before entering CABHUT"
    );
    let plant_start = advance_until_c4_claim(&mut sim, &rules, &heights, cabhut);

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
    assert!(
        hut.pending_c4_detonation.is_none(),
        "CABHUT pending C4 marker must clear after bridge dispatch"
    );

    // The hut branch must invoke the bounded gamemd CollapseBridge walker.
    // This synthetic one-column fixture collapses the local 3-cell footprint;
    // long bridges are covered separately by bridge_orchestrator tests and
    // must NOT be treated as full-span flood fills.
    let bs = sim.bridge_state.as_ref().unwrap();
    let destroyed_cells = BRIDGE_CELLS
        .iter()
        .filter(|&&(rx, ry)| {
            bs.cell(rx, ry)
                .is_some_and(|cell| matches!(cell.damage_state, DamageState::Destroyed))
        })
        .count();
    assert_eq!(
        destroyed_cells, 3,
        "CABHUT C4 detonation must collapse the bounded local footprint in this fixture; destroyed_cells={destroyed_cells}"
    );
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

#[test]
fn c4_on_cabhut_without_bridge_clears_pending_marker() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 10, 10);
    let cabhut_max_hp = sim.entities.get(cabhut).unwrap().health.current;
    sim.bridge_state = Some(BridgeRuntimeState::default());
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let mut bridge_state_changed_seen = false;
    for _ in 0..(rules.c4_delay_ticks as u64 + 1) {
        let result = step(&mut sim, &rules, &heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
    }

    let hut = sim.entities.get(cabhut).unwrap();
    assert_eq!(hut.health.current, cabhut_max_hp);
    assert!(!hut.dying);
    assert!(hut.pending_c4_detonation.is_none());
    assert!(
        !bridge_state_changed_seen,
        "no bridge evidence means no bridge-state change"
    );
}

#[test]
fn c4_on_invulnerable_cabhut_still_dispatches_bridge_and_clears_pending() {
    use crate::sim::superweapon::invulnerability::{InvulnKind, InvulnerabilityState};

    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 10, 10);
    let cabhut_max_hp = sim.entities.get(cabhut).unwrap().health.current;
    seed_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });
    sim.entities.get_mut(cabhut).unwrap().invulnerability = Some(InvulnerabilityState {
        start_frame: sim.tick as u32,
        duration_frames: rules.c4_delay_ticks + 20,
        kind: InvulnKind::IronCurtain,
    });

    let mut bridge_state_changed_seen = false;
    for _ in 0..(rules.c4_delay_ticks as u64 + 1) {
        let result = step(&mut sim, &rules, &heights);
        bridge_state_changed_seen |= result.bridge_state_changed;
    }

    let hut = sim.entities.get(cabhut).unwrap();
    assert_eq!(hut.health.current, cabhut_max_hp);
    assert!(!hut.dying);
    assert!(hut.pending_c4_detonation.is_none());
    assert!(bridge_state_changed_seen);
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(10, 10)
            .unwrap()
            .damage_state,
        DamageState::Destroyed
    ));
}

#[test]
fn c4_on_cabhut_bridgehead_fallback_collapses_bridge() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    let hut_hp = sim.entities.get(cabhut).unwrap().health.current;
    seed_hut_fallback_bridgehead_layout(&mut sim);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    let hut = sim.entities.get(cabhut).unwrap();
    assert_eq!(hut.health.current, hut_hp);
    assert!(!hut.dying);
    assert!(hut.pending_c4_detonation.is_none());
    assert!(bridge_state_changed_seen);
    let bs = sim.bridge_state.as_ref().unwrap();
    assert!(matches!(
        bs.cell(13, 10).unwrap().damage_state,
        DamageState::Destroyed
    ));
}

#[test]
fn c4_on_cabhut_pure_bridgehead_fallback_uses_opposite_anchor_offset() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    seed_hut_pure_bridgehead_fallback_layout(&mut sim);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(
        bridge_state_changed_seen,
        "pure 0x400 starter should resolve anchor two cells opposite the east scan"
    );
    let bs = sim.bridge_state.as_ref().unwrap();
    assert!(matches!(
        bs.cell(11, 10).unwrap().damage_state,
        DamageState::Destroyed
    ));
}

#[test]
fn c4_on_cabhut_fallback_rejects_anchor_or_direction_flags_alone() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    seed_hut_fallback_bridgehead_layout(&mut sim);
    let starter = sim
        .resolved_terrain
        .as_mut()
        .unwrap()
        .cell_mut(12, 10)
        .unwrap();
    starter.bridge_facts.raw_flags = BRIDGE_FLAG_ANCHOR_SELF | BRIDGE_FLAG_DIRECTION_ZERO;
    starter.bridge_facts.anchor = None;
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(
        !bridge_state_changed_seen,
        "0x80/0x800 alone must not trigger CABHUT no-overlay fallback"
    );
    let bs = sim.bridge_state.as_ref().unwrap();
    assert!(matches!(
        bs.cell(13, 10).unwrap().damage_state,
        DamageState::Damaged
    ));
}

#[test]
fn stock_high_cabhut_no_overlay_fallback_collapses_bridge() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    seed_stock_high_cabhut_no_overlay_fallback_fixture(&mut sim);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(bridge_state_changed_seen);
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(13, 10)
            .unwrap()
            .damage_state,
        DamageState::Destroyed
    ));
}

#[test]
fn stock_low_cabhut_no_overlay_fallback_collapses_bridge() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    seed_stock_low_cabhut_no_overlay_fallback_fixture(&mut sim);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(bridge_state_changed_seen);
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(11, 10)
            .unwrap()
            .damage_state,
        DamageState::Destroyed
    ));
}

#[test]
fn stock_cabhut_no_overlay_without_starter_is_noop() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 9, 10);
    seed_stock_no_starter_cabhut_no_overlay_fixture(&mut sim);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(!bridge_state_changed_seen);
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(13, 10)
            .unwrap()
            .damage_state,
        DamageState::Damaged
    ));
}

#[test]
fn c4_on_cabhut_low_overlay_collapses_low_bridge() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 10, 10);
    let hut_hp = sim.entities.get(cabhut).unwrap().health.current;
    seed_low_bridge_with_state(&mut sim, DamageState::Healthy { variant: 0 });
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    let hut = sim.entities.get(cabhut).unwrap();
    assert_eq!(hut.health.current, hut_hp);
    assert!(!hut.dying);
    assert!(hut.pending_c4_detonation.is_none());
    assert!(bridge_state_changed_seen);
    assert!(
        BRIDGE_CELLS.iter().any(|&(rx, ry)| matches!(
            sim.bridge_state
                .as_ref()
                .unwrap()
                .cell(rx, ry)
                .unwrap()
                .damage_state,
            DamageState::Destroyed
        )),
        "low overlay CABHUT dispatch must destroy at least one seeded bridge cell"
    );
}

#[test]
fn c4_on_cabhut_low_terminal_overlay_0x65_uses_overlay_first_scan() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 10, 10);
    seed_terminal_overlay_with_fallback_trap(&mut sim, 0x65);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(
        !bridge_state_changed_seen,
        "terminal overlay scan hit must not fall through to fallback trap"
    );
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(13, 10)
            .unwrap()
            .damage_state,
        DamageState::Damaged
    ));
}

#[test]
fn c4_on_cabhut_high_terminal_overlay_0xe8_uses_overlay_first_scan() {
    let (mut sim, rules, heights) = build_sim();
    let cabhut = spawn_cabhut(&mut sim, 9, 10);
    let seal = spawn_seal(&mut sim, 10, 10);
    seed_terminal_overlay_with_fallback_trap(&mut sim, 0xE8);
    sim.entities.get_mut(cabhut).unwrap().pending_c4_detonation = Some(PendingC4Detonation {
        plant_start_tick: sim.tick,
        attacker_id: seal,
    });

    let bridge_state_changed_seen = advance_pending_c4_to_detonation(&mut sim, &rules, &heights);

    assert!(
        !bridge_state_changed_seen,
        "terminal overlay scan hit must not fall through to fallback trap"
    );
    assert!(matches!(
        sim.bridge_state
            .as_ref()
            .unwrap()
            .cell(13, 10)
            .unwrap()
            .damage_state,
        DamageState::Damaged
    ));
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
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: false,
                has_bridge_deck: false,
                bridge_walkable: false,
                bridge_transition: false,
                bridge_deck_level: 0,
                bridge_layer: None,
                bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                tube_index: None,
                radar_left: [0, 0, 0],
                radar_right: [0, 0, 0],
                has_damaged_data: true,
                bridgehead_anchor_class_at_load: None,
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
    seed_isolated_anchor(
        &mut bs,
        (10, 10),
        1,
        DamageState::Healthy { variant: 0 },
        false,
    );
    seed_isolated_anchor(
        &mut bs,
        (11, 10),
        2,
        DamageState::Healthy { variant: 0 },
        false,
    );
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
    seed_isolated_anchor(
        &mut bs,
        (11, 10),
        2,
        DamageState::Healthy { variant: 0 },
        true,
    );
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
fn build_ns_bridge_with_bridgehead_for_dispatch() -> (
    crate::map::resolved_terrain::ResolvedTerrainGrid,
    BridgeRuntimeState,
) {
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
                is_wood_bridge_repair_tile: false,
                // level must be >= 4 so the HighStateMachine path matches.
                // Z-gate accepts impact_z within [level-1, level+1].
                level: 4,
                filled_clear: false,
                tileset_index: Some(0),
                land_type: 0,
                yr_cell_land_type: 0,
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
                base_land_type: 0,
                base_yr_cell_land_type: 0,
                base_terrain_class: Default::default(),
                base_speed_costs: Default::default(),
                build_blocked: false,
                has_bridge_deck: true,
                bridge_walkable: true,
                bridge_transition: false,
                bridge_deck_level: 4,
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
        let state_changed = crate::sim::world::bridge_orchestrator::apply_bridge_damage_events(
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
    // Anchor's bridgehead_anchor_class = AboutToFall (idempotent across
    // hits). Matches the reference engine's first-hit anchor-tile write
    // target — the most-damaged variant, 4th enum slot.
    assert_eq!(
        bs.cell(2, 2).unwrap().bridgehead_anchor_class,
        BridgeheadAnchorClass::AboutToFall,
        "anchor tile-class must transition to AboutToFall on first hit",
    );
    // Neither bridgehead nor anchor entered Destroyed.
    for cell in [bs.cell(2, 4).unwrap(), bs.cell(2, 2).unwrap()] {
        assert!(
            !matches!(cell.damage_state, DamageState::Destroyed),
            "no Destroyed cell from sustained bridgehead direct fire",
        );
    }
}
