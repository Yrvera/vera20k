# Native Sidebar Power Strip Presentation Design

## Goal

Make the ordinary YR sidebar power strip visible and retail-convincing without changing
power simulation totals or predicates.

## Architecture Context

`sim::power_system` owns deterministic per-house output and drain. The app reads those
values in `app_building_anim::update_power_bar_anim`, while
`sidebar::power_bar_anim::PowerBarAnimState` owns presentation-only segment counts.
`app_sidebar_build::render_power_bar` turns those counts into `POWERP.SHP` sprite
instances from the active `SidebarChromeAtlas`.

The retail asset is five zero-offset frames: Allied frames are `12x2`, while Soviet/Yuri
frames are `16x2`. Active YR draws them at native size, advances three pixels per segment,
uses sidebar-surface x=5 for Allied and x=0 for Soviet/Yuri, and starts at
sidebar-surface y=227.

Sources:

- `docs/research/RETAIL_SOVIET_SIDEBAR_SHP_DIMENSIONS_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/SIDEBAR_DRAW_COMPOSITION_ORDER_AND_SURFACE_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/traces/POWER_BAR_PIXEL_RENDERING_LAYOUT_TRACE.md`
- `docs/research/traces/POWER_BAR_FLASH_PHASE_TRACE.md`

## Impact Analysis

- `src/render/sidebar_chrome.rs`: preserve decoded `POWERP.SHP` alpha policy rather than
  overriding it after decode.
- `src/app_sidebar_build.rs`: render native-size frames at faction-specific native
  coordinates with a three-pixel advance.
- `src/app_building_anim.rs`: derive segment capacity from the same native presentation
  origin and UI scale.
- Focused sidebar tests: verify native geometry/origins, zero-surplus blink behavior, and
  retail asset dimensions/transparency.

No `sim/` state, tick order, state hashing, power formula, or INI authority changes.

## Chosen Approach

Keep the current Rust-native split between simulation, animation state, atlas loading,
and sprite emission. Correct the presentation seam instead of introducing a native-style
sidebar surface object.

The active requested sidebar theme is passed into sprite emission so Allied versus
Soviet/Yuri placement does not depend on which atlas happened to satisfy a fallback.
Frames retain decoded alpha, render at `entry.pixel_size * ui_scale`, and advance by
`3 * ui_scale`. Segment capacity is calculated in unscaled/native pixels before the
animation state receives it; this prevents the scale factor from being applied once to
capacity and again to sprite spacing.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING`: segment capacity must be calculated in native pixels. The old
  path divided a scaled physical height by an unscaled 3-pixel stride, then drew every
  resulting segment with the stride scaled again. At UI scale above 1, the empty band
  pushed the colored bands below the visible sidebar; this matches the reported
  "second power plant makes it grow" symptom. [current Rust:
  `src/app_building_anim.rs`, `src/app_sidebar_build.rs`]
- `MILESTONE-BLOCKING`: frames retain their side-specific native size (`12x2` Allied,
  `16x2` Soviet/Yuri), rather than being stretched to `10x3`.
  [retail asset report]
- `MILESTONE-BLOCKING`: x=5 Allied, x=0 Soviet/Yuri; y=227 sidebar-local; vertical
  advance=3. [sidebar layout report and pixel trace]
- `COMPOUNDING`: the blink frame is emitted on an even positive flash counter even when
  surplus is zero. [flash trace; `PowerClass::Draw @ 0x0063FB20`]
- `EXACTIFICATION-RESIDUAL`: stock retail `POWERP.SHP` frames contain no index-zero
  pixels, so removing the post-decode opaque override does not change stock pixels. It
  restores the general SHP alpha contract for replacement art without being credited as
  the cause of this stock symptom. [retail regression test]
- `EXACTIFICATION-RESIDUAL`: the current renderer still batches the meter with chrome,
  while native composition draws it after the active strip and before radar. Stock art
  does not overlap this lane; a full retained sidebar-surface compositor is a separate
  architecture slice. [draw composition report]
- `EXACTIFICATION-RESIDUAL`: segment-slide compensation and wall-clock cadence remain
  outside this visibility/geometry slice. Their trigger is a power transition, their
  effect is transition feel rather than static meter visibility, and they do not alter
  sim state. [segment-slide trace]

## Design

### Components

- Add shared native presentation constants alongside `PowerBarAnimState`.
- Keep `SidebarChromeAtlas` as the asset owner.
- Keep `app_sidebar_build` as the sprite-emission owner.

### Interfaces / Contracts

- `build_sidebar_chrome_instances_for_layout` receives the requested
  `SidebarTheme`.
- `PowerBarAnimState::set_max_segments` continues to consume unscaled native pixels.
- Sprite geometry is native pixels multiplied once by `ui_scale`.

### Data Flow

House power totals -> `PowerBarAnimState` segment counts -> frame selection ->
native sidebar-local origin/stride -> shared camera/screen transform.

### Error Handling

Missing frames retain the existing graceful behavior: advance the segment cursor while
omitting the unavailable sprite. A missing blink frame falls back to frame 1.

### Testing Strategy

- Pure unit checks for Allied and Soviet/Yuri origins under UI scaling.
- Pure unit check that an active flash emits/consumes a boundary segment with zero
  surplus.
- Ignored retail-assets test for five zero-offset side-native frames and decoded
  transparency.
- Scoped sidebar/render module tests and `cargo check -p vera20k`.

## Architectural Decisions

This follows the existing sim/presentation boundary and atlas/builder pattern. It does
not introduce sidebar state into simulation and does not recreate the native inheritance
chain. No new technical debt is introduced; the pre-existing full sidebar composition
residual remains explicitly out of scope.

## Alternatives Considered

- Geometry-only patch: rejected because it would retain the opaque-black pixel
  conversion that directly contributes to the symptom.
- Full retained sidebar-surface compositor: rejected for this slice because it is a
  broad multi-layer rework and is unnecessary to restore the stock power lane.
