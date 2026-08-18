# Skirmish Shell 640x480 Final Visible Layout Trace

Date: 2026-05-22
Mode: `/trace-action skirmish shell 640x480 final visible layout`
Scope: standard offline Yuri's Revenge Skirmish dialog `0x102` at exactly `640x480`, from dialog creation and DLU conversion through fullscreen shell hosting, child layout, parent/chrome/background draw, preview/button/control placement, and current Rust comparison.

Task constraints honored: Rust/source read-only. This pass wrote only this report.

## Summary

Overall verdict: `FAIL` for current Rust player-visible parity, with several `PASS` layout formulas and one major `UNCHECKED` live-visual gap.

The binary layout at `640x480` is now implementation-safe at formula level:

- parent shell HWND: `(0,0,640,480)`;
- parent background: `MNSCRNS.SHP` at `(0,0,472,448)`;
- lower strip: `LWSCRNS.SHP` at `(0,448,472,32)`;
- right panel: top `(472,0,168,199)`, six tile rows beginning `(472,199,168,42)`, bottom `(472,451,168,29)`;
- Start `0x617`: `(484,241,156,42)`;
- Choose Map `0x5AA`: `(484,283,156,42)`;
- Back `0x5C0`: `(484,409,156,42)`;
- map preview placeholder `0x468`: `(484,37,144,112)`.

No live retail 640x480 capture was made in this pass. All exact final-screen claims that depend on rendered pixels rather than binary formulas remain `UNCHECKED`.

## Pipeline

```text
Main_Game
  -> FUN_006AE2C0 offline Skirmish launcher
  -> FUN_0072CF40 preload Skirmish background/palette
  -> FUN_00622650 CreateDialogIndirectParamA(dialog 0x102)
  -> Win32 DLU conversion using MS Sans Serif 8pt, baseX=6/baseY=13
  -> FUN_00622B50 WM_INITDIALOG common shell setup
  -> FUN_0060C4A0 MoveWindow(parent,0,0,640,480) + EnumChildWindows
  -> ResizeShellChildControl_0060C0C0 selective child anchoring/fixups
  -> WM_PAINT_Handler: right-panel chrome, background overlay, optional extras, blit
  -> FUN_006AE3F0 Skirmish-specific preview/start-position paint
  -> child owner-draw callbacks for buttons, combos, checkboxes, trackbars
  -> final 640x480 screen
```

## Stage Trace

### Stage 1 - Dialog creation and DLU conversion

Input: offline Skirmish creates dialog resource `0x102`, `533x369` DLU, `MS Sans Serif 8pt`.

Formula: Win32 `CreateDialogIndirectParamA` converts positive DLUs as `MulDiv(x,6,4)` and `MulDiv(y,13,8)`.

Concrete outputs:

| Resource DLU | Pixel result |
|---|---:|
| dialog `(0,0,533,369)` | `(0,0,800,600)` |
| Start `(425,149,108,23)` | `(638,242,162,37)` before shell snap |
| Choose `(425,176,108,23)` | `(638,286,162,37)` before shell snap |
| Back `(425,346,108,23)` | `(638,562,162,37)` before shell snap |
| Preview `(429,23,96,69)` | `(644,37,144,112)` before right anchor |

gamemd: verified by the DLU report and active dialog resource report.

Rust: `src/ui/skirmish_shell/layout.rs` implements the same `BASE_X=6`, `BASE_Y=13`, and rounding helper.

Verdict: `PASS`.

### Stage 2 - Fullscreen parent hosting

Input: newly created `0x102` child dialog, current video mode `640x480`.

gamemd formula: `FUN_0060C4A0` calls `MoveWindow(parent, 0, 0, g_ScreenWidth, g_ScreenHeight, 0)` then enumerates children through `ResizeShellChildControl_0060C0C0`.

Output: parent shell client area is `(0,0,640,480)`. Children are not globally scaled.

Rust: active experimental shell layout uses `compute_layout(state.render_width(), state.render_height())`; the layout model uses raw screen dimensions and does not globally scale children.

Verdict: `PASS` for represented layout math. Default product path remains dev-gated, so full Skirmish shell parity is not active by default.

### Stage 3 - Right-panel layout globals

Input: `RightPanel__ComputeLayoutRects(640,480)`, assets `SDTP=168x199`, `SDBTNBKGD=168x42`, `SDBTM=168x65`.

Formula:

```text
left_margin = 0                 ; 640 <= 1023
top_margin = 0                  ; 480 <= 767
effective_right = 640
tile_count = (480 - 199) / 42 = 6
```

Output:

| Global / role | Rect |
|---|---:|
| `DAT_00B0FC20` / top `SDTP` | `(472,0,168,199)` |
| `DAT_00B0FC24` / tile `SDBTNBKGD` | `(472,199,168,42)` |
| tile count | `6` |
| `DAT_00B0FC28` / bottom `SDBTM` clipped remainder | `(472,451,168,29)` |

Rust: `right_panel_rects(640,480)` matches these values and has a focused test.

Verdict: `PASS`.

### Stage 4 - Parent background and lower strip

Input: `0x102`, width `640`, height `480`.

gamemd:

- `FUN_0072CF40` loads the Skirmish palette and only loads `MnScrnLCoopGameSetup.shp` at exact width `800`.
- parent record `+0xE0` receives `DAT_00B0FB50`, mapped to `MNSCRNS.SHP`;
- `Background_Overlay` selects `+0xE0` only when `g_ScreenWidth == 640`;
- `RightPanel__ComputeLayoutRects` writes background rect `(0,0,472,448)` for `MNSCRNS.SHP`;
- lower side rect uses `LWSCRNS.SHP` at `(0,448,472,32)`.

Rust:

- `parent_background_role()` selects `Mnscrns640` for width `640`;
- `lower_strip_role()` selects `Lwscrns640`;
- atlas loading maps `MNSCRNS.SHP` and `LWSCRNS.SHP`.

Verdict: `PASS` for represented asset selection and rects.

Live visual caveat: no retail screenshot was captured, so final palette/output pixels are `UNCHECKED`.

### Stage 5 - Start, Choose Map, and Back buttons

Input: owner-draw button controls `0x617`, `0x5AA`, `0x5C0`.

gamemd:

- `ResizeShellChildControl_0060C0C0` tests owner-draw button metadata before generic right-anchor.
- Start and Choose route through `FUN_0060B000`, not `FUN_0060B1D0`.
- Back routes through `FUN_0060B350`.
- `SDBTNANM.SHP` dimensions are `156x42`.

Concrete `640x480` results:

| Control | Original pixel rect | Final rect | Verdict |
|---|---:|---:|---|
| Start `0x617` | `(638,242,162,37)` | `(484,241,156,42)` | `PASS` |
| Choose `0x5AA` | `(638,286,162,37)` | `(484,283,156,42)` | `PASS` |
| Back `0x5C0` | `(638,562,162,37)` | `(484,409,156,42)` | `PASS` |

Rust: `owner_draw_button_snap()` and `back_rect()` match these rects; `cargo test -q skirmish_shell` includes `key_rects_match_640x480_formula`.

Verdict: `PASS`.

Stale-doc note: older `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md` listed Start/Choose as generic right-anchored `(475,242,162,37)` / `(475,286,162,37)`. That is superseded by the complete `ResizeShellChildControl_0060C0C0` policy and the Ghidra spot-check of `FUN_0060B000`.

### Stage 6 - Map preview and right-panel text statics

Input: preview static `0x468`, title `0x694`, game-type text `0x6EC`, map label `0x5A8`.

gamemd:

- `0x468`, `0x694`, `0x6EC`, and `0x5A8` route through generic right-anchor `FUN_0060B1D0`.
- `FUN_0060B950` adds `y+1` to title `0x694`.
- map preview remains a child placeholder; preview image and start markers are drawn later by Skirmish paint code.

Concrete formula results:

| Control | Final rect |
|---|---:|
| title `0x694` | `(475,3,162,16)` |
| map preview `0x468` | `(484,37,144,112)` |
| game type `0x6EC` | `(489,167,135,16)` |
| map/scenario label `0x5A8` | `(489,189,135,33)` |

Rust:

- `layout.map_preview` matches `(484,37,144,112)`.
- the current experimental renderer decodes/draws a preview texture when available, but start-marker overlays are called with empty positions and `real_preview_surface_available=false`.
- title/game-type/map-label statics are not represented as rendered shell text surfaces in the current renderer.

Verdict: `PASS` for map preview rect; `FAIL` for current Rust visible title/game-type/map-label surfaces; `UNCHECKED` for live 640x480 preview pixels and overlay marker visibility.

### Stage 7 - Slot table, flags, combos, checkboxes, and trackbars

Input: ordinary controls from dialog `0x102`.

gamemd:

- ordinary row controls, color combos, flags, checkboxes, and trackbars are not right-anchored and are not globally scaled;
- fallback preserves DLU-derived coordinates, then `FUN_0060B950` applies selected one-pixel fixups;
- checkboxes use `OwnerDraw_Checkbox_006163A0`;
- trackbars use `OwnerDraw_Trackbar_0061D950`.

Representative `640x480` final rects:

| Surface | Final rect / behavior |
|---|---:|
| player name `0x6A0` | `(58,59,151,23)` after `x+1,w+1` |
| player flag `0x6DA` | `(225,59,48,20)` |
| player color combo `0x6A2` | `(423,59,44,119)` |
| Short Game checkbox `0x54E` | `(71,286,150,16)` after `x-1`; icon blit at `(71,286,18,18)` |
| MCV Repacks checkbox `0x693` | `(71,314,150,16)` after `x-1` |
| Crates checkbox `0x696` | `(71,341,150,16)` after `x-1` |
| Super Weapons checkbox `0x69A` | `(71,371,155,16)` after `x-1` |
| Build Off Ally checkbox `0x69D` | `(302,369,249,18)` |
| game speed trackbar `0x529` | `(404,286,128,21)` |
| credits trackbar `0x511` | `(404,314,128,21)` |
| unit count trackbar `0x50C` | `(404,340,128,21)` after `y-1` |

Rust:

- layout currently represents player name, flags, color combos, and trackbars;
- renderer draws some flags and buttons, but does not render the full slot table, combo chrome, checkbox owner-draw PCXs, trackbar rail/thumb/value plaques, or most static labels;
- hit testing covers owner-draw buttons and color combos, not checkboxes, trackbars, side/start/team combos, or text inputs.

Verdict: `FAIL` for current Rust final visible layout. These are normal Skirmish controls and the missing visuals/interactions are player-visible on every Skirmish setup visit.

### Stage 8 - Parent/chrome/background draw order

gamemd order for parent `WM_PAINT`:

```text
RightPanel__Draw
Background_Overlay
optional generic extras
blit parent BSurface
Skirmish-specific preview/start-position paint after common handler returns
child owner-draw callbacks at child paint boundaries
```

Rust current semantic order:

```text
parent background
right panel top/tile/bottom
lower strip
buttons
preview texture pass
button text
```

At `640x480`, the background/lower strip and right panel mostly abut at `x=472`, so this ordering mismatch may not show if all assets are opaque and exactly clipped. It is still a mismatch in the current render model and should not be treated as pixel-verified parity without a screenshot.

Verdict: `UNCHECKED` for visible 640x480 impact; `FAIL` for strict draw-order parity in the current Rust model.

## Current Rust Comparison

High-confidence matches:

- `src/ui/skirmish_shell/layout.rs` computes the corrected 640x480 right-panel and key button rects.
- `src/render/skirmish_shell_chrome.rs` loads the verified 640 background/lower strip and right-panel assets.
- focused tests pass: `cargo test -q skirmish_shell` -> `38 passed`, `2 ignored`.

Player-visible failures:

1. The current Rust screen does not render the complete Skirmish dialog surface: player/AI row controls, headers, most right-panel labels, checkboxes, trackbars, and combo chrome are absent or only partially represented.
2. Checkbox and trackbar owner-draw visuals are not implemented. Players will not see the retail `cue_i/cce_i` checkbox icons, `trakgrip/trof*` slider visuals, or value plaques.
3. Start-position overlays are disabled in the renderer path. Stock maps may have baked preview markers, but `STARTBUT.SHP` overlay visibility at 640x480 remains `UNCHECKED` without live retail capture.
4. The experimental shell is dev-gated, so the verified Rust shell renderer is not the normal Skirmish setup path by default.

## PASS / FAIL / UNCHECKED Summary

| Area | Verdict | Notes |
|---|---|---|
| `0x102` DLU conversion | `PASS` | `533x369` DLU -> `800x600`, base `6x13` |
| fullscreen parent hosting | `PASS` | parent moves to `(0,0,640,480)` |
| no global child scaling | `PASS` | selective anchor/fallback policy verified |
| right-panel rects | `PASS` | `(472,0,168,199)`, 6 tiles, bottom `(472,451,168,29)` |
| parent background/lower strip formula | `PASS` | `MNSCRNS` `(0,0,472,448)`, `LWSCRNS` `(0,448,472,32)` |
| Start/Choose/Back rects | `PASS` | Rust layout matches corrected owner-draw snap formulas |
| map preview rect | `PASS` | `(484,37,144,112)` |
| full Rust visible shell | `FAIL` | many player-visible controls not rendered/interactable |
| checkbox/trackbar Rust parity | `FAIL` | layout partial, owner-draw paint/input missing |
| parent/chrome draw-order parity | `FAIL` model, `UNCHECKED` visible impact | order differs; 640 overlap impact not screenshot-checked |
| live 640x480 retail screenshot | `UNCHECKED` | not captured in this pass |
| exact final rendered pixels/palette | `UNCHECKED` | formula-level evidence only |

## Top Player-Visible Failures

1. Missing lower-options controls: retail shows checkboxes and trackbars in the lower half of the screen; current Rust does not render or operate them in the experimental shell.
2. Incomplete slot table: retail shows the full player/AI table with headers, player name, country/side/color/start/team controls, and row state; current Rust renders only a small subset.
3. Incomplete right-panel text/preview behavior: key labels and optional `STARTBUT.SHP` overlays are not fully represented; live 640x480 overlay behavior still needs retail capture.
4. Dev-gated route: the experimental shell is not the default Skirmish setup screen, so even passing layout formulas do not yet produce the normal player-facing flow.

## Evidence Ledger

Primary docs:

- `C:/Users/enok/Documents/ra2-rust-game-docs/DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`

Ghidra spot-checks in this pass:

- `FUN_0060B000 @ 0x0060B000`: owner-draw Start/Choose snap helper.
- `FUN_0060B350 @ 0x0060B350`: Back button tile anchor.
- `FUN_0060C4A0 @ 0x0060C4A0`: fullscreen parent `MoveWindow` plus child enumeration.
- `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0`: branch ordering, owner-draw button preemption, fallback.
- `RightPanel__ComputeLayoutRects @ 0x0072EC70`: 640 thresholds, right-panel/background/lower-strip rects.
- `FUN_0060B950 @ 0x0060B950`: `0x102` one-pixel fixups.

Rust surfaces read-only:

- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`

Verification:

- `cargo test -q skirmish_shell` passed: `38 passed`, `2 ignored`.

## Follow-up Queue

1. Capture or live-debug retail YR at exactly `640x480` on dialog `0x102` to verify final pixels, palette, and whether `STARTBUT.SHP` overlays appear for stock maps.
2. Implement/render the missing checkbox and trackbar owner-draw surfaces from the existing verified geometry report.
3. Expand Rust shell rendering to cover the full slot table, right-panel labels, and combo chrome before treating the 640x480 shell as player-visible parity.
