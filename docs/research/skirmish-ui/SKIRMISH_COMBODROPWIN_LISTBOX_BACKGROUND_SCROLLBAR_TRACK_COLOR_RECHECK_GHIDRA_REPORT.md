# Skirmish ComboDropWin/ListBox Background And Scrollbar Track Color Recheck - Ghidra Research Report

**Address(es):** `FUN_0060F9A0 @ 0x0060F9A0`, `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `OwnerDraw_ListBox_00618D40 @ 0x00618D40`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `FUN_006208F0 @ 0x006208F0`, `DSurface +0x30 @ 0x007BA5E0/0x007BA610`  
**Investigation Mode:** coverage-map with connected Ghidra static slice  
**Claimed Scope:** Recheck whether connected Ghidra evidence is strong enough to replace Rust placeholder constants for open combo/listbox background fill and scrollbar track fill in standard offline Skirmish shell rendering.  
**Non-Scope:** Runtime retail surface capture. Ghidra MCP was connected to `testProsjekt:/gamemd.exe`, but no live `gamemd.exe` game process was running for debugger/surface sampling.  
**Confidence:** High that the current Rust constants are unverified placeholders; High that native ComboBox/ListBox/ScrollBar paint does not reduce to the current two Rust literal RGBs; High that connected Ghidra identifies the relevant source globals and composition calls; Low/none on exact final 800x600 RGB pixels until runtime capture.  
**Active in YR:** Yes for the combo/listbox/scrollbar owner-draw paint paths. Exact final RGB remains conditional on owner-draw initializer state, DirectDraw pixel conversion, copied parent/backing surface pixels, and must be captured from a live frame.

## 1. Overview

The current Rust constants named `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE` and `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE` are still correctly classified as unverified. Connected Ghidra confirms the native paths use owner-draw runtime globals, backing-surface copies, alpha blending, primitive frame/fill helpers, and PCX scrollbar pieces. They are not proven by the current Rust solid fills.

The important implementation result is still negative: do not "fix" these constants by renaming the current guessed RGBs as verified. Ghidra narrows the mechanisms and source globals, but the next proof step for pixel parity is a runtime retail capture of the 800x600 shell surface.

Connected session facts:

- Ghidra project: `testProsjekt:/gamemd.exe`
- Program image base: `0x00400000`
- Function count: `9903`
- Ghidra MCP health: connected to program `gamemd.exe`
- Live game/debugger process: not running during this pass

## 2. Current Rust Constants

| Rust constant | Current value | Use | Status |
|---|---:|---|---|
| `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE` | `[0.015, 0.024, 0.018]` | Solid fill behind combo dropdowns and Choose Map listboxes | placeholder / unverified |
| `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE` | `[0.035, 0.042, 0.034]` | Solid fill behind combo/listbox scrollbar arrows/thumb | placeholder / unverified |

These are used in:

- `src/app_skirmish_shell_render.rs`
- `src/app_skirmish_shell_render/controls.rs`
- `src/app_skirmish_shell_render/modals.rs`

## 3. Verified Existing Color Facts

| Native source | Verified value / behavior | Evidence | Applies to requested constants? |
|---|---|---|---|
| Primitive bevel globals | `DAT_00AC1B98 = 0x00C5BEA7`, `DAT_00AC1B94 = 0x00807A68`; swapped between outer/inner rings for 2 px frames | `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`; `FUN_006208F0` evidence `0x00620A90..0x00620C7F` | No; bevel/frame pixels only |
| Primitive line raster | `DSurface +0x30` writes native packed pixels after conversion; endpoints are inclusive | `SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md` | No; confirms frame line mechanics only |
| Selected dropdown row fill | Normal selected fill uses converted `DAT_00AC4604`; grey selected fill uses `DAT_00AC4880` | `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`; `0x0060DD42..0x0060DE0A` | No; selected row only |
| Normal dropdown text | Normal text uses `DAT_00AC18A4`; grey text uses `DAT_00AC1CB0`; swatch rows can overwrite text color | `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`; `0x0060DE1F..0x0060DE60` | No; text only |
| Scrollbar pieces | Scrollbar uses `FUN_00620720` arrows and `sbgripm/sbgript/sbgripb.pcx`, with grey variants | `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`; `OwnerDraw_ScrollBar_0061C690` | Partly; thumb/arrows are asset-backed, not enough for track fill |
| Scrollbar grey candidate | `DAT_00AC1DD8 = 0x00929292` is identified as a scrollbar grey candidate | `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` | No; not proven as the normal scrollbar track color for this shell path |

## 3.1 Connected Ghidra Findings From This Pass

| Finding | Evidence | Why it matters |
|---|---|---|
| Owner-draw initializer sets runtime color globals before class-specific handlers run | `FUN_0060F9A0 @ 0x0060FA35..0x0060FACD` writes `DAT_00AC1890=0x7F`, `DAT_00AC4604=0xFF`, `DAT_00AC4624=0xFF`, `DAT_00AC1DD8=0x929292`, `DAT_00AC1B98=0xC5BEA7`, `DAT_00AC1B94=0x807A68`, and mask globals | Static data reads at those addresses are zero before initialization; final colors must be reasoned from runtime writes, not file-initialized bytes |
| `FUN_0060F9A0` maps control classes to different owner-draw handlers | `0x0060FBD4` ScrollBar -> `0x0061C690`; `0x0060FC1C` ListBox -> `0x00618D40`; `0x0060FC64` ComboBox -> `0x00617250` | ComboBox popup/listbox/scrollbar are distinct native paths; one shared Rust background constant is unproven |
| ComboBox paint allocates/caches a `BSurface` and optionally copies parent/backing pixels before alpha processing | `OwnerDraw_ComboBox_00617250 @ 0x006175A5..0x006176F4`; vtable `+0x08` copy, `+0x5C/+0x60` lock/unlock, pixel loop using alpha byte at owner-draw state `+0xC8` and masks `DAT_00AC48B8/BA/BC` | Unselected popup background is composition over existing backing pixels, not a simple native literal RGB in this slice |
| ComboBox then draws primitive frame/fill using the same helper family as other owner-draw controls | `0x0061770F..0x0061791D`: disabled branch uses `DAT_00AC1DD8`; enabled branch uses `DAT_00AC4624`; style fallback can substitute `DAT_00AC4620`/`DAT_00AC1CA8`; then `FUN_006208F0` and `FUN_00620720` are called | Background/track source choice is stateful; enabled and grey paths diverge |
| ListBox paint creates a primitive frame first, then fills selected rows separately | `OwnerDraw_ListBox_00618D40 @ 0x00619230..0x006194C2`; `FUN_006208F0(2,-1)` then `DAT_00AC4604` conversion and selected-row vtable `+0x14` fill if `LB_GETSEL > 0` | Real ListBox interior/row behavior is separate from ComboBox popup; selected fill source does not prove unselected background |
| ListBox text is pre-truncated before draw | `OwnerDraw_ListBox_00618D40 @ 0x00619962..0x006199AC` zero-terminates UTF-16 text until measured width fits | Confirms the row text truncation fix direction and reinforces that row rendering uses native pre-measure loops |
| ScrollBar paint also allocates/caches a backing `BSurface`, copies parent/backing pixels, alpha processes, then paints the track/frame/thumb pieces | `OwnerDraw_ScrollBar_0061C690 @ 0x0061CBC2..0x0061CD50`; same lock/unlock and alpha/mask loop as ComboBox/ListBox cache path | Scrollbar track is composited; Rust's single solid track rect is still a placeholder |
| Scrollbar enabled track/frame color source is `DAT_00AC4624`; grey state source is `DAT_00AC1DD8` | `0x0061CD80..0x0061CE3F` converts `DAT_00AC4624`, switches to `DAT_00AC1DD8` when state byte `+0xCD` is set, then calls `FUN_006208F0(2,color)` | This proves the source globals for primitive track/frame fill, but not final sampled pixels after full composition |
| Scrollbar grip pieces are PCX assets drawn after the primitive/fill stage | `0x0061CEAE..0x0061CFD0`: normal `sbgripm.pcx`, `sbgript.pcx`, `sbgripb.pcx`; grey `gsbgripm.pcx`, `gsbgript.pcx`, `gsbgripb.pcx` | Pixel tests must sample track gaps between PCX pieces, not thumb pixels |
| Arrow/button areas use `DAT_00AC1890=0x7F` through `FUN_006205D0`/`FUN_00620720` | `0x0061D04A..0x0061D14C` | Arrow/button chrome is another distinct source; do not reuse it as listbox/dropdown background |

## 4. What The Existing Reports Do Not Prove

No report plus this connected Ghidra pass proves either of these as a final single sampled RGB:

- open `ComboDropWin` unselected/background fill;
- real owner-drawn `LISTBOX` unselected/background fill;
- normal scrollbar track/background fill behind the PCX thumb and arrow buttons.

Prior docs repeatedly defer final color proof:

- `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md` defers exact final RGB for primitive frame, popup background, selected fill, and disabled alpha.
- `SKIRMISH_COMBODROPWIN_DROPDOWN_VISUAL_INPUT_RECHECK_GHIDRA_REPORT.md` defers exact final RGB for selected fill/background after display conversion.
- `SKIRMISH_COMBO_DROPDOWN_LISTBOX_ASSEMBLY_GHIDRA_REPORT.md` says Rust still uses approximate pending-capture colors for popup background/track/final RGB.

That means the current placeholder constants are not merely poorly named; the runtime pixel evidence needed to replace them is still absent.

## 5. Current Native Model From Prior Evidence

The native combo/listbox path is not a flat generic fill plus text:

1. `ComboDropWin` is a registered popup window with its own WndProc block, not a real `LISTBOX` row callback.
2. Popup row paint uses source combo item count, source combo item height, selected index, top index, and current client width after scrollbar shrink.
3. Selected row fill, swatch fill, and text are actively verified; unselected background pixels are composed through backing-surface copy/alpha plus helper calls and are not reduced to a captured final RGB here.
4. Real `LISTBOX` controls use `OwnerDraw_ListBox_00618D40`, which draws a primitive frame and rows; Choose Map rows are not the same renderer as standard combo popup rows.
5. Scrollbar visuals are a mixture of backing-surface composition, primitive/background handling, arrow/button helpers, and grip PCX pieces.

Rust currently collapses the unverified background/track parts into two solid RGB rectangles. That may be a reasonable temporary renderer simplification, but it is not proven pixel parity.

## 6. Current Rust Implementation Status

| Surface | Current behavior | Parity status |
|---|---|---|
| `push_dropdown_instances` | Fills the whole dropdown rect with `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE`, then draws selected row, swatches, optional scrollbar, frame | DRIFT/UNCHECKED for background RGB |
| `push_choose_map_listbox_instances` | Fills the whole listbox rect with the same pending dropdown background constant | DRIFT/UNCHECKED for real `LISTBOX` background RGB |
| `push_dropdown_scrollbar_instances` | Fills the whole scrollbar rect with `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE`, then draws arrow PCXs, thumb, frame | DRIFT/UNCHECKED for normal track RGB/composition |
| primitive bevel constants | Uses verified decoded source RGB for bevel globals in current Rust constants | Better-supported, but final DirectDraw pixel conversion still separate |
| selected row fill | Uses `0x000000FF` decoded as red source RGB | Source-global-supported; final display RGB capture still separate |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Research-index scan for topic | verified | `python tools/research_index/brief.py --system skirmish-ui "ComboDropWin listbox background scrollbar track colors"` | none |
| Current Rust placeholder usage | verified | `rg PENDING_.*SOURCE_CAPTURE src/app_skirmish_shell_render*` | none |
| Primitive bevel color source | verified | `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md` | not a substitute for background/track |
| Selected row and text color sources | verified | `SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md` | final display RGB still capture-only |
| ComboDropWin unselected background source | touched-not-exhausted | connected Ghidra identifies backing-copy/alpha/primitive helper composition | runtime final-pixel capture needed |
| Real `LISTBOX` unselected background source | touched-not-exhausted | connected Ghidra covers row/frame behavior, selected fill, and text truncation, not final background RGB | runtime final-pixel capture needed |
| Scrollbar track source/composition | touched-not-exhausted | connected Ghidra identifies backing-copy/alpha path, primitive source globals, and PCX pieces | runtime final-pixel capture needed |
| Connected Ghidra decompile/disassembly in this session | verified | `testProsjekt:/gamemd.exe`; `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ListBox_00618D40`, `OwnerDraw_ScrollBar_0061C690`, `FUN_0060F9A0`, `FUN_006208F0` | none for static slice |
| Runtime retail surface capture | deferred | no live `gamemd.exe` game/debugger process found | run retail shell and capture 800x600 surface pixels |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-COLOR-001 - Are the Rust `PENDING_*_SOURCE_CAPTURE` constants still present and active? -> Yes.` (evidence: `src/app_skirmish_shell_render.rs`, `controls.rs`, `modals.rs` source scan)
- `[RESOLVED] OQ-COLOR-002 - Do existing verified docs provide replacement final RGB values for the dropdown background and scrollbar track? -> No; they repeatedly defer final RGB/background/track capture.` (evidence: combo visual/input/listbox assembly reports)
- `[RESOLVED] OQ-COLOR-003 - Can primitive bevel globals be reused as dropdown/listbox background colors? -> No; they only prove 2 px primitive frame line colors and averaged corners.` (evidence: `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-COLOR-004 - Can selected fill global `DAT_00AC4604` be reused as background/track? -> No; it is selected row fill only.` (evidence: `0x0060DD42..0x0060DE0A`)
- `[RESOLVED] OQ-COLOR-005 - Is `DAT_00AC1DD8 = 0x00929292` proven as the normal scrollbar track? -> No; connected Ghidra shows it is the grey-state source, while enabled primitive track/frame uses `DAT_00AC4624 = 0x000000FF` before conversion/composition.` (evidence: `0x0061CD80..0x0061CE3F`, `0x0060FA81..0x0060FA87`)
- `[RESOLVED] OQ-COLOR-006 - Is a Rust patch replacing these constants justified by current evidence? -> No; patching guessed values would relabel uncertainty as parity.` (evidence: current source plus docs)
- `[RESOLVED] OQ-COLOR-007 - Fresh Ghidra decompile/disassembly of owner-draw fill/copy calls. -> Connected Ghidra confirms surface copy/alpha composition and source globals, but not final sampled pixels.` (evidence: `FUN_0060F9A0`, `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ListBox_00618D40`, `OwnerDraw_ScrollBar_0061C690`)
- `[DEFERRED] OQ-COLOR-008 - Exact final ComboDropWin unselected background pixels.` (category: `needs-runtime-capture`; reason: requires retail screenshot/surface capture from a live 800x600 Skirmish frame; next-step-if-pursued: open standard side dropdown and sample unselected row/background pixels)
- `[DEFERRED] OQ-COLOR-009 - Exact real LISTBOX unselected background pixels for Choose Map controls.` (category: `needs-runtime-capture`; reason: current Ghidra pass proves row/frame/copy composition but not final background RGB; next-step-if-pursued: capture `0x6B` Choose Map listbox unselected interior pixels)
- `[DEFERRED] OQ-COLOR-010 - Exact scrollbar track pixels behind arrows/thumb.` (category: `needs-runtime-capture`; reason: owner-draw scrollbar composition includes backing copy/alpha, primitive fills, arrow helpers, and PCX pieces; next-step-if-pursued: capture side dropdown overflow scrollbar track between arrow and thumb)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `ComboDropWin` paint block `0x0060D540` | popup open | cached/background surface path | popup client | unresolved | yes | popup backing/background |
| 1a | `OwnerDraw_ComboBox_00617250 @ 0x006175A5..0x006176F4` | combo paint `WM_PAINT` | `BSurface` cache | control client | parent/backing copy plus alpha masks | yes | backing composition |
| 2 | primitive frame helper | popup/list frame paths | none | frame rectangle | bevel globals through DirectDraw conversion | yes | frame/chrome |
| 2a | `OwnerDraw_ComboBox_00617250 @ 0x00617749..0x00617898` | enabled/grey/style state | none | combo/drop rect | `DAT_00AC4624`, `DAT_00AC1DD8`, fallback globals through DirectDraw conversion | yes | combo/drop primitive fill/frame |
| 2b | `OwnerDraw_ListBox_00618D40 @ 0x00619230..0x0061926B` | listbox paint | none | listbox client | `FUN_006208F0(2,-1)` | yes | listbox primitive frame/fill |
| 3 | selected row fill | current row equals selected/hot | none | full row/content rect | `DAT_00AC4604` or grey `DAT_00AC4880` converted | conditional | selected row |
| 4 | swatch fill | swatch mode and valid row swatch | none | row rect inset by 2 | swatch color converted | conditional | color row content |
| 5 | text draw | every visible text row | BitFont glyphs | `x+3`, row top/bottom | `DAT_00AC18A4`, grey, or swatch color | yes | row label |
| 6 | `OwnerDraw_ScrollBar_0061C690 @ 0x0061CBC2..0x0061D14C` | overflow rows | backing cache, `sbgrip*.pcx`/`gsbgrip*.pcx`, primitive helpers | scrollbar column | `DAT_00AC4624` normal, `DAT_00AC1DD8` grey, `DAT_00AC1890` arrow/button helper | conditional | scrollbar |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `sbgripm.pcx` / `sbgript.pcx` / `sbgripb.pcx` | yes | yes | conditional overflow | no | scrollbar thumb | no | no | no | combo visual parity report |
| grey `gsbgrip*.pcx` variants | yes/conditional | conditional | grey state only | no | scrollbar thumb | no | no | no | combo visual parity report |
| arrow PCXs via `FUN_00620720` | yes | yes | combo/scrollbar | no | arrow chrome | no | no | no | combo visual parity report |
| backing `BSurface` cache | yes | yes | yes | no | owner-draw backing | no | no | no | connected Ghidra `0x006175A5..0x006176F4`, `0x0061CBC2..0x0061CD50` |
| synthetic Rust white pixel solid rect | Rust-only | yes | yes | no | temporary fill | no | no | no | current Rust source |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Current dropdown/listbox background constants are not verified native RGB | source scan plus existing reports' final-RGB deferrals | mismatch/unchecked | `src/app_skirmish_shell_render.rs`, `controls.rs`, `modals.rs` | Keep the `PENDING` meaning or replace only after capture; do not rename current guesses as verified | Audit finds no `PENDING_*_SOURCE_CAPTURE` constant left unless each replacement cites a source global or capture | Do not tune by eye or borrow bevel/selected/text colors for background |
| Scrollbar track/frame source globals are now identified, but final pixels are not captured | `OwnerDraw_ScrollBar_0061C690 @ 0x0061CD80..0x0061CE3F`; `FUN_0060F9A0 @ 0x0060FA81..0x0060FA87` | mismatch/unchecked | `push_dropdown_scrollbar_instances`, Choose Map listbox scrollbar path | Capture/verify the native track composition before choosing final fill; normal primitive source is `DAT_00AC4624=0xFF`, grey source is `DAT_00AC1DD8=0x929292`, but final visible pixels include backing and surrounding PCX/primitive composition | Overflow side dropdown track pixels between arrow and thumb match retail sample at 800x600 | Do not treat `DAT_00AC1DD8` as normal track; it is grey-state source |
| Real `LISTBOX` and `ComboDropWin` are separate row painters | ComboDropWin and listbox row reports | Rust currently shares same background constant for both | dropdown renderer and modal listbox renderer | Verify each background independently; they may share colors, but that needs evidence | Choose Map listbox and ComboDropWin unselected interiors match their own retail captures | Do not assume one capture/value applies to both controls |
| Runtime final RGB remains display-format dependent | DirectDraw conversion reports | Rust uses decoded source RGB floats | renderer color conversion/pixel tests | For exact pixel parity, either capture final 800x600 pixels or model DirectDraw conversion path and compare labels | A captured retail sample and Rust screenshot have identical sampled pixels for unselected background and track | Do not call decoded source RGB byte-perfect final pixel parity |

## 11. Negative Facts / Do Not Do

- Do not remove `PENDING` from the dropdown background or scrollbar track constants until a capture or fresh Ghidra proof identifies the native source/composition.
- Do not use `0x00C5BEA7` / `0x00807A68` as popup background colors; those are primitive bevel frame line globals.
- Do not use `0x000000FF` as a background color; it is selected-row fill.
- Do not use `0x00929292` as normal scrollbar track; connected Ghidra shows it is used when the owner-draw grey-state byte is set.
- Do not treat `0x000000FF` / `DAT_00AC4624` as final normal scrollbar track pixels without sampling; it is the source global converted before primitive draw, not a completed screenshot pixel.
- Do not collapse ComboDropWin popup rows and Choose Map real listbox rows into one verified visual claim; the renderers are related but distinct.

## 12. Sources

- `docs/research/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COMBODROPWIN_DROPDOWN_VISUAL_INPUT_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_COMBO_DROPDOWN_LISTBOX_ASSEMBLY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_DROPDOWN_ROW_INTERNAL_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30_RASTER_CONTRACT_GHIDRA_REPORT.md`
- Current Rust scan: `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/controls.rs`, `src/app_skirmish_shell_render/modals.rs`
- Connected Ghidra MCP: `testProsjekt:/gamemd.exe`; `get_current_program_info`; `get_function_by_address`; `decompile_function`; `disassemble_function`; `get_xrefs_to`; `read_memory`
- Runtime state: Ghidra CodeBrowser was open, but no live `gamemd.exe` game process was found for debugger/surface capture.
