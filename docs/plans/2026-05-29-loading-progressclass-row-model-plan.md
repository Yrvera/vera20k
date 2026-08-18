# Loading-Screen ProgressClass Row-Draw Model — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained.

**Goal:** Reproduce gamemd's standard-skirmish loading bar — milestone-driven
fill (G2), solid backing fill + player color-scheme remap (G3), country side-icon
(G4), and exact geometry (G6) — as one coherent ProgressClass row-draw model.

**Architecture:** Loading lives entirely in the app layer (above sim). A
`LoadingProgressSink` is injected into `app_init`'s load functions; at each real
phase boundary the loader emits a milestone, and the app-owned sink advances the
already-faithful `LoadingProgressState` gate and synchronously re-renders the
loading screen — mirroring gamemd's per-milestone `WM_PAINT`. No sim/determinism
impact.

**Design Doc:** docs/plans/2026-05-29-loading-progressclass-row-model-design.md

---

## Grounding Summary

- **Docs:** `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md` (geometry,
  +0x308/+0x30C color, +0x70/+0x71 flags, ftol fill, frame 0, flags 0x400);
  `LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md` (the
  ordered milestone ledger with per-phase gamemd addresses);
  `LOADING_FIRST_RENDERER_00552D60` (ls###<country>.shp background — already
  correct in Rust).
- **Ghidra verified this session:** `0x00643720` (row geometry: bar at base+5,
  icon/text after `base_x + W + 0x15`, vertical centering, text ptr null for
  skirmish); `0x004e3560` (side icon = country insignia PCX: usai/japi/frai/geri/
  gbri/djbi/arbi/lati/rusi/yrii.pcx, obsi idx −3, rani idx −2).
- **Repo pattern:** `LoadingProgressState` (app_loading.rs) already implements the
  monotonic gate + ftol fill faithfully and is only consumed in tests; the
  milestone ledger constant + `theater_ramp_changed_values` already exist.
  `loading_screen_chrome.rs` builds the per-variant atlas (background + PROGBARM
  frame0). `house_colors.rs` provides the 16-shade player color ramp.
  `pcx_file.rs` decodes PCX.
- **Order finding:** our loader parses the map (incl. IsoMapPack) in the
  *InitialMapSelection* phase, before `load_map_from_initial` does theater/rules.
  To keep milestone emission monotonic in OUR order, the dense map-section values
  (63–69) are assigned to our terrain/tile-atlas build in `load_map_from_initial`
  (where the map is materialized), not the raw parse.
- **INI:** thememd.ini is not involved here (that was the menu-music fix). No new
  INI keys; side index = local launch-node country (already in `LaunchCountry`).
- **Still unknown after grounding:** exact ColorScheme struct fields at +0x308
  (solid backing color) and +0x30C (bar remap) — Task 1 decodes them read-only.

## Key Technical Decisions

- **Injected `LoadingProgressSink` trait, synchronous repaint on advance** —
  matches gamemd's WM_PAINT mechanism; app_init depends on a trait, not render.
  **Confidence:** high. **Source:** design doc; LOADING_FULL_INIT ledger.
- **Milestone values emitted in OUR execution order, full gamemd value set,
  monotonic** — because our load order differs from gamemd's Full_Init order, the
  value-to-phase pairing is internal. Observable result (bar sweeps the same
  coarse value set 0→100) is preserved; per-phase timing already differs
  inherently between engines. **Confidence:** medium — flag for /review-plan.
  **Source:** LOADING_FULL_INIT ledger + load_map_from_initial read.
- **Skip dynamic theater ramp 13–25** (conditional on theater-cache mismatch +
  runtime `DAT_00B054E0` even in gamemd); emit 12/30 around our single theater
  load. **Confidence:** high. **Source:** LOADING_FULL_INIT row 5 (conditional).
- **Coalesce beacon-art 90** into post-setup (no distinct Rust equivalent).
  **Confidence:** medium. **Source:** LOADING_FULL_INIT row 29.
- **Side-icon = country PCX, NO label for skirmish** (text ptr 0).
  **Confidence:** high. **Source:** Ghidra 0x004e3560, 0x00643720; PROGBARM §5.
- **G3 solid fill = ColorScheme+0x308 HSV → RGB** via `FUN_00517440` (an
  HSV→RGB routine: +0x308 holds hue/sat/value bytes), packed to surface format
  and filled over `(x+3, y+3, W, H)` via surface vtable +0x58 before the clipped
  span. Rust source = the player scheme's representative color in
  `house_colors.rs` (`SCHEME_BASES`). **Confidence:** high for mechanism
  (verified `0x00643400`, `0x00517440` this session); medium for exact shade
  match — pin in Task 1/Task 8. The +0x30C convert (SHP remap) still needs
  confirmation during Task 8.

## Open Questions

### Resolved During Planning
- Milestone order mismatch → assign 63–69 to terrain/atlas build, not raw parse
  (keeps monotonic). 
- Side label → none for skirmish (text ptr 0).
- Theater ramp / beacon art → skip / coalesce (both conditional in gamemd).
- Side-icon asset → country insignia PCX via FUN_004e3560 mapping.

### Deferred to Implementation
- Exact ColorScheme +0x308 / +0x30C RGB (Task 1 read-only Ghidra; until then G3
  color is unconfirmed).
- Final per-phase emit line numbers inside `load_map_from_initial` /
  `load_map_initial_with_assets` — pinned in Task 4 against current code.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_loading.rs` | `LoadingProgressSink` trait + recording sink; render-triggering sink; pump wiring; geometry; render path (G3/G4/G6) |
| Modify | `src/app_init.rs` | Thread sink param; emit milestones at phase boundaries |
| Modify | `src/render/loading_screen_chrome.rs` | Add side-icon PCX entry + color-scheme color to atlas |
| Read | `src/rules/house_colors.rs` | Player color ramp source for G3 |
| Read | `src/assets/pcx_file.rs` | PCX decode for side icon |
| Read-only Ghidra | ColorScheme struct, `0x00643400` | Identify +0x308 / +0x30C |

## Interface Changes

- `app_init::load_map_initial_with_assets` and `load_map_from_initial` gain a
  `progress: &mut dyn LoadingProgressSink` parameter. Callers: `pump_loading_after_present`
  (app_loading.rs:289, :309) — both updated in Task 6. The trait is defined in
  `app_loading.rs` (app layer) so `app_init` references the trait without
  importing `render`.

## Sim Checklist
Not applicable — no `sim/` files touched. Loading is pre-game; no tick-order,
state-hash, or determinism impact.

## Risk Areas

- **Synchronous mid-load present**: the sink renders+presents during a load that
  currently runs in one frame. Must reuse the existing `render_loading_screen`
  path and the app's present mechanism; a sink render failure must be non-fatal
  (log + continue) so it never aborts the map load. Regression: existing
  `LoadingProgressState` tests must stay green; the generic (non-native) fallback
  path must be untouched.
- **Interface threading**: adding the sink param touches both load entry points
  and the pump. Confirm no other callers of these functions exist (grep in
  Task 3).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | ColorScheme +0x308 / +0x30C | Bar backing color + remap; wrong color visible every match | Ghidra struct decode + in-game |
| 4 | Milestone value set + monotonic order | Bar sweep cadence, visible every match | Recording-sink test vs ledger; in-game |
| 4 | Suppress non-visible 6/58/60 via gate | Avoids extra/duplicate repaints | Recording-sink test |
| 7 | Bar origin base+8 x + vertical centering | Bar position, every match | Geometry unit test vs PROGBARM §6 |
| 8 | Solid backing fill before clipped span (+0x71) | Empty-bar region color, every match | In-game vs gamemd |
| 8 | Bar remap via +0x30C (not native palette) | Bar color, every match | In-game vs gamemd |
| 9 | Side icon at base_x+W+0x15, vert-centered | Country insignia beside bar, every match | Geometry test + in-game |
| 9 | No label (skirmish text ptr 0) | Must NOT draw a label | Code review + in-game |

---

## Tasks

### Task 1: Pin the G3 color source (mostly done; read-only Ghidra)

**Why:** G3's solid backing color comes from ColorScheme+0x308; we must not guess
the RGB. **Mechanism already verified this session** (`0x00643400` +0x71 branch →
`FUN_00517440` HSV→RGB on +0x308 → surface vtable +0x58 fill). This task pins the
exact Rust value + confirms the +0x30C convert. Read-only — no Ghidra mutation.

**Files:** none (writes findings into Task 8's notes).

**Step 1:** Decompile `0x00642BB0` (PriorityToColorScheme → g_ColorSchemeArray)
to confirm how the player's ColorScheme is selected from session priority, and
that +0x308 holds the scheme's HSV triple (corroborating `0x00517440`).

**Step 2:** Determine which `house_colors.rs` value equals the +0x308 HSV→RGB
result — most plausibly the scheme's representative base (`SCHEME_BASES[idx]`).
Confirm by computing HSV→RGB of a known scheme's +0x308 and matching it to the
ramp/base. Record the exact source (base vs a specific shade) in Task 8.

**Step 3:** Confirm the +0x30C convert used by `CC_Draw_Shape(*(param_1+0x54),
0, ...)` for the PROGBARM frame0 — verify it is the player color-scheme remap
(maps to `Palette::with_house_colors`, sprite_atlas.rs:1225) and not the SHP's
native palette.

**Step 4: Verify** findings stated as verified-from-binary with cited addresses.
If the exact shade cannot be resolved, implement Task 8 with the scheme base
(`SCHEME_BASES`) and flag the shade as a follow-up — the mechanism (single HSV
+0x308 fill) is confirmed, so this is a value-precision detail, not a blocker.

**Step 5: Commit** — no code; this task feeds Task 8.

### Task 2: Define `LoadingProgressSink` trait + recording sink

**Why:** Interface-first; both load entry points and tests depend on it.

**Files:** Modify `src/app_loading.rs`.

**Pattern:** trait-object injection (new, minimal pattern; app layer owns it).

**Step 1: Define the trait**
```rust
// src/app_loading.rs
/// Receives a native loading milestone (0..=100) at a real load-phase boundary.
/// Implementors apply the monotonic gate and may synchronously repaint.
pub(crate) trait LoadingProgressSink {
    fn milestone(&mut self, percent: u32);
}
```

**Step 2: Recording sink (tests)**
```rust
#[cfg(test)]
#[derive(Default)]
pub(crate) struct RecordingProgressSink {
    progress: LoadingProgressState,
    pub emitted: Vec<u32>,
}

#[cfg(test)]
impl RecordingProgressSink {
    fn standard() -> Self {
        Self { progress: LoadingProgressState::standard_skirmish(), emitted: Vec::new() }
    }
}

#[cfg(test)]
impl LoadingProgressSink for RecordingProgressSink {
    fn milestone(&mut self, percent: u32) {
        if self.progress.advance_progress(percent) {
            self.emitted.push(percent);
        }
    }
}
```

**Step 3: Verify** `cargo check` compiles.

**Step 4: Commit.**

### Task 3: Thread the sink parameter through the load entry points

**Why:** Establish the interface before adding emit calls.

**Files:** Modify `src/app_init.rs` (signatures of `load_map_initial_with_assets`
~311 and `load_map_from_initial` ~366); Modify `src/app_loading.rs` (pump call
sites ~289, ~309).

**Step 1:** Grep for all callers: `load_map_initial_with_assets` and
`load_map_from_initial` — confirm only `pump_loading_after_present` calls them
(plus tests). List them in the commit message.

**Step 2:** Add `progress: &mut dyn crate::app_loading::LoadingProgressSink` as the
final param to both functions. `app_init` references the trait by path only — do
NOT add a `use crate::render::*`.

**Step 3:** At the two pump call sites, pass a temporary sink that wraps the
session's `NativeLoadingScreenState::progress` (full sink lands in Task 5). For
now a thin adapter calling `advance_progress` keeps behavior identical.

**Step 4: Verify** `cargo check`.

**Step 5: Commit.**

### Task 4: Emit milestones at real phase boundaries

**Why:** The core of G2 — drive the bar through the gamemd value set in our
execution order.

**Files:** Modify `src/app_init.rs` (`load_map_initial_with_assets`,
`load_map_from_initial`); Modify `src/app_loading.rs` (`loading_screen_presented`
keeps 3, pump keeps 100).

**Pattern:** call `progress.milestone(v)` at each boundary below. Emit each
listed value exactly once, in ascending order; the monotonic gate then needs no
manual suppression. NOTE: 58 and 60 ARE visible gamemd milestones (emitted inside
Read_INI_Basic) and are correctly in the list — emit each once. The values the
gate suppresses in gamemd are the raw `6` (theater reload after `8` — never emit
it) and the *duplicate direct re-emissions* of 58/60 (we simply never double-emit
them). Do not add `6`; do not emit 58 or 60 twice.

**Step 1:** In `load_map_initial_with_assets`, after the map is parsed, emit `8`.

**Step 2:** In `load_map_from_initial`, emit at these boundaries (monotonic in our
order; values from the gamemd ledger):
```
after theater::load_theater (382)              -> 12, 30
after rules+art loaded/merged/processed (435)  -> 31, 35, 45
after house_roster/color_map (502)             -> 50
after parse_lighting (480)  [reorder: emit after 502] -> 55, 58, 60
after ResolvedTerrainGrid::build + tile_atlas (493) -> 63, 65, 67, 68, 69, 70
after build_terrain_grid_from_resolved (477)   -> 72
after spawn_entities returns (554)             -> 74, 76, 78
after sim house/alliance setup                 -> 82, 86, 93, 96, 98   (90 coalesced here)
```
Pin the exact insertion lines against current code while editing (the line refs
above are anchors, not final). Each `milestone(v)` call sits immediately after the
work it represents completes.

**Step 3: Verify** with a recording-sink unit test asserting the emitted sequence
equals the monotonic-gated ledger:
```rust
#[test]
fn milestone_emit_sequence_matches_gated_ledger() {
    let mut sink = RecordingProgressSink::standard();
    for v in [3u32,8,12,30,31,35,45,50,55,58,60,63,65,67,68,69,70,72,74,76,78,82,86,93,96,98,100] {
        sink.milestone(v);
    }
    assert_eq!(sink.emitted.first(), Some(&3));
    assert_eq!(sink.emitted.last(), Some(&100));
    assert!(sink.emitted.windows(2).all(|w| w[0] < w[1])); // strictly monotonic
}
```

**Step 4:** `cargo test --lib loading` — verify pass.

**Step 5: Commit.**

### Task 5: Implement the render-triggering sink

**Why:** Make milestones actually repaint, matching gamemd's synchronous WM_PAINT.

**Files:** Modify `src/app_loading.rs`.

**Step 1:** Define a sink that borrows the pieces `render_loading_screen` needs
(gpu, surface/target, batch_renderer, encoder creation) plus the session's
progress state.

**Step 2:** `milestone(v)`: call `progress.advance_progress(v)`; if it advanced,
acquire the next surface frame, run the existing loading render path, and present.
On any render/acquire error, `log::warn!` and continue (non-fatal — must not abort
the load).

**Step 3:** Ensure the progress state used by the sink is the SAME
`NativeLoadingScreenState::progress` the render path reads (no divergent copy).

**Step 4: Verify** `cargo check`. (Full repaint cadence is device-dependent —
covered by in-app verification in Task 11.)

**Step 5: Commit.**

### Task 6: Wire the pump to drive the load through the real sink

**Why:** Replace the current 8/12/100-only emission with the full sweep.

**Files:** Modify `src/app_loading.rs` (`pump_loading_after_present`).

**Step 1:** Construct the render-triggering sink and pass it into the load calls.
Remove the now-redundant inline `advance_native_progress(8/12/100)` calls (100 is
still emitted at `Finished`; 3 still at `loading_screen_presented`).

**Step 2:** Keep the two-phase pump structure (InitialMapSelection →
RemainingLegacyLoad) so the first LS frame still presents before heavy load.

**Step 3: Verify** `cargo check` + `cargo test --lib loading`.

**Step 4: Commit.**

### Task 7: Fix bar geometry (G6)

**Why:** Bar currently at base+3; gamemd is base+8 x with vertical row-centering.

**Files:** Modify `src/app_loading.rs` (`standard_skirmish_progress_position`,
render path).

**Step 1:** Change x to `base_x + 5 + 3` (helper +5, inset +3) and y to
`base_y + ((row_h - (H + 6)) / 2) + 3`, where `H` = `progress_frame0.pixel_size[1]`
and `row_h = max(side_icon_h, H + 6, font_h) + 4`. `side_icon_h` comes from the
side-icon entry (Task 9); `font_h` from the loading font (use the UI font height
already used elsewhere, or the icon/H max if font unavailable — document choice).

**Step 2: Verify** with a unit test pinning x = base+8 and y for a known
(H, row_h):
```rust
#[test]
fn bar_origin_uses_helper_offset_and_row_centering() { /* assert x,y per formula */ }
```

**Step 3:** `cargo test --lib loading`.

**Step 4: Commit.**

### Task 8: Solid backing fill + bar remap (G3)

**Why:** `+0x71=1` fills the full frame rect with ColorScheme+0x308 before the
clipped span; the bar is remapped via +0x30C, not its native palette.

**Files:** Modify `src/render/loading_screen_chrome.rs` (expose color-scheme
color), Modify `src/app_loading.rs` (render path + thread color_index).

**Step 0 (prerequisite — color_index threading):** The bar fill color comes from
the player's MP **color scheme** (color_index → one of 9 `SCHEME_BASES`), NOT the
country variant. `NativeLoadingScreenState` currently stores only `variant`
(country). Surface the player color: read
`session.request.launch.local.color_index` (a `SkirmishLaunchSession`) in the
render path, or store a `color_index: HouseColorIndex` on `NativeLoadingScreenState`
at `from_request` time (mirroring how `variant` is derived). Prefer storing it on
`NativeLoadingScreenState` for symmetry with `variant`. The generic
(non-skirmish) fallback has no color → no solid fill (matches gamemd, which only
sets +0x71=1 for skirmish).

**Step 1:** Using Task 1's mapping, resolve the player's +0x308 RGB from
`house_colors.rs` (`SCHEME_BASES[color_index]`, or the verified shade) and pass it
to the loading atlas/render.

**Step 2:** In the render path, BEFORE the clipped PROGBARM span, push a solid
full-frame rect (W×H at the bar origin) tinted with the +0x308 color.

**Step 3:** Apply the +0x30C remap to the PROGBARM frame0 (mirror
`Palette::with_house_colors` used at sprite_atlas.rs:1225) instead of tint white.

**Step 4: Verify** `cargo check`; visual confirmation deferred to Task 11. If
Task 1 marked color BLOCKED, skip Steps 1–3 and leave G3 as a documented
follow-up.

**Step 5: Commit.**

### Task 9: Country side-icon (G4)

**Why:** gamemd draws the country insignia PCX beside the bar (`+0x70=1`); no
label for skirmish.

**Files:** Modify `src/render/loading_screen_chrome.rs` (load the PCX into the
atlas), Modify `src/app_loading.rs` (draw it).

**Step 1:** Map the loading side index → PCX name (verified via FUN_004e3560):
America=usai, Korea/Alliance=japi, France=frai, Germany=geri, GreatBritain=gbri,
Libya=djbi, Iraq=arbi, Cuba=lati, Russia=rusi, Yuri=yrii; Observer=obsi,
Random=rani. Add the selected PCX as a third atlas entry (decode via
`pcx_file.rs`).

**Step 2:** In the render path, after the bar, draw the side icon at
`x = base_x + W + 0x15`, vertically centered against `row_h`
(`y = row_y + (row_h - icon_h)/2`, mirroring FUN_00643720's centering). Draw NO
label (skirmish text ptr is 0).

**Step 3:** Feed `side_icon_h` (icon entry height) back into Task 7's `row_h` so
the bar's vertical centering uses the real row height.

**Step 4: Verify** unit test: side-icon x = `base_x + W + 0x15`; no label drawn.

**Step 5:** `cargo test --lib loading`.

**Step 6: Commit.**

### Task 10: Full focused test pass

**Why:** Confirm the cluster compiles and unit-level parity holds.

**Files:** none (run only).

**Step 1:** `cargo test --lib loading` — milestone sequence, geometry, side-icon
tests, plus existing `LoadingProgressState` tests all green.

**Step 2:** `cargo check` clean.

**Step 3: Commit** (if any test-only fixups needed).

### Task 11: In-app visual verification against gamemd

**Why:** Repaint cadence, colors, and composition are device/visual — unit tests
can't confirm them.

**Verify:**
- Launch a stock skirmish; the loading bar fills through coarse milestone jumps
  (not one snap) from empty to full — compare cadence feel to gamemd.
- The empty bar region shows the solid +0x308 backing color (not background
  pixels); the bar fill is the player color (remap), not PROGBARM's native palette.
- The correct country insignia PCX appears to the right of the bar
  (`base_x + W + 0x15`), vertically centered; no text label.
- Bar sits at the corrected origin (base+8 x, vertically centered) at both 640
  and 800 widths.
- Expected from gamemd: progressive fill, solid color backing, country icon,
  no label.

## Sources & References

- **Design doc:** docs/plans/2026-05-29-loading-progressclass-row-model-design.md
- **Ghidra reports:** PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md,
  LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60_GHIDRA_REPORT.md,
  LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md,
  LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md.
- **gamemd.exe addresses:** ProgressClass draw `0x00643400`/`0x00643720`/
  `0x00643C50`; side icon `0x004e3560`; color scheme `0x00642BB0`; Full_Init
  milestone callsites per LOADING_FULL_INIT ledger.
- **INI keys:** none new (side index from `LaunchCountry`).
- **Related code:** src/app_loading.rs, src/app_init.rs,
  src/render/loading_screen_chrome.rs, src/rules/house_colors.rs,
  src/assets/pcx_file.rs, src/render/sprite_atlas.rs:1225 (remap pattern).
