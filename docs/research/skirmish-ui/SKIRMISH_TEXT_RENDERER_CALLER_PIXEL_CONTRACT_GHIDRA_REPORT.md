# Skirmish Text Renderer Caller Pixel Contract - Ghidra Research Report

**Address(es):** `FUN_00621040 @ 0x00621040`, caller sites `0x006135EE`, `0x00616674`, `0x00617C04`, `0x0060DFC8`, `DrawStartPositions @ 0x00640710` / `0x00640A15`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Caller-visible rectangle, flags, color, clipping/truncation, and anchoring contract for standard offline Skirmish buttons, collapsed combos, `ComboDropWin` rows, checkbox labels, and map-preview numeric labels.
**Non-Scope:** full glyph raster internals, CSF/localized string content, text animation internals beyond caller arguments, non-Skirmish dialogs, and screenshot-only final RGB validation.
**Confidence:** High for caller arguments and binary control flow; Medium for exact final color appearance on a retail display mode without runtime capture.
**Active in YR:** Yes for the scoped standard offline Skirmish callers; preview numeric labels are active only when live start overlays are eligible.

## 1. Overview

`FUN_00621040` is the shared owner-draw shell text wrapper used by Skirmish buttons, collapsed combos, checkbox labels, static labels, trackbar values, and popup dropdown rows. It treats the caller-provided `RECT` as both layout bounds and clip bounds, converts a packed RGB-like caller color through the DirectDraw loss/shift globals, optionally vertical-centers before drawing, then delegates wrapping and per-line horizontal alignment to the `BitFont` draw core.

Map-preview numeric labels are the important exception in this slice: `DrawStartPositions` does not call `FUN_00621040`; it uses `FUN_004A61C0` with a `DAT_0081B3D0` format string and full destination-surface clipping.

## 2. Key Argument Model / Offsets

| Area | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00621040` rectangle | Reads `left/top/right/bottom`; computes `w = right - left`, `h = bottom - top`; passes the same rect to `FUN_00433CA0` as the font clip rect. | decompile `0x00621040` | Yes |
| `FUN_00621040` vertical center | If `flags & 0x04`, measures text using `BitFont__MeasureText(..., max_width = w)` and adds `(h - measured_h) / 2` to draw Y before lower draw. | `0x00621040`; `BitFont__MeasureText @ 0x00433CF0` | Yes |
| Horizontal flags | Bits are interpreted by `FUN_00434CD0`: `0x01` center, `0x02` right; caller bit `0x04` has already affected Y in the wrapper and is also passed down. | `FUN_00434CD0 @ 0x00434CD0` | Yes |
| Color conversion | Caller color channels are shifted/loss-reduced into display format: low byte -> R lane, next byte -> G lane, third byte -> B lane, using `g_DD_*Loss` / `g_DD_*Shift`. | decompile `0x00621040` | Yes |
| Null handling | Lower core returns 0 if the font pointer or text pointer is null; scoped callers normally gate text pointer before call. | `FUN_00434CD0 @ 0x00434CD0`; caller gates | Yes |

## 3. Caller Pixel Contracts

### Buttons: Start Game `0x617`, Choose Map `0x5AA`, Back `0x5C0`

Active in YR: Yes. `OwnerDraw_Button_00612B70` is installed for the scoped Skirmish buttons and calls `FUN_00621040` at `0x006135EE`.

The text call is gated by default visual mode (`state+0x14 == 0`) and non-null text pointer (`state+0x28 != 0`). Released/enabled buttons pass a rect with `left = button_left`, `top = button_top + 1`, `right = button_left + width - 2`, `bottom = button_top + height`. Pressed buttons shift only `left` and `top`: `left += 2`, `top += 5`; right/bottom stay as above. Flags are `0x05` (`h-center | v-center`) at the wrapper call, using normal text color from `DAT_00AC18A4` unless disabled color logic replaces it first. Evidence: button rect build and call `0x00613591..0x006135EE`.

Tiny detail: older prose in some reports says `0x0C` for button text flags because of stack/decompiler ordering around the font argument; the assembly call site pushes `0x5` as the flags value immediately before the font argument. The wrapper behavior that matters is horizontal center plus vertical center.

### Collapsed Combos

Active in YR: Yes. `OwnerDraw_ComboBox_00617250` paints standard collapsed Skirmish combo faces and calls `FUN_00621040` at `0x00617C04`.

The caller first truncates the selected item string in a shared UTF-16 buffer (`DAT_00AC18F8`) until `BitFont__GetTextWidth <= combo_client_width - 20`. The text rect then uses `left = combo_left + 2`, unchanged row/control top, full computed bottom, and the already arrow-reserved width context. Flags are `0x04`, so the collapsed selected text is vertically centered but not horizontally centered; it is left anchored at `left + 2`. Evidence: width limit/truncation `0x00617B42..0x00617BAF`; rect/call setup `0x00617BAF..0x00617C04`.

Color mode is caller-selected before the draw: normal uses the shell text global, grey/disabled uses owner-draw grey/disabled globals, and color-combo swatch state can replace the draw color. Evidence: `0x00617A39..0x00617B2E`.

### `ComboDropWin` Dropdown Rows

Active in YR: Yes. Standard combo popup rows are painted by the registered `ComboDropWin` WndProc block at `0x0060D540`; text calls `FUN_00621040` at `0x0060DFC8`.

Each visible row uses source-combo row height from `CB_GETITEMHEIGHT 0x154`, row Y `(item_index - top_index) * item_height`, and current popup client width after any scrollbar shrink. The row text rect starts at `row_left + 3`, `row_top`, spans to current `row_left + row_width`, `row_top + item_height`. The caller truncates the UTF-16 row text until measured width is `<= client_width - 20`, then draws with flags `0x04`: vertical-centered, left anchored. Evidence: row geometry `0x0060DC09..0x0060DC6D`; text rect/truncation/call `0x0060DE1F..0x0060DFC8`.

Selection fill and optional full-row color swatch are drawn before text. If a valid color swatch is present, its color value replaces the normal/grey row text color for the subsequent call. Evidence: selected fill `0x0060DD42..0x0060DE0A`; swatch/color overwrite `0x0060DE60..0x0060DF2A`.

### Checkbox Labels

Active in YR: Yes for standard offline Skirmish option checkboxes. `OwnerDraw_Checkbox_006163A0` calls `FUN_00621040` at `0x00616674`.

The checkbox label rect is the control rect after adding `0x1A` (`26`) to left. The icon is independently blitted at the control top-left as an `18x18` PCX and does not change the label rect except for that left offset. Flags are `0x04`, so label text is vertically centered within the checkbox control rect but left anchored. Normal color is `DAT_00AC18A4`; disabled `WS_DISABLED` switches to `DAT_00AC1CB4`. Evidence: left advance and call setup `0x0061663E..0x00616674`.

Click/toggle hit testing is separate from the text rect: only `x < 0x12 && y < 0x12` toggles. Clicking the label text does not toggle in this owner-draw path. Evidence: `0x006166EE..0x00616708`.

### Map Preview Numeric Labels

Active in YR: Conditional. This path runs only when the selected map has a live overlay count `0 < ScenarioClass+0x113C < 9`; loose Dustbowl-style maps with no `[Header]` skip live labels.

Preview numeric labels do not use `FUN_00621040`. After optional `STARTBUT.SHP` frame 0, `DrawStartPositions` increments the loop index, subtracts `2` from projected marker X and `6` from projected marker Y, then calls `FUN_004A61C0` at `0x00640A15` with format string `DAT_0081B3D0`, arguments including `0x08`, `0x19`, and the 1-based marker number. Clipping is the destination-surface clip returned from `DAT_00887310` vtable `+0x78`, not the preview child rect. Evidence: marker/label branch `0x006409D7..0x00640A15`.

## 4. INI Keys

No INI key controls this caller-visible text layout. Labels and map names come through dialog resources/string table/CSF/session buffers, and map preview label eligibility comes from scenario `[Header] NumberStartingPoints` loaded into `ScenarioClass+0x113C`, not from a text-rendering INI key.

## 5. Integration Points

| Caller | Function | Text helper | Contract summary | Active in YR |
|---|---|---|---|---|
| Start/Choose/Back buttons | `OwnerDraw_Button_00612B70` | `FUN_00621040` | centered both ways; button-specific inset/pressed shift | Yes |
| Collapsed combos | `OwnerDraw_ComboBox_00617250` | `FUN_00621040` | left+2, v-centered, pre-truncated to `width-20` | Yes |
| Dropdown rows | `ComboDropWin` WndProc `0x0060D540` | `FUN_00621040` | left+3, v-centered, pre-truncated to client `width-20` | Yes |
| Checkboxes | `OwnerDraw_Checkbox_006163A0` | `FUN_00621040` | left+26, v-centered, left anchored | Yes |
| Preview numbers | `DrawStartPositions` | `FUN_004A61C0` | label at projected anchor `(-2,-6)`, destination-surface clipping | Conditional |

## 6. Current Rust Implementation Status

Rust has a reasonably matching generic `shell_text::draw_in_rect` wrapper for rect=scissor, `0x01/0x02/0x04` alignment, and v-center measurement (`src/render/shell_text.rs`). The Skirmish caller usage is incomplete: button labels still use the full button rect plus a simple pressed Y offset, not the binary `top+1/right-2` and pressed `left+2/top+5`; combo/dropdown/checkbox text surfaces are not yet implemented in the experimental Skirmish shell; start-marker labels are currently skipped until real preview overlay data is available and, when enabled, should use destination-surface clipping rather than preview-rect clipping.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00621040` wrapper rect/color/v-center contract | verified | `0x00621040` | none |
| Button label caller | verified | `0x00613591..0x006135EE` | runtime screenshot can validate final transient pressed pixels |
| Collapsed combo label caller | verified | `0x00617B42..0x00617C04` | exact CSF text content out of scope |
| `ComboDropWin` row label caller | verified | `0x0060DE1F..0x0060DFC8` | exact final display colors screenshot-only |
| Checkbox label caller | verified | `0x0061663E..0x00616674` | none |
| Preview numeric label caller | verified | `0x006409D7..0x00640A15` | `FUN_004A61C0` glyph internals out of scope |
| Full glyph raster / CSF localization | deferred | scope boundary | separate text/glyph audit if needed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Which scoped callers actually use FUN_00621040? -> buttons, collapsed combos, ComboDropWin rows, and checkbox labels do; preview numbers do not.` (evidence: callers of `FUN_00621040`; `DrawStartPositions @ 0x00640710`)
- `[RESOLVED] OQ2 - Is the rect a layout rect, clip rect, or both? -> both; the wrapper computes width/height from it and stores it as the BitFont clip rect.` (evidence: `0x00621040`, `FUN_00433CA0`)
- `[RESOLVED] OQ3 - Which scoped callers are centered? -> buttons h+v center; combos/dropdown rows/checkbox labels v-center only and left anchored.` (evidence: `0x006135EE`, `0x00617C04`, `0x0060DFC8`, `0x00616674`)
- `[RESOLVED] OQ4 - Where does truncation happen? -> collapsed combos and dropdown rows truncate caller-side before `FUN_00621040`; buttons/checkboxes rely on wrapper/core clipping/wrapping.` (evidence: `0x00617B42..0x00617BAF`, `0x0060DF2D..0x0060DFA1`, `FUN_00434CD0`)
- `[RESOLVED] OQ5 - Are preview numeric labels preview-rect clipped? -> no; they use destination-surface clip from `DAT_00887310 +0x78` through `FUN_004A61C0`, and only run for live overlay-eligible maps.` (evidence: `0x00640710`, `0x00640A15`)
- `[DEFERRED] OQ6 - What are exact final screenshot RGB values for grey/selected text across display modes?` (category: `needs-runtime-debugger`; reason: binary identifies source globals and DirectDraw conversion but runtime capture would validate final display output; next-step-if-pursued: capture retail 16-bit surface pixels for a disabled checkbox and selected dropdown row)
- `[DEFERRED] OQ7 - What is the full `FUN_004A61C0` glyph/layout contract beyond preview numbers?` (category: `out-of-scope`; reason: preview caller use is bounded, but the helper is a broader sidebar/overlay text path; next-step-if-pursued: investigate `FUN_004A5EB0` / `FUN_004A61C0` as a separate text helper)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Button labels use inset rect and pressed `left+2/top+5`, centered both ways | `0x00613591..0x006135EE` | mismatch | `src/app_skirmish_shell_render.rs` `push_button_label_draw` | Use binary text rect, not full button rect | Press Start/Choose/Back: label shifts right/down like retail and clips within inset rect | Do not use a generic full-rect centered label |
| Collapsed combo text is left+2, v-centered, pre-truncated to `width-20` | `0x00617B42..0x00617C04` | missing | Skirmish combo renderer/state | Add selected-item text truncation and v-centered left anchor | Long country/map-side combo text trims before arrow reserve | Do not let text draw under the arrow |
| Dropdown rows use `left+3`, v-center, pre-truncate to current client width minus 20 | `0x0060DE1F..0x0060DFC8` | missing | Skirmish dropdown renderer | Use source-combo row height/top index/client width and row text truncation | Side dropdown with scrollbar keeps text out of scrollbar area | Do not reuse collapsed-combo swatch/text geometry directly |
| Checkbox labels use control-left+26, v-center, disabled color switch | `0x0061663E..0x00616674` | missing | Skirmish checkbox renderer | Draw icon and label as separate rects | Clicking label does not toggle; label aligns beside 18x18 icon | Do not make the whole label area a checkbox hit target |
| Preview numeric labels do not use `FUN_00621040` and are destination-surface clipped | `0x006409D7..0x00640A15` | missing/partially gated | `push_start_marker_labels` / preview overlay path | Use live overlay gate and destination-surface clipping, label offset `(-2,-6)` from marker anchor | Live-overlay-eligible `.yro` maps show labels even if near preview edge; Dustbowl skips live labels | Do not infer labels from baked red PreviewPack pixels |

## Stale Docs / Follow-up Docs

- `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` should replace "flags `0x0C`" with "flags `0x05` (`h-center | v-center`) at the `FUN_00621040` wrapper call; apparent `0x0C` is stack/decompiler ordering around adjacent font argument."
- Any future implementation doc should distinguish preview labels (`FUN_004A61C0`) from ordinary owner-draw labels (`FUN_00621040`).

## Sources

- Ghidra decompiled/read-only: `FUN_00621040 @ 0x00621040`, `FUN_00434CD0 @ 0x00434CD0`, `BitFont__MeasureText @ 0x00433CF0`, `FUN_004A61C0 @ 0x004A61C0`.
- Ghidra call-site evidence: `OwnerDraw_Button_00612B70 @ 0x006135EE`, `OwnerDraw_Checkbox_006163A0 @ 0x00616674`, `OwnerDraw_ComboBox_00617250 @ 0x00617C04`, `ComboDropWin` WndProc block call `0x0060DFC8`, `DrawStartPositions @ 0x00640710` / `0x00640A15`.
- Prior docs referenced: `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING_GHIDRA_REPORT.md`.
- Rust scanned: `src/render/shell_text.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/*`.
