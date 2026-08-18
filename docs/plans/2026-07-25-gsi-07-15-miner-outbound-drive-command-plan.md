# GSI-07.15 Miner Outbound Drive Command Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained.

Date: 2026-07-25  
Status: REPAIRED AFTER FIX-FIRST REVIEW; RE-REVIEW REQUIRED  
Committed planning base: `dev` `6e5a3d2f172be23ebffd996777c3d586146030f3`

**Goal:** Make stock HARV and CMIN hand a selected outbound ore cell to the
existing normal Drive destination/path authority with the merged-rule speed
profile, while preserving CMIN's primary-Teleport/active-Drive piggyback
ownership and leaving every heterogeneous scripted direct-movement caller
unchanged.

**Architecture:** The miner mission remains the target-selection and command
producer. A private adapter in `miner_system.rs` reuses the existing
`Simulation::resolve_move_info`, layered movement command, NavCom,
DriveLocomotion, DriveTrack, and locomotor-piggyback authorities. Dedicated
merged-retail tests exercise the public `Simulation::advance_tick` loop without
adding state or widening the generic movement contract.

**Design Doc:**
`docs/plans/2026-07-25-gsi-07-15-miner-outbound-drive-command-design.md`

**Implementation Contract:**
`docs/contracts/2026-07-25-gsi-07-15-miner-outbound-drive-command-implementation-contract.md`

---

## Grounding Summary

- The verified active-YR state-0 path calls
  `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`; a selected
  non-current cell reaches the UnitClass destination virtual.
- `FootClass::Set_Destination_Internal @ 0x004D94B0` writes owner NavCom before
  the active locomotor receives `Head_To_Coord`.
- The asm-verified Teleporter block at `0x007423CD..0x007427C0` sends a stock
  CMIN with old NavCom `NULL` through Drive piggyback for outbound ore travel;
  state 0 cannot arm the return teleport.
- Current `handle_move_to_ore` instead splits adjacent ore through
  `issue_direct_move` and farther ore through a reduced command, then exempts
  the path from terrain cost.
- `issue_move_command_with_layered` already owns A*, NavCom, Drive
  destination/head-to, Drive directions, turn state, and DriveTrack setup.
- `Simulation::resolve_move_info` is the current merged-rule/entity snapshot,
  and the player-command producer already demonstrates the feasible disjoint
  borrow and three-field profile stamp.
- `begin_drive_piggyback_for_teleporter` mutates only `kind`, `primary_kind`
  indirectly through its read, `piggyback`, `layer`, and `phase`; exact
  transaction rollback therefore snapshots and writes back those five stored
  fields. `restore_primary_from_piggyback` is not an undo primitive.
- Retail CMIN and HARV are `Harvester=yes`, `Speed=4`, `ROT=5`,
  `Crusher=yes`, `MovementZone=Crusher`; CMIN is `Teleporter=yes` with
  Teleport primary locomotor, while HARV uses Drive.
- Both stock types omit `Accelerates`, `AccelerationFactor`,
  `DeaccelerationFactor`, and `SlowdownDistance`, so merged parser defaults are
  `true`, `0.03`, `0.002`, and `500` leptons.
- Retail `[Tiberium] Track=70%`; the target belongs under normal terrain-cost
  authority. The test grid must copy merged `[Clear]` and `[Tiberium]`
  `SpeedCostProfile` values into resolved cells before building cost grids.
- An adjacent 256-lepton target is inside the 500-lepton slowdown distance and
  takes the exact 0.3 destination-brake floor. A target three cells away starts
  outside slowdown and gains exactly the parsed acceleration factor.
- Current tick ordering is Phase 1 movement, Phase 2 piggyback restoration, and
  Phase 7 miner dispatch. A Phase-7 issue moves on the following tick, and the
  Phase-2 restore gate remains closed while `MovementTarget` exists.
- Fresh plan review re-decompiled `0x004DCFE0` and confirmed that owner NavCom
  is checked before every other scan-wrapper operation. The caller gate must
  therefore precede target validation and physical arrival, not merely another
  scan/issue.
- Native track completion clears arrived Drive state before
  `FootClass::AI @ 0x004DA530` releases the old active Drive locomotor on
  piggyback completion. Rust currently restores primary Teleport while retaining
  `DriveLocomotionRuntime`; this plan now retires that runtime on successful
  restoration.
- The research-index brief for the outbound HARV/CMIN Drive handoff returned
  the same verified reports and Rust touchpoints; it exposed no missing
  prerequisite or contradictory active-YR evidence.
- `dev` advanced after design approval from `5130d139` to `6e5a3d2f`, but both
  new commits touch only RMG/app paths. None of the owned or depended-on miner,
  movement, rules, or world-command files changed.
- Full DriveTrack cadence, blocked/repath continuation, collision, bridge,
  exact arrival-frame, rendering, and inbound CMIN warp parity remain
  explicitly unverified outside this prerequisite.

## Key Technical Decisions

- Use a miner-private producer adapter instead of changing
  `issue_direct_move`. **Confidence: high**
  - **Source:** verified native state-0 destination chain; current heterogeneous
    direct-move callsite census; approved design.
- Route adjacent and farther outbound ore through the same layered command.
  **Confidence: high**
  - **Source:** `0x004DCFE0`, `0x004D94B0`, retail
    `[Tiberium] Track=70%`, and current layered command implementation.
- Resolve `MoveInfo` once and stamp `accel_factor`, `decel_factor`, and
  `slowdown_distance` only after successful issue. **Confidence: high**
  - **Source:** compiled player-command pattern in
    `src/sim/world/world_commands.rs`.
- Activate Drive over primary Teleport only for the resolved
  teleporter-harvester case, and restore the exact five-field tuple after a
  synchronous false return. **Confidence: high**
  - **Source:** asm-verified Teleporter predicate and current
    `LocomotorState` mutation surface.
- Treat either owner NavCom or transitional `MovementTarget` as a function-entry
  gate before target validation, teleport/arrival checks, rescan, or issue.
  **Confidence: high**
  - **Source:** fresh `decompile_function(0x004DCFE0)`, which reads
    `param_1[0x169]` first; Drive arrival/NavCom lifecycle reports.
- On successful primary-Teleport restoration, remove the retired Drive runtime
  and Drive track. **Confidence: high**
  - **Source:** `FootClass::AI @ 0x004DAE5F..0x004DAEC6` releases the old active
    locomotor before installing the stored one; current Rust state-hash surface.
- Use no entity-block set/map in the new adapter. **Confidence: medium**
  - **Source:** the existing miner producer has no entity-block-map authority;
    the live scan filter already checks occupancy and reachability. Full
    collision/repath authority is excluded.
- Keep the failure retry behavior unchanged: retain `MoveToOre` and the selected
  target after a false issue. **Confidence: medium**
  - **Source:** current Rust behavior. This Rust-only failure branch is a
    transactional safety requirement, not a certified native retry cadence.

The two medium-confidence decisions are explicit review targets. Neither may be
silently broadened into a full blocked-path parity claim.

## Open Questions

### Resolved During Planning

- **Does an adjacent stock target use the acceleration branch?** No. At 256
  leptons it is inside stock `SlowdownDistance=500`, so the first normal Drive
  frame reaches the 0.3 brake floor. A separate three-cell fixture proves the
  acceleration branch.
- **Can failed piggyback activation be undone through the lifecycle restore
  method?** No. The method resets phase and primary state. Direct restoration of
  `(kind, primary_kind, piggyback, layer, phase)` is required.
- **What suppresses another scan after command handoff?** Owner NavCom is the
  verified native gate. Rust's `MovementTarget` remains a transitional
  additional gate for existing in-flight states.
- **Does the gate run after target validation or physical arrival?** No.
  `0x004DCFE0` reads NavCom first and returns immediately when it is non-null.
  Removed-target validation and arrival state progression must wait until both
  Rust owner representations are absent.
- **May a restored CMIN keep its old Drive runtime?** No. Native
  `FootClass::AI` releases the old active Drive locomotor when piggyback ends.
  Rust must drop `DriveLocomotionRuntime` and `drive_track` after successful
  restoration.
- **Can the real production test make Tiberium walkable for Track?** Yes, when
  the resolved ore cell carries the merged `[Tiberium]` speed profile before
  `TerrainCostGrid` construction; Track cost is 70.
- **Did the post-design RMG merge restructure any dependency?** No. The
  `5130d139..6e5a3d2f` name-status diff is limited to `src/app*` and
  `src/map/rmg/**`.

### Deferred to Implementation

- The literal number of ticks until physical arrival is not a parity
  certification. The production oracle uses a bounded window and asserts the
  mechanism/state transitions, while full DriveTrack cadence remains residual.
- The rollback oracle is expected to pass before the production edit because
  the old producer never activates CMIN Drive. Its role is to prevent the new
  adapter from leaking partial locomotor state.
- If the layered command mutates any entity field before returning `false` on
  the chosen no-path fixture, execution stops and the contract is repaired
  before implementation continues.
- If `dev` changes any owned or depended-on file before worktree creation,
  re-run the overlap audit and update the frozen base before any Rust edit.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `src/sim/miner/outbound_drive_tests.rs` | Merged-retail production-loop oracles for outbound command ownership, speed profile, CMIN piggyback, rollback, NavCom gating, RNG, and physical travel |
| Modify | `src/sim/miner/mod.rs` | Register the separate test-only module |
| Modify | `src/sim/miner/miner_system.rs` | Add the private outbound producer adapter and replace the direct/reduced dispatch split |
| Modify | `src/sim/movement/mod.rs` | Release retired Drive runtime/track after successful primary-locomotor restoration |
| Append | `docs/goals/2026-07-24-system-by-system-parity-state.md` | Literal red/green/build/commit/merge results, residuals, and next action |

Deliberately unchanged:

- `src/sim/movement/movement_commands.rs`
- `src/sim/movement/locomotor.rs`
- `src/sim/world/world_commands.rs`
- `src/sim/components.rs`
- serialization, snapshots, state hashing, render, UI, sidebar, audio, and net

`miner_system.rs` is already larger than the nominal module target. This slice
adds only one cohesive private producer helper; all new test volume goes into a
separate file, avoiding growth of the existing large `miner_tests.rs` and the
suspended parent's owned diff.

## Interface Changes

No public API, trait, struct, enum, serialization schema, INI schema, or command
schema changes.

One private function is added:

```rust
fn issue_outbound_ore_move(
    sim: &mut Simulation,
    rules: &RuleSet,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
) -> bool
```

Only `handle_move_to_ore` calls it. Existing `issue_move_if_idle` remains for
its three other miner flows. The global `issue_direct_move` contract and all
other consumers remain unchanged.

## Sim Checklist

- [x] All production math remains `SimFixed`; no `f32`/`f64` is added.
- [x] No new state is added, so no deterministic-state-hash change is needed.
- [x] No dependency on render, UI, sidebar, audio, or net is introduced.
- [x] Tick ordering is unchanged; the plan explicitly tests Phase-7 issue then
  next-tick Phase-1 movement, Phase-2 restoration/release, pending NavCom, and
  later owner clear.
- [x] No EntityStore iteration is added; entity access is by stable ID.
- [x] No RNG call or per-tick collection allocation is added.
- [x] Existing NavCom, locomotor, Drive, DriveTrack, and MovementTarget state
  remains under its current serialization/hash owner.

## Risk Areas

- **CMIN ownership leak on false A\*:** mitigated by exact five-field snapshot
  and rollback plus a real production failure oracle.
- **Owner gate ordered too late:** separate removed-target and arrival-order
  production tests prove validation and Harvest progression wait for NavCom.
- **Generic movement semantics broaden:** guarded by a source-diff assertion
  over the three deliberately unchanged shared files and focused neighbor tests.
- **Terrain fixture lies about retail:** merged `[Clear]` and `[Tiberium]`
  profiles are copied into resolved cells, Track cost 70 is asserted, and no
  bypass flag is allowed.
- **Adjacent test proves the wrong speed branch:** exact 0.3 floor and separate
  far-target exact acceleration assertions prevent conflation.
- **CMIN restores Teleport but leaks retired Drive state:** every moving tick
  asserts active Drive/primary Teleport/piggyback; the arrival oracle requires
  primary Teleport, no piggyback, and no `DriveLocomotionRuntime` after
  restoration.
- **NavCom test only proves no command, not no scan:** a newly inserted
  preferable adjacent ore node makes a scan observable through target
  replacement.
- **Concurrent integration drift:** feature worktree creation freezes an exact
  reviewed base; guarded merge requires no tracked root changes and allows only
  the protected untracked `system_map/` and `tools/system_map/` paths.
- **Global Cargo lease collision:** every Cargo command is preceded by a
  cargo/rustc process check and commands run serially.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|---|---|---|---|
| 2 | Old NavCom `NULL` before CMIN issue | Selects the verified default Drive-piggyback branch | Explicit pre-issue production assertion |
| 2 | Active Drive, primary Teleport, stored piggyback | Determines whether outbound CMIN drives or wrongly warps | Per-tick state assertions through arrival |
| 2 | Owner NavCom written before Drive execution | Native destination owner suppresses another scan/dispatch | NavCom/destination assertions and preferable-candidate negative oracle |
| 2 | Owner gate precedes validation/arrival | Removed ore or physical arrival must not outrun native NavCom ownership | Removed-target and pending-arrival production oracles |
| 2 | Retired CMIN Drive runtime released | Native FootClass AI releases the old active locomotor on piggyback completion | No Drive runtime after real arrival restoration |
| 2 | Layered path used even for adjacent ore | Direct two-cell movement omits native-shaped owners | Path, NavCom, Drive direction, and no-bypass assertions |
| 2 | Merged rule profile | Zero or hardcoded fields change speed bytes/timing | Exact ObjectType-to-MovementTarget comparison |
| 2 | Adjacent 0.3 brake floor | A one-frame/fraction difference is real drift | First normal Drive frame exact equality |
| 2 | Far-target `+AccelerationFactor` | Proves stock accelerating Drive does not remain at zero | Exact parsed factor and current-speed equality |
| 2 | No terrain/grid bypass | Retail Track Tiberium is 70%, not exempt | Terrain cost 70 plus false bypass flags |
| 2 | Exact failed-issue rollback | Partial active-Drive state would corrupt later locomotor lifecycle | Five-field tuple equality |
| 2 | No RNG consumption | Extra draws desynchronize lockstep and later gameplay | Full three-stream state equality |
| 3 | Existing direct/scripted callers unchanged | Refinery, passenger, sell, and crush flows have different contracts | Source diff guard and focused regressions |
| 6 | Parent ring-1 physical travel | The prerequisite exists only to unblock the real GSI-07.15 loop | Seven prerequisite plus six parent tests after replay |

---

## Tasks

### Task 1: Freeze the owned feature worktree and hydrate retail fixtures

**Why:** Isolate the implementation from integration-only `dev`, the suspended
parent, the protected damage worktree, and concurrent RMG/system-map work.

**Files:** No tracked source edit.

**Pattern:** Existing goal-contract feature-worktree ownership and one global
Cargo lease.

**Step 1: Reconcile immediately before creation**

Run from `.`:

```powershell
git rev-parse HEAD
git status --short --branch
git diff --name-status 6e5a3d2f172be23ebffd996777c3d586146030f3..HEAD -- src/sim/miner/mod.rs src/sim/miner/miner_system.rs src/sim/movement/mod.rs src/sim/movement/locomotor.rs src/sim/movement/movement_commands.rs src/sim/world/world_commands.rs src/rules/object_type.rs
git worktree list --porcelain
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU,StartTime
```

Expected:

- HEAD is `6e5a3d2f172be23ebffd996777c3d586146030f3`;
- root has no tracked change;
- the only root untracked paths are the protected `system_map/` and
  `tools/system_map/`;
- the scoped diff is empty;
- no Cargo/rustc process is active.

Any mismatch is a hard stop for another state reconciliation, not permission to
touch unowned work.

**Step 2: Create the feature branch and worktree**

```powershell
git worktree add -b feature/gsi-07-15-miner-outbound-drive-prereq-20260725-122214 <local>/Documents/ra2-rust-game-gsi-07-15-miner-outbound-drive-prereq-20260725-122214 6e5a3d2f172be23ebffd996777c3d586146030f3
```

Expected: the worktree is on the named branch at the exact reviewed base.

**Step 3: Hydrate only the six ignored retail INIs required by the tests and
crate compile-time fixtures**

```powershell
$sourceIni = 'ini'
$featureIni = '<local>/Documents/ra2-rust-game-gsi-07-15-miner-outbound-drive-prereq-20260725-122214/ini'
New-Item -ItemType Directory -Path $featureIni -Force
Copy-Item -LiteralPath "$sourceIni/rules.ini" -Destination "$featureIni/rules.ini"
Copy-Item -LiteralPath "$sourceIni/rulesmd.ini" -Destination "$featureIni/rulesmd.ini"
Copy-Item -LiteralPath "$sourceIni/art.ini" -Destination "$featureIni/art.ini"
Copy-Item -LiteralPath "$sourceIni/artmd.ini" -Destination "$featureIni/artmd.ini"
Copy-Item -LiteralPath "$sourceIni/mpmodesmd.ini" -Destination "$featureIni/mpmodesmd.ini"
Copy-Item -LiteralPath "$sourceIni/temperatmd.ini" -Destination "$featureIni/temperatmd.ini"
Get-FileHash -Algorithm SHA256 "$featureIni/rules.ini","$featureIni/rulesmd.ini","$featureIni/art.ini","$featureIni/artmd.ini","$featureIni/mpmodesmd.ini","$featureIni/temperatmd.ini"
```

Expected: all six files exist and remain Git-ignored. `mpmodesmd.ini` and
`temperatmd.ini` are not read by the new miner fixture; they satisfy existing
crate `include_str!` declarations in `src/skirmish_modes.rs` and
`src/map/rmg/tiles.rs`. Record all literal hashes in the operational journal.

### Task 2: Add red-first merged-retail outbound Drive production oracles

**Why:** Pin command ownership, exact speed branches, transactional failure,
NavCom authority, RNG, and real physical travel before changing production.

**Files:**

- Modify: `src/sim/miner/mod.rs`
- Create: `src/sim/miner/outbound_drive_tests.rs`

**Pattern:** The canonical-spawn and real-`advance_tick` pattern already used by
the suspended parent's production tests, corrected to populate retail terrain
speed profiles.

**Step 1: Register the isolated test module**

Add immediately after the existing `miner_tests` registration:

```rust
#[cfg(test)]
#[path = "outbound_drive_tests.rs"]
mod outbound_drive_tests;
```

**Step 2: Create the complete production test module**

Create `src/sim/miner/outbound_drive_tests.rs` with:

```rust
//! Merged-retail production oracles for stock miner outbound Drive commands.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::map::bridge_facts::BridgeCellFacts;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid, zone_class};
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::sim::components::{DriveCoord, NavTargetRef};
use crate::sim::miner::{MinerKind, MinerState, ResourceNode, ResourceType};
use crate::sim::movement::locomotor::{
    GroundMovePhase, MovementLayer, PiggybackLocomotor,
};
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::passability::LandType;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
use crate::sim::world::Simulation;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, ra2_speed_to_leptons_per_second};

const GRID_SIZE: u16 = 64;
const START: (u16, u16) = (32, 32);
const ONE_ORE_LEVEL: u16 = 120;

struct RetailOutboundOracle {
    rules: RuleSet,
    overlays: OverlayTypeRegistry,
    tib01: u8,
    clear_speed_costs: SpeedCostProfile,
    tiberium_speed_costs: SpeedCostProfile,
}

fn merged_ini(base: &str, patch: &str) -> IniFile {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ini = IniFile::from_str(
        &fs::read_to_string(root.join(base))
            .unwrap_or_else(|error| panic!("read {base}: {error}")),
    );
    let patch_ini = IniFile::from_str(
        &fs::read_to_string(root.join(patch))
            .unwrap_or_else(|error| panic!("read {patch}: {error}")),
    );
    ini.merge(&patch_ini);
    ini
}

fn retail_outbound_oracle() -> RetailOutboundOracle {
    let rules_ini = merged_ini("ini/rules.ini", "ini/rulesmd.ini");
    let mut rules = RuleSet::from_ini(&rules_ini).expect("merged retail rules");
    let art_ini = merged_ini("ini/art.ini", "ini/artmd.ini");
    rules.merge_art_data(&ArtRegistry::from_ini(&art_ini));
    let overlays = OverlayTypeRegistry::from_ini(&rules_ini, None);
    let tib01 = overlays.id_for_name("TIB01").expect("retail TIB01");
    let clear_speed_costs = rules
        .terrain_rules
        .semantics_by_name("Clear")
        .expect("merged [Clear]")
        .speed_costs;
    let tiberium_speed_costs = rules
        .terrain_rules
        .semantics_by_name("Tiberium")
        .expect("merged [Tiberium]")
        .speed_costs;

    assert_eq!(tiberium_speed_costs.track, Some(70));
    assert!(overlays.flags(tib01).is_some_and(|flags| flags.tiberium));
    for (type_id, expected_locomotor, teleporter) in [
        ("HARV", LocomotorKind::Drive, false),
        ("CMIN", LocomotorKind::Teleport, true),
    ] {
        let object = rules
            .object(type_id)
            .unwrap_or_else(|| panic!("retail {type_id}"));
        assert!(object.harvester, "{type_id} Harvester=yes");
        assert_eq!(object.speed, 4, "{type_id} Speed=4");
        assert_eq!(object.turret_rot, 5, "{type_id} ROT=5");
        assert!(object.crusher, "{type_id} Crusher=yes");
        assert_eq!(object.movement_zone, MovementZone::Crusher);
        assert_eq!(object.speed_type, SpeedType::Track);
        assert_eq!(object.locomotor, expected_locomotor);
        assert_eq!(object.teleporter, teleporter);
        assert!(object.accelerates);
        assert_eq!(object.accel_factor, SimFixed::lit("0.03"));
        assert_eq!(object.decel_factor, SimFixed::lit("0.002"));
        assert_eq!(object.slowdown_distance, 500);
    }

    RetailOutboundOracle {
        rules,
        overlays,
        tib01,
        clear_speed_costs,
        tiberium_speed_costs,
    }
}

fn production_sim(seed: u64, oracle: &RetailOutboundOracle) -> Simulation {
    let mut sim = Simulation::with_seed(seed);
    oracle.rules.intern_all_ids(&mut sim.interner);
    sim.resolve_type_handles(&oracle.rules);
    sim
}

fn resolved_cell(
    rx: u16,
    ry: u16,
    terrain_class: TerrainClass,
    land_type: u8,
    speed_costs: SpeedCostProfile,
) -> ResolvedTerrainCell {
    ResolvedTerrainCell {
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
        land_type,
        yr_cell_land_type: land_type,
        slope_type: 0,
        template_height: 0,
        render_offset_x: 0,
        render_offset_y: 0,
        terrain_class,
        speed_costs,
        is_water: false,
        is_cliff_like: false,
        is_rough: false,
        is_road: false,
        accepts_smudge: false,
        allows_tiberium: false,
        is_cliff_redraw: false,
        variant: 0,
        has_ramp: false,
        canonical_ramp: None,
        ground_walk_blocked: false,
        terrain_object_blocks: false,
        overlay_blocks: false,
        zone_type: zone_class::GROUND,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: LandType::Clear.as_index(),
        base_yr_cell_land_type: LandType::Clear.as_index(),
        base_terrain_class: TerrainClass::Clear,
        base_speed_costs: speed_costs,
        build_blocked: false,
        has_bridge_deck: false,
        bridge_walkable: false,
        bridge_transition: false,
        bridge_deck_level: 0,
        bridge_layer: None,
        bridge_facts: BridgeCellFacts::default(),
        tube_index: None,
        radar_left: [0, 0, 0],
        radar_right: [0, 0, 0],
        has_damaged_data: false,
        bridgehead_anchor_class_at_load: None,
    }
}

fn staged_terrain(
    oracle: &RetailOutboundOracle,
    ore_cells: &[(u16, u16)],
) -> ResolvedTerrainGrid {
    let clear_land_type = LandType::Clear.as_index();
    let mut terrain = ResolvedTerrainGrid::from_cells(
        GRID_SIZE,
        GRID_SIZE,
        (0..GRID_SIZE)
            .flat_map(|ry| {
                (0..GRID_SIZE).map(move |rx| {
                    resolved_cell(
                        rx,
                        ry,
                        TerrainClass::Clear,
                        clear_land_type,
                        oracle.clear_speed_costs,
                    )
                })
            })
            .collect(),
    );
    for &(rx, ry) in ore_cells {
        let cell = terrain.cell_mut(rx, ry).expect("staged ore cell");
        let tiberium_land_type = LandType::Tiberium.as_index();
        cell.land_type = tiberium_land_type;
        cell.yr_cell_land_type = tiberium_land_type;
        cell.terrain_class = TerrainClass::Tiberium;
        cell.speed_costs = oracle.tiberium_speed_costs;
        cell.allows_tiberium = true;
    }
    terrain
}

fn install_world(
    sim: &mut Simulation,
    oracle: &RetailOutboundOracle,
    grid: &PathGrid,
    ore_cells: &[(u16, u16)],
    nodes: &[(u16, u16)],
    install_zones: bool,
) {
    let terrain = staged_terrain(oracle, ore_cells);
    sim.terrain_costs = SpeedType::ALL_WITH_COSTS
        .iter()
        .copied()
        .map(|speed_type| {
            (
                speed_type,
                TerrainCostGrid::from_resolved_terrain(&terrain, speed_type),
            )
        })
        .collect();
    sim.resolved_terrain = Some(terrain);
    sim.overlay_grid = Some(OverlayGrid::new(GRID_SIZE, GRID_SIZE));
    for &(rx, ry) in ore_cells {
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(rx, ry, oracle.tib01, 0);
    }
    for &cell in nodes {
        sim.production.resource_nodes.insert(
            cell,
            ResourceNode {
                resource_type: ResourceType::Ore,
                remaining: ONE_ORE_LEVEL,
            },
        );
    }
    if install_zones {
        sim.rebuild_zone_grid(grid);
        assert!(sim.zone_grid.is_some());
    } else {
        sim.zone_grid = None;
    }
    for &(rx, ry) in ore_cells {
        assert_eq!(
            sim.terrain_costs
                .get(&SpeedType::Track)
                .expect("Track terrain costs")
                .cost_at(rx, ry),
            70,
        );
    }
}

fn spawn_stock_miner(
    sim: &mut Simulation,
    oracle: &RetailOutboundOracle,
    type_id: &str,
    expected_kind: MinerKind,
) -> u64 {
    let id = sim
        .spawn_object(
            type_id,
            "Americans",
            START.0,
            START.1,
            0,
            &oracle.rules,
            &BTreeMap::new(),
        )
        .unwrap_or_else(|| panic!("spawn {type_id}"));
    let entity = sim.substrate.entities.get(id).expect("spawned miner");
    assert!(entity.lifecycle.object_alive);
    assert!(!entity.lifecycle.in_limbo);
    assert!(entity.lifecycle.cell_marked);
    assert!(entity.in_logic_vector);
    assert!(sim.live_object_order_snapshot().contains(&id));
    assert_eq!(
        entity.miner.as_ref().expect("miner component").kind,
        expected_kind,
    );
    assert_eq!(
        entity
            .locomotor
            .as_ref()
            .expect("stock locomotor")
            .movement_zone,
        MovementZone::Crusher,
    );
    id
}

fn arm_search(sim: &mut Simulation, entity_id: u64) {
    let entity = sim
        .substrate
        .entities
        .get_mut(entity_id)
        .expect("miner entity");
    let miner = entity.miner.as_mut().expect("miner component");
    miner.state = MinerState::SearchOre;
    miner.target_ore_cell = None;
    miner.harvest_timer.clear();
}

fn advance(sim: &mut Simulation, oracle: &RetailOutboundOracle, grid: &PathGrid) {
    let _ = sim.advance_tick(
        &[],
        Some(&oracle.rules),
        &BTreeMap::new(),
        Some(grid),
        Some(&oracle.overlays),
        67,
    );
}

fn position_tuple(
    sim: &Simulation,
    entity_id: u64,
) -> (u16, u16, SimFixed, SimFixed) {
    let position = &sim
        .substrate
        .entities
        .get(entity_id)
        .expect("entity")
        .position;
    (position.rx, position.ry, position.sub_x, position.sub_y)
}

fn assert_ore_intact(
    sim: &Simulation,
    oracle: &RetailOutboundOracle,
    target: (u16, u16),
) {
    let node = sim
        .production
        .resource_nodes
        .get(&target)
        .expect("positive ore node");
    assert_eq!(node.resource_type, ResourceType::Ore);
    assert_eq!(node.remaining, ONE_ORE_LEVEL);
    let overlay = sim
        .overlay_grid
        .as_ref()
        .expect("overlay grid")
        .cell(target.0, target.1);
    assert_eq!(overlay.overlay_id, Some(oracle.tib01));
    assert_eq!(overlay.overlay_data, 0);
}

fn assert_command_state(
    sim: &Simulation,
    oracle: &RetailOutboundOracle,
    entity_id: u64,
    type_id: &str,
    target: (u16, u16),
) {
    let object = oracle.rules.object(type_id).expect("retail miner type");
    let entity = sim.substrate.entities.get(entity_id).expect("miner entity");
    let movement = entity.movement_target.as_ref().expect("movement target");
    assert_eq!(movement.path.first().copied(), Some(START));
    assert_eq!(movement.path.last().copied(), Some(target));
    assert_eq!(movement.final_goal, Some(target));
    assert_eq!(
        movement.speed,
        ra2_speed_to_leptons_per_second(object.speed),
    );
    assert_eq!(movement.accel_factor, object.accel_factor);
    assert_eq!(movement.decel_factor, object.decel_factor);
    assert_eq!(
        movement.slowdown_distance,
        SimFixed::from_num(object.slowdown_distance),
    );
    assert!(!movement.ignore_terrain_cost);
    assert!(!movement.bypass_grid);
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(target.0, target.1)),
    );
    let expected_coord = DriveCoord::cell(target.0, target.1, 0);
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    assert_eq!(drive.destination, Some(expected_coord));
    assert_eq!(drive.head_to, Some(expected_coord));
    assert_eq!(
        drive.path.directions.len(),
        movement.path.len().saturating_sub(1),
    );
    assert!(!drive.path.directions.is_empty());
    assert_eq!(drive.current_speed_fraction, SIM_ZERO);
    assert_eq!(
        entity.locomotor.as_ref().expect("active locomotor").kind,
        LocomotorKind::Drive,
    );
}

fn locomotor_tuple(
    sim: &Simulation,
    entity_id: u64,
) -> (
    LocomotorKind,
    Option<LocomotorKind>,
    Option<PiggybackLocomotor>,
    MovementLayer,
    GroundMovePhase,
) {
    let locomotor = sim
        .substrate
        .entities
        .get(entity_id)
        .and_then(|entity| entity.locomotor.as_ref())
        .expect("locomotor");
    (
        locomotor.kind,
        locomotor.primary_kind,
        locomotor.piggyback,
        locomotor.layer,
        locomotor.phase,
    )
}

#[test]
fn production_stock_miners_use_drive_command_for_adjacent_ore() {
    let oracle = retail_outbound_oracle();
    let target = (32, 31);
    for (type_id, kind) in [("HARV", MinerKind::War), ("CMIN", MinerKind::Chrono)] {
        let mut sim = production_sim(0x0715_D001, &oracle);
        let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
        install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
        let entity_id = spawn_stock_miner(&mut sim, &oracle, type_id, kind);
        let start_position = position_tuple(&sim, entity_id);
        arm_search(&mut sim, entity_id);
        let rng_before_search = sim.rng_state();

        advance(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before_search, "{type_id} search RNG");
        let miner = sim
            .substrate
            .entities
            .get(entity_id)
            .and_then(|entity| entity.miner.as_ref())
            .expect("miner");
        assert_eq!(miner.state, MinerState::MoveToOre);
        assert_eq!(miner.target_ore_cell, Some(target));

        if type_id == "CMIN" {
            let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(entity.navigation.nav_com, None);
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
            assert_eq!(locomotor.piggyback, None);
        }

        let rng_before_issue = sim.rng_state();
        advance(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before_issue, "{type_id} issue RNG");
        assert_command_state(&sim, &oracle, entity_id, type_id, target);
        {
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            let locomotor = entity.locomotor.as_ref().expect("locomotor");
            if type_id == "CMIN" {
                assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
                assert_eq!(
                    locomotor.piggyback.expect("CMIN Drive piggyback").kind,
                    LocomotorKind::Teleport,
                );
            } else {
                assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Drive));
                assert_eq!(locomotor.piggyback, None);
            }
            assert!(entity.teleport_state.is_none());
        }

        advance(&mut sim, &oracle, &grid);
        {
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
            let movement = entity.movement_target.as_ref().expect("movement");
            assert_eq!(drive.current_speed_fraction, SimFixed::lit("0.3"));
            assert_eq!(
                movement.current_speed,
                movement.speed * SimFixed::lit("0.3"),
            );
        }

        let mut physically_departed = position_tuple(&sim, entity_id) != start_position;
        let mut reached_harvest = false;
        for _ in 0..128 {
            advance(&mut sim, &oracle, &grid);
            let entity = sim.substrate.entities.get(entity_id).expect("miner");
            physically_departed |= position_tuple(&sim, entity_id) != start_position;
            reached_harvest |=
                entity.miner.as_ref().expect("miner").state == MinerState::Harvest;
            assert!(entity.teleport_state.is_none());
            if type_id == "CMIN" && entity.movement_target.is_some() {
                let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
                assert_eq!(locomotor.kind, LocomotorKind::Drive);
                assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
                assert!(locomotor.piggyback.is_some());
            }
            if reached_harvest {
                break;
            }
        }
        assert!(physically_departed, "{type_id} must leave {START:?}");
        assert!(reached_harvest, "{type_id} must reach Harvest");
        let entity = sim.substrate.entities.get(entity_id).expect("miner");
        assert_eq!(entity.navigation.nav_com, None);
        assert!(!entity.navigation.pending_arrival_clear);
        if type_id == "CMIN" {
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
            assert_eq!(locomotor.piggyback, None);
            assert!(
                entity.drive_locomotion.is_none(),
                "native FootClass::AI releases retired Drive"
            );
        }
        assert_eq!(
            sim.rng_state(),
            rng_before_search,
            "{type_id} outbound RNG"
        );
        assert_ore_intact(&sim, &oracle, target);
    }
}

#[test]
fn production_harv_outbound_drive_uses_rule_profile() {
    let oracle = retail_outbound_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D002, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "HARV", target);
    let harv = oracle.rules.object("HARV").expect("HARV");
    assert!(3 * 256 > harv.slowdown_distance);
    let acceleration = harv.accel_factor;

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let drive = entity.drive_locomotion.as_ref().expect("Drive runtime");
    let movement = entity.movement_target.as_ref().expect("movement");
    assert_eq!(drive.current_speed_fraction, acceleration);
    assert_eq!(movement.current_speed, movement.speed * acceleration);
    assert!(movement.current_speed > SIM_ZERO);
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_cmin_outbound_drive_keeps_teleport_primary() {
    let oracle = retail_outbound_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D003, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    {
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
        assert_eq!(entity.navigation.nav_com, None);
        assert_eq!(locomotor.kind, LocomotorKind::Teleport);
        assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
        assert_eq!(locomotor.piggyback, None);
    }

    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "CMIN", target);
    {
        let locomotor = sim
            .substrate
            .entities
            .get(entity_id)
            .and_then(|entity| entity.locomotor.as_ref())
            .expect("CMIN locomotor");
        assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
        assert!(locomotor.piggyback.is_some());
    }

    let mut reached_harvest = false;
    for _ in 0..240 {
        advance(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        assert!(entity.teleport_state.is_none());
        if entity.movement_target.is_some() {
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Drive);
            assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
            assert!(locomotor.piggyback.is_some());
        }
        if entity.miner.as_ref().expect("miner").state == MinerState::Harvest {
            assert!(entity.movement_target.is_none());
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
            assert_eq!(locomotor.piggyback, None);
            assert_eq!(entity.navigation.nav_com, None);
            assert!(!entity.navigation.pending_arrival_clear);
            assert!(entity.drive_locomotion.is_none());
            reached_harvest = true;
            break;
        }
    }
    assert!(reached_harvest);
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_cmin_failed_outbound_issue_restores_locomotor_exactly() {
    let oracle = retail_outbound_oracle();
    let target = (32, 29);
    let mut sim = production_sim(0x0715_D004, &oracle);
    let mut grid = PathGrid::test_all_blocked(GRID_SIZE, GRID_SIZE);
    grid.set_blocked(START.0, START.1, false);
    grid.set_blocked(target.0, target.1, false);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], false);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    let before = locomotor_tuple(&sim, entity_id);
    assert_eq!(before.0, LocomotorKind::Teleport);
    assert_eq!(before.1, Some(LocomotorKind::Teleport));
    assert_eq!(before.2, None);
    assert_eq!(
        sim.substrate
            .entities
            .get(entity_id)
            .expect("CMIN")
            .navigation
            .nav_com,
        None,
    );

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
    assert!(entity.movement_target.is_none());
    assert_eq!(entity.navigation.nav_com, None);
    assert_eq!(locomotor_tuple(&sim, entity_id), before);
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_harv_navcom_without_movement_target_is_not_reissued() {
    let oracle = retail_outbound_oracle();
    let original = (32, 29);
    let preferable = (32, 31);
    let mut sim = production_sim(0x0715_D005, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(
        &mut sim,
        &oracle,
        &grid,
        &[original, preferable],
        &[original],
        true,
    );
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_eq!(
        sim.substrate
            .entities
            .get(entity_id)
            .expect("HARV")
            .navigation
            .nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );

    {
        let entity = sim.substrate.entities.get_mut(entity_id).expect("HARV");
        entity.movement_target = None;
    }
    sim.production.resource_nodes.insert(
        preferable,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: ONE_ORE_LEVEL,
        },
    );
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    assert_eq!(
        entity.miner.as_ref().expect("miner").target_ore_cell,
        Some(original),
    );
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );
    assert!(
        entity.movement_target.is_none(),
        "non-null NavCom must suppress scan and command reissue",
    );
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_harv_navcom_defers_removed_target_revalidation() {
    let oracle = retail_outbound_oracle();
    let original = (32, 29);
    let replacement = (32, 31);
    let mut sim = production_sim(0x0715_D006, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(
        &mut sim,
        &oracle,
        &grid,
        &[original, replacement],
        &[original],
        true,
    );
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "HARV", MinerKind::War);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "HARV", original);

    {
        let entity = sim.substrate.entities.get_mut(entity_id).expect("HARV");
        entity.movement_target = None;
        assert_eq!(
            entity.navigation.nav_com,
            Some(NavTargetRef::cell(original.0, original.1)),
            "fixture must isolate the native NavCom owner gate",
        );
    }
    sim.production.resource_nodes.remove(&original);
    sim.production.resource_nodes.insert(
        replacement,
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: ONE_ORE_LEVEL,
        },
    );
    let rng_before = sim.rng_state();

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("HARV");
    let miner = entity.miner.as_ref().expect("miner");
    assert_eq!(miner.state, MinerState::MoveToOre);
    assert_eq!(miner.target_ore_cell, Some(original));
    assert_eq!(
        entity.navigation.nav_com,
        Some(NavTargetRef::cell(original.0, original.1)),
    );
    assert!(
        entity.movement_target.is_none(),
        "non-null NavCom must defer depletion validation and command reissue",
    );
    assert_eq!(sim.rng_state(), rng_before);
}

#[test]
fn production_cmin_arrival_waits_for_navcom_and_releases_drive() {
    let oracle = retail_outbound_oracle();
    let target = (32, 31);
    let mut sim = production_sim(0x0715_D007, &oracle);
    let grid = PathGrid::new(GRID_SIZE, GRID_SIZE);
    install_world(&mut sim, &oracle, &grid, &[target], &[target], true);
    let entity_id = spawn_stock_miner(&mut sim, &oracle, "CMIN", MinerKind::Chrono);
    arm_search(&mut sim, entity_id);

    advance(&mut sim, &oracle, &grid);
    advance(&mut sim, &oracle, &grid);
    assert_command_state(&sim, &oracle, entity_id, "CMIN", target);

    let mut saw_pending_owner = false;
    for _ in 0..128 {
        advance(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
        assert!(entity.teleport_state.is_none());
        if entity.navigation.pending_arrival_clear {
            saw_pending_owner = true;
            assert_eq!((entity.position.rx, entity.position.ry), target);
            assert!(entity.movement_target.is_none());
            assert_eq!(
                entity.navigation.nav_com,
                Some(NavTargetRef::cell(target.0, target.1)),
            );
            assert_eq!(
                entity.miner.as_ref().expect("miner").state,
                MinerState::MoveToOre,
            );
            let locomotor = entity.locomotor.as_ref().expect("CMIN locomotor");
            assert_eq!(locomotor.kind, LocomotorKind::Teleport);
            assert_eq!(locomotor.primary_kind, Some(LocomotorKind::Teleport));
            assert_eq!(locomotor.piggyback, None);
            assert!(
                entity.drive_locomotion.is_none(),
                "restoring primary Teleport must release retired Drive runtime",
            );
            break;
        }
    }
    assert!(saw_pending_owner, "must observe track-end owner-NavCom interval");

    advance(&mut sim, &oracle, &grid);
    let entity = sim.substrate.entities.get(entity_id).expect("CMIN");
    assert_eq!(entity.navigation.nav_com, None);
    assert!(!entity.navigation.pending_arrival_clear);
    assert_eq!(
        entity.miner.as_ref().expect("miner").state,
        MinerState::Harvest,
    );
    assert!(entity.drive_locomotion.is_none());
    assert_ore_intact(&sim, &oracle, target);
}
```

**Step 3: Format only the two edited Rust files**

```powershell
rustfmt --edition 2024 src/sim/miner/mod.rs src/sim/miner/outbound_drive_tests.rs
git diff --check
git diff -- src/sim/miner/mod.rs src/sim/miner/outbound_drive_tests.rs
```

Expected: no unrelated formatting churn.

**Step 4: Run each new test red-first, serially**

Before every command:

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU,StartTime
```

Then run:

```powershell
cargo test production_stock_miners_use_drive_command_for_adjacent_ore -- --nocapture
cargo test production_harv_outbound_drive_uses_rule_profile -- --nocapture
cargo test production_cmin_outbound_drive_keeps_teleport_primary -- --nocapture
cargo test production_cmin_failed_outbound_issue_restores_locomotor_exactly -- --nocapture
cargo test production_harv_navcom_without_movement_target_is_not_reissued -- --nocapture
cargo test production_harv_navcom_defers_removed_target_revalidation -- --nocapture
cargo test production_cmin_arrival_waits_for_navcom_and_releases_drive -- --nocapture
```

Expected pre-fix:

- adjacent command test fails because direct movement has no NavCom/Drive owner
  setup and CMIN is not Drive-active;
- far HARV profile test fails because the producer leaves the three profile
  fields at zero;
- CMIN ownership test fails because the producer does not activate Drive over
  primary Teleport;
- exact rollback test may pass, honestly recorded, because the old producer
  never attempts piggyback activation;
- NavCom authority test fails because the old caller rescans first and selects
  the new preferable candidate;
- removed-target test fails because the old caller validates depletion before
  owner NavCom;
- CMIN arrival test fails because the old caller enters Harvest while NavCom is
  pending and the restore path retains the retired Drive runtime.

Record each literal `test result:` line and first load-bearing assertion in the
operational journal. A compile error is a test-harness defect to repair before
production code, not an acceptable red parity result.

### Task 3: Implement the bounded outbound producer and caller flow

**Why:** Close the earliest load-bearing handoff and completion divergences while
preserving scripted direct-move contracts.

**Files:**

- Modify: `src/sim/miner/miner_system.rs`
- Modify: `src/sim/movement/mod.rs`

**Pattern:** `Simulation::resolve_move_info` plus the existing player-command
layered issue/profile-stamp pattern; current CMIN piggyback primitive.

**Step 1: Replace `handle_move_to_ore` with the reviewed owner-first flow**

Move the combined owner gate to the first operation in the function, before
target presence/depletion validation and the teleport/arrival checks. Then
replace the rescan, adjacent/direct, and reduced-command block with:

```rust
fn handle_move_to_ore(
    sim: &mut Simulation,
    rules: &RuleSet,
    config: &MinerConfig,
    path_grid: Option<&PathGrid>,
    snap: &mut MinerSnapshot,
) {
    let has_destination_or_movement = sim
        .substrate
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| {
            entity.navigation.nav_com.is_some() || entity.movement_target.is_some()
        });

    // Native Search_For_Tiberium_And_Move returns immediately for a non-null
    // owner NavCom before target validation, arrival, or scan. MovementTarget
    // remains Rust's transitional second owner until the broader Drive host is
    // migrated.
    if has_destination_or_movement {
        return;
    }

    let Some(current_target) = snap.miner.target_ore_cell else {
        snap.miner.state = MinerState::SearchOre;
        return;
    };

    let still_has_ore = sim
        .production
        .resource_nodes
        .get(&current_target)
        .is_some_and(|node| node.remaining > 0);
    if !still_has_ore {
        snap.miner.target_ore_cell = None;
        snap.miner.state = MinerState::SearchOre;
        return;
    }

    let has_teleport = sim
        .substrate
        .entities
        .get(snap.entity_id)
        .is_some_and(|entity| entity.teleport_state.is_some());
    if has_teleport {
        return;
    }

    // Keep the read-only filter in its own scope so its captured `&Simulation`
    // is dropped before the mutable command producer runs below.
    let new_target = {
        let scan_filter = build_scan_filter(sim, path_grid, snap);
        let filter_ref: Option<&dyn Fn((u16, u16)) -> bool> = scan_filter.as_deref();
        search_local_ore(
            &sim.production.resource_nodes,
            (snap.rx, snap.ry),
            config.long_scan_radius,
            filter_ref,
            config.ore_bale_value,
            config.gem_bale_value,
        )
    };
    let target = new_target.unwrap_or(current_target);
    if target != current_target {
        snap.miner.target_ore_cell = Some(target);
    }

    if (snap.rx, snap.ry) == target {
        snap.miner.state = MinerState::Harvest;
        // This physical-arrival anchor is legacy Rust behavior; native initializes
        // the timer when search/move succeeds, a separately tracked acquisition-
        // timing drift. Retain +1 for the verified mission-before-timer observation.
        snap.miner.harvest_timer.arm(
            sim.session.binary_frame,
            u32::from(config.harvest_tick_interval) + 1,
        );
        return;
    }

    if let Some(grid) = path_grid {
        let _ = issue_outbound_ore_move(sim, rules, grid, snap.entity_id, target);
    }
}
```

This deliberately:

- orders the owner gate before target validation and physical arrival;
- preserves owner-free target validation and teleport wait;
- preserves current no-owner rescan/retarget behavior;
- performs one arrival transition against the retained or newly selected
  target;
- suppresses validation, arrival, scan, and issue when NavCom or
  MovementTarget owns work;
- removes adjacency branching, direct movement, and terrain-cost exemption.

**Step 2: Add the complete private producer helper before
`issue_move_if_idle`**

```rust
/// Hand a selected outbound ore cell to the normal Drive command authority.
fn issue_outbound_ore_move(
    sim: &mut Simulation,
    rules: &RuleSet,
    grid: &PathGrid,
    entity_id: u64,
    target: (u16, u16),
) -> bool {
    if target.0 >= grid.width() || target.1 >= grid.height() {
        return false;
    }
    let Some(info) = sim.resolve_move_info(entity_id, Some(rules)) else {
        return false;
    };

    let activation_snapshot = if info.is_teleporter && info.is_harvester {
        sim.substrate
            .entities
            .get_mut(entity_id)
            .and_then(|entity| entity.locomotor.as_mut())
            .map(|locomotor| {
                let snapshot = (
                    locomotor.kind,
                    locomotor.primary_kind,
                    locomotor.piggyback,
                    locomotor.layer,
                    locomotor.phase,
                );
                let _ = locomotor.begin_drive_piggyback_for_teleporter();
                snapshot
            })
    } else {
        None
    };

    let terrain_costs = sim.terrain_costs.get(&info.speed_type);
    let issued = movement::issue_move_command_with_layered(
        &mut sim.substrate.entities,
        grid,
        entity_id,
        target,
        info.speed,
        false,
        terrain_costs,
        None,
        sim.resolved_terrain.as_ref(),
        sim.zone_grid.as_ref(),
        None,
        info.mover_is_crusher,
    );
    if !issued {
        if let Some((kind, primary_kind, piggyback, layer, phase)) = activation_snapshot
            && let Some(locomotor) = sim
                .substrate
                .entities
                .get_mut(entity_id)
                .and_then(|entity| entity.locomotor.as_mut())
        {
            locomotor.kind = kind;
            locomotor.primary_kind = primary_kind;
            locomotor.piggyback = piggyback;
            locomotor.layer = layer;
            locomotor.phase = phase;
        }
        return false;
    }

    if let Some(movement) = sim
        .substrate
        .entities
        .get_mut(entity_id)
        .and_then(|entity| entity.movement_target.as_mut())
    {
        movement.accel_factor = info.accel_factor;
        movement.decel_factor = info.decel_factor;
        movement.slowdown_distance = info.slowdown_distance;
    }
    true
}
```

Do not:

- change or delete `issue_move_if_idle`;
- call `restore_primary_from_piggyback`;
- call the broader Teleporter destination bridge;
- pass an entity-block set/map not owned by the miner producer;
- set `ignore_terrain_cost`, `bypass_grid`, speed fractions, or RNG state;
- add state, allocations, a public interface, or a cross-layer dependency.

**Step 3: Release the retired Drive runtime on successful piggyback restore**

Replace `tick_locomotor_piggyback_restore` in `src/sim/movement/mod.rs` with:

```rust
pub fn tick_locomotor_piggyback_restore(entities: &mut EntityStore) -> usize {
    let mut restored = 0usize;
    let keys = entities.keys_sorted();
    for id in keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
        let owner_moving = entity.movement_target.is_some() || entity.forced_drive_track.is_some();
        let owner_teleporting = entity.teleport_state.is_some();
        let owner_deploying = entity.building_up.is_some()
            || entity.building_down.is_some()
            || entity.deploy_state.is_some();
        let mut retired_drive = false;
        let restored_now = if let Some(ref mut loco) = entity.locomotor {
            retired_drive =
                loco.active_kind() == crate::rules::locomotor_type::LocomotorKind::Drive;
            loco.can_restore_primary_from_piggyback(
                owner_moving,
                owner_teleporting,
                owner_deploying,
            ) && loco.restore_primary_from_piggyback()
        } else {
            false
        };
        if restored_now {
            if retired_drive {
                // Native FootClass::AI releases the old active locomotor before
                // installing the stored primary. Do not retain hashed Drive
                // state after primary Teleport is active again.
                entity.drive_locomotion = None;
                entity.drive_track = None;
            }
            restored = restored.saturating_add(1);
        }
    }
    restored
}
```

Do not delay primary restoration, clear an active Drive runtime, or change the
field-level `restore_primary_from_piggyback` primitive used by rollback tests.

**Step 4: Format and inspect only the four owned Rust files**

```powershell
rustfmt --edition 2024 src/sim/miner/mod.rs src/sim/miner/miner_system.rs src/sim/miner/outbound_drive_tests.rs src/sim/movement/mod.rs
git diff --check
git diff --stat
git diff -- src/sim/miner/mod.rs src/sim/miner/miner_system.rs src/sim/miner/outbound_drive_tests.rs src/sim/movement/mod.rs
git diff --exit-code -- src/sim/movement/movement_commands.rs src/sim/movement/locomotor.rs src/sim/world/world_commands.rs
```

Expected: only the registered test module, new test file, private helper, caller
flow, adjacent stale comments, and successful-restore Drive-runtime retirement
change.

### Task 4: Validate the prerequisite branch through the real loop

**Why:** A helper-level pass cannot establish that Phase-7 dispatch, next-tick
Drive processing, CMIN restoration, arrival, and neighboring consumers still
work.

**Files:** No new source edit unless a failing test identifies an in-scope
defect. Any broader defect triggers plan repair.

**Step 1: Run all seven production oracles serially**

Check the Cargo lease before every run, then:

```powershell
cargo test production_stock_miners_use_drive_command_for_adjacent_ore -- --nocapture
cargo test production_harv_outbound_drive_uses_rule_profile -- --nocapture
cargo test production_cmin_outbound_drive_keeps_teleport_primary -- --nocapture
cargo test production_cmin_failed_outbound_issue_restores_locomotor_exactly -- --nocapture
cargo test production_harv_navcom_without_movement_target_is_not_reissued -- --nocapture
cargo test production_harv_navcom_defers_removed_target_revalidation -- --nocapture
cargo test production_cmin_arrival_waits_for_navcom_and_releases_drive -- --nocapture
```

Expected literal result for each:

```text
test result: ok. 1 passed; 0 failed
```

Record the complete literal line, including filtered count and duration.

**Step 2: Run focused neighboring regressions serially**

```powershell
cargo test drive_accelerates_false_tick_stores_modified_fraction_without_mutating_speed -- --nocapture
cargo test drive_accelerates_true_tick_ramps_fraction_before_movement_speed -- --nocapture
cargo test drive_piggyback_restores_primary_teleport_only_after_not_moving -- --nocapture
cargo test teleporter_building_destination_activates_drive_piggyback -- --nocapture
cargo test stock_departing_hands_directly_to_search_without_exit_move -- --nocapture
```

Expected: every command reports one passing test and zero failures.

**Step 3: Run the final production compile check**

```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU,StartTime
cargo check -q
```

Expected: exit code 0 with no diagnostic.

**Step 4: Adversarial branch review**

Review the final diff against the contract and ask:

> Why should this be approved, and what evidence could still make it wrong?

Required answers:

- the production loop proves NavCom, Drive ownership, profiles, physical
  departure, arrival, CMIN no-warp, restoration, and RNG;
- the failed issue is transactionally exact for every activation-mutated field;
- the preferable candidate proves the NavCom path did not scan;
- the removed-target and pending-arrival oracles prove the owner gate is first;
- successful CMIN restoration removes the retired Drive runtime;
- shared command and locomotor primitive files are byte-unchanged;
- no test claims full Drive/path/collision/tick/pixel certification;
- any unexpected path redirection, extra mutation, RNG draw, or post-arrival
  owner state is named as DRIFT/UNCHECKED rather than hidden.

### Task 5: Commit the coherent prerequisite milestone and merge locally into dev

**Why:** The parent must resume only from a reviewed, validated, recoverable
integration point.

**Files:** The four owned Rust paths only. The ignored journal is appended
separately and is not staged.

**Step 1: Guard the branch scope and commit**

```powershell
git status --short
git diff --check
git diff --name-only
git add -- src/sim/miner/mod.rs src/sim/miner/miner_system.rs src/sim/miner/outbound_drive_tests.rs src/sim/movement/mod.rs
git diff --cached --name-status
git commit -m "miner: use native-shaped outbound Drive command"
```

Expected: one coherent feature commit with exactly those four paths.

**Step 2: Guard integration-only dev**

In the root worktree:

```powershell
git rev-parse --abbrev-ref HEAD
git status --porcelain=v1
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU,StartTime
git log -1 --format=%H
```

Expected:

- branch is `dev`;
- no tracked modification;
- untracked output contains only protected `system_map/` and
  `tools/system_map/`;
- no Cargo/rustc process is active;
- any dev advance is reconciled for overlap before merge.

**Step 3: Merge locally without pushing**

```powershell
git merge --no-ff feature/gsi-07-15-miner-outbound-drive-prereq-20260725-122214 -m "Merge miner outbound Drive prerequisite"
```

Do not push.

**Step 4: Re-run post-merge production validation**

Run the seven new tests serially in root, then:

```powershell
cargo test production_stock_miners_use_drive_command_for_adjacent_ore -- --nocapture
cargo test production_harv_outbound_drive_uses_rule_profile -- --nocapture
cargo test production_cmin_outbound_drive_keeps_teleport_primary -- --nocapture
cargo test production_cmin_failed_outbound_issue_restores_locomotor_exactly -- --nocapture
cargo test production_harv_navcom_without_movement_target_is_not_reissued -- --nocapture
cargo test production_harv_navcom_defers_removed_target_revalidation -- --nocapture
cargo test production_cmin_arrival_waits_for_navcom_and_releases_drive -- --nocapture
cargo test drive_accelerates_true_tick_ramps_fraction_before_movement_speed -- --nocapture
cargo test drive_piggyback_restores_primary_teleport_only_after_not_moving -- --nocapture
cargo test stock_departing_hands_directly_to_search_without_exit_move -- --nocapture
cargo check -q
```

Expected: all focused tests pass and `cargo check -q` exits 0. Append exact
commit IDs, merge ID, test-result lines, and residuals to the operational
journal.

### Task 6: Resume and complete the suspended GSI-07.15 parent loop

**Why:** The prerequisite is not complete in isolation; its acceptance condition
is the blocked stock level-zero scan/archive/move parent oracle.

**Files owned by the suspended parent:**

- `src/sim/miner/miner_system.rs`
- `src/sim/miner/miner_tests.rs`
- `src/sim/slave_miner.rs`

**Pattern:** Preserve the existing uncommitted parent diff, advance its branch
from old base `4910e8ff` to validated dev, replay, resolve the expected
`miner_system.rs` overlap, and rerun the complete parent suite.

**Step 1: Reconcile and preserve the exact parent diff**

In
`<local>/Documents/ra2-rust-game-gsi-07-15-level-zero-scan-move-20260725-102933`:

```powershell
git status --short
git diff --name-only
git stash push -m "gsi-07-15-parent-before-outbound-drive-prereq" -- src/sim/miner/miner_system.rs src/sim/miner/miner_tests.rs src/sim/slave_miner.rs
git status --short
```

Expected: exactly the three recorded paths enter the named stash, and the
parent worktree becomes clean. Keep the stash as a recovery backup until the
replayed parent is committed and merged.

**Step 2: Advance the parent branch to validated dev and replay**

```powershell
git merge --ff-only dev
git stash apply stash^{/gsi-07-15-parent-before-outbound-drive-prereq}
git status --short
```

Expected: `miner_tests.rs` and `slave_miner.rs` replay directly. Resolve any
`miner_system.rs` conflict with `apply_patch`, retaining both:

- the prerequisite's combined NavCom/MovementTarget gate and
  `issue_outbound_ore_move`, with the owner gate before target validation and
  arrival;
- the parent's standard present-node scan policy and key-presence target
  validity;
- the unchanged Slave positive-only boundary.

Do not drop the stash yet.

**Step 3: Format only replayed owned Rust files and rerun the complete parent
suite**

```powershell
rustfmt --edition 2024 src/sim/miner/miner_system.rs src/sim/miner/miner_tests.rs src/sim/slave_miner.rs
cargo test production_stock_miners_use_drive_command_for_adjacent_ore -- --nocapture
cargo test production_harv_outbound_drive_uses_rule_profile -- --nocapture
cargo test production_cmin_outbound_drive_keeps_teleport_primary -- --nocapture
cargo test production_cmin_failed_outbound_issue_restores_locomotor_exactly -- --nocapture
cargo test production_harv_navcom_without_movement_target_is_not_reissued -- --nocapture
cargo test production_harv_navcom_defers_removed_target_revalidation -- --nocapture
cargo test production_cmin_arrival_waits_for_navcom_and_releases_drive -- --nocapture
cargo test production_stock_miners_accept_present_zero_ring_zero -- --nocapture
cargo test production_stock_miners_filter_and_travel_to_present_zero_ring_one -- --nocapture
cargo test production_full_harv_archives_zero_through_dock_and_drives_back -- --nocapture
cargo test standard_present_zero_scan_preserves_value_tie_and_first_ring_order -- --nocapture
cargo test slave_search_preserves_current_unverified_zero_rejection -- --nocapture
cargo test present_zero_resource_node_changes_state_hash -- --nocapture
cargo check -q
```

Check the global Cargo lease before every command. Expected: every focused test
reports one pass and zero failures, including physical ring-1 travel and the
full archive/dock/drive-back loop, all prerequisite owner/cleanup oracles, and
`cargo check -q` exits 0.

Revisit every changed assumption:

- standard HARV/CMIN callers use present-node eligibility;
- Slave callers remain positive-only;
- NavCom suppresses another non-arrived scan;
- NavCom also precedes target validation and arrival;
- outbound CMIN drives with Teleport primary;
- restored CMIN has no retired Drive runtime;
- level-zero node/overlay lifecycle and hash behavior remain literal;
- no extra RNG draw appears.

**Step 4: Review, commit, merge, and remove the recovery stash only after
post-merge validation**

Commit the coherent parent milestone on its feature branch, guard clean
integration-only `dev`, merge locally without pushing, rerun all seven
prerequisite tests, all six parent tests, and `cargo check -q` on merged dev,
then drop only the exact named recovery stash. Record literal results and the
next owner/dependency-stack action in the operational journal.

If any parent oracle still fails, keep the stash and parent branch recoverable,
suspend the parent at the newly proven earliest divergence, and close only the
smallest new prerequisite required by that oracle.

## Sources & References

- **Approved design:**
  `docs/plans/2026-07-25-gsi-07-15-miner-outbound-drive-command-design.md`
- **Implementation contract:**
  `docs/contracts/2026-07-25-gsi-07-15-miner-outbound-drive-command-implementation-contract.md`
- **Design approval:**
  `docs/approvals/2026-07-25-gsi-07-15-miner-outbound-drive-command-design-approval.md`
- **Operational state:**
  `docs/goals/2026-07-24-system-by-system-parity-state.md`
- **Ghidra reports:**
  - `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
  - `docs/research/miner/HARV_HARVEST_STATE_RETARGET_VISUAL_FLAG_GHIDRA_REPORT.md`
  - `docs/research/miner/CHRONO_MINER_SET_DESTINATION_GATE_GHIDRA_REPORT.md`
  - `docs/research/miner/CHRONO_MINER_WARP_TRIGGER_GHIDRA_REPORT.md`
  - `docs/research/FOOTCLASS_SET_DESTINATION_INTERNAL_NAVCOM_HEADTO_HANDOFF_GHIDRA_REPORT.md`
  - `docs/research/UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`
  - `docs/research/DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`
  - `docs/research/FOOTCLASS_AI_GHIDRA_REPORT.md`
  - `docs/research/DRIVE_ACCELERATES_TRUE_FALSE_SPEED_RAMP_GHIDRA_REPORT.md`
  - `docs/research/DRIVE_RULES_FIELDS_SPEED_INPUTS_GHIDRA_REPORT.md`
- **Verified active-YR anchors:**
  - `0x004DCFE0` — `Search_For_Tiberium_And_Move`
  - `0x004D94B0` — `FootClass::Set_Destination_Internal`
  - `0x00741970` — UnitClass selected-cell destination path
  - `0x007423CD..0x007427C0` — Teleporter old-NavCom destination predicate
- **Retail INI authority:**
  - `ini/rulesmd.ini:7351-7404` — stock CMIN
  - `ini/rulesmd.ini:8215-8260` — stock HARV
  - `ini/rulesmd.ini:30267-30275` — `[Tiberium]`, including `Track=70%`
- **Current Rust patterns:**
  - `src/sim/miner/miner_system.rs`
  - `src/sim/movement/locomotor.rs`
  - `src/sim/movement/movement_commands.rs`
  - `src/sim/movement/movement_tick.rs`
  - `src/sim/movement/drive_locomotion.rs`
  - `src/sim/pathfinding/terrain_cost.rs`
  - `src/sim/world/world_commands.rs`
  - `src/sim/world/world_spawn.rs`
- **Reviewed current base:** `dev`
  `6e5a3d2f172be23ebffd996777c3d586146030f3`
