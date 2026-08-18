# Loading Screen WM_PAINT Mode 2 Composition - Ghidra Research Report

**Address(es):** `0x00621E90` (primary), `0x004AED70` (`CC_Draw_Shape`), `0x00775690` (window-to-client rect helper)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** only the `piVar9[0x2c] == 2` branch of `WM_PAINT_Handler`, including its surface copy, loading-screen SHP selection, `CC_Draw_Shape` call, and final blit.  
**Non-Scope:** mode 1 shell/sidebar composition, dialog activation path into mode 2, SHP load/free lifecycle, palette load lifecycle, runtime progress manager updates, and child-window paints outside this branch.  
**Confidence:** High for branch composition and call arguments; Medium for player-facing activation because this slot did not trace the upstream mode writer.  
**Active in YR:** Conditional - active when the shell/dialog record mode field is `2`; prior reports and current code context identify this as the loading-screen mode.

## 1. Overview

`WM_PAINT_Handler` mode 2 composes the loading screen by copying the current `DAT_00887310` surface region into the dialog's cached `BSurface`, selecting exactly one `PUDLGBG*.SHP` plus one DIALOG-family `ConvertClass`, drawing frame `0` at `(0,0)`, and blitting the cached surface back to `DAT_00887310`. The branch contains no text, map name, status, spinner, or progress-bar draw call.

## 2. Class Layout / Key Offsets

| Field / global | Offset / address | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|---|
| Dialog record data pointer used as `piVar9` | `record + 4` | int* | Base for mode and cache fields | `0x00621E90` decompile | Conditional |
| Suppress paint gate | `piVar9[8]` / data `+0x20` | int | If nonzero, all composition is skipped | `0x00621E90` decompile | Yes |
| Cached `BSurface` pointer | `piVar9[4]` / data `+0x10` | pointer | Lazy-created offscreen composition surface | `0x00621E90` allocation block | Yes |
| Mode selector | `piVar9[0x2c]` / data `+0xB0` | int | `1` shell/sidebar, `2` loading screen, other PCX fallback | `0x00621E90` branch | Yes |
| Main/alternate surface | `DAT_00887310` | surface pointer | Source for first copy and destination for final blit | `0x00621E90` calls vtable `+8` | Yes |
| Neutral loading SHP | `DAT_00B0FC80` | SHP pointer | `PUDLGBGN.SHP` | branch read + string `0x00845328` | Conditional |
| Allied loading SHP | `DAT_00B0FC84` | SHP pointer | `PUDLGBGA.SHP` | branch read + string `0x00845318` | Conditional |
| Soviet loading SHP | `DAT_00B0FC88` | SHP pointer | `PUDLGBGS.SHP` | branch read + string `0x00845308` | Conditional |
| Yuri loading SHP | `DAT_00B0FC8C` | SHP pointer | `PUDLGBGY.SHP` | branch read + string `0x008452F8` | Conditional |
| Side selector | `g_ScenarioClass_Instance + 0x34B8` | int | `0` Allied, `1` Soviet, all other in-game values route to Yuri art/palette | `0x00621E90` decompile | Conditional |

## 3. Core Logic

1. Resolve the window rectangle relative to `g_hWnd` client coordinates via `FUN_00775690`.
2. Compute cached-surface width/height as at least the client rect dimensions and at least the window-relative rect span.
3. If no cached `BSurface` exists, allocate a `0x20` header, set width and height from `GetClientRect`, set pixel format constant `2`, and initialize a `width * height * 2` byte pixel buffer.
4. If mode is `2`, build a source/draw rect `{x=0, y=0, w=local_38, h=local_34}`.
5. Copy `DAT_00887310` into the cached `BSurface` using source rect `{window_left, window_top, local_38, local_34}` and destination rect `{0,0,local_38,local_34}`. This copy happens before the SHP draw.
6. Select loading art and palette:

| Condition | ConvertClass | SHP | Evidence | Active in YR |
|---|---|---|---|---|
| `FUN_0069BBE0()` returns `0` | `FUN_0072B030()` -> `DAT_00B0FB60` / `DIALOGN.PAL` | `DAT_00B0FC80` / `PUDLGBGN.SHP` | `0x006221F4..0x0062226A` branch and accessors | Conditional: no-game/menu state |
| In game and side `0` | `FUN_0072AFF0()` -> `DAT_00B0FB68` / `DIALOG.PAL` | `DAT_00B0FC84` / `PUDLGBGA.SHP` | same | Conditional: Allied |
| In game and side `1` | `FUN_0072AFF0()` -> `DAT_00B0FB68` / `DIALOG.PAL` | `DAT_00B0FC88` / `PUDLGBGS.SHP` | same | Conditional: Soviet |
| In game and side not `0` or `1` | `FUN_0072B010()` -> `DAT_00B0FB70` / `DIALOGY.PAL` | `DAT_00B0FC8C` / `PUDLGBGY.SHP` | same | Conditional: Yuri / other side value |

7. If either selected `ConvertClass` or selected SHP pointer is null, skip the shape draw and continue to the final blit.
8. Otherwise call `CC_Draw_Shape @ 0x004AED70` with:

| Argument role | Value in mode 2 | Evidence |
|---|---|---|
| Destination surface (`ECX`) | cached `BSurface` | assembly at `0x006222A6` |
| ConvertClass (`EDX`) | selected accessor return | assembly at `0x006222A4` |
| SHP pointer | selected `PUDLGBG*` pointer | assembly at `0x006222A3` |
| Frame | `0` | assembly at `0x006222A2` |
| Position | `{x=0, y=0}` | stack locals written at `0x0062226C..0x00622270` |
| Clip/draw rect | `{x=0, y=0, w=local_38, h=local_34}` | mode-2 local setup before first blit |
| Flags | `0x400` | assembly at `0x00622297` |
| Z/depth parameter | `0` | pushes around `0x00622290..0x00622292` |
| Priority / z-height style constant | `1000` (`0x3E8`) | assembly at `0x0062228B` |
| Remaining optional params | zero/null | pushes around `0x00622286..0x00622292` |

`CC_Draw_Shape` then intersects the caller rect with the destination-surface clip, returns early if width or height is `< 1`, stores the zero Z parameter without ORing flag `0x10`, and does not center because flag `0x200` is absent. Existing `CC_Draw_Shape` evidence says bit `0x400` is reserved/no-effect in the core selector path; it is still passed by this call and should be preserved for parity.

9. After mode 2 completes, the shared tail blits the cached `BSurface` to `DAT_00887310` at the window-relative destination rect. The source rect is the corresponding `{0,0,local_38,local_34}` region of the cached surface.

## 4. INI Keys

No INI keys are read in the scoped branch. The selected side comes from `ScenarioClass + 0x34B8`, not from an INI lookup in this function.

## 5. Integration Points

| Integration | Direction | Evidence | Notes |
|---|---|---|---|
| Common shell `WM_PAINT` path | caller/context | prior reports cite `FUN_00622B50` dispatch to `WM_PAINT_Handler` | Upstream activation is slot-5 scope, not re-traced here |
| `FUN_00775690` | called | decompiled `0x00775690` | Converts window rect to main-client coordinates |
| Cached `BSurface` vtable `+8` | called before and after SHP draw | `0x00621E90` decompile | First copy from `DAT_00887310`; tail copy back to it |
| DIALOG palette accessors | called | `0x0072AFF0`, `0x0072B010`, `0x0072B030` decompiled | Accessors return already-loaded ConvertClass globals |
| `CC_Draw_Shape` | called | `0x006222A8` | Only mode-2 shape draw in this branch |

## 6. Current Rust Implementation Status

Current Rust does not match the scoped branch. `src/app.rs` sets `GameScreen::Loading` from quickplay/skirmish launch paths, clears the screen, starts an egui frame, calls `main_menu::draw_loading_screen`, presents, then synchronously transitions to in-game after one presented loading frame. `src/ui/main_menu.rs::draw_loading_screen` paints themed egui background, optional texture, rounded overlay panel, and text labels including "Mission deployment", "Loading...", map name, and descriptive status text; `loading_screen_image()` currently returns `None`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `WM_PAINT_Handler` mode `2` branch | verified | `0x00621E90`, assembly around `0x006221F4..0x006222AD` | none for scoped branch |
| First BSurface copy from `DAT_00887310` | verified | `0x00621E90` decompile; vtable `+8` call before side selection | concrete vtable name not typed |
| Loading SHP side selection | verified | `0x00621E90`; strings `0x008452F8..0x00845328` | load/free lifecycle out of scope |
| DIALOG-family palette accessors | verified | `0x0072AFF0`, `0x0072B010`, `0x0072B030` | palette construction lifecycle out of scope |
| `CC_Draw_Shape` argument stack/register handoff | verified | assembly context at `0x00622282..0x006222A8` | no runtime screenshot in this slot |
| Text/progress/status overlay inside mode 2 branch | verified negative | no text/progress API calls in branch between `0x006221F4` and `0x006222AD`; branch jumps to shared blit | progress-manager activation outside WM_PAINT branch remains separate slot |
| Mode-2 upstream activation | deferred | not traced in this slot | slot 5 should trace Start Game -> mode field writer |
| SHP asset load/free lifecycle | deferred | globals consumed only here | slot 2 should trace `DAT_00B0FC80..8C` writers |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What is the bounded target? -> Exhaustive slice of WM_PAINT mode 2 only.` (evidence: user scope; `0x00621E90`)
- `[RESOLVED] OQ-2 - Does mode 2 allocate/reuse the same cached BSurface as other branches? -> Yes, allocation is shared before mode dispatch and stored at data +0x10.` (evidence: `0x00621E90`)
- `[RESOLVED] OQ-3 - Does mode 2 clear the BSurface before drawing? -> No explicit clear in the branch; it copies from `DAT_00887310` first, then draws the SHP.` (evidence: `0x00621E90`)
- `[RESOLVED] OQ-4 - How many SHP draw calls are in the branch? -> One guarded `CC_Draw_Shape` call.` (evidence: `0x006222A8`)
- `[RESOLVED] OQ-5 - Which SHP frame is drawn? -> Frame 0.` (evidence: push zero immediately before SHP pointer at `0x006222A2..0x006222A3`)
- `[RESOLVED] OQ-6 - What is the anchor? -> Top-left `(0,0)`, not centered.` (evidence: position locals set to zero; flags omit `0x200`)
- `[RESOLVED] OQ-7 - Which palette does Soviet use? -> `DIALOG.PAL`, same accessor as Allied, not `DIALOGY.PAL`.` (evidence: `0x00622240..0x00622250`, `0x0072AFF0`)
- `[RESOLVED] OQ-8 - What happens when no game is active? -> Neutral SHP plus `DIALOGN.PAL`.` (evidence: `0x0062225F..0x00622265`, `0x0072B030`)
- `[RESOLVED] OQ-9 - Does a null palette or null SHP draw anything? -> No; both are checked before the call.` (evidence: `0x0062226A..0x0062227C`)
- `[RESOLVED] OQ-10 - Is there text, map name, status, or progress overlay in this branch? -> No.` (evidence: only vtable blit, accessors, and `CC_Draw_Shape` are called in branch)
- `[RESOLVED] OQ-11 - What clips the loading SHP? -> Caller rect `{0,0,local_38,local_34}` intersected inside `CC_Draw_Shape` with destination-surface clip.` (evidence: `0x00621E90`; `0x004AED70`)
- `[RESOLVED] OQ-12 - Does nonzero Z/depth alter flags? -> No; mode 2 passes Z/depth `0`, so `CC_Draw_Shape` does not OR `0x10`.` (evidence: `0x00622290..0x006222A8`; `0x004AED70`)
- `[DEFERRED] OQ-13 - What creates mode 2 before this paint?` (category: `requires-different-system-context`; reason: upstream Start Game/loading activation is outside this slot; next-step-if-pursued: trace dialog record mode writer and invalidation path)
- `[DEFERRED] OQ-14 - Where are `PUDLGBG*` globals loaded/freed?` (category: `requires-different-system-context`; reason: this branch only consumes globals; next-step-if-pursued: trace writers to `DAT_00B0FC80..8C`)
- `[DEFERRED] OQ-15 - Does the progress manager trigger repaints or status outside this branch?` (category: `requires-different-system-context`; reason: no overlay in this branch, but runtime progress callbacks are a separate owner; next-step-if-pursued: trace load progress callback path)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `BSurface` vtable `+8` from `0x00621E90` | mode `2`, paint gate clear | current `DAT_00887310` pixels | src `{window_left,window_top,w,h}` -> dst `{0,0,w,h}` | already-present surface pixels | yes | backbuffer seed/copy |
| 2 | `CC_Draw_Shape @ 0x004AED70` call at `0x006222A8` | selected ConvertClass != null and SHP != null | `PUDLGBGN/A/S/Y.SHP`, frame `0` | anchor `{0,0}`, clip `{0,0,w,h}`, flags `0x400` | `DIALOGN`, `DIALOG`, or `DIALOGY` ConvertClass in `EDX` | yes, conditional by state/side | loading-screen chrome/content |
| 3 | shared tail `DAT_00887310` vtable `+8` | cached BSurface non-null | composed cached `BSurface` | src `{0,0,w,h}` -> dst `{window_left,window_top,w,h}` | 16-bpp surface copy | yes | present/blit |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `PUDLGBGN.SHP` | consumed from `DAT_00B0FC80` | yes if no-game state | conditional | yes | yes | no text overlay | yes | inactive in in-game side branches | `0x006221F4..0x0062226A` |
| `PUDLGBGA.SHP` | consumed from `DAT_00B0FC84` | yes if side `0` | conditional | yes | yes | no text overlay | yes | inactive for Soviet/Yuri/no-game | `0x00622233..0x0062223E` |
| `PUDLGBGS.SHP` | consumed from `DAT_00B0FC88` | yes if side `1` | conditional | yes | yes | no text overlay | yes | inactive for Allied/Yuri/no-game | `0x00622240..0x00622250` |
| `PUDLGBGY.SHP` | consumed from `DAT_00B0FC8C` | yes if side not `0/1` while in-game | conditional | yes | yes | no text overlay | yes | inactive for Allied/Soviet/no-game | `0x00622252..0x0062225D` |
| `DIALOG.PAL` ConvertClass | accessor return `DAT_00B0FB68` | palette input, not drawn | Allied/Soviet | no | no | palette | yes | inactive for Yuri/no-game | `0x0072AFF0`; call sites |
| `DIALOGY.PAL` ConvertClass | accessor return `DAT_00B0FB70` | palette input, not drawn | Yuri | no | no | palette | yes | inactive for Allied/Soviet/no-game | `0x0072B010`; call site |
| `DIALOGN.PAL` ConvertClass | accessor return `DAT_00B0FB60` | palette input, not drawn | no-game/menu state | no | no | palette | yes | inactive in in-game side branches | `0x0072B030`; call site |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Loading paint draws one side-selected `PUDLGBG*` SHP frame 0 at top-left with DIALOG-family ConvertClass and no egui text. | `0x00621E90`, `0x006222A8` | missing/mismatch: Rust egui overlay/text, no SHP draw | `src/app.rs`, `src/ui/main_menu.rs`, future render/assets loading-screen surface | Render indexed retail SHP frame 0, top-left anchored, side-selected. | Start skirmish as Allied/Soviet/Yuri and first loading frame matches side art without text overlays. Proposed test: `loading_screen_draws_side_pudlgbg_frame0_without_egui_text`. | Do not render map name, "Loading...", rounded panels, gradients, or explanatory status text on this surface. |
| Allied and Soviet differ by SHP only; both use `DIALOG.PAL`. Yuri uses `DIALOGY.PAL`; no-game uses `DIALOGN.PAL`. | `0x00622233..0x00622265`; accessors `0x0072AFF0/B010/B030` | missing/mismatch: no side palette routing | loading-screen asset resolver / palette conversion | Keep side-to-palette routing exact, including Soviet using non-Yuri `DIALOG.PAL`. | Pixel/palette snapshot for Soviet loading uses `PUDLGBGS.SHP` decoded with `DIALOG.PAL`, not `DIALOGY.PAL`. Proposed test: `loading_screen_soviet_uses_dialog_pal_not_dialogy_pal`. | Do not infer palette from side name; "Soviet" is not Yuri palette. |
| Mode 2 seeds cached BSurface from existing `DAT_00887310`, draws the SHP, then blits the cached surface back; there is no branch-local clear. | `0x00621E90` first and final vtable `+8` calls | mismatch: Rust calls `clear_screen` before egui loading draw | `src/app.rs` loading render path / renderer composition | Model the native composition order or consciously replace only after screenshot evidence proves the seed copy is visually irrelevant. | Composition trace records order: copy existing surface -> draw SHP -> present; no clear command immediately before loading SHP in the parity path. Proposed test: `loading_screen_composition_copies_surface_before_pudlgbg_draw`. | Do not insert a solid-color clear as part of the parity path unless runtime captures show the source copy is always fully covered by opaque SHP pixels. |

Stale Docs / Follow-up Docs:

- Replace the deferred item in `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md` Q13 with: "Closed by `LOADING_SCREEN_WM_PAINT_MODE2_COMPOSITION_GHIDRA_REPORT.md`: `WM_PAINT_Handler` mode 2 draws exactly one selected `PUDLGBG*` SHP frame 0 through `CC_Draw_Shape`, top-left anchored at `(0,0)`, clipped to the dialog surface rect, with flags `0x400`, Z/depth `0`, priority/z-height-style constant `1000`, and no text/progress overlay in that branch."

## Sources

- Ghidra read-only decompile: `WM_PAINT_Handler @ 0x00621E90`
- Ghidra read-only assembly context: `0x006221F4..0x006222AD`
- Ghidra read-only decompile: `CC_Draw_Shape @ 0x004AED70`
- Ghidra read-only decompile: `FUN_00775690 @ 0x00775690`
- Ghidra read-only decompile: `FUN_0072AFF0`, `FUN_0072B010`, `FUN_0072B030`
- Ghidra string search/read: `PUDLGBGY.SHP @ 0x008452F8`, `PUDLGBGS.SHP @ 0x00845308`, `PUDLGBGA.SHP @ 0x00845318`, `PUDLGBGN.SHP @ 0x00845328`
- Prior report: `ALLIED_SIDEBAR_PALETTE_SELECTOR_GHIDRA_REPORT.md`
- Current Rust scan: `src/app.rs`, `src/ui/main_menu.rs`
