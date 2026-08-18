# Frame Increment Guard Flags — Ghidra Research Report

**Date:** 2026-05-28
**Addresses:** `Main_Tick @ 0x0055D360` (guard at `0x0055DE4F..0x0055DE6A`); writers listed per flag below.
**Investigation Mode:** exhaustive-slice
**Confidence:** High for all four flags. Evidence sourced from exhaustive xref and decompile work already in
`MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` and `timing/logic-vs-render-loop.md`, both produced
this same session (2026-05-28) with direct Ghidra MCP citations. No novel Ghidra queries needed; existing docs
provide complete coverage.
**Active in YR:** Conditional — all four flags are zero throughout normal active skirmish play; they are set only
at session-end events. The counter therefore advances normally every tick during active play.

---

## Target Question

What does each of the four globals that gate `g_CurrentFrameCounter++` in `Main_Tick @ 0x0055D360` mean, and
what player-facing states cause the counter to freeze?

## Non-goals

- Throttle math, speed setting, timer basis, render coupling (those are slots 1–4).
- Runtime measurement of wall-clock frame rate.
- Full pause/modal matrix beyond what is needed to establish that ordinary pause does NOT set these four flags.

## Evidence Needed for COMPLETE

| Requirement | Status | Evidence |
|---|---|---|
| Assembly of all four flag reads in `Main_Tick` | Met | `0x0055DE4F..0x0055DE6A`; cited in `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.1 |
| Static set-to-1 writer for each flag | Met | `HouseClass::Update`, `State_Machine`, `EventClass::Execute`; cited below |
| Decompile showing writer set-condition | Met | `0x004F8600`, `0x0048C8B0`, `0x004C7600`; cited in source doc §3.3 |
| Confirmation that ordinary pause/menu does NOT set these flags | Met | `Main_Tick` gating plus xref inventory shows no pause-modal writer; source doc §3.4 |

## Stop Conditions

Stop when all four flags are named, writers are confirmed, and "frozen states" list is complete.
Do not re-decompile functions already fully covered in same-session docs.

---

## 1. Overview

Near the end of `Main_Tick @ 0x0055D360`, after `LogicClassPerTickUpdateLiveVector` and `Network_ServiceLoop`,
four single-byte globals are tested in sequence:

```text
if DAT_00A83D49 == 0
and DAT_00A8ECD0 == 0
and DAT_008B41C0 == 0
and DAT_00A83D48 == 0:
    g_CurrentFrameCounter += 1
    ...
    FUN_0055E160()       ; throttle/wait helper
    FUN_00725C70()       ; pending-delete drain
    FUN_00637270()
    return g_GameActive == 0
else:
    ; skip everything above, return immediately
```

(Verified via assembly context `0x0055DE4F..0x0055DE9F`; cited in
`MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.1.)

All four are zero throughout active gameplay. The counter advances on every tick while the match is live.
Any non-zero value jumps to `0x0055DEC8`, which also skips the throttle helper, the pending-delete drain,
and `FUN_00637270`.

Session initialization (`Main_Game @ 0x0052DA78..0x0052DA8D`) clears all four at match start.
`FUN_0055CFD0` clears all four after routing the session-end condition.

---

## 2. Key Globals

| Address | Semantic name | Set-to-1 writer(s) | Set condition | Active in YR |
|---:|---|---|---|---|
| `0x00A83D49` | **Local/session victory** | `HouseClass::Update @ 0x004F867C, 0x004F8692, 0x004F86EE, 0x004F87BB` | Local player's house wins (`House+0x1F7`); SP/MP paths differ on `this == g_PlayerPtr` | Yes — fires when this player wins |
| `0x00A8ECD0` | **Local/session defeat** | `HouseClass::Update @ 0x004F86F7, 0x004F879C, 0x004F87B2` | Local player's house loses (`House+0x1F8`); SP/MP paths differ | Yes — fires when this player loses |
| `0x008B41C0` | **User confirmed quit-to-main** | `State_Machine @ 0x0048CB2E` (case 3, sub-case 5) | Player confirms quit from the exit dialog | Yes — fires when player acknowledges quit prompt |
| `0x00A83D48` | **Graceful disconnect / EXIT event** | `EventClass::Execute @ 0x004C7917` (event case `0x13`) | `EXIT` network event received and executed | Yes — fires on network disconnect / leave event |

Evidence for all four: xref inventory `get_bulk_xrefs(0x00A83D49, 0x00A8ECD0, 0x008B41C0, 0x00A83D48)`;
writer decompiles `0x004F8600`, `0x0048C8B0`, `0x004C7600`; session-end handler `FUN_0055CFD0 @ 0x0055CFD0`;
all cited in `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.3–3.4.

---

## 3. Core Logic

### 3.1 Flag `0x00A83D49` — local/session victory

Set by `HouseClass::Update @ 0x004F8600` at four write sites when the local player's house achieves a win
state (`House+0x1F7`). SP path writes `0x004F867C`; MP/local-player path writes `0x004F8692` and
`0x004F86EE`; an opposite-side-loss-implies-local-win path writes `0x004F87BB`.

`FUN_0055CFD0` routes this flag to `FUN_00685670` (victory screen / session-win path) and clears it at
`0x0055D1B1..0x0055D1C3`.

Active in YR: Yes, when the match ends in victory. Not a TS-legacy path; `HouseClass::Update` is the active
per-tick house evaluator in normal YR skirmish.
(Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.3, `timing/logic-vs-render-loop.md` §"four session-end flags".)

### 3.2 Flag `0x00A8ECD0` — local/session defeat

Set by `HouseClass::Update @ 0x004F8600` at three write sites when the local player's house loses
(`House+0x1F8`). The inverse of the victory path; `0x004F86F7` handles the local-player-defeat branch
inside the `0x1F7` win-state check; `0x004F879C` and `0x004F87B2` handle the `0x1F8` defeat field directly.

`FUN_0055CFD0` routes to `FUN_00685DC0` (defeat/loss screen) and clears at `0x0055D123..0x0055D135`.

Active in YR: Yes, when the match ends in defeat. Same active house-update path as victory flag.
(Evidence: same docs as §3.1 above.)

### 3.3 Flag `0x008B41C0` — user confirmed quit-to-main

Set by `State_Machine @ 0x0048C8B0` in the `g_GameState == 3` (exit dialog) handler, nested result case `5`,
at `0x0048CB2E`. `FUN_004F1840` is called and its return value is switched; case `5` writes
`DAT_008B41C0 = 1`.

`FUN_0055CFD0` routes to `FUN_006863E0` (exit/quit cleanup) and clears at `0x0055D219..0x0055D22B`.

Active in YR: Yes, when the player confirms quit from the in-game exit dialog. Does NOT fire on ordinary
in-game pause/options open — `g_GameState` gates the gameplay/render block but no pause-modal code writes
this flag.
(Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.3; `logic-vs-render-loop.md` §"four session-end flags".)

### 3.4 Flag `0x00A83D48` — graceful disconnect / EXIT event

Set by `EventClass::Execute @ 0x004C7600`, event case `0x13`. The function logs
`"Processing EXIT event on frame"` with `g_CurrentFrameCounter`, then writes `DAT_00A83D48 = 1` at
`0x004C7917` before returning.

`FUN_0055CFD0` routes to `FUN_0072DFB0`, `FUN_0069BB40`, `GameExit__BattleControlTerminated`,
`FUN_0069BAB0`, logging `"Disconnect Gracefully"`, and clears at `0x0055D25C..0x0055D26E`.

Active in YR: Yes, in multiplayer/network sessions when a peer or the local player sends an EXIT event.
In single-player skirmish, this flag fires only if a network EXIT event is somehow enqueued; ordinary
SP quit goes through the quit-dialog path (`0x008B41C0`) instead.
(Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.3.)

### 3.5 What ordinary pause/menu does NOT do

In-game options/pause (`OptionsClass::ShowInGameDialog`) sets `g_GameState = 5` and spins on its own
modal loop. `Main_Tick`'s gameplay block is gated off by `g_GameState != 0`, so input/AI/map/render are
skipped. However, **none of the four session-end bytes is set by pause/modal state**. The late frame
increment and pending-delete drain still execute on pause ticks (because all four flags remain zero).

This was confirmed by exhaustive xref inventory: no writer of `0x00A83D49`, `0x00A8ECD0`, `0x008B41C0`,
or `0x00A83D48` is reachable from the in-game options/pause flow except the quit-confirm path
(`State_Machine` case `5`) triggered after the player explicitly confirms quit.
(Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.4; `logic-vs-render-loop.md` §`OptionsClass::ShowInGameDialog`.)

---

## 4. Frame Counter Frozen States — Consolidated List

The frame counter (`g_CurrentFrameCounter`) does **not** advance in the following states:

| State | Which flag | Trigger | Active in standard skirmish |
|---|---|---|---|
| Local player wins the match (SP or MP) | `0x00A83D49` | `HouseClass::Update` detects `House+0x1F7` win-state for local player | Yes — fires once, at match victory |
| Local player loses the match (SP or MP) | `0x00A8ECD0` | `HouseClass::Update` detects `House+0x1F8` defeat-state for local player | Yes — fires once, at match defeat |
| Player confirms quit-to-main from exit dialog | `0x008B41C0` | `State_Machine` exit-dialog case `5` result | Yes — fires once, after player clicks "OK" on quit prompt |
| Graceful disconnect / network EXIT event received | `0x00A83D48` | `EventClass::Execute` case `0x13` EXIT event | Yes in MP; rare in SP |

Counter advances normally in all other states, including:
- Active gameplay ticks (all four flags zero)
- In-game pause/options menu (`g_GameState != 0` but flags still zero)
- Scenario-delay / intro-cinematic early return path (separate branch before the late gate)
- All intermediate ticks leading up to a session-end condition

---

## 5. INI Keys

None. The four flags are pure engine/session lifecycle state set by hardcoded paths in `HouseClass::Update`,
`State_Machine`, and `EventClass::Execute`. No INI key enables or disables any of them.
(Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §4.)

---

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/sim/world/mod.rs::advance_tick` | `binary_frame` derived at tick start; no session-end gate around frame increment | Missing: no model of the four session-end flags; `binary_frame` always advances each tick regardless of match state |
| `src/app_sim_tick.rs:151..159` | `run_sim = false` when `state.paused` | Mismatch: native pause does NOT freeze the frame counter; only session-end flags do. Current Rust freezes `binary_frame` on pause, which is stricter than native but in the wrong direction (native counter advances during pause, native counter freezes only on end-of-match) |
| Session/victory/defeat handling | Various `defeat_detected`, game-over checks | Missing: no equivalent one-way latch that suppresses the frame increment for the remainder of that tick |

---

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Test name | Risk |
|---|---|---|---|---|---|---|---|
| Frame counter does NOT advance on the victory/defeat/quit/disconnect tick; it freezes for exactly one `Main_Tick` call, then `FUN_0055CFD0` clears the flag and routes post-session cleanup | Assembly `0x0055DE4F..0x0055DE71`; `FUN_0055CFD0 @ 0x0055CFD0`; cited in source doc | Rust `binary_frame` always advances; no end-of-match skip | `src/sim/world/mod.rs::advance_tick`, future session-end state | Model the four flags as a one-way latch; skip the late frame increment when any is set; clear after routing | Trigger a match victory: the tick where `HouseClass::Update` sets victory must not increment `binary_frame` | `test_frame_counter_frozen_while_session_ends` | Do not use a generic `paused` boolean — ordinary pause does NOT freeze the native counter |
| Native pause/options menu does NOT freeze the frame counter | `Main_Tick` gating; xref inventory no pause writer; `logic-vs-render-loop.md` §`OptionsClass` | Rust `state.paused` prevents `advance_fixed_simulation` entirely, freezing `binary_frame` on pause | `src/app_sim_tick.rs:151..159` | Separate "skip gameplay/render block" from "freeze frame counter"; counter must advance during pause | Open in-game options menu, advance 10 ticks: `binary_frame` increments 10 times, gameplay AI is frozen | `test_frame_counter_advances_during_pause` | Do not use the same boolean for gameplay-skip and frame-counter-freeze |
| The four flags are session-end only (victory / defeat / quit / disconnect), not a general pause mechanism | Writer xrefs and decompiles for all four; cited in `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` | Rust has no equivalent flag taxonomy | Session-state model | Represent the four conditions distinctly from pause/modal state | Confirming quit-to-main must suppress the drain on the confirm tick; opening pause menu must not | `test_quit_confirm_suppresses_drain_not_pause` | Do not collapse quit, disconnect, victory, and defeat into one untyped stop flag |

---

## 8. Negative Facts / Do Not Do

1. **Do not treat these flags as a pause mechanism.** Ordinary in-game pause / options menu does not set any of them. Verified by exhaustive xref inventory — no pause-modal writer found. (Evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.4.)

2. **Do not assume the counter can be frozen during active gameplay.** The only states that freeze it are match-end events. Any mid-match freeze would require a writer not present in the static xref set. (Evidence: xref inventory shows exactly four set-to-1 writers across all four flags; `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` §3.4.)

3. **Do not model this as a multi-tick freeze.** The four flags are set on one tick, skip the increment on that tick, and are cleared by `FUN_0055CFD0` on the very next `Main_Game` loop iteration. The counter freezes for exactly one `Main_Tick` call. (Evidence: `FUN_0055CFD0` clear groups; `Main_Game @ 0x0048CE93` caller; cited in source doc §3.2.)

4. **Do not use `g_GameState != 0` as equivalent to these flags.** `g_GameState` gates the gameplay/render block earlier in `Main_Tick`; the four session-end flags gate only the late frame increment and pending-delete drain. They are independent mechanisms. (Evidence: `logic-vs-render-loop.md` §`OptionsClass::ShowInGameDialog`.)

5. **Do not freeze `binary_frame` on Rust pause for parity.** Native counter advances during pause; Rust currently stops it. These are different behaviors. The correct parity is to freeze only on session-end flags. (Evidence: `Main_Tick` decompile; `MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` §3.3.)

---

## 9. Remaining Uncertainty

None for the four flag identities, writers, and frozen-states list. All four are fully pinned with writer
decompile + xref inventory from same-session verified docs.

Minor open items (not blocking handoff):
- Exact event construction paths that enqueue event case `0x13` (EXIT event) — relevant for MP but not needed to understand the freeze mechanism (deferred as OQ-017 in source doc).
- Runtime queue contents during session-end teardown — static evidence proves the drain is skipped; exact memory state is debugger-only (deferred as OQ-018).

---

## 10. Stale-Doc Corrections

**`docs/research/timing/game-speed-master-clock.md` §"Pause / resume"** contains this sentence:

> `g_CurrentFrameCounter` is **still incremented** at the end of `Main_Tick` unless one of
> `DAT_00a83d49 / DAT_00a8ecd0 / DAT_008b41c0 / DAT_00a83d48` is set — those four flags
> collectively define "freeze the clock".

This wording is misleading: it implies these are general pause flags. Replace with:

> `g_CurrentFrameCounter` is still incremented during active play and during pause/menu states;
> it is skipped only when one of the four **session-end flags** is set:
> `DAT_00a83d49` (victory), `DAT_00a8ecd0` (defeat), `DAT_008b41c0` (quit-to-main confirmed),
> `DAT_00a83d48` (graceful disconnect). These fire once at match end, not during ordinary pause.

(Note: that doc already has a "Correction (resolved)" annotation pointing to `logic-vs-render-loop.md`;
this report reinforces that correction.)

---

## Sources

All claims synthesized from same-session verified docs — no novel Ghidra queries added here.
Original Ghidra evidence is in the cited docs:

- `docs/research/MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md` — exhaustive flag investigation,
  xref inventory, writer decompiles, handler routing. All four OQs fully resolved.
- `docs/research/timing/logic-vs-render-loop.md` §"The four session-end flags (corrected from iteration 1)"
  and §`FUN_0055cfd0` — independent verification of flag semantics and `FUN_0055CFD0` routing.
- `docs/research/GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md` §3.1 — records the four-flag guard.
- `docs/research/MAIN_TICK_FRAME_COUNTER_PLACEMENT_VS_ADVANCE_TICK_GHIDRA_REPORT.md` §3.3 — notes
  session-end/freeze as conditional branch; defers full flag lifecycle (now resolved here).
- `docs/research/FRAME_COUNTER_PREINCREMENT_VS_RUST_BINARY_FRAME_GHIDRA_REPORT.md` §1 — transcribes
  the guard condition.
- `docs/research/SESSIONCLASS_GHIDRA_REPORT.md` §"Network/Multiplayer Globals" — early labeling of
  `DAT_00a83d49` as "GameWon flag" and `DAT_00a8ecd0` as "GameLost flag", consistent with
  full writer evidence.
