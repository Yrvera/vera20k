# FrameSendRate — Command Send Cadence (Lockstep)

**Target question:** What is FrameSendRate — its source, default value, what it gates (how often the local peer broadcasts command batches), and its relationship to MaxAhead and the 10-frame boundary?

**Non-goals:** Frame barrier mechanics (slot 1), MaxAhead computation deep-dive (slot 2), EventClass::Execute internals (slot 4), CRC/desync detection (slot 5).

**Evidence for COMPLETE:** All three anchor strings traced to their functions; `FUN_006475f0` (per-tick network turn manager) fully decompiled; `FUN_00794ba0` (session-start initializer) fully decompiled; `FUN_00649ca0` (send function) fully decompiled. Send-gating expression, default values, LAN vs WOL distinction, and MaxAhead bounds all verified directly.

**Stop conditions:** FrameSendRate source (INI vs hardcode), default/range, send-gate expression, MaxAhead floor/ceiling relationship, LAN vs WOL distinction — all resolved.

**Active in YR:** Yes — verified live code path for `g_GameMode == 3` (LAN) and `== 4` (WOL/Internet).

---

## 1. What is FrameSendRate?

`FrameSendRate` = `DAT_00a8b554` (1-byte global). It is the number of frames the local peer waits between consecutive outgoing command-batch transmissions. A value of 5 means the local machine sends its accumulated input batch every 5 game frames; a value of 10 means every 10 frames.

**It does NOT come from INI.** No `rulesmd.ini` / `artmd.ini` key configures it. It is hardcoded at session start and adaptively updated via event `0x20`.

verified via `decompile_function 0x006475f0` and `decompile_function 0x00794ba0`.

---

## 2. Initial Value (Session Start)

Set in `FUN_00794ba0` (the "Start Game Now" function, reached from the lobby UI), verified via `decompile_function 0x00794ba0`:

```c
DAT_00a8b554 = 5;   // default before switch
switch(DAT_00a8b268) {   // DAT_00a8b268 = connection-speed preset (0..5, or default)
  case 0: g_NetworkFrameBudget = 0x28; DAT_00a8b570 = 0x3c; DAT_00a8b554 = 10; break;
  case 1: g_NetworkFrameBudget = 0x28; DAT_00a8b570 = 0x2d; DAT_00a8b554 = 10; break;
  case 2: g_NetworkFrameBudget = 0x1e; DAT_00a8b570 = 0x1e; break;   // keeps 5
  case 3: g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x14; break;   // keeps 5
  case 4: g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x0f; break;   // keeps 5
  case 5: g_NetworkFrameBudget = 0x14; DAT_00a8b570 = 0x0c; break;   // keeps 5
  default: g_NetworkFrameBudget = 10;  DAT_00a8b570 = 10;            // keeps 5
}
```

**Default: 5.** Overridden to **10** only for connection-speed presets 0 or 1 (the two slowest presets, representing high-latency connections).

Then it is immediately logged:
```c
Register_heap_pool(s_FrameSendRate_is__d_0084c540, DAT_00a8b554);
```

This explains anchor string `0x0084c540` ("FrameSendRate is %d") at function `FUN_00794ba0` — it fires exactly once at session start. (verified via `get_xrefs_to 0x0084c540` → `FUN_00794ba0`)

---

## 3. Runtime Override (WOL / Internet only)

In `FUN_006475f0` (per-tick network turn manager, called every 128 frames for adaptation), verified via `decompile_function 0x006475f0`:

```c
bStack_65 = (byte)DAT_00a8b554;   // read current value
if (g_GameMode == 4) {             // WOL/Internet only
    bStack_65 = ((iStack_88 < 0x1f) - 1U & 5) + 5;
    // iStack_88 = capped measured max RTT in ms (from peer ping array)
    // if iStack_88 < 31 ms: ((1-1) & 5) + 5 = 0 + 5 = 5
    // if iStack_88 >= 31 ms: ((0-1) & 5) + 5 = 5 + 5 = 10
}
```

So in WOL/Internet: new FrameSendRate = **5** if max-peer RTT < 31 ms, else **10**.

In LAN (`g_GameMode == 3`): **no override** — uses the static value set at session start (5 or 10 from preset).

The computed `bStack_65` is packed into an event `0x20` (NETWORK_FRAME_BUDGET) and enqueued into `g_CommandBuffer`:

```c
auStack_70[0] = 0x20;
// ... payload includes new MaxAhead (sStack_67) and new FrameSendRate (bStack_65)
// then standard g_CommandBuffer enqueue
```

Event `0x20`'s `EventClass::Execute` case writes it back:
```c
DAT_00a8b554 = (uint)param_1[0xb];   // param_1[0xb] = new FrameSendRate byte
```

(verified via the `case 0x20:` body in the existing `multiplayer-frame-step.md` doc, address `0x006475f0` event-construction block).

Anchor string `0x00837e98` ("FrameSendRate=%d") is in `FUN_00652070` (late-packet diagnostic logger), which fires when an event arrives after its scheduled frame. verified via `get_xrefs_to 0x00837e98` → `FUN_00652070`, `decompile_function 0x00652070`.

Anchor string `0x00838e04` ("FrameSendRate: %d") is in `FUN_0064dea0` (soft desync recovery). It logs the current FrameSendRate when entering the recovery path. verified via `get_xrefs_to 0x00838e04` → `FUN_0064dea0`.

---

## 4. How FrameSendRate Gates Outgoing Sends

`FUN_006475f0` contains the send-gating check **before** calling `FUN_00649ca0`:

```c
if ((DAT_00a8b24c == 2) &&
    (g_CurrentFrameCounter !=
     (((DAT_00a8b554 - 1) + g_CurrentFrameCounter) / DAT_00a8b554) * DAT_00a8b554)) {
    (**(code **)(*piVar13 + 4))();   // receive-only tick, no send
    ...
    return;
}
// fall through to:
sVar2 = FUN_00649ca0(local_8c, g_NetworkFrameBudget, DAT_00afa400 & 0xffff);
```

Breaking down the condition `g_CurrentFrameCounter != (((FrameSendRate - 1) + g_CurrentFrameCounter) / FrameSendRate) * FrameSendRate`:

- `(((FrameSendRate - 1) + frame) / FrameSendRate) * FrameSendRate` is integer division rounding frame up to the **next multiple of FrameSendRate** (ceiling-multiple formula).
- When `frame` is already a multiple of FrameSendRate, `ceil = frame`, and the condition is false → **send happens**.
- When `frame` is not a multiple, `ceil > frame`, condition is true → **send is skipped**, only a receive-side tick runs.

**Conclusion:** sends happen exactly on frames that are exact multiples of FrameSendRate.

- FrameSendRate = 5: frames 0, 5, 10, 15, 20 ...
- FrameSendRate = 10: frames 0, 10, 20, 30 ...

`DAT_00a8b24c == 2` is the condition that enables this gating (MP send mode, as opposed to single-player or replay mode).

verified via `decompile_function 0x006475f0` — the `return` path and the direct call to `FUN_00649ca0`.

---

## 5. Relationship to MaxAhead and the "10-frame boundary"

The "10-frame batch boundary" referenced in the DEFEAT_WIN_LOSS doc refers to the alignment formula above, which aligns command execution frames to multiples of FrameSendRate. **When FrameSendRate = 10, the alignment ceiling rounds command execution up to multiples of 10** — this is the "10-frame boundary."

The formula in `FUN_00649ca0` (send function) confirms command scheduling in MP:
```c
*(uint *)(param_2 + 3) =
    ((DAT_00a8b554 + g_CurrentFrameCounter + -1 + param_4) / DAT_00a8b554) * DAT_00a8b554;
// = ceiling((CurrentFrame + MaxAhead - 1) / FrameSendRate) * FrameSendRate
// = next FrameSendRate-aligned frame at or after CurrentFrame+MaxAhead
```
(verified via `decompile_function 0x00649ca0`, the `DAT_00a8b24c == 2` branch)

Commands are therefore scheduled on the **next FrameSendRate-aligned frame ≥ (CurrentFrame + MaxAhead)**.

MaxAhead bounds as a function of FrameSendRate (from `FUN_006475f0`):
- **Floor:** `FrameSendRate × 3` — MaxAhead is always at least 3 send-rounds ahead.
- **Max growth per step:** `FrameSendRate` — grows at most one send-round per adaptation.
- **Hard ceiling:** `((FrameSendRate + 0xf9) / FrameSendRate) * FrameSendRate` = the largest FrameSendRate-aligned multiple ≤ 256.

For FrameSendRate=5: floor=15, hard ceiling=255. For FrameSendRate=10: floor=30, hard ceiling=250.

---

## 6. Distinction from Network_Keepalive (every-8-frames WOL)

`Network_Keepalive` @ `0x00542520` fires every 8 frames in WOL (`(g_CurrentFrameCounter & 7) == 7 && g_GameMode == 4`). It **measures per-peer RTT and stores into the per-peer array** (`DAT_00a8b5b4 + i*0x68`). This is a **measurement cadence**, not a send cadence. The measured RTT feeds into the adaptation logic in `FUN_006475f0` (the `iStack_88` variable above). FrameSendRate is the **send cadence** — unrelated to the 8-frame keepalive period. They are independent clocks with different purposes.

---

## 7. Scale Concern (30-player target)

gamemd.exe's per-peer arrays are sized for 8 players (`g_HouseClass_Array_Count`, `DAT_00a8b5b4 + i*0x68` loop bounds). The per-peer RTT array is 8 × 0x68 = 0x340 bytes. At 30 players, these must be expanded. FrameSendRate itself is a single global; its semantics are unchanged at any player count. The fan-out concern is in `FUN_00649ca0`, which sends one packet per peer (via `(**(code **)(*param_1 + 8))(...)`) on each send frame — at 30 players and FrameSendRate=5 that is 30 × (game_ticks_per_second / 5) sends/sec per peer, which needs transport-layer fan-out profiling.

---

## 8. INI Surface (None)

No key in `rulesmd.ini`, `artmd.ini`, or `RA2MD.INI` sets FrameSendRate. The connection-speed preset (`DAT_00a8b268`) that determines the initial value comes from the lobby UI, not from INI. verified via the switch block in `FUN_00794ba0`.

---

## Implementation Handoff

### Handoff A — Send-gating alignment

**Behavior:** Outgoing command batch is transmitted only on frames that are exact multiples of FrameSendRate (the ceiling-alignment gate in `FUN_006475f0`). On non-multiple frames, only the receive side runs.

**Rust delta:** In the MP tick path, gate `send_command_batch()` behind `current_frame % frame_send_rate == 0`. Command execution frame must be computed as `ceil_align(current_frame + max_ahead, frame_send_rate)`, matching the formula in `FUN_00649ca0`.

**Surface:** `sim/world/world_orders.rs` or a new `net/lockstep.rs` — wherever the MP command dispatch lives.

**Acceptance:** At FrameSendRate=5 with MaxAhead=15, a command issued on frame 3 must schedule for frame `ceil_align(3+15, 5) = ceil_align(18,5) = 20`. A command issued on frame 5 must schedule for frame `ceil_align(5+15,5) = 20`. Commands issued on frames 1..4 must not be sent until frame 5.

**Test:** `test_command_batch_sent_every_framesendrate` — mock tick loop with FrameSendRate=5, verify that `send_batch` is called only on frames 0,5,10,15,20 and that each batch carries execution frames aligned to the same boundary.

**Risk:** Off-by-one in the ceiling formula produces commands scheduled one FrameSendRate-step late. Walk the concrete fixture (frame=3, FSR=5, MaxAhead=15 → 20) before shipping.

### Handoff B — Session-start initialization

**Behavior:** FrameSendRate is set to 5 by default, overridden to 10 for connection-speed presets 0 or 1 (the two slowest lobby presets). No INI source.

**Rust delta:** Map the lobby connection-speed enum to `frame_send_rate: u8 = if preset <= 1 { 10 } else { 5 }`. Also initialize `max_ahead` from the same table (`0x28` / `0x28` / `0x1e` / `0x14` / `0x14` / `0x14` / `10` for presets 0..6+default).

**Surface:** Session/lobby setup code.

**Acceptance:** For preset=0: FrameSendRate=10, MaxAhead=40. For preset=3: FrameSendRate=5, MaxAhead=20.

**Test:** `test_session_init_framesendrate_by_preset` — for each of the 7 cases, assert the correct (frame_send_rate, max_ahead) pair.

**Risk:** Low — the switch table is explicit in the binary. The only edge case is `DAT_00a8e2c8 != 0` (some "high-latency" flag) which adds `+0x14` to MaxAhead; include that branch.

### Handoff C — WOL runtime adaptation

**Behavior:** Every 128 frames in WOL, FrameSendRate is recomputed from max-peer RTT: 5 if RTT < 31 ms, else 10. This is propagated via event `0x20` so all peers adopt the same value.

**Rust delta:** In the 128-frame adaptation tick (WOL only), compute `new_fsr = if max_peer_rtt_ms < 31 { 5 } else { 10 }`, pack into the `NETWORK_FRAME_BUDGET` event alongside new MaxAhead.

**Surface:** MP adaptation subsystem / event emitter.

**Acceptance:** With simulated RTT=20ms: FSR stays 5. With simulated RTT=40ms: FSR becomes 10. The event `0x20` payload byte at offset +0x0b carries the new FSR.

**Test:** `test_wol_framesendrate_adaptation` — drive the adaptation tick with synthetic RTT values and assert the emitted event payload.

**Risk:** Confirm that LAN (`g_GameMode == 3`) does NOT get this override — the `if g_GameMode == 4` guard is explicit; LAN uses the static session-start value for the life of the session.

---

## Negative Facts / Do Not Do

1. **Do not read FrameSendRate from INI.** No INI key exists — it is hardcoded and adaptive only.
2. **Do not override FrameSendRate in LAN mode.** The `g_GameMode == 4` gate is explicit; LAN uses the preset value indefinitely.
3. **Do not confuse FrameSendRate with the 8-frame WOL keepalive.** The keepalive measures RTT every 8 frames; FrameSendRate gates sends. They are independent.
4. **Do not confuse FrameSendRate with the 128-frame adaptation cadence.** The `& 0x7f == 0` check fires the RTT measurement that may propose a new FrameSendRate; FrameSendRate itself is the resulting send period.
5. **Do not confuse FrameSendRate with DAT_00a8b558** (the "target FPS" / `PrecalcDesiredFrameRate` field in event `0x20`). That is a separate field at event payload offset +7..8; FrameSendRate is at payload offset +0x0b.

---

## Remaining Uncertainty

1. **`DAT_00a8b268` (connection-speed preset) source** — the switch in `FUN_00794ba0` branches on it, but where the lobby UI writes this global (which combo-box entry maps to which index) was not traced. The 0=Modem/Slow, 5=LAN/Fast assumption is inferred from the MaxAhead/FSR pattern (case 0 most conservative, higher cases more aggressive) but not verified from the lobby code.

2. **`DAT_00a8e2c8` (high-latency flag)** — adds `+0x14` to initial MaxAhead if set. Source and conditions not traced this session.

3. **LAN FrameSendRate initial value derivation** — `FUN_00794ba0` is reachable from both LAN lobby and WOL lobby. Both paths go through the same switch; in LAN, presets 2+ yield FSR=5, presets 0/1 yield FSR=10. Whether the LAN lobby exposes presets 0/1 to the player is not verified.

---

## Stale / Extend Notes

`docs/research/timing/multiplayer-frame-step.md` already documents this system at high confidence and matches all findings here. No corrections needed; the facts in that doc are confirmed. This report adds:
- The **session-start switch table** from `FUN_00794ba0` with exact MaxAhead and FSR values per preset.
- The **send-gating expression** (ceiling-alignment) verified directly from both `FUN_006475f0` and `FUN_00649ca0`.
- Explicit confirmation that FrameSendRate is **not** the "10-frame boundary" itself, but rather the parameter the boundary aligns to — when FSR=10, the boundary is multiples of 10; when FSR=5, it is multiples of 5.

`docs/research/DEFEAT_WIN_LOSS_SYSTEM_GHIDRA_REPORT.md` — the "10-frame batch boundary" mentioned there is the ceiling-alignment to multiples of FrameSendRate at the time FSR=10. No correction needed, but the phrasing "10-frame" is a specific-case snapshot, not an invariant.

---

*Sources: `decompile_function 0x006475f0` (FUN_006475f0 per-tick network turn manager); `decompile_function 0x00794ba0` (FUN_00794ba0 session-start initializer); `decompile_function 0x00649ca0` (FUN_00649ca0 send function); `decompile_function 0x00652070` (FUN_00652070 late-packet logger); `get_xrefs_to 0x00837e98`, `0x00838e04`, `0x0084c540` (anchor string xrefs).*
