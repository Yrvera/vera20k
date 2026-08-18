# GSI-07.15 Level-Zero Scan, Archive, and Move Implementation Plan

Date: 2026-07-25  
Plan status: **APPROVED FOR OWNED FEATURE EXECUTION**  
Committed base reviewed:
`dev` `4910e8ffe3d5ef9559b81b98a68b9b9d7bab18f9`

> **For Codex:** Execute this plan task-by-task in the dedicated feature
> worktree. The primary coordinator alone edits owned paths, holds the global
> Cargo lease, commits, and integrates.

**Goal:** Let production-spawned stock HARV and CMIN treat the parent-provided
present-zero `ResourceNode` as a valid bounded local-scan/archive/move target,
while preserving the current positive-only Slave Miner behavior and stopping
before parent-owned zero cleanup.

**Architecture:** Extract the existing local scan into one private
policy-bearing core. A private standard-miner wrapper admits every present
node; the existing exported helper remains a positive-only compatibility
wrapper for Slave callers. Route the four standard callsites through the
standard wrapper and make the standard `MoveToOre` validity gate use key
presence. Change no traversal, ranking, filter, movement, timer, scheduler,
RNG, state schema, or hash implementation.

**Implementation Contract:**
`docs/contracts/2026-07-24-gsi-07-15-level-zero-scan-archive-move-implementation-contract.md`

**Design:**
`docs/plans/2026-07-24-gsi-07-15-level-zero-scan-archive-move-design.md`

**Design Approval:**
`docs/approvals/2026-07-24-gsi-07-15-level-zero-scan-archive-move-design-approval.md`

---

## Execution Preconditions

- Immediately before worktree creation, reconcile:
  - main checkout is clean and on `dev`;
  - `refs/heads/dev` still resolves to the reviewed SHA or owned paths are
    re-reviewed against any newer SHA;
  - no Git operation, Cargo/rustc process, or global rebaseline is active;
  - no worktree/branch already uses the exact names below;
  - none of the three owned paths is dirty in any other worktree.
- Planned branch:
  `feature/gsi-07-15-level-zero-scan-move-20260725-102933`
- Planned linked worktree:
  `<local>/Documents/ra2-rust-game-gsi-07-15-level-zero-scan-move-20260725-102933`
- Create it from the exact committed `refs/heads/dev` observed at creation.
- Feature-owned paths:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/slave_miner.rs` (test module only)
- Protected non-owned checkout:
  `<local>/Documents/ra2-rust-game-gsi-08-10-damage-authority`.
  Do not inspect it again from the feature worktree and do not modify any of
  its dirty rules/combat/entity/world/snapshot/hash paths.
- Do not touch `world_hash.rs`, `SNAPSHOT_VERSION`, snapshots, goldens,
  reducer/growth/cell authority, production fallback search, UI, render,
  research docs, Cargo manifests, or Ghidra labels in this feature.
- `ini/` is intentionally ignored. Copy the main checkout's exact physical
  `ini/` directory into the feature worktree after verifying the source and
  destination are ordinary directories, not reparse points. Never stage it.

## Grounding Summary

- Live `FootClass::Scan_For_Tiberium @ 0x004DD0A0`:
  - accepts ring 0 solely from effective LandType 5;
  - applies no ring-0 density, filter, or value gate;
  - for rings 1+, calls `Is_Cell_Harvestable`, then
    `Get_Tiberium_Value`;
  - keeps strict first-seen ties and returns after the first productive ring.
- Live `CellClass::Get_Tiberium_Value @ 0x00485020` computes
  `type.Value * (OverlayData + 1)`. Stock data byte zero therefore has positive
  value.
- Corrected state-0 Mission_Harvest evidence consumes an archive before the
  bounded scan and has no verified post-result density-zero rejection.
- Current standard Rust rejects `remaining == 0` at:
  - local-scan ring 0;
  - local-scan rings 1+;
  - the initial `MoveToOre` target-validity check.
- Archive consumption first checks `resource_nodes.contains_key`, but then
  applies the current scan filter. Native consumes and destinations a
  still-present archive without that reachability/occupancy re-check. The
  bounded parent fixture keeps its archive reachable; repairing that separate
  archive-filter DRIFT is explicitly outside this prerequisite.
- Four standard callsites use the scan:
  - SearchOre long scan;
  - MoveToOre per-tick rescan;
  - Harvest short continuation;
  - full-gate archive save.
- The exported helper also has four Slave consumers, whose harvest path
  separately rejects zero. A blanket behavior change would create an
  inconsistent unverified Slave loop.
- Current ordinary producers do not create a persistent present-zero node.
  Suspended GSI-04.09 is the expected producer. Tests therefore stage the
  parent invariant and stop before cleanup.
- `Simulation::advance_tick` runs ground movement before special movement and
  standard miner dispatch. The existing `tick_miners_n` helper does not match
  this production order and is not an oracle for this feature.
- Production miner dispatch walks live LogicVector order with no EntityStore
  fallback. Every oracle actor must use canonical `spawn_object`.
- Stock merged data pins:
  - `TiberiumShortScan=6`, `TiberiumLongScan=48`;
  - HARV storage 40, CMIN storage 20;
  - ore/gem values 25/50;
  - HARV and CMIN both list GAREFN in `Dock=`;
  - GAREFN art foundation 4x3, `Bib=yes`,
    `NumberImpassableRows=3`, and wait queue `(4,1)`;
  - CAN_DOCK/pad handoff is hardcoded anchor `+(3,1)`;
  - `TIB01` compact overlay ID is 102 and maps to tiberium type zero.
- Default `ProductionState::ore_growth_config` is disabled, so the production
  oracle can carry a staged zero through `advance_tick` without a growth phase
  rewriting it.

## Technical Decisions

- **One private admission enum, one traversal.** The policy controls only
  whether a found node is eligible. It cannot reorder filtering/value/ties.
  **Confidence: high.**
- **Private standard wrapper; exported positive-only wrapper unchanged.**
  Standard scope is explicit and Slave source/behavior compatibility stays
  intact. **Confidence: high for bounded scope; Slave parity remains
  UNCHECKED.**
- **Standard `MoveToOre` uses `contains_key`.** The standard dispatcher excludes
  `MinerKind::Slave`, so this does not alter the neighbor path. **Confidence:
  high.**
- **No overlay/LandType read in production scan code.** The parent provides the
  projected node; adding a second cell authority here would repeat the rejected
  shadow-authority design. Tests assert companion state only. **Confidence:
  high for architecture, bounded by parent representation.**
- **Retail fixture loads both rules and art patches.** `RuleSet` and
  `OverlayTypeRegistry` share one merged rules source; `merge_art_data` supplies
  the actual refinery footprint and queue cell. **Confidence: high.**
- **The archive oracle uses stock zero-link departure.** It expects no
  fabricated exit destination: the HARV remains on `(13,11)` until SearchOre
  sends it toward the distinct `(20,11)` archive. **Confidence: high from
  current production code and corrected dock research.**
- **RNG proof shadows every production tick.** Full-gate scan, local
  acquisition, archive consumption, and target retention leave all streams
  unchanged. During the dock loop, a cloned scenario stream advances exactly
  once on each due Approach/MissionEnter/FaceSync cadence dispatch, once on
  Pivoting-to-Unloading, and once on Departing-to-SearchOre; every other tick
  consumes no scenario draw, and main/mapgen never move. **Confidence: high.**
- **Known archive-filter DRIFT stays visible and bounded.** The parent oracle
  requires only a present-zero archive that remains reachable. An archived
  cell that becomes unreachable/occupied while docking is consumed directly
  by gamemd but rejected by current Rust. That separate behavior is neither
  changed nor certified here. **Confidence: high.**

## Open Questions

### Resolved During Planning

- Direct test insertion is invalid because production uses LogicVector only:
  use `spawn_object` and assert membership.
- A `PathGrid` without `zone_grid` makes the filter optional: bind resolved
  terrain, terrain-cost grids, then call `rebuild_zone_grid`.
- A merely supplied registry is vacuous: assert `TIB01 -> TiberiumTypeId(0)`.
- Ring-0 coverage cannot be inferred from ring 1: run both stock miner kinds
  through both boundaries.
- The full dock fixture is feasible with GAREFN `(10,10)`, wait queue
  `(14,11)`, accepted/pad `(13,11)`, and full HARV/archive `(20,11)`.
- The real Slave boundary can be tested without a public seam because its test
  child module can construct `SlaveSnapshot` and call private `process_slave`.

### Hard Stops During Execution

- If merged retail GAREFN/HARV cannot traverse every named dock phase through
  `advance_tick`, do not weaken the oracle. Record the first failing production
  owner and contract the smallest prerequisite.
- If CMIN cannot physically drive outward under the existing movement owner,
  do not fold locomotor repair into this slice. Preserve the ring-0/ring-1
  selection/retention claim, record the movement blocker, and recontract before
  implementation.
- If current `dev` changes any owned path after plan approval, re-run the
  adversarial plan review.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/miner/miner_system.rs` | Private admission policy/core/wrappers, four standard routes, standard key-presence move validity |
| Modify | `src/sim/miner/miner_tests.rs` | Merged-retail fixture and production ring/archive/ranking/hash oracles |
| Modify tests only | `src/sim/slave_miner.rs` | Actual private Slave dispatcher preserves current zero rejection |

No file is created, no module boundary changes, and no public API changes.

## Interface Changes

- Add private `LocalOreEligibility`.
- Add private `search_local_ore_with_eligibility`.
- Add `pub(super)` `search_standard_local_ore` so sibling miner tests can
  exercise ranking without exposing it outside `sim::miner`.
- Preserve the existing `pub(crate) search_local_ore` name, signature, and
  positive-only result for every current caller.

## Sim Checklist

- [x] No new gameplay math or floating point.
- [x] No allocation in the scan hot path beyond the existing filter closure.
- [x] No state/schema/hash implementation change.
- [x] No sim dependency on render/ui/sidebar/audio/net.
- [x] BTreeMap traversal, live order, and same-tick shared resource visibility
  remain unchanged.
- [x] No RNG call is added, removed, or reordered.
- [x] No global fallback or unbounded search change.
- [x] No Slave harvesting behavior change.
- [x] No cell/overlay authority is reconstructed in the miner.

## Parity-Critical Items

| Task | Item | Required proof |
|---|---|---|
| 1 | Retail rules/art source | Exact merged file loader plus stock-field assertions |
| 1 | Canonical lifecycle | `spawn_object`, alive/revealed/cell-marked, `in_logic_vector`, live-order membership, miner kind, full refinery foundation occupancy |
| 1 | Companion invariant | present node/zero remaining, mapped overlay/data zero, native tiberium terrain byte at every checkpoint |
| 1 | Ring 0 | HARV and CMIN accept despite blocked cell/filter path |
| 1 | Ring 1 filter | earlier blocked zero loses to later reachable zero |
| 1 | Production cadence | selection, retention, issue, later physical movement, Harvest |
| 1 | Ranking | gem value, strict first tie, first productive ring through standard wrapper |
| 1 | Archive loop | full gate, real blocked/bib-open GAREFN geometry, exact dock transition order, reservation/contact authority, unload/credits, release, consume, outbound physical travel |
| 1 | RNG | no scan-policy draw; exact existing scenario draw count on every dock tick; main/mapgen unchanged |
| 1 | Hash identity | zero present differs from absent |
| 2 | Neighbor boundary | real Slave dispatcher still rejects lone zero |
| 3 | One traversal | admission only differs; positive ranking/filter order unchanged |
| 3 | Move validity | node presence, not density, only in standard handler |
| 4 | Owned scope | exactly three approved paths staged and committed |

---

## Tasks

### Task 0: Create the owned worktree and hydrate ignored retail INIs

**Owner:** Primary coordinator only.

From the clean main checkout:

```powershell
$featureBranch = 'feature/gsi-07-15-level-zero-scan-move-20260725-102933'
$featurePath = '<local>/Documents/ra2-rust-game-gsi-07-15-level-zero-scan-move-20260725-102933'
$sourceIni = (Resolve-Path -LiteralPath 'ini').Path
$sourceItem = Get-Item -LiteralPath $sourceIni -Force
if (($sourceItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Source INI directory is a reparse point: $sourceIni"
}
if (Test-Path -LiteralPath $featurePath) {
    throw "Feature path already exists: $featurePath"
}
$baseSha = (git rev-parse refs/heads/dev).Trim()
git worktree add -b $featureBranch $featurePath $baseSha
if ($LASTEXITCODE -ne 0) {
    throw "git worktree add failed"
}
if ((git -C $featurePath rev-parse HEAD).Trim() -ne $baseSha) {
    throw "Feature worktree base mismatch"
}
```

Hydrate only the ignored flat INI directory:

```powershell
$targetIni = Join-Path $featurePath 'ini'
New-Item -ItemType Directory -Path $targetIni | Out-Null
$targetItem = Get-Item -LiteralPath $targetIni -Force
if (($targetItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Target INI directory is a reparse point: $targetIni"
}
Get-ChildItem -LiteralPath $sourceIni -File |
    Copy-Item -Destination $targetIni
$sourceManifest = Get-ChildItem -LiteralPath $sourceIni -File |
    Sort-Object Name |
    ForEach-Object { "$($_.Name) $((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash)" }
$targetManifest = Get-ChildItem -LiteralPath $targetIni -File |
    Sort-Object Name |
    ForEach-Object { "$($_.Name) $((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash)" }
if (Compare-Object $sourceManifest $targetManifest) {
    throw "Feature INI copy differs from source"
}
if ((Get-ChildItem -LiteralPath $targetIni -File).Count -ne 27) {
    throw "Unexpected feature INI file count"
}
git -C $featurePath status --short --ignored=matching ini
```

Expected: the copied directory reports only ignored entries and no tracked
change. Record `$baseSha`, branch, worktree, file count, and manifest hash in
the journal before editing Rust.

### Task 1: Add merged-retail production oracles first

**Why:** Establish expected red evidence through the real production loop
before changing scan or move policy.

**File:**

- Modify `src/sim/miner/miner_tests.rs`

#### Step 1: Add exact imports

Extend the test module imports with:

```rust
use std::fs;
use std::path::PathBuf;

use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
use crate::rules::art_data::ArtRegistry;
use crate::rules::locomotor_type::SpeedType;
use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
use crate::rules::tiberium_type::TiberiumTypeId;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::pathfinding::terrain_cost::TerrainCostGrid;
```

Keep the existing `BTreeMap`, miner types, `PathGrid`, credit helper, and
`Simulation` imports.

#### Step 2: Add the retail data loader

Place this support beside `miner_rules`; it is intentionally separate from the
many synthetic legacy fixtures:

```rust
struct RetailMinerOracle {
    rules: RuleSet,
    overlays: OverlayTypeRegistry,
    config: MinerConfig,
    tib01: u8,
}

fn merged_ini(base: &str, patch: &str) -> IniFile {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ini = IniFile::from_str(
        &fs::read_to_string(root.join(base))
            .unwrap_or_else(|err| panic!("read {base}: {err}")),
    );
    let patch_ini = IniFile::from_str(
        &fs::read_to_string(root.join(patch))
            .unwrap_or_else(|err| panic!("read {patch}: {err}")),
    );
    ini.merge(&patch_ini);
    ini
}

fn retail_miner_oracle() -> RetailMinerOracle {
    let rules_ini = merged_ini("ini/rules.ini", "ini/rulesmd.ini");
    let mut rules = RuleSet::from_ini(&rules_ini).expect("merged retail rules");
    let art_ini = merged_ini("ini/art.ini", "ini/artmd.ini");
    rules.merge_art_data(&ArtRegistry::from_ini(&art_ini));
    let overlays = OverlayTypeRegistry::from_ini(&rules_ini, None);
    let config = MinerConfig::from_rules(&rules);
    let tib01 = overlays.id_for_name("TIB01").expect("retail TIB01");

    assert_eq!(rules.general.tiberium_short_scan, 6);
    assert_eq!(rules.general.tiberium_long_scan, 48);
    assert_eq!(config.ore_bale_value, 25, "retail Riparius Value");
    assert_eq!(config.gem_bale_value, 50, "retail Cruentus Value");
    for (type_id, storage) in [("HARV", 40), ("CMIN", 20)] {
        let obj = rules.object(type_id).unwrap_or_else(|| panic!("{type_id}"));
        assert!(obj.harvester, "{type_id} Harvester=yes");
        assert_eq!(obj.storage, storage, "{type_id} stock storage");
        assert!(
            rules.harvester_can_dock_at(type_id, "GAREFN"),
            "{type_id} Dock= includes GAREFN"
        );
    }
    let refinery = rules.object("GAREFN").expect("GAREFN");
    assert!(refinery.refinery);
    assert_eq!(refinery.foundation, "4x3");
    assert!(refinery.bib, "GAREFN Bib=yes");
    assert_eq!(refinery.number_impassable_rows, 3);
    assert_eq!(refinery.queueing_cell, Some((4, 1)));
    assert_eq!(tib01, 102, "retail compact overlay slot");
    assert!(overlays.flags(tib01).is_some_and(|flags| flags.tiberium));
    assert_eq!(
        overlays.tiberium_type_for_overlay(&rules.tiberium_types, tib01),
        Some(TiberiumTypeId(0))
    );

    RetailMinerOracle {
        rules,
        overlays,
        config,
        tib01,
    }
}
```

#### Step 3: Add terrain, zone, staging, and lifecycle helpers

Use the complete current `ResolvedTerrainCell` shape so the test does not rely
on a hidden constructor:

```rust
const RETAIL_GRID: u16 = 64;
const NATIVE_TIBERIUM_LAND_TYPE: u8 = 5;

fn clear_retail_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
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
        zone_type: crate::map::resolved_terrain::zone_class::GROUND,
        base_ground_walk_blocked: false,
        base_build_blocked: false,
        base_land_type: 0,
        base_yr_cell_land_type: 0,
        base_terrain_class: TerrainClass::Clear,
        base_speed_costs: SpeedCostProfile::default(),
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
    }
}

fn staged_retail_terrain(cells: &[(u16, u16)]) -> ResolvedTerrainGrid {
    let mut out = ResolvedTerrainGrid::from_cells(
        RETAIL_GRID,
        RETAIL_GRID,
        (0..RETAIL_GRID)
            .flat_map(|ry| (0..RETAIL_GRID).map(move |rx| clear_retail_cell(rx, ry)))
            .collect(),
    );
    for &(rx, ry) in cells {
        let cell = out.cell_mut(rx, ry).expect("staged terrain cell");
        cell.land_type = NATIVE_TIBERIUM_LAND_TYPE;
        cell.yr_cell_land_type = NATIVE_TIBERIUM_LAND_TYPE;
        cell.terrain_class = TerrainClass::Tiberium;
    }
    out
}

fn install_retail_zero_world(
    sim: &mut Simulation,
    grid: &PathGrid,
    tib01: u8,
    nodes: &[((u16, u16), ResourceType)],
) {
    let terrain_cells: Vec<_> = nodes.iter().map(|(cell, _)| *cell).collect();
    let terrain = staged_retail_terrain(&terrain_cells);
    sim.terrain_costs = SpeedType::ALL_WITH_COSTS
        .iter()
        .copied()
        .map(|speed| {
            (
                speed,
                TerrainCostGrid::from_resolved_terrain(&terrain, speed),
            )
        })
        .collect();
    sim.resolved_terrain = Some(terrain);
    sim.overlay_grid = Some(OverlayGrid::new(RETAIL_GRID, RETAIL_GRID));

    for &(cell, resource_type) in nodes {
        sim.production.resource_nodes.insert(
            cell,
            ResourceNode {
                resource_type,
                remaining: 0,
            },
        );
        sim.overlay_grid
            .as_mut()
            .expect("overlay grid")
            .place_overlay(cell.0, cell.1, tib01, 0);
    }
    sim.rebuild_zone_grid(grid);
    assert!(sim.zone_grid.is_some(), "scan filter requires zone authority");
}

fn assert_retail_zero(
    sim: &Simulation,
    oracle: &RetailMinerOracle,
    cell: (u16, u16),
) {
    let node = sim
        .production
        .resource_nodes
        .get(&cell)
        .unwrap_or_else(|| panic!("present-zero node at {cell:?}"));
    assert_eq!(
        node.resource_type,
        ResourceType::Ore,
        "staged archive remains stock ore"
    );
    assert_eq!(node.remaining, 0, "level-zero node remains present");
    let overlay = sim
        .overlay_grid
        .as_ref()
        .expect("overlay grid")
        .cell(cell.0, cell.1);
    assert_eq!(overlay.overlay_id, Some(oracle.tib01));
    assert_eq!(overlay.overlay_data, 0);
    assert_eq!(
        oracle
            .overlays
            .tiberium_type_for_overlay(&oracle.rules.tiberium_types, oracle.tib01),
        Some(TiberiumTypeId(0))
    );
    let terrain = sim
        .resolved_terrain
        .as_ref()
        .and_then(|terrain| terrain.cell(cell.0, cell.1))
        .expect("resolved staged cell");
    assert_eq!(terrain.land_type, NATIVE_TIBERIUM_LAND_TYPE);
    assert_eq!(terrain.yr_cell_land_type, NATIVE_TIBERIUM_LAND_TYPE);
    assert_eq!(terrain.terrain_class, TerrainClass::Tiberium);
}

fn spawn_retail_miner(
    sim: &mut Simulation,
    oracle: &RetailMinerOracle,
    type_id: &str,
    expected_kind: MinerKind,
    cell: (u16, u16),
) -> u64 {
    let id = sim
        .spawn_object(
            type_id,
            "Americans",
            cell.0,
            cell.1,
            64,
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
        expected_kind
    );
    id
}

fn advance_retail_tick(
    sim: &mut Simulation,
    oracle: &RetailMinerOracle,
    grid: &PathGrid,
) {
    let _ = sim.advance_tick(
        &[],
        Some(&oracle.rules),
        &BTreeMap::new(),
        Some(grid),
        Some(&oracle.overlays),
        67,
    );
}

fn advance_retail_dock_tick_with_exact_rng(
    sim: &mut Simulation,
    oracle: &RetailMinerOracle,
    grid: &PathGrid,
    miner_id: u64,
) {
    let frame = sim.session.binary_frame;
    let (
        state_before,
        phase_before,
        approach_due,
        enter_due,
        deploy_due,
    ) = {
        let miner = get_miner(sim, miner_id);
        (
            miner.state,
            miner.dock_phase,
            miner.approach_hello_timer.due(frame),
            miner.dock_enter_retry.due(frame),
            miner.mission_deploy_timer.due(frame),
        )
    };
    let streams_before = sim.rng_state();
    let mut expected_scenario = sim.clone_scenario_rng();

    advance_retail_tick(sim, oracle, grid);

    let (state_after, phase_after) = {
        let miner = get_miner(sim, miner_id);
        (miner.state, miner.dock_phase)
    };
    let consumes_one_scenario_draw = state_before == MinerState::Dock
        && match phase_before {
            RefineryDockPhase::Approach => approach_due,
            RefineryDockPhase::MissionEnter | RefineryDockPhase::FaceSync => enter_due,
            RefineryDockPhase::Pivoting => {
                deploy_due && phase_after == RefineryDockPhase::Unloading
            }
            RefineryDockPhase::Departing => state_after == MinerState::SearchOre,
            RefineryDockPhase::AwaitingAcceptedCell
            | RefineryDockPhase::MissionQueued
            | RefineryDockPhase::Unloading
            | RefineryDockPhase::DepositCooldown => false,
        };
    if consumes_one_scenario_draw {
        let _ = expected_scenario.next_range_u32_inclusive(0, 2);
    }

    let streams_after = sim.rng_state();
    assert_eq!(
        streams_after.scenario,
        expected_scenario.logical_state(),
        "scenario RNG mismatch at frame {frame}, {state_before:?}/{phase_before:?} -> \
         {state_after:?}/{phase_after:?}"
    );
    assert_eq!(
        streams_after.main, streams_before.main,
        "main RNG moved during dock oracle"
    );
    assert_eq!(
        streams_after.mapgen, streams_before.mapgen,
        "mapgen RNG moved during dock oracle"
    );
}
```

#### Step 4: Add production ring-0 coverage for both stock miners

```rust
#[test]
fn production_stock_miners_accept_present_zero_ring_zero() {
    let oracle = retail_miner_oracle();
    for (type_id, kind) in [("HARV", MinerKind::War), ("CMIN", MinerKind::Chrono)] {
        let mut sim = Simulation::with_seed(0x0715_0000);
        let mut grid = PathGrid::new(RETAIL_GRID, RETAIL_GRID);
        let cell = (32, 32);
        grid.set_blocked(cell.0, cell.1, true);
        install_retail_zero_world(
            &mut sim,
            &grid,
            oracle.tib01,
            &[(cell, ResourceType::Ore)],
        );
        let miner_id = spawn_retail_miner(&mut sim, &oracle, type_id, kind, cell);
        {
            let miner = sim
                .substrate
                .entities
                .get_mut(miner_id)
                .and_then(|entity| entity.miner.as_mut())
                .expect("miner");
            miner.state = MinerState::SearchOre;
            miner.harvest_timer.clear();
        }

        let rng_before = sim.rng_state();
        advance_retail_tick(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before, "{type_id} ring-0 scan RNG");
        let miner = get_miner(&sim, miner_id);
        assert_eq!(miner.target_ore_cell, Some(cell));
        assert_eq!(miner.state, MinerState::MoveToOre);
        assert_retail_zero(&sim, &oracle, cell);

        let rng_before = sim.rng_state();
        advance_retail_tick(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before, "{type_id} ring-0 retain RNG");
        let entity = sim.substrate.entities.get(miner_id).expect("miner");
        assert_eq!(entity.miner.as_ref().unwrap().state, MinerState::Harvest);
        assert!(entity.teleport_state.is_none());
        assert_retail_zero(&sim, &oracle, cell);
    }
}
```

The blocked current cell proves ring 0 did not apply the ring-1+ path filter.

#### Step 5: Add non-vacuous ring-1 filter/travel coverage

```rust
#[test]
fn production_stock_miners_filter_and_travel_to_present_zero_ring_one() {
    let oracle = retail_miner_oracle();
    for (type_id, kind) in [("HARV", MinerKind::War), ("CMIN", MinerKind::Chrono)] {
        let mut sim = Simulation::with_seed(0x0715_0001);
        let center = (32, 32);
        let blocked_first = (31, 31);
        let reachable = (32, 31);
        let mut grid = PathGrid::new(RETAIL_GRID, RETAIL_GRID);
        grid.set_blocked(blocked_first.0, blocked_first.1, true);
        install_retail_zero_world(
            &mut sim,
            &grid,
            oracle.tib01,
            &[
                (blocked_first, ResourceType::Ore),
                (reachable, ResourceType::Ore),
            ],
        );
        let miner_id = spawn_retail_miner(&mut sim, &oracle, type_id, kind, center);
        {
            let miner = sim
                .substrate
                .entities
                .get_mut(miner_id)
                .and_then(|entity| entity.miner.as_mut())
                .expect("miner");
            miner.state = MinerState::SearchOre;
            miner.target_ore_cell = None;
            miner.harvest_timer.clear();
        }

        let rng_before = sim.rng_state();
        advance_retail_tick(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before, "{type_id} ring-1 scan RNG");
        let miner = get_miner(&sim, miner_id);
        assert_eq!(
            miner.target_ore_cell,
            Some(reachable),
            "earlier blocked candidate must be rejected by the live filter"
        );
        assert_eq!(miner.state, MinerState::MoveToOre);
        assert_retail_zero(&sim, &oracle, blocked_first);
        assert_retail_zero(&sim, &oracle, reachable);

        let rng_before = sim.rng_state();
        advance_retail_tick(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before, "{type_id} movement issue RNG");
        let entity = sim.substrate.entities.get(miner_id).expect("miner");
        assert!(entity.movement_target.is_some());
        assert!(entity.teleport_state.is_none(), "CMIN outbound must not warp");

        let start = (entity.position.rx, entity.position.ry);
        let mut moved = false;
        let mut harvested = false;
        for _ in 0..96 {
            advance_retail_tick(&mut sim, &oracle, &grid);
            let entity = sim.substrate.entities.get(miner_id).expect("miner");
            moved |= (entity.position.rx, entity.position.ry) != start;
            harvested |= entity.miner.as_ref().unwrap().state == MinerState::Harvest;
            assert!(entity.teleport_state.is_none());
            assert_retail_zero(&sim, &oracle, blocked_first);
            assert_retail_zero(&sim, &oracle, reachable);
            if moved && harvested {
                break;
            }
        }
        assert!(moved, "{type_id} must physically leave {start:?}");
        assert!(harvested, "{type_id} must reach Harvest");
    }
}
```

#### Step 6: Add the existing-hash identity test

```rust
#[test]
fn present_zero_resource_node_changes_state_hash() {
    let a = Simulation::new();
    let mut b = Simulation::new();
    b.production.resource_nodes.insert(
        (20, 20),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 0,
        },
    );
    assert_ne!(a.state_hash(), b.state_hash());
}
```

#### Step 7: Add the exact production archive/dock/outbound oracle

```rust
#[test]
fn production_full_harv_archives_zero_through_dock_and_drives_back() {
    let oracle = retail_miner_oracle();
    let config = &oracle.config;
    let mut sim = Simulation::with_seed(0x0715_A11C);
    let refinery_type = oracle.rules.object("GAREFN").expect("GAREFN");
    let mut grid = PathGrid::new(RETAIL_GRID, RETAIL_GRID);
    let archive = (20, 11);
    let refinery_anchor = (10, 10);
    let pad = (13, 11);
    let wait_queue = (14, 11);
    grid.block_building_movement_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
        refinery_type.bib,
    );
    assert!(!grid.is_walkable(10, 10), "GAREFN interior blocked");
    assert!(!grid.is_walkable(12, 11), "GAREFN west bib cells blocked");
    assert!(grid.is_walkable(pad.0, pad.1), "east-edge bib/pad remains open");
    assert!(
        grid.is_walkable(wait_queue.0, wait_queue.1),
        "queue cell outside foundation remains open"
    );
    assert!(
        grid.is_walkable(archive.0, archive.1),
        "bounded archive fixture remains path-walkable"
    );
    install_retail_zero_world(
        &mut sim,
        &grid,
        oracle.tib01,
        &[(archive, ResourceType::Ore)],
    );

    let refinery_id = sim
        .spawn_object(
            "GAREFN",
            "Americans",
            refinery_anchor.0,
            refinery_anchor.1,
            0,
            &oracle.rules,
            &BTreeMap::new(),
        )
        .expect("spawn GAREFN");
    let refinery = sim.substrate.entities.get(refinery_id).expect("refinery");
    assert!(refinery.lifecycle.object_alive);
    assert!(!refinery.lifecycle.in_limbo);
    assert!(refinery.lifecycle.cell_marked);
    assert!(refinery.in_logic_vector);
    assert!(sim.live_object_order_snapshot().contains(&refinery_id));
    for (rx, ry) in crate::sim::production::building_base_foundation_cells(
        refinery_anchor.0,
        refinery_anchor.1,
        &refinery_type.foundation,
    ) {
        assert!(
            sim.substrate
                .occupancy
                .contains_entity(rx, ry, refinery_id),
            "GAREFN lifecycle occupancy missing at ({rx},{ry})"
        );
    }
    assert_ne!(archive, pad);
    assert_ne!(archive, wait_queue);

    let miner_id =
        spawn_retail_miner(&mut sim, &oracle, "HARV", MinerKind::War, archive);
    {
        let miner = sim
            .substrate
            .entities
            .get_mut(miner_id)
            .and_then(|entity| entity.miner.as_mut())
            .expect("HARV miner");
        assert_eq!(miner.capacity_bales, 40);
        miner.cargo = (0..miner.capacity_bales)
            .map(|_| CargoBale {
                resource_type: ResourceType::Ore,
                value: config.ore_bale_value,
            })
            .collect();
        miner.state = MinerState::Harvest;
        miner.target_ore_cell = Some(archive);
        miner.last_harvest_cell = None;
        miner.harvest_timer.clear();
    }

    let credits_before = credits_for_owner(&sim, "Americans");
    let rng_before = sim.rng_state();
    advance_retail_tick(&mut sim, &oracle, &grid);
    assert_eq!(sim.rng_state(), rng_before, "full-gate scan adds no RNG draw");
    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.state, MinerState::ReturnToRefinery);
    assert_eq!(miner.last_harvest_cell, Some(archive));
    assert_eq!(miner.reserved_refinery, None);
    assert_retail_zero(&sim, &oracle, archive);

    let mut phase_trace = Vec::new();
    let mut last_dock_phase = None;
    for _ in 0..4_000 {
        advance_retail_dock_tick_with_exact_rng(&mut sim, &oracle, &grid, miner_id);
        let entity = sim.substrate.entities.get(miner_id).expect("HARV");
        let miner = entity.miner.as_ref().expect("miner");
        if miner.state == MinerState::Dock {
            assert_eq!(
                miner.reserved_refinery,
                Some(refinery_id),
                "Dock retains the selected refinery through Departing"
            );
            if last_dock_phase != Some(miner.dock_phase) {
                phase_trace.push(miner.dock_phase);
                last_dock_phase = Some(miner.dock_phase);
            }

            let contacts = &sim.production.dock_reservations;
            let has_contact = contacts.has_contact(refinery_id, miner_id);
            let has_entered = contacts.has_contact_entered(refinery_id, miner_id);
            let expected_contact = miner.dock_phase != RefineryDockPhase::Approach;
            let expected_entered = matches!(
                miner.dock_phase,
                RefineryDockPhase::FaceSync
                    | RefineryDockPhase::MissionQueued
                    | RefineryDockPhase::Pivoting
                    | RefineryDockPhase::Unloading
                    | RefineryDockPhase::Departing
            );
            assert_eq!(
                has_contact, expected_contact,
                "HELLO contact begins exactly at first MissionEnter"
            );
            assert_eq!(
                has_entered, expected_entered,
                "entered contact begins exactly at FaceSync"
            );
            assert_eq!(
                entity.has_live_contact_with(refinery_id),
                has_contact,
                "radio contact and reservation mirror agree"
            );
            assert_eq!(
                sim.substrate
                    .entities
                    .get(refinery_id)
                    .expect("GAREFN")
                    .radio_contacts
                    .contains(miner_id),
                has_contact,
                "refinery receiver contact mirrors the reservation"
            );
            assert_eq!(
                entity.dock_entered_with,
                has_entered.then_some(refinery_id),
                "ENTER_DOCK bus fact and reservation mirror agree"
            );
            assert!(
                !contacts.is_on_pad(refinery_id, miner_id),
                "stock GAREFN/HARV zero-link path never creates an on-pad link"
            );
            if expected_contact {
                assert!(has_contact);
                assert!(contacts.is_occupied(refinery_id));
            }
        }
        assert_eq!(miner.last_harvest_cell, Some(archive));
        assert_retail_zero(&sim, &oracle, archive);
        if miner.state == MinerState::Dock
            && miner.dock_phase == RefineryDockPhase::Departing
        {
            break;
        }
    }
    assert_eq!(
        phase_trace,
        vec![
            RefineryDockPhase::Approach,
            RefineryDockPhase::MissionEnter,
            RefineryDockPhase::AwaitingAcceptedCell,
            RefineryDockPhase::MissionEnter,
            RefineryDockPhase::FaceSync,
            RefineryDockPhase::MissionQueued,
            RefineryDockPhase::Pivoting,
            RefineryDockPhase::Unloading,
            RefineryDockPhase::Departing,
        ],
        "exact stock HARV/GAREFN dock transition order"
    );
    let before_depart = sim.substrate.entities.get(miner_id).expect("HARV");
    assert_eq!(
        (before_depart.position.rx, before_depart.position.ry),
        pad,
        "stock zero-link unload remains on the accepted/pad cell"
    );
    assert!(before_depart.miner.as_ref().unwrap().cargo.is_empty());
    assert_eq!(
        credits_for_owner(&sim, "Americans") - credits_before,
        40 * i32::from(config.ore_bale_value)
    );
    assert!(
        sim.production
            .dock_reservations
            .has_contact(refinery_id, miner_id)
    );
    assert!(
        sim.production
            .dock_reservations
            .has_contact_entered(refinery_id, miner_id)
    );
    assert!(
        sim.production
            .dock_reservations
            .is_occupied(refinery_id)
    );

    advance_retail_dock_tick_with_exact_rng(&mut sim, &oracle, &grid, miner_id);
    let miner = get_miner(&sim, miner_id);
    assert_eq!(miner.state, MinerState::SearchOre);
    assert_eq!(miner.last_harvest_cell, Some(archive));
    assert_eq!(miner.reserved_refinery, None);
    assert!(
        !sim.production
            .dock_reservations
            .is_occupied(refinery_id),
        "Departing releases all dock authority"
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact(refinery_id, miner_id)
    );
    assert!(
        !sim.production
            .dock_reservations
            .has_contact_entered(refinery_id, miner_id)
    );
    assert!(
        !sim.production
            .dock_reservations
            .is_on_pad(refinery_id, miner_id)
    );
    let departed = sim.substrate.entities.get(miner_id).expect("HARV");
    assert!(!departed.has_live_contact_with(refinery_id));
    assert_eq!(departed.dock_entered_with, None);
    assert!(
        !sim
            .substrate
            .entities
            .get(refinery_id)
            .expect("GAREFN")
            .radio_contacts
            .contains(miner_id),
        "Departing clears the refinery receiver contact"
    );
    assert!(
        !sim.substrate
            .occupancy
            .contains_entity(archive.0, archive.1, miner_id),
        "HARV has physically vacated the reachable archive before consumption"
    );
    assert_retail_zero(&sim, &oracle, archive);

    let mut consumed = false;
    for _ in 0..4 {
        let rng_before = sim.rng_state();
        advance_retail_tick(&mut sim, &oracle, &grid);
        assert_eq!(sim.rng_state(), rng_before, "archive wait/consume adds no draw");
        let miner = get_miner(&sim, miner_id);
        assert_retail_zero(&sim, &oracle, archive);
        if miner.last_harvest_cell.is_none() {
            assert_eq!(miner.state, MinerState::MoveToOre);
            assert_eq!(miner.target_ore_cell, Some(archive));
            consumed = true;
            break;
        }
        assert_eq!(miner.last_harvest_cell, Some(archive));
    }
    assert!(consumed, "resume jitter is bounded to 0..=2 frames");

    let rng_before = sim.rng_state();
    advance_retail_tick(&mut sim, &oracle, &grid);
    assert_eq!(sim.rng_state(), rng_before, "target retention adds no draw");
    let entity = sim.substrate.entities.get(miner_id).expect("HARV");
    assert_eq!(entity.miner.as_ref().unwrap().target_ore_cell, Some(archive));
    assert!(entity.movement_target.is_some(), "outbound drive issued");
    assert_retail_zero(&sim, &oracle, archive);

    let start = (entity.position.rx, entity.position.ry);
    assert_eq!(start, pad);
    let start_distance = start.0.abs_diff(archive.0) + start.1.abs_diff(archive.1);
    let mut moved_toward_archive = false;
    for _ in 0..160 {
        advance_retail_tick(&mut sim, &oracle, &grid);
        let entity = sim.substrate.entities.get(miner_id).expect("HARV");
        let pos = (entity.position.rx, entity.position.ry);
        let distance = pos.0.abs_diff(archive.0) + pos.1.abs_diff(archive.1);
        assert_retail_zero(&sim, &oracle, archive);
        if pos != start && distance < start_distance {
            moved_toward_archive = true;
            break;
        }
    }
    assert!(moved_toward_archive, "HARV physically travels toward archive");
}
```

This test deliberately stops before `Harvest` can invoke the parent cleanup.

#### Step 8: Run and record red-first evidence

After checking Cargo ownership, run serially:

```powershell
cargo test production_stock_miners_accept_present_zero_ring_zero -- --nocapture
cargo test production_stock_miners_filter_and_travel_to_present_zero_ring_one -- --nocapture
cargo test production_full_harv_archives_zero_through_dock_and_drives_back -- --nocapture
cargo test present_zero_resource_node_changes_state_hash -- --nocapture
```

Expected on unpatched `dev`:

- the first three compile and fail at zero admission/archive for the asserted
  reason;
- the hash identity test passes;
- fixture compile failures or absent LogicVector dispatch are not valid red
  evidence and must be repaired before Task 2.

### Task 2: Add the actual Slave-caller preservation regression

**Why:** Prove the compatibility wrapper preserves the currently implemented
neighbor behavior rather than merely testing the wrapper itself.

**File:**

- Modify only the `#[cfg(test)]` module in `src/sim/slave_miner.rs`

Keep the existing imports and add:

```rust
#[test]
fn slave_search_preserves_current_unverified_zero_rejection() {
    let rules = make_test_rules();
    let config = MinerConfig::from_rules(&rules);
    let mut sim = Simulation::new();
    let master_id = sim
        .spawn_object_at_height("YAREFN", "YuriCountry", 10, 10, 0, 0, &rules)
        .expect("spawn slave master");
    let slave_id = sim
        .spawn_object_at_height("SLAV", "YuriCountry", 11, 10, 0, 0, &rules)
        .expect("spawn slave");
    sim.production.resource_nodes.insert(
        (10, 10),
        crate::sim::miner::ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 0,
        },
    );

    let entity = sim.substrate.entities.get(slave_id).expect("slave entity");
    let mut snap = SlaveSnapshot {
        entity_id: slave_id,
        owner: entity.owner,
        rx: entity.position.rx,
        ry: entity.position.ry,
        harvester: SlaveHarvester::new(master_id, 4),
    };
    process_slave(&mut sim, &rules, &config, None, &mut snap);

    assert_eq!(snap.harvester.state, SlaveHarvestState::Idle);
    assert_eq!(snap.harvester.target_cell, None);
    assert_eq!(
        sim.production
            .resource_nodes
            .get(&(10, 10))
            .expect("zero node preserved")
            .remaining,
        0
    );
}
```

Run it before the production change:

```powershell
cargo test slave_search_preserves_current_unverified_zero_rejection -- --nocapture
```

Expected: pass on the base and after implementation.

### Task 3: Implement one admission seam and standard key-presence validity

**Why:** Close only the two proven standard consumer rejections without
duplicating parity-sensitive traversal or widening Slave behavior.

**File:**

- Modify `src/sim/miner/miner_system.rs`

#### Step 1: Add the private policy and core

Immediately above the current `search_local_ore`, add:

```rust
#[derive(Clone, Copy)]
enum LocalOreEligibility {
    PresentNode,
    PositiveRemaining,
}

impl LocalOreEligibility {
    #[inline]
    fn admits(self, node: &ResourceNode) -> bool {
        match self {
            Self::PresentNode => true,
            Self::PositiveRemaining => node.remaining > 0,
        }
    }
}
```

Rename the current implementation body to:

```rust
fn search_local_ore_with_eligibility(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
    ore_base: u16,
    gem_base: u16,
    eligibility: LocalOreEligibility,
) -> Option<(u16, u16)> {
    // existing body, with only the two admission predicates changed below
}
```

Change ring 0 from:

```rust
if let Some(node) = nodes.get(&center)
    && node.remaining > 0
{
    return Some(center);
}
```

to:

```rust
if let Some(node) = nodes.get(&center)
    && eligibility.admits(node)
{
    return Some(center);
}
```

Change the ring-1+ zero rejection from:

```rust
if node.remaining == 0 {
    continue;
}
```

to:

```rust
if !eligibility.admits(node) {
    continue;
}
```

Do not move that admission relative to the existing filter/value calculation.
Do not change `value_of`, bounds, arm order, strict comparison, or ring exit.

#### Step 2: Restore the exported compatibility wrapper and add standard wrapper

Keep the current exported signature exactly:

```rust
pub(crate) fn search_local_ore(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
    ore_base: u16,
    gem_base: u16,
) -> Option<(u16, u16)> {
    search_local_ore_with_eligibility(
        nodes,
        center,
        radius,
        filter,
        ore_base,
        gem_base,
        LocalOreEligibility::PositiveRemaining,
    )
}
```

Add the sibling-visible standard wrapper:

```rust
pub(super) fn search_standard_local_ore(
    nodes: &std::collections::BTreeMap<(u16, u16), ResourceNode>,
    center: (u16, u16),
    radius: u16,
    filter: Option<&dyn Fn((u16, u16)) -> bool>,
    ore_base: u16,
    gem_base: u16,
) -> Option<(u16, u16)> {
    search_local_ore_with_eligibility(
        nodes,
        center,
        radius,
        filter,
        ore_base,
        gem_base,
        LocalOreEligibility::PresentNode,
    )
}
```

The positive-only wrapper comment must state that it preserves current
unverified Slave behavior; it must not claim native parity.

#### Step 3: Add the focused standard-wrapper ranking regression

Now that the private wrapper exists, add this test to
`src/sim/miner/miner_tests.rs`:

```rust
#[test]
fn standard_present_zero_scan_preserves_value_tie_and_first_ring_order() {
    let oracle = retail_miner_oracle();
    let mut nodes = BTreeMap::new();
    nodes.insert(
        (9, 9),
        ResourceNode {
            resource_type: ResourceType::Ore,
            remaining: 0,
        },
    );
    nodes.insert(
        (9, 11),
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: 0,
        },
    );
    assert_eq!(
        super::miner_system::search_standard_local_ore(
            &nodes,
            (10, 10),
            3,
            None,
            oracle.config.ore_bale_value,
            oracle.config.gem_bale_value,
        ),
        Some((9, 11)),
        "zero-density gem keeps its higher stock base value"
    );

    nodes.get_mut(&(9, 11)).unwrap().resource_type = ResourceType::Ore;
    assert_eq!(
        super::miner_system::search_standard_local_ore(
            &nodes,
            (10, 10),
            3,
            None,
            oracle.config.ore_bale_value,
            oracle.config.gem_bale_value,
        ),
        Some((9, 9)),
        "strict comparison keeps the first native scan-order tie"
    );

    nodes.insert(
        (10, 12),
        ResourceNode {
            resource_type: ResourceType::Gem,
            remaining: 100,
        },
    );
    assert_eq!(
        super::miner_system::search_standard_local_ore(
            &nodes,
            (10, 10),
            4,
            None,
            oracle.config.ore_bale_value,
            oracle.config.gem_bale_value,
        ),
        Some((9, 9)),
        "the first productive ring beats a richer farther ring"
    );
}
```

Run it immediately. It must pass because the new standard wrapper is already
the `PresentNode` policy. The production red-first tests from Task 1 are the
pre-implementation failure evidence; this focused test guards ranking details
introduced by the new internal seam.

#### Step 4: Route exactly four standard callsites

Replace `search_local_ore(` with `search_standard_local_ore(` only in:

- `handle_search_ore`;
- `handle_move_to_ore` per-tick rescan;
- `handle_harvest` short continuation;
- `save_archive_via_short_scan`.

After the edit, a focused `rg` must show:

- four standard wrapper callsites in `miner_system.rs`;
- the exported positive-only wrapper definition;
- four unchanged Slave calls in `slave_miner.rs`;
- no call from `pick_best_resource_node`.

#### Step 5: Change only standard MoveToOre validity

Replace:

```rust
let still_has_ore = sim
    .production
    .resource_nodes
    .get(&current_target)
    .is_some_and(|n| n.remaining > 0);
```

with:

```rust
let target_is_present = sim
    .production
    .resource_nodes
    .contains_key(&current_target);
```

Use `if !target_is_present` in the unchanged missing-target branch. Update its
comment from “depleted” to “still present.” Do not alter stale movement
cleanup, retargeting, arrival timing, or movement issue.

#### Step 6: Run the new tests serially

```powershell
cargo test production_stock_miners_accept_present_zero_ring_zero -- --nocapture
cargo test production_stock_miners_filter_and_travel_to_present_zero_ring_one -- --nocapture
cargo test standard_present_zero_scan_preserves_value_tie_and_first_ring_order -- --nocapture
cargo test production_full_harv_archives_zero_through_dock_and_drives_back -- --nocapture
cargo test slave_search_preserves_current_unverified_zero_rejection -- --nocapture
cargo test present_zero_resource_node_changes_state_hash -- --nocapture
```

Expected: every command exits zero and prints a literal passing
`test result:` line.

### Task 4: Format, validate, inspect, and commit the feature

#### Step 1: Format only edited Rust

```powershell
rustfmt --edition 2024 src/sim/miner/miner_system.rs
rustfmt --edition 2024 src/sim/miner/miner_tests.rs
rustfmt --edition 2024 src/sim/slave_miner.rs
git diff --check
```

Inspect formatter churn and retain only owned/local edits.

#### Step 2: Run adjacent production regressions serially

```powershell
cargo test scan_ring_0_allows_harvesters_own_cell -- --nocapture
cargo test move_to_ore_target_stable_when_world_unchanged -- --nocapture
cargo test exit_pad_preserves_archive_on_arrival -- --nocapture
cargo test harvester_continues_to_short_scan_when_partial_then_empty -- --nocapture
cargo test chrono_miner_does_not_warp_outbound -- --nocapture
cargo test move_to_ore_avoids_tree_blocked_cell_from_start -- --nocapture
```

Expected: each exits zero with a literal passing `test result:` line.

#### Step 3: Run exact module scopes and compile check

```powershell
cargo test 'sim::miner::miner_tests::' -- --nocapture
cargo test 'sim::slave_miner::tests::' -- --nocapture
cargo check -q
```

Run one at a time under the global lease. Report literal module
`test result:` lines and the literal `cargo check -q` exit code.

#### Step 4: Audit exact scope

```powershell
git status --short
git diff --name-only
git diff --check
git diff -- src/sim/miner/miner_system.rs
git diff -- src/sim/miner/miner_tests.rs
git diff -- src/sim/slave_miner.rs
rg -n "search_(standard_)?local_ore\\(" src/sim/miner src/sim/slave_miner.rs
```

Expected modified paths exactly:

```text
src/sim/miner/miner_system.rs
src/sim/miner/miner_tests.rs
src/sim/slave_miner.rs
```

Reject the milestone if another tracked path appears.

#### Step 5: Commit coherent feature milestone

```powershell
git add -- src/sim/miner/miner_system.rs src/sim/miner/miner_tests.rs src/sim/slave_miner.rs
git diff --cached --name-only
git diff --cached --check
git commit -m "miner: accept present level-zero scan targets"
```

Record the feature SHA and verify the feature worktree is clean. Do not push.

### Task 5: Guarded no-commit integration into `dev`

**Owner:** Primary coordinator only, from the main checkout.

#### Step 1: Reconcile before merge

```powershell
git status --porcelain=v2 --branch
git rev-parse refs/heads/dev
git rev-parse feature/gsi-07-15-level-zero-scan-move-20260725-102933
git worktree list --porcelain
Get-Process cargo,rustc -ErrorAction SilentlyContinue |
  Select-Object ProcessName,Id,CPU,StartTime
```

Stop with `MERGE_DEFERRED_DIRTY_DEV` if tracked `dev` is dirty. If `dev` moved
and any owned path changed, review and revalidate the combined diff first.

#### Step 2: Run clean-dev baseline

Run the existing adjacent regression set from Task 4 on clean current `dev`
and record its literal results. New feature tests do not exist on baseline.

#### Step 3: Begin guarded merge

```powershell
git merge --no-ff --no-commit feature/gsi-07-15-level-zero-scan-move-20260725-102933
```

If conflicts occur, use `git merge --abort`, record exact paths, and never
discard either side.

#### Step 4: Validate combined state

Run:

- every new focused test from Task 3;
- both exact module scopes from Task 4;
- `cargo check -q`;
- `git diff --cached --check`;
- staged path inspection.

No Cargo command runs in parallel.

#### Step 5: Complete merge only after validation

```powershell
git commit -m "Merge GSI-07.15 level-zero scan target parity"
```

Record merge SHA, confirm clean `dev`, and do not push.

#### Step 6: Crash-safe cleanup and parent unwind

- Update the operational journal with:
  - base/feature/merge SHAs;
  - literal red-first and passing result lines;
  - owned paths;
  - honest verification level and residuals;
  - exact next dependency-stack action.
- Verify the copied feature `ini/` target resolves inside the named feature
  worktree and is not a reparse point, then remove only that copied directory
  with native PowerShell.
- Remove the clean linked worktree without force after the journal is durable.
- Keep the feature branch as a recoverable reference unless later cleanup is
  separately authorized.
- Pop this bounded consumer prerequisite. Resume current validated `dev` at
  suspended GSI-01.05 unless the parent stack journal identifies a smaller
  immediate GSI-04.09 oracle prerequisite.
- Rerun the complete parent loop after the natural present-zero producer and
  cleanup are implemented; this merge alone is not parity completion.

## Validation Interpretation

- Rust tests establish regression and integration behavior only.
- Live Ghidra evidence establishes the bounded native mechanism.
- No native executable harvester oracle exists in the canonical Oracle project
  for this slice.
- Therefore final status may be:
  `VERIFIED BINARY MECHANISM + RUST PRODUCTION REGRESSION`,
  never `VERIFIED PARITY`.

## Residuals After This Plan

- No natural present-zero producer or cleanup.
- No synchronous canonical LandType/zone recomputation.
- Growth remains scheduled after standard miners in Rust.
- Positive-density projection/value remains non-native.
- Global unbounded fallback remains non-native and still rejects zero.
- Search success/timer/destination timing remains divergent.
- Native consumes and destinations a still-present archive without rerunning
  zone/path/occupancy eligibility; current Rust applies the scan filter and can
  discard an archive that became unreachable or occupied while docking.
- Missing-target stale physical movement remains separate.
- UI/manual order paths still reject zero.
- `last_harvest_cell` remains omitted from manual world hash.
- Slave zero behavior remains UNCHECKED and intentionally unchanged.

## Sources

- Contract and approved design listed above.
- Research:
  - `docs/research/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
  - `docs/research/miner/HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`
- Binary anchors:
  - `0x004DD0A0`
  - `0x004DCE80`
  - `0x00485020`
  - `0x0073E5E0`
- Retail data:
  - `ini/rules.ini`
  - `ini/rulesmd.ini`
  - `ini/art.ini`
  - `ini/artmd.ini`
- Rust:
  - `src/sim/miner/miner_system.rs`
  - `src/sim/miner/miner_tests.rs`
  - `src/sim/slave_miner.rs`
  - `src/sim/miner/miner_dock_sequence.rs`
  - `src/sim/world/mod.rs`
  - `src/sim/world/world_spawn.rs`
  - `src/map/overlay_types.rs`
  - `src/sim/overlay_grid.rs`
  - `src/map/resolved_terrain.rs`

## Post-Plan Self-Review

- [x] Every contract/design requirement maps to an executable task.
- [x] No implementation choice is delegated to the feature phase.
- [x] The actual production scheduler and lifecycle are used.
- [x] Retail classifier, geometry, zone filter, companion state, RNG, archive,
  credit, and physical travel assertions are non-vacuous.
- [x] Ring 0 and ring 1 cover both stock miner kinds.
- [x] The actual private Slave caller pins the neighbor boundary.
- [x] One traversal authority and the existing exported signature are retained.
- [x] No public API, schema, hash implementation, RNG, or layer edge changes.
- [x] Red-first, focused, adjacent, module, and compile validation are named.
- [x] Feature/worktree/commit/guarded-integration steps preserve dev and never
  push.
- [x] Residuals and honest verification status are explicit.
