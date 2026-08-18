# Choose Map Modal (0x6B) — Static Text Labels Visual Trace

**Scenario:** Modal open at 800×600; dialog resource `0x6B` fullscreen shell.  
**Scope:** Title `0x694`, `GUI:SelectEngagement`, `GUI:GameType`, `GUI:GameMap`, status strip `0x695` — string source, font, color, alignment, and rect only.  
**Evidence sources:** `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_TEXT_COLOR_SOURCE_GHIDRA_REPORT.md`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.  
**Active in YR:** Yes for all stages. `FUN_006ACEE0 @ 0x006AD947` → `FUN_005E68A0` → dialog `0x6B`.

---

## Stage 1: Title 0x694 — FAIL (alignment)

| Dimension | gamemd | Our code | Match |
|---|---|---|---|
| String key | `GUI:ChooseMap` (resource static title) | `"GUI:ChooseMap"` | PASS |
| CSF lookup | `FUN_0060F9A0` snapshots resource title via `WM_GETTEXT`, translates via `StringTable__LoadString` | `localized_label(state, "GUI:ChooseMap", "Choose Map")` via `state.csf.get(key)` | PASS |
| Rect at 800×600 | `(635,3,162,16)` — DLU `(425,1,108,10)` → base px `(638,2,162,16)` → `0x0060B1D0` right-anchor + `0x0060B950` +1y → `(635,3,162,16)` | `right_anchor(..., title_base).translate(0,1)` → `(635,3,162,16)` | PASS |
| Color | `DAT_00AC18A4 = 0x0000FFFF` = RGB(255,255,0) yellow. Evidence: `FUN_0060F9A0` sets `[0x00AC18A4] = 0xFFFF`; `OwnerDraw_Static_006153E0` at `WM_PAINT` reads `piVar11[0x3B]` initialized to yellow. | `SHELL_LABEL_TEXT_RGB = [1.0, 1.0, 0.0]` | PASS |
| Alignment | Style `0x50020001`, low bits `0x01` = `SS_CENTER` → `OwnerDraw_Static_006153E0` passes align `0x11` (H-center, no V-center) to `FUN_00621040`. Flag `0x04` (V-center) is NOT set by style low bits `0x01`; the text is **top-anchored**. Evidence: `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md` §4: "static label flags `0x10/0x11/0x12` do not set bit `0x04`, so these labels are top-anchored, not vertically centered." | `ShellAlign::H_CENTER \| ShellAlign::V_CENTER` — adds V_CENTER which gamemd does not set. File: `src/app_skirmish_shell_render/text.rs:779` | **FAIL** |
| Font | `g_GAME_FNT` / `GAME.FNT`, cell_height 17, via `FUN_00621040` → `FUN_00434CD0`. Evidence: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`. | Same single global BitFont / `GAME.FNT` | PASS |

**Player-visible effect:** At 16px rect height and GAME.FNT cell_height 17, V_CENTER would try to offset by `(16-17)/2 = -0.5` → 0px (clamped). In practice the strip is too tight for visible shift at 800×600. **FAIL on principle** (wrong alignment flag), but observable shift is 0px for this rect height. Still a verifiable parity miss.

---

## Stage 2: Select Engagement Static — FAIL (alignment)

| Dimension | gamemd | Our code | Match |
|---|---|---|---|
| String key | `GUI:SelectEngagement` (resource static control title `-1`) | `"GUI:SelectEngagement"` | PASS |
| Rect at 800×600 | `(120,33,386,20)` — DLU `(80,20,257,12)` → base px `(120,33,386,20)` → fallback preserve (id `-1`, not in right-anchor allowlist). Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` table row "fallback preserve-current rect". | `dlu_rect(80, 20, 257, 12)` → `(120,33,386,20)` | PASS |
| Color | Yellow `0xFFFF` / RGB(255,255,0). Same `DAT_00AC18A4` path. | `SHELL_LABEL_TEXT_RGB` yellow | PASS |
| Alignment | Style `0x50000201`, low bits `0x01` = `SS_CENTER` → align `0x11` (H-center, top-anchored). No `0x04`. | `ShellAlign::H_CENTER \| ShellAlign::V_CENTER`. File: `src/app_skirmish_shell_render/text.rs:785` | **FAIL** |
| Font | `GAME.FNT` via `FUN_00621040` | Same | PASS |

**Player-visible effect:** Rect h=20, GAME.FNT height 17. V_CENTER offset = `(20-17)/2 = 1px`. Text draws 1px lower than gamemd. Visible on close inspection but subtle.

---

## Stage 3: Game Type Heading — FAIL (alignment)

| Dimension | gamemd | Our code | Match |
|---|---|---|---|
| String key | `GUI:GameType` (resource static, id `-1`) | `"GUI:GameType"` | PASS |
| Rect at 800×600 | `(116,98,195,16)` — DLU `(77,60,130,10)` → base px `(116,98,195,16)` → fallback preserve. Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`. | `dlu_rect(77, 60, 130, 10)` → `(116,98,195,16)` | PASS |
| Color | Yellow `DAT_00AC18A4` | `SHELL_LABEL_TEXT_RGB` yellow | PASS |
| Alignment | Style `0x50000201`, low bits `0x01` → H-center, top-anchored, no V-center. | `ShellAlign::H_CENTER \| ShellAlign::V_CENTER`. File: `src/app_skirmish_shell_render/text.rs:791` | **FAIL** |
| Font | `GAME.FNT` | Same | PASS |

**Player-visible effect:** h=16, GAME.FNT height 17. V_CENTER offset = `(16-17)/2 = 0` (saturated). 0px shift at this size. FAIL on principle but no observable pixel shift.

---

## Stage 4: Game Map Heading — FAIL (alignment)

| Dimension | gamemd | Our code | Match |
|---|---|---|---|
| String key | `GUI:GameMap` (resource static, id `-1`) | `"GUI:GameMap"` | PASS |
| Rect at 800×600 | `(338,98,195,16)` — DLU `(225,60,130,10)` → base px `(338,98,195,16)` → fallback preserve. Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`. | `dlu_rect(225, 60, 130, 10)` → `(338,98,195,16)` | PASS |
| Color | Yellow `DAT_00AC18A4` | `SHELL_LABEL_TEXT_RGB` yellow | PASS |
| Alignment | Style `0x50000201`, low bits `0x01` → H-center, top-anchored, no V-center. | `ShellAlign::H_CENTER \| ShellAlign::V_CENTER`. File: `src/app_skirmish_shell_render/text.rs:797` | **FAIL** |
| Font | `GAME.FNT` | Same | PASS |

**Player-visible effect:** h=16, same 0px shift as GameType heading. FAIL on principle.

---

## Stage 5: Status Strip 0x695 — FAIL (alignment)

| Dimension | gamemd | Our code | Match |
|---|---|---|---|
| Rect at 800×600 | `(10,579,455,20)` — `0x0060B550` bottom-left helper: `x = center_offset_x + 10 = 0+10 = 10`, `y = screen_h - ctrl_h - center_offset_y - 1 = 600-20-0-1 = 579`, w=455, h=20. Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`. | `choose_map_status_help_rect(800,600)` → `(10,579,455,20)` | PASS |
| Blank by default | Yes — no permanent text; `0x00887734` empty string used when no hover resolver produces text. `status_help_text` starts empty in `SkirmishShellState`. Evidence: `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md` §3.5. | `choose_map_modal_status_help_text` returns `None` when empty; `push_label_draw` not called. | PASS |
| String source | STT:* keys via `FUN_006040B0` on hover. Static: blank until hover event. | `state.skirmish_shell_state.status_help_text` | PASS |
| Color | `DAT_00AC18A4` yellow — same `0x4B2` path writes to `0x695` via `SendMessageA(status, 0x4B2, 0, text)`. Evidence: thunk `0x00611E75..0x00611E8B`. | `SHELL_LABEL_TEXT_RGB` yellow via `push_label_draw` | PASS |
| Alignment | Style `0x50000200`, low bits `0x00` = `SS_LEFT` → align `0x10` (left, top-anchored). No H-center, no V-center. Evidence: resource table in `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`; same `OwnerDraw_Static_006153E0` style-bit path. | `push_label_draw` uses `ShellAlign::V_CENTER` (left + v-center). File: `src/app_skirmish_shell_render/text.rs:338..353`. | **FAIL** |
| Font | `GAME.FNT` | Same | PASS |

**Player-visible effect:** h=20, GAME.FNT height 17. V_CENTER offset = `(20-17)/2 = 1px`. Status text draws 1px lower than native. Same as SelectEngagement: subtle but measurable.

---

## Stage 6: Text Color — PASS

gamemd global `DAT_00AC18A4` is initialized by `FUN_0060F9A0` to `0x0000FFFF` = source RGB(255,255,0). `FUN_00621040` decodes: low byte → red (0xFF=255), next byte → green (0xFF=255), third byte → blue (0x00=0) → yellow. Evidence: `SKIRMISH_OWNERDRAW_BUTTON_TEXT_COLOR_SOURCE_GHIDRA_REPORT.md` §3 and §4.

Our `SHELL_LABEL_TEXT_RGB = [1.0, 1.0, 0.0]` = RGB(255,255,0). **PASS.**

---

## Stage 7: Font — PASS

gamemd uses `g_GAME_FNT @ 0x0089C4D0` holding `GAME.FNT` (cell_height=17). All `FUN_00621040` shell text calls, including every owner-draw static on `0x6B`, route through this single global font. Evidence: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` §1: "One global instance lives at `g_GAME_FNT @ 0x0089C4D0`, holding `GAME.FNT`." Our code uses the same single BitFont / `GAME.FNT`. **PASS.**

---

## Summary Table

| Stage | Verdict | Key delta |
|---|---|---|
| Title 0x694 string + rect | PASS | — |
| Title 0x694 color | PASS | — |
| Title 0x694 alignment | FAIL | V_CENTER spuriously added; gamemd top-anchors |
| SelectEngagement string + rect | PASS | — |
| SelectEngagement color | PASS | — |
| SelectEngagement alignment | FAIL | V_CENTER adds 1px Y shift (h=20, font=17) |
| GameType heading string + rect | PASS | — |
| GameType heading alignment | FAIL | V_CENTER; 0px shift (h=16, font=17), principle miss |
| GameMap heading string + rect | PASS | — |
| GameMap heading alignment | FAIL | V_CENTER; 0px shift (h=16, font=17), principle miss |
| Status 0x695 rect | PASS | — |
| Status 0x695 blank default | PASS | — |
| Status 0x695 color | PASS | — |
| Status 0x695 alignment | FAIL | V_CENTER adds 1px Y shift (h=20, font=17); gamemd uses SS_LEFT top-anchor |
| Text color global | PASS | — |
| Font (GAME.FNT) | PASS | — |

---

## Root Cause

All FAIL stages share one cause: `push_choose_map_modal_text_draws` hardcodes `ShellAlign::H_CENTER | ShellAlign::V_CENTER` for the four static labels, and `push_label_draw` hardcodes `ShellAlign::V_CENTER` for the status text. The native `OwnerDraw_Static_006153E0` paint derives alignment from the Win32 style low bits: bit `0x01` = H-center, bit `0x02` = right-align, no bit → left. Vertical center (bit `0x04`) is a caller-side flag not present in the style low bits for these statics.

Fix: use `ShellAlign::H_CENTER` alone for the four labeled statics and `ShellAlign::NONE` for the status strip text.

---

## Adjacent Finding (not in scope — flagged only)

The status strip hover resolver is unimplemented: `handle_skirmish_shell_mouse_move` returns early when `choose_map_modal` is open; no `STT:Scenario*` keys are looked up. This is a separate behavior gap covered by `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md` §6. Not a static-text-label parity issue but player-visible on mouse hover.

---

## Sources

- `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md` — resource template, control styles, string keys.
- `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` — final pixel rects at 800×600 and helper routing.
- `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md` — status strip behavior, blank default, `0x4B2` write.
- `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md` — `OwnerDraw_Static_006153E0` alignment flag derivation from style low bits; top-anchor finding.
- `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md` — `FUN_00621040` flag `0x04` = V-center, shared wrapper contract.
- `SKIRMISH_OWNERDRAW_BUTTON_TEXT_COLOR_SOURCE_GHIDRA_REPORT.md` — `DAT_00AC18A4 = 0xFFFF` = yellow source color.
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` — `g_GAME_FNT`, `GAME.FNT`, cell_height 17.
- `src/app_skirmish_shell_render/text.rs` lines 765–845 (Rust implementation).
- `src/ui/skirmish_shell/layout.rs` lines 552–584 (rect computation).
