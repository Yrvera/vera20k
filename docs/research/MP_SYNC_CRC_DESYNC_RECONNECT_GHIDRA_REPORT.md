# Multiplayer Sync CRC, Desync Detection, and Reconnect — Ghidra Research Report

**Date:** 2026-05-28
**Slot:** 5 of VERA20k lockstep frame-model /re-swarm batch
**Addresses:** `Main_Tick @ 0x0055D360`, `Network_ServiceLoop @ 0x0048D080`,
`FUN_0048D1E0` (MP inner loop), `Desync_Handler @ 0x0048DC90` (mislabeled),
`FUN_006475f0` (reconnect trigger), `FUN_00648710` (reconnect dialog loop)
**Active in YR:** Per-finding below.

---

## 0. Scope, Non-goals, and Completion Criteria

**Target question:** What is the live-MP state CRC/hash used for desync detection,
when/how often is it compared between peers, what happens on mismatch, and what
does the reconnect/drop-recovery path do?

**Non-goals:** Frame-barrier/MaxAhead (slot 1), MaxAhead negotiation (slot 2),
FrameSendRate (slot 3), command-queue internals (slot 4). RNG seed (covered in
`RNG_SYSTEM_GHIDRA_REPORT.md`). Recording/replay mechanism (covered in
`DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md`).

**Evidence for COMPLETE:** All three sub-questions answered with direct Ghidra
decompilation evidence: (1) no live per-frame hash compare in LAN/WOL modes —
confirmed via `decompile_function 0x0055D360` + `0x0048D080` + `0x0048D1E0`;
(2) desync detection is command-gate only; (3) reconnect dialog fully traced via
`decompile_function 0x006475f0` + `0x00648710`.

**Stop conditions:** All three areas verified with inline citation; no remaining
HIGH-priority open questions.

---

## 1. CRITICAL: Desync_Handler @ 0x0048DC90 is a Selection-Clear, Not a Desync Handler

**Verified via `decompile_function 0x0048DC90` + `get_function_callers 0x0048DC90`.**

```c
void Desync_Handler(void) {
  while (g_CurrentObjects_Count != 0)
    (*vtable_unselect)(g_CurrentObjects_Data[0]);
  Selection__ResetMode();
  FUN_00431060();   // clears selection bit 0x02 across an 8×3 grid
}
```

All callers are selection/UI/building-placement paths (BandBox_LeftUp,
ObjectClass__Select, SelectClass__Action, HouseClass__Begin_Building_Placement,
etc.). The `Main_Tick` call site fires only in `DAT_00A8D5F8 & 2` (replay
**playback**) when the selection-object checksum from the recorded stream mismatches
the live sum — this is a single-player replay divergence reset, NOT a live-MP
network desync handler in any mode.

**Active in YR:** Yes — as selection-clear helper. NOT as desync comparator.
**Correct label:** `Selection__ClearAll` or `Deselect_All`.

---

## 2. State CRC / Hash: What Exists and What Is Dead

### 2.1 The `*ComputeCRC` vtable chain — dead in live play

`AbstractClass__ComputeCRC @ 0x00410410` hashes `UniqueID (+0x10)` and `Dirty flag
(+0x20)` via `CRCEngine__AddData @ 0x004A1DE0` (CRC-32, table at `0x0081F7B4`).
Derived overrides:
- `BombClass__ComputeCRC @ 0x00438A90`
- `DiskLaserClass__ComputeCRC @ 0x004A7B80`
- `SpawnManagerClass__ComputeCRC @ 0x006B7DE0`
- `TiberiumClass__ComputeCRC @ 0x00721DC0`

**These are dead in normal live play.** `FootClass__ComputeChecksum @ 0x004DBAD0`
has exactly one caller — `FootClass__Save_Convoy_State @ 0x00744640` — which itself
has zero callers. This is TS/campaign-era convoy-save code. (Verified via
`get_function_callers 0x004DBAD0` + `get_function_callers 0x00744640`.)

**Active in YR:** No — the `*ComputeCRC` chain is physically present but has no
live caller path in a standard skirmish or LAN/WOL game.

### 2.2 8-byte MapClass state hash — replay write-back only

During recording (`DAT_00A8D5F8 & 1`), `Main_Tick` writes two DWORDs from
`Scen+0xD64/0xD68` to the recording stream each frame. During playback
(`DAT_00A8D5F8 & 2`), these bytes are **written back** into `MapClass` via
`FUN_006D6000` — they are NOT compared. Only the selection-object sum (packed
house+index words) is compared during replay. (Verified via `decompile_function
0x0055D360`.)

**Active in YR:** Conditional (recording/replay only; both flags are zero in live
LAN/WOL/skirmish).

---

## 3. Live-MP Desync Mechanism: Command-Gate Lockstep, No Hash Compare

**Verified via `decompile_function 0x0055D360`, `0x0048D080`, `0x0048D1E0`.**

### 3.1 Main_Tick live-MP path

For `g_GameMode == 3` (LAN) or `4` (WOL), the tick sequence is:

```
1. Input: GScreenClass__Input
2. LogicClass__AI
3. Optional House_AI_Tick
4. Network keepalive every 8 frames (mode 4 only)
5. Map__Logic
6. RenderFrame_main
7. Service work: FUN_00551A30, LogicClass__PerTickUpdateLiveVector
8. Network_ServiceLoop()
9. g_CurrentFrameCounter++ — gated by:
       DAT_00A83D49 == 0 && DAT_00A8ECD0 == 0
       && DAT_008B41C0 == 0 && DAT_00A83D48 == 0
```

**No hash comparison exists in this path.** The frame counter does not advance
until all four stop flags are clear; the net layer holds these flags until remote
commands arrive. Divergence is prevented by gate, not detected by comparison.

### 3.2 Network_ServiceLoop @ 0x0048D080

```c
void Network_ServiceLoop(void) {
  FUN_00406F70();                          // audio/theme tick
  if (0 < DAT_00B45B68 && *DAT_00B45B5C)
    (*(*DAT_00B45B5C + 8))();              // plug-in vtable (WOL SDK hook, content unknown)
  if (g_GameMode == 3 || g_GameMode == 4)
    FUN_0048D1E0();                        // MP inner loop — command-queue drain
  if (g_GameMode == 1) { ... }            // modem path
  else if (g_GameMode != 2) goto done;
  // mode 2 (IPX/modem): reads packets of type 0x6C; if remote CRC byte != DAT_00A8DB30
  //   → FUN_00643C50(...) — a speed/progress-tracking update, NOT a disconnect
  done:
  // mode 4 WOL screen-size sync
}
```

For modes 3/4 there is no hash compare; `FUN_0048D1E0` is called.
The mode-2 `0x6C` packet handling calls `FUN_00643C50` — that function is a
speed-ratio update (reads `param_1+0x48` as a double scale factor), not a
disconnect or abort handler. (Verified via `decompile_function 0x0048D080`,
`0x00643C50`.)

**Active in YR (mode 2 path):** No — mode 2 is IPX/serial, not used in standard
YR LAN or WOL.

### 3.3 FUN_0048D1E0 — MP inner frame-sync loop

This function waits until the expected frame-budget timer elapses, then:
- When `DAT_00A8DA84` (player count) > 0 and all remote players have committed
  commands (`FUN_005422d0` check == 0): sends a frame-ready packet (`0x29`) to
  each remote player.
- Per-remote timeout: 120-tick deadline (≈2 s at 60 fps); on timeout resends
  `FUN_005410F0` packet type `0x5b` if remote frame > `0x3FF`.
- Handles event switch on `DAT_00A8D638`: `0x0F` = player drop → `FUN_00643C50`;
  `0x29` = per-player frame-number sync; `0x1C/0x1D/0x20/0x21/0x22/0x27/0x2F` =
  various notifications.

**No hash comparison in FUN_0048D1E0.** The "sync" is purely command-event-based
(wait for everyone to commit → advance). (Verified via `decompile_function
0x0048D1E0`.)

**Active in YR:** Yes — g_GameMode 3 (LAN) and 4 (WOL).

---

## 4. Desync Detection Summary Table

| Mode | Detection site | Mechanism | On-mismatch action |
|---|---|---|---|
| Replay playback (`DAT_00A8D5F8 & 2`) | `Main_Tick` | Selection-object sum compare | `Desync_Handler()` = clear selection. No abort. |
| IPX/serial (mode 2) | `Network_ServiceLoop` CRC byte `!= DAT_00A8DB30` | CRC byte from packet type `0x6C` | `FUN_00643C50(...)` = speed-ratio update, not disconnect |
| LAN (mode 3) / WOL (mode 4) | **None found** | Command-gate prevents divergence | N/A — no live desync detection |

**Conclusion: gamemd has NO live per-frame state-hash comparison in standard
LAN/WOL multiplayer.** (Verified via `decompile_function 0x0055D360`,
`0x0048D080`, `0x0048D1E0`.)

---

## 5. Reconnect / Drop-Recovery Path

### 5.1 Reconnect strings exist — mechanism verified

`search_strings "reconnect"` yields 16 CSF/STT strings at `0x008348C8`–
`0x0083FA30`:
- `STT:ReconnectButtonPlayer` (`0x008348C8`) — tooltip for player-status buttons
- `STT:ReconnectProgressPlayer` (`0x008348E4`) — progress bar tooltips
- `STT:ReconnectButtonLeaveGame` (`0x00834900`) — "Leave Game" button
- `STT:ReconnectEditInfo` (`0x00834920`) — info text box
- `STT:ReconnectLabelTime` (`0x00834938`) — countdown timer label
- `TXT_RECONNECTING_TO` (`0x00837700`) — "Reconnecting to %s"
- `TXT_RECONNECT_HELP1`–`HELP5` — help text shown in dialog
- `TXT_RECONNECT_KICK_SELF` (`0x008379E8`) — "You have been voted to be kicked"
- `TXT_RECONNECT_KICK_RECEIVED` (`0x00837A30`) — "Vote to kick received"

XRefs: `STT:ReconnectButtonPlayer` → `FUN_006040B0` (tooltip-string dispatcher,
dialog type `0xEA`). `TXT_RECONNECTING_TO` → `FUN_00648710` (×2). `TXT_RECONNECT_KICK_SELF`
→ `FUN_0064AAE0`. `TXT_RECONNECT_KICK_RECEIVED` → `FUN_0064AD10`.
(Verified via `get_xrefs_to 0x008348C8`, `0x00837700`, `0x008379E8`, `0x00837A30`.)

### 5.2 Reconnect dialog trigger: FUN_006475f0

`FUN_006475f0 @ 0x006475f0` is the per-frame network-tick orchestrator called
from the main game loop (not Main_Tick — from a parallel path). It:
1. Sets `DAT_00AFA268 | 1` (reconnect-active flag) on first call.
2. Calls `FUN_00648710(...)` with parameters derived from `DAT_00837340 + g_GameMode*0x10`
   (a table of reconnect parameters indexed by game mode).
3. On non-zero return (reconnect failed/timeout/kicked): sets `g_GameActive = 0`,
   clears `DAT_00A8B8BC / DAT_00A8B8B8`, optionally calls WOL cleanup
   `FUN_006C8820`, then falls through to `SidebarSurface__Init + FUN_006842F0`
   (return to post-game UI).
4. On zero return (all peers reconnected): resets the command-queue draining
   state (`DAT_00A8DB7C` array zeroed), calls `(*piVar13+0x2C)(1)` (resync call),
   and if in WOL mode sends a timing event every `0x7FF` frames.

(Verified via `decompile_function 0x006475f0`.)

**Active in YR:** Yes — confirmed by mode-3/4 dispatch in the function body. Called
when `DAT_00AFA268 & 1` is not yet set (first time a peer drop is detected) AND
`g_GameMode` is 3 or 4.

### 5.3 Reconnect dialog loop: FUN_00648710

`FUN_00648710 @ 0x00648710` is the blocking reconnect modal dialog loop. Key behavior:
- **Timer parameters:** `param_3` = reconnect-wait timeout (milliseconds),
  `param_4` = player count, `param_5` = frame number (used for initial progress
  seeding).
- **Per-peer state:** `DAT_00A8BB40[player_idx]` = number of kick-votes for that
  player; `DAT_00A8BB60[player_idx*8 + vote_idx]` = voter list.
- **Progress bar:** Windows dialog control `0x651` shows info text; controls
  `0x640`–`0x64F` are per-player progress bars; `0x650` = countdown timer label.
- **WOL keepalive:** When `g_GameMode == 4 && DAT_00A8DBA0 != 0`, sends "I'm still
  here" packet type `0x27` to all remote players via `FUN_005410F0` at intervals.
- **Resend progress:** Sends packet type `0x33` (guaranteed) to all remotes every
  `0xB4` ms. Modes 3/4 use `FUN_005410F0`; modes 1/2 use `FUN_0053F200` (IPX).
- **Kick vote:** `FUN_0064AAE0` proposes a kick (sends packet type `0x1C7` to all
  remotes). `FUN_0064AD10` records an incoming kick-vote in `DAT_00A8BB60` and logs
  `TXT_RECONNECT_KICK_RECEIVED`. When a player's vote count reaches `DAT_00A8DA84-1`
  (majority), they are kicked.
- **Return codes:**
  - `0` = success (all peers reconnected)
  - `2` = kicked out (client is voted out)
  - `3` = kicked out (variant — missing peer forcibly removed)
  - `7` = player chose "Leave Game" button
  - `8` = reconnect success but someone was kicked en route
  - `9` = reconnect success variant

(Verified via `decompile_function 0x00648710`; XRefs via `get_xrefs_to 0x00837700`.)

**Active in YR:** Yes for LAN (mode 3) and WOL (mode 4). WOL-only features
(keepalive packet `0x27`, DAT_00A8DBA0 gate) are conditional on mode-4.

### 5.4 What reconnect does NOT do

- It does NOT resend simulation state (no snapshot transfer).
- It does NOT rewind/replay missed frames.
- It waits for the missing peer to return to the current frame number.
- During the wait, `Network_ServiceLoop` continues to run (called inside the dialog
  loop body: `LAB_006497EC`), so network message processing stays live.
- The dialog also runs `GScreenClass__Input` + `LogicClass__AI` + `RenderFrame_main`
  while waiting (param_1 == 0 path), keeping the local game rendered.
- Reconnect is pause-based: game state is frozen (lockstep stall prevents frame advance).

**Active in YR:** Yes.

---

## 6. Scale Concerns for 30 Players (VERA20k)

1. **Command-gate scales with player count.** `FUN_005422D0` checks whether all
   remote players have committed. For 30 players this means waiting for 29 acks
   per frame before incrementing `g_CurrentFrameCounter`. The 120-tick (≈2 s)
   resend timeout in `FUN_0048D1E0` is per remote player, polled sequentially.
   At 30 players with one laggard, the inner loop retries against 29 players
   before advancing. **Risk: linear per-player polling in the frame-gate inner loop.**

2. **No per-frame hash compare exists** — so adding one for 30-player parity
   detection would require a NEW mechanism not in gamemd. The gamemd design
   assumption is that command-gate + identical RNG seed + deterministic sim =
   identical state automatically.

3. **Reconnect dialog arrays are fixed-size.** `DAT_00A8BB40` / `DAT_00A8BB60`
   appear to be indexed by `player_idx` (0..DAT_00A8DA84); if these are sized
   for 8 players (gamemd's limit), a 30-player reimplementation must replace
   these structures with dynamic collections.

4. **WOL keepalive broadcasts 1:N.** The "I'm still here" loop in `FUN_00648710`
   iterates `DAT_00A8DA84` players and sends a unicast packet to each. At 30
   players this is 29 sends per interval — manageable but worth noting.

---

## 7. Implementation Handoff

### Handoff A — No live hash-compare; lockstep enforced by command-gate
**Behavior:** gamemd LAN/WOL uses command-gate-then-advance, not
hash-compare-then-abort. No per-frame state hash is computed or exchanged
between live peers.
**Rust delta:** `src/sim/world/world_hash.rs` `state_hash()` is correct as a
replay/debug tool but the net layer MUST NOT implement "hash mismatch → abort
session" to mirror a gamemd mechanism — that mechanism does not exist.
**Affected surface:** `src/sim/world/world_hash.rs`, future net layer.
**Acceptance:** LAN game completes without spurious hash-mismatch aborts.
**Test:** `test_no_live_hash_abort` — two identical sims with same commands; assert
`state_hash()` equal at each tick; assert no abort condition fires.
**Risk:** Low — absence confirmed; the state hash is still valuable for debug.

### Handoff B — Frame-gate uses 4 stop flags, not hash
**Behavior:** `g_CurrentFrameCounter++` gates on 4 zero flags. Lockstep stall =
holding any flag. The Rust tick loop must block `advance_tick` while the net
layer is waiting for remote commands.
**Rust delta:** `src/sim/world/mod.rs` `advance_tick` needs a lockstep-gate
parameter or equivalent; calling `advance_tick` and then comparing hashes is
architecturally wrong.
**Affected surface:** `src/sim/world/mod.rs`, net integration layer.
**Acceptance:** Under artificial command-stall, local tick counter holds until
remote commands arrive.
**Test:** `test_lockstep_stall_blocks_advance_tick` — assert tick counter does
not advance when gate flag is set.
**Risk:** High if net layer assumes free-running ticks with post-hoc hash compare.

### Handoff C — Reconnect is pause-and-wait, not state-resend
**Behavior:** On peer drop, `FUN_006475F0` triggers a modal reconnect dialog
(`FUN_00648710`). During reconnect the game is locked (lockstep stall); normal
`Network_ServiceLoop` + render continues; peers send keepalive packets. On
reconnect success (return 0) the game resumes from the frame where the drop was
detected. No simulation state is transferred.
**Rust delta:** Rust net layer should implement a drop-recovery path that: (a)
halts `advance_tick`, (b) opens a reconnect wait loop with keepalive sends,
(c) supports kick-vote by majority, (d) resumes on success. No snapshot/state-
resend is needed to match gamemd behavior.
**Affected surface:** Future net layer, sim gate integration.
**Acceptance:** Simulated peer drop → local game pauses, reconnect UI shows,
resumes on peer return; on majority kick, dropped player removed cleanly.
**Test:** `test_reconnect_pause_and_resume` — simulate peer timeout; assert tick
counter frozen; simulate peer return; assert tick counter resumes.
**Risk:** Medium — not implementing the reconnect wait loop means any disconnect
immediately aborts the game, losing parity with gamemd's "wait and vote" behavior.

---

## 8. Negative Facts / Do Not Do

1. **DO NOT treat `Desync_Handler @ 0x0048DC90` as the live-MP desync detector.**
   It is a selection-list clear. Rename target: `Selection__ClearAll`. All callers
   are UI/selection paths. (verified via `decompile_function 0x0048DC90` +
   `get_function_callers 0x0048DC90`)

2. **DO NOT implement a per-frame state-hash-compare → abort loop in the net layer.**
   No such mechanism exists in gamemd LAN/WOL modes. (verified via
   `decompile_function 0x0055D360` + `0x0048D080` + `0x0048D1E0`)

3. **DO NOT conflate the recording/replay sync path with live-MP desync.** The
   `DAT_00A8D5F8 & 1/2` recording/replay flags are zero in all live skirmish/MP
   modes. (verified via `decompile_function 0x0055D360`)

4. **DO NOT implement reconnect as a state-resend / snapshot-transfer.** gamemd
   reconnect is pause-and-wait only; no simulation state is transferred to the
   returning peer. (verified via `decompile_function 0x00648710`)

5. **DO NOT treat the mode-2 CRC-mismatch handler (`FUN_00643C50`) as a disconnect.**
   It is a speed-ratio tracker update. Mode 2 (IPX/serial) is not active in standard
   YR LAN/WOL anyway. (verified via `decompile_function 0x00643C50`, `0x0048D080`)

---

## 9. Remaining Uncertainty

1. **WOL SDK plug-in vtable call in `Network_ServiceLoop`:** The
   `(*(*DAT_00B45B5C + 8))()` call at `0x0048D094` is a plug-in object vtable slot
   2. Whether this slot performs any out-of-band desync notification for WOL is
   unknown (object type and vtable content untraced). Low priority — WOL servers
   are offline; LAN (mode 3) has no equivalent call.

2. **`FUN_005422D0` semantics at scale:** This is the "have all remote players
   committed?" check in `FUN_0048D1E0`. Full parameter layout not decoded;
   for 30 players, the polling loop's complexity is unknown. Medium priority for
   VERA20k net-layer design.

3. **Reconnect timeout value:** `FUN_006475F0` reads the timeout from
   `DAT_00837348 + g_GameMode*0x10` (a table). The table value for mode 3 (LAN)
   has not been read from memory. Relevant for matching gamemd's reconnect wait
   duration. Low priority — behavior is "wait with modal dialog", duration is tunable.

4. **`DAT_00A8B8B8` and `DAT_00A8B8BC` semantics:** These globals are set/cleared
   around the reconnect and MP session flags. Their exact role in the session-state
   machine was not fully decoded. Referenced in `SESSIONCLASS_GHIDRA_REPORT.md`
   as deferred items.

---

## 10. Sources

**Ghidra addresses decompiled this session:**
- `0x0048DC90` `Desync_Handler` (mislabeled selection-clear) — confirmed
- `0x0055D360` `Main_Tick` — live-MP tick sequence verified
- `0x0048D080` `Network_ServiceLoop` — no hash compare for modes 3/4 confirmed
- `0x0048D1E0` `FUN_0048D1E0` — command-queue drain, per-frame lockstep handshake
- `0x006475F0` `FUN_006475F0` — reconnect trigger / per-frame network orchestrator
- `0x00648710` `FUN_00648710` — reconnect modal dialog loop
- `0x0064AAE0` `FUN_0064AAE0` — propose-kick sender
- `0x0064AD10` `FUN_0064AD10` — record-kick-vote receiver
- `0x00643C50` `FUN_00643C50` — speed-ratio tracker (NOT disconnect)
- `0x005422D0` `FUN_005422D0` — "all remote players committed?" check
- `0x005410F0` `FUN_005410F0` — network send wrapper

**get_function_callers verified:**
- `0x004DBAD0` `FootClass__ComputeChecksum` — one caller (`0x00744640`), that has zero callers
- `0x00744640` `FootClass__Save_Convoy_State` — zero callers (TS dead code)
- `0x0048DC90` `Desync_Handler` — callers enumerated; none is a live-MP hash comparator
- `0x00648710` — one caller: `FUN_006475F0`

**search_strings verified:**
- `"reconnect"` → 16 strings, addresses `0x008348C8`–`0x00837A30`

**get_xrefs_to verified:**
- `0x008348C8` (`STT:ReconnectButtonPlayer`) → `FUN_006040B0` (tooltip dispatcher)
- `0x00837700` (`TXT_RECONNECTING_TO`) → `FUN_00648710` (×2)
- `0x008379E8` (`TXT_RECONNECT_KICK_SELF`) → `FUN_0064AAE0`
- `0x00837A30` (`TXT_RECONNECT_KICK_RECEIVED`) → `FUN_0064AD10`

**Related docs:**
- `RNG_SYSTEM_GHIDRA_REPORT.md` §5.2–5.3 — per-frame CRC hash (recording/replay,
  `*ComputeCRC` chain dead in live play)
- `DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md` — full replay-path analysis,
  `FUN_0048D1E0` command-queue drain
- `SESSIONCLASS_GHIDRA_REPORT.md` — `DAT_00A8B8B8`, session globals
- `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md` — case 0xB reconnect/post-disconnect

---

## Stale-Doc Note

`DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md` §6 mentions
`FUN_00643C50(1, ...)` as the mode-2 CRC-mismatch action and labels it as
"likely a UI notification." This session confirms `FUN_00643C50` is a speed-ratio
tracker (`_g_ImpassableSpeedThreshold_0_01` multiply, `SendMessageA(0xF)` repaint).
It is not a disconnect or UI notification. The mode-2 CRC mismatch path is
effectively dead in standard YR (mode 2 = IPX/serial). The stale description
in `DESYNC_DETECTION_MAINTICK_COMPARE_GHIDRA_REPORT.md` §9 ("behavior unknown,
low priority") remains accurate in its conclusion — the mechanism is dead — but
the "UI notification" inference should not be propagated.
