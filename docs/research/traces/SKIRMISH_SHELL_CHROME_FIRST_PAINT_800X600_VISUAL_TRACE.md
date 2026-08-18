# Skirmish Shell Chrome First Paint 800x600 Visual Trace

Scenario: enter standard offline Yuri's Revenge Skirmish setup at `800x600` and trace only the first visual paint composition: parent background role, right-panel top/middle/bottom/lower strip, `SDBTNANM.SHP` frame-10 overlay state, and draw order relative to child controls.

## Verdict

PASS: 7 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

The experimental Rust pixel shell now matches the verified first-paint chrome slice for the checked 800x600 pieces: right-panel geometry, asset/palette choices, skipped `SDBTNANM` frame-10 overlay, top-clipped `SDBTM`, lower strip, and parent-before-child paint layering. The player-visible blocker is that standard Skirmish entry still does not show this pixel shell by default; it is gated behind the dev/experimental shell path. Full screenshot pixel equality remains unchecked because this trace did not capture retail and Rust framebuffers.

## Pipeline

`standard Skirmish entry -> dialog 0x102 common parent paint -> RightPanel__Draw -> Background_Overlay -> child/control paint -> Skirmish preview/text overlays`

Gamemd active standard YR path is verified in existing Ghidra reports as:

`FUN_006AE2C0 -> FUN_0072CF40 -> CreateDialogIndirectParamA(dialog 0x102, proc 0x006AE3F0) -> FUN_00622B50 -> WM_PAINT_Handler @ 0x00621E90 -> RightPanel__Draw @ 0x0072E450 -> Background_Overlay @ 0x0072E730`.

## Stage Table

| Stage | Boundary Output | gamemd 800x600 | Rust 800x600 | Verdict |
|---:|---|---|---|---|
| 1 | Standard player reachability | Opening offline Skirmish reaches dialog `0x102` first-paint shell chrome directly. | `render_skirmish_shell` is used only when `dev_skirmish_shell_enabled` is true; default route is still non-pixel/egui setup. | NOT-IMPLEMENTED |
| 2 | Parent background asset and rect | `Background_Overlay` selects `MnScrnLCoopGameSetup.shp` at exact width `800`, drawn from `(0,0)` with `MnScrnLCoopGameSetup.PAL`; verified parent image is `632x568`. | `parent_background_role(800)` selects `CoopGameSetup800`; atlas maps `MnScrnLCoopGameSetup.shp`; render emits it natively at `(0,0)`. | PASS |
| 3 | Right-panel geometry | `SDTP=(632,0,168,199)`, first `SDBTNBKGD=(632,199,168,42)`, `tile_count=9`, `SDBTM=(632,577,168,23)`. | `compute_layout(800,600)` encodes the same top/tile/count/bottom rects. | PASS |
| 4 | Right-panel assets and palettes | `SDTP#0/SHELL.PAL`, `SDBTNBKGD#0/SHELL2.PAL`, `SDBTM#0/SHELL.PAL`, lower strip `LWSCRNL#0/SHELL.PAL`. | Atlas loads the same asset/frame/palette set. | PASS |
| 5 | `SDBTNANM` frame-10 first-paint overlay | Standard offline `0x102` first paint leaves gate byte zero; `RightPanel__Draw` skips all frame-10 rows, visible overlay count `0`. | `right_panel_frame10_overlay_active` returns `false`; semantic order contains no `RightPanelOverlaySdbtnanmFrame10`. | PASS |
| 6 | `SDBTM` source sampling | Native `SDBTM.SHP` is `168x65`; destination clip height is `23`, so source rows `0..22` draw 1:1. | `push_entry_top_clipped_native` clips UV height by `23/65` and emits size `[168,23]`. | PASS |
| 7 | Lower strip | Non-640 width selects `LWSCRNL.SHP`, rect `(0,568,632,32)`. | `lower_strip_role(800)` selects large strip; `lower_strip_rect` returns `(0,568,632,32)`. | PASS |
| 8 | Parent/chrome order relative to children | Common parent paint draws right-panel stack, lower strip, then parent background; child controls and Skirmish preview work paint after common handler returns. | Instance order emits right-panel pieces, lower strip, parent background, then owner-draw controls; later pass draws preview/markers/text above shell chrome. | PASS |
| 9 | Aggregate first-frame pixels | Retail framebuffer was not captured in this trace. | Rust framebuffer was not captured in this trace. | UNCHECKED |

## Not Implemented

Stage 1 - standard reachability: a normal player entering Skirmish in Rust still does not necessarily see the recovered pixel chrome. The code path is gated by `RA2_DEV_SKIRMISH_SHELL` / `dev_skirmish_shell_enabled`, and `render_skirmish_shell` is only called from the `GameScreen::MainMenu` branch when that flag is true. Player-visible effect: the screen can functionally enter setup but will not look retail-complete by default.

Rust evidence: `src/app.rs:67`, `src/app.rs:406`, `src/app.rs:1292`.

Gamemd evidence: `FUN_006AE2C0` calls the Skirmish background loader, creates dialog `0x102`, and pumps it as the normal offline Skirmish setup UI without a dev flag.

## Checked Passes

Right-panel geometry is numerically equal for the checked 800x600 rects: `SDTP=(632,0,168,199)`, tile origin `(632,199)`, `tile_count=9`, and `SDBTM=(632,577,168,23)`. Rust evidence is in `src/ui/skirmish_shell/layout.rs:336` and the parity assertions at `src/ui/skirmish_shell/layout.rs:707`.

`SDBTNANM` frame 10 is correctly absent on first paint. Gamemd evidence is the data `+0xD4` zero-fill/read/inversion in `SKIRMISH_SDBTNANM_FRAME10_FIRST_PAINT_FLAG_GHIDRA_REPORT.md`; Rust evidence is `src/app_skirmish_shell_render.rs:1061` and the test at `src/app_skirmish_shell_render.rs:2258`.

`SDBTM` no longer has the old compressed-source mismatch. Rust now uses `push_entry_top_clipped_native`, and the local test verifies `uv_size.y=0.23` for a `23/65` visible slice. Rust evidence: `src/app_skirmish_shell_render.rs:141`, `src/app_skirmish_shell_render.rs:1148`, and `src/app_skirmish_shell_render.rs:2123`.

Semantic order now matches the common parent/chrome sequence for this slice. Rust evidence: `src/app_skirmish_shell_render.rs:1067` and the emission order at `src/app_skirmish_shell_render.rs:1119`.

## Unchecked

Full pixel equality is unchecked. This report compares the recovered numeric/layout/asset decisions against current Rust source; it did not run a retail capture or a Rust framebuffer screenshot/pixel diff for the whole first frame.

## Adjacent Findings

Right-panel static text, combo internals, flag PCX statics, map preview content, Choose Map modal visuals, and owner-draw button internals are intentionally out of scope for this slot. They matter for making the UI look complete, but they should be covered by their own trace-swarm slots.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SDBTNANM_FRAME10_FIRST_PAINT_FLAG_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`
- Rust files checked: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/ui/skirmish_shell/layout.rs`.
