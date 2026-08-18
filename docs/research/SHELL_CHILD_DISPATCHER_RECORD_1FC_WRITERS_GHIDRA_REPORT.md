# Shell Child Dispatcher Record +0x1FC Writers - Ghidra Research Report

**Date:** 2026-05-27  
**Target:** `SHELL_CHILD_DISPATCHER_RECORD_1FC_WRITERS`  
**Address(es):** `0x00610CA0..0x006128FE`, writer sites `0x0061191F`, `0x006120F7`, `0x006125EB`, `0x0061266F`, `0x00612699`, allocator/insert helper `FUN_00624530`, parent setup `FUN_0060f4b0`, common shell proc `FUN_00622b50`  
**Mode:** targeted `/re-swarm` slot, read-only Ghidra/raw-disassembly research.  
**Non-scope:** full semantic naming of every `0x00610CA0` message branch, full `FUN_006071E0` pixel schedule, runtime debugger capture, Rust/INI edits.

## Summary

The shell child dispatcher record field `record+0x1FC` is a paint/invalidation state byte-sized-in-meaning but stored as a dword. Static evidence found five direct field writes in the raw dispatcher body:

- `0x0061191F`: writes `0`, not `1`. This branch has `EDI = 0` and resets the field on `WM_SHOWWINDOW (0x18)` when `wParam == 0`.
- `0x006120F7`: writes `1`. This is the only static writer to state `1` found in the dispatcher slice. It occurs in the paint-phase descendant/child invalidation path when the target child record exists and its current `+0x1FC < 1`, after a `CallWindowProcA` dispatch.
- `0x006125EB`: writes `2` on a pre-transition paint-count path when `GetWindowLongA(hwnd, 4)` is nonzero and `DAT_00AC48DC > 1`.
- `0x0061266F`: writes `2` immediately before the conditional `FUN_00608260` call.
- `0x00612699`: writes `3` only after `FUN_00608260` returns success.

Meaningful initialization sets the field to `0`: `FUN_0060f4b0` builds a zeroed 0x80-dword local record template, then `FUN_00624530` allocates a `0x208`-byte hash entry, zeroes its 0x80-dword record body, and copies the template body into the entry. No setup initializer to `1` was found.

Key answer: there is **no static evidence** that the standard Single Player dialog `0x100` Skirmish child control `0x579` receives `record+0x1FC == 1` before the route result `0x0B` completes. Static evidence proves only that subclassed shell children can enter the writer path during paint/descendant invalidation. A runtime breakpoint/watchpoint is still required to prove the exact `0x579` click.

## Load-Bearing Verified Facts

1. `record+0x1FC` is zero-initialized by the shell record insertion path. `FUN_0060f4b0` zeroes a local 0x80-dword record template before calling `FUN_00624530`; `FUN_00624530` allocates `0x208` bytes, zeroes `ESI+4` for `0x80` dwords, then `MOVSD.REP` copies the template into the entry body.  
   Evidence: `FUN_0060f4b0` decompile; `FUN_00624530` decompile; assembly `0x0062454C..0x00624558`, `0x006245D3..0x006245DA`. Active in YR: Yes, through `FUN_00622b50` `WM_INITDIALOG`.

2. The early dispatcher write at `0x0061191F` is a reset to `0`, not a state-`1` writer. The branch starts with `XOR EDI, EDI` at `0x006118AB`, checks `ESI == 0x18`, requires `wParam == 0` via `[ESP+0x388] == EDI`, then writes `MOV [EBX+0x1FC], EDI`.  
   Evidence: assembly `0x006118AB..0x00611925`; byte-pattern hit `89 BB FC 01 00 00` only at `0x0061191F`. Active in YR: Conditional, shell child `WM_SHOWWINDOW`/hide path.

3. The only found state-`1` writer is `0x006120F7`, where `EDI` is set to `1` at `0x006120C2` after the target record's current `+0x1FC` is tested and found `< 1`; the code dispatches through `CallWindowProcA` and then writes `MOV [ESI+0x1FC], EDI`.  
   Evidence: assembly `0x0061208F..0x00612104`; byte-pattern hit `89 BE FC 01 00 00` only at `0x006120F7`; no direct immediate `C7 ?? +0x1FC, 1` matches were found. Active in YR: Conditional, paint/descendant invalidation path.

4. State `2` means "transition/invalidation in progress or consumed" in this slice: `0x006125EB` writes `2` when `GetWindowLongA(hwnd, 4)` is nonzero and `DAT_00AC48DC > 1`; `0x0061266F` writes `2` immediately before the `FUN_00608260` attempt.  
   Evidence: assembly `0x006125D3..0x00612600`, `0x00612642..0x00612690`; byte-pattern `C7 87 FC 01 00 00 02 00 00 00` hits only `0x006125EB` and `0x0061266F`. Active in YR: Conditional, paint/state branch.

5. State `3` means "transition helper succeeded" in this slice: `0x00612699` writes `3` only after `FUN_00608260` returns nonzero at `0x00612690..0x00612697`.  
   Evidence: assembly `0x00612679..0x006126A3`; byte-pattern `C7 87 FC 01 00 00 03 00 00 00` hits `0x00612699`. Active in YR: Conditional, only after successful helper call.

6. The dispatcher body has no other direct `+0x1FC` displacement hits besides reads/writes at `0x0061191F`, `0x006120A8`, `0x006120B3`, `0x006120F7`, `0x006125EB`, `0x00612642`, `0x0061266F`, and `0x00612699`.  
   Evidence: Ghidra byte-pattern search for `FC 01 00 00` and assembly context for the in-range hits. Active in YR: Search evidence, not a runtime path label.

## State Model For `record+0x1FC`

| Value | Verified local meaning | Writer(s) | Active in YR |
|---:|---|---|---|
| `0` | Initial/reset/not currently marked for this paint/invalidation transition path | zeroed template/entry body; `0x0061191F` hide/reset branch | Yes for initialization; conditional for reset |
| `1` | Marked child/descendant record after paint-phase `CallWindowProcA` when previous value was `< 1`; this is the gate value later required by `0x00612642` | `0x006120F7` | Conditional |
| `2` | Consumed/in-progress state around multi-pass paint or before attempting `FUN_00608260` | `0x006125EB`, `0x0061266F` | Conditional |
| `3` | Successful `FUN_00608260` state | `0x00612699` | Conditional |

This field should not be named as a generic "button pressed" state. It is specifically tied to shell paint/descendant invalidation and the transition helper gate.

## `0x579` Static Reachability Answer

Static evidence still does **not** prove that Single Player dialog `0x100` control `0x579` receives `record+0x1FC == 1` before its route completes.

What is proven:

- Dialog `0x100` control `0x579` is a visible `Button` child with owner-draw style and can be subclassed through the common shell setup. Active in YR: Yes, per prior `SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md` and `FUN_0060F9A0` setup reports.
- The common dispatcher contains a live conditional path that can set a child record's `+0x1FC` to `1` during paint/descendant invalidation. Active in YR: Conditional.
- The route action itself is direct: dialog proc `0x0052D640` writes result `0x0B` for command `0x579` and does not directly call `FUN_00608260`. Active in YR: Yes, per prior raw disassembly `0x0052D6F1..0x0052D720`.

What remains unproven:

- Whether the exact retail `0x579` HWND enters the `0x006120F7` writer path during the click.
- Whether that state remains `1` until the later `0x00612642` comparison before the direct `0x0B` route tears down or advances away from dialog `0x100`.
- Whether the parent/child paint count `DAT_00AC48DC` and `GetWindowLongA(hwnd, 4)` gates are satisfied for the exact click.

Runtime closure: set breakpoints on `0x006120F7`, `0x00612642`, `0x00612690`, and `0x0052D6F1..0x0052D720` while clicking retail Single Player `0x100` Skirmish `0x579`. Log `hwnd`, control id via `GetDlgCtrlID(hwnd)`, record pointer, `record+0x1FC`, `DAT_00AC48DC`, and return order. A hardware watchpoint on the specific `0x579` record `+0x1FC` after subclass setup would be stronger.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|
| `record+0x1FC` starts/reset as `0`; state `1` is conditional paint/descendant invalidation, not ordinary button activation. | `FUN_0060f4b0`; `FUN_00624530`; `0x0061191F`; `0x006120F7` | Rust has no native shell-record state model and should not claim one for route parity. | `src/app_shell_transition.rs`, future shell transition metadata | A visual bridge may exist, but route docs/UI must label it bridge/DRIFT until the exact `0x579` writer path is runtime-proven. | `single_player_skirmish_transition_record_1fc_unproven_bridge_only` | Medium: easy to overclaim native parity from generic helper machinery. |
| State `2` is written before/around helper consumption; state `3` only after successful `FUN_00608260`. | `0x006125EB`, `0x0061266F`, `0x00612699` | Current Rust transition completion is app-state driven, not native state byte driven. | `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs` | If a native-state model is added later, completion cannot flip to done before helper success equivalent. | `shell_transition_bridge_does_not_emit_native_state_3_without_success` | Low now, higher if native record parity is attempted. |
| `0x579 -> 0x0B` remains direct and independent from proving `record+0x1FC == 1`. | prior `0x0052D640` route report plus this writer scan | Rust already preserves `Skirmish0x579 -> Some(0x0B)`. | `src/ui/single_player_shell/state.rs`, `src/app.rs` | Adding animation must not replace or obscure the route result identity/order. | `single_player_skirmish_control_returns_0x0b_even_when_bridge_animates` | Medium: animation glue could accidentally shortcut native route semantics. |

## Negative Facts / Do Not Do

- Do not treat `0x0061191F` as the missing state-`1` writer. Evidence: `EDI` is zeroed at `0x006118AB`, and the branch writes `MOV [EBX+0x1FC], EDI`.
- Do not claim an immediate initializer sets `record+0x1FC = 1`. Evidence: setup zeroes the record body; byte-pattern searches for direct `C7 ?? FC 01 00 00 01 00 00 00` found no matches.
- Do not infer that every owner-draw shell button click triggers `FUN_00608260`. Evidence: the only state-`1` writer is inside a paint/descendant invalidation path, while `0x579 -> 0x0B` is direct in `0x0052D640`.
- Do not model `record+0x1FC` as gameplay or `sim/` state. Evidence: all observed writers are shell HWND/paint/USER32 paths.
- Do not mark Rust's whole-screen bridge as native `FUN_00608260` parity from this evidence. Evidence: route-active `0x579` reachability remains unproven.

## Remaining Uncertainty

- Exact runtime order for retail `0x579` click remains unknown: does `0x006120F7` hit for that child before `0x0052D640` writes `0x0B`?
- Exact per-child record pointer for `0x579` at click time is runtime data; static analysis proves lookup mechanics but not the live HWND/control instance.
- The broad dispatcher may have indirect caller/message combinations that affect whether the paint-count gates are satisfied, but no additional direct `+0x1FC` writer was found in the bounded dispatcher byte-pattern scan.

## Stale-Doc Replacement Wording

For `docs/research/SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md`, replace:

> Which caller/message first sets record+0x1FC to 1 for the 0x579 control in the target scenario? ... broad dispatcher has many message paths ...

with:

> The only static state-`1` writer found in the shell child dispatcher is `0x006120F7`, inside the paint/descendant invalidation path. It sets `record+0x1FC = 1` only when the target record's current state is `< 1`, after dispatching through `CallWindowProcA`. Static evidence still does not prove that the exact retail Single Player `0x100` Skirmish child `0x579` reaches this writer before `0x0052D640` writes route result `0x0B`; close this with a runtime breakpoint/watchpoint on the `0x579` record `+0x1FC`.

For `docs/research/SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`, append to the `Dataflow into shell per-control record +0x1FC` reinvestigation item:

> Static writer scan found `0x006120F7` as the only state-`1` writer and `0x0061191F` as a zero reset, but did not prove route-active reachability for control `0x579`.

## Sources

- Ghidra read-only byte-pattern search: `FC 01 00 00`; direct writer patterns `89 BE FC 01 00 00`, `89 BB FC 01 00 00`, `C7 87 FC 01 00 00 02 00 00 00`, `C7 87 FC 01 00 00 03 00 00 00`, and no-match direct immediate `1` patterns.
- Ghidra read-only assembly context: `0x006118AB..0x00611925`, `0x0061208F..0x00612104`, `0x006125D3..0x006126A3`.
- Ghidra read-only decompile: `FUN_0060f4b0`, `FUN_00622b50`, `FUN_00624530`.
- Prior docs: `docs/research/SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md`, `docs/research/SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`, `docs/research/skirmish-ui/SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `docs/research/SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md`.

## Status

COMPLETE for static writer/initializer inventory in the bounded dispatcher-record slice. PARTIAL only for the runtime target question of whether the exact retail `0x579` click reaches the `0x006120F7 -> record+0x1FC = 1` writer before route completion; static analysis cannot close that without live breakpoint/watchpoint capture.
