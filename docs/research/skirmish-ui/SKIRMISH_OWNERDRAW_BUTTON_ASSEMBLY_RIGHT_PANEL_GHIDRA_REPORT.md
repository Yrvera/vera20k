# Skirmish Owner-Draw Button Assembly Right Panel - Ghidra Research Report

## Superseded Asset-Family Correction - 2026-05-24

For standard Skirmish setup sidebar Start Game `0x617`, Choose Map `0x5AA`, and
Back `0x5C0`, this report's generic PCX path conclusion is superseded. The
corrected classifier recheck proves these three right-panel buttons are
owner-draw type `1` and draw `SDBTNANM.SHP` frames `2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` for the
current asset-family contract.

**Address(es):** `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `RightPanel__Draw @ 0x0072E450`, `WM_PAINT_Handler @ 0x00621E90`, `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006ACEE0 @ 0x006ACEE0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** active standard Yuri's Revenge offline Skirmish dialog `0x102` right-panel owner-draw push-button assembly for Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0`: child placement against right-panel chrome/lower strip, default PCX cap/middle/right assembly, clipping/tiling/source phase, text rect and pressed offsets, disabled/active visual state, and click state only where it affects visual/input.
**Non-Scope:** Choose Map modal `0x6B` internals, Create Random Map modal button visuals, Start launch validation semantics except disabled Start visual effect, combo/check/trackbar/listbox drawing, runtime screenshot/audio capture, and Rust implementation.
**Confidence:** High for the scoped binary route, geometry, composition, and current Rust deltas; Medium only for final display-format disabled text pixels because runtime DirectDraw format capture was not performed.
**Active in YR:** Yes for `0x102` Start/Choose/Back on the normal offline Skirmish path. Conditional only for disabled Start and sound/pressed transitions.

## 0. Working Notes

**Target question:** How does `gamemd.exe` assemble and place the Start, Choose Map, and Back owner-draw buttons inside the Skirmish right panel, including PCX pieces, middle tiling/source phase, text rects, active/disabled/pressed visual state, and chrome/lower-strip alignment?

**Non-goals:** Do not re-investigate settled `bue_*30`/`bde_*30` names except to verify no contradiction; do not cover modal `0x6B` Create Random Map button; do not modify Rust, INI, in-repo docs, Ghidra state, or swarm claim files.

**Evidence needed to mark COMPLETE:** decompile plus assembly/context for active dialog route, owner-draw callback install, Start/Choose/Back resize helpers, right-panel chrome draw order, PCX cap/middle/right assembly, `FUN_006BA3E0` source phase, text rect/pressed offset, disabled overlay, click-state gate, and current Rust scan of affected render/layout/input surfaces.

**Stop conditions:** stop after the three `0x102` right-panel owner-draw buttons are exhausted; record Random/Create Random Map as out of scope if no separate `0x102` owner-draw right-panel button exists; ship no unresolved open questions.

## 1. Overview

The active offline Skirmish setup path creates dialog `0x102`; its Start Game, Choose Map, and Back controls are ordinary Win32 Button children that are subclassed to `OwnerDraw_Button_00612B70`. The parent right-panel chrome is painted first (`SDTP`, repeated `SDBTNBKGD`, optional `SDBTNANM` frame 10, `SDBTM`, then `LWSCRN*` lower strip), and the child buttons paint over that chrome in their moved child rectangles.

At 800x600, the scoped button HWND rects are Start `(644,241,156,42)`, Choose `(644,283,156,42)`, and Back `(644,535,156,42)`. The visible PCX strip is the native 30px-high `bue_*30`/`bde_*30` cap/middle/right assembly vertically centered in those 42px controls; the middle piece is tiled by `FUN_006BA3E0` with centered source phase before modulo copying.

## 2. Key Offsets / State Fields

| Field / value | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Button style low bits `0x0B` | `FUN_0060F9A0` installs `OwnerDraw_Button_00612B70` for Button controls whose style satisfies `(style & 0x0B) == 0x0B` | decompile `FUN_0060F9A0`; assembly `0x0060FE78..0x0060FE8B` from prior sound report | Yes |
| owner record `+0xB0` / `piVar17[0x2C]` | visual mode; `0` takes the default PCX cap/middle/right path used here | `OwnerDraw_Button_00612B70` decompile before `0x00613240` | Yes for these buttons |
| owner record `+0x14` / `piVar17[5]` | custom image pointer; nonzero bypasses default PCX assembly and text drawing condition | `OwnerDraw_Button_00612B70` decompile | No for normal Start/Choose/Back |
| owner record `+0x28` / `piVar17[10]` | text-present gate; zero skips label draw | `0x00613568..0x00613578` | Yes when labels exist |
| owner record `+0x64` / `piVar17[0x19]` | label pointer passed to `FUN_00621040` | `0x006135D1..0x006135EE` | Yes |
| owner record byte `+0xBC` | suppress/blocked byte; nonzero skips paint work and mouse-down sound path | paint early-out in decompile; mouse assembly `0x0061374B..0x00613753` | Conditional |
| owner byte `+0xC5` | timer/custom-message visual byte for alternate SHP modes; not used in the default `bue/bde` PCX filename block | decompile `OwnerDraw_Button_00612B70`; prior pixel report | Conditional; no effect on this default PCX path |
| `WS_DISABLED` `0x08000000` | forces released state char `'u'`, suppresses paint-transition click state, and applies alpha overlay `0x80` after drawing | assembly `0x00613254..0x00613262`, `0x006135F3..0x0061361B` | Conditional, e.g. Start validation failure |
| `DAT_00AC18A4 = 0x0000FFFF` | normal enabled label source color, interpreted by `FUN_00621040` as yellow | `FUN_0060F9A0` decompile initialization; text call `0x006135DD..0x006135EE` | Yes |

## 3. Core Logic

### 3.1 Active Route and Random Non-Participation

`FUN_006AE2C0` is the live offline Skirmish setup launcher: it calls the Skirmish shell loader, creates dialog `0x102`, pumps until result `0x617` or `0x5C0`, and returns true only for `0x617`. `FUN_006AE3F0` delegates messages to the common shell proc first and sends `WM_COMMAND` ids to `FUN_006ACEE0`.

The right-panel owner-draw push buttons in dialog `0x102` are exactly Start `0x617`, Choose Map `0x5AA`, and Back `0x5C0`. There is no separate Random owner-draw button in the `0x102` right panel. Random appears as combo-list text and as the Create Random Map action inside the Choose Map modal `0x6B`, which is outside this report's claimed slice.

Active in YR: Yes for Start/Choose/Back; No for a separate right-panel Random owner-draw button. Evidence: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006ACEE0` decompile; `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md` RT_DIALOG `0x102` child matrix.

### 3.2 Placement Against Right-Panel Chrome

Start `0x617` and Choose `0x5AA` route through the `SDBTNANM` snap helper `FUN_0060B000`; Back `0x5C0` routes through the bottom helper `FUN_0060B350`. At 800x600 the results are:

| Button | Helper | Final rect | Chrome relation |
|---|---|---:|---|
| Start `0x617` | `FUN_0060B000` | `(644,241,156,42)` | right edge flush with screen/right panel edge; snapped to a 42px `SDBTNBKGD` tile row |
| Choose Map `0x5AA` | `FUN_0060B000` | `(644,283,156,42)` | next 42px tile row below Start |
| Back `0x5C0` | `FUN_0060B350` | `(644,535,156,42)` | last complete 42px tile row directly above `SDBTM` |

The button x is `screen_w - centered_offset_x - 156`, not the generic 162px DLU right-anchor rect. At 800, that is `644`; at 640, `484`; at 1024, `756`.

Active in YR: Yes. Evidence: `FUN_0060B000` decompile; `FUN_0060B350` decompile; dispatcher assembly `0x0060C1B0..0x0060C1C8` and `0x0060C213..0x0060C227`; complete child matrix report.

### 3.3 Parent Chrome and Lower Strip Order

`WM_PAINT_Handler` mode `1` calls `RightPanel__Draw` before `Background_Overlay` and before the cached parent surface is blitted to the destination. `RightPanel__Draw` draws:

1. `SDTP`
2. repeated `SDBTNBKGD`
3. optional repeated `SDBTNANM` frame `10` only when its boolean parameter is zero
4. `SDBTM`
5. width-selected lower strip: `LWSCRNS` at 640, otherwise `LWSCRNL`

The child owner-draw buttons are not the parent chrome. They paint over the already-composed shell surface in their own child windows.

Active in YR: Yes. Evidence: `WM_PAINT_Handler` decompile; `RightPanel__Draw` decompile; assembly contexts `0x00621FFE`, `0x0062211B`, `0x0072E547`, `0x0072E594`, `0x0072E60D`, `0x0072E68C`, `0x0072E71F`.

### 3.4 PCX Family, State, and Vertical Placement

The default PCX path seeds height suffix candidates `24` and `30`; the 42px Start/Choose/Back controls select suffix `30`. Released art formats `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`; pressed art formats `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`.

The first state char is `'u'` unless the Button pressed bit is set, in which case it becomes `'d'`. `WS_DISABLED` forces it back to `'u'`; disabled does not select `bud_*` here. The second char in the default filename path is hardcoded `'e'`.

The 30px PCX strip is vertically centered inside the 42px child rect and then shifted down by `+2` while pressed. At 800x600 this yields Start art y `247/249`, Choose `289/291`, and Back `541/543` for released/pressed states.

Active in YR: Yes. Evidence: state char assembly `0x00613240..0x00613262`; suffix table assembly `0x006132B9..0x006132F9`; art y assembly `0x00613394..0x006133AE`; filename format calls `0x006133B2..0x006134DA`.

### 3.5 Cap/Middle/Right Destination Geometry and Tiling Source Phase

The cap/middle/right assembly is:

- left cap: direct blit at button x/y, native left PCX width (`7` for the verified `30` family)
- middle: destination starts at `x + left_w`; width is `button_width - right_w`, not `button_width - left_w - right_w`
- right cap: direct blit at `x + button_width - right_w`, native right PCX width (`10`)

Because the middle width includes the right-cap area, the right cap overwrites the final `left_w` pixels of the middle span. For a 156px button with 7px left and 10px right, the middle destination is `146px` wide from x `651` through x `796`, then the right cap covers x `790..799`.

`FUN_006BA3E0` locks source and destination, computes source offsets as `(source_size - dest_size) / 2`, masks negative offsets to zero, and reads `(source_start + column) % source_width` and `(source_start_y + row) % source_height`. This is clipping/tiling with centered source phase, not stretching and not plain x=0 tiling when the source is wider than the destination.

Active in YR: Yes. Evidence: left blit assembly `0x00613441`; middle rect/call assembly `0x0061348D..0x006134C4`; right rect/blit assembly `0x0061351D..0x0061355D`; `FUN_006BA3E0` decompile modulo-copy loop.

### 3.6 Text Rect, Alignment, and Pressed Offset

Text is drawn after art and only when the default PCX path is active (`owner +0x14 == 0`) and a text pointer exists (`owner +0x28 != 0`). The rect passed to `FUN_00621040` is:

| State | Left | Top | Right | Bottom |
|---|---:|---:|---:|---:|
| released | `x` | `y + 1` | `x + w - 2` | `y + h` |
| pressed | `x + 2` | `y + 5` | `x + w - 2` | `y + h` |

The effective flags slot is `0x05` for horizontal center plus vertical center in the recovered `FUN_00621040` signature; a nearby pushed `0x0C` is not the flags argument used by that function. The enabled color source is `0x0000FFFF` from `DAT_00AC18A4`.

Active in YR: Yes. Evidence: text rect assembly `0x00613591..0x006135CD`; call sequence `0x006135DD..0x006135EE`; `FUN_00621040` prior caller-contract report.

### 3.7 Disabled and Click-State Visual/Input Boundary

Disabled Start uses released art, applies the same text/art path, then overlays alpha `0x80` on the button rect. Disabled style also prevents the paint-time pressed-state click transition by forcing the state char back to `'u'`.

For input, mouse down and double-click are still handled in the owner-draw subclass before command dispatch. If owner byte `+0xBC` is nonzero, the callback returns early; otherwise it plays `GUIMainButtonSound` and continues to the previous Button proc so the standard Button pressed bit and later `WM_COMMAND` behavior occur. The visual state is therefore driven by the standard Button pressed bit, not by the later Start/Choose/Back command action.

Active in YR: Yes, conditional on enabled/disabled and pressed state. Evidence: mouse assembly `0x0061374B..0x00613776`; paint transition assembly `0x00613254..0x0061329B`; disabled alpha assembly `0x006135F3..0x0061361B`; `FUN_006AE3F0`/`FUN_006ACEE0` command split.

## 4. INI Keys

No INI key controls the scoped button placement, PCX filenames, cap geometry, middle tiling, text rect, or disabled alpha. These are binary callback/layout behaviors and retail asset dimensions.

| INI key | Effect in this slice | Active in YR |
|---|---|---|
| `[AudioVisual] GUIMainButtonSound` | mouse-down sound only; relevant here only because the same mouse-down path continues into visual pressed state | Yes, conditional |
| `[AudioVisual] GenericClick` | paint-time released-to-pressed transition sound only; relevant here only as a visual state transition marker | Yes, conditional |

## 5. Current Rust Implementation Status

| Rust surface | Current status | Evidence |
|---|---|---|
| `src/ui/skirmish_shell/layout.rs` | Uses `SDBTNANM_W=156`, `SDBTNANM_H=42`; `owner_draw_button_snap_rect` and `back_rect` match the verified 800/1024 placement formulas | read-only scan; `rg` hits around constants and helpers |
| `src/ui/skirmish_shell/state.rs` | `OwnerDrawButton` contains only Start, Choose, Back; hit-test and action mapping use the three layout rects | read-only scan lines for enum, `hit_test_owner_draw_button`, `action_for_owner_draw_button` |
| `src/app_skirmish_shell_render.rs` `skirmish_shell_semantic_draw_order` / `build_skirmish_shell_instances` | Emits right-panel stack, lower strip, optional parent background, then three owner-draw buttons; matches the parent/child visual layering boundary | read-only scan |
| `push_button_30` / `button_art_y` | Uses `bue/bde` 30-family atlas entries, native entry height, centered art y, and pressed +2 y when enabled | read-only scan |
| `build_button_segments` | Preserves the right-cap overlap destination rule by making middle destination `rect.w - right_w`; remaining mismatch is source phase: partial middle UV starts at source x=0 instead of `max(0,(src_w-dest_w)/2)` | read-only scan |
| `button_text_rect` / `push_button_label_draw` | Matches fixed right/bottom text rect and yellow centered enabled text | read-only scan |
| disabled Start visual | Disabled alpha exists in `push_button_30`, but current call sites pass `disabled=false` for Start/Choose/Back in `build_skirmish_shell_instances`; disabled Start validation visual remains unchecked/missing for this surface | read-only scan |
| click-state sound/input | Setup `0x102` press and paint-transition sound state are implemented in dirty current Rust per existing sound report; this report did not re-audit audio beyond the visual/input boundary | prior sound docs plus `rg` hits |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish dialog creation | verified | `FUN_006AE2C0` decompile | none |
| Common proc and command boundary | verified | `FUN_006AE3F0`, `FUN_006ACEE0` decompile | command semantics beyond visual/input out of scope |
| Owner-draw callback install | verified | `FUN_0060F9A0`; style docs | none |
| Start/Choose placement | verified | `FUN_0060B000`; assembly `0x0060C1B0..0x0060C1C8` | none |
| Back placement | verified | `FUN_0060B350`; assembly `0x0060C213..0x0060C227` | none |
| Right-panel/lower-strip chrome order | verified | `WM_PAINT_Handler`, `RightPanel__Draw`; assembly contexts listed above | first-paint frame-10 runtime gate remains parent-paint concern, not button assembly |
| Default PCX names and disabled `bud_*` exclusion | verified | `0x00613240..0x006134DA` | none |
| Cap/middle/right destination geometry | verified | `0x00613441`, `0x0061348D..0x006134C4`, `0x0061351D..0x0061355D` | none |
| Middle source phase and modulo tiling | verified | `FUN_006BA3E0` decompile | runtime screenshot optional |
| Text rect / pressed offsets / color source | verified | `0x00613591..0x006135EE`; `FUN_00621040` prior report | disabled final RGB runtime screenshot optional |
| Separate Random owner-draw right-panel button | verified negative | RT_DIALOG `0x102` matrix; `FUN_006ACEE0` Choose path opens modal `0x6B` | modal Create Random Map button out of scope |
| Current Rust scan | verified for named surfaces | read-only `rg`/file scan | no patch made |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path active in standard YR? -> Yes; `FUN_006AE2C0` creates/pumps dialog `0x102`, and `FUN_0060F9A0` routes the scoped Button controls to `OwnerDraw_Button_00612B70`.` (evidence: `FUN_006AE2C0`, `FUN_0060F9A0`)
- `[RESOLVED] OQ-02 - Which right-panel buttons are in scope? -> Start `0x617`, Choose Map `0x5AA`, and Back `0x5C0`; no separate `0x102` right-panel Random owner-draw button exists.` (evidence: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `FUN_006ACEE0`)
- `[RESOLVED] OQ-03 - How do Start/Choose/Back align to the right panel? -> They use `SDBTNANM` 156x42 rects at right edge; Start/Choose snap to tile rows and Back is the last complete row above `SDBTM`.` (evidence: `FUN_0060B000`, `FUN_0060B350`)
- `[RESOLVED] OQ-04 - Does parent chrome draw before or after buttons? -> Parent right-panel/lower-strip chrome draws in common parent paint; owner-draw child buttons paint over it in their child rects.` (evidence: `WM_PAINT_Handler`, `RightPanel__Draw`, `FUN_0060F9A0`)
- `[RESOLVED] OQ-05 - Which PCX asset family is used? -> `bue_li/mi/ri30` released and `bde_li/mi/ri30` pressed; disabled still uses released `bue`, not `bud`.` (evidence: `0x00613240..0x006134DA`)
- `[RESOLVED] OQ-06 - Are cap/middle/right scaled? -> Caps are direct native blits; middle is tiled/modulo-copied with centered source phase.` (evidence: `0x00613441`, `0x006134C4`, `FUN_006BA3E0`)
- `[RESOLVED] OQ-07 - What are text rect and pressed offsets? -> Released text uses top+1/right-2; pressed changes only left+2/top+5 while right/bottom stay fixed.` (evidence: `0x00613591..0x006135CD`)
- `[RESOLVED] OQ-08 - What visual changes on disabled? -> State char is forced to released, label/art draw, then alpha overlay `0x80` applies over the button rect.` (evidence: `0x00613254..0x00613262`, `0x006135F3..0x0061361B`)
- `[RESOLVED] OQ-09 - Does click sound determine command action? -> No; mouse-down sound and pressed visual state occur in the child subclass before the standard Button proc produces parent `WM_COMMAND`.` (evidence: `0x0061374B..0x00613776`, `FUN_006AE3F0`)
- `[RESOLVED] OQ-10 - What is the current Rust delta? -> Current layout, art y, pressed text rect, draw order, and destination overlap match; middle source phase and disabled Start call-site state remain the scoped deltas.` (evidence: read-only Rust scan)
- `[DEFERRED] OQ-11 - Pixel-perfect disabled text/display-format output.` (category: `needs-runtime-debugger`; reason: binary proves disabled color conversion and alpha overlay, but final 16-bit display-format pixel values need runtime capture; next-step-if-pursued: capture retail Start validation disabled frame)
- `[DEFERRED] OQ-12 - Modal `0x6B` Create Random Map button assembly.` (category: `out-of-scope`; reason: user target is right-panel `0x102`; next-step-if-pursued: separate modal owner-draw button assembly report)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Middle PCX samples from `max(0,(src_w-dest_w)/2)` and wraps modulo over source while preserving the overlapping middle destination. | `FUN_006BA3E0` decompile; call `0x006134C4`; destination assembly `0x0061348D..0x0061355D` | mismatch | `src/app_skirmish_shell_render.rs` `build_button_segments` / middle UV construction | Keep current middle destination width/right-cap overwrite, but offset the middle UV origin for centered source phase. | At 800x600, Start/Choose/Back middle strip begins at the same source phase as retail for both `bue_mi30` and `bde_mi30`. Proposed test: `skirmish_button_middle_tile_uses_centered_source_phase_800`. | Do not "fix" phase by removing the verified 7px right-cap overwrite. |
| Start/Choose/Back are 156x42 `SDBTNANM` child rects aligned to right-panel tile rows, not generic 162x37 right-anchor controls. | `FUN_0060B000`, `FUN_0060B350`; dispatcher assembly `0x0060C1B0..0x0060C227` | none observed | `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs` | Preserve current 800/1024 snap rects and hit-test identity. | `compute_layout(800,600)` returns Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`; hit-test uses the same rects. Proposed test: `skirmish_ownerdraw_buttons_align_to_sdbtnanm_right_panel_rows`. | Do not revive stale `(635,242,162,37)` / `(635,286,162,37)` rows. |
| Owner-draw buttons paint over parent right-panel/lower-strip chrome after common parent composition; Back sits in the last complete tile row directly above `SDBTM`. | `WM_PAINT_Handler`, `RightPanel__Draw`, `FUN_0060B350` | none observed for order | `src/app_skirmish_shell_render.rs` `skirmish_shell_semantic_draw_order`, `build_skirmish_shell_instances` | Preserve right-panel stack/lower strip before owner-draw buttons. | Semantic order starts with right-panel stack and lower strip, then owner-draw button roles after optional parent background. Proposed test: `skirmish_buttons_draw_over_right_panel_chrome_after_lower_strip`. | Do not treat button PCX pieces as part of `RightPanel__Draw` frame 10 overlay. |
| Disabled Start must use released art plus alpha overlay, not `bud_*`, and should suppress pressed visual/click transition. | `0x00613254..0x00613262`, `0x006135F3..0x0061361B`, `0x00613264..0x0061329B` | partially missing/unchecked: current button call sites pass `disabled=false` | `src/app_skirmish_shell_render.rs` `push_button_30` call sites; future Start validation UI state | Feed disabled Start state into the renderer when native would disable `0x617`; keep released art and apply `0x80`-equivalent alpha. | Force a Start validation failure: Start visibly dims in released position and never shows pressed/down art while disabled. Proposed test: `skirmish_start_disabled_uses_released_button_alpha_overlay`. | Do not switch disabled Start to `bud_*`; `bud_*` is not the default path for these buttons. |
| Text rect/color contract is fixed-right/bottom with pressed left+2/top+5 and enabled yellow `0x0000FFFF`. | `0x00613591..0x006135EE`; `FUN_0060F9A0`; `FUN_00621040` prior report | none observed | `src/app_skirmish_shell_render.rs` `button_text_rect`, `push_button_label_draw` | Keep current text rect and enabled yellow source color. | Press Start: text centers in `(x+2,y+5,right=x+w-2,bottom=y+h)`, not in a shifted full-width rect. Proposed test: `skirmish_button_pressed_text_keeps_binary_right_bottom_edges`. | Do not use dark `0x00000C05` for enabled labels; do not interpret the adjacent pushed `0x0C` as the flags slot. |

## 9. Negative Facts / Do Not Do

- Do not implement a separate Random owner-draw button in the Skirmish `0x102` right panel. Active in YR: No. Evidence: RT_DIALOG `0x102` child matrix has no such owner-draw Button; `FUN_006ACEE0` reaches random-map behavior only through the Choose Map modal path.
- Do not use `bud_*30.pcx` for disabled Start/Choose/Back on this path. Active in YR: No. Evidence: `WS_DISABLED` forces state char `'u'` at `0x00613254..0x00613262`, and the second filename char remains `'e'`.
- Do not stretch the 30px PCX strip to the full 42px child rect. Active in YR: No. Evidence: height suffix selection and vertical centering at `0x006132B9..0x006133AE`.
- Do not tile the middle from source x=0 when the source is wider than the destination. Active in YR: No. Evidence: `FUN_006BA3E0` centered source offset before modulo addressing.
- Do not draw Start/Choose as stale generic 162x37 right-anchor rects. Active in YR: No for current verified `0x102` Button metadata. Evidence: `FUN_0060B000` and complete child matrix.
- Do not invent hover/focus PCX variants for these default PCX buttons. Active in YR: No for this default path. Evidence: focus/enable messages return without selecting alternate `b*` filenames; timer byte `+0xC5` is absent from the default filename block.
- Do not move command action to mouse down just because mouse-down sound occurs there. Active in YR: No. Evidence: child callback continues to the prior Button proc; `WM_COMMAND` is later handled by `FUN_006AE3F0`/`FUN_006ACEE0`.

## 10. Remaining Uncertainty

- Runtime screenshot capture would still be useful for final disabled text/display-format pixels and alpha blend appearance, but the binary route and alpha value are verified.
- This report does not claim modal `0x6B` Use Map/Create Random Map/Cancel button assembly, even though those modal buttons reuse the same owner-draw callback family.
- Exact retail PCX checksums/dimensions were inherited from prior asset reports; the binary formula is independent of those checksums, and the scoped remaining Rust delta is source phase rather than asset identity.

## 11. Stale Docs / Follow-Up Docs

- `docs/research/traces/SKIRMISH_START_CHOOSE_BACK_OWNER_DRAW_BUTTONS_800X600_TRACE.md`: replace rows claiming current Start/Choose rects `(635,242,162,37)` and `(635,286,162,37)` with "Current verified standard `0x102` Start and Choose owner-draw rects are `(644,241,156,42)` and `(644,283,156,42)` via `FUN_0060B000`; Back is `(644,535,156,42)` via `FUN_0060B350`."
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`: replace stale current-Rust status with "Current Rust now matches snap rects, native 30px vertical centering, pressed +2 art movement, fixed-right/bottom text rects, enabled yellow text, and destination right-cap overlap. Remaining scoped visual mismatch: middle PCX source phase must use `FUN_006BA3E0`'s centered crop offset before modulo tiling."
- `docs/research/traces/SKIRMISH_BUTTON_TEXT_RECTS_PRESSED_OFFSETS_800X600_TRACE.md`: keep the binary text-rect facts but remove any current-Rust FAIL wording for fixed right/bottom text rects if still present.

## Sources

- Fresh read-only Ghidra decompile / context: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `FUN_0060B1D0 @ 0x0060B1D0`, `RightPanel__Draw @ 0x0072E450`, `WM_PAINT_Handler @ 0x00621E90`, `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_006ACEE0 @ 0x006ACEE0`, `VocClass__PlayAtPos @ 0x00750920`.
- Fresh assembly contexts: `0x00613240`, `0x006132B9`, `0x00613394`, `0x00613441`, `0x0061348D`, `0x006134C4`, `0x0061351D`, `0x00613591`, `0x006135F3`, `0x0061374B`, `0x0060C1B0`, `0x0060C213`, `0x0072E547`, `0x0072E594`, `0x0072E60D`, `0x0072E68C`, `0x0072E71F`.
- Prior docs reconciled: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`, `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`, `traces/SKIRMISH_START_CHOOSE_BACK_OWNER_DRAW_BUTTONS_800X600_TRACE.md`, `traces/SKIRMISH_OWNER_DRAW_BUTTON_PRESS_RELEASE_SOUND_TRACE.md`.
- Rust read-only scan: `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`.
