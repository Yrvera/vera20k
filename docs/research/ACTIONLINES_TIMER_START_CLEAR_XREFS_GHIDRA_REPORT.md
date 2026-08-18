# Action Lines Timer Start/Clear Xrefs - Ghidra Research Report

**Address(es):** `0x0070D150` (`ActionLines__StartTimer`), `0x006F2AB0` (`ActionLines__ClearTimer`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Global action-line timer start/clear/preserve behavior and immediate player-visible xrefs affecting selected-unit target-line visibility: click commands, bandbox, control-group recall, save/load scene reinitialization, and selection-only actions.
**Non-Scope:** Pixel raster style, endpoint selection, radar action lines, target acquisition, and per-unit order execution internals beyond confirming the timer call context.
**Confidence:** High for xrefs and timer writes; Medium for `ClearTimer` runtime reachability because Ghidra shows only a data xref.
**Active in YR:** Conditional. The timer is active in standard YR when selected-unit action lines are drawn, gated by `[Options] UnitActionLines` / render pass selection gates documented elsewhere. `ClearTimer` has no verified player-visible caller in this slice.

## 1. Overview

The selected-unit target-line timer is a two-global timer: `StartTimer` stores the current frame and a fixed 25-frame duration; `DrawActionLines` later expires lines by comparing the current frame against that start frame. Player-visible line disappearance is usually caused by either timer expiry or deselection removing the unit from the render gate, not by `ClearTimer`.

Ghidra reports exactly four direct code xrefs to `StartTimer`: three inside `DisplayClass__BandBox_LeftUp` and one at the end of `ControlGroup__Recall`. Ghidra reports no direct code xrefs to `ClearTimer`; only a data pointer at `0x00815078` references it.

## 2. Timer Globals / Key Offsets

| Address | Type | Writer | Purpose | Active in YR |
|---|---|---|---|---|
| `0x00B0EA80` | frame dword | `StartTimer`, `ClearTimer`, `FUN_00685120` | Timer start/reference frame | Yes; read by `DrawActionLines` |
| `0x00B0EA84` | dword | `StartTimer` only in decompiler output | Written from an uninitialized local in current decompilation; no timer role verified here | Conditional; side-effect exists but no visible consumer verified |
| `0x00B0EA88` | dword | `StartTimer`, `ClearTimer`, `FUN_00685120` | Remaining/duration frames | Yes; `0x19` means 25 frames |
| Object `+0x83` | byte | selection system | `IsSelected`; render gate must be true for selected-unit lines | Yes |

## 3. Core Logic

### `ActionLines__StartTimer` (`0x0070D150`)

Verified behavior:

- Writes `g_ActionLines_StartFrame = g_CurrentFrameCounter`.
- Writes `g_ActionLines_Duration = 0x19`.
- Also writes `_DAT_00B0EA84 = local_8` in the decompiler output. `local_8` is not initialized in this function, and this field is not required for the target-line expiry logic verified in this slot.
- Active in YR: Yes. Evidence: direct xrefs from `DisplayClass__BandBox_LeftUp` at `0x004ABCF0`, `0x004ABE83`, `0x004ABFAE`, and `ControlGroup__Recall` at `0x00731385`.

### `ActionLines__ClearTimer` (`0x006F2AB0`)

Verified behavior:

- Writes `g_ActionLines_StartFrame = g_CurrentFrameCounter`.
- Writes `g_ActionLines_Duration = 0`, which makes the normal draw-time remaining check immediately fail.
- Calls `FUN_007C978A(&LAB_006F2AD0)` after the writes; no player-visible action-line effect was verified from that callee in this slice.
- Active in YR: No verified player-visible caller. Evidence: `get_function_xrefs(0x006F2AB0)` returns only `From 00815078 [DATA]`; no direct code xrefs were found.

### Save/load scene preservation helper `FUN_00685120`

Verified behavior:

- If `g_ActionLines_StartFrame != -1`, compares `g_CurrentFrameCounter - g_ActionLines_StartFrame` against `g_ActionLines_Duration`.
- If elapsed is still less than duration, rewrites duration to the remaining frames.
- Otherwise rewrites duration to zero.
- Always rewrites `g_ActionLines_StartFrame = g_CurrentFrameCounter` afterward.
- Active in YR: Conditional. Evidence: `FUN_0067E440` calls `FUN_00685120` during loading-game reinitialization after storage/content load and before tactical/sidebar/radar refresh. This preserves remaining target-line time across that scene rebuild rather than restarting a full 25 frames.

## 4. Player-Visible Start Triggers

| Trigger | Timer behavior | Evidence | Active in YR |
|---|---|---|---|
| Bandbox drag-select release | Starts timer after `Tactical__ProcessBandBoxSelection` | `DisplayClass__BandBox_LeftUp` call at `0x004ABCF0` | Yes; standard selection path |
| Left-click object selection path | Starts timer after object select/type-select branch and display update | `DisplayClass__BandBox_LeftUp` call at `0x004ABE83` | Yes; standard click-selection path |
| Click command / target command dispatch | Starts timer after `Selection__DispatchMultiUnitOrder` | `DisplayClass__BandBox_LeftUp` call at `0x004ABFAE` | Yes; this is the main "click a selected unit somewhere" path |
| Control-group recall (`N`) | Starts timer at the end of recall, after selection loop | `ControlGroup__Recall` call at `0x00731385` | Yes; standard digit-key group recall |
| Double-tap group center | Does not restart timer on the center-camera path | `ControlGroup__Recall` returns through `ControlGroup__CenterCamera`; `ControlGroup__CenterCamera` has no `StartTimer` xref | Yes; preserves the first tap's timer if still active |
| Type-select / health-nav / veterancy-nav hotkeys | No direct `StartTimer` call verified | `get_function_xrefs(0x0070D150)` has only the four xrefs above; `FUN_00732280`, `FUN_00733380`, `FUN_007336C0` decompiled with no timer start | Yes, but selection-only hotkeys do not start target-line timer |

## 5. Selection Clearing vs Timer Clearing

`Unselect_All` (`0x006DA740`) does not call `ClearTimer` or `StartTimer`. It loops through `g_CurrentObjects` and calls each selected object's vtable `+0x150` deselect method, then calls `Selection__ResetMode`.

This matters because selected-unit action lines are render-gated by selection. A deselected unit stops drawing target lines immediately even if the global action-line timer still has remaining frames.

Active in YR: Yes. Evidence: `Unselect_All` xrefs include `ControlGroup__Recall` (`0x007312CB`), `ControlGroup__CenterCamera` (`0x00731419`), `FUN_00732280` (`0x0073247F`), health/veterancy selection flows (`0x007333EF`, `0x00733430`, `0x0073372F`, `0x00733770`), and observer/spectator transitions (`0x00637463`, `0x0063A4B9`).

## 6. Integration Points

- `DisplayClass__BandBox_LeftUp` is the central mouse-up path for both selection and command dispatch. It starts the timer for bandbox selection, click selection, and selected-unit commands.
- `Selection__DispatchMultiUnitOrder` does not start the timer itself; the caller starts it immediately after dispatch. Active in YR: Yes, evidence `0x004ABFAE` caller order.
- `ObjectClass__Select` does not start the timer. Active in YR: Yes, evidence `ObjectClass__Select` decompilation calls `Selection__ResetMode` but has no `StartTimer` call.
- `Selection__ResetMode` does not clear the action-line timer. Active in YR: Yes, evidence `0x00731D00` only writes `g_SelectionMode` and `g_SelectionSubMode`.
- `ClearTimer` is not the normal visibility-clear path for selected-unit lines in the verified player paths. Active in YR: No verified player-visible use, evidence xrefs to `0x006F2AB0`.

## 7. Current Rust Implementation Status

Not re-scanned in this slot. Prior target-line report says selected-unit target lines are not implemented, but this report's evidence is limited to gamemd.exe timer behavior and xrefs.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ActionLines__StartTimer` body | verified | Decompile `0x0070D150` | none |
| `ActionLines__StartTimer` xrefs | verified | Xrefs: `0x004ABCF0`, `0x004ABE83`, `0x004ABFAE`, `0x00731385` | none |
| `ActionLines__ClearTimer` body | verified | Decompile `0x006F2AB0` | none |
| `ActionLines__ClearTimer` xrefs | touched-not-exhausted | Xref result only `0x00815078 [DATA]` | exact ownership/use of data table at `0x00815078` |
| Bandbox selection timer start | verified | `DisplayClass__BandBox_LeftUp` after `Tactical__ProcessBandBoxSelection` | none |
| Click-selection timer start | verified | `DisplayClass__BandBox_LeftUp` select/type-select branch before call at `0x004ABE83` | action-code naming remains external |
| Command-dispatch timer start | verified | `DisplayClass__BandBox_LeftUp` calls `Selection__DispatchMultiUnitOrder`, then `StartTimer` at `0x004ABFAE` | none |
| Control-group recall timer start | verified | `ControlGroup__Recall` at `0x00731385` | none |
| Double-tap center preserves/no restart | verified | `ControlGroup__CenterCamera` decompiled; no `StartTimer`; invoked by recall early return | none |
| Save/load scene timer preservation | verified | `FUN_00685120`, caller `FUN_0067E440` | only save/load scene covered |
| Type-select/health/veterancy selection-only hotkeys | verified | No `StartTimer` xrefs; decompiled `0x00732280`, `0x00733380`, `0x007336C0` | command-class wrapper names not resolved |
| Raw `ObjectClass__Select` | verified | Decompile `0x005F4520`; calls `Selection__ResetMode`, no `StartTimer` | none |
| `Unselect_All` visual clear by deselection | verified | Decompile `0x006DA740`; no timer write | none |

## 9. Open Questions - Final State

[RESOLVED] OQ-1 - What does `StartTimer` write? It writes current frame to `0x00B0EA80` and `0x19` to `0x00B0EA88`; decompiler also shows an uninitialized write to `0x00B0EA84`. Evidence: `0x0070D150`.

[RESOLVED] OQ-2 - Which code paths start the timer? Exactly four direct code xrefs were found: three in `DisplayClass__BandBox_LeftUp`, one in `ControlGroup__Recall`. Evidence: xrefs to `0x0070D150`.

[RESOLVED] OQ-3 - Does a click command start the timer? Yes, after `Selection__DispatchMultiUnitOrder`. Evidence: `DisplayClass__BandBox_LeftUp` call at `0x004ABFAE`.

[RESOLVED] OQ-4 - Does bandbox selection start the timer? Yes, immediately after `Tactical__ProcessBandBoxSelection`. Evidence: `DisplayClass__BandBox_LeftUp` call at `0x004ABCF0`.

[RESOLVED] OQ-5 - Does group recall start the timer? Yes, after the recall selection loop. Evidence: `ControlGroup__Recall` call at `0x00731385`.

[RESOLVED] OQ-6 - Does double-tap group center restart the timer? No verified restart; `ControlGroup__CenterCamera` has no `StartTimer` call and `ControlGroup__Recall` returns early into it. Evidence: `0x007313A0`, `0x007312A8`.

[RESOLVED] OQ-7 - Does `ClearTimer` clear visible lines in the verified player paths? No direct player-visible caller was found. Evidence: xrefs to `0x006F2AB0` show only `0x00815078 [DATA]`.

[RESOLVED] OQ-8 - How are lines preserved across scene rebuild/load? `FUN_00685120` converts the timer duration to remaining frames or zero, then resets the start frame to current. Evidence: `0x00685120`, caller `0x0067E440`.

[RESOLVED] OQ-9 - Do selection-only hotkey flows start target-line timer? Not the verified type-select, health-nav, or veterancy-nav functions. Evidence: no `StartTimer` xrefs beyond the four verified; decompiled `0x00732280`, `0x00733380`, `0x007336C0`.

[DEFERRED] OQ-10 - What owns the data table at `0x00815078` containing the `ClearTimer` pointer? Category: bounded-cost-too-high. Reason: current slot is xrefs/player-visible behavior; no code xref to the data table was found with available tools.

## Sources

- Ghidra decompile: `ActionLines__StartTimer` `0x0070D150`
- Ghidra decompile: `ActionLines__ClearTimer` `0x006F2AB0`
- Ghidra xrefs: `0x0070D150`, `0x006F2AB0`, `0x00685120`, `0x006DA740`
- Ghidra decompile: `DisplayClass__BandBox_LeftUp` `0x004ABCF0` function body
- Ghidra decompile: `ControlGroup__Recall` `0x007311C0`
- Ghidra decompile: `ControlGroup__CenterCamera` `0x007313A0`
- Ghidra decompile: `Selection__DispatchMultiUnitOrder` `0x004AE750`
- Ghidra decompile: `Unselect_All` `0x006DA740`
- Ghidra decompile: `Selection__ResetMode` `0x00731D00`
- Ghidra decompile: `ObjectClass__Select` `0x005F4520`
- Ghidra decompile: `FUN_00685120`, `FUN_0067E440`
- Ghidra decompile: `FUN_00732280`, `FUN_00733380`, `FUN_007336C0`
- Starting docs only: `TARGET_LINES_GHIDRA_REPORT.md`, `SELECTION_SYSTEM_GHIDRA_REPORT.md`
