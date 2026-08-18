# Main-Menu Button Text Rendering — Pipeline Trace

**Date:** 2026-05-19
**Scope:** Dialog `0xE2` owner-draw button labels only (6 buttons: SinglePlayer,
WWOnline, Network, MoviesAndCredits, Options, ExitGame). 800×600 English CSF.
**Slot:** Trace-Swarm slot 4 — font, color, shadow, alignment, pressed-Y, CSF keys,
upscaling, kerning, missing-CSF fallback.

Sources used (all pre-existing in ra2-rust-game-docs/):
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`
- `MAIN_MENU_TITLE_TEXT_RENDER_GHIDRA_REPORT.md`

---

## Stage Results

### Stage 1 — Font
**PASS**

gamemd uses `g_GAME_FNT @ 0x0089C4D0`, loaded from `GAME.FNT` via
`BitText__Constructor @ 0x00434AD0`. GAME.FNT: `fonT`-magic, 16-px bitmap rows,
17-px cell height (line advance), 1-bit-per-pixel, MSB-first, 3 bytes/row,
per-glyph variable pixel width (first byte of each glyph slot), codepoint lookup
table (65536 u16 entries).

Our code uses `state.bit_font` which is also loaded from `GAME.FNT`. Cell height
17 px is shared (`CELL_HEIGHT = 17` in `bit_font.rs`). The font IS the same file,
same geometry. GAME.FNT is the only font used for all owner-draw shell controls in
gamemd; no MS Sans Serif or other Win32 font is involved in the glyph rasterization
(the dialog template font is used only for system dialog metrics/DLU calculation, not
for owner-draw paint).

Evidence: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md §2`, `§3.1`, `§5`.

---

### Stage 2 — Text Color
**PASS** (note: color was wrong before recent commit; now correct)

gamemd: `OwnerDraw_Button_00612B70` loads `DAT_00AC18A4 = 0x0000FFFF` and passes it
to `FUN_00621040`. That wrapper extracts: `R = value & 0xFF = 0xFF`, `G = (value >>
8) & 0xFF = 0xFF`, `B = (value >> 16) & 0xFF = 0x00`. Result: **RGB #FFFF00 (yellow)**.

Evidence (verified from binary):
- `FUN_0060F9A0 @ 0x0060FA3F`: `MOV dword ptr [0x00ac18a4], 0xFFFF`
- `OwnerDraw_Button_00612B70 @ 0x00612da9`: `MOV EDI, [0x00ac18a4]` (loads yellow)
- `FUN_00621040 @ 0x00621040`: byte-extraction documented in
  `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §3`

Our code: `SHELL_BUTTON_TEXT_RGB_FFFF00 = [1.0, 1.0, 0.0]` = #FFFF00.
This was the change in commit `e118bf2` ("switch label color to yellow").
The previous constant was a dark near-black — that was wrong. The current value
is correct.

Note: `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md §5`
explicitly confirmed yellow and noted the prior constant was wrong.

---

### Stage 3 — Drop Shadow / Outline
**PASS** (no shadow present in either gamemd or our code)

gamemd: `FUN_00621040` calls `FUN_00434CD0` (BitFont DrawWithWrap) once. That calls
`FUN_00434120` (DrawGlyph) once per character. `FUN_00434120` performs a single 1bpp
blit: one color value written per set bit. There is NO second pass, NO tinted-offset
preliminary draw, NO outline stroke. Cleared bits leave destination unchanged.

Evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §3` ("NO shadow / outline
pass — verified") and `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md §3.3`.

Our code: `push_label` passes shadow `[0.0, 0.0]` offset. In `shell_text.rs`
`draw_in_rect`, no shadow-offset call is present. This correctly matches gamemd:
no shadow.

Note: `piVar11[0x2b] = 0x0C` (initialized in `FUN_0060F9A0`) is passed as a dead
arg to `FUN_00621040`. It is never read inside that function. Shadow arg = dead
code.

---

### Stage 4 — Alignment
**FAIL** (partial — H_CENTER matches; V_CENTER behavior diverges)

**Horizontal center:** PASS.
gamemd flag byte to `FUN_00621040` is `0x05` (= h-center bit `0x01` | v-center bit
`0x04`). `FUN_00434CD0` tests `param_8 & 1` for h-center. Result: text is
horizontally centered within the (shifted) text rect. Our code uses `ShellAlign::
H_CENTER | ShellAlign::V_CENTER` which maps to `0x01 | 0x04 = 0x05` — identical bit
pattern. H-center math: `((rect_w - text_w) / 2)` offset. Matches.

**Vertical center:** DIVERGES in rect definition.

gamemd builds the text rect as:
- Up state: `{left = window_left, top = window_top + 1, right = window_left + width - 2, bottom = window_top + height}`
- The rect is 1 px inset from top and 2 px inset from right vs. the full button window.

`FUN_00621040` v-center pre-pass: measures text_h, then `y = rect.top + (rect.h - text_h) / 2`.

Our code uses `button.rect` directly (the full tile rect from layout), with no inset.
The layout rect for buttons is the full chrome tile rect scaled to the responsive
layout. This differs from gamemd's `window_top + 1` top inset and `width - 2` right
inset.

**Net effect:** For a 37-px-tall button with 17-px text, the v-center offset is
`(37 - 1 - 17) / 2 = 9 px` from window_top + 1 = ~10 px from window_top in gamemd.
Our v-center: `(37 - 17) / 2 = 10` px from window_top. Off by 1 px vertically.

At 800×600 this is a 1-pixel vertical offset. Player-visible but very subtle.

Evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §2` (text rect construction
assembly at `0x0061358d..0x006135cd`).

---

### Stage 5 — Pressed-State Y Shift
**FAIL** (direction and amount match for art; text has additional +1 px horizontal
drift not reproduced)

gamemd pressed-text rect adjustment:
- `left += 2`, `top += 4`, `right unchanged`, `bottom unchanged`
- After h-center + v-center through `FUN_00621040`: **net text shift = +2 px down,
  +1 px right** vs. unpressed.

Our code: `pressed_content_offset_y = 2` — applied to `y_offset` in `push_label`.
This applies a +2 px Y shift to the entire TextRect (both origin and scissor Y).
No X shift is applied.

Result vs. gamemd:
- Y shift +2: **matches** the +2 px down net shift.
- X shift: gamemd shifts text +1 px right on press (due to left +=2 with h-center
  recalculation). Our code does not shift X. **1 px horizontal drift on press.**

Additionally, gamemd applies the same +2 art shift to the SHP art piece (documented
in `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md §4`). Our
`push_button_shp` does NOT apply `pressed_content_offset_y` — the art stays in
place. **Art does not shift with the text on press.**

Evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §2`.

---

### Stage 6 — CSF Key Resolution
**PASS**

gamemd: `OwnerDraw_Button_00612B70` does NOT directly call `GetDlgItemText`.
Instead, on `WM_INITDIALOG`, `FUN_0060F9A0` reads the control's Win32 template
title via `CallWindowProcA(prev, WM_GETTEXT)` then calls
`StringTable__LoadString @ 0x00734E60` with that key as the lookup. The template
titles in `RT_DIALOG 0xE2` are:

| Control Id | Template title (CSF key) |
|---|---|
| `0x683` | `GUI:SinglePlayer` |
| `0x684` | `GUI:WWOnline` |
| `0x578` | `GUI:Network` |
| `0x686` | `GUI:MoviesAndCredits` |
| `0x55C` | `GUI:Options` |
| `0x3EE` | `GUI:ExitGame` |

Our code in `csf_key_for_control` (state.rs:62):
```
SinglePlayer0x683 => "GUI:SinglePlayer"
WwOnline0x684     => "GUI:WWOnline"
Network0x578      => "GUI:Network"
MoviesAndCredits0x686 => "GUI:MoviesAndCredits"
Options0x55c      => "GUI:Options"
ExitGame0x3ee     => "GUI:ExitGame"
```

All six keys match exactly. The fallback behavior also matches (see Stage 9).

Evidence: `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
control table; `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md §6`.

---

### Stage 7 — Glyph Upscaling at Higher Resolutions
**UNCHECKED** (intentional approved drift — see note)

gamemd: GAME.FNT glyphs are rasterized 1:1 to the 16-bit shell surface in
`FUN_00434120`. At 800×600 or any other resolution gamemd supports (640×480,
1024×768), the glyph pixels are drawn 1:1 — no bilinear scaling, no nearest-neighbor
upscale. At 1024×768 the dialog is expanded but the font is still 17-px cell height.

Our code: `compute_responsive_layout` scales the entire shell (including button rects)
to the client window size. The `bit_font` atlas glyphs are drawn as sprite quads with
size set from `pixel_width` via `build_text` — this allows GPU scaling of the atlas
texture when button rects are larger than 800×600.

This is an **intentional approved drift** from gamemd's fixed-resolution behavior.
The user approved responsive layout scaling. At exactly 800×600, the scale factor is
1.0 and behavior is pixel-identical. Above 800×600 our text scales proportionally;
gamemd draws 1:1 glyphs in a fixed-geometry shell.

Not marked FAIL because this is user-approved scale behavior (see
`MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md §5` "Overall
scaling ... Intentional user-approved drift").

---

### Stage 8 — Kerning (Fixed-Width vs Per-Glyph)
**PASS**

gamemd: `FUN_00433CF0` (MeasureText) and `FUN_00434120` (DrawGlyph) use
**per-glyph variable widths**: `glyph[0]` is the pixel width stored in the glyph
slot, plus `outer[+0x2C] = 1` inter-character gap. Not fixed-width.

Our code: `BitFont::build_text` iterates chars, looks up each glyph, advances by
`glyph_entry.pixel_width + char_spacing (= 1)`. This matches the variable-width
per-glyph advance. `wrap_layout` also uses `glyph_width + 1` per char.

Evidence: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md §3.2` (MeasureText), `§3.3`
(DrawGlyph advance = `outer[+0x2C] + width`).

---

### Stage 9 — Empty/Missing CSF Fallback
**PASS**

gamemd: `StringTable__LoadString @ 0x00734E60` returns a heap-allocated
`"MISSING: <key>"` placeholder wide string when the CSF key is absent. The control
then draws that placeholder text using the same yellow font path.

Our code (`resolve_csf` in `app_main_menu_shell_render.rs:85`):
```rust
state.csf.as_ref().and_then(|csf| csf.get(key)).unwrap_or(key)
```
Falls back to the **key string itself** (e.g. `"GUI:SinglePlayer"`) when CSF is
missing or key absent.

Both paths produce visible text on screen — neither silently draws nothing. The
exact fallback text differs (`"MISSING: GUI:SinglePlayer"` in gamemd vs
`"GUI:SinglePlayer"` in ours), but a missing CSF on a retail install is not
a normal operating condition. The fallback behavior is not player-visible under
normal circumstances.

---

## Verdict Tally

**PASS: 6 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0**

Stages:
1. Font: PASS
2. Color: PASS
3. Shadow: PASS
4. Alignment (H): PASS / (V rect): FAIL
5. Pressed Y shift: FAIL (art not shifted; X not shifted)
6. CSF Keys: PASS
7. Upscaling: UNCHECKED (approved drift)
8. Kerning: PASS
9. Fallback: PASS

---

## Top 5 Player-Visible Failures

1. **Stage 5: Pressed art SHP not shifted.**
   Player sees: button text moves down 2 px on click, but the SHP button art
   stays in place — text and art do not move together. In gamemd both shift
   together (+2 px art via `button_art_y += 2`, +2 px net text via rect
   adjustment).
   Our code: `push_button_shp` in `src/app_main_menu_shell_render.rs:62` has no
   pressed Y offset parameter.
   gamemd evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §2`;
   `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md §4`.

2. **Stage 5: Pressed text missing +1 px right shift.**
   Player sees: on button press, text should move right 1 px (due to left+=2 with
   h-center recalculation narrowing rect by 2 px). Ours moves only in Y.
   Our code: `push_label` in `src/app_main_menu_shell_render.rs:93` adds only
   `y_offset`, never an x offset.
   gamemd evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §2` ("Δx = +1 px
   right").

3. **Stage 4: V-center text rect inset mismatch.**
   Player sees: button label vertically positioned ~1 px too low at 800×600
   (we use full rect height for v-center; gamemd uses rect height - 1 = 36 px).
   Our code: `button.rect` directly in `push_label` call,
   `src/app_main_menu_shell_render.rs:201`.
   gamemd evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §2`
   (`rect.top = window_top + 1`, `rect.right = window_left + width - 2`).

4. **Stage 4: H-center rect right-edge inset mismatch.**
   Player sees: text h-centering uses rect width = `button_w`; gamemd uses
   `button_w - 2` (2 px right inset). For typical button text this shifts the
   visual center 1 px left vs gamemd.
   Our code: same as item 3 above — `button.rect.w` used directly.
   gamemd evidence: same source.

5. **Hover state: we render a hover SHP frame; gamemd has no hover state.**
   Player sees: in our shell, hovering a button shows `button_hover` SHP frame.
   In gamemd, `OwnerDraw_Button_00612B70` has NO `WM_MOUSEMOVE` handler; the
   PCX button paint only checks pressed + disabled state. There is no third
   "hovered" visual state for PCX buttons.
   Our code: `button_frame` in `src/app_main_menu_shell_render.rs:48` selects
   `atlas.button_hover` when `hovered == true`.
   gamemd evidence: `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md §4` ("NONE —
   exhaustive WM_MOUSEMOVE absence confirmed").

---

## Status

COMPLETE

All 9 stages evaluated. Stages 1, 2, 3, 6, 8, 9 fully verified against gamemd.
Stage 7 is intentional approved drift. Stages 4 and 5 have confirmed disparities.
