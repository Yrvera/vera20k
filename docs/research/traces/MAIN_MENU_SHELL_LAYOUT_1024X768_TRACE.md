# Main Menu Shell Layout 1024x768 Trace

**Scenario:** `/trace-action main menu shell layout at 1024x768`  
**Date:** 2026-05-22  
**Scope:** dialog creation, DLU conversion, fullscreen parent resize, 1024x768 layout placement, parent/movie/button/text render composition, input/hit-test, and final screen output.  
**Rust/code policy:** read-only. No Rust fixes were implemented.

## Summary

Retail YR does **not** scale the 800x600 main-menu shell at 1024x768. It creates dialog `0xE2` from Win32 RT_DIALOG resources, expands the parent HWND to fullscreen, then places the visible shell block at `(112,84)` with native-size art and targeted child reposition helpers.

Current Rust active main-menu render and input both use `compute_responsive_layout(1024,768)`, which scales the base 800x600 layout by `1.28x`. The player sees a full-window stretched shell instead of the centered retail shell.

**PASS: 8 | FAIL: 7 | UNCHECKED: 2 | NOT IMPLEMENTED: 0**

Top player-visible failures:

1. **Whole main-menu shell is scaled instead of centered.** Retail movie is `(112,84,632,570)`; Rust active path renders it as `(0,0,809,730)`.
2. **Right panel is too large and too far right/up.** Retail SDTP is `(744,84,168,199)`; Rust active path renders `(809,0,215,255)`.
3. **Input hit-test uses scaled button rectangles.** Retail first button hit zone is the right-panel tile at `(744,283,168,42)`; Rust active path uses `(809,255,215,53)`.
4. **Hover/focus behavior still differs.** Rust highlights on mouse move; gamemd highlight is focus/timer driven through owner-draw messages.
5. **Non-active `compute_layout(1024,768)` is not ready to replace the active path without care.** Its title is centered to `(750,86)` even though the verified binary nudge for `0x694` is only a local `+7y/+1h`, and its button snap math uses unshifted DLU Y against shifted tile Y.

## Pipeline Diagram

```text
Main_Game enters main menu
  -> FUN_00531CC0 creates dialog 0xE2
  -> FUN_00622650 loads RT_DIALOG and CreateDialogIndirectParamA converts DLU to pixels
  -> FUN_00622B50 WM_INITDIALOG subclasses children and marks common shell mode
  -> FUN_0060C4A0 MoveWindow(parent, 0,0,1024,768) + EnumChildWindows
  -> ResizeShellChildControl_0060C0C0 applies targeted child moves/nudges
  -> RightPanel__ComputeLayoutRects computes centered 800x600 shell rects
  -> WM_PAINT_Handler draws parent background + right panel + lower strip
  -> MainMenuDialog0xE2_Proc sends 0x4F0 to child 0x71A for RA2TS movie draw
  -> OwnerDraw_Button_00612B70 paints six SDBTNANM owner-draw buttons/text
  -> Win32 button release emits WM_COMMAND return code 1..6
```

## Stage Trace

### Stage 1 - Dialog Creation

Input: main-menu loop enters the initial YR shell menu.

gamemd:

- `FUN_00531CC0` creates dialog `0xE2` and installs `MainMenuDialog0xE2_Proc_00531F60`.
- `FUN_00622650` wraps `CreateDialogIndirectParamA`; the Win32 dialog manager handles template layout.

Rust:

- Main-menu shell render starts from app state and hand-authored layout structs; no Win32 dialog resource is parsed at runtime.
- This internal difference is acceptable only if the observable rects and behavior match.

Verdict: **PASS** for intended Rust modeling boundary; **FAIL** later where modeled rects diverge.

Evidence:

- Existing doc: `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`.
- Spot-check: `FUN_00622650 @ 0x00622650` decompiles to `CreateDialogIndirectParamA(...)`.

### Stage 2 - DLU Conversion

Input: RT_DIALOG `0xE2`, `533x369` DLU, font `MS Sans Serif 8pt`.

gamemd:

```text
baseX = 6, baseY = 13
width  = MulDiv(533, 6, 4)  = 800
height = MulDiv(369, 13, 8) = 600
```

Key raw child rects before post-creation moves:

```text
0x694 title:    DLU (425, 1,   108, 10) -> px (638, 2,   162, 16)
0x695 tooltip:  DLU (2,   355, 303, 12) -> px (3,   577, 455, 20)
0x71D version:  DLU (425, 357, 108, 10) -> px (638, 580, 162, 16)
buttons:        DLU x=425,w=108,h=23 -> x=638,w=162,h=37
```

Rust:

- `dlu_rect` in `src/ui/main_menu_shell/layout.rs` uses `BASE_X=6`, `BASE_Y=13`.

Verdict: **PASS** for DLU arithmetic.

Evidence:

- Existing doc: `DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`.
- Rust: `src/ui/main_menu_shell/layout.rs:7`, `src/ui/main_menu_shell/layout.rs:93`.

### Stage 3 - Parent Fullscreen Resize

Input: dialog parent originally created from an 800x600 template.

gamemd:

```c
MoveWindow(hwnd, 0, 0, g_ScreenWidth, g_ScreenHeight, 0);
DAT_00ac48a8 = hwnd;
EnumChildWindows(hwnd, ResizeShellChildControl_0060C0C0, param_2);
```

At 1024x768, the parent HWND becomes `(0,0,1024,768)`.

Rust:

- Uses the swapchain size as the render target. This matches the fullscreen parent extent.

Verdict: **PASS** for parent/screen extent.

Evidence:

- Spot-check: `FUN_0060C4A0 @ 0x0060C4A0`.

### Stage 4 - 1024x768 Right-Panel Layout

Input: `screen_w=1024`, `screen_h=768`, SDTP `168x199`, SDBTNBKGD `168x42`, SDBTM native width `168`.

gamemd formula from `RightPanel__ComputeLayoutRects @ 0x0072EC70`:

```text
left_margin = (1024 - 800) / 2 = 112    because 1024 >= 1024
right_edge  = 1024 - 112 = 912
top_margin  = (768 - 600) / 2 = 84      because 768 >= 768
effective_h = 768 - 84*2 = 600

SDTP       = (912 - 168, 84, 168, 199) = (744,84,168,199)
Tile[0]    = (744, 84 + 199, 168, 42) = (744,283,168,42)
tile_count = (600 - 199) / 42 = 9
SDBTM      = (744, 283 + 9*42, 168, 600 - 199 - 9*42)
           = (744,661,168,23)
```

SDBTNANM frame-10 background/button-art rect is right-anchored inside the tile:

```text
button_art_x = 744 + (168 - 156) = 756
button_art_y = 283
button_art_w/h = 156x42
```

Rust active path:

```text
compute_responsive_layout(1024,768)
scale_x = scale_y = 1.28

right_panel.top    = scale((632,0,168,199))   = (809,0,215,255)
right_panel.tile   = scale((632,199,168,42)) = (809,255,215,53)
right_panel.bottom = scale((632,577,168,23)) = (809,739,215,29)
```

Verdict: **FAIL**.

Player-visible result: the right panel fills the full height and right edge of the window instead of appearing as a native-size panel centered in the 800x600 shell block. This is visible every frame at 1024x768.

Evidence:

- Spot-check: `RightPanel__ComputeLayoutRects @ 0x0072EC70`.
- Rust active render: `src/app_main_menu_shell_render.rs:372`.
- Rust responsive scaler: `src/ui/main_menu_shell/layout.rs:356`.

### Stage 5 - RA2TS Movie Placement

Input: `screen_w=1024`, `screen_h=768`.

gamemd:

```text
asset = Ra2ts_l because screen_w != 640
x = (1024 - 800) / 2 = 112
y = (768 - 600) / 2 = 84
w/h = native BIK size = 632x570
final rect = (112,84,632,570)
```

Rust active path:

```text
base movie = (0,0,632,570)
scaled by 1.28 -> (0,0,809,730)
asset remains Ra2ts_l
```

Verdict: **FAIL**.

Player-visible result: the intro/menu movie is stretched and starts at the top-left corner instead of being native-size and centered inside the shell block.

Evidence:

- Spot-check: `FUN_00531CC0 @ 0x00531CC0` chooses `Ra2ts_l` unless width is exactly `640`, then calls `SetWindowPos` with centered coordinates.
- Rust active render uses `compute_responsive_layout` in `ensure_movie_for_current_layout` and `render_main_menu_shell`: `src/app_main_menu_shell_render.rs:301`, `src/app_main_menu_shell_render.rs:372`.

### Stage 6 - Lower Strip Placement

gamemd:

```text
asset = LWSCRNL because screen_w != 640
x = left_margin = 112
y = local_4 - strip_h = (768 - 84) - 32 = 652
w/h from SHP header, expected large strip 632x32
final rect = (112,652,632,32)
```

Rust active path:

```text
base lower_strip = (0,568,632,32)
scaled by 1.28 -> (0,727,809,41)
asset selected from layout.screen.w <= 640 ? small : large -> large at 1024
```

Verdict: **FAIL**.

Player-visible result: the lower strip is stretched to 809x41 and sits at the bottom of the full window instead of the centered shell bottom.

Evidence:

- Spot-check: `RightPanel__ComputeLayoutRects @ 0x0072EC70`, `RightPanel__Draw @ 0x0072E450`.
- Rust active path: `src/app_main_menu_shell_render.rs:213`, `src/ui/main_menu_shell/layout.rs:370`.

### Stage 7 - Static Text Controls

gamemd:

- Tooltip `0x695`: `FUN_0060B550` anchors to bottom-left of the centered 800x600 shell.

```text
x = 112 + 10 = 122
y = 768 - 20 - 84 - 1 = 663
rect = (122,663,455,20)
```

- Version `0x71D`: `FUN_0060B610` anchors to the right-panel bottom cap.

```text
inset = (168 - 162) / 2 = 3
x = 1024 - 3 - 162 - 112 = 747
y = 661 + 23 - 16 = 668
rect = (747,668,162,16)
```

- Heading `0x694`: `FUN_0060B950` applies the common `0xE2` heading nudge: local `y += 7`, `h += 1`. No Ghidra evidence was found in this trace for a general `(112,84)` centering offset being applied to this child.

Rust:

- Active responsive path scales all static text from the 800x600 base.
- Non-active `compute_layout(1024,768)` offsets title and website static by `(112,84)` at `src/ui/main_menu_shell/layout.rs:289`.
- `compute_layout(1024,768)` does match the verified tooltip/version formulas.

Verdict: **FAIL** for active path; **UNCHECKED/LIKELY FAIL** for `compute_layout` title/website placement until a runtime capture or resource-position watch confirms title final rect at 1024x768.

Player-visible result: active Rust places title/version/tooltip as scaled full-window UI. If switched directly to `compute_layout`, title may jump into the right panel instead of keeping the verified local nudge behavior.

Evidence:

- Spot-check: `FUN_0060B550 @ 0x0060B550`, `FUN_0060B610 @ 0x0060B610`, `FUN_0060B950 @ 0x0060B950`.
- Rust active path: `src/app_main_menu_shell_render.rs:372`.
- Rust title offset: `src/ui/main_menu_shell/layout.rs:289`.

### Stage 8 - Button Rects and Hit-Test

gamemd:

Six command buttons exist in `MainMenuDialog0xE2_Proc_00531F60`:

```text
0x683 -> return 1
0x684 -> return 2
0x578 -> return 3
0x686 -> return 4
0x55C -> return 5
0x3EE -> return 6
```

At 1024x768, the retail right-panel tile rows are:

```text
Single Player:       tile 0 -> (744,283,168,42); SDBTNANM art at (756,283,156,42)
WW Online:           tile 1 -> (744,325,168,42); SDBTNANM art at (756,325,156,42)
Network:             tile 2 -> (744,367,168,42); SDBTNANM art at (756,367,156,42)
Movies and Credits:  tile 3 -> (744,409,168,42); SDBTNANM art at (756,409,156,42)
Options:             tile 4 -> (744,451,168,42); SDBTNANM art at (756,451,156,42)
Exit Game:           tile 8 -> (744,619,168,42); SDBTNANM art at (756,619,156,42)
```

Rust active path:

```text
Single Player:       (809,255,215,53)
WW Online:           (809,308,215,54)
Network:             (809,362,215,54)
Movies and Credits:  (809,416,215,53)
Options:             (809,470,215,54)
Exit Game:           (809,685,215,54)
```

Rust active hit-test uses those scaled rects through `hit_test_owner_draw_button`.

Verdict: **FAIL**.

Player-visible result: clicks in the retail button positions, such as `(760,300)`, do not hit the Rust active first button at 1024x768. The button art and text are also too large.

Evidence:

- Spot-check: `RightPanel__ComputeLayoutRects @ 0x0072EC70`, `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`, `OwnerDraw_Button_00612B70 @ 0x00612B70`.
- Rust active input: `src/app.rs:603`, `src/app.rs:625`, `src/app.rs:813`.
- Rust hit-test: `src/ui/main_menu_shell/state.rs:95`.

### Stage 9 - Non-active `compute_layout(1024,768)` Button Snap Risk

Rust `compute_layout(1024,768)` currently shifts the right-panel tile Y to `283` but computes button tile indices from unshifted raw DLU Y values:

```text
raw DLU y px: 125->203, 152->247, 179->291, 206->335, 233->379, 330->536
tile_y at 1024x768 = 283
tile_index = (raw_y - tile_y + 21) / 42, clamped to >=0
```

With Rust integer division, this produces:

```text
0x683: 0 -> y 283
0x684: 0 -> y 283
0x578: 0 -> y 283
0x686: 1 -> y 325
0x55C: 2 -> y 367
0x3EE: 6 -> y 535
```

That is not the expected retail 1024x768 tile sequence listed in Stage 8.

Verdict: **FAIL** if `compute_layout` is substituted as the active path without fixing the snap math. This failure is currently masked because the active path uses `compute_responsive_layout`.

Evidence:

- Rust: `src/ui/main_menu_shell/layout.rs:267`, `src/ui/main_menu_shell/layout.rs:436`.
- Existing tests only assert the first 1024x768 button rect, not the full six-button sequence.

### Stage 10 - Button Paint Composition

gamemd:

- Shared right-panel draw paints SDTP, SDBTNBKGD tiles, optional repeated SDBTNANM frame 10, SDBTM, and LWSCRNL/LWSCRNS.
- Owner-draw button paint uses SDBTNANM frames `2` default, `3` highlight, `4` pressed for this active path.
- SDBTNANM art is right-anchored inside the `168x42` tile: `x = tile_x + 12`.

Rust:

- Current `push_button_shp` right-anchors the frame and scales it according to the active layout tile. At 1024 active layout that becomes a scaled `199.7x53.8` frame, not native `156x42`.
- Current `push_clipped_top` clips SDBTM top rows rather than squashing the full SHP; that specific old failure appears fixed in the current worktree.

Verdict: **PASS** for frame selection and right-anchor formula in the current render code; **FAIL** because the active destination rect is scaled.

Evidence:

- Spot-check: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `RightPanel__ComputeLayoutRects @ 0x0072EC70`.
- Rust: `src/app_main_menu_shell_render.rs:62`, `src/app_main_menu_shell_render.rs:166`.

### Stage 11 - Hover, Focus, and Tooltip Input

gamemd:

- `WM_NCHITTEST` path in `FUN_00622B50` updates tooltip control `0x695` on mouse movement.
- SDBTNANM frame-3 highlight comes from owner-draw focus/timer state, not simple cursor-over state. `OwnerDraw_Button_00612B70` handles `WM_SETFOCUS` and custom `0x4DC`; no direct `WM_MOUSEMOVE` highlight branch is present.
- Click action is emitted by normal Win32 button release/`WM_COMMAND`.

Rust:

- `mouse_move` sets `hovered_owner_draw_button` directly from cursor position.
- Renderer uses `hovered_owner_draw_button` to drive frame-3 highlight timing.
- `mouse_up` requires press and release over the same button, matching the click/action boundary.

Verdict: **FAIL** for highlight behavior; **PASS** for release-to-action semantics.

Player-visible result: Rust can animate/highlight a button merely by moving over it; retail focus/highlight behavior is not that direct.

Evidence:

- Spot-check: `FUN_00622B50 @ 0x00622B50`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`.
- Rust: `src/ui/main_menu_shell/state.rs:116`, `src/app_main_menu_shell_render.rs:131`, `src/ui/main_menu_shell/state.rs:124`.

### Stage 12 - Render Order and Screen Output

gamemd visible shell at 1024x768:

```text
Centered shell block: x 112..912, y 84..684
RA2TS_l movie:        (112,84,632,570)
Right panel:          (744,84,168,600) made from top/tile/bottom pieces
Lower strip:          (112,652,632,32)
Buttons:              six native SDBTNANM 156x42 frames at x=756
Outer margins:        filled by the common shell paint path
```

Rust active screen output at 1024x768:

```text
Movie:       (0,0,809,730)
Right panel: (809,0,215,768)
Lower strip: (0,727,809,41)
Buttons:     scaled 215x53/54 hit rects, scaled SDBTNANM art
```

Render order caveat:

- Ghidra path indicates parent shell paint runs before the explicit `0x71A` movie draw request. Current Rust draws movie first, then chrome/buttons/text. Because retail lower strip `(112,652,632,32)` overlaps the bottom two rows of a `632x570` movie at `y=652..653`, exact z-order for that 2-pixel overlap remains **UNCHECKED** without a retail capture.

Verdict: **FAIL** for overall active screen output; **UNCHECKED** for exact movie-vs-lower-strip overlap pixels.

## Rust Surface Comparison

| Surface | Current Rust behavior | Verdict |
|---|---|---|
| `src/ui/main_menu_shell/layout.rs::compute_layout` | Correct DLU base units and right-panel top rect at 1024, but title offset and full six-button snap sequence need re-check before active use | **PARTIAL** |
| `src/ui/main_menu_shell/layout.rs::compute_responsive_layout` | Scales 800x600 base shell by full swapchain ratio | **FAIL** for parity |
| `src/app_main_menu_shell_render.rs::ensure_movie_for_current_layout` | Uses responsive layout to choose/render movie rect | **FAIL** at 1024 |
| `src/app_main_menu_shell_render.rs::render_main_menu_shell` | Uses responsive layout for movie/chrome/buttons/text | **FAIL** at 1024 |
| `src/app.rs` main-menu mouse handlers | Use responsive layout for down/up/move hit-tests | **FAIL** at 1024 |
| `src/ui/main_menu_shell/state.rs::mouse_up` | Press+release same button action semantics | **PASS** |
| `src/ui/main_menu_shell/state.rs::mouse_move` | Cursor-over hover drives highlight | **FAIL** |

## Verification Performed

- Read existing research docs first:
  - `DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`
  - `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
  - `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md`
  - `traces/MAIN_MENU_RIGHT_PANEL_CHROME_STACK_TRACE.md`
  - `traces/MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE.md`
  - `traces/MAIN_MENU_OWNER_DRAW_BUTTON_SHP_FRAMES_TRACE.md`
  - `traces/MAIN_MENU_STATIC_LABELS_AND_LOWER_STRIP_TRACE.md`
- Targeted Ghidra spot-checks:
  - `FUN_00531CC0 @ 0x00531CC0`
  - `FUN_00622650 @ 0x00622650`
  - `FUN_00622B50 @ 0x00622B50`
  - `FUN_0060C4A0 @ 0x0060C4A0`
  - `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`
  - `FUN_0060B550 @ 0x0060B550`
  - `FUN_0060B610 @ 0x0060B610`
  - `FUN_0060B950 @ 0x0060B950`
  - `WM_PAINT_Handler @ 0x00621E90`
  - `RightPanel__Draw @ 0x0072E450`
  - `RightPanel__ComputeLayoutRects @ 0x0072EC70`
  - `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`
  - `OwnerDraw_Button_00612B70 @ 0x00612B70`
- Rust read-only comparison:
  - `src/ui/main_menu_shell/layout.rs`
  - `src/app_main_menu_shell_render.rs`
  - `src/app.rs`
  - `src/ui/main_menu_shell/state.rs`
- Focused tests:
  - `cargo test -q main_menu_shell`
  - Result: passed, `12 passed`; warnings only. Existing tests do not cover the full 1024x768 active path mismatch or all six non-active `compute_layout(1024,768)` button rects.

## Implementation Handoff

Do not implement from this trace without preserving these constraints:

1. Active main-menu render and input at 1024x768 must stop using `compute_responsive_layout` for parity mode.
2. Do not switch blindly to current `compute_layout(1024,768)` until the full six-button rect sequence and title/website static placement are corrected or separately verified.
3. Keep native asset sizes: movie `632x570`, right panel `168` wide, SDBTNANM button art `156x42`.
4. Keep button actions on release-over-same-control.
5. Re-check movie/lower-strip overlap against a retail capture before asserting exact final pixels.

Status: **COMPLETE / FAILING TRACE**
