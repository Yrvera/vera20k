# Skirmish Shell Chrome 800x600 Trace

Scenario: open the standard Yuri's Revenge offline Skirmish setup dialog resource `0x102` at `800x600` and verify only the first-paint shell chrome: parent background, right-panel `SDTP` / `SDBTNBKGD` / optional `SDBTNANM` frame `10` / `SDBTM` stack, `SDBTM` clipping versus scaling, lower strip, and semantic draw order.

## Verdict

PASS: 4 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

The experimental Rust Skirmish shell contains most 800x600 chrome geometry and asset choices, but it is not the default player path. When the dev shell path is enabled, the most visible first-paint chrome error is `SDBTM.SHP`: gamemd draws the top 23 rows of a 168x65 SHP through a 168x23 destination/clipping rect, while Rust samples the full 65-pixel source into a 23-pixel destination, vertically compressing the bottom cap.

## Active YR Path

Verified binary path is active in standard YR:

`FUN_006AE2C0 -> FUN_0072CF40 -> CreateDialogIndirectParamA(dialog 0x102, proc 0x006AE3F0) -> FUN_00622B50 -> WM_PAINT_Handler -> RightPanel__Draw -> Background_Overlay -> child control paints -> Skirmish WM_PAINT preview branch`.

Read-only Ghidra spot checks performed this run:

- `0x006AE2C0`: standard offline Skirmish launcher calls background loader, creates/shows dialog, pumps until Start `0x617` or Back `0x5C0`, then frees resources.
- `0x006AE3F0`: dialog proc delegates to common shell handling first; on `WM_PAINT`, it only runs preview marker work after common paint returns.
- `0x00621E90`: parent `WM_PAINT_Handler` calls `RightPanel__Draw`, then `Background_Overlay`, then blits the cached parent surface.
- `0x0072E450`: right panel draw order is `SDTP`, repeated `SDBTNBKGD`, conditional repeated `SDBTNANM` frame `10`, `SDBTM`, then width-selected `LWSCRN*`.
- `0x0072EC70`: computes the 800x600 right-panel rects and tile count from live SHP header dimensions.
- `0x00623340`: shell metadata records are zero-filled before specific flags/fields are set.

## Stage Table

| Stage | Boundary Output | gamemd 800x600 | Rust 800x600 | Verdict |
|---:|---|---|---|---|
| 1 | Standard player reachability | Opening Skirmish reaches dialog `0x102` shell paint path. | Pixel shell is behind `dev_skirmish_shell_enabled`; normal path keeps egui Skirmish setup visible. | NOT-IMPLEMENTED |
| 2 | Parent background asset and rect | `Background_Overlay` selects `MnScrnLCoopGameSetup.shp` at exact width `800`; rect `(0,0,632,568)` using `MnScrnLCoopGameSetup.PAL` convert path. | `parent_background_role(800)` selects `CoopGameSetup800`; `push_entry_native` draws at `(0,0)` with retail decoded `632x568`. | PASS |
| 3 | Right-panel geometry | `SDTP=(632,0,168,199)`, `SDBTNBKGD first=(632,199,168,42)`, `tile_count=9`, `SDBTM=(632,577,168,23)`. | `compute_layout(800,600)` produces the same top/tile/count/bottom rects. | PASS |
| 4 | Right-panel asset frames and palettes | `SDTP` frame `0` with shell palette, `SDBTNBKGD` frame `0` with `SHELL2`, `SDBTM` frame `0` with shell palette. | Atlas loads `SDTP.SHP#0/SHELL.PAL`, `SDBTNBKGD.SHP#0/SHELL2.PAL`, `SDBTM.SHP#0/SHELL.PAL`. | PASS |
| 5 | `SDBTNANM` frame `10` overlay first-paint presence | Binary loop draws frame `10` only when the `RightPanel__Draw` flag reaches `0`; this run did not prove the first-paint value after all init helpers. | Rust always returns `true` from `right_panel_frame10_overlay_active` and emits 9 overlays at `(644,199 + row*42,156,42)`. | UNCHECKED |
| 6 | `SDBTM` source bounds | `SDBTM.SHP` native `168x65`; destination/clipping rect height is `23`, so gamemd exposes top 23 rows at 1:1. | `push_entry` uses the full atlas UV for `SDBTM` and destination `168x23`, scaling 65 source rows down to 23. | FAIL |
| 7 | Lower strip asset and rect | Non-640 width selects `LWSCRNL.SHP`, frame `0`, rect `(0,568,632,32)`. | `lower_strip_role(800)` selects `LwscrnlLarge`; `lower_strip_rect` returns `(0,568,632,32)`. | PASS |
| 8 | Semantic parent/right-panel draw order | Common parent paint draws right panel stack including lower strip first, then `Background_Overlay`; child control paints follow. | Instance construction emits parent background first, then right-panel stack and lower strip. | FAIL |

## Failures

### Stage 6 - SDBTM source clipping

Player-visible difference: the right-panel bottom cap at `x=632,y=577,w=168,h=23` is vertically compressed in Rust instead of clipped. The retail source is `168x65`; gamemd uses a `168x23` draw/clipping rect from `RightPanel__ComputeLayoutRects` and `RightPanel__Draw`. Rust calls `push_entry` for `layout.right_panel.bottom`, and `push_entry` maps the full atlas UV into the shorter destination.

Our code:

- `src/app_skirmish_shell_render.rs:76-91` copies full `entry.uv_size` into every `SpriteInstance`.
- `src/app_skirmish_shell_render.rs:549-550` draws `atlas.right_panel_bottom_sdbtm` through `push_entry`.
- `src/ui/skirmish_shell/layout.rs:136-144` computes the same 23-pixel destination height as gamemd, so the mismatch is source sampling, not layout.

gamemd evidence:

- `RightPanel__ComputeLayoutRects @ 0x0072EC70`: at 800x600 computes `SDBTM=(632,577,168,23)`.
- `RightPanel__Draw @ 0x0072E450`: calls `CC_Draw_Shape(DAT_00B0FA38, frame 0, DAT_00B0FC28, ...)`; retail `SDBTM.SHP` is `168x65`.

### Stage 8 - Semantic draw order

Player-visible difference at exactly 800x600: none expected from current rectangles because the parent background `(0,0,632,568)`, lower strip `(0,568,632,32)`, and right panel `(632,0,168,600)` abut without overlap. It is still not semantically equal to gamemd and will matter if any recovered source bounds or overlap behavior changes.

Our code:

- `src/app_skirmish_shell_render.rs:510-560` emits parent background first, then right panel pieces, then lower strip.
- `src/app_skirmish_shell_render.rs:462-493` records the same parent-first semantic order in the test helper.

gamemd evidence:

- `WM_PAINT_Handler @ 0x00621E90`: calls `RightPanel__Draw` before fetching parent background fields and calling `Background_Overlay`.
- `RightPanel__Draw @ 0x0072E450`: draws `SDTP`, repeated `SDBTNBKGD`, conditional repeated `SDBTNANM` frame `10`, `SDBTM`, and `LWSCRNL`/`LWSCRNS` before returning to `WM_PAINT_Handler`.

## Not Implemented

### Stage 1 - Standard player reachability

Player-visible difference: by default, a player opening Skirmish in Rust does not see the recovered `0x102` shell chrome at all; the pixel shell is an opt-in research/dev path.

Our code:

- `src/app.rs:142-143` documents the opt-in dev shell path.
- `src/app.rs:394-405` enables it only from `DEV_SKIRMISH_SHELL_ENV`.
- `src/app.rs:1201-1207` renders the Skirmish shell only when `state.dev_skirmish_shell_enabled` is true.

gamemd evidence:

- `FUN_006AE2C0` is the standard offline Skirmish launcher and reaches dialog `0x102` without a dev flag.

## Unchecked

### Stage 5 - First-paint `SDBTNANM` frame 10 presence

The asset, frame, dimensions, rect formula, and loop are verified, but this run did not prove the exact first-paint value of the `RightPanel__Draw` flag after all dialog initialization helpers. Rust currently forces the overlay active. Do not mark this stage PASS until the first-paint flag is computed literally or confirmed by retail screenshot/pixel capture.

## Adjacent Findings

- Owner-draw buttons, text, flags, map preview, and `STARTBUT.SHP` start markers are intentionally out of scope for this slot.
- Existing docs disagree historically on whether the 640 parent is `MNSCRNL` or `MNSCRNS`; the newer right-panel/background follow-up resolves this to `MNSCRNS` at 640 and does not affect this 800x600 trace.
- `mmpb.shp` is active elsewhere but is not part of the standard offline `0x102` first-paint chrome path.

## Sources

- `docs/research/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`
- Read-only Ghidra decompiles this run: `0x006AE2C0`, `0x006AE3F0`, `0x00621E90`, `0x0072E450`, `0x0072E730`, `0x0072EC70`, `0x0060CF00`, `0x00623340`.
