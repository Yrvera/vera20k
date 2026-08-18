# Skirmish Shell Text Pixel Contract Hotfix Scan - Ghidra Research Report

**Address(es):** `FUN_00621040 @ 0x00621040`, button call `0x006135EE`, combo call `0x00617C04`, dropdown row call `0x0060DFC8`, checkbox call `0x00616674`, static call `0x00615AE8`, preview marker text call `0x00640A15`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active standard YR offline Skirmish dialog `0x102` caller-visible text pixel contract for button labels, collapsed combo text, open dropdown row text, checkbox labels, static/right-panel labels including the map name label, and pressed/disabled/selected text color/rect offsets.
**Non-Scope:** full glyph raster internals beyond caller contract, parent/background composition, Choose Map modal `0x6B`, and runtime screenshot capture.
**Confidence:** High for rects, flags, active call sites, and packed RGB channel order; Medium for final retail screenshot appearance because DirectDraw display format and capture were not sampled live.
**Active in YR:** Yes for all scoped `0x102` owner-draw text callers; map preview numeric labels are Conditional on live start-position overlay eligibility.

## Working Notes

Target question: What exact caller-visible text rect/color/offset contract does active standard YR offline Skirmish dialog `0x102` use, and where does current Rust diverge?
Non-goals: Glyph raster internals, parent/background composition, and Choose Map modal except if text caller contract differs.
Evidence needed to mark COMPLETE: Prior-doc cross-check plus Ghidra decompile and assembly/xref evidence for button, combo/dropdown, checkbox/static/map-label text paths, with Rust surface comparison.
Stop conditions: All open questions resolved or explicitly deferred; zero-add review pass over primary verified functions; exactly one report written at this path.

## 1. Overview

Standard offline Skirmish shell text is mostly routed through `FUN_00621040`, the shell owner-draw wrapper around `GAME.FNT`. The wrapper treats the caller `RECT` as both layout bounds and clip bounds, applies vertical centering only when flag bit `0x04` is set, converts caller packed RGB as `0x00BBGGRR`, then calls the BitFont wrap/draw core.

The hotfix-relevant result was the packed-color ordering: a source value
`0x00000C05` means RGB `(5, 12, 0)`, not `(0, 12, 5)`. As of
`SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`, current Rust
constants now decode packed source colors in the verified low-byte-red order.
Normal owner-draw label color `DAT_00AC18A4 = 0x0000FFFF` means RGB
`(255, 255, 0)`.

## 2. Verified Binary Findings

| Area | Verified behavior | Active in YR | Evidence |
|---|---|---|---|
| `FUN_00621040` rect contract | `right-left` and `bottom-top` become max width/height; the same rect is installed as the BitFont clip rect. | Yes | decompile `0x00621040`; assembly `0x006210A1..0x006210B6`, `0x006210FA..0x00621112` |
| `FUN_00621040` vertical center | Only `flags & 0x04` calls `BitFont__MeasureText` and adds `(rect_h - measured_h) / 2` to Y. | Yes | decompile `0x00621040`; assembly `0x006210C8..0x006210F2` |
| Packed color channel order | Low byte is red, next byte is green, third byte is blue before DirectDraw loss/shift packing. | Yes | decompile `0x00621040`; assembly `0x00621054..0x006210B1` extracts `AH`, `>>16`, and low byte separately |
| Normal owner-draw shell label color | `FUN_0060F9A0` initializes `DAT_00AC18A4 = 0xFFFF`, which the wrapper interprets as RGB `(255,255,0)`. | Yes | decompile `0x0060F9A0`; static/checkbox/combo callers read `DAT_00AC18A4` |
| Disabled label color | Disabled style switches text to `DAT_00AC1CB4`; final display value still goes through wrapper conversion. | Conditional: only `WS_DISABLED` controls | checkbox assembly `0x00616651..0x00616655`; static decompile `0x006153E0`; combo grey/disabled docs |

## 3. Caller Pixel Contract

### Buttons: Start Game `0x617`, Choose Map `0x5AA`, Back `0x5C0`

Active in YR: Yes. `FUN_0060F9A0` maps Button style low bits `(style & 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70`; dialog `0x102` resource/layout docs identify these controls on the standard Skirmish path.

Verified behavior: released text rect is `left = button_x`, `top = button_y + 1`, `right = button_x + width - 2`, `bottom = button_y + height`. Pressed text shifts only `left += 2` and `top += 5`; `right/bottom` stay unchanged. Flags at the wrapper are `0x05`, meaning horizontal center plus vertical center in the verified `FUN_00621040`/`FUN_00434CD0` flag model. Assembly evidence: rect setup `0x00613591..0x006135CD`; call sequence `0x006135D4..0x006135EE` pushes `0x5` immediately before the color/text/rect stack arguments for `FUN_00621040`. `PUSH 0x0C` is the adjacent wrapper parameter, not the text alignment flag.

Current Rust: `src/app_skirmish_shell_render.rs` now has `button_text_rect(rect, pressed)` with `top+1`, `right-2`, and pressed `left+2/top+5`; this rect delta appears fixed. Remaining risk is the packed RGB constant channel order for `SHELL_BUTTON_TEXT_RGB_00000C05`.

### Collapsed Combo Text

Active in YR: Yes. `FUN_0060F9A0` maps `"ComboBox"` controls to `OwnerDraw_ComboBox_00617250`; `FUN_006AE6E0` initializes the standard offline Skirmish combos.

Verified behavior: selected text is copied into `DAT_00AC18F8` and caller-truncated until `BitFont__GetTextWidth <= client_width - 0x14`. The draw rect starts at `left + 2`, uses the combo client top/bottom, and passes wrapper flags `0x04` for vertical center only. Assembly evidence: truncation `0x00617B42..0x00617BAF`; call sequence `0x00617BAF..0x00617C04` pushes `0x4` for flags. Color is normal `DAT_00AC18A4` unless grey/disabled/swatch state replaces it.

Current Rust: `combo_text_rect` uses `x + 2`, height `24`, and a `20px` arrow reserve; `push_label_draw` uses `ShellAlign::V_CENTER`. This is visibly aligned with the caller contract. Rust clips the collapsed text to the arrow-reserved width, while the binary separately truncates to `width-20` and passes a wider client rect; that is probably pixel-equivalent for ordinary strings but should be pinned by a test.

2026-05-27 narrow recheck: `OwnerDraw_ComboBox_00617250` decompile confirms the collapsed selected-text fit limit is `client_width - 0x14`, but the `FUN_00621040` rect uses `left = client_left + 2`, `top = client_top`, `right = client_right`, and `bottom = client_bottom` before the call at `0x00617C04`. Rust should keep the `width - 20` pre-truncation budget separate from the wider draw/scissor rect.

2026-05-27 UTF-16 truncation recheck: the collapsed selected-text loop trims one UTF-16 code unit per failed fit pass. Assembly evidence: `0x00617B75` computes the end pointer as `DAT_00AC18F8 + length*2`; on overflow `0x00617B82` subtracts `2`, `0x00617B8F` decrements the length, and `0x00617B90` writes a zero word at the new end before remeasuring through `BitFont__GetTextWidth`.

### `ComboDropWin` Dropdown Row Text

Active in YR: Yes. `FUN_0060D450` registers class `"ComboDropWin"` with WndProc label `LAB_0060D540`; `OwnerDraw_ComboBox_00617250` creates it on `CB_SHOWDROPDOWN`.

Verified behavior: row height comes from source combo `CB_GETITEMHEIGHT 0x154`; row Y is `(item_index - top_index) * row_height`; text rect is `left + 3`, row top, current row/client right, row bottom. Text is caller-truncated to current client width minus `0x14`, so rows do not draw under a scrollbar after client shrink. Flags are `0x04`, vertical center only. Assembly evidence: row rect setup `0x0060DE1F..0x0060DE47`; truncation `0x0060DF2D..0x0060DFA1`; call sequence `0x0060DFAD..0x0060DFC8` pushes `0x4`.

2026-05-27 UTF-16 truncation recheck: the dropdown row loop uses the same one-code-unit deletion mechanism. Assembly evidence: `0x0060DF63` computes the row string end pointer as `buffer + length*2`, then each overflow pass executes `0x0060DF70 SUB EDI,0x2`, `0x0060DF80 DEC ESI`, and `0x0060DF81 MOV word ptr [EDI],0x0` before remeasuring.

Current Rust: dropdown row text uses `content.x + 3`, `COMBO_DROPDOWN_ROW_H`, `combo_dropdown_content_rect` after scrollbar shrink, and `ShellAlign::V_CENTER`. This matches the hotfix contract; color-selected/grey final RGB still needs screenshot validation.

### Checkbox Labels

Active in YR: Yes. `FUN_0060F9A0` maps Button style low bits `(style & 3) == 3` to `OwnerDraw_Checkbox_006163A0`; `FUN_006AE6E0` initializes the five standard offline Skirmish option checkboxes.

Verified behavior: icon blit is fixed `18x18` at the control top-left. Label rect is the full control rect with `left += 0x1A` (`26px`). Flags are `0x04`, vertical center only. Disabled style switches text color from `DAT_00AC18A4` to `DAT_00AC1CB4`. Assembly evidence: left advance and color gate `0x0061663E..0x0061665A`; call sequence `0x0061665D..0x00616674` pushes `0x4`.

Current Rust: `checkbox_text_rect` uses `x + 26`; label draw uses `ShellAlign::V_CENTER`; hit tests already keep label clicks from toggling. Main remaining text risk is packed color accuracy.

### Static / Right-Panel Labels and Map Name Label

Active in YR: Yes. `FUN_0060F9A0` maps `"Static"` controls to `OwnerDraw_Static_006153E0`; prior static-control reports verify Skirmish `0x102` IDs `0x694`, `0x6EC`, and `0x5A8` become kind-1 text animation labels. Map preview static `0x468` is a no-text anchor, not the map-label renderer.

Verified behavior: static text alignment comes from style low bits: default `0x10`, centered `0x11`, right `0x12`. These flags do not include bit `0x04`, so static labels are top-anchored, not vertically centered. Text still wraps/clips through `FUN_00621040` using the control rect. Disabled style switches to `DAT_00AC1CB4`. Assembly evidence: static wrapper call at `0x00615AE8`; decompile `0x006153E0` shows style-to-flag selection and call `FUN_00621040(..., color, uVar10, piVar11[0x2B], 0, reveal_count, reveal_range)`.

Current Rust: `push_static_label_draw` passes `ShellAlign::H_CENTER` only for title/game-type/map label, so top anchoring matches. The visual mismatch risk is color (`SHELL_LABEL_TEXT_RGB` should represent packed `0x0000FFFF` unless a runtime screenshot proves a different perceptual target) and missing animated reveal timing, not vertical centering.

### Map Preview Numeric Labels

Active in YR: Conditional. `DrawStartPositions @ 0x00640710` draws numeric labels only when the selected scenario has live start-position overlay data in the `ScenarioClass+0x113C` range `1..8`.

Verified behavior: preview numbers do not use `FUN_00621040`. After marker positioning, the caller pushes the destination-surface clip from `DAT_00887310` vtable `+0x78`, a format string at `DAT_0081B3D0`, and calls `FUN_004A61C0` at `0x00640A15`. Existing text-contract docs record marker-label offset as `x - 2`, `y - 6`.

Current Rust: `push_start_marker_labels` draws only if the projected point is inside `preview_rect`, uses `build_text` directly, and does not apply the `(-2,-6)` numeric offset or destination-surface clipping contract. This is visible only on maps where live start overlays are eligible.

## 4. Current Rust Implementation Status

| Surface | Status vs verified contract |
|---|---|
| `src/render/shell_text.rs::draw_in_rect` | Implements wrapper-level rect scissor, v-center, h-center/right, and wrapping; suitable for `FUN_00621040` caller contract. |
| `src/app_skirmish_shell_render.rs::button_text_rect` | Matches verified button released/pressed rect offsets. |
| `src/app_skirmish_shell_render.rs::SHELL_BUTTON_TEXT_RGB_00000C05` | Likely channel-swapped. Verified wrapper treats `0x00000C05` as RGB `(5,12,0)`, while Rust uses `(0,12,5)`. |
| `src/app_skirmish_shell_render.rs::SHELL_LABEL_TEXT_RGB` | Does not encode verified normal packed color `0x0000FFFF` as RGB `(255,255,0)`; current muted label color may be a visual compromise but is not the binary caller color. |
| `combo_text_rect` / dropdown row text | Geometry mostly matches; collapsed combo clips to arrow-reserved width instead of binary's wider rect plus caller truncation. |
| `checkbox_text_rect` | Matches `left + 26` and v-center contract. |
| `push_static_label_draw` | Correctly does not vertical-center right-panel static labels. Color/reveal timing remain parity risks. |
| `push_start_marker_labels` | Does not match preview numeric label helper, offset, clip, or live-overlay gate. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00621040` rect/flags/color wrapper | verified | decompile `0x00621040`; assembly `0x00621054..0x006210F2` | runtime screenshot for final perceived color |
| button label rect/flags | verified | decompile `OwnerDraw_Button_00612B70`; assembly `0x00613591..0x006135EE` | exact normal source color source for EDI can be re-read if broader color audit is needed |
| collapsed combo selected text | verified | decompile `OwnerDraw_ComboBox_00617250`; assembly `0x00617B42..0x00617C04` | screenshot-level color |
| dropdown row text | verified | assembly `0x0060DE1F..0x0060DFC8`; prior dropdown report | full popup wndproc function boundary remains a label, but row text call is verified |
| checkbox label text | verified | assembly `0x0061663E..0x00616674`; checkbox geometry report | none |
| static/right-panel/map label text | verified | decompile `0x006153E0`; assembly `0x00615AE8` | dynamic text-copy thunk internals remain separate scope |
| preview numeric labels | touched-not-exhausted | assembly `0x00640A15`; prior text contract report | `FUN_004A61C0` internals out of scope |
| full glyph raster/fallback/localized glyph coverage | deferred | scope boundary; `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` | separate glyph parity pass if needed |

## 6. Open Questions - Final State

- `[RESOLVED] OQ1 - Which active Skirmish text callers use FUN_00621040? -> buttons, collapsed combos, dropdown rows, checkboxes, and static labels; preview numeric labels do not.` (evidence: call sites `0x006135EE`, `0x00617C04`, `0x0060DFC8`, `0x00616674`, `0x00615AE8`, `0x00640A15`)
- `[RESOLVED] OQ2 - Does flag 0x04 mean vertical center for scoped callers? -> Yes; wrapper checks `flags & 4`, measures, and offsets Y before drawing.` (evidence: `0x006210C8..0x006210F2`)
- `[RESOLVED] OQ3 - Are button flags 0x0C or 0x05? -> The text alignment flags are 0x05; adjacent pushed 0x0C is a different wrapper parameter.` (evidence: assembly `0x006135D4..0x006135EE`; wrapper signature from `0x00621040`)
- `[RESOLVED] OQ4 - Are static right-panel labels vertically centered? -> No; style-derived flags are 0x10/0x11/0x12 and do not include bit 0x04.` (evidence: decompile `0x006153E0`; assembly call `0x00615AE8`)
- `[RESOLVED] OQ5 - What channel order does packed RGB use? -> low byte red, middle byte green, high byte blue.` (evidence: decompile `0x00621040`; assembly `0x00621054..0x006210B1`)
- `[RESOLVED] OQ6 - Does checkbox label text start at icon width or another offset? -> It starts at control-left + 0x1A, not 18 or 20.` (evidence: `0x0061663E..0x00616644`)
- `[RESOLVED] OQ7 - Does dropdown row text account for scrollbar width? -> Yes, through current client/content width before text rect/truncation.` (evidence: `0x0060DE1F..0x0060DFC8`; dropdown row report)
- `[DEFERRED] OQ8 - Exact final screenshot RGB after DirectDraw format conversion.` (category: needs-runtime-debugger; reason: static binary proves source color and pack order, but final perceived pixels should be sampled from retail)
- `[DEFERRED] OQ9 - Full `FUN_004A61C0` contract for preview numbers.` (category: out-of-scope; reason: this scan only needed caller distinction and offset/clip handoff)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Packed shell RGB is `0x00BBGGRR`; `0x00000C05` decodes to RGB `(5,12,0)` | `FUN_00621040` decompile; assembly `0x00621054..0x006210B1` | mismatch: Rust uses `[0,12,5]` | `src/app_skirmish_shell_render.rs` text color constants | Reorder packed RGB constants to red/green/blue before creating float RGB | Start/Choose/Back and trackbar value text use the same dark olive tone as retail source color | Do not treat packed constants as `0x00RRGGBB` |
| Normal owner-draw label source color is `DAT_00AC18A4 = 0x0000FFFF`, i.e. RGB `(255,255,0)` before display packing | `FUN_0060F9A0` init; callers read `DAT_00AC18A4`; wrapper channel order | mismatch/compromise: `SHELL_LABEL_TEXT_RGB` is muted tan | `src/app_skirmish_shell_render.rs` label color constants | Use verified source RGB or gate any perceptual override behind screenshot evidence | Column labels, checkbox labels, combo text, dropdown rows, title/map label render with consistent shell yellow | Do not tune label color by eye before one retail screenshot comparison |
| Button text rect is released `top+1/right-2`, pressed `left+2/top+5`, flags `0x05` | assembly `0x00613591..0x006135EE` | none observed for rect; tests should lock it | `button_text_rect`, `push_button_label_draw` | Keep current rect and center+vcenter flags | Press Start: text moves right/down exactly while clip remains within inset rect | Do not regress to full-button-rect centering |
| Static right-panel labels are top-anchored h-center, not v-centered | decompile `0x006153E0`; call `0x00615AE8` | none observed for vertical alignment; color/reveal still risk | `push_static_label_draw`, right-panel layout | Preserve no `V_CENTER` for title/game-type/map label; add reveal timing only if needed separately | Long map name wraps/clips from the top of `0x5A8` rect, not centered in the 33px box | Do not use `push_label_draw` for static right-panel labels |
| Preview numeric labels use `FUN_004A61C0`, destination-surface clip, and marker offset `(-2,-6)` | assembly `0x00640A15`; prior text contract report | mismatch/partial | `push_start_marker_labels`, preview overlay path | Apply live-overlay gate, offset, and destination-surface clipping when enabling numeric labels | Live-overlay-eligible maps show retail-aligned numbers; loose maps without live starts skip labels | Do not infer numbers from baked PreviewPack pixels |

Proposed Rust test names:

- `packed_shell_rgb_00000c05_decodes_as_red_green_blue`
- `skirmish_button_text_rect_keeps_verified_pressed_offsets`
- `static_right_panel_labels_are_top_anchored_not_vcentered`
- `collapsed_combo_text_uses_left_inset_and_arrow_fit_limit`
- `preview_start_marker_labels_use_live_overlay_offset_and_clip`

## 8. Negative Facts / Do Not Do

- Do not document button text flags as `0x0C`; active wrapper flags are `0x05`. Evidence: assembly `0x006135DD..0x006135E8` plus `FUN_00621040` signature/order.
- Do not treat packed shell colors as `0x00RRGGBB`; the active wrapper extracts low byte as red. Evidence: assembly `0x00621054..0x006210B1`.
- Do not vertically center right-panel static labels or the map name label. Evidence: `OwnerDraw_Static_006153E0` uses style flags `0x10/0x11/0x12`, no `0x04`.
- Do not make checkbox label clicks toggle the checkbox while fixing label text. Evidence: checkbox click gate remains `x < 0x12 && y < 0x12` in prior checkbox report; text rect starts at `+0x1A`.
- Do not route preview start numbers through `FUN_00621040`/ordinary shell-text rects. Evidence: `DrawStartPositions` calls `FUN_004A61C0` at `0x00640A15`.

## 9. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` replacement wording: "The `FUN_00621040` text alignment flags for default Skirmish buttons are `0x05` (`h-center | v-center`). The nearby `PUSH 0x0C` is an adjacent wrapper argument and must not be interpreted as the alignment flags."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md` replacement wording for color notes: "Packed shell text colors are caller RGB in `0x00BBGGRR` byte order before DirectDraw loss/shift packing; therefore `0x00000C05` is RGB `(5,12,0)` and `DAT_00AC18A4 = 0x0000FFFF` is RGB `(255,255,0)`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md` follow-up wording: "Collapsed combo text is caller-truncated to `client_width - 0x14` and drawn with `left+2`, `V_CENTER` only; an implementation may clip to the arrow-reserved area if tests prove pixel equivalence, but the binary caller passes the ordinary client-derived rect."

## Sources

- Ghidra read-only decompile: `FUN_00621040 @ 0x00621040`.
- Ghidra read-only decompile: `OwnerDraw_Button_00612B70 @ 0x00612B70`.
- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`.
- Ghidra read-only decompile: `OwnerDraw_Static_006153E0 @ 0x006153E0`.
- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`.
- Ghidra read-only decompile: `FUN_0060D450 @ 0x0060D450`.
- Ghidra assembly contexts: `0x006135D4..0x006135EE`, `0x00617BAF..0x00617C04`, `0x0060DE1F..0x0060DFC8`, `0x0061663E..0x00616674`, `0x00615AE8`, `0x00640A15`, `0x00621054..0x006210F2`.
- Prior docs referenced: `C:/Users/enok/Documents/ra2-rust-game-docs/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`.
- Rust read-only scan: `C:/Users/enok/Documents/ra2-rust-game/src/render/shell_text.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`.
