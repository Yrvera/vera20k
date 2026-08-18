# Gap Generator Visual Parity Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make the Allied Gap Generator's three observable effects match gamemd.exe — strict
radius footprint, hostile coverage blacking out terrain (tactical + minimap), and owner/allied
coverage rendering half-bright fog.

**Architecture:** dev recomputes per-owner visibility from scratch every tick
(`recompute_owner_visibility_in_place` → `apply_spy_sat` → `apply_gap_generators`). We keep that
recompute model — no per-cell reference counter (gamemd's counter is an artifact of its
edge-triggered apply/remove; dev re-derives coverage each tick, so overlap handles itself). The
work is: fix the footprint test, set distinct per-viewer flags for hostile vs friendly coverage,
and teach the tactical + minimap renderers to honor those flags.

**Design Doc:** none — design was settled in-session against the binary (no separate brainstorm).
This plan carries the full design inline.

---

## Grounding Summary

- **Authoritative path (verified):** Gap Generator coverage is applied by
  `TechnoClass__UpdateCloakShroud @ 0x006FB170` and reversed by `RemoveCloakShroud @ 0x006FB470`,
  dispatched from `BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0` via a build/power latch
  (`+0x269`). This is **not** the `RevealFogCell`/`FUN_00487630-90` path the stale
  `feat/shroud-visibility-parity` branch modeled — that branch is a misread and is **not** used.
- **Per-cell effect (verified from `0x006FB170`/`0x006FB470`):**
  - Footprint: cell accepted when `dx*dx + dy*dy < (radius+1)*(radius+1)` (strict `<`), radius from
    `TechnoType+0xCD2` = per-building `GapRadiusInCells`.
  - Hostile cell (gap owner not human/allied to local viewer): clears explored bits `0x08|0x10` on
    `CellClass+0x12C` ⇒ `IsShrouded @ 0x00586360` returns true ⇒ terrain renders **black**.
  - Owner/allied cell: increments fog counter `CellClass+0x13C` ⇒ `IsFogged @ 0x005864A0` returns
    true ⇒ terrain renders **half-bright** (channels `>> 1`).
  - The `+0x130`/`+0x134` counters exist only to drive gamemd's incremental restore-on-removal;
    dev's per-tick recompute makes them unnecessary.
- **gamemd is a single-viewer (`g_PlayerPtr`) computation; dev generalizes to per-owner grids.** The
  effect is view composition, not gameplay state — shroud does not gate unit logic in YR. So this is
  render-facing; determinism risk is limited to what dev already hashes (`fog.cells_raw()`).
- **Repo pattern:** mirror the existing `apply_gap_generators` / `apply_spy_sat` pass
  (`src/sim/vision/mod.rs:798-864`) and the existing per-cell flag model (`FLAG_REVEALED` 0x01,
  `FLAG_VISIBLE` 0x02, `FLAG_GAP_COVERED` 0x04). dev already has a GPU multiplicative shroud pass
  (ABuffer 0x00=black .. 0x7F=bright) and `dim_color(c, 0.5)` for the minimap.
- **INI:** `[GAGAP] GapGenerator=yes`, `GapRadiusInCells=10` (`ini/rulesmd.ini:12226-12227`). Parsed
  as `object.gap_radius_in_cells` (`src/rules/object_type.rs:805,1251`). `[General] GapRadius`
  (`gap_radius`, default 10, `src/rules/ruleset.rs:201,1097`) is the wrong source and is replaced by
  the per-building value.
- **Still unknown after grounding:** the local restore-gate byte `g_PlayerPtr+0x577A` (deferred —
  see Open Questions; dev's recompute corresponds to the restore-enabled common case).

## Key Technical Decisions

- **Keep the per-tick recompute; do NOT add a `+0x130`/`+0x134` reference counter.** dev re-derives
  coverage every tick, so overlap correctness is free and no permanent cell state is mutated.
  — **Confidence:** high — **Source:** Ghidra `0x006FB170`/`0x006FB470` (counters drive
  edge-triggered restore only) + repo recompute pattern `src/sim/vision/mod.rs:1628`.
- **Hostile coverage drives BLACK via the existing `FLAG_GAP_COVERED`; add a new `FLAG_GAP_FOG`
  (0x08) for owner/allied half-bright.** Flags live in the per-owner `cells` bytes, recomputed each
  tick. — **Confidence:** high — **Source:** Ghidra `IsShrouded 0x00586360` (black on `0x08`
  clear), `IsFogged 0x005864A0` (`+0x13C >= 1`).
- **Strict footprint `dx*dx + dy*dy < (r+1)*(r+1)`, per-building radius.** — **Confidence:** high —
  **Source:** Ghidra `0x006FB170` loop predicate; INI `gap_radius_in_cells`.
- **Tactical half-bright = write `0x3F` (≈ half of `0x7F`) into the shroud ABuffer diamond for
  fogged cells; minimap half-bright = `dim_color(terrain, 0.5)`.** — **Confidence:** medium (exact
  half value is an approximation of gamemd's per-channel `>>1`; visually equivalent under the
  multiply pass) — **Source:** Ghidra fog branch `0x005864A0`; repo `src/render/shroud_buffer.rs`,
  `src/render/minimap.rs` `dim_color`.

## Open Questions

### Resolved During Planning

- *Is the branch's RevealArea2 counter the right thing to port?* No — wrong code path, and
  unnecessary for dev's recompute. Resolved via Ghidra `0x006FB170`/`0x006FB470`.
- *Does dev need to hash a new counter for lockstep?* No — the new flag bits live in
  `fog.cells_raw()`, already hashed per owner (`src/sim/world/world_hash.rs:365-367`).
- *Radius source?* Per-building `GapRadiusInCells`, not `[General] GapRadius`.

### Deferred to Implementation

- **`g_PlayerPtr+0x577A` restore gate.** gamemd only restores explored bits on gap removal when this
  local byte is set; if clear, a hostile-gap cell could stay black after removal. dev's recompute
  always restores (re-derives each tick). Verify the byte's meaning before assuming full
  equivalence; expected to be the restore-enabled common case (low risk, observer/debug-tier).
  Verification: decompile writers of `g_PlayerPtr+0x577A` in Ghidra.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/vision/mod.rs` | Add `FLAG_GAP_FOG`; strict radius; per-viewer hostile/friendly flag set; clear new flag in `clear_all_visible`; per-building radius in signature; query helper `is_cell_gap_fog` |
| Modify | `src/sim/world/mod.rs:1640-1679` | Collect per-building gap radius; pass `(owner, rx, ry, radius)` tuples |
| Modify | `src/render/shroud_buffer.rs` | Black out hostile-gap cells; half-bright fogged cells in the ABuffer |
| Modify | `src/render/minimap.rs` (+ `minimap_helpers.rs` for `cell_visibility_color`) | Hostile gap → `COLOR_SHROUD`; friendly gap fog → `dim_color(terrain, 0.5)` |

## Interface Changes

- `vision::apply_gap_generators` signature changes from
  `(&mut FogState, &[(InternedId, u16, u16)], i32, &StringInterner)` to
  `(&mut FogState, &[(InternedId, u16, u16, i32)], &StringInterner)` — radius moves into the tuple.
  Only caller is `World::refresh_fog` (`src/sim/world/mod.rs:1673`). Tests in
  `src/sim/vision/vision_tests.rs` call it and must update.
- New `const FLAG_GAP_FOG: u8 = 0x08;` and `OwnerVisibility::is_gap_fog` / `FogState::is_cell_gap_fog`
  public query — additive, no existing caller breaks.
- `cell_visibility_color` (minimap helper) gains fog/gap awareness — internal to `src/render`.

## Sim Checklist

- [x] All math integer — no f32/f64 (radius test is integer; the stale branch's `f64` is **not**
      copied).
- [x] New state included in deterministic state hash — `FLAG_GAP_FOG` is a bit in
      `fog.cells_raw()`, already hashed (`world_hash.rs:367`). No new hash field needed.
- [x] No dependencies on render/ui/sidebar/audio/net — `vision/mod.rs` stays sim-only; renderers
      only *read* `FogState`.
- [x] Tick ordering unchanged — same `recompute → spy_sat → gap` order
      (`src/sim/world/mod.rs:1668`).
- [x] BTreeMap iteration order — `apply_gap_generators` iterates `fog.by_owner`; flag writes are
      commutative per cell, order-independent.

## Risk Areas

- **Blast radius of `apply_gap_generators` signature change** — single sim caller + vision tests.
  Low. Regression: keep an existing hostile-suppression test green plus add the new-flag tests.
- **Tactical black-out interacting with "once revealed, stays bright" invariant** — blacking a
  previously-explored cell while a hostile gap covers it is intended and reverts on removal (recompute
  stops setting the flag). Add a test asserting revert after the generator is gone.
- **Minimap helper change** affects every cell pixel each refresh — verify no regression on
  non-gap cells (visible/explored/unexplored unchanged).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | Strict footprint `dx²+dy² < (r+1)²` | dev's `≤ r²` makes the gap field ~1 ring too small; fires for every Gap Generator | Ghidra `0x006FB170` predicate; unit test on radius 10 boundary cells (accept where `d² ≤ 120`, reject `≥ 121`) |
| Task 3 | Hostile gap → terrain black on tactical + minimap | Enemy gap should black out your map like unexplored, not just hide unit dots | Ghidra `IsShrouded 0x00586360`; in-game enemy `[GAGAP]` over explored terrain → black |
| Task 4 | Owner/allied gap → half-bright fog | Your own gap field reads half-bright on the minimap; subtle but observable | Ghidra `IsFogged 0x005864A0`; in-game own `[GAGAP]` → half-bright pixels |
| Task 2 | Per-building radius (`GapRadiusInCells`), not `[General] GapRadius` | Correct source field; matters if a mod sets per-building radius ≠ 10 | Ghidra `type+0xCD2`; INI `gap_radius_in_cells` |

---

## Tasks

### Task 1: Add `FLAG_GAP_FOG` flag and query helpers

**Why:** The new per-cell flag is the foundation every later task reads/writes. Define it and its
accessors first.

**Files:**
- Modify: `src/sim/vision/mod.rs` (flag consts near line 23-26; `OwnerVisibility` impl; `FogState`
  impl near `is_cell_gap_covered` line 265-272)

**Pattern:** Mirror existing `FLAG_GAP_COVERED` and `is_gap_covered` / `is_cell_gap_covered`.

**Step 1: Add the constant**
```rust
// src/sim/vision/mod.rs, next to the other FLAG_* consts (~line 26)
/// Bit flag: cell is covered by a friendly (own/allied) gap generator (rebuilt
/// each tick). Renders half-bright fog rather than black.
const FLAG_GAP_FOG: u8 = 0x08;
```

**Step 2: Add the `OwnerVisibility` accessor** (next to `is_gap_covered`, ~line 86)
```rust
    /// Returns true if the cell is covered by a friendly gap generator this tick.
    pub fn is_gap_fog(&self, rx: u16, ry: u16) -> bool {
        self.index(rx, ry)
            .map_or(false, |i| self.cells[i] & FLAG_GAP_FOG != 0)
    }
```

**Step 3: Add the `FogState` alliance-aware accessor** (next to `is_cell_gap_covered`, ~line 265)
```rust
    /// Returns true if the cell is covered by a friendly gap generator for this owner.
    pub fn is_cell_gap_fog(&self, owner: InternedId, rx: u16, ry: u16) -> bool {
        if let Some(vis) = self.merged_vis(owner) {
            return vis.is_gap_fog(rx, ry);
        }
        self.by_owner
            .get(&owner)
            .is_some_and(|s| s.is_gap_fog(rx, ry))
    }
```

**Step 4: Clear the new flag each tick** — in `clear_all_visible` (~line 104), extend the mask:
```rust
    pub fn clear_all_visible(&mut self) {
        for cell in &mut self.cells {
            *cell &= !(FLAG_VISIBLE | FLAG_GAP_COVERED | FLAG_GAP_FOG);
        }
    }
```

**Step 5: Verify** — `cargo check -p vera20k`. Expected: compiles (new flag may be unused-warned
until Task 2; acceptable mid-plan).

**Step 6: Commit** — `vision: add FLAG_GAP_FOG flag + gap-fog query helpers`.

---

### Task 2: Strict radius + per-building radius + friendly-fog branch in `apply_gap_generators`

**Why:** Fixes the footprint and splits hostile (black) vs friendly (fog) coverage — the sim core of
the feature. Must precede renderer work so the flags exist to read.

**Files:**
- Modify: `src/sim/vision/mod.rs:821-864` (`apply_gap_generators`)
- Modify: `src/sim/world/mod.rs:1640-1679` (collection + call site)

**Pattern:** Same loop structure as current `apply_gap_generators`; add the friendly branch instead
of `continue`.

**Step 1: Rewrite `apply_gap_generators`** — radius per generator, strict test, both branches:
```rust
/// Apply Gap Generator coverage for one tick. For each generator at (cx,cy) with
/// the generator's own `GapRadiusInCells`, mark every cell in the strict circular
/// footprint `dx*dx + dy*dy < (radius+1)*(radius+1)`:
///   - enemy viewers: clear FLAG_VISIBLE, set FLAG_GAP_COVERED (renders black);
///   - friendly viewers (owner + allies): set FLAG_GAP_FOG (renders half-bright).
/// Call AFTER spy_sat so gap wins in contested areas.
pub fn apply_gap_generators(
    fog: &mut FogState,
    gap_generators: &[(InternedId, u16, u16, i32)],
    interner: &StringInterner,
) {
    let width = fog.width;
    let height = fog.height;
    if width == 0 || height == 0 {
        return;
    }
    for &(gap_owner_id, center_rx, center_ry, radius) in gap_generators {
        if radius <= 0 {
            continue;
        }
        let gap_owner = interner.resolve(gap_owner_id);
        let cx = i32::from(center_rx);
        let cy = i32::from(center_ry);
        // Strict native footprint: accept when dx*dx + dy*dy < (radius+1)^2.
        let threshold = (radius + 1) * (radius + 1);
        let min_x = (cx - radius).max(0);
        let max_x = (cx + radius).min(i32::from(width) - 1);
        let min_y = (cy - radius).max(0);
        let max_y = (cy + radius).min(i32::from(height) - 1);

        for (viewer_id, vis) in fog.by_owner.iter_mut() {
            let viewer = interner.resolve(*viewer_id);
            let friendly = are_houses_friendly(&fog.alliances, gap_owner, viewer);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let dx = x - cx;
                    let dy = y - cy;
                    if dx * dx + dy * dy >= threshold {
                        continue;
                    }
                    if let Some(i) = vis.index(x as u16, y as u16) {
                        if friendly {
                            vis.cells[i] |= FLAG_GAP_FOG;
                        } else {
                            vis.cells[i] &= !FLAG_VISIBLE;
                            vis.cells[i] |= FLAG_GAP_COVERED;
                        }
                    }
                }
            }
        }
    }
}
```

**Step 2: Update the call site** — `src/sim/world/mod.rs`, collection loop (~line 1641-1664) push the
per-building radius, and drop the global-radius argument:
```rust
            let mut gap_generators: Vec<(InternedId, u16, u16, i32)> = Vec::new();
            // ... inside the entity loop, replacing the existing gap push:
                    if obj.gap_generator && active {
                        let radius = i32::from(obj.gap_radius_in_cells);
                        gap_generators.push((
                            entity.owner,
                            entity.position.rx,
                            entity.position.ry,
                            radius,
                        ));
                    }
```
And the call (~line 1672-1678):
```rust
            if !gap_generators.is_empty() {
                vision::apply_gap_generators(&mut self.fog, &gap_generators, &self.interner);
            }
```

**Step 3: Add radius-boundary unit test** in `src/sim/vision/vision_tests.rs`:
```rust
#[test]
fn gap_radius_uses_strict_radius_plus_one_squared() {
    // radius 10 -> threshold (10+1)^2 = 121. Cell at d^2=120 covered, d^2=121 not.
    let mut interner = StringInterner::new();
    let enemy = interner.intern("Enemy");
    let gapper = interner.intern("Gapper");
    let mut fog = FogState::new(64, 64);
    fog.by_owner.insert(enemy, OwnerVisibility::new(64, 64));
    // Pre-reveal so suppression is observable.
    for c in &mut fog.by_owner.get_mut(&enemy).unwrap().cells {
        *c |= FLAG_VISIBLE | FLAG_REVEALED;
    }
    // No alliance entries => enemy is hostile to gapper.
    apply_gap_generators(&mut fog, &[(gapper, 30, 30, 10)], &interner);
    let v = &fog.by_owner[&enemy];
    // (30+2,30+10) -> dx=2,dy=10 -> d^2=104 < 121 : covered.
    assert!(v.is_gap_covered(32, 40));
    // dx=0,dy=11 -> d^2=121 not < 121 : NOT covered.
    assert!(!v.is_gap_covered(30, 41));
    // dx=0,dy=10 -> 100 < 121 : covered, and visible cleared.
    assert!(v.is_gap_covered(30, 40));
    assert!(!v.is_visible(30, 40));
}
```

**Step 4: Add friendly-fog unit test:**
```rust
#[test]
fn gap_marks_friendly_viewer_as_fog_not_covered() {
    let mut interner = StringInterner::new();
    let gapper = interner.intern("Gapper");
    let mut fog = FogState::new(64, 64);
    fog.by_owner.insert(gapper, OwnerVisibility::new(64, 64));
    for c in &mut fog.by_owner.get_mut(&gapper).unwrap().cells {
        *c |= FLAG_VISIBLE | FLAG_REVEALED;
    }
    apply_gap_generators(&mut fog, &[(gapper, 30, 30, 10)], &interner);
    let v = &fog.by_owner[&gapper];
    assert!(v.is_gap_fog(30, 35));        // friendly => fog
    assert!(!v.is_gap_covered(30, 35));   // not black
    assert!(v.is_visible(30, 35));        // own vision NOT suppressed
}
```

**Step 5: Verify** — `cargo test -p vera20k vision`. Expected: new tests pass; existing gap tests
updated to the new signature pass.

**Step 6: Commit** — `vision: strict gap footprint + per-building radius + friendly fog branch`.

---

### Task 3: Tactical + minimap render hostile gap as black

**Why:** The highest-visibility parity effect — an enemy gap field must black out terrain, not just
hide unit dots.

**Files:**
- Modify: `src/render/shroud_buffer.rs` (per-cell reveal check, ~line 354)
- Modify: `src/render/minimap.rs` / `minimap_helpers.rs` (`cell_visibility_color`)

**Pattern:** Mirror the existing `!fog.is_cell_revealed(...)` → black-diamond branch; OR in the
gap-covered condition.

**Step 1: Tactical** — in `shroud_buffer.rs` where it decides a cell is dark (~line 354), treat a
hostile-gap cell as not-revealed:
```rust
                if !fog.is_cell_revealed(owner, rx, ry)
                    || fog.is_cell_gap_covered(owner, rx, ry)
                {
                    // unrevealed OR under a hostile gap -> full black diamond
                    self.blit_diamond(vx, vy, vp_w, vp_h, BLACK);
                    continue;
                }
```
(Adapt to the exact local variable names in that function; the existing `!is_cell_revealed` branch
already performs the black-diamond blit — extend its condition.)

**Step 2: Minimap** — in `cell_visibility_color` (minimap helper), return the shroud color for a
hostile-gap cell. It takes `(local_owner, fog, terrain_pixel)`; add, before the
revealed/visible logic:
```rust
    if fog.is_cell_gap_covered(local_owner, terrain_pixel.rx, terrain_pixel.ry) {
        return Some(COLOR_SHROUD);
    }
```
(`COLOR_SHROUD` is already imported in `minimap.rs`; ensure the helper has access.)

**Step 3: Verify (logic)** — `cargo check -p vera20k`. Then in-game check (Task 6).

**Step 4: Commit** — `render: hostile gap generator coverage renders terrain black`.

---

### Task 4: Tactical + minimap render friendly gap as half-bright fog

**Why:** Completes the third effect — your own gap field reads half-bright.

**Files:**
- Modify: `src/render/shroud_buffer.rs`
- Modify: `src/render/minimap.rs` / `minimap_helpers.rs`

**Pattern:** dev's GPU shroud pass multiplies scene by `abuffer/0x7F`; writing `0x3F` ≈ ×0.5.
Minimap uses `dim_color(c, 0.5)`.

**Step 1: Tactical** — for a revealed, non-black cell that is fogged, fill its diamond with a
half-bright value instead of leaving it neutral. After the black-out check, before the
fully-revealed early-out:
```rust
                if fog.is_cell_gap_fog(owner, rx, ry) {
                    // friendly gap -> half-bright fog (0x3F ~= 0.5 * 0x7F under the multiply pass)
                    self.blit_diamond(vx, vy, vp_w, vp_h, 0x3F);
                    continue;
                }
```
Add `const FOG_HALF: u8 = 0x3F;` near `BLACK`/`NEUTRAL` and use it instead of the literal.

**Step 2: Minimap** — in `cell_visibility_color`, after the gap-covered black check, dim fogged
cells:
```rust
    if fog.is_cell_gap_fog(local_owner, terrain_pixel.rx, terrain_pixel.ry) {
        // friendly gap -> half-bright terrain
        return Some(dim_color(base_terrain_color, 0.5));
    }
```
(Use the same base terrain color the visible branch returns; apply `dim_color(.., 0.5)`.)

**Step 3: Verify (logic)** — `cargo check -p vera20k`.

**Step 4: Commit** — `render: friendly gap generator coverage renders half-bright fog`.

---

### Task 5: Regression — hostile gap reverts when the generator is gone

**Why:** Confirms the recompute-based model restores terrain on removal (the counter's job in gamemd)
without permanent state mutation.

**Files:**
- Modify: `src/sim/vision/vision_tests.rs`

**Step 1: Add test**
```rust
#[test]
fn gap_coverage_clears_when_no_generator_present() {
    let mut interner = StringInterner::new();
    let enemy = interner.intern("Enemy");
    let gapper = interner.intern("Gapper");
    let mut fog = FogState::new(64, 64);
    fog.by_owner.insert(enemy, OwnerVisibility::new(64, 64));
    for c in &mut fog.by_owner.get_mut(&enemy).unwrap().cells {
        *c |= FLAG_VISIBLE | FLAG_REVEALED;
    }
    apply_gap_generators(&mut fog, &[(gapper, 30, 30, 10)], &interner);
    assert!(fog.by_owner[&enemy].is_gap_covered(30, 35));
    // New tick: clear_all_visible drops gap flags; no generator => stays clear.
    fog.by_owner.get_mut(&enemy).unwrap().clear_all_visible();
    apply_gap_generators(&mut fog, &[], &interner);
    assert!(!fog.by_owner[&enemy].is_gap_covered(30, 35));
}
```

**Step 2: Verify** — `cargo test -p vera20k vision`. Expected: pass.

**Step 3: Commit** — `test(vision): gap coverage reverts when generator removed`.

---

### Task 6: Verify against gamemd.exe in-game

**Why:** Confirm the implementation matches the original engine's observable output.

**Verify:**
- Build an Allied Gap Generator (`[GAGAP]`) in a skirmish (this engine vs. retail `gamemd.exe`).
- **Enemy gap over your explored terrain:** terrain (tactical + minimap) goes **black**, like
  unexplored shroud — not merely hidden unit dots. Matches `IsShrouded` clearing `0x08`.
- **Your own gap field:** minimap pixels render **half-bright**, not normal terrain and not black.
  Matches `IsFogged` (`+0x13C`).
- **Footprint:** the covered radius matches retail (strict `(r+1)²` — ~1 cell wider than dev's old
  `≤ r²`). Compare edge cells side-by-side.
- **Removal:** sell/depower the generator; covered terrain restores next tick.
- Expected result from original: identical black/half-bright/footprint behavior.

**If a discrepancy appears:** re-check the relevant Ghidra function before adjusting (`0x006FB170`,
`0x006FB470`, `IsShrouded 0x00586360`, `IsFogged 0x005864A0`).

---

## Sources & References

- **Ghidra (verified this session):**
  - `BuildingClass__UpdateGapAndSpecialEffects @ 0x004549B0` (latch dispatch)
  - `TechnoClass__UpdateCloakShroud @ 0x006FB170` (apply: strict footprint, hostile `0x12C` clear,
    friendly `+0x13C` increment)
  - `TechnoClass__RemoveCloakShroud @ 0x006FB470` (remove: restore gated on `+0x577A`)
  - `IsShrouded @ 0x00586360` (black when `+0x12C & 0x08` clear)
  - `IsFogged @ 0x005864A0` (fog when `+0x13C >= 1`)
  - Radius source `TechnoType+0xCD2`
- **Research docs:** `docs/research/GAP_RADAR_SHROUD_MINIMAP_INTERACTION_GHIDRA_REPORT.md`
  (verified, 2026-05-27); `docs/research/SHROUD_ALGORITHM_DISTILLED.md` (gap ref-counting listed as
  not-implemented in dev).
- **INI:** `ini/rulesmd.ini:12226-12227` `[GAGAP] GapGenerator=yes`, `GapRadiusInCells=10`.
- **Repo touchpoints:** `src/sim/vision/mod.rs:798-864` (gap/spysat pass), `:265-272` (gap query),
  `src/sim/world/mod.rs:1628-1680` (refresh_fog), `src/sim/world/world_hash.rs:365-367` (fog hash),
  `src/render/shroud_buffer.rs:354` (tactical black), `src/render/minimap.rs:249-310` +
  `cell_visibility_color` (minimap), `src/rules/object_type.rs:805,1251` (`gap_radius_in_cells`).
- **Not used (reference only):** stale branch `feat/shroud-visibility-parity` — wrong code path
  (`RevealFogCell`/`FUN_00487630-90`), `f64` in sim, per-owner counter. Do not copy.
