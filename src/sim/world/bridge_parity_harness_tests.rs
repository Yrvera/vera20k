//! Bridge-crossing replay golden — determinism + baseline ratchet for the one
//! scenario no committed fixture in this repo covered: a ground vehicle driving
//! across a **high bridge** under the live `advance_tick` path.
//!
//! `global_parity_harness_tests.rs` is the model for the shape (record a
//! `ReplayLog`, replay it through `ReplayRunner` on a fresh `Simulation`, assert
//! tick-for-tick hash equality plus a committed final baseline). Every "bridge"
//! in that harness is the *mission* bridge — a code concept — so a regression in
//! deck height, the `on_bridge` flag, or the bridge occupancy layer sails
//! straight through the default `--lib` suite. This file closes that gap.
//!
//! Scope note: the fixture stamps a synthetic span into a `PathGrid` rather than
//! loading a retail map, so it runs with no assets and belongs in the DEFAULT
//! suite. The retail-map half of the same question — does this hold on real
//! stamped map data — is `sim::movement::movement_bridge_retail_tests`, which is
//! `#[ignore]`d because it needs `RA2_DIR`.
//!
//! The height invariant asserted on every deck frame is the one recorded on
//! `resolve_cell_transition_bridge_state` in `sim::movement::movement_bridge`
//! (`FootClass::Set_Height_On_Bridge` @ `0x005F5FA0`, read back by
//! `ObjectClass::GetHeight` @ `0x005F5F30`):
//!
//! ```text
//! position.z == own cell's signed terrain level + (on_bridge ? 4 : 0)
//! ```
//!
//! No new gamemd claim is made here; this fixture only pins the Rust behavior
//! those already-recorded citations describe.

use super::*;
use crate::map::entities::{EntityCategory, MapEntity};
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::pathfinding::PathGrid;
use crate::sim::replay::{ReplayHeader, ReplayLog, ReplayRunner};
use std::collections::{BTreeMap, BTreeSet};

const BRIDGE_HARNESS_SEED: u64 = 0x0B21_D6E5_C0DE;
const BRIDGE_HARNESS_TICKS: u64 = 200;
const BRIDGE_HARNESS_TICK_MS: u32 = 67;

const GRID_W: u16 = 64;
const GRID_H: u16 = 64;

/// The span runs west-to-east along this row.
const SPAN_Y: u16 = 20;
/// Plateau (approach) terrain level on both banks.
const APPROACH_LEVEL: u8 = 4;
/// Terrain level of the gorge the span crosses — the riverbed the tank must
/// never sit on while it is flagged on-bridge.
const GORGE_LEVEL: u8 = 0;
/// `FootClass::Set_Height_On_Bridge`'s deck term, in levels. Same number as
/// `sim::movement::movement_occupancy::BRIDGE_DECK_LEVEL_DELTA`, which is
/// `pub(super)` to the movement module and therefore not nameable from here.
const DECK_LEVEL_DELTA: i16 = 4;

/// Start cell: plain plateau ground, one step before the entry ramp.
const APPROACH_A_X: u16 = 14;
/// Entry ramp: a bridgehead **transition** cell that is deliberately NOT
/// structural. `set_cell_for_test` ties `bridge_structural` to `bridge_walkable`
/// and so cannot express it; `set_bridge_cell_decoupled_for_test` can. It has to
/// be non-structural because the runtime Exit arm fires on
/// `src structural && !dst structural` — a structural exit ramp would leave the
/// tank flagged on-bridge after it had already driven off the span.
const ENTRY_RAMP_X: u16 = 15;
/// First and last structural deck cell (inclusive) — eight cells over the gorge.
const DECK_FIRST_X: u16 = 16;
const DECK_LAST_X: u16 = 23;
/// Exit ramp, mirror of the entry ramp.
const EXIT_RAMP_X: u16 = 24;
/// Destination: plain plateau ground on the far bank.
const APPROACH_B_X: u16 = 25;

/// Distinct structural deck cells the tank must actually stand on. Eight exist;
/// requiring six leaves room for the sub-cell curve to skip an end cell without
/// letting a fixture that barely touches the span pass.
const MIN_DISTINCT_DECK_CELLS: usize = 6;

/// Committed final-hash baseline for the recorded bridge crossing.
///
/// **This is a Rust-vs-prior-Rust regression ratchet, NOT gamemd parity
/// evidence.** ENGINE.md is explicit that replay fixtures and Rust-derived
/// hashes are regression ratchets; only machine-derived goldens (binary
/// emulation, live capture, retail bytes) are parity references. What this
/// constant proves is that the committed crossing — path, per-tick positions,
/// `on_bridge`, `bridge_occupancy`, deck heights, and every other hashed field —
/// did not move without someone noticing.
///
/// Captured from the first green run of this fixture. Re-baseline at most once
/// per behavior-bearing change, with a one-line documented reason, and only
/// after the coverage tripwires below still pass — a baseline over a tank that
/// stopped crossing is worthless.
///
/// It has its own name on purpose: `GLOBAL_HARNESS_FINAL_HASH` and the other
/// committed goldens are shared, coordination-gated constants and are not
/// touched by this file.
/// Re-baselined for TechnoClass::TechnoClass @ 0x006F2B90: the two authored
/// Technos now consume and persist the raw Scenario words written at
/// 0x006F3254. Path nodes, visited cells, bridge tripwires, and record/replay
/// equality remain exact; this is a Rust regression ratchet, not parity evidence.
/// Re-baselined 2026-08-30 for GSI-04.03 Drive/Ship slope payload ownership.
/// The ordered path, all 12 visited cells, bridge height/occupation tripwires,
/// all three RNG streams, and tick-for-tick replay remain exact; only the
/// hash composition moved from retired body-rocking bytes to the locomotor.
/// Re-baselined 2026-08-30 for v107's Spark shared-dummy tag plus level/slope
/// folds. The path, bridge tripwires, and RNG streams remain byte-identical.
/// Re-baselined 2026-08-30 for v110's unconditional ordered BasePlan authority.
/// The dedicated pre-v110 probe below reproduces the prior baseline exactly;
/// path, bridge tripwires, RNG streams, and tick-for-tick replay remain exact.
/// Re-baselined 2026-09-01 for v114's unconditional raw 256-slot crate authority.
/// The dedicated pre-v114 probe reproduces the prior current baseline exactly;
/// the same crossing, tripwires, streams, and replay equality remain exact.
/// Re-baselined 2026-09-01 for v115's retained wall-neighbor count authority mode and
/// shared-dummy overlay identity/state folds. The dedicated pre-v115 probe reproduces the
/// prior current baseline exactly; this fixture builds a legacy `None`-count grid, so only
/// current-schema composition moved.
// Re-baselined 2026-09-02 for the native tiberium queue store (OQ-38, bridge transaction 3
// slice D): every class now carries the native entry array, float min-heap, capacity, and
// `native_rect`, rebuilds walk `CellIterator` order, and spread admission applies the
// `FirstObject` occupier gate. This is behavior-bearing on every fixture with ore, so the
// historical probes move as well; the RNG stream tuple and tick-for-tick record/replay
// equality remain exact.
const BRIDGE_HARNESS_PRE_BASE_PLAN_V110_HASH: u64 = 0x8211_8936_272A_AA53;
const BRIDGE_HARNESS_PRE_CRATE_AUTHORITY_V114_HASH: u64 = 0x6CB4_48E4_0471_9419;
const BRIDGE_HARNESS_PRE_WALL_RUNTIME_V115_HASH: u64 = 0x9996_B042_ACF6_7C2E;
const BRIDGE_HARNESS_FINAL_HASH: u64 = 0x32F4_252D_E18A_4139;

fn bridge_ini() -> IniFile {
    // One armed ground vehicle and one distant infantryman on a second house, so
    // no side is defeated on frame one. Their ranges keep them out of each
    // other's reach for the whole run.
    IniFile::from_str(
        "[InfantryTypes]\n0=E1\n\n\
         [VehicleTypes]\n0=MTNK\n\n\
         [AircraftTypes]\n\n\
         [BuildingTypes]\n\n\
         [E1]\nLocomotor={4A582744-9839-11d1-B709-00A024DDAFD1}\nStrength=125\nArmor=flak\nSpeed=4\nPrimary=M60\n\n\
         [MTNK]\nLocomotor={4A582741-9839-11d1-B709-00A024DDAFD1}\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
         [M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\n\n\
         [105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
         [SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n\n\
         [AP]\nVerses=100%,100%,90%,75%,75%,75%,60%,30%,20%,0%,0%\n",
    )
}

fn bridge_rules() -> RuleSet {
    RuleSet::from_ini(&bridge_ini()).expect("bridge harness rules should parse")
}

/// Is `x` a gorge column — the low ground the span crosses?
fn is_gorge_column(x: u16) -> bool {
    (DECK_FIRST_X..=DECK_LAST_X).contains(&x)
}

/// Stamp the span into a fresh grid.
///
/// Geometry, west to east on row [`SPAN_Y`]:
///
/// ```text
///   ..14  |  15   | 16 .. 23  |  24   | 25 ..
///  approach  entry   8 deck      exit    far
///  plateau   ramp    cells       ramp    approach
///  level 4   lvl 4   level 0     lvl 4   level 4
/// ```
///
/// Everything off the row in columns 16..=23 is the gorge: level 0 and
/// **blocked**, so the span is the only crossing and A* cannot route around it.
/// Both banks are level 4, which is also what makes the crossing legal: the
/// runtime Enter predicate fires only on `dst_level == src_level - 4` AND
/// `dst.has_structural_bridge()`, so 0 == 4 - 4 on a structural deck cell.
fn bridge_grid() -> PathGrid {
    let mut grid = PathGrid::new(GRID_W, GRID_H);
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            if is_gorge_column(x) {
                // Gorge floor: low ground, impassable, no bridge.
                grid.set_cell_for_test(x, y, GORGE_LEVEL, false, false);
                grid.set_blocked(x, y, true);
            } else {
                // Plateau: ordinary level-4 ground on both banks.
                grid.set_cell_for_test(x, y, APPROACH_LEVEL, false, false);
            }
        }
    }

    // Both ramps: bridge-walkable transition cells at plateau level that are NOT
    // structural (see ENTRY_RAMP_X).
    for ramp_x in [ENTRY_RAMP_X, EXIT_RAMP_X] {
        grid.set_bridge_cell_decoupled_for_test(
            ramp_x,
            SPAN_Y,
            APPROACH_LEVEL,
            false, // not structural
            true,  // bridge-walkable
            APPROACH_LEVEL,
            true, // bridgehead/transition
        );
    }

    // The deck itself: structural, bridge-walkable, over the gorge, with the
    // stored deck value at plateau level. The transition flag matches what the
    // map stamper writes for Anchor/Forward1/Opposite deck slots.
    for x in DECK_FIRST_X..=DECK_LAST_X {
        grid.set_bridge_cell_decoupled_for_test(
            x,
            SPAN_Y,
            GORGE_LEVEL,
            true, // structural deck
            true, // bridge-walkable
            APPROACH_LEVEL,
            true,
        );
    }

    grid
}

/// Terrain heights matching the grid, so a spawned object starts at its cell's
/// real level rather than 0.
fn bridge_heights() -> BTreeMap<(u16, u16), u8> {
    let mut heights = BTreeMap::new();
    for y in 0..GRID_H {
        for x in 0..GRID_W {
            // Terrain only — the deck is not terrain, so the cells under the
            // span carry the gorge floor here exactly like the rest of the gorge.
            let level = if is_gorge_column(x) {
                GORGE_LEVEL
            } else {
                APPROACH_LEVEL
            };
            heights.insert((x, y), level);
        }
    }
    heights
}

fn unit(owner: &str, type_id: &str, cx: u16, cy: u16, cat: EntityCategory) -> MapEntity {
    MapEntity {
        owner: owner.to_string(),
        type_id: type_id.to_string(),
        health: 256,
        cell_x: cx,
        cell_y: cy,
        facing: 64,
        category: cat,
        sub_cell: 0,
        veterancy: 0,
        high: false,
        mission: None,
        recruitable_a: true,
        recruitable_b: true,
        structure_upgrades: [None, None, None],
    }
}

/// Spawn order fixes stable ids: 1 = the crossing tank, 2 = a far-away Soviet
/// rifleman that exists only so neither house is defeated at once.
fn seed_bridge_scenario(sim: &mut Simulation, rules: &RuleSet, heights: &BTreeMap<(u16, u16), u8>) {
    sim.spawn_from_map(
        &[
            unit(
                "Americans",
                "MTNK",
                APPROACH_A_X,
                SPAN_Y,
                EntityCategory::Unit,
            ), // 1
            unit("Soviet", "E1", 58, 58, EntityCategory::Infantry), // 2
        ],
        Some(rules),
        heights,
    );
}

const TANK_ID: u64 = 1;

/// One scripted order: drive the tank from the near approach to the far one.
fn bridge_script() -> Vec<(u64, Command)> {
    vec![(
        2,
        Command::Move {
            entity_id: TANK_ID,
            target_rx: APPROACH_B_X,
            target_ry: SPAN_Y,
            queue: false,
            group_id: None,
        },
    )]
}

fn due_commands(sim: &Simulation, script: &[(u64, Command)], tick: u64) -> Vec<CommandEnvelope> {
    let owner = sim.interner.get("Americans").expect("Americans interned");
    script
        .iter()
        .filter(|(t, _)| *t == tick + 1)
        .map(|(t, c)| CommandEnvelope::new(owner, *t, c.clone()))
        .collect()
}

/// One committed frame of observed mover state.
#[derive(Debug, Clone, Copy)]
struct CrossingFrame {
    tick: u64,
    cell: (u16, u16),
    z: u8,
    on_bridge: bool,
    occupancy_deck: Option<u8>,
    terrain_level: i16,
    structural: bool,
}

impl CrossingFrame {
    /// `position.z == own cell's signed terrain level + (on_bridge ? 4 : 0)`.
    fn expected_z(&self) -> i16 {
        self.terrain_level + if self.on_bridge { DECK_LEVEL_DELTA } else { 0 }
    }

    fn holds_invariant(&self) -> bool {
        i16::from(self.z as i8) == self.expected_z()
    }
}

#[test]
fn bridge_crossing_replay_is_deterministic_and_baseline_stable() {
    let rules = bridge_rules();
    let heights = bridge_heights();
    let grid = bridge_grid();
    let script = bridge_script();

    // ---- Record pass: drive the crossing through the live advance_tick path. ----
    let mut rec = Simulation::with_seed(BRIDGE_HARNESS_SEED);
    seed_bridge_scenario(&mut rec, &rules, &heights);
    let mut log = ReplayLog::new(ReplayHeader {
        version: 1,
        tick_hz: 15,
        seed: BRIDGE_HARNESS_SEED,
        map_name: "bridge_parity_harness".to_string(),
        rules_hash: 0,
    });

    let mut frames: Vec<CrossingFrame> = Vec::with_capacity(BRIDGE_HARNESS_TICKS as usize);
    let mut order_accepted_path: Option<usize> = None;
    for tick in 0..BRIDGE_HARNESS_TICKS {
        let due = due_commands(&rec, &script, tick);
        let result = rec.advance_tick(
            &due,
            Some(&rules),
            &heights,
            Some(&grid),
            None,
            BRIDGE_HARNESS_TICK_MS,
        );
        let entity = rec
            .substrate
            .entities
            .get(TANK_ID)
            .expect("the crossing tank must stay alive for the whole run");
        if order_accepted_path.is_none() {
            order_accepted_path = entity.movement_target.as_ref().map(|t| t.path.len());
        }
        let cell = (entity.position.rx, entity.position.ry);
        let facts = grid
            .cell(cell.0, cell.1)
            .copied()
            .unwrap_or_else(|| panic!("the tank left the path grid at {cell:?}"));
        frames.push(CrossingFrame {
            tick,
            cell,
            z: entity.position.z,
            on_bridge: entity.on_bridge,
            occupancy_deck: entity.bridge_occupancy.map(|occ| occ.deck_level),
            terrain_level: facts.signed_level(),
            structural: facts.has_structural_bridge(),
        });
        log.record_tick(tick, due, result.state_hash);
    }

    // ---- Coverage tripwires. These are the point of the fixture: without them
    // the baseline below would happily pin a tank that never moved. ----
    let visited: Vec<(u16, u16)> = {
        let mut seen: Vec<(u16, u16)> = Vec::new();
        for frame in &frames {
            if seen.last() != Some(&frame.cell) {
                seen.push(frame.cell);
            }
        }
        seen
    };
    println!(
        "[bridge parity] ordered path nodes: {order_accepted_path:?}; cells visited: {visited:?}"
    );
    // One line per cell the tank entered, not per frame: the run is ~200 frames
    // and a failure report has to stay readable.
    let mut previous: Option<(u16, u16)> = None;
    for frame in &frames {
        if previous == Some(frame.cell) {
            continue;
        }
        previous = Some(frame.cell);
        println!(
            "  tick {:4} cell {:?} z={} on_bridge={} occ_deck={:?} terrain={} deck={} expect_z={}",
            frame.tick,
            frame.cell,
            frame.z,
            frame.on_bridge,
            frame.occupancy_deck,
            frame.terrain_level,
            frame.structural,
            frame.expected_z(),
        );
    }

    assert!(
        order_accepted_path.is_some(),
        "the ordinary Command::Move onto the span was never turned into a path — \
         a player could not cross at all. Cells visited: {visited:?}"
    );

    // 1. The Enter predicate must have fired: on_bridge became true at some tick.
    assert!(
        frames.iter().any(|f| f.on_bridge),
        "on_bridge never became true — BridgeStateUpdate::Set never fired, so the \
         tank did not enter the span. Cells visited: {visited:?}"
    );

    // 2. The tank stood on a real run of the deck, not just its first cell.
    let deck_cells: BTreeSet<(u16, u16)> = frames
        .iter()
        .filter(|f| f.structural)
        .map(|f| f.cell)
        .collect();
    assert!(
        deck_cells.len() >= MIN_DISTINCT_DECK_CELLS,
        "the tank stood on only {} distinct structural deck cell(s), need at least \
         {MIN_DISTINCT_DECK_CELLS}: {deck_cells:?}. Cells visited: {visited:?}",
        deck_cells.len(),
    );

    // 3. Every deck frame is at deck height, flagged on-bridge, and agrees with
    //    its own BridgeOccupancy — never dropped to the gorge floor underneath.
    for frame in frames.iter().filter(|f| f.structural) {
        assert!(
            frame.on_bridge,
            "on structural deck cell {:?} the tank was not marked on_bridge: {frame:?}",
            frame.cell
        );
        assert_eq!(
            i16::from(frame.z as i8),
            frame.terrain_level + DECK_LEVEL_DELTA,
            "deck height broke at {:?}: z must be terrain + {DECK_LEVEL_DELTA}: {frame:?}",
            frame.cell
        );
        assert_ne!(
            i16::from(frame.z as i8),
            i16::from(GORGE_LEVEL as i8),
            "the tank dropped to the gorge floor under the span at {:?}: {frame:?}",
            frame.cell
        );
        assert_eq!(
            frame.occupancy_deck,
            Some(frame.z),
            "BridgeOccupancy.deck_level disagrees with position.z at {:?}: {frame:?}",
            frame.cell
        );
    }

    // 4. The invariant holds on the off-bridge frames too — approaches and ramps.
    let violations: Vec<&CrossingFrame> = frames.iter().filter(|f| !f.holds_invariant()).collect();
    assert!(
        violations.is_empty(),
        "position.z left the native height model on {} frame(s); first: {:?}",
        violations.len(),
        violations.first(),
    );

    // 5. The tank finished the crossing: it reached the far approach and the
    //    Exit arm cleared on_bridge again.
    let last = frames.last().copied().expect("at least one frame recorded");
    assert_eq!(
        last.cell,
        (APPROACH_B_X, SPAN_Y),
        "the tank never reached the far approach; it ended at {:?}. Cells visited: {visited:?}",
        last.cell
    );
    assert!(
        !last.on_bridge,
        "the tank is still flagged on_bridge on the far approach: {last:?}"
    );
    assert_eq!(
        last.occupancy_deck, None,
        "BridgeOccupancy survived the Exit transition: {last:?}"
    );

    // ---- Replay pass: fresh sim, real ReplayRunner, tick-for-tick equality. ----
    let mut rep = Simulation::with_seed(BRIDGE_HARNESS_SEED);
    seed_bridge_scenario(&mut rep, &rules, &heights);
    let replayed = ReplayRunner::run_fixture_with_overlay_registry(
        &mut rep,
        &log,
        Some(&rules),
        &heights,
        Some(&grid),
        None,
        BRIDGE_HARNESS_TICK_MS,
    );
    assert_eq!(
        replayed.len(),
        log.ticks.len(),
        "replay tick count must match record"
    );
    for (i, h) in replayed.iter().enumerate() {
        assert_eq!(
            *h, log.ticks[i].state_hash,
            "intra-run determinism: replay tick {i} hash must equal the recorded hash"
        );
    }

    let final_hash = *replayed.last().expect("at least one tick replayed");
    let pre_base_plan_hash = rep.state_hash_without_base_plan_v110();
    let pre_crate_authority_hash = rep.state_hash_without_crate_authority_v114();
    let pre_wall_runtime_hash = rep.state_hash_without_wall_runtime_v115();
    println!(
        "[bridge parity] final_hash={final_hash:016X} pre-v110:{pre_base_plan_hash:016X} pre-v114:{pre_crate_authority_hash:016X} pre-v115:{pre_wall_runtime_hash:016X} streams={:016X},{:016X},{:016X}",
        rep.scenario_rng.state(),
        rep.main_rng.state(),
        rep.mapgen_rng.state(),
    );
    assert_eq!(
        pre_base_plan_hash, BRIDGE_HARNESS_PRE_BASE_PLAN_V110_HASH,
        "the dedicated pre-v110 probe must reproduce the prior bridge baseline"
    );
    assert_eq!(
        pre_crate_authority_hash, BRIDGE_HARNESS_PRE_CRATE_AUTHORITY_V114_HASH,
        "the dedicated pre-v114 probe must reproduce the prior bridge current baseline"
    );
    assert_eq!(
        pre_wall_runtime_hash, BRIDGE_HARNESS_PRE_WALL_RUNTIME_V115_HASH,
        "the dedicated pre-v115 probe must reproduce the prior bridge current baseline"
    );
    assert_eq!(
        final_hash, BRIDGE_HARNESS_FINAL_HASH,
        "committed bridge-harness baseline drifted. Do not paste the observed value \
         until the tripwires above are still green and you can say which behavior \
         moved; this constant is a Rust-vs-prior-Rust ratchet, not gamemd evidence"
    );
}
