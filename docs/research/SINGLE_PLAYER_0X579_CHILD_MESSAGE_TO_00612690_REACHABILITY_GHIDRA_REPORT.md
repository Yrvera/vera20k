# Single Player 0x579 Child Message To 0x00612690 Reachability - Ghidra Report

**Date:** 2026-05-27  
**Target:** `SINGLE_PLAYER_0X579_CHILD_MESSAGE_TO_00612690_REACHABILITY`  
**Address(es):** Single Player dialog proc `0x0052D640`, shell child subclass dispatcher `0x00610CA0..0x006128FE`, transition call site `0x00612690`, `FUN_00608260 @ 0x00608260`, Button owner proc `OwnerDraw_Button_00612B70 @ 0x00612B70`, installer `FUN_0060F9A0 @ 0x0060F9A0`.  
**Investigation mode:** narrow static reachability closure for dialog `0x100` Skirmish child button `0x579`.  
**Ghidra mode:** read-only. No function creation, labels, renames, comments, or save operations.

## Working Notes

**Target question:** Can static evidence prove that retail Single Player dialog `0x100` Skirmish child button `0x579` reaches the `0x00612690 -> FUN_00608260` transition branch before dialog proc `0x0052D640` writes result `0x0B`?

**Non-goals:** Do not re-prove all shell buttons, all `FUN_006071E0` frame composition, all `0x00610CA0` messages, or Rust implementation. Do not claim framebuffer parity.

**Evidence needed to mark COMPLETE:** route-specific `0x579` command proof, subclass installer/classification proof, child thunk message gates around default Button processing, local `0x00612690` gates, and a smallest runtime-breakpoint plan for any values static evidence cannot decide.

**Stop conditions:** stop after route-specific reachability is classified; unresolved broader shell global or framebuffer questions go to Remaining Uncertainty.

## Summary

Static evidence does **not** prove that the standard retail `0x100` Skirmish click reaches `0x00612690 -> FUN_00608260` before the direct `0x0B` route completes.

What static evidence does prove:

- Dialog `0x100` Skirmish control `0x579` is a visible owner-draw Button with style `0x5000000B`, so it is eligible for the common shell child subclass dispatcher `0x00610CA0`.
- `FUN_0060F9A0` classifies `Button` controls with `(style & 0x0B) == 0x0B` as `OwnerDraw_Button_00612B70`, then installs dispatcher `0x00610CA0` as the child WndProc with `SetWindowLongA(hwnd, -4, 0x00610CA0)`.
- The dispatcher can call the previous Button WndProc and then write the per-control record field `+0x1FC = 1` on one child-message path.
- The actual `0x00612690` transition branch is later gated by paint-phase state: `[ESP+0x1F] != 0`, non-null record, final paint-count/global conditions, `record+0x1FC == 1`, `GetWindowLongA(hwnd, 4) != 0`, and then `FUN_00608260(parent_hwnd)`.
- The route result owner remains `0x0052D640`: parent `WM_COMMAND` low word `0x579` writes `0x0B` directly and returns.

The missing proof is route-specific ordering and live state: static disassembly does not show that the `0x579` activation message sequence sets the same control record to `+0x1FC == 1`, enters the `WM_PAINT`-phase transition branch, drains the paint-count gate, and calls `FUN_00608260` before `0x0052D640` receives `WM_COMMAND` and writes `0x0B`.

## Load-Bearing Verified Facts

1. **Dialog `0x100` Skirmish route remains direct.**  
   Active in YR: Yes.  
   Evidence: `0x0052D656..0x0052D663` calls common shell proc first; `0x0052D6DF..0x0052D6F7` masks `LOWORD(wParam)` and compares `0x579`; `0x0052D713..0x0052D720` writes `0x0B` and returns. No call to `FUN_00608260` exists in this direct branch.

2. **Control `0x579` is subclass-eligible but that only proves message-path plausibility.**  
   Active in YR: Yes for shell child messages.  
   Evidence: retail resource facts from `SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md` list control `0x579` as `Button #128`, title `GUI:Skirmish`, style `0x5000000B`; `0x0060FE78..0x0060FE82` checks `(style & 0x0B) == 0x0B` and selects `OwnerDraw_Button_00612B70`; `0x0060FF05..0x0060FF0D` installs `0x00610CA0` as WndProc.

3. **The child subclass dispatcher has a default/previous-WndProc path that can set `record+0x1FC = 1`.**  
   Active in YR: Conditional.  
   Evidence: `0x006120B3..0x006120BC` reads `record+0x1FC` and requires it to be `< 1`; `0x006120C2` sets local `EDI = 1`; `0x006120D9..0x006120E9` calls the previous WndProc through `CallWindowProcA` import; `0x006120F7` writes `[ESI+0x1FC] = 1` afterward.

4. **The `0x00612690` transition branch is paint/state gated, not a direct command handler.**  
   Active in YR: Conditional.  
   Evidence: `0x0061113C..0x0061114D` sets local byte `[ESP+0x1F] = 1` only for message `0x0F` (`WM_PAINT`) under the global byte gate; `0x006125C0..0x006125C6` requires that byte nonzero; `0x00612642` requires `record+0x1FC == 1`; `0x0061266F` writes `2`; `0x00612689..0x00612690` calls `FUN_00608260`; `0x00612699` writes `3` on success.

5. **The direct helper `FUN_00608260` calls `FUN_006071E0` in transition mode only when invoked by a caller such as `0x00612690`.**  
   Active in YR: Conditional.  
   Evidence: `0x0060833F` sets `DL = 1`; `0x00608341..0x00608343` calls `FUN_006071E0`. The known command route `0x0052D640` does not invoke this helper.

## Route-Specific Reachability Classification

| Predicate | Static result | Evidence | Active in YR |
|---|---|---|---|
| `0x579` child exists and is a shell owner-draw Button | Proven | Resource table plus `0x0060FE78..0x0060FF0D` | Yes |
| Child messages can enter dispatcher `0x00610CA0` | Proven for subclassed shell controls | `SetWindowLongA(hwnd, -4, 0x610ca0)` at `0x0060FF05..0x0060FF0D` | Yes |
| Dispatcher can call previous Button proc and set same record `+0x1FC = 1` | Proven conditionally | `0x006120B3..0x006120F7` | Conditional |
| `0x00612690` can call `FUN_00608260` when `record+0x1FC == 1` | Proven conditionally | `0x006125C0..0x00612699` | Conditional |
| The specific retail `0x579` click reaches `0x00612690` before parent `WM_COMMAND 0x579` writes `0x0B` | **Not statically proven** | requires live message order, paint byte, paint count/global state, same record identity, and dialog lifetime | Unchecked |

The important narrowing is that static evidence does contain a writer of `record+0x1FC = 1` inside the same dispatcher, but that is still insufficient for route-active proof. It is not enough to know the button can be subclassed and the dispatcher can set `+0x1FC`; the target scenario needs the exact click-time message sequence to enter the paint-phase branch before `0x0B` completes.

## Smallest Breakpoint Plan

Use a retail runtime debugger trace on standard Yuri's Revenge:

1. Break on `0x0052D713` and log when parent dialog proc writes result `0x0B`.  
   Log: parent HWND, `wParam`, `lParam`, current message, and timestamp/order.

2. Break on `0x00612690`.  
   Log: current child HWND from dispatcher arg `[ESP+0x380]`, message `[ESP+0x384]`, `wParam`, `lParam`, `record` pointer in `EDI`, `record+0x1FC`, `[ESP+0x1F]`, `DAT_00AC48DC`, and `GetWindowLongA(hwnd,4)` result.

3. Optional earlier breakpoint on `0x006120F7`.  
   Log: child HWND, message, record pointer `ESI`, and value written to `record+0x1FC`. This proves whether `0x579` itself, another sibling control, or the parent is the record that enters state `1`.

4. Optional helper breakpoint on `0x0060833F`.  
   Log: helper parent HWND in `ESI`/`ECX` context and whether this call follows the `0x579` button click or unrelated shell paint.

Closure rule: route-active native transition is proven only if the same click produces `0x00612690 -> FUN_00608260` for the `0x579` child/parent shell before `0x0052D713` writes `0x0B`. If `0x0052D713` fires first or no `0x00612690` hit occurs, the Rust bridge remains DRIFT for this route.

## Current Rust Scan

- `src/ui/single_player_shell/state.rs` defines `Skirmish0x579`, maps it to `SinglePlayerShellAction::Skirmish`, and returns route code `Some(0x0B)`.
- `src/app.rs` opens the Single Player shell through `open_single_player_shell`; `SinglePlayerShellAction::Skirmish` currently calls `enter_native_skirmish_from_single_player`, which flips immediately to Skirmish shell.
- `src/app_shell_transition.rs` still contains `start_main_menu_to_skirmish`, a bridge/DRIFT compositor path not wired to the `0x100` `0x579` route.
- `src/app_single_player_shell_render.rs` renders pressed Single Player owner-draw button state; it does not model native `record+0x1FC` or child paint-count transition state.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|
| `0x579` route result is direct `0x0B`; native transition helper is not required for route correctness. | Preserve `Skirmish0x579 -> Some(0x0B)` and keep Skirmish entry behind Single Player shell action even if visual animation is added. | `src/ui/single_player_shell/state.rs`, `src/app.rs` | Clicking Single Player shell Skirmish records route `0x579 -> 0x0B` before entering `0x102`. | `single_player_shell_skirmish_0x579_emits_route_0x0b_before_skirmish_setup` | Low for route identity; high if visual bridge rewires route semantics. |
| `0x00612690` is only conditionally reachable through subclass paint/state gates. | Any current smooth Single Player-to-Skirmish animation must remain labeled bridge/DRIFT until runtime trace proves route-active native helper invocation. | `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs`, `src/app.rs` | A bridge can animate from Single Player render target to Skirmish render target, but tests/docs do not claim `FUN_00608260` parity. | `single_player_skirmish_transition_remains_bridge_until_native_owner_verified` | High; easy to overstate a plausible static path as proven parity. |
| Dispatcher can set `record+0x1FC = 1`, but exact route ordering is runtime-dependent. | Do not model `record+0x1FC` as a deterministic route-state transition for `0x579` without runtime capture. | future shell transition metadata/state | Forced/synthetic native-helper tests may exercise `+0x1FC`, but ordinary `0x579` route tests must not require it. | `single_player_skirmish_route_does_not_require_record_1fc_transition_state` | Medium; mixing native state bytes with bridge timing can hide drift. |

## Negative Facts / Do Not Do

- Do not claim static evidence proves retail `0x579` click plays `FUN_00608260 -> FUN_006071E0`. Evidence: final route-specific predicate remains live message/order dependent; direct route write at `0x0052D713` is proven.
- Do not treat `0x00612690` as a `WM_COMMAND 0x579` handler. Evidence: its local branch requires paint-phase byte `[ESP+0x1F]` and `record+0x1FC == 1`; parent command handling is in `0x0052D640`.
- Do not say `OwnerDraw_Button_00612B70` owns the helper call. Evidence: the selected owner proc is installed as a callback, but the helper call site is in dispatcher `0x00610CA0..0x006128FE`.
- Do not resurrect main-menu-to-Skirmish direct transition as parity. Evidence: verified native route is `0x683 -> 1 -> 0x100 -> 0x579 -> 0x0B -> 0x102`.
- Do not make `record+0x1FC = 1` a required Rust route result state for `0x579`. Evidence: static path proves a conditional writer, not route-active ordering before `0x0B`.

## Remaining Uncertainty

- Whether the retail `0x579` click reaches `0x00612690` before `0x0052D713` writes `0x0B` remains unresolved and requires the breakpoint plan above.
- Which HWND/control record receives `record+0x1FC = 1` at `0x006120F7` during the exact Single Player Skirmish click is not statically tied to control `0x579`.
- Whether the `WM_PAINT` phase byte `[ESP+0x1F]`, `DAT_00AC48DC` paint count, and `GetWindowLongA(hwnd,4)` gate are all satisfied during that click is runtime-dependent.
- Exact framebuffer pixels for any native route-active transition remain deferred until the helper call is proven for the route.

## Stale-Doc Replacement Wording

Replace any wording that says "the `0x100` Skirmish click reaches `0x00612690 -> FUN_00608260`" with:

> The `0x100` Skirmish button `0x579` is subclass-eligible and the shell dispatcher contains a conditional path that can set `record+0x1FC = 1` and later call `FUN_00608260` from `0x00612690`, but static evidence does not prove that the retail `0x579` activation reaches that paint/state branch before dialog proc `0x0052D640` writes route result `0x0B`. Treat route-active transition playback as runtime-unchecked.

Suggested canonical doc for this replacement: `docs/research/SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`.

## Sources

- Read-only Ghidra assembly context: `0x0052D656`, `0x0052D6DF`, `0x0052D6F1`, `0x0052D713`, `0x0060FE78`, `0x0060FF05`, `0x0061113C`, `0x006120B3`, `0x006120E9`, `0x006120F7`, `0x006125C0`, `0x00612642`, `0x00612690`, `0x0060833F`, `0x00622CA6`.
- `docs/research/SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md`
- `docs/research/SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md`
- `docs/research/SINGLE_PLAYER_TO_SKIRMISH_FUN_006071E0_FLAGS_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`
- Rust read-only scan: `src/app.rs`, `src/app_shell_transition.rs`, `src/ui/single_player_shell/state.rs`, `src/app_single_player_shell_render.rs`, `src/render/shell_transition_pass.rs`.

## Status

COMPLETE for static route-specific reachability classification. Static evidence proves subclass plausibility and a conditional `record+0x1FC` writer, but does not prove the retail `0x579` click reaches `0x00612690 -> FUN_00608260` before `0x0B` completes. The exact remaining values/messages are enumerated in the breakpoint plan.
