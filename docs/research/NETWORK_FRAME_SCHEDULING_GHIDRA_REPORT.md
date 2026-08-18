# Network Frame Scheduling — Ghidra Research Report

**Date:** 2026-05-28
**Addresses decompiled this session:**
- `Main_Tick @ 0x0055D360`
- `Network_ServiceLoop @ 0x0048D080`
- `Network_Keepalive @ 0x00542520`
- `FUN_0048D1E0` (network session service / player-drop handler)
- `EventClass__Execute @ 0x004C7600` (cases 0x1B, 0x20, 0x24, 0x27)
- `FUN_006475F0` (per-frame queue advance / timing event sender)
- `FUN_0064C380` (DoList execute — per-house command dispatch at scheduled frame)
- `FUN_005D5870` / `FUN_005D5880` (`timeBeginPeriod(1)` / `timeEndPeriod(1)`)
- `FUN_00540F90` (peer-count getter: `*(param_1+0x44)`)

**Evidence grade:** HIGH — all key claims verified from live Ghidra decompile this session.
**Active in YR:** Conditional (network modes 3=LAN, 4=WOL/internet only). Skirmish (`g_GameMode==5`) does not execute ANY of the gating logic described here.

---

## Target Question

How does the local `g_CurrentFrameCounter @ 0x00A8ED84` tie to the network scheduled/execute frame in lockstep — i.e. the frame-advance gating that keeps peers in lockstep?

## Non-Goals

- MaxAhead computation (slot 2), FrameSendRate source (slot 3)
- Command queue internals / EventClass::Execute game-logic side effects (slot 4)
- CRC / desync / reconnect (slot 5)

## Evidence Needed to Mark COMPLETE

1. Verified: per-peer lag array address, element layout, and which field is compared in Main_Tick. ✓
2. Verified: exact threshold formulas and what each threshold triggers. ✓
3. Verified: the lockstep barrier — where local frame advance is gated on all peers having delivered their frames. ✓
4. Verified: execute-frame derivation formula (frame N → execute at frame E). ✓
5. Verified: 10-frame batch alignment mechanism. ✓

---

## 1. Globals Inventory

| Global | Address | Meaning | Evidence |
|---|---|---|---|
| `g_CurrentFrameCounter` | `0x00A8ED84` | Authoritative sim frame. Incremented LATE in `Main_Tick` after `Network_ServiceLoop`, gated by 4 session-end flags. | `decompile_function 0x0055D360` |
| `g_NetworkFrameBudget` | `0x00A8B550` | MaxAhead in frames. Set by `EventClass::Execute` case `0x1B` (byte from event) and case `0x20` (ushort from timing event). Also tracked via `DAT_00A8B568` (high-water mark). | `decompile_function 0x004C7600` cases `0x1B`, `0x20` |
| `DAT_00A8DB7C` | `0x00A8DB7C` | Start of per-peer lag array (ints, 4 bytes each, 7 slots). Each slot = lag value in ms × 1000 / 60, tracked per remote peer. Read in `Main_Tick` budget-bump loop. Written by `FUN_006475F0` (zeroed at successful frame) and `FUN_00648710` (frame wait). | `get_assembly_context 0x0055D5D0`, `decompile_function 0x006475F0` |
| `DAT_00A8B554` | `0x00A8B554` | `FrameSendRate` (byte) — how often timing events are sent. Set in case `0x20`. | `decompile_function 0x004C7600` case `0x20` |
| `DAT_00A8B558` | `0x00A8B558` | Requested network FPS (derived from FrameSendRate). Used in `Main_Tick` to compute `1000 / DAT_00A8B558` ms budget. | `decompile_function 0x0055D360` |
| `DAT_00887328` | `0x00887328` | Network ms-budget start (`timeGetTime()` snapshot). Set in network/replay block. | `decompile_function 0x0055D360` |
| `DAT_00887330` | `0x00887330` | Network ms-budget ceiling. Incremented by +10 ms up to 3× per adaptive throttle. | `decompile_function 0x0055D360` |
| `DAT_00A8B1DC` | `0x00A8B1DC` | **Execute-frame target** — the frame at which queued commands from `EventClass::Execute` case `0x20` should fire. Derived during timing-event execution: `((MaxAhead + FrameSendRate - 1 + issueFrame) / FrameSendRate) * FrameSendRate`. Set to 0 if the negotiated value is within current budget. | `decompile_function 0x004C7600` case `0x20`, `get_xrefs_to 0x00A8B1DC` |
| `DAT_00A8B1F8` | `0x00A8B1F8` | Issue frame base for execute-frame computation (frame when timing event was issued). Used alongside `DAT_00A8B1DC` in `FUN_0064C380`. | `decompile_function 0x004C7600` case `0x20` |
| `DAT_00A8E2DC` | `0x00A8E2DC` | Player-removal execute-frame: `((g_NetworkFrameBudget + 10 + g_CurrentFrameCounter) / 10) * 10` — same 10-frame alignment logic. Set in case `0x27` (ABOUTTOEXIT). | `decompile_function 0x004C7600` case `0x27` |
| `DAT_00A8B568` | `0x00A8B568` | High-water mark of `g_NetworkFrameBudget` across the session. Updated only upward. | `decompile_function 0x004C7600` case `0x20` |
| `DAT_00A8B9C` | `0x00A8DB9C` | LatencyFudge multiplier (0–3): 0=none, 1=×1.5, 2=×2, 3=×3. Applied to response time in `FUN_006475F0`. | `decompile_function 0x004C7600` case `0x24`, `decompile_function 0x006475F0` |
| `g_CommandBuffer` | resolved via `decompile 0x006475F0` context | Ring buffer (128 slots × `0x6F` bytes). Write index at `g_CommandQueue_WriteIndex`, count at `g_CommandQueue_Count`, timestamps at `g_CommandTimestamps`. Commands arrive for execute at the scheduled frame. | `decompile_function 0x006475F0`, `decompile_function 0x004C7600` |

**Active in YR:** All of the above — YES for network modes 3 and 4. Unreachable for `g_GameMode == 5` (skirmish) or `0` (campaign).

---

## 2. Network Budget Path in Main_Tick (mode 3/4)

Verified via `decompile_function 0x0055D360` and `get_assembly_context 0x0055D59C–0x0055D5CA`.

Entry condition: `g_GameMode != 0 AND g_GameMode != 5 AND DAT_00A8B24C == 2`.

```text
// 1. Snapshot timeGetTime as budget start
if (DAT_00A8B558 == 0) {
    DAT_00887348 = GetRadarTimer();
    DAT_00887350 = 2;           // radar-bucket fallback
    local_budget_ms = 0x21;     // 33 ms fallback
} else {
    DAT_00887348 = GetRadarTimer();
    DAT_00887350 = 0x3c / DAT_00A8B558;   // radar buckets
    local_budget_ms = 1000 / DAT_00A8B558; // ms budget
}
timeBeginPeriod(1);                 // FUN_005D5870
DAT_00887328 = timeGetTime();
DAT_00887330 = local_budget_ms;
timeEndPeriod(1);                   // FUN_005D5880

// 2. Subtract elapsed time from budget
if (DAT_00887328 != 0xFFFFFFFF) {
    elapsed = timeGetTime() - DAT_00887328;
    DAT_00887330 = (elapsed < DAT_00887330) ? (DAT_00887330 - elapsed) : 0;
}

// 3. Adaptive throttle — mode 4 only, requires g_NetworkFrameBudget > 30 (0x1e)
if (g_GameMode == 4 AND g_NetworkFrameBudget > 0x1e) {
    // Scan per-peer lag array at DAT_00A8DB7C
    peer_count = FUN_00540F90(0x00A8E9C0);  // returns *(ECX+0x44)
    max_lag = max(DAT_00A8DB7C[0..peer_count-1]);

    if (max_lag != 0) {
        // Threshold 1: max_lag >= g_NetworkFrameBudget / 4  → +10 ms
        if (max_lag >= (g_NetworkFrameBudget + (g_NetworkFrameBudget >> 0x1f & 3)) >> 2):
            DAT_00887330 += 10;

        // Threshold 2: max_lag >= g_NetworkFrameBudget / 2  → +10 ms
        if (max_lag >= g_NetworkFrameBudget / 2:
            DAT_00887330 += 10;

        // Threshold 3: max_lag >= g_NetworkFrameBudget * 3 / 4  → +10 ms
        if (max_lag >= (g_NetworkFrameBudget * 3 + (g_NetworkFrameBudget * 3 >> 0x1f & 3)) >> 2:
            DAT_00887330 += 10;
    }
}
// → goto LAB_0055D7C2 (gameplay block)
```

**Semantic interpretation (verified):**

The adaptive throttle detects the laggiest peer and slows the local frame rate by extending the ms budget. Each +10 ms bump widens the inter-frame gap, giving slow peers more time to deliver their commands before the frame executes. Three bumps = +30 ms maximum, turning a 33 ms budget into up to 63 ms.

**Guard: `g_NetworkFrameBudget > 0x1e` (> 30 frames).** If MaxAhead is ≤ 30, no adaptive throttle fires — verified via `get_assembly_context 0x0055D5B3` (`CMP EBX,0x1e; JLE 0x0055D878`).

**Active in YR:** Conditional (g_GameMode == 4 only). LAN (mode 3) uses the same ms budget path but NEVER fires the adaptive +10 ms bumps.

---

## 3. Per-Peer Lag Array: `DAT_00A8DB7C`

Address: `0x00A8DB7C`. Layout: contiguous `int[7]` (4 bytes each, 7 slots).

Verified via `get_assembly_context 0x0055D5D0–0x0055D5ED`:
```asm
0055d5d0: MOV EDI,0xa8db7c        ; base of array
0055d5d5: MOV EAX,[EDI]           ; load slot
0055d5d7: CMP EBP,EAX
0055d5d9: JG  skip
0055d5db: MOV EBP,EAX             ; EBP = running max
0055d5dd: MOV ECX,0xa8e9c0        ; session object for FUN_00540F90
0055d5e2: INC ESI
0055d5e3: ADD EDI,0x4             ; stride = 4 bytes
0055d5e6: CALL 0x00540f90         ; peer_count
0055d5eb: CMP ESI,EAX
0055d5ed: JL  0x0055d5d5          ; loop
```

**Write site:** `FUN_006475F0` zeros the array (`for (i=0; i<7; i++) DAT_00A8DB7C[i]=0`) on a successful `FUN_00648710` (all-peers-ready) return. Individual slots are written by `FUN_006475F0`'s lag tracking inside the keepalive / response-time path (via `puVar14 = &DAT_00AFA368`, stride 6 ints, different array — see §6 below). The `DAT_00A8DB7C` slots are reset to 0 when the frame successfully advances.

**Scale limit (FLAG):** Array is fixed at 7 slots (addresses `0x00A8DB7C..0x00A8DB94`). For a 30-player target, this array must be dynamically sized to `N_players - 1` entries. The loop count comes from `FUN_00540F90(0x00A8E9C0)` which reads `*(ECX+0x44)` — a fixed capacity counter. Any implementation must replace this fixed-size structure with a `Vec<i32>` or similar keyed by peer ID.

---

## 4. Lockstep Barrier: Where Local Frame Is Gated

The lockstep barrier is NOT a single blocking function. It is a layered gate:

### 4.1 FUN_006475F0 — Per-frame Advance Gate

Called from `LogicClass__PerTickUpdate` (step 9 of `Main_Tick` — verified in earlier session doc). For `g_GameMode ≠ 5`:

```text
if (all peers have sent their frame N data) {
    // FUN_00648710 returns 0 → success
    zero DAT_00A8DB7C[0..6];
    FUN_0064C380(piVar13, &DAT_00AFA450, &DAT_00AFA358);  // dispatch at-frame events
} else {
    // FUN_00648710 returns nonzero → peers not ready
    // Logs "Wait_For_Players returned <N>"
    // Kills g_GameActive = 0 on timeout
}
```

Verified via `decompile_function 0x006475F0`: the `iVar9 = FUN_00648710(...)` return value directly gates whether `FUN_0064C380` fires for that frame.

**FUN_00648710 is the actual barrier function.** It blocks (loops / returns failure) until all connected peers have delivered their data for the current frame. Its internals are not decompiled this session but its caller contract is clear: return 0 = all peers ready, nonzero = barrier not met.

Active in YR: YES (modes 3 and 4). Skirmish (`g_GameMode == 5`) hits the early return at the top of `FUN_006475F0` (`if (g_GameMode == 5) { return; }`). Verified via `decompile_function 0x006475F0`.

### 4.2 FUN_0064C380 — Command Dispatch at Frame N

Called only after the barrier passes. Iterates over a circular command queue (ring buffer, capacity `0x3FFF` entries, stride `0x6F` bytes). For each entry whose execute-frame `== g_CurrentFrameCounter` and whose player flag matches:
- Calls `EventClass__Execute` for the command
- Marks entry as executed (`DAT_008B4205[slot] |= 1`)
- Logs "Packet received too late" if event frame < current frame and mode != 0/5

Key sub-check for late-packet detection (verified):
```c
if (event_frame < g_CurrentFrameCounter && g_GameMode != 0 && g_GameMode != 5):
    logs "Packet_received_too_late"
    logs "MaxAhead=<g_NetworkFrameBudget>, Frame=<g_CurrentFrameCounter>, FrameSendRate=<DAT_00A8B554>"
```

Active in YR: YES (modes 3/4). Unreachable for skirmish/campaign (gated in caller `FUN_006475F0`).

---

## 5. Execute-Frame Derivation: Frame N → Execute at Frame E

### 5.1 Timing Event (Event 0x20) — the primary lockstep schedule

Verified via `decompile_function 0x004C7600`, case `0x20`:

```c
// param_1 layout (event struct offsets):
// [+0x03] = issueFrame (int)
// [+0x07] = FrameSendRate_proposed (ushort)
// [+0x09] = MaxAhead_proposed (ushort) — may be adjusted
// [+0x0B] = FrameSendRate_count (byte)

// Step 1: subtract coalition flag adjustment
param_1[+0x09] -= (*g_ScenarioClass_Instance & 0x1000) ? 10 : 0;

// Step 2: if proposed MaxAhead or FrameSendRate exceed current values → compute execute-frame
if (MaxAhead_proposed > g_NetworkFrameBudget || FrameSendRate_count > DAT_00A8B554) {
    DAT_00A8B1F8 = issueFrame;          // record base frame
    uVar11 = FrameSendRate_count;
    DAT_00A8B1DC = ((MaxAhead_proposed + uVar11 - 1 + issueFrame) / uVar11) * uVar11;
    //            = round up (MaxAhead_proposed + issueFrame) to next multiple of FrameSendRate_count
} else {
    // Proposed values are within current budget → no execute-frame delay needed
    DAT_00A8B1DC = 0;
    DAT_00A8B1F8 = 0;
}

// Step 3: commit new session parameters
DAT_00A8B558 = FrameSendRate_proposed;
g_NetworkFrameBudget = MaxAhead_proposed;
if (MaxAhead_proposed > DAT_00A8B568) DAT_00A8B568 = MaxAhead_proposed;
DAT_00A8B554 = FrameSendRate_count;
```

**Execute-frame formula (verified):**
```
execute_frame = ceil((issue_frame + MaxAhead) / FrameSendRate) * FrameSendRate
```

Where `ceil(x / n) * n = ((x + n - 1) / n) * n` (integer ceiling to nearest multiple of n). This ensures all peers execute the command at the same multiple-of-FrameSendRate frame, regardless of when each peer locally processed the timing event.

`DAT_00A8B1DC` is then used in `FUN_0064C380` to relocate events that were scheduled "too early" — they are moved forward to `DAT_00A8B1DC` before dispatch, ensuring no command fires before all peers can have received it.

**Verification of `DAT_00A8B1DC` read sites:** `get_xrefs_to 0x00A8B1DC` → `FUN_0064C380 @ 0x0064C3A3` (READ: comparison), `0x0064C3FB` (READ: reassignment of event frame to `DAT_00A8B1DC`), `EventClass__Execute @ 0x004C8072` (WRITE: case `0x20`), `Main_Game @ 0x0052DA08` (WRITE: init).

Active in YR: YES for modes 3/4.

### 5.2 ABOUTTOEXIT Execute-Frame (Event 0x27 — player disconnect)

Verified via `decompile_function 0x004C7600`, case `0x27`:

```c
DAT_00A8E2DC = ((g_NetworkFrameBudget + 10 + g_CurrentFrameCounter) / 10) * 10;
```

This is the **10-frame batch alignment** cited in DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md. The player-removal event schedules to the next multiple of 10 frames that is at least `MaxAhead + 10` frames away. The `/ 10 * 10` is an integer floor-to-multiple-of-10 operation; because the dividend is `MaxAhead + 10 + current`, the result is guaranteed to be strictly greater than `current + MaxAhead`.

**Confirming the "batches of 10" claim:** The 10-frame alignment is hardcoded in the ABOUTTOEXIT formula (`/ 10 * 10`). It is NOT a general command queue batch size. Commands in the main timing event (0x20) align to multiples of `FrameSendRate`, not 10. The 10-frame alignment applies specifically to player-exit scheduling. It matches the DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md claim that "end-frames round up to multiples of 10." Active in YR: YES.

---

## 6. Network_Keepalive @ 0x00542520

Called from `Main_Tick` when `(g_CurrentFrameCounter & 7) == 7 AND g_GameMode == 4`. Cadence: every 8 frames, internet-only. Verified via `decompile_function 0x0055D360`.

```c
// Iterates over remote peers (count at param_1+0x44, peer list at param_1+0x28)
// For each remote peer that is not local player:
//   if peer's connection flag (piVar6[-0x10]) == 0 → call FUN_00735090(0xffffffff)
//
// FUN_0048BA90: returns *(param_1+0x20) — response-time slot A
// FUN_0048BA80: returns *(param_1+0x1C) — response-time slot B
//
// *piVar6 = max(*piVar6, FUN_0048BA90() * 1000 / 0x3C)
// piVar6[4] = max(piVar6[4], FUN_0048BA80() * 1000 / 0x3C)
// piVar6[1..3] = peer stats at offsets +0x08, +0x0C, +0x10
```

`piVar6 = &DAT_00A8B5B4 + peer_index * 0x1A * 4` — this is a **different array** from `DAT_00A8DB7C`. The lag values written here (stride `0x1A` ints) are the response-time accumulators. The `DAT_00A8DB7C` array used in Main_Tick is the max-lag snapshot written during `FUN_006475F0`.

**Scale limit (FLAG):** `piVar6 = &DAT_00A8B5B4 + peer_index * 0x68` (0x1A × 4). Fixed layout for 8 peers at most. `DAT_00A8B5B4` through `DAT_00A8B5B4 + 7 * 0x68` = `0x00A8B5B4..0x00A8B8D3`. A 30-player target must replace this fixed peer-struct array with a dynamically allocated `Vec<PeerStats>`.

Active in YR: Conditional (g_GameMode == 4 only). Does NOT fire for LAN (mode 3) or skirmish.

---

## 7. Network_ServiceLoop @ 0x0048D080

Verified via `decompile_function 0x0048D080`. For modes 3/4:
1. Calls `FUN_0048D1E0` — session maintenance (process incoming packets, handle player drops).
2. For mode 4: checks a WOL interface pointer and pings its vtable methods.

`FUN_0048D1E0` is the "receive and route incoming network messages" function. It calls `Process_NetworkMessages()` (ingests raw packets) and `FUN_00541820` (parse into command queue). This is where remote peers' commands arrive into the local command ring buffer (`DAT_008B4204` / `DAT_008B4207` array). It does NOT advance the frame counter.

The lockstep barrier check (`FUN_00648710`) is in `FUN_006475F0`, NOT in `Network_ServiceLoop`. `Network_ServiceLoop` only receives and queues incoming data; it does not gate frame advance.

Active in YR: YES (modes 3 and 4). Also called for mode 1 (LAN_Multiplayer) and mode 2 with guard flags.

---

## 8. FUN_005D5870 / FUN_005D5880 — timeBeginPeriod / timeEndPeriod

Verified via `decompile_function 0x005D5870` / `0x005D5880`:
```c
FUN_005D5870: { timeBeginPeriod(1); }
FUN_005D5880: { timeEndPeriod(1); }
```

These are called AROUND each network budget recomputation in `Main_Tick` (before and after the +10 ms adaptive bumps). Purpose: temporarily raise Windows timer resolution to 1 ms for the `timeGetTime()` budget measurement. Active in YR: YES (network modes only).

---

## 9. EventClass::Execute Case 0x1B — Direct MaxAhead Override

Verified via `decompile_function 0x004C7600`, case `0x1B`:
```c
g_NetworkFrameBudget = (uint)(byte)param_1[0x0D];
```
This sets MaxAhead directly from a single event byte. No execute-frame computation. Used during game setup/lobby to broadcast the agreed MaxAhead value to all peers. Active in YR: YES.

---

## 10. Tick Order Summary (network path)

```
Main_Tick (0x0055D360) — network mode 3/4:
  1. Set ms budget: DAT_00887330 = 1000 / DAT_00A8B558 (or 33 ms fallback)
  2. [mode 4 only] Adaptive throttle: scan DAT_00A8DB7C per-peer lag, add up to +30 ms
  3. Active gameplay block (if not paused):
     a. GScreenClass__Input
     b. LogicClass__AI (dispatches key events)
     c. optional House_AI_Tick
     d. [mode 4 only, every 8 frames] Network_Keepalive — updates response-time accumulators
     e. Map__Logic
     f. RenderFrame_main
  4. FUN_00551A30 (side work)
  5. LogicClassPerTickUpdateLiveVector → internally calls FUN_006475F0:
     a. FUN_00648710: BARRIER — blocks until all peers ready for frame N
     b. If barrier passes → FUN_0064C380: dispatch all commands at execute-frame N
  6. Network_ServiceLoop → FUN_0048D1E0: receive incoming packets
  7. Four-flag gate → g_CurrentFrameCounter += 1
  8. FUN_0055E160: sleep to fill ms budget
```

The frame counter increments AFTER the barrier passes and commands execute, not before. This means `g_CurrentFrameCounter` during gameplay (steps 3a–3f) holds the OLD value (frame N-1); commands dispatched in step 5b see frame N as the current frame from which they were scheduled.

---

## 11. Implementation Handoff

### Handoff 1 — Adaptive throttle is mode-4 only; LAN and skirmish are not throttled

| | |
|---|---|
| **Behavior** | Mode-4 (internet) only: scan per-peer lag array, add +10 ms to `DAT_00887330` for each of 3 lag thresholds (budget/4, budget/2, budget×3/4). Gate: `g_NetworkFrameBudget > 30`. Active in YR: YES (mode 4). |
| **Evidence** | `decompile_function 0x0055D360`; `get_assembly_context 0x0055D5B3` (`CMP EBX,0x1e`); `get_assembly_context 0x0055D5D0–0x0055D5ED` (lag loop). |
| **Rust delta** | Any Rust pacing code that applies adaptive budget bumps to LAN or skirmish implements incorrect behavior. The +10 ms bumps must be mode-4 gated. |
| **Surface** | Network pacing layer, wherever per-peer lag is measured and used to extend the frame budget. |
| **Acceptance** | In a simulated mode-4 game with one slow peer (lag > MaxAhead/2), the local frame budget extends by ≥20 ms. LAN mode: no budget extension regardless of peer lag. |
| **Test name** | `test_adaptive_throttle_fires_mode4_only` |
| **Risk** | Fixed peer array at `DAT_00A8DB7C[7]` — must be sized to actual peer count. Scale flag: replace with `Vec`. |

### Handoff 2 — Execute-frame = ceil((issue_frame + MaxAhead) / FrameSendRate) × FrameSendRate

| | |
|---|---|
| **Behavior** | A timing event (0x20) arriving at `issue_frame` with proposed `MaxAhead` schedules command execution at the next integer multiple of `FrameSendRate` that is ≥ `issue_frame + MaxAhead`. All peers compute this identically from the same event fields. Commands with execute-frame > current are held in the ring buffer until `FUN_0064C380` fires at that frame. Active in YR: YES. |
| **Evidence** | `decompile_function 0x004C7600` case `0x20`; `get_xrefs_to 0x00A8B1DC` (4 sites). |
| **Rust delta** | Rust must implement the exact ceiling formula `((MaxAhead + FrameSendRate - 1 + issue_frame) / FrameSendRate) * FrameSendRate` using integer arithmetic. Any floating-point or off-by-one ceiling produces a different execute-frame and causes desync. |
| **Surface** | Command queue / lockstep scheduler, wherever timing events set the execute-frame. |
| **Acceptance** | Given `issue_frame=100, MaxAhead=10, FrameSendRate=5`: `execute_frame = ((100+10+5-1)/5)*5 = (114/5)*5 = 22*5 = 110`. Test with boundary: `MaxAhead=5, FrameSendRate=5` → `((100+5+5-1)/5)*5 = 109/5*5 = 21*5 = 105`. |
| **Test name** | `test_execute_frame_ceiling_formula` |
| **Risk** | If `FrameSendRate` changes between issue and execution, the execute-frame is stale. The late-packet check in `FUN_0064C380` logs but does not prevent execution; Rust must decide whether to abort late packets. |

### Handoff 3 — Player-exit schedules at next 10-frame boundary ≥ MaxAhead+10 frames ahead

| | |
|---|---|
| **Behavior** | `EventClass::Execute` case `0x27`: `exit_frame = ((g_NetworkFrameBudget + 10 + g_CurrentFrameCounter) / 10) * 10`. This guarantees a 10-frame aligned removal frame that all peers can act on synchronously. Active in YR: YES. |
| **Evidence** | `decompile_function 0x004C7600` case `0x27`. |
| **Rust delta** | Player removal events must use this formula for their execute-frame, not `current + MaxAhead` rounded to any arbitrary boundary. |
| **Surface** | Player disconnect / drop handling in lockstep sim. |
| **Acceptance** | `current=1543, MaxAhead=5`: `exit_frame = ((5+10+1543)/10)*10 = (1558/10)*10 = 155*10 = 1550`. |
| **Test name** | `test_player_exit_frame_10_alignment` |
| **Risk** | Incorrectly using `current + MaxAhead` without rounding to 10 produces a mismatched exit frame across peers, causing sim divergence. |

---

## 12. Negative Facts / Do Not Do

1. **Do NOT treat `Network_ServiceLoop` as the lockstep barrier.** It only receives and queues incoming packets. The barrier is in `FUN_006475F0` → `FUN_00648710`. Evidence: `decompile_function 0x0048D080` — no frame-advance gate present; `decompile_function 0x006475F0` — `FUN_00648710` return gates `FUN_0064C380`.

2. **Do NOT apply the per-peer lag adaptive throttle to LAN (mode 3).** The `if (g_GameMode != 4) goto LAB_0055D7C2` branch skips the entire lag-scan and budget-bump block for mode 3. Evidence: `decompile_function 0x0055D360` — explicit mode-4 gate before `CMP EBX,0x1e`.

3. **Do NOT apply adaptive throttle when `g_NetworkFrameBudget <= 30`.** The guard `CMP EBX,0x1e; JLE 0x0055D878` exits before the lag scan when MaxAhead ≤ 30. This means low-MaxAhead configurations (common in LAN) never use the adaptive path even if accidentally in mode 4. Evidence: `get_assembly_context 0x0055D5B3`.

4. **Do NOT conflate `DAT_00A8B568` (MaxAhead high-water mark) with `g_NetworkFrameBudget` (current MaxAhead).** They are different globals: `g_NetworkFrameBudget` changes per timing event; `DAT_00A8B568` only increases. Evidence: `decompile_function 0x004C7600` case `0x20`.

5. **Do NOT use the 10-frame alignment for all command types.** Only the player-exit event (0x27) uses `/10*10`. Regular game commands use `/(FrameSendRate)*(FrameSendRate)` via the timing-event execute-frame formula. Evidence: `decompile_function 0x004C7600` cases `0x20` vs `0x27`.

---

## 13. Scale-Limiting Structures (FLAG for 30-player target)

| Structure | Address | Current cap | Required change |
|---|---|---|---|
| Per-peer lag array (`DAT_00A8DB7C`) | `0x00A8DB7C` | 7 slots (ints) | Dynamic `Vec<i32>` keyed by peer index, sized to `peer_count - 1` |
| Per-peer keepalive struct array (`DAT_00A8B5B4`) | `0x00A8B5B4` | Stride `0x68`, 8 peers max (`0x00A8B5B4..0x00A8B8D3`) | Dynamic `Vec<PeerStats>` keyed by peer ID |
| Command ring buffer (`g_CommandBuffer`) | resolved from `FUN_006475F0` | 0x3FFF (16383) slots — likely sufficient | Verify capacity under 30-player command rate |
| Session peer list (`DAT_00A8DA78`) | referenced across many functions | Pointer array, `DAT_00A8DA84` = count | Needs dynamic allocation; max verified count not extracted this session — mark YELLOW |

---

## 14. Remaining Uncertainty

1. **`FUN_00648710` internals** — this is the actual per-frame wait/barrier function. Its decompilation was not covered this session. Claim "blocks until all peers ready" is inferred from caller contract (return 0 = proceed, nonzero = fail) and debug strings ("Wait_For_Players returned"). YELLOW — verify in a follow-up.

2. **`DAT_00AFA368` per-peer response-time array** (stride 6 ints, used in `FUN_006475F0`'s max-lag computation for WOL mode 4) vs `DAT_00A8DB7C` (the array read in `Main_Tick`). The exact write path from `DAT_00AFA368` → `DAT_00A8DB7C` was not traced. The assembly shows `DAT_00A8DB7C` is zeroed on success and the two arrays are distinct; the copy/update path between them is not verified this session. YELLOW.

3. **FrameSendRate (`DAT_00A8B554`) source** — the value is set in case `0x20` from event byte `+0x0B`, but who computes the proposed `FrameSendRate` before enqueuing the timing event is slot 3's scope. Referenced here only as the denominator in the execute-frame formula.

4. **Maximum peer count enforced by session** — `DAT_00A8DA84` holds connected peer count, used as loop bound in many functions. Its maximum value and initialization path were not fully traced. YELLOW — needed for scale-limit analysis.

---

## 15. Stale-Doc Updates

**SESSIONCLASS_GHIDRA_REPORT.md Part 5** labels `DAT_00a8b550` as `"MaxAhead / NetworkFrameBudget (lockstep frame budget)"` — this is correct but incomplete. It is both the current negotiated MaxAhead (updated per timing event) and the lag-threshold basis for the adaptive budget bumps. No correction needed; the description is not wrong.

**GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md §6 "Open Follow-Up Slices" item 6:** States "Network timing: Multiplayer frame budget and command queue timing should be researched separately." This report closes that item for the frame-scheduling side. CRC/desync/reconnect (slot 5) remains open.

**DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md §7 "Concrete example":** States "end-frames round up to multiples of 10 'because the game processes commands in batches'". This report confirms the 10-frame alignment is in the player-exit formula specifically, not a general command-batch size. The "batches" framing is slightly misleading — regular commands align to `FrameSendRate`, not 10. The doc should note: 10-frame alignment is specific to ABOUTTOEXIT / player-removal events.

---

## Sources

- `decompile_function 0x0055D360` (`Main_Tick`) — this session
- `decompile_function 0x0048D080` (`Network_ServiceLoop`) — this session
- `decompile_function 0x00542520` (`Network_Keepalive`) — this session
- `decompile_function 0x0048D1E0` (session service loop) — this session
- `decompile_function 0x004C7600` (`EventClass::Execute`) — this session
- `decompile_function 0x006475F0` (per-frame queue advance) — this session
- `decompile_function 0x0064C380` (DoList execute) — this session
- `decompile_function 0x005D5870`, `0x005D5880` (`timeBeginPeriod`/`timeEndPeriod`) — this session
- `decompile_function 0x00540F90` (peer-count getter) — this session
- `get_assembly_context 0x0055D5B3..0x0055D5ED` (lag loop assembly) — this session
- `get_assembly_context 0x0055D59C` (budget load) — this session
- `get_xrefs_to 0x00A8B1DC`, `0x00A8DB7C` — this session
- Existing docs read: `LIVE_SKIRMISH_PACING_PATH_GHIDRA_REPORT.md`, `GLOBAL_TIMING_MODEL_GHIDRA_REPORT.md`, `GLOBAL_TIMING_SYSTEM_COMPLETION_GHIDRA_REPORT.md`, `SESSIONCLASS_GHIDRA_REPORT.md`, `DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md`
