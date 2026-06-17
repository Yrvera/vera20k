//! End-to-end determinism + snapshot tests for the smudge system.
//!
//! These tests verify the determinism contract for SmudgeGrid: identical seeds
//! plus identical SmudgeGrid mutations must produce identical state hashes,
//! state hashes must be stable across `advance_tick` calls, and a SmudgeGrid
//! survives a snapshot round-trip with state-hash equality preserved.
//!
//! Approach: seed SmudgeGrid directly via the `#[cfg(test)]` `test_force_set`
//! helper rather than driving the full combat -> anim -> smudge_dispatch
//! pipeline. The combat path requires a complete RuleSet, terrain grid,
//! occupancy, overlay registry, etc.; the contract being verified here is
//! purely about determinism + serialization, so the simpler seeding is enough.

use std::collections::BTreeMap;

use super::Simulation;
use crate::sim::smudge_grid::{SmudgeCell, SmudgeGrid};
use crate::sim::snapshot::GameSnapshot;

/// Build a Simulation with a populated SmudgeGrid seeded with a fixed pattern.
///
/// Same seed -> same starting state. The pattern uses small but nontrivial
/// data so the hash is sensitive to all SmudgeCell fields (type_id,
/// footprint_origin, frame_offset).
fn build_test_sim_with_seed(seed: u64) -> Simulation {
    let mut sim = Simulation::with_seed(seed);
    let mut grid = SmudgeGrid::new(16, 16);

    // A 1x1 crater + a 2x2 footprint laid out across 4 cells.
    grid.test_force_set(
        2,
        3,
        SmudgeCell {
            type_id: Some(0),
            footprint_origin: Some((2, 3)),
            frame_offset: 0,
        },
    );
    // 2x2 footprint at origin (5,5).
    let w: u8 = 2;
    for dy in 0..2u16 {
        for dx in 0..2u16 {
            grid.test_force_set(
                5 + dx,
                5 + dy,
                SmudgeCell {
                    type_id: Some(1),
                    footprint_origin: Some((5, 5)),
                    frame_offset: (dx as u8) + (dy as u8) * w,
                },
            );
        }
    }

    sim.smudge_grid = Some(grid);
    sim
}

/// Run `advance_tick` n times with empty inputs.
fn advance_n(sim: &mut Simulation, n: u32) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..n {
        sim.advance_tick(&[], None, &height_map, None, None, 33);
    }
}

#[test]
fn same_seed_same_smudge_state_yields_same_hash() {
    let sim_a = build_test_sim_with_seed(42);
    let sim_b = build_test_sim_with_seed(42);
    assert_eq!(
        sim_a.state_hash(),
        sim_b.state_hash(),
        "same seed + same SmudgeGrid pattern must hash identically",
    );
}

#[test]
fn different_smudge_state_yields_different_hash() {
    // Same seed, but one sim has an extra smudge cell. The hash must diverge —
    // this proves SmudgeGrid is actually contributing to state_hash.
    let mut sim_a = build_test_sim_with_seed(42);
    let sim_b = build_test_sim_with_seed(42);
    if let Some(grid) = sim_a.smudge_grid.as_mut() {
        grid.test_force_set(
            10,
            10,
            SmudgeCell {
                type_id: Some(0),
                footprint_origin: Some((10, 10)),
                frame_offset: 0,
            },
        );
    }
    assert_ne!(
        sim_a.state_hash(),
        sim_b.state_hash(),
        "an extra smudge cell must change the state hash",
    );
}

#[test]
fn smudge_state_hash_stable_across_advance_tick() {
    // SmudgeGrid is mutated only by combat events (none fired here). Across
    // 100 empty ticks, the smudge contribution to the hash must stay constant
    // and both sims must agree at every tick. This verifies determinism of
    // the smudge-hashing path under the normal advance loop.
    let mut sim_a = build_test_sim_with_seed(42);
    let mut sim_b = build_test_sim_with_seed(42);

    for tick in 0..100 {
        advance_n(&mut sim_a, 1);
        advance_n(&mut sim_b, 1);
        assert_eq!(
            sim_a.state_hash(),
            sim_b.state_hash(),
            "state_hash diverged at tick {} (sim_a.tick={}, sim_b.tick={})",
            tick,
            sim_a.session.tick,
            sim_b.session.tick,
        );
    }

    // Sanity: every seeded cell is still present (advance_tick didn't clobber
    // SmudgeGrid in either sim).
    let occ_a = sim_a
        .smudge_grid
        .as_ref()
        .map(|g| g.iter_occupied().count())
        .unwrap_or(0);
    let occ_b = sim_b
        .smudge_grid
        .as_ref()
        .map(|g| g.iter_occupied().count())
        .unwrap_or(0);
    assert_eq!(occ_a, occ_b);
    assert_eq!(occ_a, 5, "expected 1 (1x1) + 4 (2x2) = 5 occupied cells");
}

#[test]
fn smudge_grid_survives_snapshot_roundtrip() {
    // Build, advance, save, load, advance further on both — final hashes
    // must match.
    let mut sim_orig = build_test_sim_with_seed(7);
    advance_n(&mut sim_orig, 25);

    let hash_at_save = sim_orig.state_hash();
    let bytes = GameSnapshot::save(&sim_orig, 0, 0, "smudge_roundtrip_test", 0);
    let snap = GameSnapshot::load(&bytes).expect("snapshot load must succeed");
    let mut sim_restored = snap.sim;

    assert_eq!(
        sim_restored.state_hash(),
        hash_at_save,
        "restored sim must hash identically to the source at save time",
    );

    // Verify the SmudgeGrid is preserved cell-for-cell.
    let orig_cells: Vec<_> = sim_orig
        .smudge_grid
        .as_ref()
        .unwrap()
        .iter_occupied()
        .map(|(rx, ry, c)| (rx, ry, c.type_id, c.footprint_origin, c.frame_offset))
        .collect();
    let restored_cells: Vec<_> = sim_restored
        .smudge_grid
        .as_ref()
        .unwrap()
        .iter_occupied()
        .map(|(rx, ry, c)| (rx, ry, c.type_id, c.footprint_origin, c.frame_offset))
        .collect();
    assert_eq!(
        orig_cells, restored_cells,
        "SmudgeGrid contents must round-trip through bincode unchanged",
    );

    // Continue both sims for another 25 ticks — both must reach the same
    // final hash, proving the snapshot didn't corrupt any state needed by
    // future ticks.
    advance_n(&mut sim_orig, 25);
    advance_n(&mut sim_restored, 25);
    assert_eq!(
        sim_orig.state_hash(),
        sim_restored.state_hash(),
        "original and restored sim must reach identical state after 25 more ticks",
    );
}
