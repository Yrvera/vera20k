# Skirmish 0x102 Static Text Rects / Colors - Ghidra Research Report

**Address(es):** `FUN_00621040 @ 0x00621040`, `OwnerDraw_Static_006153E0 @ 0x006153E0`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin` row call `0x0060DFC8`, `DrawStartPositions @ 0x00640710`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Text caller rectangles, alignment flags, color sources, clip/reveal behavior, and disabled-state text behavior for the visible text surfaces in standard offline Skirmish setup dialog `0x102` that current Rust renders through `src/app_skirmish_shell_render.rs`: right-panel statics, Start/Choose/Back labels, checkbox labels, trackbar value text, collapsed combo selected text, dropdown row text, static/column labels when routed through the static/text helper, and preview marker numeric labels when the live overlay path is active.
**Non-Scope:** Full child-control final rect matrix, full dropdown input/scrollbar behavior, glyph raster internals, CSF string contents, Choose Map `0x6B`, modal listbox text, and runtime screenshot capture.
**Confidence:** High for caller rect formulas, align flags, source color globals, active call sites, and wrapper clipping; Medium for final perceived display RGB without a retail surface capture.
**Active in YR:** Yes for standard offline `0x102` owner-draw text callers; Conditional for disabled-state branches and preview marker numbers.

## Working Notes

- Target question: For every visible text surface in offline Skirmish setup `0x102` that Rust renders, what rect, alignment flags, color source, clipping/reveal behavior, and disabled-state behavior does active `gamemd.exe` use?
- Non-goals: Do not solve complete child-control geometry, dropdown behavior beyond row text, Choose Map `0x6B`, or glyph raster internals.
- Evidence needed to mark COMPLETE: Ghidra decompile plus assembly/call-site evidence for the shared text wrapper and each caller family; prior-doc cross-check for active `0x102` reachability; Rust surface comparison and handoff tests.
- Stop conditions: All scoped text caller families resolved or explicitly deferred; no Ghidra mutations; exactly this report plus `.swarm-claims.md` may be written.

## 1. Overview

Most standard Skirmish setup text reaches `FUN_00621040`, a shell text wrapper around `GAME.FNT`. The caller-supplied rect is both the layout box and the BitFont clip rect. The wrapper interprets color as packed `0x00BBGGRR`, converts it through the active DirectDraw loss/shift globals, vertically centers only when flag bit `0x04` is set, then delegates wrapping/horizontal alignment to the BitFont core.

Preview start-position numbers are the important exception. They are not `FUN_00621040` text and are drawn by `DrawStartPositions` through `FUN_004A61C0` only when the selected scenario supplies live preview start overlay fields.

## 2. Shared Wrapper Contract

| Finding | Active in YR | Evidence |
|---|---|---|
| `FUN_00621040` computes width `right-left` and height `bottom-top` from the caller rect, then installs the same rect as the BitFont clip rect. | Yes | decompile `0x00621040`; assembly `0x006210A1..0x00621112` from prior text scan; spot-check decompile reads `param_3[0..3]` and calls `FUN_00433CA0(piVar6)` |
| Color byte order is low byte red, next byte green, third byte blue. `0x00000C05` is RGB `(5,12,0)` and `0x0000FFFF` is RGB `(255,255,0)` before display packing. | Yes | decompile `0x00621040`; assembly `0x00621054..0x006210B1` extracts low byte, `AH`, and `>>16` separately |
| `flags & 0x04` triggers a measure pass and adds `(rect_h - measured_h) / 2` to Y. No `0x04` means top anchored. | Yes | decompile `0x00621040`; assembly `0x006210C8..0x006210F2` |
| Horizontal center/right are handled downstream by `FUN_00434CD0`; callers pass `0x01` for center and `0x02` for right, while `0x04` remains vertical center. | Yes | prior decompile `FUN_00434CD0 @ 0x00434CD0`; caller assembly below |
| Default shell label color is initialized to `DAT_00AC18A4 = 0x0000FFFF`; disabled text commonly switches to `DAT_00AC1CB4 = 0x9F`. | Yes / Conditional for disabled branch | `FUN_0060F9A0` decompile initializes globals; static/checkbox/trackbar assembly reads `DAT_00AC18A4` and disabled reads `DAT_00AC1CB4` |

## 3. Caller Rects, Flags, Colors

### Right-panel statics: title `0x694`, game type `0x6EC`, map label `0x5A8`

Active in YR: Yes. `FUN_0060F9A0` maps class `"Static"` to `OwnerDraw_Static_006153E0`; `FUN_00602490` returns true for parent dialog `0x102` and controls `0x694`, `0x6EC`, `0x5A8`; `FUN_0060A5B0` reclassifies them to kind `1` animated text controls.

Text rect: the transformed child client rect from `FUN_00775690` is passed directly to `FUN_00621040`; no caller inset is added. Prior rect docs give the final rects at standard sizes:

| Control | 640x480 / 800x600 final rect | Text rect | Align flags | Color | Clip / reveal |
|---|---|---|---|---|---|
| `0x694` title | `(475,3,162,16)` / `(635,3,162,16)` | same as child rect | style low bits -> `0x10/0x11/0x12`; title resource uses centered static style, so horizontal center and no v-center | normal `DAT_00AC18A4`; disabled `DAT_00AC1CB4` | rect clip; kind-1 reveal count/range |
| `0x6EC` game type | `(489,167,135,16)` / `(649,167,135,16)` | same as child rect | style-derived, centered in current Rust surface | same | rect clip; kind-1 reveal |
| `0x5A8` map label | `(489,189,135,33)` / `(649,189,135,33)` | same as child rect | style-derived, centered in current Rust surface | same | rect clip; top anchored; multi-line wraps from top |

Assembly/call evidence: `OwnerDraw_Static_006153E0` call at `0x00615AE8`. The call path pushes reveal count/range from state (`+0x80`, `+0x8C` in decompiler indexing), style-derived align, color, and the rect built from `FUN_00775690`. Disabled style `0x08000000` selects `DAT_00AC1CB4`. Decompile evidence: `0x006153E0` paint branch.

Reveal/animation: `0x4EE` starts kind-1 animation by setting running byte true, reveal count `1`, timer `0`, and invalidating. `WM_PAINT` draws kind `1` only while running byte is set, increments reveal count by the configured step (`1`) until `text_len + 1 + reveal_range`, then kills timer `0`. For these labels, `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` verifies interval `0x1E`, step `1`, reveal range `8`, and that reveal starts only through the `0x4EC -> 0x4EE` transition/text-update path; standard common first paint and `0x4ED` do not start reveal.

### Generic static/column labels and player-name note

Active in YR: Yes for Static-class labels routed through `OwnerDraw_Static_006153E0`; unknown/not claimed for edit-like player-name text if it routes through owner-draw edit instead of static.

Column labels in Rust (`Players`, `Side`, `Color`, `Start`, `Team`) use the same visual shell text helper. Binary-side static text rules are the same as above when the resource control class is Static and is not reclassified by `FUN_00602490`: kind `0`, text draws immediately, caller rect is the child rect, style low bits choose `0x10/0x11/0x12`, no vertical centering unless an unrelated caller passes bit `0x04` (the static proc does not). Exact final child rects/IDs belong to the slot-1 child matrix; this report only verifies that the text caller rect for routed statics is the child rect.

Do not apply this static rule to player-name edit text (`0x6A0`) unless a separate owner-draw edit trace proves it. Prior resize docs verify `0x6A0` has `x+1,w+1` geometry fixup, but this slot did not verify its edit text paint helper.

### Start `0x617`, Choose Map `0x5AA`, Back `0x5C0` button labels

Active in YR: Yes. `FUN_0060F9A0` maps Button style low bits `(style & 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70`; these three standard `0x102` buttons use that path.

Text rect:

- Released/enabled: `left = button_left`, `top = button_top + 1`, `right = button_left + width - 2`, `bottom = button_top + height`.
- Pressed: only `left += 2` and `top += 5`; `right/bottom` remain the released right/bottom.
- Alignment: flags `0x05` (`h-center | v-center`).
- Color: normal `DAT_00AC18A4` unless the disabled/button-art branch substitutes a disabled-derived packed color. Current text caller evidence for standard enabled buttons uses the normal color argument.
- Clip: `FUN_00621040` rect clip; no caller-side truncation.

Evidence: decompile `OwnerDraw_Button_00612B70`; assembly `0x00613591..0x006135EE`. At `0x006135DD..0x006135E8`, the call pushes `0x0C` as the adjacent wrapper/font/shadow argument and `0x05` as the actual alignment flags before `CALL 0x00621040` at `0x006135EE`.

### Checkbox labels

Active in YR: Yes for standard option checkboxes. `FUN_0060F9A0` maps Button style low bits `(style & 3) == 3` to `OwnerDraw_Checkbox_006163A0`; prior setup docs verify `0x54E`, `0x693`, `0x696`, `0x69A`, and `0x69D` are initialized in standard `0x102`.

Text rect: `left = control_left + 0x1A`, `top = control_top`, `right = control_right`, `bottom = control_bottom`. The checkbox icon is independent `18x18` at the control top-left; the `+26` label inset is not just icon width.

Alignment/color/clip: flags `0x04` (vertical center only, left anchored); normal color `DAT_00AC18A4`; disabled `WS_DISABLED` switches to `DAT_00AC1CB4`; `FUN_00621040` clips to the label rect.

Evidence: decompile `OwnerDraw_Checkbox_006163A0`; assembly `0x0061663E..0x00616674` shows the `+0x1A` left advance, disabled color gate at `0x00616651..0x00616655`, push `0x04`, and `CALL 0x00621040` at `0x00616674`.

### Trackbar value text: game speed `0x529`, credits `0x511`, unit count `0x50C`

Active in YR: Yes. `FUN_0060F9A0` routes `"msctls_trackbar32"` to `OwnerDraw_Trackbar_0061D950`; `FUN_006AE6E0` initializes all three standard `0x102` trackbars.

Text rect: if numeric display is enabled (`state[0x41] != 0`, standard after default step/display normalization), the value text uses the rightmost `0x31` pixels of the trackbar client rect:

```
left   = control_right - 0x31
top    = control_top
right  = control_right
bottom = control_bottom
```

The plaque art reserves `0x32` pixels, but the text rect is `0x31` pixels wide. For the verified 800x600 rects this yields:

| Control | Final control rect | Value text rect |
|---|---|---|
| `0x529` game speed | `(404,286,128,21)` | `(483,286,49,21)` |
| `0x511` credits | `(404,314,128,21)` | `(483,314,49,21)` |
| `0x50C` unit count | `(404,340,128,21)` after one-pixel fixup | `(483,340,49,21)` |

Alignment/color/clip: flags `0x05` (`h-center | v-center`); normal color `DAT_00AC18A4`; disabled style switches to `DAT_00AC1CB4`; `FUN_00621040` clips to the 49px rect. The text is formatted through `FUN_007CA564` with `DAT_0081B3D0` and the quantized absolute value `((min + relative) / step) * step`.

Evidence: decompile `OwnerDraw_Trackbar_0061D950` paint branch; assembly `0x0061E296..0x0061E30A` formats text, chooses normal/disabled color (`0x0061E2A5..0x0061E2B6`), pushes flags `0x05`, sets the rect at `0x0061E2E8..0x0061E2FC`, and calls `FUN_00621040` at `0x0061E30A`.

### Collapsed combo selected text

Active in YR: Yes. `FUN_0060F9A0` routes `"ComboBox"` to `OwnerDraw_ComboBox_00617250`; `FUN_006AE6E0` initializes the standard offline player/AI, side, color, start, and team combos.

Text rect: selected-item text is copied into `DAT_00AC18F8`, caller-truncated until `BitFont__GetTextWidth <= client_width - 0x14`, then drawn with `left = combo_left + 2`, top/bottom from the combo face/client area, and arrow-reserved width context. Current Rust's `combo_text_rect` uses `x+2`, `height=24`, and a 20px arrow reserve, matching the verified text fit intent.

Alignment/color/clip: flags `0x04` (vertical center only); normal color `DAT_00AC18A4`; disabled/grey/swatch branches can replace the color. Color-combo swatches may avoid ordinary label text or overwrite row color; full color-row visuals belong to the dropdown/scrollbar slot.

Evidence: decompile `OwnerDraw_ComboBox_00617250`; assembly `0x00617B42..0x00617BAF` for truncation and `0x00617BAF..0x00617C04` for rect/call, with push `0x04` before `CALL 0x00621040`.

### `ComboDropWin` dropdown row text

Active in YR: Yes when a standard `0x102` combo is open. `OwnerDraw_ComboBox_00617250` creates a `ComboDropWin`; row text call site is inside the popup window proc block.

Text rect: for each visible row, row height is `CB_GETITEMHEIGHT 0x154`; row Y is `(item_index - top_index) * row_height`; text rect starts at `row_left + 3`, row top, current row/client right, row bottom. The text is pre-truncated to current client width minus `0x14`, so rows respect scrollbar-shrunk content width.

Alignment/color/clip: flags `0x04` (vertical center only); normal color starts from `DAT_00AC18A4`; selected/grey/disabled/swatch branches may replace it before the text call; `FUN_00621040` clips to row text rect.

Evidence: assembly `0x0060DE1F..0x0060DFC8`; prior dropdown row report verifies `ComboDropWin` creation from `OwnerDraw_ComboBox_00617250` and current-client-width shrink before row text.

### Preview marker numeric labels

Active in YR: Conditional. `DrawStartPositions @ 0x00640710` draws live numeric labels only when `0 < ScenarioClass+0x113C < 9`. Loose maps without `[Header]` preview start fields, such as the verified loose Dustbowl path, skip live `STARTBUT.SHP` overlays and numeric labels even if baked red pixels exist in `[PreviewPack]`.

Text rect/helper: preview numbers do not use `FUN_00621040` and do not use the ordinary shell text rect contract. After the marker anchor is projected, the label call offsets the label position by `x - 2`, `y - 6`, pushes the destination-surface clip returned from `DAT_00887310` vtable `+0x78`, uses format string `DAT_0081B3D0`, and calls `FUN_004A61C0`.

Color/clip: the caller passes arguments including `0x19` and `0x08` to `FUN_004A61C0`; this slot did not drain that helper's full glyph/color semantics. Clipping is destination-surface clip, not the preview child rect.

Evidence: `DrawStartPositions` assembly `0x006409D7..0x00640A15`, including `CALL [EAX+0x78]` for clip at `0x00640A01` and `CALL 0x004A61C0` at `0x00640A15`; prior preview-header report verifies the live overlay gate.

## 4. Current Rust Implementation Status

| Surface | Status vs verified contract |
|---|---|
| `src/render/shell_text.rs::draw_in_rect` | Matches wrapper-level scissor-as-rect, wrapping, h-center/right, and v-center behavior closely enough for caller-contract tests. |
| `src/app_skirmish_shell_render.rs` color constants | Current scan shows `SHELL_BUTTON_TEXT_RGB_00000C05 = [5,12,0]/255` and `SHELL_LABEL_TEXT_RGB = [1,1,0]`, matching the verified source byte order and default yellow source. |
| `button_text_rect` / `push_button_label_draw` | Matches released/pressed rect offsets and flags `0x05`. |
| `push_static_label_draw` for right-panel statics | Correctly omits `V_CENTER`; reveal animation is not modeled. |
| `checkbox_text_rect` and checkbox label draw | Matches `left+26` and v-center. Disabled text color path is not obviously surfaced. |
| `trackbar_value_text_rect` and trackbar value draw | Matches rightmost `49px` rect and `h-center|v-center`; relies on current final control rects for the `0x50C y-1` fixup. |
| `combo_text_rect` and dropdown row text | Geometry mostly matches: `left+2` collapsed, `left+3` dropdown, v-center, 20px arrow/scrollbar reserve. Binary truncates before drawing; Rust clips/wraps through `draw_in_rect`, so long-string pixel equivalence should be pinned. |
| `push_start_marker_labels` | Mismatch/partial: uses `build_text` directly, checks preview-rect containment, and does not apply the binary `(-2,-6)` label offset or destination-surface clip/helper. |
| `player_name` text | Unchecked for this slice: Rust renders a player-name-ish text surface, but binary `0x6A0` routing is not proven through static/`FUN_00621040` here. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00621040` wrapper rect/flags/color | verified | decompile `0x00621040`; assembly `0x00621054..0x006210F2` | retail screenshot for final perceived RGB optional |
| Static right-panel text controls `0x694/0x6EC/0x5A8` | verified | `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0`; call `0x00615AE8`; prior final rect docs | full dynamic text-copy thunk already covered by separate static-thunk report |
| Generic Static-class column labels | touched-not-exhausted | `FUN_0060F9A0` Static route; `OwnerDraw_Static_006153E0` | exact per-control IDs/final rects are slot-1 child matrix scope |
| Button labels | verified | decompile `0x00612B70`; assembly `0x00613591..0x006135EE` | none for caller rect/color |
| Checkbox labels | verified | decompile `0x006163A0`; assembly `0x0061663E..0x00616674` | disabled runtime screenshot optional |
| Trackbar value text | verified | decompile `0x0061D950`; assembly `0x0061E296..0x0061E30A` | none for text rect/color |
| Collapsed combo selected text | verified | decompile `0x00617250`; assembly `0x00617B42..0x00617C04` | full color-row visual behavior belongs combo/dropdown slot |
| Dropdown row text | verified | assembly `0x0060DE1F..0x0060DFC8`; prior dropdown report | popup full behavior/scrollbar outside this slot |
| Preview numeric labels | touched-not-exhausted | assembly `0x006409D7..0x00640A15`; prior preview reports | full `FUN_004A61C0` color/glyph contract |
| Player-name/edit text `0x6A0` | deferred | resize docs only | separate owner-draw edit text investigation if Rust keeps rendering it |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which text callers use FUN_00621040? -> statics, buttons, checkboxes, trackbar value text, collapsed combos, and dropdown rows; preview numbers do not.` (evidence: `0x00615AE8`, `0x006135EE`, `0x00616674`, `0x0061E30A`, `0x00617C04`, `0x0060DFC8`, `0x00640A15`)
- `[RESOLVED] OQ-02 - What is the exact source color byte order? -> caller colors are `0x00BBGGRR`; low byte is red.` (evidence: `0x00621054..0x006210B1`)
- `[RESOLVED] OQ-03 - Are right-panel statics vertically centered? -> No; static style flags are `0x10/0x11/0x12`, which do not set `0x04`.` (evidence: `OwnerDraw_Static_006153E0`, call `0x00615AE8`)
- `[RESOLVED] OQ-04 - What are button label pressed offsets? -> pressed changes left by `+2` and top by `+5`; released uses top `+1` and right `-2`.` (evidence: `0x00613591..0x006135EE`)
- `[RESOLVED] OQ-05 - What is the checkbox label inset? -> `+0x1A` from control left; icon remains `18x18`.` (evidence: `0x0061663E..0x00616674`)
- `[RESOLVED] OQ-06 - What is the trackbar value text rect? -> rightmost `0x31` pixels of the control, full control height, flags `0x05`.` (evidence: `0x0061E296..0x0061E30A`)
- `[RESOLVED] OQ-07 - Does combo/dropdown text reserve arrow/scrollbar width? -> Yes; collapsed and dropdown text are caller-truncated to current width minus `0x14`, with left insets `+2` and `+3`.` (evidence: `0x00617B42..0x00617C04`, `0x0060DE1F..0x0060DFC8`)
- `[RESOLVED] OQ-08 - Are preview numeric labels ordinary shell text? -> No; they call `FUN_004A61C0` with destination-surface clip and offset `(-2,-6)`.` (evidence: `0x006409D7..0x00640A15`)
- `[DEFERRED] OQ-09 - Exact final screenshot RGB after DirectDraw/capture path.` (category: `needs-runtime-debugger`; reason: binary source colors and packing are verified, but final display/capture appearance needs retail sampling; next-step-if-pursued: capture retail 16-bit surface pixels for normal and disabled text)
- `[DEFERRED] OQ-10 - Player-name/edit control text path for `0x6A0`.` (category: `out-of-scope`; reason: target only included player labels if routed through static/text helpers, and `0x6A0` is an edit/control-specific path; next-step-if-pursued: trace owner-draw edit `0x00614190` / `OwnerDraw_NewEdit_00614B30` for `0x102` control `0x6A0`)
- `[DEFERRED] OQ-11 - Full `FUN_004A61C0` color/glyph contract.` (category: `out-of-scope`; reason: preview caller offset/clip/helper distinction is enough for this rect/color slice; next-step-if-pursued: investigate `FUN_004A5EB0` / `FUN_004A61C0` as standalone text helper)

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Right-panel statics use child rect, horizontal style alignment, top anchoring, default yellow, and conditional kind-1 reveal animation | `0x00602490`, `0x0060A5B0`, `0x00615AE8`; `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` | reveal missing; rect/color mostly present | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | Preserve no v-center for title/game/map; add reveal timing only as a `0x4EC -> 0x4EE` transition/text-update animation, not as standard first-paint behavior | Transition-triggered Skirmish title/game/map text reveals from count `1` with range `8`; ordinary common first paint / `0x4ED` does not start reveal | Do not use generic v-centered labels or treat `0x4ED` as reveal start; proposed test `skirmish_right_panel_static_labels_top_anchor_and_transition_reveal` |
| Trackbar value text is rightmost 49px, centered both ways, normal yellow or disabled `0x9F` | `0x0061E296..0x0061E30A` | rect/color normal present; disabled color unchecked | `trackbar_value_text_rect`, `push_text_draw` for trackbars | Keep 49px rect and add disabled color branch if trackbars can be disabled | Disable a trackbar in a harness: plaque/value text dims but rect remains `(right-49,top,49,h)` | Do not use the full 50px plaque width as text width; proposed test `skirmish_trackbar_value_text_uses_49px_center_rect` |
| Combo text is caller-truncated before draw: collapsed `left+2`, dropdown `left+3`, width minus `0x14`, v-center only | `0x00617B42..0x00617C04`, `0x0060DE1F..0x0060DFC8` | clipping mostly present; binary-style pre-truncation not proven equivalent | `combo_text_rect`, dropdown row renderer, shell text wrapper | Add/lock tests for long labels so text never enters arrow/scrollbar area | Long side/country label in collapsed combo and open dropdown clips/truncates before arrow/scrollbar | Do not rely on wrapping into a second line inside a 24px combo row; proposed test `skirmish_combo_text_truncates_before_arrow_reserve` |
| Preview numeric labels use live-overlay gate, `FUN_004A61C0`, offset `(-2,-6)`, and destination-surface clip | `0x006409D7..0x00640A15`; preview-header reports | mismatch/partial | `push_start_marker_labels`, preview overlay path | Apply live overlay gate and numeric offset; do not gate solely on preview-rect point containment | Overlay-eligible map shows numbers offset from `STARTBUT.SHP`; loose Dustbowl with baked markers shows no live numbers | Do not infer labels from baked `[PreviewPack]`; proposed test `preview_start_marker_labels_use_live_overlay_offset_and_clip` |

## 8. Negative Facts / Do Not Do

- Do not treat packed shell colors as `0x00RRGGBB`; active wrapper order is `0x00BBGGRR`. Active in YR: Yes. Evidence: `0x00621054..0x006210B1`.
- Do not document button label flags as `0x0C`; active alignment flags are `0x05`, while nearby `0x0C` is a separate wrapper argument. Active in YR: Yes. Evidence: `0x006135DD..0x006135EE`.
- Do not vertically center right-panel statics or map label `0x5A8`; the static proc style flags do not include `0x04`. Active in YR: Yes. Evidence: `0x006153E0`, `0x00615AE8`.
- Do not make checkbox label text part of the toggle hit target while fixing label rects. Active in YR: Yes. Evidence: label starts at `+0x1A` (`0x0061663E..0x00616674`), but click gate remains `x < 0x12 && y < 0x12` in `OwnerDraw_Checkbox_006163A0`.
- Do not route preview start numbers through ordinary `FUN_00621040` shell text or draw them for maps that only have baked red PreviewPack markers. Active in YR: Conditional. Evidence: `DrawStartPositions` `0x006409D7..0x00640A15`; missing-header gate reports for loose Dustbowl.
- Do not apply static-label assumptions to `0x6A0` player-name/edit text without a separate owner-draw edit trace. Active in YR: unresolved for this slice. Evidence: resize docs identify `0x6A0` geometry fixup, but no static/text-helper route was verified here.

## 9. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md` has stale implementation status for several Rust surfaces. Replacement wording: "Current Rust now uses `SHELL_BUTTON_TEXT_RGB_00000C05 = RGB(5,12,0)`, `SHELL_LABEL_TEXT_RGB = RGB(255,255,0)`, verified button text rect offsets, checkbox `+26` label rects, and trackbar 49px value rects; remaining deltas are reveal animation, disabled-state color coverage, binary-style combo pre-truncation tests, preview numeric labels, and unverified player-name/edit text."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md` follow-up wording: "The color-channel mismatch it identified has been reflected in current Rust constants; keep the binary source color finding, but do not describe the current constants as still channel-swapped."
- Any doc that treats setup `0x468` as a static text surface should say: "Preview child `0x468` is a parent-drawn preview anchor; live marker numbers are drawn by `DrawStartPositions` through `FUN_004A61C0`, not by the static owner-draw text path."

## Sources

- Ghidra read-only decompile/spot-check: `FUN_00621040 @ 0x00621040`, `OwnerDraw_Static_006153E0 @ 0x006153E0`, `OwnerDraw_Button_00612B70 @ 0x00612B70`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_0060A5B0 @ 0x0060A5B0`, `FUN_00602490 @ 0x00602490`.
- Ghidra assembly/call-site evidence: `0x00621054..0x006210F2`, `0x00615AE8`, `0x00613591..0x006135EE`, `0x0061663E..0x00616674`, `0x0061E296..0x0061E30A`, `0x00617B42..0x00617C04`, `0x0060DE1F..0x0060DFC8`, `0x006409D7..0x00640A15`.
- Prior docs referenced: `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`, `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`.
- Rust read-only scan: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/render/shell_text.rs`.
