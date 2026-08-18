# Loading First Renderer: Corrected Composition, Data Readiness, and Display Handoff

**Date:** 2026-07-27  
**Program:** active `gamemd.exe`  
**Primary owner:** `ScenarioClass__DrawLoadingScreen @ 0x00552D60` (`0x00552D60..0x005540F1`)  
**Mode:** exhaustive-slice, gap-only correction  
**Status:** COMPLETE for the bounded standard-offline-Skirmish composition/data/handoff slice; runtime repaint-branch selection remains UNVERIFIED.

## Target Question

For standard offline Skirmish, what does `0x00552D60` compose from entry to return, which scenario/player/start data already exists, which surface receives it, and when is that composition first confirmed to reach the displayed frame?

## Non-goals

- Do not decode the marker projection/formula inside `0x00640A40`.
- Do not decode the text-helper internals at `0x00553EC0..0x00554100`.
- Do not claim native/Rust pixel parity.
- Do not analyze campaign or random-map generation beyond branch boundaries needed to separate them from selected-map Skirmish.
- Do not modify Rust, INI, other research documents, or the audit log.

## Evidence Needed

- Whole-owner decompile plus assembly at every load-bearing call boundary.
- Caller ordering through scenario parsing, house creation, and start assignment.
- Surface identity and display-copy chain through the first advancing progress callback.
- Standard offline Skirmish activation evidence.
- Current Rust first-frame/load-phase ordering and data ownership.

## Stop Conditions

- Stop when entry-to-return top-level composition order and first confirmed display handoff are closed.
- Stop when waypoint, house, player-node, and assigned-start readiness are classified.
- Stop at call order/arguments for marker and text helpers; leave their internals to sibling reports.
- Stop with runtime HWND/direct-repaint selection explicitly UNVERIFIED.

## Verified Standard-YR Activation

- **Active in YR: Yes.** Standard offline Skirmish is `g_GameMode == 5`; `Main_Game` routes its successful shell return into scenario startup, and `ScenarioClass__Full_Init @ 0x00686B20` is the sole caller of `0x00552D60`. Evidence: `decompile_function(0x0052D9A0, program=gamemd.exe)` as recorded in `SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md`; `get_function_call_graph(0x00552D60, depth=2, direction=both, program=gamemd.exe)`.
- **Active in YR: Yes.** `g_GameMode != 0` selects the non-campaign body at `0x00552E65`; mode 5 therefore reaches the country-loading composition path. Evidence: `disassemble_bytes(0x00552D60..0x00552E85, program=gamemd.exe)` and final `batch_decompile(0x00552D60, program=gamemd.exe)`.
- **Active in YR: Yes.** The selected-map path has `ScenarioClass+0x34BD == 0`; it computes the scenario preview/digest rather than loading `RandMap.img`, and raw progress `3` remains `3` rather than being integer-halved to `1`. Evidence: `disassemble_bytes(0x00553300..0x005536B0, program=gamemd.exe)`; `batch_decompile(0x0069AE90, program=gamemd.exe)`.

## Entry-to-Return Composition

1. **Active in YR: Yes.** Entry allocates a full-screen 16-bit `BSurface`, stores it at loading manager `+0x60`, and clears/fills it through vtable slot `+0x10`. Allocation failure stores null and returns. Evidence: `disassemble_bytes(0x00552D60..0x00552E85, program=gamemd.exe)`.
2. **Active in YR: Yes.** The stock non-campaign branch reads the first network/player node, resolves its country/index, formats `ls640<country>.shp` or `ls800<country>.shp`, accesses the previously built loading ConvertClass, and loads the LS SHP into manager `+0x48`. Evidence: `disassemble_bytes(0x00553300..0x005536B0, program=gamemd.exe)`.
3. **Active in YR: Yes.** For an ordinary selected map, the temporary preview holder is initialized and populated by `MPGameOptions__ComputeScenarioDigest`; random-map loading is a separate `RandMap.img` branch. Evidence: callsites `0x00553515..0x005535A7` in `search_instructions(function=0x00552D60, mnemonic=CALL, program=gamemd.exe)` and owner decompile.
4. **Active in YR: Conditional.** If both the loading ConvertClass and LS SHP exist, `CC_Draw_Shape @ 0x005535FE` draws frame 0 into manager `+0x60` at the manager origin. Evidence: `disassemble_bytes(0x00553300..0x005536B0, program=gamemd.exe)`.
5. **Active in YR: Yes; marker pixels conditional.** After the LS background attempt, `0x00553687` calls `0x00640A40` with the preview holder in `ECX` and manager `+0x60` as the pushed destination surface. The helper can no-op when its holder is null. Evidence: assembly `0x0055367B..0x0055368C`; `disassemble_bytes(0x00640A40..0x00640A95, program=gamemd.exe)`.
6. **Active in YR: Yes.** Country/localized text selection and drawing follow the marker helper; all text-helper calls through `0x005540A8` target the same composed loading surface before cleanup and return. Evidence: `disassemble_bytes(0x00553680..0x00553A80, program=gamemd.exe)` plus full owner callsite enumeration.

**Verified top-level order:** clear offscreen surface -> LS country background -> selected-map preview/assigned-player markers -> localized text/chrome -> return. No progress bar is drawn inside `0x00552D60`.

## Palette/ConvertClass Correction

- **Active in YR: Yes.** `0x00552CC0` is earlier setup: it ensures loading MIX availability, gets the country, then calls `0x0072B530` to build/cache `DAT_00B0FB98`. Later, `0x00552D60` independently calls `0x0072B500`, which only returns that cached ConvertClass. Evidence: `batch_decompile(0x00552CC0,0x0072B500,0x0072B530, program=gamemd.exe)`.
- **Active in YR: Yes.** The stock country table selects the `MPLS*.PAL` family: `MPYLS.PAL`, `MPLSC.PAL`, `MPLSUK.PAL`, `MPLSR.PAL`, `MPLSL.PAL`, `MPLSK.PAL`, `MPLSI.PAL`, `MPLSG.PAL`, `MPLSF.PAL`, `MPLSU.PAL`. Evidence: `disassemble_bytes(0x0072B650..0x0072B820, program=gamemd.exe)` and `read_memory(0x00844C0C/0x00845370, program=gamemd.exe)`.
- Therefore, the old wording "`0x00552CC0 -> 0x0072B530 -> 0x0072B500`" is not a direct call chain. The correct cross-phase sequence is setup/build (`0x00552CC0 -> 0x0072B530`) followed later by composition/access (`0x00552D60 -> 0x0072B500`).

## Data Readiness at `0x00552D60`

- **Active in YR: Yes.** `[Waypoints]` parsing is complete before the renderer. `Full_Init` calls `0x0068BDC0` at `0x006873DB`; that function reads 702 waypoint keys into `ScenarioClass+0x632` and marks valid waypoint cells. Evidence: `batch_decompile(0x0068BDC0, program=gamemd.exe)`; `disassemble_bytes(0x006873B0..0x006875A0, program=gamemd.exe)`.
- **Active in YR: Yes.** Player-node records exist before composition: `ScenarioClass__Create_Houses @ 0x00687F10` consumes `DAT_00A8DA78`, constructs human/AI houses, assigns color schemes, teams, and local-player ownership before the renderer. Evidence: `batch_decompile(0x00687F10, program=gamemd.exe)`; call at `0x0068745E`.
- **Active in YR: Yes.** Start assignment dispatch occurs before the renderer: selected-mode vtable `+0x80`, then either `ScenarioClass__AssignStartingPoints @ 0x005EE9D0` when `DAT_00A8B244 == 2` or selected-mode vtable `+0x84` otherwise; only then are manager construction and `0x00552D60` called. Evidence: assembly `0x00687558..0x00687588`.
- **Active in YR: Yes for the explicit assignment branch.** `0x005EE9D0` first gathers start positions and then fills the scenario assignment table from active houses; `0x00640A40` later reads that assignment table and house color state. Evidence: `batch_decompile(0x005EE9D0, program=gamemd.exe)` and bounded top-level decompile of `0x00640A40`.
- The exact selected-mode `+0x84` implementation was not decoded here. Its dispatch position is verified; equivalence of its assigned-start outputs is **UNCHECKED**.

## First Confirmed Display Handoff

- **Active in YR: Yes.** `0x00552D60` only composes manager `+0x60`; it does not reference `DAT_0088730C`, `DAT_00887308`, `DAT_00887310`, or `0x004F4780`. Evidence: `audit_globals_in_function(0x00552D60, program=gamemd.exe)`.
- **Active in YR: Yes.** Immediately after the compositor returns, `Full_Init` calls `FUN_0069AE90(3)` with no intervening render/present call. Evidence: assembly `0x00687581..0x00687594`.
- **Active in YR: Yes.** On the advancing callback, `ProgressMeterClass__SetPercent` invokes manager message `0x11AE` before repaint. Manager vtable slot 0 (`0x00554400`) copies manager `+0x60` into `DAT_0088730C` via destination vtable slot `+4` with arguments `(source,0,1)`. Evidence: `disassemble_bytes(0x00643CF0..0x00643D55, program=gamemd.exe)`; `disassemble_bytes(0x005543F0..0x00554435, program=gamemd.exe)`; final `batch_decompile(0x00554400, program=gamemd.exe)`.
- **Active in YR: Yes.** Repaint overlays progress onto `DAT_0088730C`, then `0x004F4780(1,DAT_0088730C,NULL)` synchronously blits that surface through the display chain to `DAT_00887308`. This is a synchronous hidden-to-primary-style blit, not evidence of native `Present` or `Flip`. Evidence: `disassemble_bytes(0x00643B80..0x00643C50, program=gamemd.exe)` and `decompile_function(0x004F4780, program=gamemd.exe)`.
- **Corrected conclusion:** the first confirmed displayed selected-map frame contains the completed background/preview-marker/text composition plus the `3%` progress repaint. No pre-3 visible frame is verified.
- Manager-copy ordering is common to both repaint branches. Whether standard offline Skirmish always has null progress HWND `+0x64` and takes the direct redraw branch is **UNVERIFIED**; a non-null HWND instead receives synchronous `WM_PAINT`.

## Current Rust Disparity

- Rust `begin_loading` constructs `LoadingSession` with `LoadingJobPhase::InitialMapSelection`, then `render_loading_screen` advances raw `3` and submits the first native frame before `pump_loading_after_present` begins `load_map_initial_with_assets`. Evidence: `src/app_loading.rs:318-417,461-509,769-853`; `src/app.rs:4464-4481,4716-4724`.
- `SkirmishLaunchSession` carries country/color and start-position indices, but no parsed waypoint coordinates or finalized start-slot-to-house assignment table; `LoadingRequest` carries only startup, presentation, and fallback settings. Evidence: `src/skirmish_launch.rs:251-288`; `src/app_loading.rs:188-255`.
- `build_native_loading_instances` currently emits background plus progress backing/bar/side icon only; its own module comment records marker/text layers as blocked. Evidence: `src/app_loading.rs:949-1016`; `src/render/loading_screen_chrome.rs:1-5`.
- Thus Rust now matches the corrected first displayed `3%` timing, but its first frame is earlier than native scenario-data readiness and omits the native selected-map preview/assigned markers/text. Truth verdict: **DRIFT**, priority to be decided separately.

## Implementation Handoff

1. Add a pre-first-frame selected-map composition payload produced after initial map parsing, containing preview pixels/handle, parsed start coordinates, and finalized slot-to-house/color assignments. Test: `native_loading_first_frame_waits_for_selected_map_preview_data`.
2. Compose native layers in verified order before the first submitted 3% frame: LS background -> preview/assigned markers -> localized text -> progress chrome. Test: `native_loading_first_display_orders_preview_markers_text_before_progress_3`.
3. Keep the existing selected-map raw-3 display contract and random-map integer-halving gate separate. Test: `selected_map_first_display_is_3_random_map_first_display_is_1`.

## Negative Facts / Do Not Do

- Do not claim `0x00552D60` itself presents or flips; it composes offscreen.
- Do not expose a separate visible 0%/pre-3 selected-map frame without new runtime evidence.
- Do not generate marker coordinates before the selected scenario waypoints and slot assignments exist.
- Do not replace the `MPLS*.PAL` family with DIALOG palettes or describe `0x0072B500` as part of the earlier builder call chain.
- Do not implement the `0x00640A40` projection formula or text-helper internals from this report.

## Remaining Uncertainty

- Exact live selection of HWND `WM_PAINT` versus direct redraw for every standard offline configuration is UNVERIFIED.
- Exact selected-mode vtable `+0x84` assignment behavior is UNCHECKED.
- Marker pixel projection, text content/layout, and exact native/Rust pixel parity remain owned by sibling work or UNCHECKED.

## Open Questions Log / Adversarial Pass

- `[RESOLVED]` What if the offscreen allocation fails? Manager `+0x60` is null and the compositor returns; the manager callback skips its copy.
- `[RESOLVED]` What if LS art or ConvertClass is missing? The guarded background draw is skipped; later composition continues.
- `[RESOLVED]` What if the preview holder is null? `0x00640A40` returns without markers; text still follows.
- `[RESOLVED]` What if this is a random map? It loads `RandMap.img` and halves raw `3` to `1`; this is outside selected-map conclusions.
- `[DEFERRED]` What if progress HWND `+0x64` is non-null? The manager copy still precedes repaint, but exact window liveness needs runtime capture.
- `[RESOLVED]` Could a display blit occur inside `0x00552D60`? No display globals or `0x004F4780` are referenced.
- `[RESOLVED]` Are waypoint coordinates available before composition? Yes, parser call `0x006873DB` precedes house/start setup and renderer call `0x00687588`.
- Zero-add pass: final fresh decompiles of `0x00552D60`, `0x00554400`, and `0x0069AE90` added no new in-scope questions and confirmed composition, manager-copy, and raw-3/halving facts.

## Exact Stale-Doc Replacement

In `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`, replace the palette-chain claim, pre-progress visibility wording, remaining-uncertainty blit inference, and replacement paragraph with:

> For standard offline Skirmish, early `ScenarioClass__Full_Init` parses `[Waypoints]`, creates houses, and dispatches start assignment before calling `0x00552D60`. Earlier loading setup builds/caches the country `MPLS*.PAL` ConvertClass through `0x00552CC0 -> 0x0072B530`; the later compositor independently reads it through `0x00552D60 -> 0x0072B500`. `0x00552D60` composes manager `+0x60` offscreen in the order LS country background, selected-map preview/assigned-player marker helper, then localized text/chrome. It returns without presenting. The immediately following advancing raw-3 callback copies that completed surface into `DAT_0088730C`, repaints the progress bar, and synchronously blits through `0x004F4780`; therefore the first confirmed displayed selected-map frame is the completed composition plus 3%, not a verified pre-3 frame. This is a synchronous hidden-to-primary-style blit, not a native Present/Flip claim.
