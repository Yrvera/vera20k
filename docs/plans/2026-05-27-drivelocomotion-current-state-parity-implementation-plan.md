# DriveLocomotion Current-State Parity Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not write broad movement refactors outside the listed files.

**Goal:** Execute the first high-value Drive-locomotor parity slice by making the existing NavCom and DriveLocomotion runtime scaffold authoritative for AMCV-style cell moves, regular-crusher behavior, DriveTrack residual movement, cell/entity NavCom re-aim, and bounded low-bridge tube traversal without replacing unrelated movement systems.

**Architecture:** This stays inside deterministic `sim/` movement, pathfinding, entity state, and map tube facts. `MovementTarget` remains a transitional path adapter, while `NavigationState`, `DriveLocomotionRuntime`, `DriveTrackState`, and unit tube payloads become authoritative for the Drive-only behavior covered by this slice. Building/object dock re-aim and full blocked-tube side effects remain separate follow-up work.

**Design Doc:** `docs/plans/2026-05-27-drivelocomotion-current-state-parity-design.md`

---

## Grounding Summary

- Research-index with `--system locomotion` returned zero exact-system docs, so the plan reran without system and used `handoff.py` for Drive/NavCom/crush/tube evidence.
- Verified Drive/NavCom reports say `FootClass::Set_Destination_Internal` writes owner NavCom and calls locomotor head-to, while empty Drive arrival clears via `Set_Destination(NULL, 1)`.
- Verified Drive reports identify current Rust touchpoints: `src/sim/components.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_commands.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_step.rs`, and `src/sim/movement/drive_track.rs`.
- Current Rust already has `NavigationState`, `NavTargetRef`, `DriveCoord`, `DrivePathQueue`, `DriveTurnState`, `DriveTubePayload`, and `DriveLocomotionRuntime` in `src/sim/components.rs:272-411`.
- Current Rust already parses and stores `regular_crusher` and `drive_accelerates` in `src/sim/game_entity.rs:295-299`, `src/sim/world/world_spawn.rs:170-171`, and `src/sim/world/world_commands.rs:93-95`.
- Current Rust still computes `mover_is_crusher` without `regular_crusher` in `src/sim/world/world_commands.rs:95-100` and `src/sim/movement/movement_occupancy.rs:483-489`.
- Current Rust already has Drive residual/track budget support in `src/sim/movement/movement_step.rs:46-103` and `src/sim/movement/drive_track.rs:3725-3782`.
- Current Rust still centralizes Drive and non-Drive movement in `tick_movement_with_grids`; integration tasks must avoid changing Walk, Teleport, Fly, Jumpjet, miner scripted moves, or forced tracks.
- Verified crush evidence says `UnitClass::PerCellProcess @ 0x741700` checks `Crusher=yes` or veteran crusher, scatters on `entering != 0`, crushes on `entering == 0`, applies distance squared `<= 0x3FFF`, and plays `CrushSound` at crusher coordinates.
- Current Rust crush helpers are cell-based and movement-zone-based in `src/sim/movement/bump_crush.rs:415-582`; sound emission currently uses victim coordinates in `src/sim/movement/bump_crush.rs:511-535`.
- Verified low-bridge tube reports say direction-8 traversal requires a real nonzero `TubeClass+0x1C0` path, active tube cursor/state, speed-budget movement, unit final X/Y snap to `TubeClass+0x28`, and unit final Z from the accumulator.
- Current Rust low-bridge tube movement is simplified in `src/sim/movement/tube_movement.rs:24-155` and finalizes through bridge-like landing, which conflicts with the unit TubeMovement reports.
- INI data for the AMCV fixture is active YR `rulesmd.ini [AMCV] Speed=4`, `ROT=5`, `Crusher=yes`, Drive locomotor, and `MovementZone=Normal` at `ini/rulesmd.ini:6969-7000`.
- INI data for the crush victim fixture is active YR `rulesmd.ini [E1] CrushSound=InfantrySquish` and `Crushable=yes` at `ini/rulesmd.ini:3713-3758`.
- Recent git history touching movement files includes `103aee0`, `2e39817`, `1102241`, and related movement/parity commits; the reviewed design matches current code better than the older stale DriveLocomotion implementation plan.

## Key Technical Decisions

- Finish the existing scaffold instead of introducing a new locomotion framework. **Confidence:** high. **Source:** design doc; repo pattern in `src/sim/components.rs`, `src/sim/movement/navcom.rs`, `src/sim/movement/movement_step.rs`.
- Introduce an explicit regular crusher capability and keep it separate from `MovementZone`. **Confidence:** high. **Source:** `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`; AMCV trace; `ini/rulesmd.ini:6988` and `:7000`.
- Preserve DriveTrack residual budget and no-fresh-speed retry by routing through existing `advance_drive_track_with_budget`. **Confidence:** high. **Source:** `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`; `src/sim/movement/movement_step.rs:525-735`.
- Implement cell-target Drive parity first, then add entity-target NavCom re-aim through a narrow resolver. **Confidence:** medium. **Source:** design doc requires full non-cell re-aim, but this implementation slice deliberately excludes object/building dock/approach targets until a verified provider is wired.
- Replace Drive unit low-bridge traversal with UnitClass tube payload semantics before claiming bridge-ramp AMCV parity. **Confidence:** high for unit final Z/landing; medium for blocked-exit active-state preservation. **Source:** low-bridge tube reports; exact blocked-exit object-list scatter/stop side effects need a follow-up RE slice before "full blocked-exit parity" is claimed.
- Keep commits conditional on user approval. **Confidence:** high. **Source:** project AGENTS.md says do not commit unless the user asks; this plan uses verification checkpoints instead of mandatory commits.

## Open Questions

### Resolved During Planning

- Is `Crusher=yes` already parsed and stored? Yes. It is present as `regular_crusher` on `GameEntity` and propagated from rules in spawn/commands.
- Is the old AMCV 3x deployable speed multiplier still present? No. `resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier` pins stock speed.
- Does Drive already have native-shaped state? Yes. `NavigationState`, `DriveLocomotionRuntime`, `DriveTubePayload`, and DriveTrack residual helpers are present.
- Are zero-step low-bridge tube shells valid visible traversal inputs? No for checked Drive/Walk direction-8 producers; they divide by nonzero `TubeClass+0x1C0`.

### Deferred to Implementation

- Exact stock YR AMCV initial facing in the diagonal trace remains unchecked; do not assert pixel/frame-perfect initial-turn parity until a runtime oracle exists.
- Exact retail DriveTrack point coordinates for the AMCV diagonal leg remain unchecked; use existing DriveTrack verified tables, but do not promote AMCV diagonal trace to pixel-perfect acceptance until verified.
- Exact obstacle-detour cell sequence for the blocker fixture remains unchecked; pathing tests should assert mechanism and branch selection, not Rust's current chosen detour cells as the oracle.
- Object/building NavCom target re-aim is deferred for this implementation slice. Entity targets can re-aim to the target entity coordinate; buildings and arbitrary objects must wait for a verified dock/approach coordinate provider rather than aiming at the anchor/center by approximation.
- Exact low-bridge blocked-exit side effects beyond "do not clear active tube state when ground object list is nonempty" are narrower than the full design. This slice preserves active state and returns a blocked result; scatter/stop side effects require a focused RE task before full blocked-exit parity is claimed.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/components.rs` | Extend Drive tube payload and any Drive-owned process state; serde/hash-compatible state only. |
| Modify | `src/sim/game_entity.rs` | Ensure new movement/crush/tube state defaults live on entities. |
| Modify | `src/sim/world/world_hash.rs` | Hash every new deterministic state field. |
| Modify | `src/sim/world/world_commands.rs` | Carry `regular_crusher` into movement/path options without overwriting MovementZone semantics. |
| Modify | `src/sim/world/world_spawn.rs` | Pin propagation tests for Drive/crusher fields from rules to entities. |
| Modify | `src/sim/movement/drive_locomotion.rs` | Add Drive-owned process helpers, NavCom re-aim resolver shell, speed/arrival contracts. |
| Modify | `src/sim/movement/navcom.rs` | Preserve cell/null destination side effects and add target coord resolution support. |
| Modify | `src/sim/movement/movement_commands.rs` | Keep command setup writing NavCom/Drive path directions and pass zone/crusher options. |
| Modify | `src/sim/movement/movement_tick.rs` | Route Drive units through Drive-owned phase, apply crush/tube/arrival ordering. |
| Modify | `src/sim/movement/movement_step.rs` | Prevent unproven straight-vector Drive fallback when DriveTrack/path direction should be authoritative. |
| Modify | `src/sim/movement/bump_crush.rs` | Add regular crusher capability, distance gate, phase split helpers, crusher-coordinate sound. |
| Modify | `src/sim/movement/movement_occupancy.rs` | Thread regular crusher into runtime Can_Enter_Cell and separate entering scatter from full-cell crush. |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Extend occupied-cell classification to accept explicit crush capability. |
| Modify | `src/sim/pathfinding/zone_search.rs` | Add/adjust tests for zone-grid producer use and retry behavior if implementation touches zone inputs. |
| Modify | `src/sim/movement/movement_path.rs` | Ensure player move command paths pass available `ZoneGrid` and keep direction-8 explicit tube paths. |
| Modify | `src/sim/movement/tube_movement.rs` | Replace unit low-bridge tube cadence/final landing with UnitClass payload semantics. |
| Modify | `src/map/tube_facts.rs` | Expose real tube path length/steps needed by Drive unit tube traversal. |
| Modify | `src/sim/movement/movement_tests.rs` | Add end-to-end movement command, AMCV crush, NavCom arrival, and Drive fallback regressions. |

## Interface Changes

- Add `CrushCapability` in `src/sim/movement/bump_crush.rs`:
  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrushCapability {
    pub regular_crusher: bool,
    pub omni_crusher: bool,
}
```
- Change UnitClass crush checks from `(mover_zone, mover_omni_crusher)` to `CrushCapability` in `bump_crush.rs`, `cell_entry.rs`, and `movement_occupancy.rs`. Keep `MovementZone` passability/pathing separate from UnitClass crush capability.
- Add `DriveCrushPhase` and `DriveCrushOutcome` in `bump_crush.rs` for PerCellProcess-compatible entering/full-cell behavior.
- Add a narrow Drive target resolver in `navcom.rs` or `drive_locomotion.rs`:
  ```rust
  pub(super) fn resolve_entity_nav_target_drive_coord(
      target: NavTargetRef,
      entities: &EntityStore,
  ) -> Option<DriveCoord>
  ```
  This implementation slice refreshes only entity targets. Static cell targets keep the terrain-aware `set_destination_internal_cell` coordinate, and `Object`/`Building` return `None` until a verified dock/approach provider is wired.
- Extend the existing `DriveTubePayload` fields (`tube_index`, `cursor`, `path_buffer`, `destination`, `z_accumulator`) with a signed per-step Z delta. Keep current field names unless a code edit proves a rename is necessary.
- Any `DriveTubePayload` field addition must be reflected in serde defaults and deterministic hashing.

## Sim Checklist

- [ ] All math uses integer or fixed-point types; no `f32`/`f64` in sim movement.
- [ ] New Drive/crush/tube state is included in deterministic state hash.
- [ ] No `sim/` dependency on render, ui, sidebar, audio, or net.
- [ ] Tick ordering impact is explicit for Drive phase, crush phase, tube active-state dispatch, and arrival clear.
- [ ] Entity iteration stays deterministic through existing `BTreeMap<u64, GameEntity>` and sorted kill/scatter collections.

## Risk Areas

- Widening crusher logic can accidentally let non-Drive normal vehicles crush through infantry if `regular_crusher` is not read from rules correctly.
- Changing `cell_entry` signatures touches pathfinding, runtime occupancy, gates, bridges, and scatter behavior.
- Moving Drive authority out of `MovementTarget` can regress Walk or scripted movement if helpers are generalized too far.
- Low-bridge tube changes can regress bridge occupancy/Z if unit tube final landing is mixed with high-bridge landing code.
- Zone precheck fixes can change path selection; tests must separate "zone-grid producer used" from "exact detour cells match original" until the detour oracle exists.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 2-5 | `regular_crusher` separate from `MovementZone` | AMCV has `Crusher=yes` with `MovementZone=Normal`; conflating them prevents route infantry crush | `CRUSH_SYSTEM_GHIDRA_REPORT.md`; `cargo test amcv_regular_crusher` |
| 6-8 | PerCellProcess phase split | Entering-cell scatter and fully-in-cell kill occur on different branches/ticks | Tests for entering no-kill and full-cell kill |
| 6-8 | Distance squared `<= 0x3FFF` | Prevents crushing objects too far from crusher center | Boundary tests at `0x3FFF` and `0x4000` |
| 8 | Crush sound at crusher coord | Player-visible audio event position differs if victim coord is used | Sound event assertion uses crusher rx/ry |
| 10-12 | DriveTrack residual/no-fresh-speed retry | Open-ground and diagonal Drive timing depend on residual budget | Existing DriveTrack tests plus no-vector-fallback regression |
| 13-14 | NavCom pending arrival clear | Empty Drive path must clear via null-destination lifecycle, not direct target deletion | Movement test checks one-tick pending clear |
| 15-18 | Direction-8 tube active state | AMCV low-bridge traversal must not use zero-step shells or cell-per-tick movement | Explicit tube tests with nonzero and zero path length |
| 17-18 | Unit tube final Z and blocked exit | Final Z/occupancy differs from high-bridge landing; blocked exit must keep active state | Unit final-Z remainder test and blocked ground-list test |
| 19 | ZoneGrid producer use | Obstacle detours and unreachable checks depend on zone precheck/retry surface | Marker test proves `ZoneGrid` reaches path producer |

---

## Tasks

### Task 1: Pin Current-State Scaffold Invariants

**Why:** Prevent stale trace assumptions from returning while the Drive implementation changes nearby movement code.

**Files:**
- Modify: `src/sim/world/world_commands.rs`
- Modify: `src/sim/world/world_spawn.rs`
- Modify: `src/rules/object_type.rs`
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing inline `#[cfg(test)]` modules beside the code being pinned.

**Step 1: Confirm existing tests remain and rename only if necessary**
Keep these tests or equivalent assertions:
```rust
#[test]
fn resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier() { /* existing */ }

#[test]
fn object_type_parses_regular_crusher_for_amcv_fixture() { /* existing */ }

#[test]
fn resolve_move_info_carries_accelerates_flag() { /* existing */ }
```

**Step 2: Add a spawn propagation assertion if missing**
Add an entity created from `[AMCV]` and assert:
```rust
assert!(entity.regular_crusher);
assert!(entity.drive_accelerates);
assert!(entity.drive_locomotion.is_some());
```

**Step 3: Add a movement command scaffold assertion**
In `movement_tests.rs`, issue a normal Drive move and assert:
```rust
assert_eq!(entity.navigation.nav_com, Some(NavTargetRef::cell(goal_rx, goal_ry)));
assert!(entity.drive_locomotion.as_ref().unwrap().destination.is_some());
assert!(!entity.drive_locomotion.as_ref().unwrap().path.directions.is_empty());
assert!(entity.movement_target.is_some(), "MovementTarget remains adapter state");
```

**Step 4: Verify**
Run:
```powershell
cargo test resolve_move_info_uses_stock_amcv_speed_without_deployable_multiplier object_type_parses_regular_crusher_for_amcv_fixture resolve_move_info_carries_accelerates_flag -- --nocapture
cargo test drive_move_command -- --nocapture
```
Expected: PASS.

### Task 2: Add Explicit CrushCapability

**Why:** AMCV crusher parity depends on `Crusher=yes`, not on `MovementZone=Normal` becoming a crusher zone.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Small copyable value structs near existing crush helpers.

**Step 1: Define the capability**
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrushCapability {
    pub regular_crusher: bool,
    pub omni_crusher: bool,
}

impl CrushCapability {
    pub const fn new(regular_crusher: bool, omni_crusher: bool) -> Self {
        Self { regular_crusher, omni_crusher }
    }
}
```

**Step 2: Replace `can_crush` inputs**
Change:
```rust
pub fn can_crush(
    mover_zone: MovementZone,
    mover_omni_crusher: bool,
    ...
) -> bool
```
to:
```rust
pub fn can_crush(
    capability: CrushCapability,
    target_category: EntityCategory,
    target_crushable: bool,
    target_low_silhouette: bool,
    target_omni_crush_resistant: bool,
) -> bool
```

**Step 3: Implement the decision tree**
- Return `false` for structures and aircraft.
- Return `false` for `target_omni_crush_resistant`.
- Return `true` for `capability.omni_crusher`.
- Return `true` for infantry only when `capability.regular_crusher && target_crushable && !target_low_silhouette`.
- Return `false` otherwise.

Do not treat `MovementZone::Destroyer`, `MovementZone::AmphibiousDestroyer`, `MovementZone::InfantryDestroyer`, or `MovementZone::CrusherAll` as a UnitClass crush grant here. `UnitClass::PerCellProcess @ 0x741700` gates normal crush on `Crusher=yes` or veteran crusher ability; MovementZone belongs to terrain/path behavior.

**Step 4: Update unit tests**
Add:
```rust
#[test]
fn normal_zone_regular_crusher_crushes_crushable_infantry() {
    assert!(can_crush(
        CrushCapability::new(true, false),
        EntityCategory::Infantry,
        true,
        false,
        false,
    ));
}

#[test]
fn normal_zone_non_crusher_still_cannot_crush() {
    assert!(!can_crush(
        CrushCapability::new(false, false),
        EntityCategory::Infantry,
        true,
        false,
        false,
    ));
}

#[test]
fn missing_crusher_flag_does_not_crush_infantry() {
    assert!(!can_crush(
        CrushCapability::new(false, false),
        EntityCategory::Infantry,
        true,
        false,
        false,
    ));
}
```

**Step 5: Verify**
Run:
```powershell
cargo test can_crush -- --nocapture
```
Expected: PASS.

### Task 3: Thread CrushCapability Through CellEntry

**Why:** Runtime `Can_Enter_Cell` and path occupancy classification must consume the same crusher capability instead of re-deriving crusher status from movement zone.

**Files:**
- Modify: `src/sim/pathfinding/cell_entry.rs`
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Existing `CanEnterLayerContext` explicit-parameter style.

**Step 1: Change function signatures**
Change occupied-cell classifiers from:
```rust
mover_zone: MovementZone,
mover_omni_crusher: bool,
```
to:
```rust
crush_capability: bump_crush::CrushCapability,
```
in `classify_occupied_cell`, `classify_occupied_cell_with_layers`, and the internal helper.

For the lower-level helpers in `bump_crush.rs`, keep the current cell/occupancy/layer API shape and replace only the crusher capability inputs:
```rust
pub fn collect_crush_victims(
    cell: (u16, u16),
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    crush_capability: CrushCapability,
    entities: &EntityStore,
) -> Vec<u64>

pub fn cell_passable_after_crush(
    cell: (u16, u16),
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    crush_capability: CrushCapability,
    entities: &EntityStore,
) -> bool
```

**Step 2: Update crush calls**
Preserve the existing `CanEnterLayerContext` split: victims are collected from `layers.object_list_layer`, while passability-after-crush is checked against `layers.occupancy_bits_layer`.

Call:
```rust
let victims = bump_crush::collect_crush_victims(
    target,
    occupancy,
    layers.object_list_layer,
    crush_capability,
    entities,
);
```
and:
```rust
bump_crush::cell_passable_after_crush(
    target,
    occupancy,
    layers.occupancy_bits_layer,
    crush_capability,
    entities,
)
```

**Step 3: Update local tests**
Add a cell-entry test:
```rust
#[test]
fn normal_zone_regular_crusher_yields_crushable_entry() {
    let result = classify_occupied_cell_with_layers(
        target_cell,
        CanEnterLayerContext::single(MovementLayer::Ground),
        mover_id,
        CrushCapability::new(true, false),
        mover_owner,
        LocomotorKind::Drive,
        false,
        &occupancy,
        &entities,
        &alliances,
        &interner,
    );
    assert_eq!(result.yr_code(), 1);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test normal_zone_regular_crusher_yields_crushable_entry -- --nocapture
```
Expected: PASS.

### Task 4: Thread Regular Crusher Through MoveInfo and Occupancy

**Why:** `resolve_move_info` already carries `regular_crusher`; runtime movement still ignores it when computing `mover_is_crusher`.

**Files:**
- Modify: `src/sim/world/world_commands.rs`
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/movement/movement_commands.rs`

**Pattern:** Existing `MoveInfo` flow from world command to movement command/path options.

**Step 1: Replace the stale inert-crusher test**
Replace:
```rust
assert!(!info.mover_is_crusher);
```
with assertions that distinguish capability fields:
```rust
assert!(info.regular_crusher);
assert!(!matches!(info.movement_zone, MovementZone::Crusher | MovementZone::CrusherAll));
assert!(info.can_crush_units());
```

**Step 2: Add `omni_crusher` and helpers on `MoveInfo`**
Extend `MoveInfo` with the missing field:
```rust
pub(crate) omni_crusher: bool,
```
and set it in `resolve_move_info`:
```rust
omni_crusher: e.omni_crusher,
```

Then add:
```rust
impl MoveInfo {
    pub(crate) fn crush_capability(&self) -> CrushCapability {
        CrushCapability::new(self.regular_crusher, self.omni_crusher)
    }

    pub(crate) fn can_crush_units(&self) -> bool {
        self.regular_crusher || self.omni_crusher
    }
}
```

**Step 3: Update movement occupancy snapshots**
Add `regular_crusher` to the snapshot struct used by `handle_deferred_occupancy`, then build:
```rust
let crush_capability = CrushCapability::new(
    snap.regular_crusher,
    snap.omni_crusher,
);
```

**Step 4: Keep pathfinding soft-block intent explicit**
Where path options need the old boolean for unit-crush soft-block handling, pass `info.can_crush_units()` and name the receiving field `mover_can_crush_units` if the local struct is touched. Do not change terrain passability from `MovementZone`, and do not use MovementZone destroyer-family values as proof of UnitClass crush capability.

**Step 5: Verify**
Run:
```powershell
cargo test resolve_move_info_carries_regular_crusher -- --nocapture
cargo test normal_zone_regular_crusher -- --nocapture
```
Expected: PASS.

### Task 5: Add PerCellProcess Crush Phase Types

**Why:** The trace review found a missing entering-vs-full-cell split; AMCV must scatter on entry and kill only after it is fully in the cell.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Existing pure helper tests in `bump_crush.rs`.

**Step 1: Define phase and outcome**
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriveCrushPhase {
    EnteringCell,
    FullyInCell,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriveCrushOutcome {
    None,
    Scatter { blockers: Vec<u64> },
    Kill { victims: Vec<u64> },
}
```

**Step 2: Add distance gate helper**
```rust
pub const CRUSH_DISTANCE_SQ_LIMIT: i64 = 0x3fff;

pub fn within_crush_distance_sq(
    crusher: (i32, i32),
    victim: (i32, i32),
) -> bool {
    let dx = i64::from(victim.0 - crusher.0);
    let dy = i64::from(victim.1 - crusher.1);
    dx * dx + dy * dy <= CRUSH_DISTANCE_SQ_LIMIT
}
```

**Step 3: Add tests**
```rust
#[test]
fn crush_distance_gate_includes_0x3fff() {
    assert!(within_crush_distance_sq((0, 0), (127, 14)));
}

#[test]
fn crush_distance_gate_excludes_0x4000() {
    assert!(!within_crush_distance_sq((0, 0), (128, 0)));
}
```

**Step 4: Verify**
Run:
```powershell
cargo test crush_distance_gate -- --nocapture
```
Expected: PASS.

### Task 6: Implement Pure PerCellProcess Crush Classification

**Why:** Classification must be testable before it is wired into movement tick ordering.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Existing `collect_crush_victims` with explicit capability and entity-store inputs.

**Step 1: Add the helper**
```rust
pub fn classify_drive_crush_phase(
    phase: DriveCrushPhase,
    occ: &[u64],
    entities: &EntityStore,
    crusher_id: u64,
    alliances: &HouseAllianceMap,
    interner: &StringInterner,
    crusher_coord: (i32, i32),
    capability: CrushCapability,
) -> DriveCrushOutcome
```

**Step 2: Implement exact branch shape**
- If the crusher lacks regular/veteran/omni capability, return `None`.
- For `EnteringCell`, return `Scatter { blockers }` with eligible occupants that are on the selected cell object list and not the crusher.
- For `FullyInCell`, return `Kill { victims }` only for occupants where `can_crush` passes, `are_houses_friendly(alliances, crusher_owner, victim_owner)` is false unless a verified train exception is represented, limbo/falling checks pass through current entity flags, and `within_crush_distance_sq` passes.
- Return victim ids in sorted ascending order for deterministic state updates.

**Step 3: Add tests**
```rust
#[test]
fn entering_phase_scatters_without_kill() {
    let outcome = classify_drive_crush_phase(
        DriveCrushPhase::EnteringCell,
        &[2],
        &entities,
        1,
        &alliances,
        &interner,
        (128, 128),
        CrushCapability::new(true, false),
    );
    assert_eq!(outcome, DriveCrushOutcome::Scatter { blockers: vec![2] });
}

#[test]
fn full_cell_phase_kills_centered_enemy() {
    let outcome = classify_drive_crush_phase(
        DriveCrushPhase::FullyInCell,
        &[2],
        &entities,
        1,
        &alliances,
        &interner,
        (128, 128),
        CrushCapability::new(true, false),
    );
    assert_eq!(outcome, DriveCrushOutcome::Kill { victims: vec![2] });
}

#[test]
fn full_cell_phase_skips_allied_victim() {
    let outcome = classify_drive_crush_phase(
        DriveCrushPhase::FullyInCell,
        &[2],
        &entities,
        1,
        &alliances,
        &interner,
        (128, 128),
        CrushCapability::new(true, false),
    );
    assert_eq!(outcome, DriveCrushOutcome::None);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test classify_drive_crush_phase -- --nocapture
```
Expected: PASS.

### Task 7: Wire Entering-Cell Scatter Without Premature Kill

**Why:** Movement entry must not remove victims during the `entering != 0` PerCellProcess branch.

**Files:**
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing deferred occupancy and `scatter_blocker` handling.

**Step 1: Add an entering-phase call before final occupancy removal**
When a Drive unit first attempts to enter a cell with eligible occupants, call:
```rust
let outcome = bump_crush::classify_drive_crush_phase(
    DriveCrushPhase::EnteringCell,
    occ,
    entities,
    mover_id,
    alliances,
    interner,
    crusher_coord,
    crush_capability,
);
```

**Step 2: Apply only scatter side effects**
For `DriveCrushOutcome::Scatter { blockers }`, call existing `scatter_blocker` for each blocker if it has not already scattered this occupancy pass. Do not push to `crush_kills`.

**Step 3: Add regression test**
```rust
#[test]
fn amcv_entering_crush_cell_scatters_but_does_not_kill() {
    tick_until_entering_crush_cell(&mut sim);
    assert!(sim.entities.contains_key(&victim_id));
    assert!(sim.entities.get(&victim_id).unwrap().movement_target.is_some());
}
```

**Step 4: Verify**
Run:
```powershell
cargo test amcv_entering_crush_cell_scatters_but_does_not_kill -- --nocapture
```
Expected: PASS.

### Task 8: Wire Fully-In-Cell Crush Kill and Sound Coordinates

**Why:** The AMCV route-infantry trace must remove the E1 and emit the crush sound at the crusher position.

**Files:**
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/movement/bump_crush.rs`

**Pattern:** Existing `crush_kills` drain and `emit_crush_kill_sounds`.

**Step 1: Add sound helper with crusher coordinates**
Replace or overload:
```rust
pub fn emit_crush_kill_sounds(
    victim: &GameEntity,
    rules: &Rules,
    interner: &mut StringInterner,
    sound_events: &mut Vec<SimSoundEvent>,
)
```
with:
```rust
pub fn emit_crush_kill_sounds_at(
    victim: &GameEntity,
    crush_coord: (i32, i32),
    rules: &Rules,
    interner: &mut StringInterner,
    sound_events: &mut Vec<SimSoundEvent>,
)
```

**Step 2: Store pending crusher coordinate with kill**
Change `crush_kills: Vec<u64>` to:
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingCrushKill {
    victim_id: u64,
    crusher_id: u64,
    crush_coord: (i32, i32),
}
```

**Step 3: Apply full-cell phase**
Only when a Drive unit is fully in the cell, call `classify_drive_crush_phase(FullyInCell, ...)`, remove victims from occupancy, and queue `PendingCrushKill`.

**Step 4: Add tests**
```rust
#[test]
fn amcv_fully_in_cell_crushes_centered_enemy_e1() {
    tick_until_fully_in_crush_cell(&mut sim);
    assert!(!sim.entities.contains_key(&victim_id));
}

#[test]
fn crush_sound_uses_crusher_coordinates() {
    let event = drain_entity_crushed_event(&mut sim);
    assert_eq!((event.rx, event.ry), crusher_cell);
}
```

**Step 5: Verify**
Run:
```powershell
cargo test amcv_fully_in_cell_crushes_centered_enemy_e1 crush_sound_uses_crusher_coordinates -- --nocapture
```
Expected: PASS.

### Task 9: Preserve Deterministic Crush Side Effects

**Why:** Crush kills update entity store, occupancy, sound, and kill ownership; ordering must be deterministic.

**Files:**
- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/world/world_hash.rs` only if new persistent kill state is introduced.

**Pattern:** Existing deferred kill and sound event drains.

**Step 1: Drain pending kills in stable order**
Sort by `(victim_id, crusher_id)` before removal if the collection can receive from multiple movers in one tick:
```rust
pending_crush_kills.sort_by_key(|kill| (kill.victim_id, kill.crusher_id));
```

**Step 2: Keep occupancy removal immediate**
When a victim is selected for full-cell crush, remove it from ground/bridge occupancy before draining the entity store so later same-tick entry checks see the cell as cleared.

**Step 3: Record killer id where existing kill/stat hooks support it**
Use existing kill/stat APIs only. If there is no kill attribution surface, leave entity deletion and sound identical to current deferred death behavior and record the gap in the final implementation notes, not as code state.

**Step 4: Add hash stability test if persistent state changes**
If `PendingCrushKill` remains frame-local, no hash change is required. If persistent queue state is added to entities/world, add:
```rust
#[test]
fn pending_crush_kill_changes_world_hash() {
    assert_ne!(hash_without_pending, hash_with_pending);
}
```

**Step 5: Verify**
Run:
```powershell
cargo test crush -- --nocapture
```
Expected: PASS.

### Task 10: Add Drive Process Outcome Shell

**Why:** Drive-specific movement needs a named owner before behavior moves out of the broad movement tick loop.

**Files:**
- Modify: `src/sim/movement/drive_locomotion.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing small helper functions in `drive_locomotion.rs`.

**Step 1: Add the outcome type**
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DriveProcessOutcome {
    NotDrive,
    Processed,
    Waiting,
    Arrived,
    Blocked,
}
```

**Step 2: Add a no-behavior wrapper**
```rust
pub(super) fn process_drive_locomotion_shell(entity: &GameEntity) -> DriveProcessOutcome {
    if entity.drive_locomotion.is_none() {
        return DriveProcessOutcome::NotDrive;
    }
    DriveProcessOutcome::Processed
}
```

**Step 3: Call the shell from `movement_tick.rs` without changing behavior**
Use the shell only for debug assertion/tests in this task. Do not early-return from the existing movement loop yet.

**Step 4: Verify**
Run:
```powershell
cargo test drive_locomotion -- --nocapture
```
Expected: PASS.

### Task 11: Make DriveTrack/Path Direction Authority Explicit

**Why:** A Drive unit with path directions must not silently use generic full-speed vector stepping when gamemd would consume DriveTrack/path direction state.

**Files:**
- Modify: `src/sim/movement/drive_locomotion.rs`
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing `advance_drive_track_retry_after_selection` and `drive_track_fresh_budget_from_current_speed`.

**Step 1: Add a predicate**
```rust
pub(super) fn drive_requires_native_step(drive: &DriveLocomotionRuntime) -> bool {
    drive.active_tube.is_some()
        || !drive.path.directions.is_empty()
        || drive.residual_budget != 0
}
```

**Step 2: Gate generic vector fallback**
In `advance_lepton_position`, when `drive_locomotion` is `Some` and `drive_requires_native_step` is true, select/advance DriveTrack or return a `Blocked/Waiting` result. Do not fall through to straight vector stepping for that tick.

**Step 3: Add regression test**
```rust
#[test]
fn drive_with_pending_direction_does_not_use_straight_vector_fallback() {
    let before = entity.position;
    let moved = advance_one_drive_tick_with_no_track_selected(&mut entity);
    assert!(entity.drive_track.is_some() || !moved.used_vector_step);
    assert_ne!(entity.position, before);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test drive_track_completion_retries_new_track_with_residual_only drive_with_pending_direction_does_not_use_straight_vector_fallback -- --nocapture
```
Expected: PASS.

### Task 12: Preserve Drive Speed Fraction and Residual Budget

**Why:** `Accelerates=` and current-speed fraction now exist; Drive process extraction must keep those results byte-identical.

**Files:**
- Modify: `src/sim/movement/drive_locomotion.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing `compute_drive_target_speed_fraction` and `update_drive_speed_fraction` tests.

**Step 1: Move speed fraction calls into the Drive process phase**
For Drive units, call:
```rust
let target = compute_drive_target_speed_fraction(...);
let current = update_drive_speed_fraction(...);
```
inside the Drive-specific branch before DriveTrack budget calculation.

**Step 2: Keep non-Drive movement speed path unchanged**
Leave non-Drive `MovementTarget.current_speed` handling in the existing branch.

**Step 3: Add AMCV fixture assertion**
```rust
#[test]
fn amcv_drive_speed_uses_stock_speed_and_fraction() {
    assert_eq!(move_info.speed, ra2_speed_to_leptons_per_second(4));
    assert!(entity.drive_locomotion.as_ref().unwrap().target_speed_fraction > 0);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test drive_speed_fraction amcv_drive_speed_uses_stock_speed_and_fraction -- --nocapture
```
Expected: PASS.

### Task 13: Pin Arrival Clear Through NavCom Null Destination

**Why:** Empty Drive arrival must clear through the native-shaped pending null-destination lifecycle already scaffolded in Rust.

**Files:**
- Modify: `src/sim/movement/navcom.rs`
- Modify: `src/sim/movement/movement_tick.rs`
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing `defer_drive_arrival_clear` and `process_pending_empty_drive_arrivals`.

**Step 1: Add test for pending clear**
```rust
#[test]
fn drive_arrival_defers_then_clears_navcom_via_null_destination() {
    tick_until_drive_reaches_goal(&mut sim, mover_id);
    assert!(sim.entities[&mover_id].navigation.pending_arrival_clear);
    sim.advance_tick();
    assert_eq!(sim.entities[&mover_id].navigation.nav_com, None);
    assert!(!sim.entities[&mover_id].navigation.pending_arrival_clear);
}
```

**Step 2: Ensure finalization uses the helper**
Keep:
```rust
if !defer_drive_arrival_clear(entity) {
    set_destination_internal_null(entity);
}
```
for Drive arrival. Do not directly clear `movement_target` without the NavCom helper.

**Step 3: Verify**
Run:
```powershell
cargo test drive_arrival_defers_then_clears_navcom_via_null_destination -- --nocapture
```
Expected: PASS.

### Task 14: Add Cell and Entity NavCom Re-Aim Resolver

**Why:** This implementation slice needs moving entity targets to refresh Drive head-to coordinates without inventing building/object dock behavior.

**Files:**
- Modify: `src/sim/movement/navcom.rs`
- Modify: `src/sim/movement/drive_locomotion.rs`

**Pattern:** Existing `NavTargetRef` enum and `DriveCoord` conversion helpers.

**Step 1: Add resolver**
```rust
pub(super) fn resolve_entity_nav_target_drive_coord(
    target: NavTargetRef,
    entities: &EntityStore,
) -> Option<DriveCoord> {
    match target {
        NavTargetRef::Entity { id } => entities.get(&id).map(|entity| {
            let pos = &entity.position;
            DriveCoord {
                x: i32::from(pos.rx) * 256 + pos.sub_x.to_num::<i32>(),
                y: i32::from(pos.ry) * 256 + pos.sub_y.to_num::<i32>(),
                z: i32::from(pos.z),
            }
        }),
        NavTargetRef::Cell { .. }
        | NavTargetRef::Object { .. }
        | NavTargetRef::Building { .. } => None,
    }
}
```

Do not refresh cell targets through this helper. Cell destinations are static and already use `target_cell_coord(..., resolved_terrain)` in `set_destination_internal_cell`, including bridge deck or terrain level. A resolver that reconstructs cells without terrain can incorrectly overwrite `drive.head_to.z`.

**Step 2: Add tests**
```rust
#[test]
fn resolve_nav_target_drive_coord_tracks_moving_entity() {
    let first = resolve_entity_nav_target_drive_coord(NavTargetRef::Entity { id: 2 }, &entities).unwrap();
    entities.get_mut(&2).unwrap().position.rx += 1;
    let second = resolve_entity_nav_target_drive_coord(NavTargetRef::Entity { id: 2 }, &entities).unwrap();
    assert_ne!(first, second);
}

#[test]
fn resolve_nav_target_drive_coord_does_not_reaim_cell_targets() {
    assert_eq!(
        resolve_entity_nav_target_drive_coord(NavTargetRef::Cell { rx: 12, ry: 34 }, &entities),
        None
    );
}

#[test]
fn resolve_nav_target_drive_coord_does_not_guess_building_anchor() {
    assert_eq!(
        resolve_entity_nav_target_drive_coord(NavTargetRef::Building { id: 7 }, &entities),
        None
    );
}
```

**Step 3: Verify**
Run:
```powershell
cargo test resolve_nav_target_drive_coord -- --nocapture
```
Expected: PASS.

### Task 15: Wire Drive Head-To Re-Aim

**Why:** A Drive unit chasing a moving entity target must refresh its locomotor head-to coordinate instead of staying aimed at the original target location.

**Files:**
- Modify: `src/sim/movement/drive_locomotion.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing `set_destination_internal_cell` side effects for `drive.destination` and `drive.head_to`.

**Step 1: Add re-aim helper**
```rust
pub(super) fn refresh_drive_head_to_from_navcom(
    entity: &mut GameEntity,
    entities: &EntityStore,
) -> bool
```
Return `true` when an entity-target re-aim changes `drive.head_to`. Return `false` for cell targets, object targets, and building targets.

**Step 2: Avoid borrow conflicts**
Collect only `(entity_id, nav_target)` rows whose `nav_target` is `NavTargetRef::Entity { .. }` in a first immutable pass, resolve target coordinates from `entities`, then mutably borrow the moving entity to update `drive.head_to`. Do not include `NavTargetRef::Cell` in this refresh pass unless the helper is extended to accept `ResolvedTerrainGrid` and reuse the same terrain-aware path as `set_destination_internal_cell`.

**Step 3: Add moving-target test**
```rust
#[test]
fn drive_navcom_entity_target_reaims_when_target_moves() {
    issue_drive_move_to_entity(&mut sim, mover_id, target_id);
    let before = sim.entities[&mover_id].drive_locomotion.as_ref().unwrap().head_to;
    sim.entities.get_mut(&target_id).unwrap().position.rx += 2;
    sim.advance_tick();
    let after = sim.entities[&mover_id].drive_locomotion.as_ref().unwrap().head_to;
    assert_ne!(before, after);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test drive_navcom_entity_target_reaims_when_target_moves -- --nocapture
```
Expected: PASS.

### Task 16: Extend DriveTubePayload for UnitClass TubeMovement

**Why:** Current `DriveTubePayload` already has active tube index, cursor, copied path buffer, destination, and Z accumulator fields; it still needs an explicit signed per-step Z delta so UnitClass tube traversal can preserve the verified final-Z remainder behavior.

**Files:**
- Modify: `src/sim/components.rs`
- Modify: `src/sim/world/world_hash.rs`

**Pattern:** Existing serde/default/hash tests for `DriveLocomotionRuntime`.

**Step 1: Extend the payload**
Keep the current field names and add only the missing signed Z step:
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveTubePayload {
    pub tube_index: Option<u16>,
    pub cursor: u16,
    pub path_buffer: Vec<DriveCoord>,
    pub destination: Option<DriveCoord>,
    pub z_accumulator: i32,
    #[serde(default)]
    pub z_step: i32,
}
```

**Step 2: Add serde defaults if fields are optional for old saves**
If existing `DriveTubePayload` is deserialized from older JSON tests, use `#[serde(default)]` on new fields and keep `Default` deterministic.

**Step 3: Hash all fields**
Update `world_hash.rs` to feed `tube_index`, `cursor`, every `path_buffer` entry, `destination`, `z_accumulator`, and `z_step`.

**Step 4: Add tests**
```rust
#[test]
fn drive_tube_payload_hash_changes_with_z_accumulator() {
    let mut a = DriveLocomotionRuntime::default();
    let mut b = DriveLocomotionRuntime::default();
    a.active_tube = Some(DriveTubePayload::default());
    b.active_tube = Some(DriveTubePayload::default());
    b.active_tube.as_mut().unwrap().z_accumulator += 1;
    assert_ne!(hash_drive(&a), hash_drive(&b));
}
```

**Step 5: Verify**
Run:
```powershell
cargo test drive_tube -- --nocapture
```
Expected: PASS.

### Task 17: Begin Direction-8 Unit Tube Traversal With Nonzero Guard

**Why:** Direction-8 traversal must use real explicit tube path data and must not consume zero-step low-bridge shells.

**Files:**
- Modify: `src/sim/movement/tube_movement.rs`
- Modify: `src/map/tube_facts.rs`
- Modify: `src/sim/movement/movement_step.rs`

**Pattern:** Existing `begin_low_bridge_tube_movement` error handling.

**Step 1: Expose path length**
In `tube_facts.rs`, add or confirm:
```rust
impl TubeFact {
    pub fn path_len(&self) -> usize { self.path_steps.len() }
    pub fn path_steps(&self) -> &[u8] { &self.path_steps }
}
```

**Step 2: Reject zero-step traversal**
In tube begin logic:
```rust
if tube.path_len() == 0 {
    return Err(TubeBeginError::ZeroLengthTube);
}
```

**Step 3: Seed UnitClass payload**
Compute:
```rust
let z_step = (exit_ground - entry_ground) / tube.path_len() as i32;
let z_accumulator = entry_ground + z_step;
```
Use signed integer division and do not compensate for remainder. Store the copied path as target `DriveCoord` values in `DriveTubePayload.path_buffer`; `TubeFact.path_steps` remains `Vec<u8>` in current Rust, and the end-of-path condition is `cursor >= path_buffer.len()` rather than storing a `-1` sentinel.

**Step 4: Add tests**
```rust
#[test]
fn direction8_rejects_zero_step_tube_shell() {
    assert_eq!(begin_drive_tube_traversal(...).unwrap_err(), TubeBeginError::ZeroLengthTube);
}

#[test]
fn direction8_seeds_z_step_with_signed_truncation() {
    let payload = begin_drive_tube_traversal(...).unwrap();
    assert_eq!(payload.z_step, (exit_ground - entry_ground) / path_len as i32);
}
```

**Step 5: Verify**
Run:
```powershell
cargo test direction8_ -- --nocapture
```
Expected: PASS.

### Task 18: Tick Unit TubeMovement by Speed Budget

**Why:** Unit TubeMovement advances by available movement budget and at most one cursor increment per tick, not one cell per Rust tick.

**Files:**
- Modify: `src/sim/movement/tube_movement.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing Drive budget math and `tick_low_bridge_tube_movement` entrypoint.

**Step 1: Add pure advancement result**
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitTubeAdvance {
    Partial,
    AdvancedStep,
    ReachedFinal,
    BlockedFinal,
}
```

**Step 2: Implement budget movement**
```rust
pub fn tick_unit_tube_payload(
    payload: &mut DriveTubePayload,
    position: &mut Position,
    budget: i32,
    tube: &TubeFact,
) -> UnitTubeAdvance
```
Move toward `payload.destination` by `budget`. If target distance is greater than budget, update position partially and return `Partial`. If the target is reached, increment `cursor` once, add `z_step` to `z_accumulator`, seed `destination` from the next `path_buffer` entry if one remains, and return `AdvancedStep`.

**Step 3: Add tests**
```rust
#[test]
fn unit_tube_partial_budget_does_not_increment_cursor() {
    let result = tick_unit_tube_payload(&mut payload, &mut position, 4, &tube);
    assert_eq!(result, UnitTubeAdvance::Partial);
    assert_eq!(payload.cursor, 0);
}

#[test]
fn unit_tube_reaches_one_step_per_tick() {
    let result = tick_unit_tube_payload(&mut payload, &mut position, large_budget, &tube);
    assert_eq!(result, UnitTubeAdvance::AdvancedStep);
    assert_eq!(payload.cursor, 1);
}
```

**Step 4: Verify**
Run:
```powershell
cargo test unit_tube_ -- --nocapture
```
Expected: PASS.

### Task 19: Implement Unit Tube Final Landing and Bounded Blocked Exit

**Why:** Unit final tube exit must not use high-bridge landing; this slice proves the verified blocked-exit invariant that a nonempty ground list does not clear active tube state.

**Files:**
- Modify: `src/sim/movement/tube_movement.rs`
- Modify: `src/sim/movement/movement_occupancy.rs`
- Modify: `src/sim/movement/movement_tick.rs`

**Pattern:** Existing occupancy list checks and final movement state updates.

**Step 1: Add final landing helper**
```rust
pub fn finish_unit_tube_movement(
    entity: &mut GameEntity,
    tube: &TubeFact,
    exit_ground_blocked: bool,
) -> UnitTubeAdvance
```

**Step 2: Empty ground list behavior**
When `exit_ground_blocked == false`:
- Set X/Y to `TubeClass+0x28` cell center.
- Set Z to `payload.z_accumulator`.
- Clear `drive.active_tube`.
- Do not set high-bridge `on_bridge` or bridge occupancy as part of this low-bridge final branch.

**Step 3: Nonempty ground list behavior**
When `exit_ground_blocked == true`:
- Return `UnitTubeAdvance::BlockedFinal`.
- Keep `drive.active_tube` unchanged.
- Do not snap X/Y to exit.
- Do not clear active tube state.
- Do not add scatter/stop side effects in this task; those require a follow-up RE-backed blocked-exit task.

**Step 4: Add tests**
```rust
#[test]
fn unit_tube_final_empty_ground_list_keeps_accumulated_z() {
    let result = finish_unit_tube_movement(&mut entity, &tube, false);
    assert_eq!(result, UnitTubeAdvance::ReachedFinal);
    assert_eq!(entity.position.z, payload_z_before_finish);
    assert!(entity.drive_locomotion.as_ref().unwrap().active_tube.is_none());
}

#[test]
fn unit_tube_final_blocked_ground_list_keeps_active_tube() {
    let result = finish_unit_tube_movement(&mut entity, &tube, true);
    assert_eq!(result, UnitTubeAdvance::BlockedFinal);
    assert!(entity.drive_locomotion.as_ref().unwrap().active_tube.is_some());
}
```

**Step 5: Verify**
Run:
```powershell
cargo test unit_tube_final_ -- --nocapture
```
Expected: PASS.

### Task 20: Audit and Fix ZoneGrid Producer Use for Player Move Commands

**Why:** The zone-search implementation exists, but player command producers can still bypass it by passing `zone_grid: None`.

**Files:**
- Modify: `src/sim/world/world_commands.rs`
- Modify: `src/sim/movement/movement_commands.rs`
- Modify: `src/sim/movement/movement_path.rs`
- Modify: `src/sim/pathfinding/zone_search.rs`

**Pattern:** Existing `MovementPathContext { zone_grid: Option<&ZoneGrid>, ... }`.

**Step 1: Add a marker test**
Use the existing marker path function to prove whether a normal player AMCV move receives `Some(&ZoneGrid)`:
```rust
#[test]
fn player_drive_move_command_passes_zone_grid_to_path_search() {
    let marker = issue_marked_move_command(&mut sim, amcv_id, goal);
    assert!(marker.used_zone_grid);
}
```

**Step 2: Thread available zone grid from world command**
Where `apply_command` or its movement callsite has access to grid/path context, pass `Some(zone_grid)` into `MovementPathContext`.

**Step 3: Keep fallback explicit**
If no zone grid exists in a test-only fixture, pass `None` explicitly and assert the fallback path still works.

**Step 4: Verify**
Run:
```powershell
cargo test player_drive_move_command_passes_zone_grid_to_path_search explicit_tube_path_survives_zone_precheck_and_smoothing -- --nocapture
```
Expected: PASS.

### Task 21: End-to-End AMCV Regression Fixtures

**Why:** The isolated pieces must produce the player-visible trace outcomes without using current Rust drift as the oracle.

**Files:**
- Modify: `src/sim/movement/movement_tests.rs`

**Pattern:** Existing movement integration tests.

**Step 1: Add open-ground acceptance**
Assert AMCV `Speed=4`, Drive locomotor, NavCom cell target, DriveTrack/path direction consumption, and no generic vector fallback flag for the first moving tick.

**Step 2: Add crush-on-path acceptance**
Place AMCV and enemy E1 centered on the route. Assert E1 is alive during entering scatter phase, dead after fully-in-cell phase, occupancy is cleared, and crush sound coordinate equals AMCV coordinate.

**Step 3: Add low-bridge acceptance**
Use explicit nonzero tube path fixture. Assert direction 8 starts active tube, cursor advances by budget, final Z preserves signed-truncation remainder, and blocked exit keeps active tube.

**Step 4: Keep obstacle-detour assertion mechanism-based**
Assert `ZoneGrid` was used, crushable soft-blocks use `regular_crusher`, and runtime cell-entry is called with Drive-compatible arguments. Do not assert exact detour cells until a gamemd oracle is captured.

**Step 5: Verify**
Run:
```powershell
cargo test amcv_open_ground amcv_crush_on_path amcv_low_bridge -- --nocapture
```
Expected: PASS.

### Task 22: Focused Regression and Final Check

**Why:** Movement changes touch shared sim behavior; focused tests must pass before a final compile check.

**Files:**
- No code edits unless a preceding test exposes a failure in a touched file.

**Pattern:** Project cargo coordination rule: check for active cargo/rustc before long runs.

**Step 1: Check for active builds**
Run:
```powershell
Get-Process cargo,rustc -ErrorAction SilentlyContinue | Select-Object ProcessName,Id,CPU
```
If another active session owns Cargo, wait or ask before starting another build.

**Step 2: Run focused test groups**
Run:
```powershell
cargo test drive_locomotion -- --nocapture
cargo test drive_track -- --nocapture
cargo test crush -- --nocapture
cargo test unit_tube -- --nocapture
cargo test zone_search -- --nocapture
cargo test movement_tests -- --nocapture
```

**Step 3: Run final check**
Run:
```powershell
cargo check -q
```
Expected: PASS.

**Step 4: Final implementation notes**
Report any remaining `UNCHECKED` parity limits explicitly:
- AMCV exact initial facing if no runtime oracle exists.
- Exact diagonal DriveTrack pixel path if not independently verified.
- Exact obstacle-detour cells if no gamemd cell sequence oracle exists.
- Exact blocked-tube side effects beyond active-state preservation if no follow-up research was run.

## Sources & References

- **Design doc:** `docs/plans/2026-05-27-drivelocomotion-current-state-parity-design.md`
- **Trace reports:** `docs/research/traces/AMCV_OPEN_GROUND_DRIVE_TRACE_20260527.md`; `docs/research/traces/AMCV_TURNING_DIAGONAL_DRIVE_TRACE_20260527.md`; `docs/research/traces/AMCV_OBSTACLE_DETOUR_TRACE_20260527.md`; `docs/research/traces/AMCV_BRIDGE_RAMP_TRAVERSAL_TRACE_20260527.md`; `docs/research/traces/AMCV_CRUSH_ON_PATH_TRACE_20260527.md`; `docs/research/traces/MCV_CRUSH_SINGLE_CONSCRIPT_ALONG_PATH_TRACE.md`
- **Drive/NavCom reports:** `docs/research/DRIVELOCOMOTION_ARRIVAL_QUEUE_NULL_DESTINATION_GHIDRA_REPORT.md`; `docs/research/DRIVELOCOMOTION_HEAD_TO_COORD_CLEAR_NAVIGATION_STATE_GHIDRA_REPORT.md`; `docs/research/UNITCLASS_SET_DESTINATION_NORMAL_DRIVE_CELL_GHIDRA_REPORT.md`; `docs/research/FOOTCLASS_SET_DESTINATION_INTERNAL_NAVCOM_HEADTO_HANDOFF_GHIDRA_REPORT.md`; `docs/research/NAVCOM_LIFECYCLE_GHIDRA_REPORT.md`; `docs/research/DRIVELOCOMOTOR_ACCEPTED_CELL_ARRIVAL_VISIBILITY_GHIDRA_REPORT.md`
- **DriveTrack/path reports:** `docs/research/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md`; `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`; `docs/research/miner/DRIVELOCOMOTION_PROCESS_DRIVE_TRACK_CHRONO_MINER_004B0F20_GHIDRA_REPORT.md`; `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- **Crush reports:** `docs/research/CRUSH_SYSTEM_GHIDRA_REPORT.md`; `docs/research/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
- **Bridge/tube reports:** `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`; `docs/research/bridges/04-locomotion-height-tubes/LOW_BRIDGE_TUBEMOVEMENT_FINAL_Z_INTERPOLATION_GHIDRA_REPORT.md`; `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`; `docs/research/RUNTIME_CAN_ENTER_CELL_NONCOVERED_CALLSITES_GHIDRA_REPORT.md`
- **INI keys:** `ini/rulesmd.ini:6969-7000 [AMCV] Speed=4, ROT=5, Crusher=yes, Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}, MovementZone=Normal`; `ini/rulesmd.ini:3713-3758 [E1] CrushSound=InfantrySquish, Crushable=yes`; `ini/rulesmd.ini:365 [General] TunnelSpeed=1`
- **Current Rust:** `src/sim/components.rs`; `src/sim/game_entity.rs`; `src/sim/world/world_commands.rs`; `src/sim/world/world_spawn.rs`; `src/sim/movement/navcom.rs`; `src/sim/movement/movement_commands.rs`; `src/sim/movement/drive_locomotion.rs`; `src/sim/movement/movement_tick.rs`; `src/sim/movement/movement_step.rs`; `src/sim/movement/bump_crush.rs`; `src/sim/movement/movement_occupancy.rs`; `src/sim/pathfinding/cell_entry.rs`; `src/sim/movement/tube_movement.rs`; `src/map/tube_facts.rs`; `src/sim/world/world_hash.rs`
- **Recent commits touching plan files:** `103aee0 sim: MCV deploy facing turn + ConstructionYard undeploy gating`; `2e39817 sim: add bridge-crossing oracle diagnostic surfaces`; `1102241 sim/movement: locomotor piggyback model + CMIN teleport routing`; `e0206d0 Tighten sim parity across movement, rules, and resource systems`

## Post-Plan Self-Review

- Spec coverage: The plan maps current-state pinning, regular crusher, DriveTrack authority, NavCom arrival/entity re-aim, bounded low-bridge tube traversal, ZoneGrid producer use, and trace regression tasks to the first implementation slice. Building/object dock re-aim and full blocked-exit scatter/stop side effects are explicit follow-up work.
- Placeholder scan: No task contains unresolved placeholder wording or vague follow-up work items.
- Architecture check: All edited files remain in `sim/`, `map/`, `rules/`, or tests; no presentation-layer dependencies are introduced.
- Interface ordering: New capability/types precede callsite threading and integration.
- Risk coverage: Crusher, `cell_entry`, Drive fallback, tube final landing, and zone producer risks each have tests.
- Self-containment: Each task names files, required code shape, assertions, and verification command.
- Sim compliance: The plan requires integer/fixed-point math, hash coverage for new state, and deterministic sorted kill ordering.
- Grounding coverage: The plan cites research reports, current Rust, INI keys, and recent git history.
- Confidence tagging: Key technical decisions include confidence and sources; medium-confidence areas are called out for review.
- Deferred questions: Remaining oracle gaps are explicit and not used as acceptance claims.
- Parity-critical items: Populated with crusher, phase split, distance, DriveTrack residual, NavCom clear, tube, and ZoneGrid details.
