# Skirmish Right-Panel Clipping/Text 640/800 Visual Trace

Date: 2026-05-22

Scenario: standard offline Yuri's Revenge Skirmish setup dialog `0x102`, comparing only the right-panel composition at `640x480` and `800x600`: `SDTP`, repeated `SDBTNBKGD`, `SDBTM`, lower strip, and right-panel static title/game/map text rectangles, clipping, alignment, and color.

Scope boundaries: no Choose Map modal, no dropdown rows, no checkbox/trackbar audit, no map-preview marker trace except where the preview static constrains right-panel layout.

## Verdict

PASS: 6 | FAIL: 0 | UNCHECKED: 4 | NOT-IMPLEMENTED: 2

The current Rust implementation has caught up on the most visible right-panel geometry issue from older traces: `SDBTM.SHP` is now submitted through a top-clipped native-source helper instead of scaling the full 65-pixel source into the short bottom remainder. The right-panel static text rects are also now modeled and rendered through scissored text draws.

Remaining player-visible risk is concentrated in two areas: the recovered pixel Skirmish shell is still dev-gated rather than the standard player path, and the three right-panel static labels are drawn as immediate full text instead of the binary kind-1 reveal/animation statics. Exact final text color and final rendered pixels at both resolutions remain unchecked because no retail screenshot/pixel capture was computed in this slot.

## Active YR Evidence

Read-only Ghidra spot checks confirmed the standard YR path and the same active functions cited by prior docs:

- `0x72EC70` decompiles as `RightPanel__ComputeLayoutRects(screen_w, screen_h)`, using live SHP dimensions and integer tile counts.
- `0x72E450` decompiles as `RightPanel__Draw`, drawing `SDTP`, repeated `SDBTNBKGD`, conditional `SDBTNANM`, `SDBTM`, then `LWSCRN*`.
- `0x621E90` decompiles as the common shell parent `WM_PAINT` handler that calls `RightPanel__Draw`, then `Background_Overlay`.
- `0x621040` decompiles as the shell text wrapper that uses the caller rect as layout and clip, with vertical-centering only when flag bit `0x04` is set.

Prior verified docs confirm standard offline Skirmish reaches dialog `0x102` through `FUN_006AE2C0 -> FUN_006AE3F0`, and that `0x694`, `0x6EC`, and `0x5A8` are active right-panel static text controls in YR.

## Findings

### 1. Standard Rust reachability

gamemd output: opening offline Skirmish reaches dialog `0x102` and paints the recovered right-panel shell by default.

Rust output: the pixel shell is only rendered from `GameScreen::MainMenu` when `state.dev_skirmish_shell_enabled` is true. Otherwise the normal path uses the existing egui/menu flow.

Verdict: NOT-IMPLEMENTED for standard player reachability. The visual work exists behind the dev shell gate, but it is not yet the normal Skirmish setup screen.

Rust evidence: `src/app.rs:406` reads `RA2_DEV_SKIRMISH_SHELL`; `src/app.rs:1292` gates `render_skirmish_shell` on `state.dev_skirmish_shell_enabled`.

gamemd evidence: prior active-path reports plus read-only Ghidra confirmation of the common paint chain through `0x621E90`.

### 2. Right-panel destination rects at 640x480

gamemd output:

- `SDTP`: `(472,0,168,199)`
- first `SDBTNBKGD`: `(472,199,168,42)`
- tile count: `6`
- `SDBTM`: `(472,451,168,29)`

Rust output: `right_panel_rects(640,480)` computes the same values: `effective_right=640`, `tile_count=(480-199)/42=6`, `bottom=(472,451,168,29)`.

Verdict: PASS.

Rust evidence: `src/ui/skirmish_shell/layout.rs:336` through `src/ui/skirmish_shell/layout.rs:364`.

gamemd evidence: `RightPanel__ComputeLayoutRects @ 0x72EC70`, verified active in standard YR.

### 3. Right-panel destination rects at 800x600

gamemd output:

- `SDTP`: `(632,0,168,199)`
- first `SDBTNBKGD`: `(632,199,168,42)`
- tile count: `9`
- `SDBTM`: `(632,577,168,23)`

Rust output: `right_panel_rects(800,600)` computes the same values: `effective_right=800`, `tile_count=(600-199)/42=9`, `bottom=(632,577,168,23)`.

Verdict: PASS.

Rust evidence: `src/ui/skirmish_shell/layout.rs:336` through `src/ui/skirmish_shell/layout.rs:364`.

gamemd evidence: `RightPanel__ComputeLayoutRects @ 0x72EC70`, verified active in standard YR.

### 4. `SDTP` and `SDBTNBKGD` source use

gamemd output: `RightPanel__Draw` submits frame `0` of `SDTP.SHP` and frame `0` of `SDBTNBKGD.SHP` at native `168`-wide rects. The tile loop advances Y by the tile SHP height.

Rust output: `build_skirmish_shell_instances` submits `atlas.right_panel_top_sdtp` to `layout.right_panel.top`, then repeats `atlas.right_panel_tile_sdbtnbkgd` for `layout.right_panel.tile_count`, advancing by `layout.right_panel.tile.h`.

Verdict: PASS for the scoped rect/source usage. Final palette pixels are still covered by the aggregate pixel-capture unchecked item.

Rust evidence: `src/app_skirmish_shell_render.rs:1119` through `src/app_skirmish_shell_render.rs:1132`.

gamemd evidence: `RightPanel__Draw @ 0x72E450`.

### 5. `SDBTM` source clipping versus scaling

gamemd output: `SDBTM.SHP` native source is `168x65`; `RightPanel__Draw` passes only the computed destination origin, so the destination clip exposes top source rows only. At `640x480`, visible source rows are `0..28`; at `800x600`, visible source rows are `0..22`.

Rust output: `push_entry_top_clipped_native` computes `draw_h = min(rect.h, src_h)`, scales V UV height by `draw_h / src_h`, and submits the shortened quad without vertical resampling of the full source. At `640x480`, that is `29/65`; at `800x600`, `23/65`.

Verdict: PASS. This was a previous player-visible compression mismatch, but current code now matches the verified top-clip model.

Rust evidence: `src/app_skirmish_shell_render.rs:141` through `src/app_skirmish_shell_render.rs:164`, called for `right_panel_bottom_sdbtm` at `src/app_skirmish_shell_render.rs:1148`.

gamemd evidence: `RightPanel__Draw @ 0x72E450`, `CC_Draw_Shape` clipping behavior documented in `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`.

### 6. Lower strip rect and asset selection

gamemd output: `640x480` selects `LWSCRNS.SHP` at `(0,448,472,32)`. `800x600` selects `LWSCRNL.SHP` at `(0,568,632,32)`.

Rust output: `lower_strip_role` selects `Lwscrns640` only at width `640`; all other scoped widths use `LwscrnlLarge`. `lower_strip_rect` places the asset at shell bottom using the native decoded width and height.

Verdict: PASS, assuming retail asset decode dimensions from the verified atlas inputs. Final palette pixels are unchecked.

Rust evidence: `src/app_skirmish_shell_render.rs:1007` through `src/app_skirmish_shell_render.rs:1048`, and draw at `src/app_skirmish_shell_render.rs:1152`.

gamemd evidence: `RightPanel__ComputeLayoutRects @ 0x72EC70` and `RightPanel__Draw @ 0x72E450`.

### 7. Static text rectangles and alignment

gamemd output:

- `640x480`: title `0x694` `(475,3,162,16)`, game type `0x6EC` `(489,167,135,16)`, map label `0x5A8` `(489,189,135,33)`.
- `800x600`: title `0x694` `(635,3,162,16)`, game type `0x6EC` `(649,167,135,16)`, map label `0x5A8` `(649,189,135,33)`.
- Dialog resource styles for these statics have low style bit `1`, so shell text is horizontally centered. The shell text wrapper top-anchors unless flag bit `0x04` is present.

Rust output: `compute_layout` now exposes matching `right_panel_text` rects, and `build_shell_text_draws` calls `push_static_label_draw` with `ShellAlign::H_CENTER` only, so it is horizontally centered and not vertically centered.

Verdict: PASS for rects and caller alignment flags.

Rust evidence: `src/ui/skirmish_shell/layout.rs:385` through `src/ui/skirmish_shell/layout.rs:389`; `src/app_skirmish_shell_render.rs:1480` through `src/app_skirmish_shell_render.rs:1509`.

gamemd evidence: `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` resource styles; `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`; `FUN_00621040 @ 0x621040`.

### 8. Static text clip behavior

gamemd output: `FUN_00621040` passes the caller `RECT` to the BitFont clip path. The same rect constrains layout and clipping.

Rust output: `shell_text::draw_in_rect` creates a scissor equal to the `TextRect` and wraps against `rect.w`, then each static text draw is rendered with that scissor.

Verdict: PASS for the caller-visible clip rectangle contract. Glyph-level raster equality remains unchecked.

Rust evidence: `src/render/shell_text.rs:57` through `src/render/shell_text.rs:111`; scissored draw use at `src/app_skirmish_shell_render.rs:1977` through `src/app_skirmish_shell_render.rs:1993`.

gamemd evidence: `FUN_00621040 @ 0x621040`.

### 9. Static text animation/reveal cadence

gamemd output: `0x694`, `0x6EC`, and `0x5A8` are kind-1 animated statics. `FUN_0060A5B0` initializes running byte clear, reveal count `1`, interval `0x1E`, step `1`, and reveal range `8`. Broadcast/start messages then set the running byte and reveal the text over time.

Rust output: `build_shell_text_draws` always submits the full title, game type, and map label strings when the dev shell renders. There is no per-static running byte, reveal count, timer interval, step, or reveal range.

Verdict: NOT-IMPLEMENTED. The final resting text can look acceptable, but entry/update animation is visibly different from retail.

Rust evidence: immediate text submission at `src/app_skirmish_shell_render.rs:1480` through `src/app_skirmish_shell_render.rs:1509`.

gamemd evidence: `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, active static setup via `FUN_0060A5B0`.

### 10. Static text color exactness

gamemd output: statics use shell default color `DAT_00AC18A4` unless disabled, with conversion through DirectDraw loss/shift globals in `FUN_00621040`.

Rust output: right-panel static labels use `SHELL_LABEL_TEXT_RGB = [0.94, 0.84, 0.42]`.

Verdict: UNCHECKED. The Rust value is a plausible yellow, but this trace did not compute retail final display RGB at 640/800 and compare literal pixel values.

Rust evidence: `src/app_skirmish_shell_render.rs:42`, used by `push_static_label_draw` at `src/app_skirmish_shell_render.rs:1417`.

gamemd evidence: `FUN_00621040 @ 0x621040`; `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`.

### 11. Dynamic game-type and map-label string contents

gamemd output: game type `0x6EC` is updated through `FUN_005E2EF0`; map label `0x5A8` is updated through `FUN_005E2F60` from the selected map name buffer.

Rust output: game type is currently localized from `GUI:Battle`; map label reads `maps[shell.selected_map_idx].display_name`.

Verdict: UNCHECKED. The default Battle/default-map case may match, but this slot did not compute the exact startup selected map record and mode string on both sides.

Rust evidence: `src/app_skirmish_shell_render.rs:1489` through `src/app_skirmish_shell_render.rs:1501`.

gamemd evidence: `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`.

### 12. Final rendered pixel capture at both resolutions

gamemd output: not captured in this slot.

Rust output: not captured in this slot.

Verdict: UNCHECKED. The numeric rect/source contracts are strong enough for implementation work, but final “looks good” validation still needs side-by-side screenshots or pixel sampling at `640x480` and `800x600`.

## Adjacent Findings

- `SDBTNANM` frame-10 first-paint state is adjacent to right-panel visual polish but outside this exact scenario, which named only `SDTP`, `SDBTNBKGD`, and `SDBTM`.
- Dropdown row text and scrollbar geometry are adjacent to Skirmish shell completeness, but were intentionally not traced here.
- Choose Map modal text/listbox rendering is a separate modal surface, not part of this right-panel trace.

## Implementation Handoff

1. Keep the current `SDBTM` top-clipped native-source path; do not revert to full-source scaling.
2. Promote the recovered Skirmish shell out of the dev-gated path only when the whole setup flow can use it as the normal player surface.
3. Add kind-1 static text reveal state if we care about retail transition/update polish, not just the final resting screen.
4. Capture retail and Rust screenshots/pixels at exact `640x480` and `800x600` to settle static text RGB, glyph raster, and aggregate draw-order/palette equality.

## Sources

- Read-only Ghidra: `RightPanel__ComputeLayoutRects @ 0x72EC70`, `RightPanel__Draw @ 0x72E450`, `WM_PAINT_Handler @ 0x621E90`, `FUN_00621040 @ 0x621040`.
- Existing research docs: `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_POSITIONING_SYSTEM_MODEL_SYNTHESIS.md`, `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`, `SKIRMISH_SHELL_CHROME_800X600_TRACE.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`.
- Rust read-only surfaces: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/render/shell_text.rs`.
