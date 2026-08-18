# High-Res Skirmish Shell Screenshot Parity Design

## Goal

Create a repeatable 1024x768 dev Skirmish shell verification path that captures aggregate high-resolution composition parity without changing the verified `>800` parent-background SHP policy.

## Architecture Context

The Skirmish shell is implemented as a dev-gated UI/render path, not as simulation logic. The main surfaces are:

- `src/ui/skirmish_shell/layout.rs`: computes dialog/control/chrome rectangles from render size.
- `src/app_skirmish_shell_render.rs`: builds Skirmish shell `SpriteInstance`s and exposes semantic draw-role tests.
- `src/render/skirmish_shell_chrome.rs`: loads shell PCX/SHP assets and palette-backed parent backgrounds.
- `src/app.rs`: gates the dev shell with `RA2_DEV_SKIRMISH_SHELL` and uses `render_width()` / `render_height()`.

The relevant binary model is already researched. The parent dialog is full-screen top-left at high resolution, while selected right-panel controls and shell chrome use centered 800x600 logical offsets. The exact `>800` parent-background SHP decision is resolved: fresh `>800` Skirmish does not load or draw `MnScrnLCoopGameSetup.shp`.

One prior high-res placement table is superseded for Start/Choose only: owner-draw Start `0x617` and Choose Map `0x5AA` route through the PCX-button snap helper, not the generic static right-anchor helper. Their high-res rects use `SDBTNANM.SHP` dimensions `156x42`.

This design stays in `ui/`, `render/`, and app-level verification code. It does not touch `sim/`.

## Impact Analysis

Primary touched surfaces for implementation:

- `src/app_skirmish_shell_render.rs`: expose or extend role/rect verification helpers if needed.
- `src/ui/skirmish_shell/layout.rs`: add missing invariant tests only if gaps are found.
- A new visual-check harness location, likely under `docs/visual-checks/` or a focused ignored integration test, depending on existing tooling fit.
- Optional output artifacts under `docs/visual-checks/skirmish-shell/`, ignored or explicitly curated according to repo convention.

Risk areas:

- Accidentally treating an asset cache as semantic state and drawing the 800 background above 800.
- Testing only role order while missing pixel placement drift.
- Producing screenshots with a different render scale, swapchain size, or asset set than the in-game dev shell.
- Comparing against stale docs instead of the current GT800 pointer lifecycle report.

## Chosen Approach

Use a screenshot parity harness first, then patch only observed high-res composition deltas.

The implementation sequence should be:

1. Verify and tighten deterministic 1024x768 role/rect assertions for the existing Skirmish shell builder.
2. Add a reproducible 1024x768 screenshot capture path for the dev Skirmish shell.
3. Produce a current Rust screenshot and annotate any visible deltas.
4. Patch only the deltas confirmed by the screenshot and existing Ghidra/layout reports.
5. Re-run the capture after each patch until the high-res composition matches the verified model.

The harness should make it hard to regress the resolved background decision: above 800, `ParentBackgroundMnscrns640` and `ParentBackgroundCoopGameSetup800` must both be absent.

## Tiny-Detail Ledger

- Parent dialog high-res host is `(0,0,W,H)`, not a centered 800x600 child. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`, `FUN_0060C4A0`.
- At 1024x768, right-panel top is `(744,84,168,199)`. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md` final placement table.
- At 1024x768, right-panel tile starts at `(744,283,168,42)`. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`.
- At 1024x768, right-panel bottom cap starts at `(744,661,168,23)`. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`.
- At 1024x768, Start is `(756,325,156,42)` and Choose Map is `(756,367,156,42)` because owner-draw buttons route through `FUN_0060B000` and snap to `SDBTNANM.SHP` dimensions. Source: `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`.
- At 1024x768, Back is `(756,619,156,42)`. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`.
- Ordinary non-allowlisted slot-table controls retain 800-layout pixel positions and are not globally centered or scaled. Source: `SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`.
- `MnScrnLCoopGameSetup.shp` is loaded only at exact width 800. Source: `SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`, `0x0072CF49..0x0072CF65`.
- Fresh `>800` parent-background SHP draw is skipped because `CC_Draw_Shape(null)` returns early. Source: `SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`, `0x004AED84..0x004AED8E`.
- Exact 800 remains a distinct `ParentBackgroundCoopGameSetup800` path. Source: `SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`.
- Lower strip uses `LWSCRNL` for non-640 widths and should be positioned from the common shell origin. Source: current Rust tests plus high-res hosting report.
- `SDBTM.SHP` bottom cap must be source-clipped, not vertically scaled. Source: `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`, citing `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`.
- Right-panel draw order is top, repeated tile, optional animation overlay, bottom cap, lower strip, then parent background overlay if any. Source: `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`.
- Dropdown content, scrollbar, and hit-test rects must stay within their computed dropdown/content rectangles at high res. Source: current dropdown implementation and combo/dropdown Ghidra reports cited by synthesis.

## Design

### Components

`SkirmishShellHighResInvariants`

An implementation-time helper or test-only module that constructs `compute_layout(1024, 768)` and validates all known high-res rectangles and role absence/presence. The repo already has several of these assertions; implementation should tighten missing coverage instead of duplicating existing tests. It should not depend on GPU state.

`SkirmishShellScreenshotCapture`

A visual-check harness that renders the dev Skirmish shell at a fixed 1024x768 render size using the same atlas and instance builder as runtime. It writes a PNG artifact for inspection and optional future image comparison.

`SkirmishShellVisualReport`

A small markdown output or checked artifact note under `docs/visual-checks/skirmish-shell/` that records the current screenshot path, render dimensions, enabled shell state, and any observed deltas.

### Interfaces / Contracts

- The capture path must use `compute_layout(1024, 768)` and `build_skirmish_shell_instances()` or the same runtime path that calls it.
- The invariant tests must assert no parent-background role appears at 1024x768.
- The screenshot artifact must state whether preview data and start markers are present, because those alter visible composition.
- Any future pixel comparison must compare like-for-like states: same selected map, same open dropdown state, same checkbox/trackbar state, same asset source.

### Data Flow

1. Build or load the existing `SkirmishShellChromeAtlas`.
2. Construct a deterministic `SkirmishShellState` and map list.
3. Compute `SkirmishShellLayout` at 1024x768.
4. Build instances through the normal Skirmish shell renderer.
5. Assert the role/rect invariants.
6. Render to an offscreen texture or controlled window/surface.
7. Save the screenshot and record observed deltas.

### Error Handling

- Missing retail assets should fail the screenshot capture with a clear asset name.
- Missing optional map preview data should be reported in the visual report, not silently treated as a layout failure.
- GPU/offscreen capture failures should not mutate any parity code; they should fail the harness and leave the invariant tests as the minimal check.

### Testing Strategy

Immediate tests:

- `compute_layout(1024, 768)` expected right-panel, owner-draw button, preview, and lower strip rects.
- `skirmish_shell_semantic_draw_order(1024,768,...)` excludes both parent-background roles.
- `parent_background_role(1024,768) == None` remains pinned.

Visual checks:

- Generate `1024x768` closed-dropdown screenshot.
- Generate `1024x768` one-open-dropdown screenshot after dropdown work is involved.
- Inspect right edge, bottom strip, preview area, right-panel labels, and button stack placement.

No deterministic sim/hash tests are needed.

## Architectural Decisions

- Keep the `>800` parent-background behavior as a hard invariant, not a screenshot-derived guess.
- Prefer a small visual-check harness over embedding screenshot comparison into normal `cargo test`, because GPU and retail asset availability can be environment-sensitive.
- Keep rect and role assertions in normal or focused tests where possible, because they are deterministic and cheap.
- Do not introduce a new UI layout abstraction for this pass; use the existing `compute_layout` and render-role helpers.

## Alternatives Considered

### Patch likely high-res deltas immediately

Rejected for first step. It risks changing verified behavior from inference and gives no artifact proving whether the total composition improved.

### Retail screenshot capture before Rust harness

Useful later, but not required to protect the known Rust invariants. The current issue is that our own high-res output needs repeatable capture and inspection first.

### Full automated pixel diff now

Deferred. It requires a stable retail reference, controlled palettes/assets, and exact state matching. A screenshot harness plus deterministic role/rect assertions gets the workflow moving without pretending we have a pixel oracle yet.
