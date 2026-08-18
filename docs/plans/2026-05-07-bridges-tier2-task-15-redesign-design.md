# Bridges Tier 2 — Task 15 Redesign (Bridgehead-Cell State-Machine Driver)

## Goal

Replace the stale Task 15 plan section with a verified-against-binary design for the bridgehead-cell branch of `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (HIGH §3.2). Mirrors the body-driver redesign that just shipped (commits 5478e17 → 9711833 → 20b8fdc); ships state-machine + 3-cell BlowUp cascade + adjacency/zones flags now; defers the visible overlay-byte progression and 10-slot debris loop to Task 15.5 alongside the body driver's deferred Task 13.5.

## Architecture Context

`BridgeRuntimeState` ([src/sim/bridge_state.rs](../../src/sim/bridge_state.rs)) owns mutable bridge runtime state layered on top of `ResolvedTerrainGrid`. Bridgehead cells are classified at map-load Pass 3 ([bridge_state.rs:449-473](../../src/sim/bridge_state.rs#L449-L473)) when their `bridge_layer.overlay_id` is non-anchor — they get `role = BridgeCellRole::Bridgehead`, `axis = bridge_layer.direction`, no `anchor_span_id`.

The bridgehead branch of `0x576BA0` (verified live this session):

1. Filter: cell's `flags & 0x100 == 0` AND `(IsoTileType - BridgeSet) + 1 ∈ NS-bridgehead-window ∪ EW-bridgehead-window`.
2. Walk to anchor via height predicate (NS: reject `h & 1`, walk until `h == 4`; EW: reject `h > 4`, walk until `h == 2`). On odd-height/out-of-window: return 0.
3. Switch on `iVar2 = (cell.IsoTileType - BridgeSet) + 1`:
   - `iVar2 ∈ {ABAD30, ABAD30+1, ABAD30+2}` (NS; symmetric AA1028 for EW): progressive damage. Write `SetOverlayAndPropagate(anchor, ABAD30+2+BridgeSet)`. `UpdateRamp_*_DamageA(anchor, perpA_dir)`. `UpdateRamp_*_DamageB(anchor, perpB_dir)`. Return 0.
   - `iVar2 == ABAD30+3` (NS; symmetric AA1028+3 for EW): final collapse. Compute 3-cell BlowUp row (height-bit predicate). `BlowUpBridge` × 3. `SetOverlayAndPropagate(anchor, ABAD30+3+BridgeSet, level-4)`. `UpdateRamp_*_CollapseA(anchor, perpA_dir)`. `UpdateRamp_*_CollapseB(anchor, perpB_dir)`. `UpdateAdjacentBridges_High` × 2 (perpendiculars of bridgehead cell, NOT anchor). `InvalidateBridgeZones` → maybe `UpdateBridgeZonesHelper`. 10-slot debris loop. Return 1.

**Critical structural difference vs body branch:** the bridgehead branch does **NOT** call `SetBridgeDirection_NESW`. Body branch's collapse path explicitly clears anchor-span flag bits and writes `anchor.+0x11E ← 0`; bridgehead branch leaves those untouched. The body span survives the bridgehead's destruction with its `+0x11E` state byte advanced one tier (Healthy → PartialCollapseA/B) by the perpendicular `UpdateRamp_*_Collapse` call — subsequent damage on the body continues the collapse via the body driver. This is a multi-stage destruction mechanic.

**Phase B/C primitives already on `dev` and verified live, all reusable:**

- `BridgeCellRole::Bridgehead` ([bridge_state.rs:110](../../src/sim/bridge_state.rs#L110)) — branch dispatch gate.
- `bridgehead_walk_to_anchor` ([bridge_specs.rs:608](../../src/sim/bridge_specs.rs#L608), commit 2474058) — anchor lookup.
- `update_ramp_perpendicular` ([bridge_specs.rs:538](../../src/sim/bridge_specs.rs#L538), commit 9711833) — anchor-perp `UpdateRamp_*` state-byte branch.
- `apply_ramp_transition` ([bridge_specs.rs:351](../../src/sim/bridge_specs.rs#L351), commit c9395be) — composed via `update_ramp_perpendicular`.
- `compute_adjacent_bridges_dirty` ([bridge_state.rs:950](../../src/sim/bridge_state.rs#L950)) — perpendicular pair, axis-driven.
- `StateOutcome::{Absorbed, Collapsed{...}, NoChange}` ([bridge_state.rs:238-265](../../src/sim/bridge_state.rs#L238-L265)) — return shape.
- `SetBridgeDirectionResult.actions: Vec<(cell, dir, CellAction)>` ([bridge_specs.rs:468](../../src/sim/bridge_specs.rs#L468)) — orchestrator-facing cascade list. Bridgehead emits a `SetBridgeDirectionResult` carrying just the 3 BlowUpBridge entries (no anchor-span cascade, no `set_bridge_direction` call).

## Impact Analysis

**Files touched:**

| File | Change |
|---|---|
| [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) | **Commit 1**: drop `bridgehead_step: u8` field from `BridgeRuntimeCell` (currently dead — never written, always 0 at map-load). Update 5 init sites + 3 test fixtures + world_hash ([bridge_state.rs:388,470,1241,1262,1366](../../src/sim/bridge_state.rs#L388)). **Commit 3**: new method `BridgeRuntimeState::bridgehead_advance_state(rx, ry, is_high_bridge, terrain) -> StateOutcome` (~120 LOC + tests). |
| [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) | **Commit 2**: new pure helper `bridgehead_blow_up_row(anchor_pos, axis, anchor_height, map_w, map_h) -> [Option<(u16, u16)>; 3]`. ~30 LOC + tests. |
| [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | **Commit 1**: drop `bridgehead_step` from cell hash. Determinism preserved (field was always 0). |

**Predecessor / co-shipping:** none. All Phase B/C dependencies are on `dev`.

**Risk areas:**

1. **Bridgehead's anchor's `damage_state` is NOT directly modified by the driver.** The binary writes `anchor.+0x44` (overlay byte, deferred) and `anchor.+0x11E` is left untouched in the bridgehead branch. Only the perpendicular partner's state byte advances via `update_ramp_perpendicular`. Result: anchor's `damage_state` stays whatever it was (`Healthy{0}` for an undamaged bridge). `is_bridge_walkable(anchor)` returns true. **This is parity-correct** — body cells of a bridge whose ramp was destroyed are still passable in gamemd until their own collapse damage lands. Documented in driver comments + tests.
2. **The 3-cell BlowUp row geometry is body-axis-aligned, NOT perpendicular.** §3.2 / §11.1 wording was wrong; the binary blows up 3 cells in a column (NS) or row (EW) along the body axis, offset by anchor's height bit. Lives in `bridgehead_blow_up_row` helper with explicit per-axis case table.
3. **`adjacent_bridges_dirty` is computed from the BRIDGEHEAD's coord, not the anchor's.** Mirror of body driver's convention: original damaged cell's perpendicular pair.
4. **Determinism:** purely state-machine. No RNG. Outer RNG (BridgeStrength + IonCannon retry) lives at orchestrator boundary (Phase F). Method invocations are deterministic per damage event.
5. **`bridgehead_step` field removal:** currently dead state; refactor is mechanical. Snapshot round-trip tests will catch any oversight in serialization.

## Chosen Approach

**Approach A — three small commits, mirrors body-driver shipping cadence (5478e17 → 9711833 → 20b8fdc).**

- **Commit 1** — `bridge_state`: drop `bridgehead_step` field; migrate to `damage_state` for all roles.
- **Commit 2** — `bridge_specs`: `bridgehead_blow_up_row` pure helper.
- **Commit 3** — `bridge_state`: `bridgehead_advance_state` driver method composing existing helpers.

Each commit independently `cargo test` green; each reverts cleanly if a regression surfaces.

**Settled by brainstorm Q1–Q2:**

- **Q1 (scope):** Approach (a) — full state-machine + 3-cell BlowUp cascade + adjacency/zones flags, overlay-byte deferred. Same parity scope as body driver's commit 20b8fdc.
- **Q2 (state representation):** Approach (b) — drop `bridgehead_step` field, reuse `damage_state: DamageState` for body and bridgehead. Bridgehead's `Healthy{variant: 0..=2}` mirrors initial overlay slot (variant fidelity recovered in Task 15.5 once `ABAD30 / AA1028` constants are observed live); `Damaged` mirrors step 3 (ready-to-collapse); `Destroyed` mirrors post-collapse; `PartialCollapseA/B` are body-only. Unified queryable state — `is_bridge_walkable` gets the right answer for free.

## Tiny-Detail Ledger

Each item must be preserved in implementation; cited source.

| # | Detail | Source | Implementation home |
|---|---|---|---|
| 1 | Entry filter: `cell.role == BridgeCellRole::Bridgehead`. Mirror of binary's `flags & 0x100 == 0 + IsoTileType in bridgehead-window`. | `[GHIDRA 0x576BA0]` + map-load Pass 3 | Driver method gate |
| 2 | RESOLVED — drop `bridgehead_step`, use `damage_state` for both body and bridgehead | `[GHIDRA 0x576BA0]` verified | Commit 1 migration |
| 3 | Anchor walk via height predicate: NS rejects `h & 1`, walks until `h == 4`; EW rejects `h > 4`, walks until `h == 2` | `[GHIDRA 0x576BA0]` + Task 14 (commit 2474058) | `bridgehead_walk_to_anchor` (already shipped) |
| 4 | Step 3 (iVar2 == ABAD30+3 NS / AA1028+3 EW) → final collapse | `[GHIDRA 0x576BA0]` | Driver `Damaged` arm |
| 5 | Steps 0/1/2 (any healthy variant) → progressive damage one-shot to step 3 | `[GHIDRA 0x576BA0]` verified live | Driver `Healthy{..}` arm |
| 6 | UpdateRamp dirs: NS dir 2 (E) for A, dir 6 (W) for B; EW dir 4 (S) for A, dir 0 (N) for B (same as body) | `[GHIDRA 0x576BA0]` + already-shipped `perpendicular_direction` | `update_ramp_perpendicular` (already shipped) |
| 7 | RESOLVED — 2-hit destroy from healthy: hit 1 → Damaged; hit 2 → Destroyed. NOT a 4-step progression. iVar2 formula is `(IsoTileType - BridgeSet) + 1`, so writing slot 2 raw → next read = step 3 = collapse trigger. | `[GHIDRA 0x576BA0]` verified live | Driver `Healthy{..} → Damaged` (single arm) |
| 8 | Collapse path: anchor `UpdateRamp_*_CollapseA(anchor, perpA)` + `UpdateRamp_*_CollapseB(anchor, perpB)`. Anchor's own `+0x11E` is NOT directly written — only the perpendicular target's state byte advances. | `[GHIDRA 0x576BA0]` verified live | Driver `Damaged` arm: 2× `update_ramp_perpendicular` |
| 9 | RESOLVED — 3-cell BlowUp row is BODY-AXIS-ALIGNED (NOT perpendicular). NS: column at `(anchor.X, ±1, 0)` if `anchor.h & 1 == 0`, else `(anchor.X-1, ±1, 0)`. EW: row at `(±1, 0, anchor.Y)` if `anchor.h < 5`, else `(±1, 0, anchor.Y-1)`. | `[GHIDRA 0x576BA0]` verified live | `bridgehead_blow_up_row` helper (commit 2) |
| 10 | Bridgehead branch does **NOT** call `SetBridgeDirection_NESW` — distinct from body branch. Anchor span flag bits are NOT cleared; body cells survive ramp destruction with state byte advanced one tier via `UpdateRamp_*_Collapse`. | `[GHIDRA 0x576BA0]` verified live + HIGH §3.2 (no SBD call listed) | Driver does NOT compose `set_bridge_direction(span, false)` |
| 11 | `UpdateAdjacentBridges_High × 2` perpendiculars of *bridgehead* cell (NOT anchor) | `[GHIDRA 0x576BA0]` (calls `MapCoord_Add(&local_20, &DAT_0089f6a0/8/...)` where `local_20` is bridgehead-relative) | `compute_adjacent_bridges_dirty(rx, ry, axis)` with bridgehead's coord |
| 12 | `InvalidateBridgeZones` → maybe `UpdateBridgeZonesHelper` (zones dirty flag) | `[GHIDRA 0x576BA0]` | `zones_dirty: true` in `StateOutcome::Collapsed` |
| 13 | **Deferred (Task 15.5).** Overlay-byte progression `SetOverlayAndPropagate(anchor, ABAD30+offset+BridgeSet, …, level-4, …)` for visible damage. Same blocker as body Task 13.5 — runtime-init globals not observed in static binary image. | HIGH §3.2 + §2 runtime-init globals | Out of scope |
| 14 | **Deferred (Task 15.5+).** 10-slot debris-anim spawn loop on collapse | `[GHIDRA 0x576BA0]` final loop | Out of scope — orchestrator can attach later |
| 15 | Return: `StateOutcome::Absorbed` (steps 0/1/2 → 3), `StateOutcome::Collapsed{..}` (step 3 → destroyed), `StateOutcome::NoChange` otherwise | HIGH §3.3 | Driver match arms |
| 16 | Anchor unreachable (off-map / odd-height intermediate / span-already-Destroyed) → `NoChange` | `[GHIDRA 0x576BA0]` early-return on `(h & 1)` predicate; defensive | Defensive guards in driver |
| 17 | Bridgehead axis from `cell.axis` (set at map-load Pass 3) | already shipped | Driver reads `cell.axis` |
| 18 | Bridgehead's own `damage_state ← Destroyed` after collapse cascade. The bridgehead cell IS dead even if anchor's body span survives. | model invariant for `is_bridge_walkable(bridgehead) → false` | Driver collapse arm |
| 19 | `cell_height` for height predicate is read from `ResolvedTerrainGrid`, not stored on `BridgeRuntimeCell`. Same closure pattern as `bridgehead_walk_to_anchor`. | already shipped | Driver builds closure from `&terrain` parameter |

## Design

### Components

**1. Schema migration (Commit 1):**

```rust
// src/sim/bridge_state.rs
pub struct BridgeRuntimeCell {
    // ... existing fields ...
    pub damage_state: DamageState,   // unified for body + bridgehead
    pub axis: Option<Axis>,
    pub role: BridgeCellRole,
    pub anchor_span_id: Option<u16>,
    // bridgehead_step: u8,            // REMOVED
    pub overlay_byte: u8,
}
```

Removal touches:

- `BridgeRuntimeCell` struct definition.
- `from_resolved_terrain` Pass 1 init (line 388) — drop field.
- `from_resolved_terrain` Pass 3 bridgehead classifier (line 470) — drop write.
- `world_hash` cell-state hashing — drop field from hash.
- 3 test fixtures (lines 1241, 1262, 1366) — drop field from struct literal.
- Snapshot round-trip test — schema delta verified by serde derive (no manual changes).

**2. `bridgehead_blow_up_row` helper (Commit 2):**

```rust
// src/sim/bridge_specs.rs

/// Three cells receiving `BlowUpBridge` on bridgehead final-step collapse.
/// Geometry verified live `[GHIDRA 0x576BA0]` step-3 branch.
///
/// Body-axis-aligned 3-cell row (NOT perpendicular). Offset to which row/column
/// is chosen depends on `anchor_height` parity bit:
///
/// | Axis | `anchor_height` predicate | Row geometry |
/// |------|---------------------------|--------------|
/// | NS   | `h & 1 == 0` (even)       | column at `anchor.X`,    Y = {anchor.Y - 1, anchor.Y, anchor.Y + 1} |
/// | NS   | `h & 1 != 0` (odd)        | column at `anchor.X - 1`, Y = {anchor.Y - 1, anchor.Y, anchor.Y + 1} |
/// | EW   | `h < 5`                   | row    at `anchor.Y`,    X = {anchor.X - 1, anchor.X, anchor.X + 1} |
/// | EW   | `h >= 5`                  | row    at `anchor.Y - 1`, X = {anchor.X - 1, anchor.X, anchor.X + 1} |
///
/// Off-map cells return `None` and are skipped by the caller.
pub fn bridgehead_blow_up_row(
    anchor_pos: (u16, u16),
    axis: Axis,
    anchor_height: u8,
    map_width: u16,
    map_height: u16,
) -> [Option<(u16, u16)>; 3]
```

Pure function. ~30 LOC. Tests cover all 4 (axis × predicate) cases plus map-edge clamping.

**3. `bridgehead_advance_state` driver method (Commit 3):**

```rust
// src/sim/bridge_state.rs

impl BridgeRuntimeState {
    /// Bridgehead state-machine driver. Mirrors the bridgehead branch of
    /// binary `ProcessBridgeDamageStateMachine_High @ 0x576BA0` (HIGH §3.2,
    /// verified live).
    ///
    /// Counterpart to `body_cell_advance_state`. Filters on `role == Bridgehead`,
    /// walks to anchor via height predicate, transitions per match arms, fires
    /// perpendicular `UpdateRamp_*` writes via `update_ramp_perpendicular`, and
    /// on collapse emits a 3-cell BlowUp row + `adjacent_bridges_dirty` +
    /// `zones_dirty` flags. Does NOT compose `set_bridge_direction(span, false)`
    /// — bridgehead branch in binary does not clear anchor-span flag bits.
    ///
    /// Returns:
    /// - `Absorbed` — bridgehead `Healthy → Damaged` (any of the 3 healthy
    ///   variants jump-transitions to the single Damaged tier in one hit).
    /// - `Collapsed { destroyed_cells, set_bridge_direction (3-entry BlowUp row),
    ///   adjacent_bridges_dirty (perpendiculars of *bridgehead* coord),
    ///   zones_dirty: true }` — bridgehead `Damaged → Destroyed`.
    /// - `NoChange` — non-bridgehead role, anchor walk failed, or already
    ///   `Destroyed`.
    pub fn bridgehead_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &ResolvedTerrainGrid,
    ) -> StateOutcome { ... }
}
```

### Interfaces / Contracts

- **Single entry point** for bridgehead damage. Caller (Phase F orchestrator's `apply_area_damage`) gates damage and dispatches by role; on bridgehead-cell hit it invokes this method once per damage event.
- **`StateOutcome::Collapsed.set_bridge_direction`** carries 3 entries (one per BlowUp cell, dir = 0, action = `BlowUpBridge`). The orchestrator iterates `result.actions` and dispatches each `BlowUpBridge` action — same iteration shape as body driver. The "SetBridgeDirection" name is slightly misleading for bridgehead (no actual `SetBridgeDirection_NESW` call) but the shape is reused for orchestrator simplicity. Driver docstring documents the misnomer.
- **`destroyed_cells`** lists bridgehead's coord plus any perpendicular UpdateRamp targets that hit collapse-final via `update_ramp_perpendicular`'s recurse path. Mirror of body driver convention.
- **`adjacent_bridges_dirty`** — perpendiculars of *bridgehead* cell coord (not anchor). Axis = bridgehead's `cell.axis`.
- **No errors.** `NoChange` covers all defensive cases.

### Data Flow

```
apply_area_damage (Phase F orchestrator)
  ├─ outer gate (BridgeStrength RNG, IonCannon retry — Phase F scope)
  ├─ classify cell by role
  ├─ if role == Bridgehead:
  │    bridgehead_advance_state(rx, ry, is_high, terrain)
  │      ├─ guard: cell.role == Bridgehead, cell.axis Some, damage_state != Destroyed
  │      ├─ anchor_pos = bridgehead_walk_to_anchor(start=(rx,ry), axis, dir, terrain.height_closure(), w, h)
  │      ├─ guard: anchor_pos != None; anchor cell exists; anchor span lookup not strictly needed
  │      │        (bridgehead branch does NOT touch anchor-span flag bits)
  │      ├─ match cell.damage_state:
  │      │   ├─ Healthy{..} → bridgehead.damage_state = Damaged;
  │      │   │                update_ramp_perpendicular(anchor_pos, axis, DamageA);
  │      │   │                update_ramp_perpendicular(anchor_pos, axis, DamageB);
  │      │   │                return Absorbed
  │      │   ├─ Damaged → anchor_height = terrain.cell(anchor_pos).height;
  │      │   │            blow_row = bridgehead_blow_up_row(anchor_pos, axis, anchor_height, w, h);
  │      │   │            update_ramp_perpendicular(anchor_pos, axis, CollapseA);
  │      │   │            update_ramp_perpendicular(anchor_pos, axis, CollapseB);
  │      │   │            bridgehead.damage_state = Destroyed;
  │      │   │            destroyed_cells = [(rx,ry)] + any perpendicular targets that hit Destroyed;
  │      │   │            sbd = SetBridgeDirectionResult { actions: blow_row.iter().filter_map(|c|
  │      │   │                       c.map(|cell| (cell, 0, CellAction::BlowUpBridge))).collect() };
  │      │   │            adj = compute_adjacent_bridges_dirty(rx, ry, axis);
  │      │   │            return Collapsed { destroyed_cells, set_bridge_direction: sbd,
  │      │   │                               adjacent_bridges_dirty: adj, zones_dirty: true }
  │      │   ├─ PartialCollapseA/B → NoChange (bridgehead-illegal; defensive)
  │      │   └─ Destroyed → NoChange
  └─ orchestrator dispatches actions:
       - kill ground occupants per BlowUpBridge cell (Phase F)
       - run UpdateAdjacentBridges_High on adjacent_bridges_dirty cells (Phase F Task 27)
       - rebuild zone graph if zones_dirty (Phase F Task 28)
       - spawn debris animations (Task 15.5+)
```

### Error Handling

No `Result` types. Defensive `NoChange` for: non-bridgehead role, anchor walk failure (off-map / odd-height intermediate), bridgehead already `Destroyed`, `cell.axis == None`. Phase B map-load establishes invariants — anything outside is a sim bug, not a runtime error. Assertions in `debug_assert!` for dev builds.

### Testing Strategy

**Commit 1 tests** (in [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) `mod tests`):

- Snapshot round-trip preserves all bridge state without `bridgehead_step` field.
- World hash differs only on real state changes (no spurious diff from removed field).
- Existing `body_cell_advance_state` tests still green (regression check).

**Commit 2 tests** (in [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) `mod tests`):

- All 4 cases (axis × height predicate) yield correct 3-cell row.
- NS even: column at `(X, X, X)` y in `(Y-1, Y, Y+1)`.
- NS odd: column at `(X-1, X-1, X-1)` y in `(Y-1, Y, Y+1)`.
- EW low (`h < 5`): row at x in `(X-1, X, X+1)` `(Y, Y, Y)`.
- EW high (`h >= 5`): row at x in `(X-1, X, X+1)` `(Y-1, Y-1, Y-1)`.
- Map-edge clamping: cells at edge return `None` for off-map slots.

**Commit 3 tests** (in [src/sim/bridge_state.rs](../../src/sim/bridge_state.rs) `mod tests`):

- NS bridgehead `Healthy{0} → Damaged`: `Absorbed`, anchor's perpendicular partner in `PartialCollapseA` or `PartialCollapseB` (depending on partner's prior state).
- NS bridgehead `Damaged → Destroyed`: `Collapsed { destroyed_cells: [(rx,ry), ...], set_bridge_direction.actions has 3 BlowUpBridge entries, adjacent_bridges_dirty has 2 perpendiculars of (rx,ry), zones_dirty: true }`. Bridgehead's own `damage_state == Destroyed`.
- EW bridgehead Healthy → Damaged → Destroyed: symmetric.
- Bridgehead with `damage_state == Destroyed`: `NoChange` (no double-cascade).
- Body cell input (wrong role): `NoChange`.
- Anchor walk fails (off-map / odd-height intermediate): `NoChange`.
- Anchor's `damage_state` is NOT modified by bridgehead collapse — assert anchor stays in its prior state (e.g., `Healthy{0}`).
- BlowUp row geometry varies by anchor_height bit — fixture-driven cases for both NS parity values and both EW height tiers.
- Determinism: same `(state, sequence of damage events)` → same final state hash across runs.

## Architectural Decisions

**Patterns followed:**

- Method on `BridgeRuntimeState` (matches `apply_damage`, `body_cell_advance_state`).
- Pure helpers in `bridge_specs.rs`, mutating method in `bridge_state.rs` (matches Phase B/C precedent).
- Compose existing helpers; no duplication.
- `StateOutcome` enum reused — single orchestrator-facing return shape for both branches.
- `SetBridgeDirectionResult` reused as the cascade-list carrier even though bridgehead doesn't actually call `SetBridgeDirection_NESW`. Documented misnomer; saves a parallel cascade type.

**Patterns deviated from:** none.

**Tech debt introduced:** none. Removing `bridgehead_step` cleans up an existing dead field.

**Determinism:** preserved. No RNG in Task 15 scope; switch-arm logic is total over `(DamageState × Axis × height-predicate)`; perpendicular lookup is deterministic from `anchor + Direction::offset()`; 3-cell row is deterministic from `(anchor_pos, axis, anchor_height)`.

**Layering:** `sim/` only. No `render/`, `ui/`, `audio/`, `net/` dependencies introduced.

## Alternatives Considered

- **Approach B (single bundled commit):** all three landings as one commit. Rejected — body driver shipped as 3 commits; precedent favors separating the field-removal migration from the new helper from the new driver for cleaner reviewability and easier revert.
- **Approach C (defer bridgehead_step migration; ship driver writing both fields):** keeps two-field synchronization, requires `is_bridge_walkable` to also check `bridgehead_step`, spreads coupling across consumers. Rejected.
- **Reuse `damage_state` for both body and bridgehead, drop `bridgehead_step`** (Q2 chosen): unified queryable state. Cleaner than two parallel state encodings; small mechanical refactor since `bridgehead_step` is currently dead state.
- **Compose `set_bridge_direction(anchor.span, false)` in bridgehead collapse path:** would mirror body driver's cascade, but binary doesn't do this — bridgehead branch leaves anchor-span flag bits untouched, lets the body span survive with state-byte advanced. Implementing it would over-collapse (player-visible drift: bridge cells fall when only the ramp should). Rejected.
- **Add a separate `BridgeheadOutcome` enum instead of reusing `StateOutcome`:** cleaner type-level separation but doubles the orchestrator's match shape for no functional gain. Rejected.
- **Pre-compute `anchor_height` at map-load and store on `BridgeRuntimeCell`:** would avoid the `&terrain` parameter on the driver. Rejected — closure-over-terrain is the established pattern (`bridgehead_walk_to_anchor`), and storing redundant terrain data on runtime cells creates a sync risk if terrain ever mutates.
- **Inline the bridgehead branch into `body_cell_advance_state` (single advance method dispatching by role):** mixes two different state machines into one method, breaks the "natural unit = binary's branch" precedent set by body redesign. Rejected.
