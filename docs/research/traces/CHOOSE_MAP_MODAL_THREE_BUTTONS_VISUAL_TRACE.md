# Choose Map Modal — Three Buttons Visual Rendering Trace

**Scenario:** Standard offline YR Skirmish, 800×600, Choose Map modal (dialog `0x6B`) open.
Buttons: Use Map `0x6C5`, Create Random Map `0x583`, Cancel `0x5C0`.

**Scope:** Visual rendering only — art source (SDBTNANM vs gray PCX), button cell geometry,
pressed-state art frame, pressed content/text offset, button text label (string, align,
color, pressed offset). No sounds, list behavior, or preview content.

**Date:** 2026-06-01

**Verdict Tally:** PASS: 5 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

---

## Stage 1 — Art Source: SDBTNANM type-1 vs gray PCX

**gamemd evidence:** `OwnerDraw_Button_00612B70` type `1` branch (set by `FUN_0060A330` classifier)
draws `g_SDBTNANM_SHP` frames 2/4 for these buttons. All three modal buttons
(`0x6C5`, `0x583`, `0x5C0`) have style `0x5000000B`; `FUN_0060F9A0` routes style
`(& 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70`. The classifier then sets owner-draw
type `1`, activating the `g_SDBTNANM_SHP` path. Source:
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`.

**Rust (`src/app_skirmish_shell_render/modals.rs:141-161`):** The loop over
`(use_map_button, UseMap0x6c5)`, `(cancel_button, Cancel0x5c0)`,
`(create_random_map_button, CreateRandomMap0x583)` calls
`push_right_panel_button_shp` for each — the SDBTNANM path. The gray PCX
fallback (`push_button_30`) is reached only when the atlas entry is `None`
(asset missing), not as the normal path.

**Verdict:** PASS. Art source correctly uses SDBTNANM frames 2/4 for idle/pressed; gray PCX
is fallback-only, matching gamemd type-1 classification.

---

## Stage 2 — Button Cell Geometry at 800×600

**gamemd evidence (`SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` §Control Routing):**

| Button | gamemd 800×600 final rect |
|---|---|
| Use Map `0x6C5` | `(644, 199, 156, 42)` |
| Create Random Map `0x583` | `(644, 241, 156, 42)` |
| Cancel `0x5C0` | `(644, 535, 156, 42)` |

Helper routes: Use Map and Create Random Map → `0x0060B000` tile-snap; Cancel → `0x0060B350`
bottom/right helper. All use SDBTNANM 156-wide cell flush-right.

**Rust (`src/ui/skirmish_shell/layout.rs:552-584`):**

`compute_choose_map_modal_layout(800, 600)`:
- `panel = right_panel_rects(800, 600)` → `tile.y = 199`, `tile_count = 9`, SDBTNANM_W = 156
- `use_map_base = dlu_rect(425, 122, 108, 23)` → `(638, 198, 162, 37)`
  - `snap_button_biased_truncate(800, 600, ..., panel, 156)` → `tile_index = 0` → `(644, 199, 156, 42)` ✓
- `create_random_map_base = dlu_rect(425, 149, 108, 23)` → `(638, 242, 162, 37)`
  - `snap_button_biased_truncate(800, 600, ..., panel, 156)` → `tile_index = 1` → `(644, 241, 156, 42)` ✓
- `cancel_button = back_rect(800, panel)`:
  - `x = 800 - 0 - 156 = 644`, `y = 199 + (9-1)*42 = 535` → `(644, 535, 156, 42)` ✓

All three rects are pixel-identical to gamemd's expected positions.

**Verdict:** PASS. Button cell geometry matches gamemd at 800×600 for all three controls.

---

## Stage 3 — Atlas Load: SDBTNANM.SHP Frames 2 and 4

**Expected:** `right_panel_button_sdbtnanm_frame2` = `sdbtnanm.shp#2` (idle),
`right_panel_button_sdbtnanm_frame4` = `sdbtnanm.shp#4` (pressed).

**Rust (`src/render/skirmish_shell_chrome.rs:352-354`):**
```
right_panel_button_sdbtnanm_frame2: by_label.get("sdbtnanm.shp#2").copied(),
right_panel_button_sdbtnanm_frame4: by_label.get("sdbtnanm.shp#4").copied(),
```

Both entries are loaded from the atlas by exact SHP label. `push_right_panel_button_shp`
(`src/app_skirmish_shell_render/chrome.rs:373-376`) selects frame4 when pressed else frame2.
`right_panel_button_sdbtnanm_frame_index(pressed, disabled)` returns `4` only when
`pressed && !disabled` (line 388).

**gamemd evidence:** `OwnerDraw_Button_00612B70` type-1 branch — frame 2 for released/default,
frame 4 for pressed. Source: `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`.

**Verdict:** PASS. Frame indices 2/4 correct; atlas labels match.

---

## Stage 4 — Pressed State: Art Frame and Content Offset

**gamemd evidence:** Pressing a type-1 button activates frame 4 of `SDBTNANM.SHP` (the
pressed-look art). For the text content offset, `OwnerDraw_Button_00612B70` §3.5
(`SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`) documents — shared by all
button styles through the same text-rect setup block `0x00613591..0x006135CD`:
- Released: `(left, top+1, left+w-2, top+h)`
- Pressed: `(left+2, top+5, left+w-2, top+h)` → net shift: right +2, down +4 relative to released

Note: the `top+1` baseline for released means pressed is effectively `+4 py` below released
baseline. The binary uses these for the text draw; the art frame switch (frame 2→4) encodes
the visual pressed look without an art position shift.

**Rust art (`src/app_skirmish_shell_render/chrome.rs:377-378`):**
`push_entry(out, entry, rect, depth)` — passes `rect` unchanged. The SDBTNANM frame itself
encodes the pressed look; no y-shift of the art position is needed or applied. Correct.

**Rust text (`src/app_skirmish_shell_render/text.rs:826-834`):**
The three choose map button labels are drawn with `rect_to_text_rect(rect)` — this maps
the button rect directly to `TextRect { x: rect.x, y: rect.y, w: ..., h: ... }` with NO
offset. Neither the released `+1` y baseline nor the pressed `+2x/+5y` shift is applied.

Compare with the setup shell buttons (e.g., the validation modal OK button at
`text.rs:919`) which correctly uses `button_text_rect(layout.ok_button, modal.ok_button_pressed)`
— that function applies `y = rect.y + 1` released / `y = rect.y + 5, x += 2` pressed.

**Player-visible diff:** On a 42-px tall button at 800×600:
- Released text: centered in the full rect (`rect.y` through `rect.y+42`). With `rect_to_text_rect`, center-V puts text at ~`rect.y + (42-font_h)/2`. With `button_text_rect`, text rect is `(rect.y+1, rect.y+42)`, so center-V places text at `rect.y + 1 + (41-font_h)/2`. Difference: 0 or 1 px depending on font height parity — marginal but a literal inequality.
- Pressed text: with `button_text_rect` the text rect shifts to `(left+2, top+5)`. With `rect_to_text_rect` there is no shift. Pressing any button does not move the text label — visibly wrong on mouse-down press.

**Verdict:** FAIL. Button text for all three choose map modal buttons uses `rect_to_text_rect`
instead of `button_text_rect(rect, pressed)`. The released `+1y` inset and the pressed
`+2x/+5y` offset are both absent.
- File: `src/app_skirmish_shell_render/text.rs:830`
- Fix: replace `rect_to_text_rect(rect)` with `button_text_rect(rect, modal.pressed_button == Some(id))`
  and thread the pressed state into the loop (matching the chrome loop at `modals.rs:157`).

---

## Stage 5 — Button Text Label: String Keys, Alignment, Color

**gamemd evidence:** Type-1 owner-draw buttons draw their Win32 resource caption string
through `FUN_00621040` with flags `0x0C` (H_CENTER=8, V_CENTER=4). Color comes from
`DAT_00AC18A4 = 0x0000FFFF` — yellow in the shell's display format; as RGB float
`[1.0, 1.0, 0.0]`. Source: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` §3.5
and `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`.

**Rust (`src/app_skirmish_shell_render/text.rs:812-834`):**
- Use Map: `localized_label(state, "GUI:UseMap", "Use Map")` ✓
- Cancel: `localized_label(state, "GUI:Cancel", "Cancel")` ✓
- Create Random Map: `localized_label(state, "GUI:CreateRandomMap", "Create Random Map")` ✓
- Alignment: `ShellAlign::H_CENTER | ShellAlign::V_CENTER` ✓
- Color: `SHELL_LABEL_TEXT_RGB = [1.0, 1.0, 0.0]` which equals `DAT_00AC18A4 = 0x0000FFFF`
  (ABGR: A=0, B=0, G=0xFF, R=0xFF → R=1.0, G=1.0, B=0.0) ✓

String keys, alignment flags, and color all match gamemd.

**Verdict:** PASS (independent of the pressed-offset FAIL in Stage 4 which affects
the text *rect*, not the string key / alignment flags / color).

---

## Stage 6 — YR-Active Confirmation

All three buttons (`0x6C5`, `0x583`, `0x5C0`) are active in standard offline YR Skirmish:
- `0x6C5` (Use Map) and `0x5C0` (Cancel): always present; active in YR per
  `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`.
- `0x583` (Create Random Map): present in the resource; active when the player clicks it
  (Conditional — requires player action, not TS-only disabled).

No TS-only gate found for any of these three controls. The owner-draw type-1 classification
path through `FUN_0060A330` / `OwnerDraw_Button_00612B70` is a standard YR shell mechanism.

---

## Summary of Findings

| Stage | Topic | Verdict |
|---|---|---|
| 1 | Art source: SDBTNANM type-1 (not gray PCX) | PASS |
| 2 | Button cell geometry 800×600: (644,199), (644,241), (644,535) | PASS |
| 3 | Atlas loads sdbtnanm.shp#2 / #4 correctly | PASS |
| 4 | Pressed art frame (4) correct; pressed text offset MISSING | FAIL |
| 5 | Text keys GUI:UseMap/CreateRandomMap/Cancel, H+V center, yellow | PASS |
| 6 | YR-active confirmation | PASS (informational) |

**PASS: 5 | FAIL: 1 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0**

---

## Top Player-Visible Failure

**Stage 4 — Pressed text offset absent**

- **What player sees:** when pressing Use Map, Create Random Map, or Cancel, the button art
  switches to frame 4 (visually pressed) but the text label does NOT shift down-right.
  In gamemd the label moves ~4 px down and 2 px right on mouse-down, giving tactile feedback
  that the button is depressed. Here the text stays fixed.
- **Frequency:** every modal button press — fires every time the player interacts with the
  Choose Map modal.
- **Our file:line:** `src/app_skirmish_shell_render/text.rs:830` (`rect_to_text_rect(rect)`)
- **gamemd evidence:** `OwnerDraw_Button_00612B70` text rect block `0x00613591..0x006135CD`;
  pressed rect is `(left+2, top+5, left+w-2, top+h)` vs released `(left, top+1, left+w-2, top+h)`.
  `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` §3.5.
- **Fix:** change the three-button text loop in `push_choose_map_modal_text_draws`
  (`src/app_skirmish_shell_render/text.rs:812-834`) to use
  `button_text_rect(rect, modal.pressed_button == Some(id))` matching the
  `push_right_panel_button_shp` loop's pressed-state threading at `modals.rs:157`.

---

## Adjacent Finding (out of scope — not traced)

The prior `SKIRMISH_CHOOSE_MAP_MODAL_FIRST_PAINT_VISUAL_TRACE.md` (Stage 4) flagged that
`compute_choose_map_modal_layout` was computing geometry incorrectly. This has since been
fixed: the current Rust code correctly uses `right_panel_rects` + `snap_button_biased_truncate`
+ `back_rect`, and the final pixel positions match the binary evidence. That stale FAIL
in the earlier trace is resolved.
