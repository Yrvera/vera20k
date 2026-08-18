# Bridgehead Anchor Renderer Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** When `BridgeRuntimeCell.bridgehead_anchor_class != Variant0`, render the cell using the matching BridgeSet variant tile_id — closing the visual loop on sim G3's bridgehead direct-damage path so direct ramp fire produces the same anchor-tile transition gamemd's `SetOverlayAndPropagate` writes to `IsoTileTypeIndex (+0x38)`.

**Architecture:** Theater-derived 4-NS + 4-EW variant tile_id table computed at theater load → resolved-terrain pre-classification populates per-cell `bridgehead_anchor_class_at_load` → sim init copies pre-classification → renderer overrides `tile_id` at per-frame visible-instance build time when class != Variant0. Sim has no theater dependency; theater data flows through the resolved-terrain layer.

**Design Doc:** [docs/plans/2026-05-13-bridgehead-anchor-renderer-design.md](2026-05-13-bridgehead-anchor-renderer-design.md)

---

## Grounding Summary

**RE evidence (Ghidra-verified during brainstorm session 2026-05-13):**
- `DAT_00abad30` (NS) and `DAT_00aa1028` (EW) are **scalar integers** holding `BridgeSet`-relative tile_id offsets, read at theater load from `[General] BridgeMiddle1=` / `BridgeMiddle2=` (`Read_Theater_TileSets_INI` @ 0x00545150, write at 0x00545c1e/0x00545c3a). Values in all five retail theater INIs: `BridgeMiddle1=7`, `BridgeMiddle2=12`.
- Entry gate (`ProcessBridgeDamageStateMachine_High` @ 0x00576BA0): `iVar2 = (cell.tile_id - BridgeSet_start) + 1`, fires when `iVar2 ∈ {ABAD30..ABAD30+3}` (NS) or `{AA1028..AA1028+3}` (EW).
- First-hit anchor write (NS line 0x00577701 / EW line 0x0057769b): `anchor.tile_id ← ABAD30 + 2 + BridgeSet_start` = the **4th variant** in the enum's slot ordering (iVar2 = ABAD30+3 → tile_id `BS+9` NS, `BS+14` EW).
- Cross-confirmed by [ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:196-197](../../../ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md).

**Discovered prerequisite (sim parity bug):** Sim G3's `bridgehead_advance_state` writes `BridgeheadAnchorClass::Damaged` (3rd slot, tile_id `BS+8`) on first-hit. gamemd writes the 4th slot (`BS+9`). One-tile-id drift. Fix: change sim to write `AboutToFall`, update enum doc-comment, update 4 test fixtures. Included as **Task 1** of this plan per user approval.

**Existing repo pattern (mirrored by the renderer override):** [src/map/terrain.rs:579-586](../../src/map/terrain.rs#L579-L586) already has a per-frame variant override from sim state for `damaged_variant`. The new tile_id override sits next to it, same shape (Option-chain against `bridge_state.cell(rx,ry)`), one level up (tile_id vs FA2 variant slot).

**INI keys driving behavior:**
- `[General] BridgeSet=N` — already parsed at theater.rs:453. Tileset index for concrete BridgeSet.
- `[General] BridgeMiddle1=N` — **not yet parsed**, plan adds it.
- `[General] BridgeMiddle2=N` — **not yet parsed**, plan adds it.
- Confirmed in all 5 retail theater INIs ([ini/temperat.ini:97-98](../../ini/temperat.ini#L97-L98), snow, urban, desert, lunar — same values).

**Repo state check (git log):** Last 15 commits on touched files. Most recent bridge-system commits land on dev tip `f001a37` (G7 bridgehead registration) and `bf0573a` (sim G3 add `bridgehead_anchor_class` field). No parallel-session activity on theater/resolved_terrain/terrain.rs since the design doc was written. Design premise holds.

**Still unknown:** none load-bearing. The 5th tile (collapse-cascade output `BS+10`) is documented as out-of-scope per design doc R14; sim doesn't write it from any current path.

## Key Technical Decisions

- **Sim parity fix included (Task 1).** Renderer alone cannot deliver gamemd parity because sim writes the wrong enum slot on first-hit. **Confidence:** high. **Source:** Ghidra decompilation 0x00577701 + HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md:185-200.
- **Override location: per-frame in `build_visible_instances`.** Mirrors existing `damaged_variant` pattern; preserves immutable TerrainGrid. **Confidence:** high. **Source:** repo pattern src/map/terrain.rs:579-586.
- **Theater data flows through ResolvedTerrain layer, not directly into sim.** Pre-classification at resolved-terrain build time; sim reads only the resolved cell. **Confidence:** high. **Source:** design doc + sim/render boundary rule from CLAUDE.md.
- **Atlas pre-load extends `collect_used_tiles` injection set.** All 8 variant tile_ids × all sub_tiles loaded at theater init. Variant=0 only (no FA2 a/b/c/d siblings — gamemd doesn't use them for bridgehead variants). **Confidence:** medium. **Source:** inferred from gamemd's static asset access; **flag for /review-plan** if anyone has evidence variants exist.
- **New `map → sim::bridge_state` type-only import.** `src/map/theater.rs` and `src/map/resolved_terrain.rs` will import `BridgeheadAnchorClass` and `Axis` from `crate::sim::bridge_state`. Currently `src/map/` has zero `sim` imports. This is a **new architectural pattern** — type-only dep, no behavior crossover. Alternatives: (A) move both enums to a shared low module like `src/map/bridge_types.rs` and update all sim consumers — cleaner architecturally, ~6 file touches; (B) keep enums in sim and accept the type-only import — pragmatic, no refactor cost. Plan uses (B); flag for /review-plan if (A) is preferred. **Confidence:** medium. **Source:** repo layering convention in CLAUDE.md ("Stable layering rules").
- **Variant `effective_variant=0` when override fires.** The bridgehead-variant tiles ARE themselves the damage progression; no further FA2 sibling swap. **Confidence:** medium-high. **Source:** inferred from gamemd's flat tile_id write; flag for visual check at Task 8.
- **WoodBridgeSet and the collapse 5th tile (BS+10) out of scope.** Per design + brainstorm user choice; LOW state machine doesn't write the field today. **Confidence:** high. **Source:** brainstorm scope approval.

## Open Questions

### Resolved During Planning

- *Where does gamemd populate `DAT_00abad30` / `DAT_00aa1028`?* — `Read_Theater_TileSets_INI` @ 0x00545150 reads them as scalar `[General]` keys. Resolved.
- *Are the values theater-portable or hardcoded per theater?* — Per-theater INI key. Both retail and the desert/lunar mod theaters use 7/12. Resolved.
- *Do the 4 variant tile_ids share a common sub_tile layout, or does each have its own template?* — Each variant tile_id is a separate TMP template; `sub_tile` is preserved across the `SetOverlayAndPropagate` write (only `+0x38` changes). Resolved (R6 in design doc).
- *Is the sim's first-hit `Damaged` write a naming convention issue or a real bug?* — Real bug. The DamageB neighbor progression uses Variant0→Variant1→Damaged as offsets +0/+1/+2; first-hit should be offset +3 (AboutToFall). Resolved during grounding; user approved sim fix.

### Deferred to Implementation

- *Exact `effective_variant` value when the override fires.* Set to `0` initially. If visual diff against gamemd shows wrong art at Task 8 verification, may need to thread `cell.variant` or `damaged_variant` through. Flagged at Task 6 verification.
- *Are bridge anchor variant TMPs sometimes absent in mods?* — `inject_bridge_anchor_variant_tiles` must handle missing files gracefully (log once, skip). Verify at Task 7 with a synthetic theater missing a variant TMP.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) | Sim parity fix (first-hit → AboutToFall); init reads pre-classification |
| Modify | [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs) | Test fixture update for parity fix |
| Modify | [src/map/theater.rs](../../src/map/theater.rs) | Parse `BridgeMiddle1/2`; add `BridgeAnchorVariantTable`; add `inject_bridge_anchor_variant_tiles` helper |
| Modify | [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) | Add `bridgehead_anchor_class_at_load` field; pre-classification pass |
| Modify | [src/map/terrain.rs](../../src/map/terrain.rs) | TerrainGrid carries variant table; new tile_id override in `build_visible_instances` |
| Modify | [src/app_init.rs](../../src/app_init.rs) | Thread variant table into `build_terrain_grid_from_resolved` call |
| Modify | [src/app_init_helpers.rs](../../src/app_init_helpers.rs) | Call `inject_bridge_anchor_variant_tiles` in atlas pre-load |

No files created (all changes additive to existing modules).

## Interface Changes

**New public surface:**
- `TheaterData::bridge_middle_1: Option<u8>`, `bridge_middle_2: Option<u8>` (additive).
- `theater::BridgeAnchorVariantTable` struct + 3 methods (`from_theater`, `tile_id_for`, `match_tile_id`).
- `theater::inject_bridge_anchor_variant_tiles` helper function.
- `ResolvedTerrainCell::bridgehead_anchor_class_at_load: Option<BridgeheadAnchorClass>` (additive).
- `TerrainGrid::anchor_variant_table: Option<BridgeAnchorVariantTable>` (additive).

**Modified signatures:**
- `build_terrain_grid_from_resolved(resolved, local_bounds)` → adds `anchor_variant_table: Option<BridgeAnchorVariantTable>` parameter.

**Consumers updated:**
- `src/app_init.rs:322` — sole caller of `build_terrain_grid_from_resolved`. Pass through.
- `src/map/terrain.rs:746` — existing test, passes `None`.

## Sim Checklist

(Task 1 + Task 5 touch sim/.)

- [x] No new f32/f64 — only an enum-value change.
- [x] No new sim state — uses existing `bridgehead_anchor_class` field (already in state hash).
- [x] No new sim dependency on render/ui/sidebar/audio/net — pre-classification flows through `ResolvedTerrainCell`, which sim already reads.
- [x] No tick-ordering impact — both touched paths are init-time (Task 5) or single-cell write (Task 1).
- [x] BTreeMap iteration order — not relevant; bridge cells stored in a flat `Vec<Option<_>>`.

## Risk Areas

From design doc + grounding:

- **State-hash drift on maps with pre-damaged anchors** — Task 5 changes initial sim state for any map that authors an anchor in a variant tile_id. Mitigation: world-hash tests don't currently use pre-damaged anchors (verified via grep); add a focused state-hash test at Task 5.
- **Atlas miss** — Task 7's pre-load must enumerate sub_tiles correctly. Mitigation: explicit test that walks 4 NS + 4 EW tile_ids and asserts the atlas has at least one valid `TileKey` per tile_id.
- **`effective_variant=0` correctness** — Open question deferred to Task 8 visual check. Risk: low; bridgehead variants are flat TMPs without baked sub-variants in retail data.
- **Theater INI missing keys** — Variant table is `None`; renderer override disabled; cell renders native tile_id. Mitigation: graceful Option propagation throughout; verified at Task 2 with absent-key test.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | First-hit anchor write target | Anchor renders the right damage-progression tile on direct ramp fire. Off-by-one variant = wrong art every time a unit fires on a bridge ramp. | Ghidra 0x00577701 + in-game observation after Task 8 |
| Task 5 | Map-load anchor classification | Maps authoring pre-damaged anchors must render damaged from frame 1 (gamemd reads +0x38 at load). | Synthetic-map test asserting pre-damaged anchor renders AboutToFall sprite |
| Task 6 | `effective_variant` passed to atlas | Wrong variant arg → atlas returns a/b/c/d sibling TMP that visually doesn't match gamemd's pristine variant for the damaged anchor | Visual diff at Task 8 |
| Task 6 | `sub_tile` preservation in override | gamemd's `SetOverlayAndPropagate` only writes +0x38 (tile_id); preserving sub_tile keeps cells within a multi-cell anchor template visually aligned | Unit test on the override branch |
| Task 7 | Atlas pre-load of all 4 NS + 4 EW × all sub_tiles | Atlas miss = blank cell instantly when damage hits, visible to player on the same tick | Test verifies post-pre-load atlas has variant tile_ids loaded |

---

## Tasks

### Task 1: Sim parity fix — first-hit anchor writes AboutToFall, not Damaged

**Why:** Sim G3 writes `Damaged` (3rd slot, tile_id `BS+8`) but gamemd writes the 4th slot (`BS+9`, `AboutToFall`). Without this fix, the renderer in subsequent tasks would faithfully render the wrong tile_id. This is the discovered prerequisite from grounding.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) (enum doc lines 148-171, driver doc + write at lines 1262-1336)
- Modify: [src/sim/world/world_orders_bridge_repair_tests.rs](../../src/sim/world/world_orders_bridge_repair_tests.rs) (assertion at line 862-867)

**Pattern:** N/A — single-enum-value correction.

**Step 1: Rewrite the `BridgeheadAnchorClass` enum doc-comment (mod.rs:148-171).**

Replace lines 148-171 with:

```rust
/// Per-cell anchor tile-class for bridgehead-adjacent cells.
///
/// Mirrors the four `IsoTileTypeIndex` slots used by the bridgehead state
/// machine. Each value corresponds to a BridgeSet-relative tile_id offset
/// (slot 0..3); the actual tile_ids are theater-portable via
/// `BridgeMiddle1` / `BridgeMiddle2`.
///
/// - `Variant0` — pristine bridgehead (map-load default for cells with no
///   author-damaged anchor placement).
/// - `Variant1` — first DamageB intermediate. Reached only via neighbor
///   `UpdateRamp_*_DamageB` progression on a Variant0 target.
/// - `Damaged` — second DamageB intermediate. Reached only via neighbor
///   `UpdateRamp_*_DamageB` progression on a Variant1 target. Also written
///   by Collapse* paths advancing any non-AboutToFall variant.
/// - `AboutToFall` — most-damaged variant. Two reach paths:
///   1. **Direct hit on a bridgehead cell** — the bridgehead state machine
///      writes the anchor straight to this slot (skipping Variant1/Damaged).
///   2. **Map-load author-damaged anchor** — maps may place this tile_id
///      directly; the renderer reflects it from frame 1.
///
/// Meaningful only when `BridgeRuntimeCell.role` is `Anchor` or
/// `Bridgehead`; the renderer ignores it on other roles.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum BridgeheadAnchorClass {
    #[default]
    Variant0,
    Variant1,
    Damaged,
    AboutToFall,
}
```

**Step 2: Update `bridgehead_advance_state` driver doc-comment (mod.rs:1262-1289).**

Find and replace the existing docstring above `pub fn bridgehead_advance_state` (~line 1290). Replace lines 1262-1289 with:

```rust
/// Bridgehead-cell state-machine driver.
///
/// Sparse-by-design: most bridgehead cells absorb damage via the per-axis
/// start-cell gate inside `bridgehead_walk_to_anchor` (NS rejects odd
/// heights; EW rejects heights > 4). Only the small subset that passes
/// the gate reaches the anchor-write path.
///
/// On a successful walk:
/// - Writes `bridgehead_anchor_class = AboutToFall` on the anchor cell.
///   This is the **most-damaged variant** (4th slot in the enum, matching
///   the reference engine's anchor-tile write target). The write is
///   idempotent — repeat hits leave the anchor at AboutToFall.
/// - Fires `update_ramp_perpendicular(DamageA)` and `DamageB` on the
///   anchor's perpendicular neighbors. These do both the existing
///   state-byte bump (on Anchor targets) AND the asymmetric A/B
///   tile-class progression (on Anchor and Bridgehead targets) —
///   `Variant0 → Variant1 → Damaged` via DamageB; DamageA preserves.
/// - The hit bridgehead cell's own `damage_state` is NEVER modified.
///
/// Returns:
/// - `StateOutcome::Absorbed` on a successful walk + anchor write.
/// - `StateOutcome::NoChange` on role mismatch, missing axis, gated
///   start cell, or walk-off-map.
/// - **Never** returns `Collapsed`. Sustained bridgehead direct fire
///   cannot collapse a bridge on this path; the body-cell cascade
///   (via `body_cell_advance_state`) is the only collapse route.
///
/// `is_high_bridge` is currently unused (state transitions identical
/// for HIGH and LOW per HIGH §11.1) but kept for API symmetry.
///
/// Height-source: `ResolvedTerrainCell.template_height`.
pub fn bridgehead_advance_state(
```

**Step 3: Fix the first-hit write (mod.rs:1331-1336).**

Replace lines 1331-1336 with:

```rust
        // 4. Write the anchor's bridgehead_anchor_class to AboutToFall
        //    (the most-damaged variant, 4th enum slot). Matches the
        //    reference engine's first-hit write to the anchor's tile-class
        //    field. The write is idempotent on repeat hits (AboutToFall
        //    stays AboutToFall). The hit bridgehead cell's own
        //    damage_state is never touched.
        if let Some(anchor_cell) = self.cell_mut(anchor_pos.0, anchor_pos.1) {
            anchor_cell.bridgehead_anchor_class = BridgeheadAnchorClass::AboutToFall;
        }
```

**Step 4: Update direct-hit test assertions (mod.rs:2649-2660, 2675-2690, 2791-2807).**

At mod.rs:2657-2660, replace:

```rust
        // Anchor's bridgehead_anchor_class becomes Damaged.
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Damaged
        );
```

with:

```rust
        // Anchor's bridgehead_anchor_class becomes AboutToFall (4th slot —
        // first-hit writes the most-damaged variant directly, skipping
        // intermediate slots).
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall
        );
```

At mod.rs:2685-2690 (the 100x repeat-hit test), replace:

```rust
        // Anchor's tile class stays Damaged (idempotent).
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Damaged
        );
```

with:

```rust
        // Anchor's tile class stays AboutToFall (idempotent across hits).
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall
        );
```

At mod.rs:2801-2807 (the mid-walk odd-intermediate test), replace:

```rust
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::Damaged,
            "walk must pass through odd-h intermediate and damage the anchor",
        );
```

with:

```rust
        let outcome = state.bridgehead_advance_state(2, 4, true, &terrain);
        assert_eq!(outcome, StateOutcome::Absorbed);
        assert_eq!(
            state.cell(2, 2).unwrap().bridgehead_anchor_class,
            BridgeheadAnchorClass::AboutToFall,
            "walk must pass through odd-h intermediate and damage the anchor",
        );
```

**Step 5: Update the orchestrator integration test (world_orders_bridge_repair_tests.rs:862-867).**

Replace lines 862-867 with:

```rust
    // Anchor's bridgehead_anchor_class = AboutToFall (idempotent across
    // hits). Matches the reference engine's first-hit anchor-tile write
    // target — the most-damaged variant, 4th enum slot.
    assert_eq!(
        bs.cell(2, 2).unwrap().bridgehead_anchor_class,
        BridgeheadAnchorClass::AboutToFall,
        "anchor tile-class must transition to AboutToFall on first hit",
    );
```

**Step 6: Verify.**

Run: `cargo test -p vera20k --lib bridgehead_advance -- --nocapture`
Expected: PASS (3 tests).

Run: `cargo test -p vera20k --lib bridgehead_damage_progression -- --nocapture`
Expected: PASS (or whatever the relevant world_orders test name is).

Run: `cargo test -p vera20k --lib bridge_state`
Expected: PASS (full bridge_state test suite).

**Step 7: Commit.**

```
sim/bridge_state: fix first-hit anchor write to AboutToFall (parity)

gamemd's ProcessBridgeDamageStateMachine_High writes the 4th BridgeSet
variant slot on first hit (ABAD30+2+BridgeSet = the AboutToFall enum
slot in our model). Sim G3 was writing the 3rd slot (Damaged), causing
a one-tile-id visual drift the renderer can't compensate for.

Fix the writer, update the enum doc-comment to match the actual
semantics (Damaged is now correctly documented as a DamageB neighbor
intermediate, not the direct-hit target), and update 4 test fixtures
that baked in the wrong assertion.

Prereq for the renderer follow-up that reads bridgehead_anchor_class
to pick the rendered tile_id.
```

---

### Task 2: Parse BridgeMiddle1 / BridgeMiddle2 from theater INI

**Why:** Theater data must expose the two scalars that drive the variant table. Foundation for Tasks 3, 6, 7.

**Files:**
- Modify: [src/map/theater.rs](../../src/map/theater.rs) (struct `TheaterData` ~line 362, `load_theater` body ~line 449-462)

**Pattern:** Mirror the existing `bridge_set` / `wood_bridge_set` parsing block at theater.rs:449-462.

**Step 1: Add 2 fields to `TheaterData` (theater.rs:362-379).**

Find `pub struct TheaterData` (around line 362). Locate the `wood_bridge_set` field and insert after it:

```rust
    /// `[General] BridgeMiddle1=N` — BridgeSet-relative offset for the NS
    /// bridgehead variant block. The 4 NS variant tile_ids occupy
    /// `BridgeSet_start + {N-1, N, N+1, N+2}`. None if the key is absent.
    pub bridge_middle_1: Option<u8>,
    /// `[General] BridgeMiddle2=N` — same for EW.
    pub bridge_middle_2: Option<u8>,
```

The final struct fields list (after the existing `wood_bridge_set`) should read:

```rust
    pub bridge_set: Option<u16>,
    pub wood_bridge_set: Option<u16>,
    pub bridge_middle_1: Option<u8>,
    pub bridge_middle_2: Option<u8>,
}
```

**Step 2: Parse the keys in `load_theater` (theater.rs:449-462).**

Find the existing block:

```rust
    let ini_text = String::from_utf8_lossy(&ini_data);
    let bridge_set = parse_general_int(&ini_text, "BridgeSet");
    let wood_bridge_set = parse_general_int(&ini_text, "WoodBridgeSet");
```

Replace with:

```rust
    let ini_text = String::from_utf8_lossy(&ini_data);
    let bridge_set = parse_general_int(&ini_text, "BridgeSet");
    let wood_bridge_set = parse_general_int(&ini_text, "WoodBridgeSet");
    // BridgeMiddle1/2 select which 4 consecutive BridgeSet-relative tile_ids
    // are the NS / EW bridgehead variant blocks. Parsed as u8 because retail
    // values fit (temperate/snow/urban/desert/lunar = 7/12). Use the existing
    // parse_general_int (returns u16) and downcast; None on absent key or
    // out-of-range value.
    let bridge_middle_1: Option<u8> = parse_general_int(&ini_text, "BridgeMiddle1")
        .and_then(|v| u8::try_from(v).ok());
    let bridge_middle_2: Option<u8> = parse_general_int(&ini_text, "BridgeMiddle2")
        .and_then(|v| u8::try_from(v).ok());
```

Then update the existing log block (lines ~455-462) and the `TheaterData` construction:

```rust
    if bridge_set.is_some() || wood_bridge_set.is_some() {
        log::info!(
            "Theater {}: BridgeSet={:?}, WoodBridgeSet={:?}, BridgeMiddle1={:?}, BridgeMiddle2={:?}",
            theater_name,
            bridge_set,
            wood_bridge_set,
            bridge_middle_1,
            bridge_middle_2,
        );
    }
    // Diagnostic: bridge anchor variant rendering needs both BridgeSet
    // AND BridgeMiddle1/2. Log once at theater load if any are missing so
    // mods without these keys produce a clear single warning rather than
    // silent per-frame visual drift.
    if bridge_set.is_some() && (bridge_middle_1.is_none() || bridge_middle_2.is_none()) {
        log::info!(
            "Theater {}: BridgeSet present but BridgeMiddle{} missing — bridgehead anchor damage visuals disabled",
            theater_name,
            match (bridge_middle_1, bridge_middle_2) {
                (None, None) => "1+2",
                (None, _) => "1",
                _ => "2",
            },
        );
    }
```

Add `bridge_middle_1` and `bridge_middle_2` to the `TheaterData { ... }` constructor at the end of `load_theater`:

```rust
    Some(TheaterData {
        lookup,
        iso_palette,
        unit_palette,
        tiberium_palette,
        extension: def.extension,
        ini_data,
        bridge_set,
        wood_bridge_set,
        bridge_middle_1,
        bridge_middle_2,
    })
```

**Step 3: Add unit test.**

Find the test module include at the bottom of theater.rs (line 722-724) — tests live in `theater_tests.rs`. Add a new test there:

```rust
// Append to src/map/theater_tests.rs

#[test]
fn parse_general_int_finds_bridge_middle_keys() {
    // Synthetic theater INI fragment with the relevant [General] keys.
    let ini = "[General]\nBridgeSet=5\nBridgeMiddle1=7\nBridgeMiddle2=12\n\n[TileSet0000]\nTilesInSet=1\nFileName=clear\n";
    let bridge_middle_1 = super::parse_general_int(ini, "BridgeMiddle1");
    let bridge_middle_2 = super::parse_general_int(ini, "BridgeMiddle2");
    assert_eq!(bridge_middle_1, Some(7));
    assert_eq!(bridge_middle_2, Some(12));
}

#[test]
fn parse_general_int_missing_bridge_middle_returns_none() {
    let ini = "[General]\nBridgeSet=5\n\n[TileSet0000]\nTilesInSet=1\nFileName=clear\n";
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle1"), None);
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle2"), None);
}
```

(Note: if `parse_general_int` is `pub(crate)`-scoped or private, both visibility and the `super::` import path may need adjustment. Check theater.rs:479 for the existing visibility.)

**Step 4: Verify.**

Run: `cargo test -p vera20k --lib parse_general_int -- --nocapture`
Expected: PASS (2 new tests + any existing).

Run: `cargo check`
Expected: clean.

**Step 5: Commit.**

```
map/theater: parse BridgeMiddle1/2 from theater [General] section

Foundation for bridge anchor damage rendering. The two scalars are
BridgeSet-relative tile_id offsets that define the 4 NS + 4 EW
bridgehead variant block locations within BridgeSet.

Confirmed values across all 5 retail theaters (temperate, snow,
urban, desert, lunar): BridgeMiddle1=7, BridgeMiddle2=12.

Log one INFO line at theater load when BridgeSet is present but
BridgeMiddle1/2 isn't — mods without these keys get a clear single
warning rather than silent per-frame visual drift.
```

---

### Task 3: BridgeAnchorVariantTable struct + methods

**Why:** Centralize the variant tile_id math in one place. Both the renderer override (Task 6) and the resolved-terrain pre-classification (Task 4) need this table; doing the math twice in two places invites bugs.

**Files:**
- Modify: [src/map/theater.rs](../../src/map/theater.rs) (new struct + impl, near the existing `TheaterData` definition)

**Pattern:** New struct, lives next to `TilesetLookup` / `TheaterData` since it derives from both.

**Step 1: Define the struct + axis import.**

Add near the top of theater.rs (after the existing imports ~line 12-18):

```rust
use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};
```

(Note: this introduces a `theater → sim::bridge_state` dependency. The dependency is purely on the **types** `Axis` and `BridgeheadAnchorClass`, not on any sim logic. If `theater` is below `sim` in the dep graph and this creates a cycle, move the two types to a shared lower module — at write-plan time, check whether `theater` already imports anything from `sim`. If not, prefer extracting `Axis` + `BridgeheadAnchorClass` into a small shared `map/bridge_types.rs` instead.)

Define the struct (place after the `TheaterData` struct, around line 380):

```rust
/// Theater-derived 4-NS + 4-EW tile_id table for HIGH bridge anchor variants.
///
/// Built once at theater load from `BridgeSet` (tileset start tile_id)
/// + `BridgeMiddle1` / `BridgeMiddle2` (BridgeSet-relative offsets).
/// The 4 variant tile_ids per axis occupy consecutive slots starting at
/// `BridgeSet_start + (BridgeMiddle* - 1)`.
///
/// Enum order: `[Variant0, Variant1, Damaged, AboutToFall]`.
#[derive(Debug, Clone, Copy)]
pub struct BridgeAnchorVariantTable {
    /// NS variant tile_ids in enum order (Variant0..AboutToFall).
    pub ns: [u16; 4],
    /// EW variant tile_ids in enum order.
    pub ew: [u16; 4],
}

impl BridgeAnchorVariantTable {
    /// Derive the variant table from a fully-loaded TheaterData.
    ///
    /// Returns None when BridgeSet, BridgeMiddle1, or BridgeMiddle2 is
    /// absent, BridgeMiddle1 or BridgeMiddle2 is 0 (Variant0 = BS+M-1
    /// would underflow), or any of the 8 computed tile_ids falls outside
    /// the tileset bounds.
    pub fn from_theater(td: &TheaterData) -> Option<Self> {
        let bs_idx = td.bridge_set?;
        let m1 = td.bridge_middle_1?;
        let m2 = td.bridge_middle_2?;
        if m1 < 1 || m2 < 1 {
            return None;
        }
        // BridgeSet's first tile_id from the parsed TileSet bounds.
        let bs_start = td
            .lookup
            .bounds()
            .get(bs_idx as usize)
            .map(|b| b.start)?;
        let max_tid = td.lookup.len() as u32;

        // Compute the 8 variant tile_ids; reject if any exceed the
        // tileset's known length.
        let compute_axis = |m: u8| -> Option<[u16; 4]> {
            let base = bs_start as u32 + (m as u32) - 1;
            let highest = base + 3;
            if highest >= max_tid {
                return None;
            }
            Some([
                (base) as u16,
                (base + 1) as u16,
                (base + 2) as u16,
                (base + 3) as u16,
            ])
        };
        let ns = compute_axis(m1)?;
        let ew = compute_axis(m2)?;
        Some(Self { ns, ew })
    }

    /// Look up the tile_id for a (axis, class) pair. Returns None when
    /// class is Variant0 — callers fall through to the cell's native
    /// tile_id in that case (no render-side override needed).
    pub fn tile_id_for(&self, axis: Axis, class: BridgeheadAnchorClass) -> Option<u16> {
        let slot = match class {
            BridgeheadAnchorClass::Variant0 => return None,
            BridgeheadAnchorClass::Variant1 => 1usize,
            BridgeheadAnchorClass::Damaged => 2usize,
            BridgeheadAnchorClass::AboutToFall => 3usize,
        };
        let arr = match axis {
            Axis::NS => &self.ns,
            Axis::EW => &self.ew,
        };
        Some(arr[slot])
    }

    /// Reverse-match a tile_id to (axis, class). Used at map load to
    /// pre-classify author-damaged anchors. None when the tile_id is not
    /// a variant.
    pub fn match_tile_id(&self, tile_id: u16) -> Option<(Axis, BridgeheadAnchorClass)> {
        const CLASS_ORDER: [BridgeheadAnchorClass; 4] = [
            BridgeheadAnchorClass::Variant0,
            BridgeheadAnchorClass::Variant1,
            BridgeheadAnchorClass::Damaged,
            BridgeheadAnchorClass::AboutToFall,
        ];
        if let Some(slot) = self.ns.iter().position(|&t| t == tile_id) {
            return Some((Axis::NS, CLASS_ORDER[slot]));
        }
        if let Some(slot) = self.ew.iter().position(|&t| t == tile_id) {
            return Some((Axis::EW, CLASS_ORDER[slot]));
        }
        None
    }
}
```

**Step 2: Add unit tests** to `theater_tests.rs`:

```rust
// Append to src/map/theater_tests.rs

use crate::map::theater::{BridgeAnchorVariantTable, TheaterData, TilesetLookup};
use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};

/// Helper: build a minimal TheaterData for table-derivation tests.
/// BridgeSet at tileset index 0 with 20 tiles starting at tile_id 0.
fn synthetic_theater_with_bridge_keys(
    bridge_middle_1: Option<u8>,
    bridge_middle_2: Option<u8>,
) -> TheaterData {
    // Use a fully synthetic TilesetLookup with one tileset starting at 0.
    // Direct construction not exposed publicly — use parse_tileset_ini.
    let ini = b"[TileSet0000]\nTilesInSet=20\nFileName=bridge\nSetName=Bridge\n";
    let lookup = crate::map::theater::parse_tileset_ini(ini, "tem").unwrap();
    // Palette has no Default impl; build from 768 zero bytes (VGA 6-bit
    // input → all-black palette is fine for tests that never read pixels).
    let empty_palette = crate::assets::pal_file::Palette::from_bytes(&[0u8; 768])
        .expect("768-byte zero palette parses");
    TheaterData {
        lookup,
        iso_palette: empty_palette.clone(),
        unit_palette: empty_palette.clone(),
        tiberium_palette: empty_palette,
        extension: "tem",
        ini_data: Vec::new(),
        bridge_set: Some(0),
        wood_bridge_set: None,
        bridge_middle_1,
        bridge_middle_2,
    }
}

#[test]
fn variant_table_temperate_values() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).expect("table");
    // BridgeSet starts at tile_id 0 (TilesInSet=20, first tileset). NS
    // variants: BS + M1 + {-1, 0, 1, 2} = {6, 7, 8, 9}. EW: {11, 12, 13, 14}.
    assert_eq!(table.ns, [6, 7, 8, 9]);
    assert_eq!(table.ew, [11, 12, 13, 14]);
}

#[test]
fn variant_table_returns_none_on_missing_middle_1() {
    let td = synthetic_theater_with_bridge_keys(None, Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_missing_middle_2() {
    let td = synthetic_theater_with_bridge_keys(Some(7), None);
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_zero_middle() {
    let td = synthetic_theater_with_bridge_keys(Some(0), Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_out_of_bounds() {
    // TilesInSet=20 → max tile_id 19. BridgeMiddle1=18 would push the
    // 4th variant to 19+0+(-1)+3 = 20 (oob).
    let td = synthetic_theater_with_bridge_keys(Some(18), Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn tile_id_for_variant0_returns_none() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Variant0),
        None
    );
    assert_eq!(
        table.tile_id_for(Axis::EW, BridgeheadAnchorClass::Variant0),
        None
    );
}

#[test]
fn tile_id_for_each_class_per_axis() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Variant1),
        Some(7)
    );
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Damaged),
        Some(8)
    );
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::AboutToFall),
        Some(9)
    );
    assert_eq!(
        table.tile_id_for(Axis::EW, BridgeheadAnchorClass::AboutToFall),
        Some(14)
    );
}

#[test]
fn match_tile_id_round_trip_all_variants() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    for (axis, expected_arr) in [(Axis::NS, &table.ns), (Axis::EW, &table.ew)] {
        for (slot, &tid) in expected_arr.iter().enumerate() {
            let (got_axis, got_class) = table.match_tile_id(tid).expect("matched");
            assert_eq!(got_axis, axis);
            assert_eq!(
                got_class,
                [
                    BridgeheadAnchorClass::Variant0,
                    BridgeheadAnchorClass::Variant1,
                    BridgeheadAnchorClass::Damaged,
                    BridgeheadAnchorClass::AboutToFall,
                ][slot]
            );
        }
    }
}

#[test]
fn match_tile_id_rejects_non_variant() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    // BS+5 (one before Variant0 NS), BS+10 (between NS and EW), BS+15
    // (post-AboutToFall EW), 999 (outside BridgeSet).
    assert_eq!(table.match_tile_id(5), None);
    assert_eq!(table.match_tile_id(10), None);
    assert_eq!(table.match_tile_id(15), None);
    assert_eq!(table.match_tile_id(999), None);
}
```

(Note: `Palette` has no `Default` impl; the helper uses `Palette::from_bytes(&[0u8; 768]).expect(...)` above to build an empty palette. If the executor sees a `from_bytes` signature change in pal_file.rs that breaks this, adapt accordingly.)

**Step 3: Verify.**

Run: `cargo test -p vera20k --lib variant_table -- --nocapture`
Expected: PASS (8 tests).

Run: `cargo test -p vera20k --lib tile_id_for match_tile_id`
Expected: PASS.

Run: `cargo check`
Expected: clean.

**Step 4: Commit.**

```
map/theater: add BridgeAnchorVariantTable

Centralizes the 4-NS + 4-EW bridgehead variant tile_id math:
  Variant0..AboutToFall tile_ids = BridgeSet_start + BridgeMiddle*
  + {-1, 0, 1, 2}.

Two consumers will use this: the resolved-terrain pre-classification
(author-damaged anchors) and the per-frame renderer override.
Centralizing the math avoids duplicating the +1 / -1 offsets between
the gate (entry direction) and the storage (enum slot direction).

Defensive defaults: None on missing keys, M1/M2 == 0 (underflow), or
any variant tile_id outside the tileset's bounds.
```

---

### Task 4: ResolvedTerrainCell pre-classification field + build-time pass

**Why:** Captures author-damaged anchor placements at map load (R9). Sim then reads this in Task 5 without taking a theater dependency. Closes the parity hole where maps shipping pre-damaged anchors would render as undamaged until first hit.

**Files:**
- Modify: [src/map/resolved_terrain.rs](../../src/map/resolved_terrain.rs) (struct ~line 68-135, build pass ~line 605-611)

**Pattern:** Mirror the existing post-build mutation pattern in `ResolvedTerrain::build` that detects bridgehead cells (~line 544-605).

**Step 1: Add the field to `ResolvedTerrainCell`.**

Add to the struct (after `has_damaged_data` at line 134, before the closing brace):

```rust
    /// Author-damaged anchor pre-classification: `Some(class)` if this
    /// cell's `final_tile_index` matches one of the 8 bridgehead anchor
    /// variant tile_ids in the current theater's BridgeAnchorVariantTable.
    /// `None` when not a variant tile (the common case for both
    /// non-bridge cells and pristine anchor cells).
    ///
    /// Sim's BridgeRuntimeState::from_resolved_terrain reads this to
    /// initialize BridgeRuntimeCell.bridgehead_anchor_class instead of
    /// the unconditional Variant0 default. None defaults to Variant0
    /// sim-side.
    pub bridgehead_anchor_class_at_load: Option<crate::sim::bridge_state::BridgeheadAnchorClass>,
```

**Step 2: Initialize the field to `None` in the per-cell constructor.**

Find the `cells.push(ResolvedTerrainCell { ... })` block (~line 378-423). Add `bridgehead_anchor_class_at_load: None,` to the field initializer list — alphabetical or with other Bridge-related fields. Suggested placement after `has_damaged_data: metadata.has_damaged_data,` (line 422):

```rust
                    has_damaged_data: metadata.has_damaged_data,
                    bridgehead_anchor_class_at_load: None,
                });
```

(Also update any other `ResolvedTerrainCell { ... }` construction sites — search via grep to confirm; the gap-fill pass and the side-cell extrapolation pass may construct cells too. See lines ~432-525 of the same file.)

**Step 3: Add the pre-classification pass to `ResolvedTerrain::build`.**

After the existing bridgehead detection pass (~line 599-605) and **before** the gap-fill pass (~line 611), add:

```rust
        // Pre-classify author-damaged anchor placements: cells whose
        // tileset is BridgeSet AND whose final_tile_index matches one of
        // the 4 NS or 4 EW variant tile_ids get a non-None
        // bridgehead_anchor_class_at_load. Sim's bridge-state init reads
        // this so maps that author pre-damaged anchors render correctly
        // from frame 1.
        if let Some(td) = theater_data {
            if let Some(table) = crate::map::theater::BridgeAnchorVariantTable::from_theater(td) {
                if let Some(bs_idx) = td.bridge_set {
                    for cell in cells.iter_mut() {
                        // Only cells in BridgeSet are candidates.
                        if cell.tileset_index != Some(bs_idx) {
                            continue;
                        }
                        // Skip sentinel tile_ids (NO_TILE = -1, 0xFFFF
                        // legacy sentinel).
                        if cell.final_tile_index < 0 {
                            continue;
                        }
                        let tid = if cell.final_tile_index == 0xFFFF {
                            0
                        } else {
                            cell.final_tile_index as u16
                        };
                        if let Some((_axis, class)) = table.match_tile_id(tid) {
                            cell.bridgehead_anchor_class_at_load = Some(class);
                        }
                    }
                }
            }
        }
```

**Step 4: Add unit tests.**

`resolved_terrain.rs` has a test module — find it or add one. If a `#[cfg(test)] mod tests` doesn't already exist, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::bridge_state::BridgeheadAnchorClass;

    // Synthetic ResolvedTerrainCell builder for testing the pre-classification pass.
    fn make_cell(rx: u16, ry: u16, tile_index: i32, tileset_index: Option<u16>) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: tile_index,
            source_sub_tile: 0,
            final_tile_index: tile_index,
            final_sub_tile: 0,
            level: 0,
            filled_clear: false,
            tileset_index,
            land_type: 0,
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
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    #[test]
    fn pre_classify_pristine_bridge_cell_leaves_at_load_none() {
        // A BridgeSet cell at the non-variant slot 0 (anchor base tile).
        let cell = make_cell(0, 0, 100, Some(0));
        // Direct call to the matcher logic (refactor the pre-classification
        // block into a free function for unit-testability OR test through
        // ResolvedTerrain::build with a synthetic map). For now, assert
        // the initial state.
        assert_eq!(cell.bridgehead_anchor_class_at_load, None);
    }

    // (Integration-style test exercising ResolvedTerrain::build with a
    // synthetic 1-cell map placed at a variant tile_id would be ideal but
    // requires plumbing a MapFile + TheaterData fixture. Defer to Task 8
    // end-to-end verification if a focused unit test proves expensive.)
}
```

**Step 5: Verify.**

Run: `cargo test -p vera20k --lib resolved_terrain -- --nocapture`
Expected: PASS.

Run: `cargo check`
Expected: clean (the new field is `Option`-typed so all serde-derive and existing construction sites compile cleanly with `None`).

**Step 6: Commit.**

```
map/resolved_terrain: pre-classify author-damaged anchor tiles

ResolvedTerrainCell gains bridgehead_anchor_class_at_load: Option<_>.
Build-time pass scans BridgeSet cells and reverse-matches their
final_tile_index against the theater's BridgeAnchorVariantTable.

This closes the parity hole where maps that author pre-damaged anchor
tile_ids (e.g., damaged-from-start bridge in a campaign map) would
have rendered as undamaged in our engine until the first weapon hit,
then jumped to the damaged sprite. With this field, sim init in the
follow-up task picks up the correct starting class.
```

---

### Task 5: Sim init reads pre-classification (replace Variant0 default)

**Why:** Wire Task 4's `bridgehead_anchor_class_at_load` into sim's `from_resolved_terrain`. Without this, the pre-classification work is dead data and pre-damaged anchors still render as undamaged.

**Files:**
- Modify: [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs) (one init site at line 549, plus walker.rs equivalents)
- Modify: [src/sim/bridge_state/walker.rs](../../src/sim/bridge_state/walker.rs) (lines 955, 1164, 1206, 1288 — all in `#[cfg(test)] mod tests`; all are test fixtures, see Step 2 below)

**Pattern:** Mirror the existing `from_resolved_terrain` reads of `ResolvedTerrainCell` fields (e.g., `resolved.bridge_deck_level`, `resolved.bridge_layer`).

**Step 1: Update `BridgeRuntimeState::from_resolved_terrain` site at mod.rs:549.**

The current code reads:

```rust
                    has_damaged_data: false,
                    bridgehead_anchor_class: BridgeheadAnchorClass::Variant0,
```

Change line 549 to:

```rust
                    bridgehead_anchor_class: resolved
                        .bridgehead_anchor_class_at_load
                        .unwrap_or(BridgeheadAnchorClass::Variant0),
```

Where `resolved` is the loop-iterated `ResolvedTerrainCell` (verify the variable name in context — likely `resolved` or `cell`).

**Step 2: Update walker.rs init sites (verified test-only — leave as Variant0).**

Pre-verified: the 4 sites at walker.rs:955, 1164, 1206, 1288 are all inside `#[cfg(test)] mod tests` — they're synthetic test builders (`seed_high_body_cell` / `seed_low_body_cell` helpers and inline `BridgeRuntimeCell { ... }` constructions in `#[test]` functions). None of these have access to a `ResolvedTerrainCell`. **Leave them all at `Variant0`** — they're testing isolated behavior where the default is the correct starting state.

If the executor wants to double-check (line numbers can drift), run:

```
grep -n "bridgehead_anchor_class: BridgeheadAnchorClass::Variant0" src/sim/bridge_state/walker.rs
```

Confirm each result is inside `#[cfg(test)]` before leaving it as-is. No code change required at any walker.rs site.

**Step 3: Add a focused unit test for the init path.**

In bridge_state/mod.rs tests (~line 2415 onward where the test module lives), add:

```rust
    #[test]
    fn from_resolved_terrain_copies_pre_damaged_anchor_class() {
        use crate::map::resolved_terrain::{
            ResolvedTerrainCell, ResolvedTerrainGrid,
        };
        use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};

        // 3x3 grid; cell (1,1) is a BridgeSet anchor with
        // bridgehead_anchor_class_at_load = AboutToFall (author-damaged).
        let mut cells = Vec::with_capacity(9);
        for ry in 0..3u16 {
            for rx in 0..3u16 {
                let pre_class = if rx == 1 && ry == 1 {
                    Some(BridgeheadAnchorClass::AboutToFall)
                } else {
                    None
                };
                cells.push(ResolvedTerrainCell {
                    rx, ry,
                    source_tile_index: 0, source_sub_tile: 0,
                    final_tile_index: 0, final_sub_tile: 0,
                    level: 0, filled_clear: false,
                    tileset_index: None,
                    land_type: 0, slope_type: 0, template_height: 0,
                    render_offset_x: 0, render_offset_y: 0,
                    terrain_class: TerrainClass::Clear,
                    speed_costs: SpeedCostProfile::default(),
                    is_water: false, is_cliff_like: false, is_cliff_redraw: false,
                    variant: 0, is_rough: false, is_road: false,
                    accepts_smudge: false,
                    has_ramp: false, canonical_ramp: None,
                    ground_walk_blocked: false,
                    terrain_object_blocks: false,
                    overlay_blocks: false,
                    zone_type: 0,
                    base_ground_walk_blocked: false,
                    base_build_blocked: false,
                    build_blocked: false,
                    has_bridge_deck: rx == 1 && ry == 1,
                    bridge_walkable: rx == 1 && ry == 1,
                    bridge_transition: false,
                    bridge_deck_level: 0,
                    bridge_layer: None,
                    radar_left: [0, 0, 0],
                    radar_right: [0, 0, 0],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: pre_class,
                });
            }
        }
        let terrain = ResolvedTerrainGrid::from_cells(3, 3, cells);
        let state = BridgeRuntimeState::from_resolved_terrain(&terrain, true, 1500);
        // The author-damaged cell at (1,1) should have AboutToFall, not Variant0.
        let cell = state.cell(1, 1).expect("bridge cell");
        assert_eq!(cell.bridgehead_anchor_class, BridgeheadAnchorClass::AboutToFall);
    }

    #[test]
    fn from_resolved_terrain_defaults_to_variant0_when_pre_class_is_none() {
        // Same setup as above but with bridgehead_anchor_class_at_load=None.
        // Cell at (1,1) should default to Variant0.
        // (Implementation similar to above with pre_class always None.)
        // ... (mirror the above test with pre_class = None)
    }
```

**Step 4: Verify.**

Run: `cargo test -p vera20k --lib from_resolved_terrain -- --nocapture`
Expected: PASS.

Run: `cargo test -p vera20k --lib bridge_state`
Expected: PASS (regression).

Run: `cargo test -p vera20k --lib`
Expected: PASS (full test suite — guards against world-hash regression on test fixtures).

**Step 5: Commit.**

```
sim/bridge_state: init bridgehead_anchor_class from resolved-terrain pre-classification

BridgeRuntimeState::from_resolved_terrain now reads
ResolvedTerrainCell.bridgehead_anchor_class_at_load instead of
unconditionally defaulting to Variant0. Falls back to Variant0 when
the resolved cell carries None (the common case for pristine cells
and non-BridgeSet tiles).

Net effect: maps that author pre-damaged anchor tile_ids now reach
sim with the correct enum state from frame 1, instead of starting at
Variant0 and "snapping" to damaged on first hit.

State-hash impact: maps using pre-damaged anchors produce a different
initial hash than the prior build. No retail YR map verified to use
this pattern; impact is forward-only and deterministic. Existing
test fixtures unaffected (verified via the test suite).
```

---

### Task 6: TerrainGrid carries the variant table + per-frame tile_id override

**Why:** Main renderer change. The override is the gate that swaps `cell.tile_id` for the variant's tile_id when sim reports a non-Variant0 class. Mirrors the existing `damaged_variant` pattern one level up.

**Files:**
- Modify: [src/map/terrain.rs](../../src/map/terrain.rs) (TerrainGrid struct ~line 169-182, `build_terrain_grid_from_resolved` ~line 424-495, `build_visible_instances` ~line 524-623, test at ~line 700-756)
- Modify: [src/app_init.rs](../../src/app_init.rs) (call site at line 322)

**Pattern:** Mirror existing `damaged_variant` override at terrain.rs:579-586. New override sits adjacent (just below the existing one) and replaces the local `tile_id` and `variant` arguments before the `uv_fn` call.

**Step 1: Add field to `TerrainGrid` (terrain.rs:169-182).**

Find:

```rust
pub struct TerrainGrid {
    pub cells: Vec<TerrainCell>,
    pub world_width: f32,
    pub world_height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub local_bounds: Option<LocalBounds>,
}
```

Replace with:

```rust
pub struct TerrainGrid {
    pub cells: Vec<TerrainCell>,
    pub world_width: f32,
    pub world_height: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    pub local_bounds: Option<LocalBounds>,
    /// Theater-derived bridge anchor variant tile_ids, threaded from
    /// TheaterData at map-load. None when theater lacks BridgeMiddle1/2
    /// keys — renderer override is then bypassed.
    pub anchor_variant_table: Option<crate::map::theater::BridgeAnchorVariantTable>,
}
```

**Step 2: Update `build_terrain_grid_from_resolved` signature + initializer.**

At line 424, replace the function signature:

```rust
pub fn build_terrain_grid_from_resolved(
    resolved: &ResolvedTerrainGrid,
    local_bounds: Option<LocalBounds>,
) -> TerrainGrid {
```

with:

```rust
pub fn build_terrain_grid_from_resolved(
    resolved: &ResolvedTerrainGrid,
    local_bounds: Option<LocalBounds>,
    anchor_variant_table: Option<crate::map::theater::BridgeAnchorVariantTable>,
) -> TerrainGrid {
```

Then at the end of the function (the existing `TerrainGrid { ... }` construction ~line 487-494), add the new field:

```rust
    TerrainGrid {
        cells,
        world_width: max_x - min_x,
        world_height: max_y - min_y,
        origin_x: min_x,
        origin_y: min_y,
        local_bounds,
        anchor_variant_table,
    }
```

Also update the other constructor `build_terrain_grid` at the top of the file (the legacy non-resolved variant ~line 345-418, if it returns a `TerrainGrid` too) — pass `anchor_variant_table: None`. Search for `TerrainGrid {` to find all construction sites.

**Step 3: Update the in-file test fixture at terrain.rs:700-756.**

The test constructs `TerrainGrid { ... }` directly. Add `anchor_variant_table: None,` to the field-init list. Also update line 746 if it calls `build_visible_instances` with explicit args.

**Step 4: Update the renderer override in `build_visible_instances`.**

Find the existing `damaged_variant` block at terrain.rs:579-595:

```rust
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
            None => Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            }),
        };
```

Replace with:

```rust
        // Existing damaged_variant override (FA2 a/b/c/d sibling-TMP swap).
        let damaged_variant_swap: u8 = if cell.has_damaged_data {
            bridge_state
                .and_then(|bs| bs.cell(cell.rx, cell.ry))
                .map(|bc| bc.damaged_variant as u8)
                .unwrap_or(0)
        } else {
            cell.variant
        };

        // New: bridge anchor tile_id override. Fires when sim reports a
        // non-Variant0 bridgehead_anchor_class AND the theater carries
        // BridgeMiddle1/2. Swaps the cell's tile_id for the variant's
        // tile_id; sub_tile is preserved (only +0x38 changes in the
        // reference engine). When the override fires, the variant tile's
        // FA2 sibling-TMP slot is set to 0 (no damaged_variant a/b/c/d
        // swap — variant tiles ARE the damage progression).
        let anchor_override = grid.anchor_variant_table.and_then(|table| {
            let bc = bridge_state?.cell(cell.rx, cell.ry)?;
            let axis = bc.axis?;
            table.tile_id_for(axis, bc.bridgehead_anchor_class)
        });

        let (effective_tile_id, effective_variant) = match anchor_override {
            Some(tid) => (tid, 0u8),
            None => (cell.tile_id, damaged_variant_swap),
        };

        let placement: Option<TilePlacement> = match &uv_fn {
            Some(f) => f(effective_tile_id, cell.sub_tile, effective_variant),
            None => Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            }),
        };
```

**Step 5: Update the call site in `app_init.rs:322`.**

Replace:

```rust
    let mut grid: TerrainGrid =
        terrain::build_terrain_grid_from_resolved(&resolved_terrain, local_bounds);
```

with:

```rust
    let anchor_variant_table = theater_data
        .as_ref()
        .and_then(crate::map::theater::BridgeAnchorVariantTable::from_theater);
    let mut grid: TerrainGrid = terrain::build_terrain_grid_from_resolved(
        &resolved_terrain,
        local_bounds,
        anchor_variant_table,
    );
```

(Verify the local name `theater_data` matches the surrounding context at app_init.rs — adjust if it's `theater` or `td`.)

**Step 6: Add a focused renderer test for the override.**

In the `#[cfg(test)] mod tests` block at terrain.rs:625+, add:

```rust
    #[test]
    fn override_fires_when_class_is_aboutto_fall_with_table() {
        use crate::map::theater::BridgeAnchorVariantTable;
        use crate::sim::bridge_state::{
            Axis, BridgeRuntimeCell, BridgeRuntimeState, BridgeCellRole,
            BridgeheadAnchorClass, DamageState,
        };

        // 1x1 grid with a single TerrainCell having tile_id=100 (native).
        let grid = TerrainGrid {
            cells: vec![TerrainCell {
                screen_x: 0.0, screen_y: 0.0,
                tile_id: 100, sub_tile: 0, z: 0,
                rx: 0, ry: 0,
                is_water: false, is_cliff_redraw: false,
                variant: 0, tint: [1.0; 3],
                radar_left: [0; 3], radar_right: [0; 3],
                has_damaged_data: false,
            }],
            world_width: TILE_WIDTH,
            world_height: TILE_HEIGHT,
            origin_x: 0.0, origin_y: 0.0,
            local_bounds: None,
            anchor_variant_table: Some(BridgeAnchorVariantTable {
                ns: [200, 201, 202, 203],
                ew: [300, 301, 302, 303],
            }),
        };

        // Bridge state: cell (0,0) reports class = AboutToFall (NS).
        // (Use a test_seed_cell helper or whatever exists in bridge_state.)
        let mut bs = BridgeRuntimeState::default();
        bs.test_seed_cell(0, 0, BridgeRuntimeCell {
            deck_present: true,
            destroyable: true,
            deck_level: 0,
            bridge_group_id: Some(1),
            damage_state: DamageState::Healthy { variant: 0 },
            axis: Some(Axis::NS),
            role: BridgeCellRole::Anchor,
            anchor_span_id: Some(1),
            overlay_byte: 0,
            damaged_variant: false,
            bridgehead_anchor_class: BridgeheadAnchorClass::AboutToFall,
        });

        // Capture what tile_id is passed to uv_fn.
        let captured = std::cell::RefCell::new(None);
        let uv_fn: UvLookupFn = Some(&|tid, sub, var| {
            *captured.borrow_mut() = Some((tid, sub, var));
            Some(TilePlacement {
                uv_origin: [0.0, 0.0],
                uv_size: [1.0, 1.0],
                pixel_size: [TILE_WIDTH, TILE_HEIGHT],
                draw_offset: [0.0, 0.0],
            })
        });

        let _ = build_visible_instances(
            &grid, 0.0, 0.0, 1024.0, 768.0,
            uv_fn,
            None,
            Some(&bs),
        );

        let (tid, sub, var) = captured.borrow().unwrap();
        // Override fired: tile_id = NS AboutToFall (slot 3) = 203.
        assert_eq!(tid, 203);
        // Sub_tile preserved.
        assert_eq!(sub, 0);
        // Variant reset to 0 (no damaged_variant sibling swap on variant tiles).
        assert_eq!(var, 0);
    }

    #[test]
    fn override_bypassed_when_class_is_variant0() {
        // Same setup, class = Variant0 → effective_tile_id = native (100).
        // (Mirror the above test with bridgehead_anchor_class = Variant0;
        // assert captured tid == 100.)
    }

    #[test]
    fn override_bypassed_when_table_is_none() {
        // anchor_variant_table = None → effective_tile_id = native, even
        // if class is non-Variant0.
        // (Mirror with anchor_variant_table = None.)
    }
```

**Step 7: Verify.**

Run: `cargo test -p vera20k --lib override -- --nocapture`
Expected: PASS (3 new tests).

Run: `cargo test -p vera20k --lib terrain`
Expected: PASS (full terrain test suite).

Run: `cargo build`
Expected: clean.

**Step 8: Commit.**

```
map/terrain: per-frame tile_id override for bridgehead anchor variants

TerrainGrid gains anchor_variant_table: Option<_>, threaded from
TheaterData at build_terrain_grid_from_resolved. build_visible_instances
adds a tile_id override that fires when sim reports a non-Variant0
bridgehead_anchor_class AND the table is present, swapping the cell's
tile_id for the corresponding variant tile_id from the table.

Sub_tile is preserved (matches the reference engine's SetOverlay write
behavior — only +0x38 changes). Variant slot is reset to 0 when the
override fires — the variant tile_ids ARE the damage progression and
don't need a further FA2 sibling-TMP swap.

Mirrors the existing damaged_variant variant-slot override at
terrain.rs:579 in shape, one level up (tile_id vs variant).
```

---

### Task 7: Atlas pre-load — inject the 8 variant tile_ids

**Why:** The atlas only loads tiles that appear in the map at load time. Variant tile_ids the map didn't explicitly place would be atlas misses at the moment damage hits. Pre-loading guarantees the atlas has them ready.

**Files:**
- Modify: [src/map/theater.rs](../../src/map/theater.rs) (new helper `inject_bridge_anchor_variant_tiles` after `collect_used_tiles` ~line 540-550)
- Modify: [src/app_init_helpers.rs](../../src/app_init_helpers.rs) (call the helper in `build_tile_atlas` ~line 194)

**Pattern:** Sibling helper to `collect_used_tiles`, called in the same atlas pre-load sequence.

**Step 1: Add the helper to theater.rs.**

After the existing `collect_used_tiles` function (~line 540-550), add:

```rust
/// Inject TileKey entries for the 8 bridge anchor variant tile_ids × all
/// sub_tiles in each tile_id's TMP template into the `needed` set used
/// by the atlas pre-loader.
///
/// Required so the atlas has the variant tiles loaded before any bridge
/// damage happens at runtime — without this, the first damage hit would
/// be an atlas miss for the variant cell, producing a blank or fallback
/// sprite on the same tick that the damage applies.
///
/// Silently skips tile_ids whose TMP file is absent from `asset_manager`
/// (e.g., mod theaters missing a variant TMP). Logs one `WARN` per
/// missing TMP at theater load.
pub fn inject_bridge_anchor_variant_tiles(
    needed: &mut HashSet<TileKey>,
    table: &BridgeAnchorVariantTable,
    lookup: &TilesetLookup,
    asset_manager: &crate::assets::asset_manager::AssetManager,
) {
    let all_tile_ids = table.ns.iter().chain(table.ew.iter()).copied();
    for tile_id in all_tile_ids {
        let Some(filename) = lookup.filename(tile_id as i32) else {
            log::warn!(
                "Bridge anchor variant tile_id {} has no entry in TilesetLookup; skipping pre-load",
                tile_id
            );
            continue;
        };
        let Some(tmp_data) = asset_manager.get_ref(filename) else {
            log::warn!(
                "Bridge anchor variant TMP {} missing from MIX archives; cell will render as native tile_id on damage",
                filename
            );
            continue;
        };
        let tmp = match crate::assets::tmp_file::TmpFile::from_bytes(tmp_data) {
            Ok(t) => t,
            Err(e) => {
                log::warn!(
                    "Bridge anchor variant TMP {} failed to parse: {:#}",
                    filename,
                    e
                );
                continue;
            }
        };
        let cell_count = (tmp.template_width * tmp.template_height) as usize;
        for sub_tile in 0..cell_count {
            // Skip empty sub-tiles (consistent with how load_tile_images
            // handles them via the empty_cell_count path).
            if tmp.tiles[sub_tile].is_none() {
                continue;
            }
            needed.insert(TileKey {
                tile_id,
                sub_tile: sub_tile as u8,
                variant: 0,
            });
        }
    }
}
```

**Step 2: Call the helper in `build_tile_atlas`.**

Modify [src/app_init_helpers.rs:179-213](../../src/app_init_helpers.rs#L179-L213). The function currently signature is:

```rust
pub(crate) fn build_tile_atlas(
    asset_manager: &AssetManager,
    lookup: &theater::TilesetLookup,
    palette: &Palette,
    _ext: &str,
    grid: &TerrainGrid,
    gpu: &GpuContext,
    batch: &BatchRenderer,
) -> Option<TileAtlas> {
```

Update to take the variant table too:

```rust
pub(crate) fn build_tile_atlas(
    asset_manager: &AssetManager,
    lookup: &theater::TilesetLookup,
    palette: &Palette,
    _ext: &str,
    grid: &TerrainGrid,
    gpu: &GpuContext,
    batch: &BatchRenderer,
    anchor_variant_table: Option<theater::BridgeAnchorVariantTable>,
) -> Option<TileAtlas> {
```

Then after `needed.insert(...)` for the clear-ground tile (line 196-200), add:

```rust
    // Inject bridge anchor variant tile_ids (× all sub_tiles) so the
    // atlas has them loaded before any damage happens at runtime.
    if let Some(table) = anchor_variant_table {
        theater::inject_bridge_anchor_variant_tiles(
            &mut needed,
            &table,
            lookup,
            asset_manager,
        );
        log::info!(
            "Atlas pre-load: injected up to {} bridge anchor variant TileKeys (8 tile_ids × sub_tiles)",
            // Estimated upper bound; actual injected count = sum of cell_counts.
            8 * 9 /* rough estimate, refined by log inside the helper if needed */
        );
    }
```

Locate the caller of `build_tile_atlas` (grep for `build_tile_atlas(`) — likely in app_init.rs near the terrain_grid construction. Update the call site to pass the variant table:

```rust
    let anchor_variant_table = theater_data
        .as_ref()
        .and_then(theater::BridgeAnchorVariantTable::from_theater);
    let atlas = build_tile_atlas(
        &asset_manager,
        &theater_data.as_ref().unwrap().lookup,
        &theater_data.as_ref().unwrap().iso_palette,
        ext,
        &grid,
        &gpu,
        &batch,
        anchor_variant_table,
    );
```

(Note: `anchor_variant_table` is computed twice — once at Task 6's `build_terrain_grid_from_resolved` call and once here. Consider hoisting it to a single binding above both calls.)

**Step 3: Unit-test coverage strategy.**

`AssetManager::new(&Path)` requires a real RA2 install dir to construct (loads `ra2.mix` at the path). There's no in-process empty-AssetManager constructor, so a focused unit test for `inject_bridge_anchor_variant_tiles` would need either:

- A tempdir + minimal-valid `ra2.mix` fixture (heavy — out of scope for a single task).
- A refactor of the helper to take a trait-bound asset lookup so a mock can be passed (refactor surgery — out of scope; the function already has the right signature for production callers).

**Decision: skip the unit test for this task. Graceful-fallback behavior (missing TMP → WARN + skip, no panic) is verified by Task 8's in-game test on a real theater, where the WARN logs would appear if any of the 8 variant TMPs are unexpectedly absent.**

The helper's `log::warn!` branches are exercised by the existing release-build path on any mod theater that ships incomplete BridgeSet variant TMPs — Task 8's first boot of the temperate map will trip them if they fire.

(If a focused unit test is desired later, the cleanest refactor is to make the helper generic over a small `BridgeTmpFetcher` trait with one method `fn fetch(&self, filename: &str) -> Option<&[u8]>`, implementable by both `AssetManager` and a `HashMap<String, Vec<u8>>` mock. Out of scope for this plan.)

**Step 4: Verify.**

Run: `cargo test -p vera20k --lib inject_bridge -- --nocapture`
Expected: PASS (1 test).

Run: `cargo build`
Expected: clean.

**Step 5: Commit.**

```
map/theater: pre-load bridge anchor variant tile_ids into atlas

inject_bridge_anchor_variant_tiles extends the atlas pre-load needed-
set with the 8 variant tile_ids (× all sub_tiles per template). Runs
once at theater load via build_tile_atlas.

Without this, the variant tile_ids would be atlas misses on the first
weapon hit, producing a blank or fallback sprite on the same tick the
damage applies. With it, the atlas is ready and the renderer override
in build_visible_instances finds valid placements immediately.

Graceful fallback: missing TMPs (rare in retail, possible in mods)
log one WARN and skip; cells with those variants then render at their
native tile_id forever, matching the existing override-disabled
fallback path.
```

---

### Task 8: End-to-end verification

**Why:** Confirm the full pipeline — sim parity fix + variant table + pre-classification + renderer override — produces the same visible behavior gamemd does on real maps.

**Files:** No code changes — verification only. May produce small fix-up commits if issues surface.

**Step 1: Run full test suite.**

Run: `cargo test -p vera20k --lib`
Expected: PASS across all modules.

Run: `cargo clippy -p vera20k --all-targets -- -D warnings`
Expected: no new warnings.

Run: `cargo fmt --all -- --check`
Expected: clean.

**Step 2: Boot the game and load a high-bridge map.**

Pick a temperate map with a clearly visible HIGH bridge (e.g., one of the multiplayer maps with a long water-crossing bridge — sift through `ini/missions.ini` or just pick "Country Swing" / a similar standard map).

Run: `cargo run --release` (or whatever the project's launch command is — check the project root for a justfile or CLAUDE.md notes if uncertain).

**Step 3: Verify pristine anchor rendering.**

Confirm: at map load, the bridge anchors render exactly as they do in retail gamemd.exe (compare side-by-side if needed). No visible drift.

If anchors look different from the start: the pre-classification (Task 4) may be wrongly tagging pristine anchors as damaged. Inspect via log spam (re-add a log in the resolved-terrain pre-classify pass if needed).

**Step 4: Verify first-hit transition.**

Bring up a Grizzly Tank, position it within range of a bridge ramp. Fire on the bridgehead cell (the ramp connecting the bridge to the ground).

Expected behavior matching gamemd:
- Anchor cell visibly transitions from pristine to "most damaged" sprite (the 4th BridgeSet variant for the bridge's axis).
- Bridgehead cell's own sprite does NOT change.
- Sustained fire (10+ shots) does NOT collapse the bridge.

If the rendered sprite is wrong:
- Wrong variant slot rendered (e.g., 3rd instead of 4th): check Task 6's `tile_id_for` math.
- Sub_tile mismatch (cell renders a wrong piece of the template): check Task 6's `sub_tile` preservation.
- FA2 sibling-variant rendered (a/b/c/d): check the `effective_variant = 0` reset in Task 6.

**Step 5: Verify EW direction.**

Position to fire on an EW-oriented bridge (the bridge runs east-west, ramps face north and south). Same expected behavior — anchor transitions to the 4th EW variant tile_id.

If EW renders the NS variant or vice versa: Axis routing is wrong somewhere. Check `Axis::NS` vs `Axis::EW` in the table lookup (Task 6) and that sim's bridge cell carries the correct `axis` field.

**Step 6: (Optional) Verify pre-damaged anchor map.**

If a campaign map exists with an author-damaged anchor, load it and confirm the bridge starts damaged on tick 1 (no blank-frame regression). If no such map is easily available, skip — the synthetic test at Task 5 covers the same code path.

**Step 7: Run `/fidelity-check` if it's the cheapest path to a binary-side comparison.**

(Optional / time-permitting.) The `/fidelity-check` skill specifically compares observable output against gamemd. Not strictly required if Steps 3-5 are visually clean.

**Step 8: Commit any fix-up changes.**

If any fixes are required after the visual check, commit them as targeted patches per the issue found. Otherwise no commit at this step.

**Step 9: Document residual gaps.**

If Task 8 reveals known parity gaps that are out of scope for this plan (e.g., the 5th collapse tile, low-bridge variants, FA2-variant interactions on damaged anchors), add a `## Known Deferred` section to the design doc with a one-line description of each. Do NOT silently leave them undocumented.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-13-bridgehead-anchor-renderer-design.md](2026-05-13-bridgehead-anchor-renderer-design.md)
- **Brainstorm-time Ghidra MCP session:** 2026-05-13
- **Ghidra reports:**
  - [ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md) §3.2, §11.1, lines 185-200 (state machine writers); §4 (sub-tile invariants)
  - [ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BRIDGE_RENDERING_GHIDRA_REPORT.md) (general bridge rendering context)
  - [ra2-rust-game-docs/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/ASSET_PARSING_BRIDGES_GHIDRA_REPORT.md) (TMP parsing for bridge tilesets)
- **gamemd.exe addresses:**
  - `Read_Theater_TileSets_INI` @ 0x00545150 (BridgeMiddle1/2 INI read at 0x00545c1e / 0x00545c3a)
  - `ProcessBridgeDamageStateMachine_High` @ 0x00576BA0 (entry gate 0x00576BD2, NS first-hit write 0x00577701, EW first-hit write 0x0057769b)
  - `SetOverlayAndPropagate` @ 0x0056EB80 (sub_tile preservation: writes +0x38 only)
- **INI keys (all 5 retail theaters identical):**
  - `[General] BridgeMiddle1 = 7`
  - `[General] BridgeMiddle2 = 12`
  - `[General] BridgeSet = N` (tileset index, already parsed)
- **Related code:**
  - Existing per-frame variant override pattern: [src/map/terrain.rs:579-586](../../src/map/terrain.rs#L579-L586)
  - Existing TheaterData INI parsing pattern: [src/map/theater.rs:449-462](../../src/map/theater.rs#L449-L462)
  - Existing bridgehead-detection pass in resolved-terrain build: [src/map/resolved_terrain.rs:544-605](../../src/map/resolved_terrain.rs#L544-L605)
- **Sim G3 design (predecessor):** [docs/plans/2026-05-12-bridgehead-damage-progression-design.md](2026-05-12-bridgehead-damage-progression-design.md)
- **Sim G3 commits on dev:** `bf0573a` (add field), `71041e6` (add enum), `d474114` (rewrite state machine), `f001a37` (G7 register bridgeheads)
