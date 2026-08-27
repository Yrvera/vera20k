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
use crate::map::resolved_terrain::{BridgeDirection, ResolvedTerrainGrid};
use crate::rules::locomotor_type::MovementZone;
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
    /// `PathCell::low_bridge_tube_cell` — the `TubeClass` low-span marker
    /// (`tube_index` present **and** final CellClass LandType 10). Recorded on
    /// both drivers so a high-span run states the two markers are disjoint
    /// rather than leaving it assumed.
    low_tube: bool,
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

/// The per-frame observation table both crossing drivers print.
fn print_tick_table(rows: &[TickRow]) {
    println!(
        "\ntick  cell        z  on_bridge  occ_deck  terrain  deck?  walkable  lowtube  stored_deck  layer   prefix_z  expect_z  ok"
    );
    for row in rows {
        println!(
            "{:5} ({:3},{:3}) {:3}  {:9}  {:8}  {:7}  {:5}  {:8}  {:7}  {:11}  {:6}  {:8}  {:8}  {}",
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
            row.low_tube,
            row.stored_deck_level,
            format!("{:?}", row.loco_layer),
            row.prefix_z,
            row.expected_z(),
            if row.holds_invariant() { "ok" } else { "FAIL" },
        );
    }
}

/// Give the loaded scenario a commandable house and return its name.
///
/// A headless load is a spectatorless map load, so the roster may be empty; the
/// mover needs an owner whose name the order path can match. Every house is made
/// passive for the run: the defeat scan would otherwise resolve the match on the
/// first frame and stop committing ticks.
fn prepare_commanding_house(scenario: &mut crate::headless_scenario::HeadlessScenario) -> String {
    let sim = &mut scenario.runtime.simulation;
    let existing: Vec<String> = sim
        .houses
        .keys()
        .map(|id| sim.interner.resolve(*id).to_string())
        .collect();
    println!("map roster houses: {existing:?}");
    let name = existing
        .iter()
        .find(|name| !name.eq_ignore_ascii_case("Neutral") && !name.eq_ignore_ascii_case("Special"))
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
}

/// Report why the order never became a `MovementTarget`, so a harness failure is
/// distinguishable from a height defect.
///
/// Takes the span geometry as loose cells rather than a `HighBridgeSpan` so the
/// low-span driver gets the same diagnosis — a refusal is exactly as plausible
/// there, and it has never been measured.
fn diagnose_rejected_order(
    scenario: &mut crate::headless_scenario::HeadlessScenario,
    owner_name: &str,
    entity_id: u64,
    start_cell: (u16, u16),
    approach_a: (u16, u16),
    approach_b: (u16, u16),
    deck: &[(u16, u16)],
) {
    use crate::sim::pathfinding::{AStarOptions, astar_search, find_path};

    /// Borrowed view so the ablation body below reads identically for a high
    /// span (`HighBridgeSpan`) and a low one (`LowBridgeSpan`).
    struct SpanView<'a> {
        approach_a: (u16, u16),
        approach_b: (u16, u16),
        deck: &'a [(u16, u16)],
    }
    let span = SpanView {
        approach_a,
        approach_b,
        deck,
    };

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

    let owner_name = prepare_commanding_house(&mut scenario);
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
            diagnose_rejected_order(
                &mut scenario,
                &owner_name,
                entity_id,
                start_cell,
                span.approach_a,
                span.approach_b,
                &span.deck,
            );
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
            low_tube: facts.low_bridge_tube_cell,
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

    print_tick_table(&rows);

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

// ---------------------------------------------------------------------------
// Low (`TubeClass`) spans — matrix rows T1-09 / T1-10 / T1-11.
//
// A low span is NOT a high span with different numbers, and it is also not the
// `TubeClass` thing the matrix calls it. Both halves of that are measured by
// `retail_low_bridge_inventory`, whose output is the sole basis for the
// invariant asserted here.
//
// What a low deck cell actually carries, on all four loose low-span fixtures
// (Lostlake, Shrapnel, Killer, EB3):
//
//   has_bridge_deck = true        bridge_deck_level == ground_level  (no +4)
//   bridge_structural = false     bridge_walkable   = false
//   transition        = false     ground_walkable   = true
//   zone_type = 0 (Ground)        LandType  = 1 (Road)   is_water = false
//   tube_index = None             low_bridge_tube_cell = false
//
// The first line is the whole geometry: `resolve_overlay`
// (`map/resolved_terrain.rs`) routes every bridge overlay id that is not
// 24/25/237/238 to `BridgeDirection::Low` and gives it `deck_level = level`,
// and `high_bridge_stamp_for_overlay` (`map/bridge_facts.rs`) returns `None`
// for those ids, so no `0x100` structural stamp is ever written and no Bridge
// A* plane exists over a low span. Both approach cells sit at the deck's own
// level on every fixture, so there is no height event at either end — which is
// why the ledger collapses low onto/along/off into one row.
//
// The last line is the surprise and it is recorded, not worked around: the
// `TubeClass` marker is absent from the entire measured corpus. It is gated on
// `yr_cell_land_type == YR_CELL_LAND_TUNNEL` (10), but a low-bridge overlay
// rewrites the cell's Land to Road (1), and `build_auto_low_bridge_tubes`
// matches *iso-tile* ids, not overlays. So `tube_movement.rs` and
// `PathCell::low_bridge_tube_cell` are not on the stock low-bridge crossing
// path at all. Whether gamemd tubes these cells is UNCHECKED — settling it
// needs the binary, not this harness.
// ---------------------------------------------------------------------------

/// One intact low-bridge span discovered from live map facts.
#[derive(Debug, Clone)]
struct LowBridgeSpan {
    /// Off-deck cell the drive starts on, one step before the first deck cell.
    approach_a: (u16, u16),
    /// Off-deck cell on the far side, one step past the last deck cell.
    approach_b: (u16, u16),
    /// Every low-deck cell between them, in travel order.
    deck: Vec<(u16, u16)>,
    /// The subset of `deck` whose **pre-overlay** terrain is Water — the cells a
    /// mover could not occupy at all if the bridge overlay were removed.
    ///
    /// This is what makes a low-span run a bridge crossing rather than a walk
    /// across flat ground. A low deck is a plain ground cell in every field the
    /// mover reads, so without this the whole test would pass on any straight
    /// stretch of road and would settle nothing.
    water_gap: Vec<(u16, u16)>,
    approach_a_level: u8,
    approach_b_level: u8,
    step: (i32, i32),
}

/// A low deck cell, taken from the map's own overlay classification.
///
/// `resolve_overlay` (`map/resolved_terrain.rs`) maps overlay ids 24/237 to
/// `EastWest`, 25/238 to `NorthSouth` and **every other bridge overlay id** to
/// `Low`, then gives the low ones `deck_level = level` with no `+4`. That is the
/// authoritative low-vs-high split at load, so the span finder reads it rather
/// than guessing from heights — which for a low span are identical to the
/// surrounding terrain and would discriminate nothing.
fn is_low_deck_cell(terrain: &ResolvedTerrainGrid, cell: (u16, u16)) -> bool {
    terrain.cell(cell.0, cell.1).is_some_and(|c| {
        c.bridge_layer
            .as_ref()
            .is_some_and(|layer| layer.direction == BridgeDirection::Low)
    })
}

/// A low deck cell whose **pre-overlay** terrain is Water: a cell that exists as
/// standing room only because the bridge overlay is there.
fn is_water_backed_low_deck(terrain: &ResolvedTerrainGrid, cell: (u16, u16)) -> bool {
    is_low_deck_cell(terrain, cell)
        && terrain.cell(cell.0, cell.1).is_some_and(|c| {
            c.base_land_type == crate::rules::terrain_rules::LandType::Water.as_index()
        })
}

/// Longest straight run of low-deck cells entered and left through ordinary
/// ground-walkable, non-deck cells.
///
/// Deliberately imposes **no** height relation between deck and approach: the
/// high finder demands `approach == deck_terrain + 4` because that is the
/// geometry its predicate fires on, and assuming any analogue here would be
/// assuming the answer. The observed levels are carried on the span and printed.
/// The span the crossing drivers use: longest, ties broken by the lowest first
/// deck cell so the choice is stable across runs and across maps.
///
/// On `Lostlake.mmx` this resolves to `(39,117)..(51,117)` with approaches
/// `(38,117)` and `(52,117)` — the exact fixture the frozen ledger names for
/// T1-09/10/11, confirmed by `retail_low_bridge_inventory` rather than adopted
/// on trust. Nine other 13-cell runs on that map tie on length.
fn find_low_bridge_span(terrain: &ResolvedTerrainGrid, grid: &PathGrid) -> Option<LowBridgeSpan> {
    find_low_bridge_spans(terrain, grid).into_iter().next()
}

/// Every maximal straight low run on the map, deduplicated so a run and its
/// mirror image count once. Used by the inventory: the frozen ledger names one
/// fixture per map, and the only way to check that claim is to enumerate.
fn find_low_bridge_spans(terrain: &ResolvedTerrainGrid, grid: &PathGrid) -> Vec<LowBridgeSpan> {
    let mut seen: std::collections::BTreeSet<((u16, u16), (u16, u16))> =
        std::collections::BTreeSet::new();
    let mut spans = Vec::new();
    for span in enumerate_low_bridge_spans(terrain, grid) {
        let first = span.deck[0];
        let last = *span.deck.last().expect("non-empty run");
        let key = (first.min(last), first.max(last));
        if seen.insert(key) {
            spans.push(span);
        }
    }
    spans.sort_by_key(|span| (std::cmp::Reverse(span.deck.len()), span.deck[0]));
    spans
}

fn enumerate_low_bridge_spans(
    terrain: &ResolvedTerrainGrid,
    grid: &PathGrid,
) -> Vec<LowBridgeSpan> {
    let mut found = Vec::new();
    for y in 0..terrain.height() {
        for x in 0..terrain.width() {
            if !is_low_deck_cell(terrain, (x, y)) {
                continue;
            }
            for step in STEPS {
                let back = (-step.0, -step.1);
                let Some(approach_a) = offset((x, y), back) else {
                    continue;
                };
                if is_low_deck_cell(terrain, approach_a) {
                    continue;
                }
                let Some(cell_a) = grid.cell(approach_a.0, approach_a.1) else {
                    continue;
                };
                if !cell_a.ground_walkable {
                    continue;
                }
                let mut deck = vec![(x, y)];
                let mut cursor = (x, y);
                loop {
                    let Some(next) = offset(cursor, step) else {
                        break;
                    };
                    if !is_low_deck_cell(terrain, next) {
                        break;
                    }
                    deck.push(next);
                    cursor = next;
                }
                let Some(approach_b) = offset(cursor, step) else {
                    continue;
                };
                if is_low_deck_cell(terrain, approach_b) {
                    continue;
                }
                let Some(cell_b) = grid.cell(approach_b.0, approach_b.1) else {
                    continue;
                };
                if !cell_b.ground_walkable {
                    continue;
                }
                let water_gap = deck
                    .iter()
                    .copied()
                    .filter(|cell| is_water_backed_low_deck(terrain, *cell))
                    .collect();
                let candidate = LowBridgeSpan {
                    approach_a,
                    approach_b,
                    deck,
                    water_gap,
                    approach_a_level: cell_a.ground_level,
                    approach_b_level: cell_b.ground_level,
                    step,
                };
                found.push(candidate);
            }
        }
    }
    found
}

fn print_low_inventory(terrain: &ResolvedTerrainGrid, grid: &PathGrid, span: &LowBridgeSpan) {
    println!(
        "span: approach_a {:?} (level {}) -> {} low deck cell(s) -> approach_b {:?} (level {}), \
         step {:?}; {} of the deck cells sit on pre-overlay Water: {:?}",
        span.approach_a,
        span.approach_a_level,
        span.deck.len(),
        span.approach_b,
        span.approach_b_level,
        span.step,
        span.water_gap.len(),
        span.water_gap,
    );
    let mut cells: Vec<(u16, u16)> = vec![span.approach_a];
    cells.extend(span.deck.iter().copied());
    cells.push(span.approach_b);
    for cell in cells {
        let Some(rt) = terrain.cell(cell.0, cell.1) else {
            println!("  {cell:?}: NOT IN RESOLVED TERRAIN");
            continue;
        };
        let Some(pc) = grid.cell(cell.0, cell.1) else {
            println!("  {cell:?}: NOT IN PATH GRID");
            continue;
        };
        let role = if is_low_deck_cell(terrain, cell) {
            "deck"
        } else {
            "appr"
        };
        println!(
            "  {role} {cell:?}: overlay={:?} lvl={} rt_deck_lvl={} rt_has_deck={} rt_bridge_walkable={} \
             tube={:?} tube_src={:?} yr_land={} land={} base_land={} base_blocked={} water={} zone={} \
             | path: gw={} bw={} struct={} trans={} lowtube={} g_lvl={} deck_lvl={}",
            rt.bridge_layer.as_ref().map(|b| b.overlay_id),
            rt.level,
            rt.bridge_deck_level,
            rt.has_bridge_deck,
            rt.bridge_walkable,
            rt.tube_index.map(|t| t.0),
            rt.tube_index
                .and_then(|t| terrain.tube(t))
                .map(|t| t.source),
            rt.yr_cell_land_type,
            rt.land_type,
            rt.base_land_type,
            rt.base_ground_walk_blocked,
            rt.is_water,
            rt.zone_type,
            pc.ground_walkable,
            pc.bridge_walkable,
            pc.bridge_structural,
            pc.transition,
            pc.low_bridge_tube_cell,
            pc.ground_level,
            pc.bridge_deck_level,
        );
    }
}

/// The low-span per-frame invariant, applied to every recorded frame.
///
/// **Measured, not assumed** — see the block comment above for the cell facts
/// `retail_low_bridge_inventory` printed, and note in particular that both
/// approaches sit at the deck's own level on every fixture. There is no height
/// event anywhere on a low span; the crossing *is* the ground plane.
///
/// So the correct invariant is the degenerate case of
/// `ObjectClass::GetHeight @ 0x005F5F30` with OnBridge clear:
///
/// ```text
/// position.z == GroundHeight(own cell)   and   OnBridge == false
/// ```
///
/// The `OnBridge == false` half is load-bearing, not decoration: were a low deck
/// to set it, `Set_Height_On_Bridge` would add four levels of nothing and float
/// the mover over a flat span. Asserting only "z == ground" would pass a
/// hypothetical implementation that sets `on_bridge` and then re-derives z from
/// it, so both halves are checked, plus the absence of a `BridgeOccupancy`
/// entry, which is the third place the deck term is stored.
fn assert_low_span_invariant(rows: &[TickRow], deck_frames: &[&TickRow]) {
    let violations: Vec<&TickRow> = rows.iter().filter(|row| !row.holds_invariant()).collect();
    assert!(
        violations.is_empty(),
        "position.z left the native model on {} frame(s); first: {:?} (expected z {}, got {})",
        violations.len(),
        violations[0],
        violations[0].expected_z(),
        violations[0].z,
    );
    for row in deck_frames {
        assert!(
            !row.on_bridge,
            "a LOW deck cell {:?} set on_bridge; there is no deck plane over a low span, so \
             Set_Height_On_Bridge would lift the mover four levels above flat ground: {row:?}",
            row.cell
        );
        assert!(
            !row.structural,
            "a low deck cell {:?} reported bridge_structural; the low overlays get no \
             SetBridgeDirection stamp: {row:?}",
            row.cell
        );
        assert_eq!(
            i16::from(row.z as i8),
            i16::from(row.terrain_level as i8),
            "the mover is not at ground height on low deck cell {:?}: {row:?}",
            row.cell
        );
        assert_eq!(
            row.occupancy_deck, None,
            "a low deck cell {:?} produced a BridgeOccupancy entry: {row:?}",
            row.cell
        );
        assert_eq!(
            row.stored_deck_level, row.terrain_level,
            "the stored per-cell deck level on low cell {:?} is not the ground level: {row:?}",
            row.cell
        );
    }
}

/// Load a retail map, spawn `unit_type` on one approach of the longest intact
/// low span, order it to the far approach with an ordinary undisabled
/// `Command::Move`, and record `position.z` after every committed frame.
///
/// Same contract as `drive_across_high_bridge` — a refused order panics after a
/// diagnosis rather than being re-issued through a widened search — with the low
/// invariant of `assert_low_span_invariant` in place of the deck-height one.
fn drive_across_low_bridge(map_file: &str, unit_type: &str) {
    let Some(retail) = retail_dir() else {
        eprintln!("SKIPPED: no retail root (set RA2_DIR or provide config.toml)");
        return;
    };
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
        let sim = scenario.sim();
        let terrain = sim
            .resolved_terrain
            .as_ref()
            .expect("headless load keeps resolved terrain");
        let grid = sim.path_grid().expect("headless load publishes navigation");
        let Some(span) = find_low_bridge_span(terrain, grid) else {
            panic!("{map_file} exposes no low-bridge span");
        };
        print_low_inventory(terrain, grid, &span);
        span
    };
    assert!(
        span.deck.len() >= 3,
        "a two-cell span proves nothing about mid-span behaviour; found {} deck cell(s)",
        span.deck.len()
    );

    let owner_name = prepare_commanding_house(&mut scenario);
    println!("commanding house: {owner_name}");

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
    {
        let SimRuntime {
            simulation,
            resources,
        } = &mut scenario.runtime;
        simulation.resolve_type_handles(&resources.rules);
    }
    let movement_zone = {
        let entity = scenario
            .sim()
            .entities()
            .get(entity_id)
            .expect("spawned mover present");
        let zone = entity
            .locomotor
            .as_ref()
            .map(|loco| loco.movement_zone)
            .expect("spawned mover has a locomotor");
        println!(
            "spawned {unit_type} id={entity_id} at {start_cell:?} z={} on_bridge={} zone={zone:?}",
            entity.position.z, entity.on_bridge
        );
        zone
    };
    // Whether the river under the span is an obstacle to *this* mover. A
    // Robot Tank is `AmphibiousDestroyer` and crosses open water unaided, so for
    // it the water gap cannot be used as proof that the bridge was needed.
    let water_is_an_obstacle = !matches!(
        movement_zone,
        MovementZone::Amphibious
            | MovementZone::AmphibiousCrusher
            | MovementZone::AmphibiousDestroyer
            | MovementZone::Water
            | MovementZone::WaterBeach
    );

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

    match scenario
        .sim()
        .entities()
        .get(entity_id)
        .and_then(|entity| entity.movement_target.as_ref())
        .map(|target| target.path.clone())
    {
        Some(path) => println!(
            "move order accepted: {} node(s) {:?} -> {:?}",
            path.len(),
            path.first(),
            path.last()
        ),
        None => {
            diagnose_rejected_order(
                &mut scenario,
                &owner_name,
                entity_id,
                start_cell,
                span.approach_a,
                span.approach_b,
                &span.deck,
            );
            panic!(
                "the ordinary Command::Move to {:?} was REFUSED — a player cannot cross this low \
                 span at all. See the diagnosis above.",
                span.approach_b
            );
        }
    }

    let mut rows: Vec<TickRow> = Vec::new();
    let mut idle_frames = 0u32;
    for _ in 0..MAX_TICKS {
        scenario.tick();
        let sim = scenario.sim();
        let Some(entity) = sim.entities().get(entity_id) else {
            panic!("the mover vanished mid-crossing");
        };
        let cell = (entity.position.rx, entity.position.ry);
        let facts = sim
            .path_grid()
            .and_then(|grid| grid.cell(cell.0, cell.1))
            .copied();
        let Some(facts) = facts else {
            panic!("the mover left the path grid at {cell:?}");
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
            low_tube: facts.low_bridge_tube_cell,
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

    print_tick_table(&rows);
    let last = rows.last().copied().expect("at least one frame observed");
    println!(
        "\n{} frame(s) recorded, last cell {:?} (target {:?})",
        rows.len(),
        last.cell,
        span.approach_b
    );

    let terrain = scenario
        .sim()
        .resolved_terrain
        .as_ref()
        .expect("headless load keeps resolved terrain");

    // 1. The drive must actually have used the span.
    //
    // Deck membership is "the mover's cell is a low deck cell", not "the cell is
    // in the discovered single-file run". MEASURED reason: a low span is often
    // several lanes wide — Killer's is three, columns 92/93/94 — and A* is free
    // to change lane mid-crossing between equal-cost parallel deck cells. The
    // Hover mover does exactly that on Killer, leaving the discovered lane at
    // y=136 and rejoining it at y=144. That is ordinary tie-breaking, not a
    // defect, but a single-lane membership test calls it a failed crossing.
    let deck_frames: Vec<&TickRow> = rows
        .iter()
        .filter(|row| is_low_deck_cell(terrain, row.cell))
        .collect();
    assert!(
        !deck_frames.is_empty(),
        "the {unit_type} never stood on a low deck cell — it did not cross the span, so this run \
         says nothing about low-span movement. Cells visited: {:?}",
        rows.iter().map(|row| row.cell).collect::<Vec<_>>()
    );
    let visited_deck: std::collections::BTreeSet<(u16, u16)> =
        deck_frames.iter().map(|row| row.cell).collect();
    println!(
        "stood on {} low deck cell(s) ({} of the {} in the discovered lane): {:?}",
        visited_deck.len(),
        visited_deck
            .iter()
            .filter(|cell| span.deck.contains(cell))
            .count(),
        span.deck.len(),
        visited_deck
    );

    // 1b. It was a *bridge* crossing. A low deck reads as a plain ground cell in
    //     every field the mover consults, so reaching the far approach proves
    //     nothing on its own — the same run would pass on a straight stretch of
    //     road. What makes it a crossing is that the middle of the span is water
    //     under the overlay, and the mover was over all of it.
    //
    //     Coverage is measured along the travel axis rather than cell by cell,
    //     which is what makes it lane-agnostic: for every position along the
    //     river the span bridges, the mover must have stood on *some*
    //     water-backed deck cell at that position.
    assert!(
        !span.water_gap.is_empty(),
        "this span has no pre-overlay water under it, so crossing it says nothing about \
         bridges; deck {:?}",
        span.deck
    );
    let axis = |cell: (u16, u16)| if span.step.0 != 0 { cell.0 } else { cell.1 };
    let gap_axis: std::collections::BTreeSet<u16> =
        span.water_gap.iter().copied().map(axis).collect();
    let covered_axis: std::collections::BTreeSet<u16> = visited_deck
        .iter()
        .copied()
        .filter(|cell| is_water_backed_low_deck(terrain, *cell))
        .map(axis)
        .collect();
    if water_is_an_obstacle {
        let uncovered: Vec<u16> = gap_axis.difference(&covered_axis).copied().collect();
        assert!(
            uncovered.is_empty(),
            "the mover was never over the river at {uncovered:?} along the travel axis; the span \
             bridges water at {gap_axis:?} and the mover only covered {covered_axis:?}"
        );
        println!(
            "spanned the whole {}-wide water gap at axis positions {gap_axis:?}",
            gap_axis.len()
        );
    } else {
        // MEASURED, and it is a real weakening of this row rather than a
        // convenience: on `Killer.mmx` the `ROBO` leaves the deck at y=136 and
        // hovers the river directly to y=143 before rejoining, because
        // `AmphibiousDestroyer` treats the water as passable and the two routes
        // cost near enough the same. On `Lostlake.mmx` the same unit stays on
        // all 13 deck cells. Neither is a defect; both mean the water gap
        // cannot certify that an amphibious mover *needed* the bridge.
        //
        // What is still asserted is that it used the bridge over open water
        // somewhere, which rules out a run that bypassed the span entirely.
        assert!(
            !covered_axis.is_empty(),
            "the {unit_type} ({movement_zone:?}) never stood on a water-backed deck cell, so it \
             did not use the span at all; the span bridges water at {gap_axis:?}"
        );
        println!(
            "NOTE: {unit_type} is {movement_zone:?} and crosses open water unaided, so full \
             water-gap coverage is NOT required of it. Covered {} of the {} axis positions: \
             {covered_axis:?}",
            covered_axis.len(),
            gap_axis.len(),
        );
    }

    // 2/3. The measured low-span invariant.
    assert_low_span_invariant(&rows, &deck_frames);

    // 4. Mid-span, not just the two end cells. The matrix collapses low
    //    onto/along/off into one row precisely because there is no height event
    //    at either end, so a run that only clipped the ends would settle nothing.
    let mid_span: Vec<&TickRow> = deck_frames
        .iter()
        .filter(|row| row.cell != span.deck[0] && row.cell != *span.deck.last().unwrap())
        .copied()
        .collect();
    assert!(
        !mid_span.is_empty(),
        "the mover only ever touched the span's end cells; mid-span behaviour is the row"
    );
    println!("{} mid-span frame(s) all at ground height", mid_span.len());

    // 5. The crossing finished. Entering the span and holding height on it is not
    //    the same as getting across.
    assert_eq!(
        last.cell,
        span.approach_b,
        "the mover never reached the far approach {:?}; it stopped at {:?} after {} frame(s)",
        span.approach_b,
        last.cell,
        rows.len()
    );

    let tube_frames = rows.iter().filter(|row| row.low_tube).count();
    println!(
        "frames on a cell carrying the TubeClass low-bridge marker: {tube_frames} of {}",
        rows.len()
    );
}

/// Matrix row T1-09 — Drive along an intact low span, on the exact fixture the
/// ledger names: `Lostlake.mmx` `(39,117)..(51,117)`, ordered `(38,117)` →
/// `(52,117)`. No low span had been crossed by any locomotor anywhere in this
/// project; low spans are on ~32 % of stock maps.
///
/// Stands for the collapsed onto/along/off set — a low deck sits at the
/// surrounding terrain level (measured: both approaches share the deck's level
/// on all four fixtures), so there is no height event at either end and one
/// crossing exercises all three relations.
///
/// It does **not** stand for the wood/concrete pair the way the ledger assumed.
/// That collapse was justified by both materials sharing the `TubeClass`
/// movement path; the inventory shows neither of them reaches it. What they do
/// share is the ordinary ground plane, which is a stronger reason for the same
/// collapse — but it is a different reason, and only the wood fixture is loose.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn tank_crosses_lostlake_low_bridge_at_ground_height() {
    drive_across_low_bridge("Lostlake.mmx", "MTNK");
}

/// Matrix row T1-10 — Walk along the same span.
///
/// Still separate from T1-09, but **not for the reason the ledger gives.** The
/// row cites the `EntityCategory::{Unit, Infantry}` gate in
/// `begin_path_tube_step` (`tube_movement.rs`) as infantry's own arm; the
/// inventory shows no stock low-bridge cell carries a tube index, so that gate
/// is never reached on any of these maps and cannot be what separates the rows.
///
/// What does separate them is the same thing that separated T1-06 from T1-03:
/// infantry take a sub-cell reservation arm no vehicle touches, and drive the
/// Walk locomotion twin rather than the drive track. Neither had ever been run
/// over a low deck.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn infantry_crosses_lostlake_low_bridge_at_ground_height() {
    drive_across_low_bridge("Lostlake.mmx", "E1");
}

/// Matrix row T1-11 — Hover along the same span.
///
/// The ledger notes `tube_movement.rs` gates on category and layer, never on
/// locomotor kind — which the inventory makes moot, since a stock low span never
/// enters that file. The row stands on the part that was always the real point:
/// the planner, the crossing loop and the height commit all sit upstream, and
/// T1-01 showed Hover taking a different planner branch from Drive on a high
/// span and stalling indefinitely for it. "Hover is the same as Drive here" is
/// exactly the kind of assumption this row exists to measure.
///
/// **This row is weaker than T1-09 and T1-10, on purpose.** `ROBO` is
/// `AmphibiousDestroyer`, so the river the span bridges is not an obstacle to
/// it, and the water-gap coverage check that certifies the Drive and Walk rows
/// cannot certify this one. Measured: on `Lostlake.mmx` it stays on all 13 deck
/// cells and covers the whole 7-cell gap; on `Killer.mmx` it leaves the deck at
/// y=136 and hovers open water to y=143 before rejoining, covering 6 of 14. Both
/// are legitimate. What this row therefore settles is that an ordinary Move
/// across a low span is accepted, that the mover holds ground height with no
/// `on_bridge` and no `BridgeOccupancy` on every deck cell it does use, and that
/// it arrives — not that it needed the bridge.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn hover_tank_crosses_lostlake_low_bridge_at_ground_height() {
    drive_across_low_bridge("Lostlake.mmx", "ROBO");
}

// The second low-span geometry, for the same reason the high rows carry two
// maps. `Lostlake.mmx`'s fixture runs east-west across a 13-cell span with a
// 7-cell water gap and Road approaches; `Killer.mmx` runs north-south across 22
// cells with a 14-cell water gap and Rough approaches. A row settled on one axis
// and one approach LandType is settled on one map, not on low spans.

#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn tank_crosses_killer_low_bridge_at_ground_height() {
    drive_across_low_bridge("Killer.mmx", "MTNK");
}

#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn infantry_crosses_killer_low_bridge_at_ground_height() {
    drive_across_low_bridge("Killer.mmx", "E1");
}

#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn hover_tank_crosses_killer_low_bridge_at_ground_height() {
    drive_across_low_bridge("Killer.mmx", "ROBO");
}

/// Inventory only: where the stock maps put low spans and what facts those cells
/// carry. Diagnostic — it asserts nothing about movement, and exists because the
/// low-span invariant had to be measured before it could be written down.
#[test]
#[ignore = "requires a retail RA2/YR install (RA2_DIR or config.toml)"]
fn retail_low_bridge_inventory() {
    let Some(retail) = retail_dir() else {
        eprintln!("SKIPPED: no retail root (set RA2_DIR or provide config.toml)");
        return;
    };
    for map_file in [
        "Lostlake.mmx",
        "Shrapnel.mmx",
        "Killer.mmx",
        "EB3.mmx",
        "Dustbowl.mmx",
        "Bermuda.mmx",
        "BayOPigs.mmx",
        "Hills.mmx",
    ] {
        println!("\n===== {map_file} =====");
        match headless_scenario::load(&retail, map_file, SEED) {
            Ok(scenario) => {
                let sim = scenario.sim();
                let terrain = sim
                    .resolved_terrain
                    .as_ref()
                    .expect("headless load keeps resolved terrain");
                let grid = sim.path_grid().expect("navigation published");
                let all: Vec<(u16, u16)> = (0..terrain.height())
                    .flat_map(|y| (0..terrain.width()).map(move |x| (x, y)))
                    .collect();
                let low = all
                    .iter()
                    .filter(|c| is_low_deck_cell(terrain, **c))
                    .count();
                let structural = all
                    .iter()
                    .filter(|(x, y)| grid.cell(*x, *y).is_some_and(|c| c.bridge_structural))
                    .count();
                let tube_marked = all
                    .iter()
                    .filter(|(x, y)| grid.cell(*x, *y).is_some_and(|c| c.low_bridge_tube_cell))
                    .count();
                let tube_indexed = all
                    .iter()
                    .filter(|(x, y)| terrain.cell(*x, *y).is_some_and(|c| c.tube_index.is_some()))
                    .count();
                println!(
                    "{}x{}: {low} low-deck cell(s), {structural} structural high cell(s), \
                     {tube_marked} low_bridge_tube_cell, {tube_indexed} with a tube index",
                    terrain.width(),
                    terrain.height(),
                );
                let spans = find_low_bridge_spans(terrain, grid);
                println!("{} crossable low run(s), longest first:", spans.len());
                for span in &spans {
                    println!(
                        "  {:?} .. {:?} ({} cells, step {:?}) approaches {:?} lvl {} / {:?} lvl {}",
                        span.deck[0],
                        span.deck.last().expect("non-empty"),
                        span.deck.len(),
                        span.step,
                        span.approach_a,
                        span.approach_a_level,
                        span.approach_b,
                        span.approach_b_level,
                    );
                }
                match spans.first() {
                    Some(span) => print_low_inventory(terrain, grid, span),
                    None => println!("no usable low span"),
                }
            }
            Err(error) => println!("{map_file}: load failed: {error}"),
        }
    }
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
