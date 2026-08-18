# Random Map Generate Preview Loop - Ghidra Research Report

**Address(es):** `RandomMapGenerator__Generate @ 0x00598960`, `GenerateTerrainPreview @ 0x00641140` (call-site only, internals out of scope), `RandomMapSetupDialog__Proc @ 0x00596300`, `ScenarioClass__Read_Scenario @ 0x00684620`, unresolved caller at `0x00596182`
**Investigation Mode:** exhaustive-slice (single function, call-site + repaint focus)
**Claimed Scope:** the preview-flag argument's effect inside `0x00598960` - where/how often it triggers `GenerateTerrainPreview`, and dialog repaint/message-pump interaction around those points.
**Non-Scope:** the terrain generation algorithm itself (blobs, water, cliffs, tiberium placement, region partitioning) and `GenerateTerrainPreview`'s internal pixel/dimension formulas - both separately researched.
**Confidence:** High for call count/location, repaint mechanism, caller argument values, and 0x639 negative. Medium for the exact identity/purpose of the `g_ScenarioClass_Instance+0x3598` progress-display gate (functionally proven, semantic name not recovered) and for the unresolved 5th caller at `0x00596182` (no function boundary exists to name it).
**Active in YR:** Conditional. Live whenever the player opens the random-map setup dialog and clicks Generate (`0x620`) or OK (`0x6C5`) with no existing preview; also runs with the preview flag forced to `0` at ordinary scenario launch and in the map-editor's OK path.

## 0. Working Notes Gate

**Target question:** How many times, and at which stage boundaries, does `0x00598960` call `GenerateTerrainPreview` when its preview-flag argument is nonzero, does the dialog repaint between those calls, and what does the player actually observe during the block?

**Non-goals:** terrain generation algorithm internals; `GenerateTerrainPreview`'s own pixel/surface formulas; full re-audit of `RandomMapSetupDialog__Proc`'s unrelated commands (only the parts that bear on Generate's preview/repaint behavior).

**Evidence needed to mark COMPLETE:** exact call-site addresses inside `0x00598960` cited via `get_function_callees`/`disassemble_function`/`get_xrefs_to`; the repaint API and its call-site addresses; confirmation of caller argument values (preview-flag, HWND) for every caller of `0x00598960`; confirmation of whether/where control `0x639` is referenced.

**Stop conditions:** met once call-site count/order, repaint mechanism, and all caller argument values were read directly from disassembly/decompile. Reached below.

## 1. Overview

`RandomMapGenerator__Generate @ 0x00598960` is a `__thiscall(MapSeedClass* this, BOOL bPreview, HWND hDlg)` function (stack cleanup `RET 0x8` confirms two stack arguments). When `bPreview != 0` it calls `GenerateTerrainPreview` at exactly **8** fixed points interleaved with its generation phases, and after each call it issues `SendMessageA(hDlg, WM_PAINT(0xF), 0, 0)` - a synchronous, same-thread dispatch that re-enters the dialog's own `WM_PAINT` handler immediately, before `RandomMapGenerator__Generate` resumes the next phase. The player-visible effect is a genuine progressive fill: the preview control repaints from inside the blocking call, 8 times, before the caller's own final `GenerateTerrainPreview()` + repaint after `0x00598960` returns.

Evidence: `decompile_function 0x00598960`; `disassemble_function 0x00598960`; `get_function_callees 0x00598960`; `get_xrefs_to 0x00641140`.

## 2. Key Offsets And Globals

| Item | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `RandomMapGenerator__Generate @ 0x00598960` | RMG spine; preview-flag-gated repeated `GenerateTerrainPreview` + `WM_PAINT` | `decompile_function 0x00598960` | Conditional |
| `[ESP+0x42c]` at function entry (decompiled as `param_2`) | the preview-flag `BOOL` argument; gates all 8 `GenerateTerrainPreview` calls via `(char)param_2 != '\0'` | `disassemble_function 0x00598960` stack-offset trace; `decompile_function 0x00598960` | Conditional |
| `[ESP+0x430]` at function entry (decompiled as `param_3`, typed `HWND`) | the dialog HWND; used for `SendMessageA(WM_PAINT)` x8, `GetDlgItem(0x638)` x2, and one progress-init helper call | `disassemble_function 0x00598960`; `decompile_function 0x00598960` | Conditional |
| `g_ScenarioClass_Instance + 0x3598` | single byte, read once at entry into `bVar14`; gates `0x638` static show/hide and numeric-vs-text progress display - **independent of** `bPreview` | `decompile_function 0x00598960` | Conditional |
| Control `0x638` ("Working/Please Wait" static) | shown via `GetDlgItem(hDlg,0x638)+ShowWindow(...,SW_SHOW)` at function entry, hidden via `ShowWindow(...,SW_HIDE)` at function exit - both gated on `bVar14`, not on `bPreview` | `decompile_function 0x00598960` | Conditional |
| Control `0x639` (hidden progress button) | **never referenced** anywhere in `0x00598960` | `decompile_function 0x00598960`; `disassemble_function 0x00598960` (no `0x639` literal in the listing) | No |
| `0x005e7eb0` (Ghidra label `Pipe__Constructor` - **mislabeled**) | a network multiplayer map-preview upload/broadcast routine (`"Starting map preview upload"`, `"Preview.bin"`, `SessionClass__SendFileToClients`), gated on `g_GameMode==4 && DAT_00a8b244==3` and `1 < DAT_00a8da84` (player count); called once after each of the 8 preview blocks | `decompile_function 0x005e7eb0` | Conditional: multiplayer lobby only |

## 3. Core Logic

### 3.1 Exactly 8 `GenerateTerrainPreview` call sites inside `0x00598960`, each gated only on `bPreview`

Active in YR: Conditional on `bPreview != 0`. `get_function_callees 0x00598960` lists `GenerateTerrainPreview @ 00641140` as a callee; `get_xrefs_to 0x00641140` resolves the call sites to these 8 addresses, all inside `RandomMapGenerator__Generate`:

```text
0x00598aa8
0x00598b6a
0x00598bf0
0x00598dd9
0x0059904b
0x005990f0
0x005991db
0x0059930d
```

`disassemble_function 0x00598960` confirms each call is preceded by `CMP byte ptr [ESP+0x42c],BL` (or the equivalent `MOV AL,[ESP+0x42c]; CMP AL,BL` for the first one) and a `JZ` that skips the block when the byte is zero; `decompile_function 0x00598960` renders all 8 as `if ((char)param_2 != '\0') { GenerateTerrainPreview(); SendMessageA(param_3,0xf,0,0); ... }`. No other gating condition (map type, theater, debug flag) affects whether these calls fire - only the `bPreview` argument.

Evidence: `get_function_callees 0x00598960`; `get_xrefs_to 0x00641140`; `disassemble_function 0x00598960`; `decompile_function 0x00598960`.

Stage boundaries (labeled only by the `Register_heap_pool` phase strings already visible in the decompile, not by re-deriving what each phase computes):

| # | Address | Immediately preceded by (last phase label before this call) |
|---|---|---|
| 1 | `0x00598aa8` | function entry / "Init random map" (before any water/region work) |
| 2 | `0x00598b6a` | "Seeding water" |
| 3 | `0x00598bf0` | (directly after call 2; no new phase label between them, only a progress-marker update) |
| 4 | `0x00598dd9` | "Init regions" -> "Making regions" |
| 5 | `0x0059904b` | "Recalculating cell attributes" (2nd pass, after starting points / tech buildings / tiberium) |
| 6 | `0x005990f0` | (directly after call 5; no new phase label between them) |
| 7 | `0x005991db` | "Recalculating cell attributes" (3rd pass) -> "Creating hills" |
| 8 | `0x0059930d` | "Creating LATs/rocks/etc" |

After call 8 the function still runs a 4th cell-attribute recalc, tiberium growth/spread queue init, cleanup, and radar-bounds/surface rebuild, but issues **no further** `GenerateTerrainPreview` call before returning - the function's own last refresh is call 8.

Evidence: `decompile_function 0x00598960` (phase ordering from `Register_heap_pool` string arguments, read only for sequencing, not for algorithm content).

### 3.2 Repaint mechanism: synchronous `SendMessageA(hDlg, WM_PAINT, 0, 0)`, not Post/Invalidate/Update/RedrawWindow

Active in YR: Conditional (fires only alongside the 8 gated calls above). Every one of the 8 blocks is immediately followed by `SendMessageA(param_3,0xf,0,0)` (`0xF` = `WM_PAINT`). `get_function_callees 0x00598960` lists `SendMessageA @ EXTERNAL:000000e7` as the only message-dispatch import used in this function; there is no `InvalidateRect`, `UpdateWindow`, `RedrawWindow`, or `PostMessageA` callee anywhere in `0x00598960`.

Because `SendMessageA` to a window owned by the calling thread dispatches synchronously (it calls the window procedure directly, it does not queue), each of these 8 calls re-enters `RandomMapSetupDialog__Proc`'s own `WM_PAINT` branch **before** `RandomMapGenerator__Generate` resumes its next phase. That branch (already documented in `SKIRMISH_RANDOM_MAP_SETUP_DIALOG_CONTROLS_OPTIONS_GHIDRA_REPORT.md` and reconfirmed here via `decompile_function 0x00596300`) checks `DAT_00abe154 != 0`, fetches child `0x468`, and calls `DrawStartPositions` to blit the just-regenerated preview surface into the visible control - i.e. each of the 8 stage boundaries produces one real, synchronous, on-screen repaint of the preview box while the click handler is still blocked inside `RandomMapGenerator__Generate`.

Evidence: `get_function_callees 0x00598960`; `disassemble_function 0x00598960`; `decompile_function 0x00598960`; `decompile_function 0x00596300` (`WM_PAINT` branch: `if (param_2==0xf) { if (DAT_00abe154!=0){ GetDlgItem(param_1,0x468); ...; DrawStartPositions(param_1); } ...}`).

Implementation consequence: **the preview box fills in progressively, stage-by-stage, 8 times during a single Generate click** - it is not blank-until-the-end.

### 3.3 The `0x638` "Working/Please Wait" static is shown/hidden by `0x00598960` itself, gated by a flag independent of `bPreview`; `0x639` is never touched

Active in YR: Conditional on `g_ScenarioClass_Instance+0x3598 == 0` (captured once at entry as `bVar14`), **not** on the preview-flag argument. At entry, if `bVar14`, the function calls `GetDlgItem(hDlg,0x638)` then `ShowWindow(...,SW_SHOW)`; at exit, if `bVar14`, it calls `GetDlgItem(hDlg,0x638)` then `ShowWindow(...,SW_HIDE)`, resets the progress display to `0`, and calls a cleanup helper (`FUN_00643e70`). This is in addition to, not instead of, the `WndProc`'s own `ShowWindow(0x638,SW_SHOW)` before it calls `RandomMapGenerator__Generate(1,hDlg)` (documented in `0x00596300`'s plate comment) - the two calls are redundant on entry, but `0x00598960` is the one that hides `0x638` again, and it does so **before** the caller's final direct `GenerateTerrainPreview()` call and control re-enable, so the "please wait" text is already gone by the time the caller finishes.

Control `0x639` (the hidden progress button, style `0x40000007`, no `WS_VISIBLE`, documented separately) is never referenced anywhere in `0x00598960` - no `PUSH 0x639` literal exists in the disassembly.

At each of roughly 13-15 phase boundaries (a strict superset of the 8 preview-refresh points), the same `bVar14`-derived check chooses between a numeric progress-percent update (`FUN_00643c50` on a global object at `0xac4f58`) and a text/log print (`FUN_0069ae90` with a phase-specific string id) - this is a status-display mechanism layered on top of, and orthogonal to, the preview-refresh gating; it never itself calls `GenerateTerrainPreview` or touches `0x639`.

Evidence: `decompile_function 0x00598960`; `disassemble_function 0x00598960` (no `0x639` literal); cross-reference `decompile_function 0x00596300` for the caller-side `ShowWindow(0x638,SW_SHOW)`.

### 3.4 The HWND argument is used for exactly three purposes, all UI/status related

Active in YR: Conditional. `param_3` (`hDlg`) is used for: (1) `GetDlgItem(hDlg,0x638)` x2 (show at entry, hide at exit, both gated on `bVar14`); (2) `SendMessageA(hDlg,WM_PAINT,0,0)` x8 (one per preview-flag-gated block); (3) one call `FUN_00642a60(0, 0x40590000, 1, hDlg)` at entry (gated on `bVar14`) that appears to initialize the same progress-display object later driven by `FUN_00643c50`/`Register_heap_pool`. No other use of the HWND exists in this function - it is never used for a distinct child-control lookup, subclassing, or any non-UI purpose.

Evidence: `decompile_function 0x00598960`; `disassemble_function 0x00598960`.

### 3.5 Caller argument values: preview flag is `1` from three confirmed sites and one unresolved site, and `0` from two confirmed sites

Active in YR: Conditional per caller. `get_function_callers 0x00598960` and `get_xrefs_to 0x00598960` together resolve 5 call sites:

| Call site | Enclosing function | `bPreview` | `hDlg` | Evidence |
|---|---|---|---|---|
| `0x0059664c` | `RandomMapSetupDialog__Proc`, command `0x620` (Generate) | `1` | dialog HWND | `get_assembly_context 0x0059664c`: `PUSH 0x1; PUSH EBP; ... CALL 0x00598960` |
| `0x00596a49` | `RandomMapSetupDialog__Proc`, command `0x6C5` (OK), **map-editor branch** (`g_IsMapEditor != 0`) | `0` | dialog HWND | `decompile_function 0x00596300`: `else { RandomMapGenerator__Generate(0,param_1); }`; `get_assembly_context 0x00596a49`: `PUSH 0x1(hDlg-order)/PUSH 0x0` gated behind `TEST AL,AL; JZ` on `g_IsMapEditor` |
| `0x00596a66` | `RandomMapSetupDialog__Proc`, command `0x6C5` (OK), **normal/non-editor branch**, only when no preview already exists | `1` | dialog HWND | `decompile_function 0x00596300`: `if (g_IsMapEditor=='\0') { if (( ...preview exists...) || (RandomMapGenerator__Generate(1,param_1), g_IsMapEditor=='\0')) goto LAB_00596a90; }` |
| `0x00684989` | `ScenarioClass__Read_Scenario` (ordinary scenario/map load, no dialog) | `0` | `NULL` (`0`) | `get_assembly_context 0x00684989`: `PUSH 0x0; PUSH 0x0; ... CALL 0x00598960` |
| `0x00596182` | **unresolved** - `get_function_by_address 0x00596182` returns no function; address falls just before `RandomMapSetupDialog__Proc @ 0x00596300` in memory | `1` | HWND (`EBP`) | `get_assembly_context 0x00596182`: `PUSH 0x1; PUSH EBP; ... CALL 0x00598960`, followed by a direct `GenerateTerrainPreview` call and the same `g_GameMode==4 && DAT_00a8b244==3` gate seen after the WndProc's `0x620` handler |

This directly confirms the assignment's premise: **the OK `0x6C5` path calls `RandomMapGenerator__Generate(1, hDlg)` in normal skirmish play** (only when no usable preview already exists), and **the map-editor branch of the same `0x6C5` handler calls it with `0`** - the difference is gated by `g_IsMapEditor`, read via `decompile_function 0x00596300`, not guessed.

Evidence: `get_function_callers 0x00598960`; `get_xrefs_to 0x00598960`; `get_assembly_context` for all 5 addresses; `decompile_function 0x00596300`; `get_function_by_address 0x00596182`.

## 4. INI Keys

None. This slice reads dialog/global state and function arguments only; no INI key is consulted by `0x00598960`'s preview/repaint gating.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `0x620` Generate click | `RandomMapSetupDialog__Proc` calls `g_DisplayChain+0xC` once, disables all 13 controls, shows `0x638`, calls `RandomMapGenerator__Generate(1,hDlg)` (8 internal progressive repaints), then calls `GenerateTerrainPreview()` once more directly, re-enables controls, calls `g_DisplayChain+0x10` once, then `PostMessageA(hDlg,WM_PAINT,0,0)` (async, queued) | `decompile_function 0x00596300` | Conditional |
| `0x6C5` OK click, no preview yet | Same `RandomMapGenerator__Generate(1,hDlg)` path runs inline before accepting the dialog result | `decompile_function 0x00596300` | Conditional |
| `0x6C5` OK click, map editor | `RandomMapGenerator__Generate(0,hDlg)` - no preview refresh at all - then unconditionally saves the map file | `decompile_function 0x00596300` | No (map editor only) |
| Scenario/map load | `ScenarioClass__Read_Scenario` calls `RandomMapGenerator__Generate(0, NULL)` - no dialog, no preview, no repaint | `get_assembly_context 0x00684989` | Yes (random-map scenario launch) |
| Unresolved caller `0x00596182` | Same `Generate(1,hDlg)` + direct `GenerateTerrainPreview()` pattern as `0x620`, no function boundary recovered | `get_assembly_context 0x00596182` | Conditional (unidentified trigger) |
| Outer display-chain pause/resume | `g_DisplayChain+0xC`/`+0x10` bracket the **entire** `0x620` handler once each - they are not re-triggered per internal preview refresh and do not appear anywhere inside `0x00598960` itself | `decompile_function 0x00596300`; absence confirmed in `get_function_callees 0x00598960` | Conditional |

## 6. Current Rust Implementation Status

| Rust area | Status vs binary | Evidence |
|---|---|---|
| `Control::Generate0x620` handler | renders the preview **exactly once**, after full generation completes | `src/app.rs` `handle_random_map_setup_mouse_up`, `Self::render_random_map_setup_preview(state, &options)` |
| Progressive/incremental preview during generation | missing - no equivalent of the 8 internal `SendMessageA(WM_PAINT)` refreshes | no per-stage preview call found in `src/map/rmg/build.rs` / `src/map/rmg/preview.rs` |
| `generating` flag / control-disable state | present, matches the "disable all controls during Generate" behavior at a coarse level | `src/ui/skirmish_shell/state/random_map_setup.rs` `begin_generate()`/`finish_generate()`, field `generating` |
| `0x638`/"please wait" static equivalent | not verified in this slice (out of scope: dialog chrome rendering) | not scanned here |

## 7. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| `0x00598960` calls `GenerateTerrainPreview` 8 times during generation (not once), each immediately followed by a synchronous `WM_PAINT` that repaints the dialog's preview control before the next phase runs (`get_function_callees 0x00598960`; `get_xrefs_to 0x00641140`; `decompile_function 0x00598960`) | Current Rust renders the preview exactly once, after `generate_map` fully completes (`src/app.rs` `Control::Generate0x620` arm) - this is a **visible behavioral difference**: gamemd shows a progressively-filling preview across the pause, Rust shows a frozen dialog then one final image | `src/app.rs`, `src/map/rmg/build.rs`, `src/map/rmg/preview.rs`, `src/ui/skirmish_shell/state/random_map_setup.rs` | Trigger Generate on a map large enough to take multiple frames; assert the preview texture is rebuilt/updated more than once before the final image, at stage boundaries corresponding to the 8 native call sites | `test_generate_emits_incremental_preview_frames` | Medium: requires threading a preview-refresh callback/channel through `generate_map`'s phases without breaking determinism of the generation algorithm itself (out of this doc's scope) |
| The `0x6C5` OK handler in normal (non-editor) skirmish play calls `Generate(1,hDlg)` only when no usable preview already exists; the map-editor branch of the same handler calls `Generate(0,hDlg)` - no preview, no repaint, always followed by a map-file save (`decompile_function 0x00596300`) | Rust's OK-path equivalent (if/when implemented) must gate its own generate-with-preview call on "no existing preview", and must never call the map-editor's preview-suppressed variant in skirmish | future OK/accept handler for the random-map setup dialog | Click OK with no prior Generate; assert a preview is produced before accept. Click OK after a prior Generate already produced a preview; assert no second generation runs | `test_ok_generates_only_if_no_existing_preview` | Low: dialog/state-machine only, no rendering-pipeline change |
| Ordinary scenario/map load (`ScenarioClass__Read_Scenario`) calls `Generate(0, NULL)` - gameplay generation never triggers a preview surface or any repaint (`get_assembly_context 0x00684989`) | No Rust delta needed if the scenario-launch generation path is already preview-free; confirm it does not accidentally build/populate a preview texture | scenario/map-load random-generation entry point | Load a random-map scenario directly (skip setup dialog); assert no preview texture/surface is allocated as a side effect | `test_scenario_launch_generation_produces_no_preview_surface` | Low: negative-assertion test, cheap to add |

### Negative Facts / Do Not Do

- Do not implement a live-repainting main-game display-chain flip inside the generation loop: `g_DisplayChain+0xC`/`+0x10` bracket the entire `0x620` WndProc handler exactly once each and never appear inside `RandomMapGenerator__Generate` itself. Active in YR: No as a per-stage mechanism. Evidence: `decompile_function 0x00596300`; absence confirmed via `get_function_callees 0x00598960`.
- Do not treat `Pipe__Constructor @ 0x005e7eb0` as a display/repaint mechanism or a per-stage preview-persistence step: it is a network multiplayer lobby map-preview upload/broadcast routine (compresses and sends `Preview.bin` to connected session clients), gated on `g_GameMode==4 && DAT_00a8b244==3` and player count `>1`. The Ghidra label "Pipe__Constructor" is misleading for this address - its body is not primarily a constructor. Active in YR: Conditional, multiplayer-lobby only; not relevant to a local/skirmish Generate click. Evidence: `decompile_function 0x005e7eb0`.
- Do not implement or wire up control `0x639` (hidden progress button) as part of this flow - it is never shown, hidden, or driven by `0x00598960`. Active in YR: No. Evidence: `disassemble_function 0x00598960` / `decompile_function 0x00598960` (no `0x639` literal anywhere).
- Do not assume the `0x638` "Working/Please Wait" static's visibility is controlled solely by the WndProc's `0x620` handler - `RandomMapGenerator__Generate` itself also shows it at entry and (unconditionally within its own `bVar14` gate) hides it again before returning, independent of the `bPreview` argument. Active in YR: Conditional on `g_ScenarioClass_Instance+0x3598`. Evidence: `decompile_function 0x00598960`.
- Do not conflate the numeric/text progress-display gate (`g_ScenarioClass_Instance+0x3598`) with the preview-repaint gate (`bPreview`/`param_2`) - they are read from different locations and can differ independently; all 8 `GenerateTerrainPreview`+`WM_PAINT` blocks are gated ONLY on `bPreview`. Active in YR: confirmed both flags are checked separately at every phase boundary. Evidence: `decompile_function 0x00598960`.

### Remaining Uncertainty

- The exact identity/semantic name of the `g_ScenarioClass_Instance+0x3598` flag (candidate: a "batch/headless generation" or "quiet mode" indicator) was not resolved beyond its observed effect (gates `0x638` visibility and numeric-vs-text progress display). Does not block the preview/repaint findings above, which depend only on `bPreview`.
- The function containing the 5th call site at `0x00596182` has no recovered boundary in the current Ghidra project (`get_function_by_address` returns none); its surrounding code is structurally identical to the `0x620` handler's tail (`Generate(1,hDlg)` + direct `GenerateTerrainPreview()` + the same multiplayer-upload gate), suggesting another UI entry point that triggers random-map generation directly, but its trigger (which command/dialog) was not identified in this slice.
- The precise identity of the progress-display singleton at global `0xac4f58` (driven by `FUN_00642a60`/`FUN_00642c20`/`FUN_00642c80`/`FUN_00643ae0`/`FUN_00643c50`/`FUN_00643e70`) was not resolved; it is orthogonal to the preview-repaint mechanism and was only characterized functionally (numeric percent vs. text-log routing).

## Sources

- Ghidra read-only decompile/disassembly: `RandomMapGenerator__Generate @ 0x00598960` (`decompile_function`, `disassemble_function`), `RandomMapSetupDialog__Proc @ 0x00596300` (`decompile_function`), `Pipe__Constructor @ 0x005e7eb0` (`decompile_function`).
- Ghidra xrefs/callers/callees: `get_function_callees 0x00598960`; `get_xrefs_to 0x00641140`; `get_function_callers 0x00598960`; `get_xrefs_to 0x00598960`; `get_function_by_address 0x00596182` (negative result).
- Ghidra assembly context: `get_assembly_context` for `0x00684989`, `0x0059664c`, `0x00596a49`, `0x00596a66`, `0x00596182`.
- Prior plate comments read (not re-derived): `get_plate_comment 0x00598960`; `get_plate_comment 0x00596300`.
- Prior docs read for context/cross-check (not duplicated as new evidence): `SKIRMISH_RANDMAP_IMG_PREVIEW_GENERATION_HANDOFF_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/app.rs`, `src/ui/skirmish_shell/state/random_map_setup.rs`.
