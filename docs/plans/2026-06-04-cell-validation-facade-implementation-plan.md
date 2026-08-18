# Cell-Validation Facade (Slice 2) — Implementation Plan

**Status:** IMPLEMENTATION PLAN (doc-only — no `src/` touched by this run). Tasks below CONTAIN proposed Rust; they are not applied.
**Date:** 2026-06-04
**Rule:** Rust-native structure, gamemd-native semantics. Default-to-DRIFT; no internal-only escape hatch for active cell-validation behavior.
**Design spec:** `docs/plans/2026-06-04-cell-validation-facade-design.md` (GREEN, design-review applied).
**Contract source:** `docs/research/CELL_VALIDATION_ENGINE_SUBSTRATE_SERVICE_STUDY.md` (PASS-2 binary-verified; cited inline as STUDY Cn / Ln).
**Slots into:** master roadmap item **#7 (map/cell substrate)** in `docs/plans/2026-05-29-core-engine-substrate-todo.md`.

---

## Plan-review corrections (2026-06-04, /review-plan)

**Verdict: YELLOW (corrections applied, ready).** Every codebase line-anchor the plan asserts was read this run and is CORRECT (see table below). One DOC-vs-binary gap was added to T0; two doc imprecisions fixed.

**Verified CORRECT this run (read, not assumed):**
- `cell_rect.rs` (617 lines): `CellRect`@22/29, `CellReservationGrid`@44, `CellRectPassabilityContext`@84, `CellRectOccupancyContext`@99, `check_passability_rect`@110, `check_occupancy_rect`@131, fused `+0x4C`/`+0x11C` block@158-166, `check_cell_passability`@188, `speed_type_allows_cell`@271, `rect_in_playfield`@321, `reservation_mask`@337, `to_cell_coord`@345, and all four existing tests — ALL exactly as cited. `pub mod cell_rect;` @ `mod.rs:69` CORRECT.
- `production_spawn.rs`: `find_spawn_cell_near_structure`@237, `nearest_walkable_around` CALL@290 (radius 12), DEF@355, `spawn_fallback_candidate_passable`@506 (already calls both facade predicates), `cell_available_for_spawn`@553, `spawn_cell_passable`@585, `find_spawn_selection_for_owner_with_type`@68 (`sim: &mut Simulation`), inner call site@157 — ALL CORRECT.
- `snapshot.rs:24` `SNAPSHOT_VERSION = 17` CORRECT (17→18 valid). `binary_frame` field @ `world/mod.rs:302` with documented late-commit "read as current frame N" semantics; commit @ `world/mod.rs:1742` `((self.total_sim_ms * 15) / 1000) as u32` — CORRECT, off-by-one risk is real and correctly described.
- Types: `PathGrid::cell`@`core.rs:1642` (`y*width+x`, None OOB), `ResolvedTerrainGrid::cell`@`resolved_terrain.rs:363`, `ZoneGrid::map_for`@`zone_map.rs:266`, `zone_at`@`zone_map.rs:86`, `ZoneId=u16`@`zone_map.rs:28`, `OverlayGrid`/`OccupancyGrid`/`EntityStore` module paths — ALL CORRECT.
- Binary: STUDY corroborates `CheckOccupancy 0x00586780` (live), `IsRectInPlayfield 0x00578390` (DOC-ONLY, call-confirmed), `FNPC 0x0056DC20` identity (live), C16 frame-counter selection PASS-2 VERIFIED live (`g_CurrentFrameCounter 0x00A8ED84`). The hash-flip rests on a VERIFIED selection rule. (Addresses not independently re-decompiled this run — sourced from the cited STUDY, which carries inline Ghidra-call citations.)

**Corrections applied:**
1. **T0 — added gate #4: C15 diamond-ring ENUMERATION was DOC-ONLY** (STUDY line 255), load-bearing for the T7 hash. **NOW CLOSED** (`docs/research/GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md`, `decompile_function`/`disassemble_function 0x0056DC20`): ring shape/order/cap/early-out/direct-split all pinned. T7 is unblocked; no user sign-off gate remains. The reconcile is now three CONCRETE corrections to the Rust shadow (see T4) — ring visit order, direct/indirect identity, per-ring early-out.
2. **T8 — harness path fixed:** it is `world/global_parity_harness_tests.rs` declared at `world/mod.rs:2476-2477` (uses `ReplayLog`/`ReplayRunner`, `GLOBAL_HARNESS_FINAL_HASH`@:40), NOT `src/sim/mod.rs` as drafted.

**Residual risk (UPDATED):** RESOLVED. All three gates that fed the hashed cell choice or its premises are CLOSED — #1 (playfield corner formula), #3 (Track-over-Clear passability), #4 (FNPC ring enumeration). The only remaining open gate is #2 (dummy `0x00ABDC50` field values), which is non-blocking (gates T1 only on the never-yet-found human-path dummy-field reader). Everything load-bearing for the T7 hash is binary-verified.

---

## CRITICAL PLAN-TIME CORRECTION TO THE DESIGN (read first)

The design spec proposes a **NEW** module `src/sim/cell_validation/` containing fresh `check_passability_rect` / `check_occupancy_rect` / `check_cell_passability` / `Reservation` / playfield-corner / reservation-mask code. **That code already exists in the tree.** The "first slice" this design says it *extends* landed as **`src/sim/cell_rect.rs`** (declared `pub mod cell_rect;` at `src/sim/mod.rs:69`, verified this run), and it already contains:

| Already-present in `src/sim/cell_rect.rs` (verified this run) | Line |
|---|---|
| `pub struct CellRect { x,y,width,height: i32 }` + `CellRect::new` / `CellRect::single` | `:22`, `:29` |
| `pub struct CellReservationGrid` (`1<<(arg&0x1F)` mask map) | `:44` |
| `pub struct CellRectPassabilityContext<'a>` (9-arg config) | `:84` |
| `pub struct CellRectOccupancyContext<'a>` | `:99` |
| `pub fn check_passability_rect(ctx) -> bool` (zero-size→true AND-fold) | `:110` |
| `pub fn check_occupancy_rect(ctx) -> bool` (blocker scan + `rect_in_playfield`) | `:131` |
| `fn check_cell_passability` (Winged fast-pass, MovementZone zone-id, `+0x124/+0x128` selection, wall exception, speed-table) | `:188` |
| `fn rect_in_playfield` (4-corner `x+w-1`/`y+h-1`) | `:321` |
| `fn reservation_mask` (`-1`→0 else `1<<(arg&0x1F)`) | `:337` |
| Tests: `cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers`, `cellrect_occupancy_house_reservation_blocks_same_house_only`, `cellrect_passability_uses_movement_zone_zone_id_and_speed_type_separately`, `cellrect_passability_bridge_bits_are_not_occupancy_rect_blockers` | `:424`–`:615` |

**Consequence — this plan re-scopes the design's P1–P3 from "create fresh" to "extend `cell_rect.rs` for the parity gaps the first slice did not cover," and adds the two genuinely-missing pieces (cell-index/dummy fallback, and FNPC).** This is exactly the design's stated intent ("EXTENDS, does not replace the first-slice boundary") — the design author wrote the signatures from the STUDY without re-reading that `cell_rect.rs` already implements them. Building a parallel `cell_validation/` module would duplicate `check_passability_rect`/`check_occupancy_rect` and violate "do not duplicate the predicate." **Decision (must be confirmed at plan-review): grow `cell_rect.rs` (split into a `cell_rect/` directory only if it crosses ~600 lines), do NOT create `cell_validation/`.**

Genuine gaps the first slice left (this is what the tasks below actually build):
1. **No cell-index/dummy fallback** — `cell_rect.rs::to_cell_coord` (`:345`) is a `u16`-bounds gate returning `None`; there is no `y*0x200+x` fixed-stride index and no non-null dummy (L1/L2/L3). Current production-path lookups use `PathGrid::cell` (`y*width+x`, `None` OOB, `core.rs:1642`).
2. **No `find_nearby_passable_cell`** — diamond-ring + frame-counter selection (L20–L29). Current Rust uses `nearest_walkable_around` (box-ring, first-match) at `production_spawn.rs:355` (def) / `:290` (call, radius 12).
3. **The hash flip** — making FNPC authoritative for the spawn/exit/scatter/chrono-return cell (P6).

Two pre-existing-behavior deltas in `cell_rect.rs` that the STUDY contract flags and the tasks must reconcile (NOT silently inherit):
- `check_occupancy_rect` (`:158-166`) rejects on `zone_type != GROUND || slope_type != 0`. The STUDY blocker scan (L14/C10) is `+0x44 overlay` → `+0x4C != 0` → `+0x11C != 0`; `+0x4C` is the reduced-ZoneType column (C19) and `+0x11C` is the slope/special byte (C10e). The current Rust fuses both into one terrain-cell test. **Order is observable (L14 "first-blocker semantics").** Task T5 re-reads the live order and reconciles.
- `check_cell_passability` does not yet implement the `&0xE0`/`&0x5F` occupation-mask modifier args (L9/C8) — the wrapper path passes both zero, so full-byte-must-be-zero is correct for the wrapper, but the sub-cell-aware path is absent. Documented as a deferred sub-cell seam (Open Question 7); not built here unless a human-path caller needs it.

---

## Task graph (dependency order)

```
T0 (research gate, non-code; gates #1/#3/#4 CLOSED, #2 DEFERRED) ─┬─> T1 (cell-index + dummy)
                              ├─> T2 (extend passability: shadow-vs-PathGrid test only) [Track-over-Clear CONFIRMED]
                              └─> T3 (extend occupancy: reconcile blocker order) ──> T3.5 (playfield corner formula) [Gate #1 CLOSED — READY]
T1 ──> T4 (find_nearby_passable_cell, shadow; Gate #4 CLOSED — 3 reconcile items) ──> T5 (shadow-assert FNPC vs nearest_walkable_around)
T2,T3,T3.5,T4 ──> T6 (invert spawn fallback to facade predicates; still hash-neutral)
T5,T6 ──> T7 (AUTHORITATIVE FNPC + thread binary_frame + SNAPSHOT_VERSION 17→18)  [HASH-RELEVANT — UNBLOCKED]
T7 ──> T8 (replay/parity harness)
```

Nine implementable tasks (T1–T8 plus T3.5) plus the non-code research gate T0. **Only T7 is hash-relevant.** Gates #1, #3, #4 are CLOSED (resolution docs cited in T0); only the non-blocking dummy-field gate #2 (L3) remains DEFERRED.

---

## T0 — Research gate (non-code; closes before T3/T4 land their math)

**Type:** non-code (Ghidra). Not a build task; gates T3 and T7.
**STUDY status:** the three former BLOCKING gates (CheckCellPassability body, FNPC selection source, RTTI-0x24 identity, save/load order) are CLOSED PASS-2. **Three of the four remaining gates below are now CLOSED (see resolution docs); only #2 (dummy field values) stays open.**

1. **CLOSED — `IsRectInPlayfield 0x00578390` exact 4-corner formula (L19, was blocking T3's corner reconcile).** Resolved in `docs/research/GATE_PLAYFIELD_RECT_RESOLUTION_GHIDRA_REPORT.md` (CLOSED for the corner formula a/b/c/e; only the human field-names of the bound source d are left YELLOW — the formula is exact regardless). **Resolved fact:** the function tests exactly four corners — NW `(x,y)`, NE `(x+w-1,y)`, SW `(x,y+h-1)`, SE `(x+w-1,y+h-1)` in that fixed AND-chained, short-circuit order using **inclusive** `x+width-1`/`y+height-1`, and EACH corner is judged by `Is_Cell_In_Playfield 0x00578460` as an **isometric diamond** test (sum `sx+sy` in a half-open band `(low, high]`, plus strict `(sx-sy) < RIGHT` and `(sy-sx) < LEFT`) against `MapClass` bound fields `+0xF4/+0xFC/+0x100/+0x104/+0x108` — **NOT** a rectangular `0 <= x < 512` array-index test. The sole caller (CheckOccupancy) passes `height_flag=1`, enabling the `+0x11B`/`+0x11C` slope extension. **CONTRADICTS the current Rust** (`cell_rect.rs:321-335` uses `0 <= c < dim` rect bounds). **Decision: split T3.5** — it is now READY TO IMPLEMENT (the corner formula and the named test). See T3.5 below.
2. **STILL OPEN (non-blocking) — Dummy `0x00ABDC50` runtime-init field values (L3, gates T1 only IF a human-path caller reads dummy fields).** Statically BSS-zero; needs a live-init dump. Action: triage callers first (T1) — if no human-path caller reads fields off an OOB dummy, the only observable is non-null-vs-`None`, and L3 stays DEFERRED. Design Open Question 3. **Remaining query:** `read_memory 0x00ABDC50` at runtime (or decode its initializer) ONLY if a human-path caller is found to read fields off the OOB dummy; otherwise leave DEFERRED.
3. **CLOSED — Speed-table dump values (L12, was a spot-confirm before T2/T6 per-cell math).** Resolved in `docs/research/GATE_SPEEDTYPE_MATRIX_RESOLUTION_GHIDRA_REPORT.md` (CLOSED-PASS for the gate question). **Resolved fact:** `SpeedType=Track` over `LandType=Clear` PASSES — the binary land-speed table at `0x0089EA40`, indexed `[LandType*9 + SpeedType]` (Track = column 1, Clear = row 0), holds `[Clear] Track=100%` = `1.0`, and `Can_Enter_Cell 0x0073F0A0` returns Impassable ONLY when that entry `== 0.0` (constant `0x007E1748`). So `speed_type_allows_cell(Clear, Track)`==true and the `find_nearby_*` flat-terrain test premise is binary-confirmed and safe to keep — T0 #3 / T2 / T6 are unblocked. **DRIFT noted (not blocking this slice):** `src/sim/pathfinding/passability.rs::is_passable_for_speed_type` does NOT use this table — it routes Track→MovementZone row 2 of the separate 13×8 `ZonePassabilityMatrix 0x0082A594` and indexes `[2][0]`==1; the boolean coincidentally agrees for (Clear, Track) but conflates two distinct tables and drops the per-terrain speed multiplier (Tiberium/Weeds Track=70%, Ice=80%). Legality should be modeled as `float[12][9] != 0.0`; file the multiplier as a separate speed/cost follow-up. Do NOT "fix" the boolean now — it is correct for this gate.

4. **CLOSED — FNPC diamond-ring ENUMERATION (C15), the last DOC-ONLY input feeding the hashed cell choice.** Resolved in `docs/research/GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` (CLOSED). The frame-counter *selection* was already PASS-2 VERIFIED, and the candidate-POOL enumeration shape is now read from the body (`decompile_function`/`disassemble_function 0x0056DC20`). **Resolved facts:** (a) **shape** = concentric DIAMOND rings r=0..min(Speed[+0xF4]+Sight[+0xF8], 32)-1, capped at **24** candidates (`0x18`); (b) **per-ring visit order** = N/S apex rows `(ox+d, oy∓r)` for `d=-r..+r` (N then S inside the loop), THEN W/E columns `(ox∓r, oy+e)` for `e=1-r..r-1` (W then E) — a fixed 4-segment order, NOT row-major and NOT a continuous CW/CCW walk; (c) **per-ring early-out** = once any DIRECT candidate is accepted, finish the current ring then STOP scanning further rings; (d) **direct/indirect** = the `FUN_006d6410` height-projection identity test (a cell whose lepton-center resolves back to itself is "direct"), NOT "on a cardinal axis from the seed"; direct pool is preferred whenever non-empty; (e) **selection** = `g_FrameCounter [0x00a8ed84] % pool.len()` when no target (deterministic global per-tick counter, incremented at `0x0055de73`, NOT RNG) or strict-`<` nearest `sqrt(dx²+dy²)` (first-found-on-tie) when a target is given. **Label trap corrected in the doc:** the decompiler's `g_CurrentFrameCounter 0x00887324` is actually a `this`-pointer arg to `FUN_006d6410`, NOT a counter — the true selection counter is `0x00a8ed84`. **CONTRADICTS the current Rust shadow** on three points (ring visit order is row-major, `direct`=cardinal-axis test, no per-ring early-out) — these must be reconciled before the T7 authority flip; see T4 and T7 below. **T7 is unblocked** — C15 is pinned; no user sign-off gate remains for the hash flip.

**Output:** §9 STUDY rows updated; gates #1/#3/#4 CLOSED with resolved facts recorded inline; T3.5 split out and READY; T2/T6 unblocked (Track-over-Clear confirmed); T7 unblocked (C15 pinned) with three reconcile items now mandatory. Only gate #2 (L3 dummy field values) stays DEFERRED, non-blocking.

---

## T1 — Cell-index + non-null dummy fallback (read-only, hash-neutral)

**Goal:** add the fixed `y*0x200+x` index (L1) and the non-null dummy lookup (L2) the engine uses, replacing the `None`/`u16`-bounds behavior on the *parity* path (the `PathGrid` width-index stays as the cache).

**File to edit:** `src/sim/cell_rect.rs` (extend; currently 617 lines — adding ~60 lines + tests keeps it cohesive but watch the ~600 guideline; if it crosses, this is the trigger to split into `src/sim/cell_rect/{mod.rs,index.rs}` in this task).

**Add near the top of `cell_rect.rs` (after the `use` block, before `CellRect`):**

```rust
/// Fixed cell-array stride — the engine indexes cells y*0x200+x regardless of the
/// loaded map's playfield width. Valid linear range [0, MAX_CELL_INDEX].
pub const CELL_ROW_STRIDE: i64 = 0x200;
pub const MAX_CELL_INDEX: i64 = 0x3FFFF;

/// Linear cell index using the fixed 512-wide stride (NOT the loaded-map width).
/// `None` only when the index falls outside [0, MAX_CELL_INDEX]; the dummy
/// fallback (`get_cellclass_fallback`) turns that into a non-null reference.
pub fn cell_linear_index(x: i32, y: i32) -> Option<i64> {
    let idx = (y as i64) * CELL_ROW_STRIDE + (x as i64);
    (0..=MAX_CELL_INDEX).contains(&idx).then_some(idx)
}

/// A non-null cell reference — `Real` for an in-range, present cell, or `Dummy`
/// carrying the requested coord for an OOB/missing lookup. NEVER `None` (L2):
/// the engine returns a non-null dummy that stores the requested coord and lets
/// the caller keep dispatching on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRef<'a> {
    Real(&'a crate::map::resolved_terrain::ResolvedTerrainCell),
    Dummy { coord: (i32, i32) },
}

/// gamemd `Get_CellClass`: coord -> cell via the fixed stride; OOB or missing
/// cell returns `CellRef::Dummy { coord }` (the requested coord, NOT (0,0)).
pub fn get_cellclass_fallback<'a>(
    terrain: Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid>,
    x: i32,
    y: i32,
) -> CellRef<'a> {
    if cell_linear_index(x, y).is_some() {
        if let (Ok(rx), Ok(ry)) = (u16::try_from(x), u16::try_from(y)) {
            if let Some(cell) = terrain.and_then(|t| t.cell(rx, ry)) {
                return CellRef::Real(cell);
            }
        }
    }
    CellRef::Dummy { coord: (x, y) }
}
```

**Why `ResolvedTerrainCell`, not `PathCell`:** the design sketch used `&PathCell`, but the occupancy/passability validators in `cell_rect.rs` already key off `ResolvedTerrainGrid::cell` (`:160`, `:203`). Using the same backing type keeps one source of truth and avoids importing the `core.rs` `PathCell` (whose `cell()` is the width-indexed cache we are explicitly NOT making authoritative). The dummy carries only the coord (L2); per L3 it exposes no field values until T0 item 2 closes — callers must match on `Dummy` and treat field reads as DEFERRED.

**Rollout:** pure-additive, read-only. No caller migrates yet; `to_cell_coord` (`:345`) stays for the existing rect loops. Hash-neutral by construction.

**Verification:**
- `cargo test -p vera20k --lib cell_rect::tests::cell_index_uses_512_wide_stride_not_map_width`
- `cargo test -p vera20k --lib cell_rect::tests::get_cellclass_oob_returns_dummy_with_requested_coord`

**Named tests to add (in `cell_rect.rs` `#[cfg(test)]`):**
```rust
#[test]
fn cell_index_uses_512_wide_stride_not_map_width() {
    // (x=0, y=1) -> 0x200 under the fixed stride, regardless of any loaded width.
    assert_eq!(cell_linear_index(0, 1), Some(0x200));
    assert_eq!(cell_linear_index(1, 0), Some(1));
    // Out of the [0,0x3FFFF] linear range -> None (then dummy at the caller).
    assert_eq!(cell_linear_index(-1, 0), None);
}

#[test]
fn get_cellclass_oob_returns_dummy_with_requested_coord() {
    let g = flat_terrain(2, 2);
    assert!(matches!(get_cellclass_fallback(Some(&g), 0, 0), CellRef::Real(_)));
    // Out of bounds: non-null dummy carrying the *requested* coord (never None, never (0,0)).
    assert_eq!(
        get_cellclass_fallback(Some(&g), -3, 7),
        CellRef::Dummy { coord: (-3, 7) }
    );
}
```

---

## T2 — Passability: shadow-vs-PathGrid agreement test (read-only, hash-neutral) — READY (Gate #3 CLOSED)

**Status:** READY TO IMPLEMENT. T0 item 3 (Gate #3) is CLOSED — `docs/research/GATE_SPEEDTYPE_MATRIX_RESOLUTION_GHIDRA_REPORT.md` confirms `(Clear, Track)` PASSES via the binary land-speed table `[Clear(0)*9 + Track(1)] = 1.0`. The `speed_type_allows_cell(Clear, Track)`==true premise is binary-verified, so the shadow agreement test below is safe to write and the `speed_type_allows_cell` body needs NO change for this gate.

**Goal:** the predicate body already exists (`check_passability_rect`, `:110`); this task only adds the *shadow guarantee* test the design names (P2) — surface, never equalize, divergence from `PathGrid::is_walkable` on plain cells.

**File to edit:** `src/sim/cell_rect.rs` (tests only). Do NOT touch `speed_type_allows_cell` (`:271`) — Gate #3 verified its boolean is correct for (Clear, Track). **Resolved-fact note for the implementer:** the binary legality contract is `landSpeedTable[LandType*9 + SpeedType] != 0.0` (SpeedType col 1 = Track, LandType row 0 = Clear); the per-terrain speed *multiplier* (Tiberium/Weeds Track=70%, Ice=80%) is a SEPARATE speed/cost follow-up — do not fold it into legality here, and do not model legality through `is_passable_for_speed_type`'s 13×8 zone matrix (structural DRIFT flagged in the resolution doc).

**Rollout:** read-only (test-only). Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib cell_rect::tests::passability_rect_shadow_agrees_with_pathgrid_on_plain_cells`
- Re-run the existing `cellrect_passability_uses_movement_zone_zone_id_and_speed_type_separately` (already present, `:499`) to prove no regression.

**Named tests to add:**
```rust
#[test]
fn passability_rect_shadow_agrees_with_pathgrid_on_plain_cells() {
    // On cells with no overlay/zone/height constraint, a 1x1 passability rect
    // must AGREE with PathGrid::is_walkable. Divergence is surfaced (assert
    // names the cell), never equalized away.
    let terrain = flat_terrain(4, 4);
    let path_grid = PathGrid::from_resolved_terrain(&terrain);
    for ry in 0..4u16 {
        for rx in 0..4u16 {
            let ctx = CellRectPassabilityContext {
                rect: CellRect::single(rx, ry),
                speed_type: SpeedType::Track,
                required_zone_id: None,
                movement_zone: MovementZone::Normal,
                required_height_or_level: None,
                bridge_aware_zone: false,
                reject_any_overlay: false,
                path_grid: Some(&path_grid),
                resolved_terrain: Some(&terrain),
                overlay_grid: None,
                occupancy: None,
                zone_grid: None,
            };
            assert_eq!(
                check_passability_rect(ctx),
                path_grid.is_walkable(rx, ry),
                "passability/PathGrid divergence at ({rx},{ry})"
            );
        }
    }
}

#[test]
fn passability_zero_size_rect_returns_true() {
    let terrain = flat_terrain(1, 1);
    let ctx = CellRectPassabilityContext {
        rect: CellRect::new(0, 0, 0, 0),
        speed_type: SpeedType::Track,
        required_zone_id: None,
        movement_zone: MovementZone::Normal,
        required_height_or_level: None,
        bridge_aware_zone: false,
        reject_any_overlay: false,
        path_grid: None,
        resolved_terrain: Some(&terrain),
        overlay_grid: None,
        occupancy: None,
        zone_grid: None,
    };
    assert!(check_passability_rect(ctx)); // L4: width<=0 -> true, no cell read
}
```

---

## T3 — Occupancy: reconcile blocker order (read-only, hash-neutral)

**Status:** READY TO IMPLEMENT. The blocker-order split below was never gated on a Ghidra answer; the corner-formula reconcile (formerly part of T3, gated on Gate #1) is now SPLIT OUT to **T3.5** which is also READY (Gate #1 CLOSED). T3 lands only the blocker-order split.

**Goal:** make the existing `check_occupancy_rect` blocker scan match the engine's **observable order** (L14/C10). The corner check moves to T3.5.

**File to edit:** `src/sim/cell_rect.rs` — `check_occupancy_rect` (`:131`), specifically the per-cell scan (`:145-169`) and `rect_in_playfield` (`:321`).

**Current scan order in code (`:145-169`):**
`terrain_object_blocks` (a) → reservation `+0xDC` (b) → `overlay_present` (c) → fused `zone_type != GROUND || slope_type != 0` (d) → `ground_building_present` (f).

**Engine order (L14/C10):** (a) RTTI-0x24 TerrainClass → (b) `+0xDC & mask` → (c) `+0x44 overlay != -1` → (d) `+0x4C != 0` (reduced-ZoneType column) → (e) `+0x11C != 0` (slope/special) → (f) `WhatAmI()==6` building.

**Required change:** split the fused step (d) into the two **separately ordered** rejects `+0x4C` (d, zone-type column) then `+0x11C` (e, slope). The current code checks both in one `is_some_and` and is therefore order-ambiguous between them. Because all rejects return the same `false`, the order is only observable when two of these conditions disagree on a cell AND a caller distinguishes *which* fired — no current caller does, BUT default-to-DRIFT requires the order be reproduced and tested (L14). Proposed body for the per-cell block:

```rust
// (a) TerrainClass-category occupant present (RTTI-0x24, L18/C21).
if terrain_object_blocks(ctx.resolved_terrain, rx, ry) {
    return false;
}
// (b) per-house/site reservation bit (skipped when mask == 0, i.e. arg -1).
if mask != 0
    && ctx.reservations.is_some_and(|r| r.has_reservation(rx, ry, ctx.reservation_arg))
{
    return false;
}
// (c) overlay present (+0x44 != -1).
if overlay_present(ctx.overlay_grid, rx, ry) {
    return false;
}
let tcell = ctx.resolved_terrain.and_then(|t| t.cell(rx, ry));
// (d) reduced-ZoneType column nonzero (+0x4C; C19 column 0 == Ground passes).
if tcell.is_some_and(|c| c.zone_type != zone_class::GROUND) {
    return false;
}
// (e) slope/special byte nonzero (+0x11C).
if tcell.is_some_and(|c| c.slope_type != 0) {
    return false;
}
// (f) building occupant on the ground list (WhatAmI()==6).
if ground_building_present(ctx.occupancy, ctx.entities, rx, ry) {
    return false;
}
```

**Corner check:** moved to T3.5 (Gate #1 CLOSED — the current `:321-335` formula CONTRADICTS the binary and must be rewritten). T3 leaves `rect_in_playfield` untouched.

**Rollout:** read-only. The order split changes no result on any current fixture (same `false`), so hash-neutral — proven by re-running the existing occupancy tests unchanged. Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib cell_rect::tests::occupancy_blocker_order_matches_engine`
- Re-run existing `cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers` (`:424`) and `cellrect_occupancy_house_reservation_blocks_same_house_only` (`:456`) — must stay green.

**Named tests to add:**
```rust
#[test]
fn occupancy_blocker_order_matches_engine() {
    // Each rejecter fires in C10 order; a fixture with ONLY a slope (+0x11C)
    // still rejects, and ONLY an overlay still rejects, independently.
    let mut terrain = flat_terrain(3, 1);
    terrain.cells[1].slope_type = 2;          // (e) only
    terrain.cells[2].zone_type = zone_class::WATER; // (d) only
    let ctx0 = CellRectOccupancyContext {
        rect: CellRect::single(0, 0), reservation_arg: -1, reservations: None,
        occupancy: None, entities: None, resolved_terrain: Some(&terrain),
        overlay_grid: None, map_size: None,
    };
    assert!(check_occupancy_rect(ctx0));      // clear cell passes
    let ctx_slope = CellRectOccupancyContext { rect: CellRect::single(1, 0), ..ctx0_like() };
    assert!(!check_occupancy_rect(ctx_slope));
    let ctx_zone = CellRectOccupancyContext { rect: CellRect::single(2, 0), ..ctx0_like() };
    assert!(!check_occupancy_rect(ctx_zone));
}
```
*(Plan-review note: the `..ctx0_like()` shorthand above is illustrative; the real tests construct each context literal explicitly — `CellRectOccupancyContext` has no `Default` and is not `Copy`.)*

---

## T3.5 — Playfield-corner formula: rewrite `rect_in_playfield` to the diamond test (read-only, hash-neutral) — READY (Gate #1 CLOSED)

**Status:** READY TO IMPLEMENT. Gate #1 is CLOSED — `docs/research/GATE_PLAYFIELD_RECT_RESOLUTION_GHIDRA_REPORT.md` (CLOSED for the corner formula a/b/c/e; only the human field-names of the bound source d are YELLOW, formula exact regardless). The current Rust `rect_in_playfield` (`cell_rect.rs:321-335`) uses a `0 <= c < dim` rectangular bounds check, which **CONTRADICTS the binary** — the binary is an isometric diamond test, not a rectangle. This task rewrites it.

**File to edit:** `src/sim/cell_rect.rs` — `rect_in_playfield` (`:321-335`) and the `occupancy_zero_size_rect_still_runs_playfield_corners` test (currently `#[ignore]`'d — un-ignore and assert the resolved degenerate behavior).

**Required correction (exact, from the resolution doc):**
1. Test exactly **four** corners, AND-chained, short-circuit on first failure, in this fixed order: NW `(x, y)`, NE `(x+w-1, y)`, SW `(x, y+h-1)`, SE `(x+w-1, y+h-1)`. Use **inclusive** `w-1`/`h-1` (NOT `x+w`). Returns true only if all four pass.
2. Each corner predicate is the **isometric diamond** test (NOT `0 <= x < 512 && 0 <= y`). With `sx`,`sy` the corner's signed cell coords and `h` the height extension:
   - `(base + LOW)  <  (sx + sy)` — strict low (sum band lower bound exclusive)
   - `(sx + sy)     <= (base + HIGH)` — inclusive high (sum band upper bound inclusive)
   - `(sx - sy)     <  RIGHT` — strict
   - `(sy - sx)     <  LEFT` — strict
   where, against `MapClass`-equivalent bound fields (Rust: the loaded playfield bounds; `+0xF4/+0xFC/+0x100/+0x104/+0x108` in the binary, all doubled, `+base` origin):
   - `base = field(+0xF4)`
   - `LOW  = field(+0x100)*2 + h`
   - `HIGH = 2 + (field(+0x108)+field(+0x100))*2 + h`
   - `RIGHT = (field(+0x104)+field(+0xFC))*2 - base`
   - `LEFT  = base - field(+0xFC)*2`
3. Include the **height-flag extension** since the live caller (CheckOccupancy) always passes `height_flag=1`: fetch the cell at `(sx,sy)`, set `h = (signed)cell.level_byte (+0x11B)`, and if `cell.slope_byte (+0x11C) != 0` AND `sx+sy < base + 4 + field(+0x100)*2 + h` then `h += 1`.

**Bound-source naming (YELLOW from the doc):** the human names for the five `MapClass` bound fields are UNVERIFIED; the formula is exact regardless. The Rust implementer maps them to the engine's playfield-bound fields already loaded at map-load — if the exact Rust source struct for these five values is not yet available, that is the one residual sub-blocker for THIS task: name them after the real loaded-map bounds, or surface a follow-up to decode the writers (`MapClass` init / `RecalcCellsAndRebuildZones 0x00586990`). The four-corner/inclusive/diamond SHAPE is fully resolved and should be implemented now.

**Degenerate (0-size) rect — resolved fact (CONTRADICTS the old "no-op" assumption):** `IsRectInPlayfield` does NO loop and does NOT special-case `width<=0`/`height<=0`. With `width=0`, NE/SE x = `x+0-1 = x-1`; with `height=0`, SW/SE y = `y-1`. So a 0-size rect **evaluates the four corners at decremented `(x-1,y-1)`-style coords** and all four must still satisfy the diamond — it is NOT a no-op and NOT an auto-pass. The `occupancy_zero_size_rect_still_runs_playfield_corners` test must assert exactly this (un-ignore it).

**Rollout:** read-only. Changes the *shape* of the playfield test, but `check_occupancy_rect` only consults it on the occupancy path which no hashed caller currently exercises with an OOB rect in a way that flips state on current fixtures — re-run the existing occupancy tests to prove hash-neutral. Hash-neutral by construction (no caller migrates to occupancy here).

**Verification:**
- `cargo test -p vera20k --lib cell_rect::tests::rect_in_playfield_is_isometric_diamond_inclusive_four_corners`
- `cargo test -p vera20k --lib cell_rect::tests::occupancy_zero_size_rect_still_runs_playfield_corners` (un-ignored)
- Re-run all existing `cell_rect::tests::cellrect_occupancy_*` — must stay green.

**Named tests to add:**
```rust
#[test]
fn rect_in_playfield_is_isometric_diamond_inclusive_four_corners() {
    // A cell on the diamond's inclusive HIGH edge of the sum band passes;
    // one just past it fails. A rect whose far corner (x+w-1, y+h-1) leaves
    // the diamond fails even when its NW corner is inside (proves inclusive w-1/h-1
    // far corner AND the diamond, not a rectangle).
    // ... construct a small playfield-bound fixture and assert per the diamond formula.
}

#[test]
fn occupancy_zero_size_rect_still_runs_playfield_corners() {
    // Degenerate rect runs the corner check at DECREMENTED coords (x-1,y-1), not a no-op.
    // A 0-size rect at the diamond edge whose (x-1,y-1) corner falls OUT of the diamond
    // FAILS; one whose decremented corners stay inside PASSES.
    // ... un-ignore; assert decremented-corner behavior per Gate #1 §6.
}
```
*(Note: the bound-field fixture must expose the five playfield-bound values; if the Rust map-load type for these is not yet wired into `CellRectOccupancyContext`, thread them in as part of this task or stub a test-only `PlayfieldBounds` carrying the five values — do NOT regress to the rect-bounds approximation.)*

---

## T4 — `find_nearby_passable_cell` (diamond-ring + selection) — shadow, read-only — READY (Gate #4 CLOSED, 3 mandatory reconcile items)

**Status:** READY TO IMPLEMENT. Gate #4 is CLOSED — `docs/research/GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` pins the ring SHAPE, per-ring VISIT ORDER, early-out, direct/indirect classification, and selection from the body (`0x0056DC20`). The enumeration is no longer DOC-only; T7 (the authority flip that depends on this pool) is therefore unblocked. **The resolved facts CONTRADICT the current Rust shadow on three points — these reconciles are now MANDATORY (not optional) for the bit-identical pool T7 hashes:**

1. **Ring visit order (CONTRADICTS Rust).** Engine per-ring order is `{N(ox+d, oy-r), S(ox+d, oy+r)} for d=-r..+r` (N then S), THEN `{W(ox-r, oy+e), E(ox+r, oy+e)} for e=1-r..r-1` (W then E) — a fixed 4-segment order. The current Rust `diamond_ring` emits **row-major top→bottom, left-then-right** — a DIFFERENT order. This changes (i) which 24 cells survive when the cap truncates a partially-scanned ring and (ii) the nearest-distance TIE winner. **Align `collect_candidates`/`diamond_ring` to the engine's 4-segment sequence.**
2. **Direct vs indirect (CONTRADICTS Rust).** Engine "direct" = the `FUN_006d6410` height-projection identity test (lepton-center resolves back to itself), NOT the Rust `Candidate.direct = (cx==seed.0 || cy==seed.1)` cardinal-axis test. They AGREE on flat terrain, DIVERGE on slopes/bridges. For full parity classify by the height-projection identity. (If slice-2 scope is flat-terrain only this CAN be deferred, but RECORD it — it is a real divergence on sloped/bridge cells.)
3. **Per-ring early-out (MISSING in Rust).** Once any DIRECT candidate is accepted, finish the current ring then STOP scanning further rings (biases toward the nearest direct-hit ring). The current Rust collects until the 24-cap or `radius_cap` with no early-out. **Add the "direct found → finish ring → stop" termination.**

Selection itself is correct as designed: `frame_counter % pool.len()` over the direct-preferred pool (frame_counter = the global per-tick counter `0x00a8ed84` = `Simulation::binary_frame`, NEVER RNG), and the nearest-distance target path's integer `dx*dx+dy*dy` `min_by_key` matches the engine's strict-`<` sqrt with first-found-on-tie (monotonic; `min_by_key` keeps the first minimum). Cap = 24 candidates; radius cap = `min(Speed+Sight, 32)`.

**Goal:** implement the engine's diamond-ring search (L20) with the three reconciles above, the SkipReservation occupancy call (L23), the post-passability bridge filter (L22), the frame-counter selection with direct-preferred (L24), the nearest-distance target path (L25), same-tick aliasing (L26), and the no-candidate→`None` result (L27). **Do not migrate any caller yet** — built and unit-tested standalone.

**File to create:** `src/sim/cell_rect/find_nearby.rs` (if T1 split `cell_rect.rs` into a directory) OR a new sibling `src/sim/find_nearby_cell.rs` declared `pub mod find_nearby_cell;` at `src/sim/mod.rs`. **Prefer the `cell_rect/` directory** so the FNPC search sits beside the predicates it calls. The plan assumes the directory split happens in T1 if needed; otherwise create `src/sim/find_nearby_cell.rs` and `use crate::sim::cell_rect::*;`.

**Proposed types + signature (real types verified this run):**

```rust
//! Nearby-passable-cell search (engine Find_Nearby_Passable_Cell). Diamond-ring
//! expansion around a seed; per-candidate passability (+ optional occupancy with
//! SkipReservation); frame-counter selection when no target, nearest-distance when
//! a target is given. Read-only over the cell grids; consumes no RNG stream.

use crate::sim::cell_rect::{
    CellRect, CellRectOccupancyContext, CellRectPassabilityContext,
    check_occupancy_rect, check_passability_rect,
};

/// FNPC config (engine Find_Nearby_Passable_Cell caller args).
pub struct NearbyQuery<'a> {
    pub passability: PassabilityArgs,        // built into a 1x1 CellRectPassabilityContext per candidate
    pub allow_bridge_cells: bool,            // L22: filter applied AFTER passability
    pub check_height: bool,                  // L21: FNPC ±2 internal gate (DEFERRED gate, see OQ6)
    pub check_occupancy: bool,               // L23: call check_occupancy_rect(.., -1)
    pub radius_cap: u16,                      // L20: min(Speed+Sight, 32), computed by caller
    pub target_cell: Option<(i32, i32)>,     // None => frame-counter (L24); Some => nearest-dist (L25)
    // Borrowed grids the per-candidate predicates read:
    pub path_grid: Option<&'a crate::sim::pathfinding::PathGrid>,
    pub resolved_terrain: Option<&'a crate::map::resolved_terrain::ResolvedTerrainGrid>,
    pub overlay_grid: Option<&'a crate::sim::overlay_grid::OverlayGrid>,
    pub occupancy: Option<&'a crate::sim::occupancy::OccupancyGrid>,
    pub entities: Option<&'a crate::sim::entity_store::EntityStore>,
    pub zone_grid: Option<&'a crate::sim::pathfinding::zone_map::ZoneGrid>,
    pub map_size: Option<(u16, u16)>,
}

/// The subset of CellRectPassabilityContext fields FNPC always supplies the same
/// way; FNPC always passes required_height_or_level = -1 (L21) and reject_any_overlay
/// = false (L5: chrono-return passes 0).
pub struct PassabilityArgs {
    pub speed_type: crate::rules::locomotor_type::SpeedType,
    pub required_zone_id: Option<crate::sim::pathfinding::zone_map::ZoneId>,
    pub movement_zone: crate::rules::locomotor_type::MovementZone,
    pub bridge_aware_zone: bool,
}

/// gamemd Find_Nearby_Passable_Cell. `frame_counter` MUST be the sim per-tick
/// counter (Simulation::binary_frame); never an RNG draw (L24/L26). Returns the
/// engine null-cell {0,0} as `None` (L27).
pub fn find_nearby_passable_cell(
    seed: (i32, i32),
    q: &NearbyQuery<'_>,
    frame_counter: u32,
) -> Option<(u16, u16)> {
    // 1. Collect candidates over concentric DIAMOND rings outward from seed
    //    (Gate #4 CLOSED — GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md):
    //    radius cap = q.radius_cap = min(Speed+Sight, 32); largest ring scanned = cap-1.
    //    Per-ring visit order is the engine's FIXED 4-segment sequence (NOT row-major):
    //      segment 1: for d = -r..=r: N cell (ox+d, oy-r) then S cell (ox+d, oy+r)
    //      segment 2: for e = 1-r..=r-1: W cell (ox-r, oy+e) then E cell (ox+r, oy+e)
    //    (ring 0 degenerates to the single seed cell; segment 2 range is empty.)
    //    Early-terminate at 24 candidates (cap 0x18). For each candidate cell:
    //      a. check_passability_rect(1x1, required_height_or_level = None /*-1*/,
    //         reject_any_overlay = false).  (L21/L5)
    //      b. if q.check_occupancy: check_occupancy_rect(1x1, reservation_arg = -1).  (L23)
    //      c. if !q.allow_bridge_cells AND candidate is a structural-bridge cell: drop.  (L22, AFTER a/b)
    //    Classify each surviving candidate as "direct" vs "indirect" by the height-projection
    //    IDENTITY test (FUN_006d6410: lepton-center resolves back to itself), NOT the
    //    cardinal-axis test (agree on flat terrain, diverge on slope/bridge); two Vecs in
    //    deterministic ring order. PER-RING EARLY-OUT: once any DIRECT candidate is accepted,
    //    finish the current ring then STOP scanning further rings.
    // 2. Pool selection:
    //    let pool = if !directs.is_empty() { &directs } else { &indirects };  (L24 direct-preferred)
    //    if pool.is_empty() { return None; }                                   (L27)
    //    match q.target_cell {
    //        None        => pool[(frame_counter as usize) % pool.len()],       (L24 frame-counter modulo)
    //        Some(tgt)   => pool min-by Euclidean distance to tgt,             (L25; no frame/RNG)
    //    }
    todo!("implement per the comment contract; all math fixed-point / integer")
}
```

**Determinism notes baked into the contract:** the diamond-ring candidate ORDER must be deterministic (it feeds both the modulo index and the nearest-distance tie-break); use integer Euclidean comparison (`dx*dx + dy*dy`, no float) for L25 to stay fixed-point per the layering invariant. `frame_counter % pool.len()` reproduces same-tick aliasing (L26) by construction — do NOT add any per-call perturbation.

**Rollout:** pure-additive, read-only. No caller wired. Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib find_nearby_cell::tests::find_nearby_diamond_ring_visit_order`
- `cargo test -p vera20k --lib find_nearby_cell::tests::find_nearby_calls_occupancy_with_skip_reservation`
- `cargo test -p vera20k --lib find_nearby_cell::tests::find_nearby_allow_bridge_filters_after_passability`
- `cargo test -p vera20k --lib find_nearby_cell::tests::find_nearby_no_candidate_returns_none`
- `cargo test -p vera20k --lib find_nearby_cell::tests::find_nearby_passes_required_height_minus_one`

---

## T5 — Shadow-assert FNPC against `nearest_walkable_around` (read-only, hash-neutral)

**Goal:** in a test fixture, run BOTH `find_nearby_passable_cell` and `nearest_walkable_around` over the same grid and assert their candidate *sets* (not the chosen cell — they choose differently by design). Surfaces divergence in the search shape before T7 flips the chosen cell. No production code changes the chosen cell yet.

**File to edit:** add a test module in the FNPC file (T4) that imports `production_spawn`'s helpers. Note `nearest_walkable_around` is a private `fn` at `production_spawn.rs:355`; to shadow it from a test, either (a) add `#[cfg(test)] pub(crate)` visibility to it, or (b) write the shadow test *inside* `production_spawn.rs`'s `#[cfg(test)]` module. **Prefer (b)** — no production visibility change.

**Rollout:** read-only (test-only). Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib production::production_spawn::tests::find_nearby_candidate_set_shadows_nearest_walkable_around`

---

## T6 — Invert spawn fallback to facade predicates (read-only, hash-neutral)

**Goal:** route the spawn fallback's per-candidate passability/occupancy decision through `check_passability_rect` / `check_occupancy_rect` so the predicate is single-sourced — WITHOUT yet changing the *search shape or chosen cell*. `nearest_walkable_around` keeps its box-ring; only its inner candidate test delegates to the facade (it already partly does — `spawn_fallback_candidate_passable` at `production_spawn.rs:506` already calls both `check_passability_rect` and `check_occupancy_rect`). This task ensures `cell_available_for_spawn` (`:553`) and `spawn_cell_passable` (`:585`) do not *additionally* gate in a way the facade does not, so the predicate result is the facade's.

**File to edit:** `src/sim/production/production_spawn.rs` — reconcile `cell_available_for_spawn` (`:553`) and `spawn_cell_passable` (`:585`) against the facade so there is exactly one passability+occupancy verdict per candidate.

**Rollout:** read-only IF predicates agree (proven by the no-hash-change test). This is the design's P4. Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib production::production_spawn::tests::spawn_fallback_uses_validator_predicates`
- `cargo test -p vera20k --lib production::production_spawn::tests::spawn_fallback_no_hash_change_when_predicates_agree`

---

## T7 — AUTHORITATIVE FNPC + thread `binary_frame` + bump SNAPSHOT_VERSION 17→18  **[HASH-RELEVANT — UNBLOCKED]**

**Status:** UNBLOCKED / READY TO IMPLEMENT (was gated on Gate #4's DOC-ONLY ring enumeration). Gate #4 is CLOSED — `docs/research/GATE_FNPC_RING_RESOLUTION_GHIDRA_REPORT.md` pins the pool composition from the binary body, so the cell `pool[frame % len]` selects is now trustworthy to the binary, not just to the DOC ring. **No user sign-off gate remains for the hash flip.** Precondition: T4's three mandatory reconciles (ring order, direct/indirect identity, per-ring early-out) must be implemented FIRST so the hashed pool is bit-identical — flipping authority on the un-reconciled row-major/cardinal-axis pool would lock in a wrong-but-deterministic hash. **Implementing this READY task flips hashed state: it follows shadow (T4/T5) → invert (T6) → authoritative (here) → SNAPSHOT_VERSION 17→18 → parity harness (T8).**

**Goal:** replace `nearest_walkable_around` (box-ring, first-match) with `find_nearby_passable_cell` (diamond-ring + frame-counter selection) in the spawn/exit fallback; the chosen cell becomes authoritative and feeds the hashed spawn position. Thread `Simulation::binary_frame` as the `frame_counter` (= the engine global per-tick counter `0x00a8ed84`, confirmed NOT RNG in the resolution doc). Bump `SNAPSHOT_VERSION`.

**Files to edit:**
1. `src/sim/production/production_spawn.rs`:
   - `find_spawn_cell_near_structure` (`:237`): the fallback at `:290` currently calls `nearest_walkable_around(...)`. Replace with `find_nearby_passable_cell(seed, &q, frame_counter)`. This requires threading `frame_counter: u32` (the value of `sim.binary_frame`) into `find_spawn_cell_near_structure` — add it as a parameter and pass `sim.binary_frame` from the call site at `:157` inside `find_spawn_selection_for_owner_with_type` (`sim: &mut Simulation` is in scope, `:69`).
   - Retire `nearest_walkable_around` (`:355`) once no caller remains.
2. `src/sim/snapshot.rs:24`: `SNAPSHOT_VERSION = 17` → `18`, with a new comment line documenting "17 -> 18: FNPC cell choice authoritative for spawn fallback (frame-counter selection); state hash changed."
3. (If/when scatter + chrono-return migrate in this slice's scope — design Open Question 1) the scatter caller and the chrono-miner-return seed. **Scope decision (Open Question 1): this task migrates the human-reachable spawn/exit fallback ONLY**; scatter / chrono-return / the other ~38 FNPC callsites (STUDY §P2.2) migrate in later slices but MUST all reuse `find_nearby_passable_cell` so any migrated/unmigrated mix stays internally consistent. Plan-review must confirm the single-caller-flip scope vs migrating all non-AI callers at once.

**`binary_frame` semantics (design-review correction 1 — getting this off-by-one desyncs every FNPC pick):** `Simulation::binary_frame` (field decl `world/mod.rs:302`) is **committed late** at the end of `advance_tick` (`world/mod.rs:1742` `self.binary_frame = ((self.total_sim_ms * 15) / 1000) as u32`), so DURING the tick it holds the pre-increment frame `N` this tick executes under. FNPC must read it **as the current frame** (the value modulo'd), never a next-frame value. It derives from the serialized/hashed `total_sim_ms`, so it is lockstep-shared by construction — no new hashed state is introduced by threading it.

**Rollout:** HASH-RELEVANT — the chosen cell changes the hashed spawn position. `SNAPSHOT_VERSION 17→18`. This is the design's P6.

**Rollback note (hash-flipping task):**
- The change is isolated to one call site (`production_spawn.rs:290`) + the version constant. To roll back: revert the `find_nearby_passable_cell` call to `nearest_walkable_around` and revert `SNAPSHOT_VERSION` to 17. `find_nearby_passable_cell` (T4) and `cell_linear_index`/`get_cellclass_fallback` (T1) are additive and can stay (they are unreferenced by the hashed path after rollback, so they remain hash-neutral).
- Because the bump is `17→18`, any save written at 18 is rejected by an engine still at 17 (and vice-versa) — there is no silent cross-version load. Capture the pre-flip baseline hash sequence (T8) on the commit BEFORE this task so the parity harness has a 17-era baseline to diff the 18-era replay against deterministically (the baseline asserts internal replay determinism at 18, not 17==18 — the cell choice is intentionally different).

**Verification:**
- `cargo test -p vera20k --lib snapshot::tests::snapshot_version_is_18`
- `cargo test -p vera20k --lib production::production_spawn::tests::find_nearby_selection_uses_frame_counter_modulo`
- `cargo test -p vera20k --lib production::production_spawn::tests::find_nearby_same_tick_aliasing`
- `cargo test -p vera20k --lib production::production_spawn::tests::find_nearby_target_selection_uses_nearest_distance`
- `cargo test -p vera20k --lib production::production_spawn::tests::war_factory_exit_cell_matches_baseline`

---

## T8 — Replay / parity harness (acceptance)

**Goal:** the end-to-end determinism gate — a scripted command stream (build + war-factory exit through the FNPC fallback) replayed twice yields a bit-identical per-tick `state_hash()` sequence, and a captured baseline (at the T7 flip commit) equals the live sequence on replay.

**File to create:** reuse the existing global parity harness (commit `b452c537` "Slice 8 T6: global parity harness (deterministic replay + baseline)") — add a scenario module OR extend the existing harness's scenario list. **Plan-review CONFIRMED (this run): the harness is `src/sim/world/global_parity_harness_tests.rs`, declared `#[path = "global_parity_harness_tests.rs"] mod global_parity_harness_tests;` at `world/mod.rs:2476-2477` — NOT `src/sim/mod.rs` as drafted above.** It uses `ReplayLog`/`ReplayRunner::run` with a committed `GLOBAL_HARNESS_FINAL_HASH` constant (`global_parity_harness_tests.rs:40`), `HARNESS_TICKS = 600`, seed `0xC0FFEE_1234`. Prefer a sibling scenario file `src/sim/world/cell_validation_parity_tests.rs` declared the same `#[path]` way at `world/mod.rs`, reusing `ReplayLog`/`ReplayRunner` — do NOT declare it at `src/sim/mod.rs`.

**Rollout:** acceptance (test-only). Hash-neutral.

**Verification:**
- `cargo test -p vera20k --lib cell_validation_replay_is_bit_identical`
- `cargo test -p vera20k --lib cell_validation_parity_vs_baseline_hash`

---

## Acceptance-test section (deterministic, named)

| Test (named, deterministic) | Proves (ledger) | Task | Hash? |
|---|---|---|---|
| `cell_index_uses_512_wide_stride_not_map_width` | fixed `y*0x200+x`, width-independent (L1) | T1 | no |
| `get_cellclass_oob_returns_dummy_with_requested_coord` | OOB → non-null dummy carrying requested coord, never `None`/(0,0) (L2) | T1 | no |
| `passability_rect_shadow_agrees_with_pathgrid_on_plain_cells` | shadow agreement, divergence surfaced not equalized (P2) | T2 | no |
| `passability_zero_size_rect_returns_true` | `width<=0` → true, no cell read (L4) | T2 | no |
| `cellrect_passability_uses_movement_zone_zone_id_and_speed_type_separately` (existing) | MovementZone-rowed zone-id ≠ SpeedType (L7) | T2 | no |
| `occupancy_blocker_order_matches_engine` | first-blocker scan order a→f, separated `+0x4C`/`+0x11C` (L14/C10) | T3 | no |
| `rect_in_playfield_is_isometric_diamond_inclusive_four_corners` | inclusive 4-corner diamond test, NOT rect bounds (Gate #1 / L19) | T3.5 | no |
| `occupancy_zero_size_rect_still_runs_playfield_corners` | degenerate rect runs corners at decremented `(x-1,y-1)` coords, NOT a no-op (Gate #1 §6 / L16) | T3.5 | no |
| `cellrect_occupancy_minus_one_skips_reservation_but_rejects_cell_blockers` (existing) | `-1` skips `+0xDC`, still rejects blockers (L13/L15) | T3 | no |
| `cellrect_occupancy_house_reservation_blocks_same_house_only` (existing) | `1<<(idx&0x1F)` query rule via InternedId/mask (L15) | T3 | no |
| `find_nearby_diamond_ring_visit_order` | diamond rings + radius cap `min(Speed+Sight,32)` + 24-cap (L20) | T4 | no |
| `find_nearby_calls_occupancy_with_skip_reservation` | occupancy call uses `-1`, never a house index (L23) | T4 | no |
| `find_nearby_allow_bridge_filters_after_passability` | bridge filter AFTER passability (L22) | T4 | no |
| `find_nearby_no_candidate_returns_none` | zero candidates → `None` (engine `{0,0}`) (L27) | T4 | no |
| `find_nearby_passes_required_height_minus_one` | required-height `-1` (L21) | T4 | no |
| `find_nearby_candidate_set_shadows_nearest_walkable_around` | candidate-set shadow vs legacy ring (P5) | T5 | no |
| `spawn_fallback_uses_validator_predicates` | facade predicate single-sourced for spawn (P4) | T6 | no |
| `spawn_fallback_no_hash_change_when_predicates_agree` | invert is hash-neutral (P4) | T6 | no |
| `find_nearby_selection_uses_frame_counter_modulo` | `candidates[binary_frame % count]`, direct-preferred, NOT RNG (L24) | T7 | **YES** |
| `find_nearby_same_tick_aliasing` | same-tick same-count → same index; reproduce, don't spread (L26) | T7 | **YES** |
| `find_nearby_target_selection_uses_nearest_distance` | target path = nearest Euclidean, no frame/RNG (L25) | T7 | **YES** |
| `war_factory_exit_cell_matches_baseline` | exit-cell sequence matches recorded baseline | T7 | **YES** |
| `snapshot_version_is_18` | `SNAPSHOT_VERSION == 18` | T7 | **YES** |
| `cell_validation_replay_is_bit_identical` | ~600-tick scripted stream replayed twice → identical `Vec<hash>` | T8 | acceptance |
| `cell_validation_parity_vs_baseline_hash` | live sequence == captured 18-era baseline (any C1–C20 regression flips a tick hash) | T8 | acceptance |

**Full verification pass (after all tasks):** `cargo check -p vera20k` then `cargo test -p vera20k --lib cell_rect:: find_nearby_cell:: production::production_spawn::tests:: snapshot::tests::snapshot_version_is_18`. Read the literal `test result:` line; the package is **`vera20k`** (a wrong `-p` exits 101 without running).

---

## Hash-relevance summary

- **Hash-neutral (read-only by construction):** T1, T2, T3, T4, T5, T6, T8. The facade only reads; shadow asserts surface divergence without acting. No `state_hash()` bit changes.
- **Hash-relevant (needs shadow precedent + version bump + parity harness):** **T7 only** — FNPC cell choice becomes authoritative → `SNAPSHOT_VERSION 17→18` + the T8 replay/baseline harness. Rollback note in T7.
- **Deferred-AI seam (joins the hash on its own future bump, NOT this slice):** `CellReservationGrid` (already in `cell_rect.rs:44`) for `Reservation::House(id)`; `FUN_005060B0` AI-site internals not built.

---

## Assumptions the plan-review MUST verify

1. **Extend `cell_rect.rs`, do NOT create `cell_validation/`.** The design's new-module proposal duplicates already-present code (`check_passability_rect`/`check_occupancy_rect`/`CellReservationGrid` exist at `cell_rect.rs:110/131/44`, declared `mod.rs:69`). Confirm the extend-in-place decision and the file-split trigger (~600 lines).
2. **`binary_frame` is on `Simulation` (field `world/mod.rs:302`), not a `World`-named type;** reachable as `sim.binary_frame` at the spawn entry point (`sim: &mut Simulation`, `production_spawn.rs:69`). Confirm threading it into `find_spawn_cell_near_structure` honors the late-commit "read as current frame N" semantics (`:1742`) — off-by-one here desyncs every FNPC pick.
3. **T0: Gate #1 (`IsRectInPlayfield`) is now CLOSED** — `docs/research/GATE_PLAYFIELD_RECT_RESOLUTION_GHIDRA_REPORT.md`. The corner formula is the inclusive four-corner isometric-diamond test (NOT the current `0 <= c < dim` rect bounds) — T3.5 is READY and rewrites it. The dummy `0x00ABDC50` runtime-init field values (L3, Open Question 3) STAY OPEN but non-blocking — gates T1 only if a human-path caller reads dummy fields; remaining query is `read_memory 0x00ABDC50` at runtime ONLY if such a caller is found.
4. **Gate #3 (L12 speed-table) is now CLOSED** — `docs/research/GATE_SPEEDTYPE_MATRIX_RESOLUTION_GHIDRA_REPORT.md` confirms `(Clear, Track)` PASSES via `[Clear(0)*9 + Track(1)] = 1.0`. T2/T6 are unblocked; keep `speed_type_allows_cell`'s boolean as-is. Structural DRIFT (passability.rs uses the wrong 13×8 zone matrix + drops per-terrain multipliers) is a separate speed/cost follow-up, NOT this slice.
5. **`check_occupancy_rect` pre-existing fusion of `+0x4C`/`+0x11C`** (`cell_rect.rs:158-166`) must be split into the two separately-ordered rejects (T3) to satisfy L14 first-blocker order; confirm no current caller depends on the fused behavior (none found this run; all rejects return the same `false`).
6. **FNPC `check_height` ±2 gate (L21/OQ6)** comparison semantics (against seed level? required height?) were not the subject of Gate #4 (which covered ring shape/order/selection only — the per-candidate ±2/IsOnScreen/CheckPassability pipeline is documented separately in `pathfinding/FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` per the resolution doc §0). The `NearbyQuery::check_height` flag stays a DEFERRED gate in T4 until that pipeline is confirmed — confirm it is not load-bearing for the spawn fallback (spawn passes required-height `-1`, so the ±2 internal gate likely no-ops on the spawn path).
7. **P6 caller scope (Open Question 1):** T7 flips the spawn/exit fallback ONLY; scatter/chrono-return/~38 other FNPC callsites migrate later (all sharing `find_nearby_passable_cell`). Confirm single-caller scope vs all-non-AI-callers-at-once.
8. **T8 harness path** — the global parity harness exists (commit `b452c537`); this run did not read its module path. Confirm before T8 creates/extends a scenario file.
