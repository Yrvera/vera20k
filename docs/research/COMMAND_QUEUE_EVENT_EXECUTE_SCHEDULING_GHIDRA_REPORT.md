# Command Queue, EventClass::Execute, and Lockstep Scheduling — Ghidra Research Report

**Date:** 2026-05-28
**Addresses:** `FUN_004C6AE0` (event builder), `EventClass__Execute @ 0x004C7600` / `0x004C6CB0`,
`FUN_00649ca0` (network send / execute-frame stamp), `FUN_00647260` (DoList drain caller),
`FUN_0064c380` (DoList execute loop), `FUN_006475f0` (multiplayer network heartbeat)
**Active in YR:** Yes — entire pipeline is live in g_GameMode 3 (LAN) and 4 (WOL);
skirmish/campaign use the simpler single-player path through `FUN_00647260` `case 0/5` branch.

---

## Target Question

How are player commands queued, stamped with an execute-frame, ordered deterministically
across peers, and dispatched by `EventClass::Execute`?

## Non-Goals

- Frame barrier implementation (slot 1)
- MaxAhead negotiation formula (slot 2)
- FrameSendRate timing (slot 3)
- CRC / desync detection (slot 5)

## Evidence Needed for COMPLETE

| Item | Status | Evidence |
|------|--------|----------|
| Event record layout and size | Met | `decompile_function 0x004C6AE0`; `decompile_function 0x004C7600` (case 0x27 queue write) |
| Execute-frame stamp math | Met | `decompile_function 0x00649ca0` — `*(param_2+3) = g_CurrentFrameCounter + param_4` |
| DoList buffer layout | Met | `decompile_function 0x0064c380` — stride `0x6f`, `DAT_008b4204`, cap `0x3fff` |
| DoList drain / ordering | Met | `decompile_function 0x0064c380` — house-indexed outer loop + frame-guard condition |
| EventClass::Execute dispatch | Met | `decompile_function 0x004C7600` / `0x004C6CB0` — full switch body |
| Single-player path | Met | `decompile_function 0x00647260` — `case 0/5` copies g_CommandBuffer → DoList directly |

## Stop Conditions

All five items above met. No further Ghidra sessions needed for this slice.

---

## 1. Event Record Layout (0x6F bytes = 111 bytes)

Confirmed via `decompile_function 0x004C6AE0` (the event builder) and the copy loops in
`EventClass__Execute` case 0x27 (`get_assembly_context` `0x004C6B39`, `0x004C6B3F`):

```
event[+0x00]  byte   opcode (event type)
event[+0x01]  byte   flags (bit 0 = "executed" marker; cleared during DoList copy: `& 0xfe`)
event[+0x02]  byte   house/player ID (signed; 0xFF = "all" / invalid)
event[+0x03]  uint32 execute-frame  ← stamped by builder as g_CurrentFrameCounter at enqueue time;
                                       overwritten to (issue-frame + MaxAhead) by network send path
event[+0x07]  uint32 arg0 (meaning depends on opcode)
event[+0x0B]  uint32 arg1
event[+0x0F]  uint32 arg2
event[+0x13]  uint16 coord_x / extra
event[+0x15]  uint16 coord_y / extra
... (remaining bytes to 0x6E carry command-specific payload)
```

The builder writes `event[+3] = g_CurrentFrameCounter` (verified via
`get_assembly_context 0x004C6B39`: `MOV ECX,[0x00A8ED84]` then `MOV [ESI+3],ECX`).
This is the **issue frame**, not the execute frame — the execute frame is written later by
the network send path (`FUN_00649ca0`), which overwrites field `+3` and `+0x07` with the
current player ID before placing the record into the DoList.

Verified: `decompile_function 0x004C6AE0`.

---

## 2. Local Command Ring Buffer (g_CommandBuffer)

```
g_CommandBuffer              @ ~0x00A802D5   ring buffer of 0x6F-byte event records
g_CommandQueue_WriteIndex    @ ~0x00A802CC   masked with 0x7F on every write
g_CommandQueue_Count                         current depth; cap checked against 0x80 (128)
g_CommandTimestamps[i]       @ ~0x00A8ADDC   timeGetTime() per slot (4 bytes × 128)
```

The cap check is `if (g_CommandQueue_Count < 0x80)` before every enqueue. If full, the
command is silently dropped. The write-index wrap is `+1 & 0x7F`.

Verified: `decompile_function 0x004C7600` case 0x27 write loop; `decompile_function 0x006475f0`
enqueue of event 0x21; `get_assembly_context 0x004C6B39`.

**Scale flag (30-player concern):** The buffer cap is 128 entries across ALL pending
commands from the local player. This is a per-peer local queue, not a per-house structure,
so it is not sized for player count. However, 128 slots may be tight under sustained
heavy-order spam from 20k units. Rust may use a larger ring (e.g., 1024) without
behavioral change — the contract is FIFO ordering within the player's stream, not the
exact depth 128.

---

## 3. Execute-Frame Stamping: issue-frame + MaxAhead

The execute-frame is written in `FUN_00649ca0` (the network send helper called from
`FUN_006475f0`), not in the event builder.

**Standard (non-FrameSendRate-2) path** (`DAT_00a8b24c != 2`):

```c
*(int *)(param_2 + 3) = g_CurrentFrameCounter + param_4;
```

where `param_4` = `g_NetworkFrameBudget` (MaxAhead). This is a simple addition:
execute_frame = issue_frame + MaxAhead.

**FrameSendRate-2 path** (`DAT_00a8b24c == 2`): rounds up to next FrameSendRate boundary:

```c
*(uint *)(param_2 + 3) =
    ((DAT_00a8b554 + g_CurrentFrameCounter - 1 + param_4) / DAT_00a8b554) * DAT_00a8b554;
```

Verified: `decompile_function 0x00649ca0` — confirmed both branches set field `+3`;
`get_assembly_context 0x0064c3bb` (DoList read-back of `+3` as the "execute frame").

The same path also overwrites field `+0x02` (house ID) with `*(g_PlayerPtr + 0x30)` and
field `+0x07` (arg0) with `DAT_00AC51FC` (frame checksum). The original event struct
arg fields survive from the command buffer entry.

---

## 4. DoList Buffer (Received-from-All-Peers)

```
DAT_008b4204   base of DoList ring — 0x6F bytes × 0x4000 (16384) entries
DAT_008b4200   write index (masked & 0x3FFF)
DAT_008b41f8   current fill count (cap 0x3FFF = 16383 entries)
DAT_008b41fc   read index (masked & 0x3FFF) — drain pointer used in FUN_0064c380
DAT_00A70204   timeGetTime() timestamps for DoList entries (4 bytes × 0x4000 entries)
```

Entry layout within DoList is identical to the event record layout (`0x6F` bytes).
Field offsets within a DoList entry:
- `+0`: opcode (type byte)
- `+1`: flags (bit 0 = executed marker)
- `+2`: house ID (signed byte)
- `+3`: execute-frame (uint32)
- `+0x0B`: packed as `+0x20B` offset from base = contains the MaxAhead value for type-0x1c
  (TIMING) entries at `+0xD`

Verified: `decompile_function 0x0064c380` — stride `[ECX + ECX*0x2 + 0x8b4204]` = base
`0x8b4204` with scale factor confirming stride `3 × dword = 0x6F` bytes (LEA pattern);
`get_assembly_context 0x0064c3bb`.

**Scale flag (30-player concern):** The DoList is 16384 entries (vs. the local ring's 128).
At 30 players × high command rate, saturation is possible but unlikely within typical
MaxAhead windows. The 0x3FFF cap is not house-count-dependent. Rust may safely use a
larger VecDeque without behavioral change.

---

## 5. Cross-Peer Ordering (Determinism Contract)

Ordering is enforced by the **execute-frame field** (`event[+3]`), not by peer ID.

In `FUN_0064c380` (the DoList drain loop, called from `FUN_006475f0` and `FUN_00647260`):

1. **Outer loop** iterates over `g_HouseClass_Array` entries (per-house scan).
2. **Inner loop** walks the DoList read-forward from `DAT_008b41fc`, matching entries where
   `event[+2] == HouseClass[i].house_id` AND `event[+3] <= g_CurrentFrameCounter` AND
   `event[+1] & 1 == 0` (not yet executed).
3. When a matching entry is found: if `event[+3] < g_CurrentFrameCounter`, the "packet
   received too late" log fires (on network modes only); then `EventClass__Execute` is
   called regardless.
4. After execution the entry is marked: `event[+1] |= 1`.

This means commands are executed in **house-array order** (fixed at game start) for any
given execute-frame. Within a single house's stream, commands are ordered by their
DoList insertion sequence (network arrival order), not by original issue order.

The type-0x1c (TIMING/barrier) entries are specifically skipped from execution dispatch
and only consumed for their latency data — confirmed by the `CMP DL,0x1c` / `JZ 0x0064c63d`
branch that skips the `EventClass__Execute` call.

Verified: `decompile_function 0x0064c380` full body; `get_assembly_context 0x0064c581`.

**Scale flag (30 players):** `g_HouseClass_Array` is indexed `0..g_HouseClass_Array_Count`.
In gamemd this array is sized for 8 entries. At 30 players Rust must use a dynamically
sized collection. The ordering contract (house-array-index order per execute-frame) must
be preserved exactly — the Rust equivalent must sort by player registration order, not by
arbitrary BTreeMap key.

---

## 6. EventClass::Execute Dispatch (0x004C7600 / 0x004C6CB0)

The function receives a raw `undefined1 *param_1` pointer to the event record.
`iVar13 = (int)(char)param_1[2]` extracts the house ID, then `switch(*param_1)` dispatches
on the opcode byte.

Key cases verified via `decompile_function 0x004C7600`:

| Case | Opcode | Behavior |
|------|--------|----------|
| 0x0B | Place Production | reads `param_1[+0x13]` and `[+0x15]` → `HouseClass__Place_Production` |
| 0x0E | Begin Production | → `HouseClass__Begin_Production(this)` |
| 0x0D | Game speed change | `DAT_00A8EB60 = *(param_1+7)` (game speed index) |
| 0x13 | EXIT / disconnect | logs frame, sets `DAT_00A83D48 = 1` at `0x004C7917` |
| 0x1B | MaxAhead update | `g_NetworkFrameBudget = (uint)(byte)param_1[0xd]` |
| 0x20 | Network FPS / MaxAhead negotiate | updates `DAT_00A8B558`, `g_NetworkFrameBudget`, `DAT_00A8B554`; handles FrameSendRate-2 reschedule |
| 0x23 | Remove player | logs frame, calls `HouseClass__Flag_To_Win_Check` for player-control house |
| 0x24 | Latency fudge | `DAT_00A8DB9C = *(param_1+7)` |
| 0x27 | About-to-exit | inserts own EXIT event (opcode 0x13) into command queue |

The function is entered from `FUN_0064c380` (the DoList drain) directly — there is only
one caller (`get_function_callers 0x004C7600` → `FUN_0064c380`).

Verified: `decompile_function 0x004C7600`.

---

## 7. Single-Player / Skirmish Path

In `FUN_00647260`, for `g_GameMode == 0` (campaign) or `5` (skirmish), the path skips the
network send path entirely and moves directly from `g_CommandBuffer` into the DoList:

```c
while (g_CommandQueue_Count != 0) {
    // copy g_CommandBuffer[DAT_00a802cc] → DoList[DAT_008b4200]
    // advance both ring indices
    FUN_00652540(); // dequeue the command
}
FUN_0064c380(0, 0, 0); // execute everything now
```

The execute-frame field in this case is whatever `FUN_004C6AE0` wrote (`g_CurrentFrameCounter`
at enqueue time) — no MaxAhead offset is added. Commands execute on the very next drain call
after enqueue.

Verified: `decompile_function 0x00647260` `switchD_006474d5_caseD_0` branch.

---

## 8. Implementation Handoff

### H1: Execute-Frame Stamping

**Behavior:** Network path stamps `execute_frame = issue_frame + MaxAhead` (or
round-up for FrameSendRate-2) into event field `+3` when moving the event from
`g_CommandBuffer` to the outgoing network packet and DoList. Single-player skips
this and uses the raw issue-frame.

**Rust delta:** The Rust command queue must overwrite the execute-frame field at
network-send time, not at enqueue time. Stamping at enqueue (with `current_frame`)
and never correcting it will cause all commands to execute one MaxAhead too early
(zero-latency, bypassing the lockstep barrier). The stamp override happens in the
per-frame send function, not in the event builder.

**Surface:** `src/sim/world/world_commands.rs` or wherever the Rust network send
path stages outgoing events.

**Acceptance:** All peers receive events with `execute_frame > current_frame` by
at least `MaxAhead` frames; single-player events execute with zero additional latency.

**Test:** `test_execute_frame_stamped_as_issue_plus_maxahead` — build two events at
frames 100 and 101, run the stamp logic with MaxAhead=15, assert
`event.execute_frame == 115` and `116` respectively.

**Risk:** HIGH. Wrong stamp value causes all commands to execute at the wrong frame.
Peers that use current_frame instead of (current_frame + MaxAhead) will diverge
immediately on frame 1.

---

### H2: Cross-Peer Ordering (House-Array Order)

**Behavior:** Within a single execute-frame, events from different peers are executed in
`g_HouseClass_Array` index order (player registration order), not by network arrival
order, not by peer ID value, not sorted by execute-frame within a batch.

**Rust delta:** If the Rust event executor uses BTreeMap or HashMap iteration order,
it must be reconciled with game-start registration order. Any ordering mismatch
between peers causes divergence even when both peers have the same set of events at
the same frame. The canonical ordering key is `HouseClass.house_array_index`, not
`house_id` byte, not `PlayerClass` address.

**Surface:** `src/sim/world/world_commands.rs` or wherever the per-frame event dispatch
loop iterates houses.

**Acceptance:** Given two peers with players registered in order [A, B, C], both peers
process player A's commands before B's, B's before C's, within any single execute-frame.

**Test:** `test_commands_ordered_deterministically_across_peers` — three players each
issue one command at execute-frame 10; assert the Rust dispatch processes them in
registration-index order [0, 1, 2], not insertion or ID order.

**Risk:** HIGH. This is the primary determinism contract. Any reordering between peers
on any frame produces a desync.

---

### H3: DoList Execute-Frame Guard

**Behavior:** `FUN_0064c380` only executes an event when `event[+3] <= g_CurrentFrameCounter`.
Events stamped for a future frame are left in the DoList and re-checked each tick. Events
with `event[+3] < g_CurrentFrameCounter` (late packets) fire a "received too late" log in
multiplayer but still execute.

**Rust delta:** The Rust DoList must not execute events whose execute-frame is in the future.
Late events (execute_frame < current_frame) must not be silently dropped — they must execute
on the current frame with an optional diagnostic. If Rust drops late events, player commands
can be silently lost during network hiccups.

**Surface:** Frame-by-frame event dispatch in `src/sim/world/world_commands.rs`.

**Acceptance:** An event stamped for frame N is not dispatched before frame N; an event
stamped for frame N-1 received at frame N is dispatched at frame N, not dropped.

**Test:** `test_late_event_executes_not_dropped` — inject an event with execute_frame =
current_frame - 1; assert it executes on current_frame rather than being discarded.

**Risk:** MEDIUM. Late events in well-behaved games are rare. Incorrect handling causes
silent command loss during lag, which diverges game state from the peer that received
the event on time.

---

## 9. Negative Facts / Do Not Do

1. **Do not stamp the execute-frame at event-build time.** The builder writes
   `g_CurrentFrameCounter` into field `+3`, but this is a placeholder. The real
   execute-frame is written by `FUN_00649ca0` at send time.
   Verified: `decompile_function 0x004C6AE0` (builder) vs. `decompile_function 0x00649ca0` (stamper).

2. **Do not sort DoList events by frame within a batch.** Events at the same execute-frame
   are dispatched in house-array order, not by a secondary sort key.
   Verified: `decompile_function 0x0064c380` — no sort, strictly iterates HouseClass array.

3. **Do not drop events whose opcode is 0x1C (TIMING barrier).** They are consumed from the
   DoList but not forwarded to `EventClass__Execute` — only their latency fields are read.
   Verified: `get_assembly_context 0x0064c581` — `CMP DL,0x1c; JZ 0x0064c63d` skips the
   `EventClass__Execute` call.

4. **Do not use a 128-slot cap as a behavioral constraint.** The 0x80 cap on `g_CommandBuffer`
   is a gamemd resource limit, not a protocol rule. Rust may safely use deeper queues.
   Verified: `decompile_function 0x004C7600` case 0x27 — cap check is `< 0x80` not equality.

5. **Do not skip execution of late events in multiplayer.** The "received too late" log fires
   but execution continues. Late-event execution is mandatory for correctness.
   Verified: `decompile_function 0x0064c380` — late check writes to log then falls through
   to `EventClass__Execute`.

---

## 10. Remaining Uncertainty

1. **Exact per-house DoList slot assignment:** `FUN_0064c380` matches on `event[+2] ==
   HouseClass[i].house_id`. Whether `house_id` and the house-array index are always the same
   value (in standard 8-player lobby), or can diverge (e.g. after player drop / REMOVEPLAYER),
   was not fully traced. In the REMOVEPLAYER path (`EventClass::Execute` case 0x23) the
   HouseClass entry is not removed from the array — it is marked `HouseClass[i].field_0x248 = 1`.
   The house-array-order dispatch therefore remains valid post-disconnect.
   Confidence: MEDIUM — caller trace confirms `event[+2]` is used as the house ID match, but
   the mapping from lobby slot to house_id in games with bot fills was not verified.

2. **g_CommandBuffer base address:** Addresses `0x00A802D5` / `0x00A802CC` / `0x00A802DF`
   are read from `FUN_00649ca0` Ghidra output without explicit `read_memory` confirmation.
   Treat as YELLOW until a `read_memory` or label cross-check confirms the symbol binding.

---

## 11. Stale-Doc Note

`DISPLAYCLASS_BANDBOX_AND_MI_CORRECTION_GHIDRA_REPORT.md` (§ "Command queue") correctly
documents the 0x80 cap, 0x7F-masked index, 0x6F record size, and `timeGetTime` stamps.
This report extends it with: execute-frame math, DoList layout, cross-peer ordering
mechanism, and the `EventClass::Execute` full dispatch table. No corrections to the
existing doc are needed.

---

## 12. Sources

**Ghidra functions decompiled (READ-ONLY — no mutations performed):**
- `0x004C6AE0` — event builder (`FUN_004c6ae0`); field layout
- `0x004C6CB0` / `0x004C7600` — `EventClass__Execute`; full dispatch switch
- `0x00649ca0` — `FUN_00649ca0` (network send / execute-frame stamp)
- `0x00647260` — `FUN_00647260` (DoList drain caller; single-player vs multiplayer split)
- `0x0064c380` — `FUN_0064c380` (DoList execute loop; per-house ordering)
- `0x006475f0` — `FUN_006475f0` (multiplayer network heartbeat)

**Assembly context:**
- `get_assembly_context 0x004C6B39` — confirmed `[ESI+3] = g_CurrentFrameCounter` in builder
- `get_assembly_context 0x0064c3bb` — DoList read-index pattern, stride `0x6f`, base `0x8b4204`
- `get_assembly_context 0x0064c581` — type-0x1c skip, execute path, house-id match

**Callers:**
- `get_function_callers 0x004C7600` — sole caller is `FUN_0064c380`
- `get_function_callers 0x004C6AE0` — three callers: `DisplayClass__BandBox_LeftUp`,
  `SelectClass__Action`, `StripClass__AI`
