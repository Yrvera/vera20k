# Loading Direct-Draw Repaint / Present Path - Ghidra Research Report

**Address(es):** `0x0069AE90`, `0x00643C50`, `0x00643AE0`, `0x00642A60`, `0x00684620`, `0x004F4780`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline Skirmish loading-progress repaint visibility after progress updates: `ProgressClass+0x64` HWND/null branch, direct-draw ownership, callback-local paint/blit timing, and Rust app-loop implications.  
**Non-Scope:** progress geometry/math except branch connection, first LS background composition, milestone ledger enumeration, campaign loading text, random-map generation preview UI.  
**Confidence:** High for standard Skirmish null-HWND direct-draw cadence and synchronous display flush; Medium for semantic naming of the low-level blit helper internals.  
**Active in YR:** Yes. Standard offline Skirmish (`g_GameMode == 5`, `g_IsMapEditor == 0`) reaches `ScenarioClass__Read_Scenario @ 0x00684620`, initializes `ProgressClass` with null HWND, then uses `FUN_0069AE90` milestones during scenario/full-init loading.

## 1. Overview

Standard YR scenario loading does not just store progress and wait for a later window repaint. Advancing milestones synchronously draw the progress row on the DirectDraw surface and call the display blit helper before loader execution continues.

For ordinary offline Skirmish, the `ProgressClass+0x64` HWND field is initialized to null, so the live path is the direct-draw fallback in `FUN_00643AE0`, not the `msctls_progress32`/child-HWND branch.

## 2. Target Question

Verify the standard offline Skirmish direct-draw repaint/present path after progress updates: `ProgressClass` HWND/null branch, BSurface/screen blit timing, whether `FUN_0069AE90` or its callees make pixels visible immediately, and what this implies for Rust app-loop pumping/present boundaries.

## 3. Non-Goals

- Do not re-investigate `PROGBARM.SHP` frame geometry except where it proves draw ownership.
- Do not enumerate every milestone caller; sibling slots own the milestone ledger.
- Do not decode first-renderer LS background composition.
- Do not write Rust.

## 4. Evidence Needed To Mark COMPLETE

- Prove standard Skirmish initializes `ProgressClass+0x64` to null.
- Prove the null path calls direct draw rather than `WM_PAINT`.
- Prove changed milestones cause immediate visible work and unchanged/lower milestones do not.
- Prove the direct draw path reaches the display blit/present helper before returning to the loader.
- Compare against current Rust loading transition timing.

## 5. Stop Conditions

- Stop after `0x0069AE90`, `0x00643C50`, `0x00643AE0`, `0x00642A60`, standard `0x00684620` setup, and display helper `0x004F4780` are accounted for.
- Stop before full owner-draw `msctls_progress32` internals beyond branch exclusion.
- Stop before milestone enumeration and pixel geometry.

## 6. Core Verified Findings

| Finding | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| Standard scenario load initializes `ProgressClass+0x64` to null. `ScenarioClass__Read_Scenario` calls `FUN_00642A60` with final argument `0`; `FUN_00642A60` writes `param_1[0x19]` only when that argument is nonzero and otherwise leaves `+0x64` zero. | `0x006846FB..0x00684706`, `0x00642A60` | High | Yes |
| `ProgressClass+0x64` is the repaint branch switch. Non-null sends synchronous `SendMessageA(hwnd, WM_PAINT, 0, 0)`; null calls `FUN_00643AE0(x,y)`. | `0x00643C50`, assembly `0x00643D26..0x00643D4B` | High | Yes |
| Standard offline Skirmish therefore uses the direct-draw fallback after progress changes, not the child-HWND progress-control branch. | combination of `0x00684620`, `0x00642A60`, `0x00643C50` | High | Yes |
| `FUN_0069AE90` suppresses non-advancing milestones before any draw call: it compares current percent times `100.0` against the requested milestone and calls `FUN_00643C50` only when the requested milestone is greater. | `0x0069AE90` | High | Yes |
| `FUN_00643C50` also suppresses unchanged stored values. It stores/clamps `max * 0.01 * percent`, compares old lane double to the new stored value, and only then sends `WM_PAINT` or calls direct draw. | `0x00643C50` | High | Yes |
| Direct draw is visible before returning to the loader. `FUN_00643AE0` draws rows via `FUN_00643720`/`FUN_00643400`, then calls `FUN_004F4780(0)`; assembly shows `EDX = DAT_0088730C`, `CL = 1`, `PUSH 0`, then `CALL 0x004F4780`. | `0x00643AE0`, assembly `0x00643C38..0x00643C42`, `0x004F4780` | High | Yes |
| `FUN_004F4780` is a display-surface blit/present helper: it reads the game HWND client rect, translates to screen coordinates, computes the source rect from the passed surface, and calls the display chain / primary display surface vtable methods to copy visible pixels. | `0x004F4780` decompile | Medium-High | Yes |
| The callback path does not use deferred-only invalidation for standard load. No `InvalidateRect` appears in `FUN_0069AE90`, `FUN_00643C50`, or the standard null-HWND branch; the non-null branch uses synchronous `SendMessageA(WM_PAINT)`. | `0x0069AE90`, `0x00643C50`, `0x00643AE0` | High | Yes |

## 7. Direct-Draw / Present Sequence

Standard offline Skirmish sequence for an advancing milestone:

1. Loader calls `FUN_0069AE90(milestone)`.
2. If random-map flag `ScenarioClass+0x34BD` is set, the milestone is halved; normal maps skip this branch.
3. `FUN_0069AE90` reads current progress fraction from `FUN_00643E90(0)` and compares `current * 100.0 < milestone`.
4. Only if the milestone advances, it calls `FUN_00643C50(row 0, milestone, -1, -1)`.
5. `FUN_00643C50` writes the lane value as `max * 0.01 * milestone`, clamps above max, and compares old vs new lane double.
6. If the lane changed and `+0x64 == 0`, it calls `FUN_00643AE0(-1, -1)`.
7. `FUN_00643AE0` substitutes stored origin `+0x68/+0x6C`, draws the row(s), and calls `FUN_004F4780(0)`.
8. `FUN_004F4780` copies the draw surface to the visible client area before control returns to the loader.

Therefore, a native milestone is a synchronous "draw and blit now" boundary. It is not merely an event that the outer Windows message pump will later render when loading finishes.

## 8. HWND / Null Branch Ledger

| Path | Setup evidence | `+0x64` | Visibility behavior | Active for standard offline Skirmish? |
|---|---|---:|---|---|
| Standard `ScenarioClass__Read_Scenario` | `0x006846FB..0x00684706` passes final arg `0` to `0x00642A60` | null | `FUN_00643AE0(-1,-1)` direct draw, then `FUN_004F4780(0)` | Yes |
| Non-null ProgressClass setup | callers such as `FUN_00598960` can call `FUN_00642A60(..., hwnd)` when not in scenario-load state | HWND | synchronous `SendMessageA(hwnd, WM_PAINT, 0, 0)` | Conditional, not the ordinary Skirmish scenario-load progress path |
| `msctls_progress32` control plumbing | prior report verified subclass install around `0x0060F9A0` / proc `0x0061D6D0` | child HWND | child control paint/validate path | Conditional, not needed for standard Skirmish load |

## 9. Current Rust Implementation Status

| Surface | Current behavior | Delta |
|---|---|---|
| `src/app.rs` `GameScreen::Loading` | Draws egui loading text for the loading screen. | Missing native direct-draw-equivalent surface. |
| `src/app.rs` post-present block | After presenting the one loading frame, immediately calls `app_transitions::transition_to_in_game`. | Missing per-milestone render/present boundaries. |
| `src/app_transitions.rs::transition_to_in_game` | Calls `app_init::load_map(...)` synchronously and only returns with completed game state. | Missing pumpable loader or progress callback that can draw/present between native milestones. |
| `src/ui/main_menu.rs::draw_loading_screen` | Shows invented egui loading copy/map display. | Not a parity surface for standard Skirmish loading. |

## 10. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0069AE90` advancing-milestone gate | verified | decompile `0x0069AE90` | none for repaint visibility |
| `FUN_00643C50` old/new gate and HWND switch | verified | decompile `0x00643C50`, assembly `0x00643D26..0x00643D4B` | none for scoped branch |
| Standard `Read_Scenario` `+0x64` initialization | verified | `0x00684620`, `0x00642A60`, assembly `0x00684700..0x00684706` | none |
| `FUN_00643AE0` direct draw fallback | verified | decompile `0x00643AE0`, assembly `0x00643C38..0x00643C42` | exact progress geometry belongs to geometry report |
| `FUN_004F4780` display blit helper | verified | decompile `0x004F4780`, xref from `0x00643C42` | exact DirectDraw vtable names remain semantic inference |
| Non-null HWND path | verified for branch behavior | `0x00643C50`, prior `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md` | all-mode runtime HWND inventory out-of-scope |
| `msctls_progress32` proc internals | deferred | prior report only | not needed for standard null-HWND path |

## 11. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the standard Skirmish load path live in YR? -> Yes, `g_GameMode == 5` goes through `ScenarioClass__Read_Scenario @ 0x00684620`, which initializes loading progress when not in map editor.` (evidence: `0x00684620`)
- `[RESOLVED] OQ-02 - What does standard load store in `ProgressClass+0x64`? -> Null/zero.` (evidence: `0x006846FB..0x00684706`, `0x00642A60`)
- `[RESOLVED] OQ-03 - What does `+0x64` control? -> Non-null sends synchronous `WM_PAINT`; null calls `FUN_00643AE0`.` (evidence: `0x00643C50`)
- `[RESOLVED] OQ-04 - Does `FUN_0069AE90` itself make visible pixels? -> Indirectly yes for advancing milestones: it calls `FUN_00643C50`, which calls direct draw on the standard null-HWND path.` (evidence: `0x0069AE90`, `0x00643C50`)
- `[RESOLVED] OQ-05 - Do duplicate/lower milestones blit again? -> No; they fail the advance gate or unchanged-value gate before paint/direct draw.` (evidence: `0x0069AE90`, `0x00643C50`)
- `[RESOLVED] OQ-06 - Does the standard branch rely on `InvalidateRect`? -> No; no callback-local invalidation is used, and null-HWND direct draw bypasses window invalidation.` (evidence: `0x00643C50`, `0x00643AE0`)
- `[RESOLVED] OQ-07 - Does direct draw flush to the visible surface before returning? -> Yes; `FUN_00643AE0` ends with `FUN_004F4780(0)`, whose body copies the draw surface through display/primary-surface calls.` (evidence: `0x00643AE0`, `0x004F4780`)
- `[RESOLVED] OQ-08 - What does this imply for Rust app-loop boundaries? -> Rust must expose a render/present point at advancing milestones; one pre-load egui frame plus synchronous load cannot match native visible cadence.` (evidence: binary sequence plus `src/app.rs`, `src/app_transitions.rs`)
- `[DEFERRED] OQ-09 - Exact runtime HWND inventory for every loading/dialog progress mode.` (category: out-of-scope; reason: target is standard offline Skirmish null-HWND path; next-step-if-pursued: runtime trace `ProgressClass+0x64` for all dialog/random-map modes)
- `[DEFERRED] OQ-10 - Full semantic names of every `FUN_004F4780` vtable method.` (category: bounded-cost-too-high; reason: scoped evidence only needs visible blit ownership and call order; next-step-if-pursued: dedicated DirectDraw surface/display-chain investigation)

## 12. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `FUN_0069AE90 @ 0x0069AE90` | requested milestone must be greater than current percent | none | n/a | n/a | yes | progress gate |
| 2 | `FUN_00643C50 @ 0x00643C50` | changed lane value; `+0x64 == 0` in standard load | none | n/a | n/a | yes | repaint/direct-draw dispatch |
| 3 | `FUN_00643AE0 @ 0x00643AE0` | `+0x60 != 0`, `+0x54 != 0`, null HWND | `PROGBARM.SHP` through row helpers | stored `+0x68/+0x6C` when args are `-1,-1` | player/session color scheme via row helpers | yes | immediate progress draw |
| 4 | `FUN_00643720` / `FUN_00643400` | row draw from direct fallback | `PROGBARM.SHP` frame 0 | geometry covered by sibling report | color scheme convert | yes | progress fill/text row |
| 5 | `FUN_004F4780 @ 0x004F4780` | called after direct row draw | drawn surface, not an asset | HWND client rect to screen rect | n/a | yes | visible blit/present boundary |

Asset role matrix:

| Asset / Surface | Loaded | Drawn | Visible in target | Overlay | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---|
| `PROGBARM.SHP` | yes | yes | yes | progress row | no | `0x00643AE0`, sibling geometry report |
| `DAT_0088730C` draw surface | yes/global | yes | copied to client | display backing | no | assembly `0x00643C38`, `0x004F4780` |
| `msctls_progress32` child surface/proc | conditional | conditional | no for standard null-HWND load | progress-control branch | yes for standard Skirmish load | `0x00643C50`, prior repaint report |

## 13. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard Skirmish loading progress uses null-HWND direct draw; advancing milestone draws and blits before loader continues. | `0x00684620`, `0x00642A60`, `0x00643C50`, `0x00643AE0`, `0x004F4780` | missing | `src/app.rs`, `src/app_transitions.rs`, future loading job/render state | Pump loading in phases and render/present after each native advancing milestone before continuing loader work. | Starting a standard Skirmish map shows multiple native progress states before InGame transition. | Do not perform one presented loading frame and then block through the whole load. |
| Duplicate/lower milestones cause zero visible work. | `0x0069AE90`, `0x00643C50` | missing/unchecked | future loading progress state/test harness | Gate redraw/present on strictly advancing stored progress, not on every loader callback. | Replaying `[3,3,2,8]` presents only `3` and `8`. | Do not animate, smooth, or continuously repaint between milestones. |
| Non-null HWND path is synchronous `SendMessageA(WM_PAINT)`, but it is not the standard Skirmish scenario-load branch. | `0x00643C50`; standard null setup at `0x00684706` | unchecked | future dialog/progress-control emulation only | Keep HWND path as a possible later UI mode, but model standard loading with direct-draw-equivalent renderer. | A future HWND-backed progress dialog can synchronously repaint; standard Skirmish still uses direct cadence. | Do not route standard Skirmish through generic child progress-control invalidation. |

Proposed Rust test names:

- `loading_progress_standard_skirmish_presents_on_advancing_milestones`
- `loading_progress_duplicate_or_lower_milestones_do_not_present`
- `loading_progress_standard_skirmish_does_not_use_hwnd_progress_path`

## 14. Negative Facts / Do Not Do

- Do not treat `FUN_0069AE90` as telemetry only; on the standard path it reaches immediate draw and blit.
- Do not model ordinary Skirmish loading as child-HWND / `msctls_progress32` paint.
- Do not rely on deferred invalidation or the outer OS message pump to expose progress after the full load.
- Do not present on duplicate/lower milestones.
- Do not use Rust egui loading text/map-name frames as the parity loading surface.

## 15. Remaining Uncertainty

- Exact DirectDraw vtable method names inside `FUN_004F4780` remain semantic rather than symbol-verified.
- All-mode runtime HWND inventory is outside this standard offline Skirmish slice.

## 16. Stale Docs / Follow-up Docs

Suggested replacement wording for `C:/Users/enok/Documents/ra2-rust-game/docs/plans/2026-05-23-standard-offline-skirmish-loading-plan.md`:

> Standard offline Skirmish loading progress is a null-HWND direct-draw path. `Read_Scenario` initializes `ProgressClass+0x64` to zero, so advancing `FUN_0069AE90` milestones call `FUN_00643AE0` and then `FUN_004F4780(0)` before loader execution continues. Rust must render and present at advancing native milestones; a single loading frame followed by synchronous map load is not equivalent. Duplicate/lower milestones must not present.

Suggested replacement wording for `C:/Users/enok/Documents/ra2-rust-game-docs/LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`:

> The standard offline Skirmish progress branch is now verified as null-HWND direct draw: `ScenarioClass__Read_Scenario` passes zero into `FUN_00642A60`, leaving `ProgressClass+0x64` null; changed progress values call `FUN_00643AE0(-1,-1)`, which draws the progress rows and calls `FUN_004F4780(0)` to blit to the visible client surface. The non-null `SendMessageA(WM_PAINT)` branch is real but not the ordinary Skirmish scenario-load path.

## Sources

- Ghidra decompile: `0x0069AE90`, `0x00643C50`, `0x00643AE0`, `0x00642A60`, `0x00684620`, `0x004F4780`, `0x00598960`.
- Ghidra assembly context: `0x00643C38..0x00643C42`, `0x00643D26..0x00643D4B`, `0x00684700..0x00684706`.
- Prior reports checked: `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`, `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md`, `PROGBARM_PROGRESSCLASS_DRAW_GEOMETRY_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs`, `src/app_transitions.rs`, `src/ui/main_menu.rs`.

## Status

COMPLETE for the scoped standard offline Skirmish repaint/present path.
