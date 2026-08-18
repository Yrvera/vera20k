# Skirmish 0x102 First-Paint Composition vs Rust Draw Order - Ghidra Report

Date: 2026-05-22

Investigation mode: exhaustive-slice

Target question: Does active standard YR offline Skirmish dialog `0x102` first paint compose parent background, right-panel stack, lower strip, preview/start-position drawing, and cached parent-surface blit in an order that current Rust gets wrong?

Non-goals: combo internals, text caller rects/colors, Choose Map modal `0x6B`, gameplay launch, owner-draw button internals, and full start-marker projection math.

Evidence needed to mark COMPLETE: decompile plus assembly context for the common paint order, right-panel order, background selection, cached parent blit, Skirmish preview/start boundary, and current Rust scan of the matching render surfaces.

Stop conditions: stop once top-level composition and Rust deltas are resolved; record related control/text/modal gaps as out-of-scope rather than investigating them here.

Status: COMPLETE for the scoped first-paint composition order and Rust comparison.

## Executive Summary

The active `gamemd.exe` first-paint order for offline Skirmish dialog `0x102` is:

1. Standard Skirmish launcher creates dialog `0x102` with proc `0x006AE3F0`.
2. `0x006AE3F0` delegates `WM_PAINT` to common shell proc `0x00622B50` first.
3. Common `WM_PAINT_Handler` composes into the cached parent `BSurface`.
4. In mode `1`, common paint calls `RightPanel__Draw` first.
5. `RightPanel__Draw` draws `SDTP`, repeated `SDBTNBKGD`, optional repeated `SDBTNANM` frame `10`, `SDBTM`, then `LWSCRN*` lower strip.
6. Common paint calls `Background_Overlay` after the right-panel/lower-strip draw.
7. Common paint blits the cached parent `BSurface` to `DAT_00887310`.
8. Only after the common handler returns does Skirmish-specific `DrawStartPositions` draw the map preview surface and optional live start overlays.

Current dirty Rust has caught up on the highest-level chrome order: `skirmish_shell_semantic_draw_order` and `build_skirmish_shell_instances` now emit right panel, lower strip, parent background, owner-draw controls, then preview in a later pass. Older traces saying Rust emits the parent background first are stale for the current worktree.

The remaining composition-risk handoff from this slot is the preview/start overlay boundary: `gamemd.exe` draws preview pixels after the cached parent-surface blit, then draws live `STARTBUT.SHP` and numeric labels after preview. The marker and label path requests full destination-surface clipping, not fitted-preview-rect clipping. Current Rust still scissor-clips marker sprites and labels to the fitted preview rect in `render_skirmish_shell_with_atlas`.

## Verified Findings

### 1. Standard offline Skirmish reaches dialog `0x102`

Active in YR: Yes.

Evidence: `FUN_006ae2c0` decompile calls `FUN_0072cf40()` before `FUN_00622650(0)`, stores the resulting HWND in `DAT_00b0b59c`, runs the modal pump until result `0x617` or `0x5c0`, then cleans preview state and calls `FUN_0072cf90()`. Prior report assembly for the same path records dialog `0x102` and proc `0x006AE3F0`.

Why it matters: this is the normal player-visible offline Skirmish shell path, not a TS-only or research-only path.

### 2. Skirmish dialog proc delegates common paint before preview work

Active in YR: Yes.

Evidence: `FUN_006ae3f0` decompile starts with `FUN_00622b50(...)` and returns immediately if it returns nonzero. Assembly context at `0x006AE40A` shows `CALL 0x00622b50`, then `TEST EAX,EAX` / `JNZ`, and only later branches on `WM_PAINT`. The preview branch checks `DAT_00AC1154`, gets child `0x468`, calls `0x006067A0`, and calls `DrawStartPositions` at `0x006AE47B`.

Why it matters: preview/start-position rendering cannot underlie the parent cached surface; it is drawn after common shell chrome has reached the destination surface.

### 3. Common parent paint composes into a cached parent `BSurface` and blits once

Active in YR: Yes.

Evidence: `WM_PAINT_Handler` decompile allocates/reuses a `BSurface` stored in the parent record (`piVar9[4]`) when unsuppressed. It composes shell elements into that surface, then at the final block calls the destination surface vtable slot `+8` from `DAT_00887310`. Assembly context at `0x006223B3` shows `CALL dword ptr [EAX + 0x8]` immediately before function epilogue.

Why it matters: Rust does not need to copy the internal cached surface model, but the visible ordering must match: common chrome first as one parent-surface result, then Skirmish preview work after.

### 4. Common mode-1 paint calls right panel before parent background overlay

Active in YR: Yes for dialog `0x102` when the common mode-1 branch is unsuppressed and right-panel resources are ready.

Evidence: `WM_PAINT_Handler` decompile in mode `piVar9[0x2c] == 1` calls `RightPanel__Draw((char)piVar9[0x35] == '\0')`, then re-reads parent background fields and calls `Background_Overlay(iVar10,iVar4,iVar7)`. Assembly context confirms `CALL 0x0072e450` at `0x00621FFE`, and later `CALL 0x0072e730` at `0x0062211B`. Optional `Sidebar_TopHighlight`, `Minimap_Button`, and `RadarBackground` checks occur after `Background_Overlay`.

Current Rust comparison: current `src/app_skirmish_shell_render.rs` emits right-panel top/tile/optional overlay/bottom, then lower strip, then parent background in `build_skirmish_shell_instances`; `skirmish_shell_semantic_draw_order` records the same role order. This now matches the verified top-level gamemd order.

### 5. Right-panel internal order includes lower strip before returning to background overlay

Active in YR: Yes.

Evidence: `RightPanel__Draw` decompile draws `g_SDTP_SHP`, loops `g_SDBTNBKGD_SHP`, conditionally loops `g_SDBTNANM_SHP` frame `10` when `param_3 == '\0'`, draws `DAT_00b0fa38` (`SDBTM.SHP`), then width-selects `DAT_00b0fae8` (`LWSCRNS.SHP`) at width `640` or `DAT_00b0fa54` (`LWSCRNL.SHP`) otherwise. Assembly contexts: `0x0072E547`, `0x0072E594`, `0x0072E60D`, `0x0072E68C`, `0x0072E6CD` / `0x0072E6F7`, final lower-strip `CALL 0x004aed70` at `0x0072E71F`.

Current Rust comparison: current layout computes right panel and lower strip as separate roles and uses `push_entry_top_clipped_native` for `SDBTM`, preserving the newer clipped-source fix rather than the stale full-source scaling path.

### 6. Parent background selection is width-gated and may be no-op above 800

Active in YR: Yes; `>800` is conditional on high-resolution mode.

Evidence: `Background_Overlay` decompile selects the small pointer only when `g_ScreenWidth == 0x280`; otherwise it calls `CC_Draw_Shape` with the alternate pointer. Assembly context at `0x0072E7AD` compares against `0x280`, with small-path call at `0x0072E7DF` and non-640 call at `0x0072E815`. `FUN_0072cf40` only loads `DAT_00B0FA18` when `g_ScreenWidth == 800` (`0x0072CF49..0x0072CF65`), and cleanup clears it at `0x0072CFCB`. `CC_Draw_Shape` null SHP test at `0x004AED84..0x004AED8E` returns before frame lookup.

Current Rust comparison: `parent_background_role` returns `MNSCRNS` at width `640`, `MnScrnLCoopGameSetup` at exact `800`, and `None` above `800`. This matches the verified normal fresh lifecycle.

### 7. Preview surface and live start overlays are after parent blit, with a separate clipping boundary

Active in YR: Yes when `DAT_00AC1154` and its inner preview surface are non-null; live overlays are conditional on available start count.

Evidence: `0x006AE47B` calls `DrawStartPositions` after common paint returns. `DrawStartPositions` first validates, converts child `0x468`, aspect-fits the preview source, calls the destination surface path around `0x00640860` to blit the preview surface, then later draws `STARTBUT.SHP` via `CC_Draw_Shape` at `0x006409D2` and numeric labels via the font path. The marker call obtains its clip/bounds from `DAT_00887310 +0x78` at `0x006409A7..0x006409B6`, not from the fitted preview rect. This matches `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`.

Current Rust comparison: Rust draws the preview texture after shell chrome, which matches the boundary. It still scissor-clips marker sprites and marker labels to the fitted preview rect in `render_skirmish_shell_with_atlas`; that is stricter than the verified gamemd overlay clipping boundary.

## Current Rust Status

Affected files scanned:

- `src/app_skirmish_shell_render.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app.rs`

The current worktree is ahead of older trace docs:

- `skirmish_shell_semantic_draw_order` starts with right-panel roles, includes lower strip before parent background, omits parent background above `800`, and only records preview/start marker roles when preview/overlay availability is supplied.
- `build_skirmish_shell_instances` emits right-panel stack, lower strip, then parent background; button/combo/checkbox/trackbar/flag/dropdown drawing follows.
- `render_skirmish_shell_with_atlas` draws shell chrome first, then preview texture, then marker sprites, marker labels, bare text, and scissored text.
- `SDBTM` currently uses `push_entry_top_clipped_native`, so the old full-source-scaling mismatch described in older docs is no longer current for this file.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Common paint order is right-panel stack, lower strip, parent background, optional extras, cached blit; preview work follows after common handler returns. | Preserve current order; do not reintroduce parent-first ordering from stale trace docs. | `src/app_skirmish_shell_render.rs` semantic helper and instance construction. | At `800x600`, semantic roles begin with `SDTP`, nine `SDBTNBKGD`, no first-paint `SDBTNANM` unless explicitly enabled, `SDBTM`, `LWSCRNL`, then `ParentBackgroundCoopGameSetup800`; preview roles are absent unless a real preview is available. | `skirmish_0x102_first_paint_order_matches_common_parent_blit_boundary` | Medium: stale reports still say Rust was parent-first and can mislead a future cleanup. |
| Fresh `>800` Skirmish entry passes null alternate background to `CC_Draw_Shape`; parent background is no-op, while lower strip/right panel still draw around the centered common shell origin. | Keep `parent_background_role(width > 800) == None`; keep lower strip large asset active. | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`. | At `1024x768`, semantic order contains `LowerSideLwscrnl`, contains no parent background role, and the lower strip rect remains centered at `(112,652,632,32)`. | `skirmish_1024_first_paint_keeps_parent_blank_but_draws_lower_strip` | High if someone stretches/reuses the 800 background to make the screen feel fuller. |
| `DrawStartPositions` draws live `STARTBUT.SHP` and labels after the preview surface, but those overlays request full destination-surface clipping rather than fitted-preview-rect scissor clipping. | Remove or relax marker/label preview-rect scissor once live overlay parity is implemented; keep preview pixels themselves clipped/aspect-fitted to child `0x468`. | `render_skirmish_shell_with_atlas`, `build_start_marker_instances`, `build_start_marker_label_instances`. | A start marker whose sprite crosses the fitted preview image edge is submitted and clipped only by the render target/surface bounds, while the preview image remains aspect-fitted in child `0x468`. | `start_marker_overlays_use_destination_surface_clip_not_preview_rect` | Medium: visible only for edge start positions, but current scissor can cut marker pixels that gamemd would show. |

## Negative Facts / Do Not Do

- Do not draw the parent background before the right-panel/lower-strip stack. Active in YR: No for standard `0x102` first paint. Evidence: `RightPanel__Draw` call at `0x00621FFE` precedes `Background_Overlay` call at `0x0062211B`.
- Do not stretch or reuse `MnScrnLCoopGameSetup.shp` above width `800` for a fresh Skirmish entry. Active in YR: No for normal fresh `>800`. Evidence: exact-width loader compare at `0x0072CF49`, cleanup clear at `0x0072CFCB`, null-SHP no-op at `0x004AED84..0x004AED8E`.
- Do not treat `MNSCRNL.SHP` as the standard `640` dialog `0x102` parent background. Active in YR: No for this role. Evidence: loader table stores `0x00844CE0 -> MNSCRNS.SHP` result to `DAT_00B0FB50` at `0x0072EBAA`; `MNSCRNL.SHP` goes to `DAT_00B0FA04`.
- Do not scale the full `SDBTM.SHP` source into the short bottom remainder rect. Active in YR: No. Evidence: `RightPanel__Draw` submits native `SDBTM` frame through `CC_Draw_Shape` with a shorter destination/clip rect; current Rust already uses top-clipped native source.
- Do not infer first-paint `SDBTNANM` frame `10` should draw unless the caller flag says so. Active in YR: Conditional. Evidence: `RightPanel__Draw` loops frame `10` only when `param_3 == '\0'`; current Rust first-paint helper returns false.

## Remaining Uncertainty

- Exact runtime history for abnormal stale `DAT_00B0FA18` above `800` without normal cleanup remains a runtime-watchpoint question, but the standard fresh lifecycle is resolved.
- This slot did not verify combo/dropdown, text rect/color, owner-draw button, or Choose Map modal composition; those remain separate swarm slots.

## Stale Docs / Replacement Wording

- `docs/research/skirmish-ui/SKIRMISH_SHELL_CHROME_800X600_TRACE.md` lines describing current Rust parent-first ordering are stale. Replacement wording: "Current Rust emits right-panel stack and lower strip before parent background in `build_skirmish_shell_instances`, and `skirmish_shell_semantic_draw_order` records the same order; this now matches the verified gamemd common parent paint order for the top-level chrome."
- `docs/research/skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md` and `docs/research/skirmish-ui/SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md` have stale `MNSCRNL.SHP`-as-640-parent wording. Replacement wording: "For standard offline Skirmish dialog `0x102`, parent `+0xE0` receives `DAT_00B0FB50`, and the corrected loader-table mapping is `DAT_00B0FB50 = MNSCRNS.SHP`; `MNSCRNL.SHP` is `DAT_00B0FA04` and is not the 640 parent background for this dialog."

## Sources

- Live Ghidra decompile / assembly context: `FUN_006ae2c0`, `FUN_006ae3f0`, `FUN_00622b50`, `WM_PAINT_Handler`, `RightPanel__Draw`, `Background_Overlay`, `DrawStartPositions`, `CC_Draw_Shape`, `FUN_0072cf40`, `FUN_0072cf90`.
- Assembly contexts checked: `0x006AE40A`, `0x006AE47B`, `0x00621FFE`, `0x0062211B`, `0x006223B3`, `0x0072E547`, `0x0072E594`, `0x0072E60D`, `0x0072E68C`, `0x0072E6CD`, `0x0072E6F7`, `0x0072E71F`, `0x0072E7AD`, `0x0072E7DF`, `0x0072E815`, `0x006409A7`, `0x006409D2`, `0x004AED84`, `0x0072CF49`, `0x0072CF65`, `0x0072CFCB`.
- Prior reports cross-checked: `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`, `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`, and `SKIRMISH_SHELL_CHROME_800X600_TRACE.md`.
- Rust scan: `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app.rs`.
