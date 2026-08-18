# Skirmish Checkbox/Trackbar Rect Color Recheck - Ghidra Research Report

**Address(es):** `FUN_0060B950 @ 0x0060B950`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006208F0 @ 0x006208F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline Skirmish dialog `0x102` at 800x600/current fixed-resource geometry for the five option checkboxes, three trackbars, value plaques, and shell text/primitive color extraction.  
**Non-Scope:** combo dropdown details, Start/Choose/Back button art placement, status strip, random map modal, runtime audio beyond stale-claim checks, and broader non-Skirmish owner-draw controls.  
**Confidence:** High for rects, text source colors, active owner-draw callbacks, and current Rust status; Medium for final retail 16-bit/DirectDraw sampled rail pixels because no live screenshot was captured.  
**Active in YR:** Yes for standard offline `0x102` checkbox/trackbar controls; disabled color branches are Conditional on `WS_DISABLED` and not normally reached for the three standard trackbars per prior report.

## 0. Working Notes

Target question: Does current Rust still diverge from retail `gamemd.exe` for 800x600 first-four checkbox x placement, unit-count trackbar y, trackbar plaque x/y, shell label/value/button packed color extraction, and active checkbox/trackbar render/input status?

Non-goals: Do not re-audit sound except to detect stale claims; do not re-open combo/dropdown/button art; do not modify Rust or INI; do not mutate Ghidra.

Evidence needed to mark COMPLETE: binary decompile plus assembly context for the `0x102` one-pixel fixups, checkbox label/text call, trackbar plaque/value paint, packed-color wrapper, and setup globals; current Rust source comparison with line evidence; stale-doc wording.

Stop conditions: all scoped open questions resolved or explicitly deferred; write only this report plus shared claims; no unverified handoff-critical deltas.

## 1. Overview

The scoped retail controls are active owner-draw Win32 children, not generic widgets. `FUN_0060B950` applies narrow `0x102` fixups after resource/DLU geometry: only trackbar `0x50C` moves up one pixel, only checkboxes `0x54E/0x693/0x696/0x69A` move left one pixel, and `0x69D` is intentionally excluded.

Current Rust has already absorbed the previously reported rect/color deltas: checkbox x `71`, unit-count y `340`, value plaque x/y `483/y-1`, checkbox/trackbar rendering and input paths, and low-byte-red packed text colors are present in the current source.

## 2. Key Offsets / Fields

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| owner-draw record `+0x28` / `[10]` | checkbox text pointer gate; no label draw if null | `OwnerDraw_Checkbox_006163A0`, `0x0061663E..0x00616674` | Yes |
| owner-draw record `+0x64` / `[0x19]` | font pointer passed to `FUN_00621040` | checkbox call `0x00616674`, trackbar call `0x0061E30A` | Yes |
| trackbar record `+0x104` / `[0x41]` | numeric display/plaque enabled flag | `OwnerDraw_Trackbar_0061D950`, decompile around plaque branch | Yes |
| `DAT_00AC18A4` | normal shell label/text packed color `0x0000FFFF` | `FUN_0060F9A0` initialization | Yes |
| `DAT_00AC4624` | normal trackbar primitive caller color before `FUN_006208F0`; border-2 helper substitutes bevel globals | `FUN_0060F9A0`; trackbar calls `0x0061E204/0x0061E269`; `FUN_006208F0` | Yes |
| `DAT_00AC1B98` / `DAT_00AC1B94` | bevel color globals `0x00C5BEA7` / `0x00807A68` for border-2 primitive frames | `FUN_0060F9A0`; `FUN_006208F0` | Yes |

## 3. Core Logic

### 3.1 `0x102` Child-Rect Fixups

Active in YR: Yes. `FUN_0060B950` reads the parent owner-draw record id and child control id, then applies the standard Skirmish branch when parent id is `0x102`.

- `0x50C` only: `CMP ESI,0x102`, `CMP EDI,0x50c`, then `DEC EBP` at `0x0060BE0A..0x0060BE1B`. Result: unit-count trackbar y is resource/DLU y `341` minus one, final `(404,340,128,21)`.
- `0x54E/0x693/0x696/0x69A` only: child-id compares at `0x0060BE20..0x0060BE4A` jump to the x decrement site; `0x0060C065..0x0060C06A` decrements the pending x. Result: first four checkbox control rects use x `71`, while `0x69D` remains x `302`.
- No high-res centering or scale is applied to these scoped controls in this path.

### 3.2 Checkbox Paint/Input

Active in YR: Yes. `FUN_0060F9A0` maps class `Button` with `(style & 3) == 3` to `OwnerDraw_Checkbox_006163A0`. The standard five Skirmish options are initialized by the standard `0x102` setup path in prior settled reports.

`OwnerDraw_Checkbox_006163A0` paints the icon at the control top-left with a fixed `18x18` destination. It advances the label rect by `0x1A` (`26`) before calling `FUN_00621040` with flags `0x04`, so the label is left anchored and vertically centered. The left advance/color gate and call are at `0x0061663E..0x00616674`.

Mouse click and double-click toggle only when local `x < 0x12` and `y < 0x12`. This is active in YR for the standard checkbox controls and is already represented in current Rust source.

### 3.3 Trackbar Paint/Input

Active in YR: Yes. `FUN_0060F9A0` maps class `msctls_trackbar32` to `OwnerDraw_Trackbar_0061D950`. The standard three trackbars are Game Speed `0x529`, Credits `0x511`, and Unit Count `0x50C`.

With numeric display enabled, `OwnerDraw_Trackbar_0061D950` reserves a `0x32` (`50`) px value plaque and computes active width as `(client_width - plaque_width) - 0x0D`; for the standard `128x21` controls this is `65`. The plaque middle uses `trofm.pcx` tiled at local x `client_width - 50 + 1`, y `-1`; left cap `trofl.pcx` is direct-blitted at the same plaque origin, and right cap `trofr.pcx` is right-aligned inside the plaque. Evidence: decompile and assembly around `0x0061DE9C..0x0061E005`.

The thumb uses `trakgrip.pcx` at `control_left + 1 + pixel_offset`, y `control_top`; evidence `0x0061E00C..0x0061E0AD`. The value text rect is `[right - 0x31, top, right, bottom]`, flags `0x05`; evidence `0x0061E29C..0x0061E30A`. The mouse y gate accepts only `mouse_y > bottom - 0x12` and `< bottom`, and mouse x maps through `(mouse_x - 6)` clamped to `[1, right - plaque_width - 0x0C]`, then `((x - 1) * (span + 1)) / active_width`, saturated and step-quantized.

### 3.4 Color Extraction

Active in YR: Yes. `FUN_00621040` treats caller colors as packed source RGB in `0x00BBGGRR` byte order before converting through DirectDraw loss/shift globals. Decompile and assembly `0x00621054..0x006210B1` extract the low byte as red, next byte as green, and third byte as blue. Therefore:

- `0x00000C05` is source RGB `(5,12,0)` for button and trackbar value text.
- `DAT_00AC18A4 = 0x0000FFFF` is source RGB `(255,255,0)` for normal checkbox/static/combo labels.
- `DAT_00AC1B98 = 0x00C5BEA7` is source RGB `(167,190,197)` before DirectDraw conversion; `FUN_006208F0` uses this plus `DAT_00AC1B94` for border-2 primitive bevels.

## 4. INI Keys

| INI key | Source | Value | Effect in scoped controls | Active in YR |
|---|---|---:|---|---|
| `MinMoney` | `ini/rulesmd.ini:3018` | `5000` | credits trackbar min | Yes |
| `Money` | `ini/rulesmd.ini:3019` | `10000` | credits default | Yes |
| `MaxMoney` | `ini/rulesmd.ini:3020` | `10000` | credits max | Yes |
| `MoneyIncrement` | `ini/rulesmd.ini:3021` | `100` | credits step | Yes |
| `MinUnitCount` | `ini/rulesmd.ini:3022` | `0` | unit-count min | Yes |
| `UnitCount` | `ini/rulesmd.ini:3023` | `10` | unit-count default | Yes |
| `MaxUnitCount` | `ini/rulesmd.ini:3024` | `10` | unit-count max | Yes |
| `GameSpeed` | `ini/rulesmd.ini:3026` | `1` | game-speed stored default; visual is inverted by standard setup per prior settled report | Yes |
| `Crates` | `ini/rulesmd.ini:3034` | `yes` | Crates checkbox default | Yes |
| `ShortGame` | `ini/rulesmd.ini:3039` | `yes` | Short Game checkbox default | Yes |

## 5. Integration Points

`FUN_0060F9A0` is the common shell subclass/setup router. It initializes color globals, registers owner procs, and maps `Button` style low bits to checkbox or push-button callbacks and `msctls_trackbar32` to the trackbar callback. Active in YR: Yes for standard Skirmish shell controls; evidence decompile `0x0060F9A0`.

`FUN_0060B950` is invoked as the shell child resize/fixup helper and is active for dialog id `0x102`. Evidence: decompile plus assembly fixup sites above.

`OwnerDraw_Checkbox_006163A0`, `OwnerDraw_Trackbar_0061D950`, `FUN_00621040`, and `FUN_006208F0` are called by the standard Skirmish paint paths. Active in YR: Yes for enabled standard controls; disabled overlay/text branches are Conditional on `WS_DISABLED`.

## 6. Current Rust Implementation Status

Current Rust status: mostly implemented for this slice.

| Surface | Current status |
|---|---|
| Checkbox rects | Matches final geometry: first four x `71`, BuildOffAlly x `302`; tests in `src/ui/skirmish_shell/layout.rs:730..775`. |
| Unit-count trackbar rect | Matches final y `340`; tests in `src/ui/skirmish_shell/layout.rs:695..727`. |
| Plaque/value/thumb helpers | Match scoped formulas: plaque `x + w - 50 + 1`, `y - 1`; value text `right-49`; active width `65`; tests in `src/ui/skirmish_shell/layout.rs:787..810`. |
| Checkbox render/input | Implemented: `cue_i/cce_i` atlas fields and icon render in `src/app_skirmish_shell_render.rs:787..803`; icon-only hit test in `src/ui/skirmish_shell/state.rs:431..437`; stale "missing" claims are no longer true. |
| Trackbar render/input | Implemented: plaque/thumb/rail render in `src/app_skirmish_shell_render.rs:754..834`; mouse y gate/value mapping/drag state in `src/ui/skirmish_shell/state.rs:440..496`; stale "render/input missing" claims are no longer true. |
| Text colors | Current constants match verified source order: `SHELL_BUTTON_TEXT_RGB_00000C05 = [5,12,0]/255` and `SHELL_LABEL_TEXT_RGB = [1,1,0]` in `src/app_skirmish_shell_render.rs:43..45`; tests at `src/app_skirmish_shell_render.rs:2807..2810`. |
| Primitive rail color | Implemented as a pre-rendered RGBA approximation from verified bevel globals in `src/render/skirmish_shell_chrome.rs:214..220` and `:485..545`; final DirectDraw-format screenshot parity remains a non-blocking uncertainty. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0060B950` `0x102` branch | verified | decompile `0x0060B950`; assembly `0x0060BE0A..0x0060BE4A`, `0x0060C065..0x0060C06A` | none |
| checkbox callback mapping | verified | `FUN_0060F9A0` decompile, Button style branch to `OwnerDraw_Checkbox_006163A0` | none |
| trackbar callback mapping | verified | `FUN_0060F9A0` decompile, `msctls_trackbar32` branch to `OwnerDraw_Trackbar_0061D950` | none |
| checkbox label rect/color/click gate | verified | `0x0061663E..0x00616674`, `0x006166EE..0x00616708` | none |
| trackbar plaque/value/thumb geometry | verified | `0x0061DE9C..0x0061E30A` | none |
| packed text source color order | verified | `FUN_00621040`; assembly `0x00621054..0x006210B1` | final capture appearance optional |
| primitive bevel helper source globals | verified | `FUN_0060F9A0`; `FUN_006208F0`; callers `0x0061E204/0x0061E269` | exact retail 16-bit sampled pixels deferred |
| disabled trackbar path in standard offline `0x102` | verified conditional | prior `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`; shared callback decompile | no normal runtime delta |
| current Rust rect/render/input/color status | verified | source scan and line refs in section 6 | none for scoped deltas |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which function applies the first-four checkbox x and unit-count trackbar y fixups? -> FUN_0060B950, active for parent dialog id 0x102.` (evidence: `0x0060B950`; assembly `0x0060BE0A..0x0060BE4A`, `0x0060C065..0x0060C06A`)
- `[RESOLVED] OQ-02 - Are all five checkboxes shifted left? -> No; only 0x54E/0x693/0x696/0x69A are in the 0x102 fixup set; 0x69D is omitted.` (evidence: `0x0060BE20..0x0060BE4A`)
- `[RESOLVED] OQ-03 - Is unit-count y 340 or 341 in the final child rect? -> 340; 0x50C decrements y from the resource/DLU result.` (evidence: `0x0060BE12..0x0060BE1B`)
- `[RESOLVED] OQ-04 - Where is the trackbar value plaque positioned? -> local x = client_width - 50 + 1, y = -1, so 128x21 at x 404 uses plaque x 483 and y rect.y - 1.` (evidence: `OwnerDraw_Trackbar_0061D950`, `0x0061DE9C..0x0061DF04`)
- `[RESOLVED] OQ-05 - Which packed color channel is red? -> The low byte is red; colors are source `0x00BBGGRR`.` (evidence: `FUN_00621040`, assembly `0x00621054..0x006210B1`)
- `[RESOLVED] OQ-06 - Is current Rust still channel-swapped for 0x00000C05? -> No; current source uses `[5,12,0]/255`.` (evidence: `src/app_skirmish_shell_render.rs:43..45`, test `:2807..2810`)
- `[RESOLVED] OQ-07 - Is current Rust still using a muted non-binary label color? -> No for the constant; current source uses `[1,1,0]` for `DAT_00AC18A4 = 0x0000FFFF`.` (evidence: `src/app_skirmish_shell_render.rs:45`, test `:2810`)
- `[RESOLVED] OQ-08 - Are checkbox visuals/input still absent in current Rust? -> No; current source renders PCX icons, label text, and icon-only hit testing.` (evidence: `src/app_skirmish_shell_render.rs:787..803`, `src/ui/skirmish_shell/state.rs:431..437`)
- `[RESOLVED] OQ-09 - Are trackbar visuals/input still absent in current Rust? -> No; current source renders rail/plaque/thumb/value text and implements y gate/value/drag paths.` (evidence: `src/app_skirmish_shell_render.rs:754..834`, `src/ui/skirmish_shell/state.rs:440..496`)
- `[RESOLVED] OQ-10 - Does standard offline 0x102 disable these trackbars? -> No normal runtime disable flow was verified; disabled paint remains conditional callback behavior only.` (evidence: `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`; `OwnerDraw_Trackbar_0061D950` style check)
- `[RESOLVED] OQ-11 - Do scoped INI defaults still match YR md overrides? -> Yes for money/unit/game speed/crates/short game keys in rulesmd.` (evidence: `ini/rulesmd.ini:3018..3039`)
- `[DEFERRED] OQ-12 - Do Rust's pre-rendered RGBA primitive rail pixels exactly match a retail 16-bit DirectDraw capture?` (category: `needs-runtime-debugger`; reason: binary source globals and helper behavior are verified, but this pass did not capture a live retail frame; next-step-if-pursued: sample retail `0x102` trackbar rail pixels and compare against Rust atlas output)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x50C` final rect is `(404,340,128,21)` and first-four checkbox x is `71`; `0x69D` stays x `302` | `FUN_0060B950`; `0x0060BE0A..0x0060BE4A`, `0x0060C065..0x0060C06A` | none observed | `src/ui/skirmish_shell/layout.rs` | Keep exact per-control fixups and tests | 800x600 and 1024x768 layout snapshots keep checkbox/trackbar rects unchanged | Do not apply a blanket shift to all checkboxes or all trackbars |
| Trackbar plaque uses x `right-50+1`, y `top-1`, value text `[right-49,top,right,bottom]`, active width `65` in 128px control | `OwnerDraw_Trackbar_0061D950`, `0x0061DE9C..0x0061E30A` | none observed | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Preserve helper formulas and plaque/thumb rendering | Game Speed/Credits/Unit Count show plaque at x 483 and value text centered in 49px rect | Do not replace with normalized full-width sliders |
| Shell text colors decode low byte as red; normal labels use RGB `(255,255,0)`; dark value/button color is `(5,12,0)` | `FUN_00621040`; `0x00621054..0x006210B1`; `FUN_0060F9A0` globals | none observed for constants; final display capture still optional | `src/app_skirmish_shell_render.rs`, shell text tests | Keep constants in decoded source RGB order | Button/trackbar values use dark olive source color; labels use shell yellow | Do not interpret source colors as `0x00RRGGBB` or tune by eye without screenshot evidence |

Proposed test names:

- `skirmish_option_checkboxes_apply_0102_fixup_x_minus_one` - already present.
- `skirmish_unit_count_trackbar_applies_0102_fixup_y_minus_one` - already present.
- `skirmish_trackbar_plaque_and_value_text_use_native_offsets` - currently covered by `trackbar_geometry_helpers_follow_owner_draw_constants`; split only if a future regression needs a narrower assertion.
- `packed_shell_rgb_00000c05_decodes_as_red_green_blue` - current color test covers the constant; add a decoder-level test if a shared decoder is introduced.
- `skirmish_trackbar_rail_rgba_matches_retail_capture` - future screenshot/pixel test if retail capture is produced.

## Negative Facts / Do Not Do

- Do not claim current Rust checkbox/trackbar render/input is absent. Active in YR: Yes for the binary controls, and current Rust implements the scoped render/input paths. Evidence: `src/app_skirmish_shell_render.rs:787..834`, `src/ui/skirmish_shell/state.rs:431..496`.
- Do not shift BuildOffAlly `0x69D` left with the first four checkboxes. Active in YR: No for that fixup; it is omitted from the `0x102` compare list. Evidence: `0x0060BE20..0x0060BE4A`.
- Do not leave unit-count at raw DLU y `341`. Active in YR: No; `0x50C` takes the `DEC EBP` branch. Evidence: `0x0060BE12..0x0060BE1B`.
- Do not treat packed shell colors as `0x00RRGGBB`; active wrapper uses low byte as red. Evidence: `FUN_00621040`, `0x00621054..0x006210B1`.
- Do not replace scoped trackbar art with `BTN-MINS.SHP` / `BTN-PLUS.SHP` or checkbox art with `bst_*`. Active in YR: No for these standard `0x102` callbacks; active assets are `cue_i/cce_i`, `trakgrip`, `trofl/trofm/trofr`, plus primitive bevel. Evidence: `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_Trackbar_0061D950`.

## Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`: replace current Rust status rows that say checkbox layout/rendering, trackbar rendering/input, or checkbox hit testing are missing with: `Current Rust now exposes final checkbox rects, renders cue_i/cce_i icons and labels, renders trackbar rail/plaque/thumb/value text, and implements icon-only checkbox hits plus trackbar y-gated click/drag behavior. Remaining scoped risk is only final retail-pixel validation of the pre-rendered primitive rail color, not missing widget functionality.`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`: replace implementation-facing color mismatch rows with: `Current Rust constants now decode packed source colors in the verified low-byte-red order: 0x00000C05 -> RGB (5,12,0), and DAT_00AC18A4/0x0000FFFF -> RGB (255,255,0). Runtime screenshot comparison may still validate final display-format appearance, but the source constant channel order is no longer a current Rust mismatch.`
- `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`: replace implementation-facing mismatch wording for checkbox x and unit-count y with: `Current Rust now applies the verified 0x102 fixups: first-four checkbox rects are x=71, BuildOffAlly remains x=302, and unit-count trackbar is y=340 at both 800x600 and preserved high-res resource coordinates.`

## Sources

- Ghidra read-only decompile: `FUN_0060B950 @ 0x0060B950`, `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`, `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_006208F0 @ 0x006208F0`.
- Ghidra assembly contexts: `0x0060BE0A`, `0x0060BE20`, `0x0060C065`, `0x0061663E`, `0x00616674`, `0x0061DE9C`, `0x0061E00C`, `0x0061E1F3`, `0x0061E30A`, `0x00621054`.
- Prior docs: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_TEXT_PIXEL_CONTRACT_HOTFIX_SCAN_GHIDRA_REPORT.md`, `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHROME_CONTROL_ART_SUBSTITUTIONS_GHIDRA_REPORT.md`, `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust checked: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
