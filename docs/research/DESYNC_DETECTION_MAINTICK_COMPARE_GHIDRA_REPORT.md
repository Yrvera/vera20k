# Desync Detection in Main_Tick — Ghidra Research Report

**Date:** 2026-05-28  
**Target addresses:** `Main_Tick @ 0x0055D360`, `Desync_Handler @ 0x0048DC90`,
`Network_ServiceLoop @ 0x0048D080`, `FUN_0048D1E0` (MP network inner loop)  
**Confidence:** HIGH for all verified findings (direct decompilation).  
**Active in YR:** Per-finding.  
**Slot:** 4 of /re-swarm desync-detection batch.

---

## 0. Investigation Scope

**Target question:** How does gamemd compare per-frame sync checksums between
clients in live multiplayer (cadence, comparison site, on-mismatch behavior)?
Is `Desync_Handler @ 0x0048DC90` the comparison site?

**Non-goals:** Checksum computation (slot 1 / `RNG_SYSTEM_GHIDRA_REPORT.md` §5.2–5.3),
RNG state (slot 2/3), PerTickUpdate order (slot 5).

---

## 1. MISLABEL CONFIRMED: `Desync_Handler @ 0x0048DC90` is NOT a network comparator

**Verified via `decompile_function 0x0048DC90`:**

```c
void Desync_Handler(void) {
  while (g_CurrentObjects_Count != 0) {
    (**(code **)(*(int *)*g_CurrentObjects_Data + 0x150))();
  }
  Selection__ResetMode();
  FUN_00431060();   // clears "selected" flag bits in a grid structure
  return;
}
```

This function:
- Iterates the `CurrentObjects` selected-object list, calling a vtable method per entry
- Calls `Selection__ResetMode()` — resets the selection UI mode
- Calls `FUN_00431060()` — clears bit 0x02 of field +0xC across an 8×3 grid of objects

**Callers (verified via `get_function_callers 0x0048DC90`):**
```
DisplayClass__BandBox_LeftUp @ 004ab9b0
FUN_00430f70 @ 00430f70       (proximity/radius check helper)
FUN_004aad30 @ 004aad30
FUN_004ac660 @ 004ac660
FUN_004ac700 @ 004ac700
FUN_004ac820 @ 004ac820
FUN_004ac8c0 @ 004ac8c0
FUN_004ac960 @ 004ac960
HouseClass__Begin_Building_Placement @ 004fb840
Main_Tick @ 0055d360          (replay playback branch only — see §3)
ObjectClass__Select @ 005f4520
SelectClass__Action @ 006aad00
```

All non-`Main_Tick` callers are selection/UI/building-placement paths.
`FUN_00430f70` calls it when a unit is within 128-lepton proximity of something
(a range/distance check for a tile-grid structure), which is a selection-assist
callback, not a network hash comparison.

**`Main_Tick` call site** (see §3): fires in the `DAT_00A8D5F8 & 2` (replay
**playback**) branch, when the summed selected-object checksum from the recorded
stream mismatches the live computed sum. This is a single-player recording-playback
divergence alert — NOT a live multiplayer network comparison.

**Active in YR:** Yes, but only as a selection-clear / selection-flag-reset helper.
It is NOT a network desync comparator in any mode.

---

## 2. REPLAY-ONLY DESYNC DETECTION in `Main_Tick`

**Verified via `decompile_function 0x0055D360` (full function).**

The sync-hash block is gated on `DAT_00A8D5F8`:

### 2.1 Recording path (`DAT_00A8D5F8 & 1`)

Each frame, when recording is active:
1. Read 8-byte `state_hash` from `MapClass` (via `FUN_006D6170(local_180)` — returns two DWORDs read from `Scen+0xD64/0xD68`).
2. Write 8 bytes to recording stream via `FUN_00473AE0(&local_1b4, 8)`.
3. Write `g_CurrentObjects_Count` (4 bytes).
4. Sum `g_CurrentObjects_Data[i]` entries (packed house+index word) → `local_1a4`.
5. Write summed value (4 bytes).
6. Write each individual packed entry (4 bytes each).
7. Write `DAT_00ABCDFC` and `DAT_00ABCE00` (8 bytes) then zero them.

**Active in YR:** Conditional — only when recording (`DAT_00A8D5F8 & 1`). Not active
in a standard skirmish or live MP game.

### 2.2 Playback path (`DAT_00A8D5F8 & 2`)

Each frame during replay:
1. Read 8 bytes from stream into `local_1b4` via `FUN_00473B10`. If exactly 8 bytes
   read, call `FUN_006D6000(&local_1b4)` — writes the 8-byte value back into the
   MapClass state-hash field (sets the expected hash for this frame).
2. Read `local_1a8` (expected selected-count from stream).
3. Compute live `local_1a4` (sum of current `CurrentObjects` packed entries).
4. Read expected sum from stream into `local_190`.
5. **COMPARE:** `if (local_190 != local_1a4) → Desync_Handler();`
6. For each expected object in the recorded stream, look up by packed ID and attempt
   to (re-)select it. If the ID resolves to an object AND the sum still mismatches:
   call vtable method `+0x14C` (a selection/voice callback) and clear
   `g_SelectionVoice_Enable`.
7. Read `DAT_00ABCDFC` and `DAT_00ABCE00` back from stream. Call `FUN_004F42F0(0)`.
8. Call `RenderFrame_main()`.

**On mismatch (replay mode):** `Desync_Handler()` fires — clears the selection list
and resets selection mode. This is a selection-state "reset on detected recording
divergence," not an abort, disconnect, or network message. **Active in YR:**
Conditional — replay playback only.

**Frame counter behavior:** During replay playback, `DAT_00A8D5F8 & 2` prevents
the normal gameplay path (`GScreenClass__Input`, `LogicClass__AI`, `Map__Logic`)
from running. The frame counter increment gate is also skipped because the function
returns before the `g_CurrentFrameCounter++` site. Cadence: **every frame** (no
every-N gating; the entire recording/playback block runs unconditionally when the
flag is set).

---

## 3. LIVE MULTIPLAYER: No per-frame hash comparison in `Main_Tick`

**Verified via `decompile_function 0x0055D360`.**

The normal `Main_Tick` flow for `g_GameMode == 3` (LAN) or `g_GameMode == 4` (WOL)
does NOT contain a hash comparison step. The tick sequence for live MP is:

1. `g_GameRunning == 0` wait loop → `Process_NetworkMessages()` (sleep 10ms, retry)
2. Frame-budget throttle calculation (network modes use different timer buckets —
   `DAT_00887350 = 0x3C / DAT_00A8B558`, `DAT_00887330 = 1000 / DAT_00A8B558`)
3. Mode-4 latency back-pressure: up to 3× `+10 ms` budget adjustments based on
   `g_NetworkFrameBudget` vs `DAT_00A8DB7C` array max
4. `GScreenClass__Input`
5. `LogicClass__AI`
6. Optional `House_AI_Tick`
7. Network keepalive every 8 frames (`if ((g_CurrentFrameCounter & 7) == 7) && g_GameMode == 4`)
8. `Map__Logic`
9. `RenderFrame_main`
10. `FUN_00551A30` (side work)
11. `LogicClassPerTickUpdateLiveVector`
12. Tactical service calls
13. `Network_ServiceLoop()`
14. Frame counter increment (gated on `DAT_00A83D49`, `DAT_00A8ECD0`, `DAT_008B41C0`, `DAT_00A83D48` all zero)
15. `FUN_0055E160` (throttle wait)

**There is no explicit hash compare or desync check in this path.** The live-MP
desync mechanism, if any, must live in `Network_ServiceLoop` or below.

---

## 4. `Network_ServiceLoop @ 0x0048D080` — No hash comparison

**Verified via `decompile_function 0x0048D080`:**

```c
void Network_ServiceLoop(void) {
  FUN_00406F70();
  if ((0 < DAT_00B45B68) && (*DAT_00B45B5C != 0))
      (*(*DAT_00B45B5C + 8))();    // vtable[2] call on an object
  if ((g_GameMode == 3) || (g_GameMode == 4))
      FUN_0048D1E0();              // MP inner loop
  if (g_GameMode == 1) { ... }    // modem path, gated on DAT_00A8DAB8 & 1
  else if (g_GameMode != 2) goto LAB_0048d166;
  // mode 2 (IPX?) path: reads events from FUN_005F1C30, handles type 0x6C
  // (CRC mismatch packet from remote — triggers FUN_00643C50)
  LAB_0048d166:
  // mode 4 WOL screen-size sync: FUN_007CA67F vs DAT_0089E9B0
  return;
}
```

The mode-2 (IPX/serial?) branch reads packets of type `0x6C` and checks if the
remote's CRC byte (`uStack_ec & 0xFF`) differs from local `DAT_00A8DB30`. If it
differs, calls `FUN_00643C50(1, ...)`. **Active in standard YR LAN/WOL skirmish:**
**Conditional** — mode 2 is not LAN (mode 3) or WOL (mode 4); this is a legacy
serial/modem/IPX path.

For modes 3/4, `FUN_0048D1E0` is called.

---

## 5. `FUN_0048D1E0` — MP inner frame-synchronization loop

**Verified via `decompile_function 0x0048D1E0`.**

This is the lockstep frame-synchronization protocol for modes 3 and 4. Key behavior:

- Waits until the expected frame budget timer (`DAT_0089E928` = `DAT_0081D4E4`) elapses.
- When `DAT_00A8DA84` (player count?) > 0 and `FUN_005422D0()` == 0 (all remote
  players have committed commands for this frame):
  - Sends a "ready for next frame" packet (`local_1d0 = 0x29`) to each remote.
  - For each remote player, times out with a 120-tick (≈2 s at 60 fps) deadline
    waiting for their ack.
  - On timeout: checks remote frame-number `*(ushort *)(player_obj + 0x83)` vs
    local `DAT_00A8B2B2` ("current frame"); if mismatched AND remote frame >
    `0x3FF` → resends `FUN_005410F0(&local_1d0, 0x5b)`.
- **Switch on `DAT_00A8D638`** (event type): handles cases including:
  - `0x0C` — "place building" command received; fires VocClass play + radar update
  - `0x0F` — player drop / leave notification: `FUN_00643C50(player_idx)` (drop player)
  - `0x1C` — `FUN_0064AC90()`
  - `0x1D` — taunt voice
  - `0x20` — beacon placement
  - `0x21`, `0x22` — `FUN_004311C0`, `FUN_00431450`
  - `0x27` — player kick (WOL mode 4, lobby player)
  - `0x29` — per-player frame-number sync update
  - `0x2F` — WOL game-speed change

**No hash comparison in `FUN_0048D1E0`.** The function implements command-queue
draining and lockstep frame synchronization (all clients must commit commands before
the frame advances), not a state-hash comparison.

**Active in YR:** Yes — `g_GameMode 3` (LAN) and `4` (WOL).

---

## 6. On-mismatch behavior: What gamemd does when a state divergence is detected

Based on the full decompilation:

| Mode | Detection site | Trigger | On-mismatch action |
|---|---|---|---|
| Replay playback (`DAT_00A8D5F8 & 2`) | `Main_Tick` selection-sum compare | `local_190 != local_1a4` (selection-object checksum mismatch) | `Desync_Handler()` = clear selection list + reset selection mode. **No abort. No disconnect. Log-only level effect.** |
| IPX/serial (mode 2) | `Network_ServiceLoop` | Remote CRC byte `!= DAT_00A8DB30` | `FUN_00643C50(1, ...)` — address not fully traced; likely a UI notification |
| LAN (mode 3) / WOL (mode 4) | NOT FOUND in any traced path | — | **No explicit per-frame state-hash comparison found** |

**Finding: gamemd does NOT have a live per-frame state-hash comparison in standard
LAN/WOL multiplayer.** The lockstep mechanism (`FUN_0048D1E0`) ensures all clients
commit the same input commands before each frame advances; divergence is prevented
by command-queue synchronization, not detected by hash comparison. The state hash
written during recording (`DAT_00A8D5F8 & 1`) is for replay verification only.

**Active in YR:** The live-MP desync-abort path does NOT exist in standard LAN/WOL
mode. The recording/replay compare fires only when `DAT_00A8D5F8 & 2`.

---

## 7. Implementation Handoff

### Handoff A — Desync detection is NOT needed in the Rust net layer (for parity)
**Verified behavior:** gamemd has no per-frame hash comparison in live LAN/WOL MP.
Lockstep is enforced by command-queue synchronization (all clients commit before
advancing), not by hash-compare-on-mismatch.  
**Rust delta:** `Simulation::state_hash` in `src/sim/world/world_hash.rs` is correct
as a replay verification tool, but the Rust net layer should NOT implement a
"hash mismatch → abort session" loop expecting to mirror a gamemd mechanism — no
such mechanism exists in live MP.  
**Affected surface:** `src/sim/world/world_hash.rs`, future net layer.  
**Acceptance scenario:** LAN game completes without false-positive desync aborts.  
**Proposed test:** `net_lockstep_no_spurious_hash_abort` — run two sims to same
tick with identical commands, assert `state_hash()` equal; no abort fires.  
**Risk:** Low — absence of a gamemd mechanism is confirmed; the Rust hash is still
valuable for replay tooling and internal consistency checks.

### Handoff B — Replay verification uses selection-object sum, not the 8-byte MapClass hash alone
**Verified behavior:** The recording write path (recording `& 1`) writes (1) the 8-byte
`MapClass` state-hash, (2) `g_CurrentObjects_Count`, (3) sum of selected-object
packed IDs, (4) individual packed IDs, (5) `DAT_00ABCDFC/00` (two 4-byte values).
The playback compare (`& 2`) only compares the summed selected-object value; it does
NOT compare the 8-byte `MapClass` hash — that is written back into `MapClass` as the
"expected" value without comparison.  
**Rust delta:** If replay verification is implemented, the comparison should be on the
selection-object sum (which maps to `g_CurrentObjects_Count` + sum of entity IDs in
the current-selection list), not on the raw 64-bit state hash.  
**Affected surface:** Future replay/recording infrastructure.  
**Acceptance scenario:** Replay playback of a recorded session matches original without
spurious `Desync_Handler` fires.  
**Proposed test:** `replay_selection_sum_matches_recording` — record 10 ticks, play
back, assert selection sum matches each frame.  
**Risk:** Medium — the 8-byte hash passthrough (write-back without compare) means the
MapClass hash is not verified during replay; only selection state is compared.

### Handoff C — Lockstep frame advance is gated by four stop flags, not a hash compare
**Verified behavior:** `g_CurrentFrameCounter++` in `Main_Tick` is gated by
`DAT_00A83D49 == 0 && DAT_00A8ECD0 == 0 && DAT_008B41C0 == 0 && DAT_00A83D48 == 0`.
When any flag is set, the frame counter does NOT advance and the function returns with
a "game paused/stopped" value. Lockstep stall is implemented by holding these flags
until all clients have committed — not by hash mismatch.  
**Rust delta:** The Rust `advance_tick` should respect an equivalent 4-flag pause
gate. Any net-layer pause (waiting for remote commands) must block `advance_tick`,
not call `advance_tick` and then compare hashes.  
**Affected surface:** `src/sim/world/mod.rs` `advance_tick`, net integration layer.  
**Acceptance scenario:** Under artificial command-stall (remote player delay), local
simulation holds at same tick until remote commands arrive.  
**Proposed test:** `lockstep_stall_blocks_advance_tick` — assert tick counter does
not advance when a pause flag equivalent is set.  
**Risk:** High if net layer is built around hash-compare-then-abort instead of
command-gate-then-advance.

---

## 8. Negative Facts / Do Not Do

1. **DO NOT chase `Desync_Handler @ 0x0048DC90` as the live-MP desync detector.**
   It is a selection-list clear helper. Callers: all selection/UI/building-placement
   paths. `Main_Tick` calls it only in replay-playback mode on selection-sum mismatch.
   *(verified via `decompile_function 0x0048DC90` + `get_function_callers 0x0048DC90`)*

2. **DO NOT implement a per-frame hash-compare → abort loop to mirror gamemd.**
   No such loop exists in `Main_Tick` or `Network_ServiceLoop` for LAN/WOL modes.
   *(verified via `decompile_function 0x0055D360` + `decompile_function 0x0048D080`)*

3. **DO NOT conflate the recording/replay desync path with live-MP desync.**
   Recording (`DAT_00A8D5F8 & 1`) and replay-playback (`& 2`) are single-player tools;
   they are absent in modes 3/4. Standard skirmish vs AI uses mode 0 or 5, neither of
   which hits the recording block in normal play.

4. **DO NOT assume the 8-byte MapClass hash is "compared" during replay.**
   The playback path writes it back into MapClass (`FUN_006D6000`) without any
   comparison; only the selection-object sum is compared.

5. **DO NOT treat `FUN_0048D1E0` as a hash-comparison function.**
   It is the command-queue drain and per-frame lockstep handshake for modes 3/4;
   its "sync" is command-event based, not state-hash based.
   *(verified via `decompile_function 0x0048D1E0`)*

---

## 9. Remaining Uncertainty

- **What `FUN_00643C50` does on CRC mismatch in mode-2 path:** The IPX/modem/serial
  mode-2 packet type `0x6C` mismatch calls `FUN_00643C50(1, value, -1, -1)`. This is
  not the LAN/WOL path, but its behavior (UI notification vs disconnect) is untraced.
  Low priority — mode 2 is not active in standard YR LAN/WOL.

- **Whether the WOL (mode 4) path has a separate out-of-band desync notification**
  via the WOL SDK (the vtable call `(*DAT_00B45B5C + 8)()` in `Network_ServiceLoop`
  before the mode check). This is a plug-in object vtable; its desync-reporting
  behavior is unknown. Moderate priority for WOL parity; not relevant for LAN mode 3.
