# Incremental random-map generation (progressive preview + Working/Please Wait)

**Date:** 2026-07-22
**Status:** all four tasks done (2026-07-23). Two follow-ups remain, both found
while doing the work: the dialog's progress bar is drawn empty (§Progress bar),
and two owed tests are blocked on fixtures (§Owed tests).
**Blocks:** the last two known drifts in the random-map setup dialog `0x105`

## Why this exists

Two separate observable drifts turn out to have one cause.

1. **No progressive preview.** `RandomMapGenerator__Generate @ 0x00598960` calls
   `GenerateTerrainPreview @ 0x00641140` **8 times** when its preview-flag argument is
   nonzero — call sites `0x00598aa8, 0x00598b6a, 0x00598bf0, 0x00598dd9, 0x0059904b,
   0x005990f0, 0x005991db, 0x0059930d` — and each is immediately followed by a
   **synchronous** `SendMessageA(hDlg, WM_PAINT, 0, 0)`. The player watches the map
   build up stage by stage during the blocking Generate. Our port renders the preview
   exactly once, at the end.
   Evidence: `docs/research/skirmish-ui/RANDOM_MAP_GENERATE_PREVIEW_LOOP_GHIDRA_REPORT.md`,
   independently spot-checked via `get_xrefs_to 0x00641140`.

2. **"Working / Please Wait" is never seen.** Static `0x638` is shown for the duration of
   the generate block. Our modal already sets `generating` and the text pass already
   draws it — but the flag is set and cleared inside a single input handler, so no frame
   ever renders while it is true.

### The actual blocker

Not cost. **Re-entrancy.** The original re-enters its own paint handler synchronously
from inside generation; Win32 permits that. Our renderer cannot be: `Generate` runs
inside `handle_random_map_setup_mouse_up`, and drawing happens only on a separate redraw
event, so nothing can paint until that handler returns.

Therefore *any* faithful fix requires generation to span frames. That one change fixes
both drifts.

## Chosen approach: worker thread

Rejected alternative: making the pipeline resumable (a state machine stepped one stage
per frame). It restructures exactly the code all generation parity rests on — the x87
math, the RNG consumption order, the 18-stage ordering in `STAGE_ORDER`. Not worth the
risk for a cosmetic effect.

The worker keeps `generate_map` byte-identical and changes only *where* it runs.
Determinism is unaffected: generation stays single-threaded and seed-driven.

### The piece to get right first

The rasteriser currently needs `ResolvedTerrainGrid`, which needs the `AssetManager` —
not something to hand to a worker. So the main thread must build a small `Send` snapshot
up front and move it in.

**Do NOT** "optimise" this by having the preview skip `ResolvedTerrainGrid` and read tile
indices straight from the generated `MapFile`. The resolver decides which tile a cell
actually ends up with; bypassing it can silently change colours that are correct today.
Whatever the snapshot contains must come from the same resolution the final preview uses.

## Tasks

### 1. `Send` radar snapshot — DONE 2026-07-22 (`56357881`)

Landed as `PreviewPalette` in `src/map/rmg/preview.rs`, plus
`preview_cells_from_palette`. Keyed on the raw cell identity, valued from what the
resolver produced, so it agrees with the on-screen colours by construction rather than
by re-deriving them. Unknown tiles yield black, which the rasteriser already greys.

Deferred from this task: the byte-identical equivalence test the original plan asked
for. `ResolvedTerrainCell` and `MapFile` have no `Default`, so it needs a real fixture;
the palette's own drift risks (channel order applied exactly once, black-as-absent,
unknown-tile fallback) are covered by unit tests instead. **The equivalence test is
still owed** — until it exists, "the palette path matches the direct path" is asserted
by construction, not proven.

Original task text:
- New type in `src/map/rmg/preview.rs` holding what the rasteriser needs and nothing else:
  a per-cell `(left, right)` colour pair plus the overlay colour lookup already resolved
  to `(overlay_id, density) -> [u8; 3]`.
- Built on the main thread from `ResolvedTerrainGrid` + the overlay SHP lookup that
  currently lives in the `overlay_radar` closure in `App::render_random_map_setup_preview`
  (`src/app.rs`).
- Must be `Send`; no asset handles, no `Rc`.
- Test: a snapshot rasterises to a byte-identical image to the current direct path.
  `test_preview_snapshot_matches_direct_rasterise`

### 2. Stage callback through the pipeline — DONE 2026-07-23

`run_pipeline` takes a `StageObserver` and fires it at every boundary in
`STAGE_ORDER` it reaches (Water..RecalcFinal — `Emit` is the caller's step).
`build::generate_map_observed` wraps that with a `GenerationPointView`, adding the
`Initial` boundary before any terrain work. Observation is shared-only: the RNG,
scratch and phase state are never handed over, and the grid arrives `&`.

Projection is lazy — `view.snapshot()` walks every cell, so a boundary the
observer ignores costs nothing. Snapshots carry cells, overlays and start
positions; trees and tech buildings are left out because the pipeline only hands
those over at the end and the preview draws neither.

The worker publishes a snapshot at the six distinct boundaries (below) through the
existing channel as `RandomMapUpdate::Progress`; `poll_random_map_generation`
drains, keeps the newest, rasterises it through the same
`preview_cells_from_map` path the final map uses, and calls
`show_progress_preview`, which bumps `preview_generation` without ending the
generate block. Only the finished map is written to `RandMap.img`.

Tests: `observing_a_run_does_not_change_what_it_generates` (identical projection
and `stages_run` with an observer that snapshots at *every* boundary),
`every_boundary_is_reported_once_in_pipeline_order`,
`a_skipped_stage_still_reports_its_boundary`,
`snapshots_share_the_final_maps_dimensions` (so the preview box cannot change
size mid-generate), `start_positions_appear_from_the_starts_boundary_onwards`,
and `six_preview_boundaries_cover_the_originals_eight_redraws`.

### 3. Move generation onto a worker
- `Generate` spawns the worker with options + snapshot inputs, returns immediately.
- `RandomMapSetupModalState.generating` stays true for the whole run, so the existing
  `0x638` text draw becomes visible with no render change.
- Worker publishes `PreviewImage`s through a channel; the main loop drains and bumps
  `preview_generation`, which the existing texture cache already keys on.
- Every control stays inert while `generating` — already modelled in `is_enabled`.

### 4. Accept-while-generating — DONE 2026-07-23

Verified there is no route out of the dialog mid-generate: `is_enabled` returns
false for every control including Cancel (pinned by
`every_control_is_inert_during_generate_including_cancel`), a disabled control
swallows its click without arming a press, and the setup modal has no keyboard
handler at all — only two places in the tree clear it.

That made the safety an accident of input gating rather than an invariant, so
both close sites now go through `close_random_map_setup`, which drops the job
with the dialog, and `poll_random_map_generation` drops an orphaned job on sight
rather than trusting future close paths to remember. What this protects is
concrete: a late finish would have written `RandMap.img`, changing the chooser's
thumbnail to a map the player had walked away from.

Also removed: a commit branch in the mouse-up handler that became unreachable
when generation moved to the worker (`commit_options` was never assigned again).

**The originally-planned test does not exist.**
`test_closing_mid_generate_discards_late_frames` needs an `AppState`, which owns
a `Window` and a `GpuContext` and cannot be built in a unit test. Rather than
fake it, the invariant is made structural (single close helper + self-healing
poll) and the testable half is pinned at the modal level:
`progress_previews_show_without_ending_the_generate_block` and
`each_progress_preview_bumps_the_texture_key`. **The app-level path is
UNVERIFIED-pending-fixture** — an `AppState` test harness would close it.

## Open questions

- ~~**Which of our 18 stages correspond to the native 8 preview points?**~~ **PARTLY
  ANSWERED 2026-07-22.** Each preview is preceded by a progress-percent update
  (`FUN_00643C50` on the singleton at `0x00AC4F58`, taking a double) and followed by the
  synchronous `SendMessageA(WM_PAINT)`. Decoded ladder and confirmed interleaving:

  | Address | What | Percent |
  |---|---|---|
  | `0x00598AA8` | preview #1 | — (precedes terrain work) |
  | `0x00598AFF` | water seed `FUN_0059A6C0` | |
  | `0x00598B6A` | preview #2 | 55 |
  | `0x00598BF0` | preview #3 | 60 |
  | `0x00598C8F` | region partition `FUN_0058CF90` | |
  | `0x00598DD9` | preview #4 | 70 |
  | `0x00598EAB` | start generation `FUN_00594B50` | |
  | `0x00598EF4` | tiberium creation `FUN_005A23A0` | |
  | `0x0059904B` | preview #5 | 80 |
  | `0x005990F0` | preview #6 | 85 |
  | `0x005991DB` | preview #7 | 90 |
  | `0x0059930D` | preview #8 | 95 |

  Evidence: `get_xrefs_to 0x00641140`; `get_assembly_context` on all eight; `get_xrefs_to`
  on each generation function; recorded as a plate comment on `0x00598960`.

  **Use the percentage ladder as the cadence anchor**, not our stage names — it is the
  original's own sense of progress and it is exact. Our `STAGE_ORDER` entries can be
  assigned percentages and previews fired when the reported percent crosses each rung.

  **RESOLVED 2026-07-22** by a `get_bulk_xrefs` sweep over every generation callee of
  `0x00598960`, giving the complete call ordering. Full sequence with the work between
  each preview:

  | Preview | % | Generation since the previous preview |
  |---|---|---|
  | #1 `0x00598AA8` | — | (initial; `Random__Seed` at `0x00598985`) |
  | #2 `0x00598B6A` | 55 | water seed `0x0059A6C0`, `0x0059C580`, `0x0059C630` |
  | #3 `0x00598BF0` | 60 | **nothing** |
  | #4 `0x00598DD9` | 70 | `0x005AC290`, region partition `0x0058CF90`, `0x0058E740`, `0x0058E9B0`, `0x0058D010`, `0x0058EBC0`, `0x0058EF10`, `0x005A19E0`, bridges `0x00578E60`, `0x005A17F0` |
  | #5 `0x0059904B` | 80 | `0x0059B740`, recalc `0x0047D2B0`, starts `0x00594B50`, `0x005A1FB0`, `0x005A95B0`, tiberium `0x005A23A0`, `0x005AD790`, recalc |
  | #6 `0x005990F0` | 85 | **nothing** |
  | #7 `0x005991DB` | 90 | recalc, `0x005A35F0` |
  | #8 `0x0059930D` | 95 | `0x005A38C0`, `0x005A3AE0`, `0x005A4280` |

  The only thing in the #2→#3 and #5→#6 gaps is `FUN_0069AE90` (`0x00598BB6`,
  `0x005990B6`), which is a **progress-report/network-throttle helper and does no terrain
  work** — `decompile_function 0x0069AE90`, plate-commented.

  **So eight preview calls produce only SIX distinct images.** #2/#3 are byte-identical
  to each other, as are #5/#6; they differ only in the percentage shown beside them.
  Firing eight rasterises in the port would do two for nothing and look no different.

  Stage mapping for our `STAGE_ORDER`, from the confirmed ordering:
  - #1 — before `Water`
  - #2 @55 / #3 @60 — after `Water`, `WaterFinalize`
  - #4 @70 — after `Regions`, `IslandPasses`, `GreenSpread`, `RecalcAfterTerrain`
  - #5 @80 / #6 @85 — after `Starts`, `TechBuildings`, `Tiberium`, `RecalcAfterTiberium`
  - #7 @90 — after `Hills`
  - #8 @95 — after `LatPatches`, `RecalcAfterPatches`, `Trees`, `Rocks`

  Implement six rasterises fired at those stage boundaries, with the percentage ladder
  driving what the progress readout says.
- Does the native block pump other messages (is the dialog draggable mid-generate)?
  Only `SendMessageA(WM_PAINT)` was found inside `0x00598960`; no pump call was
  identified. If there is none, our worker must not make the dialog *more* responsive
  than the original.

## Owed tests

Two checks the plan asked for do not exist, both blocked on a missing fixture
rather than on effort. Neither is a parity claim, but both are places where a
regression would land silently.

1. `test_preview_snapshot_matches_direct_rasterise` (task 1) — `ResolvedTerrainCell`
   and `MapFile` have no `Default`, so it needs a real fixture. Until it exists,
   "the palette path matches the direct path" is asserted by construction, not
   proven. Lower priority now that the worker rasterises through
   `preview_cells_from_map` and the palette is unused on the live path.
2. `test_closing_mid_generate_discards_late_frames` (task 4) — needs an
   `AppState`, which owns a `Window` and a `GpuContext`.

Both would fall out of one `AppState` test harness. That harness would also make
the whole poll/close path testable, which is currently the least-covered code in
this feature.

## Progress bar — open follow-up (found 2026-07-23)

The percentage ladder is not decorative: it drives a real widget, and ours is drawn
empty. Verified this session and written back into Ghidra as
`ProgressMeterClass__SetPercent` / `__Redraw` / `__DrawFill` with plate comments.

- `ProgressMeterClass__SetPercent @ 0x00643C50` — stores
  `value = max(this+0x48) * 0.01 * percent` into the per-slot double at
  `this+8+slot*8`, clamps to the max, and repaints **only when the stored value
  changed**. Two calls at the same percent paint once. Repaint is
  `SendMessageA(this+0x64, WM_PAINT)` when a dialog HWND is set, else it falls
  through to `__Redraw`. Verified via `decompile_function 0x00643C50`.
- `ProgressMeterClass__Redraw @ 0x00643AE0` — the dialog path calls
  `GetDlgItem(hwnd, 0x639)`, so **the progress bar's control id is 0x639**, and
  passes `__DrawFill` the fraction `*(double*)(this+8) / *(double*)(this+0x48)`.
  Gated on `this+0x54` (an SHP handle) being non-null — with no shape loaded it
  draws nothing at all. Verified via `decompile_function 0x00643AE0`.
- `ProgressMeterClass__DrawFill @ 0x00643400` — frame 0 of that SHP supplies the
  extent; the fill rect starts at **(x+3, y+3)** and its width is `Math__ftol` of
  the fraction scaled by the frame width. An optional solid colour (gated on
  `this+0x71`) is blitted first through the DirectDraw channel loss/shift globals,
  then the SHP is drawn over it with `CC_Draw_Shape`. Verified via
  `decompile_function 0x00643400`.

Corroboration: our extracted template rect for the bar is 100x21 DLU
(`SETUP_PROGRESS_BAR_RECT`), consistent with control 0x639.

**Not implemented, deliberately.** Two facts are still missing and both are pixels:
*which* SHP `this+0x54` holds, and whether the solid-colour path is on for this
dialog (and with what colour). Filling the bar without them would be inventing a
visual. Today `modals.rs` draws a bevelled outline there and never fills it — that
is drift, and it is now visible for the whole generate block.

Next step for whoever picks this up: find the meter's construction (xrefs to the
singleton at `0x00AC4F58`) to recover the SHP name and the `+0x71` flag, then
render fill-inset-3px at `percent/100` of frame 0's width. The ladder to drive it
is the table above: 55, 60, 70, 80, 85, 90, 95 at the eight preview points.

## Must remain unchanged

- `generate_map`'s internals — the whole point of this approach.
- The preview's cell-admission test (`Playfield`), projection, doubling, and baked
  markers. Those are verified and pinned by tests.
- Ore/gem colours come from the overlay SHP frame with `RadarColor=` as fallback.
