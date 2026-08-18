# Loading-Screen ProgressClass Row-Draw Model Design

## Goal
Implement the standard offline-skirmish loading bar (PROGBARM) as one coherent
ProgressClass row-draw model that reproduces gamemd's milestone-driven fill,
solid backing, country side-icon, and exact geometry — closing G2/G3/G4/G6 from
the 2026-05-28 disparity scan without inventing smooth progress.

## Architecture Context

The loading screen lives entirely in the app layer (above sim):

- `src/app_loading.rs` owns `LoadingSession` / `NativeLoadingScreenState` /
  `LoadingProgressState` and the frame pump `pump_loading_after_present`.
- The progress *model* (`LoadingProgressState`) is already faithful:
  `advance_progress` applies gamemd's monotonic gate (strictly-increasing only),
  `set_percent` does `max*0.01*percent` with clamp-above-max + skip-on-unchanged,
  and `fill_width_gamemd_ftol_positive_domain` does `ftol(W*lane/max)`. The full
  milestone ledger constant and `theater_ramp_changed_values` already exist —
  **but they are only referenced in `#[cfg(test)]`.**
- The live pump emits only **3** (`loading_screen_presented`), **8**
  (`InitialMapSelection`), **12** then **100** (`RemainingLegacyLoad`), across
  two synchronous phases.
- `render_loading_screen` draws background SHP (`ls###<country>.shp`, already
  correct) + the clipped PROGBARM frame0 with tint white. No solid backing fill,
  no side icon, position = base+3 only.
- The actual load work is `app_init::load_map_initial_with_assets` then the
  ~700-line synchronous `app_init::load_map_from_initial` (theater → atlases →
  rules → units → buildings → cell init). `load_map_from_initial` already takes
  `&gpu` + `&batch_renderer`, so app_init already depends on render/gpu — a
  loading-time render trigger is not a new architectural violation.
- Player color-scheme remap machinery exists in `src/render/sprite_atlas.rs`
  (used for unit remaps) but is not exposed to the loading renderer.

## Impact Analysis

Touched:
- `src/app_loading.rs` — render path (solid fill + side icon + geometry), a new
  progress-sink type, and the pump/load wiring.
- `src/app_init.rs` — add ~25 milestone-emit hook points at real phase
  boundaries inside `load_map_initial_with_assets` + `load_map_from_initial`,
  threaded via an injected sink param.
- `src/render/loading_screen_chrome.rs` — atlas must also carry the country
  side-icon PCX entry and expose the player color-scheme background color
  (`+0x308`) and remap (`+0x30C`) for the bar.
- Possibly `src/render/sprite_atlas.rs` (read-only) to source the color-scheme
  values.

Risk: low. Loading is pre-game, outside the lockstep sim tick — **no determinism
or state-hash impact.** Blast radius is the loading screen only. The generic
(non-native) fallback path is unaffected.

## Chosen Approach — Synchronous-repaint progress sink (user-selected)

Inject a `LoadingProgressSink` into the load functions. At each real phase
boundary the loader calls `sink.milestone(value)`. The real sink:

1. `progress.advance_progress(value)` — gamemd's monotonic gate.
2. If it advanced (strictly increasing), **synchronously render + present** the
   loading screen — mirroring gamemd's per-milestone `SendMessage(WM_PAINT)`.

This matches gamemd's actual mechanism (single synchronous load pass with a
repaint at each advancing milestone) rather than faking a smooth bar or
rewriting the load into a resumable state machine. app_init depends only on the
sink trait, not on render internals; the app layer owns the concrete
render-triggering sink. A recording sink is used in tests to assert the emitted
milestone sequence.

The phase→milestone mapping is the parity-critical part and is enumerated in the
ledger below. Where our loader lacks a distinct boundary for a gamemd phase, the
milestone is emitted at the nearest real boundary we *do* cross (coalesce
forward) — never on a timer and never interpolated. Milestones for work we
genuinely do not perform (see UNKNOWN/coalesced rows) are emitted at the
adjacent real boundary; the plan finalizes each mapping by reading
`load_map_from_initial` line-by-line.

### Why not the alternatives
- **Pumped multi-phase**: would require making the 700-line synchronous load
  resumable (suspend/restore partial map state across frames) — large, high-risk
  refactor for no parity gain over synchronous repaint.
- **Faithful-partial (coarse)**: emits ~4 steps vs gamemd's ~25 — a named DRIFT
  the user rejected.

## Tiny-Detail Ledger (parity constraints)

Progress model / gating (already implemented — preserve):
- Monotonic gate: only strictly-increasing values repaint; equal/lower
  suppressed. [doc: PROGBARM §7; app_loading.rs `advance_progress`]
- `set_percent` = `max*0.01*percent`, clamp above max only, skip on unchanged.
  [doc: PROGBARM §5/§7]
- Fill width = `ftol(frame0_width * lane/max)`, height = frame0 H, frame 0 only.
  [doc: PROGBARM §5/§6]
- Random-map halving (`ScenarioClass+0x34BD`) is **inactive** for selected-map
  skirmish — do NOT implement halving. [doc: LOADING_FULL_INIT §"Progress
  Visibility Rule", line 39]

Milestone ledger — emit at the load phase that does the same logical work
[all rows: doc LOADING_FULL_INIT_PROGRESS_SEQUENCE_AFTER_00552D60 §"Ordered
Milestone Ledger", with cited gamemd addresses]:

| % | gamemd phase | Rust emit point (to finalize in plan) |
|---|---|---|
| 3 | first LS render handoff | `loading_screen_presented` (done) |
| 8 | theater init entry | start of `theater::load_theater` |
| 12 | theater archive reload (cond.) | after theater MIX load |
| 13–25 | theater palette/remap ramp (dynamic, only-on-increase) | UNKNOWN — see below |
| 25 | theater finish (dup-suppressed if ramp reached 25) | after theater palettes extracted |
| 30 | after Init_Theater returns | after `theater_result` ready |
| 31 | command-bar rules load | rules/command-bar load |
| 35 | rules CD/file setup | rules file setup |
| 45 | RulesClass::Process | after rules processed |
| 50 | side-mix init | side/house mix init |
| 55 | `[Basic]`/lighting read | map `[Basic]`/lighting parse |
| 58 | player/house setup | house setup (may coalesce into 60) |
| 60 | end of Read_INI_Basic | after basic INI read |
| 63 | map/theater section start | map section parse start |
| 65 | tileset/surface setup | tile atlas build |
| 67 | cell-tags pass | cell-tags parse |
| 68 | IsoMapPack decode | IsoMapPack decode |
| 69 | post-IsoMapPack helper | after iso decode |
| 70 | map/overlay prelude | overlay prelude |
| 72 | terrain/tiberium init | terrain + ore growth/spread queue init |
| 74 | units section | units read |
| 76 | infantry/object pass | infantry/object read |
| 78 | buildings read | buildings read |
| 82 | random-map rules boundary (cond.) | coalesce (no TMCJ4F path) |
| 86 | cell-attributes init | cell-attributes init |
| 90 | beacon art init | UNKNOWN — coalesce if no equivalent |
| 93 | post-map-init inner | post-map setup |
| 96 | post-map-init + tactical cleanup | tactical/view setup |
| 98 | final pre-render refresh | end of `load_map_from_initial` |
| 100 | load complete | pump `Finished` |

Non-visible raw calls to suppress naturally via the gate (do NOT emit as
separate steps): **6** (theater reload, after 8), direct **58** (after 60),
direct **60** (duplicate). [doc: LOADING_FULL_INIT §"Core Logic Notes",
lines 84–86]

Render composition (G3/G4/G6) [doc: PROGBARM §5/§6, lines 48–49, 68–71;
+ Ghidra `0x00643720`, `0x004e3560`, `0x00643400` read this session]:
- **G3 solid backing**: with `+0x71=1`, fill the full frame rect with the RGB
  from player `ColorScheme+0x308` **before** the clipped SHP span. [PROGBARM §5
  line 49]
- **Bar convert**: the PROGBARM fill is remapped through the player/session
  `ColorScheme+0x30C` (not the SHP's native palette). Current Rust draws it tint
  white — this is part of the same color-scheme dependency as G3. [PROGBARM §5
  line 48]
- **G6 geometry**: bar pixels start at `base_x + 5 + 3` (helper +5, inset +3) and
  `row_y + ((row_h-(H+6))/2) + 3`. `row_h = max(side_icon_h, H+6, font_h) + 4`;
  `row_y = base_y` (single row, index 0). `base` from `FUN_00552BE0` =
  LoadProgressMgr point + (12,256)@640 / (16,321)@800. Current Rust uses
  `base+3` (the (12,256)/(16,321) offsets are already in
  `standard_skirmish_progress_position`, but it is **missing the +5 x and the
  vertical centering term**). [PROGBARM §6 lines 66, 68–69; verified
  `decompile_function 0x00643720`]
- **G4 side icon**: when `+0x70=1`, draw the country insignia PCX selected by
  `FUN_004e3560(side_index)` — `usai/japi/frai/geri/gbri/djbi/arbi/lati/rusi/
  yrii.pcx`, plus `obsi.pcx` (observer, idx −3) and `rani.pcx` (random, idx −2).
  Index = local launch-node country (−3→0 mapping), via HouseType+0xBC →
  ProgressClass+0x80; aligns with our `LaunchCountry`. Drawn **after** the bar at
  `base_x + W + 0x15`, then icon, vertically centered against `row_h`. Icon
  width/height from the surface's vtable `+0x7c`/`+0x80`. Blit via `FUN_006ba580`
  with color `FUN_004355d0(0xff)`. [verified `decompile_function 0x004e3560`,
  `0x00643720`]
- **G4 label = NONE for skirmish**: skirmish passes **text pointer 0** to
  `FUN_00642C80`, so the row label/status text is null — only the side icon is
  drawn. Do NOT add a label string. [PROGBARM §5 line 42; FUN_00643720 param_3]
- **Draw flags**: clipped span uses CC_Draw_Shape frame 0, flags `0x400`,
  z/priority `1000`. [PROGBARM §5 line 45]

UNKNOWN — needs RE before/within /write-plan (do not guess):
- **13–25 theater ramp count**: `min(i/(DAT_00B054E0/13)+0x0C, 0x19)`, redraws
  only on increase. The ramp only fires on theater-cache mismatch and its step
  count depends on a runtime `DAT_00B054E0`. Our theater load is a single op —
  decide whether to emit the ramp endpoints (e.g. just 25) or skip the dynamic
  middle. [doc: LOADING_FULL_INIT row 5; conditional path]
- **90 beacon art**: confirm whether our loader has an equivalent step; if not,
  coalesce into the adjacent boundary.
- **ColorScheme+0x308 / +0x30C exact source in Rust**: confirm how to derive the
  player color-scheme background RGB + remap for the loading side; sprite_atlas
  has remap machinery but the loading-bar exposure is unbuilt.

## Design

### Components
- `LoadingProgressSink` (trait or `FnMut(u32)` closure) — injected into app_init
  load functions. Two impls: a render-triggering sink (app layer) and a
  recording sink (tests).
- Extend `LoadingScreenAtlas` (loading_screen_chrome.rs) with: the country
  side-icon entry, and the player color-scheme background color (+0x308) + remap
  (+0x30C) needed for the bar.
- Render path in `render_loading_screen`: (1) background SHP, (2) **solid backing
  fill** rect from +0x308, (3) clipped PROGBARM frame0 remapped via +0x30C at the
  corrected geometry, (4) **side icon** at `base_x + W + 0x15`, vertically
  centered.

### Interfaces / Contracts
- `load_map_initial_with_assets(..., sink: &mut dyn LoadingProgressSink)` and
  `load_map_from_initial(..., sink: &mut dyn LoadingProgressSink)` — app_init
  calls `sink.milestone(v)` at the ledger boundaries; never imports render.
- The render-triggering sink borrows gpu/surface/batch_renderer and re-renders
  the loading screen on an advancing milestone.

### Data Flow
Pump enters load → load runs synchronously → at each phase boundary the loader
emits a milestone → sink advances the gated progress model → on advance, sink
synchronously renders+presents the loading screen → load continues → pump returns
Finished at 100.

### Error Handling
Sink render failures are non-fatal to the load (log + continue); a failed render
must not abort map loading. Load failures propagate as today via `LoadingPump::
Failed`.

### Testing Strategy
- Recording sink asserts the emitted milestone sequence equals the gated ledger
  (reuse the existing ledger constant + `theater_ramp_changed_values`).
- Geometry unit tests: bar origin = base+8 x and the vertical-centering term for
  a known `row_h`; side-icon x = `base_x + W + 0x15`.
- Keep existing `LoadingProgressState` tests green.
- Note: end-to-end repaint cadence needs a real device — verify visually
  in-app (loading is shown every match).

## Architectural Decisions
- Follows the existing app-layer-owns-loading pattern; sim untouched, determinism
  untouched. The sink keeps app_init free of render internals while preserving
  gamemd's synchronous-repaint mechanism (Rust-native structure, gamemd-native
  semantics).
- Reuses the already-faithful `LoadingProgressState` gate rather than a new
  progress type.

## Alternatives Considered
- Pumped multi-phase resumable load — rejected (large/risky, no parity gain).
- Faithful-partial coarse milestones — rejected by user (named DRIFT).
