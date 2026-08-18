# Skirmish Owner-Draw Button Pixel Layout - Ghidra Research Report

## Superseded Asset-Family Correction - 2026-05-24

For standard Skirmish setup sidebar Start Game `0x617`, Choose Map `0x5AA`, and
Back `0x5C0`, this report's `bue_*30.pcx` / `bde_*30.pcx` asset-family
conclusion is superseded. The corrected classifier recheck proves these three
right-panel buttons are owner-draw type `1` and draw `SDBTNANM.SHP` frames
`2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` for the
current contract.

**Address(es):** `OwnerDraw_Button_00612B70`, `FUN_006ba3e0`, `FUN_00621040`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline Skirmish dialog `0x102` owner-draw button visual behavior for Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0`: PCX asset selection, cap/middle/cap layout, middle tiling, text rectangle, pressed/disabled/focused/hover state behavior, and label color conversion.  
**Non-Scope:** Click sound timing except where the visual state path reuses the same callback; command semantics after `WM_COMMAND`; non-Skirmish shell buttons; screenshot/runtime audio validation.  
**Confidence:** High for the verified binary slice; Medium for exact final rendered pixels only where Windows message timing or retail screenshots would be needed to observe a transient state.  
**Active in YR:** Yes. `FUN_0060f9a0` assigns `OwnerDraw_Button_00612B70` to Button controls whose low style bits satisfy `(style & 0x0B) == 0x0B`, and prior resource/layout reports map Skirmish `0x102` controls `0x617`, `0x5AA`, and `0x5C0` to that style.

## 1. Overview

The three requested Skirmish buttons are not native/GDI buttons at paint time. They are subclassed Win32 Button controls whose `WM_PAINT` path composes PCX cap/middle/cap pieces into the main destination surface, draws a centered bitfont label, and optionally alpha-blends a disabled overlay.

For the normal Skirmish sizes, the buttons use the `30` PCX family, not the `24` family. Released/normal art is `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`; pressed art is `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`.

Current Rust status as of 2026-05-23: the implementation now matches the
800x600 owner-draw button snap rects, native 30 px art y centering, pressed +2
art movement, fixed-right/bottom text rects, and yellow enabled label color.
The remaining scoped current mismatch is middle PCX source phase: Rust must use
`FUN_006BA3E0`'s centered crop offset before modulo tiling while preserving the
verified 7 px right-cap overlap.

## 2. Key Offsets / State Fields

| Field / value | Meaning | Evidence | Active in YR |
|---|---|---|---|
| owner state `+0xB0` / `piVar17[0x2C]` | visual mode selector; `0` is default PCX cap/middle/cap button path | `OwnerDraw_Button_00612B70` branch before `0x00613240` | Yes for the requested Skirmish main buttons |
| owner state `+0x14` / `piVar17[5]` | custom image pointer; when nonzero bypasses default PCX composition | `OwnerDraw_Button_00612B70` alternate branch after `iVar14 == 0` | Conditional; not used by normal requested buttons |
| owner state `+0x28` / `piVar17[10]` | text pointer gate; if zero, no button text is drawn | `0x00613568..0x00613578` | Yes when the resource/control text exists |
| owner state `+0x64` / `piVar17[0x19]` | text pointer passed to `FUN_00621040` | `0x006135D1..0x006135EE` | Yes |
| owner state `+0xBC` / `piVar17[0x2F]` | blocked/disabled callback flag; suppresses paint and mouse-down sound path | `OwnerDraw_Button_00612B70` early paint/mouse branches | Conditional |
| owner byte `+0xC5` | timer-toggled highlight byte for other button/SHP modes; not used in the default `bue/bde` PCX filename path | `0x0061363F..0x0061365C`, `0x00612ED7..0x00612F56`; absent from default filename block | Conditional; no visual effect for these default PCX buttons |
| `WS_DISABLED` `0x08000000` | forces released art, then applies `AlphaBlendRect(..., 0x80)` | `0x00613254..0x00613262`, `0x006135F3..0x0061361B` | Yes when Start is disabled during validation |
| global `DAT_00AC18A4` | normal owner-draw text RGB value, initialized to `0xFFFF` | `FUN_0060f9a0` init; text call uses `piVar20` unless disabled color path overrides | Yes |
| disabled text color path | display-format-derived disabled color computed from shell globals and DirectDraw shifts/losses | `OwnerDraw_Button_00612B70` block after `0x00612F6F` | Conditional on `WS_DISABLED` |

## 3. Core Visual Logic

### 3.1 Active control route

`FUN_0060f9a0` classifies controls by class name and style. For `"Button"` it checks `(style & 7) == 7` first, then `(style & 0x0B) == 0x0B`, then `(style & 3) == 3`, then `(style & 9) == 9`. The requested controls have style low bits `0x0B`, so they route to `OwnerDraw_Button_00612B70`.

Active in YR: Yes. Evidence: `FUN_0060f9a0` Button branch; `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` lists `0x617`, `0x5AA`, `0x5C0` as `BUTTON` controls with `0x...000B` style; `FUN_006AE2C0` creates/pumps dialog `0x102`.

### 3.2 Size-family selection

The paint block seeds two candidate suffixes: `24` (`0x18`) and `30` (`0x1E`). It compares the actual client height against those candidates and chooses the larger suffix when the client height is at least `30`.

Prior resource decoding proves the three normal 800x600 Skirmish button client heights:

| Control | Final active rect at 800x600 | Height | Selected suffix |
|---|---:|---:|---:|
| Start Game `0x617` | `(635,242,162,37)` | `37` | `30` |
| Choose Map `0x5AA` | `(635,286,162,37)` | `37` | `30` |
| Back `0x5C0` | `(644,535,156,42)` | `42` | `30` |

Active in YR: Yes. Evidence: `0x006132B9..0x006132C9` seeds `24` and `30`; `0x006132D7..0x006132FD` selection loop; final rects from `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`.

### 3.3 PCX names and visual states

The default path formats three filenames:

| Piece | Format string | Released state | Pressed state |
|---|---|---|---|
| left cap | `b%c%c_li%d.pcx` | `bue_li30.pcx` | `bde_li30.pcx` |
| middle | `b%c%c_mi%d.pcx` | `bue_mi30.pcx` | `bde_mi30.pcx` |
| right cap | `b%c%c_ri%d.pcx` | `bue_ri30.pcx` | `bde_ri30.pcx` |

The first `%c` is the state char: `'u'` by default, `'d'` when the button state low bit is set. The second `%c` is hardcoded to `'e'` in this default path. Disabled style does not make `bud_*`: it forces `'u'` and later alpha-blends the button.

Active in YR: Yes. Evidence: `0x00613240..0x0061325E` state char and disabled forcing; `0x006133B2..0x006133C8`, `0x00613444..0x00613455`, `0x006134C9..0x006134DA` filename formatting call sites.

### 3.4 Cap/middle/cap draw behavior

The left and right PCX pieces are direct destination-surface blits. The middle piece is passed through `FUN_006ba3e0`, which locks source and destination surfaces, centers the tile phase when the destination rect is larger than the source, and writes pixels using modulo addressing across the source width and height. It is tiling, not scaling.

The visual implication is:

- caps retain native source pixels;
- the middle fills the space between caps by repeated source pixels;
- if the final middle span is not a whole multiple of the source tile, the helper clips through the destination rect and modulo loop rather than stretching a final strip;
- no primitive/GDI fallback is used when a cap/middle lookup succeeds.

Active in YR: Yes. Evidence: direct blit via destination surface vtable `+0x08` at `0x00613441` and `0x0061355D`; middle call `0x006134C4`; helper `FUN_006ba3e0` modulo-copy loop.

### 3.5 Text rectangle, pressed offset, and color

Text is drawn only after the art path completes and only when `state+0x14 == 0` and `state+0x28 != 0`. The rectangle supplied to `FUN_00621040` is:

| State | Text left | Text top | Text right | Text bottom |
|---|---:|---:|---:|---:|
| released/enabled | `button_left` | `button_top + 1` | `button_left + width - 2` | `button_top + height` |
| pressed | `button_left + 2` | `button_top + 5` | `button_left + width - 2` | `button_top + height` |

The call passes horizontal center flag `0x08` and vertical center flag `0x04` together as `0x0C`. `FUN_00621040` measures text height when flag `4` is set and adds `(rect_height - measured_height) / 2` to the draw Y. The final draw color is converted from an RGB-like value to the active 16-bit display format through the DirectDraw loss/shift globals before calling the lower bitfont draw routine.

Active in YR: Yes. Evidence: text rect setup `0x00613591..0x006135CD`, flags/color call `0x006135D1..0x006135EE`, `FUN_00621040` vertical centering and color conversion.

### 3.6 Disabled, focus, hover, and timer states

`WS_DISABLED` forces released art and then applies `AlphaBlendRect(..., 0x80)` over the button rect after text/art drawing. In the disabled color block, the text color is recalculated from display-format shell color globals before the text call.

The callback returns `0` for `WM_ENABLE` (`0x0A`), `WM_KILLFOCUS` (`0x08`), and `0x21`, so those messages do not directly choose new art. `WM_TIMER` toggles `+0xC5` and invalidates, and custom `0x4DC` starts/stops a 1000 ms timer, but the default PCX filename path for these requested buttons uses only the pressed bit, disabled style, and fixed second char `'e'`. No hover-specific `b*` PCX family is selected for this default path.

Active in YR: Yes for disabled/pressed behavior; Conditional for timer/highlight because it requires custom `0x4DC` and does not alter the default PCX filenames. Evidence: `0x00612D59` message switch; `0x0061363F..0x0061365C`; `0x00612DE3..0x00612E3F`; `0x00613240..0x006135EE`.

## 4. INI Keys

No INI key controls the requested visual layout. The button labels are loaded through the dialog/string-table path, and the PCX asset names are hardcoded format strings in `OwnerDraw_Button_00612B70`.

| INI key | Effect in this slice | Active in YR |
|---|---|---|
| none | Button art selection, tiling, text rect, and disabled alpha are binary-defined callback behavior | Yes |

## 5. Integration Points

| Function / site | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006AE2C0` | creates and pumps Skirmish setup dialog `0x102` until Start/Back result | prior Start/launcher docs | Yes |
| `FUN_00622B50` | common shell dialog setup enumerates/hooks children | prior common paint docs; `FUN_0060f9a0` hook target | Yes |
| `FUN_0060f9a0` | assigns `OwnerDraw_Button_00612B70` to requested Button style | decompile `FUN_0060f9a0` | Yes |
| `OwnerDraw_Button_00612B70` | handles paint, timer/custom highlight, mouse down/double-click sound, disabled alpha | decompile and assembly spot checks | Yes |
| `FUN_006ba140` | returns cached converted PCX surface by formatted name | decompile `FUN_006ba140` | Yes |
| `FUN_006ba3e0` | tiles the middle PCX into the destination span | decompile `FUN_006ba3e0`, call `0x006134C4` | Yes |
| `FUN_00621040` | draws centered button text with RGB-to-display conversion | decompile `FUN_00621040`, call `0x006135EE` | Yes |

## 6. Current Rust Implementation Status

Rust already implements the broad button concept, the correct `bue/bde` 30-family asset names, pressed-state selection, disabled alpha, and middle-piece tiling tests. The verified deltas are in the exact text rectangle and edge behavior.

| Rust area | Current status | Evidence |
|---|---|---|
| button identities and hit testing | present | `src/ui/skirmish_shell/state.rs` `OwnerDrawButton`, `hit_test_owner_draw_button` |
| asset names | matches normal released/pressed families | `src/app_skirmish_shell_render.rs` `button_piece_asset_names` |
| middle tiling | approximates whole-tile repeats with clipped final UV strip | `src/app_skirmish_shell_render.rs` `build_button_segments` |
| button art height | scales each segment to full control height; binary blits native-height 30px pieces and leaves destination/background outside that art area | `src/app_skirmish_shell_render.rs` `push_button_30` |
| text rect | currently uses full button rect with `y_offset` only; binary uses top `+1`, right `-2`, and pressed left `+2` / top `+5` | `src/app_skirmish_shell_render.rs` `push_button_label_draw`, `build_shell_text_draws` |
| disabled text color | unchecked; Rust constant is normal button RGB | `src/app_skirmish_shell_render.rs` `SHELL_BUTTON_TEXT_RGB_00000C05` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x617`, `0x5AA`, `0x5C0` routing to `OwnerDraw_Button_00612B70` | verified | `FUN_0060f9a0`; layout docs | none |
| size suffix selection | verified | `0x006132B9..0x006132FD`; rect docs | none |
| released/pressed PCX format strings | verified | `0x006133B2..0x006134DA` | none |
| `bud_*` non-use on normal path | verified | state char block and fixed `'e'`; prior xref scan | none for requested buttons |
| left/right direct blits | verified | `0x00613441`, `0x0061355D` | none |
| middle tiling | verified | `0x006134C4`, `FUN_006ba3e0` | none |
| text rect and pressed offset | verified | `0x00613591..0x006135CD` | none |
| text alignment/color conversion | verified | `FUN_00621040` | none |
| disabled alpha | verified | `0x006135F3..0x0061361B`, `AlphaBlendRect` | none |
| hover/focus visual changes for requested default buttons | verified negative | no hover/focus asset branch in default filename path | runtime screenshot can still validate message timing |
| Rust parity scan | touched-not-exhausted | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs` | implementation not changed in this report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-BTN-001 - Are the three requested buttons active on the standard YR Skirmish path? -> Yes; dialog 0x102 creates these controls and owner-draw hook selects OwnerDraw_Button_00612B70 for style low bits 0x0B.` (evidence: `FUN_0060f9a0`; `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-BTN-002 - Which PCX height family do these buttons use? -> The 30 family, because their live heights are 37/42 and the callback chooses suffix 30 when client height is at least 30.` (evidence: `0x006132B9..0x006132FD`; `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-BTN-003 - Are caps scaled or tiled? -> No; caps are direct-blitted native PCX pieces.` (evidence: `0x00613441`, `0x0061355D`)
- `[RESOLVED] OQ-BTN-004 - Is the middle stretched? -> No; FUN_006ba3e0 tiles by modulo addressing over the destination rect.` (evidence: `0x006134C4`; `FUN_006ba3e0`)
- `[RESOLVED] OQ-BTN-005 - Does disabled Start use bud_* art? -> No for this default path; WS_DISABLED forces state back to 'u' and applies AlphaBlendRect 0x80.` (evidence: `0x00613254..0x00613262`; `0x006135F3..0x0061361B`)
- `[RESOLVED] OQ-BTN-006 - Does hover/focus select a separate visual frame? -> No separate hover/focus PCX is selected in the default PCX path; focus/enable messages return 0 and +0xC5 timer state is not used in the default filename block.` (evidence: `OwnerDraw_Button_00612B70`)
- `[RESOLVED] OQ-BTN-007 - What text rectangle is used? -> Released uses left x, top y+1, right x+w-2, bottom y+h; pressed uses left x+2 and top y+5 before centered text draw.` (evidence: `0x00613591..0x006135CD`)
- `[RESOLVED] OQ-BTN-008 - What text alignment flags are used? -> `0x0C`, meaning vertical centering flag 4 plus horizontal centering flag 8 in FUN_00621040/FUN_00434CD0 path.` (evidence: `0x006135DD..0x006135EE`; `FUN_00621040`)
- `[RESOLVED] OQ-BTN-009 - Are INI keys involved in pixel layout? -> No direct INI key is read in this slice.` (evidence: inspected callback/callees; no INI string xrefs in path)
- `[DEFERRED] OQ-BTN-010 - Does retail runtime ever visibly toggles +0xC5 on these exact three controls?` (category: needs-runtime-debugger; reason: binary proves the custom 0x4DC/timer path but not that standard Skirmish sends 0x4DC to these buttons; next-step-if-pursued: runtime message trace on dialog 0x102 buttons)
- `[DEFERRED] OQ-BTN-011 - Pixel-perfect final screenshot comparison for disabled alpha and text color.` (category: needs-runtime-debugger; reason: binary proves color conversion and alpha call, but final display-format pixels depend on runtime DirectDraw mode; next-step-if-pursued: capture retail 16-bit shell screenshot during Start validation failure)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Use `bue_li30/mi30/ri30` released and `bde_li30/mi30/ri30` pressed for requested buttons | `0x006133B2..0x006134DA` | none observed for names | `src/app_skirmish_shell_render.rs` | keep current normal/pressed assets | Start/Choose/Back visibly switch to pressed PCX while armed | Do not use `bud_*` for normal disabled Start |
| Blit cap pieces native-size and tile middle with modulo behavior | `0x00613441`, `0x006134C4`, `FUN_006ba3e0`, `0x0061355D` | partial; Rust segments tile horizontally but scales segment height to full control rect | `src/app_skirmish_shell_render.rs` `push_button_30`, `build_button_segments` | button art should preserve native PCX height and tile middle, leaving parent chrome/background outside the art where the control rect is taller | Compare 37px Start button: 30px PCX art should not be vertically stretched to 37px | Do not stretch middle/caps to fill the full button height |
| Released text rect starts at top+1 and right-2; pressed text shifts to left+2/top+5 | `0x00613591..0x006135CD` | mismatch; Rust uses full rect and only y offset | `src/app_skirmish_shell_render.rs` `push_button_label_draw` | apply exact rect before `ShellAlign::H_CENTER | V_CENTER` | Text baseline moves down/right only in pressed state, with released label centered in the binary's inset rect | Do not simply add a global y offset to all states |
| Disabled style forces released art and overlays alpha `0x80` | `0x00613254..0x00613262`, `0x006135F3..0x0061361B` | partially present as disabled alpha; disabled text color unchecked | `src/app_skirmish_shell_render.rs` | keep released art, apply half alpha, and verify disabled text color path if implementing Start validation failure | Force a Start validation error; Start is disabled/re-enabled with dimmed released art | Do not swap to `bud_*` unless a future use-site proves it |
| No hover/focus PCX variant for these default buttons | `OwnerDraw_Button_00612B70` default path | Rust has no Skirmish hover art, which matches this slice | `src/app_skirmish_shell_render.rs`, input state | keep hover visual neutral for these controls unless custom 0x4DC path is proven | Moving mouse over Start without pressing does not change button art | Do not invent hover-highlight assets from main-menu logic |

### Stale Docs / Follow-up Docs

- `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md` listed exact cap/middle tiling and text y offset as a fresh gap. Replacement wording: "Resolved by `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`: default Skirmish buttons use `bue/bde_*30` PCX cap/middle/cap; caps are direct-blitted, middle is tiled by `FUN_006ba3e0`, released text rect is top+1/right-2, and pressed text rect is left+2/top+5."

## Sources

- Ghidra decompiled / assembly-checked: `OwnerDraw_Button_00612B70`, `FUN_006ba3e0`, `FUN_006ba140`, `FUN_00621040`, `AlphaBlendRect`, `FUN_0060f9a0`, `FUN_00775690`, `VocClass__PlayAtPos`.
- Prior docs: `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs`, `src/render/skirmish_shell_chrome.rs`.
