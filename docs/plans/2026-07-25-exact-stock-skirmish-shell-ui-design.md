# Exact Stock-Skirmish Shell UI Goal Design

## Goal

Drive the complete offline-stock-skirmish shell route to exact active-retail
player-visible UI parity while remaining safe to run unattended.

## Architecture Context

The production route is already split into useful Rust-native owners:

- `AppState` and `GameScreen` own top-level routing and the handoff from shell to
  loading and gameplay.
- `ui::shell` owns shared dialog identity, geometry, input, focus, modal stacking,
  and owner-draw control conventions.
- `ui::main_menu_shell`, `ui::single_player_shell`, and
  `ui::skirmish_shell` own screen-specific state and layout.
- `app_main_menu_shell_render`, `app_single_player_shell_render`, and
  `app_skirmish_shell_render` compose production frames.
- `render::shell_paint`, the shell chrome atlases, bit-font rendering, cursor,
  Bink playback, and audio provide shared presentation primitives.
- `app_loading` and `render::loading_screen_chrome` own the selected-skirmish
  loading presentation and milestone repaint path.

System Map v2 identifies `GSI-03.10` as owner of
`LOOP-001-SKIRMISH-LAUNCH`, with `GSI-03.01`, `GSI-03.09`, `GSI-03.11`,
the UI surface of `GSI-03.12`, and the launch boundary in `GSI-03.17`
forming the focused shell family. Asset, palette, CSF/font, input, audio,
movie, and window-state systems are dependencies rather than replacement
owners.

The route already exists in production Rust and is substantial, but it is not
certified exact. Known examples include degraded egui fallbacks, unresolved
aggregate pixel capture, incomplete loading-bar palette remap, stale research
status sections, and remaining screen-specific composition/input details.

The canonical native Oracle is the separate local
`<local>/Documents/vera20k-oracle` repository. It has guarded native
shell inspection, navigation, and DXGI capture machinery, but its capabilities
must be inspected rather than assumed. Its current dirty checkout is protected
from this goal.

## Impact Analysis

Likely VERA20k surfaces include:

- `src/app.rs`, `src/app_shell_transition.rs`, `src/app_loading.rs`;
- `src/ui/shell/`, `src/ui/main_menu_shell/`,
  `src/ui/single_player_shell/`, and `src/ui/skirmish_shell/`;
- the corresponding `src/app_*_shell_render.rs` files;
- `src/render/shell_paint.rs`, shell chrome/loading atlases, bit-font, Bink,
  cursor, palette/convert, and audio trigger paths;
- launch, selected-map, preview, CSF, settings-persistence, and first-frame
  handoff contracts when their output is consumed by the shell.

Risks are cross-screen shared primitives, invalid screenshot comparison bases,
input/focus regressions, audio duplication, loading lifecycle/order changes,
and collisions with other tasks working on RMG, launch, palette, audio, or
Oracle tooling.

## Chosen Approach

Use route-first checkpoint closure:

`main menu -> Single Player -> Skirmish -> owned modal/interaction -> Start ->
loading -> first tactical frame`

Keep one checkpoint owner at a time. Capture and trace its complete native
composition/interaction, trace the production Rust route, introduce a decisive
comparison or regression, implement the earliest divergence, validate the real
route, merge, and continue. Persistent full-frame and common-interaction
differences are fixed before rare state pixels, but every residual stays
visible and must be closed before exact completion.

This approach produces usable end-to-end progress and exercises shared shell
mechanisms in real callers without pretending that one helper or asset list is
the screen.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — Native route identity is
  `0xE2 -> 0x100 -> 0x102`; the main menu must not shortcut directly to
  Skirmish. `[doc: SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md]`
- `MILESTONE-BLOCKING` — Each screen must close its complete paint order:
  background, movie/preview, right-panel chrome, controls, text, modal,
  cursor, and transition/fade layers.
  `[doc: MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md]`
  `[doc: skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md]`
- `MILESTONE-BLOCKING` — Layout uses native dialog/control policies at the
  active retail breakpoints, not global proportional scaling.
  `[doc: skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md]`
- `MILESTONE-BLOCKING` — Mouse press/release, capture, hit boundaries,
  keyboard/default/cancel routing, focus, caret, dropdown/scroll/drag state,
  and modal blocking must match the visible route. `[research-index: skirmish-ui]`
- `MILESTONE-BLOCKING` — Retail asset names, frames, palettes/converts, source
  clipping, destination rectangles, text/CSF/font metrics, and z-order must be
  proven for each scoped role rather than inferred from filenames.
- `MILESTONE-BLOCKING` — Start tears down shell authority before loading;
  loading persists through real coarse milestones and hands off only after the
  load path completes.
  `[doc: SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md]`
  `[doc: LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md]`
- `MILESTONE-BLOCKING` — Loading background, country/side art, palette,
  player-color remap, progress geometry, repaint cadence, and final transition
  are part of the exact UI result.
  `[doc: LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md]`
- `COMPOUNDING` — Common shell paint, bit-font, palette conversion, cursor,
  click sounds, music, and transition primitives affect several checkpoints;
  changes require neighbor revalidation.
- `UNKNOWN-RISK` — A native screenshot is not a valid pixel oracle until crop,
  resolution, scaling, color space, cursor policy, and capture timing are proven
  comparable to Rust.
- `EXACTIFICATION-RESIDUAL` — A rare state or one-pixel/frame discrepancy may
  be postponed behind larger scoped UI drift, but it remains named and must be
  closed before this exact goal may complete.

## Design

### Components

Maintain a machine-readable checkpoint matrix and concise operational state.
Each row names route, resolution, state/interaction, native evidence, Rust
evidence, comparison result, owner, and residuals. The matrix is evidence
routing, not a hand-certified parity ledger.

Use the existing screen-specific state/layout/render boundaries. Improve shared
shell mechanisms only when multiple proven callers need the same contract.
Keep simulation independent of UI and keep loading progress injected through
the existing sink/orchestration boundary.

### Interfaces / Contracts

- One UI checkpoint, one feature branch/worktree, and one integration decision
  at a time.
- One global Cargo lease and one Oracle capture/input lease.
- Native executable evidence or exhaustive proof is required for exact closure.
- Normal retail assets must remain on the native-shaped production renderer;
  degraded fallbacks cannot certify the scoped route.
- Other tasks' worktrees, branches, dirty files, processes, and Oracle checkout
  are read-only unless explicitly and uniquely transferred.

### Data Flow

Native evidence and retail assets define the checkpoint contract. Screen state
and app routing drive layout/paint/input/audio. Start produces the launch
session, loading consumes it and emits progress, and the app commits the first
tactical frame. Comparisons feed a bounded implementation slice and then the
checkpoint matrix.

### Error Handling

Fail closed on ambiguous ownership, invalid native capture, missing required
retail assets, or unsafe Oracle state. Mark evidence `UNVERIFIED` rather than
manufacturing a comparison. If one checkpoint is externally blocked, record
the blocker and continue with a disjoint scoped checkpoint.

### Testing Strategy

Use focused unit tests for formulas and state transitions, production-route
interaction tests, native/Rust stable-frame pixel comparisons, transition
frame sequences, and sound-event assertions. The canonical matrix covers
640x480, 800x600, 1024x768, all distinct active stock loading-art branches,
normal/cancel/error interactions, and the complete Start-to-first-frame route.
Run focused Cargo checks serially and one final post-merge check.

## Architectural Decisions

The design follows existing Rust-native screen/state/render separation and
System Map ownership. It does not emulate Win32 classes or native inheritance.
It tightens observable UI semantics, shared ownership, and capture evidence.

The goal may add the smallest indispensable Oracle capture/navigation
prerequisite only in a separate uniquely owned Oracle worktree. It must never
edit or merge through the existing dirty Oracle checkout.

## Alternatives Considered

### Shared-substrate-first

Perfect every generic shell primitive before returning to screens. Rejected as
the primary loop because it can optimize helpers that are not active in the
selected route and delays production evidence.

### Screen inventory / gap scan

List every missing control, asset, or research item and implement down the
list. Rejected because missingness does not establish active composition,
priority, or ownership.

### Route-first checkpoint closure

Chosen because it closes player-visible screens through their actual
producers, shared primitives, state handoffs, and consumers while still
converging on exact scoped parity.
