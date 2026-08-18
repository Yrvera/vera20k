# Multiplayer Frame Step (Lockstep / Network Turn)

## Overview

**Player-visible effect:** in multiplayer (LAN or Internet), every player's
clicks, hotkey presses, and unit orders feel slightly delayed — the issued
order doesn't take effect on the frame you pressed the button; it takes
effect a few frames later. That delay is the **lockstep window**: the
engine schedules every input to execute on a frame far enough in the future
that all peers will receive the order in time to execute it together.
Higher network latency → larger lockstep window → more perceived input lag.
When the network is healthy the window holds at a modest value (~5–15
frames); when the network deteriorates the engine adapts upward.

**Mechanism in plain terms:** `gamemd.exe` is a **deterministic lockstep**
engine. Every player input on every peer becomes an `EventClass` record
stamped with `g_CurrentFrameCounter + MaxAhead` (where `MaxAhead` =
`g_NetworkFrameBudget`). The event is broadcast to all peers. Each peer
buffers incoming events into a ring queue. On every tick, the queue drainer
fires `EventClass::Execute` for every event whose frame stamp `<=
g_CurrentFrameCounter` — but **only after** confirming that every peer has
sent their events for that frame. If a peer's frame-N events haven't
arrived, the local tick stalls (the "Reconnecting..." / "Waiting for player
X" dialog appears) until they do. Because every peer executes the same
events in the same order on the same frame, no state diverges. State hashes
are exchanged periodically; any divergence triggers `Desync_Handler`.

There are **three knobs** the engine tunes adaptively in MP:

1. **`MaxAhead` (= `g_NetworkFrameBudget`)** — the lockstep window size.
   Higher → more input lag, more tolerance for jitter. Sent in every
   network-budget event (`0x20`) plus directly set by event `0x1b`.
2. **`FrameSendRate` (= `DAT_00a8b554`)** — how many frames the local peer
   waits between broadcasting input batches. Typical values 5 or 10.
   Smaller → more bandwidth, lower added latency. Sent in event `0x20`.
3. **`LatencyFudge` (= `DAT_00a8db9c`)** — multiplier applied to response
   time when computing target `MaxAhead`. Values 0–3 (1×, 1.5×, 2×, 3×).
   Set by event `0x24`.

All three are sync'd across peers via events. Every 128 frames the local
peer measures response time (RTT to slowest peer) and emits a `0x20` event
proposing new values; peers consume the event on a future frame and step
in lockstep.

The **GameSpeed slider** (per-machine choice from [game-speed-master-clock.md](game-speed-master-clock.md))
is also sync'd via event `0x0d` so all peers wait the same wall-clock time
per tick. The network-budget mechanism is **independent** of GameSpeed:
GameSpeed sets the *wall-clock interval* between ticks; the
network-budget/FrameSendRate sets *how many frames in advance* inputs
schedule.

---

## INI surface

**None.** No `rulesmd.ini` or `artmd.ini` key configures any of: MaxAhead,
FrameSendRate, LatencyFudge, frame-queue size, secondary-queue size, or
desync-detection cadence. All values are hardcoded and adaptively tuned at
runtime from measured network conditions.

The closest INI surface is `[MultiplayerDialogSettings] GameSpeed` (covered
by [game-speed-master-clock.md](game-speed-master-clock.md)) — but that's
the wall-clock tick rate, not the network frame step.

Per-player `Maximums` is also adjacent:

```ini
[Maximums]
Players=8                               ; ipx layer limits this to 8 maximum
```

This `8`-player cap propagates into the network frame-step code as the
loop bound for per-player accumulators (`g_HouseClass_Array_Count` checks).
Note that the project's scale target is 30 players per
[project_scale_target.md](../../memory/project_scale_target.md) — when the
Rust reimplementation lifts this cap it must also lift the equivalent
fixed-size MP arrays.

---

## Hardcoded constants

### Event queue ring buffer

From `FUN_0064c380` @ `0x0064c380` (the queue drainer) and
`EventClass::Execute` callers:

| Constant | Meaning |
|---|---|
| `0x008b4204` | Base address of the event ring buffer |
| `0x6f = 111 bytes` | Size of each `EventClass` record |
| `& 0x3fff = 16383` | Index mask → **16384 slots** in the ring buffer |
| `DAT_008b41f8` | Event count (number of unread events) |
| `DAT_008b41fc` | Read index (advances modulo 0x3fff) |
| `0x00a83edc` | Base of secondary "MegaMission" / packed-event ring (256 slots × 0x6f bytes) |
| `& 0xff = 255` | Secondary-queue index mask |
| `DAT_00a83ed0` | Secondary queue count |
| `DAT_00a83ed4` | Secondary queue read index |
| `DAT_00a83ed8` | Secondary queue write index |

Within each 0x6f-byte event:

| Byte offset (within record) | Field |
|---|---|
| `+0x00` | Event type (one of the `case` values in [logic-vs-render-loop.md](logic-vs-render-loop.md)'s `EventClass::Execute` decompilation) |
| `+0x01` | Flags (bit 0 = "consumed" / "processed") |
| `+0x02` | House/Player ID (signed byte) |
| `+0x03..0x06` | Scheduled execution frame number (`int`, compared against `g_CurrentFrameCounter`) |
| `+0x07..0x0a` | Per-tick state checksum (used for desync detection) |
| `+0x0d` | CRC seed offset (used to recover checksum cell into `DAT_00b04474` ring) |

Quoted from `FUN_0064c380` direct memory access patterns
(`(&DAT_008b4204)[uVar2 * 0x6f]` etc.). The `0x6f` record size is verified
by the ring-buffer copy loops elsewhere (e.g. `for (iVar = 0x1b; iVar != 0;
...)` copies 27 DWORDs + 3 bytes = `27*4 + 3 = 111` = `0x6f`).

### `g_NetworkFrameBudget` (= MaxAhead)

The lockstep window. Verified at `0x00a8b1e0` (inferred from xrefs;
verified by string-format strings):

```c
// from FUN_006475f0 (per-tick network turn manager) — clamp the computed
// new MaxAhead so it never grows by more than FrameSendRate per step:
if ((g_NetworkFrameBudget < iVar10) &&
    ((int)(g_NetworkFrameBudget + (uint)bStack_65) <= iVar10)) {
    iVar10 = g_NetworkFrameBudget + (uint)bStack_65;
}
```

Format strings in the binary explicitly print "MaxAhead":

| Address | String |
|---|---|
| `0x0081d948` | `"Max MaxAhead: %d\n"` |
| `0x00828398` | `"MaxAhead is %d\n"` (in `IPX_Manager` retry-delta log) |
| `0x0082a1f4` | `"MaxAhead : %d"` (HUD telemetry) |
| `0x00837eb8` | `"MaxAhead=%d\n"` (in "Packet received too late" warning) |
| `0x0083f308` | `"MaxAhead = %d\n"` |
| `0x0084c578` | `"PrecalcMaxAhead is %d\n"` |

Default at session start is set by the host based on initial latency
sampling; clients receive the value via event `0x20`. The clamp pattern
`g_NetworkFrameBudget + FrameSendRate` means the window can only grow by
one FrameSendRate per adaptation; this prevents runaway growth from a
single bad RTT sample.

### `DAT_00a8b554` (= FrameSendRate)

How many frames the local peer waits between transmitting input batches.
Per `FUN_006475f0`:

```c
bStack_65 = (byte)DAT_00a8b554;
if (g_GameMode == 4) {   // Internet MP
    bStack_65 = ((iStack_88 < 0x1f) - 1U & 5) + 5;
}
```

The MP-only override: if measured response time `iStack_88 < 0x1f = 31`,
FrameSendRate = `5`; else `10`. So in healthy Internet play, the local
peer broadcasts every 5 frames; in laggy play, every 10 frames. LAN
(`g_GameMode == 3`) does not get this override — uses whatever
`DAT_00a8b554` was set to.

Format strings:

| Address | String |
|---|---|
| `0x00837e98` | `"FrameSendRate=%d\n"` |
| `0x00838e04` | `"FrameSendRate: %d\n"` |
| `0x0084c540` | `"FrameSendRate is %d\n"` |

### `DAT_00a8db9c` (= LatencyFudge)

A 0–3 multiplier index applied to measured RTT when computing target
MaxAhead. Set by event `0x24` (`EventClass::Execute` case `0x24` —
"LATENCYFUDGE"):

```c
case 0x24:
    DAT_00a8db9c = *(undefined4 *)(param_1 + 7);
    // logs "LatencyFudge is %d"
    ...
```

Applied in `FUN_006475f0`'s response-time math:

```c
switch (DAT_00a8db9c) {
    case 0: break;                              // ×1
    case 1: uVar3 = uVar3 + (uVar3 >> 1);       // ×1.5
            iVar10 = 10;
            iVar9 = iVar9 + iVar9 / 2; break;
    case 2: uVar3 = uVar3 * 2;                  // ×2
            iVar9 = iVar9 * 2;
            iVar10 = 0x14; break;               // +20 frames base bonus
    case 3: uVar3 = uVar3 * 3;                  // ×3
            iVar9 = iVar9 * 3;
            iVar10 = 0x1e; break;               // +30 frames base bonus
}
```

`uVar3` = measured response time (ms). Higher LatencyFudge → bigger
MaxAhead → more input lag but more tolerance for jitter. The host (or a
designated peer) emits a `0x24` event when chronic stalls are detected to
nudge the entire session into a more conservative pacing band. Note: the
`+10/+20/+30` bonuses (`iVar10` initial value) get added to the per-tick
recomputation so even with 0 measured RTT, LatencyFudge≥1 enforces a
floor.

Strings:

| Address | String |
|---|---|
| `0x00820a68` | `"LATENCYFUDGE"` |
| `0x00820c54` | `"LatencyFudge is %d\n"` |
| `0x00820c68` | `"Executing LATENCYFUDGE event. Frame is %d\n"` |
| `0x00824090` | `"LATENCYFUDGE event created - %d\n"` |

### Adaptive-pacing cadence

`FUN_006475f0` runs the response-time measurement and emits a new
network-budget event (`0x20`) every **128 frames**:

```c
if ((uVar3 & 0x7f) == 0) {   // every 128th frame
    uVar3 = (**(code **)(*piVar13 + 0x30))();   // measure RTT
    ...
    (**(code **)(*piVar13 + 0x34))(uVar3 + 10, 0xffffffff, uVar4, 0);
    // also enqueue a 0x21 frame-timing event with measured FPS
    // and a 0x20 NETWORK_FRAME_BUDGET event with new MaxAhead/FrameSendRate
}
```

The `0x7f` mask = every 128th tick. At GameSpeed=Medium (≈20 ticks/sec)
this fires every ~6 seconds; at Fastest (uncapped) it fires roughly every
2–4 seconds depending on hardware. **Therefore the engine's
self-adaptation reaction time is ~5–10 seconds in normal play** — long
enough that brief lag spikes don't thrash the pacing, short enough that a
sustained drop in network quality is responded to within a few "rounds"
of play.

A separate cadence:

```c
if ((g_CurrentFrameCounter & 0x7ff) == 0) {   // every 2048th frame
    (**(code **)(*piVar13 + 0x2c))(0);         // periodic timing-info send
}
```

`0x7ff = 2047` → every ~2048 frames (~100 seconds at Medium) emit a deeper
state-sync packet. This is the slower "are we still all on the same page"
heartbeat.

### Network keepalive cadence (from [game-speed-master-clock.md](game-speed-master-clock.md))

```c
if (((byte)g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4)
    Network_Keepalive();
```

Every 8 frames (Internet MP only) → `Network_Keepalive` measures per-peer
RTT and stores into the per-peer `DAT_00a8b5b4 + i*0x68` array. The 1000/60
divisor (`(iVar1 * 1000) / 0x3c`) converts a `60Hz` GetRadarTimer delta
back to ms.

### Desync state-hash exchange

Each event record carries a `+0x07..0x0a` checksum field. The drainer
compares:

```c
if ((*(int *)(&DAT_008b4207 + uVar3 * 0x6f) == g_CurrentFrameCounter) &&
    (uVar2 = DAT_008b41fc + local_38 & 0x3fff,
     (&DAT_00b04474)[*(int *)(&DAT_008b4207 + uVar2 * 0x6f) -
                     (uint)(byte)(&DAT_008b4211)[uVar2 * 0x6f] & 0xff] !=
     *(int *)(&DAT_008b420b + uVar2 * 0x6f))) {
    if (DAT_00b04880 == '\0') {
        FUN_0064dea0();      // soft desync recovery attempt
    } else {
        iVar18 = 0x100;
        do {
            FUN_006516f0();   // retry recovery 256 times
            iVar18 = iVar18 + -1;
        } while (iVar18 != 0);
    }
    // pop up desync dialog
}
```

`DAT_00b04474` is the local 256-entry ring of per-tick state checksums
(addressed by `g_CurrentFrameCounter & 0xff`). The drainer compares the
remote peer's recorded checksum (in the event record) against the local
checksum for the **same frame** — if they differ, peers have diverged on
that frame.

**Confidence (content): HIGH** — directly observed comparison.
**Confidence (binding to "desync" terminology): MEDIUM** — there's no
labeled `Desync` string here, but the recovery handler chain leads to
`Desync_Handler` @ `0x0048dc90` which is named. The chain is consistent:
checksum mismatch → `FUN_0064dea0` (writes to globals) → eventually
`Desync_Handler` deselects all units and resets selection mode so the user
can't issue commands while resync is attempted.

### Desync handler

`Desync_Handler` @ `0x0048dc90`:

```c
void Desync_Handler(void) {
    while (g_CurrentObjects_Count != 0) {
        (**(code **)(*(int *)*g_CurrentObjects_Data + 0x150))();
    }
    Selection__ResetMode();
    FUN_00431060();
}
```

Pops every currently-selected object via virtual `+0x150` (a "deselect"
method) and resets `Selection__ResetMode`. **Does not stop the simulation
or roll back state.** Recovery from desync is handled elsewhere (likely a
state-snapshot re-sync attempt in the IPX/SCKT manager); this routine
just guarantees the player can't keep clicking on stale selections while
resync runs.

### Late-packet handling

```c
if (*(int *)(&DAT_008b4207 + uVar3 * 0x6f) < g_CurrentFrameCounter) {
    if ((&DAT_008b4204)[uVar3 * 0x6f] != '\x1c'   // event type 0x1c is exempt
        && g_GameMode != 0 && g_GameMode != 5) {  // MP only (skip SP+replay)
        Register_heap_pool(s_Packet_received_too_late, ...);
        ...
        goto LAB_0064cad7;   // skip this event
    }
}
```

Events that arrive after their scheduled frame are dropped (with a log
warning) — except event type `0x1c` (likely a state-sync / heartbeat that
must be processed even when late) and SP/replay (where every event is
"on time" by construction). **Player-visible effect:** in a really bad
network, you may see your unit "snap back" to where it was before you
gave an order because the order arrived too late to execute and the local
prediction has nothing to confirm.

### Event-bulk-move on adaptation

When `g_NetworkFrameBudget` changes mid-session (event `0x20` arrives),
any events still queued with frame stamps in the **old** window need to
be shifted forward into the **new** window:

```c
if ((((&DAT_008b4204)[uVar2 * 0x6f] != '\x1c') &&
     (iVar4 = *(int *)(&DAT_008b4207 + uVar2 * 0x6f), DAT_00a8b1f8 < iVar4)) &&
    (iVar4 < iVar15)) {
    Register_heap_pool(s_DoList__Moving_event_from_frame, iVar4, iVar15);
    *(int *)(&DAT_008b4207 + (DAT_008b41fc + iVar12 & 0x3fffU) * 0x6f) = DAT_00a8b1dc;
}
```

`DAT_00a8b1f8` = old `MaxAhead` boundary; `DAT_00a8b1dc` = new boundary.
Events between the two get bumped forward to land on the new boundary.
Logged as "DoList: Moving event from frame X to Y". Cleared on
`EventClass::Execute` case `0x20`:

```c
case 0x20:
    *(ushort *)(param_1 + 9) =
         *(short *)(param_1 + 9) - (-(ushort)((*g_ScenarioClass_Instance & 0x1000) != 0) & 10);
    if ((g_NetworkFrameBudget < *(ushort *)(param_1 + 9)) || (DAT_00a8b554 < (byte)param_1[0xb])) {
        DAT_00a8b1f8 = *(undefined4 *)(param_1 + 3);
        uVar11 = (uint)(byte)param_1[0xb];
        DAT_00a8b1dc = ((int)(*(ushort *)(param_1 + 9) + uVar11 + -1 + *(int *)(param_1 + 3)) /
                       (int)uVar11) * uVar11;
    } else {
        DAT_00a8b1dc = 0;
        DAT_00a8b1f8 = 0;
    }
    DAT_00a8b558 = (uint)*(ushort *)(param_1 + 7);
    g_NetworkFrameBudget = (uint)*(ushort *)(param_1 + 9);
    if (DAT_00a8b568 <= g_NetworkFrameBudget) {
        DAT_00a8b568 = g_NetworkFrameBudget;
    }
    DAT_00a8b554 = (uint)param_1[0xb];
    return;
```

Event `0x20` carries (in the param_1 record):
- `+7..8`: new `DAT_00a8b558` (FrameRate target / unknown — used by `RadBeam` time-fraction divisor)
- `+9..10`: new `g_NetworkFrameBudget` (MaxAhead)
- `+11`: new `DAT_00a8b554` (FrameSendRate)

`DAT_00a8b568` tracks the **session-max MaxAhead** ever seen (logged as
"Max MaxAhead" at session end).

The `& 0x1000` SpecialFlag adjustment in the `param_1[9]` line is the
**fog-of-war reduction** — when FogOfWar is enabled (TS-legacy flag), the
effective MaxAhead is reduced by 10 frames. Confirmed by the `& 0x1000`
gate matching the iteration-1 finding. In standard YR (FogOfWar default
`no`), this reduction is **not applied** — MaxAhead is used directly.

### `DAT_00a8b558` — what is it?

The 16-bit "FrameRate-related" field from event `0x20`. Inspected usages:

- `Main_Tick` SP/replay path: `DAT_00887350 = 0x3c / DAT_00a8b558` and
  `local_1ac = 1000 / DAT_00a8b558` — would only make sense if it
  represents "target ticks per second", but this code path is only hit
  when `g_GameMode != 0` (MP only). At `DAT_00a8b558 = 60`, that gives
  `0x3c/60 = 1` GetRadarTimer-unit wait (16 ms = ~62 FPS) and
  `1000/60 = 16` ms — consistent with `DAT_00a8b558` = "target FPS".
- `RadBeam::DrawAndTickAll`: `(DAT_00abcd44 / DAT_00a8b558) ^ 3` —
  normalizes "elapsed tick count since last vsync" by the target rate to
  produce a smoothed sample. Treats `DAT_00a8b558` as a divisor that
  represents "expected ticks per network-budget unit".
- `EventClass::Execute` case `0x20`: assigned from event `+7..8` (ushort).
- `HouseClass::Begin_Production` @ `0x004fa661`: reads but not yet
  documented.
- `FUN_00652110`: reads but not yet documented.

**Best interpretation: `DAT_00a8b558` is a "frames per network turn"
target** — the lockstep window's expected tick budget — used by per-tick
pacing (MP) and by particle-time normalization (radiation beams).
**Confidence: MEDIUM** — multiple consumers agree on "divisor", and the
value source (event `0x20`) and use sites (pacing + smoothing) line up,
but the canonical name (FrameRate? PrecalcFrameRate? TargetFPS?) isn't
directly observable in any string.

### `Connection retry` constants in `IPX_Manager`

`FUN_00540c60` sets:

| Field | Default | Meaning |
|---|---|---|
| `param_2` | (passed in) | RetryDelta — ms between packet retransmits |
| `param_3` | (passed in) | MaxRetries |
| `param_4` | (passed in) | RetryTimeout |

Logged: `"IPX_Manager: RetryDelta = %d"` and `"MaxAhead is %d"` (the latter
is the network-budget that should match `g_NetworkFrameBudget`). Exact
default values not extracted this iteration; flag for follow-up if a
networking-specific doc is written.

### `Wait For Players` timeout

`FUN_00648710` is called with `0xb4 = 180` as its first arg:

```c
iVar9 = FUN_00648710(0xb4, iVar9, *(undefined4 *)(&DAT_0083734c + g_GameMode * 0x10), ...);
```

`0xb4 = 180` GetRadarTimer ticks = `180 * 16 ms ≈ 2880 ms`. So the "Wait
For Players" timeout per polling round is ~3 seconds; if the wait returns
non-zero (error code 2, 3, 7, 8, 9) the session terminates with a
localized error message.

### Command queue (outgoing local input)

The `g_CommandBuffer` ring at base address `&g_CommandBuffer` with
`& 0x7f = 127` index mask:

| Constant | Meaning |
|---|---|
| `g_CommandBuffer` | Base address of the outgoing command ring |
| `g_CommandQueue_WriteIndex` | Write index, `& 0x7f` |
| `g_CommandQueue_Count` | Count |
| `g_CommandTimestamps` | Parallel array of `timeGetTime()` ms timestamps |
| `0x6f = 111` | Record size (same as inbound) |
| `& 0x7f = 127` | Index mask → 128 slots |

So the local outgoing buffer holds **128 in-flight commands**; the
inbound queue holds **16384**. Both use the same 111-byte record format.

The standard enqueue idiom (repeated dozens of times across
`EventClass::Execute` callers):

```c
if (g_CommandQueue_Count < 0x80) {
    puVar7 = (undefined4 *)record;
    puVar11 = (undefined4 *)(&g_CommandBuffer + g_CommandQueue_WriteIndex * 0x6f);
    for (iVar = 0x1b; iVar != 0; iVar = iVar + -1) {   // copy 27 DWORDs
        *puVar11 = *puVar7;
        puVar7 = puVar7 + 1;
        puVar11 = puVar11 + 1;
    }
    *(undefined2 *)puVar11 = *(undefined2 *)puVar7;    // + 2 bytes
    *(undefined1 *)((int)puVar11 + 2) = *(undefined1 *)((int)puVar7 + 2);   // + 1 byte
    *(DWORD *)(&g_CommandTimestamps + g_CommandQueue_WriteIndex * 4) = timeGetTime();
    g_CommandQueue_WriteIndex = g_CommandQueue_WriteIndex + 1 & 0x7f;
    g_CommandQueue_Count++;
}
```

The `0x80` cap → 128 commands. If the player issues commands faster than
the outgoing queue can drain (because the network is too slow to ack),
new commands are **silently dropped** at the cap. This is the underlying
mechanism for "stuck input" complaints in extreme lag.

### Event types referenced (not exhaustive — full enumeration owned by `EventClass::Execute`)

| Type | Description | Set on frame |
|---|---|---|
| `0x04`/`0x05` | Movement order to a unit | `g_CurrentFrameCounter` (becomes scheduled by enqueuer to `+ MaxAhead`) |
| `0x08` | Flag-to-win-check | enqueued only |
| `0x0d` | Set GameSpeed slider (`DAT_00a8eb60`) | (Sync across all peers) |
| `0x13` | EXIT — player has quit gracefully | Player request → enqueued for future frame |
| `0x1b` | Set `g_NetworkFrameBudget` directly | Used in initial session sync |
| `0x1c` | (Exempt from "late packet" drop) — likely state-sync heartbeat | Periodic |
| `0x1d` | Cinematic / Wait-N-frames | Uses `GetRadarTimer` deltas (wall-clock) |
| `0x20` | NETWORK_FRAME_BUDGET — set `g_NetworkFrameBudget`, `DAT_00a8b554`, `DAT_00a8b558` | Generated every 128 frames |
| `0x21` | Frame timing measurement — local average FPS report (`auStack_70[0] = 0x21; uStack_69 = (short)(DAT_00a8b560 / DAT_00a8b564)`) | Generated every 128 frames |
| `0x23` | REMOVEPLAYER — player dropped | On peer disconnect |
| `0x24` | LATENCYFUDGE — set `DAT_00a8db9c` | When persistent stalls detected |
| `0x27` | ABOUTTOEXIT — broadcasts the player's planned EXIT frame | Player presses Quit |
| `0x28` | FALLBACKHOST — designate new host on host-loss | On host disconnect (Internet MP) |
| `0x29` | ADDRESSCHANGE — peer's network address changed | On NAT rebind / etc. |
| `0x2a`/`0x2b`/`0x2c` | Reinforcement / drop-pod / paradrop scheduling | |

A full enumeration of all `case` arms in `EventClass::Execute` belongs in
its own event-catalog doc; cross-reference is sufficient here.

---

## Tick / frame topology

`FUN_006475f0` is the **per-tick network turn manager**. Called from
`Main_Tick` (likely as part of the gameplay block after `LogicClass::AI`
returns input events). Per-tick flow:

```c
// (1) If this is a "throttling" frame (every 128th):
if ((g_CurrentFrameCounter & 0x7f) == 0) {
    measure RTT and current FPS
    if (LatencyFudge > 0): apply 1.5×/2×/3× scaling to RTT
    new_MaxAhead = (RTT_scaled * iStack_88) / 120 + LatencyFudge_floor
    clamp to [3*FrameSendRate, 256] and to <= g_NetworkFrameBudget + FrameSendRate
    new_FrameSendRate = (RTT < 31 ? 5 : 10)
    enqueue event 0x20 (NETWORK_FRAME_BUDGET) with new values
    enqueue event 0x21 (FRAME_TIMING) with current FPS average
    reset DAT_00a8b560/DAT_00a8b564 (FPS accumulators)
}

// (2) Send outgoing packets:
FUN_00649ca0(buffer, MaxAhead, DAT_00afa400 & 0xffff)
// returns number of frames just transmitted; incremented into DAT_00afa400

// (3) Wait for peers:
iVar9 = FUN_00648710(0xb4, response_time*3, ..., DAT_00afa400 & 0xffff, ...);
// 0xb4 = 180 GetRadarTimer-unit timeout (~3 seconds)
// non-zero return = error (2=NoConnection, 3=Timeout, 7=Disconnected, 8/9=Other) → terminate session

// (4) If replay-record (DAT_00a8d5f8 & 1): write to replay file
if ((DAT_00a8d5f8 & 1) != 0) FUN_0064d9e0();

// (5) Drain event queue and dispatch:
iVar9 = FUN_0064c380(piVar13, &DAT_00afa450, &DAT_00afa358);
//   For each event in the ring buffer:
//     If scheduled_frame > CurrentFrame: skip
//     If scheduled_frame < CurrentFrame: log "Packet received too late"
//     Else verify checksum, then call EventClass::Execute
// Then advance ring read-index past all consumed slots:
for (; (DAT_008b41f8 != 0 && consumed-or-past); DAT_008b41f8--) {
    DAT_008b41fc = DAT_008b41fc + 1 & 0x3fff;
}
```

### Per-frame state flow

1. **Input** (local): user clicks → enqueued into outgoing `g_CommandBuffer`
   with `*(undefined4 *)(param_1 + 3) = g_CurrentFrameCounter` (current
   frame stamp at enqueue time).
2. **Pre-broadcast**: command's scheduled execution frame is computed as
   `g_CurrentFrameCounter + MaxAhead` (the enqueuer in
   `EventClass::Execute` callers writes a future timestamp; this is
   what makes the event "deferred").
3. **Broadcast**: `FUN_00649ca0` packs the outgoing batch and sends.
4. **Wait**: `FUN_00648710` blocks until all peers have sent events for
   the upcoming frames (up to `MaxAhead` deep).
5. **Drain**: `FUN_0064c380` walks the inbound ring, fires
   `EventClass::Execute` for each due event.
6. **Advance**: `Main_Tick` continues with `LogicClass::AI`,
   `Map::Logic`, `RenderFrame_main`, `LogicClass::PerTickUpdate`,
   then `g_CurrentFrameCounter++`.

### Clock binding

| Subsystem | Clock | Evidence |
|---|---|---|
| Event scheduled frame | game-tick | `+0x03..0x06` is `int` compared against `g_CurrentFrameCounter` |
| Event queue drain | game-tick | `FUN_0064c380` runs once per `Main_Tick` |
| MaxAhead/FrameSendRate adaptation | game-tick / 128 | `& 0x7f == 0` |
| Periodic timing-info send | game-tick / 2048 | `& 0x7ff == 0` |
| Network keepalive RTT measurement | game-tick / 8 (MP) | `& 7 == 7 && g_GameMode == 4` |
| Wait-for-peers timeout | wall-clock | `0xb4` GetRadarTimer ticks = ~3000 ms |
| Outgoing command timestamps | wall-clock | `timeGetTime()` at enqueue |
| RTT measurement | wall-clock | `Network_Keepalive` uses `(t * 1000) / 60` for delta |
| Connection retry | wall-clock | `IPX_Manager.RetryDelta` is ms |

The network frame step is **fundamentally game-tick-driven** — even the
"wall-clock" subsystems (keepalive, timeouts) feed back into game-tick
adjustments via event `0x20`/`0x24`. This is what makes lockstep
deterministic across machines with different clock speeds: every peer
applies the same `EventClass::Execute` on the same `g_CurrentFrameCounter`,
no matter the wall-clock skew between machines.

---

## Multipliers and modifiers

### `g_NetworkFrameBudget` (MaxAhead)

Caps the maximum scheduled-future-frame delta. Higher → more input lag,
more jitter tolerance. Adapted via event `0x20`. Bounded by:
- Lower: `3 * FrameSendRate` (so always at least 3 send-rounds ahead)
- Upper: `g_NetworkFrameBudget + FrameSendRate` per adaptation step
- Hard cap: `((FrameSendRate + 0xf9) / FrameSendRate) * FrameSendRate`
  ≈ 250 / FrameSendRate, rounded up

### `DAT_00a8b554` (FrameSendRate)

5 or 10 in MP based on RTT < 31 ms. LAN uses configured value.

### `DAT_00a8db9c` (LatencyFudge), 0..3

Multiplier on RTT and floor on MaxAhead. See "Hardcoded constants" above.

### `g_GameMode`

| Value | Mode | Network behavior |
|---|---|---|
| `0` | Skirmish / single-player | No network — `Main_Tick` SP path, all events local |
| `1` | Campaign mission | No network |
| `2` | Campaign-related | No network |
| `3` | LAN MP | Network path; no MP-only FrameSendRate override |
| `4` | Internet MP (Westwood Online / CnCNet) | Network path; FrameSendRate=5/10 based on RTT; fallback-host on disconnect; SaveResults file written on session end |
| `5` | Replay playback | Like SP for pacing; events drained from replay file instead of network |

### `_DAT_00a8d5f8`

| Bit | Meaning |
|---|---|
| `0x01` | Replay record — `FUN_0064d9e0()` is called in `FUN_006475f0` to write events to replay file |
| `0x02` | Game transition — skips gameplay block in `Main_Tick` |

### Per-house gate `*(char *)(iVar15 + 0x1ec) || *(char *)(iVar15 + 0x1ed)`

The drain in `FUN_0064c380` only processes events for houses that have
either the `+0x1ec` flag or the `+0x1ed` flag set — likely "AI active" or
"PlayerControl active". Player-controlled and AI-driven houses get
drained; defeated/eliminated houses do not (their lingering events are
just stuck in the ring until the read index walks past them).

### `DAT_00a8e2dc` / `DAT_00a8e2d8` / `DAT_00a8e2e0` (EXIT-event scheduling)

Set by `EventClass::Execute` case `0x27` (ABOUTTOEXIT) — broadcasts the
player's planned EXIT frame as
`((g_NetworkFrameBudget + 10 + g_CurrentFrameCounter) / 10) * 10` (next
multiple of 10 after the MaxAhead-ahead point). All peers then bulk-move
any of that player's still-queued events to land at-or-before that EXIT
frame, ensuring the EXIT event is the final thing they execute. Quoted
from `EventClass::Execute` case `0x27` decompilation.

### `SpecialFlags & 0x1000` (FogOfWar)

When set, `g_NetworkFrameBudget` is effectively reduced by 10 in event-`0x20`
processing. **TS-legacy:** default off in YR (see
[game-speed-master-clock.md](game-speed-master-clock.md)). No effect in
standard YR play. If a modder forces FogOfWar=yes, expect 10 fewer frames
of MaxAhead → 10 fewer frames of input lag → tighter network requirements.

---

## Edge cases

### `g_GameMode == 5` (Replay)

`FUN_006475f0` `return`s immediately when `g_GameMode == 5`:

```c
if (g_GameMode == 5) {
    return;
}
```

In replay, there is no real network — events are pulled from the replay
file (`FUN_0064d9e0` is the replay writer; the replay reader is in the
SP-style path). Lockstep is trivial because there's only one
"participant" (the local machine replaying recorded events).

### Single-player

In `g_GameMode == 0` (skirmish), `Main_Tick`'s SP path runs `Process_QueuedEvents`
(visual color rotation) but **not** `FUN_006475f0` (no network turn). All
events execute immediately because there is no MaxAhead — every local
input has scheduled frame = `g_CurrentFrameCounter` and fires on the
next drain.

### Pause / save / load

In-game pause (`g_GameState != 0`) skips the entire gameplay block in
`Main_Tick`, which includes `FUN_006475f0`. **Player-visible effect in
MP:** if a player opens the in-game menu, the local peer stops sending
new event broadcasts. Other peers' `FUN_00648710` "Wait For Players"
will eventually time out and the session will fall to its disconnect
handler. **This is why opening the in-game menu in MP shows "Game
Paused" overlays on all peers, not just the local one** — pausing
literally desyncs the network turn. (Verification of the "Game Paused
broadcast" event itself is deferred to a future MP-UI doc.)

Save/load is normally MP-disabled. In SP, save preserves
`g_CurrentFrameCounter`, the event ring (state at suspend), and all peer
positions; loading restores and continues from the saved frame.

### Replay determinism

Determinism requires:
1. **Identical starting state** — saved at scenario start; verified by
   initial state hash in event `0x00` (or similar).
2. **Identical event sequence** — guaranteed by the event ring being
   recorded into the replay file via `FUN_0064d9e0` and replayed in
   order on the same frame.
3. **Identical RNG sequence** — `Random__RandomRanged` is seeded
   deterministically and advances on every call; since all peers
   execute the same `EventClass::Execute` calls in the same order, RNG
   stays in lockstep.
4. **Identical math** — all sim math is fixed-point (per `CLAUDE.md`
   project convention) so no FPU mode / rounding differences across
   machines.

### Mid-session adaptation cadence

128-frame adaptation interval means:
- Slowest GameSpeed (Slowest = 6 → ~10 ticks/sec): adaptation every ~13s
- Medium (3 → ~20 ticks/sec): every ~6.4s
- Fastest (0 → uncapped): every ~2–4s

The 128-frame interval is **game-tick**, not wall-clock — so if both
peers' GameSpeed differs, adaptation fires at different wall-clock rates,
**but** GameSpeed is sync'd via event `0x0d`, so this doesn't happen in
practice unless an event `0x0d` is in flight.

### Late packet drop

Events with `scheduled_frame < g_CurrentFrameCounter` are dropped (with
the warning log). Player input that arrives this late effectively never
happened — the unit will not respond to the click. Visible as "I
clicked but my unit ignored me" in laggy play.

### Outgoing queue full

When `g_CommandQueue_Count >= 0x80` (= 128), new commands are silently
discarded by every enqueuer (the standard `if (g_CommandQueue_Count <
0x80)` guard). Visible as "input freeze" in extreme lag.

### Host migration

`EventClass::Execute` case `0x27` (ABOUTTOEXIT) and case `0x28`
(FALLBACKHOST) handle host migration on Internet MP. When the current
host disconnects, the engine picks a new host (the lowest-numbered
remaining player) and broadcasts `0x28` so all peers re-target
packet-routing. Implementation lives in the IPX/SCKT subsystem —
out of scope for this timing doc.

### Network kill (`g_NetworkFrameBudget = MAX_INT`)

Defensive code (not directly observed in `FUN_006475f0` but inferred
from clamp behavior in adaptation): if the engine determines that no
recovery is possible (peer count drops below 2, host gone, fallback
exhausted), it sets `g_GameActive = 0` directly, which exits
`Main_Game`'s outer loop on the next iteration. Player sees a "you
have been disconnected" dialog and is returned to the menu.

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| Core lockstep machinery (event ring, drain, dispatch) | **Live in YR** | Every MP game uses this. |
| Adaptive MaxAhead via event `0x20` | **Live in YR** | Fires every 128 frames. |
| LatencyFudge event `0x24` | **Live in YR** | Triggered on persistent stalls. |
| FrameSendRate=5/10 RTT-based override | **Live in YR (Internet MP only)** | `g_GameMode == 4` gate. |
| `SpecialFlags & 0x1000` FogOfWar 10-frame reduction | **TS-legacy** | FogOfWar default off in YR — reduction not applied in standard play. |
| `FUN_0064d9e0` replay writer | **Live in YR** | Replays are a shipped YR feature. |
| `g_GameMode == 5` replay playback | **Live in YR** | Replay loading. |
| Late-packet drop except type `0x1c` | **Live in YR** | Verified in drain loop. |
| 16384-slot inbound ring buffer | **Live in YR** | Sizing inherited from TS but the buffer is actively used. |
| 256-slot secondary ring (`& 0xff`) | **Live in YR** | Used for "packed events" / mega-mission queue. |
| 128-slot outgoing command buffer | **Live in YR** | All MP enqueue paths use it. |
| `IPX_Manager` (despite the name "IPX") | **Live in YR** | Class name is a TS-era IPX-protocol leftover; the actual transport is TCP/UDP for Internet MP and UDP for LAN. The class abstracts both. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — defines
  `g_CurrentFrameCounter`, the GameSpeed slider, event `0x0d`
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — `EventClass::Execute`
  case-by-case dispatch table, pause behavior, session-end flags
- [animation-rate-delay.md](animation-rate-delay.md) — `Random__RandomRanged`
  determinism (animations stay in lockstep across peers)

---

## Coverage audit

| Item | Disposition |
|---|---|
| `g_NetworkFrameBudget` (MaxAhead) | Owned here |
| `DAT_00a8b554` (FrameSendRate) | Owned here |
| `DAT_00a8b558` (FrameRate target / divisor) | Owned here |
| `DAT_00a8db9c` (LatencyFudge) | Owned here |
| `DAT_00a8b1f8` / `DAT_00a8b1dc` (event bulk-move boundaries) | Owned here |
| `DAT_00a8b568` (session-max MaxAhead) | Owned here |
| `DAT_00a8b56c` / `DAT_00a8b570` (precalculated timing override) | Owned here (used in initial session handshake) |
| Event ring `DAT_008b4204` (16384 × 0x6f bytes) | Owned here |
| Secondary ring `DAT_00a83edc` (256 × 0x6f bytes) | Owned here |
| Outgoing `g_CommandBuffer` (128 × 0x6f bytes) | Owned here |
| State checksum ring `DAT_00b04474` (256 entries) | Owned here |
| Event types `0x1b`, `0x20`, `0x21`, `0x24`, `0x27`, `0x28`, `0x29` | Owned here (network-step events) |
| Event types `0x04`/`0x05` (movement), `0x08` (Flag-to-Win) | Cross-referenced; semantics in a future "player-orders" doc |
| Event type `0x0d` (set GameSpeed) | Owned by [game-speed-master-clock.md](game-speed-master-clock.md); cross-referenced |
| Event type `0x13` (EXIT) | Owned by [logic-vs-render-loop.md](logic-vs-render-loop.md)'s session-end flag table |
| Event type `0x1c` (state-sync heartbeat) | Owned here (only known property: exempt from late-drop) |
| Event type `0x1d` (Wait-N-frames cinematic) | Owned here (uses wall-clock GetRadarTimer) |
| Per-peer RTT array `DAT_00a8b5b4 + i*0x68` | Owned here |
| `0xb4 = 180` Wait-For-Players timeout | Owned here |
| `0x7f` (128-frame adaptation) / `0x7ff` (2048-frame timing-info) cadences | Owned here |
| `0xff` / `0x3fff` / `0x7f` ring-mask sizes | Owned here |
| `IPX_Manager.RetryDelta` / `MaxRetries` / `RetryTimeout` | Cross-referenced; exact values deferred to a future networking doc |
| `Network_Keepalive` `& 7 == 7` cadence | Cross-referenced; owned here |
| `Desync_Handler` deselect-all behavior | Owned here |
| Host migration (case `0x28`) | Cross-referenced; deferred to a future MP-host-migration doc |
| `Process_QueuedEvents` color-rotation handler | Cross-referenced (visual effect, not network) |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| `search_functions "Network"` | `Network_Keepalive @ 0x00542520`, `Network_ServiceLoop @ 0x0048d080`, `Process_NetworkMessages @ 0x005d4d50` |
| `decompile_function 0x00542520` | Per-peer RTT measurement, stores per-peer at `DAT_00a8b5b4 + i*0x68` |
| `decompile_function 0x0048d080` | Service loop — drains IPX manager, calls FUN_0048d1e0 for MP, handles audio cue replication |
| `search_functions "QueuedEvents"` | Only `Process_QueuedEvents @ 0x0053b560` — **not** the EventClass dispatcher (it's color rotation) |
| `decompile_function 0x0053b560` | Confirmed color-rotation; not relevant to network |
| `search_functions "EventClass"` | No `EventClass::Constructor` etc. visible — only the static `Execute` |
| `search_functions "Execute"` | `EventClass::Execute @ 0x004c6cb0` |
| `get_function_callers 0x004c6cb0` | Single caller: `FUN_0064c380` (the queue drainer) |
| `decompile_function 0x0064c380` | Confirmed event-ring drain loop, late-packet drop, checksum compare, dispatch via `EventClass::Execute` |
| `search_functions "Desync"` | `Desync_Handler @ 0x0048dc90` |
| `decompile_function 0x0048dc90` | Deselect-all-units + reset selection mode |
| `decompile_function 0x004c66c0` | `EventClass` constructor — stamps `g_CurrentFrameCounter` into `+3..6` |
| `search_strings "MaxAhead"` | 6 hits including format strings and `"PrecalcMaxAhead is %d"` |
| `search_strings "FrameSendRate"` | 3 hits: `0x00837e98`, `0x00838e04`, `0x0084c540` |
| `search_strings "LatencyFudge"` | 4 hits including `"LATENCYFUDGE event created - %d"` |
| `get_xrefs_to 0x00824090` | "LATENCYFUDGE event created" — from `0x004f15d7` (unnamed but inside `FUN_004f1...` family — options dialog?) |
| `get_xrefs_to 0x00828398` (`"MaxAhead is %d"`) | `FUN_00540c60` — `IPX_Manager.RetryDelta` setter |
| `decompile_function 0x00540c60` | Confirmed `IPX_Manager` RetryDelta + log "MaxAhead is %d" with `g_NetworkFrameBudget` value |
| `get_xrefs_to 0x00a8b554` (FrameSendRate) | 15 read sites incl. `FUN_006475f0` (network turn), `EventClass::Execute` |
| `decompile_function 0x006475f0` | **The per-tick network turn manager** — 128-frame adaptation, MaxAhead clamps, event 0x20/0x21 emission, FrameSendRate=5/10 logic, LatencyFudge multiplier table, late-packet drop guard |
| `decompile_function 0x00659211` (`RadBeam::DrawAndTickAll`) | Confirmed `DAT_00a8b558` used as divisor for tick-fraction normalization — supports "frames per network turn" interpretation |
