# Unit Can Enter Cell Bridge And Tunnel Parity Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained. Do not write implementation code until the user explicitly approves execution.

**Goal:** Bring Rust's vehicle cell-entry, bridge, low-bridge tube, and occupancy pathing behavior closer to verified Yuri's Revenge `UnitClass::Can_Enter_Cell` behavior without broad pathfinding rewrites.

**Architecture:** Immutable map facts stay in `map/`; deterministic mutable bridge and movement state stays in `sim/`. High bridge pathing already has a verified scaffold, so this plan adds the missing low-bridge TubeClass data path and cleans up stale cell-entry return-code semantics while preserving the existing `sim/` dependency boundary.

**Design Docs:**
- `docs/plans/2026-05-16-low-bridge-tubeclass-parity-design.md`
- `docs/plans/2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md`
- `docs/plans/2026-05-14-cell-occupancy-ordering-design.md`

---

## Grounding Summary

- The new verified report `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` confirms the live vehicle A* entry is `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, dispatched from A* through vtable `+0x1AC`.
- The bridge helper at vtable `+0x1B0` is `CheckBridgeTraversal @ 0x004D9C60`, not the A* entry. It returns only `0` or `7` and handles height/deck legality.
- Ghidra re-check this planning pass confirmed the A* call at `0x00429F54` is `CALL dword ptr [EDX + 0x1ac]`.
- Ghidra re-check confirmed `IsLowBridgeCell @ 0x00484AB0` requires valid `cell+0x116` tube index and `cell+0xEC == 10`.
- Ghidra re-check confirmed `GetTubeAtCell @ 0x00484F20` checks only the tube index and returns `g_TubeArray[index]` or null.
- Ghidra re-check confirmed `UnitClass::Can_Enter_Cell` direction `8` returns `0` only when a tube exists and `tube+0x28` is nonzero.
- Ghidra re-check confirmed `UnitClass::TubeMovement @ 0x007359F0` uses active tube byte `Unit+0x684`, cursor byte `Unit+0x685`, `tube+0x24/+0x28`, path steps at `tube+0x30`, and path length `tube+0x1C0`.
- The audit caveat is implementation-critical: direction-8 cell entry is not the whole visible tube contract. Same-cell automatic tube shells are valid cell facts, but full visible traversal must not consume incomplete full-tube data as if it came from explicit map tubes.
- Current Rust already contains high bridge layer and height gates in `src/sim/pathfinding/core.rs`, including `PathCell`, `check_bridge_traversal`, and `can_enter_layer_context`.
- Current Rust still names cell-entry code `3` as `BridgeRamp`, code `4` as friendly unit occupancy, and code `6` as `Cliff` in `src/sim/pathfinding/cell_entry.rs`; the verified binary semantics are allied scatter/building for code `3`, friendly wall/overlay soft block for code `4`, and stationary allied non-building for code `6`.
- Current Rust flattens low bridge overlays into road-like passability in `src/map/resolved_terrain.rs`, which conflicts with the binary low-bridge predicate.
- Current Rust `src/sim/movement/tunnel_movement.rs` implements subterranean burrow behavior, not low-bridge `TubeClass` movement.
- INI grounding: `rulesmd.ini` low bridge overlay sections `LOBRDG*`, `LOBRDGE*`, `LOBRDB*`, `LOBRDGB*` declare `Land=Road` and mostly `NoUseTileLandType=true`; this is not the same as binary `CellClass+0xEC == 10`.
- Full wall/overlay dynamic return-code parity depends on crusher flags, weapon/warhead wall flags, ownership, gates, and overlay type fields. This plan creates return-code seams but leaves that broad wall branch for a follow-up design.

## Key Technical Decisions

- **Rename Rust cell-entry results to verified semantics before behavior changes.** This prevents stale `BridgeRamp`, `OccupiedFriendly`, and `Cliff` meanings from leaking into new pathing logic. **Confidence:** high. **Source:** `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`.
- **Model low bridges with TubeClass-shaped immutable map facts.** Per-cell tube facts mirror binary `CellClass+0x116` lookup and keep static map data outside mutable sim state. **Confidence:** high. **Source:** `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`, low-bridge design doc.
- **Do not extend the existing compressed Rust `LandType` enum with binary value `10`.** Add an explicit binary/YR cell land-type field or predicate for `CellClass+0xEC`. **Confidence:** high. **Source:** low-bridge design doc, `passability.rs`.
- **Treat direction `8` as a tube sentinel, but split cell-entry legality from visible tube locomotion.** `Can_Enter_Cell` accepts direction `8` when the cell has a valid tube and nonzero exit, but movement must not consume zero-step automatic shell tubes; visible locomotion requires usable full tube data with `path_len > 0`. **Confidence:** high. **Source:** `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `MapCoord_Step_By_Direction @ 0x0042D490`, `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`.
- **Keep high bridge redirect behavior high-only.** Existing `BridgeRecordFilter::HighActiveOnly` reflects `FindBridgeRecord` skipping low records. **Confidence:** high. **Source:** `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`, `zone_build.rs`.
- **Preserve separate ground and bridge occupancy layers.** `CanEnterLayerContext` already has terrain/object-list/occupancy-bit layer separation, so new low-bridge work should use that structure rather than collapsing layers. **Confidence:** high. **Source:** `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`, `cell_entry.rs`.
- **Wall/overlay dynamic handling is not part of this first execution batch.** The verified report gives enough facts to plan it, but it needs a dedicated parser and combat-rule impact pass before implementation. **Confidence:** medium. **Source:** `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`.

## Open Questions

### Resolved During Planning

- **Is there a design doc with architecture context and impact analysis?** Yes. `2026-05-16-low-bridge-tubeclass-parity-design.md` covers low bridge TubeClass work, and the existing bridge/occupancy design docs cover the related high-bridge and list-order contracts.
- **Can the current Rust `LandType` hold binary `CellClass+0xEC == 10`?** No. It is a compressed passability-column model. A separate field or predicate is required.
- **Is `src/sim/movement/tunnel_movement.rs` the right place to extend?** No. It is subterranean burrow locomotion. Low bridge tube movement should be a separate movement module or state.

### Deferred To Implementation

- **Exact Rust producer point for active low-bridge tube state.** The binary writer is known from reports, but the Rust movement handoff point should be confirmed in current code immediately before adding the state transition. The selected producer must reject zero-step automatic shell tubes for visible movement.
- **Exact low-record duplicate/order filter inputs in `ComputeBridgeZones`.** Existing docs verify the filter exists. The implementation task below starts with a narrow binary/code re-check before choosing Rust ordering.
- **Full wall/overlay dynamic return codes.** This requires a dedicated wall/overlay design because it touches rules parsing, weapon selection, ownership/alliance, gate state, and path/runtime return-code consumers.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/pathfinding/cell_entry.rs` | Rename and document `CellEntryResult` codes to verified YR semantics. |
| Modify | `src/sim/pathfinding/core.rs` | Carry tube metadata in `PathCell`, path walking, and A* low-bridge gates. |
| Create | `src/map/tube_facts.rs` | Immutable TubeClass-shaped map facts and tube id model. |
| Modify | `src/map/mod.rs` | Export `tube_facts`. |
| Modify | `src/map/resolved_terrain.rs` | Store binary low-bridge land/tube facts and stop flattening low bridges into road passability. |
| Modify | `src/sim/bridge_state/mod.rs` | Build low bridge records from tube-backed cells while preserving high bridge behavior. |
| Modify | `src/sim/pathfinding/zone_build.rs` | Consume low records only through all-active adjacency; preserve high-only redirect. |
| Modify | `src/sim/pathfinding/zone_map.rs` | Keep current filter wiring and update tests if signatures change. |
| Modify | `src/sim/pathfinding/zone_incremental.rs` | Keep current filter wiring and update tests if signatures change. |
| Create | `src/sim/movement/tube_movement.rs` | Low-bridge TubeMovement-equivalent state and stepping. |
| Modify | `src/sim/movement/mod.rs` | Export tube movement module. |
| Modify | `src/sim/movement/movement_step.rs` | Hand off direction-8 low-bridge movement to tube movement. |
| Modify | `src/sim/world/world_hash.rs` | Hash mutable tube movement/runtime bridge state only. |

## Interface Changes

- Add `map::tube_facts::{TubeId, TubeFact, TubeSource}`.
- Add `ResolvedTerrainCell` fields or accessors for `tube_index`, binary cell land type, and `is_low_bridge_tube_cell()`.
- Add `ResolvedTerrainGrid` access to the immutable tube fact registry.
- Add `PathCell` tube metadata sufficient for `GetTubeAtCell` and direction-8 path stepping.
- Rename `CellEntryResult::BridgeRamp`, `CellEntryResult::OccupiedFriendly`, and `CellEntryResult::Cliff` to verified code semantics. Update all exhaustive matches and tests in the same task so code 4 is no longer treated as friendly unit occupancy.
- Add sim movement state for active low-bridge tube id and tube cursor. This state must be serialized and hashed if stored on entities.

## Sim Checklist

- [ ] All sim math uses fixed-point or integer cell/lepton math; no `f32` or `f64` in sim logic.
- [ ] New mutable tube movement state is included in deterministic state hash.
- [ ] No new dependency from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net`.
- [ ] Tick ordering impact is limited to the existing movement step handoff point.
- [ ] Entity iteration remains deterministic through existing `BTreeMap<u64, GameEntity>`.

## Risk Areas

- Low bridge overlay road override removal can break current routes unless tube facts and zone/path logic land in the same batch.
- `PathCell` changes touch many tests with struct literals.
- Active tube movement state affects save/hash determinism.
- Low bridge damage/repair must gate connectivity without deleting immutable tube facts.
- Return-code renaming can expose stale assumptions in movement and A* cost tests.
- Full wall/overlay behavior is intentionally out of this patch set; do not mix it into low-bridge work.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Return-code semantic cleanup | Wrong labels cause wrong soft-block movement responses and future incorrect fixes. | Unit tests assert numeric code mapping and names match verified table. |
| 3 | Low bridge cells stop acting like plain road | Ground units should not cross low bridges through ordinary road logic. | Low-bridge fixture path fails without valid tube and succeeds with tube path. |
| 4 | One automatic tube per qualifying cell | YR creates same-cell shell tubes per low-bridge tube cell, not one tube per bridge span. | Map-resolution tests assert deterministic per-cell tube ids and direction table. |
| 7 | Direction-8 tube path step | Pathing needs the sentinel direction to enter tubes. | Path-walking test uses direction 8 and current cell tube index. |
| 8 | Tube movement state | Visible unit movement through low bridges must not be ordinary ground stepping. | Movement test asserts active tube id/cursor advances and clears at exit. |
| 9 | Damage gates low-bridge connectivity | Destroyed/repaired low bridges must change reachable zones. | Zone tests compare active, damaged, and repaired bridge connectivity. |
| 10 | Ground/bridge occupancy separation | Deck units and under-bridge units should not incorrectly block each other. | Existing bridge occupancy tests remain green; add low-bridge coverage where applicable. |

---

## Tasks

### Task 0: Re-check Current Producer And Dirty State

**Why:** The codebase is dirty and the active tube producer point affects movement wiring. This task prevents planning against stale assumptions.

**Files:**
- Read: `src/sim/movement/movement_step.rs`
- Read: `src/sim/movement/movement_occupancy.rs`
- Read: `src/sim/pathfinding/core.rs`
- Read: `src/sim/game_entity.rs`

**Pattern:** Current movement step uses small helper modules under `src/sim/movement/`.

**Step 1: Inspect git status**
Run:
```powershell
git status --short
```
Expected: note unrelated modified files and do not revert them.

**Step 2: Find movement state fields**
Run:
```powershell
rg -n "MovementState|path|next_index|on_bridge|TunnelState|serde|hash" src/sim src/map
```
Expected: identify the exact struct that should hold active low-bridge tube id and cursor.

**Step 3: Confirm binary producer before coding**
Use Ghidra to re-open the producer from `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`.
Expected: producer reads `cell+0x116`, writes active tube id equivalent to `Unit+0x684`, clears cursor equivalent to `+0x685`, and runs only for direction-8 tube entry.

**Step 4: Record implementation note**
Add a short note to the implementation session summary naming the Rust handoff point. Do not edit docs in this task.

**Step 5: Checkpoint**
Show the user the selected producer point before code changes if it differs from `movement_step.rs`.

### Task 1: Rename CellEntryResult To Verified Semantics

**Why:** Return-code names are currently stale and will mislead every later pathing change.

**Files:**
- Modify: `src/sim/pathfinding/cell_entry.rs`
- Modify: tests in the same file
- Search and modify call sites reported by `rg "BridgeRamp|OccupiedFriendly|Cliff|CellEntryResult::"`

**Pattern:** Keep the existing enum and two-phase check structure; this is a semantic rename and test update.

**Step 1: Replace code-3 variant**
Change `CellEntryResult::BridgeRamp` to a verified name such as `ScatterRequired { blocker_id: Option<u64> }` if current call sites need an id, or `ScatterRequired` if they do not.

**Step 2: Replace code-4 variant**
Change `CellEntryResult::OccupiedFriendly { blocker_id }` to `FriendlyWall`. Do not use code 4 for friendly unit occupancy; code 4 is reserved for the wall/overlay friendly-wall soft result until the dedicated wall/overlay follow-up implements its full payload needs.

**Step 3: Replace code-6 variant**
Change `CellEntryResult::Cliff` to `FriendlyStationary { blocker_id: u64 }` or `FriendlyStationary` matching current call-site data needs. Friendly stationary non-building blockers must map to code 6, not code 4.

**Step 4: Update blocker classification**
In `classify_blocker`, keep enemy blockers as code 5 and moving friendly blockers as code 2. Change stationary friendly blockers to return `FriendlyStationary`, not the code-4 wall result.

**Step 5: Update movement consumers**
In `movement_occupancy.rs`, move the current friendly-blocker scatter/wait handling from `OccupiedFriendly` to `FriendlyStationary`. Add a separate `FriendlyWall` arm that blocks or waits according to the current wall/overlay support level until the dedicated wall/overlay follow-up implements full dynamic wall behavior.

**Step 6: Add numeric code helper**
Add a method:
```rust
impl CellEntryResult {
    pub fn yr_code(&self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::Crushable { .. } => 1,
            Self::TemporaryBlock { .. } => 2,
            Self::ScatterRequired { .. } => 3,
            Self::FriendlyWall => 4,
            Self::OccupiedEnemy { .. } => 5,
            Self::FriendlyStationary { .. } => 6,
            Self::Impassable => 7,
        }
    }
}
```
Adjust the code-3 and code-6 payloads to match the names chosen in Steps 1 and 3.

**Step 7: Update tests**
Add tests asserting the eight numeric codes and update stale expectations for `BridgeRamp`, `OccupiedFriendly`, and `Cliff`.

**Step 8: Verify**
Run:
```powershell
cargo test cell_entry -- --nocapture
```
Expected: tests pass. If unrelated compile failures appear from other dirty files, report them and stop before patching unrelated modules.

### Task 2: Add Tube Fact Types

**Why:** Low bridges need a map-owned TubeClass-shaped representation before pathing can use direction 8.

**Files:**
- Create: `src/map/tube_facts.rs`
- Modify: `src/map/mod.rs`

**Pattern:** Follow small map fact modules such as `bridge_facts`; keep immutable map facts in `map/`.

**Step 1: Define ids and source**
Add:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TubeId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TubeSource {
    AutoLowBridge,
    ExplicitMap,
}
```

**Step 2: Define TubeFact**
Add a struct with entry cell, exit cell, direction, path steps, path length, and source. Use `Vec<u8>` only for immutable map data, not per-tick hot-path allocation.

**Step 3: Add constructor for automatic low-bridge shell**
Add `TubeFact::auto_low_bridge(cell, direction)` that sets entry and exit to the same cell, path length to zero, and source to `AutoLowBridge`.

**Step 4: Export the module**
Add `pub mod tube_facts;` in `src/map/mod.rs`.

**Step 5: Add tests**
Test deterministic `TubeId`, same-cell auto shell fields, and direction storage.

**Step 6: Verify**
Run:
```powershell
cargo test tube_facts -- --nocapture
```
Expected: tests pass.

### Task 3: Add Low-Bridge Tube Fields To Resolved Terrain

**Why:** `IsLowBridgeCell` is a per-cell predicate over binary land type and tube index.

**Files:**
- Modify: `src/map/resolved_terrain.rs`
- Modify: any `ResolvedTerrainCell` test fixtures

**Pattern:** Add static per-cell facts next to existing `bridge_layer`, `bridge_facts`, `land_type`, and `slope_type`.

**Step 1: Add fields**
Add to `ResolvedTerrainCell`:
```rust
pub binary_cell_land_type: u8,
pub tube_index: Option<crate::map::tube_facts::TubeId>,
```

**Step 2: Add predicate**
Add:
```rust
pub fn is_low_bridge_tube_cell(&self) -> bool {
    self.binary_cell_land_type == 10 && self.tube_index.is_some()
}
```

**Step 3: Add grid storage**
Add a `tube_facts: Vec<TubeFact>` field to `ResolvedTerrainGrid` and a read accessor.

**Step 4: Update constructors**
Update `ResolvedTerrainGrid::from_cells` to accept an empty tube registry by default or add a second constructor with tube facts. Keep test fixture churn minimal.

**Step 5: Verify**
Run:
```powershell
cargo test resolved_terrain -- --nocapture
```
Expected: tests pass after fixture updates.

### Task 4: Generate Automatic Low-Bridge Tube Facts

**Why:** The binary creates automatic same-cell tubes from qualifying low/wood bridge cells during `RecalcAttributes`.

**Files:**
- Modify: `src/map/resolved_terrain.rs`

**Pattern:** Build deterministic map facts during the existing resolved terrain cell loop.

**Step 1: Add direction table**
Add a local named constant in `resolved_terrain.rs` or `tube_facts.rs`:
```rust
const AUTO_LOW_BRIDGE_DIRECTIONS: [u8; 4] = [2, 4, 6, 0];
```

**Step 2: Identify qualifying cell**
Use the verified low bridge overlay/tile classification already exposed by `overlay_effects.is_low_bridge` and final sub-tile/tile range logic from the low-bridge design. Do not infer from `Land=Road` alone.

**Step 3: Create tube fact**
For every qualifying cell in deterministic `(ry, rx)` order, push `TubeFact::auto_low_bridge((rx, ry), direction)` and store `TubeId` on the cell.

**Step 4: Set binary low-bridge land type**
For qualifying low-bridge tube cells, set `binary_cell_land_type = 10`. For all other cells, set the binary field from the best existing final terrain land-type mapping, keeping it separate from compressed passability `land_type`.

**Step 5: Add tests**
Add tests that a low bridge cell receives one tube id, the tube has same entry/exit, direction comes from `[2, 4, 6, 0]`, and repeated map resolution yields identical ids.

**Step 6: Verify**
Run:
```powershell
cargo test low_bridge -- --nocapture
```
Expected: low-bridge map tests pass.

### Task 5: Remove Ordinary Road Passability Override For Low Bridges

**Why:** Current Rust turns low bridges into road-like ground passability, but YR low-bridge legality is tube-index plus binary land type.

**Files:**
- Modify: `src/map/resolved_terrain.rs`
- Modify: tests that expected low bridge overlays to force `Road`

**Pattern:** Preserve visual bridge facts while changing gameplay passability facts.

**Step 1: Delete the gameplay road override**
Remove the block that forces `metadata.is_water = false`, `metadata.is_road = true`, `ground_blocked = false`, and `land_type = Road` for `overlay_effects.is_low_bridge`.

**Step 2: Preserve visual facts**
Keep `bridge_layer: Some(... BridgeDirection::Low ...)`, low-bridge overlay identity, radar/render facts, and damage variant facts unchanged.

**Step 3: Keep zone behavior tube-driven**
Do not set low bridge `zone_type` to `GROUND` solely from overlay identity. The zone task will add tube-backed adjacency.

**Step 4: Add regression tests**
Add a test where a water-under-low-bridge cell is not ordinary ground-walkable unless its tube path is used.

**Step 5: Verify**
Run:
```powershell
cargo test resolved_terrain low_bridge -- --nocapture
```
Expected: low bridge no longer appears as plain road in gameplay facts, while visual bridge facts remain present.

### Task 6: Carry Tube Metadata Into PathCell

**Why:** A* and path walking need a cheap equivalent of `GetTubeAtCell`.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify: `src/sim/pathfinding/core_tests.rs`
- Modify: path-grid fixtures in `src/sim/pathfinding/zone_map_tests.rs` and bridge tests as needed

**Pattern:** Follow existing `PathCell` hot-cache fields such as `bridge_walkable`, `transition`, and `slope_type`.

**Step 1: Add PathCell fields**
Add:
```rust
pub tube_index: Option<crate::map::tube_facts::TubeId>,
pub low_bridge_tube_cell: bool,
```

**Step 2: Populate from resolved terrain**
In `PathGrid::from_resolved_terrain_with_bridges`, copy `cell.tube_index` and `cell.is_low_bridge_tube_cell()`.

**Step 3: Update defaults and diff**
Set defaults to `None` and `false`, and include both fields in `diff_cells`.

**Step 4: Add accessors**
Add `tube_index_at(x, y)` and `is_low_bridge_tube_cell(x, y)` accessors on `PathGrid`.

**Step 5: Verify**
Run:
```powershell
cargo test pathfinding::core -- --nocapture
```
Expected: core pathfinding tests pass after fixture updates.

### Task 7: Add Direction-8 Tube Path Walking

**Why:** Direction `8` is the low-bridge/tube sentinel in verified path stepping and `Can_Enter_Cell`.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify: path-walking tests in the same module or closest existing test module

**Pattern:** Keep A* neighbor enumeration for normal 8 directions unchanged; add explicit handling for stored path directions containing `8`.

**Step 1: Find path-walking consumer**
Use `rg -n "direction|next_index|path_layers|LayeredPathStep|find_path" src/sim/pathfinding src/sim/movement`.

**Step 2: Add tube-step helper**
Add a helper that takes current cell and direction. If direction is `8`, it reads current cell tube id and returns the tube exit coordinate. If there is no tube, return an invalid step result that the caller treats as blocked.

**Step 3: Respect audit caveat**
For automatic same-cell shell tubes, direction-8 path walking may inspect the tube for cell-entry predicates and zone/click logic, but it must not mark the step as a valid visible locomotion transition. The movement producer in Task 9 must reject any tube whose `path_len == 0`.

**Step 4: Add tests**
Test direction `8` with a valid tube, missing tube, and same-cell auto shell.

**Step 5: Verify**
Run:
```powershell
cargo test direction_8 tube -- --nocapture
```
Expected: direction-8 tests pass and normal 0..7 path tests remain green.

### Task 8: Add Low-Bridge Tube Movement State

**Why:** Low-bridge units must not remain ordinary ground movers once pathing selects tube entry.

**Files:**
- Create: `src/sim/movement/tube_movement.rs`
- Modify: `src/sim/movement/mod.rs`
- Modify: entity movement state file identified in Task 0
- Modify: `src/sim/world/world_hash.rs`

**Pattern:** Follow `tunnel_movement.rs` for module shape and serialization style, but do not reuse subterranean burrow behavior.

**Step 1: Define state**
Add a serializable state with active `TubeId`, cursor, entry cell, exit cell, and phase needed by movement stepping.

**Step 2: Define begin function**
Add `begin_low_bridge_tube_movement(entity, tube_id, tube_fact)` that initializes active tube state and cursor to zero only after validating `tube_fact.path_len > 0`. Return a blocked/no-start result for automatic same-cell shells and any other zero-step tube.

**Step 3: Define step function**
Add a deterministic step that consumes tube steps and clears active state when it reaches the exit. This function should never receive automatic same-cell shells; add a debug assertion or early error for `path_len == 0` so a future producer cannot silently turn an invalid tube into a completed movement.

**Step 4: Hash mutable state**
Update `world_hash.rs` to hash active tube id, cursor, and phase for entities that carry this state. Immutable `TubeFact`s do not need mutable-state hashing.

**Step 5: Add tests**
Test begin rejects zero-length auto shells, multi-step explicit-style tube completion, and hash difference while active.

**Step 6: Verify**
Run:
```powershell
cargo test tube_movement world_hash -- --nocapture
```
Expected: movement-state and hash tests pass.

### Task 9: Wire Movement Direction-8 Producer

**Why:** Pathing support is not player-visible until the movement tick writes and consumes active tube state.

**Files:**
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_occupancy.rs` if the occupancy check currently blocks same-cell tube entry
- Modify: tests near movement stepping

**Pattern:** Use the producer point confirmed in Task 0 and keep movement helper code in `tube_movement.rs`.

**Step 1: Detect direction-8 step**
At the movement handoff point, when the next path direction is `8`, read the current cell tube id from `PathGrid`.

**Step 2: Validate tube**
If no tube exists, treat the movement step as blocked with `CellEntryResult::Impassable`. If the tube exists but `path_len == 0`, do not start tube locomotion; treat it as a non-traversable direction-8 movement transition and keep the unit out of ordinary ground stepping for that path entry.

**Step 3: Begin tube movement**
If a tube exists and has usable full traversal data, call `begin_low_bridge_tube_movement` and skip ordinary ground occupancy reservation for the same direction-8 step.

**Step 4: Consume active movement**
At the beginning of the movement tick, if the entity has active low-bridge tube state, call the tube movement step before ordinary cell-to-cell stepping.

**Step 5: Add tests**
Add movement tests for valid full-tube direction-8 entry, missing-tube block, and same-cell shell rejection without becoming an extra road step.

**Step 6: Verify**
Run:
```powershell
cargo test movement tube -- --nocapture
```
Expected: movement tests pass and subterranean `TunnelState` tests remain unchanged.

### Task 10: Build Tube-Backed Low Bridge Records

**Why:** Zone connectivity must use low bridge records created from tube-backed cells, not generic bridge overlay groups.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs`
- Modify: bridge-state tests in the same file

**Pattern:** Keep existing `BridgeEndpointRecord` and `BridgeRecordKind::Low`; change the low-record source.

**Step 1: Add tube-backed builder**
Add a helper that iterates resolved terrain cells in deterministic map order and considers only `cell.is_low_bridge_tube_cell()`.

**Step 2: Require opposite neighbor pair**
Require the verified low-neighbor pair pattern from the low-bridge design: direction `2` with `6`, or direction `4` with `0`.

**Step 3: Use tube exit**
Use the current cell coordinate and the tube fact exit coordinate for candidate endpoints.

**Step 4: Preserve high records**
Do not change high bridge record construction except where shared signatures need a tube registry argument.

**Step 5: Add tests**
Test lone tube cell creates no record, valid opposite-pair creates a `BridgeRecordKind::Low`, high bridge records remain `High`, and mixed maps preserve both kinds.

**Step 6: Verify**
Run:
```powershell
cargo test bridge_state low_bridge -- --nocapture
```
Expected: bridge state low/high tests pass.

### Task 11: Wire Zone Adjacency Filters

**Why:** The binary uses low bridge records for all-active zone adjacency while high-only redirect ignores low records.

**Files:**
- Modify: `src/sim/pathfinding/zone_build.rs`
- Modify: `src/sim/pathfinding/zone_map.rs`
- Modify: `src/sim/pathfinding/zone_incremental.rs`

**Pattern:** Preserve `BridgeRecordFilter::AllActive` and `BridgeRecordFilter::HighActiveOnly`.

**Step 1: Keep filter semantics**
Confirm `AllActive` accepts low and high records, and `HighActiveOnly` accepts only high records.

**Step 2: Feed tube-backed low records**
Ensure zone builders receive the updated bridge endpoint records from Task 10.

**Step 3: Add tests**
Add tests that all-active adjacency includes low records and high-only redirect excludes low records.

**Step 4: Verify**
Run:
```powershell
cargo test zone_build zone_map zone_incremental -- --nocapture
```
Expected: low bridge adjacency tests pass and high bridge redirect tests remain green.

### Task 12: Gate Low-Bridge Connectivity On Damage And Repair

**Why:** Player-visible low bridge destruction and repair must change pathing without deleting static tube ids.

**Files:**
- Modify: `src/sim/bridge_state/mod.rs`
- Modify: `src/sim/pathfinding/core.rs` if rebuild logic needs active low-bridge gating
- Modify: `src/sim/world/world_hash.rs`

**Pattern:** Existing high bridge state already gates `is_bridge_walkable` during path-grid rebuilds.

**Step 1: Add active check**
Add a low-bridge active/connectivity predicate that uses mutable bridge runtime state, not immutable `TubeFact` deletion.

**Step 2: Apply during zone/path rebuild**
When low bridge runtime state is destroyed or inactive, zone adjacency must omit the low record. Static tube ids remain on resolved terrain.

**Step 3: Repair restores connectivity**
When repair marks the low bridge active again, zone adjacency can include the low record again.

**Step 4: Hash mutable state**
Hash the low-bridge active/connectivity state if not already covered by bridge runtime state hashing.

**Step 5: Add tests**
Test active, destroyed, and repaired low bridge connectivity while asserting tube ids remain stable.

**Step 6: Verify**
Run:
```powershell
cargo test bridge_state zone low_bridge world_hash -- --nocapture
```
Expected: connectivity toggles with damage/repair and tube ids remain stable.

### Task 13: Regression Sweep For High Bridge And Occupancy Layers

**Why:** Low bridge changes share path-grid, bridge-state, and occupancy-layer code with high bridges.

**Files:**
- Read/modify only failing tests in touched modules

**Pattern:** Preserve existing `CanEnterLayerContext` separation and high bridge layer tests.

**Step 1: Run focused tests**
Run:
```powershell
cargo test bridge pathfinding movement_occupancy -- --nocapture
```

**Step 2: Check specific behaviors**
Confirm high bridge deck entry, bridgehead/ramp transition, under-bridge occupancy, and destroyed high-bridge deck tests still pass.

**Step 3: Add missing regression**
If there is no test for deck occupant not blocking ground occupant, add one in the closest occupancy/pathfinding test module using `CanEnterLayerContext`.

**Step 4: Verify**
Run the focused tests again.

**Step 5: Checkpoint**
Show the diff and list which parity-critical cases are now covered.

### Task 14: Document Wall/Overlay Follow-Up Scope

**Why:** The verified report includes dynamic wall/overlay return-code behavior, but implementing it here would mix combat rule parsing into the TubeClass patch set.

**Files:**
- Create: `docs/plans/2026-05-16-unit-can-enter-cell-wall-overlay-followup-scope.md`

**Pattern:** Short scope note, not implementation code.

**Step 1: List binary dependencies**
Include overlay fields `OverlayType+0x2AA`, `+0x2A8`, `+0x22D`, `+0x9C`, unit flags `Crusher`, movement zone, weapon, warhead `Wall` and `Wood`, ownership/alliance, and gates.

**Step 2: List Rust dependencies**
Name the rules parser, weapon/warhead model, overlay registry, `cell_entry.rs`, and movement response consumers.

**Step 3: Define acceptance**
State that wall/overlay parity starts only after a dedicated design/review-plan pass.

**Step 4: Verify**
Run a placeholder-word scan on `docs/plans/2026-05-16-unit-can-enter-cell-wall-overlay-followup-scope.md`.
Expected: no vague placeholder wording appears.

## Sources & References

- **Primary verified report:** `docs/research/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- **Audit log:** `docs/research/AUDIT_LOG.md`
- **Low bridge design:** `docs/plans/2026-05-16-low-bridge-tubeclass-parity-design.md`
- **Bridge height design:** `docs/plans/2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md`
- **Occupancy design:** `docs/plans/2026-05-14-cell-occupancy-ordering-design.md`
- **Sibling reports:** `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`, `PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`, `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`, `BRIDGE_SYSTEM.md`, `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`, `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`, `BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`, `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- **Ghidra addresses:** `0x0073F0A0 UnitClass::Can_Enter_Cell`, `0x00429F54 A* vtable +0x1AC call`, `0x004D9C60 CheckBridgeTraversal`, `0x00484AB0 CellClass::IsLowBridgeCell`, `0x00484F20 CellClass::GetTubeAtCell`, `0x007359F0 UnitClass::TubeMovement`, `0x0047D2B0 CellClass::RecalcAttributes`, `0x00429830 AStar_compute_edge_cost`
- **INI keys:** `rulesmd.ini` low bridge overlay sections `LOBRDG*`, `LOBRDGE*`, `LOBRDB*`, `LOBRDGB*`; `Land=Road`; `NoUseTileLandType=true/false`; wall and gate keys `Wall=yes`, `Gate=yes`, `Crusher=yes`, warhead `Wall`, `Wood`, `WallAbsoluteDestroyer`
- **Current Rust:** `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/core.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_incremental.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/movement/tunnel_movement.rs`, `src/sim/world/world_hash.rs`

## Post-Plan Self-Review

- Spec coverage: the plan covers return-code cleanup, low-bridge tube facts, direction-8 pathing, tube movement, low bridge records, damage/repair gating, occupancy regression, and the wall/overlay boundary.
- Placeholder scan: run a local search for vague placeholder terms before review.
- Architecture check: map facts stay in `map/`; mutable movement and bridge state stay in `sim/`.
- Interface ordering: enum/tube fact interfaces are created before pathing and movement consume them.
- Risk coverage: high bridge, zone, movement, and world hash tests are included.
- Sim compliance: no float math or render/UI/audio/net dependencies are introduced by the planned sim tasks.
- Grounding coverage: the plan cites reports, Ghidra addresses, repo files, and INI keys.
- Deferred questions: active tube producer, low-record duplicate filter inputs, and wall/overlay dynamic branch are isolated before broad implementation.
