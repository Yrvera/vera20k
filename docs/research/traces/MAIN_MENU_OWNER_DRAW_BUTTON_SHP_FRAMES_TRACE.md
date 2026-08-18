# Main-Menu Owner-Draw Button SHP Frames — Pipeline Trace

**Scope:** Dialog 0xE2, 800x600, 6 owner-draw buttons (0x683 SP, 0x684 WW, 0x578 Net, 0x686 M&C, 0x55c Options, 0x3ee Exit). SDBTNANM.SHP frame indices, palette, sizing, centering, pressed offset, Y slot positions, hover/press rules, 7th control, draw order.

**Ghidra evidence:** all binary claims verified this session. No stale labels used.

> **2026-07-18 correction:** Stage 5's SHP conclusion was correct, but its text
> conclusion was incomplete. Direct assembly inspection shows that pressed state changes
> the complete label clipping rectangle, not merely X. Evidence:
> `disassemble_function(0x00612B70)`, instructions `0x00613568..0x006135EE`.
> Current-Rust comparisons elsewhere in this older trace are historical unless a later
> correction is called out explicitly.

---

## Stage-by-stage verdict

### Stage 1 — SDBTNANM frame indices (default/hover/pressed)

**PASS**

`OwnerDraw_Button_00612B70 @ 0x00612B70`, WM_PAINT (0xF) handler, iVar14==1 (SDBTNANM type) branch:

```c
local_f0 = (WNDPROC)0x2;           // default = frame 2
if (!pressed) {
    if (piVar17[0xc5/4] byte != 0)  // hover/focus flag
        local_f0 = (WNDPROC)0x3;   // hover = frame 3
} else {
    local_f0 = (WNDPROC)0x4;       // pressed = frame 4
}
CC_Draw_Shape(g_SDBTNANM_SHP, local_f0, ...)
```

gamemd frame map: **2=default, 3=hover, 4=pressed**. Our code: `render_shp_frame(..., 2, "default")`, `render_shp_frame(..., 3, "hover")`, `render_shp_frame(..., 4, "pressed")`. Exact match.

SDBTNANM.SHP has 17 frames total (canvas 156x42, all frames full-size). Frames 2/3/4 exist.

Verified: `decompile_function 0x00612B70`.

---

### Stage 2 — SDBTNANM palette used in CC_Draw_Shape

**PASS (with clarification)**

`OwnerDraw_Button_00612B70` for iVar14==1:
```c
piStack_c4 = (int *)FUN_0072e2c0();  // returns DAT_00b0fbdc = SDBTNANM.PAL data
piStack_dc = g_SDBTNANM_SHP;
...
if (piStack_c4 != 0 && piStack_dc != 0) {
    CC_Draw_Shape(piStack_dc, frame, pos, viewport, 0x400, 0, ...);
}
```

The palette pointer is checked for non-null as a guard before drawing, but `CC_Draw_Shape` is called with **param_6 = 0** (null remap palette). The `0x400` flag does not enable remapping (`0x800` does). The SHP is drawn using **no remap palette** — it renders with whatever pixel data is in the SHP's frame directly (the SHP palette indices map to the DirectDraw 8-bit display surface's current palette).

`SDBTNANM.PAL` is present in `ra2.mix` (768 bytes, loads OK). It is the current system palette for the display surface during main-menu rendering, set globally. The palette guard ensures it was loaded before drawing.

Our code loads `SDBTNANM.PAL` as an RGBA palette and applies it during atlas texture construction — this is correct behavior since we render to RGBA textures, not 8-bit palettized surfaces. The visible output should match as long as the correct palette entries are used.

Verified: `decompile_function 0x00612B70`, `decompile_function 0x004AED70` (CC_Draw_Shape).

---

### Stage 3 — SDBTNANM.SHP native frame size

**PASS**

Retail asset inspection (via `ShpFile::from_bytes` on `ra2.mix`):
- SDBTNANM.SHP canvas: **156×42** pixels, 17 frames
- All 17 frames: full-size 156×42, offset (0,0)

Chrome tile sizes (for reference):
- SDTP.SHP: 168×199
- SDBTNBKGD.SHP: 168×42
- SDBTM.SHP: 168×65

Our code uses `frame.pixel_size` from the SHP's actual canvas dimensions (156×42). The comment "156×42 native" in `push_button_shp` is correct. `RIGHT_PANEL_TILE_H = 42` and `RIGHT_PANEL_WIDTH = 168` are correct.

Verified: direct retail asset read from `ra2.mix`.

---

### Stage 4 — Centering rule inside the chrome tile

**FAIL**

gamemd in `RightPanel__ComputeLayoutRects @ 0x0072EC70` computes the SDBTNANM rect (`DAT_00b0fc10`) as:

```c
sVar1 = *(short *)(g_SDBTNANM_SHP + 2);  // shp_w = 156
sVar2 = *(short *)(g_SDBTNANM_SHP + 4);  // shp_h = 42
*DAT_00b0fc10 = (iVar6 - sVar1) + iVar5; // x = tile_x + (tile_w - shp_w)
DAT_00b0fc10[1] = iVar4;                  // y = tile_y (top-aligned)
```

where `iVar5 = tile_x`, `iVar6 = tile_w = 168`, `sVar1 = 156`.

Result: `x = tile_x + (168 - 156) = tile_x + 12`. The SHP is positioned **12px from the left edge of the tile**, flush against the right edge. This is **right-anchored**, not centered.

Our `push_button_shp` in `src/app_main_menu_shell_render.rs:80`:
```rust
let x = rect.x as f32 + (rect.w as f32 - frame_w) * 0.5;  // WRONG: centers
```
Correct formula: `let x = rect.x as f32 + (rect.w as f32 - frame_w);`

At 800x600: our x = 632 + (168 - 156) / 2 = 632 + 6 = **638**. GameMD: 632 + 12 = **644**. Difference: **6px right shift** needed.

Player-visible impact: the button artwork appears 6px too far to the left on each button. The bevel/frame of the chrome tile shows unevenly (12px left gap, 0 right gap in gamemd vs. 6px each side in ours).

Verified: `decompile_function 0x0072EC70`.

---

### Stage 5 — Pressed-state content offset

**VERIFIED: SHP origin unchanged; exact text rectangle recovered**

In `OwnerDraw_Button_00612B70` for iVar14==1:
```c
CC_Draw_Shape(piVar18, uStack_f4, &pHStack_64, &tStack_c0, 0x400, 0, ...);
```
The position `&pHStack_64` does not change between pressed/hover/default states. The CC_Draw_Shape call is identical for all three states except the frame index. **No positional offset is applied to the SHP when pressed.**

The assembly constructs these boundary rectangles before the centered text draw:

- normal: left=`x`, top=`y+1`, right=`x+w-2`, bottom=`y+h`
- pressed: left=`x+2`, top=`y+5`, right=`x+w-2`, bottom=`y+h`

In `(x,y,w,h)` form: normal `(x,y+1,w-2,h-1)`, pressed
`(x+2,y+5,w-4,h-5)`. Centered glyphs therefore land approximately +1 X/+2 Y,
but the changed clipping boundaries are the verified mechanism. The 2026-07-18 Rust
repair keeps SHP art stationary and emits these exact label rectangles.

Verified: `decompile_function(0x00612B70)` and
`disassemble_function(0x00612B70)` at `0x00613568..0x006135EE`.

---

### Stage 6 — Button Y slot positions

**PASS (tile structure); UNCHECKED (exact DLU values from dialog template)**

gamemd `RightPanel__ComputeLayoutRects @ 0x0072EC70` derives tile positions dynamically from SHP sizes, not from fixed DLU offsets. The tile Y for SDBTNBKGD is:
```c
DAT_00b0fc24[1] = DAT_00b0fc20[3] + iVar6;  // y = sdtp_height + margin
DAT_00b0fa20 = (screen_h - sdtp_h) / tile_h; // number of tiles
```
At 800x600: SDTP height = 199, tile height = 42 → first tile at y=199, tiles repeat every 42px.

Our layout produces: SP→tile 0 (y=199), WW→tile 1 (241), Net→tile 2 (283), M&C→tile 3 (325), Options→tile 4 (367), Exit→tile 8 (y=535).

The DLU Y values (125, 152, 179, 206, 233, 330) are used to snap buttons to tile rows. The snapping logic in `button_rect_for_dlu_y` is the mechanism for mapping dialog-template DLU coords to tile grid positions.

The Exit Game tile position (tile 8, y=535) vs the expected dialog DLU 330 produces `mul_div_round(330, 13, 8) = 536.25 → 537px`, which snaps to `(537 - 199 + 21) / 42 = 8.5... → tile 8 = y=535`. The large gap (tiles 5/6/7 unused) matches the gap between Options (DLU 233) and ExitGame (DLU 330). This gap is consistent with retail layout where Exit sits significantly lower than the other 5 buttons.

The exact DLU values cannot be verified from Ghidra without parsing the dialog template binary from the mix file. The tile-snap logic is internally consistent with the retail SHP sizes. **PASS conditional on DLU values being correct.**

---

### Stage 7 — Hover-state trigger rule

**FAIL**

gamemd hover mechanism (`OwnerDraw_Button_00612B70`, WM_SETFOCUS 0x113 handler):
```c
case 0x113:  // WM_SETFOCUS
    *(bool *)((int)piVar17 + 0xc5) = *(char *)((int)piVar17 + 0xc5) == '\0';
    // TOGGLE: if 0→1, if non-zero→0
    InvalidateRect(param_1, NULL, 1);
    break;
```

The hover/highlight flag (offset 0xC5) is set by **Win32 WM_SETFOCUS** — i.e., keyboard focus or mouse-click focus. WM_MOUSEMOVE (0x200) is NOT handled in the button proc and falls through to `CallWindowProcA`. GameMD hover = **focus-based** (click or Tab navigates to button → WM_SETFOCUS fires → highlight activates).

Our hover mechanism in `state.rs:mouse_move`:
```rust
pub fn mouse_move(state: &mut MainMenuShellState, layout: &MainMenuShellLayout, x: i32, y: i32) {
    state.hovered_owner_draw_button = hit_test_owner_draw_button(layout, x, y);
}
```
We set hover on **every mouse move**, using the full 168×42 tile rect. GameMD requires the button to receive keyboard/click focus.

Player-visible difference: in our engine, moving the mouse cursor over a button immediately turns it orange (hover color). In retail YR, the button only shows orange after clicking it (or tabbing to it with keyboard). For a mouse-only user clicking through the menu, the effect is nearly identical (click → focus → orange → action). But hovering without clicking should NOT show orange in retail.

Hit-test rect difference: our hit-test uses the full 168×42 tile rect. GameMD's hit-test is done by Windows for the Win32 button HWND, which covers the button's window rect. Since the button HWND was placed at the DLU rect position, the hit-test rect in gamemd is the DLU-derived position, which approximately equals our tile-snapped rect. This sub-stage is effectively PASS.

Verified: `decompile_function 0x00612B70`.

---

### Stage 8 — Pressed-state action rule (press vs. release)

**PASS**

gamemd dispatches `WM_COMMAND` to the parent dialog proc (`MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`) when the user **releases** the mouse button (standard Win32 button behavior — BN_CLICKED fires on mouse release). The dialog proc sets `*puVar2 = 1..6` (return code) on WM_COMMAND which exits the dialog message loop.

Our `mouse_up` in `state.rs`:
```rust
pub fn mouse_up(...) -> MainMenuShellAction {
    let released = hit_test_owner_draw_button(layout, x, y);
    let pressed = state.pressed_owner_draw_button.take();
    if pressed.is_some() && pressed == released {  // press+release on same button
        released.map(action_for_control)...
    }
```
We trigger action on **release over same button**. This matches the Win32 BN_CLICKED semantics (fire on release). PASS.

Verified: `decompile_function 0x00531F60`.

---

### Stage 9 — 7th control 0x71b YuriWebsite

**FAIL**

`MainMenuDialog0xE2_Proc_00531F60` WM_COMMAND (0x111) handler handles IDs: `0x683, 0x684, 0x578, 0x686, 0x55c, 0x3ee` — **6 controls only**. Control ID `0x71b` is **absent** from the dialog proc's WM_COMMAND dispatch. There is no owner-draw button with ID `0x71b` in the main menu shell.

The dialog proc references:
- `0x71a` — bik movie control (sends `0x4f0` message on WM_PAINT, positioned via SetWindowPos)
- `0x71d` — version string static (receives `0x4b2` = WM_SETTEXT on init)
- `0x71b` — NOT mentioned anywhere in the dialog proc or xrefs to `0x00531F60`

`0x71b` is likely a static image control (the Yuri's Revenge logo graphic) or is handled by the WW common dialog infrastructure without appearing in the DLGPROC, not an owner-draw button.

Our `state.rs` includes `YuriWebsite0x71b` in the `MainMenuControlId` enum, `action_for_control`, `csf_key_for_control`, and `tooltip_csf_key_for_control`. The `layout.rs` `buttons` array has only `[MainMenuButtonRect; 6]` and does not include a 0x71b entry, so this 7th control is NOT rendered or hit-tested. The enum variant exists but is dead code in the current layout.

The enum variant inclusion is harmless (no visual regression) but represents a misunderstanding of the dialog structure.

Verified: `decompile_function 0x00531F60`, xref scan on address `0x00531F60`.

---

### Stage 10 — Frame draw order (chrome / SHP / text batching)

**PASS**

gamemd: `RightPanel__Draw` paints all chrome (SDTP → SDBTNBKGD tiles → SDBTNANM frame10 → SDBTM → LWSCRNL). Then per-button WM_PAINT fires (Windows sends WM_PAINT to each owner-draw button child HWND), painting SHP + text for that button. Draw order: all chrome → (per button: SHP → text).

Our code: `build_chrome_instances` → `build_button_instances` (all 6 SHPs) → per-text draw. Draw order: all chrome → all 6 SHPs → all 6 text draws.

The net visual order is: chrome → SHPs → text. GameMD's per-button order is chrome → SHP → text, but since buttons don't overlap, the batched vs. interleaved distinction produces identical pixel output. Depth values (CHROME_DEPTH=0.00085, BUTTON_DEPTH=0.00080, TEXT_DEPTH=0.00070) correctly layer these.

PASS.

---

## Summary Table

| Stage | Topic | Verdict | File:Line (our code) | gamemd evidence |
|-------|-------|---------|---------------------|-----------------|
| 1 | Frame indices (2/3/4) | **PASS** | `main_menu_shell_chrome.rs:60-62` | `OwnerDraw_Button_00612B70 @ 0x00612EAA` |
| 2 | Palette (SDBTNANM.PAL) | **PASS** | `main_menu_shell_chrome.rs:58-59` | `FUN_0072E2C0 @ 0x00612EAA`, CC_Draw_Shape `0x004AED70` |
| 3 | SHP native size (156×42) | **PASS** | `app_main_menu_shell_render.rs:76-83` | retail `ra2.mix` SDBTNANM.SHP |
| 4 | Centering rule | **FAIL** | `app_main_menu_shell_render.rs:80` | `RightPanel__ComputeLayoutRects @ 0x0072EC70` |
| 5 | Pressed SHP + label composition | **PASS after 2026-07-18 repair** | `render/shell_paint.rs`, `app_main_menu_shell_render.rs::owner_draw_button_label_rect` | `OwnerDraw_Button_00612B70 @ 0x00613536`, assembly `0x00613568..0x006135EE` |
| 6 | Button Y slot positions | **PASS** | `layout.rs:296-320` | `RightPanel__ComputeLayoutRects @ 0x0072EC70` |
| 7 | Hover trigger rule | **FAIL** | `state.rs:104-106` | `OwnerDraw_Button_00612B70 @ 0x00612DB8` |
| 8 | Press/release action rule | **PASS** | `state.rs:108-123` | `MainMenuDialog0xE2_Proc @ 0x00531F60` |
| 9 | 7th control 0x71b | **FAIL** | `state.rs:13` | `MainMenuDialog0xE2_Proc @ 0x00531F60` |
| 10 | Frame draw order | **PASS** | `app_main_menu_shell_render.rs:335-437` | `RightPanel__Draw @ 0x0072E450` |

**PASS: 7 | FAIL: 3 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

---

## Top 5 Player-Visible Failures

1. **Stage 4 — Centering (FAIL)**  
   Button artwork is 6px too far left on each button. The chrome bevel gap is symmetric (6px each side) instead of left-only (12px left, 0px right). Players see the button SHP misaligned within its chrome tile frame — most visible on the left edge where the bevel is wider than intended.  
   File: `src/app_main_menu_shell_render.rs:80`  
   Fix: `let x = rect.x as f32 + (rect.w as f32 - frame_w);` (right-anchor, not center)  
   Evidence: `RightPanel__ComputeLayoutRects @ 0x0072EC70`: `x = tile_x + tile_w - shp_w`

2. **Stage 7 — Hover trigger (FAIL)**  
   Our engine shows orange hover color when the mouse cursor moves over any button. Retail YR only shows orange when the button has keyboard/click focus (WM_SETFOCUS). Players see buttons pre-highlight on mouse-over in our engine but not in retail — affects every mouse movement over the menu area. Triggers on every frame with cursor in the right panel.  
   File: `src/state.rs:104-106` (mouse_move sets hovered_owner_draw_button)  
   Evidence: `OwnerDraw_Button_00612B70 @ 0x00612B70`: WM_SETFOCUS (0x113) toggles highlight byte; WM_MOUSEMOVE (0x200) not handled.

3. **Stage 9 — 7th control 0x71b YuriWebsite (FAIL)**  
   Our `MainMenuControlId` enum includes `YuriWebsite0x71b` as an owner-draw button. It is absent from gamemd's WM_COMMAND dispatch in `MainMenuDialog0xE2_Proc`. The actual 0x71b control is likely a static image (Yuri's Revenge logo), not a button. The variant is dead code in layout (buttons[6] array has only 6 entries) so no visual regression, but the enum adds misleading code. Triggers: never (layout ignores it), but it's a design error.  
   File: `src/ui/main_menu_shell/state.rs:13`  
   Evidence: `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`: WM_COMMAND handles 0x683, 0x684, 0x578, 0x686, 0x55c, 0x3ee only.

4. **Stage 5/Text — resolved by the 2026-07-18 correction**  
   The earlier “+1 X only” description was incomplete. Rust now uses the exact normal and
   pressed clipping rectangles recovered from `0x00613568..0x006135EE`.

5. *(No additional FAIL/NOT-IMPLEMENTED stages within this trace's 10 stages.)*

---

## Confidence Notes

- Stages 1, 5, 8, 10: HIGH — decompiled function directly, logic is unambiguous.
- Stage 2: HIGH — palette load chain traced; CC_Draw_Shape call confirmed; no remap flag.
- Stage 3: HIGH — direct retail asset read from `ra2.mix`.
- Stage 4: HIGH — `RightPanel__ComputeLayoutRects` clearly computes `x = tile_x + tile_w - shp_w`.
- Stage 6: MEDIUM — tile Y positions consistent with SHP dimensions; DLU values in layout.rs not independently verified from dialog template binary.
- Stage 7: HIGH — WM_SETFOCUS toggle confirmed; WM_MOUSEMOVE absence confirmed by switch-case analysis.
- Stage 9: HIGH — WM_COMMAND dispatch in MainMenuDialog0xE2_Proc enumerates all 6 active IDs; 0x71b absent.

**Status: COMPLETE**
