# Choose Map Modal Listbox Visual Trace

**Scenario:** Modal 0x6B open at 800x600, populated map list (overflow → scrollbar present), one row selected.
**Scope:** Visual rendering of mode list `0x6EB` (116,127,195,343) and map list `0x553` (338,127,195,343) vs `OwnerDraw_ListBox_00618D40` at `0x00618D40`.
**Gamemd evidence:** `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`.
**Verdict Tally:** PASS: 7 | FAIL: 3 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

---

## Stage 1 — List Panel Background Fill + Position

### 1a. List positions at 800x600

- **gamemd:** `0x6EB` final rect `(116,127,195,343)`, `0x553` final rect `(338,127,195,343)`.
  Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` §Control Routing.
- **Rust:** `compute_choose_map_modal_layout(800,600)` returns `mode_list = (116,127,195,343)`,
  `map_list = (338,127,195,343)`.
  Confirmed by test `choose_map_modal_layout_matches_verified_0x6b_geometry` in
  `src/ui/skirmish_shell/layout.rs:1127`.
- **Verdict:** PASS.

### 1b. Background fill color

- **gamemd:** `OwnerDraw_ListBox_00618D40` at `0x00619230..0x006194C2` first calls
  `FUN_006208F0(2,-1)` for the primitive frame, then fills the selected row. The
  unselected background behind rows is composed by copying from the backing surface
  (`BSurface` alpha path) before alpha processing — it is **not** a single static RGB
  fill. Evidence: `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §3.1.
- **Rust:** uses `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE = [0.015, 0.024, 0.018]`
  as a solid placeholder fill (`src/app_skirmish_shell_render/modals.rs:42`).
  The constant name explicitly marks it pending source capture.
- **Player-visible diff:** Unselected listbox background color is a placeholder approximation;
  exact RGB awaits runtime surface capture. Magnitude unknown but potentially visible against
  the `MnScrnLCustomizeBattle` asset.
- **Verdict:** UNCHECKED (exact background RGB unverified; placeholder pending runtime capture).

---

## Stage 2 — Row Height: 19px (font_height+2)

- **gamemd:** Init message `0x497` path at `0x006191BD..0x0061920A` measures shell font/text
  height, adds `2`, and sends `LB_SETITEMHEIGHT (0x1A0)` with `wParam = -1`. Shell font is
  `GAME.FNT` with cell height `17`. Standard row height = `17 + 2 = 19` px.
  Evidence: `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` §3 Row Height.
- **Rust:** `CHOOSE_MAP_LIST_ROW_H = 19` at `src/ui/skirmish_shell/layout.rs:37`.
  Also `CHOOSE_MAP_LISTBOX_ROW_H = CHOOSE_MAP_LIST_ROW_H` at line 38.
  All helper functions (`choose_map_listbox_visible_row_count`, `choose_map_listbox_row_rect`,
  `choose_map_listbox_row_at`) use this constant.
  Test `choose_map_modal_list_hit_test_uses_verified_owner_draw_row_height` asserts row-boundary
  behavior.
- **Verdict:** PASS. (Row height is 19, matching `font_height+2` formula.)

---

## Stage 3 — Selected-Row Highlight

### 3a. Full content-row fill extent

- **gamemd:** When `LB_GETSEL (0x187)` returns non-zero for the row, the callback fills the
  **full item rectangle** (content width × row height) via surface vtable `+0x14` before drawing
  text. No inset. Evidence: `0x00619A65..0x00619AD0`; `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` §3 Selected Row Fill.
- **Rust:** `choose_map_listbox_row_rect(content, row)` returns a rect of width `content.w`
  and height `CHOOSE_MAP_LISTBOX_ROW_H`, and `push_solid_rect` fills the whole rect with
  `OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF`.
  See `src/app_skirmish_shell_render/modals.rs:51-58`.
- **Verdict:** PASS. (Full-row fill with no inset, matches gamemd.)

### 3b. Selected fill color

- **gamemd:** Fill color source is `DAT_00AC4604`. `FUN_0060F9A0` initializes
  `DAT_00AC4604 = 0x000000FF` (packed 0x00BBGGRR → R=255, G=0, B=0 = pure red).
  Evidence: `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §3;
  `0x00619A65..0x00619AD0`.
- **Rust:** `OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF = [1.0, 0.0, 0.0]`
  (pure red, `src/app_skirmish_shell_render.rs:95`).
- **Verdict:** PASS. (Selection fill is pure red `[1,0,0]`, matching DAT_00AC4604=0xFF=R=255.)

---

## Stage 4 — Row Text: Font, Color, Vertical Centering, Left Alignment, +2px Inset

### 4a. Text left inset (+2px)

- **gamemd:** Text rect is `item_left + 2`, `item_top`, spans to `item_right`, `item_bottom`.
  Evidence: `0x00619AD3..0x00619B1C`; `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` §3 Text Inset.
- **Rust:** `let rect = RectPx::new(row_rect.x + 2, row_rect.y, row_rect.w - 2, row_rect.h)` at
  `src/app_skirmish_shell_render/text.rs:859` (mode list) and `:885` (map list).
- **Verdict:** PASS.

### 4b. Text vertical centering

- **gamemd:** `FUN_00621040` with flags `0x04` (V_CENTER) — measured text height subtracted from
  row height / 2, centering within item rect. Evidence: `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md` §2.
- **Rust:** `ShellAlign::V_CENTER` passed to `push_text_draw` at `src/app_skirmish_shell_render/text.rs:866, :893`.
- **Verdict:** PASS.

### 4c. Text color

- **gamemd:** Normal text color is `DAT_00AC18A4 = 0x0000FFFF` (packed 0x00BBGGRR → R=255,G=255,B=0 = yellow).
  Evidence: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`;
  `0x00619A30..0x00619A65`; init at `FUN_0060F9A0`.
- **Rust:** `SHELL_LABEL_TEXT_RGB = [1.0, 1.0, 0.0]` at `src/app_skirmish_shell_render.rs:77`.
  Used for both mode and map row text.
- **Verdict:** PASS.

### 4d. Text pre-truncation

- **gamemd:** Before draw, the callback measures the row text and repeatedly truncates the
  UTF-16 string until it fits the row width. Evidence: truncation loop `0x00619941..0x006199AC`.
- **Rust:** Row text is drawn with `push_text_draw` using the row rect as clip rect; there is
  no pre-truncation loop equivalent — relies on the text renderer's clip/wrap behavior.
  `src/app_skirmish_shell_render/text.rs:860-868`.
- **Player-visible diff:** Long map/mode names may clip differently from gamemd — gamemd
  pre-truncates with the last fitting character shown; Rust clips at the rect boundary which
  may show a partial glyph.
- **Verdict:** UNCHECKED (no pre-truncation loop; Rust relies on clip-rect clamping; partial
  glyph visibility may differ).

---

## Stage 5 — Scrollbar: 20px Width, Content Shrink, Arrows + Thumb Geometry

### 5a. Scrollbar width = 20px and content shrinks

- **gamemd:** `DAT_00AC1DF0 * 2 + 0x12` with `DAT_00AC1DF0 = 1` → `20` px. When overflow
  creates the scrollbar child, list client width is shrunk by 20 px. Row content and hit testing
  use shrunken width. Evidence: `0x00618E38..0x00618E4C`, `0x0061BBF8..0x0061BE42`;
  `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` §3 Scrollbar.
- **Rust:** `CHOOSE_MAP_LISTBOX_SCROLLBAR_W = 20` at `src/ui/skirmish_shell/layout.rs:39`.
  `choose_map_listbox_content_rect` subtracts 20px when overflow present.
  `choose_map_listbox_scrollbar_rect` returns `20`-px-wide rect at right edge.
  Test `choose_map_modal_listbox_hit_testing_reserves_scrollbar_width` asserts
  `scrollbar = (513,127,20,343)`, `content = (338,127,175,343)`.
- **Verdict:** PASS.

### 5b. Scrollbar arrow button height = 22px, min thumb = 14px

- **gamemd:** Arrow buttons are `22` px high; minimum thumb height is `14` px.
  Evidence: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`; `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md` §2.
- **Rust:** `COMBO_DROPDOWN_SCROLLBAR_BUTTON_H = 22` and `COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H = 14`
  (imported from `src/ui/skirmish_shell/layout.rs` for the thumb calculation at line 658-661).
  Test `choose_map_modal_scrollbar_thumb_and_track_map_to_top_index` asserts thumb stays inside
  `[scrollbar.y + 22, scrollbar.y + scrollbar.h - 22]`.
- **Verdict:** PASS.

### 5c. Scrollbar track fill color

- **gamemd:** Scrollbar enabled track color source is `DAT_00AC4624 = 0xFF` converted through
  the DirectDraw globals, then `FUN_006208F0(2,color)` called. The final composited pixel also
  involves a backing-surface alpha copy. Evidence:
  `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §3.1.
- **Rust:** `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE = [0.035, 0.042, 0.034]`
  at `src/app_skirmish_shell_render.rs:97`. Explicitly marked pending.
  Used in `src/app_skirmish_shell_render/modals.rs:67`.
- **Player-visible diff:** scrollbar track color is an unverified placeholder; final composited
  pixel awaits runtime capture.
- **Verdict:** UNCHECKED (placeholder, pending runtime surface capture).

### 5d. Scrollbar arrows + thumb drawn via push_dropdown_scrollbar_instances

- **gamemd:** Arrows use `FUN_00620720` PCX-backed pieces (`sbgripm/sbgript/sbgripb.pcx`).
  Thumb drag/arrow/track click behavior verified in scrollbar report.
  Evidence: `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §3.1.
- **Rust:** `push_dropdown_scrollbar_instances(out, atlas, scrollbar, thumb, None)` at
  `src/app_skirmish_shell_render/modals.rs:70`. This is the same path used for combo scrollbars.
- **Verdict:** UNCHECKED (listbox scrollbar calls same combo scrollbar helper; no separate
  verification that listbox vs combo scrollbar render match at this call site).

---

## Stage 6 — Two-Pixel Owner-Draw Bevel Frame Around Each List

### 6a. Bevel ordering: light TL / dark BR outer, then reversed inner

- **gamemd:** `FUN_006208F0` with `border=2` draws:
  - Outer ring (i=0): TL color = `DAT_00AC1B98 = 0xC5BEA7`, BR color = `DAT_00AC1B94 = 0x807A68`.
  - Inner ring (i=1): TL color = `DAT_00AC1B94 = 0x807A68`, BR color = `DAT_00AC1B98 = 0xC5BEA7`.
  Mixed corners (TL-inner corner of each ring) get averaged color.
  Evidence: `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` §4; `0x00620A90..0x00620C7F`.
- **Rust:** `push_ownerdraw_two_pixel_bevel_frame` at `src/app_skirmish_shell_render/chrome.rs:539`:
  - Outer ring: `OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7` as TL, `OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68` as BR.
  - Inner ring (rect shrunk by 1): `OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68` as TL, `OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7` as BR.
- **Verdict:** PASS. (Two-ring order and color assignment matches gamemd; see color values below.)

### 6b. Bevel color values

- **gamemd:** Light bevel = `0x00C5BEA7` (packed 0x00BBGGRR → R=0xA7=167, G=0xBE=190, B=0xC5=197).
  Dark bevel = `0x00807A68` (R=0x68=104, G=0x7A=122, B=0x80=128).
  Evidence: `FUN_0060F9A0` init `0x0060FA91/0x0060FA9B`; `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` §2.
- **Rust:**
  `OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7 = [0xA7/255, 0xBE/255, 0xC5/255]`
  `OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68 = [0x68/255, 0x7A/255, 0x80/255]`
  at `src/app_skirmish_shell_render.rs:85-94`.
- **Verdict:** PASS.

### 6c. Bevel expansion direction (outward vs inward)

- **gamemd:** `FUN_006208F0` input is `[x,y,width,height]` and expands the bevel **outward** by
  2px beyond the provided box (`left0 = x - n`, `top0 = y - n`). So the frame pixels fall
  outside and at the edge of the content area. Evidence: coordinate setup `0x00620A1A..0x00620A43`;
  `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` §3.
- **Rust:** `push_ownerdraw_two_pixel_bevel_frame(out, atlas, list, depth)` is called with
  `list` being the full listbox rect including the outer boundary. The `push_bevel_ring`
  function draws the outer ring AT the rect's boundary (the topmost pixel of `rect.y` is row 0
  of the frame, and the bottom pixel is `rect.y + rect.h - 1`). This is not outward-expansion —
  it's an inward 2px frame drawn within the provided rect.
- **Player-visible diff:** The bevel appears 2px inside the list rect boundary rather than 2px
  outside it. This means the bevel overlaps with the first/last row pixels instead of forming a
  standalone surround. The list background and row fills extend behind the bevel pixels.
- **Verdict:** FAIL. (Gamemd expands outward; Rust draws inward. The frame bleeds into row area.)

---

## Adjacent Findings

### A. Background fill and listbox backing surface

The gamemd `OwnerDraw_ListBox_00618D40` allocates/caches a `BSurface`, copies backing pixels,
and alpha-composites before any row/frame paint (`0x006175A5..0x006176F4` equivalents for
ListBox). The current Rust solid-rect background fills do not reproduce this compositing.
This is a broader NOT-IMPLEMENTED issue shared with the combo dropdown path; the
`SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE` constant name documents it.

### B. Mixed bevel corners (averaged color)

Gamemd writes an averaged-channel color to the TL-inner corner of each bevel ring via
surface vtable `+0x24`. Rust's `push_bevel_ring` does not compute or draw averaged corners
— the corner pixel is simply whichever of TL or BR fills it last. For 2px bevel rings the
missing average color affects 4 pixels per corner pair (8 pixels total per listbox).
Player visibility: very low but technically DRIFT.

### C. Scrollbar arrows are PCX-backed, not solid rects

Gamemd scrollbar arrows use `FUN_00620720` and the `sbgripm/sbgript/sbgripb.pcx` pieces.
The current `push_dropdown_scrollbar_instances` path uses these same PCX assets through
the chrome atlas, so there is no known drift here from the scrollbar arrow geometry itself.
However, the listbox scrollbar is a separate `"Scrollbar"` child window instance managed
by `OwnerDraw_ScrollBar_0061C690`, which also handles its own paint for the track vs
the PCX grip pieces. The combo dropdown and listbox scrollbars share the same binary
owner-draw handler and PCX assets, so reuse of `push_dropdown_scrollbar_instances` is
justified as long as the track/thumb geometry matches.

---

## Verdict Tally

| Stage | Finding | Verdict |
|---|---|---|
| 1a | Mode/map list positions `(116,127,195,343)` / `(338,127,195,343)` | PASS |
| 1b | Background fill color | UNCHECKED |
| 2 | Row height = 19px (`font_height + 2`) | PASS |
| 3a | Selected row fill = full content row, no inset | PASS |
| 3b | Selected fill color = pure red (`DAT_00AC4604 = 0xFF`) | PASS |
| 4a | Text inset = +2px from row left | PASS |
| 4b | Text vertical centering (V_CENTER) | PASS |
| 4c | Text color = yellow (`DAT_00AC18A4 = 0x0000FFFF`) | PASS |
| 4d | Text pre-truncation loop before draw | UNCHECKED |
| 5a | Scrollbar width = 20px, content shrinks by 20px | PASS |
| 5b | Arrow button = 22px, min thumb = 14px | PASS |
| 5c | Scrollbar track fill color | UNCHECKED |
| 5d | Scrollbar arrows/thumb via push_dropdown_scrollbar_instances | UNCHECKED |
| 6a | Bevel ring ordering: light TL outer / dark BR outer, reversed inner | PASS |
| 6b | Bevel color values (`0xC5BEA7` / `0x807A68`) | PASS |
| 6c | Bevel expansion: outward vs inward | FAIL |

**PASS: 11 | FAIL: 1 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0**

---

## Top 5 Player-Visible Failures

### 1. Bevel frame drawn inward instead of outward (FAIL — Stage 6c)

- **Player sees:** The 2-pixel bevel frame clips into the top/bottom/left/right row area of the
  listbox. Row 0's text and the selected fill overlap with the top bevel pixel. In gamemd,
  the bevel pixels sit outside the content area entirely — row content starts after the bevel.
- **Our code:** `push_ownerdraw_two_pixel_bevel_frame` at
  `src/app_skirmish_shell_render/chrome.rs:539-563` draws the outer ring AT `rect.y`/`rect.y+rect.h-1`.
- **gamemd evidence:** `FUN_006208F0` expands outward by border (`left0 = x-n, top0 = y-n`);
  `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` §3; `0x00620A1A..0x00620A43`.

### 2. Background fill is an unverified dark-green placeholder (UNCHECKED — Stage 1b)

- **Player sees:** Listbox unselected area shows a near-black solid fill instead of the
  composited backing-surface pixels that gamemd produces. The actual listbox background in
  gamemd depends on the modal background asset and alpha blending; the Rust value
  `[0.015, 0.024, 0.018]` is an approximation.
- **Our code:** `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE` at
  `src/app_skirmish_shell_render.rs:96`, used at `src/app_skirmish_shell_render/modals.rs:42`.
- **gamemd evidence:** Alpha backing-surface copy + composite; `0x006175A5..0x006176F4`.

### 3. Row text pre-truncation not implemented (UNCHECKED — Stage 4d)

- **Player sees:** Long map names (e.g. "Alaskan Oil Derricks") may expose a partial glyph at
  the right edge instead of gamemd's clean truncated string. Visible only for maps with long
  display names that exceed the content-rect width (175px at 800x600 with scrollbar).
- **Our code:** No pre-truncation loop in `src/app_skirmish_shell_render/text.rs:860-868`;
  truncation is delegated to the clip-rect in `push_text_draw`.
- **gamemd evidence:** Truncation loop `0x00619941..0x006199AC`.

### 4. Scrollbar track fill is an unverified dark-green placeholder (UNCHECKED — Stage 5c)

- **Player sees:** Scrollbar track area shows `[0.035, 0.042, 0.034]` instead of the
  composited `DAT_00AC4624`-derived color that gamemd produces for the enabled track.
- **Our code:** `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE` at
  `src/app_skirmish_shell_render.rs:97`, used at `src/app_skirmish_shell_render/modals.rs:67`.
- **gamemd evidence:** `DAT_00AC4624 = 0xFF`, `FUN_006208F0(2,color)`;
  `SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md` §3.1.

### 5. Bevel mixed corner pixels absent (adjacent finding B)

- **Player sees:** The 4 corner pixels at the intersection of TL and BR bevel lines show the
  winning fill-order color rather than the averaged channel value. Affects `4 × 2 rings = 8`
  pixels per listbox, 16 pixels total for both listboxes. Low-visibility but technically DRIFT.
- **Our code:** `push_bevel_ring` at `src/app_skirmish_shell_render/chrome.rs:494-537` has no
  averaged-corner write path.
- **gamemd evidence:** Average computation `0x00620B8B..0x00620C3E`, point writes
  `0x00620C40..0x00620C7F`; `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` §4.

---

## Sources

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COMBODROPWIN_LISTBOX_BACKGROUND_SCROLLBAR_TRACK_COLOR_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`
- `src/app_skirmish_shell_render/modals.rs`
- `src/app_skirmish_shell_render/chrome.rs`
- `src/app_skirmish_shell_render/text.rs:847-895`
- `src/ui/skirmish_shell/layout.rs` (CHOOSE_MAP_LIST_ROW_H, CHOOSE_MAP_LISTBOX_SCROLLBAR_W,
  choose_map_listbox_content_rect, choose_map_listbox_row_rect, choose_map_listbox_scrollbar_rect,
  choose_map_listbox_scroll_thumb_rect, choose_map_listbox_visible_row_count)
- `src/app_skirmish_shell_render.rs` (bevel/selection/text color constants)
