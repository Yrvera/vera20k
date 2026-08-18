# Radar Transition Close/Open Asset Lifecycle - Ghidra Research Report

Date: 2026-05-27

**Address(es):** `0x0072D460`, `0x0072D300`, `0x006C8E80`, `0x0072D830`, `0x0072D730`, `0x005C9720`, `0x0072E9F0`, `0x0072EAD0`, `0x00603870`, `0x0060A5B0`, `0x006153E0`
**Investigation Mode:** exhaustive-slice for the requested asset lifecycle gaps
**Claimed Scope:** `SSCRA*` close-frame consumer, `MPSSCRN*` transition movie load/draw/cleanup timing, cheap palette/ConvertClass route, and ordinary in-game vs right-panel/owner-draw split.
**Non-Scope:** settled `SSCR*` placement, minimap terrain/object dots, bridge dirty events, minimap input mapping, and full retail screenshot capture.
**Confidence:** High for binary load/draw/cleanup/owner-draw mechanics; Medium for resource-derived `SSCRA*` child pixel geometry because no live `GetClientRect` hook was captured.
**Active in YR:** Conditional. The selector and direct transition wrappers are active in standard YR scenario start/exit flow; `SSCRA*` visible draw is conditional on radar dialog child `0x72B` owner-draw setup for dialogs `0x103`/`0xBC7`.

## Working Notes

Target question: Resolve active-YR radar close/open transition asset lifecycle for `SSCRA*` / `g_RadarFrameClose_SHP` and `MPSSCRN*`, including consumer, timing/lifecycle, cheap palette/ConvertClass evidence, and ordinary in-game vs right-panel split.

Non-goals: Do not rediscover settled `SSCR` placement, minimap aperture/inset, bridge dirty events, or general radar object dots/input mapping.

Evidence needed to mark COMPLETE: Ghidra read-only xrefs/decompile/disassembly evidence for close/open consumers or no-consumer status, `MPSSCRN` timing/state owner, lifecycle/load path, target-mode activity, Rust touchpoints, and stale-doc wording.

Stop conditions: All seeded open questions are resolved or explicitly deferred; a zero-add pass over entry points adds no new material question; report and optional shared claims update stay within allowed paths.

## 1. Overview

Native YR has three related but separate radar transition surfaces. `SSCRBK*` / `SSCRT*` / `SSCRA*` are loaded together by `RadarBackground_SHPLoad`, but only `SSCRT*` has a direct right-panel open-frame draw; `SSCRA*` is consumed by the owner-draw static path for radar dialog child `0x72B`. `MPSSCRN*` is a separate transition movie SHP loaded, drawn once through `FUN_0072EAD0`, displayed through the message pump, then cleaned up.

Active in YR: Yes for the load/draw wrappers reached by `FUN_00685670` / `FUN_00685DC0`; Conditional for `SSCRA*` visible owner-draw use because it requires dialog `0x103` or `0xBC7` child `0x72B`.

## 2. Key Globals / Offsets

| Global / record field | Role | Evidence | Active in YR |
|---|---|---|---|
| `0x00B0FB34` | `g_RadarFrameOpen_SHP`; `SSCRBK*` for Soviet selector; direct open-frame draw input | `0x0072D509..0x0072D51F`, `0x0072EA44` | Yes |
| `0x00B0FB00` | `g_RadarBackground_SHP`; `SSCRT*`; static radar background draw input | `0x0072D514..0x0072D534`, `0x0072E962` | Yes |
| `0x00B0FB30` | `g_RadarFrameClose_SHP`; `SSCRA*`; owner-draw static image input | `0x0072D529..0x0072D53E`, `0x00603870` | Conditional |
| `0x00B0FB1C` | `g_MinimapMovie_SHP`; `MPSSCRN*`; direct movie draw input | `0x0072D87D..0x0072D899`, `0x0072EB24` | Yes |
| `0x00B0FBA8` | ConvertClass/palette object for `SSCR*` radar-frame family | `0x0072D32E..0x0072D338`, `0x0072D450` | Yes |
| `0x00B0FBB4` | ConvertClass/palette object for `MPSSCRN*` movie family | `0x0072D75E..0x0072D768`, `0x0072EB2E` | Yes |
| owner-draw record `+0x70` | static kind; `4` means animated SHP owner-draw | `0x0060A944..0x0060A987` | Conditional |
| owner-draw record `+0x78` | cached SHP pointer returned by `FUN_00603870` | `0x0060A9B2..0x0060A9B7` | Conditional |
| owner-draw record `+0x94` | frame count from SHP header `+6` | `0x0060A9ED..0x0060AA41`, decompile | Conditional |
| owner-draw record `+0x98` | current frame; read by paint, written by `0x4D5`, advanced by paint | `0x006153E0`, `0x006159EB..0x00615A17` | Conditional |

## 3. Core Logic

### 3.1 `SSCR*` selector loads close, but direct right-panel draws skip it

Active in YR: Yes for loading; No for direct right-panel draw of `SSCRA*`.

`RadarBackground_SHPLoad @ 0x0072D460` selects three SHPs by side and exact small-width predicate. For Soviet side `1`, `g_ScreenWidth == 0x280` requests:

| Global | Filename pointer | Filename |
|---|---:|---|
| `0x00B0FB34` | `0x00844C70` | `SSCRBKSM.SHP` |
| `0x00B0FB00` | `0x00844C78` | `SSCRTSM.SHP` |
| `0x00B0FB30` | `0x00844C80` | `SSCRASM.SHP` |

For Soviet side `1`, `g_ScreenWidth != 0x280` requests:

| Global | Filename pointer | Filename |
|---|---:|---|
| `0x00B0FB34` | `0x00844C74` | `SSCRBKMD.SHP` |
| `0x00B0FB00` | `0x00844C7C` | `SSCRTMD.SHP` |
| `0x00B0FB30` | `0x00844C84` | `SSCRAMD.SHP` |

Assembly evidence: `0x0072D4F4` compares side with `1`; `0x0072D4FD` compares width with `0x280`; `0x0072D509..0x0072D53E` and `0x0072D544..0x0072D579` load the filename pointers and store returned SHP pointers to `0x00B0FB34`, `0x00B0FB00`, and `0x00B0FB30`.

`FUN_0072E9F0 @ 0x0072E9F0` draws `0x00B0FB34` frame `0` at `DAT_00B0FC1C.x/y`; `RadarBackground @ 0x0072E920` draws `0x00B0FB00` frame `0` with `+80` x only when `g_ScreenWidth > 799`; `FUN_0072EAD0 @ 0x0072EAD0` draws `0x00B0FB1C`. None of these direct right-panel paths read `0x00B0FB30`.

### 3.2 `SSCRA*` consumer is owner-draw static child `0x72B`

Active in YR: Conditional. Evidence: the branch is live in the generic owner-draw static setup/paint path and explicitly matches dialogs `0x103`/`0xBC7`, child `0x72B`; visibility depends on those dialogs being active.

`FUN_00603870 @ 0x00603870` returns `g_RadarFrameClose_SHP @ 0x00B0FB30` only when the parent dialog metadata is `0x103` or `0xBC7` and `GetDlgCtrlID(child) == 0x72B`. `FUN_0060A5B0 @ 0x0060A5B0` arms that same `(dialog, child)` pair as owner-draw kind `4`, then calls `FUN_006035F0` for the convert object and `FUN_00603870` for the SHP pointer. Assembly `0x0060A9B2..0x0060A9C1` stores the SHP to record `+0x78`, the convert object to `+0x74`, and the cleanup byte to `+0x7C`.

`OwnerDraw_Static_006153E0 @ 0x006153E0` paints kind `3`/`4` SHPs by reading record `+0x78` and current frame `+0x98`, computing a destination from the child client rect, and calling `CC_Draw_Shape(shape, frame, dest, clip, flags=0x400, z=1000, ...)`. The centering checks are strict `<`: if `shape_width < client_width`, add half-margin to x; if `shape_height < client_height`, add half-margin to y. Equal-size or larger SHPs draw at the client origin and clip.

Prior retail geometry report verified child `0x72B` resource rects for `0x103`/`0xBC7` map to `423x229`, while `SSCRA*` is `424x230`. In standard resource mapping, `SSCRA*` therefore draws at child origin and clips one right/bottom pixel; no centering branch fires.

### 3.3 Owner-draw frame protocol

Active in YR: Conditional for the child `0x72B` record; the message handler and send site are live.

`OwnerDraw_Static_006153E0` handles:

| Message | Behavior | Evidence |
|---|---|---|
| `0x4D3` | Start kind-4 animation if SHP pointer exists, kind is `4`, and running byte `+0xA8` is clear; calls `FUN_006033F0`, then `SetTimer(id=0, interval)` | decompile `0x006153E0`; assembly send site `0x006C93C2` |
| `0x4D4` | Stop kind-4 animation, kill timer id `0` | decompile `0x006153E0` |
| `0x4D5` | Store caller-provided frame to `+0x98` and invalidate | decompile `0x006153E0`; send site `0x006C93CD..0x006C93DB` sends frame `0x1D` to child `0x72B` |
| `0x4D6` | Return current frame `+0x98` for kind `4` | decompile `0x006153E0` |

`FUN_006033F0 @ 0x006033F0` returns `100` ms for `(0x103/0xBC7, 0x72B)`. Paint advances current frame by one, wraps to `0` when it reaches record `+0x94`, and optionally sends `0x4D8` to callback HWND `+0xA4`.

### 3.4 `MPSSCRN*` movie load/draw/cleanup lifecycle

Active in YR: Yes. Evidence: `FUN_00685670` and `FUN_00685DC0` call `FUN_005C9700` then `FUN_005C9720` on standard YR scenario exit/restart branches when not in the network-special branch.

`RadarTransitionMovie_SHPLoad @ 0x0072D830` selects `g_MinimapMovie_SHP @ 0x00B0FB1C`. For Soviet side `1`, exact small width `0x280` uses `MPSSCRNS.SHP`; every other width uses `MPSSCRNL.SHP`. Assembly: `0x0072D86C` compares side with `1`, `0x0072D871` compares width with `0x280`, `0x0072D87D` loads `[0x00844CA8] = MPSSCRNS.SHP`, and `0x0072D88E` loads `[0x00844CAC] = MPSSCRNL.SHP`.

`FUN_005C9720 @ 0x005C9720` performs the movie lifecycle:

1. Reads `ScenarioClass+0x34B8` into `ECX`, calls `FUN_0072D730`.
2. `FUN_0072D730` lazily calls `RadarTransitionMovie_SHPLoad`, then loads the movie palette/ConvertClass wrapper into `0x00B0FBB4`, and sets `DAT_00B0FBB8 = 1`.
3. Calls `FUN_0072EAD0` with full-screen rect `{0,0,g_ScreenWidth,g_ScreenHeight}` against `DAT_0088730C`.
4. Calls `FUN_004F4780(0)`.
5. Runs message pump / wait helper `FUN_0060D380` with message handler pointer `0x005C9B10` and dialog/message id `0x108`.
6. Calls `FUN_0072EAD0` again, then `FUN_004F4780(0)` again.
7. Calls `FUN_0072D780` cleanup.

Assembly evidence: `0x005C9723..0x005C97A4`. `FUN_0072EAD0` itself calls `Fill_Margins`, `RightPanel__Draw(0)`, then draws `0x00B0FB1C` frame `0` at `DAT_00B0FC1C.x/y` with flags `0x400` and z `1000` (`0x0072EAD9..0x0072EB37`).

Cleanup `FUN_0072D780 @ 0x0072D780` clears `DAT_00B0FBB8`, conditionally frees `g_MinimapMovie_SHP` only when `DAT_00B0FC7D != 0`, zeros `0x00B0FB1C`, frees wrapper `0x00B0FBB0`, and destroys ConvertClass object `0x00B0FBB4`.

### 3.5 Palette / ConvertClass routes

Active in YR: Yes.

The transition wrappers load palette-specific `ConvertClass` objects through `0x0072ADE0`. That helper reads a `.PAL`, expands 256 RGB triplets by shifting each channel left by `2`, constructs `ConvertClass(*pal, *pal, DAT_00887310, 1, 0)`, and stores it to the wrapper global.

For Soviet:

| Path | Wrapper assembly | Palette pointer | Palette | ConvertClass global |
|---|---|---:|---|---|
| `SSCR*` radar-frame family | `0x0072D31D..0x0072D338` | `0x00844BCC` | `SSCORE.PAL` | `0x00B0FBA8` |
| `MPSSCRN*` movie family | `0x0072D74D..0x0072D768` | `0x00844BD8` | `MPSSCRN.PAL` | `0x00B0FBB4` |

String evidence from retail `gamemd.exe`: `0x00844BCC -> 0x0084549C "SSCORE.PAL"` and `0x00844BD8 -> 0x00845478 "MPSSCRN.PAL"`. `FUN_0072D450` returns `0x00B0FBA8` for owner-draw `SSCRA*`; `FUN_0072E9F0` and `FUN_0072EAD0` pass `0x00B0FBA8` / `0x00B0FBB4` respectively to the draw helper.

### 3.6 Ordinary in-game minimap split

Active in YR: Yes.

The ordinary player-sidebar minimap path is not the `SSCRA*` owner-draw static path and not the `MPSSCRN*` right-panel movie path. Existing verified `PowerClass::Draw -> RadarClass::Draw` reports show ordinary content uses `BKGDLG/BKGDLGY`, `g_SidebarSurface`, and `RadarClass` fields `+0x11E4..+0x1218` with content aperture `140x108` at sidebar-local `(16,49)`. `SSCR*`/`MPSSCRN*` use the separate right-panel parent rect `DAT_00B0FC1C`, while `SSCRA*` uses owner-draw child `0x72B`.

## 4. INI Keys

No INI key controls these specific selector filenames, palette names, or wrapper lifetimes. The side value comes from `ScenarioClass+0x34B8`, and the width split uses `g_ScreenWidth == 0x280`.

## 5. Integration Points

| Integration point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00685670` | scenario flow reaches open-frame wrapper `FUN_006C8E80`; also reaches `FUN_005C9720` on nonzero game-mode exit/restart branch | decompile `0x00685670`; xref to `0x006C8E80` at `0x00685960`; xref to `0x005C9720` at `0x0068588B` | Yes / Conditional by branch |
| `FUN_00685DC0` | sibling scenario flow reaches `FUN_005C9720` | decompile `0x00685DC0`; xref at `0x00685FD5` | Conditional |
| `FUN_006C8E80` | open/background right-panel draw wrapper: load, draw twice around message pump, cleanup | `0x006C8E80..0x006C8F09` | Yes |
| `FUN_005C9720` | `MPSSCRN*` transition movie wrapper: load, draw twice around message pump, cleanup | `0x005C9720..0x005C97A9` | Yes |
| `WM_PAINT_Handler @ 0x00621F00` | static background `RadarBackground @ 0x0072E920` on right-panel flag byte | xref `0x006221C7`; decompile | Conditional on window flag |
| `FUN_0060A5B0` / `OwnerDraw_Static_006153E0` | `SSCRA*` owner-draw setup/paint | decompile and assembly above | Conditional |

## 6. Current Rust Implementation Status

`src/render/radar_anim.rs` models one generic `radar.shp` 33-frame state machine. It starts offline at last frame, opens by decrementing toward frame `0`, closes by incrementing, and uses `64.0` ms per frame.

`src/render/sidebar_chrome.rs` builds Soviet sidebar chrome from `sidec02.mix`, `sidebar.pal`, and generic `radar.shp`; it also pre-renders `radar.shp` frames and derives content insets from frame transparency. It does not model `SSCRBK*` / `SSCRT*` / `SSCRA*`, `MPSSCRN*`, `SSCORE.PAL`, `MPSSCRN.PAL`, right-panel `DAT_00B0FC1C`, or owner-draw child `0x72B`.

`src/app_building_anim.rs` updates radar availability via `update_radar_state`, calling `RadarAnimState::set_has_radar` and `tick`. That ties gameplay radar availability to the generic `radar.shp` animation; no shell/right-panel transition movie or owner-draw static protocol exists.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RadarBackground_SHPLoad @ 0x0072D460` Soviet `SSCRA*` selector | verified | decompile + assembly `0x0072D4F4..0x0072D579`; retail string pointers | none |
| `FUN_0072D300` `SSCR*` wrapper and `SSCORE.PAL` route | verified | assembly `0x0072D300..0x0072D338`, `0x00844BCC` string | none |
| `FUN_006C8E80` open-frame wrapper | verified | decompile + assembly `0x006C8E80..0x006C8F09` | none |
| `FUN_0072E9F0` direct open-frame draw | verified | decompile + assembly `0x0072EA44..0x0072EA57` | none |
| `RadarBackground @ 0x0072E920` static `SSCRT*` draw | verified | decompile + assembly `0x0072E920..0x0072E97D` | none |
| `RadarTransitionMovie_SHPLoad @ 0x0072D830` Soviet `MPSSCRN*` selector | verified | decompile + assembly `0x0072D86C..0x0072D899`; retail strings | none |
| `FUN_0072D730` movie wrapper and `MPSSCRN.PAL` route | verified | assembly `0x0072D730..0x0072D768`, `0x00844BD8` string | none |
| `FUN_005C9720` movie timing/lifecycle wrapper | verified | decompile + assembly `0x005C9720..0x005C97A9` | exact message-handler internals at `0x005C9B10` deferred |
| `FUN_0072EAD0` movie draw | verified | decompile + assembly `0x0072EAD0..0x0072EB37` | none |
| `FUN_0072D780` movie cleanup | verified | decompile + assembly `0x0072D780..0x0072D7F1` | live cache contents deferred |
| `FUN_00603870` `SSCRA*` provider | verified | decompile `0x00603870` | none |
| `FUN_0060A5B0` owner-draw setup | verified | decompile + assembly `0x0060A944..0x0060A9C1` | none |
| `OwnerDraw_Static_006153E0` kind-4 paint and message protocol | verified | decompile + assembly `0x006159EB..0x00615A17`, `0x006C93C2` | runtime screenshot not captured |
| Ordinary in-game minimap split | verified by sibling docs | `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `RADAR_CHROME_COMPOSITING.md` | none for this scope |

## 8. Open Questions - Final State

- `[RESOLVED] Q1 - What consumes loaded SSCRA*/g_RadarFrameClose_SHP? -> Owner-draw static provider FUN_00603870 returns it for dialogs 0x103/0xBC7 child 0x72B; setup stores it at record +0x78 and paint draws from that record.` (evidence: `0x00603870`, `0x0060A9B2`, `0x006153E0`)
- `[RESOLVED] Q2 - Is SSCRA* drawn by FUN_0072EAD0? -> No; FUN_0072EAD0 reads 0x00B0FB1C, not 0x00B0FB30.` (evidence: `0x0072EB24`)
- `[RESOLVED] Q3 - Is SSCRA* drawn by direct right-panel open/background helpers? -> No; open reads 0x00B0FB34 and static background reads 0x00B0FB00.` (evidence: `0x0072EA44`, `0x0072E962`)
- `[RESOLVED] Q4 - What starts the SSCRA* owner-draw animation? -> Message 0x4D3 starts kind-4 timer; active send site pushes 0x4D3 to child 0x72B.` (evidence: `0x006153E0`, `0x006C93C2`)
- `[RESOLVED] Q5 - Can SSCRA* frame be forced? -> Message 0x4D5 stores frame into record +0x98; send site passes frame 0x1D to child 0x72B.` (evidence: `0x006153E0`, `0x006C93CD..0x006C93DB`)
- `[RESOLVED] Q6 - What is the MPSSCRN* lifecycle? -> FUN_005C9720 loads through 0x0072D730, draws 0x0072EAD0, pumps 0x0060D380, draws again, then cleans up through 0x0072D780.` (evidence: `0x005C9720..0x005C97A9`)
- `[RESOLVED] Q7 - Which palette/ConvertClass does Soviet SSCR* use? -> SSCORE.PAL through wrapper global 0x00B0FBA8.` (evidence: `0x0072D31D..0x0072D338`, `0x00844BCC`)
- `[RESOLVED] Q8 - Which palette/ConvertClass does Soviet MPSSCRN* use? -> MPSSCRN.PAL through wrapper global 0x00B0FBB4.` (evidence: `0x0072D74D..0x0072D768`, `0x00844BD8`)
- `[RESOLVED] Q9 - Is ordinary in-game minimap content based on SSCRA*/MPSSCRN* geometry? -> No; sibling docs prove ordinary content uses RadarClass fields and sidebar-local 140x108 aperture at (16,49).` (evidence: `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`)
- `[DEFERRED] Q10 - What are live runtime cache contents before a transition in an already-running process?` (category: `needs-runtime-debugger`; reason: static evidence proves cache rules, not current process cache state; next-step-if-pursued: attach debugger and inspect `LoadFileFromMIX` cache tree before `FUN_005C9720`)
- `[DEFERRED] Q11 - Does Win32 runtime ever MoveWindow child 0x72B after resource creation?` (category: `needs-runtime-debugger`; reason: resource geometry and owner-draw setup were checked, but no live window hook was captured; next-step-if-pursued: breakpoint `MoveWindow`/`SetWindowPos` for child `0x72B`)
- `[DEFERRED] Q12 - What does message handler 0x005C9B10 do internally during the MPSSCRN wait?` (category: `out-of-scope`; reason: wrapper timing/lifecycle was enough for asset handoff; next-step-if-pursued: separate dialog/message-pump investigation)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `FUN_0072E9F0 @ 0x0072E9F0` | called by `FUN_006C8E80`; side-selected wrapper already loaded | `SSCRBK*` frame `0` via `0x00B0FB34` | `DAT_00B0FC1C.x/y` | `SSCORE.PAL` -> `0x00B0FBA8` | Yes | direct right-panel open/static frame |
| 2 | `RadarBackground @ 0x0072E920` | `WM_PAINT_Handler` flag path | `SSCRT*` frame `0` via `0x00B0FB00` | `DAT_00B0FC1C.x + (screen_w > 799 ? 80 : 0)`, y unchanged | `0x00B0FBA8` | Conditional | right-panel static background |
| 3 | `FUN_0072EAD0 @ 0x0072EAD0` | called by `FUN_005C9720` before/after message pump | `MPSSCRN*` frame `0` via `0x00B0FB1C` | `DAT_00B0FC1C.x/y` | `MPSSCRN.PAL` -> `0x00B0FBB4` | Yes | direct transition movie |
| 4 | `OwnerDraw_Static_006153E0 @ 0x006153E0` | kind `4` record for `(0x103/0xBC7, 0x72B)` | `SSCRA*` current frame via record `+0x78/+0x98` | child client rect; strict-centering only if SHP smaller | `SSCORE.PAL` -> record `+0x74` | Conditional | owner-draw close/static animation |
| 5 | `RadarClass::Update @ 0x00656EC0` | ordinary radar online redraw | `BKGDLG/BKGDLGY` frame `32`, then minimap surface blit | sidebar-local chrome `(0,48)`, content `(16,49)` max `140x108` | sidebar radar convert + surface blit | Yes | ordinary in-game minimap |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `SSCRBKSM/MD.SHP` | Yes | Yes | Yes on direct open/right-panel path | No | Yes | No | Yes | No | `0x0072D460`, `0x0072E9F0` |
| `SSCRTSM/MD.SHP` | Yes | Yes | Conditional static background | No | Yes | No | Conditional | No | `0x0072D460`, `0x0072E920` |
| `SSCRASM/MD.SHP` | Yes | Conditional | Conditional in owner-draw dialog child | No | No | Static child image | Conditional | No | `0x00603870`, `0x006153E0` |
| `MPSSCRNS/L.SHP` | Yes | Yes | Yes on `FUN_005C9720` transition path | No | Yes/movie frame | No | Yes | No | `0x0072D830`, `0x0072EAD0` |
| `BKGDLG/BKGDLGY.SHP` | Yes in ordinary sidebar path | Yes | Yes for ordinary in-game minimap | No | Yes | No | No | No | `RADAR_CHROME_COMPOSITING.md` |
| generated minimap surface | Yes | Yes | Yes when ordinary radar online | Yes | No | No | No | No | `0x00656EC0`, sibling docs |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `SSCRA*` is loaded with `SSCRBK*`/`SSCRT*` but is not the direct `MPSSCRN*` transition movie; it is consumed by owner-draw static child `0x72B` for dialogs `0x103/0xBC7`. | `0x0072D460`, `0x00603870`, `0x0060A9B2`, `0x006153E0`; negative draw evidence `0x0072EB24` | Missing; Rust has one generic `radar.shp` animation. | `src/render/radar_anim.rs`, future shell/static owner-draw support, `src/render/sidebar_chrome.rs` | Keep `SSCRA*` as a separate owner-draw/dialog asset family, not as the right-panel movie or ordinary minimap chrome. | Trigger child `0x72B` kind-4 setup for Soviet and verify image source is `SSCRASM/SSCRAMD`, while `FUN_0072EAD0` equivalent uses `MPSSCRNS/L`. Proposed test: `test_soviet_sscra_close_static_is_not_mp_transition_movie`. | HIGH; conflating assets produces wrong frames/palette/placement. |
| `MPSSCRN*` transition wrapper loads, draws frame `0`, pumps messages, draws frame `0` again, then clears globals and only frees MIX-backed payload when fallback flag is set. | `0x005C9720..0x005C97A9`, `0x0072D730`, `0x0072EAD0`, `0x0072D780`; cache report | Missing; Rust radar availability only ticks generic `RadarAnimState`. | `src/render/radar_anim.rs`, `src/render/sidebar_chrome.rs`, app transition flow | Add a distinct right-panel/movie transition lifecycle for `MPSSCRNS/L` if this shell/scenario path is implemented; respect separate wrapper and cleanup/cache semantics. | At Soviet non-640 scenario transition, load `MPSSCRNL.SHP`, draw frame `0` at `DAT_00B0FC1C`, pump/wait, draw again, and clear global without dropping cached MIX payload. Proposed test: `test_soviet_mpsscrn_transition_draws_twice_then_clears_global_cache_safe`. | HIGH; do not approximate with `radar.shp` open/close frames. |
| Soviet `SSCR*` uses `SSCORE.PAL`/`0x00B0FBA8`, while Soviet `MPSSCRN*` uses `MPSSCRN.PAL`/`0x00B0FBB4`. | `0x0072D31D..0x0072D338`, `0x0072D74D..0x0072D768`, strings `0x00844BCC`, `0x00844BD8`, `0x0072ADE0` | Missing; `src/render/sidebar_chrome.rs` decodes Soviet generic sidebar pieces with `sidebar.pal`. | asset decode/palette routing for sidebar/radar transition assets | Decode each transition family with its native palette and keep ConvertClass ownership separate. | Soviet `SSCRAMD` colors come from `SSCORE.PAL`; `MPSSCRNL` colors come from `MPSSCRN.PAL`; changing one palette does not recolor the other. Proposed test: `test_soviet_sscr_and_mpsscrn_use_distinct_palette_routes`. | HIGH pixel parity risk; do not decode all Soviet radar transition art through `sidebar.pal`. |
| Ordinary in-game minimap content is not based on `SSCRA*` or `MPSSCRN*`; it uses `RadarClass` surface blit into `BKGDLG/BKGDLGY` at sidebar-local `(16,49)` max `140x108`. | `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `RADAR_CHROME_COMPOSITING.md`, `0x00656EC0` | Rust currently derives content insets from generic `radar.shp` transparency and conflates chrome/animation paths. | `src/render/sidebar_chrome.rs`, `src/render/minimap.rs`, `src/app_render/build_instances.rs` | Split ordinary in-game minimap aperture from right-panel `SSCR*`/`MPSSCRN*` and owner-draw `SSCRA*` paths. | Moving/changing right-panel `DAT_00B0FC1C` or owner-draw child rect does not move the ordinary in-game terrain aperture. Proposed test: `test_ingame_minimap_aperture_ignores_sscr_mpsscrn_and_sscra_paths`. | HIGH; do not use transition-art geometry as minimap content geometry. |

## Negative Facts / Do Not Do

- Do not draw `SSCRA*` from `FUN_0072EAD0`; active-YR direct movie draw reads `0x00B0FB1C` (`MPSSCRN*`), not `0x00B0FB30` (`SSCRA*`). Evidence: `0x0072EB24`.
- Do not place `SSCRA*` at `DAT_00B0FC1C` or apply the static background `+80` x-offset; the verified `SSCRA*` paint path is owner-draw child centering/clipping in `OwnerDraw_Static_006153E0`. Evidence: `0x006153E0`, `0x0072E920`, `0x0072EAD0`.
- Do not model `MPSSCRN*` and `SSCRA*` as the same close animation. They are loaded into different globals, use different palettes, and have different consumers. Evidence: `0x0072D460`, `0x0072D830`, `0x00844BD8`, `0x00844BCC`.
- Do not use `sidebar.pal` as the native palette for Soviet right-panel `SSCR*`/`MPSSCRN*` transition assets. Evidence: wrapper palette pointers `SSCORE.PAL` and `MPSSCRN.PAL`.
- Do not infer ordinary in-game minimap aperture from `SSCR*`/`MPSSCRN*` or `SSCRA*`; that aperture is the separate `RadarClass` surface path. Evidence: sibling minimap content report and no usage in `0x00656EC0`.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace the section title `FUN_0072ead0 -- Radar Frame Close Transition` with `FUN_0072EAD0 -- MP*SCRN* transition movie draw; this function does not draw g_RadarFrameClose_SHP / SSCRA*`.
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace any wording that places `SSCRA*` through the right-panel `DAT_00B0FC1C` path with `SSCRA* is drawn by the owner-draw static path for child 0x72B under dialogs 0x103/0xBC7; standard resource geometry maps the child to 423x229, so the 424x230 SSCRA* frame draws at child origin and clips rather than centering.`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/RADAR_CHROME_COMPOSITING.md`: replace generic close-frame wording with `Current Rust uses generic radar.shp, but native Soviet shell/radar paths use separate SSCRBK*/SSCRT*/SSCRA* and MPSSCRN* assets; SSCRA* is consumed by the owner-draw static path for radar dialog child 0x72B, not by FUN_0072EAD0.`

## Remaining Uncertainty

- Live runtime cache contents before transition were not inspected. Static cache rules and stock cold winner are covered by `MPSSCRNL_DUPLICATE_RUNTIME_WINNER_CACHE_STATES_GHIDRA_REPORT.md`.
- A live Win32 hook was not used to prove no later `MoveWindow` affects child `0x72B`; resource geometry plus owner-draw setup/paint evidence are sufficient for this handoff but leave a runtime-screenshot audit useful.
- Message handler `0x005C9B10` was not decompiled in this slot; wrapper-level `FUN_005C9720` timing and draw/cleanup order were verified.

## Sources

- Ghidra read-only decompile/assembly: `0x0072D460`, `0x0072D300`, `0x006C8E80`, `0x0072E920`, `0x0072E9F0`, `0x0072D830`, `0x0072D730`, `0x005C9720`, `0x0072EAD0`, `0x0072D350`, `0x0072D780`, `0x00603870`, `0x0060A5B0`, `0x006153E0`, `0x006035F0`, `0x006033F0`, `0x0072ADE0`.
- Retail `gamemd.exe` string pointer reads: `0x00844C70`, `0x00844C74`, `0x00844C78`, `0x00844C7C`, `0x00844C80`, `0x00844C84`, `0x00844CA8`, `0x00844CAC`, `0x00844BCC`, `0x00844BD8`.
- Prior reports: `SOVIET_RADAR_LEFT_PANEL_SHP_SELECTORS_FOLLOWUP_GHIDRA_REPORT.md`, `SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT.md`, `SOVIET_RADAR_MINIMAP_CONTENT_INSET_GHIDRA_REPORT.md`, `SSCRA_CLOSE_FRAME_DRAW_LIFECYCLE_GHIDRA_REPORT.md`, `SSCRA_CHILD_0X72B_DIALOG_RECT_GEOMETRY_GHIDRA_REPORT.md`, `MPSSCRNL_DUPLICATE_RUNTIME_WINNER_CACHE_STATES_GHIDRA_REPORT.md`, `RADAR_CHROME_COMPOSITING.md`.

## Status

COMPLETE for the requested close/open transition asset lifecycle slice. Remaining uncertainty is limited to live runtime cache/window-position observation and does not change the verified Rust-facing split between `SSCRA*`, `MPSSCRN*`, and ordinary in-game minimap paths.
