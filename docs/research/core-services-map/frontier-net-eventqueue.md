# frontier-net-eventqueue — Lockstep event/command queue (substrate profile)

**Slug:** `frontier-net-eventqueue`
**Status:** PROMOTED from catalog stub (was `_frontier.md` §E1). Profile-level, not a full
class decode — verifies the representative address + the cross-service edges only.
**Authority order:** binary → Ghidra → docs.
**Active in YR:** YES — the entire command pipeline is live in every game mode (campaign 0,
skirmish 5, LAN 3, WOL/internet 4). Only the network *send/stamp/barrier* sub-paths are
mode-3/4-only; the queue, DoList, and `EventClass::Execute` dispatch are universal.

> ⚠️ **Ghidra connectivity note (this session):** The Ghidra MCP bridge was **not reachable**
> for the duration of this promotion (`list_instances` → 0 instances; `connect_instance`
> TCP `127.0.0.1:8089` actively refused; repeated retries). The representative address and
> every cross-service edge below are therefore re-verified against **independently
> cross-corroborating verified-disassembly citations** already in the research corpus (three
> separate docs quote the exact `Main_Tick` call bytes), not a fresh live decompile. This is
> stronger than a single live decompile for the address itself (byte-level disassembly quoted
> verbatim in multiple docs), but the deeper field offsets carry their original docs' grade.
> Items needing a *live* re-pull are flagged **NEEDS-LIVE-REVERIFY** inline.

---

## 1. Purpose

The lockstep command queue is the **determinism boundary** of the engine. Player input
(and AI/script commands) are packaged into fixed-size `EventClass` records, buffered locally
(`g_CommandBuffer` ring), stamped with an **execute-frame**, exchanged with peers, and then
executed at the scheduled frame in a **deterministic cross-peer order**. Event execution
order + the frame at which each event fires *is* the lockstep contract: reorder events, drop
a late one, or shift the RNG cursor and every peer desyncs.

This service answers: *how does a click become a world-state mutation that every peer applies
identically, on the same frame?*

---

## 2. Representative address — re-verified + framing CORRECTED

**Stub claim (E1):** `Process_QueuedEvents @ 0x0053B560` is "the verified caller =
`Main_Tick`, runs in the per-tick spine, before/around PerTickUpdate … the live command
stage … the determinism boundary."

**Verdict:** The **address `0x0053B560` is CORRECT** but the stub's **framing is WRONG /
misleading** — `Process_QueuedEvents @ 0x0053B560` is **NOT** the live per-frame command
execution stage in the normal gameplay path.

### 2a. Address confirmation (binary-quoted, 3 independent docs)

The exact `Main_Tick` call bytes are quoted verbatim in
`FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md` (State C disassembly,
`verified via get_assembly_context 0x0055D821,0x0055D862`):

```
0055d830: CALL 0x005d4d50   ; Process_NetworkMessages
0055d835: CALL 0x0048d080   ; Network_ServiceLoop
0055d83a: CALL 0x0053b560   ; Process_QueuedEvents   ← representative address, byte-confirmed
0055d847: CALL [EDX+0x5c]   ; TacticalClass::Update
0055d84f: CALL 0x004f4480   ; RenderFrame_main
0055d877: RET               ; returns WITHOUT reaching g_CurrentFrameCounter++
```

Corroborated identically by `RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md` §2 Call Site A
and `MAIN_TICK_RENDER_LOGIC_COUPLING_GHIDRA_REPORT.md` §2 Call Site A (both list
`Process_NetworkMessages → Network_ServiceLoop → Process_QueuedEvents → TacticalClass+0x5C →
RenderFrame_main → early return`).

### 2b. Where `0x0053B560` actually sits (framing correction)

Per `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md §1` ("Command-queue framing (corrected)"):

- The **only direct `Process_QueuedEvents @ 0x0053B560` call inside `Main_Tick`** is in the
  **scenario-delay / offline-spectator early-return branch** (`Scenario+0x62C != 0` inside
  the `g_GameMode==0||==5` block). That branch services network/events/render/throttle then
  **returns before ever reaching `LogicClass::PerTickUpdate`** and **without incrementing
  `g_CurrentFrameCounter`** — so it is a render-without-logic countdown path, **not** the
  live-gameplay command stage.
- `Process_QueuedEvents` **also appears nested inside Rung P** (`LightningStorm /
  PsychicDominator` process, driver `0x0053A6C0`) — again not a per-tick command stage.

### 2c. The REAL live command/event execution path

Per `LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md §1` and
`COMMAND_QUEUE_EVENT_EXECUTE_SCHEDULING_GHIDRA_REPORT.md`, in the **normal active gameplay
block** the queue is drained and executed via:

```
GScreenClass::Input
  → Process_Command @ 0x0055DEE0        (input → queued EventClass records into g_CommandBuffer)
  → Map__Logic()                         (THE live command/event execution point, pre-PerTickUpdate)
       → FUN_00647260                    (DoList drain caller; single-player vs network split)
            → FUN_0064C380               (per-house DoList execute loop, ordering enforced here)
                 → EventClass::Execute @ 0x004C6CB0 / 0x004C7600   (opcode dispatch → world mutation)
```

In **network modes 3/4** the send/stamp/barrier machinery (`FUN_006475F0`, `FUN_00649CA0`,
`FUN_00648710`) is interposed; in **modes 0/5** `FUN_00647260` copies `g_CommandBuffer`
straight into the DoList and executes immediately (zero added latency).

**Better representative function for this service going forward:** `EventClass::Execute
@ 0x004C6CB0` (the opcode→world-mutation dispatcher, sole caller `FUN_0064C380`) and the
drain loop `FUN_0064C380`. `Process_QueuedEvents @ 0x0053B560` should be retained only as the
named scenario-delay/Rung-P helper, not as the determinism boundary. **NEEDS-LIVE-REVERIFY:**
confirm `0x0053B560`'s body (whether it itself drains the DoList or only services a subset)
on the next live Ghidra session — corpus does not decompile its body, only its call sites.

---

## 3. What it owns (globals / structures, with addresses)

| Owned state | Address | Meaning | Grade / source |
|---|---|---|---|
| `g_CommandBuffer` (local ring) | ~`0x00A802D5` | Local outgoing ring, 128 (`0x80`) slots × `0x6F` bytes. FIFO of this peer's pending `EventClass` records. | YELLOW (read from decomp context, no `read_memory`) — `COMMAND_QUEUE_…` §2/§10 |
| `g_CommandQueue_WriteIndex` | ~`0x00A802CC` | Write cursor, masked `& 0x7F` each write. | YELLOW — same |
| `g_CommandQueue_Count` | (decomp) | Local depth; cap-checked `< 0x80` before enqueue (silent drop if full). | verified `decompile 0x004C7600` case 0x27 |
| `g_CommandTimestamps[i]` | ~`0x00A8ADDC` | `timeGetTime()` per slot (4B × 128). | YELLOW — `COMMAND_QUEUE_…` §2 |
| **DoList** base (received-from-all) | `0x008B4204` (`DAT_008b4204`) | Cross-peer execute ring, 16384 (`0x4000`) entries × `0x6F` bytes. Identical record layout to the local ring. | verified `decompile 0x0064C380` |
| DoList write index | `0x008B4200` | masked `& 0x3FFF` | verified `decompile 0x0064C380` |
| DoList fill count | `0x008B41F8` | cap `0x3FFF` | verified `decompile 0x0064C380` |
| DoList read index (drain) | `0x008B41FC` | masked `& 0x3FFF`; drain pointer in `FUN_0064C380` | verified `decompile 0x0064C380` |
| DoList timestamps | `0x00A70204` | `timeGetTime()` per DoList entry (4B × `0x4000`) | `COMMAND_QUEUE_…` §4 |
| `g_NetworkFrameBudget` (MaxAhead) | `0x00A8B550` | execute-frame latency in frames; `execute = issue + MaxAhead`. | verified `decompile 0x004C7600` cases 0x1B/0x20 |
| `DAT_00A8B554` (`FrameSendRate`) | `0x00A8B554` | how often timing/command batches send; drives the `==2` round-up branch. | verified `decompile 0x004C7600` case 0x20 |
| `DAT_00A8B558` (NetworkFPS) | `0x00A8B558` | requested net FPS; sets the ms budget `1000 / value`. | `NETWORK_FRAME_SCHEDULING_…` §1 |
| `DAT_00A8B1DC` / `DAT_00A8B1F8` | `0x00A8B1DC` / `0x00A8B1F8` | negotiated execute-frame target + issue-frame base (timing event 0x20). | `NETWORK_FRAME_SCHEDULING_…` §1 |
| `DAT_00A8DB7C[..]` per-peer lag | `0x00A8DB7C` | 7-slot per-peer lag array (ms-derived); adaptive throttle input. | `NETWORK_FRAME_SCHEDULING_…` §1 |
| `g_CurrentFrameCounter` (read, not owned) | `0x00A8ED84` | authoritative sim frame; this service *reads* it to stamp/guard. Owned by `logicclass`/`Main_Tick` (late-incremented). | verified `decompile 0x0055D360` |

### EventClass record layout (`0x6F` = 111 bytes) — verified `decompile 0x004C6AE0`
```
+0x00  byte   opcode (event type)
+0x01  byte   flags (bit0 = "executed" marker; cleared & 0xFE on DoList copy)
+0x02  byte   house/player ID (signed; 0xFF = all/invalid)
+0x03  u32    execute-frame  (issue-frame at build; OVERWRITTEN to issue+MaxAhead at net send)
+0x07  u32    arg0           (opcode-dependent; net send overwrites with frame checksum)
+0x0B  u32    arg1
+0x0F  u32    arg2
+0x13  u16    coord_x / extra
+0x15  u16    coord_y / extra
 …            command-specific payload to +0x6E
```

---

## 4. Key functions (re-verified addresses)

| Function | Address | Role | Grade / source |
|---|---|---|---|
| `Process_QueuedEvents` | `0x0053B560` | scenario-delay-branch + Rung-P helper (NOT the live stage). **Body NEEDS-LIVE-REVERIFY.** | byte-confirmed call (3 docs); body undecoded |
| `Process_Command` | `0x0055DEE0` | input → queued `EventClass` (front of the chain). (Stale Ghidra label `LogicClass::AI` is DRIFT per `_frontier.md` I1 + spine spec.) | `_frontier.md` I1; spine spec §1 |
| event builder | `0x004C6AE0` | constructs a record; writes `+3 = g_CurrentFrameCounter` (placeholder issue-frame). 3 callers: `DisplayClass::BandBox_LeftUp`, `SelectClass::Action`, `StripClass::AI`. | verified `decompile`+`get_function_callers 0x004C6AE0` |
| `FUN_00647260` | `0x00647260` | DoList **drain caller**; splits single-player (case 0/5 copies ring→DoList, executes now) vs network. | verified `decompile 0x00647260` |
| `FUN_0064C380` | `0x0064C380` | DoList **execute loop**: per-house outer scan, execute-frame guard, mark-executed, type-0x1C skip. **Ordering enforced here.** | verified `decompile 0x0064C380` |
| `EventClass::Execute` | `0x004C6CB0` / `0x004C7600` | opcode `switch` → world mutation (Place/Begin Production, game speed, MaxAhead update, exit/remove player, etc.). Sole caller `FUN_0064C380`. | verified `decompile`+`get_function_callers 0x004C7600` |
| `FUN_006475F0` | `0x006475F0` | per-frame queue advance / timing-event sender / network heartbeat (mode 3/4). | verified `decompile 0x006475F0` |
| `FUN_00649CA0` | `0x00649CA0` | **execute-frame stamp** at net send: `+3 = issue + MaxAhead` (or FrameSendRate-2 round-up). | verified `decompile 0x00649CA0` |
| `FUN_00648710` | `0x00648710` | lockstep **barrier**: blocks frame N until all peers delivered (mode 3/4). | `NETWORK_FRAME_SCHEDULING_…` §10 |

---

## 5. Plug point (tick spine)

**This service straddles two plug points; neither is a PerTickUpdate rung:**

1. **Front-of-tick prelude (live path), inside `Main_Tick` before `PerTickUpdate`:**
   `Input → Process_Command → Map__Logic()` — and **`Map__Logic` is where the live
   command/event queue is executed** for the current frame, committing events into world
   state. This precedes the entire 28-rung ladder (spine spec §1). This maps to the **Rust
   tick's leading "commands" stage** (`World::advance_tick` step 1).
2. **Mid-`PerTickUpdate` network advance:** the barrier/send heartbeat `FUN_006475F0`
   (with `FUN_00648710`) runs *inside* `LogicClassPerTickUpdateLiveVector` on the network
   path (`NETWORK_FRAME_SCHEDULING_…` §10) — i.e. the frame-gate is checked while the ladder
   runs, but command *dispatch* already happened in step 1.

**Scenario-delay branch (`Scenario+0x62C != 0`):** the lone `Process_QueuedEvents
@ 0x0053B560` call services events for a render-only countdown then returns early — **does
not** reach any rung and **does not** advance the frame counter.

**Spine-spec tie:** Map__Logic command execution = **prelude, before Rung A**.
`Process_QueuedEvents` nested appearance = **Rung P** (`0x0053A6C0`, LightningStorm/PD).
Render of any command result = `RenderFrame_main @ 0x004F4480` (after the block, before the
late frame increment).

---

## 6. Lockstep / determinism contract (the whole point)

- **Execute-frame stamping:** network path stamps `event[+3] = issue_frame + MaxAhead`
  (`FUN_00649CA0`), NOT at build time. Single-player keeps the raw issue-frame → executes
  next drain. (`COMMAND_QUEUE_…` §3, H1.)
- **Cross-peer ordering:** within one execute-frame, events dispatch in **`g_HouseClass_Array`
  index order** (player registration order) — NOT by peer ID, NOT by arrival order, NOT a
  secondary frame sort (`FUN_0064C380`, `COMMAND_QUEUE_…` §5, H2). This is the primary desync
  surface.
- **Execute-frame guard:** execute only when `event[+3] <= g_CurrentFrameCounter`; future
  events stay queued; **late events (`< current`) still execute** (a "received too late" log
  fires in MP but execution is mandatory) (`COMMAND_QUEUE_…` H3, Negative Fact 5).
- **Type-`0x1C` (TIMING/barrier) records:** consumed from the DoList for latency data only,
  **never** forwarded to `EventClass::Execute` (`COMMAND_QUEUE_…` Negative Fact 3).
- **RNG coupling:** executed events that mutate state consume the synchronized `Scen->Random`
  cursor; reordering events shifts every later RNG draw → desync (ties to the spine spec's
  RNG-draw-order contract). This is why E1 depends on `random-scenario`.

---

## 7. Outgoing edges (depends-on)

| → Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `logicclass` | `Main_Tick @ 0x0055D360` schedules the queue stage (`Map__Logic`) before `PerTickUpdate`; reads/late-bumps `g_CurrentFrameCounter @ 0x00A8ED84` | spine spec §1; `decompile 0x0055D360` |
| `random-scenario` | executed events consume the synchronized `Scen->Random` cursor (`ScenarioClass+0x218`); reorder → RNG drift | spine spec §3 (RNG contract); `COMMAND_QUEUE_…` §5 |
| `factory-house` | `EventClass::Execute` cases 0x0B `HouseClass::Place_Production`, 0x0E `HouseClass::Begin_Production`, 0x23 remove-player → `HouseClass::Flag_To_Win_Check`; ordering keyed on `g_HouseClass_Array` index | `decompile 0x004C7600` §6 dispatch table |
| `techno-foot` | command events route through object vtable `+0x480` for movement/stop dispatch (`EventClass::Execute @ 0x004C6CB0`) | `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_…` §5 |
| `frontier-net-transport` | DoList is populated from peer packets via `Network_ServiceLoop @ 0x0048D080` / `FUN_0048D1E0`; send via `FUN_006475F0`→`FUN_00649CA0` | `NETWORK_FRAME_SCHEDULING_…` §1, §10 |
| `frontier-input-command` | local commands enter via `Process_Command @ 0x0055DEE0` from `GScreenClass::Input`; builder callers `DisplayClass::BandBox_LeftUp`, `SelectClass::Action`, `StripClass::AI` | `get_function_callers 0x004C6AE0`; spine spec §1 |

---

## 8. Incoming edges (used-by)

| ← Service (slug) | Via (symbol) | Evidence |
|---|---|---|
| `frontier-input-command` | clicks/hotkeys produce `EventClass` records into `g_CommandBuffer` (builder `0x004C6AE0`) — I1 sits in front of E1 | `_frontier.md` I1; `get_function_callers 0x004C6AE0` |
| `frontier-net-transport` | E2 delivers peer-serialized events into the DoList and accepts E1's outgoing batches | `_frontier.md` E2; `NETWORK_FRAME_SCHEDULING_…` |
| `logicclass` | `Main_Tick`/`PerTickUpdate` drive the drain (prelude `Map__Logic`) and the in-ladder net barrier (`FUN_006475F0`) | spine spec §1; `NETWORK_FRAME_SCHEDULING_…` §10 |
| `factory-house` | `StripClass::AI` (a production driver) is one of the 3 event-builder callers — AI/production self-issues Place/Begin events through the same queue | `get_function_callers 0x004C6AE0` |

---

## 9. Active-in-YR / TS-legacy

- **Active in YR — YES, universally.** The queue, DoList, `FUN_00647260`/`FUN_0064C380`
  drain+execute, and `EventClass::Execute` dispatch run in **every** mode including local
  skirmish (mode 5) and campaign (mode 0) — single-player just skips the network
  stamp/send/barrier and copies the ring straight into the DoList.
- **Mode-3/4-only (not TS-legacy, just network-gated):** execute-frame stamping
  (`FUN_00649CA0`), MaxAhead/FrameSendRate negotiation (`EventClass::Execute` cases
  0x1B/0x20/0x24), the per-peer lag adaptive throttle, and the `FUN_00648710` barrier. These
  are *unreachable* in skirmish/campaign but are correct, current YR network code — not TS
  ghosts.
- **No TS-legacy dead paths identified in this service's core.** (The transport layer it
  feeds — `frontier-net-transport` E2: IPX / null-modem — is where the TS-legacy verification
  lives; out of scope here.)

---

## 10. Scale flags (30-player / 20k-unit target)

- Local ring cap `0x80` (128) and DoList cap `0x3FFF` (16383) are **gamemd resource limits,
  not protocol rules** — Rust may use deeper queues without behavioral change (FIFO ordering
  within a peer's stream is the contract, not the depth). (`COMMAND_QUEUE_…` Negative Fact 4.)
- `g_HouseClass_Array` is sized for 8 in gamemd; the **ordering contract (house-array-index
  order per execute-frame) must be preserved exactly** at 30 players — Rust must dispatch by
  player *registration order*, not arbitrary `BTreeMap` key. (`COMMAND_QUEUE_…` H2.)

---

## 11. Remaining uncertainty / follow-ups

1. **`Process_QueuedEvents @ 0x0053B560` body undecoded.** Corpus confirms the *call sites*
   (byte-level) but never decompiles the function itself. Confirm on next live session whether
   it drains the full DoList or only a network-message subset. **NEEDS-LIVE-REVERIFY.**
2. **`g_CommandBuffer` base `0x00A802D5` / index `0x00A802CC`** are YELLOW (decomp context,
   no `read_memory` confirmation) — verify symbol binding live.
3. **lobby-slot ↔ house_id mapping** with bot fills / post-REMOVEPLAYER not fully traced
   (`COMMAND_QUEUE_…` §10 #1) — affects the house-array-order dispatch under disconnects.
4. **Ghidra was unreachable this session** — re-run `decompile_function 0x0053B560`,
   `0x0064C380`, `0x004C7600` to upgrade the YELLOW/3-doc-corroborated items to fresh-live
   grade.

---

## 12. Sources

- `docs/research/COMMAND_QUEUE_EVENT_EXECUTE_SCHEDULING_GHIDRA_REPORT.md` (record layout,
  rings, stamping, ordering, dispatch table, single-player path, handoffs) — primary.
- `docs/research/NETWORK_FRAME_SCHEDULING_GHIDRA_REPORT.md` (globals, budget path, barrier,
  tick-order summary).
- `docs/research/LOGICCLASS_PERTICKUPDATE_SPINE_SPEC.md` §1 (command-queue framing correction;
  Map__Logic = live stage; Rung P nesting), §3 (RNG lockstep contract).
- `docs/research/FRAME_COUNTER_NONADVANCE_PAUSE_SCENARIO_MATRIX_GHIDRA_REPORT.md` State C
  (byte-level disassembly `0055d83a: CALL 0x0053b560`).
- `docs/research/RENDER_LOGIC_COUPLING_MAIN_TICK_GHIDRA_REPORT.md` §2 Call Site A;
  `docs/research/MAIN_TICK_RENDER_LOGIC_COUPLING_GHIDRA_REPORT.md` §2 Call Site A.
- `docs/research/NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` §5 (object vtable +0x480
  command routing).
- `docs/research/core-services-map/_frontier.md` §E1 (seed stub), §I1, §E2.
