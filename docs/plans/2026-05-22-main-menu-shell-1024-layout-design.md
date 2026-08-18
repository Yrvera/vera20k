# Main Menu Shell 1024 Layout Design

## Goal

Make the active main-menu shell render and input layout match retail YR at 1024x768 by using native-size, centered shell geometry instead of responsive scaling.

## Architecture Context

The initial main-menu shell is isolated from simulation code. Its layout model lives in `src/ui/main_menu_shell/layout.rs`, its input state and hit testing live in `src/ui/main_menu_shell/state.rs`, and its custom render path lives in `src/app_main_menu_shell_render.rs`. `src/app.rs` calls into the layout functions for mouse move/down/up and selects the custom shell renderer when assets load successfully.

Current active render and input both use `compute_responsive_layout`, which scales the 800x600 base shell to the swapchain. This is intentionally documented in the code as a drift from retail. The trace `MAIN_MENU_SHELL_LAYOUT_1024X768_TRACE.md` verifies that retail YR does not scale at 1024x768: it centers the 800x600 logical shell block at `(112,84)` and keeps movie/chrome/button art at native size.

## Impact Analysis

Touched surfaces:

- `src/ui/main_menu_shell/layout.rs`: make the retail layout path authoritative for 1024x768, fix full button sequence, and keep responsive scaling out of parity code.
- `src/app_main_menu_shell_render.rs`: render and movie asset selection should consume the retail layout.
- `src/app.rs`: mouse move/down/up should use the same retail layout as rendering.
- `src/ui/main_menu_shell/state.rs`: add or adjust tests for 1024x768 hit boxes; likely no behavior change beyond layout inputs.

Risk areas:

- Directly switching to current `compute_layout(1024,768)` is unsafe because the trace found the full six-button sequence is wrong after vertical centering.
- Title/static placement at 1024x768 has one unresolved detail: the trace found no hard evidence for the current `(112,84)` title offset. This should be spot-checked or left explicitly marked before asserting full pixel parity.
- The old responsive tests encode intentional non-parity behavior and should be retired, renamed, or restricted to a non-active helper if kept.

## Chosen Approach

Use a single retail layout model for the active shell path. Fix the model first, then wire both render and input to it.

The implementation should not introduce a viewport transform or post-scale mapping layer. The layout already expresses screen-space coordinates; render and input should share the same `MainMenuShellLayout` so art and hit boxes cannot drift apart.

## Tiny-Detail Ledger

- At 1024x768, retail logical shell centering margins are `left_margin=112`, `top_margin=84`. Source: `MAIN_MENU_SHELL_LAYOUT_1024X768_TRACE.md`, `RightPanel__ComputeLayoutRects @ 0x0072EC70`.
- RA2TS movie uses `ra2ts_l.bik`, rect `(112,84,632,570)`, not scaled. Source: trace Stage 5.
- Right panel top rect is `(744,84,168,199)`. First tile is `(744,283,168,42)`. Bottom cap rect is `(744,661,168,23)`. Source: trace Stage 4.
- Lower strip uses large `LWSCRNL`, rect `(112,652,632,32)`. Source: trace Stage 6.
- Main buttons occupy right-panel tile rows `0,1,2,3,4,8`, with full hit rects `(744,y,168,42)` and SDBTNANM art right-anchored at `x=756`, `156x42`. Source: trace Stage 8.
- Main-button return codes stay `SinglePlayer=1`, `WWOnline=2`, `Network=3`, `Movies=4`, `Options=5`, `Exit=6`. Source: `MainMenuDialog0xE2_Proc_00531F60`, existing state tests.
- Active input hit tests must use the same unscaled button rects as render. Source: trace Stage 8; current mismatch in `src/app.rs`.
- `SDBTM.SHP` bottom cap is clipped top rows at native scale, not vertically squashed. Source: `MAIN_MENU_RIGHT_PANEL_CHROME_STACK_TRACE.md`; current render already has `push_clipped_top`.
- `compute_responsive_layout` produces visible parity drift at 1024x768 and must not be used by the active parity shell. Source: trace summary.
- Title `0x694` final 1024 placement is `(747,93,162,17)`: the right-anchor helper applies sidebar inset and oversized-screen horizontal compensation first, then `FUN_0060B950` applies the main-menu `+7y/+1h` heading nudge. Source: Ghidra spot-check of `FUN_0060B1D0` and `FUN_0060B950`.
- Movie/lower-strip 2-pixel overlap draw order remains `UNCHECKED`; do not claim exact final pixels there without a retail capture. Source: trace Stage 12.

## Design

### Components

`layout.rs` should expose one active parity layout function for main-menu shell consumers. It should compute screen-space retail coordinates directly:

- movie rect from retail movie rules;
- right-panel rects from `RightPanel__ComputeLayoutRects`;
- lower strip rect from shell centering margins;
- version and tooltip rects from verified anchor helpers;
- button rects from DLU-derived Y plus the same vertical centering offset used by shell helpers before snapping to tile rows.

The responsive layout helper should be removed from active imports or renamed/test-scoped so future code cannot accidentally use it for parity rendering.

### Interfaces / Contracts

Render and input should both call the same active retail layout function with `state.gpu.config.width/height`.

`ensure_movie_for_current_layout` only needs the layout for movie asset selection, not to scale the movie. At width `640`, use `ra2ts_s`; otherwise use `ra2ts_l`.

### Data Flow

```text
Window size
  -> main_menu_shell::compute_layout
  -> app_main_menu_shell_render: movie/chrome/buttons/text instances
  -> app.rs mouse handlers: hit-test same layout
  -> state.rs returns shell action
```

### Testing Strategy

Add focused tests before or with the implementation:

- `compute_layout(1024,768)` returns movie `(112,84,632,570)`.
- right panel returns top `(744,84,168,199)`, tile `(744,283,168,42)`, bottom `(744,661,168,23)`.
- all six 1024 button rects are exactly `(744,283/325/367/409/451/619,168,42)`.
- hit-test at `(760,300)` returns Single Player and at the old scaled-only location outside retail rect does not.
- active render helper uses `compute_layout`, not responsive scaling.

Retain existing 800x600 tests and add a regression proving no responsive scaling path is active for 1024x768.

## Architectural Decisions

Follow the existing pattern: hand-authored Rust layout model backed by Ghidra reports and unit tests. Do not parse Win32 resources at runtime for this patch.

Do not introduce a generic shell viewport transform. It would make render/input harder to audit and risks hiding the exact per-control anchoring exceptions that the shell reports keep surfacing.

Do not solve skirmish shell missing controls in this patch. That is a separate, larger owner-draw implementation with its own report and state model changes.

## Alternatives Considered

### A. Activate corrected retail layout directly

Recommended. Smallest patch, aligns render and input, and gives exact tests for the known 1024x768 failure.

### B. Keep 800x600 base layout and render through a viewport transform

Rejected for now. It can center simple art, but it obscures per-control rules and still needs special cases for version/tooltip/title/button snapping. It also complicates hit testing.

### C. Keep responsive scaling as an optional mode

Rejected for the active shell. It is explicitly non-retail and visibly wrong at 1024x768. It can stay only as dead/test helper code if there is a dev reason, but not in the parity path.

## Implementation Handoff

Implement option A after a quick title/static placement spot-check. The first patch should be limited to main-menu shell layout/render/input and tests. Acceptance is `cargo test -q main_menu_shell` plus a manual or screenshot check at 1024x768 showing the movie at `(112,84)` and the right panel at `(744,84)`.
