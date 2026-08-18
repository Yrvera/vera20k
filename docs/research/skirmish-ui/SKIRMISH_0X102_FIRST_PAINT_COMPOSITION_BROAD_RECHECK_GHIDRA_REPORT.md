# Skirmish 0x102 First-Paint Composition Broad Recheck - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072E450`, `0x0072E730`, `0x00640710`, `0x006040B0`, `0x0060B550`, `0x006153E0`, `0x004E3560`, `0x004AED70`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Standard offline Yuri's Revenge Skirmish dialog `0x102` first-paint visual composition and current Rust render backlog across parent background, right panel chrome, lower chrome, static text, flag statics, map preview surface, `STARTBUT.SHP` overlays, status strip `0x695`, invalidation/redraw boundaries, and high-res/background fallback.  
**Non-Scope:** Button middle-PCX phase, checkbox/trackbar geometry/render/input, ComboDropWin wheel/input behavior, full Choose Map `0x6B` composition, gameplay launch, and full retail screenshot pixel validation.  
**Confidence:** High for active path, top-level draw order, status-strip existence, high-res parent background no-op, flag/static/preview boundaries, and current Rust source comparison; Medium for exact aggregate first-frame pixels because this pass used static Ghidra plus Rust scan, not a retail screenshot capture.  
**Active in YR:** Yes for standard offline Skirmish `0x102`; conditional where noted for preview overlays, high-resolution width branches, static reveal, and hover/status text.

## Working Notes Gate

- Target question: Reconcile active standard offline Skirmish `0x102` first-paint composition and the current Rust backlog for broad shell surfaces outside the recently settled button/checkbox/dropdown details.
- Non-goals: Do not re-investigate button middle PCX phase, checkbox/trackbar geometry/render/input, ComboDropWin wheel/input, or `0x6B` modal details unless they alter the first-frame composition boundary.
- Evidence needed to mark COMPLETE: read-only Ghidra decompile plus assembly/caller evidence for active `0x102` entry, common parent paint, chrome/background/preview/status/static/flag paths, high-res fallback, and a current Rust scan naming implemented vs missing surfaces.
- Stop conditions: Stop once each scoped surface is either verified with an Active-in-YR judgment and Rust handoff, or explicitly deferred as screenshot/runtime-only or out-of-scope; do not mutate Ghidra or Rust.

## 1. Overview

Standard offline Skirmish enters dialog `0x102`, delegates `WM_PAINT` to the common shell first, composes right-panel/lower chrome and parent background into the parent cached surface, blits that cached result to the display surface, and only then runs Skirmish-specific preview/start-position drawing.

Current Rust is no longer in the stale "parent background first / absent widgets" state. The remaining broad first-paint backlog is: missing status/help strip `0x695`, missing right-panel static reveal state for transition-driven updates, incomplete player-name edit treatment, first-frame text/status hover plumbing, and aggregate screenshot validation. Chrome order, high-res parent-background no-op, flag PCX native clipping, preview-after-chrome ordering, and destination-surface marker clipping are now represented in current Rust.

## 2. Key Surfaces / Offsets

| Surface / field | Verified role | Active in YR | Evidence |
|---|---|---|---|
| Dialog id `0x102` + proc `0x006AE3F0` | Standard offline Skirmish setup dialog | Yes | `FUN_006AE2C0` decompile; `0x006AE40A` common-proc call |
| Parent record `+0xB0` | Common paint mode; value `1` selects right-panel/background branch | Yes for `0x102` | `WM_PAINT_Handler`; prior `FUN_0060C540`/common-parent report |
| Parent record `+0x14` | Cached parent `BSurface` pointer | Yes | `WM_PAINT_Handler` allocation and final display blit `0x006223B3` |
| Parent record `+0xE0` | 640 parent background pointer, `DAT_00B0FB50 = MNSCRNS.SHP` | Yes at width 640 | `0x0060D29C..0x0060D2A2`; loader mapping `0x0072EB9A/0x0072EBAA` |
| Parent record `+0xE4` | non-640 parent background pointer, copied from `DAT_00B0FA18` | Yes; non-null only at exact width 800 in normal lifecycle | `0x0060D2A8..0x0060D2AE`; `0x0072CF49..0x0072CF65` |
| Child `0x468` | Preview anchor; parent proc calls `DrawStartPositions`, not static paint | Yes when `DAT_00AC1154` exists | `0x006AE47B`, `DrawStartPositions @ 0x00640710` |
| Child `0x695` | Bottom-left blank-by-default status/help static | Yes | `0x00622CCB` lookup, `0x00622E83` sends `0x4B2`; `FUN_0060B550` placement |
| Static kind `1` | Animated text/reveal statics such as `0x694`, `0x6EC`, `0x5A8`, `0x695` | Conditional | `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0` |
| Static kind `2` | Image static used by flag controls `0x6DA..0x6E1` | Yes | `FUN_004E3560`, `FUN_00603D30`, `OwnerDraw_Static_006153E0` |

## 3. Core Logic

### 3.1 Active standard path

Active in YR: Yes. `FUN_006AE2C0` performs offline Skirmish setup, calls `FUN_0072CF40`, creates dialog `0x102`, pumps until Start `0x617` or Back `0x5C0`, tears down preview state, then calls `FUN_0072CF90`. `FUN_006AE3F0` begins by calling `FUN_00622B50`; assembly at `0x006AE40A` shows `CALL 0x00622b50`, `TEST EAX,EAX`, and early return if nonzero.

### 3.2 Parent/common first-paint order

Active in YR: Yes for standard `0x102` when the mode-1 branch is unsuppressed and right-panel resources are ready.

Verified order:

1. `FUN_00622B50` handles parent `WM_PAINT` and calls `WM_PAINT_Handler` at `0x00622C4F`.
2. `WM_PAINT_Handler` allocates/reuses a cached parent `BSurface`.
3. In mode `1`, it calls `RightPanel__Draw` at `0x00621FFE`.
4. It re-reads parent background fields and calls `Background_Overlay` at `0x0062211B`.
5. Optional generic extras (`Sidebar_TopHighlight`, `Minimap_Button`, `RadarBackground`) occur after background overlay.
6. The cached parent surface blits to `DAT_00887310` through vtable `+8` at `0x006223B3`.
7. `FUN_006AE3F0` then runs its `WM_PAINT` branch and calls `DrawStartPositions` at `0x006AE47B` if preview state is available and not suppressed.

This means preview pixels and live start markers are not underneath the common parent cached surface. Rust should preserve the visible ordering without copying Win32's cache internals.

### 3.3 Right panel and lower chrome

Active in YR: Yes. `RightPanel__Draw @ 0x0072E450` draws `SDTP.SHP`, repeats `SDBTNBKGD.SHP` `DAT_00B0FA20` times, optionally repeats `SDBTNANM.SHP` frame `10` only when its boolean parameter is zero, draws `SDBTM.SHP`, then draws the lower side piece. The lower piece is `LWSCRNS.SHP` at `g_ScreenWidth == 640`, otherwise `LWSCRNL.SHP`.

Current Rust status: `src/app_skirmish_shell_render.rs` now emits `RightPanelTopSdtp`, repeated `RightPanelTileSdbtnbkgd`, optional `RightPanelOverlaySdbtnanmFrame10`, `RightPanelBottomSdbtm`, lower strip, then parent background in both `skirmish_shell_semantic_draw_order` and `build_skirmish_shell_instances`. `right_panel_frame10_overlay_active` currently returns false for standard first paint, matching the verified first-frame gate. `SDBTM` uses top-clipped native drawing rather than stale full-source scaling.

### 3.4 Parent background and high-res fallback

Active in YR: Yes for 640/800; conditional for high-resolution widths.

`Background_Overlay @ 0x0072E730` compares `g_ScreenWidth` to `0x280` and uses parent `+0xE0` only at 640. Every non-640 width uses parent `+0xE4`. For standard `0x102`, `+0xE4` is copied from `DAT_00B0FA18`; `FUN_0072CF40` loads that SHP only at exact width `800`, while `FUN_0072CF90` clears it during normal cleanup. At fresh `>800`, `+0xE4` is null and `CC_Draw_Shape @ 0x004AED70` returns before frame lookup or blit; assembly `0x004AED84..0x004AED8E` is the null gate.

Current Rust status: `parent_background_role` returns `Mnscrns640` at 640, `CoopGameSetup800` at exact 800, and `None` above 800. This matches the standard fresh lifecycle. Do not regress by stretching/reusing the 800 background at 1024.

### 3.5 Static text surfaces

Active in YR: Yes for right-panel/column/static text routed through the static owner-draw path; conditional for reveal timing.

`OwnerDraw_Static_006153E0` consumes record text during `WM_PAINT`; `FUN_00602490` classifies `0x694`, `0x6EC`, and `0x5A8` as kind-1 text/reveal statics for `0x102`, and `FUN_0060A5B0` initializes kind `1` with count `1`, interval from `FUN_00600CA0`, step/range helpers, and running byte false. `0x4EE` starts reveal only if kind `1` and not already running; ordinary common first paint does not start reveal by itself.

Current Rust status: right-panel title/game/map text is rendered with top anchoring and yellow shell text, but Rust has no transition-triggered reveal state for `0x694`, `0x6EC`, and `0x5A8`. Rust still renders a literal `Player` text surface for the player-name area; the true active `0x6A0` path is an edit control and is outside this broad visual-composition proof.

### 3.6 Flag statics

Active in YR: Yes. Flag statics `0x6DA..0x6E1` are kind-2 image statics driven by side/country combo item data. `FUN_004E3560` maps item data directly to PCX cache names: `-3 -> obsi.pcx`, `-2 -> rani.pcx`, `0..9 -> usai/japi/frai/geri/gbri/djbi/arbi/lati/rusi/yrii.pcx`. The standard country item data is consistent with `ini/rulesmd.ini:959..971`.

`OwnerDraw_Static_006153E0` restores the saved background, reads native source dimensions, centers only when source is smaller than the static rect, clips when larger, applies magenta transparency, and validates the rect. It does not scale flags to fit.

Current Rust status: the stale report saying flags scaled-to-fit is no longer current. `push_flag_entry_native_clipped_centered` implements native-size centered/clipped placement, and tests cover side item-data mapping and native clipping. This surface has no broad first-paint Rust delta other than final screenshot validation.

### 3.7 Preview surface and start marker overlay

Active in YR: Preview surface is active when `DAT_00AC1154` and the child/suppress gate allow; `STARTBUT.SHP` overlays are conditional on `0 < ScenarioClass+0x113C < 9`.

`DrawStartPositions @ 0x00640710` validates the parent, gets child `0x468`, converts its rect, aspect-fits the preview source with integer `*1000` math, blits the preview surface to `DAT_00887310`, lazily loads `STARTBUT.SHP`, then draws marker frame `0` at projected anchor `(-9,-6)` and numeric label at `(-2,-6)`. The marker draw requests clip/bounds from the destination surface, not from the fitted preview image rect. Assembly contexts: marker `CC_Draw_Shape` at `0x006409D2`, label helper `FUN_004A61C0` at `0x00640A15`.

Current Rust status: preview texture draw is after shell chrome; start marker instances and labels are drawn after the preview and are no longer scissored to the fitted preview rect. Current tests include `start_marker_overlays_use_destination_surface_clip_not_preview_rect`. Remaining risk is the overlay availability gate: Rust derives overlay data from decoded preview/header bounds, while stock maps without live header start fields can have baked red PreviewPack pixels but no live `STARTBUT.SHP` overlay.

### 3.8 Status/help strip `0x695`

Active in YR: Yes. `FUN_00622B50` looks up child `0x695` during `WM_NCHITTEST` (`0x00622CCB`), hit-tests the hovered child, tries child `0x4E8`, Skirmish parent `0x4E9`, and `FUN_006040B0` `STT:*` fallback, then sends dynamic text message `0x4B2` to `0x695` (`0x00622E83`). If no source exists, the empty wide string `0x00887734` is sent.

Placement is bottom-left through `FUN_0060B550`: preserve size and set `x = center_x + 10`, `y = screen_h - child_h - center_y - 1` in the normal shell branch. Prior matrix plus the helper give `(10,459,615,20)` at 640x480, `(10,579,615,20)` at 800x600, and `(122,663,615,20)` at 1024x768. The strip is visible but blank on a no-hover first paint.

Current Rust status: there is no `status/help` field in `SkirmishShellLayout`, no state for hovered status text, and no render path for `0x695`. This is the clearest broad first-frame visual backlog item.

### 3.9 Invalidation/redraw order

Active in YR: Yes for parent/child message order; conditional for transient overlay globals.

Parent `WM_PAINT` composes and validates first; Skirmish preview then validates parent again. Owner-draw statics own cached backing surfaces and invalidate on text/backing changes, but current Rust's direct redraw model does not need Win32's cached surface objects. Separate investigation of `DAT_00AC1CC8..DAT_00AC1DD0` found those globals are a conditional transient saved-surface/status-overlay state, not a general Skirmish dirty-rectangle queue, and no standard `0x102` activation writer was statically proven.

Current Rust status: direct redraw every frame is acceptable for this composition slice. Add dirty/resource invalidation only for real Rust caches such as preview textures, not for the Win32 global overlay model.

## 4. INI Keys

No INI key controls parent paint order, static/control first-paint order, status strip placement, high-res background fallback, or preview paint ordering. The only scoped INI cross-check is the country list feeding flag item-data interpretation:

| INI source | Scoped use | Active in YR | Evidence |
|---|---|---|---|
| `ini/rulesmd.ini:959..971` `[Countries]` | Confirms standard country indices `0..9` used by side/country item data and flag PCX selection | Yes | INI scan plus `FUN_004E3560` item-data switch |

YR `rulesmd.ini` takes priority over base RA2 for this list.

## 5. Integration Points

| Integration point | Active in YR | Evidence | Rust implication |
|---|---|---|---|
| Offline Skirmish launcher -> dialog `0x102` | Yes | `FUN_006AE2C0`; `0x006AE40A` common proc | This is the standard path to target |
| Common shell `WM_PAINT` -> parent cache -> display blit | Yes | `0x00622C4F`, `0x006223B3` | Render common chrome before preview |
| Right-panel/lower strip before parent background | Yes | `0x00621FFE` before `0x0062211B` | Preserve current Rust order |
| Background width selection | Yes / conditional | `0x0072E7AD`, `0x0072E815`, `0x004AED84` | 640/800 only; none above 800 |
| Preview/start overlay after common paint | Yes / conditional | `0x006AE47B`, `0x006409D2`, `0x00640A15` | Preview and markers are a later layer |
| Status/help child `0x695` | Yes | `0x00622CCB`, `0x00622E83`, `FUN_0060B550` | Missing Rust surface |
| Flag static update/render path | Yes | `FUN_004E3560`, `OwnerDraw_Static_006153E0` | Current Rust mostly caught up |

## 6. Current Rust Implementation Status

Scanned surfaces:

- `src/app_skirmish_shell_render.rs`
- `src/render/skirmish_shell_chrome.rs`
- `src/ui/skirmish_shell/layout.rs`
- `src/app.rs`

Current Rust now matches these broad composition points:

- `skirmish_shell_semantic_draw_order` and `build_skirmish_shell_instances` place right-panel stack and lower strip before parent background.
- `parent_background_role` uses only width 640/800 and returns `None` above 800.
- `render_skirmish_shell_with_atlas` draws shell atlas instances before preview texture, marker sprites, marker labels, and text.
- `push_flag_entry_native_clipped_centered` implements native flag placement/clipping; item-data to PCX mapping is present.
- `build_start_marker_instances` / `build_start_marker_label_instances` are not preview-rect-scissored.

Current Rust still misses or leaves risky:

- No layout/render/state for status/help strip `0x695`.
- No hover/status resolver using `0x4E8 -> 0x4E9 -> STT:*` order.
- No right-panel static reveal animation state for transition-triggered `0x4EC -> 0x4EE`.
- Player-name `0x6A0` is still not a faithful edit control surface in this broad first-paint composition.
- No aggregate retail screenshot validation for 640/800/1024 first-frame pixels.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard offline `0x102` reachability | verified | `FUN_006AE2C0`, `0x006AE40A` | none |
| Common parent `WM_PAINT` order | verified | `0x00622C4F`, `0x00621FFE`, `0x0062211B`, `0x006223B3` | runtime screenshot optional |
| Right-panel internal order | verified | `RightPanel__Draw @ 0x0072E450`; draw sequence | exact per-pixel screenshot validation |
| `SDBTNANM` frame-10 first-paint gate | verified for standard no-overlay | `RightPanel__Draw` param branch; current Rust returns false | transition state beyond first paint |
| Lower strip width selection | verified | `RightPanel__Draw` width branch | none for broad handoff |
| Parent background 640/800/>800 | verified | `Background_Overlay`, `FUN_0072CF40`, `CC_Draw_Shape` | abnormal stale pointer needs runtime watchpoint only |
| Static text/reveal boundary | verified | `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0` | Rust reveal implementation later |
| Flag statics | verified | `FUN_004E3560`, `OwnerDraw_Static_006153E0`, `rulesmd.ini:959..971` | lower PCX decoder internals out of this scope |
| Preview surface order | verified | `FUN_006AE3F0`, `DrawStartPositions` | live overlay gate needs regression tests |
| `STARTBUT.SHP` marker/label layering | verified | `0x006409D2`, `0x00640A15` | full `FUN_004A61C0` glyph/color contract deferred |
| Status strip `0x695` | verified | `0x00622CCB`, `0x00622E83`, `FUN_0060B550` | Rust implementation missing |
| Invalidation globals `DAT_00AC1CC8..DD0` | verified-by-prior for non-general-dirty role | invalidation report | runtime activation of overlay writer deferred |
| Recently settled button/checkbox/dropdown details | deferred | parent scope instruction | separate reports own those surfaces |
| Aggregate first-frame screenshot pixels | deferred | none static | retail runtime capture at 640/800/1024 |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this a coverage-map or exhaustive slice? -> coverage-map, because the target spans multiple shell surfaces and uses prior focused reports for sub-surface details.` (evidence: user target and report scope)
- `[RESOLVED] OQ-02 - Does standard offline YR Skirmish reach dialog 0x102? -> Yes, through `FUN_006AE2C0` and proc `FUN_006AE3F0`.` (evidence: `FUN_006AE2C0`; `0x006AE40A`)
- `[RESOLVED] OQ-03 - Does common parent paint run before preview? -> Yes, `FUN_006AE3F0` calls `FUN_00622B50` before its `WM_PAINT` preview branch.` (evidence: `0x006AE40A`, `0x006AE47B`)
- `[RESOLVED] OQ-04 - What is the parent chrome/background order? -> Right-panel/lower strip before background overlay, then cached parent blit.` (evidence: `0x00621FFE`, `0x0062211B`, `0x006223B3`)
- `[RESOLVED] OQ-05 - Does current Rust still draw parent background first? -> No; current source places right panel and lower strip before parent background.` (evidence: `src/app_skirmish_shell_render.rs` source scan)
- `[RESOLVED] OQ-06 - Which 640 parent background should be used? -> `MNSCRNS.SHP`, not stale `MNSCRNL.SHP` wording.` (evidence: `DAT_00B0FB50` mapping from common-parent report, current atlas field `background_640_mnscrns`)
- `[RESOLVED] OQ-07 - What happens at fresh >800 width? -> Parent background draw is a no-op because `+0xE4` is null and `CC_Draw_Shape` returns.` (evidence: `0x0072CF49..0x0072CF65`, `0x004AED84..0x004AED8E`)
- `[RESOLVED] OQ-08 - Are flag statics scaled? -> No, native-size centered/clipped image statics; current Rust now matches this broad rule.` (evidence: `OwnerDraw_Static_006153E0`; Rust `push_flag_entry_native_clipped_centered`)
- `[RESOLVED] OQ-09 - Does preview surface draw before or after common chrome? -> After common parent paint and blit.` (evidence: `0x006AE47B`)
- `[RESOLVED] OQ-10 - Are `STARTBUT.SHP` overlays clipped to the fitted preview rect? -> No, marker/label calls use destination-surface clip/bounds.` (evidence: `0x006409D2`, `0x00640A15`; current Rust test)
- `[RESOLVED] OQ-11 - Is status strip `0x695` present on first paint? -> Yes as a visible blank static, later updated on hover/status hit-test.` (evidence: `0x00622CCB`, `0x00622E83`, `FUN_0060B550`)
- `[RESOLVED] OQ-12 - Does Rust implement status strip `0x695`? -> No layout field, status state, or render path was found.` (evidence: `layout.rs`, `app_skirmish_shell_render.rs` scan)
- `[RESOLVED] OQ-13 - Are right-panel statics immediately reveal-animated on common first paint? -> No; kind-1 reveal starts through `0x4EE`, not ordinary first paint.` (evidence: `OwnerDraw_Static_006153E0`, `FUN_0060A5B0`)
- `[RESOLVED] OQ-14 - Does Rust implement reveal animation for `0x694/0x6EC/0x5A8`? -> No transition-triggered reveal state was found.` (evidence: `app_skirmish_shell_render.rs` scan)
- `[RESOLVED] OQ-15 - Are INI keys material to the composition order? -> No, except country item-data cross-check for flag mappings.` (evidence: Ghidra paths and `ini/rulesmd.ini:959..971`)
- `[RESOLVED] OQ-16 - Are the invalidation globals a required Rust dirty-rect model? -> No; prior report shows they are a conditional transient overlay state, not a general queue.` (evidence: `SKIRMISH_SHELL_INVALIDATION_GLOBALS_DAT_00AC1CC8_00AC1DD0_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-17 - Exact aggregate first-frame pixels at 640/800/1024.` (category: `needs-runtime-debugger`; reason: static Ghidra proves order and surfaces, but aggregate pixel parity needs retail screenshots; next-step-if-pursued: capture standard Skirmish first paint at 640x480, 800x600, and 1024x768)
- `[DEFERRED] OQ-18 - Full `FUN_004A61C0` numeric-label glyph/color contract.` (category: `out-of-scope`; reason: this broad slot only needs marker-label layer/offset/clip boundary; next-step-if-pursued: investigate the standalone helper)
- `[DEFERRED] OQ-19 - Abnormal stale `DAT_00B0FA18` process history above 800.` (category: `needs-runtime-debugger`; reason: normal lifecycle is verified; only non-normal skipped-cleanup histories require watchpointing; next-step-if-pursued: watch `DAT_00B0FA18` across shell transitions)
- `[DEFERRED] OQ-20 - Recently settled button/checkbox/dropdown details.` (category: `out-of-scope`; reason: parent swarm explicitly excluded these unless they affect first-frame composition; next-step-if-pursued: use the focused recheck reports)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| First-paint order is right panel, lower strip, parent background, parent blit, then Skirmish preview/start overlay | `0x00621FFE`, `0x0062211B`, `0x006223B3`, `0x006AE47B` | none observed; current Rust matches broad order | `src/app_skirmish_shell_render.rs` | Preserve current order and semantic tests | At 800x600 roles begin with right-panel/lower strip, include exact-800 parent background, then preview only when a real preview exists | Do not follow stale parent-first docs |
| Fresh `>800` parent background is no-op, while right panel/lower strip still draw | `0x0072CF49..0x0072CF65`, `0x0072E815`, `0x004AED84..0x004AED8E` | none observed | `parent_background_role`, `lower_strip_rect`, atlas selection | Keep `None` above 800 and keep large lower strip | 1024x768 order has `LowerSideLwscrnl` and no parent background role | Do not stretch/reuse `MnScrnLCoopGameSetup.shp` above 800 |
| Status/help child `0x695` is visible, blank by default, bottom-left anchored, and hover-updated via `0x4E8 -> 0x4E9 -> STT:* -> 0x4B2` | `0x00622CCB`, `0x00622E83`, `FUN_006040B0`, `FUN_0060B550` | missing | `src/ui/skirmish_shell/layout.rs`, state/input, `build_shell_text_draws` | Add status strip rect/state/render, default blank, hover text resolver | Fresh no-hover shell renders no status text; hovering Start shows localized `STT:SkirmishButtonStartGame`; 1024 rect is `(122,663,615,20)` | Do not hardcode "Status", map name, or visible GUI labels |
| Right-panel statics are kind-1 text statics; reveal starts only through transition/text-update messages, not ordinary common first paint | `FUN_00602490`, `FUN_0060A5B0`, `OwnerDraw_Static_006153E0` | reveal missing | static text render/state in `src/app_skirmish_shell_render.rs` | Add transition-triggered reveal state for `0x694/0x6EC/0x5A8` when implementing shell transitions | Common first paint shows text without starting reveal; transition-driven update reveals with count/range behavior | Do not start reveal from `WM_PAINT` alone or v-center map label |
| Flag statics use native PCX surfaces with magenta transparency, centered only if smaller and clipped if larger | `FUN_004E3560`, `OwnerDraw_Static_006153E0`, `rulesmd.ini:959..971` | none observed for broad rule | `push_flag_entry_native_clipped_centered`, atlas flag loading | Preserve native/clipped flag rendering and item-data mapping | Random player row uses `rani.pcx`; active Yuri row uses `yrii.pcx`; no scaling to fit the placeholder | Do not map by country string names or scale flags |
| Preview is parent-proc content; live `STARTBUT.SHP`/labels are after preview surface and clipped to destination surface | `0x006AE47B`, `0x006409D2`, `0x00640A15` | mostly fixed; overlay live-data gate remains risky | preview decode/cache and marker build in `src/app_skirmish_shell_render.rs` | Keep preview-after-chrome and destination clip; gate live overlays on real live start fields, not just baked PreviewPack pixels | Loose map with baked red PreviewPack markers but absent live header start fields does not draw `STARTBUT.SHP`; map with live fields does | Do not synthesize live overlays from baked preview pixels alone |
| Win32 invalidation/cache objects do not require a Rust global dirty-rect model for this first-paint composition | `WM_PAINT_Handler`; invalidation globals report | none observed | direct renderer and preview texture cache | Keep direct redraw; invalidate only real Rust GPU/cache resources | Toggling controls or changing map redraws deterministically without emulating `DAT_00AC1CC8` | Do not add fake shell-global dirty queues to `sim/` or UI |

### Negative Facts / Do Not Do

- Do not draw parent background before right-panel/lower strip. Active in YR: No for standard `0x102`; evidence `0x00621FFE` precedes `0x0062211B`.
- Do not draw a parent background above width 800 for a fresh Skirmish entry. Active in YR: No; evidence exact-800 loader `0x0072CF49` and null gate `0x004AED84`.
- Do not use stale `MNSCRNL.SHP` wording for the 640 parent background. Active in YR: No for parent `+0xE0`; evidence later mapping `DAT_00B0FB50 = MNSCRNS.SHP`.
- Do not render a permanent status/help label. Active in YR: No; evidence no-hover fallback sends empty string to `0x695`.
- Do not treat `0x468` as an ordinary static that draws its own preview. Active in YR: No; evidence parent `WM_PAINT` calls `DrawStartPositions`.
- Do not treat flag statics as fit-scaled images. Active in YR: No; evidence static image path uses native width/height and clips.
- Do not re-open settled button middle phase, checkbox/trackbar geometry/input, or ComboDropWin wheel behavior from this broad report; those are sibling findings.

### Stale Docs / Follow-up Docs

- `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md` still contains stale `MNSCRNL.SHP`-as-640-parent wording. Replacement: "For standard offline Skirmish `0x102`, parent `+0xE0` uses `DAT_00B0FB50`, now verified as `MNSCRNS.SHP`; `MNSCRNL.SHP` is a separate shell asset and not the `0x102` 640 parent background."
- `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md` has stale current-Rust status saying semantic draw order was parent-first. Replacement: "Current Rust emits right-panel stack and lower strip before parent background in both semantic draw order and atlas instance construction."
- `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md` current Rust status is stale where it says flags use `push_entry_fit` and scale. Replacement: "Current Rust now uses native centered/clipped flag rendering via `push_flag_entry_native_clipped_centered`; keep only screenshot validation as the broad remaining flag risk."
- `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md` current Rust delta for marker preview-rect scissor is stale. Replacement: "Current Rust has a destination-surface clipping regression test and marker/label instances are not scissored to the fitted preview rect; remaining risk is live-overlay eligibility versus baked PreviewPack pixels."

## Sources

- Ghidra read-only decompile: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_00622B50`, `WM_PAINT_Handler @ 0x00621E90`, `RightPanel__Draw @ 0x0072E450`, `Background_Overlay @ 0x0072E730`, `DrawStartPositions @ 0x00640710`, `FUN_006040B0`, `FUN_0060B550`, `OwnerDraw_Static_006153E0`, `FUN_004E3560`, `CC_Draw_Shape @ 0x004AED70`, `FUN_0072CF40`, `FUN_0072CF90`, `FUN_0060CF00`, `FUN_0060F9A0`, `FUN_0060A5B0`, `FUN_00602490`.
- Ghidra assembly contexts: `0x006AE40A`, `0x006AE47B`, `0x00622C4F`, `0x00621FFE`, `0x0062211B`, `0x006223B3`, `0x0072E7AD`, `0x0072E815`, `0x004AED84`, `0x00622CCB`, `0x00622E83`, `0x006409D2`, `0x00640A15`.
- Prior docs referenced: `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_HIGH_RES_RIGHT_PANEL_BACKGROUND_FALLBACK_GT800_GHIDRA_REPORT.md`, `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_INVALIDATION_GLOBALS_DAT_00AC1CC8_00AC1DD0_GHIDRA_REPORT.md`, `SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini:959..971`.
- Rust source scanned: `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app.rs`.
