# Bridge P2/P3 Repair Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Repair Priority 2 and Priority 3 from `2026-05-15-bridge-parity-fix-priority-list.md`: exact high-bridge ramp recognition and binary-shaped high-bridge traversal gates.

**Architecture:** Priority 2 is a `map/` data correction: theater tile keys and `cell+0x11A` identify ramp tiles, but do not stamp traversal facts by themselves. Priority 3 is a `sim/pathfinding` and `sim/movement` correction: traversal must consume map facts and path height without making `sim/` depend on render/UI/audio/net.

**Design Input:** No separate `*-design.md` exists. This plan treats these green/audited docs as the design/spec inputs:

- `docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`
- `docs/plans/2026-05-15-bridge-authoritative-cell-facts-plan.md`
- `docs/research/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`

---

## Grounding Summary

Priority 1 introduced or partially introduced `BridgeCellFacts`, `BridgeRampTile`, and `BridgeRampTileTable`. The current working tree is dirty, so implementers must read the current files before editing and must not revert unrelated miner or bridge changes.

The P2 source doc is green after the 2026-05-15 repair: `MapClass::IsBridgeRampTile` is a pure predicate using theater bridge keys and exact `cell+0x11A` values. It is not bridge fact stamping. The inspected high-edge caller prepares a **one-based BridgeSet-relative key**: `(IsoTileTypeIndex - BridgeSetStart) + 1`.

The current Rust ramp callsite in `src/map/resolved_terrain.rs` computes `tile_id.checked_sub(bridge_set_start)` and passes that zero-based value to `BridgeRampTileTable::match_relative_tile`. That is the first P2 repair target.

The P3 traversal source docs are green after the targeted headless Ghidra pass. `CheckBridgeTraversal @ 0x004D9C60` takes candidate cell, direction, mutable path height, mutable bridge-list byte, and optional parent/predecessor. Unit and Infantry forward `Can_Enter_Cell` arg4 into this fifth argument. A* passes an explicit current-node cell to `Can_Enter_Cell`; runtime probes often pass `0`, causing fallback reconstruction or candidate-only `direction == -1` behavior.

Current Rust pathfinding has bridge-related pieces in `src/sim/pathfinding/core.rs`: `is_at_bridge_level`, `compute_neighbor_height`, dual came-from arrays, and `PathCell` bridge booleans. This is not a faithful `CheckBridgeTraversal` equivalent yet. `src/sim/movement/movement_bridge.rs` drives runtime `on_bridge`, but it is not the A*/CanEnter traversal predicate and should not absorb pathfinding logic.

INI/theater grounding: ramp keys come from theater INI `[General]` keys already parsed in `src/map/theater.rs`: `BridgeSet`, `BridgeTopLeft1/2`, `BridgeTopRight1/2`, `BridgeMiddle1/2`. High-bridge overlay stamping remains driven by rules overlay IDs `0x18`, `0x19`, `0xED`, `0xEE` from Priority 1.

Still unknown after grounding: exact low-bridge tube semantics remain Priority 5 and must not be folded into this plan.

---

## Key Technical Decisions

- P2 must convert global `final_tile_index` into a one-based BridgeSet-relative key before matching ramp tiles. **Confidence: high.** Source: repaired map-load doc and `MapClass::UpdateBridgeEdgeTiles_High` Ghidra pass.
- Ramp recognition must populate `BridgeCellFacts.ramp_tile` or diagnostic metadata only; it must not create `0x100`/`0x200` traversal facts unless a binary-stamped fact already exists. **Confidence: high.** Source: `MapClass::IsBridgeRampTile` pure predicate.
- P3 should introduce a pure `CheckBridgeTraversal`-equivalent function in `sim/pathfinding`, not overload `movement_bridge.rs`. **Confidence: high.** Source: `CheckBridgeTraversal` is called from Unit/Infantry `Can_Enter_Cell` and A*, while `movement_bridge.rs` handles runtime `on_bridge`.
- The traversal function must keep three modes separate: explicit parent, null parent with valid direction reconstruction, and `direction == -1` candidate-only seed. **Confidence: high.** Source: parent-fallback report.
- The first implementation should wire P3 into A* explicit-parent expansion only. Runtime locomotor probes with parent `0` should get the same pure helper available, but broad movement rewiring can wait for a separate locomotor pass. **Confidence: medium.** Source: A* callsite is high-confidence; runtime locomotor decompilation is lower-confidence and broader.
- `PathCell` should expose raw bridge facts needed by traversal (`bridge_structural`, `bridge_transition`, `bridge_marker_0x80`, `slope_type`, signed `ground_level`) but not render-only fields. **Confidence: high.** Source: sim/render layering and `CheckBridgeTraversal` reads.

---

## Open Questions

### Resolved During Planning

- **Are both P2/P3 source docs green enough to plan from?** Yes. P2 is green for high-bridge ramp recognition. P3 is green after the headless Ghidra argument-binding pass.
- **Should broad bridgehead inference remain as a fallback?** No. It can remain only as non-authoritative diagnostics until proven; this plan removes it from traversal facts.
- **Should P3 use source/destination terminology?** No. Use `candidate` and `parent/predecessor` in code and tests to avoid reintroducing the earlier doc bug.

### Deferred to Implementation

- **Which current tests are encoding the earlier broad ramp/transition behavior?** Implementation must run the focused suites and update only expectations that conflict with binary facts.
- **How far should runtime movement be rewired after A*?** This plan creates the pure helper and uses it in A*. Runtime locomotor probes should be listed as follow-up unless a local compile/test change requires minimal integration.

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/map/theater.rs` | Convert raw tile IDs to one-based BridgeSet-relative ramp keys and expose exact ramp predicate helpers. |
| Modify | `src/map/theater_tests.rs` | Lock one-based key behavior and boundary conditions. |
| Modify | `src/map/resolved_terrain.rs` | Use exact ramp predicate without stamping broad bridgehead facts. |
| Modify | `src/map/bridge_facts.rs` | Store ramp predicate metadata without conflating it with `0x100`/`0x200`. |
| Modify | `src/sim/pathfinding/core.rs` | Add `CheckBridgeTraversal`-equivalent pure predicate and wire it into A* neighbor expansion. |
| Modify | `src/sim/pathfinding/core_tests.rs` | Cover diff 0/1/4, explicit parent, null-parent reconstruction, and `direction == -1`. |
| Modify | `src/sim/movement/movement_bridge.rs` | Only adjust comments/tests if names conflict; do not move traversal logic here. |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Only if needed to preserve `Can_Enter_Cell` return-code semantics in tests. |

---

## Interface Changes

- Add a theater helper that converts a global tile id to a one-based BridgeSet-relative key:
  `BridgeRampTileTable::match_tile_id(tile_id, bridge_set_start, bridge_set_count, height_byte)`.
- Add `PathCell` accessors or fields for traversal facts:
  `bridge_structural`, `transition`, `bridge_marker_0x80` if not already present, `ground_level`, `slope_type`.
- Add a pure traversal API in `src/sim/pathfinding/core.rs` or a small sibling module:

```rust
pub(crate) struct BridgeTraversalInput<'a> {
    pub candidate: &'a PathCell,
    pub direction: i8,
    pub path_height: i16,
    pub parent: Option<&'a PathCell>,
    pub candidate_coord: (u16, u16),
}

pub(crate) struct BridgeTraversalResult {
    pub allowed: bool,
    pub path_height: i16,
    pub force_bridge_list: bool,
}
```

The implementation may adjust names, but it must preserve candidate/parent terminology and mutable height/list-byte semantics.

---

## Sim Checklist

- [ ] No `f32`/`f64` introduced in sim logic.
- [ ] No new deterministic runtime state added outside path nodes/local A* state.
- [ ] No dependencies from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net`.
- [ ] Tick ordering unchanged; A* expansion semantics change only during path search.
- [ ] `BTreeMap` iteration order unaffected.

---

## Risk Areas

- P2 may break tests that currently expect BridgeSet tiles to imply bridgehead/transition. Those expectations should be rewritten to require exact key plus `0x11A`, and then only as ramp metadata.
- P3 touches central A* neighbor filtering. It can change route selection on non-bridge maps if the helper is applied too broadly. Gate the traversal helper to bridge-relevant candidates/parents or diff cases.
- Current `compute_neighbor_height` pre-shifts some bridge transitions before legality checks. The new traversal gate must not double-apply the deck offset.
- The P3 helper needs signed height behavior. Do not use unchecked `u8` subtraction for the diff logic.
- Runtime movement `on_bridge` tests are not a substitute for A* traversal tests. Keep those layers separate.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | One-based BridgeSet-relative ramp key | Zero-based matching shifts every ramp tile and creates wrong bridgeheads | Unit tests for raw tile `BridgeSetStart + key - 1` |
| 2 | Ramp recognition does not stamp `0x100`/`0x200` by itself | Broad bridgehead facts let units enter bridges from cells YR rejects | Resolved terrain tests prove ramp metadata can exist without structural flags |
| 4 | Explicit-parent `CheckBridgeTraversal` | A* bridge entry/exit depends on current-node cell and carried height | Unit tests for candidate/parent diff 0/1/4 cases |
| 5 | Null-parent reconstruction mode | Runtime lookahead probes reconstruct predecessor from candidate+direction, not current entity cell | Unit tests for `parent=None`, valid direction |
| 6 | `direction == -1` candidate-only mode | Jumpjet/hover landing probes skip directed bridgehead/diff/slope checks | Unit test seeds candidate deck height and returns allowed |
| 7 | Two-pass list/height distinction | Occupancy/list layer can diverge at bridgeheads | A* test asserts `force_bridge_list` can differ from final height layer |

---

## Tasks

### Task 1: Repair Ramp Key Conversion

**Why:** The current partial P2 code appears to pass zero-based `tile_id - BridgeSetStart` into `match_relative_tile`; Ghidra evidence requires one-based `(tile_id - BridgeSetStart) + 1`.

**Files:**
- Modify: `src/map/theater.rs`
- Modify: `src/map/theater_tests.rs`
- Modify: `src/map/resolved_terrain.rs`

**Pattern:** Follow existing `BridgeAnchorVariantTable::from_theater` bounds handling.

**Step 1: Add a one-based helper in `BridgeRampTileTable`**

Add a method:

```rust
pub fn match_tile_id(
    &self,
    tile_id: u16,
    bridge_set_start: u16,
    bridge_set_count: u16,
    height_byte: u8,
) -> Option<BridgeRampTile> {
    let zero_based = tile_id.checked_sub(bridge_set_start)?;
    if zero_based >= bridge_set_count {
        return None;
    }
    let one_based_key = zero_based + 1;
    self.match_relative_tile(one_based_key, height_byte)
}
```

Keep `match_relative_tile` as the low-level helper because tests and docs talk about theater keys.

**Step 2: Update resolved terrain callsite**

Replace the manual `checked_sub` and range check in `src/map/resolved_terrain.rs` with `match_tile_id`. The call must pass `tile_id`, `bridge_set_start`, `bridge_set_bounds.count`, and `cell.template_height`.

**Step 3: Add boundary tests**

In `src/map/theater_tests.rs`, add:

- `ramp_tile_match_tile_id_uses_one_based_bridge_key`
- `ramp_tile_match_tile_id_rejects_tile_before_bridge_set`
- `ramp_tile_match_tile_id_rejects_tile_at_bridge_set_end`

Use a synthetic `BridgeSetStart=100`, `BridgeTopRight1=4`; assert raw tile `103` matches and raw tile `104` does not match `TopRight1`.

**Step 4: Verify**

Run:

```powershell
cargo test theater_tests::ramp_tile -- --nocapture
```

Expected: all ramp table tests pass.

Do not commit unless explicitly asked.

### Task 2: Keep Ramp Recognition Separate From Bridge Fact Stamping

**Why:** Binary `IsBridgeRampTile` is recognition only. It does not create structural bridge flags.

**Files:**
- Modify: `src/map/resolved_terrain.rs`
- Modify: `src/map/bridge_facts.rs`
- Modify: resolved terrain tests in the same file or nearby test module

**Pattern:** Follow `BridgeCellFacts.ramp_tile: Option<BridgeRampTile>` already present in the current working tree.

**Step 1: Audit current writes**

Search for:

```powershell
rg -n "ramp_tile|bridge_transition = true|has_bridge_deck = true|BRIDGE_FLAG_TRANSITION" src/map/resolved_terrain.rs src/map/bridge_facts.rs
```

Identify any branch where ramp recognition alone sets `bridge_transition`, `has_bridge_deck`, `bridge_walkable`, or raw `0x100/0x200`.

**Step 2: Restrict ramp writes to metadata**

When `match_tile_id` returns a `BridgeRampTile`, set:

```rust
cell.bridge_facts.ramp_tile = Some(ramp_tile);
```

Do not set `bridge_transition` from this branch unless the cell already has stamped `BRIDGE_FLAG_TRANSITION`.

**Step 3: Preserve diagnostics**

If diagnostics need to know "this tile is a ramp-looking tile", read `bridge_facts.ramp_tile`. Do not overload `bridge_transition` for diagnostics.

**Step 4: Add tests**

Add resolved terrain tests:

- exact ramp tile stores `bridge_facts.ramp_tile`;
- exact ramp tile without stamped `0x200` does not set `bridge_transition`;
- stamped transition remains `bridge_transition` even when `ramp_tile` is absent;
- broad BridgeSet tile with wrong `0x11A` sets neither ramp metadata nor transition.

**Step 5: Verify**

Run:

```powershell
cargo test resolved_terrain bridge_ramp -- --nocapture
```

Expected: ramp metadata and transition facts are distinct.

Do not commit unless explicitly asked.

### Task 3: Extend PathCell With Explicit Binary Bridge Facts

**Why:** P3 cannot faithfully model `CheckBridgeTraversal` if `PathCell` only exposes `bridge_walkable` and `transition` with old inferred meanings.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify: tests/fixtures that construct `PathCell`

**Pattern:** Keep `PathCell` simple and copy-only; it is the existing pathfinding cell DTO.

**Step 1: Ensure fields exist**

Keep or add these fields:

```rust
pub bridge_structural: bool,   // cell flags & 0x100
pub transition: bool,          // cell flags & 0x200
pub bridge_marker_0x80: bool,  // cell flags & 0x80, if needed for diagnostics only
pub ground_level: u8,
pub slope_type: u8,
```

If `bridge_structural` already exists, verify it is populated from `BridgeCellFacts::has_structural_bridge`, not from `bridge_walkable && !transition`.

**Step 2: Add accessors**

Add:

```rust
pub fn has_structural_bridge(&self) -> bool { self.bridge_structural }
pub fn has_bridgehead_transition(&self) -> bool { self.transition }
pub fn signed_level(&self) -> i16 { self.ground_level as i8 as i16 }
```

**Step 3: Update PathGrid construction**

In the `PathGrid::from_resolved_terrain` conversion, populate:

- `bridge_structural` from `cell.bridge_facts.has_structural_bridge()` when available;
- `transition` from `cell.bridge_facts.has_transition_flag()` when available;
- `bridge_walkable` from structural + runtime bridge intactness, not from ramp metadata.

**Step 4: Update fixtures**

Update local `PathCell` constructors in tests to set both `bridge_walkable` and `bridge_structural` explicitly. Do not let helper defaults hide the distinction.

**Step 5: Verify**

Run:

```powershell
cargo test pathfinding::core -- --nocapture
```

Expected: compile succeeds and existing tests still pass or fail only on known bridge expectations.

Do not commit unless explicitly asked.

### Task 4: Add Pure CheckBridgeTraversal Equivalent

**Why:** A* and later runtime probes need the same binary-shaped predicate instead of ad hoc bridge height rules.

**Files:**
- Modify: `src/sim/pathfinding/core.rs` or create `src/sim/pathfinding/bridge_traversal.rs`
- Modify: `src/sim/pathfinding/mod.rs` if a new module is created
- Modify: `src/sim/pathfinding/core_tests.rs`

**Pattern:** Use pure helper functions near existing A* helpers. No allocation, no floating point.

**Step 1: Define input/result types**

Use candidate/parent terminology:

```rust
pub(crate) struct BridgeTraversalInput<'a> {
    pub candidate: &'a PathCell,
    pub candidate_coord: (u16, u16),
    pub direction: i8,
    pub path_height: i16,
    pub parent: Option<(&'a PathCell, (u16, u16))>,
}

pub(crate) struct BridgeTraversalResult {
    pub allowed: bool,
    pub path_height: i16,
    pub force_bridge_list: bool,
}
```

If creating a separate module, keep it under `sim/pathfinding` and import only `PathCell`.

**Step 2: Implement parent resolution**

Implement:

```rust
fn resolve_parent_for_bridge_traversal<'a>(
    grid: &'a PathGrid,
    candidate_coord: (u16, u16),
    direction: i8,
    explicit_parent: Option<(&'a PathCell, (u16, u16))>,
) -> Option<(&'a PathCell, (u16, u16))>
```

Rules:

- explicit parent returns unchanged;
- `direction == -1` may still compute fallback in the binary, but the directed branch ignores it, so the helper may skip lookup and document that choice;
- valid directions `0..=7` reconstruct `(direction - 4) & 7` from candidate coordinates;
- direction values outside `-1` and `0..=7` return no parent and should block directed traversal unless an explicit parent was supplied.

**Step 3: Implement direction `-1` branch**

If `direction == -1`:

- if `path_height == -1` and candidate has `0x100`, set `path_height = candidate.level + 4`;
- return allowed;
- do not require `0x200`;
- do not check slope or diff cases.

**Step 4: Implement directed `path_height == -1` seed**

For non-`-1` direction with parent:

- if `path_height == -1` and parent has `0x100`, set `path_height = parent.level + 4`;
- if that seed happened and candidate lacks `0x200`, return blocked.

**Step 5: Implement diff cases**

Use signed levels:

```rust
candidate_level = candidate.signed_level();
selected = if parent.has_structural_bridge() {
    parent.signed_level()
} else {
    path_height
};
diff = selected - candidate_level;
```

Rules:

- `abs(diff) == 0`: if not all `{candidate.0x100, candidate.0x200, parent.0x100}` and `path_height != -1 && path_height != candidate_level`, block.
- `abs(diff) == 1`: if `diff < 1`, require parent slope nonzero; else require candidate slope nonzero.
- `abs(diff) == 4`: if `parent.level == candidate.level - 4`, require `path_height == candidate.level` and parent `0x100`; if `candidate.level == parent.level - 4`, require candidate `0x100` and `0x200`, set `force_bridge_list = true`, return allowed.
- all other diffs block.

**Step 6: Add unit tests**

Add focused tests:

- `direction_minus_one_seeds_candidate_bridge_height_without_bridgehead`;
- `explicit_parent_unknown_height_seeds_from_parent_and_requires_candidate_transition`;
- `diff_zero_blocks_when_path_height_disagrees`;
- `diff_one_lower_parent_requires_parent_slope`;
- `diff_one_higher_parent_requires_candidate_slope`;
- `diff_four_candidate_low_parent_high_forces_bridge_list`;
- `diff_four_candidate_high_parent_low_requires_parent_structural`;
- `invalid_diff_blocks`;
- `null_parent_valid_direction_reconstructs_predecessor`.

**Step 7: Verify**

Run:

```powershell
cargo test bridge_traversal -- --nocapture
```

Expected: traversal unit tests pass.

Do not commit unless explicitly asked.

### Task 5: Wire Traversal Into A* Neighbor Expansion

**Why:** Priority 3 is only useful once A* uses the binary-shaped bridge gate during neighbor expansion.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify: `src/sim/pathfinding/core_tests.rs`

**Pattern:** A* already has current node, current cell, neighbor/candidate cell, direction, and current path height in scope.

**Step 1: Call the pure traversal helper before walkability filtering**

For each neighbor:

- candidate = `neighbor_cell`;
- parent = `cur_cell`;
- direction = `dir_index as i8`;
- path_height = `current.height as i16`.

Call the helper and `continue` if blocked.

**Step 2: Use returned path height deliberately**

Use `BridgeTraversalResult.path_height` only for legality/list decision where it maps to the binary. Keep `AStar_create_node`-equivalent `compute_neighbor_height` as the node-height producer unless the new tests prove it must be replaced in the same task.

Do not double-add `+4`.

**Step 3: Map `force_bridge_list` to closed-list selection**

If `force_bridge_list` is true, the neighbor should use bridge closed/list behavior even if the previous heuristic would choose ground.

Document the exact polarity: binary Unit/Infantry list byte `1` means bridge list; A* local ground byte has opposite polarity in older docs. Keep code names explicit.

**Step 4: Preserve terrain and entity checks**

Do not remove:

- terrain cost checks;
- entity block checks;
- diagonal corner checks;
- zone corridor filters.

Only insert the traversal gate and list-layer correction.

**Step 5: Add A* integration tests**

Add tests:

- bridge entry from candidate transition passes when parent/current path height matches binary conditions;
- bridge entry from non-transition candidate is rejected;
- one-level ramp without slope is rejected;
- `force_bridge_list` path stores came-from in bridge array;
- non-bridge flat path remains unchanged.

**Step 6: Verify**

Run:

```powershell
cargo test pathfinding::core bridge -- --nocapture
cargo test pathfinding::core -- --nocapture
```

Expected: bridge traversal integration tests pass; non-bridge A* tests remain unchanged.

Do not commit unless explicitly asked.

### Task 6: Keep Movement Bridge State Layered, Do Not Fold It Into A*

**Why:** `movement_bridge.rs` controls runtime `on_bridge` state, not the A* CanEnter predicate. The earlier implementation risk was mixing these layers.

**Files:**
- Modify: `src/sim/movement/movement_bridge.rs` only if comments/tests need terminology updates
- Modify: movement tests only if expectations conflict with corrected facts

**Pattern:** Preserve current separation: A* path layer drives `loco.layer`; bridge transition predicate drives `on_bridge` and `BridgeOccupancy`.

**Step 1: Audit for old source/dest terminology**

Search:

```powershell
rg -n "source|destination|bridgehead|transition|CheckBridgeTraversal" src/sim/movement/movement_bridge.rs src/sim/movement
```

Update comments that imply `movement_bridge.rs` is a port of full `CheckBridgeTraversal`.

**Step 2: Do not import pathfinding traversal into movement_bridge.rs**

If movement runtime needs the pure helper later, add that in a locomotor-specific plan. This task only prevents concept drift.

**Step 3: Verify existing on_bridge tests**

Run:

```powershell
cargo test movement_bridge -- --nocapture
cargo test on_bridge -- --nocapture
```

Expected: tests still describe `on_bridge`, not A* candidate/parent traversal.

Do not commit unless explicitly asked.

### Task 7: Repair Or Retire Tests That Encode Broad Inference

**Why:** Some tests may have been written against the old model. Keep tests that assert player-visible behavior, but update tests that assert broad inference as if it were gamemd behavior.

**Files:**
- Modify: `src/map/theater_tests.rs`
- Modify: `src/map/resolved_terrain.rs` tests
- Modify: `src/sim/pathfinding/core_tests.rs`
- Modify: `src/sim/pathfinding/zone_map_tests.rs` only if failures directly relate to bridge facts

**Pattern:** Replace broad expectations with explicit binary facts from the green docs.

**Step 1: Run focused failures**

Run:

```powershell
cargo test bridge -- --nocapture
cargo test ramp -- --nocapture
cargo test pathfinding -- --nocapture
```

**Step 2: Classify each failure**

For every failure, classify it as:

- P2 exact-ramp key correction;
- P2 ramp-vs-transition separation;
- P3 traversal gate correction;
- unrelated dirty-worktree failure.

**Step 3: Update only bridge-parity expectations**

Do not weaken tests to "whatever passes." Each changed expected value must cite one of:

- one-based BridgeSet key;
- `0x11A` exact height byte;
- candidate/parent `CheckBridgeTraversal` case;
- no side-cell expansion / no gap fill / no deck normalization.

**Step 4: Verify**

Run the same focused commands again.

Expected: bridge/ramp/pathfinding failures either pass or are documented as unrelated dirty-worktree failures.

Do not commit unless explicitly asked.

### Task 8: Full Verification

**Why:** P2/P3 affect map data and central route selection.

**Files:** No planned edits except missed imports or test fixtures.

**Step 1: Format**

```powershell
cargo fmt
```

**Step 2: Focused suites**

```powershell
cargo test bridge_facts -- --nocapture
cargo test theater_tests::ramp -- --nocapture
cargo test resolved_terrain bridge -- --nocapture
cargo test pathfinding::core bridge -- --nocapture
cargo test movement_bridge -- --nocapture
```

**Step 3: Broader suite**

```powershell
cargo test
```

Expected: pass, except unrelated pre-existing failures from the dirty worktree. If failures remain, list exact test names and whether they depend on edited files.

**Step 4: Review diff**

Run:

```powershell
git diff -- src/map src/sim/pathfinding src/sim/movement
```

Expected:

- no broad BridgeSet bridgehead inference remains authoritative;
- no new side expansion, deck normalization, or gap fill;
- P3 code uses candidate/parent naming;
- no sim dependency on render/UI/audio/net.

Do not commit unless explicitly asked.

---

## Sources & References

- **Priority list:** `docs/plans/2026-05-15-bridge-parity-fix-priority-list.md`
- **Priority 1 plan:** `docs/plans/2026-05-15-bridge-authoritative-cell-facts-plan.md`
- **P2 source doc:** `docs/research/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- **P3 source doc:** `docs/research/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- **Parent fallback doc:** `docs/research/BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
- **Two-pass doc:** `docs/research/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`
- **Callsite matrix:** `docs/research/BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
- **Low bridge supplement:** `docs/research/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- **gamemd.exe addresses:** `MapClass::IsBridgeRampTile`, `MapClass::UpdateBridgeEdgeTiles_High`, `CheckBridgeTraversal @ 0x004D9C60`, `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `InfantryClass::Can_Enter_Cell @ 0x0051BF90`, `AStar_main_loop @ 0x00429A90`, `AStar_create_node @ 0x0042A460`
- **Theater INI keys:** `[General] BridgeSet`, `BridgeTopLeft1`, `BridgeTopLeft2`, `BridgeTopRight1`, `BridgeTopRight2`, `BridgeMiddle1`, `BridgeMiddle2`
- **Current Rust:** `src/map/bridge_facts.rs`, `src/map/theater.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_bridge.rs`

---

## Review Notes

Run `/review-plan` before implementation. The review should be especially critical on:

- Whether Task 2 should completely stop setting `bridge_transition` from ramp recognition, or preserve a compatibility field behind a different name until P3 lands.
- Whether Task 5 should replace `compute_neighbor_height` in the same patch or only gate around it.
- Whether runtime locomotor parent-`0` probes should be wired immediately after A*, or split into a separate Priority 3b plan.
- Whether current dirty Rust changes already implement parts of this plan and need repair rather than duplicate work.
