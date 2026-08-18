# Skirmish SDBTM Bottom Cap Source Clip - Ghidra Research Report

**Address(es):** `0x0072E450`, `0x0072EC70`, `0x004AED70`, `0x00421B60`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline YR Skirmish dialog `0x102` right-panel `SDBTM.SHP` bottom-cap destination, source, and clip behavior.  
**Non-Scope:** Full right-panel chrome order, `SDBTNANM` first-paint state, child controls, map preview, WOL dialogs, and lower strip behavior except as adjacent draw context.  
**Confidence:** High for standard 800x600 and the reusable clipping rule; Medium-high for 640/1024 formula outputs because they are statically derived but not screenshot-captured in this slot.  
**Active in YR:** Yes. The standard offline Skirmish launcher reaches dialog `0x102`, delegates `WM_PAINT` to common shell paint, and calls `RightPanel__Draw`.

## 1. Overview

`SDBTM.SHP` is drawn at the right-panel bottom remainder origin, using frame `0` and `SHELL.PAL`. The draw call does not scale the native `168x65` frame into the computed remainder height. It draws the native frame 1:1 at the computed origin and lets the target clip rectangle cut off the bottom.

At 800x600 this means the frame is submitted at `(632,577)`. Because the common paint clip is the parent surface `(0,0,800,600)`, only destination rows `577..599` are visible, corresponding to source rows `0..22`. The visible output is the top 23 rows of `SDBTM.SHP`, not a resampled full-frame 168x23 image and not a source window from lower in the SHP.

## 2. Key Offsets and Globals

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00B0FA38` | Loaded `SDBTM.SHP` pointer | `Sidebar_RightPanel_SHP_Loading` stores call result to `DAT_00B0FA38` at `0x0072EBFE`; later read at `0x0072E68C` | Yes |
| `DAT_00B0FC20` | `SDTP` top rect `{x,y,w,h}` | allocated/written from `g_SDTP_SHP` dimensions at `0x0072ED41..0x0072ED6D` | Yes |
| `DAT_00B0FC24` | repeated `SDBTNBKGD` tile rect `{x,y,w,h}` | written at `0x0072EDB2..0x0072EDE9` | Yes |
| `DAT_00B0FA20` | tile count `(effective_h - SDTP.h) / SDBTNBKGD.h`, signed integer truncation | `IDIV [DAT_00B0FC24+0xC]` and store at `0x0072EE88..0x0072EE95` | Yes |
| `DAT_00B0FC28` | bottom-cap remainder rect `{x,y,w,h}`; draw call consumes only `x,y` directly | writes at `0x0072EEA0..0x0072EED2`; draw uses `[+0]`/`[+4]` at `0x0072E65C..0x0072E667` | Yes |
| `DAT_00B0FBCC` | `SHELL.PAL` convert object used for `SDBTM` | read before `CC_Draw_Shape` at `0x0072E697` | Yes |

## 3. Core Logic

### 3.1 Active Skirmish path

Standard offline Skirmish calls the Skirmish shell loader and creates dialog `0x102`. The dialog proc calls the common shell proc first; on `WM_PAINT`, that common proc calls `WM_PAINT_Handler`, which calls `RightPanel__Draw` on the non-WOL mode-1 path.

**Evidence:** `FUN_006AE2C0` calls `FUN_0072CF40` then creates the dialog; prior address context `0x006AE317..0x006AE328`. `FUN_006AE3F0` delegates to `FUN_00622B50` before its own `WM_PAINT` branch. `WM_PAINT_Handler @ 0x00621E90` calls `RightPanel__Draw` when mode `+0xB0 == 1` and right-panel resources are ready.  
**Active in YR:** Yes, standard offline Skirmish dialog `0x102`.

### 3.2 Bottom-cap rect formula

`RightPanel__ComputeLayoutRects(screen_w, screen_h)` computes the bottom-cap rect after the top cap and an integer number of full background tiles:

```text
tile_count = (effective_h - SDTP.h) / SDBTNBKGD.h
bottom_y   = tile_y + tile_count * SDBTNBKGD.h
bottom_x   = effective_right - SDBTM.w
bottom_w   = SDBTM.w
bottom_h   = effective_h - (SDTP.h + tile_count * SDBTNBKGD.h)
```

The division is signed integer `IDIV`, so it truncates toward zero. In normal screen heights the dividend is positive.

For 800x600 with verified dimensions `SDTP=168x199`, `SDBTNBKGD=168x42`, `SDBTM=168x65`:

```text
tile_count = (600 - 199) / 42 = 9
bottom_y   = 199 + 9*42 = 577
bottom_h   = 600 - (199 + 9*42) = 23
bottom     = (632,577,168,23)
```

For 1024x768 the centered shell formula yields `(744,661,168,23)`. For 640x480 it yields `(472,451,168,29)`.

**Evidence:** `0x0072EE88..0x0072EE95` computes/stores `DAT_00B0FA20`; `0x0072EEA0..0x0072EED2` writes `DAT_00B0FC28` with `x=local_right-SDBTM.w`, `y=tile_y+tile_count*tile_h`, `w=SDBTM.w`, `h=effective_h-(SDTP.h+tile_count*tile_h)`. Existing Rust tests encode the same expected rects at `src/ui/skirmish_shell/layout.rs:245`.  
**Active in YR:** Yes for standard shell layout.

### 3.3 Draw call consumes `DAT_00B0FC28` as origin, not as scaling box

`RightPanel__Draw` reads `DAT_00B0FC28`, copies only `[+0]` and `[+4]` into the point/local position passed to `CC_Draw_Shape`, then passes `DAT_00B0FA38` (`SDBTM.SHP`), frame `0`, flags `0x400`, and `DAT_00B0FBCC`.

No source-window argument is passed for `SDBTM`: the optional secondary-shape/source arguments are zero at the call site. No argument tells `CC_Draw_Shape` to squeeze 65 rows into 23 rows.

**Evidence:** `RightPanel__Draw @ 0x0072E450`; assembly around `0x0072E65C..0x0072E69F` reads `DAT_00B0FC28`, pushes zero optional arguments, pushes `0x400`, pushes shape pointer `DAT_00B0FA38`, sets `EDX=DAT_00B0FBCC`, `ECX=surface`, and calls `0x004AED70`.  
**Active in YR:** Yes for standard offline `0x102` parent paint.

### 3.4 Clip semantics expose the top rows

`CC_Draw_Shape @ 0x004AED70` resolves the frame rect with `SHP_frame_rect_getter @ 0x0069E7E0`, builds a temporary frame surface at the frame's native width/height, then clips the destination against the target clip rectangle. The clipping helper `AlphaShapeClass__ClipRect @ 0x00421B60` reduces width/height when the destination exceeds the clip bounds. It only advances source offsets when clipping removes pixels from the left or top; bottom clipping reduces height and leaves source `y` at zero.

For the 800x600 bottom cap, destination y is `577`, native frame height is `65`, and the parent clip bottom is `600`. Clipping reduces drawn height to `23`; because the clipped edge is the bottom edge, source y remains `0`.

**Evidence:** `CC_Draw_Shape @ 0x004AED70` calls target rect/clip setup at `0x004AEDB3..0x004AEE46`, resolves frame rect/data at `0x004AEE6D..0x004AEEA0`, and reaches the standard blitter path at `0x004AF1C9..0x004AF269`. `AlphaShapeClass__ClipRect @ 0x00421B60` writes a clipped `{x,y,w,h}` and adjusts optional source offsets only from `param_3.x - clipped_x` / `param_3.y - clipped_y`; bottom trimming changes only height.  
**Active in YR:** Yes. This is the active `CC_Draw_Shape` path used by `RightPanel__Draw` with flags `0x400` and no centering flag.

### 3.5 Rust atlas/source-bounds consequence

The correct Rust mapping for Skirmish `SDBTM` is a cropped atlas source:

```text
source x = 0
source y = 0
source w = 168
source h = layout.right_panel.bottom.h
dest   x = layout.right_panel.bottom.x
dest   y = layout.right_panel.bottom.y
dest   w = 168
dest   h = layout.right_panel.bottom.h
```

In UV terms, keep the normal `uv_origin`, keep the full U width, and multiply V height by:

```text
visible_h / native_sdbtm_h
```

At 800x600 that is `23 / 65`; at 640x480 it is `29 / 65`; at 1024x768 it is again `23 / 65`.

Current Skirmish Rust does not do that: `push_entry` always uses the full `entry.uv_size` for the destination rectangle, and `SDBTM` is emitted through `push_entry`, so the full 65-row source is resampled into the shorter destination.

**Evidence:** `src/app_skirmish_shell_render.rs:85` and `:91..95` use full UVs; `:557..558` emits `right_panel_bottom_sdbtm` via `push_entry`. The main-menu shell already has the equivalent crop helper at `src/app_main_menu_shell_render.rs:166` and `:176..183`.  
**Active in YR:** Rust comparison only; gamemd behavior is active in YR as above.

## 4. INI Keys

No INI key participates in this bottom-cap source/clip behavior. Inputs are shell resource globals, screen dimensions, and SHP header dimensions.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| Offline Skirmish launcher to dialog `0x102` | verified | `FUN_006AE2C0`; prior context `0x006AE317..0x006AE328` | Yes |
| Common `WM_PAINT` to `RightPanel__Draw` | verified | `WM_PAINT_Handler @ 0x00621E90` decompile | Yes, non-WOL mode-1 path |
| `RightPanel__Draw` bottom-cap call | verified | `0x0072E65C..0x0072E69F` | Yes |
| `RightPanel__ComputeLayoutRects` bottom rect | verified | `0x0072EE88..0x0072EED2` | Yes |
| `CC_Draw_Shape` frame/native/clipping path | verified | `0x004AED70`; `0x00421B60`; `0x0069E7E0` | Yes |

## 6. Current Rust Implementation Status

Skirmish layout computes the same destination bottom rects as gamemd for the checked sizes (`800x600`, `1024x768`, `640x480`) in `src/ui/skirmish_shell/layout.rs:245`.

Skirmish rendering still samples the full atlas UV into `layout.right_panel.bottom`, which vertically compresses the `SDBTM.SHP` source. The main-menu shell already contains a matching crop pattern, but this slot did not modify any Rust.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard offline reachability | verified | `FUN_006AE2C0`, `FUN_006AE3F0`, `WM_PAINT_Handler` | none for this slice |
| `SDBTM.SHP` load/global | verified | `0x0072EBDE..0x0072EBFE`; `0x0072E68C` | none |
| `DAT_00B0FC28` formula | verified | `0x0072EE88..0x0072EED2` | live screenshot still useful but not required for formula |
| `RightPanel__Draw` bottom-cap call arguments | verified | `0x0072E65C..0x0072E69F` | none |
| `CC_Draw_Shape` clipping rule | verified | `0x004AED70`, `0x00421B60`, `0x0069E7E0` | exact inner blitter routine implementation not needed for source-window decision |
| 640/1024 visible heights | verified formula | `RightPanel__ComputeLayoutRects`; Rust expected rects | screenshot capture deferred |
| Child controls / lower strip / preview | not-touched | none | out of scope |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Does standard offline Skirmish reach this draw path? Yes, dialog `0x102` reaches common `WM_PAINT` then `RightPanel__Draw`. Evidence: `FUN_006AE2C0`, `FUN_006AE3F0`, `WM_PAINT_Handler @ 0x00621E90`.
- [RESOLVED] Q2 - What is the 800x600 bottom-cap destination rect? `(632,577,168,23)`. Evidence: `0x0072EE88..0x0072EED2`; formula with SHP dimensions.
- [RESOLVED] Q3 - Is `DAT_00B0FC28.h` passed as a source scaling height? No. The bottom-cap call reads only x/y from `DAT_00B0FC28`; optional source/mask arguments are zero. Evidence: `0x0072E65C..0x0072E69F`.
- [RESOLVED] Q4 - Which source rows are visible at 800x600? Source rows `0..22`, because bottom clipping reduces draw height from 65 to 23 without advancing source y. Evidence: `AlphaShapeClass__ClipRect @ 0x00421B60`; `CC_Draw_Shape @ 0x004AED70`.
- [RESOLVED] Q5 - How should Rust map atlas UV/source bounds? Use top-cropped source height `visible_h/native_h` and no vertical resampling. Evidence: gamemd clipping path plus Rust full-UV use at `app_skirmish_shell_render.rs:91..95`.
- [DEFERRED] Q6 - Pixel screenshot confirmation at 640x480 and 1024x768. Category: out-of-scope. Reason: this slot is static Ghidra research; formula and source clipping are resolved.

## Sources

- Ghidra decompile/assembly: `0x006AE2C0`, `0x006AE3F0`, `0x00621E90`, `0x0072E450`, `0x0072EC70`, `0x004AED70`, `0x00421B60`, `0x0069E7E0`.
- Prior docs read first: `SKIRMISH_SHELL_CHROME_800X600_TRACE.md`, `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`.
- Rust comparison references: `src/app_skirmish_shell_render.rs`, `src/app_main_menu_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`.
