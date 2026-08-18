# Main Tick Late Frame-Increment Gate Globals - Reswarm 2026-05-28

**Address(es):** `Main_Tick @ 0x0055D360`, late gate `0x0055DE4F..0x0055DE81`, session handler `FUN_0055CFD0`, writers `HouseClass::Update @ 0x004F8600`, `State_Machine @ 0x0048C8B0`, `EventClass::Execute @ 0x004C7600`, defaults in `Main_Game @ 0x0052D9C0`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** writers/defaults and branch semantics for the four late `Main_Tick` byte globals `DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, and `DAT_00A83D48`; pause/menu/replay/session reachability only as needed for native frame-clock implementation.
**Non-Scope:** full native wall-clock pacing; complete replay packet semantics; exact victory/defeat screen UI; event construction paths beyond the `EXIT` event writer; native pending-delete queue contents during teardown; Rust implementation.
**Confidence:** High for the four globals, default clears, writer identities, late-gate branch matrix, and standard/replay/pause reachability; Medium for Rust deltas because this report scans surfaces but does not implement them.
**Active in YR:** Conditional. The `Main_Tick` gate is on the active standard YR tick path; each byte is active only when its session-end condition is reached.

## 0. Investigation Contract

**Target question:** What writes/defaults the late frame-increment gate globals in `Main_Tick`, and exactly how do those globals gate the late native frame increment for pause/menu/replay/session-end cases?

**Non-goals:** Do not re-prove the already settled tick spine, pre-increment visibility, or current Rust early `binary_frame` mismatch except where the four globals affect the frame clock. Do not implement Rust. Do not expand into full replay, pending-delete, victory/defeat UI, or wall-clock pacing.

**Evidence needed to mark COMPLETE:** `Main_Tick` decompile plus assembly at `0x0055DE4F..0x0055DE81`; static xrefs and writer/default proof for all four globals; decompile evidence from writer/default/handler functions; active-YR classification for pause/menu/replay/session reachability; implementation handoff with concrete Rust test proposals.

**Stop conditions:** Stop after all four globals are resolved or explicitly deferred, a zero-add pass over the late gate/replay/pause contexts adds no material question, and this report is written to `docs/research/MAIN_TICK_LATE_FRAME_INCREMENT_GATE_GLOBALS_RESWARM_20260528.md`.

## 1. Overview

The four late `Main_Tick` globals are not generic pause, modal, or replay freeze switches. They are session-end bytes: local/session victory, local/session defeat, user-confirmed quit-to-main, and graceful disconnect/`EXIT` event.

`Main_Tick` reads them after `Network_ServiceLoop`. If any byte is nonzero, it branches to the late return path and skips `g_CurrentFrameCounter++`, the post-increment wait helper, the pending-delete drain, and one additional late service helper. Ordinary pause/menu state does not set these bytes by itself. Replay playback (`DAT_00A8D5F8 & 2`) skips the normal gameplay/render block, then still continues through replay bookkeeping, pre-object order helper, `LogicClass::PerTickUpdate`, `Network_ServiceLoop`, and the same four-byte late increment gate.

## 2. Key Globals

| Address | Verified semantic name | Type | Set-to-1 writers | Default / clears | Late-gate read | Active in YR |
|---:|---|---|---|---|---|---|
| `0x00A83D49` | local/session victory flag | byte | `HouseClass::Update @ 0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F87BB` | session start clear `Main_Game @ 0x0052DA78`; session handler clears on all routes | `Main_Tick @ 0x0055DE4F` | Conditional: Yes when local/session victory is reached |
| `0x00A8ECD0` | local/session defeat flag | byte | `HouseClass::Update @ 0x004F86F7`, `0x004F879C`, `0x004F87B2` | session start clear `Main_Game @ 0x0052DA7F`; session handler clears on all routes | `Main_Tick @ 0x0055DE58` | Conditional: Yes when local/session defeat is reached |
| `0x008B41C0` | user-confirmed quit-to-main flag | byte | `State_Machine @ 0x0048CB2E`, `g_GameState == 3`, dialog result case `5` | session start clear `Main_Game @ 0x0052DA86`; session handler clears on all routes | `Main_Tick @ 0x0055DE61` | Conditional: Yes when user confirms quit |
| `0x00A83D48` | graceful disconnect / `EXIT` event flag | byte | `EventClass::Execute @ 0x004C7917`, event case `0x13` | session start clear `Main_Game @ 0x0052DA8D`; session handler clears on all routes | `Main_Tick @ 0x0055DE6A` | Conditional: Yes on network/exit event path |

No INI-backed writer was found for any of the four bytes. They are hardcoded engine/session state.

## 3. Core Logic

### 3.1 Late `Main_Tick` Gate

Read-only Ghidra decompile of `Main_Tick @ 0x0055D360` shows this late sequence after `LogicClassPerTickUpdateLiveVector`, frame-time accumulation, `FUN_00647260`, `FUN_00637550`, and `Network_ServiceLoop`:

```text
Network_ServiceLoop();
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter = g_CurrentFrameCounter + 1
    if DAT_00B07784 != 0 and DAT_00B07784 < g_CurrentFrameCounter:
        FUN_00684290()
        DAT_00B07784 = 0
    FUN_0055E160()
    FUN_00725C70()
    FUN_00637270()
    DAT_00ABCD58 = 0
    return g_GameActive == 0
else:
    DAT_00ABCD58 = 0
    return 1
```

Assembly context confirms byte reads and branch semantics:

| Address | Instruction | Effect | Active in YR |
|---:|---|---|---|
| `0x0055DE4A` | `CALL 0x0048D080` | `Network_ServiceLoop` runs before the gate | Yes |
| `0x0055DE4F` | `MOV AL,[0x00A83D49]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | any nonzero victory byte skips increment tail | Conditional |
| `0x0055DE58` | `MOV AL,[0x00A8ECD0]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | any nonzero defeat byte skips increment tail | Conditional |
| `0x0055DE61` | `MOV AL,[0x008B41C0]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | any nonzero quit byte skips increment tail | Conditional |
| `0x0055DE6A` | `MOV AL,[0x00A83D48]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | any nonzero disconnect byte skips increment tail | Conditional |
| `0x0055DE73..0x0055DE81` | read `0x00A8ED84`, `INC EDX`, write `0x00A8ED84` | native frame increment only when all four bytes are zero | Yes unless gated |
| `0x0055DE9A` | `CALL 0x0055E160` | wait/throttle helper only on all-zero path | Yes unless gated |
| `0x0055DE9F` | `CALL 0x00725C70` | pending-delete drain only on all-zero path | Yes unless gated |
| `0x0055DEA4` | `CALL 0x00637270` | late helper only on all-zero path | Yes unless gated |
| `0x0055DEC8` | writes `DAT_00ABCD58 = 0`, then returns `1` | skip target for any nonzero byte | Conditional |

Tiny detail: the gate is not checked immediately when a session-end byte is set. If `HouseClass::Update` sets victory/defeat during `LogicClass::PerTickUpdate`, the remaining post-PerTick late service work still runs up through `Network_ServiceLoop`, then the frame increment is skipped at `0x0055DE4F..0x0055DE71`.

### 3.2 Defaults and Clears

`Main_Game @ 0x0052D9C0` resets the frame counter and clears all four bytes at session setup:

| Address | Instruction | Meaning | Active in YR |
|---:|---|---|---|
| `0x0052DA78` | `MOV byte ptr [0x00A83D49],0x0` | victory flag default clear | Yes, session setup |
| `0x0052DA7F` | `MOV byte ptr [0x00A8ECD0],0x0` | defeat flag default clear | Yes, session setup |
| `0x0052DA86` | `MOV byte ptr [0x008B41C0],0x0` | quit flag default clear | Yes, session setup |
| `0x0052DA8D` | `MOV byte ptr [0x00A83D48],0x0` | disconnect flag default clear | Yes, session setup |

`FUN_0055CFD0 @ 0x0055CFD0` is called from `Main_Game` after `Main_Tick`/`State_Machine`. It reads the same four bytes, routes the session-end condition, and clears all four on each handled path. Static caller evidence: `get_function_callers 0x0055CFD0` returns only `Main_Game @ 0x0048CCC0`.

Clear groups confirmed by assembly:

| Route | Clear group | Routed work | Active in YR |
|---|---|---|---|
| defeat | `0x0055D123..0x0055D135` | `FUN_00685DC0` after disconnect/lobby prep when needed | Conditional: defeat |
| victory | `0x0055D1B1..0x0055D1C3` | `FUN_00685670` after disconnect/lobby prep when needed | Conditional: victory |
| quit-to-main | `0x0055D219..0x0055D22B` | `FUN_006863E0` | Conditional: user quit |
| graceful disconnect | `0x0055D25C..0x0055D26E` | `GameExit__BattleControlTerminated`, "Disconnect Gracefully" logging, network cleanup | Conditional: network/exit |

Tiny detail: if multiple bytes are set, `FUN_0055CFD0` prioritizes victory/defeat handling before quit/disconnect according to its nested branches, not a flat first-set address order.

### 3.3 Writer Semantics

`0x00A83D49` is set by `HouseClass::Update` from win/loss result handling. Decompile shows `House+0x1F7` (win-like result byte) and inverse opponent-loss handling can set this global. Assembly confirms set-to-1 writes at `0x004F867C`, `0x004F8692`, `0x004F86EE`, and `0x004F87BB`. Active in YR: Conditional, reached through standard house update when victory/session-win conditions resolve.

`0x00A8ECD0` is set by `HouseClass::Update` from defeat result handling. Decompile shows `House+0x1F8` (loss-like result byte) and inverse win handling can set this global. Assembly confirms set-to-1 writes at `0x004F86F7`, `0x004F879C`, and `0x004F87B2`. Active in YR: Conditional, reached through standard house update when defeat/session-loss conditions resolve.

`0x008B41C0` is set by `State_Machine`. Decompile of `State_Machine @ 0x0048C8B0` shows `g_GameState == 3` calls `FUN_004F1840`; nested result case `5` writes `DAT_008B41C0 = 1`. Assembly confirms `MOV byte ptr [0x008B41C0],0x1` at `0x0048CB2E`. Active in YR: Conditional, when the in-game quit/exit-confirm dialog returns case `5`.

`0x00A83D48` is set by `EventClass::Execute`. Decompile of `EventClass::Execute @ 0x004C7600` shows event case `0x13` logs "Processing EXIT event on frame" with `g_CurrentFrameCounter`, then writes `DAT_00A83D48 = 1` and returns. Assembly confirms `MOV byte ptr [0x00A83D48],0x1` at `0x004C7917`. Active in YR: Conditional, network/exit event path.

## 4. Branch Matrix for Native Frame Clock

| Case | Gate source | Normal gameplay/render block? | Replay block? | `LogicClass::PerTickUpdate`? | Late four-byte gate reached? | Frame increments? | Active in YR |
|---|---|---:|---:|---:|---:|---:|---|
| Standard active gameplay | `DAT_00A8D5F8 & 2 == 0`, `g_GameState == 0`, `g_GameRunning != 0`, four bytes zero | Yes | no unless `&1` record | Yes | Yes | Yes | Yes |
| Ordinary pause/menu modal | `g_GameState != 0`, four bytes zero | No | no unless replay flag also set | Yes | Yes | Yes | Conditional: menu/pause |
| Quit confirmed from menu | `State_Machine` later sets `0x008B41C0 = 1` | prior `Main_Tick` with modal skips gameplay block | No | prior `Main_Tick` still runs PerTick; later set byte affects next late gate reached | Yes once byte set before/within tick | No on gated tick | Conditional: user confirms quit |
| Replay recording | `DAT_00A8D5F8 & 1`, four bytes zero | Usually Yes if not also playback/pause-gated | record block runs | Yes | Yes | Yes | Conditional: recording |
| Replay playback | `DAT_00A8D5F8 & 2`, four bytes zero | No | playback read/render block runs | Yes | Yes | Yes | Conditional: replay playback |
| Scenario intro/display-only | `ScenarioClass+0x62C != 0` in early branch | No standard block | No | No | No | No | Conditional: scenario intro/display-only |
| Victory/defeat set during `HouseClass::Update` | `0x00A83D49` or `0x00A8ECD0` nonzero before late gate | already past normal block for that tick | possible if replay flags set | Yes, the writer can be inside PerTick tail | Yes | No | Conditional: session end |
| Graceful disconnect/EXIT | `0x00A83D48` nonzero | depends when event processed | depends | depends; event execution path sets byte | Yes after set | No | Conditional: network/exit |
| App shutdown inactive | `g_GameActive == 0` at top | No | No | No | No | No | Conditional: application/session shutdown |

Replay clarification: in `Main_Tick`, playback render at `0x0055DBBE` is followed by `MOV ECX,0x8A0390; CALL 0x00551A30`, then the function can reach `LogicClass::PerTickUpdate @ 0x0055DC9E` and the late gate. This corrects older "replay render-only returns" wording.

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Delta for this slice |
|---|---|---|
| `src/sim/world/mod.rs::advance_tick` | updates `total_sim_ms` and `binary_frame` at tick start, before command/subsystem work | no late native frame commit or four-session-byte suppressor |
| `src/app_sim_tick.rs` pause path | `run_sim` is false when `state.paused` unless debug frame-step is requested | native modal/pause is not equivalent to "stop all sim"; the late frame gate is independent of ordinary pause |
| app/session end state | current Rust has app-level state and victory/defeat handling, but no native-equivalent four-byte stop reason feeding frame-clock commit | missing typed session-end gate for victory, defeat, quit-confirm, disconnect/EXIT |
| `src/sim/replay.rs` | Rust replay applies commands/hashes to an existing simulation | native replay playback is flag-driven by `DAT_00A8D5F8 & 2`, skips the normal gameplay/render block, but still reaches PerTick and late frame gate |
| `src/util/fixed_math.rs` comments | comments still say every 15 Hz sim tick equals one RA2 game frame, while `SIM_TICK_HZ` is 45 | stale wording; frame identity is not a fixed-step comment, it is a late-commit contract with session-end gate |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Tick` late four-byte gate | verified | decompile `0x0055D360`; assembly `0x0055DE4A..0x0055DEA4` | none |
| all static xrefs for four gate bytes | verified | `get_xrefs_to` for `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48` | runtime races with multiple flags not sampled |
| session-start defaults | verified | `Main_Game @ 0x0052D9C0`; assembly `0x0052DA78..0x0052DA8D` | none |
| `FUN_0055CFD0` reader/router/clearer | verified | decompile `0x0055CFD0`; caller `Main_Game`; clear-group assembly | exact victory/defeat screen rendering out-of-scope |
| `0x00A83D49` writers/semantics | verified | `HouseClass::Update @ 0x004F8600`; writer assembly and handler route | exact upstream field setters for `House+0x1F7` out-of-scope |
| `0x00A8ECD0` writers/semantics | verified | `HouseClass::Update @ 0x004F8600`; writer assembly and handler route | exact upstream field setters for `House+0x1F8` out-of-scope |
| `0x008B41C0` writer/semantics | verified | `State_Machine @ 0x0048C8B0`; write `0x0048CB2E`; handler route | exact `FUN_004F1840` dialog internals beyond result case `5` out-of-scope |
| `0x00A83D48` writer/semantics | verified | `EventClass::Execute @ 0x004C7600`; case `0x13`; write `0x004C7917`; handler route | all event construction sites for case `0x13` out-of-scope |
| ordinary pause/menu versus four bytes | verified for this slice | `Main_Tick` `g_GameState` gate at `0x0055D878..0x0055D897`; no writer except quit-confirm path | per-system pause-visible behavior outside this frame-clock slice |
| replay playback reachability to late gate | verified | `Main_Tick` decompile; assembly `0x0055DBBE..0x0055DC9E`; replay report | full replay packet/event semantics out-of-scope |
| scenario intro/display-only early return | verified for reachability | `Main_Tick` decompile; render/wait/return at `0x0055D84F..0x0055D877` | full scenario flag lifecycle out-of-scope |
| current Rust frame/pause surfaces | touched-not-exhausted | source scan of `world/mod.rs`, `app_sim_tick.rs`, `app_types.rs`, `fixed_math.rs`, `replay.rs` | exact implementation design and tests |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-LFIG-001 - Which bytes does `Main_Tick` read for the late increment gate? -> `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48`.` (evidence: `Main_Tick @ 0x0055D360`; assembly `0x0055DE4F..0x0055DE71`)
- `[RESOLVED] OQ-LFIG-002 - Is the gate zero-only or equality-specific? -> Each byte is loaded into `AL`, tested against zero, and any nonzero value jumps to the skip path.` (evidence: assembly `0x0055DE4F..0x0055DE71`)
- `[RESOLVED] OQ-LFIG-003 - What does the skip path bypass? -> `g_CurrentFrameCounter++`, optional `FUN_00684290`, `FUN_0055E160`, `FUN_00725C70`, and `FUN_00637270`.` (evidence: `0x0055DE73..0x0055DEA4`, skip target `0x0055DEC8`)
- `[RESOLVED] OQ-LFIG-004 - Does `Network_ServiceLoop` still run before a skip? -> Yes, call at `0x0055DE4A` precedes all four byte reads.` (evidence: assembly context `0x0055DE40..0x0055DE4F`)
- `[RESOLVED] OQ-LFIG-005 - Where are the bytes defaulted? -> `Main_Game` clears all four at session setup.` (evidence: decompile `0x0052D9C0`; assembly `0x0052DA78..0x0052DA8D`)
- `[RESOLVED] OQ-LFIG-006 - Who sets `0x00A83D49`? -> `HouseClass::Update` set-to-1 writes at `0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F87BB`.` (evidence: xrefs; decompile `0x004F8600`; writer assembly)
- `[RESOLVED] OQ-LFIG-007 - What does `0x00A83D49` mean? -> Local/session victory; handler routes it to `FUN_00685670`.` (evidence: `HouseClass::Update`; `FUN_0055CFD0`)
- `[RESOLVED] OQ-LFIG-008 - Who sets `0x00A8ECD0`? -> `HouseClass::Update` set-to-1 writes at `0x004F86F7`, `0x004F879C`, `0x004F87B2`.` (evidence: xrefs; decompile `0x004F8600`; writer assembly)
- `[RESOLVED] OQ-LFIG-009 - What does `0x00A8ECD0` mean? -> Local/session defeat; handler routes it to `FUN_00685DC0`.` (evidence: `HouseClass::Update`; `FUN_0055CFD0`)
- `[RESOLVED] OQ-LFIG-010 - Who sets `0x008B41C0`? -> `State_Machine` writes it in `g_GameState == 3`, dialog result case `5`.` (evidence: decompile `0x0048C8B0`; assembly `0x0048CB2E`)
- `[RESOLVED] OQ-LFIG-011 - What does `0x008B41C0` mean? -> User-confirmed quit-to-main/session abort route.` (evidence: `State_Machine`; `FUN_0055CFD0` route to `FUN_006863E0`)
- `[RESOLVED] OQ-LFIG-012 - Who sets `0x00A83D48`? -> `EventClass::Execute` case `0x13` writes it after logging the EXIT event frame.` (evidence: decompile `0x004C7600`; assembly `0x004C7917`)
- `[RESOLVED] OQ-LFIG-013 - What does `0x00A83D48` mean? -> Graceful disconnect / EXIT event route.` (evidence: `EventClass::Execute`; `FUN_0055CFD0` route through `GameExit__BattleControlTerminated`)
- `[RESOLVED] OQ-LFIG-014 - Does ordinary pause/menu set these bytes? -> No, not by itself; `g_GameState != 0` skips the normal gameplay/render block, but the late four-byte gate remains independent. The quit-confirm subpath can set `0x008B41C0`.` (evidence: `Main_Tick`, `State_Machine`)
- `[RESOLVED] OQ-LFIG-015 - Does replay playback return before the late gate? -> No; playback render at `0x0055DBBE` is followed by `FUN_00551A30`, `LogicClass::PerTickUpdate`, service work, and the late gate.` (evidence: `Main_Tick`; assembly `0x0055DBBE..0x0055DC9E`; `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`)
- `[RESOLVED] OQ-LFIG-016 - Does scenario intro/display-only reach the late four-byte gate? -> No; the `ScenarioClass+0x62C` branch renders, calls `FUN_0055E160`, clears `DAT_00ABCD58`, and returns before PerTick and frame increment.` (evidence: `Main_Tick`; assembly `0x0055D84F..0x0055D877`)
- `[RESOLVED] OQ-LFIG-017 - Is `g_GameMode == 5` replay playback for this gate? -> No; recent replay report verifies replay playback is `DAT_00A8D5F8 & 2`; `g_GameMode == 5` is the skirmish-style branch.` (evidence: `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`; `Main_Tick`)
- `[DEFERRED] OQ-LFIG-018 - What exact UI/file path sets `DAT_00A8D5F8 & 4` before replay playback?` (category: `out-of-scope`; reason: this slice only needs replay reachability to the late gate; next-step-if-pursued: trace all xrefs to `0x00A8D5F8`)
- `[DEFERRED] OQ-LFIG-019 - What are all upstream writers to `House+0x1F7` and `House+0x1F8`?` (category: `requires-different-system-context`; reason: this slice maps gate globals, not the full victory/defeat condition system; next-step-if-pursued: investigate house result-byte lifecycle)
- `[DEFERRED] OQ-LFIG-020 - What are all constructors/enqueuers for event case `0x13`?` (category: `out-of-scope`; reason: direct writer to `DAT_00A83D48` is proven; event production taxonomy is separate; next-step-if-pursued: trace event construction for ABOUTTOEXIT/EXIT)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native commits `g_CurrentFrameCounter++` late only when victory, defeat, quit-confirm, and disconnect/EXIT bytes are all zero. Active in YR: Yes, conditional on no session-end byte. | `Main_Tick @ 0x0055DE4F..0x0055DE81`; xrefs/writers for all four bytes | missing: Rust computes `binary_frame` at tick start and has no typed late session-end suppressor | `src/sim/world/mod.rs::advance_tick`; future native frame-clock service | frame-clock commit must be a late action, and must be skipped when the native-equivalent stop reason is victory/defeat/quit/disconnect | proposed test `native_frame_commit_skips_on_session_end_gate`: set each stop reason before late commit; visible native frame remains `N` and late post-increment hooks are not called | Do not model this with a generic `paused` boolean or per-site frame subtraction |
| Ordinary pause/menu is not one of the four late increment gates. Active in YR: Conditional, menu/pause. | `Main_Tick` gameplay-block branch `0x0055D878..0x0055D897`; `State_Machine @ 0x0048C8B0`; no four-byte writer except quit result case `5` | mismatch: `src/app_sim_tick.rs` stops fixed sim while paused | app/sim scheduling boundary; native pause state model | separate gameplay/render block suppression from late frame/per-tick clock behavior if pause parity is targeted | proposed test `menu_pause_does_not_set_late_frame_gate`: enter pause/menu without confirming quit; native-equivalent late frame commit is still eligible when the four bytes are zero | Do not let `state.paused` stand in for the session-end bytes |
| Replay playback skips the normal gameplay/render block but still reaches `LogicClass::PerTickUpdate` and the late frame gate. Active in YR: Conditional, `DAT_00A8D5F8 & 2`. | `Main_Tick` playback render `0x0055DBBE`, post-render `FUN_00551A30`, `PerTickUpdate @ 0x0055DC9E`, gate `0x0055DE4F..` | current Rust replay runner is a command/hash aid over an existing sim and has no native replay main-tick branch | `src/sim/replay.rs`; app replay playback loop; future frame-clock branch matrix | replay playback mode should not be treated as a no-PerTick/no-frame-commit render-only path | proposed test `replay_playback_reaches_late_frame_gate`: with playback flag and all four bytes zero, one tick runs replay render/bookkeeping and commits frame late | Do not implement replay playback as an early-return render-only loop; do not treat `g_GameMode == 5` as replay playback |
| Scenario intro/display-only branch returns before PerTick and frame increment. Active in YR: Conditional, `ScenarioClass+0x62C != 0`. | `Main_Tick` branch renders at `0x0055D84F`, calls `FUN_0055E160`, returns at `0x0055D877`; decompile branch on `ScenarioClass+0x62C` | unchecked/missing: Rust scenario intro/display-only branch not modeled against native frame clock | future scenario intro/cinematic tick path | display-only frames must not advance native gameplay frame or PerTick systems | proposed test `scenario_display_only_tick_does_not_commit_native_frame`: with display-only flag set, render/wait path runs but frame remains `N` | Do not reuse the ordinary late four-byte gate for this early return; it never reaches the gate |
| The four bytes are hardcoded session lifecycle state, default-cleared in `Main_Game`, not INI settings. Active in YR: Yes at session setup. | `Main_Game @ 0x0052DA78..0x0052DA8D`; no INI/xref writer beyond listed engine paths | missing: no explicit native stop-reason bytes in Rust state | app/session state; save/load/replay state if native parity expands there | represent four distinct stop reasons or an equivalent typed enum that can reproduce branch priority and suppressions | proposed test `session_end_gate_defaults_zero_on_new_session`: new session starts with all four native stop gates clear | Do not add INI or user option plumbing for these bytes |

## 9. Negative Facts / Do Not Do

- Do not call `DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, or `DAT_00A83D48` ordinary pause flags. Active in YR: No for ordinary pause; evidence: writer xrefs and `State_Machine`.
- Do not model the late increment gate as "session-freeze because menu is open." Active in YR: No; `g_GameState` only gates the earlier gameplay/render block, while the late gate reads the four session-end bytes.
- Do not implement replay playback as returning immediately after replay render. Active in YR: No; evidence: `0x0055DBBE` continues to `FUN_00551A30`, `PerTickUpdate`, and the late gate.
- Do not treat `g_GameMode == 5` as replay playback. Active in YR: No for replay; recent replay report verifies `g_GameMode == 5` is skirmish-style and playback is `DAT_00A8D5F8 & 2`.
- Do not hardcode these flags from INI or config. Active in YR: No INI path found; defaults/writers are engine/session code.

## 10. Stale Docs / Replacement Wording

- `docs/research/timing/logic-vs-render-loop.md`: replace the replay row "Reads recorded inputs, renders, returns" with: "Replay playback (`DAT_00A8D5F8 & 2`) skips the normal gameplay/render block, reads replay sync/selection/cursor data, calls `RenderFrame_main`, then continues through `FUN_00551A30`, `LogicClass::PerTickUpdate`, late service work, and the ordinary four-byte frame-increment gate."
- `docs/research/timing/logic-vs-render-loop.md`: replace "_DAT_00a8d5f8 & 2 ... Forces an early return through the render-only sub-path" with: "`ScenarioClass+0x62C` is the display-only early return. Replay playback uses `DAT_00A8D5F8 & 2` to skip the normal gameplay/render block and run replay render/bookkeeping, but it does not return before the late PerTick/frame-gate path."
- `docs/research/timing/logic-vs-render-loop.md` and `docs/research/timing/multiplayer-frame-step.md`: replace "`g_GameMode == 5` (replay)" wording with: "`g_GameMode == 5` is the skirmish-style branch in verified `Main_Game`/`Main_Tick` paths; replay playback is `DAT_00A8D5F8 & 2`."
- Current Rust comments in `src/util/fixed_math.rs` that say every fixed sim tick equals one RA2 game frame are stale. Replacement wording: "Native `g_CurrentFrameCounter` is a late-committed gameplay frame counter inside `Main_Tick`; Rust fixed-step frequency and native frame identity must remain separate."

## 11. Remaining Uncertainty

- Exact upstream lifecycle for `House+0x1F7` / `House+0x1F8` result bytes is out-of-scope.
- Exact event construction/enqueueing paths for event case `0x13` are out-of-scope.
- Exact UI/file path that sets `DAT_00A8D5F8 & 4` before replay playback remains a replay-specific follow-up.

## Sources

- Read-only Ghidra decompile: `Main_Tick @ 0x0055D360`, `Main_Game @ 0x0052D9C0`, `FUN_0055CFD0`, `HouseClass::Update @ 0x004F8600`, `State_Machine @ 0x0048C8B0`, `EventClass::Execute @ 0x004C7600`.
- Read-only Ghidra xrefs: `get_xrefs_to 0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48`; `get_function_callers 0x0055CFD0`.
- Read-only Ghidra assembly contexts: `0x0055DE4A..0x0055DEA4`, `0x0052DA78..0x0052DA8D`, writer sites in `HouseClass::Update`, `State_Machine`, `EventClass::Execute`, clear groups in `FUN_0055CFD0`, replay continuation `0x0055DBBE..0x0055DC9E`, gameplay gate `0x0055D878..0x0055D897`.
- Prior docs: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`, `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`, `TIMING_SCHEDULER_TICK_SPINE_SYSTEM_MODEL_SYNTHESIS.md`, `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md`, `FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md`, `docs/research/timing/logic-vs-render-loop.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/app_sim_tick.rs`, `src/app_types.rs`, `src/util/fixed_math.rs`, `src/sim/replay.rs`.
