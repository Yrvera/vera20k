# Skirmish Trackbar Static Value Text Color Source - Ghidra Research Report

**Address(es):** `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006AE6E0 @ 0x006AE6E0`, `OwnerDraw_Static_006153E0 @ 0x006153E0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Exact source color and text draw path for standard offline Skirmish dialog `0x102` parent-shell control numeric values: Game Speed trackbar `0x529`, Credits trackbar `0x511`, and Unit Count trackbar `0x50C`. Static-class label color is checked only to distinguish normal static/control text from button-specific text. Disabled variants are included only where the same trackbar paint path directly reveals them.
**Non-Scope:** Start/Customize/Back button labels, combo/dropdown row text, checkbox labels except as prior static/control context, player-name edit text, Choose Map dialog `0x6B`, final retail screenshot RGB after DirectDraw/capture, and Rust implementation changes.
**Confidence:** High for enabled and disabled source color constants, byte order, active standard-trackbar route, value rect, alignment flags, and current Rust mismatch.
**Active in YR:** Yes for enabled standard `0x102` trackbar value text; Conditional for disabled branch if `WS_DISABLED` is set by a nonstandard/external flow; No normal standard `0x102` runtime disable flow was found in the prior disabled-flow report.

## 0. Working Notes

- Target question: What exact text color source and text draw path does `gamemd.exe` use for Skirmish Game Speed, Credits, and Unit Count trackbar value text?
- Non-goals: Do not investigate button labels except to distinguish button-specific text from static/control text; do not re-open full trackbar geometry, sound, input, or widget disable-flow beyond same-function color evidence; do not modify Rust.
- Evidence needed to mark COMPLETE: decompile plus assembly/call-site evidence for the trackbar paint path color selection, text-wrapper byte order, and standard `0x102` reachability for `0x529/0x511/0x50C`; current Rust source comparison and implementation handoff.
- Stop conditions: all scoped color-source questions resolved or explicitly deferred; Ghidra read-only; write only this report plus the shared claims file.

## 1. Overview

The standard Skirmish trackbar value numbers are not button text. They are drawn inside `OwnerDraw_Trackbar_0061D950` through the shared shell text wrapper `FUN_00621040`, using normal shell label color `DAT_00AC18A4 = 0x0000FFFF` while enabled. Because `FUN_00621040` decodes packed colors as low-byte red, `0x0000FFFF` is source RGB `(255,255,0)`.

The current Rust render path for those three values uses `SHELL_BUTTON_TEXT_RGB_00000C05 = RGB(5,12,0)`, which is a mismatch for enabled Game Speed, Credits, and Unit Count value text.

## 2. Key Constants / Paths

| Item | Value / behavior | Active in YR | Evidence |
|---|---|---|---|
| Normal static/control text source | `DAT_00AC18A4 = 0x0000FFFF` | Yes | `FUN_0060F9A0` decompile; assembly `0x0060FA3F` stores `0xFFFF` to `0x00AC18A4` |
| Trackbar disabled text source | `DAT_00AC1CB4 = 0x0000009F` | Conditional | `FUN_0060F9A0` decompile; assembly `0x0060FA0D..0x0060FA14`; trackbar branch `0x0061E2AD..0x0061E2B1` |
| Color byte order in wrapper | source packed as `0x00BBGGRR`; low byte is red | Yes | `FUN_00621040` decompile; assembly `0x00621054..0x006210B1` extracts low byte, next byte, and third byte separately |
| Standard trackbar callback | `"msctls_trackbar32"` -> `OwnerDraw_Trackbar_0061D950` | Yes | `FUN_0060F9A0` decompile; assembly `0x0060FC76..0x0060FCB1` |
| Standard scoped controls | `0x529`, `0x511`, `0x50C` initialized by `FUN_006AE6E0` | Yes | decompile; assembly `0x006AECB3..0x006AEDA0` |

## 3. Core Logic

### 3.1 Shared Text Wrapper Color Decoding

Active in YR: Yes.

`FUN_00621040` receives a caller-provided packed source color. It does not interpret this as `0x00RRGGBB`. The wrapper reads:

- low byte -> red channel,
- next byte (`AH` / `>> 8`) -> green channel,
- third byte (`>> 16`) -> blue channel,
- then applies the active DirectDraw loss/shift globals before passing the packed display color onward.

Evidence: decompile `FUN_00621040`; assembly context `0x00621054..0x006210B1` shows low-byte masking for red, `MOV BL,AH` for green extraction, and `SHR ESI,0x10` for blue extraction before `FUN_00433C70` receives the converted color.

Resulting source colors:

| Packed source | Source RGB |
|---|---|
| `0x0000FFFF` | `(255,255,0)` |
| `0x0000009F` | `(159,0,0)` |
| `0x00000C05` | `(5,12,0)` |

### 3.2 Standard Trackbar Value Text Path

Active in YR: Yes.

`FUN_0060F9A0` routes class name `"msctls_trackbar32"` to `OwnerDraw_Trackbar_0061D950` at `0x0060FC76..0x0060FCB1`. `FUN_006AE6E0` initializes all three standard offline Skirmish trackbars:

- `0x529` Game Speed: sends `0x406` range `0..6` and `0x405` position `6 - DAT_00A8B268`.
- `0x511` Credits: sends `0x406` range from Rules `+0x1480..+0x1488`, `0x405` current credits, and `0x4AB` step from Rules `+0x148C`.
- `0x50C` Unit Count: sends `0x406` range from Rules `+0x1490..+0x1498` and `0x405` current unit count.

Evidence: `FUN_006AE6E0` decompile; assembly `0x006AECB3`, `0x006AED29..0x006AED42`, and `0x006AED85..0x006AED9E`.

### 3.3 Trackbar Value Text Color Selection

Active in YR: Yes for enabled text; Conditional for disabled text.

In the paint branch, `OwnerDraw_Trackbar_0061D950` draws value text only when numeric display is enabled (`uStack_12c != 0`). It formats the quantized absolute value through `FUN_007CA564` with format string `DAT_0081B3D0`, then selects the text color:

- load `DAT_00AC18A4` into `EAX` unconditionally,
- test the cached disabled-style bit,
- if the bit is nonzero, replace `EAX` with `DAT_00AC1CB4`,
- push that selected color into `FUN_00621040`.

Evidence: decompile `OwnerDraw_Trackbar_0061D950`; assembly `0x0061E296..0x0061E30A`.

Load-bearing instruction sequence:

- `0x0061E296`: pushes `DAT_0081B3D0` for `FUN_007CA564`.
- `0x0061E2A5`: `MOV EAX,[0x00AC18A4]`.
- `0x0061E2AD..0x0061E2B1`: tests disabled-style state and conditionally loads `[0x00AC1CB4]`.
- `0x0061E2D9`: pushes `0x0C` as the adjacent wrapper/font argument.
- `0x0061E2DE`: pushes `0x05` as alignment flags.
- `0x0061E2E0`: pushes `EAX`, the selected color.
- `0x0061E30A`: calls `FUN_00621040`.

The value text rect remains the rightmost `0x31` pixels: `left = control_right - 0x31`, `top = control_top`, `right = control_right`, `bottom = control_bottom`. Alignment flags are `0x05` (`h-center | v-center`).

### 3.4 Disabled Variant Reachability

Active in YR: Conditional.

The same paint function supports disabled value text through `WS_DISABLED` (`0x08000000`) and `DAT_00AC1CB4`. However, `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md` found no normal standard offline `0x102` path that disables `0x529`, `0x511`, or `0x50C`; these three are initialized enabled and remain enabled through standard setup interactions. Therefore the disabled color branch is real shared owner-draw behavior, but it is not normally reached for these three standard trackbars.

Evidence: `OwnerDraw_Trackbar_0061D950` style read and color branch; prior disabled-flow report evidence `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006AE3F0`, `FUN_006ADC20`, and `FUN_006ACD60`.

## 4. INI Keys

No INI key controls the scoped text color. Related value ranges/defaults are live:

| INI key | Stock YR value | Effect | Active in YR |
|---|---:|---|---|
| `[MultiplayerDialogSettings] MinMoney` | `5000` | credits min | Yes |
| `[MultiplayerDialogSettings] Money` | `10000` | credits default | Yes |
| `[MultiplayerDialogSettings] MaxMoney` | `10000` | credits max | Yes |
| `[MultiplayerDialogSettings] MoneyIncrement` | `100` | credits step | Yes |
| `[MultiplayerDialogSettings] MinUnitCount` | `0` | unit-count min | Yes |
| `[MultiplayerDialogSettings] UnitCount` | `10` | unit-count default | Yes |
| `[MultiplayerDialogSettings] MaxUnitCount` | `10` | unit-count max | Yes |
| `[MultiplayerDialogSettings] GameSpeed` | `1` | stored game-speed default; visual is inverted by setup path | Yes |

## 5. Integration Points

| Function / address | Role | Active in YR |
|---|---|---|
| `FUN_0060F9A0 @ 0x0060F9A0` | common shell subclass/setup router; initializes color globals and maps trackbar class to callback | Yes |
| `FUN_006AE6E0 @ 0x006AE6E0` | standard Skirmish `0x102` initialization for `0x529/0x511/0x50C` | Yes |
| `OwnerDraw_Trackbar_0061D950 @ 0x0061D950` | paint/input/value callback; owns numeric value text draw | Yes |
| `FUN_00621040 @ 0x00621040` | shared shell text wrapper; decodes source color and applies text rect/flags | Yes |
| `OwnerDraw_Static_006153E0 @ 0x006153E0` | static-class label path; uses `DAT_00AC18A4` as ordinary text color | Yes for Static controls; only context for this slice |

## 6. Current Rust Implementation Status

Current Rust mismatch:

- `src/app_skirmish_shell_render/text.rs` draws the three trackbar value strings in `build_shell_text_draws`.
- Labels for Game Speed/Credits/Unit Count use `push_label_draw`, which uses `SHELL_LABEL_TEXT_RGB`.
- The numeric value strings use `SHELL_BUTTON_TEXT_RGB_00000C05`.
- Binary evidence says enabled trackbar numeric values should use normal `DAT_00AC18A4`, i.e. the same yellow source color as ordinary static/control labels, not the button/dark constant.

Affected current code:

| Surface | Current Rust status |
|---|---|
| `trackbar_display_value` | Correctly derives displayed Game Speed, Credits, Unit Count strings for this color slice. |
| `trackbar_value_text_rect` | Matches the verified 49px right-side value rect. |
| Trackbar value text color | Mismatch: uses `SHELL_BUTTON_TEXT_RGB_00000C05`; should use `SHELL_LABEL_TEXT_RGB` for normal enabled `0x102` trackbars. |
| Disabled trackbar value color | Not required for normal standard `0x102`; optional only if a forced-disabled/harness state is represented. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_00621040` source color byte order | verified | decompile; assembly `0x00621054..0x006210B1` | final retail screenshot RGB optional |
| Global color initialization | verified | `FUN_0060F9A0`; assembly `0x0060FA0D..0x0060FA49` | none |
| Trackbar callback mapping | verified | `FUN_0060F9A0`; assembly `0x0060FC76..0x0060FCB1` | none |
| Standard `0x529/0x511/0x50C` initialization | verified | `FUN_006AE6E0`; assembly `0x006AECB3..0x006AEDA0` | none |
| Enabled trackbar value text color | verified | `OwnerDraw_Trackbar_0061D950`; assembly `0x0061E2A5..0x0061E30A` | none |
| Disabled trackbar value text color | verified-conditional | same branch, `0x0061E2AD..0x0061E2B1`; disabled-flow report | only retail screenshot if a forced-disabled trackbar harness is created |
| Static-class ordinary text color context | touched-not-exhausted | `OwnerDraw_Static_006153E0` decompile; prior static text report | exact static-label matrix not in this slice |
| Button label color | not-touched | intentionally excluded by scope | separate slot 1 / button-color recheck |
| Current Rust comparison | verified | source scan: `src/app_skirmish_shell_render/text.rs` trackbar loop | implement future Rust delta |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which investigation mode? -> exhaustive-slice for standard Skirmish trackbar value text color source only.` (evidence: user scope and Section 0)
- `[RESOLVED] OQ-02 - Are `0x529`, `0x511`, and `0x50C` active in standard YR? -> Yes; setup initializes all three in `FUN_006AE6E0`.` (evidence: `0x006AECB3..0x006AEDA0`)
- `[RESOLVED] OQ-03 - Which callback draws their value text? -> `OwnerDraw_Trackbar_0061D950`, selected by class-route `"msctls_trackbar32"`.` (evidence: `0x0060FC76..0x0060FCB1`)
- `[RESOLVED] OQ-04 - What normal color is selected for enabled value text? -> `DAT_00AC18A4 = 0x0000FFFF`.` (evidence: `0x0061E2A5`, `0x0060FA3F`)
- `[RESOLVED] OQ-05 - What RGB does `0x0000FFFF` represent? -> source RGB `(255,255,0)` because low byte is red and next byte is green.` (evidence: `0x00621054..0x006210B1`)
- `[RESOLVED] OQ-06 - Does the trackbar value use the button/dark `0x00000C05` color? -> No, not in the verified trackbar value text path.` (evidence: `0x0061E2A5..0x0061E30A`)
- `[RESOLVED] OQ-07 - What disabled value color exists in the same function? -> `DAT_00AC1CB4 = 0x0000009F` when disabled-style state is nonzero.` (evidence: `0x0061E2AD..0x0061E2B1`, `0x0060FA0D..0x0060FA14`)
- `[RESOLVED] OQ-08 - Is disabled trackbar value color normally reached by standard `0x102`? -> No normal standard flow found; branch is conditional shared owner-draw behavior.` (evidence: `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-09 - What rect/flags accompany the color call? -> rightmost `0x31` px, full height, flags `0x05`.` (evidence: `0x0061E2D9..0x0061E30A`)
- `[RESOLVED] OQ-10 - Does current Rust match enabled value color? -> No; it uses `SHELL_BUTTON_TEXT_RGB_00000C05` for numeric trackbar values.` (evidence: source scan `src/app_skirmish_shell_render/text.rs` trackbar loop)
- `[DEFERRED] OQ-11 - What final display RGB is captured from a retail frame?` (category: `needs-runtime-debugger`; reason: source color and DirectDraw packing are verified, but this slice did not capture a live retail frame; next-step-if-pursued: sample pixels from native `0x102` trackbar value glyphs)
- `[DEFERRED] OQ-12 - Are button labels also yellow or button-dark?` (category: `out-of-scope`; reason: assignment explicitly excludes button labels except for distinction; next-step-if-pursued: slot 1 button color recheck)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Trackbar_0061D950` paint background/rail | `WM_PAINT`; standard callback mapped by `FUN_0060F9A0` | primitive rail / optional `trof*.pcx` plaque | client rect; plaque reserves `0x32` px | primitive globals / PCX | Yes | chrome |
| 2 | `OwnerDraw_Trackbar_0061D950` thumb | same | `trakgrip.pcx` | `left + 1 + pixel_offset`, full control top | PCX blit | Yes | control |
| 3 | `FUN_007CA564` format | numeric display flag `uStack_12c != 0` | none | stack text buffer | n/a | Yes | value text preparation |
| 4 | `OwnerDraw_Trackbar_0061D950` color select | normal unless disabled-style state nonzero | none | n/a | `DAT_00AC18A4` or `DAT_00AC1CB4` source color | Yes / Conditional | value text color |
| 5 | `FUN_00621040` text draw | flags `0x05`, selected color pushed by caller | `GAME.FNT` via owner-draw record font | `[right-0x31, top, right, bottom]` | low-byte-red DirectDraw conversion | Yes | visible numeric value |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `GAME.FNT` | Yes | Yes | Yes | numeric text | no | no | no | no | `FUN_0060F9A0` owner-draw record font; `FUN_00621040` |
| `trofl/trofm/trofr.pcx` | Yes when numeric plaque enabled | Yes | Yes | no | yes | no | no | no | `OwnerDraw_Trackbar_0061D950` |
| `trakgrip.pcx` | Yes | Yes | Yes | no | yes | no | no | no | `OwnerDraw_Trackbar_0061D950` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Enabled Game Speed/Credits/Unit Count value text uses `DAT_00AC18A4 = 0x0000FFFF`, source RGB `(255,255,0)` | `0x0061E2A5..0x0061E30A`; `0x00621054..0x006210B1`; `0x0060FA3F` | mismatch: current trackbar value draw uses `SHELL_BUTTON_TEXT_RGB_00000C05` | `src/app_skirmish_shell_render/text.rs` trackbar value draw in `build_shell_text_draws` | Use the normal label/yellow color for enabled trackbar numeric values | At 800x600 Skirmish, Game Speed, Credits, and Unit Count numeric values render yellow like control/static text, while rect and centering remain unchanged | Do not reuse button-label color for trackbar values; proposed test `skirmish_trackbar_value_text_uses_shell_label_yellow_not_button_dark` |
| Disabled trackbar value text switches to `DAT_00AC1CB4 = 0x0000009F` only when disabled-style state is set | `0x0061E2AD..0x0061E2B1`; `0x0060FA0D..0x0060FA14` | no required normal-path delta; forced-disabled branch absent/unchecked | optional future render-state harness only | Keep normal standard `0x102` trackbars enabled; add disabled color only if a real disabled trackbar surface is modeled | Forced disabled harness, if introduced, renders value text using disabled source RGB `(159,0,0)` without changing rect | Do not add speculative runtime disabling to standard Game Speed/Credits/Unit Count; proposed test `skirmish_forced_disabled_trackbar_value_uses_disabled_text_color` |
| Button labels are outside this color-source proof | scope and absence of `0x00000C05` in trackbar value call | unchecked in this report | button-specific slot/report | Keep button color decisions separate from trackbar/static values | Fixing trackbar values does not alter Start/Customize/Back text color | Do not use this report as proof for button label color either way |

Stale Docs / Follow-up Docs:

- `docs/research/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace the implementation-status phrase "`trackbar_value_text_rect` and trackbar value draw matches rightmost `49px` rect and `h-center|v-center`; relies on current final control rects for the `0x50C y-1` fixup" with: "`trackbar_value_text_rect` matches the rightmost `49px` rect and `h-center|v-center`, but current Rust trackbar numeric values use `SHELL_BUTTON_TEXT_RGB_00000C05`; enabled retail uses normal `DAT_00AC18A4 = 0x0000FFFF` source RGB `(255,255,0)`, so the Rust value color is a current mismatch."
- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`: replace the statement "`0x00000C05` is source RGB `(5,12,0)` for button and trackbar value text" with: "`0x00000C05` decodes to RGB `(5,12,0)`, but the standard Skirmish trackbar value text path does not use it while enabled; `OwnerDraw_Trackbar_0061D950` selects `DAT_00AC18A4 = 0x0000FFFF` for enabled values and `DAT_00AC1CB4 = 0x9F` only for disabled-style trackbars."

## Negative Facts / Do Not Do

- Do not use `SHELL_BUTTON_TEXT_RGB_00000C05` / `0x00000C05` for enabled Game Speed, Credits, or Unit Count numeric values. Active in YR: No for this text path. Evidence: `0x0061E2A5..0x0061E30A`.
- Do not infer button label color from trackbar value color. Active in YR: unresolved by this report. Evidence: button label path intentionally excluded; trackbar value call is independent.
- Do not treat `0x0000FFFF` as blue/yellow-swapped; source byte order is low-byte red, so it is RGB `(255,255,0)`. Active in YR: Yes. Evidence: `0x00621054..0x006210B1`.
- Do not add a normal Skirmish runtime disabled state to `0x529`, `0x511`, or `0x50C` just because the owner-draw callback has a disabled paint branch. Active in YR: No for standard runtime disable flow. Evidence: `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`.
- Do not change the verified 49px value rect or `0x05` alignment while fixing color. Active in YR: Yes. Evidence: `0x0061E2D9..0x0061E30A`.

## Sources

- Ghidra read-only decompile: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006AE6E0 @ 0x006AE6E0`, `OwnerDraw_Static_006153E0 @ 0x006153E0`.
- Ghidra assembly context: `0x0061E296..0x0061E30A`, `0x00621054..0x006210B1`, `0x0060FA0D..0x0060FA49`, `0x0060FC76..0x0060FCB1`, `0x006AECB3..0x006AEDA0`.
- Prior docs: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_TRACKBAR_CHANGED_VALUE_SOUND_GHIDRA_REPORT.md`, `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish_shell_render.rs`.
