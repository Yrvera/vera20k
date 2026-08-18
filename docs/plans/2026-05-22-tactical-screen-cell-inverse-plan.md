# Tactical Screen-To-Cell Inverse Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not write Rust code until this plan is approved for implementation.

**Goal:** Replace Rust's tactical cursor-to-cell inverse approximation with the verified YR-shaped vertical scan and cardinal bridge branch, while keeping forward projection/render helpers stable.

**Architecture:** This is an app/map input conversion change. `map::terrain` owns pure tactical inverse math, `map::resolved_terrain` exposes bridge metadata needed by that math, and `app_sim_tick` remains the central screen/world-to-cell wrapper. `sim/` must not be changed for this plan.

**Design Doc:** `docs/plans/2026-05-22-tactical-screen-cell-inverse-design.md`

---

## Grounding Summary

Primary sources:

- `docs/research/TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_RECHECK_GHIDRA_REPORT.md`
- `docs/research/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_AXIS_AND_CARDINAL_POLARITY_GHIDRA_REPORT.md`
- `docs/research/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`
- `docs/research/COORDINATE_SYSTEM_GAMEMD.md`

Verified binary facts used by this plan:

- Tactical inverse `0x006D6590` is active in YR and feeds tactical picking, cursor/cell wrappers, radar update, and display paths.
- The binary computes an initial fallback cell before scanning and returns it if the scan cap is exhausted.
- Height inverse is a vertical pixel scan, not Rust's current 3-pass convergence loop.
- Scan starts at `(input_y - viewport_offset_y) + Tactical+0xB4`.
- Each failed attempt decrements by exactly one screen pixel.
- The cap compare uses `0xB4`, for an effective 180 failed attempts.
- Cell height uses the terrain height byte equivalent to `CellClass+0x11B`, multiplied by 15 screen pixels.
- Bridge logic runs only for structural bridge cells where `CellClass+0x140 & 0x100` is set.
- Bridge neighbor checks are cardinal and direction-id specific: `2`, `4`, plus either `0` or `6` depending on the `0x800` orientation bit.
- Direction ids are `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW`; `8` is a tube special and must not be treated as a normal neighbor.
- Bridge edge comparison is strict `> 15`, not `>= 15`.
- Extra bridge height adjustment is 60 screen pixels and is gated by the verified dir0/dir6 edge branch, not by a closest-cell search.

Current Rust mismatch:

- `src/map/terrain.rs::screen_to_iso_with_height_and_bridges` uses a 3-pass height refinement.
- The bridge branch searches a 7x7 neighborhood by nearest projected bridge cell.
- The existing `bridge_height_map` only carries deck height; it cannot express structural body flag or orientation bit.
- `app_sim_tick::world_point_to_cell` clamps negative output to zero without making the sentinel/fallback boundary explicit.

---

## Key Technical Decisions

- Add a tactical-only inverse API instead of changing `iso_to_screen`, `lepton_to_screen`, or the basic `screen_to_iso` helper. **Confidence:** high.
  - Those helpers are shared by rendering, overlays, minimap/debug views, and existing tests.

- Preserve `bridge_height_map: BTreeMap<(u16, u16), u8>` for current render/debug/entity-height users. **Confidence:** high.
  - It is used outside tactical picking and remains a useful deck-height view.

- Add a richer inverse-only bridge map derived from `ResolvedTerrainGrid`. **Confidence:** high.
  - Tactical inverse needs deck height, structural flag, and orientation bit. Encoding those into `u8` deck height would hide parity facts and make tests weak.

- Define tactical bridge input types in `map::terrain` and build them from `map::resolved_terrain`. **Confidence:** medium-high.
  - `terrain` owns the inverse algorithm; `resolved_terrain` owns the authoritative per-cell bridge facts.

- Keep `app_sim_tick::world_point_to_cell` as the only app-level float-to-`u16` conversion point. **Confidence:** high.
  - Downstream UI/order/entity-pick callers already converge there.

- Do not refactor building coordinate helpers, foundation projection, sidebar layout, radar rendering, or entity selection as part of this plan. **Confidence:** high.
  - Those are related consumers, not the verified mismatch itself.

Low-confidence boundary:

- The exact Rust coordinate mapping for `g_RadarViewportOffsetX/Y` must be confirmed while wiring `screen_point_to_world_cell`. Current Rust converts `screen / zoom + camera` before inverse, so the initial tactical viewport offset may be zero in the world-space path. Do not guess if a sidebar/radar-local caller is found passing pre-offset coordinates.

---

## Open Questions

### Resolved During Planning

- **Should this fix include the RED weapon-charge or refinery docs?** No. They are separate parity areas.
- **Should the current 7x7 bridge search be tuned instead of replaced?** No. The verified binary uses cardinal checks and structural/orientation flags.
- **Should the existing `bridge_height_map` be replaced everywhere?** No. Add richer metadata only where tactical inverse needs it.
- **Should direction helpers mask `direction & 7` for this branch?** No. Tactical inverse should use explicit constants and reject/ignore direction 8.

### Deferred

- Exact radar-local caller parity beyond preserving the explicit viewport-offset parameter.
- Full sentinel object modeling for off-map/null cells. Current app callers may still clamp after receiving the tactical inverse result, but that clamp must be isolated and tested.
- Pixel capture comparison against retail YR. This plan adds deterministic unit coverage first; live visual comparison can follow if cursor drift persists.

---

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/map/terrain.rs` | Add tactical inverse context/types, vertical scan, cardinal bridge branch, and focused unit tests. |
| Modify | `src/map/resolved_terrain.rs` | Build inverse-only bridge metadata from existing resolved bridge facts. |
| Modify | `src/app.rs` | Add app state field for tactical inverse bridge metadata. |
| Modify | `src/app_init.rs` | Build and return the tactical inverse bridge map when loading maps. |
| Modify | `src/app_transitions.rs` | Carry the new map-load result field into `AppState`. |
| Modify | `src/app_sim_tick.rs` | Route `world_point_to_cell` through the new tactical inverse API and update wrapper tests. |
| Optional modify | `src/app_context_order.rs`, `src/app_cursor.rs`, `src/app_entity_pick.rs`, `src/app_ui_overlays.rs`, `src/app_sidebar_render.rs` | Only if their direct `world_point_to_cell` calls need signature updates. Preserve caller ownership. |
| Read only | `src/map/bridge_facts.rs` | Reuse existing bridge flag constants/semantics where practical. |
| Read only | research docs under `docs/research/` | Source of verified parity details. |

No new source file is required. If `terrain.rs` becomes too broad, split only after the algorithm and tests are stable.

---

## Interface Changes

Expected additions in `src/map/terrain.rs`:

```rust
pub struct TacticalInverseContext<'a> {
    pub height_map: &'a BTreeMap<(u16, u16), u8>,
    pub bridge_cells: Option<&'a BTreeMap<(u16, u16), TacticalBridgeCell>>,
    pub viewport_offset_x: f32,
    pub viewport_offset_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TacticalBridgeCell {
    pub deck_z: u8,
    pub structural: bool,
    pub direction_zero: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TacticalInverseResult {
    Cell { rx: f32, ry: f32 },
    Fallback { rx: f32, ry: f32 },
}

pub fn screen_to_cell_tactical_inverse(
    screen_x: f32,
    screen_y: f32,
    context: TacticalInverseContext<'_>,
) -> TacticalInverseResult
```

Names may change to match local style, but the contract must preserve:

- explicit height map,
- optional structural/oriented bridge metadata,
- explicit viewport offset,
- a distinguishable fallback path or equivalent test-covered behavior.

Expected additions in `src/map/resolved_terrain.rs`:

```rust
pub fn build_tactical_bridge_inverse_map(
    &self,
) -> BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>
```

Expected app state additions:

```rust
pub(crate) tactical_bridge_inverse_map:
    BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>
```

Keep existing signatures temporarily only where needed for migration. Remove or de-emphasize `screen_to_iso_with_height_and_bridges` for tactical input once all callers are moved.

---

## App/Map Checklist

- [ ] No changes to `sim/` dependencies or deterministic tick state.
- [ ] No forward projection helper rewrites.
- [ ] `bridge_height_map` remains available for render/debug/height consumers.
- [ ] Tactical bridge metadata is derived from resolved terrain facts, not guessed from deck height.
- [ ] Direction constants are named and explicit.
- [ ] Direction `8` is not accepted as a normal cardinal neighbor.
- [ ] 180 scan attempts, 15 px height step, 60 px bridge adjustment, and strict `> 15` threshold are named constants.
- [ ] Negative/off-map conversion is handled in `app_sim_tick`, not hidden in low-level terrain math.
- [ ] Tests cover behavior that would fail under the old 3-pass and 7x7 algorithms.

---

## Risk Areas

- Accidentally preserving the old 3-pass loop under a renamed helper.
- Using the existing `bridge_height_map` as a proxy for structural bridge evidence.
- Treating `0x800` as a generic bridge-present flag instead of an orientation bit.
- Letting a generic `direction & 7` helper make direction `8` look valid.
- Off-by-one behavior at the strict `> 15` threshold.
- Accidentally converting fallback into a normal cell too early, hiding scan-cap behavior.
- Breaking non-tactical callers that used `world_point_to_cell` only because it was the convenient projection helper.
- Applying viewport offset twice after `screen / zoom + camera`.

---

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 1 | Rich bridge metadata | Structural/orientation branch cannot be represented by deck height alone | Resolved-terrain builder tests |
| 2 | Initial fallback | Binary returns fallback on scan cap | Unit test where scan never accepts |
| 3 | 180 one-pixel scan | Replaces current 3-pass approximation | Unit test requiring more than 3 failed pixels |
| 4 | Height step 15 | Terrain z changes tactical pick boundary | Elevated-cell tests |
| 5 | Structural bridge gate | Prevents non-structural deck cells from using bridge branch | Bridge metadata tests |
| 6 | Cardinal bridge checks | Prevents 7x7 nearest-cell drift | Synthetic bridge layout test |
| 7 | Direction-zero orientation | Chooses N/S vs E/W edge behavior | Tests for `direction_zero = true/false` |
| 8 | Strict `> 15` | Pixel-exact edge threshold | Tests at exactly 15 and 16 px |
| 9 | 60 px adjustment | Matches high bridge click band | Bridge branch tests |
| 10 | App wrapper ownership | Keeps all command paths using one inverse | `world_point_to_cell` and `screen_point_to_world_cell` tests |

---

## Tasks

### Task 1: Add Tactical Bridge Metadata Builder

**Why:** The tactical inverse branch needs structural/orientation bridge facts that are not present in `bridge_height_map`.

**Files:**

- Modify: `src/map/terrain.rs`
- Modify: `src/map/resolved_terrain.rs`

**Steps:**

1. Add `TacticalBridgeCell` to `terrain.rs`.
2. Include `deck_z`, `structural`, and `direction_zero` fields.
3. Add `ResolvedTerrainGrid::build_tactical_bridge_inverse_map`.
4. Populate only cells with high bridge deck data available from resolved terrain.
5. Set `deck_z` from `ResolvedTerrainCell.bridge_deck_level`.
6. Set `structural` from `BridgeCellFacts::has_structural_bridge()` or the same raw `0x100` meaning.
7. Set `direction_zero` from the raw `0x800` bridge flag.
8. Keep `build_bridge_height_map` unchanged.

**Tests:**

- Builder includes a known high bridge cell with deck height.
- Builder marks structural body cells.
- Builder preserves direction-zero orientation.
- Builder does not require changing existing `bridge_height_map` tests.

**Verify:**

```powershell
cargo test tactical_bridge_inverse --lib -- --nocapture
```

If the exact test filter is different after naming, run the narrow resolved-terrain and terrain test names added in this task.

### Task 2: Add Tactical Inverse Context And Constants

**Why:** The verified algorithm needs more inputs and stricter constants than the current helper signature exposes.

**Files:**

- Modify: `src/map/terrain.rs`

**Steps:**

1. Add `TacticalInverseContext`.
2. Add `TacticalInverseResult` or an equivalent result that preserves fallback visibility.
3. Add named constants:
   - `TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS: usize = 180`
   - `TACTICAL_HEIGHT_STEP_PX: f32 = 15.0`
   - `TACTICAL_BRIDGE_EDGE_THRESHOLD_PX: f32 = 15.0`
   - `TACTICAL_BRIDGE_EXTRA_HEIGHT_PX: f32 = 60.0`
   - `DIR_NORTH: u8 = 0`
   - `DIR_EAST: u8 = 2`
   - `DIR_SOUTH: u8 = 4`
   - `DIR_WEST: u8 = 6`
4. Add `screen_to_cell_tactical_inverse` as a new function.
5. Leave `screen_to_iso_with_height_and_bridges` in place until app callers are migrated.

**Tests:**

- Basic flat terrain result matches current z=0 projection for simple anchor points.
- Fallback status can be observed in a synthetic no-acceptance case.

**Verify:**

```powershell
cargo test screen_to_cell_tactical_inverse --lib -- --nocapture
```

### Task 3: Implement Vertical Height Scan

**Why:** This is the main player-visible correction for cursor/cell selection on slopes and elevated terrain.

**Files:**

- Modify: `src/map/terrain.rs`

**Steps:**

1. Compute the initial fallback cell before the scan using the same base projection convention as current flat inverse.
2. Convert input pixels by subtracting `context.viewport_offset_x/y` before scan math.
3. Start scan Y at adjusted input Y plus 180 pixels.
4. For each failed attempt:
   - project `(adjusted_x, scan_y)` to candidate cell,
   - read terrain height for the rounded candidate cell,
   - apply `height * 15` screen-pixel correction,
   - return the candidate when the corrected condition matches the verified binary threshold,
   - decrement `scan_y` by exactly 1 pixel.
5. Return `Fallback` after 180 failed attempts.
6. Keep off-map height lookup non-panicking; missing cells use zero height unless the app wrapper later models sentinel cells.

**Tests:**

- A synthetic elevated terrain case that fails under a 3-pass-only solver but succeeds under a longer vertical scan.
- A scan-cap case returns the initial fallback.
- A case verifies one-pixel decrement behavior by putting the acceptance boundary at a specific scan line.
- Existing `screen_to_iso_with_height` flat/elevated tests either still pass or are replaced with equivalent tactical inverse tests.

**Verify:**

```powershell
cargo test screen_to_iso_with_height --lib -- --nocapture
cargo test screen_to_cell_tactical_inverse --lib -- --nocapture
```

### Task 4: Implement Cardinal Bridge Branch

**Why:** Bridge cursor selection is currently approximate and can pick the wrong deck/edge because Rust searches a 7x7 neighborhood.

**Files:**

- Modify: `src/map/terrain.rs`

**Steps:**

1. Add small local cardinal offset helper that accepts only `0`, `2`, `4`, and `6`.
2. Reject or ignore direction `8`; do not mask with `& 7`.
3. During scan, if the candidate bridge cell is structural, run the bridge branch before ordinary terrain acceptance.
4. Check E (`2`) and S (`4`) candidates.
5. Check N (`0`) when `direction_zero` is set.
6. Check W (`6`) when `direction_zero` is clear.
7. Preserve the verified direct-return behavior for `+Y` and `+X` candidates.
8. Apply the 60-pixel extra bridge adjustment only through the verified dir0/dir6 edge path.
9. Use strict `> TACTICAL_BRIDGE_EDGE_THRESHOLD_PX`; exactly 15 pixels must not pass.
10. Remove the old 7x7 closest-bridge search from tactical input once tests cover the new branch.

**Tests:**

- Bridge branch ignores a closer diagonal bridge cell that the old 7x7 search would have chosen.
- Direction-zero true uses the N/S branch behavior.
- Direction-zero false uses the E/W branch behavior.
- Exactly 15 px does not trigger the strict threshold; 16 px does.
- Direction `8` does not produce a tactical neighbor.
- Non-structural bridge metadata does not enter the bridge branch.

**Verify:**

```powershell
cargo test bridge --lib -- --nocapture
cargo test screen_to_cell_tactical_inverse --lib -- --nocapture
```

### Task 5: Wire Map Load And App State

**Why:** Runtime tactical input needs the richer bridge map built once at map load, alongside the existing deck-height map.

**Files:**

- Modify: `src/app.rs`
- Modify: `src/app_init.rs`
- Modify: `src/app_transitions.rs`

**Steps:**

1. Add `tactical_bridge_inverse_map` to `AppState`.
2. Add `tactical_bridge_inverse_map` to `MapLoadResult`.
3. Build it in `load_map` immediately after `bridge_height_map`.
4. Initialize it to an empty map in empty/default app-state constructors.
5. Copy it through map transition assignment.
6. Leave `bridge_height_map` intact for all existing users.

**Tests:**

- Existing app init tests, if any, compile without changing render/debug users.
- A small map-load smoke test can assert the field is present when a map has bridge facts, if local fixtures make that practical.

**Verify:**

```powershell
cargo check
```

### Task 6: Route `world_point_to_cell` Through Tactical Inverse

**Why:** Cursor hover, orders, building placement preview, target lines, and entity picking must converge on the verified inverse path.

**Files:**

- Modify: `src/app_sim_tick.rs`
- Optional modify: direct callers that pass `bridge_height_map` to `world_point_to_cell`

**Steps:**

1. Change `world_point_to_cell` to accept `Option<&BTreeMap<(u16, u16), TacticalBridgeCell>>`.
2. Build a `TacticalInverseContext` with `viewport_offset_x/y` explicitly set.
3. For the normal world-space path after `screen / zoom + camera`, start with zero viewport offset unless a caller is proven to pass tactical-local pixels.
4. Convert `TacticalInverseResult` to `(u16, u16)` in this wrapper.
5. Keep the current negative clamp only here, and comment that it is the Rust app safety contract pending full sentinel modeling.
6. Update `screen_point_to_world_cell` to pass `state.tactical_bridge_inverse_map`.
7. Update direct call sites in `app_context_order`, `app_cursor`, `app_entity_pick`, `app_ui_overlays`, and `app_sidebar_render` only as needed for the new signature.
8. Keep entity height/render helpers using `bridge_height_map`.

**Tests:**

- `world_point_to_cell_round_trips_ground_iso_anchor` still passes.
- Replace `world_point_to_cell_forwards_bridge_height_map` with a test proving the tactical bridge inverse map is forwarded.
- `world_point_to_cell_clamps_negative_results_to_zero` still passes, with the clamp isolated to the app wrapper.
- Add a viewport-offset test if a non-zero offset path is wired.

**Verify:**

```powershell
cargo test world_point_to_cell --lib -- --nocapture
```

### Task 7: Remove Tactical Use Of Old Approximation

**Why:** Leaving two tactical inverse paths makes future cursor parity bugs hard to diagnose.

**Files:**

- Modify: `src/map/terrain.rs`
- Modify: any remaining callers found by `rg`

**Steps:**

1. Run `rg -n "screen_to_iso_with_height_and_bridges|bridge_height_map" src`.
2. Confirm no tactical input path still calls the old 3-pass/7x7 function.
3. Keep old helper only if non-tactical tests or debug tools use it deliberately.
4. If kept, update its comment to state it is an approximate legacy helper and not the tactical YR parity path.
5. If no callers remain, remove the helper and update tests.

**Tests:**

- `rg` confirms tactical input callers use `screen_to_cell_tactical_inverse` or `world_point_to_cell`.
- Existing debug/render paths compile and continue using `bridge_height_map`.

**Verify:**

```powershell
rg -n "screen_to_iso_with_height_and_bridges|screen_to_cell_tactical_inverse|world_point_to_cell" src
cargo check
```

### Task 8: Focused Regression Pass

**Why:** The change affects player input selection, so compilation alone is not enough.

**Files:**

- No new files expected unless tests reveal a missing fixture.

**Steps:**

1. Run focused terrain inverse tests.
2. Run focused app wrapper tests.
3. Run broader library tests if focused tests pass.
4. Manually inspect the remaining `bridge_height_map` callers and verify none expected structural/orientation semantics.
5. If a dev map with high bridges is convenient, run the app and smoke-test cursor hover on bridge deck, bridge edge, elevated terrain, and map edge.

**Verify:**

```powershell
cargo test screen_to_cell_tactical_inverse --lib -- --nocapture
cargo test world_point_to_cell --lib -- --nocapture
cargo test --lib
```

Optional runtime smoke:

```powershell
cargo run
```

Acceptance:

- Tactical inverse no longer uses 3-pass convergence for cursor picking.
- Tactical bridge picking no longer uses 7x7 nearest-bridge search.
- Structural/orientation bridge metadata is represented explicitly.
- Existing render/debug bridge height behavior remains intact.
- Negative/off-map clamping is visible at the app wrapper boundary.
