//! Movement integration tests — verifies ground movement, repath behavior, blocked handling,
//! stuck recovery, and infantry sub-cell mechanics using minimal simulation setups.

use super::*;
use crate::map::entities::EntityCategory;
use crate::map::terrain;
use crate::sim::components::MovementTarget;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::test_interner;
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
use crate::sim::rng::SimRng;
use crate::util::fixed_math::{SimFixed, SIM_ZERO};

// --- Facing calculation tests ---
// Cell deltas map directly to screen-relative RA2 DirStruct values:
// 0=N, 64=E, 128=S, 192=W. +dx = east, +dy = south.

#[test]
fn test_facing_iso_north() {
    // (0,-1) = north on screen → facing 0.
    let f: u8 = facing_from_delta(0, -1);
    assert_eq!(f, 0, "North (0,-1) should be facing 0");
}

#[test]
fn test_facing_iso_east() {
    // (1,0) = east on screen → facing 64.
    let f: u8 = facing_from_delta(1, 0);
    assert_eq!(f, 64, "East (1,0) should be facing 64");
}

#[test]
fn test_facing_iso_south() {
    // (0,1) = south on screen → facing 128.
    let f: u8 = facing_from_delta(0, 1);
    assert_eq!(f, 128, "South (0,1) should be facing 128");
}

#[test]
fn test_facing_iso_west() {
    // (-1,0) = west on screen → facing 192.
    let f: u8 = facing_from_delta(-1, 0);
    assert_eq!(f, 192, "West (-1,0) should be facing 192");
}

#[test]
fn test_facing_iso_northeast() {
    // (1,-1) = NE on screen → facing 32.
    let f: u8 = facing_from_delta(1, -1);
    assert_eq!(f, 32, "NE (1,-1) should be facing 32");
}

#[test]
fn test_facing_iso_southeast() {
    // (1,1) = SE on screen → facing 96.
    let f: u8 = facing_from_delta(1, 1);
    assert_eq!(f, 96, "SE (1,1) should be facing 96");
}

#[test]
fn test_facing_zero_delta() {
    let f: u8 = facing_from_delta(0, 0);
    assert_eq!(f, 0, "Zero delta should default to facing 0 (north)");
}

// --- Movement tick tests ---

#[test]
fn test_tick_movement_advances_position() {
    let mut entities = EntityStore::new();

    // Create an entity at (2, 2) with a path to (5, 2).
    let path: Vec<(u16, u16)> = vec![(2, 2), (3, 2), (4, 2), (5, 2)];

    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 2, 2);
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 4],
        next_index: 1,
        speed: SimFixed::from_num(512), // 512 leptons/sec = 2 cells/sec.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    e.facing = 64;
    entities.insert(e);

    // Tick 500ms at 512 lep/s → 256 leptons = 1 cell → snap to (3,2).
    tick_movement(&mut entities, 500, &mut test_interner());

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.position.rx, 3);
    assert_eq!(entity.position.ry, 2);
    // Entity should still have MovementTarget (not at goal yet).
    assert!(entity.movement_target.is_some());
}

#[test]
fn test_tick_movement_removes_target_at_goal() {
    let mut entities = EntityStore::new();

    // 2-cell path: (0,0) → (1,0). Speed=10 means it finishes instantly.
    let path: Vec<(u16, u16)> = vec![(0, 0), (1, 0)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(2560), // 10 cells/sec in leptons.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    // Large tick to ensure we finish the path.
    tick_movement(&mut entities, 1000, &mut test_interner());

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.position.rx, 1);
    assert_eq!(entity.position.ry, 0);
    // MovementTarget should be removed.
    assert!(
        entity.movement_target.is_none(),
        "MovementTarget should be removed when path is complete"
    );
}

#[test]
fn test_tick_movement_partial_progress() {
    let mut entities = EntityStore::new();

    let path: Vec<(u16, u16)> = vec![(0, 0), (1, 0), (2, 0)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 3],
        next_index: 1,
        speed: SimFixed::from_num(512), // 512 lep/s = 2 cells/sec.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    // 250ms at 512 lep/s → 128 leptons traveled. sub_x starts at 128 (center),
    // moves to 256 which is the cell boundary — entity should cross to next cell.
    // Use 125ms instead: 512 * 0.125 = 64 leptons → sub_x = 128 + 64 = 192 (mid-cell).
    tick_movement(&mut entities, 125, &mut test_interner());

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        entity.position.rx, 0,
        "Should not have moved to next cell yet"
    );
    assert_eq!(entity.position.ry, 0);

    // sub_x should be ~192 (128 center + 64 leptons traveled).
    let sub_x_f32: f32 = entity.position.sub_x.to_num();
    assert!(
        (sub_x_f32 - 192.0).abs() < 2.0,
        "sub_x should be ~192, got {sub_x_f32}"
    );
}

#[test]
fn test_tick_movement_updates_screen_position() {
    let mut entities = EntityStore::new();

    let path: Vec<(u16, u16)> = vec![(5, 5), (6, 5)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 5, 5);
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(1280), // 5 cells/sec in leptons.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    e.facing = 64;
    entities.insert(e);

    tick_movement(&mut entities, 1000, &mut test_interner());

    let entity = entities.get(1).expect("entity exists");
    // lepton_to_screen = CoordsToClient(cell_center) = iso_to_screen + (30, 15).
    let (corner_sx, corner_sy): (f32, f32) = terrain::iso_to_screen(6, 5, 0);
    assert!((entity.position.screen_x - (corner_sx + 30.0)).abs() < 1.0);
    assert!((entity.position.screen_y - corner_sy).abs() < 1.0);
}

#[test]
fn test_tick_movement_updates_facing() {
    let mut entities = EntityStore::new();

    // Path goes east then south.
    let path: Vec<(u16, u16)> = vec![(0, 0), (1, 0), (1, 1)];
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    e.movement_target = Some(MovementTarget {
        path,
        path_layers: vec![MovementLayer::Ground; 3],
        next_index: 1,
        speed: SimFixed::from_num(1280), // 5 cells/sec in leptons.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    e.facing = 64; // Initially facing east.
    entities.insert(e);

    // Move to (1,0). Next cell is (1,1), delta (0,1) = south → facing 128.
    tick_movement(&mut entities, 300, &mut test_interner());

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.facing, 128, "Should face south after first step");
}

#[test]
fn test_issue_move_command_sets_path() {
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(20, 20);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 2, 3);
    entities.insert(e);

    let result: bool = issue_move_command(
        &mut entities,
        &grid,
        1,
        (7, 3),
        SimFixed::from_num(768), // 3 cells/sec × 256 = 768 leptons/sec.
        false,
        None,
        None,
        None,
        false,
    );
    assert!(result, "Should find a path on open grid");

    let entity = entities.get(1).expect("entity exists");
    let target = entity
        .movement_target
        .as_ref()
        .expect("should have MovementTarget");
    assert_eq!(*target.path.first().expect("non-empty"), (2, 3));
    assert_eq!(*target.path.last().expect("non-empty"), (7, 3));
    assert_eq!(target.next_index, 1);
    assert_eq!(target.speed, SimFixed::from_num(768));
}

#[test]
fn test_issue_move_command_no_path() {
    let mut entities = EntityStore::new();
    let mut grid: PathGrid = PathGrid::new(10, 10);

    // Block column 5 completely.
    for y in 0..10 {
        grid.set_blocked(5, y, true);
    }

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    entities.insert(e);

    let result: bool = issue_move_command(
        &mut entities,
        &grid,
        1,
        (9, 9),
        SimFixed::from_num(768),
        false,
        None,
        None,
        None,
        false,
    );
    assert!(!result, "Should fail with blocked path");
    let entity = entities.get(1).expect("entity exists");
    assert!(
        entity.movement_target.is_none(),
        "Should not have MovementTarget when no path found"
    );
}

#[test]
fn test_issue_move_command_queue_appends_waypoint_path() {
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(32, 32);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 2, 2);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (8, 2),
        SimFixed::from_num(768),
        false,
        None,
        None,
        None,
        false,
    ));
    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (12, 2),
        SimFixed::from_num(768),
        true,
        None,
        None,
        None,
        false,
    ));

    let entity = entities.get(1).expect("entity exists");
    let movement = entity
        .movement_target
        .as_ref()
        .expect("should keep movement target");
    assert_eq!(
        movement.path.last().copied(),
        Some((12, 2)),
        "Queued command should append final waypoint"
    );
    assert!(
        movement.path.len() > 7,
        "Queued command should extend path beyond initial destination"
    );
}

#[test]
fn test_tick_movement_repaths_when_next_cell_becomes_blocked() {
    let mut entities = EntityStore::new();
    let mut grid: PathGrid = PathGrid::new(8, 8);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (5, 1),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    // Simulate a dynamic blocker appearing on the immediate next step.
    grid.set_blocked(2, 1, true);

    // With blockage_path_delay_ticks=60, the entity must wait 60 ticks for
    // blocked_delay to expire before a repath is attempted. After a successful
    // repath, it needs additional ticks to travel the detour to (5,1).
    for _ in 0..80 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut OccupancyGrid::new(),
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (5, 1),
        "Entity should recover and reach destination after repath"
    );
}

#[test]
fn test_tick_movement_no_stacking_same_target_cell() {
    let mut entities = EntityStore::new();

    let mut e1 = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e1.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1)],
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(1024), // 4 cells/sec in leptons.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    e1.facing = 64;
    entities.insert(e1);

    let mut e2 = GameEntity::test_default(2, "HTNK", "Americans", 1, 2);
    e2.movement_target = Some(MovementTarget {
        path: vec![(1, 2), (2, 1)],
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(1024), // 4 cells/sec in leptons.
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SimFixed::from_num(-256),
        move_dir_len: SimFixed::from_num(362), // ~sqrt(256^2 + 256^2)
        ..Default::default()
    });
    e2.facing = 64;
    entities.insert(e2);

    tick_movement_with_grid(
        &mut entities,
        None,
        &Default::default(),
        &Default::default(),
        &mut OccupancyGrid::new(),
        &mut SimRng::new(0),
        1000,
        0,
        &mut test_interner(),
    );

    let ent1 = entities.get(1).expect("e1 exists");
    let ent2 = entities.get(2).expect("e2 exists");
    assert_eq!(
        (ent1.position.rx, ent1.position.ry),
        (2, 1),
        "first mover should claim destination"
    );
    assert_eq!(
        (ent2.position.rx, ent2.position.ry),
        (1, 2),
        "second mover should stay blocked"
    );
}

#[test]
fn test_repath_cooldown_prevents_thrashing_on_unrecoverable_block() {
    let mut entities = EntityStore::new();
    let mut grid: PathGrid = PathGrid::new(8, 8);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (5, 1),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    // Make the route truly unreachable — block the entire column 2 so no
    // detour exists. (The previous 3-cell block left rows 3-7 open.)
    for y in 0..8u16 {
        grid.set_blocked(2, y, true);
    }

    // Under binary-faithful semantics, the blocked entity repaths every tick
    // while movement_delay is 0: urgency=1 during the blocked_delay grace
    // period (first 60 ticks), then urgency=2 once blocked_delay hits 0.
    // path_stuck_counter decrements once per urgency=2 failure. With
    // path_stuck_init=10 and blocked_delay=60, the entity survives ~60 ticks
    // before its first urgency=2 attempt, then aborts after ~10 more u2
    // failures (each separated by another blocked_delay grace period).
    //
    // Run 61 ticks (up to and including the first urgency=2 failure). Verify
    // the entity is still alive and that movement_delay has been set by the
    // rate-limiter in try_repath_after_block.
    for _ in 0..61 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut OccupancyGrid::new(),
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }
    let entity = entities.get(1).expect("entity exists");
    let m1 = entity
        .movement_target
        .as_ref()
        .expect("movement target should still exist after 61 ticks");
    assert!(
        m1.movement_delay > 0,
        "movement_delay {} should be > 0 after failed repath",
        m1.movement_delay,
    );
    // path_stuck_counter should have started decrementing once urgency=2
    // failures began. With init=10 it must still be positive after one
    // escalated failure (or zero if we've already exhausted it).
    assert!(
        m1.path_stuck_counter < 10,
        "path_stuck_counter {} should have decremented after urgency=2 failure",
        m1.path_stuck_counter,
    );
}

#[test]
fn test_dynamic_occupancy_repath_routes_around_stationary_blocker() {
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(10, 10);

    // Stationary blocker at (3,4). Different owner so bump doesn't apply.
    let blocker = GameEntity::test_default(1, "HTNK", "Soviet", 3, 4);
    entities.insert(blocker);

    let mover = GameEntity::test_default(2, "HTNK", "Americans", 1, 4);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        2,
        (7, 4),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    // With blockage_path_delay_ticks=60, the mover must wait ~60 ticks after
    // hitting the occupied cell before a repath is attempted. After repath
    // succeeds, it needs additional ticks to travel the detour to (7,4).
    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let mut saw_repath_success = false;
    for _ in 0..80 {
        let stats = tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut occupancy,
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
        if stats.repath_successes > 0 {
            saw_repath_success = true;
        }
    }

    let entity = entities.get(2).expect("mover should still exist");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (7, 4),
        "Mover should reach destination by routing around occupied cell"
    );
    assert!(
        saw_repath_success,
        "Should perform at least one dynamic repath"
    );
}

#[test]
fn test_stuck_recovery_clears_unreachable_movement_target() {
    let mut entities = EntityStore::new();
    let mut grid: PathGrid = PathGrid::new(7, 7);
    for y in 0..7 {
        for x in 0..7 {
            if y != 3 {
                grid.set_blocked(x, y, true);
            }
        }
    }

    // Stationary building at (3,3). Buildings hard-block in entity_blocks BTreeSet.
    let mut blocker = GameEntity::test_default(1, "GAWALL", "Soviet", 3, 3);
    blocker.category = EntityCategory::Structure;
    entities.insert(blocker);

    let mover = GameEntity::test_default(2, "HTNK", "Americans", 1, 3);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        2,
        (5, 3),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    // path_stuck_counter starts at 10 (PATH_STUCK_INIT). Each failed repath
    // decrements it by 1 and resets blocked_delay to 60. With both
    // blocked_delay=60 and path_delay_ticks=9 counting down simultaneously,
    // each cycle takes ~61 ticks. 10 failed repaths × 61 ticks ≈ 612 ticks.
    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let mut recovered = false;
    for _ in 0..700 {
        let stats = tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut occupancy,
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
        if stats.stuck_recoveries > 0 {
            recovered = true;
            break;
        }
    }

    assert!(
        recovered,
        "Stuck recovery should trigger for permanent deadlock"
    );
    let entity = entities.get(2).expect("mover exists");
    assert!(
        entity.movement_target.is_none(),
        "MovementTarget should be removed after stuck recovery"
    );
    assert_ne!(
        (entity.position.rx, entity.position.ry),
        (5, 3),
        "Stuck recovery should stop before unreachable destination"
    );
}

#[test]
fn test_movement_tick_stats_report_blocked_attempts() {
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(8, 8);

    // Stationary blocker at (2,2) owned by a different house so bump won't trigger.
    let blocker = GameEntity::test_default(1, "HTNK", "Soviets", 2, 2);
    entities.insert(blocker);

    let mover = GameEntity::test_default(2, "HTNK", "Americans", 1, 2);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        2,
        (4, 2),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let stats = tick_movement_with_grid(
        &mut entities,
        Some(&grid),
        &Default::default(),
        &Default::default(),
        &mut occupancy,
        &mut SimRng::new(0),
        250,
        0,
        &mut test_interner(),
    );
    assert_eq!(stats.movers_total, 1);
    assert_eq!(stats.moved_steps, 0);
    assert_eq!(stats.blocked_attempts, 1);
}

#[test]
fn test_friendly_scatter_issues_move_command() {
    // A friendly stationary blocker should receive a scatter movement
    // command — the blocker walks away instead of being teleported.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(8, 8);

    // Stationary friendly blocker at (2,2).
    let blocker = GameEntity::test_default(1, "HTNK", "Americans", 2, 2);
    entities.insert(blocker);

    let mover = GameEntity::test_default(2, "HTNK", "Americans", 1, 2);
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        2,
        (4, 2),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let stats = tick_movement_with_grid(
        &mut entities,
        Some(&grid),
        &Default::default(),
        &Default::default(),
        &mut occupancy,
        &mut SimRng::new(0),
        250,
        0,
        &mut test_interner(),
    );
    assert_eq!(stats.movers_total, 1);
    // Scatter succeeded: blocker was given a movement command.
    assert_eq!(stats.scatter_successes, 1);
    // Blocker should still be at (2,2) but now has a movement_target
    // (it walks away on subsequent ticks, not teleported).
    let bl = entities.get(1).expect("blocker exists");
    assert!(
        bl.movement_target.is_some(),
        "Blocker should have a scatter movement command"
    );
    assert_eq!(
        (bl.position.rx, bl.position.ry),
        (2, 2),
        "Blocker position unchanged this tick — walks next tick"
    );
}

// --- Friendly-passable pathfinding tests ---

#[test]
fn test_friendly_passable_moving_unit_not_blocked() {
    // A moving friendly unit should NOT appear in the entity block set.
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::movement::bump_crush;

    let mut entities = EntityStore::new();
    let _grid = PathGrid::new(10, 10);

    // Unit A: stationary friendly at (3, 0).
    let a = GameEntity::test_default(1, "HTNK", "Americans", 3, 0);
    entities.insert(a);

    // Unit B: moving friendly at (4, 0) — has a movement target.
    let mut b = GameEntity::test_default(2, "HTNK", "Americans", 4, 0);
    b.movement_target = Some(MovementTarget {
        path: vec![(4, 0), (5, 0), (6, 0)],
        path_layers: vec![MovementLayer::Ground; 3],
        next_index: 1,
        speed: SimFixed::from_num(1024),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(b);

    let alliances = HouseAllianceMap::new();
    let (blocks, _penalty) = bump_crush::build_entity_block_set(
        &entities,
        "Americans",
        &alliances,
        &mut test_interner(),
        None,
    );

    // Stationary friendly at (3,0) is now soft-blocked (code 6, cost 8x) in
    // entity_block_map, not in the hard-block BTreeSet.
    assert!(
        !blocks.contains(&(3, 0)),
        "Stationary friendly should be soft-blocked, not hard-blocked"
    );
    assert!(
        _penalty.contains_key(
            crate::sim::movement::locomotor::MovementLayer::Ground,
            &(3, 0)
        ),
        "Stationary friendly should be in entity_block_map"
    );
    assert_eq!(
        _penalty
            .get(
                crate::sim::movement::locomotor::MovementLayer::Ground,
                &(3, 0)
            )
            .expect("ground stationary friendly soft blocker")
            .cost_code,
        6,
        "Stationary friendly should have cost_code 6"
    );
    // Moving friendly at (4,0) should be in entity_block_map with code 2.
    assert!(
        !blocks.contains(&(4, 0)),
        "Moving friendly should be passable"
    );
    assert!(
        _penalty.contains_key(
            crate::sim::movement::locomotor::MovementLayer::Ground,
            &(4, 0)
        ),
        "Moving friendly should be in entity_block_map"
    );
    assert_eq!(
        _penalty
            .get(
                crate::sim::movement::locomotor::MovementLayer::Ground,
                &(4, 0)
            )
            .expect("ground moving friendly soft blocker")
            .cost_code,
        2,
        "Moving friendly should have cost_code 2"
    );
}

#[test]
fn test_enemy_unit_always_blocks_even_when_moving() {
    use crate::map::houses::HouseAllianceMap;
    use crate::sim::movement::bump_crush;

    let mut entities = EntityStore::new();

    // Enemy unit moving at (3, 0).
    let mut enemy = GameEntity::test_default(1, "HTNK", "Russians", 3, 0);
    enemy.movement_target = Some(MovementTarget {
        path: vec![(3, 0), (4, 0)],
        path_layers: vec![MovementLayer::Ground; 2],
        next_index: 1,
        speed: SimFixed::from_num(1024),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(enemy);

    let alliances = HouseAllianceMap::new();
    let (blocks, _penalty) = bump_crush::build_entity_block_set(
        &entities,
        "Americans",
        &alliances,
        &mut test_interner(),
        None,
    );

    // Enemy at (3,0) is now soft-blocked (code 5, cost 20x) in entity_block_map,
    // not in the hard-block BTreeSet.
    assert!(
        !blocks.contains(&(3, 0)),
        "Enemy should be soft-blocked, not hard-blocked"
    );
    assert!(
        _penalty.contains_key(
            crate::sim::movement::locomotor::MovementLayer::Ground,
            &(3, 0)
        ),
        "Enemy should be in entity_block_map"
    );
    assert_eq!(
        _penalty
            .get(
                crate::sim::movement::locomotor::MovementLayer::Ground,
                &(3, 0)
            )
            .expect("ground enemy soft blocker")
            .cost_code,
        5,
        "Enemy should have cost_code 5"
    );
}

#[test]
fn test_friendly_passable_path_goes_through_moving_friendly() {
    // Unit should be able to pathfind THROUGH a moving friendly's cell.
    use crate::sim::pathfinding::find_path_with_costs;
    use std::collections::BTreeSet;

    let grid = PathGrid::new(10, 3);
    // Only block (3,1) — force path through row 0.
    let mut blocks: BTreeSet<(u16, u16)> = BTreeSet::new();
    // (3,0) has a moving friendly — NOT in blocks.
    // (3,1) is a stationary friendly — in blocks.
    blocks.insert((3, 1));

    let path = find_path_with_costs(
        &grid,
        (0, 0),
        (6, 0),
        None,
        Some(&blocks),
        None,
        None,
        None,
        0,
        false,
    );
    assert!(
        path.is_some(),
        "Should find path through moving-friendly cell"
    );
    let path = path.unwrap();
    // Path can go through (3,0) since it's not blocked (moving friendly).
    assert_eq!(path.last(), Some(&(6, 0)));
}

// --- 24-step path segmentation tests ---

#[test]
fn test_short_path_no_truncation() {
    // A 5-step path (well under 24) should be delivered intact.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(32, 32);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (5, 0),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    let entity = entities.get(1).expect("entity exists");
    let target = entity.movement_target.as_ref().expect("has target");
    assert_eq!(
        target.path.len(),
        6,
        "5-step path = 6 entries (start + 5 moves)"
    );
    assert_eq!(target.final_goal, Some((5, 0)));
}

#[test]
fn test_long_path_truncated_to_24_steps() {
    // A path longer than 24 steps should be truncated to 25 entries.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(50, 1);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (40, 0),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    let entity = entities.get(1).expect("entity exists");
    let target = entity.movement_target.as_ref().expect("has target");
    // Path truncated: 24 steps + start = 25 entries.
    assert_eq!(
        target.path.len(),
        25,
        "Long path should be truncated to 25 entries"
    );
    assert_eq!(target.path[0], (0, 0), "Path starts at origin");
    assert_eq!(target.path[24], (24, 0), "Path ends at 24th step");
    assert_eq!(target.final_goal, Some((40, 0)), "Final goal preserved");
}

#[test]
fn test_segment_exhaustion_triggers_auto_repath() {
    // Walk a truncated 24-step segment, verify auto-repath continues to final goal.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(50, 1);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (30, 0),
        SimFixed::from_num(15360), // Very fast — finishes segment quickly.
        false,
        None,
        None,
        None,
        false,
    ));

    // Tick enough times to exhaust the first 24-step segment and auto-repath.
    for _ in 0..30 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut OccupancyGrid::new(),
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (30, 0),
        "Entity should reach final destination via auto-repath"
    );
    assert!(
        entity.movement_target.is_none(),
        "Movement should be complete"
    );
}

#[test]
fn test_exact_24_step_path_no_repath_needed() {
    // A path of exactly 24 steps should complete without needing auto-repath.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(50, 1);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 0);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (24, 0),
        SimFixed::from_num(15360),
        false,
        None,
        None,
        None,
        false,
    ));

    let entity = entities.get(1).expect("entity exists");
    let target = entity.movement_target.as_ref().expect("has target");
    assert_eq!(target.path.len(), 25, "24-step path = 25 entries");

    // Walk the full path.
    for _ in 0..20 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut OccupancyGrid::new(),
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (24, 0),
        "Should reach destination"
    );
    assert!(entity.movement_target.is_none(), "Movement should be done");
}

#[test]
fn test_auto_repath_fails_entity_stops() {
    // If auto-repath fails (goal unreachable after segment), entity should stop.
    let mut entities = EntityStore::new();
    let mut grid: PathGrid = PathGrid::new(50, 3);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 1);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (40, 1),
        SimFixed::from_num(15360),
        false,
        None,
        None,
        None,
        false,
    ));

    // After the path is issued, block column 25 completely so repath fails.
    for y in 0..3 {
        grid.set_blocked(25, y, true);
    }

    // Tick enough to exhaust the first segment (reaches cell 24) and attempt repath.
    for _ in 0..30 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut OccupancyGrid::new(),
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
    }

    let entity = entities.get(1).expect("entity exists");
    // Entity should have stopped — either at segment end or earlier.
    assert!(
        entity.movement_target.is_none(),
        "Movement should be cleared when repath fails"
    );
    assert!(
        entity.position.rx <= 24,
        "Entity should not pass the blocked column"
    );
}

#[test]
fn test_blocked_repath_uses_final_goal_not_segment_end() {
    // When blocked mid-segment, repath should target final_goal, not segment end.
    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(50, 5);

    let e = GameEntity::test_default(1, "HTNK", "Americans", 0, 2);
    entities.insert(e);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (40, 2),
        SimFixed::from_num(1024),
        false,
        None,
        None,
        None,
        false,
    ));

    let entity = entities.get(1).expect("entity exists");
    let target = entity.movement_target.as_ref().expect("has target");
    assert_eq!(target.final_goal, Some((40, 2)));
    // The segment path ends at (24, 2), but final_goal is (40, 2).
    assert_eq!(target.path.last(), Some(&(24, 2)));
}

/// Build a minimal Drive LocomotorState for layered-pathfinding tests. Required
/// because the layered A* branch in find_move_path is only entered when the
/// mover has a Drive/Walk/Mech locomotor; `test_default` leaves locomotor=None.
fn make_drive_loco_for_test() -> crate::sim::movement::locomotor::LocomotorState {
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::SIM_ONE;
    LocomotorState {
        kind: LocomotorKind::Drive,
        layer: MovementLayer::Ground,
        phase: GroundMovePhase::Idle,
        air_phase: AirMovePhase::Landed,
        speed_multiplier: SIM_ONE,
        speed_fraction: SIM_ONE,
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
        jumpjet_turn_rate: 0,
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

#[test]
fn test_initial_layered_path_avoids_friendly_building_footprint() {
    // A friendly Drive-locomotor unit ordered across a 2x2 friendly building
    // foundation must plan a path that does NOT visit any foundation cell on
    // the FIRST attempt — gamemd's Can_Enter_Cell returns code 7 (impassable)
    // for unrelated allied buildings, so the layered A* must hard-block them.
    use crate::sim::production::building_footprint_cells;
    use std::collections::BTreeSet;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(15, 15);

    // 2x2 friendly building anchored at (5,5) — covers (5,5), (6,5), (5,6), (6,6).
    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(5, 5, "2x2", &[], &[])
        .into_iter()
        .collect();
    let mut blocks = BTreeSet::new();
    blocks.extend(foundation.iter().copied());

    // Mover at (1,5), goal at (10,5) — straight east through the foundation.
    let mut mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 5);
    mover.locomotor = Some(make_drive_loco_for_test());
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (10, 5),
        SimFixed::from_num(1024),
        false,         // queue
        None,          // terrain_costs
        Some(&blocks), // entity_blocks
        None,          // entity_block_map
        false,         // mover_is_crusher
    ));

    let entity = entities.get(1).expect("mover exists");
    let target = entity
        .movement_target
        .as_ref()
        .expect("initial path was planned");

    for &cell in &target.path {
        assert!(
            !foundation.contains(&cell),
            "Initial path visited foundation cell {:?} — layered A* did not see \
             ground_blocks/bridge_blocks on the first plan. Path: {:?}",
            cell,
            target.path,
        );
    }
    assert_eq!(target.path.first().copied(), Some((1, 5)));
    assert_eq!(target.path.last().copied(), Some((10, 5)));
}

#[test]
fn test_queued_append_layered_path_avoids_friendly_building_footprint() {
    // Issue an initial move, then a queued (queue=true) move that crosses a
    // 2x2 friendly building. The appended portion must avoid the foundation.
    use crate::sim::production::building_footprint_cells;
    use std::collections::BTreeSet;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(15, 15);

    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(5, 5, "2x2", &[], &[])
        .into_iter()
        .collect();
    let mut blocks = BTreeSet::new();
    blocks.extend(foundation.iter().copied());

    // Mover at (1,5). First move to (3,5) (no obstacle). Second move queued
    // to (10,5) — appended portion crosses the foundation.
    let mut mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 5);
    mover.locomotor = Some(make_drive_loco_for_test());
    entities.insert(mover);

    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (3, 5),
        SimFixed::from_num(1024),
        false, // queue=false (initial)
        None,
        Some(&blocks),
        None,
        false,
    ));
    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (10, 5),
        SimFixed::from_num(1024),
        true, // queue=true (append)
        None,
        Some(&blocks),
        None,
        false,
    ));

    let entity = entities.get(1).expect("mover exists");
    let target = entity.movement_target.as_ref().expect("queued path exists");

    for &cell in &target.path {
        assert!(
            !foundation.contains(&cell),
            "Queued append path visited foundation cell {:?}. Path: {:?}",
            cell,
            target.path,
        );
    }
    assert_eq!(target.path.first().copied(), Some((1, 5)));
    assert_eq!(target.path.last().copied(), Some((10, 5)));
}

#[test]
fn test_segment_exhaustion_repath_avoids_friendly_building_footprint() {
    // A long path with a 2x2 friendly building at cell 30 (beyond the first
    // 24-step segment). The initial segment doesn't see the foundation; the
    // auto-repath at segment exhaustion must avoid it.
    //
    // The auto-repath at movement_tick.rs:166 builds its hard-block set freshly
    // from EntityStore via bump_crush::build_entity_block_set, NOT from the
    // entity_blocks arg passed to issue_move_command. So the foundation must be
    // present as Structure entities in the store. Without rules wired into the
    // test, build_entity_block_set adds the anchor cell of each Structure to
    // mover_entity_blocks, so we insert one Structure per foundation cell.
    use crate::sim::movement::tick_movement_with_grid;
    use crate::sim::production::building_footprint_cells;
    use std::collections::BTreeSet;

    let mut entities = EntityStore::new();
    let grid: PathGrid = PathGrid::new(45, 5);

    // 2x2 building footprint at (30,2): covers (30,2), (31,2), (30,3), (31,3).
    let foundation: BTreeSet<(u16, u16)> = building_footprint_cells(30, 2, "2x2", &[], &[])
        .into_iter()
        .collect();

    // Insert one Structure entity per foundation cell so build_entity_block_set
    // (called inside tick_movement_with_grid) puts every cell in mover_entity_blocks.
    for (i, &(rx, ry)) in foundation.iter().enumerate() {
        let mut blocker = GameEntity::test_default(100 + i as u64, "GAWALL", "Americans", rx, ry);
        blocker.category = EntityCategory::Structure;
        entities.insert(blocker);
    }

    let mut mover = GameEntity::test_default(1, "HTNK", "Americans", 1, 2);
    mover.locomotor = Some(make_drive_loco_for_test());
    entities.insert(mover);

    // entity_blocks=None at command time → initial path goes straight east,
    // truncated to 24 steps (1,2)..(24,2) which doesn't reach the foundation.
    // The post-segment-exhaustion auto-repath is what must route around it.
    assert!(issue_move_command(
        &mut entities,
        &grid,
        1,
        (40, 2),
        SimFixed::from_num(15360), // very fast — exhausts segment quickly
        false,
        None,
        None,
        None,
        false,
    ));

    // Tick until the first segment is exhausted and auto-repath fires. Capture
    // the first path whose first cell is not (1,2) — that is the post-auto-repath
    // segment, planned by the call site this test pins.
    let mut occupancy = OccupancyGrid::rebuild(&entities);
    let mut post_repath_path: Option<Vec<(u16, u16)>> = None;
    for _ in 0..40 {
        tick_movement_with_grid(
            &mut entities,
            Some(&grid),
            &Default::default(),
            &Default::default(),
            &mut occupancy,
            &mut SimRng::new(0),
            250,
            0,
            &mut test_interner(),
        );
        if post_repath_path.is_none() {
            if let Some(t) = entities.get(1).and_then(|e| e.movement_target.as_ref()) {
                if t.path.first().is_some_and(|&c| c != (1, 2)) {
                    post_repath_path = Some(t.path.clone());
                }
            }
        }
    }

    let path = post_repath_path
        .expect("auto-repath at segment exhaustion must fire and produce a new path");
    for &cell in &path {
        assert!(
            !foundation.contains(&cell),
            "Post-segment-exhaustion repath visited foundation cell {:?}. Path: {:?}",
            cell,
            path,
        );
    }
}

// ============================================================================
// Bridge on_bridge timing integration tests (Plan: 2026-05-11 G2 fix).
// Pin: predicate fires at Ramp→Body exactly, clears at Ramp→Ground exactly,
// no anticipatory BridgeOccupancy pre-claim.
// ============================================================================

use crate::map::houses::HouseAllianceMap;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::sim::components::BridgeOccupancy;
use crate::sim::movement::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState};
use crate::sim::movement::tick_movement_with_grid;
use crate::sim::pathfinding::{terrain_cost::TerrainCostGrid, PathGrid};
use crate::util::fixed_math::SIM_ONE;
use std::collections::BTreeMap;

fn make_drive_loco(layer: MovementLayer) -> LocomotorState {
    LocomotorState {
        kind: LocomotorKind::Drive,
        layer,
        phase: GroundMovePhase::Idle,
        air_phase: AirMovePhase::Landed,
        speed_multiplier: SIM_ONE,
        speed_fraction: SIM_ONE,
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

fn tick_bridge(
    entities: &mut EntityStore,
    grid: &PathGrid,
    occupancy: &mut OccupancyGrid,
    rng: &mut SimRng,
    interner: &mut crate::sim::intern::StringInterner,
    ms: u32,
) {
    let costs: BTreeMap<SpeedType, TerrainCostGrid> = BTreeMap::new();
    let alliances = HouseAllianceMap::new();
    let _ = tick_movement_with_grid(
        entities,
        Some(grid),
        &costs,
        &alliances,
        occupancy,
        rng,
        ms,
        0,
        interner,
    );
}

#[test]
fn on_bridge_fires_at_ramp_to_body_only() {
    // Layout: (1,1) is a ramp/bridgehead at raw h=4 (bridge_walkable, transition=true).
    // (2,1) is a body cell at raw h=0 (bridge_walkable, no transition). Effective deck = 4.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 4, true, true);
    grid.set_cell_for_test(2, 1, 0, true, false);

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;
    e.on_bridge = false;
    e.locomotor = Some(make_drive_loco(MovementLayer::Bridge));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1)],
        path_layers: vec![MovementLayer::Bridge, MovementLayer::Bridge],
        next_index: 1,
        speed: SimFixed::from_num(512),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(
        1,
        1,
        1,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    assert!(
        !entities.get(1).unwrap().on_bridge,
        "pre-tick: on_bridge must be false on ramp"
    );

    // 512 lep/sec * 500ms = 256 leptons = exactly one cell jump (1,1)→(2,1).
    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );

    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (2, 1));
    assert!(
        entity.on_bridge,
        "on_bridge must fire on Ramp→Body transition"
    );
    assert_eq!(
        entity
            .bridge_occupancy
            .as_ref()
            .expect("BridgeOccupancy set on Enter")
            .deck_level,
        4
    );
    let cell = occupancy.get(2, 1).expect("destination occupancy");
    assert_eq!(
        cell.count_on(MovementLayer::Bridge),
        1,
        "Ramp->Body inserts into bridge object list after on_bridge projects true"
    );
    assert_eq!(cell.count_on(MovementLayer::Ground), 0);
}

#[test]
fn on_bridge_clears_at_ramp_to_ground_only() {
    // body (1,1) raw h=0 bridge_walkable; ramp (2,1) raw h=4 bridge_walkable+transition;
    // ground (3,1) raw h=4 no bridge_walkable.
    // Path: (1,1)→(2,1)→(3,1). on_bridge stays true through the ramp tick and clears
    // on Ramp→Ground.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 0, true, false); // body
    grid.set_cell_for_test(2, 1, 4, true, true); // ramp
    grid.set_cell_for_test(3, 1, 4, false, false); // ground at h=4

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;
    e.on_bridge = true;
    e.bridge_occupancy = Some(BridgeOccupancy { deck_level: 4 });
    e.locomotor = Some(make_drive_loco(MovementLayer::Bridge));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1), (3, 1)],
        // body→ramp goes on Ground layer per is_at_bridge_level
        // (parent at deck=4, neighbor h=4 → diff=0 < 2 → not at bridge level).
        path_layers: vec![
            MovementLayer::Bridge,
            MovementLayer::Ground,
            MovementLayer::Ground,
        ],
        next_index: 1,
        speed: SimFixed::from_num(512),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(
        1,
        1,
        1,
        MovementLayer::Bridge,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    // Tick 1: body → ramp. on_bridge must STAY true (predicate NoChange).
    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );
    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (2, 1),
        "after tick 1: at ramp"
    );
    assert!(
        entity.on_bridge,
        "after tick 1 (on ramp): on_bridge must stay true"
    );
    let ramp_cell = occupancy.get(2, 1).expect("ramp occupancy");
    assert_eq!(
        ramp_cell.count_on(MovementLayer::Bridge),
        1,
        "Body->Ramp keeps bridge object list while on_bridge remains true"
    );
    assert_eq!(ramp_cell.count_on(MovementLayer::Ground), 0);

    // Tick 2: ramp → ground. on_bridge must CLEAR (predicate Exit).
    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );
    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (3, 1),
        "after tick 2: at ground"
    );
    assert!(!entity.on_bridge, "after Ramp→Ground: on_bridge must clear");
    assert!(
        entity.bridge_occupancy.is_none(),
        "after Exit: BridgeOccupancy must be None"
    );
    let ground_cell = occupancy.get(3, 1).expect("ground occupancy");
    assert_eq!(ground_cell.count_on(MovementLayer::Ground), 1);
    assert_eq!(ground_cell.count_on(MovementLayer::Bridge), 0);
}

#[test]
fn no_bridge_lookahead_pre_claim() {
    // Regression: the deleted apply_bridge_lookahead_if_needed must not have crept
    // back via another path. BridgeOccupancy must NOT be set before the unit
    // physically crosses onto a body cell.
    // ground (1,1) h=4 → ramp (2,1) raw h=4 bridge_walkable+transition → body
    // (3,1) raw h=0 bridge_walkable.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 4, false, false);
    grid.set_cell_for_test(2, 1, 4, true, true);
    grid.set_cell_for_test(3, 1, 0, true, false);

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;
    e.on_bridge = false;
    e.bridge_occupancy = None;
    e.locomotor = Some(make_drive_loco(MovementLayer::Ground));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1), (3, 1)],
        path_layers: vec![
            MovementLayer::Ground,
            MovementLayer::Bridge,
            MovementLayer::Bridge,
        ],
        next_index: 1,
        speed: SimFixed::from_num(512),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(
        1,
        1,
        1,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    assert!(
        entities.get(1).unwrap().bridge_occupancy.is_none(),
        "pre-tick: no pre-claim"
    );

    // Tick 1: ground → ramp. Predicate NoChange (src.bridge_walkable=false; entry
    // would need src_h-4 = dst_h: src=4, dst=4 → no. Exit needs src.bridge_walkable;
    // it's false → no). BridgeOccupancy stays None.
    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );
    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (2, 1),
        "after tick 1: at ramp"
    );
    assert!(
        entity.bridge_occupancy.is_none(),
        "regression: BridgeOccupancy must NOT be pre-claimed on the ramp"
    );
    let ramp_cell = occupancy.get(2, 1).expect("ramp occupancy");
    assert_eq!(
        ramp_cell.count_on(MovementLayer::Ground),
        1,
        "Ground->Ramp stays ground object list while on_bridge remains false"
    );
    assert_eq!(ramp_cell.count_on(MovementLayer::Bridge), 0);

    // Tick 2: ramp → body. Now predicate fires Enter (src.bridge_walkable=true,
    // dst.bridge_walkable=true, dst_h(0) == src_h(4)-4 → entry fires).
    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );
    let entity = entities.get(1).expect("entity exists");
    assert_eq!(
        (entity.position.rx, entity.position.ry),
        (3, 1),
        "after tick 2: on body"
    );
    assert!(entity.on_bridge, "after Ramp→Body: on_bridge must be true");
    assert_eq!(
        entity
            .bridge_occupancy
            .as_ref()
            .expect("set on Enter")
            .deck_level,
        4
    );
    let body_cell = occupancy.get(3, 1).expect("body occupancy");
    assert_eq!(body_cell.count_on(MovementLayer::Bridge), 1);
    assert_eq!(body_cell.count_on(MovementLayer::Ground), 0);
}

#[test]
fn multi_crossing_preserves_first_bridge_set_update() {
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 4, true, true);
    grid.set_cell_for_test(2, 1, 0, true, false);
    grid.set_cell_for_test(3, 1, 0, true, false);

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;
    e.on_bridge = false;
    e.locomotor = Some(make_drive_loco(MovementLayer::Bridge));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1), (3, 1)],
        path_layers: vec![
            MovementLayer::Bridge,
            MovementLayer::Bridge,
            MovementLayer::Bridge,
        ],
        next_index: 1,
        speed: SimFixed::from_num(1024),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(
        1,
        1,
        1,
        MovementLayer::Ground,
        None,
        CellListInsertion::PrependNonBuilding,
    );
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    tick_bridge(
        &mut entities,
        &grid,
        &mut occupancy,
        &mut rng,
        &mut interner,
        500,
    );

    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (3, 1));
    assert!(
        entity.on_bridge,
        "first Ramp->Body Set must survive later Unchanged"
    );
    assert_eq!(
        entity
            .bridge_occupancy
            .as_ref()
            .expect("BridgeOccupancy set")
            .deck_level,
        4
    );
    let cell = occupancy.get(3, 1).expect("final occupancy");
    assert_eq!(cell.count_on(MovementLayer::Bridge), 1);
    assert_eq!(cell.count_on(MovementLayer::Ground), 0);
}
