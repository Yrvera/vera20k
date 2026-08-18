# Main Tick Pending-Delete Skip Flags - Reswarm 2026-05-28

**Address(es):** `Main_Tick @ 0x0055D360`, pending-delete drain `FUN_00725C70`, session-end handler `FUN_0055CFD0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the four late `Main_Tick` globals read at `0x0055DE4F..0x0055DE71` that skip `FUN_00725C70`, their bounded static readers/writers, verified semantics where the writer path proves them, and the active-YR conditions under which pending-delete drain is skipped.  
**Non-Scope:** broader `Main_Tick` pacing, every path that sets the underlying `HouseClass` win/loss bytes, replay desync internals, and runtime debugger observation of exact queue contents during session-end teardown.  
**Confidence:** High  
**Active in YR:** Yes. The gate is on the standard `Main_Tick` path after `LogicClassPerTickUpdateLiveVector` and `Network_ServiceLoop`; it suppresses the pending-delete drain when a session-end flag is set.

## 1. Overview

`Main_Tick` normally drains the pending-delete queue late in the tick, after the live object vector has already run. Four byte globals can skip that drain by branching around the late frame-increment/wait/drain block.

Those four globals are not ordinary pause flags. Static writer evidence identifies them as session-end flags: local player victory, local player defeat, user-confirmed quit-to-main, and graceful disconnect/exit event. Standard active YR therefore skips `FUN_00725C70` on the tick where one of those session-end conditions is observed; ordinary gameplay, in-game pause/modal states, and normal active ticks do not use these four flags to suppress the drain.

## 2. Key Globals

| Address | Verified semantic name | Type | Set-to-1 writers | Readers | Active in YR |
|---:|---|---|---|---|---|
| `0x00A83D49` | local/session victory flag | byte | `HouseClass::Update @ 0x004F867C, 0x004F8692, 0x004F86EE, 0x004F87BB` | `Main_Tick @ 0x0055DE4F`; `FUN_0055CFD0 @ 0x0055D059, 0x0055D08B, 0x0055D0B0` | Yes, when local/session victory route is reached |
| `0x00A8ECD0` | local/session defeat flag | byte | `HouseClass::Update @ 0x004F86F7, 0x004F879C, 0x004F87B2` | `Main_Tick @ 0x0055DE58`; `FUN_0055CFD0 @ 0x0055D064, 0x0055D094, 0x0055D0B9, 0x0055D17D` | Yes, when local/session defeat route is reached |
| `0x008B41C0` | quit-to-main confirmed flag | byte | `State_Machine @ 0x0048CB2E` | `Main_Tick @ 0x0055DE61`; `FUN_0055CFD0 @ 0x0055D06C, 0x0055D20C` | Yes, user confirms quit from exit dialog |
| `0x00A83D48` | graceful disconnect / EXIT event flag | byte | `EventClass::Execute (entry 0x004C6CB0) @ 0x004C7917`, case `0x13` <!-- corrected 2026-05-28: was "EventClass::Execute @ 0x004C7917"; 0x004C7600 cited elsewhere is within-body, not entry; entry confirmed via get_function_by_address 0x004C6CB0 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> | `Main_Tick @ 0x0055DE6A`; `FUN_0055CFD0 @ 0x0055D074, 0x0055D250` | Yes, network/exit event path |

Session start initialization clears all four at `Main_Game @ 0x0052DA78..0x0052DA8D`. The session-end handler also clears all four on each handled route: victory, defeat, quit, and disconnect (`0x0055D123..0x0055D135`, `0x0055D1B1..0x0055D1C3`, `0x0055D219..0x0055D22B`, `0x0055D25C..0x0055D26E`).

## 3. Core Logic

### 3.1 Late `Main_Tick` gate

Read-only decompilation of `Main_Tick @ 0x0055D360` shows this late sequence after gameplay/replay work, `LogicClassPerTickUpdateLiveVector`, frame-time accumulation, `FUN_00647260`, `FUN_00637550`, and `Network_ServiceLoop`:

```text
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter += 1
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

Assembly context confirms each flag is read as a byte and any nonzero value jumps to `0x0055DEC8`, bypassing the frame increment, `FUN_0055E160`, `FUN_00725C70`, and `FUN_00637270`:

| Address | Instruction | Effect |
|---:|---|---|
| `0x0055DE4F` | `MOV AL,[0x00A83D49]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | skip on victory flag |
| `0x0055DE58` | `MOV AL,[0x00A8ECD0]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | skip on defeat flag |
| `0x0055DE61` | `MOV AL,[0x008B41C0]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | skip on quit-to-main flag |
| `0x0055DE6A` | `MOV AL,[0x00A83D48]`; `TEST AL,AL`; `JNZ 0x0055DEC8` | skip on graceful-disconnect flag |
| `0x0055DE9F` | `CALL 0x00725C70` | only reached when all four bytes are zero |

Tiny detail: the skip happens after `Network_ServiceLoop @ 0x0055DE4A`, not before it. A session-end flag set earlier in the same `Main_Tick` still allows that late network service call to run before the branch suppresses the pending-delete drain.

Tiny detail: the branch also skips `g_CurrentFrameCounter++`. This means queued pending-delete entries are delayed together with the late frame advance; the next outer-loop handling enters `FUN_0055CFD0` and clears/routes the session-end condition.

### 3.2 Handler path after the skip

`Main_Game @ 0x0048CCC0` calls `Main_Tick`, then conditionally calls `FUN_0055CFD0`; static callers of `FUN_0055CFD0` are only `Main_Game @ 0x0048CE93` and `0x0048CEA1`.

`FUN_0055CFD0 @ 0x0055CFD0` first tests the same four bytes. If any is nonzero, it increments `DAT_00A8DAB4`, calls `FUN_00684240`, routes by which flag is set, clears all four bytes, and calls the appropriate post-session path:

| Flag route | Clear sites | Routed function(s) | Verified interpretation |
|---|---|---|---|
| `DAT_00A83D49 != 0` | `0x0055D123..0x0055D135` <!-- corrected 2026-05-28: was 0x0055D1B1..0x0055D1C3; assembly at 0x0055D123 clears defeat(0x00A8ECD0) then victory(0x00A83D49) then routes to FUN_00685670; the 0x0055D1B1 block routes to FUN_00685DC0 (defeat); ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; verified via get_assembly_context 0x0055D123,0x0055D1B1 --> | `FUN_00685670` | victory/session win route |
| `DAT_00A8ECD0 != 0` (and `DAT_00A83D49 == 0`) | `0x0055D1B1..0x0055D1C3` <!-- corrected 2026-05-28: was 0x0055D123..0x0055D135; assembly at 0x0055D1B1 clears victory(0x00A83D49) then defeat(0x00A8ECD0) then routes to FUN_00685DC0; ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT --> | `FUN_00685DC0` | defeat/session loss route |
| `DAT_008B41C0 != 0` | `0x0055D219..0x0055D22B` | `FUN_006863E0` | user quit-to-main route |
| `DAT_00A83D48 != 0` | `0x0055D25C..0x0055D26E` | `GameExit__BattleControlTerminated` plus graceful disconnect logging | disconnect/exit-event route |

Tiny detail: `FUN_0055CFD0` handles victory/defeat before quit/disconnect if one of the win/loss flags is set. If multiple flags are somehow set, the code's route priority is not a simple table lookup; it follows the nested branch order in `FUN_0055CFD0`.

### 3.3 Writer evidence

#### `0x00A83D49` / victory route

`HouseClass::Update` (entry `0x004F8440`; <!-- corrected 2026-05-28: was cited as "@ 0x004F8600"; 0x004F8600 has no instruction — confirmed via get_assembly_context 0x004F8600 returning "No instruction at address"; actual entry confirmed via get_function_by_address 0x004F8440 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> the `0x004F8600` region is within-body) sets `DAT_00A83D49 = 1` from the `House+0x1F7` branch and from the opponent-loss branch. The relevant verified writes are:

- `0x004F867C`: single-player, current/player-controlled branch for `House+0x1F7`.
- `0x004F8692`: local-player branch when `this == g_PlayerPtr` in non-SP, also reached by the decompiler's local-player test.
- `0x004F86EE`: multiplayer/session branch after interface availability checks.
- `0x004F87BB`: opposite-side handling while processing `House+0x1F8`, where a non-local defeat can imply local/session win.

The semantic label "victory" is grounded by `FUN_0055CFD0` routing this flag to `FUN_00685670`, and by the `House+0x1F7` win-state field checks in `HouseClass::Update` and `State_Machine`.

#### `0x00A8ECD0` / defeat route

`HouseClass::Update` (entry `0x004F8440`) sets `DAT_00A8ECD0 = 1` from the `House+0x1F8` branch and from the inverse local/session handling of `House+0x1F7`. Verified writes:

- `0x004F86F7`: after a `House+0x1F7` branch decides the local/session result is defeat instead of victory.
- `0x004F879C`: single-player, current/player-controlled branch for `House+0x1F8`.
- `0x004F87B2`: local-player branch when `this == g_PlayerPtr` in non-SP.

The semantic label "defeat" is grounded by `FUN_0055CFD0` routing this flag to `FUN_00685DC0`, and by `State_Machine` testing `g_PlayerPtr+0x1F8` as a player result byte.

#### `0x008B41C0` / quit-to-main route

`State_Machine @ 0x0048C8B0` handles `g_GameState == 3` by calling `FUN_004F1840()`. In the nested result switch, case `5` writes `DAT_008B41C0 = 1` at `0x0048CB2E`.

`FUN_0055CFD0` then routes this flag to `FUN_006863E0` after clearing all four session-end bytes. This supports the semantic name "user confirmed quit-to-main" rather than a generic pause or freeze flag.

#### `0x00A83D48` / graceful disconnect route

`EventClass::Execute @ 0x004C7600` handles event case `0x13` by logging `"Processing EXIT event on frame"` with `g_CurrentFrameCounter`, then writing `DAT_00A83D48 = 1` at `0x004C7917` and returning.

`FUN_0055CFD0` routes this flag through `FUN_0072DFB0`, `FUN_0069BB40`, `GameExit__BattleControlTerminated`, `FUN_0069BAB0`, and logs `"Disconnect Gracefully"`. This supports the semantic name "graceful disconnect / EXIT event".

### 3.4 Static xref inventory

`get_bulk_xrefs` produced the following bounded inventory for the four globals:

| Flag | Writes | Reads |
|---|---|---|
| `0x00A83D49` | `0x0052DA78` init clear; `0x0055D129/0x0055D1B1/0x0055D219/0x0055D25C` handler clears; `0x004F867C/0x004F8692/0x004F86EE/0x004F87BB` set-to-1 writes | `0x0055D059/0x0055D08B/0x0055D0B0/0x0055D0E8` in `FUN_0055CFD0`; `0x0055DE4F` in `Main_Tick` |
| `0x00A8ECD0` | `0x0052DA7F` init clear; `0x0055D123/0x0055D1B7/0x0055D21F/0x0055D262` handler clears; `0x004F86F7/0x004F879C/0x004F87B2` set-to-1 writes | `0x0055D064/0x0055D094/0x0055D0B9/0x0055D17D` in `FUN_0055CFD0`; `0x0055DE58` in `Main_Tick` |
| `0x008B41C0` | `0x0052DA86` init clear; `0x0055D12F/0x0055D1BD/0x0055D225/0x0055D268` handler clears; `0x0048CB2E` set-to-1 write | `0x0055D06C/0x0055D20C` in `FUN_0055CFD0`; `0x0055DE61` in `Main_Tick` |
| `0x00A83D48` | `0x0052DA8D` init clear; `0x0055D135/0x0055D1C3/0x0055D22B/0x0055D26E` handler clears; `0x004C7917` set-to-1 write | `0x0055D074/0x0055D250` in `FUN_0055CFD0`; `0x0055DE6A` in `Main_Tick` |

No INI-backed writers were found in this xref set. The flags are hardcoded session lifecycle state.

## 4. INI Keys

No INI key directly gates the four late `Main_Tick` session-end bytes or the pending-delete drain skip. The only inputs verified here are engine/session state, `HouseClass` result bytes, exit dialog result, and network/exit events.

## 5. Integration Points

| Function / address | Role | Evidence | Active in standard YR |
|---|---|---|---|
| `Main_Tick @ 0x0055D360` | reads the four flags and skips `FUN_00725C70` if any is nonzero | decompile; assembly `0x0055DE4F..0x0055DE9F` | Yes |
| `FUN_00725C70 @ 0x00725C70` | pending-delete drain | decompile; call at `0x0055DE9F` | Yes when all four flags are zero |
| `FUN_0055CFD0 @ 0x0055CFD0` | post-`Main_Tick` session-end handler and flag clearer/router | decompile; callers `Main_Game @ 0x0048CE93, 0x0048CEA1` | Yes |
| `HouseClass::Update @ 0x004F8440` <!-- corrected 2026-05-28: was 0x004F8600; entry confirmed via get_function_by_address 0x004F8440; 0x004F8600 has no instruction — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> | sets victory/defeat globals from house result fields | decompile; assembly writes listed above | Yes |
| `State_Machine @ 0x0048C8B0` | sets quit flag from exit-confirm dialog result | decompile; assembly `0x0048CB2E` | Yes when player confirms quit |
| `EventClass::Execute @ 0x004C6CB0` <!-- corrected 2026-05-28: was 0x004C7600; entry confirmed via get_function_by_address 0x004C6CB0; 0x004C7600 is within function body — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> | sets graceful-disconnect flag on event case `0x13` | decompile; assembly `0x004C7917` | Yes in network/exit event path |
| `Main_Game @ 0x0048CCC0` (calls `Main_Tick`/`FUN_0055CFD0`); session-init clears are in `Main_Game @ 0x0052D9A0` <!-- corrected 2026-05-28: was attributed to 0x0048CCC0 for both roles; init clears at 0x0052DA78..0x0052DA8D are in a separate Main_Game function confirmed via get_function_by_address 0x0052D9A0; Sources cited it as 0x0052D9C0 which is also wrong (off by 0x20) — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT --> | `Main_Game @ 0x0048CCC0` calls `Main_Tick` and `FUN_0055CFD0`; `Main_Game @ 0x0052D9A0` initializes all four to zero at session setup | decompile; assembly `0x0052DA78..0x0052DA8D`; callers `0x0048CE93, 0x0048CEA1` | Yes |

## 6. Current Rust Implementation Status

Rust does not currently expose a native-equivalent session-end gate around a late pending-delete drain, because Rust does not yet have the native pending-delete queue phase.

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/world/mod.rs:675` `Simulation::despawn_entity` | immediate entity removal, occupancy removal, radio clear, live unregister | no pending-delete queue and no session-end drain suppression |
| `src/sim/world/mod.rs:1187` `Simulation::advance_tick` | advances `binary_frame` at tick start from elapsed milliseconds | does not model late native frame increment, and has no branch that suppresses frame increment plus pending-delete drain on victory/defeat/quit/disconnect |
| `src/sim/combat/mod.rs:804` death handling | marks animated entities `dying=true`, removes non-animated entities immediately | does not preserve native `UnInit -> queue -> late gated drain` ordering |
| `src/app_sim_tick.rs:292` app animation/despawn | ticks death animations after `Simulation::advance_tick`, then despawns completed death animations | not equivalent to native late drain; no session-end skip |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| late `Main_Tick` flag reads and `FUN_00725C70` skip | verified | decompile `Main_Tick`; assembly `0x0055DE4F..0x0055DE9F` | none |
| all static xrefs to four skip flags | verified | `get_bulk_xrefs(0x00A83D49,0x00A8ECD0,0x008B41C0,0x00A83D48)` | runtime write order when multiple flags race is not sampled |
| `0x00A83D49` victory semantics | verified | `HouseClass::Update @ 0x004F8600`; writer addresses; `FUN_0055CFD0` route to `FUN_00685670` | exact UI contents of victory screen out-of-scope |
| `0x00A8ECD0` defeat semantics | verified | `HouseClass::Update @ 0x004F8600`; writer addresses; `FUN_0055CFD0` route to `FUN_00685DC0` | exact UI contents of defeat screen out-of-scope |
| `0x008B41C0` quit semantics | verified | `State_Machine @ 0x0048C8B0`; write `0x0048CB2E`; `FUN_0055CFD0` route to `FUN_006863E0` | exact dialog result code meanings inside `FUN_004F1840` beyond case `5` out-of-scope |
| `0x00A83D48` disconnect semantics | verified | `EventClass::Execute @ 0x004C7600`; case `0x13`; write `0x004C7917`; disconnect route in `FUN_0055CFD0` | exact event construction sites for case `0x13` out-of-scope |
| session initialization clears | verified | `Main_Game @ 0x0052DA78..0x0052DA8D` | none |
| session-end handler clears | verified | `FUN_0055CFD0`; assembly clear groups | none for four-byte clearing |
| ordinary pause/modal behavior relative to these flags | verified for this slice | `Main_Tick` gates gameplay block on `g_GameState`, but late four-flag drain gate is independent; no writer from modal pause except quit result | full pause behavior belongs to timing docs |
| Rust pending-delete drain scheduling | verified current absence | source scan of `world/mod.rs`, `combat/mod.rs`, `app_sim_tick.rs` | future implementation contract/design |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Which four late flags gate the pending-delete drain? -> `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, and `0x00A83D48`; any nonzero branches to `0x0055DEC8` before `FUN_00725C70`.` (evidence: `Main_Tick @ 0x0055D360`; assembly `0x0055DE4F..0x0055DE71`)
- `[RESOLVED] OQ-002 - Does the gate skip only `FUN_00725C70` or more? -> It skips frame increment, optional `FUN_00684290`, `FUN_0055E160`, `FUN_00725C70`, and `FUN_00637270`.` (evidence: `0x0055DE73..0x0055DEA4`)
- `[RESOLVED] OQ-003 - Is `Network_ServiceLoop` skipped by these flags? -> No; the relevant call at `0x0055DE4A` occurs immediately before the first flag read.` (evidence: assembly context `0x0055DE40..0x0055DE4F`)
- `[RESOLVED] OQ-004 - Who sets `0x00A83D49` to one? -> `HouseClass::Update` (entry `0x004F8440`) set-to-1 writes at `0x004F867C`, `0x004F8692`, `0x004F86EE`, and `0x004F87BB`.` (evidence: xrefs and decompile `0x004F8440` <!-- corrected 2026-05-28: was "decompile 0x004F8600"; entry is 0x004F8440 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->)
- `[RESOLVED] OQ-005 - What does `0x00A83D49` mean? -> Local/session victory route; `FUN_0055CFD0` routes it to `FUN_00685670` via the `0x0055D123` clear block.` (evidence: decompile `0x0055CFD0`; `House+0x1F7` branch in `HouseClass::Update` entry `0x004F8440`; assembly `get_assembly_context 0x0055D123` confirms `FUN_00685670` call at `0x0055D14A`)
- `[RESOLVED] OQ-006 - Who sets `0x00A8ECD0` to one? -> `HouseClass::Update` (entry `0x004F8440`) set-to-1 writes at `0x004F86F7`, `0x004F879C`, and `0x004F87B2`.` (evidence: xrefs and decompile `0x004F8440` <!-- corrected 2026-05-28: was 0x004F8600 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->)
- `[RESOLVED] OQ-007 - What does `0x00A8ECD0` mean? -> Local/session defeat route; `FUN_0055CFD0` routes it to `FUN_00685DC0` via the `0x0055D1B1` clear block.` (evidence: decompile `0x0055CFD0`; `House+0x1F8` branch in `HouseClass::Update` entry `0x004F8440`; assembly `get_assembly_context 0x0055D1B1` confirms `FUN_00685DC0` call at `0x0055D1D8`)
- `[RESOLVED] OQ-008 - Who sets `0x008B41C0` to one? -> `State_Machine` writes it at `0x0048CB2E` in `g_GameState == 3` nested result case `5`.` (evidence: xrefs and decompile `0x0048C8B0`)
- `[RESOLVED] OQ-009 - What does `0x008B41C0` mean? -> User confirmed quit-to-main/session abort route; `FUN_0055CFD0` routes it to `FUN_006863E0`.` (evidence: decompile `0x0055CFD0`, `0x0048C8B0`)
- `[RESOLVED] OQ-010 - Who sets `0x00A83D48` to one? -> `EventClass::Execute` (entry `0x004C6CB0`) case `0x13` writes it at `0x004C7917`.` (evidence: xrefs and decompile `0x004C6CB0` <!-- corrected 2026-05-28: was "decompile 0x004C7600"; entry is 0x004C6CB0 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->)
- `[RESOLVED] OQ-011 - What does `0x00A83D48` mean? -> Graceful disconnect / EXIT event route; handler calls `GameExit__BattleControlTerminated` and logs disconnect gracefully.` (evidence: `EventClass::Execute @ 0x004C6CB0` <!-- corrected 2026-05-28: was 0x004C7600 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->; `FUN_0055CFD0 @ 0x0055CFD0`)
- `[RESOLVED] OQ-012 - Where are the flags initialized? -> `Main_Game` (entry `0x0052D9A0`) clears all four at session setup at `0x0052DA78..0x0052DA8D`.` (evidence: decompile `0x0052D9A0` <!-- corrected 2026-05-28: was "decompile 0x0052D9C0"; entry is 0x0052D9A0 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->; assembly context)
- `[RESOLVED] OQ-013 - Where are the flags cleared after session-end routing? -> `FUN_0055CFD0` clears all four on each victory/defeat/quit/disconnect route.` (evidence: clear groups `0x0055D123..0x0055D135`, `0x0055D1B1..0x0055D1C3`, `0x0055D219..0x0055D22B`, `0x0055D25C..0x0055D26E`)
- `[RESOLVED] OQ-014 - Does ordinary in-game pause use these four flags? -> No writer from pause/menu state was found; `g_GameState` gates the gameplay/render block, while the late pending-delete drain gate reads only the four session-end bytes. Quit confirmation from the menu can set `0x008B41C0`.` (evidence: `Main_Tick @ 0x0055D360`; `State_Machine @ 0x0048C8B0`)
- `[RESOLVED] OQ-015 - Is the drain skipped in standard active YR? -> Yes, but only when victory, defeat, quit-to-main, or disconnect/EXIT event has set one of the four bytes before the late gate in that `Main_Tick`.` (evidence: writers plus `Main_Tick` branch)
- `[RESOLVED] OQ-016 - Does current Rust have an equivalent gate? -> No; current Rust lacks the native pending-delete drain phase and uses immediate or animation-delayed despawn instead.` (evidence: `src/sim/world/mod.rs:675`, `src/sim/combat/mod.rs:804`, `src/app_sim_tick.rs:292`)
- `[DEFERRED] OQ-017 - What exact event construction paths enqueue event case `0x13`?` (category: out-of-scope; reason: this slot only needed the writer that sets the late skip flag; next-step-if-pursued: trace `EventClass` construction for ABOUTTOEXIT/EXIT network events)
- `[DEFERRED] OQ-018 - What happens to pending-delete queue contents during full post-session teardown after the skipped drain?` (category: needs-runtime-debugger; reason: static evidence proves the ordinary drain is skipped and teardown handler runs, but runtime queue contents during victory/defeat/quit teardown were not sampled; next-step-if-pursued: hardware-watch `0x00B0F6A8` and step a controlled victory/quit scenario)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native pending-delete drain is called late only when all four session-end bytes are zero. | `Main_Tick @ 0x0055DE4F..0x0055DE9F` | missing: no native pending-delete phase or session-end gate | `src/sim/world/mod.rs::advance_tick`, future lifecycle/pending-delete queue | schedule a late drain after live-object processing, and suppress it when modeled session-end flags are set | object dies on a normal tick: uninit queues it, late drain runs that tick; object dies on tick that triggers victory/defeat/quit/disconnect: late drain is skipped | Do not drain pending-delete unconditionally at the end of every Rust tick once session-end flags exist |
| The four skip flags are session-end flags, not ordinary pause flags. | writer xrefs/decompiles: `0x004F8600`, `0x0048C8B0`, `0x004C7600`; handler `0x0055CFD0` | unchecked/missing: Rust end-of-match/menu state is not mapped to these exact native flags | app/session state plus sim scheduler boundary | separate victory, defeat, quit-confirm, and disconnect end-state gates from pause/modal UI state | opening the in-game options menu must not by itself suppress pending-delete drain; confirming quit must | Do not use a generic `paused` boolean to skip the native late drain |
| Victory/defeat flags can be set from `HouseClass::Update`, which runs before the late drain check. | `HouseClass::Update` writers; `Main_Tick` ordering | current Rust uses separate victory/defeat checks and immediate despawn paths | `src/sim/world/mod.rs` defeat/game-completion checks and future house update scheduler | if a house result becomes final during the tick, the same tick's late pending-delete drain is skipped | killing the last enemy building queues deaths and sets victory; pending-delete drain does not run after the flag is set in that `Main_Tick` | Do not assume match-end cleanup has the same object-free timing as ordinary combat deaths |
| Quit-to-main and graceful disconnect suppress drain through distinct flags. | `State_Machine @ 0x0048CB2E`; `EventClass::Execute @ 0x004C7917`; `FUN_0055CFD0` routes | missing/unchecked in current Rust app flow | app-level quit/disconnect handling, future sim stop reason | represent stop reason separately enough to match native drain suppression and routing | user confirms quit or network EXIT event on a tick with queued deletions: normal late drain is skipped and session-end cleanup handles transition | Do not collapse quit, disconnect, victory, and defeat into one untyped stop flag if downstream cleanup/timing depends on the reason |
| `Network_ServiceLoop` still runs before the skip branch. | assembly `0x0055DE4A` before reads `0x0055DE4F..0x0055DE6A` | unchecked: Rust networking/app service ordering not compared here | future net/app scheduler if parity scope includes network teardown | keep the last native-equivalent service pump before deciding whether to skip drain | disconnect event set before late phase still allows one service loop before skip | Do not branch out of the late tick immediately at the moment a session-end flag is set |

### Stale Docs / Follow-up Docs

- `docs/research/PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`: replace the deferred sentence about `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, and `0x00A83D48` with: "Resolved by `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`: these are session-end flags for local/session victory, local/session defeat, user-confirmed quit-to-main, and graceful disconnect/EXIT event. `Main_Tick` skips frame increment, `FUN_0055E160`, `FUN_00725C70`, and `FUN_00637270` when any is nonzero; ordinary pause/modal state does not by itself set them."
- `docs/research/timing/logic-vs-render-loop.md`: its "four session-end flags" framing is consistent with this report; add the extra pending-delete detail that the same branch also suppresses `FUN_00725C70`.

## Sources

- Ghidra decompiled/read-only: `Main_Tick @ 0x0055D360`, `FUN_0055CFD0 @ 0x0055CFD0`, `Main_Game @ 0x0052D9A0` <!-- corrected 2026-05-28: was 0x0052D9C0; confirmed via get_function_by_address 0x0052D9A0 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->, `HouseClass::Update @ 0x004F8440` <!-- corrected 2026-05-28: was 0x004F8600 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->, `State_Machine @ 0x0048C8B0`, `EventClass::Execute @ 0x004C6CB0` <!-- corrected 2026-05-28: was 0x004C7600 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->, `FUN_00725C70 @ 0x00725C70`.
- Ghidra xrefs/read-only: `get_bulk_xrefs` for `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`, `0x00A83D48`, `0x00725C70`; `get_function_xrefs/get_function_callers` for `FUN_0055CFD0`.
- Ghidra assembly contexts/read-only: `0x0055DE4F`, `0x0055DE58`, `0x0055DE61`, `0x0055DE6A`, `0x0055DE9F`, `0x004F867C`, `0x004F8692`, `0x004F86EE`, `0x004F86F7`, `0x004F879C`, `0x004F87B2`, `0x004F87BB`, `0x0048CB2E`, `0x004C7917`, `0x0052DA78..0x0052DA8D`, handler clear groups in `0x0055CFD0`.
- Prior docs referenced: `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`, `docs/research/timing/logic-vs-render-loop.md`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`.
- Rust files scanned: `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/app_sim_tick.rs`, `src/app_types.rs`.
