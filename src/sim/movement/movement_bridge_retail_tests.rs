//! Retail-map high-bridge crossing check.
//!
//! The synthetic `PathGrid` fixtures elsewhere in this module answer "does the
//! resolver compute the right number". This module answers the different
//! question the bug report actually asks: on a **real retail map**, loaded
//! through the ordinary headless scenario funnel, does a ground vehicle ordered
//! across an intact high bridge stand on the deck for the whole span instead of
//! dropping to the riverbed underneath?
//!
//! The invariant asserted after every committed frame is the algebraic inverse
//! of `FootClass::Set_Height_On_Bridge` @ `0x005F5FA0` recorded by
//! `ObjectClass::GetHeight` @ `0x005F5F30`:
//!
//! ```text
//! position.z == GroundHeight(own cell) + (OnBridge ? 4 levels : 0)
//! ```
//!
//! Retail assets are not in the repo, so these are `#[ignore]`d and additionally
//! skip gracefully when no retail root resolves (the `RA2_DIR` → `config.toml`
//! order the rest of the tree uses). Run them with:
//!
//! ```text
//! cargo test -p vera20k --lib sim::movement::movement_bridge_retail_tests -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use super::movement_occupancy::BRIDGE_DECK_LEVEL_DELTA;
use crate::headless_scenario::{self, SIM_TICK_MS};
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::house_state::HouseState;
use crate::sim::pathfinding::PathGrid;
use crate::sim::runtime::SimRuntime;
use crate::sim::world::TickLane;

/// Fixed seed so a failing run is reproducible.
const SEED: u32 = 0x0B21_D6E5;
/// Enough committed frames for a ~15-cell drive at stock tank speed.
const MAX_TICKS: u64 = 2000;

/// Retail install root, or `None` to skip.
fn retail_dir() -> Option<PathBuf> {
    let dir = match std::env::var("RA2_DIR") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => match crate::util::config::GameConfig::load() {
            Ok(config) => config.paths.ra2_dir,
            Err(_) => return None,
        },
    };
    dir.is_dir().then_some(dir)
}

/// One intact high-bridge crossing discovered from live path-grid facts.
#[derive(Debug, Clone)]
struct HighBridgeSpan {
    /// Off-bridge cell the drive starts on, one step before the first deck cell.
    approach_a: (u16, u16),
    /// Off-bridge cell on the far side, one step past the last deck cell.
    approach_b: (u16, u16),
    /// Every structural deck cell between them, in travel order.
    deck: Vec<(u16, u16)>,
    /// Terrain level *under* the deck (the riverbed the tank must never sit on).
    deck_terrain_level: u8,
    /// Terrain level of both approach cells; equals `deck_terrain_level + 4`.
    approach_level: u8,
    step: (i32, i32),
}

const STEPS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

fn offset(cell: (u16, u16), step: (i32, i32)) -> Option<(u16, u16)> {
    let x = i32::from(cell.0) + step.0;
    let y = i32::from(cell.1) + step.1;
    (x >= 0 && y >= 0).then(|| (x as u16, y as u16))
}

/// Find the longest straight run of structural deck cells that is entered and
/// left through cells exactly four levels above the deck's own terrain — i.e.
/// the geometry the `Enter`/`Exit` predicate (`0x004B2561`-`0x004B259B`) fires on.
fn find_high_bridge_span(grid: &PathGrid) -> Option<HighBridgeSpan> {
    let mut best: Option<HighBridgeSpan> = None;
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let Some(first) = grid.cell(x, y) else {
                continue;
            };
            if !first.bridge_structural {
                continue;
            }
            let deck_terrain_level = first.ground_level;
            let Some(approach_level) = deck_terrain_level.checked_add(4) else {
                continue;
            };
            for step in STEPS {
                // The cell behind the first deck cell must be a real approach:
                // off-bridge, walkable, and exactly four levels up.
                let back = (-step.0, -step.1);
                let Some(approach_a) = offset((x, y), back) else {
                    continue;
                };
                let Some(cell_a) = grid.cell(approach_a.0, approach_a.1) else {
                    continue;
                };
                if cell_a.bridge_structural
                    || !cell_a.ground_walkable
                    || cell_a.ground_level != approach_level
                {
                    continue;
                }
                let mut deck = vec![(x, y)];
                let mut cursor = (x, y);
                loop {
                    let Some(next) = offset(cursor, step) else {
                        break;
                    };
                    let Some(cell) = grid.cell(next.0, next.1) else {
                        break;
                    };
                    if !cell.bridge_structural || cell.ground_level != deck_terrain_level {
                        break;
                    }
                    deck.push(next);
                    cursor = next;
                }
                let Some(approach_b) = offset(cursor, step) else {
                    continue;
                };
                let Some(cell_b) = grid.cell(approach_b.0, approach_b.1) else {
                    continue;
                };
                if cell_b.bridge_structural
                    || !cell_b.ground_walkable
                    || cell_b.ground_level != approach_level
                {
                    continue;
                }
                let candidate = HighBridgeSpan {
                    approach_a,
                    approach_b,
                    deck_terrain_level,
                    approach_level,
                    step,
                    deck,
                };
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.deck.len() > current.deck.len())
                {
                    best = Some(candidate);
                }
            }
        }
    }
    best
}

/// One committed frame of observed mover state.
#[derive(Debug, Clone, Copy)]
struct TickRow {
    tick: u64,
    cell: (u16, u16),
    z: u8,
    on_bridge: bool,
    occupancy_deck: Option<u8>,
    terrain_level: u8,
    structural: bool,
    bridge_walkable: bool,
    stored_deck_level: u8,
    /// The mover's live A* layer — the input the pre-fix height path keyed off.
    loco_layer: crate::sim::movement::locomotor::MovementLayer,
    /// What the pre-fix code would have written into `position.z` on this frame:
    /// `dst_cell.effective_cell_z_for_layer(layer)`, i.e. the stored per-cell deck
    /// value read through the `bridge_walkable` Option with an `unwrap_or(ground)`
    /// fallback. Recorded so a passing run says whether it actually discriminates
    /// the fix or merely fails to contradict it.
    prefix_z: u8,
}

impl TickRow {
    /// `ObjectClass::GetHeight` @ `0x005F5F30` inverted: the only Z the native
    /// model can produce for this cell and this OnBridge state.
    fn expected_z(&self) -> i16 {
        i16::from(self.terrain_level as i8)
            + if self.on_bridge {
                BRIDGE_DECK_LEVEL_DELTA
            } else {
                0
            }
    }

    fn holds_invariant(&self) -> bool {
        i16::from(self.z as i8) == self.expected_z()
    }
}

fn print_inventory(grid: &PathGrid, span: &HighBridgeSpan) {
    let structural = (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .filter(|(x, y)| grid.cell(*x, *y).is_some_and(|c| c.bridge_structural))
        .count();
    println!(
        "path grid {}x{}, {structural} structural high-bridge cell(s)",
        grid.width(),
        grid.height()
    );
    println!(
        "span: approach_a {:?} (level {}) -> {} deck cell(s) at terrain level {} -> approach_b {:?} (level {}), step {:?}",
        span.approach_a,
        span.approach_level,
        span.deck.len(),
        span.deck_terrain_level,
        span.approach_b,
        span.approach_level,
        span.step,
    );
    for cell in &span.deck {
        let facts = grid.cell(cell.0, cell.1).expect("deck cell in bounds");
        println!(
            "  deck {:?}: ground_level={} bridge_deck_level={} structural={} walkable={} transition={}",
            cell,
            facts.ground_level,
            facts.bridge_deck_level,
            facts.bridge_structural,
            facts.bridge_walkable,
            facts.transition,
        );
    }
}

/// Report why the order never became a `MovementTarget`, so a harness failure is
/// distinguishable from a height defect.
fn diagnose_rejected_order(
    scenario: &mut crate::headless_scenario::HeadlessScenario,
    owner_name: &str,
    entity_id: u64,
    start_cell: (u16, u16),
    span: &HighBridgeSpan,
) {
    use crate::sim::pathfinding::{AStarOptions, astar_search, find_path};

    let grid = scenario
        .sim()
        .path_grid_snapshot()
        .expect("navigation published");
    println!("--- order rejected; diagnosing ---");
    println!(
        "owner match: {}",
        scenario.sim().entity_owned_by_id(owner_name, entity_id)
    );
    println!(
        "order_actor_admits: {}",
        scenario.sim().order_actor_admits(entity_id)
    );
    println!(
        "flat A* {start_cell:?} -> {:?}: {:?} node(s)",
        span.approach_b,
        find_path(&grid, start_cell, span.approach_b).map(|path| path.len())
    );
    let layered = astar_search(
        &grid,
        start_cell,
        crate::sim::movement::locomotor::MovementLayer::Ground,
        span.approach_b,
        &AStarOptions::default(),
    );
    match &layered {
        Some(steps) => {
            println!("layered A*: {} step(s)", steps.len());
            for step in steps.iter().take(30) {
                println!("   ({:3},{:3}) layer={:?}", step.rx, step.ry, step.layer);
            }
        }
        None => println!("layered A*: no path"),
    }

    // Ablation over the four production inputs the bare `astar_search` above did
    // not carry, to name which one refuses a route the raw search finds.
    {
        use super::PathfindingContext;
        use super::movement_path::find_move_path;
        use crate::rules::locomotor_type::MovementZone;
        use crate::sim::movement::locomotor::MovementLayer;

        let sim = scenario.sim();
        let entity = sim.entities().get(entity_id).expect("tank present");
        let speed_type = entity.locomotor.as_ref().map(|loco| loco.speed_type);
        let movement_zone = entity.locomotor.as_ref().map(|loco| loco.movement_zone);
        println!(
            "in_playfield={} speed_type={:?} movement_zone={:?}",
            entity.in_playfield, speed_type, movement_zone
        );
        let costs = speed_type.and_then(|st| sim.terrain_costs.get(&st));
        println!(
            "terrain cost grid for {speed_type:?}: {}, zone_grid: {}, resolved_terrain: {}, playfield_bounds: {:?}",
            costs.is_some(),
            sim.zone_grid.is_some(),
            sim.resolved_terrain.is_some(),
            sim.playfield_bounds.is_some(),
        );
        let (blocks, block_map) = crate::sim::movement::bump_crush::build_entity_block_set(
            sim.entities(),
            owner_name,
            &sim.house_alliances,
            &sim.interner,
            Some(&scenario.runtime.resources.rules),
        );
        println!("entity blocks: {} cell(s)", blocks.len());
        let on_span: Vec<&(u16, u16)> = blocks
            .iter()
            .filter(|cell| span.deck.contains(cell) || **cell == span.approach_b)
            .collect();
        println!("  blocks on the span or its far approach: {on_span:?}");

        // The two production inputs the plain block set does not carry: the live
        // vehicle occupation plane, folded into the same set, and the blocker
        // neighbour counts the edge-cost walk reads.
        let mut occupation_blocks = blocks.clone();
        occupation_blocks.extend(
            scenario
                .sim()
                .substrate
                .cell_occupation
                .occupied_cells_ignoring(MovementLayer::Ground, entity_id),
        );
        println!(
            "entity blocks + ground vehicle occupation: {} cell(s); on span: {:?}",
            occupation_blocks.len(),
            occupation_blocks
                .iter()
                .filter(|cell| span.deck.contains(cell)
                    || **cell == span.approach_b
                    || **cell == span.approach_a)
                .collect::<Vec<_>>()
        );
        let neighbor_counts =
            crate::sim::movement::bump_crush::build_blocker_neighbor_counts_with_overlays(
                sim.entities(),
                grid.width(),
                grid.height(),
                sim.resolved_terrain.as_ref(),
                sim.overlay_grid.as_ref(),
                Some(&scenario.runtime.resources.overlay_registry),
                &sim.interner,
                Some(&scenario.runtime.resources.rules),
            );

        for (label, use_zone, use_costs, use_terrain, use_bounds, use_blocks) in [
            ("bare", false, false, false, false, false),
            ("+zone", true, false, false, false, false),
            ("+costs", false, true, false, false, false),
            ("+terrain", false, false, true, false, false),
            ("+bounds", false, false, false, true, false),
            ("+blocks", false, false, false, false, true),
            ("all", true, true, true, true, true),
        ] {
            let ctx = PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: if use_zone {
                    sim.zone_grid.as_ref()
                } else {
                    None
                },
                resolved_terrain: if use_terrain {
                    sim.resolved_terrain.as_ref()
                } else {
                    None
                },
                playfield_bounds: if use_bounds {
                    sim.playfield_bounds
                } else {
                    None
                },
                blocker_neighbor_counts: None,
            };
            let block_ref = use_blocks.then_some(&blocks);
            let result = find_move_path(
                ctx,
                true,
                start_cell,
                MovementLayer::Ground,
                span.approach_b,
                if use_costs { costs } else { None },
                block_ref,
                block_ref,
                block_ref,
                movement_zone.unwrap_or(MovementZone::Normal),
                movement_zone,
                false,
                use_blocks.then_some(&block_map),
                0,
                false,
                false,
                true,
            );
            println!(
                "  find_move_path[{label}] -> {:?}",
                result.map(|(path, _)| path.len())
            );
        }

        // Exact production argument set.
        let production = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: sim.zone_grid.as_ref(),
                resolved_terrain: sim.resolved_terrain.as_ref(),
                playfield_bounds: sim.playfield_bounds,
                blocker_neighbor_counts: Some(&neighbor_counts),
            },
            true,
            start_cell,
            MovementLayer::Ground,
            span.approach_b,
            costs,
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            movement_zone.unwrap_or(MovementZone::Normal),
            movement_zone,
            false,
            Some(&block_map),
            0,
            true,
            false,
            true,
        );
        println!(
            "  find_move_path[production] -> {:?}",
            production.as_ref().map(|(path, _)| path.len())
        );

        // Same, minus one input at a time.
        let without_occupation = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: sim.zone_grid.as_ref(),
                resolved_terrain: sim.resolved_terrain.as_ref(),
                playfield_bounds: sim.playfield_bounds,
                blocker_neighbor_counts: Some(&neighbor_counts),
            },
            true,
            start_cell,
            MovementLayer::Ground,
            span.approach_b,
            costs,
            Some(&blocks),
            Some(&blocks),
            Some(&blocks),
            movement_zone.unwrap_or(MovementZone::Normal),
            movement_zone,
            false,
            Some(&block_map),
            0,
            true,
            false,
            true,
        );
        println!(
            "  find_move_path[production - occupation] -> {:?}",
            without_occupation.as_ref().map(|(path, _)| path.len())
        );
        let without_neighbors = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: sim.zone_grid.as_ref(),
                resolved_terrain: sim.resolved_terrain.as_ref(),
                playfield_bounds: sim.playfield_bounds,
                blocker_neighbor_counts: None,
            },
            true,
            start_cell,
            MovementLayer::Ground,
            span.approach_b,
            costs,
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            movement_zone.unwrap_or(MovementZone::Normal),
            movement_zone,
            false,
            Some(&block_map),
            0,
            true,
            false,
            true,
        );
        println!(
            "  find_move_path[production - neighbors] -> {:?}",
            without_neighbors.as_ref().map(|(path, _)| path.len())
        );
        let without_block_map = find_move_path(
            PathfindingContext {
                path_grid: Some(&grid),
                zone_grid: sim.zone_grid.as_ref(),
                resolved_terrain: sim.resolved_terrain.as_ref(),
                playfield_bounds: sim.playfield_bounds,
                blocker_neighbor_counts: Some(&neighbor_counts),
            },
            true,
            start_cell,
            MovementLayer::Ground,
            span.approach_b,
            costs,
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            Some(&occupation_blocks),
            movement_zone.unwrap_or(MovementZone::Normal),
            movement_zone,
            false,
            None,
            0,
            true,
            false,
            true,
        );
        println!(
            "  find_move_path[production - block_map] -> {:?}",
            without_block_map.as_ref().map(|(path, _)| path.len())
        );

        if let Some(zones) = sim
            .zone_grid
            .as_ref()
            .and_then(|grid| grid.map_for(movement_zone.unwrap_or(MovementZone::Normal)))
        {
            let show = |cell: (u16, u16)| {
                format!(
                    "{cell:?} ground={:?} bridge={:?}",
                    zones.zone_at(cell.0, cell.1, MovementLayer::Ground),
                    zones.zone_at(cell.0, cell.1, MovementLayer::Bridge)
                )
            };
            println!("  zones: approach_a {}", show(span.approach_a));
            for cell in span.deck.iter().take(4) {
                println!("         deck       {}", show(*cell));
            }
            println!("         approach_b {}", show(span.approach_b));
        }

        // How far along the span does the production search get before the gate
        // shuts? Read-only: nothing here mutates the simulation.
        println!("  production-config search to successive goals along the span:");
        let mut goals: Vec<(u16, u16)> = vec![span.approach_a];
        goals.extend(span.deck.iter().copied());
        goals.push(span.approach_b);
        for goal in goals {
            let reached = find_move_path(
                PathfindingContext {
                    path_grid: Some(&grid),
                    zone_grid: sim.zone_grid.as_ref(),
                    resolved_terrain: sim.resolved_terrain.as_ref(),
                    playfield_bounds: sim.playfield_bounds,
                    blocker_neighbor_counts: Some(&neighbor_counts),
                },
                true,
                start_cell,
                MovementLayer::Ground,
                goal,
                costs,
                Some(&occupation_blocks),
                Some(&occupation_blocks),
                Some(&occupation_blocks),
                movement_zone.unwrap_or(MovementZone::Normal),
                movement_zone,
                false,
                Some(&block_map),
                0,
                true,
                false,
                true,
            );
            println!(
                "    -> {goal:?}: {:?}",
                reached.as_ref().map(|(path, _)| path.len())
            );
        }
    }

    let SimRuntime {
        simulation,
        resources,
    } = &mut scenario.runtime;
    println!(
        "resolve_move_info: {:?}",
        simulation
            .resolve_move_info(entity_id, Some(&resources.rules))
            .is_some()
    );
    let applied = simulation.apply_command(
        owner_name,
        &Command::Move {
            entity_id,
            target_rx: span.approach_b.0,
            target_ry: span.approach_b.1,
            queue: false,
            group_id: None,
        },
        Some(&resources.rules),
        Some(&grid),
        &resources.height_map,
    );
    println!("direct apply_command(Move) -> {applied}");
    println!(
        "movement_target after direct apply: {:?}",
        simulation
            .entities()
            .get(entity_id)
            .and_then(|entity| entity.movement_target.as_ref())
            .map(|target| target.path.len())
    );
}

/// Issue an ordinary player `Command::Move` — the same entry point a right-click
/// goes through, with nothing disabled.
///
/// Every order this harness issues, the crossing and the control alike, goes
/// through here. A control move that used a widened search could pass while the
/// crossing was refused by a gate, which would report "the mover is fine, the
/// bridge is the problem" from two different code paths and prove neither.
fn issue_ordinary_move(
    scenario: &mut crate::headless_scenario::HeadlessScenario,
    owner_name: &str,
    entity_id: u64,
    target: (u16, u16),
) -> bool {
    let grid = scenario
        .sim()
        .path_grid_snapshot()
        .expect("navigation published");
    let SimRuntime {
        simulation,
        resources,
    } = &mut scenario.runtime;
    simulation.apply_command(
        owner_name,
        &Command::Move {
            entity_id,
            target_rx: target.0,
            target_ry: target.1,
            queue: false,
            group_id: None,
        },
        Some(&resources.rules),
        Some(&grid),
        &resources.height_map,
    )
}

/// Load a retail map, spawn a tank on one bridge approach, order it to the far
/// approach, and record `position.z` against the native height invariant after
/// every committed frame.
fn drive_across_high_bridge(map_file: &str, unit_type: &str, expect_crossing: bool) {
    let Some(retail) = retail_dir() else {
        eprintln!("SKIPPED: no retail root (set RA2_DIR or provide config.toml)");
        return;
    };
    // The move-issue path reports every refusal reason through `log::warn!`; without
    // a logger a rejected order is indistinguishable from a height defect.
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Warn)
        .try_init();

    let mut scenario = match headless_scenario::load(&retail, map_file, SEED) {
        Ok(scenario) => scenario,
        Err(error) => panic!("load {map_file}: {error}"),
    };
    println!(
        "loaded {map_file} ({}x{}, theater {})",
        scenario.sim().session.map_width,
        scenario.sim().session.map_height,
        scenario.map.header.theater,
    );

    let span = {
        let grid = scenario
            .sim()
            .path_grid()
            .expect("headless load publishes navigation");
        let Some(span) = find_high_bridge_span(grid) else {
            panic!(
                "{map_file} exposes no high-bridge span: no structural deck run is entered and \
                 left through cells exactly four levels above it"
            );
        };
        print_inventory(grid, &span);
        span
    };
    assert!(
        span.deck.len() >= 3,
        "a two-cell span proves nothing about mid-span behaviour; found {} deck cell(s)",
        span.deck.len()
    );

    // House setup. A headless load is a spectatorless map load, so the roster
    // may be empty; the tank needs an owner whose name the order path can match.
    // Every house is made passive for the run: the defeat scan would otherwise
    // resolve the match on the first frame and stop committing ticks.
    let owner_name = {
        let sim = &mut scenario.runtime.simulation;
        let existing: Vec<String> = sim
            .houses
            .keys()
            .map(|id| sim.interner.resolve(*id).to_string())
            .collect();
        println!("map roster houses: {existing:?}");
        let name = existing
            .iter()
            .find(|name| {
                !name.eq_ignore_ascii_case("Neutral") && !name.eq_ignore_ascii_case("Special")
            })
            .cloned()
            .unwrap_or_else(|| {
                let name = "Americans".to_string();
                let id = sim.interner.intern(&name);
                sim.houses
                    .insert(id, HouseState::new(id, 0, None, true, 10_000, 10));
                name
            });
        for house in sim.houses.values_mut() {
            house.multiplay_passive = true;
        }
        let id = sim.interner.get(&name).expect("owner interned");
        if sim.session.house_order.is_empty() {
            sim.session.house_order = vec![id];
        }
        name
    };
    println!("commanding house: {owner_name}");

    // Spawn on the near approach; fall back one cell further back if that cell
    // is already occupied by map-placed content.
    let start_candidates = [
        span.approach_a,
        offset(span.approach_a, (-span.step.0, -span.step.1)).unwrap_or(span.approach_a),
    ];
    let mut entity_id = None;
    let mut start_cell = span.approach_a;
    for candidate in start_candidates {
        let SimRuntime {
            simulation,
            resources,
        } = &mut scenario.runtime;
        if let Some(id) = simulation.spawn_object(
            unit_type,
            &owner_name,
            candidate.0,
            candidate.1,
            0,
            &resources.rules,
            &resources.height_map,
        ) {
            entity_id = Some(id);
            start_cell = candidate;
            break;
        }
    }
    let entity_id = entity_id.unwrap_or_else(|| {
        panic!("could not place a {unit_type} on either approach cell {start_candidates:?}")
    });
    // The headless funnel does not build the type-handle table (nothing it loads
    // reaches combat); an armed vehicle does, and combat asserts on it.
    {
        let SimRuntime {
            simulation,
            resources,
        } = &mut scenario.runtime;
        simulation.resolve_type_handles(&resources.rules);
    }
    {
        let entity = scenario
            .sim()
            .entities()
            .get(entity_id)
            .expect("spawned tank present");
        println!(
            "spawned {unit_type} id={entity_id} at {start_cell:?} z={} on_bridge={}",
            entity.position.z, entity.on_bridge
        );
    }

    // Order the crossing.
    let owner_id = scenario
        .sim()
        .interner
        .get(&owner_name)
        .expect("owner interned");
    let execute_tick = scenario.sim().session.tick + 1;
    let order = CommandEnvelope::new(
        owner_id,
        execute_tick,
        Command::Move {
            entity_id,
            target_rx: span.approach_b.0,
            target_ry: span.approach_b.1,
            queue: false,
            group_id: None,
        },
    );
    scenario
        .runtime
        .advance_frame(&[order], SIM_TICK_MS, TickLane::Ordinary);

    let ordered_path = scenario
        .sim()
        .entities()
        .get(entity_id)
        .and_then(|entity| entity.movement_target.as_ref())
        .map(|target| target.path.clone());
    match &ordered_path {
        Some(path) => println!(
            "move order accepted: {} node(s) {:?} -> {:?}",
            path.len(),
            path.first(),
            path.last()
        ),
        None => {
            // RATCHET. This branch used to re-issue the order with the
            // zone-hierarchy gate switched off, so a deck-height observation was
            // still possible while the order path itself stayed broken. That
            // fallback is deleted along with the defect it worked around
            // (cf91caa3): an ordinary, undisabled `Command::Move` is the only
            // route a player has, so it must succeed here or the run fails. A
            // harness that can route around a regression is not a ratchet.
            diagnose_rejected_order(&mut scenario, &owner_name, entity_id, start_cell, &span);
            panic!(
                "the ordinary Command::Move to {:?} was REFUSED — a player cannot cross this \
                 span at all. See the diagnosis above; the bridge-deck exemption from the \
                 zone-hierarchy corridor gate (sim::pathfinding::core) is the first thing to \
                 check.",
                span.approach_b
            );
        }
    }

    // Record every committed frame.
    let mut rows: Vec<TickRow> = Vec::new();
    let mut idle_frames = 0u32;
    for _ in 0..MAX_TICKS {
        scenario.tick();
        let sim = scenario.sim();
        let Some(entity) = sim.entities().get(entity_id) else {
            panic!("the tank vanished mid-crossing");
        };
        let cell = (entity.position.rx, entity.position.ry);
        let facts = sim
            .path_grid()
            .and_then(|grid| grid.cell(cell.0, cell.1))
            .copied();
        let Some(facts) = facts else {
            panic!("tank left the path grid at {cell:?}");
        };
        let loco_layer = entity.movement_layer_or_ground();
        rows.push(TickRow {
            tick: sim.session.tick,
            cell,
            z: entity.position.z,
            on_bridge: entity.on_bridge,
            occupancy_deck: entity.bridge_occupancy.map(|occ| occ.deck_level),
            terrain_level: facts.ground_level,
            structural: facts.bridge_structural,
            bridge_walkable: facts.bridge_walkable,
            stored_deck_level: facts.bridge_deck_level,
            loco_layer,
            prefix_z: facts.effective_cell_z_for_layer(loco_layer),
        });
        if entity.movement_target.is_none() {
            idle_frames += 1;
            if idle_frames >= 30 {
                break;
            }
        } else {
            idle_frames = 0;
        }
        if cell == span.approach_b {
            break;
        }
    }

    println!(
        "\ntick  cell        z  on_bridge  occ_deck  terrain  deck?  walkable  stored_deck  layer   prefix_z  expect_z  ok"
    );
    for row in &rows {
        println!(
            "{:5} ({:3},{:3}) {:3}  {:9}  {:8}  {:7}  {:5}  {:8}  {:11}  {:6}  {:8}  {:8}  {}",
            row.tick,
            row.cell.0,
            row.cell.1,
            row.z,
            row.on_bridge,
            row.occupancy_deck
                .map(|d| d.to_string())
                .unwrap_or_else(|| "-".to_string()),
            row.terrain_level,
            row.structural,
            row.bridge_walkable,
            row.stored_deck_level,
            format!("{:?}", row.loco_layer),
            row.prefix_z,
            row.expected_z(),
            if row.holds_invariant() { "ok" } else { "FAIL" },
        );
    }

    let last = rows.last().copied().expect("at least one frame observed");
    println!(
        "\n{} frame(s) recorded, last cell {:?} (target {:?})",
        rows.len(),
        last.cell,
        span.approach_b
    );

    // 1. The drive must actually have used the deck.
    let deck_frames: Vec<&TickRow> = rows.iter().filter(|row| row.structural).collect();
    if deck_frames.is_empty() {
        // Control: can this unit move at all, away from the bridge? Separates
        // "the mover is broken" from "the bridge route is refused".
        let control_goal = offset(start_cell, (-span.step.0 * 3, -span.step.1 * 3))
            .unwrap_or((start_cell.0, start_cell.1));
        let issued = issue_ordinary_move(&mut scenario, &owner_name, entity_id, control_goal);
        println!("control move away from the bridge to {control_goal:?} issued -> {issued}");
        let mut control_cells = Vec::new();
        for _ in 0..300 {
            scenario.tick();
            if let Some(entity) = scenario.sim().entities().get(entity_id) {
                let cell = (entity.position.rx, entity.position.ry);
                if control_cells.last() != Some(&cell) {
                    control_cells.push(cell);
                }
            }
        }
        println!("control move visited: {control_cells:?}");
        if !expect_crossing {
            // Characterization, NOT desired behaviour. This mover accepts the
            // order, never leaves its cell, and then drops the order — while the
            // same mover crosses ordinary ground in the control above. Recorded
            // as an assertion so the day the block is lifted this test goes red
            // and gets rewritten into a real crossing.
            assert!(
                control_cells.len() > 1,
                "the control move away from the bridge also failed, so this says nothing                  specific about the bridge: {control_cells:?}"
            );
            println!(
                "DEFECT CHARACTERIZED: {unit_type} accepted an order onto the span and never                  left {start_cell:?} in {} frames, yet moved {} cell(s) on ordinary ground in                  the control. The height invariant held on every observed frame, but no deck                  frame exists, so this run contributes nothing to the deck-height question.",
                rows.len(),
                control_cells.len() - 1,
            );
            let violations: Vec<&TickRow> =
                rows.iter().filter(|row| !row.holds_invariant()).collect();
            assert!(
                violations.is_empty(),
                "off-bridge height invariant broke: {violations:?}"
            );
            return;
        }
    }
    assert!(
        !deck_frames.is_empty(),
        "the tank never stood on a structural deck cell — it did not cross the bridge, \
         so this run says nothing about deck height. Cells visited: {:?}",
        rows.iter().map(|row| row.cell).collect::<Vec<_>>()
    );
    let visited_deck: std::collections::BTreeSet<(u16, u16)> =
        deck_frames.iter().map(|row| row.cell).collect();
    println!(
        "stood on {} of {} deck cell(s): {:?}",
        visited_deck.len(),
        span.deck.len(),
        visited_deck
    );

    // 2. The native height invariant, every frame, ramps included.
    let violations: Vec<&TickRow> = rows.iter().filter(|row| !row.holds_invariant()).collect();
    assert!(
        violations.is_empty(),
        "position.z left the native model on {} frame(s); first: {:?} (expected z {}, got {})",
        violations.len(),
        violations[0],
        violations[0].expected_z(),
        violations[0].z,
    );

    // 3. On the deck the tank is on the deck: flagged on-bridge, at deck height,
    //    never at the riverbed level under the span.
    for row in &deck_frames {
        assert!(
            row.on_bridge,
            "on structural deck cell {:?} the tank was not marked on_bridge: {row:?}",
            row.cell
        );
        assert_eq!(
            i16::from(row.z as i8),
            i16::from(row.terrain_level as i8) + BRIDGE_DECK_LEVEL_DELTA,
            "tank is not at deck height on {:?}: {row:?}",
            row.cell
        );
        assert_ne!(
            row.z, span.deck_terrain_level,
            "tank dropped to the terrain under the bridge at {:?}: {row:?}",
            row.cell
        );
        assert_eq!(
            row.occupancy_deck,
            Some(row.z),
            "BridgeOccupancy.deck_level disagrees with position.z at {:?}: {row:?}",
            row.cell
        );
    }

    // 4. The whole mid-span portion, not just the entry cell.
    let mid_span: Vec<&TickRow> = deck_frames
        .iter()
        .filter(|row| row.cell != span.deck[0] && row.cell != *span.deck.last().unwrap())
        .copied()
        .collect();
    assert!(
        !mid_span.is_empty(),
        "the tank only ever touched the span's end cells; mid-span behaviour is what the \
         report is about"
    );
    println!("{} mid-span frame(s) all at deck height", mid_span.len());

    // 4b. The crossing finished. Reaching the deck and holding the height there
    //     is not the same as getting across: a mover that enters the span and
    //     then stalls, is snapped back, or gives up mid-deck satisfies every
    //     assertion above. The recording loop breaks on the far approach, on
    //     MAX_TICKS, or on an idle run, and only this distinguishes the first
    //     from the other two.
    let last = rows.last().copied().expect("at least one frame recorded");
    assert_eq!(
        last.cell,
        span.approach_b,
        "the mover never reached the far approach {:?}; it stopped at {:?} after {} frame(s)",
        span.approach_b,
        last.cell,
        rows.len()
    );

    // 5. Discrimination. A green run only rules the fix in if the pre-fix height
    //    path would have produced something different on at least one frame;
    //    otherwise it is a compatibility check, not a regression test.
    let discriminating: Vec<&TickRow> = rows.iter().filter(|row| row.prefix_z != row.z).collect();
    if discriminating.is_empty() {
        println!(
            "\nDISCRIMINATION: NONE. On all {} frames the pre-fix formula \
             `effective_cell_z_for_layer(loco.layer)` yields the same z as the fixed \
             `signed_level() + (on_bridge ? 4 : 0)`. This route does NOT reproduce the \
             reported symptom and does NOT distinguish the two implementations — it only \
             shows the fix did not break an ordinary crossing.",
            rows.len()
        );
    } else {
        println!(
            "\nDISCRIMINATION: {} of {} frames differ from the pre-fix formula; first {:?}",
            discriminating.len(),
            rows.len(),
            discriminating[0]
        );
    }
    let ground_layer_on_deck = rows
        .iter()
        .filter(|row| {
            row.structural
                && row.loco_layer == crate::sim::movement::locomotor::MovementLayer::Ground
        })
        .count();
    println!(
        "frames on a deck cell whose live A* layer was Ground (the C1 trigger): {ground_layer_on_deck}"
    );
}

#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn tank_crosses_bay_of_pigs_high_bridge_at_deck_height() {
    drive_across_high_bridge("BayOPigs.mmx", "MTNK", true);
}

/// Second retail map, different theater and a longer span, so the result is not
/// one map's geometry.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn tank_crosses_hills_high_bridge_at_deck_height() {
    drive_across_high_bridge("Hills.mmx", "MTNK", true);
}

/// Matrix row T1-01. The Robot Tank is a **Hover** locomotor, and until
/// `supports_layered_bridge_pathing` (`movement_path.rs`) admitted Hover it was
/// excluded from the layered path builder, received the flat fallback path whose
/// `path_layers` are `Ground` on every node including the deck cells, and was
/// then refused at the crossing loop's terrain test — which read the riverbed
/// under the span. The order was never dropped, so it drove into the abutment
/// and was snapped back to cell centre indefinitely.
///
/// This was a characterization test asserting that stall. It is now a positive
/// crossing, holding the Hover mover to exactly what the Drive crossings assert:
/// it reaches the deck at all, stands mid-span rather than only on the end
/// cells, and on every deck frame carries `on_bridge`, `z == terrain + 4`, a
/// height that is not the riverbed, and a `BridgeOccupancy` agreeing with its
/// own `z`. `drive_across_high_bridge` also now asserts arrival at the far
/// approach, so a mover that reaches the deck and then dies mid-span fails.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn hover_tank_crosses_bay_of_pigs_high_bridge_at_deck_height() {
    drive_across_high_bridge("BayOPigs.mmx", "ROBO", true);
}

/// The Hover twin on a second map's geometry. `BayOPigs.mmx` runs its span
/// north-south down a column and `Hills.mmx` east-west along a row, so a single
/// map would leave a Hover crossing certified on one span's axis and levels.
/// The Drive side is covered on both; this closes the same gap for Hover.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn hover_tank_crosses_hills_high_bridge_at_deck_height() {
    drive_across_high_bridge("Hills.mmx", "ROBO", true);
}

/// Matrix rows T1-06, T1-07 and T1-08 — Walk onto, along and off an intact high
/// span. Infantry had never been driven across a bridge anywhere in this
/// project: the harness had only ever moved `MTNK` and `ROBO`, so the native
/// twin `WalkLocomotionClass::ProcessMovement` `0x0075C154`-`0x0075C199` was
/// unexercised on the Rust side.
///
/// Walk is not a collapse of the Drive rows. Infantry take a sub-cell
/// reservation arm no vehicle touches, and it is the one Walk-specific bridge
/// code path — so a Drive crossing says nothing about whether an `E1` can hold
/// a sub-cell slot on a deck.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn infantry_crosses_bay_of_pigs_high_bridge_at_deck_height() {
    drive_across_high_bridge("BayOPigs.mmx", "E1", true);
}

#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn infantry_crosses_hills_high_bridge_at_deck_height() {
    drive_across_high_bridge("Hills.mmx", "E1", true);
}

/// Inventory only: which stock maps expose a high-bridge span at all, and what
/// the deck/approach levels are. Diagnostic — it asserts nothing about movement.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn retail_high_bridge_inventory() {
    let Some(retail) = retail_dir() else {
        eprintln!("SKIPPED: no retail root (set RA2_DIR or provide config.toml)");
        return;
    };
    for map_file in [
        "BayOPigs.mmx",
        "Dustbowl.mmx",
        "Bermuda.mmx",
        "Lostlake.mmx",
        "Hills.mmx",
    ] {
        match headless_scenario::load(&retail, map_file, SEED) {
            Ok(scenario) => {
                let grid = scenario.sim().path_grid().expect("navigation published");
                let structural = (0..grid.height())
                    .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
                    .filter(|(x, y)| grid.cell(*x, *y).is_some_and(|c| c.bridge_structural))
                    .count();
                match find_high_bridge_span(grid) {
                    Some(span) => println!(
                        "{map_file}: {structural} structural cell(s); best span {} cell(s) at \
                         terrain level {} between {:?} and {:?} (approach level {})",
                        span.deck.len(),
                        span.deck_terrain_level,
                        span.approach_a,
                        span.approach_b,
                        span.approach_level,
                    ),
                    None => println!("{map_file}: {structural} structural cell(s); no usable span"),
                }
            }
            Err(error) => println!("{map_file}: load failed: {error}"),
        }
    }
}
