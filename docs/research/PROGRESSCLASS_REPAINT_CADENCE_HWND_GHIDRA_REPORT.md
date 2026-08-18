# ProgressClass Repaint Cadence / HWND - Ghidra Research Report

**Address(es):** `0x00643C50`, `0x0069AE90`, `0x00642A60`, `0x00643AE0`, `0x0060F9A0`, `0x0061D6D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** ProgressClass value-update, repaint/direct-draw dispatch, standard scenario-load HWND state, and conditional `msctls_progress32` subclass behavior.  
**Non-Scope:** `PROGBARM.SHP` pixel geometry, full loading-background composition, and full owner-draw widget system outside the progress-control cases needed for HWND identity.  
**Confidence:** High for standard scenario-load cadence and direct-draw path; High for conditional HWND `WM_PAINT` path; Medium for exact runtime parent/child HWND identity outside standard scenario load because static Ghidra does not provide live window handles.  
**Active in YR:** Yes. Standard YR scenario loading through `ScenarioClass__Read_Scenario @ 0x00684620` initializes and uses this ProgressClass path.

## Target Question

Confirm the ProgressClass repaint/update plumbing around `FUN_00643C50`, `ProgressClass+0x64`, `WM_PAINT` send, direct draw fallback, and `msctls_progress32`/`0x0061D6D0` owner-draw candidates. Determine when updates send `WM_PAINT` versus direct draw, whether callbacks invalidate or synchronously draw, and how many visible redraws occur for unchanged milestones.

## Non-Goals

- Do not decode `PROGBARM.SHP` pixel geometry beyond identifying draw target.
- Do not expand into the full shell/loading-screen background paint stack.
- Do not implement Rust changes.
- Do not mutate Ghidra labels, comments, functions, or data.

## Evidence Needed To Mark COMPLETE

- Verify the `FUN_0069AE90 -> FUN_00643C50` update gate and unchanged-milestone behavior.
- Verify what `ProgressClass+0x64` controls in `FUN_00643C50`.
- Verify standard YR scenario-load initialization of `ProgressClass+0x64`.
- Verify whether the callback uses `InvalidateRect` or synchronous draw/repaint.
- Bound the role of the `msctls_progress32` owner-draw proc candidate at `0x0061D6D0`.

## Stop Conditions

- Stop after `FUN_00643C50`, `FUN_0069AE90`, `FUN_00642A60`, `FUN_00643AE0`, `ScenarioClass__Read_Scenario`, and the owner-draw install path around `FUN_0060F9A0`/`0x0061D6D0` are accounted for.
- Stop before full `PROGBARM` geometry and full owner-draw widget system.
- Stop if Ghidra requires DB mutation to create a missing function boundary; use assembly context only.

## Verified Facts

| Fact | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| `FUN_0069AE90` only requests a ProgressClass update when requested milestone percent is greater than current percent; equal/lower milestones produce no update call. | `0x0069AE90` reads `FUN_00643E90(0)`, multiplies by `100.0`, compares `< param_2`, then calls `FUN_00643C50` only on true. | High | Yes |
| `FUN_00643C50` also gates visible work on actual stored value change; same stored value causes no `WM_PAINT` send and no direct draw. | `0x00643C50` saves old slot double, writes/clamps new `max * 0.01 * milestone`, then only enters repaint/draw branch when old double differs from new double. | High | Yes |
| `ProgressClass+0x64` is the HWND switch: non-null sends synchronous `SendMessageA(hwnd, WM_PAINT, 0, 0)`; null calls direct draw fallback `FUN_00643AE0`. | `0x00643C50` tests `*(HWND *)(this+100)` and either calls `SendMessageA(...,0x0F,0,0)` or `FUN_00643AE0(-1,-1)`. | High | Yes |
| Standard YR scenario loading initializes `ProgressClass+0x64` to zero, so standard map load uses direct draw fallback, not a child-HWND `WM_PAINT` path. | `ScenarioClass__Read_Scenario @ 0x006846FB..0x00684706` pushes hwnd `0` into `FUN_00642A60`; `FUN_00642A60` writes zero to offset `0x64` when that argument is zero. | High | Yes |
| The direct draw fallback draws the progress surface immediately and flushes/presents through `FUN_004F4780(0)`; it does not invalidate a window. | `0x00643AE0` checks enabled/assets, draws via `FUN_00643720`/`FUN_00643400`, then calls `FUN_004F4780(0)` on the non-HWND path; callees of `0x00643C50` contain no `InvalidateRect`. | High | Yes |
| `FUN_0060F9A0` subclasses `msctls_progress32` controls by selecting proc address `0x0061D6D0`, installing dispatcher `0x00610CA0`, and sending setup message `0x497`; this is conditional HWND/control plumbing, not the standard scenario-load path. | `0x0060F9A0` class-string compare against `msctls_progress32`, assigns `pcVar13 = &LAB_0061D6D0`, `SetWindowLongA(hwnd, GWL_WNDPROC, 0x00610CA0)`, stores previous proc, sends `SendMessageA(hwnd,0x497,0,0)`. | High for install, Medium for runtime child use in target load | Conditional |
| The `0x0061D6D0` proc handles `WM_PAINT`, paints from the RA2 DirectDraw surface path, and calls `ValidateRect`; its own value/range messages can invalidate, but ProgressClass callback does not use those messages in standard load. | Assembly context `0x0061D6D0..0x0061D94B`: cases for `0x0F`, `0x401`, `0x402`, `0x497`; `WM_PAINT` case allocates/copies surface, draws fill, calls `ValidateRect`; `0x402` clamps position and calls `InvalidateRect`. | Medium-High | Conditional |

## Repaint / Cadence Semantics

For the standard scenario-loading screen, the callback cadence is milestone-driven and monotonic:

1. `FUN_0069AE90` halves random-map milestones first, then compares requested milestone against current percent.
2. If requested milestone is not greater than current percent, no ProgressClass update occurs.
3. If requested milestone advances, `FUN_00643C50` updates the stored slot value.
4. If the stored value did not actually change after scaling/clamp, no visible work occurs.
5. If the stored value changed and `+0x64 == 0`, standard scenario load calls `FUN_00643AE0(-1,-1)` and draws immediately.
6. If the stored value changed and `+0x64 != 0`, non-standard/HWND-backed uses synchronous `SendMessageA(hwnd, WM_PAINT, 0, 0)`.

Therefore unchanged milestones should cause **zero visible redraws** in the ProgressClass path. This is true both because the callback suppresses non-advancing milestones and because `FUN_00643C50` suppresses unchanged stored values.

## HWND / Child-Control Behavior

| Path | `+0x64` value | Paint behavior | Evidence | Active in standard scenario load? |
|---|---:|---|---|---|
| Standard `ScenarioClass__Read_Scenario` load | `0` | direct draw `FUN_00643AE0(-1,-1)` | `0x00684700..0x00684706`, `0x00642A60`, `0x00643C50` | yes |
| Dialog/random-map preview style setup through `FUN_00598960` | caller HWND | synchronous `SendMessageA(hwnd, WM_PAINT)` | `0x005989F5..0x005989FB`, `0x00643C50` | conditional, not the standard scenario-load progress screen |
| Owner-drawn `msctls_progress32` control | actual child HWND owned by UI system | dispatcher calls proc `0x0061D6D0`; `WM_PAINT` validates after drawing | `0x0060F9A0`, `0x0061D6D0` assembly | conditional |

The standard loading `PROGBARM.SHP` target is the direct draw ProgressClass surface selected by `FUN_00642C20("PROGBARM.SHP", ...)` for non-campaign loads and painted through `FUN_00643AE0` / `FUN_00643720`. The `msctls_progress32` proc is real progress-control plumbing, but it is not required to explain the normal YR scenario-load cadence because `+0x64` is initialized null there.

## Callback / Invalidation Result

- `FUN_0069AE90` does not invalidate a window.
- `FUN_00643C50` does not invalidate a window.
- Standard scenario-load progress updates synchronously direct-draw and flush.
- HWND-backed ProgressClass updates synchronously send `WM_PAINT`; they do not post paint or use deferred-only invalidation.
- `0x0061D6D0` may call `InvalidateRect` for its own control messages, but that is child-control internal behavior, not the standard loading callback path.

## Current Rust Implementation Status

| Surface | Current behavior | Delta |
|---|---|---|
| `src/app.rs` | Presents `GameScreen::Loading`, then after one presented frame calls `app_transitions::transition_to_in_game`. | Missing native multi-milestone visible loading cadence. |
| `src/app_transitions.rs` | Calls `app_init::load_map` synchronously from the loading transition. | Missing pumpable/staged load or progress channel that can repaint between milestones. |
| `src/ui/main_menu.rs` | Draws egui text `Loading...`; no `PROGBARM.SHP` progress surface. | Missing direct-draw-equivalent progress surface and redraw gate. |

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0069AE90` advance gate | verified | `0x0069AE90` | none |
| `FUN_00643C50` changed-value gate | verified | `0x00643C50` | none |
| `ProgressClass+0x64` HWND switch | verified | `0x00643C50`, `0x00642A60` | runtime handle identity outside standard load |
| standard scenario-load `+0x64 == 0` | verified | `0x00684700..0x00684706` | none |
| direct draw fallback | verified | `0x00643AE0` | exact pixels/rects out of scope |
| `msctls_progress32` subclass install | verified | `0x0060F9A0` | exact dialogs/control IDs using it during all modes |
| proc `0x0061D6D0` paint cases | touched-not-exhausted | assembly context `0x0061D6D0..0x0061D94B` | full owner-draw decompile blocked by missing function boundary without mutation |

## Open Questions - Final State

- `[RESOLVED] OQ-01 - Does callback repaint on unchanged milestones? -> No; non-advancing milestones do not reach `FUN_00643C50`, and unchanged stored values do not draw.` (evidence: `0x0069AE90`, `0x00643C50`)
- `[RESOLVED] OQ-02 - What decides WM_PAINT vs direct draw? -> `ProgressClass+0x64`; non-null HWND sends `WM_PAINT`, null calls `FUN_00643AE0`.` (evidence: `0x00643C50`)
- `[RESOLVED] OQ-03 - What is `+0x64` in standard scenario load? -> Zero/null.` (evidence: `0x00684700..0x00684706`, `0x00642A60`)
- `[RESOLVED] OQ-04 - Does standard scenario load use direct draw fallback? -> Yes, because `+0x64` is null and `FUN_00643AE0` draws immediately.` (evidence: `0x00643C50`, `0x00643AE0`)
- `[RESOLVED] OQ-05 - Does the callback use `InvalidateRect`? -> No in `FUN_0069AE90`/`FUN_00643C50`; repaint/draw is synchronous.` (evidence: callees of `0x00643C50`, `0x00643C50`)
- `[RESOLVED] OQ-06 - Is `0x0061D6D0` a progress-control proc candidate? -> Yes; selected specifically for `msctls_progress32` by `FUN_0060F9A0`.` (evidence: `0x0060F9A0`)
- `[RESOLVED] OQ-07 - Is `0x0061D6D0` necessary for standard scenario-load progress cadence? -> No; standard scenario load stores null HWND and bypasses child-HWND painting.` (evidence: `0x00684706`, `0x00643C50`)
- `[DEFERRED] OQ-08 - Which exact runtime HWNDs are live for every shell/progress dialog?` (category: needs-runtime-debugger; reason: static code proves the branch and standard null-HWND load, but not all live HWND instances; next-step-if-pursued: runtime trace `ProgressClass+0x64` and `GetClassNameA` during each loading/dialog mode)

## Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `FUN_00642C20` from `ScenarioClass__Read_Scenario` | non-campaign branch | `PROGBARM.SHP` | geometry deferred | loading palette path | yes | selects progress target |
| 2 | `FUN_00643C50` | milestone advances and stored value changes | none | N/A | N/A | yes | update gate |
| 3 | `FUN_00643AE0` | `+0x64 == 0` | ProgressClass shape via `+0x54` | `+0x68/+0x6C` or supplied coords | DirectDraw surface | yes for standard scenario load | immediate progress draw |
| 4 | `0x0061D6D0` proc | `msctls_progress32` subclassed HWND | generic control fill | child client rect | DirectDraw surface copy/fill | conditional | child progress control |

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard YR map load uses milestone-driven, synchronous direct draw because `ProgressClass+0x64 == 0`. | `0x00684706`, `0x00643C50`, `0x00643AE0` | missing | `src/app.rs`, `src/app_transitions.rs`, loading UI/render surface | keep loading UI live and draw progress immediately when native milestones advance | Starting a Skirmish map presents multiple visible progress states before game entry | Do not model standard load as child-HWND `WM_PAINT` only |
| Unchanged or non-advancing milestones produce zero visible redraws. | `0x0069AE90`, `0x00643C50` | missing/unchecked | future loading progress state/test harness | only emit visible redraw events on strictly advancing, stored-value-changing milestones | Replaying duplicate milestone inputs records no extra draw events | Do not smooth, animate, or redraw continuously for unchanged percent |
| HWND-backed progress controls are synchronous `WM_PAINT` fallback for non-null `+0x64`, with `msctls_progress32` subclass proc conditional outside standard load. | `0x00643C50`, `0x0060F9A0`, `0x0061D6D0` | unchecked | dialog/progress control emulation if added | support synchronous paint when a UI progress HWND surface exists, but do not use it for standard scenario load unless Rust creates an equivalent mode | A dialog progress control update sends a single immediate paint for changed value | Do not use deferred-only invalidation as the callback model |

Proposed test names:

- `loading_progress_standard_skirmish_uses_direct_draw_cadence`
- `loading_progress_duplicate_milestones_do_not_redraw`
- `loading_progress_hwnd_path_sends_synchronous_wm_paint_when_present`

## Negative Facts / Do Not Do

- Do not claim normal YR scenario loading paints through `msctls_progress32`; the verified standard path initializes `+0x64` to null.
- Do not implement progress as continuous or timer-driven smoothing.
- Do not redraw duplicate or lower milestones.
- Do not replace synchronous direct draw/paint with deferred-only invalidation if it changes visible cadence.
- Do not decode `PROGBARM` pixel geometry from this report; only the draw target and cadence are in scope.

## Remaining Uncertainty

- Exact runtime HWND identities across every shell/dialog mode remain runtime-debugger work.
- Ghidra lacks a function boundary at `0x0061D6D0`; assembly context is sufficient for the scoped paint/case claims but not a full owner-draw proc audit.
- Exact progress-bar rects and pixels are intentionally deferred.

## Stale Docs / Follow-up Docs

Suggested replacement wording for the prior medium-confidence HWND note:

> Standard YR scenario loading initializes `ProgressClass+0x64` to null, so normal map-load progress changes use the direct draw fallback `FUN_00643AE0` rather than a child-HWND `WM_PAINT`. `FUN_00643C50` still supports a non-null HWND path that synchronously sends `WM_PAINT`; `msctls_progress32` subclass proc `0x0061D6D0` is real conditional progress-control plumbing, but it is not the standard scenario-load cadence path.

## Status

COMPLETE for the scoped standard scenario-load repaint cadence, `+0x64` switch, unchanged milestone redraw count, and conditional `msctls_progress32` relationship. PARTIAL only for all-mode runtime HWND identity and full owner-draw proc internals, which are outside this slice.

## Sources

- Ghidra decompile: `FUN_0069AE90`, `FUN_00643C50`, `FUN_00643AE0`, `FUN_00642A60`, `FUN_00642C20`, `FUN_00642C80`, `ScenarioClass__Read_Scenario`, `FUN_00598960`, `FUN_0060F9A0`.
- Ghidra assembly context: `0x0061D6D0..0x0061D94B`, `0x00684700..0x00684706`, `0x005989F5..0x005989FB`, `0x0060FBD4`, `0x0060FF70`.
- Prior report checked: `docs/research/LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs`, `src/app_transitions.rs`, `src/ui/main_menu.rs`.
