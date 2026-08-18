# MaxAhead / NetworkFrameBudget — Ghidra Analysis

**Target question:** Is `DAT_00a8b550` (labeled `g_NetworkFrameBudget`) the same variable as "MaxAhead"? What computes it, what is "PrecalcMaxAhead", and what are the default values and range?

**Non-goals:** Frame-scheduling barrier (slot 1), FrameSendRate internals (slot 3), command queue draining (slot 4), CRC/desync hash (slot 5).

**Evidence for COMPLETE:** All five anchor strings have been traced to their referencing functions, which have been decompiled. The full computation path from lobby → game-start → adaptive re-tuning has been reconstructed. The identity question (same vs. distinct globals) is answered with asm-level evidence.

**Stop conditions:** All anchor strings resolved, event 0x20 write to `g_NetworkFrameBudget` verified, PrecalcMaxAhead compute path documented, range/defaults extracted from decompiled switch.

**Source:** Live Ghidra decompilation of `gamemd.exe` (YR 1.001). Read-only session.

**Active in YR:** Yes (modes 3 = LAN, 4 = WOL/Internet). Mode 5 = skirmish bypasses all MaxAhead logic entirely (`FUN_006475f0` returns immediately when `g_GameMode == 5`).

---

## 1. Identity: g_NetworkFrameBudget @ 0x00a8b550 IS MaxAhead

`g_NetworkFrameBudget @ 0x00a8b550` and MaxAhead are the **same global**. This is
not a conflation — they are one 32-bit integer at a single address.

Evidence (assembly context, verified via `get_assembly_context 0x0064c5f6` and `0x006520ba`):

```asm
; "packet received too late" log, from FUN_0064c380 at 0x0064c5f6:
MOV ECX,dword ptr [0x00a8b550]   ; load g_NetworkFrameBudget
; then pushes format string 0x00837eb8 = "MaxAhead=%d\n"
```

Both xref sites at `0x0064c5f6` (in `FUN_0064c380`) and `0x006520ba` (in `FUN_00652070`)
use an identical pattern: load `[0x00a8b550]`, push it as the argument to the
"MaxAhead=%d\n" format string. No indirection — the memory at `0x00a8b550` IS the MaxAhead
value being printed.

`list_globals` confirms: `g_NetworkFrameBudget @ 00a8b550 [Label] (undefined4)`.

**Verdict: SESSIONCLASS_GHIDRA_REPORT.md Part 5 conflation is CORRECT** — the table entry
`DAT_00a8b550 | int | MaxAhead / NetworkFrameBudget` describes one variable, not two.

---

## 2. Related Globals in the Network Timing Cluster

| Address | Name (inferred) | Type | Purpose |
|---------|-----------------|------|---------|
| `0x00a8b550` | `g_NetworkFrameBudget` (= MaxAhead) | int | Live lockstep window. Frame count stamped on events via `g_CurrentFrameCounter + MaxAhead`. |
| `0x00a8b554` | `g_FrameSendRate` | int/byte | Frames between local input broadcasts. Default 5 (healthy WOL) or 10 (laggy WOL/LAN). |
| `0x00a8b556` | unknown | — | Adjacent; not independently traced. |
| `0x00a8b558` | FrameRate target | ushort | Target ticks/second (divisor in `Main_Tick` pacing formula `0x3c / DAT_00a8b558`). Set by event 0x20 field `+7..8`. See MAIN_TICK_SPEED_BUDGET_MS_PER_FRAME_GHIDRA_REPORT.md. |
| `0x00a8b56c` | `g_PrecalcMaxAhead` | int | Pre-game computed MaxAhead derived from measured ping. Sent in first 0x20 event, then zeroed. DISTINCT from live MaxAhead. |
| `0x00a8b568` | `g_MaxMaxAhead` | int | Session high-water mark — largest MaxAhead value ever set. Logged as "Max MaxAhead: %d" at session end. |
| `0x00a8b570` | `g_PrecalcDesiredFrameRate` | int | Pre-game target frame rate, companion to PrecalcMaxAhead. Zeroed alongside PrecalcMaxAhead after first 0x20 event sends them. |

Verified via `get_xrefs_to 0x00a8b56c` (21 xref sites across `FUN_006475f0`, `FUN_00794ba0`,
`FUN_005bac60`, `FUN_005dc350`) and decompilation of those functions.

---

## 3. Initial MaxAhead — Game-Start Switch (FUN_00794ba0 @ 0x00794ba0)

This function is "Start Game Now" for WOL/LAN. It sets the **initial** `g_NetworkFrameBudget`
from `DAT_00a8b268` (connection-speed selector, 0–5 from lobby UI):

Verified via `decompile_function 0x00794ba0`:

```c
// DAT_00a8b268 = connection speed setting (0=Modem28.8, 1=Modem56, 2=Cable/DSL,
//                                          3=T1/LAN, 4=T1+, 5=LAN-local, default=10)
DAT_00a8b554 = 5;    // default FrameSendRate
switch (DAT_00a8b268) {
    case 0:  g_NetworkFrameBudget = 0x28; DAT_00a8b570 = 0x3c; DAT_00a8b554 = 10; break;
    case 1:  g_NetworkFrameBudget = 0x28; DAT_00a8b570 = 0x2d; DAT_00a8b554 = 10; break;
    case 2:  g_NetworkFrameBudget = 0x1e; DAT_00a8b570 = 0x1e; break;
    case 3:  g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x14; break;
    case 4:  g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x0f; break;
    case 5:  g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x0c; break;
    default: g_NetworkFrameBudget = 10;   DAT_00a8b570 = 10;
}
if (DAT_00a8e2c8 != 0) {
    g_NetworkFrameBudget += 0x14;   // +20 frames if "observer" flag set
}
```

**Default range by connection type:**

| Speed setting | Initial MaxAhead (frames) | FrameSendRate | PrecalcDesiredFrameRate |
|---------------|--------------------------|---------------|------------------------|
| 0 (Modem 28.8k) | 40 (0x28) | 10 | 60 (0x3c) |
| 1 (Modem 56k) | 40 (0x28) | 10 | 45 (0x2d) |
| 2 (Cable/DSL) | 30 (0x1e) | 5 | 30 (0x1e) |
| 3 (T1/LAN) | 20 (0x14) | 5 | 20 (0x14) |
| 4 (T1+) | 20 (0x14) | 5 | 15 (0x0f) |
| 5 (LAN local) | 20 (0x14) | 5 | 12 (0x0c) |
| default | 10 | 5 | 10 |

Observer bonus: `+20` frames added when `DAT_00a8e2c8 != 0`.

---

## 4. PrecalcMaxAhead (DAT_00a8b56c) — What It Is

PrecalcMaxAhead is a **pre-game estimate** of the first MaxAhead value to broadcast, computed
from measured round-trip ping times before `g_CurrentFrameCounter` starts. It is NOT the live
MaxAhead.

The computation (in `FUN_00794ba0`, verified via `decompile_function 0x00794ba0`):

1. Collect max ping across all peers from `DAT_00b7790c..0x00b7792c` (8-entry array,
   each at +0x4 stride relative to `piVar4`).
2. If observer mode is active (`DAT_00a8e2c8`), double the measured max ping.
3. If ping is 0 or 1000ms (unavailable):
   - Set `DAT_00a8db9c = 1` (LatencyFudge).
   - `PrecalcMaxAhead = (g_NetworkFrameBudget / 10) * 10`, clamped to min 10.
4. If ping is measured (1–999ms):
   - Set `LatencyFudge = 1` if ping < 600ms; `LatencyFudge = 2` if ping ≥ 1000ms.
   - Compute frames: `frames = (ping_ms * 0x3c) / 1000` (converts ms→frames at ~60fps target).
   - Add 120% margin: `frames += (frames * 0x78) / 100`.
   - Call `FUN_00540c60(frames, ...)` to set IPX retry parameters.
   - Apply LatencyFudge scaling to `g_NetworkFrameBudget`:
     - Fudge=0 or 1: `PrecalcMaxAhead = g_NetworkFrameBudget`
     - Fudge=2: `PrecalcMaxAhead = g_NetworkFrameBudget + g_NetworkFrameBudget/2`
     - Fudge=3: `PrecalcMaxAhead = g_NetworkFrameBudget * 2`
   - Round down to 10-frame boundary: `PrecalcMaxAhead = (PrecalcMaxAhead / 10) * 10`.

After logging it (`"PrecalcMaxAhead is %d\n"` at `0x0084c578`), control falls through
to set `g_MaxMaxAhead = g_NetworkFrameBudget` (initial high-water) and zero the per-peer RTT
arrays at `0x00a8b574` (0xD0 = 208 bytes cleared → covers up to 8 peers × 0x1A = 26 dword
entries).

**Key distinction:** PrecalcMaxAhead is the initial proposal sent in the **first** event 0x20.
The live `g_NetworkFrameBudget` continues to be updated every 128 frames by adaptive logic
in `FUN_006475f0`.

---

## 5. PrecalcMaxAhead → Live MaxAhead Path (FUN_006475f0 @ 0x006475f0)

The per-frame network turn manager, `FUN_006475f0`, decompiled at `0x006475f0`:

**Early-exit for skirmish:** `if (g_GameMode == 5) return;` — MaxAhead machinery is
completely inactive in local skirmish.

**PrecalcMaxAhead send branch** (verified in decompile output):
```c
if ((DAT_00a8b56c != 0) || (DAT_00a8b570 != 0) || ((byte)g_CurrentFrameCounter == 0)) {
    // PrecalcMaxAhead or PrecalcFrameRate not yet consumed
    if ((DAT_00a8b56c == 0) && (DAT_00a8b570 == 0)) {
        // steady-state: compute new MaxAhead from live RTT measurement
        ...
    } else {
        // first pass: send PrecalcMaxAhead in event 0x20
        sStack_67 = (short)DAT_00a8b56c;  // PrecalcMaxAhead → event +9..10
        uStack_69 = (undefined2)DAT_00a8b570;   // PrecalcDesiredFrameRate → event +7..8
        auStack_70[0] = 0x20;             // event type = NETWORK_FRAME_BUDGET
        bStack_65 = (-(0x1e < DAT_00a8b570) & 5U) + 5;  // FrameSendRate for this event
        // enqueue into command buffer
        ...
        DAT_00a8b56c = 0;   // zero PrecalcMaxAhead after sending
        DAT_00a8b570 = 0;   // zero PrecalcDesiredFrameRate after sending
    }
}
```

Once PrecalcMaxAhead is sent and zeroed, steady-state adaptation fires every 128 frames
(`(g_CurrentFrameCounter & 0x7f) == 0`). The adaptive formula:

```c
iVar10 = (measured_rtt_ms * target_fps_budget) / 0x78 + latency_fudge_bonus;
// clamp growth: no single step > FrameSendRate
if ((g_NetworkFrameBudget < iVar10) &&
    (g_NetworkFrameBudget + FrameSendRate <= iVar10)) {
    iVar10 = g_NetworkFrameBudget + FrameSendRate;   // max +FrameSendRate per step
}
// round up to FrameSendRate boundary
iVar10 = ((iVar10 + FrameSendRate - 1) / FrameSendRate) * FrameSendRate;
// floor at 3×FrameSendRate
if (iVar10 < FrameSendRate * 3) iVar10 = FrameSendRate * 3;
// ceiling: largest multiple of FrameSendRate ≤ 0xf9 (= 249) × FrameSendRate / FrameSendRate
iVar15 = ((249 + FrameSendRate) / FrameSendRate) * FrameSendRate;
if (iVar10 > iVar15) iVar10 = iVar15;
// this new value is placed into a 0x20 event; peers consume it via EventClass::Execute case 0x20
```

**MaxAhead CAN change mid-game.** Every 128 frames the local host measures RTT and emits a
new event 0x20 proposing updated MaxAhead; all peers consume it on the same future frame via
the lockstep event queue.

---

## 6. MaxAhead Write Sites (EventClass::Execute)

Two WRITE sites to `0x00a8b550` in `EventClass::Execute` (verified via `get_xrefs_to 0x00a8b550`
+ `get_assembly_context 0x004c7a07,0x004c808d`):

**Site 1 — event 0x20 (NETWORK_FRAME_BUDGET)** at `0x004c808d`:
```asm
MOV AX,word ptr [ESI + 0x9]    ; new MaxAhead from event record +9..10 (ushort)
XOR EAX,EAX
MOV [0x00a8b550],EAX            ; store as MaxAhead
CMP ECX,EAX                     ; if new > session max
JA skip
MOV [0x00a8b568],EAX            ; update high-water g_MaxMaxAhead
; also writes g_FrameSendRate at 0x00a8b554 from event +11
```

**Site 2 — separate event handler** at `0x004c7a07`:
```asm
XOR EAX,EAX
MOV AL,byte ptr [ESI + 0xd]    ; single byte from event record offset +0xd
MOV [0x00a8b550],EAX            ; store as MaxAhead (zero-extended byte)
```
Context suggests this is a different event type (not 0x20). The surrounding code accesses
`ESI + 0x16054` (a HouseClass player-name offset) and `g_RulesClass_Instance + 0x14c0`
(a float field in Rules), suggesting this is an event that also touches house state. Exact
event type not confirmed by case tag in this session — flagged as YELLOW (Unverified event
number).

---

## 7. "Max MaxAhead: %d" Log (FUN_0048e0b0 @ 0x0048e0b0)

Decompiled `0x0048e0b0`: This function writes an `mpstats.txt` file at game end (only for
`g_GameMode == 4`). It logs:
- Total frames played.
- Average FPS.
- **"Max MaxAhead: %d"** ← reads `DAT_00a8b568` (the high-water mark, NOT the live value).
- Latency setting (LatencyFudge index).
- Per-peer RTT stats from `DAT_00a8b5c4` array (stride `0x1A` dwords, iterates until
  `0x00a8b904`; that's (`0x00a8b904 - 0x00a8b5c4`) / (0x1A × 4) = `0x340 / 0x68` = 8 entries.

**Scale-limiting flag:** The per-peer stats array covers exactly 8 entries
(`0x00a8b5c4..0x00a8b904` at stride `0x68`). This is the hardcoded 8-player ceiling.
A 30-player Rust reimplementation must use a growable per-peer structure.

---

## 8. "MaxAhead is %d" in IPX_Manager (FUN_00540c60 @ 0x00540c60)

Decompiled `0x00540c60` (called from the pre-game PrecalcMaxAhead path):
```c
Register_heap_pool("IPX_Manager: RetryDelta = %d", RetryDelta);
Register_heap_pool("MaxAhead is %d", g_NetworkFrameBudget);
// then propagates RetryDelta/MaxRetries/RetryTimeout to each peer struct
```
This is an informational log printed when IPX retry parameters are configured. It reads the
current `g_NetworkFrameBudget` at that moment (the initial switch-table value, before
PrecalcMaxAhead further adjusts it). Not a setter.

---

## 9. FogOfWar Reduction on MaxAhead

From `multiplayer-frame-step.md` (confirmed in the event 0x20 decompile in `FUN_006475f0`):
```c
sStack_67 = (-(ushort)((*g_ScenarioClass_Instance & 0x1000) != 0) & 10) + (short)iVar10;
// If FogOfWar flag (SpecialFlags & 0x1000) is set, MaxAhead in the event is reduced by 10.
```
In standard YR (`FogOfWar=no`, SpecialFlags bit 12 = 0), this reduction is NOT applied —
MaxAhead is sent unmodified.

**Active in YR:** No (conditional on TS-legacy FogOfWar flag, always 0 in stock YR).

---

## 10. Scale-Limiting Structures (30-Player Flag)

| Address | Structure | Current cap | Scale-limiting? |
|---------|-----------|------------|-----------------|
| `0x00a8b5c4..0x00a8b904` | Per-peer stats array (mpstats.txt) | 8 entries × 0x68 bytes | YES — hardcoded 8-player limit |
| `0x00a8db7c` | Per-peer lag accumulator array | 7 entries (loop `for iVar9 = 7 ...`) in `FUN_006475f0` | YES — covers 7 non-local peers = 8 total |
| `0x00b7790c..0x00b7792c` | Peer ping time array | 8 entries (`(int)piVar4 < 0xb7792c`) | YES — 8-player |

All three must be replaced with `Vec` or equivalent growable containers in the Rust
reimplementation to reach the 30-player target.

---

## Implementation Handoff

### Handoff 1 — g_NetworkFrameBudget is MaxAhead, a single 32-bit global

**Behavior:** `DAT_00a8b550` is the single authoritative lockstep window. EventClass
stamps events with `g_CurrentFrameCounter + g_NetworkFrameBudget`. Any code that reads
or writes MaxAhead accesses this address.

**Rust delta:** SESSIONCLASS_GHIDRA_REPORT.md table already labels this correctly. The
Rust `NetworkSession` struct should expose a field `max_ahead: u32` initialized from the
connection-speed switch table.

**Surface:** Multiplayer lockstep scheduling only. Skirmish (mode 5) never touches this.

**Acceptance:** In a 2-player LAN game at speed=3 (T1/LAN), initial MaxAhead = 20
(0x14). Events broadcast by local peer carry execution frame = `current_frame + 20`.
Peers execute them exactly on that frame.

**Test:** `test_maxahead_initial_from_connection_speed` — assert that connection speed
case 3 yields MaxAhead = 20 and FrameSendRate = 5.

**Risk:** LOW — identity is clear, values are hardcoded constants.

---

### Handoff 2 — PrecalcMaxAhead is a one-shot pre-game proposal, not a persistent state

**Behavior:** Computed from measured ping before game start. Sent in the **first** event
0x20. Zeroed immediately after. Live `g_NetworkFrameBudget` is NOT updated by PrecalcMaxAhead
directly — the event 0x20 consumer updates it when the event executes on the peer.

**Rust delta:** Do not persist PrecalcMaxAhead as a permanent field. Model it as a
"first-event payload" that, once sent, is gone. The live MaxAhead on each peer is updated
only via event 0x20 consumption.

**Surface:** Game-start network negotiation path.

**Acceptance:** Immediately after a game starts (frame 0), peers' MaxAhead values equal
the PrecalcMaxAhead that was sent in their first 0x20 event. By frame 128, the adaptive
path takes over.

**Test:** `test_precalc_maxahead_zeroed_after_first_event` — assert PrecalcMaxAhead is 0
after the first 0x20 event is enqueued.

**Risk:** MEDIUM — the distinction between PrecalcMaxAhead and live MaxAhead is subtle;
conflating them makes MaxAhead skip its first adaptive step.

---

### Handoff 3 — MaxAhead adapts mid-game, growth clamped to +FrameSendRate per 128-frame step

**Behavior:** Every 128 frames (`current_frame & 0x7f == 0`), the host measures RTT, applies
LatencyFudge multiplier, computes a new MaxAhead, and emits event 0x20. The new value:
- Rounds up to the nearest FrameSendRate multiple.
- Cannot grow by more than FrameSendRate in a single step.
- Floor: `3 × FrameSendRate`.
- Ceiling: `((249 + FrameSendRate) / FrameSendRate) × FrameSendRate`.

**Rust delta:** The Rust networking layer must implement the adaptive recalculation loop.
The clamp (`new > old + FrameSendRate → new = old + FrameSendRate`) prevents runaway growth
from a single bad RTT spike.

**Surface:** Multiplayer frame scheduling. Player-visible effect: input lag gradually
adjusts to sustained network degradation within ~5-10 second windows.

**Acceptance:** In a simulated session where RTT doubles from 50ms to 100ms, MaxAhead
grows by exactly one FrameSendRate step (e.g., 20→25 for FrameSendRate=5) per 128-frame
window until it reaches the new equilibrium.

**Test:** `test_maxahead_clamped_growth_one_framesendrate_per_step`.

**Risk:** HIGH — if the growth clamp is missing, a single bad ping sample can spike
MaxAhead by dozens of frames, making the game feel sluggish.

---

## Negative Facts / Do Not Do

1. **Do NOT read PrecalcMaxAhead as the live MaxAhead.** They are distinct: PrecalcMaxAhead
   (`0x00a8b56c`) is zeroed after first use; live MaxAhead (`0x00a8b550`) persists and adapts
   throughout the game.

2. **Do NOT run MaxAhead machinery in skirmish (mode 5).** `FUN_006475f0` returns immediately
   for mode 5. MaxAhead is irrelevant when there are no remote peers.

3. **Do NOT apply the FogOfWar MaxAhead reduction in standard YR.** The `-10` adjustment is
   gated on `SpecialFlags & 0x1000` which is always 0 in stock YR.

4. **Do NOT assume INI drives MaxAhead.** There is no `rulesmd.ini` or `artmd.ini` key for
   MaxAhead, FrameSendRate, or LatencyFudge. All values are hardcoded constants adapted
   at runtime from measured RTT. INI surface is none.

5. **Do NOT use the "Max MaxAhead" log value (DAT_00a8b568) as the live MaxAhead.** It is
   a session high-water mark for the mpstats.txt file only.

---

## Remaining Uncertainty

1. **Site 2 write to g_NetworkFrameBudget at `0x004c7a07`** — the event type is not
   confirmed from assembly context in this session. The surrounding code suggests a
   player-exit or house-state event that also clears MaxAhead (XOR EAX,EAX + MOV AL,[ESI+0xd]
   writes a byte from the event payload). Candidate: event `0x1b` (SetMaxAhead direct).
   The multiplayer-frame-step.md doc mentions "event 0x1b" sets MaxAhead directly but this
   was not verified against live binary in this session. **Status: YELLOW / Unverified.**

2. **LAN vs WOL initial MaxAhead path** — the "Start Game Now" function (`FUN_00794ba0`)
   is the WOL path. The equivalent LAN game-start path (`FUN_005bac60` has WRITE xrefs to
   `0x00a8b550`) likely has a parallel switch; not decompiled in this session.
   **Status: YELLOW / Unverified for LAN.**

3. **Exact ceiling formula** — the upper bound on MaxAhead from `FUN_006475f0` uses
   `(uint)((ulonglong)(uVar3 + 0xf9) / (ulonglong)(longlong)(int)uVar3) * uVar3`
   where uVar3 = FrameSendRate. For FrameSendRate=5 this yields `((5+249)/5)*5 = 250`.
   For FrameSendRate=10: `((10+249)/10)*10 = 250`. Ceiling appears to converge to ~250
   frames regardless of FrameSendRate, but arithmetic confirmation for edge cases is
   incomplete. **Status: MEDIUM confidence.**

---

## Stale / Incorrect Claims in Existing Docs

**SESSIONCLASS_GHIDRA_REPORT.md Part 5, line:**
> `DAT_00a8b550 | int | MaxAhead / NetworkFrameBudget (lockstep frame budget)`

This label is CORRECT. The "/" does not imply two separate variables — both names refer to
the same global. No correction needed, but a clarifying note is warranted:
`g_NetworkFrameBudget` is the Ghidra label; "MaxAhead" is the in-game terminology used in
all format strings. They are identical.

**timing/multiplayer-frame-step.md, line 119:**
> `Verified at 0x00a8b1e0 (inferred from xrefs; verified by string-format strings)`

This is WRONG for the address. `g_NetworkFrameBudget` is at `0x00a8b550`, not `0x00a8b1e0`.
The address `0x00a8b1e0` does not appear in any xref to the MaxAhead format strings
(verified via `get_xrefs_to 0x00a8b550`). The doc's pseudocode snippet is correct in
substance (the clamp logic matches `FUN_006475f0`), but the stated address is incorrect.

**Path:** `C:/Users/enok/Documents/ra2-rust-game/docs/research/timing/multiplayer-frame-step.md`
**Stale claim:** Address `0x00a8b1e0` as g_NetworkFrameBudget.
**Correct value:** `0x00a8b550` (confirmed via `list_globals`, assembly context, and all xrefs).

---

## Key Facts Summary

| # | Fact | Evidence |
|---|------|----------|
| F1 | `g_NetworkFrameBudget @ 0x00a8b550` = MaxAhead; one global, one address. | `get_assembly_context 0x0064c5f6, 0x006520ba`; `list_globals filter=a8b550` |
| F2 | Initial MaxAhead set from connection-speed switch in "Start Game Now" (`FUN_00794ba0`); values 10–40 frames. | `decompile_function 0x00794ba0` |
| F3 | PrecalcMaxAhead (`DAT_00a8b56c`) is a separate pre-game estimate sent in first event 0x20, then zeroed. NOT the live MaxAhead. | `decompile_function 0x00794ba0`, `get_xrefs_to 0x00a8b56c` |
| F4 | MaxAhead adapts mid-game every 128 frames; growth clamped to +FrameSendRate per step; floor = 3×FrameSendRate. | `decompile_function 0x006475f0` |
| F5 | MaxAhead is INACTIVE in skirmish (mode 5). `FUN_006475f0` returns immediately for mode 5. | `decompile_function 0x006475f0` (first statement) |
| F6 | Per-peer stats array (`0x00a8b5c4..0x00a8b904`) and lag arrays are hardcoded for 8 players — scale-limiting for 30-player target. | `decompile_function 0x0048e0b0`, `decompile_function 0x006475f0` |
| F7 | `DAT_00a8b568` = session-high-water MaxAhead; logged as "Max MaxAhead: %d". Distinct from live MaxAhead. | `decompile_function 0x0048e0b0`, `get_assembly_context 0x004c8087` |

---

*Report generated 2026-05-28. Read-only Ghidra session on gamemd.exe YR 1.001.*
