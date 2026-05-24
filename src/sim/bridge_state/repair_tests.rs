//! Tests for bridge repair state transitions.

use super::*;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::rng::SimRng;

fn seeded_rng() -> SimRng {
    SimRng::new(0x4242_4242_4242_4242)
}

/// Minimal flat 20x20 terrain grid: all cells share tile_id=0,
/// has_damaged_data=false. Sufficient context for repair tests — the
/// flood-fill writer's gate fails on has_damaged_data=false, so repair
/// flood-fill calls become no-ops (terrain is required by signature only).
fn repair_test_terrain() -> ResolvedTerrainGrid {
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
                allows_tiberium: false,
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

/// Build a 5-cell test span along Y (NS bridge) with all cells seeded to
/// `state`. Anchor at (10,10); body cells at (10,11), (10,12), (10,13);
/// "−direction" cell at (10,9). Slot 5 = None.
fn build_single_ns_span(state: DamageState) -> BridgeRuntimeState {
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

    for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
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
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }
    bs
}

#[test]
fn repair_destroyed_main_deck_sets_zones_dirty_and_radar() {
    let mut bs = build_single_ns_span(DamageState::Destroyed);
    let mut rng = seeded_rng();
    let scan = vec![(10, 10)];
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(outcome.zones_dirty, "main-deck repair must set zones_dirty");
    assert_eq!(outcome.radar_cells.len(), 5);
    assert_eq!(outcome.repaired_cells, 5);
    for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
        let s = bs.cell(rx, ry).unwrap().damage_state;
        assert!(
            matches!(s, DamageState::Healthy { .. }),
            "cell ({rx},{ry}) state = {s:?}"
        );
    }
}

#[test]
fn repair_damaged_main_deck_zones_dirty_but_no_radar() {
    let mut bs = build_single_ns_span(DamageState::Damaged);
    let mut rng = seeded_rng();
    let scan = vec![(10, 10)];
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(outcome.zones_dirty);
    assert!(
        outcome.radar_cells.is_empty(),
        "Damaged → Healthy does NOT mark radar dirty"
    );
    assert_eq!(outcome.repaired_cells, 5);
}

#[test]
fn repair_bridgehead_no_rng_no_zones_no_radar() {
    let mut bs = build_single_ns_span(DamageState::Damaged);
    for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
        bs.cell_mut(rx, ry).unwrap().role = BridgeCellRole::Bridgehead;
    }
    let mut rng = seeded_rng();
    let rng_state_before = rng.state();
    let scan = vec![(10, 10)];
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(
        !outcome.zones_dirty,
        "bridgehead-only repair must NOT set zones_dirty"
    );
    assert!(outcome.radar_cells.is_empty());
    assert_eq!(outcome.repaired_cells, 5);
    assert_eq!(
        rng.state(),
        rng_state_before,
        "bridgehead repair must not draw RNG"
    );
    for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
        assert!(matches!(
            bs.cell(rx, ry).unwrap().damage_state,
            DamageState::Healthy { variant: 0 }
        ));
    }
}

#[test]
fn repair_healthy_cell_is_noop() {
    let mut bs = build_single_ns_span(DamageState::Healthy { variant: 3 });
    let mut rng = seeded_rng();
    let rng_state_before = rng.state();
    let scan = vec![(10, 10)];
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(!outcome.zones_dirty);
    assert!(outcome.radar_cells.is_empty());
    assert_eq!(outcome.repaired_cells, 0);
    assert_eq!(
        rng.state(),
        rng_state_before,
        "healthy cells must not draw RNG"
    );
    assert!(matches!(
        bs.cell(10, 10).unwrap().damage_state,
        DamageState::Healthy { variant: 3 }
    ));
}

#[test]
fn repair_partial_collapse_to_healthy() {
    let mut bs = build_single_ns_span(DamageState::PartialCollapseA);
    let mut rng = seeded_rng();
    let scan = vec![(10, 10)];
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(outcome.zones_dirty);
    assert!(
        outcome.radar_cells.is_empty(),
        "PartialCollapse → Healthy does NOT mark radar dirty"
    );
    assert_eq!(outcome.repaired_cells, 5);
}

#[test]
fn repair_no_bridge_in_scan_empty_outcome() {
    let mut bs = BridgeRuntimeState::default();
    let mut rng = seeded_rng();
    let rng_state_before = rng.state();
    let scan: Vec<(u16, u16)> = (0..25).map(|i| (i, 0)).collect();
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(!outcome.zones_dirty);
    assert!(outcome.radar_cells.is_empty());
    assert_eq!(outcome.repaired_cells, 0);
    assert_eq!(rng.state(), rng_state_before);
}

#[test]
fn repair_determinism_same_seed_same_variants() {
    let mut bs_a = build_single_ns_span(DamageState::Destroyed);
    let mut bs_b = build_single_ns_span(DamageState::Destroyed);
    let mut rng_a = seeded_rng();
    let mut rng_b = seeded_rng();
    let scan = vec![(10, 10)];
    bs_a.body_cell_repair_state(&scan, &mut rng_a, &repair_test_terrain());
    bs_b.body_cell_repair_state(&scan, &mut rng_b, &repair_test_terrain());
    for &(rx, ry) in &[(10, 9), (10, 10), (10, 11), (10, 12), (10, 13)] {
        let va = bs_a.cell(rx, ry).unwrap().damage_state;
        let vb = bs_b.cell(rx, ry).unwrap().damage_state;
        assert_eq!(va, vb, "variant divergence at ({rx},{ry})");
    }
}

/// Re-derive the pinned variants by replaying the exact RNG draw
/// sequence: 5 cells, all main-deck damaged → 5 sequential
/// `next_range_u32(4)` calls (matches `body_cell_repair_state`'s draw).
fn compute_pinned_variants(rng: &mut SimRng) -> Vec<u8> {
    (0..5).map(|_| rng.next_range_u32(4) as u8).collect()
}

#[test]
fn repair_strip_iteration_order_pin() {
    // Locks the RNG-draw sequence for a known 5-cell destroyed span.
    // If anyone reorders `AnchorSpan.cells` or changes the iteration
    // pattern, this test fails with diff-friendly output.
    let mut bs = build_single_ns_span(DamageState::Destroyed);
    let mut rng = seeded_rng();
    let scan = vec![(10, 10)];
    bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());

    // Variants captured in slot order from the span definition:
    //   slot 0 = (10,10), slot 1 = (10,11), slot 2 = (10,12),
    //   slot 3 = (10,13), slot 4 = (10,9), slot 5 = None.
    let variants: Vec<u8> = [(10, 10), (10, 11), (10, 12), (10, 13), (10, 9)]
        .iter()
        .map(|&(rx, ry)| match bs.cell(rx, ry).unwrap().damage_state {
            DamageState::Healthy { variant } => variant,
            other => panic!("non-Healthy after repair: {other:?}"),
        })
        .collect();

    let expected = compute_pinned_variants(&mut seeded_rng());
    assert_eq!(
        variants, expected,
        "RNG-draw iteration order changed — verify span.cells slot order"
    );

    for v in &variants {
        assert!(
            *v <= 3,
            "repair walker wrote variant {v} — must be 0..=3 (healthy)"
        );
    }
}

#[test]
fn repair_two_overlapping_spans_processed_in_btreeset_order() {
    // Span 1 already present from build_single_ns_span. Span 2 at
    // (9..=13, 11) with anchor (10,11). They share cell (10,11): span 1's
    // body cell + span 2's anchor.
    let mut bs = build_single_ns_span(DamageState::Destroyed);
    let span2 = AnchorSpan {
        id: 2,
        anchor: (10, 11),
        cells: [
            Some((10, 11)),
            Some((11, 11)),
            Some((12, 11)),
            Some((13, 11)),
            Some((9, 11)),
            None,
        ],
        axis: Axis::NS,
        direction: Direction::S,
        damage_state: DamageState::Destroyed,
        bridge_group_id: 1,
    };
    bs.test_seed_anchor_span(span2);
    for &(rx, ry) in &[(9, 11), (11, 11), (12, 11), (13, 11)] {
        bs.test_seed_cell(
            rx,
            ry,
            BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Destroyed,
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: Some(2),
                overlay_byte: 0,
                damaged_variant: false,
                bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
            },
        );
    }
    let mut rng = seeded_rng();
    let scan: Vec<(u16, u16)> = cells_in_5x5_scan((10, 10)).collect();
    let outcome = bs.body_cell_repair_state(&scan, &mut rng, &repair_test_terrain());
    assert!(outcome.zones_dirty);
    // Span 1: 5 cells; Span 2: 5 cells minus the shared (10,11) cell
    // (already Healthy after span 1's pass) = 4. Total 9.
    assert_eq!(
        outcome.repaired_cells, 9,
        "overlap cell (10,11) repaired once by span 1, skipped by span 2"
    );
}
