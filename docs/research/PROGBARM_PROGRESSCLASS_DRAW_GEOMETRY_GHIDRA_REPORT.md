# PROGBARM ProgressClass Draw Geometry - Ghidra Research Report

**Address(es):** `0x00643AE0`, `0x00643720`, `0x00643400`, plus `0x00643C50`, `0x00642C80`, `0x00552BE0`, `0x00552C90` for setup/update context.  
**Investigation Mode:** exhaustive-slice attempt, downgraded to partial for numeric asset dimensions.  
**Claimed Scope:** static `gamemd.exe` draw/update path for non-campaign/YR Skirmish `PROGBARM.SHP`: frame index, percent-to-fill math, destination geometry formulas, origin source, convert/palette source, draw flags, and percent-change side effects.  
**Non-Scope:** loading-background asset lifecycle, complete `LoadProgressMgr` composition, campaign `SPLDBR.SHP`, multiplayer wait-loop behavior beyond row-count context, runtime HWND ownership trace, Rust implementation.  
**Confidence:** High for binary draw path and formulas; Medium for absolute origin because `LoadProgressMgr+0x1C` is accepted as upstream loading-composition context; Low/unknown for numeric `PROGBARM.SHP` frame dimensions because this pass did not dump the retail SHP asset.  
**Active in YR:** Yes for standard non-campaign/Skirmish (`g_GameMode != 0`, specifically offline Skirmish `g_GameMode == 5`) after `ScenarioClass__Read_Scenario @ 0x00684620` selects `PROGBARM.SHP`.

## 1. Target Question

How does ProgressClass render `PROGBARM.SHP` for non-campaign/Skirmish: which frame is drawn, how percent maps to clipping/fill, what rect/origin is used, where the convert/palette comes from, which draw flags are used, and what changes when percent changes?

## 2. Non-Goals

- Do not investigate loading background assets except as origin context.
- Do not decode every `LoadProgressMgr` draw in `0x00552D60`.
- Do not investigate campaign `SPLDBR.SHP` beyond branch exclusion.
- Do not mutate Ghidra or write Rust.

## 3. Evidence Needed To Mark COMPLETE

- Prove non-campaign setup selects `PROGBARM.SHP` and enables the non-campaign row/side-color flags.
- Prove exact SHP frame index used for geometry and draw.
- Prove percent-to-fill math, including clamp/rounding boundary evidence.
- Prove direct-draw rect formulas from ProgressClass origin to `CC_Draw_Shape`.
- Prove convert/palette source for Skirmish.
- Prove what percent update changes and when redraw is skipped.
- Resolve or explicitly defer absolute asset dimensions and runtime HWND-vs-direct branch ownership.

## 4. Stop Conditions

- Stop after `0x00643AE0`, `0x00643720`, `0x00643400`, `0x00643C50`, and immediate setup helpers are accounted for.
- Stop before full background/loading-screen art composition.
- Stop before asset extraction if Ghidra cannot expose numeric SHP dimensions.

## 5. Core Verified Findings

| Finding | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| Non-campaign setup selects `PROGBARM.SHP`, not `SPLDBR.SHP`, and stores no convert override for that shape. | `0x006847E1..0x00684800`: `g_GameMode == 0` selects `SPLDBR.SHP`; else pushes `PROGBARM.SHP`, `0`, `0` into `FUN_00642C20`; `0x00642C20` stores shape at `ProgressClass+0x54` and `+0x74=0`. | High | Yes for Skirmish |
| Skirmish setup calls `FUN_00642C80` with explicit origin from `LoadProgressMgr::GetProgressPoint`, text pointer `0`, side flag `1`, and color-fill flag `1`. | `0x00684805..0x0068482F`; `0x00552BE0`; `0x00642C80` stores `+0x68/+0x6C`, `+0x50`, `+0x70`, `+0x71`. | High | Yes |
| The progress SHP frame used for both geometry and fill draw is frame `0`; no other `PROGBARM.SHP` frame is referenced in the investigated helpers. | `0x0064378D`, `0x00643813`, `0x00642E00`, `0x00642EF0`, and `0x00643409..0x00643417` call `SHP_frame_rect_getter(...,0)`; `0x0064358D..0x00643594` calls `CC_Draw_Shape(shape, frame 0, ...)`. | High | Yes |
| Visible fill width is `ftol(frame0_width * (lane_value / max_value))`; height remains frame0 height. | `0x00643745..0x0064374C` computes lane fraction; `0x00643409..0x00643417` reads frame0 rect; `0x0064352C..0x00643551` multiplies frame width by fraction and stores the clipped width before the draw. | High | Yes |
| `CC_Draw_Shape` uses flags `0x400`, z/priority argument `1000`, frame `0`, and mostly zero auxiliary args for the filled progress span. | `0x00643555..0x00643594`. | High | Yes |
| Direct-draw Skirmish row uses `DAT_0088730C` as destination surface and flushes through `FUN_004F4780(1, DAT_0088730C, NULL)` after all rows (fastcall register arguments `CL=1`, `EDX=DAT_0088730C`, plus stack `0`). | `0x006439AF..0x006439D3` passes `DAT_0088730C`; live `disassemble_bytes 0x00643B80..0x00643C50` (`program=gamemd.exe`) shows `MOV EDX,[DAT_0088730C]`, `PUSH 0`, `MOV CL,1`, then `CALL 0x004F4780`. (Corrected 2026-07-27: decompiler shorthand `FUN_004F4780(0)` dropped the two register arguments — `PARAM1_TYPE_MISREAD`.) | High | Yes when direct branch is used |
| HWND repaint branch is different: `0x00643C50` sends synchronous `WM_PAINT` to `ProgressClass+0x64`; if `0x00643AE0` is called with an HWND, it gets child `0x639`, reads its rect, and draws only `0x00643400` to `DAT_00887310`. | `0x00643D26..0x00643D34`; `0x00643AFD..0x00643B63`. | High for static branch; runtime ownership deferred | Conditional |
| Skirmish convert source is the local/player color scheme, not an SHP-specific palette override. The convert at `ColorScheme+0x30C` is rebuilt from the scheme's 16 generated palette entries (`i = 0..15`, written to palette indices `16..31`). | `0x00643BDA..0x00643BFE` calls `FUN_00642BB0(0)` and passes the result to `0x00643720`; `0x00642BB0` maps session priority through `SessionClass__PriorityToColorScheme` and `g_ColorSchemeArray`; `0x00643486..0x00643490` uses `ColorScheme+0x30C` unless `ProgressClass+0x74` overrides, and Skirmish setup left `+0x74=0`; live `batch_decompile 0x0068C3B0,0x0068C860` (`program=gamemd.exe`) shows the 16-iteration ramp build and `+0x30C` convert replacement. | High | Yes |
| With `+0x71=1` in Skirmish, `0x00643400` fills the full frame rect with a solid color derived from `ColorScheme+0x308` before drawing the clipped SHP span. | `0x00642C80` stores `+0x71=1`; `0x0064349B..0x00643529` derives RGB from `ColorScheme+0x308` and calls surface vtable `+0x58` over the full frame rect. | High | Yes |
| The Skirmish country PCX is transparent-keyed by RGB magenta `(255,0,255)`, not by palette index. | Live `disassemble_bytes 0x006439D0..0x00643AD5` (`program=gamemd.exe`) shows `PUSH 0xFF`, `EDX=0`, `ECX=0xFF` into `FUN_004355D0`, then the converted key is passed to `FUN_006BA580` for the country-PCX blit. | High | Yes |
| The standard one-lane Skirmish row label is the first session-node display name, drawn after the country icon with `GAME.FNT`, left aligned, and without a backing rectangle. | `0x00643AE0` supplies the first session-node text input when `ProgressClass+0x50` is null; live `disassemble_bytes 0x00643670..0x00643720` and `batch_decompile 0x004A60D0,0x004A61C0,0x00735120` (`program=gamemd.exe`) prove the `GAME.FNT` height, text formatting/draw path, left alignment, and absence of a backing branch. | High | Yes |
| Percent changes update lane storage, optional progress-manager notification, and synchronous repaint/direct draw; unchanged values skip all visible work. | `0x00643C94..0x00643CC9` stores/clamps lane value and compares old vs new; `0x00643D18..0x00643D24` sends message `0x11AE` with old/new average percents; `0x00643D26..0x00643D4B` repaints/draws only after change. | High | Yes |

## 6. Geometry / Fill Details

Terminology:

- `B = (base_x, base_y)` is `ProgressClass+0x68/+0x6C`.
- `W,H` are `PROGBARM.SHP` frame-0 width/height from `SHP_frame_rect_getter(...,0)`.
- `row_h = max(side_icon_h, H + 6, font_h) + 4`.
- `row_y = base_y + row_index * row_h`.
- `fraction = lane_value / ProgressClass.max`.

For Skirmish single-row direct draw:

| Element | Formula / behavior | Evidence |
|---|---|---|
| Stored origin | `+0x68/+0x6C = FUN_00552BE0(LoadProgressMgr)` result. For non-campaign this is `LoadProgressMgr+0x1C` point plus `(12,256)` at default width or `(16,321)` at non-default width. | `0x00552BE0`; `0x00642C80` |
| Width override | `+0x78 = 0x146` at default width, `0x196` at non-default width; used for label/right-bound math, not for SHP fill width. | `0x00552C90`; `0x00642DF0`; `0x00643804..0x0064380C`; `0x00643AA5..0x00643ACC` |
| Fill helper input origin | `0x00643720` calls `0x00643400` at `(base_x + 5, row_y + ((row_h - (H + 6)) / 2))`. | `0x00643880..0x006438A9`; `0x006439BF..0x006439D3` |
| Actual full background/fill rect origin inside `0x00643400` | `x = helper_x + 3`, `y = helper_y + 3`; therefore progress pixels start at `base_x + 8` and `row_y + ((row_h - (H + 6)) / 2) + 3`. | `0x0064341E..0x0064345E`; `0x0064352C..0x00643551` |
| Actual clipped draw rect | `x = helper_x + 3`, `y = helper_y + 3`, `width = ftol(W * fraction)`, `height = H`. | `0x0064352C..0x00643551`; `0x00643555..0x00643594` |
| Side icon / label placement | The side-icon path is gated by `+0x70`. With the icon present, `icon_x = base_x + W + 0x15`, `icon_y = row_y + (row_h - icon_h) / 2`, `label_x = icon_x + icon_w + 10`, `label_y = row_y + (row_h - GAME_FNT_h) / 2`, and `label_width = base_x + width_override - 3 - label_x`, using native integer division/truncation. The icon blit uses RGB magenta `(255,0,255)` as its transparent key. The final label call `FUN_00643670` is unconditional after the icon branch, so `+0x70` does **not** gate the label. | `0x006438AB..0x006439AF`; live `disassemble_bytes 0x006439D0..0x00643AD5`; live `disassemble_bytes 0x00643670..0x00643720`; `program=gamemd.exe` |

Important edge behavior:

- If `ProgressClass+0x60 == 0` or `+0x54 == 0`, `0x00643AE0` returns without drawing.
- If the direct branch is called with point `(-1,-1)`, `0x00643AE0` substitutes stored `+0x68/+0x6C`.
- The only lane-selection test is signed `row_index < (signed char)+0x61`. A negative row index therefore passes the native test and reads a lane at a negative offset; only `row_index >= count` uses `max` instead of a lane fraction. Standard Skirmish uses valid row `0`. (Corrected 2026-07-27: the earlier “outside” wording implied negative rejection; live `disassemble_bytes 0x00643720..0x00643750` with `program=gamemd.exe` shows `MOVSX`, signed `CMP`, then `JL 0x00643745` with no `row_index >= 0` guard — `OPERATOR_OR_ORDER_DRIFT`.)
- Percent input is capped at `max` in storage before draw; negative input was not found on the standard callback path.

## 7. Update Path

`0x00643C50(row, percent, x, y)`:

1. Computes old average percent over all lanes using max and `100.0`.
2. Stores `ProgressClass.max * 0.01 * percent` into lane `row`.
3. Caps that stored lane value to max if it exceeds max.
4. If the lane value did not change, returns without manager notification or repaint.
5. If it changed, computes new average percent and calls the attached progress manager vtable with message `0x11AE` and the old/new integer average percent pair when `ProgressClass+4` and `+0x60` are set.
6. If `ProgressClass+0x64` HWND exists, calls `SendMessageA(hwnd, WM_PAINT, 0, 0)`.
7. Otherwise calls `0x00643AE0(x,y)` for immediate direct draw.

Evidence: `0x00643C5C..0x00643D4B`.

## 8. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `ScenarioClass__Read_Scenario @ 0x00684620` setup | `g_GameMode != 0` branch | `PROGBARM.SHP` loaded into `ProgressClass+0x54`; no override convert | origin from `0x00552BE0`; flags `+0x70=1`, `+0x71=1` | side/local house setup before draw | yes | progress setup |
| 2 | `FUN_00643C50 @ 0x00643C50` | only if stored percent changes | no SHP draw itself | updates lane value; optional `WM_PAINT` | manager notified with old/new average percents | yes | update/repaint trigger |
| 3 | `FUN_00643AE0 @ 0x00643AE0` direct branch | `+0x60!=0`, `+0x54!=0`, no HWND | delegates row 0 in Skirmish | stored origin or caller point | player color scheme from `0x00642BB0(0)` | yes when direct branch | row orchestration |
| 4 | `FUN_00643720 @ 0x00643720` | signed row index `< (signed char)+0x61`; negative indices are not rejected; Skirmish row 0 | `PROGBARM.SHP` frame 0 geometry | `base_x+5`, vertically centered in row; final inset added by `0x00643400` | passes color scheme/side node | yes | row geometry |
| 5 | `FUN_00643400 @ 0x00643400` | always called by row draw | `PROGBARM.SHP` frame 0 | full rect `(helper_x+3, helper_y+3, W,H)`; clipped draw width `ftol(W*fraction)` | `ColorScheme+0x30C`, plus background color from `+0x308`; override `+0x74` inactive for Skirmish | yes | filled progress span |
| 6 | `FUN_00643670 @ 0x00643670` | after fill draw | text/status, no `PROGBARM` draw | text rect uses row/right-bound formulas | same color scheme | yes | row text/status |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `PROGBARM.SHP` | yes | yes | yes | no | yes | yes | no | no | `0x006847F2..0x00684800`; `0x00643400`; `0x00643720` |
| `SPLDBR.SHP` | no for Skirmish | no for Skirmish | no for Skirmish | campaign only | yes in campaign | yes in campaign | no | yes for Skirmish | `0x006847E1..0x006847F0` |

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00643AE0` direct/HWND draw dispatcher | verified | decompile/disassembly `0x00643AE0` | runtime frequency of HWND vs direct branch |
| `0x00643720` row geometry | verified | decompile/disassembly `0x00643720` | numeric SHP frame dimensions |
| `0x00643400` filled SHP draw | verified | decompile/disassembly `0x00643400` | exact named meaning of `0x400` draw flag |
| `0x00643C50` percent update side effects | verified | decompile/disassembly `0x00643C50` | none for static behavior |
| `0x00642C80`, `0x00552BE0`, `0x00552C90` origin/flags setup | touched-not-exhausted | decompile/disassembly listed | full upstream background composition is out of scope |
| Retail `PROGBARM.SHP` numeric frame dimensions | deferred | Ghidra frame getter proves source but not runtime asset dimensions | extract/dump retail SHP frame header |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `PROGBARM.SHP` selected for Skirmish? -> Yes, `g_GameMode != 0` selects `PROGBARM.SHP` and passes zero override args to `0x00642C20`.` (evidence: `0x006847E1..0x00684800`)
- `[RESOLVED] OQ-02 - Which SHP frame is drawn? -> Frame 0 only in investigated helpers.` (evidence: `0x00643409..0x00643594`, `0x0064378D`, `0x00643813`)
- `[RESOLVED] OQ-03 - How is percent converted to fill? -> Stored lane value is max*0.01*percent capped at max; draw fraction is lane/max; clipped width is `ftol(frame0_width*fraction)`.` (evidence: `0x00643C94..0x00643CC0`, `0x00643745..0x0064374C`, `0x0064352C..0x00643551`)
- `[RESOLVED] OQ-04 - What is the direct Skirmish pixel origin formula? -> Stored origin from `0x00552BE0`; filled pixels start at `base_x+8`, `row_y+((row_h-(frame0_h+6))/2)+3`.` (evidence: `0x00552BE0`, `0x00642C80`, `0x00643720`, `0x00643400`)
- `[RESOLVED] OQ-05 - What convert source is used? -> Player/session color scheme via `0x00642BB0(0)` and `ColorScheme+0x30C`; Skirmish does not set `ProgressClass+0x74` override.` (evidence: `0x00642BB0`, `0x00643BDA..0x00643BFE`, `0x00643486..0x00643490`)
- `[RESOLVED] OQ-06 - What draw flags are used for the progress SHP fill? -> `CC_Draw_Shape` uses frame 0 with flags `0x400` and priority/z argument `1000`.` (evidence: `0x00643555..0x00643594`)
- `[RESOLVED] OQ-07 - What updates when percent changes? -> Lane double, optional manager message `0x11AE` with old/new average percent, then synchronous `WM_PAINT` or direct draw; unchanged value skips all of these.` (evidence: `0x00643C50`)
- `[RESOLVED] OQ-10 - Are negative row indices rejected? -> No. The signed `< count` branch accepts negative indices and performs the lane read; standard Skirmish uses row 0.` (evidence: live `disassemble_bytes 0x00643720..0x00643750`, `program=gamemd.exe`)
- `[RESOLVED] OQ-11 - Does `+0x70` gate the final label/status draw? -> No. It gates the icon branch; both paths converge at `0x00643AA5` before the unconditional `FUN_00643670` call.` (evidence: live `disassemble_bytes 0x006439D0..0x00643AD5`, `program=gamemd.exe`)
- `[RESOLVED] OQ-12 - What transparency key is used for the Skirmish country PCX? -> RGB magenta `(255,0,255)`, converted to the active DirectDraw pixel format before the keyed blit.` (evidence: live `disassemble_bytes 0x006439D0..0x00643AD5`, `program=gamemd.exe`)
- `[RESOLVED] OQ-13 - What is the one-lane Skirmish row label? -> The first session-node display name, drawn through `GAME.FNT`, left aligned, without a backing rectangle, in the verified icon-relative/right-bound rect.` (evidence: live `disassemble_bytes 0x00643670..0x00643720`; live `batch_decompile 0x004A60D0,0x004A61C0,0x00735120`; live `inspect_memory_content 0x008258C0`, `program=gamemd.exe`)
- `[DEFERRED] OQ-08 - What are the exact numeric `PROGBARM.SHP` frame dimensions?` (category: requires-different-system-context; reason: static Ghidra proves frame-0 getter use but not decoded retail asset header; next-step-if-pursued: dump `PROGBARM.SHP` frame 0 from `LOADMD.MIX/LOAD.MIX` assets)
- `[DEFERRED] OQ-09 - During ordinary offline Skirmish, does every percent update use HWND repaint or direct fallback?` (category: needs-runtime-debugger; reason: static code proves both branches; exact `ProgressClass+0x64` runtime ownership is not settled; next-step-if-pursued: trace `ProgressClass+0x64` and branch hits during Skirmish load)

## 11. Negative Facts / Do Not Do

- Do not use any `PROGBARM.SHP` frame other than frame `0` for the ProgressClass bar unless a separate asset/runtime trace proves another caller.
- Do not scale the full destination rect to the percent; native clips the draw width to `ftol(frame0_width * fraction)` while keeping frame height.
- Do not invent a smooth continuous progress source; redraw only follows stored percent changes from milestone callbacks.
- Do not substitute `SPLDBR.SHP` for Skirmish/non-campaign.
- Do not use a generic UI palette by default; the Skirmish direct draw path uses the player/session color scheme convert (`ColorScheme+0x30C`) and background color (`+0x308`).
- Do not treat `+0x78` as the SHP fill width; it is a row/text right-bound override, while fill width comes from frame0 width times fraction.
- Do not claim native rejects negative row indices or use such rejection as parity evidence; no nonnegative guard exists in `0x00643720`.
- Do not use `+0x70` as a gate for the final label/status call; it gates the side-icon branch only.
- Do not key the loading country PCX by palette index `0`; native converts the RGB magenta key `(255,0,255)` to the active pixel format.
- Do not substitute the map name, `LSLoadMessage`, or a hardcoded label for the standard one-lane Skirmish row text; native uses the first session-node display name.

## 12. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Skirmish progress uses `PROGBARM.SHP` frame 0 with clipped width `ftol(frame0_width * percent/100)` and a 3px inset inside the helper rect. | `0x00643400`, `0x00643720`, `0x00643C50` | Implemented for the current production loading path: frame 0 is loaded with the player's 16-shade ramp, and the draw width uses the native positive-domain `ftol` formula. Exact `PROGBARM` pixel-mask behavior, exact 3% pixel width, and native/Rust pixel parity remain **UNCHECKED**. (Corrected 2026-07-27 after source inspection of `src/app_loading.rs` `build_native_loading_instances` / `fill_width_gamemd_ftol_positive_domain` and `src/render/loading_screen_chrome.rs` `progress_palette_with_player_ramp`; native contract rechecked via live `batch_decompile 0x00643400,0x0068C3B0,0x0068C860`, `program=gamemd.exe` — `POST_IMPLEMENTATION_STATUS_DRIFT`.) | `src/app_loading.rs`; `src/render/loading_screen_chrome.rs` | Preserve the implemented discrete clipped frame-0 draw and 16-shade remap; add a retail-derived oracle before claiming exact pixels. | Starting a stock Skirmish shows the bar filling horizontally in frame-0 pixels at milestone changes; current Rust tests cover 0/50/100 and 25% formula fixtures, not a gamemd pixel oracle. | Do not claim pixel parity, exact mask behavior, or exact 3% width from Rust-only tests. |
| Origin is native loading-manager point plus non-campaign offsets, then row math adds `+5` helper offset and `+3` draw inset. | `0x00552BE0`, `0x00642C80`, `0x00643720`, `0x00643400` | Implemented for the current standard-Skirmish production layout at 640 and wider breakpoints, including helper/inset, row centering, and side-icon placement; native/Rust pixel parity remains **UNVERIFIED**. (Corrected 2026-07-27 after source inspection of `src/app_loading.rs` `standard_skirmish_progress_position`, `standard_skirmish_row_height`, and `standard_skirmish_side_icon_position`; native ordering rechecked via live `decompile_function 0x00643720` and `decompile_function 0x00643400`, `program=gamemd.exe` — `POST_IMPLEMENTATION_STATUS_DRIFT`.) | `src/app_loading.rs` | Preserve the derived native placement; validate it against a retail capture before exact-pixel claims. | Current Rust tests pin the derived 640/non-640 positions and centering arithmetic. | Do not equate formula tests with native/Rust pixel parity. |
| Percent change side effects are discrete: lane update/clamp, optional old/new manager message, then synchronous repaint/direct draw only on changed values. | `0x00643C50` | Implemented for the current production path as a monotonic gated state plus synchronous per-advancing-milestone render/present. Rust does not reproduce the native progress-manager message, and exact first-frame/milestone dwell remains **UNVERIFIED**. (Corrected 2026-07-27 after source inspection of `src/app_loading.rs` `LoadingProgressState`, `RenderingProgressSink`, and `present_native_loading`; native gate rechecked via live `decompile_function 0x00643C50`, `program=gamemd.exe` — `POST_IMPLEMENTATION_STATUS_DRIFT`.) | `src/app_loading.rs` | Preserve duplicate/lower-milestone suppression and synchronous presentation; retain the manager-message and dwell differences as residuals. | Current Rust tests verify duplicate/lower suppression and advancing-milestone presentation requests. | Do not claim native `WM_PAINT`/hidden-to-primary mechanics or exact dwell from the wgpu presentation path. |
| Skirmish country PCX transparency is RGB magenta `(255,0,255)`. | `0x00643A64..0x00643A8E`; `FUN_004355D0`; `FUN_006BA580` | Current loading decode keys palette index `0`, unlike both native loading and the existing Rust shell flag path. | `src/render/loading_screen_chrome.rs`; `src/assets/pcx_file.rs` tests | Decode the loading country PCX with `to_rgba_with_color_key([255, 0, 255])`; preserve non-magenta palette-index-0 pixels as opaque. | America, Russia, and Yuri loading icons show only their intended silhouettes/details, without the opaque magenta rectangle; a focused decoder test proves an index-0 non-magenta pixel remains opaque. | Do not replace this with a fixed palette-index key or global shader discard. |
| Standard one-lane Skirmish draws the first session-node display name after the icon using `GAME.FNT`, left aligned, with no backing. | `0x00643AE0`; `0x00643670`; `0x004A60D0`; `0x004A61C0` | Current Rust explicitly omits the row label and uses the progress-bar height as a font-height stand-in. | `src/app_loading.rs`; `src/app_loading_progress_row.rs` (new); `src/render/bit_font.rs`; `src/skirmish_launch.rs` | Snapshot `SkirmishLaunchSession.player_name` into loading presentation state, calculate row geometry with the actual loaded `GAME.FNT` height and native integer truncation, then append a left-aligned no-backing text draw at `icon_x + icon_w + 10` through the native right bound. | At 640x480 and 800x600, stock selected-map and random-map launches show the player name after the country icon, centered with the row and not covering the icon/bar. | Do not use `LSLoadMessage`, map metadata, or a hardcoded player name; do not add a text backing rectangle. |

Current Rust coverage inspected in this audit (not executed by this read-only documentation worker):

- `loading_progress_clipped_width_matches_native_formula_for_exact_values`
- `loading_progress_fill_width_uses_gamemd_ftol_positive_domain`
- `bar_origin_uses_helper_offset_and_row_centering`
- `loading_progress_duplicate_milestones_do_not_redraw`

## 13. Remaining Uncertainty

- Exact numeric `PROGBARM.SHP` frame-0 width/height remains deferred to asset extraction. The binary evidence says all geometry comes from frame `0`; it does not embed the dimensions.
- Runtime branch choice between HWND repaint and direct fallback for every ordinary offline Skirmish percent update remains deferred. Static evidence proves both branches and their behavior.
- `0x400` is the draw flag value passed to `CC_Draw_Shape`; this report does not assign a semantic flag name beyond the value.
- Exact `PROGBARM` pixel-mask behavior, native/Rust pixel parity, the exact pixel width of the first displayed 3% state, and exact first-frame/per-milestone dwell remain **UNCHECKED/UNVERIFIED**. Rust formula and regression tests are not a gamemd-derived pixel or timing oracle.

## 14. 2026-07-27 Correction Audit

**Status: YELLOW.** The core geometry, percent mapping, 16-entry ColorScheme convert path, country-PCX RGB magenta key, and one-lane player-name label contract are confirmed. Three isolated native statements were corrected (negative row handling, the scope of the `+0x70` gate, and the redraw-tail call arguments), and the implementation handoff was refreshed against current production Rust. This audit does **not** upgrade exact `PROGBARM` pixel-mask behavior, native/Rust pixel parity, exact 3% pixel width, or exact dwell.

Live read-only evidence used with explicit `program=gamemd.exe`:

- `disassemble_bytes 0x00643720..0x00643750` — signed row-index comparison and absence of a nonnegative guard.
- `disassemble_bytes 0x006439D0..0x00643AD5` — `+0x70`-gated icon branch converges before unconditional `CALL 0x00643670`.
- `disassemble_bytes 0x00643B80..0x00643C50` — redraw tail passes `CL=1`, `EDX=DAT_0088730C`, and stack `NULL` to `FUN_004F4780`.
- `batch_decompile 0x00643400,0x00643720,0x00643670,0x00643C50,0x00642BB0,0x00643AE0` — core fill geometry, label/status draw, percent-change gate, convert selection, and redraw dispatcher.
- `batch_decompile 0x0068C3B0,0x0068C860` — 16-iteration house-color palette generation and `ColorScheme+0x30C` convert rebuild.
- `disassemble_bytes 0x006439D0..0x00643AD5` — exact RGB magenta key conversion arguments, keyed icon blit, icon-relative label x, and shared label call.
- `disassemble_bytes 0x00643670..0x00643720` — `GAME.FNT` height, label rect construction, and no-backing flags.
- `batch_decompile 0x004355D0,0x004A60D0,0x004A61C0,0x00735120` — DirectDraw RGB conversion, font metric access, formatted text draw, and lower text-rendering behavior.
- `inspect_memory_content 0x008258C0` — confirms the fallback-width seed string is `"W"`; the explicit Skirmish width path does not use that fallback.

## 15. Stale Docs / Follow-Up Docs

- Replace the deferred geometry line in `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md` with: "ProgressClass draw geometry is now statically verified in `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`: Skirmish uses `PROGBARM.SHP` frame 0, fill is a clipped frame-0 draw whose width is `ftol(frame0_width * lane/max)`, the direct-draw pixel origin is derived from `FUN_00552BE0` plus row/inset math, and exact numeric frame dimensions still require a retail SHP asset dump."

## Sources

- Ghidra read-only decompile/disassembly: `0x00643AE0`, `0x00643720`, `0x00643400`, `0x00643670`, `0x00643C50`, `0x00642C20`, `0x00642C80`, `0x00642DF0`, `0x00642E00`, `0x00642E40`, `0x00642E80`, `0x00642EF0`, `0x00642BB0`, `0x00552BE0`, `0x00552C90`, `0x00684620`, `0x0069AE90`.
- Prior reports referenced for context only: `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md`.
