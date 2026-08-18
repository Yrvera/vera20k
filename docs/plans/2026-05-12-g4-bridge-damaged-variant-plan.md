# G4 Bridge Damaged Variant — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** When a bridge takes damage that doesn't immediately destroy it, swap the affected bridge tiles to the baked damaged variant (cell.Flags bit 0x2000 equivalent), and clear back to pristine on engineer-CABHUT repair.

**Architecture:** Sim writer (`apply_damaged_variant_flood_fill`) mirrors gamemd.exe `ToggleBridgePavement` 1:1 — recursive 8-neighbor flood-fill bounded by `tile_index` equality, idempotency early-return, gate checked only on kickoff. Render side reads `BridgeRuntimeCell.damaged_variant: bool` and selects the existing-but-currently-unused `TileKey { variant: 1 }` atlas entry; the asset pipeline already loads the damaged TMP file under the `'a'`-suffixed filename convention.

**Design Doc:** [docs/plans/2026-05-12-g4-bridge-damaged-variant-design.md](2026-05-12-g4-bridge-damaged-variant-design.md)

---

## Grounding Summary

**Docs (ra2-rust-game-docs/):**
- `LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md` §4 — `ToggleBridgePavement` @ 0x0056E990 algorithm (verified literal-match in /verify-doc this session); G4-critical claims all CONFIRMED.
- `TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md` (new, this session) — damaged variant pixel data is a SEPARATE `.TEM` file with `'a'` suffix; theater loader at [theater.rs:660-703](../../src/map/theater.rs#L660-L703) already loads it as `TileKey { variant: 1 }`.
- `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` §17 — render-time variant pick: `if HasDamagedVariantAtSubTile(tile, sub) then variant = (Flags >> 13) & 1 else PRNG`.

**Ghidra verification (binary addresses anchor the parity ledger; do NOT cite in Rust code comments per project memory):**
- `0x0056E990` MapClass::ToggleBridgePavement — flood-fill algorithm (signature, gate-on-kickoff, 8-neighbor tile_id-bounded propagation, clear-then-set bit, idempotency early-return) — fully decompiled and verified this session.
- `0x005471F0` IsometricTileTypeClass::HasDamagedVariantAtSubTile — TMP +0x24 bit 2 (`0x04`) gate; Rust already exposes via `TmpTile.has_damaged_data`.
- `0x00547CF0` TMP_TileBlitter — variant chain walk via `next_variant` (+0x2BC).
- `0x00545150` Read_Theater_TileSets_INI — builds variant chain at theater load.
- Damage AND collapse callers all pass `state=1`; only repair walkers (FUN_00569760, FUN_00568E40) pass `state=0` (verified `PUSH 0x1` / `PUSH 0x0` disassembly this session).

**Repo pattern this mirrors:**
- `body_cell_advance_state` ([src/sim/bridge_state/mod.rs:780](../../src/sim/bridge_state/mod.rs#L780)), `body_cell_repair_state` (line 978), `bridgehead_advance_state` (line 1111) — same shape, take `(rx, ry, …, &ResolvedTerrainGrid)` and mutate `BridgeRuntimeState`. The new `apply_damaged_variant_flood_fill` slots into this family.

**INI keys driving behavior:**
- None. G4 is entirely binary-flag + asset-file driven. No INI keys are read by the variant-pick path.

**What's still unknown after grounding:**
- Exact 8-direction order in `g_DirectionOffsets` (0x0089F688) — BSS, populated at runtime; can't read statically. Per ledger #8 the binary uses a fixed order; since `damaged_variant` flood-fill is idempotent on a bool, the final state is order-independent. The plan specifies N→NE→E→SE→S→SW→W→NW (standard RA2 8-facing convention) and notes the order is verifiable via `/fidelity-check` after implementation. NOT a parity blocker.

---

## Key Technical Decisions

- **Recursive immediate-execution flood-fill** (not BFS-deferred): mirrors binary; same-tick semantics preserved. — **Confidence:** high — **Source:** GHIDRA 0x0056E990, design doc Approach 1
- **Gate via `terrain.cell(rx, ry).has_damaged_data` checked only on kickoff**: matches binary's `suppress_self==0` behavior. — **Confidence:** high — **Source:** GHIDRA 0x0056E990
- **Add `has_damaged_data: bool` to `ResolvedTerrainCell`** (rather than build a separate Theater accessor): per-cell flag is the natural granularity (binary checks per-sub-tile via TMP cell-header bit), and the field is already parsed at TMP load (`TmpTile.has_damaged_data`) but not propagated past `TileMetadata` today. — **Confidence:** high — **Source:** repo state grep this session
- **Add `has_damaged_data: bool` to `TerrainCell`** (sync from ResolvedTerrainCell at TerrainGrid construction): render needs the flag too; mirror the existing pattern for `variant`, `radar_left`, `radar_right`. — **Confidence:** high — **Source:** terrain.rs structure grep
- **Override `cell.variant` at draw time** in [src/map/terrain.rs build_visible_instances](../../src/map/terrain.rs) when bridge cell has `damaged_variant=true`: no mutation of TerrainCell.variant — keep the visual-diversity field separate from the damage-variant override. — **Confidence:** high — **Source:** design doc
- **No direct flood-fill call at the seed/damage cell in the Healthy arm of `body_cell_advance_state`**: the binary has zero direct callers of `ToggleBridgePavement` at the damage cell across all 31 xrefs (verified by `/review-plan`). The seed cell only acquires the bit via flood-fill propagation from a perpendicular target's region. Adding a defensive seed-cell call would diverge in tile_id-boundary edge cases. — **Confidence:** high — **Source:** Ghidra xref dump on 0x0056E990 (verified during /review-plan); replaces an earlier draft that included the call.
- **Task 8 (bridgehead hook) is RE-gated**: the LAT_RETRIGGER §4.4 caller table is wrong about `ProcessBridgeDestruction_{Low,High}` passing state=1 (binary at 0x00570506 actually passes state=0). The correct binary analog of Rust's `bridgehead_advance_state` is most likely `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`, which has not been decompiled yet. Task 8 is structured as RE-first → decide hook target → implement (or no-op). — **Confidence:** medium (target identification) / pending (hook semantics) — **Source:** /review-plan disassembly of 0x00570506-0x00570511 confirming doc error; Task 8 Step 1 will resolve hook semantics via Ghidra MCP at execution time.
- **Direction order N→NE→E→SE→S→SW→W→NW**: standard RA2 facing convention; not verified against `g_DirectionOffsets` at 0x0089F688 because flood-fill is bool-idempotent and final state is order-independent. — **Confidence:** medium — **Source:** RA2 convention; flagged for /fidelity-check only if a determinism concern surfaces (state hash sensitivity).

## Open Questions

### Resolved During Planning

- **Where does the bridge body render site live?** → [src/map/terrain.rs build_visible_instances at line 568](../../src/map/terrain.rs#L568) — the `match &uv_fn { Some(f) => f(cell.tile_id, cell.sub_tile, cell.variant) ... }` line passes `cell.variant` to the atlas lookup closure. This is where the damaged-variant override slots in.
- **Does `bridgehead_advance_state` already take terrain?** → Yes ([mod.rs:1116](../../src/sim/bridge_state/mod.rs#L1116) `terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid`). No signature migration needed for it.
- **Does `body_cell_advance_state` take terrain?** → No. Migration required (Task 5).
- **Does `body_cell_repair_state` take terrain?** → No. Migration required (Task 5).
- **Does `update_ramp_perpendicular` take terrain?** → No. Migration required (Task 5).
- **Is `has_damaged_data` already on `TileMetadata`?** → No. `merge_tmp_metadata` reads `tile.has_damaged_data` but doesn't store it (verified grep this session). Plumbing required (Task 1).
- **Is the design premise still valid?** → Yes. Latest commits since the design timestamp touched docstring sweeps, bridge orchestrator cascade, and repair tests — none invalidate the design's "current state" claims.
- **Should the seed/damage cell get a direct flood-fill call in the Healthy arm?** → No. `/review-plan` verified the binary has zero direct callers of `ToggleBridgePavement` at the damage cell across all 31 xrefs. The seed cell acquires the bit only via flood-fill propagation from the perpendicular target's region (matches binary). Earlier draft of Task 6 added an extra call; removed.
- **Does `dispatch_bridge_collapse_from_hut` take terrain?** → No (line 174 signature). Task 5 plumbs it through.
- **What's the actual field name for the resolved terrain on `Simulation`?** → `pub resolved_terrain: Option<ResolvedTerrainGrid>` ([mod.rs:265](../../src/sim/world/mod.rs#L265)). Note the leading `resolved_` and the `Option` wrapper. Earlier Task 5 draft incorrectly wrote `&self.terrain`; corrected.

### Deferred to Implementation

- **Exact 8-direction order in `g_DirectionOffsets`**: post-implementation, run `/fidelity-check` against a known damage sequence on a real map and confirm the bit-state of every cell in the flood-fill region matches the binary. If a mismatch surfaces (state hash divergence in lockstep, or a visual cell that should be damaged but isn't), revisit and inspect the actual runtime contents of `0x0089F688` via the debugger MCP tools.
- **Bridgehead damage-state-machine hook semantics (Task 8 Steps 1-2)**: the LAT_RETRIGGER doc §4.4 caller table is wrong about `ProcessBridgeDestruction_{Low,High}` passing state=1 (binary at 0x00570506 verified passing state=0). Task 8's hook target depends on decompiling `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` (and Low equivalent) at execution time. Cannot be predetermined without that decompile. Task 8 is structured as RE-first → decide → implement.
- **`/verify-doc` follow-up on LAT_RETRIGGER §4.4**: separate doc-audit task to fix the doc's incorrect caller-table claim about ProcessBridgeDestruction. Out of scope for this plan, but should be queued.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) | Add `has_damaged_data: bool` to `ResolvedTerrainCell` + `TileMetadata`; propagate via `merge_tmp_metadata`; add map-load guard at line 874 |
| Modify | [src/map/terrain.rs](../../src/map/terrain.rs) | Add `has_damaged_data: bool` to `TerrainCell`; sync from ResolvedTerrainCell; add `bridge_state: Option<&BridgeRuntimeState>` parameter to `build_visible_instances`; compute damaged-variant override at line 568 |
| Modify | [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) | Add `EIGHT_NEIGHBOR_OFFSETS` const; implement `apply_damaged_variant_flood_fill` + internal helper; signature migration for `body_cell_advance_state` + `body_cell_repair_state`; wire clear calls into repair loop; conditionally wire bridgehead hook (Task 8 RE-gated); unit tests. Note: NO direct seed-cell flood-fill in the Healthy arm — that propagates via Task 7's perpendicular hooks (matches binary). |
| Modify | [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) | Signature migration for `update_ramp_perpendicular` to take `&ResolvedTerrainGrid`; wire writer calls per phase |
| Modify | [src/sim/world/bridge_orchestrator.rs](../../src/sim/world/bridge_orchestrator.rs) | Update `body_cell_advance_state` callers (3 sites) to pass `&terrain` |
| Modify | [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) | Update `body_cell_repair_state` caller (1 site) to pass `&terrain` |
| Modify | [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) | Pass `state.simulation.as_ref().map(|s| &s.bridge_state)` to `terrain::build_visible_instances` |
| Modify | [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs) | Integration tests for damaged_variant flip on damage and clear on repair |

## Interface Changes

**Sim — signature changes:**
- `BridgeRuntimeState::body_cell_advance_state` gains `terrain: &ResolvedTerrainGrid` parameter
- `BridgeRuntimeState::body_cell_repair_state` gains `terrain: &ResolvedTerrainGrid` parameter
- `bridge_specs::update_ramp_perpendicular` gains `terrain: &ResolvedTerrainGrid` parameter
- New: `BridgeRuntimeState::apply_damaged_variant_flood_fill(rx, ry, state, terrain) -> u32`

**Render — signature changes:**
- `terrain::build_visible_instances` gains `bridge_state: Option<&BridgeRuntimeState>` parameter

**Data — field additions:**
- `ResolvedTerrainCell.has_damaged_data: bool` (new, default `false`)
- `TileMetadata.has_damaged_data: bool` (new, default `false`)
- `TerrainCell.has_damaged_data: bool` (new, default `false`)

**Callers requiring update:**
- `body_cell_advance_state`: bridge_orchestrator.rs (3 sites: lines 202, 644, 653) + 8 test sites in bridge_state/mod.rs
- `body_cell_repair_state`: world_orders.rs:340 + 10 test sites in bridge_state/mod.rs
- `update_ramp_perpendicular`: 10 internal callers in bridge_state/mod.rs + 6 test sites in bridge_specs.rs
- `terrain::build_visible_instances`: app_render/build_instances.rs (1 site)

## Sim Checklist

- [x] All math uses `fixed`-point — N/A; this design uses `u16` coords and `bool` state, no fixed-point math.
- [x] New state included in deterministic state hash — `damaged_variant` is already in [world_hash.rs:233](../../src/sim/world/world_hash.rs#L233).
- [x] No dependencies on render/ui/sidebar/audio/net — sim writer reads `&ResolvedTerrainGrid` (map layer, allowed) and mutates `BridgeRuntimeState` (sim layer). No render/ui/audio/net imports.
- [x] Tick ordering impact noted — writer fires during Phase F (combat damage application); render reads next frame. No new tick-stage hazards.
- [x] BTreeMap iteration order considered — `body_cell_repair_state` already iterates `BTreeSet<u16>` spans; the new flood-fill call inside that loop fires per-cell in span-sort order; deterministic.

## Risk Areas

- **Signature migration churn**: ~28 call sites updated across 4 production files + many tests. Risk is mechanical compile errors caught immediately by `cargo check`. Mitigation: do the signature migration as one task per function, run `cargo check` between each, fix breakage at each step.
- **Determinism**: flood-fill iteration order is fixed (N→NE→E→SE→S→SW→W→NW). The bool-flip is order-independent for FINAL state but if any future code reads damaged_variant mid-flood-fill in the same tick, order matters. No such reader exists today; defensive comment in the implementation.
- **Render performance**: per-frame closure on every terrain cell now does an extra `BridgeRuntimeState::cell()` lookup. `BridgeRuntimeState::cell` is O(1) on a flat Vec lookup ([mod.rs:613](../../src/sim/bridge_state/mod.rs#L613)). Negligible per-frame cost.
- **TerrainGrid sync**: `TerrainCell.has_damaged_data` is set at TerrainGrid construction from ResolvedTerrainCell. If a future system mutates ResolvedTerrainCell.has_damaged_data at runtime (none today), the TerrainCell copy could go stale. Defensive: document that `has_damaged_data` is map-load-immutable and any future runtime change requires resync.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 7 | UpdateRamp DamageA/B/CollapseA/CollapseB all pass `state=true`; the seed/damage cell acquires the bit ONLY via flood-fill propagation from a perpendicular target's region (matching binary's no-direct-seed-call behavior) | Ledger #1: damage AND collapse paths share state=1; if collapse passes state=false we lose persistence-through-collapse. The seed-cell-via-propagation behavior is parity-load-bearing — adding a defensive seed-cell call would diverge from gamemd in tile_id-boundary edge cases | Integration test `damage_sets_damaged_variant_bit_on_body_cell` (verifies seed flips via propagation when seed and perp share tile_id); `collapse_preserves_damaged_variant_bit`; binary disassembly already verified `PUSH 0x1` at all UpdateRamp callsites |
| Task 8 | RE-gated hook decision for the bridgehead damage-state-machine path; outcome depends on Step 1's decompile of `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` | Without RE, hook risks shipping wrong-direction state-arg (LAT_RETRIGGER §4.4 doc was wrong about ProcessBridgeDestruction passing state=1; binary at 0x00570506 actually passes state=0). Bridgehead damage frequency: every bridgehead-targeting attack | Step 1's decompile + Step 2's option choice; subsequent integration test sized to the chosen option |
| Task 9 | Repair clears flood-fill with `state=false` | Engineer repair must restore pristine appearance — every CABHUT capture | Integration test `repair_clears_damaged_variant_bit`; existing repair tests at world_orders_bridge_repair_tests.rs |
| Task 11 | Map-load guard skips PRNG variant pick on has_damaged_data tiles | If guard missing, bridge tiles get random `cell.variant` at map load and the bool override at draw time may stomp on it (or vice versa). Today safe-by-accident; explicit guard prevents regression from future modded theaters | Unit test `map_load_does_not_randomize_bridge_tile_variant`; in-game: load map with bridge, confirm bridge cells render variant=0 before any damage |
| Task 12 | Render override at terrain.rs:568 picks variant=1 when damaged_variant=true | The whole feature is invisible without this. Every damaged-bridge frame depends on this line | Manual visual check: damage bridge → see scuffed texture; repair → see pristine restored |
| Task 13 | Render closure receives bridge_state reference | Without this wiring, the override at Task 12 has no source of truth | Compile check; manual visual verification |

---

## Tasks

### Task 1: Plumb `has_damaged_data: bool` through map-load to `ResolvedTerrainCell`

**Why:** The flood-fill writer's kickoff gate (per ledger #2) and the render-side override (per ledger #18) both need to know whether a given cell's tile has a baked damaged variant. The TMP parser already extracts this bit (`TmpTile.has_damaged_data`) but doesn't propagate it past `TileMetadata` in resolved_terrain. This task wires it through end-to-end.

**Files:**
- Modify: [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) struct `ResolvedTerrainCell` (around line 68); struct `TileMetadata` (around line 929); fn `merge_tmp_metadata` (line 1127); ResolvedTerrainCell construction site (around line 256)

**Pattern:** Mirrors the existing field-plumbing pattern for `slope_type`, `radar_left`, `radar_right`, etc. — TmpTile field → TileMetadata field → ResolvedTerrainCell field.

**Step 1: Add field to `ResolvedTerrainCell`**

In `src/map/resolved_terrain.rs`, find the `ResolvedTerrainCell` struct (the line near 68 with `pub final_tile_index: i32,` and `pub final_sub_tile: u8,`). Add this field at the end of the struct definition (preserve trailing comma):

```rust
    /// True if this cell's underlying TMP sub-tile has the HasDamagedData flag
    /// (bit 0x04 of the per-cell flag DWORD at TMP +0x24). When true and the
    /// containing bridge cell's `damaged_variant` bit is set, the renderer
    /// substitutes the baked damaged variant atlas entry.
    pub has_damaged_data: bool,
```

**Step 2: Add field to `TileMetadata`**

Find the `TileMetadata` struct around line 929. Add this field at the end (before the closing `}`):

```rust
    /// Mirrors TmpTile.has_damaged_data — bit 0x04 of TMP per-cell flag DWORD.
    has_damaged_data: bool,
```

**Step 3: Update `merge_tmp_metadata`**

Find `fn merge_tmp_metadata` around line 1127. Add this line at the end of the function body, just before the closing `}`:

```rust
    metadata.has_damaged_data = tile.has_damaged_data;
```

**Step 4: Populate the field at ResolvedTerrainCell construction**

Find the construction site of `ResolvedTerrainCell` (search for the largest struct literal of that type in resolved_terrain.rs — around line 256-260 or wherever `final_tile_index`/`final_sub_tile` are set together). Add a line that reads the metadata's flag:

```rust
                    has_damaged_data: metadata.has_damaged_data,
```

If there's a second construction site that writes a default cell (the `else` branch when the tile isn't found), set:

```rust
                    has_damaged_data: false,
```

Use the Grep tool first to find ALL ResolvedTerrainCell construction sites: `Grep pattern="ResolvedTerrainCell \{" path="src/map/resolved_terrain.rs"`. Each one needs the new field.

**Step 5: Update test fixtures**

Run: `cargo check --tests 2>&1 | head -40`

If any test fixture is constructing `ResolvedTerrainCell` directly and needs the new field, add `has_damaged_data: false` to it. The same applies to `TmpTile` test fixtures that already exist (e.g., at lines 1357, 1437, 1468 per earlier grep) — those don't need new fixtures, the `has_damaged_data: false` line is already there.

**Step 6: Verify**

Run: `cargo check --tests`
Expected: PASS (no errors)

Run: `cargo test resolved_terrain -- --nocapture`
Expected: PASS (existing tests continue to pass; no new tests added in this task)

**Step 7: Commit**

```
git add src/map/resolved_terrain.rs
git commit -m "$(cat <<'EOF'
map/resolved_terrain: plumb has_damaged_data flag from TMP to ResolvedTerrainCell

Wires the per-sub-tile HasDamagedData bit (TMP +0x24 bit 0x04, already parsed
into TmpTile) through TileMetadata to ResolvedTerrainCell. Required by the G4
bridge damaged-variant flood-fill gate.
EOF
)"
```

---

### Task 2: Plumb `has_damaged_data: bool` from `ResolvedTerrainCell` to `TerrainCell` for the render path

**Why:** The render-side override (Task 12) needs to know per-cell whether a damaged variant is available so it can substitute `TileKey { variant: 1 }`. `TerrainCell` is the render-side cell representation; it must carry the flag.

**Files:**
- Modify: [src/map/terrain.rs](../../src/map/terrain.rs) struct `TerrainCell` (line near 145); construction site(s) inside the same file

**Pattern:** Same as Task 1 — adds a bool field mirroring the existing `radar_left`/`radar_right`/`variant` fields on TerrainCell.

**Step 1: Add field to `TerrainCell`**

Find the `TerrainCell` struct (search for `pub variant: u8,` in terrain.rs — that line is at 150 per prior grep). Add this field after the existing fields, before the closing `}`:

```rust
    /// Mirrors ResolvedTerrainCell.has_damaged_data — true when this cell's
    /// TMP sub-tile has a baked damaged-variant available. Cached at TerrainGrid
    /// construction; treat as map-load-immutable.
    pub has_damaged_data: bool,
```

**Step 2: Populate at construction**

Find every `TerrainCell { ... }` literal in terrain.rs. Earlier grep found these at lines around 377 (default cell), 455 (real cell with variant), 694, 709 (more defaults). For each:
- Real-cell construction (e.g., line 455 area): set `has_damaged_data: cell.has_damaged_data,` where `cell` is the source `ResolvedTerrainCell`.
- Default/sentinel construction: set `has_damaged_data: false,`.

Use Grep to find all literal `TerrainCell {`: `Grep pattern="TerrainCell \{" path="src/map/terrain.rs"`. Each construction site needs the new field.

**Step 3: Verify**

Run: `cargo check --tests`
Expected: PASS

Run: `cargo test terrain -- --nocapture`
Expected: PASS (existing tests continue to pass)

**Step 4: Commit**

```
git add src/map/terrain.rs
git commit -m "$(cat <<'EOF'
map/terrain: plumb has_damaged_data flag from ResolvedTerrainCell to TerrainCell

Cached per-cell at TerrainGrid construction so the per-frame draw loop can
decide whether to substitute the damaged-variant atlas entry without a theater
lookup. Required by G4 render-side override.
EOF
)"
```

---

### Task 3: Add `EIGHT_NEIGHBOR_OFFSETS` const + `apply_damaged_variant_flood_fill` to `BridgeRuntimeState`

**Why:** The core sim primitive. Mirrors gamemd's `MapClass::ToggleBridgePavement`. Defined now so subsequent hook tasks (6-9) can call it. Internal helper carries the `kickoff` flag to enforce ledger #2 (gate-only-on-kickoff).

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — add const at module top (around line 17, after existing imports); add methods on `impl BridgeRuntimeState` (insert after `cell_mut` at line 625 — alongside other `pub fn cell*`-family methods feels right, but the natural home is near `body_cell_advance_state` at line 780).

**Pattern:** Method shape mirrors `body_cell_advance_state` — takes `&mut self`, mutates `BridgeRuntimeState`, queries `terrain: &ResolvedTerrainGrid` for tile_index lookup. Recursive helper pattern is consistent with the binary's `ToggleBridgePavement(suppress_self)` argument.

**Step 1: Add direction-offsets const**

In `src/sim/bridge_state/mod.rs`, near the top of the file (after the `use` statements around line 17), add:

```rust
/// 8-neighbor direction offsets used by `apply_damaged_variant_flood_fill`,
/// mirroring gamemd's `g_DirectionOffsets` at 0x0089F688. Order:
/// N, NE, E, SE, S, SW, W, NW (standard RA2 8-facing convention).
///
/// The order does not affect the final bit state (flood-fill is bool-idempotent
/// via the early-return guard), but it is fixed for deterministic iteration
/// across lockstep clients.
const EIGHT_NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (0, -1),   // N
    (1, -1),   // NE
    (1, 0),    // E
    (1, 1),    // SE
    (0, 1),    // S
    (-1, 1),   // SW
    (-1, 0),   // W
    (-1, -1),  // NW
];
```

**Step 2: Add the two methods**

Insert after `body_cell_advance_state` (line 944 — the closing `}` of the impl method) and before the next method. Add:

```rust
    /// Propagate the damaged-variant bit across an 8-neighbor region bounded
    /// by underlying-terrain `final_tile_index` equality. Mirrors the
    /// `ToggleBridgePavement` flood-fill: kickoff call gates on the seed
    /// cell's `has_damaged_data`; recursive calls skip the gate (the binary
    /// trusts that all cells sharing a tile_index share the gate flag since
    /// they're rendered from the same TMP).
    ///
    /// `state == true` flips the bit on (damage / collapse caller).
    /// `state == false` flips it off (repair walker caller).
    ///
    /// Idempotent: cells already in the target state return early without
    /// recursing — matches the binary's `(byte)param_2 != current_bit` guard.
    ///
    /// Returns the count of cells mutated.
    pub fn apply_damaged_variant_flood_fill(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> u32 {
        self.apply_damaged_variant_flood_fill_internal(rx, ry, state, terrain, true)
    }

    fn apply_damaged_variant_flood_fill_internal(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
        kickoff: bool,
    ) -> u32 {
        let cell_state = match self.cell(rx, ry) {
            Some(c) => c.damaged_variant,
            None => return 0,
        };
        if cell_state == state {
            return 0;
        }

        let resolved = match terrain.cell(rx, ry) {
            Some(c) => c,
            None => return 0,
        };
        let seed_tile_id = resolved.final_tile_index;
        if seed_tile_id == 0xFFFF || seed_tile_id == 0xFF {
            return 0;
        }

        if kickoff && !resolved.has_damaged_data {
            return 0;
        }

        if let Some(c) = self.cell_mut(rx, ry) {
            c.damaged_variant = state;
        }
        let mut count: u32 = 1;

        for (dx, dy) in EIGHT_NEIGHBOR_OFFSETS {
            let nx_i = rx as i32 + dx;
            let ny_i = ry as i32 + dy;
            if nx_i < 0 || ny_i < 0 {
                continue;
            }
            let nx = nx_i as u16;
            let ny = ny_i as u16;
            if let Some(n_resolved) = terrain.cell(nx, ny) {
                if n_resolved.final_tile_index == seed_tile_id {
                    count += self.apply_damaged_variant_flood_fill_internal(
                        nx, ny, state, terrain, false,
                    );
                }
            }
        }

        count
    }
```

**Step 3: Verify the file compiles**

Run: `cargo check --tests 2>&1 | head -40`
Expected: PASS — no compile errors. The methods are defined but not yet called from anywhere production.

**Step 4: Commit**

```
git add src/sim/bridge_state/mod.rs
git commit -m "$(cat <<'EOF'
sim/bridge_state: add apply_damaged_variant_flood_fill primitive

Mirrors gamemd's ToggleBridgePavement: recursive 8-neighbor flood-fill bounded
by ResolvedTerrainCell.final_tile_index equality, idempotent on bool target
state, gate-on-kickoff-only via terrain cell's has_damaged_data flag.

No call sites yet; this is the building block for the G4 writer hooks and
repair clear.
EOF
)"
```

---

### Task 4: Unit tests for `apply_damaged_variant_flood_fill`

**Why:** Lock in the algorithm's invariants (idempotency, tile-id boundary stop, gate skip on recursion, 8-direction coverage, sentinel early-return) before any wiring. Pure-logic tests are independent of the sim-tick loop and fast to run.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — add tests inside the existing `#[cfg(test)] mod tests` block (search for `#[cfg(test)]` near the bottom of the file).

**Pattern:** Mirrors existing tests in the same module (e.g., `body_cell_advance_state_*` tests at line 2103+) — uses a fixture `BridgeRuntimeState` + a constructed `ResolvedTerrainGrid` for the terrain context.

**Step 1: Find an existing test fixture helper**

Search the test module for an existing helper that builds a small `ResolvedTerrainGrid`. If one exists (e.g., `build_test_terrain_grid` or similar), reuse it. If not, the tests below can construct a minimal grid inline.

Run: `Grep pattern="(fn build_.*terrain|fn make_.*terrain|ResolvedTerrainGrid::new)" path="src" output_mode="content"`

If no helper exists, the tests below show how to build one inline.

**Step 2: Add the test helper at the bottom of the test module (only if no shared helper exists yet)**

```rust
    /// Build a minimal ResolvedTerrainGrid for flood-fill testing.
    /// Every cell has `final_tile_index = tile_id`, sub_tile=0, has_damaged_data=true.
    /// Cells outside the (width, height) range return None from .cell().
    fn build_test_terrain(width: u16, height: u16, tile_id: i32) -> crate::map::resolved_terrain::ResolvedTerrainGrid {
        // Use the public test constructor if available, otherwise construct directly.
        // Look at existing test setup in resolved_terrain.rs for the right pattern.
        // Example fixture pattern:
        let cells: Vec<crate::map::resolved_terrain::ResolvedTerrainCell> = (0..(width as usize * height as usize))
            .map(|i| crate::map::resolved_terrain::ResolvedTerrainCell {
                rx: (i % width as usize) as u16,
                ry: (i / width as usize) as u16,
                final_tile_index: tile_id,
                final_sub_tile: 0,
                source_sub_tile: 0,
                has_damaged_data: true,
                // ... other fields default
                ..Default::default()
            })
            .collect();
        crate::map::resolved_terrain::ResolvedTerrainGrid::from_cells_for_test(width, height, cells)
    }
```

If `ResolvedTerrainGrid` has no test constructor, add one in resolved_terrain.rs:

```rust
    #[cfg(test)]
    pub fn from_cells_for_test(width: u16, height: u16, cells: Vec<ResolvedTerrainCell>) -> Self {
        Self { width, height, cells }
    }
```

If `ResolvedTerrainCell` lacks `Default`, add `#[derive(Default)]` to its definition (it likely doesn't due to `final_tile_index: i32` being non-zero default — in that case, build cells by-hand without `..Default::default()`).

**Step 3: Add the six unit tests**

Inside the test module, after existing tests, add:

```rust
    #[test]
    fn flood_fill_kickoff_skips_when_no_damaged_data() {
        // Build a grid where has_damaged_data=false everywhere; flood-fill must no-op.
        let mut bs = build_test_bridge_state_with_cells(&[(5, 5), (5, 6)]);
        let mut terrain = build_test_terrain(10, 10, 42);
        // Override has_damaged_data to false for the seed cell.
        if let Some(c) = terrain.cell_mut(5, 5) {
            c.has_damaged_data = false;
        }
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0);
        assert_eq!(bs.cell(5, 5).unwrap().damaged_variant, false);
        assert_eq!(bs.cell(5, 6).unwrap().damaged_variant, false);
    }

    #[test]
    fn flood_fill_propagates_to_same_tile_id_neighbors() {
        // 3x3 region around (5,5) all share tile_id=42; flood-fill from (5,5) flips all 9.
        let mut bs = build_test_bridge_state_with_cells(&[
            (4, 4), (5, 4), (6, 4),
            (4, 5), (5, 5), (6, 5),
            (4, 6), (5, 6), (6, 6),
        ]);
        let terrain = build_test_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 9);
        for &(rx, ry) in &[(4,4),(5,4),(6,4),(4,5),(5,5),(6,5),(4,6),(5,6),(6,6)] {
            assert_eq!(bs.cell(rx, ry).unwrap().damaged_variant, true,
                "cell ({},{}) should be damaged", rx, ry);
        }
    }

    #[test]
    fn flood_fill_stops_at_different_tile_id_boundary() {
        // Same setup, but (5,6) has tile_id=99 instead of 42; that cell and downstream do NOT flip.
        let mut bs = build_test_bridge_state_with_cells(&[(5,5), (5,6), (5,7)]);
        let mut terrain = build_test_terrain(10, 10, 42);
        if let Some(c) = terrain.cell_mut(5, 6) {
            c.final_tile_index = 99;  // different tile_id
        }
        let _ = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(bs.cell(5, 5).unwrap().damaged_variant, true);
        assert_eq!(bs.cell(5, 6).unwrap().damaged_variant, false, "boundary cell stays pristine");
        assert_eq!(bs.cell(5, 7).unwrap().damaged_variant, false, "downstream cell stays pristine");
    }

    #[test]
    fn flood_fill_idempotent_when_already_in_target_state() {
        let mut bs = build_test_bridge_state_with_cells(&[(5,5)]);
        if let Some(c) = bs.cell_mut(5, 5) {
            c.damaged_variant = true;
        }
        let terrain = build_test_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0, "no mutation when already in target state");
    }

    #[test]
    fn flood_fill_eight_directions_includes_diagonals() {
        // 3-cell-wide cross + diagonals; flood-fill from center should hit all 8 neighbors.
        let mut bs = build_test_bridge_state_with_cells(&[
            (4,4),(5,4),(6,4),
            (4,5),(5,5),(6,5),
            (4,6),(5,6),(6,6),
        ]);
        let terrain = build_test_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 9);
        // Diagonals specifically:
        assert!(bs.cell(4, 4).unwrap().damaged_variant, "NW diagonal hit");
        assert!(bs.cell(6, 4).unwrap().damaged_variant, "NE diagonal hit");
        assert!(bs.cell(4, 6).unwrap().damaged_variant, "SW diagonal hit");
        assert!(bs.cell(6, 6).unwrap().damaged_variant, "SE diagonal hit");
    }

    #[test]
    fn flood_fill_clear_propagates_state_false() {
        // Pre-damage a region, then clear from one cell; all should flip back.
        let mut bs = build_test_bridge_state_with_cells(&[(5,5),(5,6),(5,7)]);
        for &(rx, ry) in &[(5u16,5u16),(5,6),(5,7)] {
            if let Some(c) = bs.cell_mut(rx, ry) {
                c.damaged_variant = true;
            }
        }
        let terrain = build_test_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(5, 5, false, &terrain);
        assert_eq!(count, 3);
        for &(rx, ry) in &[(5,5),(5,6),(5,7)] {
            assert_eq!(bs.cell(rx, ry).unwrap().damaged_variant, false);
        }
    }

    #[test]
    fn flood_fill_off_map_returns_zero() {
        let mut bs = build_test_bridge_state_with_cells(&[(5,5)]);
        let terrain = build_test_terrain(10, 10, 42);
        let count = bs.apply_damaged_variant_flood_fill(99, 99, true, &terrain);
        assert_eq!(count, 0);
    }

    #[test]
    fn flood_fill_sentinel_tile_id_returns_zero() {
        let mut bs = build_test_bridge_state_with_cells(&[(5,5)]);
        let mut terrain = build_test_terrain(10, 10, 42);
        if let Some(c) = terrain.cell_mut(5, 5) {
            c.final_tile_index = 0xFFFF;  // clear sentinel
        }
        let count = bs.apply_damaged_variant_flood_fill(5, 5, true, &terrain);
        assert_eq!(count, 0);
    }
```

You may need a `build_test_bridge_state_with_cells(&[(rx, ry), ...])` helper that constructs a `BridgeRuntimeState` with bridge cells at the given coords. Look for an existing pattern in `bridge_state/mod.rs` tests (e.g., near the `body_cell_advance_state` tests) and follow the same constructor — likely via `BridgeRuntimeState::default()` then manually inserting cells, OR a `from_test_cells` helper. If none exists, add one inside the test module:

```rust
    fn build_test_bridge_state_with_cells(coords: &[(u16, u16)]) -> BridgeRuntimeState {
        let mut state = BridgeRuntimeState::default();
        state.width = 10;
        state.height = 10;
        state.cells = vec![None; 100];
        for &(rx, ry) in coords {
            let idx = index_of(state.width, state.height, rx, ry).unwrap();
            state.cells[idx] = Some(BridgeRuntimeCell {
                deck_present: true,
                destroyable: true,
                deck_level: 0,
                bridge_group_id: Some(1),
                damage_state: DamageState::Healthy { variant: 0 },
                axis: Some(Axis::NS),
                role: BridgeCellRole::Body,
                anchor_span_id: None,
                overlay_byte: 0,
                damaged_variant: false,
            });
        }
        state
    }
```

If `BridgeRuntimeState::default()` doesn't exist or the test pattern is different, adapt to whatever existing tests do. Run `Grep pattern="fn build_.*bridge|let mut state = BridgeRuntimeState" path="src/sim/bridge_state/mod.rs"` to find the established pattern.

**Step 4: Verify**

Run: `cargo test --lib flood_fill -- --nocapture 2>&1 | head -60`
Expected: 8 tests all pass.

If any test fails:
- Read the assertion output carefully — it usually pinpoints exactly which invariant broke.
- Re-read the algorithm in Task 3 against the failing test's expected behavior.
- Fix the implementation, not the test (the tests encode the binary's behavior).

**Step 5: Commit**

```
git add src/sim/bridge_state/mod.rs src/map/resolved_terrain.rs
git commit -m "$(cat <<'EOF'
sim/bridge_state: unit tests for apply_damaged_variant_flood_fill

Covers: kickoff gate skip on has_damaged_data=false, 8-neighbor propagation
across same-tile-id region, boundary stop at different tile_id, idempotency
early-return, diagonal coverage, state=false clear path, off-map sentinel,
0xFFFF tile_id sentinel.

Adds ResolvedTerrainGrid::from_cells_for_test test helper.
EOF
)"
```

---

### Task 5: Signature migration — add `&ResolvedTerrainGrid` to body_cell_advance_state / body_cell_repair_state / update_ramp_perpendicular and update all callers

**Why:** Tasks 6-9 will call `apply_damaged_variant_flood_fill` from inside these three functions, and the flood-fill requires `&ResolvedTerrainGrid`. The migration must happen as one task to keep `cargo check` clean; otherwise intermediate states won't compile.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — change function signatures at lines 780 + 978 (production methods); update internal callers of `update_ramp_perpendicular` at lines 837, 844, 856, 863, 903, 923, 1156, 1163, 1189, 1196 (10 sites); update test callers at lines 2103, 2112, 2126, 2160, 2172, 2184, 2192, 2199 (`body_cell_advance_state`, 8 sites) + lines 2827, 2845, 2863, 2889, 2909, 2924, 2938, 2939, 2962, 3032 (`body_cell_repair_state`, 10 sites)
- Modify: [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) — change `update_ramp_perpendicular` signature at line 533; update test callers at lines 1300, 1309, 1320, 1332, 1341, 1373 (6 sites)
- Modify: [src/sim/world/bridge_orchestrator.rs](../../src/sim/world/bridge_orchestrator.rs) — update `body_cell_advance_state` callers at lines 202, 644, 653 (3 sites)
- Modify: [src/sim/world/world_orders.rs](../../src/sim/world/world_orders.rs) — update `body_cell_repair_state` caller at line 340 (1 site)

**Pattern:** Mirrors `bridgehead_advance_state`'s existing signature (line 1111) which already takes `terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid`.

**Step 1: Update `update_ramp_perpendicular` signature in bridge_specs.rs**

Find line 533. Change:
```rust
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,
    _is_high_bridge: bool,
) -> /* existing return type */ {
```
to:
```rust
pub fn update_ramp_perpendicular(
    state: &mut BridgeRuntimeState,
    anchor_pos: (u16, u16),
    axis: Axis,
    phase: Phase,
    _is_high_bridge: bool,
    _terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
) -> /* existing return type */ {
```

(Prefix `_terrain` so the unused warning is silenced — Task 7 will use it. Drop the underscore in Task 7.)

**Step 2: Update internal callers of `update_ramp_perpendicular` in bridge_state/mod.rs**

10 call sites at lines 837, 844, 856, 863, 903, 923, 1156, 1163, 1189, 1196. Each looks like:
```rust
let _ = crate::sim::bridge_specs::update_ramp_perpendicular(
    self,
    anchor_pos,
    axis,
    Phase::DamageA,
    is_high_bridge,
);
```

The callers must pass `terrain`. Since these calls happen inside `body_cell_advance_state` (lines 837-923) and `bridgehead_advance_state` (lines 1156-1196), the methods themselves must accept terrain and pass it through:

For `body_cell_advance_state` (line 780), change signature:
```rust
    pub fn body_cell_advance_state(
        &mut self,
        rx: u16,
        ry: u16,
        is_high_bridge: bool,
        terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> StateOutcome {
```

For `bridgehead_advance_state` — it already takes terrain. No change to its signature.

In all 10 internal callsites of `update_ramp_perpendicular`, append `, terrain` as the new argument.

**Step 3: Update `body_cell_repair_state` signature**

Find line 978. Change:
```rust
    pub fn body_cell_repair_state(
        &mut self,
        scan_cells: &[(u16, u16)],
        rng: &mut crate::sim::rng::SimRng,
    ) -> RepairOutcome {
```
to:
```rust
    pub fn body_cell_repair_state(
        &mut self,
        scan_cells: &[(u16, u16)],
        rng: &mut crate::sim::rng::SimRng,
        _terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    ) -> RepairOutcome {
```

(Underscore again — Task 9 will drop it.)

**Step 4: Update production callers**

`bridge_orchestrator.rs`:
- Lines 644 and 653: `bs.body_cell_advance_state(event.rx, event.ry, true/false)` — `terrain` is already in scope at line 642 (used by the `bridgehead_advance_state` call right above). Just add `, terrain` as the new last argument.
- Line 202: `bs.body_cell_advance_state(cell_pos.0, cell_pos.1, false)` — this lives inside `dispatch_bridge_collapse_from_hut(sim, rules, hut_center)` (function definition at line 174). That function does NOT currently take terrain. Plumb it through:
  - Add `terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid` as a new parameter to `dispatch_bridge_collapse_from_hut`.
  - Find the function's caller(s) via `Grep pattern="dispatch_bridge_collapse_from_hut\(" path="src" output_mode="content" -n=true`. At each caller, pass through whatever terrain reference is in scope. If the caller is inside `impl Simulation`, use the `if let Some(terrain) = self.resolved_terrain.as_ref()` guard pattern (see world_orders.rs note below).

`world_orders.rs:340`:
- The Simulation struct in [src/sim/world/mod.rs:265](../../src/sim/world/mod.rs#L265) declares `pub resolved_terrain: Option<ResolvedTerrainGrid>` — note the leading `resolved_` AND the `Option` wrapper. The plan's earlier draft incorrectly wrote `&self.terrain`; the correct access requires unwrapping the Option.
- Mirror the existing `if let Some(bs) = self.bridge_state.as_mut()` guard pattern at line 339. Replace lines 339-343:

```rust
            let outcome = if let (Some(bs), Some(terrain)) =
                (self.bridge_state.as_mut(), self.resolved_terrain.as_ref())
            {
                bs.body_cell_repair_state(&scan, &mut self.rng, terrain)
            } else {
                crate::sim::bridge_state::RepairOutcome::default()
            };
```

If `self.bridge_state.as_mut()` and `self.resolved_terrain.as_ref()` borrow conflict (both read `self`), split into two statements:

```rust
            let outcome = match self.resolved_terrain.as_ref() {
                Some(terrain) => {
                    let terrain_ref = terrain;
                    if let Some(bs) = self.bridge_state.as_mut() {
                        bs.body_cell_repair_state(&scan, &mut self.rng, terrain_ref)
                    } else {
                        crate::sim::bridge_state::RepairOutcome::default()
                    }
                }
                None => crate::sim::bridge_state::RepairOutcome::default(),
            };
```

(The borrow checker may accept the tuple-pattern form; if it doesn't, use the split form.)

**Step 5: Update test callers**

In bridge_state/mod.rs (8 sites for `body_cell_advance_state` + 10 for `body_cell_repair_state`) and bridge_specs.rs (6 sites for `update_ramp_perpendicular`), every call must pass a `&ResolvedTerrainGrid`. Each test likely builds a `BridgeRuntimeState` standalone without terrain — the tests need to construct a minimal `ResolvedTerrainGrid` too.

Use the helper from Task 4 step 2 (`build_test_terrain`). For each test:
- Add `let terrain = build_test_terrain(10, 10, 42);` near where the bridge state is built.
- Pass `&terrain` as the new last argument.

**Step 6: Verify**

Run: `cargo check --tests 2>&1 | head -40`
Expected: PASS (no errors).

Run: `cargo test --lib bridge_state 2>&1 | tail -30`
Expected: ALL existing tests PASS (no behavior change — only signature migration).

Run: `cargo test --lib bridge_specs 2>&1 | tail -30`
Expected: ALL existing tests PASS.

Run: `cargo check --bin <main-binary-name>` (find via `Glob pattern="src/main.rs"`; if the project is a lib, use `cargo build`)
Expected: PASS.

**Step 7: Commit**

```
git add -u src/sim/bridge_state src/sim/bridge_specs.rs src/sim/world/bridge_orchestrator.rs src/sim/world/world_orders.rs
git commit -m "$(cat <<'EOF'
sim/bridge: thread &ResolvedTerrainGrid through body_cell_*/update_ramp_perpendicular

Signature migration ahead of G4 damaged-variant writer hooks. body_cell_advance_state,
body_cell_repair_state, and update_ramp_perpendicular now all carry &terrain so they
can drive the tile_index-bounded flood-fill in subsequent tasks. Terrain param is
currently unused (underscored); subsequent tasks remove the underscores.

bridgehead_advance_state already had terrain — unchanged.
EOF
)"
```

---

### Task 6: No-op — verify the Healthy→Damaged path needs no seed-cell hook

**Why:** Original draft of this task added a defensive seed-cell flood-fill at `(rx, ry)` in the Healthy arm. `/review-plan` confirmed via Ghidra xref dump (31 callers of `ToggleBridgePavement` @ 0x0056E990) that the binary has NO direct caller at the damage-cell coordinate. The damage cell only acquires the bit via flood-fill propagation from a perpendicular target's region (Task 7's hook). Adding a seed-cell call would diverge from gamemd in the edge case where seed and perpendicular target sit on different `final_tile_index` regions — gamemd leaves the seed's region pristine in that case; an extra Rust call would flip it.

**Files:** None modified in this task.

**Pattern:** Parity-bar compliance. Match observable output, not "more complete" coverage.

**Step 1: Confirm Task 5 already wired terrain through the Healthy arm**

Re-read [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) at the `DamageState::Healthy { .. }` arm (around line 831). Confirm the two `update_ramp_perpendicular` calls now pass `terrain` as the new last argument (added in Task 5). No further code change needed here.

**Step 2: Verify**

Run: `cargo test --lib body_cell_advance_state -- --nocapture 2>&1 | head -40`
Expected: existing tests PASS.

**Step 3: Commit**

Nothing to commit. Skip.

---

### Task 7: Wire flood-fill writer calls into `update_ramp_perpendicular` (4 phases pass state=true)

**Why:** Per ledger #1, the binary's `UpdateRamp_*_DamageA/B` and `UpdateRamp_*_CollapseA/B` all call `ToggleBridgePavement(state=1)` on their perpendicular targets. This is the primary path that propagates the damage visual.

**Files:**
- Modify: [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) `update_ramp_perpendicular` at line 533

**Pattern:** Mirrors the binary — after the per-phase write to the perpendicular target, fire `apply_damaged_variant_flood_fill(perp_rx, perp_ry, true, terrain)`. All four phases pass state=true.

**Step 1: Drop the `_` from `_terrain` and use it**

Find the function body. After the perpendicular cell's `+0x11E`-equivalent write is applied (look for where the perpendicular target's `damage_state` or related field is mutated — likely near where the function returns), add:

```rust
    let _ = state.apply_damaged_variant_flood_fill(perp_rx, perp_ry, true, terrain);
```

where `perp_rx`/`perp_ry` are the perpendicular target's coords (already computed earlier in the function — look for the variable names actually used and substitute).

Change the param name from `_terrain` to `terrain` at the function signature.

If the function has early-return paths (e.g., off-map perpendicular target, no-op cases), DO NOT fire the flood-fill on those branches — only on the path where the perpendicular write actually happens. Match the binary's behavior: ToggleBridgePavement is called only when UpdateRamp actually mutates state.

Read the existing function body carefully before inserting — there may be multiple branches (per-phase or per-axis) and each one needs the flood-fill on the success path.

**Step 2: Verify**

Run: `cargo test --lib update_ramp_perpendicular -- --nocapture 2>&1 | head -40`
Expected: existing tests PASS. The new flood-fill calls don't change `UpdateRampOutcome` return shape (the existing field set is unchanged).

**Step 3: Commit**

```
git add src/sim/bridge_specs.rs
git commit -m "$(cat <<'EOF'
sim/bridge_specs: fire damaged-variant flood-fill in update_ramp_perpendicular

Per ToggleBridgePavement caller pattern: all four phases (DamageA/B + CollapseA/B)
pass state=true on the perpendicular target. The persistence-through-collapse
behavior comes for free from collapse passing state=1 (matches binary).
EOF
)"
```

---

### Task 8: Wire flood-fill writer call into `bridgehead_advance_state` (RE-gated)

**Why:** Bridgehead destruction in the binary fires `ToggleBridgePavement` from at least one path. The original LAT_RETRIGGER doc §4.4 caller table claimed `ProcessBridgeDestruction_{Low,High}` passes state=1, but `/review-plan` verified (disassembly at 0x00570506-0x00570511) that ProcessBridgeDestruction_Low actually passes `PUSH 0x0` (state=0 / CLEAR), not state=1. The doc is wrong about that caller, and the real bridgehead-damage-state-machine analog of Rust's `bridgehead_advance_state` is most likely `ProcessBridgeDamageStateMachine_High @ 0x00576BA0` (referenced in AUDIT_LOG 2026-05-12 but not yet decompiled). This task BLOCKS on RE work to determine the correct hook semantics.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) `bridgehead_advance_state` (line 1111+) — only after RE confirms the hook design

**Pattern:** Hook to be determined by binary verification.

**Step 1 (REQUIRED before any code change): Decompile `ProcessBridgeDamageStateMachine_High @ 0x00576BA0`**

Use Ghidra MCP `decompile_function(address="0x00576BA0")`. Trace every callsite of `MapClass__ToggleBridgePavement` inside the function and its callees. For each callsite, record:
- Coordinates passed (anchor's coord, bridgehead's coord, perpendicular target's coord, etc.)
- `state` argument value (state=1 or state=0)
- `suppress_self` argument value (kickoff or recursive)
- The branch / state transition that gates each callsite

Also check the Low equivalent (likely `ProcessBridgeDamageStateMachine_Low` at a nearby address — search via `get_function_callers` on `ToggleBridgePavement` and look for any unverified `0x0057XXXX`-range callers in the LAT_RETRIGGER §4.4 table).

**Step 2 (REQUIRED): Decide the hook target based on binary evidence**

Based on Step 1's findings, one of:

**(a) Binary fires `ToggleBridgePavement(state=1, suppress_self=0)` at the bridgehead's coord on Healthy→Damaged or on Damaged→Destroyed.** → Implement Task 8 as originally drafted: insert `let _ = self.apply_damaged_variant_flood_fill(rx, ry, true, terrain);` in the corresponding arm of `bridgehead_advance_state`.

**(b) Binary fires `ToggleBridgePavement(state=0, suppress_self=0)` at the bridgehead's coord** (similar to ProcessBridgeDestruction_Low's normalization pattern). → Insert with state=`false` instead, with a comment documenting the binary's "show pristine briefly during destruction" semantic.

**(c) Binary fires `ToggleBridgePavement` at a coord OTHER than the bridgehead's** (e.g., perpendicular target like UpdateRamp_*, or a swept region). → Mirror that exact pattern; the call site may not be in `bridgehead_advance_state` directly but in a helper.

**(d) Binary does NOT fire `ToggleBridgePavement` from the bridgehead state machine at all.** → Task 8 is a no-op; the bridgehead's tile_id region acquires the damaged_variant bit only via propagation from body-cell or perpendicular hooks (Task 7). Skip the hook entirely; document the finding in the commit message.

**Step 3: Implement the hook (only if Step 2 selects option (a), (b), or (c))**

Read `bridgehead_advance_state` from line 1111 onward. Find the `match` arm(s) that flip the bridgehead's `damage_state` (Healthy→Damaged and/or Damaged→Destroyed). Insert the flood-fill call in the arm(s) identified by Step 2:

```rust
                let _ = self.apply_damaged_variant_flood_fill(<coord_per_step_2>, <state_per_step_2>, terrain);
```

If Step 2 selected option (d), do NOT modify `bridgehead_advance_state`; proceed directly to Step 5.

**Step 4: Verify**

Run: `cargo test --lib bridgehead_advance_state -- --nocapture 2>&1 | head -40`
Expected: existing tests PASS.

If Step 2 selected option (d), no code change to verify — skip.

**Step 5: Commit**

If a hook was added (options a/b/c):
```
git add src/sim/bridge_state/mod.rs
git commit -m "$(cat <<'EOF'
sim/bridge_state: damaged-variant flood-fill in bridgehead_advance_state

Per Ghidra decompile of ProcessBridgeDamageStateMachine_{High,Low}: <option a/b/c
summary>. <state-arg justification with binary evidence>.
EOF
)"
```

If no hook needed (option d):
```
git commit --allow-empty -m "$(cat <<'EOF'
sim/bridge_state: confirm bridgehead path needs no damaged-variant hook

Decompile of ProcessBridgeDamageStateMachine_High @ 0x00576BA0 shows the
bridgehead state machine does not fire ToggleBridgePavement; the bridgehead's
tile_id region acquires the damaged_variant bit via flood-fill propagation
from body-cell perpendicular hooks (Task 7). No hook needed at this site.
EOF
)"
```

(Empty-commit is acceptable here as a research-finding artifact in the git history.)

---

### Task 9: Wire flood-fill clear call into `body_cell_repair_state`

**Why:** Per ledger #13, only the binary's repair walkers pass state=0 to ToggleBridgePavement. The Rust equivalent is `body_cell_repair_state` — each cell it transitions to Healthy should also fire `apply_damaged_variant_flood_fill(rx, ry, false, terrain)` to clear the bit across its tile_id region.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) `body_cell_repair_state` at line 978 (specifically the Step 2 loop body around line 1038-1041)

**Pattern:** Insert the clear call immediately after `cell.damage_state = new_state` (line 1039). Drop the `_` from `_terrain`.

**Step 1: Update the function signature**

Change `_terrain` to `terrain` in the parameter list.

**Step 2: Add clear call after each repaired cell's state mutation**

Find the line that writes `cell.damage_state = new_state;` (around line 1039). Immediately after, add:

```rust
                let _ = self.apply_damaged_variant_flood_fill(cell_pos.0, cell_pos.1, false, terrain);
```

Note: the `cell.damage_state = new_state;` write is inside `if let Some(cell) = self.cell_mut(cell_pos.0, cell_pos.1)`, so the flood-fill call must be OUTSIDE that `if let` block (otherwise the borrow on `self.cell_mut` collides with `self.apply_damaged_variant_flood_fill`). Verify the borrow scope is clean — Rust's borrow checker will tell you immediately if it's wrong.

Final structure:
```rust
                if let Some(cell) = self.cell_mut(cell_pos.0, cell_pos.1) {
                    cell.damage_state = new_state;
                }
                // G4: clear the damaged-variant bit across this cell's tile_id region.
                let _ = self.apply_damaged_variant_flood_fill(cell_pos.0, cell_pos.1, false, terrain);
                outcome.repaired_cells += 1;
```

**Step 3: Verify**

Run: `cargo test --lib body_cell_repair_state -- --nocapture 2>&1 | head -40`
Expected: existing tests PASS. The new flood-fill call doesn't affect `RepairOutcome` return shape.

**Step 4: Commit**

```
git add src/sim/bridge_state/mod.rs
git commit -m "$(cat <<'EOF'
sim/bridge_state: clear damaged-variant bit in body_cell_repair_state

Mirrors gamemd's FUN_00569760/FUN_00568E40 repair walkers: every cell that
transitions to Healthy also fires a state=false flood-fill across its tile_id
region, restoring pristine bridge art on engineer-CABHUT capture.
EOF
)"
```

---

### Task 10: Integration tests — damaged_variant set on damage, cleared on repair

**Why:** End-to-end verification that the full sim path (damage→writer, repair→clear) produces the right `damaged_variant` state. Catches integration bugs that unit tests in Task 4 can't see (signature mismatches, borrow issues, hook-site placement).

**Files:**
- Modify: [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs)

**Pattern:** Mirrors the existing integration tests in that file (e.g., engineer-enters-CABHUT-resets-damage-state). Adds assertions on `damaged_variant`.

**Step 1: Read existing test patterns**

Open [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs). Identify:
- The fixture setup helper (likely a `build_test_world_with_bridge` or similar)
- An existing test that damages then repairs a bridge

**Step 2: Add four new integration tests**

After the existing tests, add:

```rust
    #[test]
    fn g4_damage_sets_damaged_variant_bit_on_body_cell() {
        let mut world = build_test_world_with_bridge();
        // (precondition checks — use whatever existing setup names)
        let bridge_coord = /* coord of a body cell on the test bridge */;
        assert_eq!(world.bridge_state.cell(bridge_coord.0, bridge_coord.1).unwrap().damaged_variant, false);

        // Apply one round of damage to transition Healthy → Damaged.
        // (use whatever the existing test pattern is — likely an apply_bridge_damage_event or similar)
        apply_bridge_damage(&mut world, bridge_coord);

        assert_eq!(
            world.bridge_state.cell(bridge_coord.0, bridge_coord.1).unwrap().damaged_variant,
            true,
            "damaged_variant must be set after Healthy->Damaged transition"
        );
    }

    #[test]
    fn g4_collapse_preserves_damaged_variant_bit() {
        let mut world = build_test_world_with_bridge();
        let bridge_coord = /* coord */;
        // Damage twice: Healthy → Damaged → Destroyed
        apply_bridge_damage(&mut world, bridge_coord);
        apply_bridge_damage(&mut world, bridge_coord);
        assert_eq!(
            world.bridge_state.cell(bridge_coord.0, bridge_coord.1).unwrap().damaged_variant,
            true,
            "damaged_variant must persist through collapse (binary passes state=1 in both damage AND collapse callers)"
        );
    }

    #[test]
    fn g4_repair_clears_damaged_variant_bit() {
        let mut world = build_test_world_with_bridge();
        let bridge_coord = /* coord */;
        apply_bridge_damage(&mut world, bridge_coord);
        assert_eq!(world.bridge_state.cell(bridge_coord.0, bridge_coord.1).unwrap().damaged_variant, true);

        // Engineer enters CABHUT (use the existing test helper for this).
        engineer_repair_cabhut(&mut world, /* hut coord */);

        assert_eq!(
            world.bridge_state.cell(bridge_coord.0, bridge_coord.1).unwrap().damaged_variant,
            false,
            "damaged_variant must be cleared after repair"
        );
    }

    #[test]
    fn g4_repair_propagates_clear_via_flood_fill() {
        let mut world = build_test_world_with_bridge();
        let coord_a = /* one body cell coord */;
        let coord_b = /* a same-tile-id neighbor of coord_a */;

        // Both cells start pristine; damage one; both end up damaged via flood-fill.
        apply_bridge_damage(&mut world, coord_a);
        assert!(world.bridge_state.cell(coord_a.0, coord_a.1).unwrap().damaged_variant);
        assert!(world.bridge_state.cell(coord_b.0, coord_b.1).unwrap().damaged_variant,
            "neighbor with same tile_id must also be flipped via flood-fill");

        // Repair clears both.
        engineer_repair_cabhut(&mut world, /* hut coord */);
        assert!(!world.bridge_state.cell(coord_a.0, coord_a.1).unwrap().damaged_variant);
        assert!(!world.bridge_state.cell(coord_b.0, coord_b.1).unwrap().damaged_variant);
    }
```

The exact `apply_bridge_damage` / `engineer_repair_cabhut` helper names depend on what already exists in the test file. Read the existing tests and reuse their helpers; adapt the calls accordingly.

**Step 3: Verify**

Run: `cargo test --lib g4_ -- --nocapture 2>&1 | head -60`
Expected: All 4 new tests PASS.

If a test fails:
- Verify the fixture builds the bridge correctly (terrain cells must have `has_damaged_data: true` for the flood-fill gate to fire).
- Trace through the damage path — does it reach `body_cell_advance_state`? Or a different path that wasn't hooked?
- Trace the repair path — does it reach `body_cell_repair_state`?

**Step 4: Commit**

```
git add src/sim/world/world_orders_bridge_repair_tests.rs
git commit -m "$(cat <<'EOF'
sim/world: integration tests for G4 damaged-variant set/clear lifecycle

Covers:
- damaged_variant set on Healthy->Damaged transition
- damaged_variant persists through collapse (state=1 from both damage and
  collapse callers per ToggleBridgePavement xref pattern)
- damaged_variant cleared on engineer-CABHUT repair
- flood-fill propagation across same-tile-id neighbors verified end-to-end
EOF
)"
```

---

### Task 11: Add map-load guard in resolved_terrain.rs — skip PRNG variant pick for has_damaged_data tiles

**Why:** Per ledger #18 and the comment at [resolved_terrain.rs:866](../../src/map/resolved_terrain.rs#L866) ("Tiles with HasDamagedData (bridges) use variants for damage states, not visual diversity — those are excluded"), the map-load random variant pick at line 874 should NOT randomize bridge tiles. Today it's safe-by-accident (bridge tiles don't ship FA2 visual-diversity variants), but explicit guard prevents regression from future modded theaters.

**Files:**
- Modify: [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) lines 864-887 (the per-cell variant assignment loop)

**Pattern:** Add an `if` guard inside the existing loop body.

**Step 1: Read the loop**

Lines 864-887 currently read:
```rust
        // Assign random tile visual variants (FA2 bRNDImage, MapData.cpp:3292-3306).
        // Uses deterministic hash of (rx, ry) for reproducibility across sessions.
        // Tiles with HasDamagedData (bridges) use variants for damage states, not
        // visual diversity — those are excluded.
        if let Some(td) = theater_data {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut variant_total: usize = 0;
            for cell in &mut cells {
                let tile_id = normalize_tile_id(cell.final_tile_index);
                let vc = td.lookup.variant_count(tile_id);
                if vc == 0 {
                    continue;
                }
                let mut hasher = DefaultHasher::new();
                (cell.rx, cell.ry).hash(&mut hasher);
                let hash = hasher.finish();
                cell.variant = (hash % (vc as u64 + 1)) as u8;
                if cell.variant > 0 {
                    variant_total += 1;
                }
            }
            // ... log message
        }
```

**Step 2: Add the explicit guard**

Add this after the `if vc == 0 { continue; }` line and before the hash computation:

```rust
                // G4 guard: bridge tiles with has_damaged_data are picked by the
                // damaged_variant bit at draw time (cell.Flags & 0x2000 equivalent),
                // NOT by the FA2 hash-of-coord visual-diversity selector. Leave
                // cell.variant = 0 and let the render path override at draw time.
                if cell.has_damaged_data {
                    continue;
                }
```

**Step 3: Verify**

Run: `cargo check --tests`
Expected: PASS.

Run: `cargo test --lib resolved_terrain -- --nocapture 2>&1 | head -40`
Expected: existing tests still PASS.

**Step 4: Commit**

```
git add src/map/resolved_terrain.rs
git commit -m "$(cat <<'EOF'
map/resolved_terrain: skip PRNG variant pick for has_damaged_data tiles

Makes the previously-implicit exclusion explicit. Bridge tiles with baked
damaged variants must keep cell.variant=0 at map load so the per-frame
damaged_variant override (G4 render hook) drives variant selection.
EOF
)"
```

---

### Task 12: Render override — substitute variant=1 in `terrain::build_visible_instances` when bridge cell is damaged

**Why:** This is the line that makes the feature visible. Without it, the damaged_variant bool exists in sim state but no pixels change. Per ledger #10, the render-side picks variant 1 when `has_damaged_data && damaged_variant`.

**Files:**
- Modify: [src/map/terrain.rs](../../src/map/terrain.rs) — change `build_visible_instances` signature to accept `bridge_state: Option<&BridgeRuntimeState>`; insert variant-override logic at line 567

**Pattern:** Replaces the `cell.variant` value passed to `uv_fn` with a computed override.

**Step 1: Update the function signature**

Find `pub fn build_visible_instances` at line 517. Add a new parameter:

```rust
pub fn build_visible_instances(
    grid: &TerrainGrid,
    camera_x: f32,
    camera_y: f32,
    sw: f32,
    sh: f32,
    uv_fn: UvLookupFn<'_>,
    fog: Option<(
        crate::sim::intern::InternedId,
        &crate::sim::vision::FogState,
    )>,
    bridge_state: Option<&crate::sim::bridge_state::BridgeRuntimeState>,
) -> Vec<SpriteInstance> {
```

(Add `bridge_state` as the last parameter to minimize churn at call sites.)

**Step 2: Compute variant per cell, substitute in `uv_fn` call**

Find line 567-568:
```rust
        let placement: Option<TilePlacement> = match &uv_fn {
            Some(f) => f(cell.tile_id, cell.sub_tile, cell.variant),
            None => Some(TilePlacement { ... }),
        };
```

Change to:
```rust
        // G4: if this cell is a bridge body cell with a damaged-variant baked in
        // AND the sim has flagged damaged_variant=true, render variant=1 instead
        // of the FA2 visual-diversity variant. PRNG jitter is intentionally lost
        // while damaged (matches gamemd: damaged-data path returns before reaching
        // the GetTileVariantIndex PRNG).
        let effective_variant: u8 = if cell.has_damaged_data {
            bridge_state
                .and_then(|bs| bs.cell(cell.rx, cell.ry))
                .map(|bc| bc.damaged_variant as u8)
                .unwrap_or(0)
        } else {
            cell.variant
        };
        let placement: Option<TilePlacement> = match &uv_fn {
            Some(f) => f(cell.tile_id, cell.sub_tile, effective_variant),
            None => Some(TilePlacement { /* defaults preserved */ }),
        };
```

Preserve the existing default `TilePlacement { ... }` body in the `None` branch — only change the `Some(f) =>` line.

**Step 3: Update the call site to pass `None` for now**

Run: `cargo check --tests 2>&1 | head -20`

Expect a compile error about missing argument at the call site in `src/app_render/build_instances.rs`. Find it (around line 140) and pass `None` for `bridge_state`:

```rust
        terrain::build_visible_instances(
            grid,
            state.camera_x,
            state.camera_y,
            sw,
            sh,
            uv_fn,
            fog_vis,
            None,  // G4 bridge_state — wired in Task 13
        )
```

**Step 4: Update any tests that call `build_visible_instances`**

Run: `cargo check --tests 2>&1 | head -40`

Add `None` as the new last argument at each test site. Most likely zero or one test sites; if more, mechanical.

**Step 5: Verify**

Run: `cargo check --tests`
Expected: PASS.

Run: `cargo test --lib terrain -- --nocapture 2>&1 | head -30`
Expected: existing tests PASS (with bridge_state=None, behavior is unchanged from prior — variant override is a no-op).

**Step 6: Commit**

```
git add src/map/terrain.rs src/app_render/build_instances.rs
git commit -m "$(cat <<'EOF'
map/terrain: substitute damaged-variant in build_visible_instances

When a cell has has_damaged_data and the sim's BridgeRuntimeState reports
damaged_variant=true for that coord, render TileKey { variant: 1 } instead
of the FA2 visual-diversity variant. Wired with bridge_state=None for now;
Task 13 plumbs the real bridge_state through.
EOF
)"
```

---

### Task 13: Plumb `BridgeRuntimeState` to `build_visible_instances` at the app_render call site

**Why:** Activates the override from Task 12. Until this task, `bridge_state` is `None` and the override is dormant.

**Files:**
- Modify: [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs)

**Pattern:** Mirrors how other sim state is exposed to render (e.g., `fog_vis` is read from `state.simulation.fog`). Read `BridgeRuntimeState` from `state.simulation.bridge_state` (or whatever the field is called — verify by grepping the sim struct).

**Step 1: Locate the BridgeRuntimeState in the simulation state**

Run: `Grep pattern="bridge_state: BridgeRuntimeState|pub bridge_state" path="src/sim" output_mode="content" -n=true`

Identify the access path — e.g., `state.simulation.as_ref().map(|s| &s.bridge_state)`.

**Step 2: Replace the `None` argument**

In [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs), find the `terrain::build_visible_instances(...)` call site and replace `None` with:

```rust
            state.simulation.as_ref().map(|sim| &sim.bridge_state),
```

(Adapt the field path to match the actual structure.)

**Step 3: Verify**

Run: `cargo check --tests`
Expected: PASS.

Run: `cargo build`
Expected: PASS.

Run: `cargo test --lib`
Expected: ALL existing tests PASS.

**Step 4: Commit**

```
git add src/app_render/build_instances.rs
git commit -m "$(cat <<'EOF'
app_render: pass BridgeRuntimeState to build_visible_instances

Activates the G4 damaged-variant render override. The render path now reads
the sim's damaged_variant bool per bridge cell and substitutes the variant=1
atlas entry when set.
EOF
)"
```

---

### Task 14: Manual visual verification against gamemd.exe

**Why:** Confirm the feature is observable and matches the original engine. Tasks 4, 10 cover sim correctness via tests; this task confirms the render integration works end-to-end and the player sees the right pixels.

**Verify:**

Run the game (`cargo run` or whatever the dev-run command is). Load a skirmish on a map with a bridge (the default skirmish maps usually have at least one bridge — Tournament_Islands or similar).

**Test 1 — pristine bridge:**
- Start the match.
- Visually confirm the bridge renders in its pristine state (no scuffs / cracks).
- Expected match against gamemd.exe: identical pristine art.

**Test 2 — damage transition:**
- Have a unit attack the bridge (or use a debug hotkey if one exists to apply damage directly).
- After the first damage hit, visually confirm the bridge tiles in the affected region show the damaged variant (scuffed / cracked baked art).
- Expected match against gamemd.exe: same regional spread of damaged tiles (8-neighbor flood-fill bounded by tile_id).
- If the textures look identical to pristine: bridge_state may not be reaching the render path, OR the TileKey {variant:1} entry is missing from the atlas (verify with `Grep` for the relevant log line at theater.rs:705 — count of loaded variants should include the bridge's `'a'` file).

**Test 3 — collapse persistence:**
- Continue damaging until the bridge collapses.
- Just before collapse, confirm the damaged variant is still showing.
- Expected: damaged_variant bit persists through collapse (ledger #9).

**Test 4 — engineer repair:**
- Send an engineer into the CABHUT.
- Confirm the bridge regenerates AND the damaged variant clears (returns to pristine baked art).
- Expected match against gamemd.exe: identical pristine restoration.

**Test 5 — radar / minimap:**
- After damaging the bridge, glance at the minimap.
- The damaged bridge cells should show their damaged-radar-pixel color (different from pristine) per ledger #19.
- Note: if the minimap doesn't render through `terrain::build_visible_instances` (different code path), this test may not be satisfied even with Tasks 1-13 complete. Flag as a follow-up if so.

**Capture findings:**

If any test fails:
- Document what was expected vs what was observed in this plan as a follow-up task.
- Common causes: bridge_state field path wrong (Task 13), tile_id mismatch at flood-fill boundary (Task 7), missing TileKey variant in atlas (theater load issue).

**Step 1: Run the dev build**

Run: `cargo run --release 2>&1 | tail -20`
(Use `--release` if the game performance is slow in debug mode.)

**Step 2: Perform the 5 manual tests above**

Record observations.

**Step 3: If anything fails, do NOT proceed to commit — open a follow-up issue or extend this plan with a corrective task.**

**Step 4: If all tests pass, commit nothing here** — there's no code change in this task. Optionally add a summary comment to the design doc noting "G4 visual verification: PASS (date)".

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-12-g4-bridge-damaged-variant-design.md](2026-05-12-g4-bridge-damaged-variant-design.md)
- **Ghidra reports:**
  - [LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/LAT_RETRIGGER_AND_BRIDGE_DAMAGE_VARIANT_GHIDRA_REPORT.md) §4 — ToggleBridgePavement algorithm
  - [TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TMP_DAMAGED_VARIANT_LAYOUT_GHIDRA_REPORT.md) — TMP file layout, variant chain, theater loader
  - [ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md) §14, §17 — variant pick at draw time
- **gamemd.exe addresses (do NOT cite in Rust code comments per project memory):**
  - 0x0056E990 MapClass::ToggleBridgePavement — the algorithm template
  - 0x005471F0 HasDamagedVariantAtSubTile — TMP +0x24 bit 2 gate
  - 0x00547CF0 TMP_TileBlitter — variant chain walk
  - 0x00545150 Read_Theater_TileSets_INI — theater load + variant chain construction
  - 0x00480350 CellOverlay_TileDraw — render-time variant pick
  - 0x0047BDB0 GetRadarPixelColor — minimap path with same pick
- **INI keys:** None (G4 is binary-flag + asset-file driven)
- **Related code:**
  - [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) — owner of `damaged_variant` field and writer/clear hooks
  - [src/sim/bridge_specs.rs](../../src/sim/bridge_specs.rs) — `update_ramp_perpendicular` hook site
  - [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) — `has_damaged_data` propagation source
  - [src/map/terrain.rs](../../src/map/terrain.rs) — render-side variant override
  - [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) — render pipeline integration
  - [src/assets/tmp_decode.rs](../../src/assets/tmp_decode.rs), [src/assets/tmp_file.rs](../../src/assets/tmp_file.rs) — TMP per-cell flag parsing (already in place)
  - [src/map/theater.rs](../../src/map/theater.rs):660-703 — variant TMP asset loading (already in place)
- **Prior commits (recent, no conflicts with this plan):**
  - `687fa77 sim/bridge: docstring sweep for capture_target overload + cargo fmt`
  - `30b45b1 sim/world: integration tests for engineer bridge repair (+ ignored C4 hook test)`
  - `447e959 sim/world/bridge_orchestrator: add dispatch_bridge_collapse_from_hut with cascade`
  - `80bab9c sim/bridge_state: unit tests for body_cell_repair_state (9 cases incl. iteration-order pin)`
  - `5285e5b sim/bridge_state: add body_cell_repair_state reverse state machine`
