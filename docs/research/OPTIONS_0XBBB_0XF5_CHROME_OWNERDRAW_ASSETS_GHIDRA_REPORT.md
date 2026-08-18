# Options 0xBBB / 0xF5 Chrome Owner-Draw Assets - Ghidra Report

**Date:** 2026-06-02  
**Target:** `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS`  
**Investigation mode:** exhaustive-slice  
**Primary addresses:** `0x004E1D00`, `0x004E1FE0`, `0x0060F9A0`, `0x0060A330`,
`0x0060C0C0`, `0x00612B70`, `0x006163A0`, `0x0061D950`, `0x006153E0`  
**Report status:** COMPLETE

## Working Notes

- **Target question:** Which live owner-draw routes, assets, frames, palettes, draw
  order, and rect anchors paint Options dialog `0xBBB` and `0xF5`, and does this
  refute the stale `0x102`/MNBTTN button assumption?
- **Non-goals:** Do not re-investigate DLU conversion, INI persistence, modal pump,
  skirmish `0x102` full behavior, or implement Rust.
- **Evidence needed to mark COMPLETE:** live `0xBBB`/`0xF5` entry proof, owner-draw
  type routing, button/checkbox/trackbar/static paint paths, SHP/PCX/PAL asset and
  frame evidence, draw-order/rect-anchoring evidence, and `0xF5` deltas with
  stronger-than-decompiler support.
- **Stop conditions:** material questions resolved or explicitly deferred, zero-add
  pass over primary routes, this one report written, and no Rust or unrelated docs
  modified.

## Summary

The stale broad assumption is only partly true. Options uses the same shell
subclass framework as skirmish `0x102`, and its trackbars, checkboxes, and statics
use the same callback families. The active-game `0xBBB` right-column buttons do
**not** use the skirmish type-1 `SDBTNANM` button art, the generic PCX button
family, or `MNBTTN`. In active scenario context they route through owner-draw
button **type 2**, which draws `DAT_00B0F9EC` through `SIDEBAR.PAL`.

The important correction is asset identity: `DAT_00B0F9EC` is loaded from
`SIDEBTTN.SHP`, not `SIDE2B.SHP`. `SIDE2B.SHP` is loaded later into a different
global (`0x00B0FA00`) and is not read by `OwnerDraw_Button_00612B70` type 2.

`0xF5` is the non-active/shell Options resource. It lacks active-only buttons
`0x52C` and `0x52D`; its verified close button is Back `0x686`. In the ordinary
non-active shell route, Back uses owner-draw type 1: `SDBTNANM.SHP` plus the
`FUN_0072E2C0` palette/convert path. `0xF5` also has wider slider rects and the
shell-only `0x50F`, `0x51A`, and `0x71C` controls already documented by the
template/parser slice.

## Liveness And Resource Selection

| Finding | Active in YR | Evidence |
|---|---:|---|
| `OptionsClass__ShowInGameDialog @ 0x004E1D00` selects RT_DIALOG `0xBBB` when byte `0x00A8E9A0 == 1`, else `0xF5`, and passes dialog proc `0x004E1FE0`. | Yes / conditional on active byte | Decompile `0x004E1D00`; assembly `0x004E1D2A..0x004E1D47` reads the byte, compares to `1`, loads `ECX=0xBBB` or `0xF5`, then calls the shell dialog factory. |
| Own proc `0x004E1FE0` delegates first to common shell proc `FUN_00622B50`, so the standard shell init/subclass path applies to both resources. | Yes | Decompile `0x004E1FE0` shows the first non-trivial branch is the common-shell delegate. Existing `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md` verifies `FUN_00622B50 -> FUN_0060F4B0 -> EnumChildWindows(..., FUN_0060F9A0, ...)`. |
| `0xF5` has no `0x52C`/`0x52D`; it adds shell-only `0x50F`, `0x51A`, `0x71C` and wider `0x529/0x52A/0x52B` slider rects. | Yes for non-active/shell resource | Prior template proof in `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`, lines reporting `0xF5` control set and explicit absence of `0x52C`/`0x52D`. |

DLU-to-pixel conversion is not re-opened here. This report relies on the settled
`DS_SETFONT` / MS Sans Serif 8pt / `baseX=6`, `baseY=13` result in
`DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`.

## Owner-Draw Route Matrix

| Control family | Scoped controls | Route | Asset path | Active in YR |
|---|---|---|---|---|
| Active `0xBBB` buttons | `0x52C`, `0x52D`, `0x686` | Button style `(style & 0x0B) == 0x0B` -> `OwnerDraw_Button_00612B70`; `FUN_0060A330` writes record `+0xB0 = 2` when scenario-active predicate is nonzero and the allow-list predicate matches. | `SIDEBTTN.SHP` via `DAT_00B0F9EC`; `SIDEBAR.PAL` via `FUN_0072F4B0` / `DAT_00B0FBE8`; frames 0/1/2. | Yes for active-game Options over active scenario; conditional on `FUN_0069BBE0()!=0`. |
| Non-active `0xF5` Back | `0x686` | Same Button callback; non-scenario classifier writes `+0xB0 = 1` for the Back allow-list. | `SDBTNANM.SHP` via `0x00B0FAC4`; `FUN_0072E2C0 -> DAT_00B0FBDC` (`SDBTNANM.PAL` per prior frame reports); frames 2/4/3. | Yes for shell/non-active Options. |
| MNBTTN type 3 | none in `0xBBB`/`0xF5` | `FUN_00609E20` does not include dialog `0xBBB` or `0xF5`; therefore `FUN_0060A330` does not write `+0xB0 = 3` for scoped Options controls. | Not used. | No for this target. |
| Checkboxes | `0x601`, `0x602`, `0x604`; `0xF5` also `0x51A` | Button style `(style & 3) == 3` -> `OwnerDraw_Checkbox_006163A0`. | Default PCX icons `cue_i.pcx` / `cce_i.pcx`; variants only if custom variant messages are sent, which `0x004E1FE0` does not do. | Yes for present controls; `0x51A` is shell-resource-only in this path. |
| Trackbars | `0x529`, `0x52A`, `0x52B`; `0xF5` also `0x50F` | class `msctls_trackbar32` -> `OwnerDraw_Trackbar_0061D950`. | `trofl.pcx`, `trofm.pcx`, `trofr.pcx`, `trakgrip.pcx`, plus primitive bevel/text. | Yes for present controls; `0x50F` is shell/inactive only. |
| Statics/text | title `0x694`, labels, `0x695`, `0xF5` `0x71C` | class `Static` -> `OwnerDraw_Static_006153E0`. | Normal text path uses `FUN_00621040` with shell text colors; image/static SHP/PCX subpaths exist but no Options proc message activates them here. | Yes for text statics; `0x71C` visible image behavior remains deferred. |

## Button Classifier And Negative Type-3 Proof

`FUN_0060F9A0` installs the standard shell button callback for `Button` controls
whose low style bits satisfy `(style & 0x0B) == 0x0B`. Assembly context at
`0x0060FE78..0x0060FE8B` shows the style mask/compare and callback selection.

`FUN_0060A330` then sets the button paint type:

- no active scenario (`FUN_0069BBE0()==0`): matching shell buttons write
  record `+0xB0 = 1`; assembly context includes the type-1 write at
  `0x0060A47C`.
- active scenario (`FUN_0069BBE0()!=0`): matching shell buttons write
  record `+0xB0 = 2`; assembly context includes the type-2 write at
  `0x0060A581`.
- type 3 requires `FUN_00609E20` after the type-1/type-2 checks. Decompile and a
  raw byte scan of `0x00609E20` show dialog IDs such as `0xCE`, `0x120`,
  `0x121`, etc., but no `0xBBB` or `0xF5`. Active in YR: No for this target.

The allow-list sources are:

- `FUN_00608CD0`: includes `0xBBB`/`0xF5` title `0x694`; includes `0xF5`
  `0x71C`; includes active `0xBBB` buttons `0x52C` and `0x52D`. Assembly context
  around `0x00608D60..0x00608D6C` shows `0xBBB`/`0xF5` in the shared static
  branch; context around `0x006092AE..0x006092C1` shows `0xBBB` with `0x52C`
  and `0x52D`.
- `FUN_00609730`: includes `0xBBB` and `0xF5` in the broad Back-button block and
  returns true for child `0x686`; assembly context around `0x00609814..0x00609820`
  shows both dialog IDs.

## Button Assets, Frames, And Draw Order

`OwnerDraw_Button_00612B70` reads the record paint type at record `+0xB0`
(`piVar17[0x2C]`) and selects one of the following paint branches:

| Type | Scoped use | SHP / PAL | Frame mapping | Evidence |
|---:|---|---|---|---|
| 1 | `0xF5` Back in non-active shell path | `SDBTNANM.SHP` / `SDBTNANM.PAL` convert (`FUN_0072E2C0 -> DAT_00B0FBDC`) | default 2, pressed 4, timer/highlight 3 | Decompile `0x00612B70`; assembly `0x00612EAA..0x00612EE1` loads `0x00B0FAC4` and selected frames. |
| 2 | `0xBBB` `0x52C`, `0x52D`, `0x686` in active scenario | `SIDEBTTN.SHP` / `SIDEBAR.PAL` convert (`FUN_0072F4B0 -> DAT_00B0FBE8`) | default 0, pressed 1, timer/highlight 2 | Decompile `0x00612B70`; assembly `0x00612EE8..0x00612F56` calls `0x0072F4B0`, reads `0x00B0F9EC`, and selects frames 0/1/2. |
| 3 | not Options | `MNBTTN.SHP` / `MAINBTTN.PAL` | 0/1/2 by type-3 state path | `0x00609E20` excludes `0xBBB`/`0xF5`; assembly `0x00612F20..0x00612F34` is only the unused type-3 branch. |

Draw order inside the button callback is: select SHP/convert and frame, call
`CC_Draw_Shape`, then draw button text with `FUN_00621040` if a text pointer is
present and the custom-image bypass is not set. Pressed state also sinks the text
rect. The final disabled alpha overlay is gated to type 0 only, so it does not
apply to type 1 or type 2 Options buttons.

### `DAT_00B0F9EC` Asset Identity Correction

`OwnerDraw_Button_00612B70` type 2 reads `DAT_00B0F9EC`, but the loader maps that
global to `SIDEBTTN.SHP`, not `SIDE2B.SHP`.

- `FUN_0072FA10` load sequence: assembly context at `0x0072FAB4..0x0072FAD4`
  loads the string table entry `[0x00844CFC]`, calls the SHP loader at
  `0x0072FAC4`, and stores the result to `0x00B0F9EC` at `0x0072FAD4`.
- Memory table read: `[0x00844CFC] -> 0x008450F4`; memory at `0x008450F4`
  is `"SIDEBTTN.SHP"`.
- `SIDE2B.SHP` is `[0x00844D20] -> 0x008450A4`; assembly context
  `0x0072FB1D..0x0072FB3D` shows that call result stored to `0x00B0FA00`, not
  `0x00B0F9EC`.

Active in YR: Yes for the asset globals and type-2 button path; the stale
`SIDE2B` mapping is refuted for this target.

## Rect Anchoring

The resource DLU rects are the input. Native then runs the common child-resize
dispatcher `ResizeShellChildControl_0060C0C0`, followed by `FUN_0060B950`.

Verified helper routing for this target:

- `0xBBB` active `0x52C`/`0x52D`: style is standard owner-draw button, record kind
  is 0, `FUN_00608CD0` returns true, so dispatcher calls `FUN_0060B000` then
  `FUN_0060B950`. Assembly context `0x0060C1B9..0x0060C1CF` shows the predicate
  and `FUN_0060B000` call.
- `0xBBB`/`0xF5` Back `0x686`: `FUN_00609730` returns true, so dispatcher calls
  `FUN_0060B350` then `FUN_0060B950`. Assembly context `0x0060C21C..0x0060C22E`
  shows the predicate and `FUN_0060B350` call.
- Title/static allow-list controls such as `0x694` (and `0xF5` `0x71C`) route
  through `FUN_0060B1D0` then `FUN_0060B950`.
- Remaining `0xBBB`/`0xF5` controls are in the dispatcher exclusion list, so they
  call `FUN_0060B7A0` then `FUN_0060B950`; assembly context
  `0x0060C3A9..0x0060C3F8` shows `0xBBB`/`0xF5` in the exclusion list and the
  `FUN_0060B7A0` call.

Active-vs-shell geometry delta:

- `FUN_0060B000` and `FUN_0060B350` call `FUN_0069BBE0`. When the scenario-active
  predicate is false, they use `g_SDBTNANM_SHP` dimensions and right-edge offset
  `0x9C`. When it is true, they use `DAT_00B0F9EC` dimensions and right-edge
  offset `0x93`.
- Assembly for `FUN_0060B000`: no-scenario branch reads `0x00B0FAC4` and subtracts
  `0x9C` (`0x0060B0AF..0x0060B0C3`); active branch reads `0x00B0F9EC` and
  subtracts `0x93` (`0x0060B124..0x0060B13F`).
- Assembly for `FUN_0060B350`: no-scenario branch reads `0x00B0FAC4` and
  subtracts `0x9C` (`0x0060B3AE..0x0060B3C4`); active branch reads
  `0x00B0F9EC` and subtracts `0x93` (`0x0060B3FC..0x0060B414`).
- `FUN_0060B7A0` only moves ordinary controls when `FUN_0069BBE0()!=0`, adding
  centered screen offsets and clamping to zero; for shell/non-active `0xF5` it
  returns without moving them.
- `FUN_0060B950` has an Options-specific finalizer for title `0x694`: when
  non-active it applies the shared title y/height nudge; when active scenario is
  true it returns without that title nudge. No scoped non-title trackbar/checkbox
  one-pixel Options nudge was found.

## Trackbar, Checkbox, And Static Paint Paths

Checkbox callback `OwnerDraw_Checkbox_006163A0`:

- `0xF0` returns stored check state; `0xF1` writes state and invalidates.
- `0x004E1FE0` initializes `0x601`, `0x602`, `0x604` with `0xF1`; no
  variant-selection message is sent, so default `cue_i.pcx`/`cce_i.pcx` icons
  are used.
- Paint draws the 18x18 icon at the checkbox origin, then draws label text with
  `FUN_00621040` after shifting the label rect right by `0x1A`. Disabled text
  uses the shell disabled color. Active in YR: Yes for present checkbox controls.

Trackbar callback `OwnerDraw_Trackbar_0061D950`:

- Uses PCX plaque/rail/thumb assets: `trofl.pcx`, `trofm.pcx`, `trofr.pcx`,
  `trakgrip.pcx`; assembly around `0x0061DE9C` and `0x0061E00C` corroborates
  the asset loads/draws.
- Draws primitive bevel/rail via `FUN_006208F0` and value text via
  `FUN_00621040`; text draw context includes `0x0061E30A`.
- `0x529` and `0x52A` have native value inversion in the separate proc/apply
  report; this report only confirms visual callback/assets. Active in YR: Yes.

Static callback `OwnerDraw_Static_006153E0`:

- Ordinary Options labels/title use the text path: style-derived alignment and
  `FUN_00621040`, normal color `DAT_00AC18A4`, disabled color `DAT_00AC1CB4`.
- Image/PCX/SHP static subpaths exist, but this slot found no `0x004E1FE0`
  message that activates them for ordinary Options statics. `0xF5` `0x71C` is
  resource-present and allow-listed in common helpers, but its visible image
  activation remains deferred.

## Current Rust Touchpoints

Read-only scan found no implemented full Options chrome renderer. Relevant likely
surfaces:

- `src/ui/shell/modal.rs`: has `InGameOptions` id/result scaffolding.
- `src/render/skirmish_shell_chrome.rs`: loads `SDBTNANM`, checkbox, trackbar,
  PCX button, and `MNBTTN` assets, but not a verified Options type-2
  `SIDEBTTN.SHP` role through `SIDEBAR.PAL`.
- `src/app_skirmish_shell_render.rs` and `src/render/shell_paint.rs`: existing
  shell/modal emitters are not a native Options `0xBBB`/`0xF5` renderer.
- No Rust files were modified in this investigation.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| Active `0xBBB` `0x52C`/`0x52D`/`0x686` buttons route to owner-draw type 2, using `SIDEBTTN.SHP` through `SIDEBAR.PAL`, frames 0 released, 1 pressed, 2 timer/highlight. | Add a type-2 Options button role and atlas assets; do not reuse `SDBTNANM`, `MNBTTN`, or PCX button pieces for active Options buttons. | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` or new Options shell renderer, `src/render/shell_paint.rs` if generalized. | Open active in-game Options at 800x600; Keyboard, Sound, and Back draw SIDEBTTN frame 0; mouse-down on Back draws frame 1 and text after art. | `ingame_options_0bbb_buttons_use_sidebttn_type2_frames` | Need actual `SIDEBTTN.SHP` dimensions from retail asset at pack time. |
| Shell/non-active `0xF5` lacks `0x52C`/`0x52D`; Back `0x686` uses type-1 `SDBTNANM` route, while `0x50F/0x51A/0x71C` are shell-only controls. | Build separate descriptors/control sets for `0xBBB` and `0xF5`; do not derive `0xF5` by only widening sliders. | `src/ui/shell/modal.rs`, shell layout/descriptor surfaces, render/hit-test. | Shell Options shows Difficulty trackbar, ScrollCoasting checkbox, `0x71C`; no Keyboard/Sound buttons; Back draws SDBTNANM frame 2/4. | `shell_options_0f5_control_set_and_back_type1` | `0x71C` visible image behavior remains a bounded follow-up. |
| Trackbars/checks/statics use existing callback families but resource rects and Options-specific anchoring helpers differ from skirmish `0x102`. | Reuse callback asset code carefully, but feed separate `0xBBB`/`0xF5` rects and the dispatcher helper semantics (`B000/B350/B1D0/B7A0/B950`). | Layout and renderer, especially right-column and title anchoring. | At active 800x600, ordinary controls are centered per `B7A0`, active buttons use `DAT_00B0F9EC` dimensions/right offset; at `0xF5`, ordinary controls remain resource-converted and title takes the shell title nudge. | `options_chrome_rect_anchoring_matches_native_helpers` | Exact asset dimensions must be parsed, not hardcoded. |

## Negative Facts / Do Not Do

- Do not implement active `0xBBB` buttons as skirmish `0x102` type-1
  `SDBTNANM` buttons. Evidence: `FUN_0060A330` writes type 2 on active scenario
  branch; `OwnerDraw_Button_00612B70` type 2 reads `DAT_00B0F9EC`.
- Do not implement active `0xBBB` buttons with `MNBTTN.SHP` / `MAINBTTN.PAL`.
  Evidence: `FUN_00609E20` excludes `0xBBB` and `0xF5`, so type 3 is not reached.
- Do not map `DAT_00B0F9EC` to `SIDE2B.SHP`. Evidence: loader table
  `[0x00844CFC] -> "SIDEBTTN.SHP"` stores to `0x00B0F9EC`; `SIDE2B.SHP` stores
  to `0x00B0FA00`.
- Do not draw active Options buttons with generic `bue_*`/`bde_*` PCX pieces.
  Evidence: type 0 PCX branch is bypassed when record `+0xB0` is 2.
- Do not add `0x52C`/`0x52D` to `0xF5`; the resource lacks them and own-proc
  command handling is active-byte gated.

## Remaining Uncertainty

- Exact framebuffer RGB/pixel equality was not captured; this report proves route,
  assets, frames, and anchoring mechanisms, not a retail screenshot diff.
- Exact decoded `SIDEBTTN.SHP` canvas dimensions were not dumped here; native reads
  the SHP header dynamically, and Rust should do the same.
- `0xF5` `0x71C` is resource-present and helper-allow-listed, but no activation
  message for an image/static kind was found in `0x004E1FE0`; visible output needs
  a separate narrow trace if it matters.

## Stale Doc Replacement Wording Found

- `docs/plans/2026-06-01-shell-substrate-slice5-plan.md`: replace "uses the
  skirmish `0x102` owner-draw control family" / "analogous to `0x102`, not
  confirmed identical" with: "Options uses the common shell owner-draw subclass
  framework. Trackbars, checkboxes, and ordinary statics share the same callback
  families as `0x102`, but active `0xBBB` buttons `0x52C`/`0x52D`/`0x686` route
  to owner-draw type 2 (`SIDEBTTN.SHP` via `DAT_00B0F9EC`, `SIDEBAR.PAL`, frames
  0/1/2), not `SDBTNANM`, PCX buttons, or `MNBTTN`. `0xF5` lacks `0x52C`/`0x52D`
  and its Back button uses the non-active type-1 `SDBTNANM` route."
- `docs/plans/2026-06-01-shell-substrate-slice5b-kickoff.md`: replace "via the
  skirmish `0x102` owner-draw control family" and "assumed same as `0x102`, not
  yet confirmed identical" with the same wording above.
- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace the row
  "`DAT_00b0f9ec` | `SIDE2B.SHP`" with: "`DAT_00b0f9ec` is loaded from
  `SIDEBTTN.SHP` in `MIX_LoadNeutral @ 0x0072FA10` (`[0x00844CFC] ->
  0x008450F4 'SIDEBTTN.SHP'`, stored at `0x0072FAD4`); `SIDE2B.SHP` is loaded
  via `[0x00844D20] -> 0x008450A4` into `0x00B0FA00`. Owner-draw button type 2
  reads `DAT_00B0F9EC`, so it draws `SIDEBTTN.SHP`, not `SIDE2B.SHP`."

## Sources

- Ghidra read-only decompile/context: `0x004E1D00`, `0x004E1FE0`, `0x0060F9A0`,
  `0x0060A330`, `0x00608CD0`, `0x00609730`, `0x00609E20`, `0x0069BBE0`,
  `0x00612B70`, `0x0060C0C0`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`,
  `0x0060B7A0`, `0x0060B950`, `0x006163A0`, `0x0061D950`, `0x006153E0`,
  `0x0072E2C0`, `0x0072F4B0`, `0x0072FA10`.
- Ghidra memory/string evidence: `0x00844CFC`, `0x008450F4`, `0x00844D20`,
  `0x008450A4`; assembly contexts around `0x0072FAC4..0x0072FAD4` and
  `0x0072FB1D..0x0072FB3D`.
- Prior research: `DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`,
  `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md`,
  `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`,
  `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`,
  `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`,
  `skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`,
  `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`.
