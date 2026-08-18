# MissionClass + RadioClass as an Engine Substrate Service — Design

**Date:** 2026-06-01
**Type:** Substrate design / migration plan (study only — no Rust written)
**Authority:** binary → Ghidra → docs. Substrate contract verified live this session
(`decompile_function` / `read_memory` on the load-bearing addresses); Rust facts read
from current `src/sim/` this session. Confidence per claim is marked; UNCHECKED items are
quarantined in §9, never mixed into the verified body.
**Rule:** Rust-native structure, gamemd-native semantics. Model the primitive, don't
approximate it; don't port the C++ class tree literally.

> Reading note: this doc treats MissionClass and RadioClass as **two cooperating substrate
> services**, not two C++ classes. gamemd happens to implement them as adjacent layers in
> the `AbstractClass → ObjectClass → MissionClass (0xAC–0xD3) → RadioClass (0xD4–0xF3) →
> TechnoClass` chain. We reproduce their *observable contract* with a Rust-native scheduler
> + bus; we do not reproduce the inheritance tree.

---

> **Corrections (2026-06-01, from live binary verification — evidence in
> `MISSION_RADIO_SUBSTRATE_BINARY_VERIFICATIONS.md`, folded into §A of
> `docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md`):**
> 1. **§3.1 Rescue(21)** — IS live (AI-only, assigned via the ReceiveDamage path, gated `IsPlayerControl()==0`), not "no live trigger". Needs a real handler. Ambush(14) confirmed dead TS stub — keep inert.
> 2. **§3.2 / §5.2.10 `+0x294`** — is an **airstrike back-pointer** (aircraft→summoning AirstrikeClass), NOT a bool radio-deaf latch. Model as `airstrike_owner: Option<EntityId>`; the radio-deaf gate is `airstrike_owner.is_some() && mission ∈ {Retreat,ParaDrop A/O,Spyplane A/O}`.
> 3. **§3.1 QMove** — the `Mission_QMove` handler (`0x00415A50`) is installed at vtable **+0x230 (Retreat)**, NOT the +0x204 (Sleep) slot; there is no AircraftClass Sleep-slot override. QMove(3) routes to the Sleep slot for all classes.
> 4. **§9 dock eviction** — now **proven DRIFT**: gamemd has no stored wait-queue; the next docker is the nearest unit that re-probes. The Rust FIFO queues must be deleted, not just flagged.

---

## 0. One-paragraph thesis

gamemd has **two shared per-object substrate services** that the entire active-techno
population flows through every tick:

1. **MissionClass = the mission scheduler.** A single `CurrentMission` selector +
   `Mission_Dispatch` switch, a frame-anchored self-throttling timer (`return N = defer
   N frames`), and a verb API (`Assign`/`Queue`/`Commence`/`Override`/`Restore`) with a
   built-in 1-deep suspend stack.
2. **RadioClass = the contact RPC bus.** A synchronous (no-queue) message protocol —
   `Transmit_Radio_Impl` calls the target's `Receive_Radio` inline and returns a response
   code — plus a sparse, capacity-bounded `Contacts[]` array with HELLO/BREAK bookkeeping.

The Rust port reproduced the **outcomes** of both, but as **~10 unrelated `Option<T>`
state machines** (mission) and **3 unrelated registries** (radio), each with its own enum,
its own timer convention, and its own tick slot. There is **no `current_mission` field, no
central dispatch, no shared mission timer, and no message protocol** anywhere in `src/sim`.
The miner has even hand-inlined the MissionClass timer fields (`+0xC8/+0xD0`) and narrated
raw radio codes (`0x02/0x0E/0x15/0x16/0x10`) as ad-hoc struct fields. This design defines
the substrate boundary that absorbs that duplication while preserving the verified contract
and the existing `advance_tick` ordering (the lockstep-critical invariant).

---

## 1. Verified active-YR responsibilities

### 1.1 MissionClass (the scheduler)
Active in **every** skirmish, every tick — `TechnoClass::AI_Update @ 0x006F9E50` is the
**sole** caller of `Mission_Dispatch @ 0x005B3060` (verified `get_function_callers`), once
per active techno per logic frame. Responsibilities:

- **Own "what is this object doing right now"** — the `CurrentMission` selector (byte
  `+0xAC`, 30 dispatched mission types).
- **Self-throttle** — call the type's handler only when the per-object dispatch timer is
  due; store the handler's integer return as the next deferral. No global scheduler.
- **Hold mission sub-state** — `MissionState` (`+0xBC`) is the substate index handlers key
  off (e.g. the harvest 5-state machine, the dock Enter retry).
- **Transition vocabulary** — `Assign`/`Queue`/`Commence`/`Override`/`Restore` with two
  hardcoded interrupt guards.
- **Feed INI mission config** — `GetMissionTimerEntry` indexes a 32-entry MissionControl
  table (Rate/AARate/NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/Scatter per mission).

### 1.2 RadioClass (the contact bus)
Active in **every** dock/board/tether/repair/reload handshake. Responsibilities:

- **Synchronous inter-object RPC** — deliver a message to one target's `Receive_Radio` and
  return its response code on the stack (ROGER=1, NEGATORY=10, QUEUED=0x17, …). No mailbox.
- **Maintain the contact set** — HELLO(0x02) inserts (ally-gated, alive-gated, idempotent,
  free-slot), BREAK(0x03) removes; capacity = `max(NumberOfDocks,1)` for buildings, 1
  otherwise; sparse (null holes, no compaction); slot-0 eviction when a sender is full.
- **Broadcast on teardown** — `Broadcast_Radio_ToAll(BREAK)` on limbo/death sends BREAK to
  **every** contact's `Receive_Radio`, running their side-effects (dock-flag clears).
- **Carry dock identity** — a contact's slot index is the basis the building uses to pick
  a `DockingOffsetN` pad (mapping lives in the building consumer, not in RadioClass).

---

## 2. Full inventory (methods, globals, registries, vtable/COM slots, TS paths)

### 2.1 MissionClass — verified addresses & vtable slots
| Symbol | Address | vtable slot | Role |
|---|---|---|---|
| `Mission_Dispatch` (AI) | `0x005B3060` | +0x05C | per-tick dispatch + rate throttle |
| `GetCurrentMission` | `0x005B3040` | +0x184 | return current, else queued |
| `Queue_Mission` | `0x005B35E0` | +0x1E8 | set queued; optional commence-now |
| `Commence` | `0x005B3570` | +0x1EC | promote queued→current, zero timer |
| `Assign_Mission` | `0x005B2FD0` | +0x1F0 | force current, reset timer (bypass queue) |
| `Override_Mission` | `0x005B3650` | +0x1F4 | suspend + switch |
| `Restore_Mission` | `0x005B36B0` | +0x1F8 | pop suspended → current |
| `Is_Mission_Suspended` | `0x005B3A10` | +0x1FC | suspended != -1 |
| `ReadyToCommence` (commence gate) | `0x004E0140` (base) | +0x200 | **base stub `return 1`**; subclass-overridden |
| `GetMissionTimerEntry` | `0x005B3A00` | — | `&MissionControl[current*0x20]` |
| `Read_INI` | `0x005B3760` | — | parse per-mission `[MissionName]` INI block |
| `Mission_From_Name` / `Mission_Name` | `0x005B3910` / `0x005B3950` | — | name↔id |
| `Constructor` | `0x005B2DA0` | — | inits `+0xAC..+0xD0` (current/queued/susp = -1) |
| Mission handler slots | — | +0x204..+0x270 | 32 slots; base = stub `0x005B2E10` `return 0x1C2` |

**Per-object state block (byte offsets, all verified live):**
`+0xAC` CurrentMission (committed) · `+0xB0` SuspendedMission · `+0xB4` QueuedMission
(pending) · `+0xB8` reset/just-started byte · `+0xBC` MissionState (substate index) ·
`+0xC0` mission-start frame · `+0xC4` MissionTickCounter (++ every AI tick) · `+0xC8`
DispatchTimer.Start · `+0xCC` scratch (uninitialized; dead) · `+0xD0` DispatchTimer.Rate
(= handler return).

**Globals / static tables:** `g_MissionNameTable @ 0x00816CAC` (32 `char*`) ·
`g_MissionControl array @ 0x00A8E3A8` (32 × 0x20-byte INI records) · `g_CurrentFrameCounter
@ 0x00A8ED84` (the time base all CDTimers snapshot).

**Singletons/registries:** none beyond the two static tables. Dispatch throttling is
**entirely per-object** (`+0xC8/+0xD0`) — there is no global mission queue or scheduler.

### 2.2 RadioClass — verified addresses & vtable slots
| Symbol | Address | vtable slot | Role |
|---|---|---|---|
| `Receive_Radio` (base tail) | `0x0065A820` | +0x194 | HELLO/BREAK bookkeeping + history shift |
| `Transmit_Radio_Impl` | `0x0065A970` | +0x27C | core send; inline `target->Receive_Radio` |
| `Transmit_Radio` | `0x0065AAA0` | +0x278 | wrapper supplying `g_RadioScratchBuffer` |
| `Transmit_Radio_ToFirst` | `0x0065ACB0` | +0x274 | send to `Contacts[0]` only |
| `Broadcast_Radio_ToAll` | `0x0065ACE0` | +0x280 | send to every non-null contact |
| `FindDockSlot` | `0x0065AD90` | — | linear scan → contact-slot index, else -1 |
| `Set_Contact_Count` | `0x0065AE60` | — | grow-only capacity (sole caller: BuildingClass ctor) |
| `DynamicVectorClass::Contains` | `0x0065AD50` | — | membership bool (used by `Can_Enter_Cell`) |
| `Constructor` | `0x0065A750` | — | capacity=1, 1-slot Contacts, history zeroed |
| `Filter_AbstractType_InMap` | `0x0040DD70` | — | RTTI sender filter (Unit/Aircraft/Building/Infantry) |

**Per-object state block:** `+0xD4/+0xD8/+0xDC` RadioHistory[0..2] (3-deep push-down dedup
log) · `+0xE4` Contacts.data (`TechnoClass**`) · `+0xE8` Contacts.Capacity (signed loop
bound) · `+0xEC` CanGrow · `+0xED` Initialized. Dock-flag bytes used by the protocol but
owned by TechnoClass: `+0x418` dock-entered flag (0x18 sets, 0x19 clears) · `+0x2E4`
reciprocal two-sided link pointer (tank bunker / service-depot install).

**Globals:** `g_RadioScratchBuffer @ 0x00A8EC30` (single static payload scratch — only safe
because gamemd is single-threaded; a concurrent port must use per-call scratch).

### 2.3 Response codes (return values, not messages)
ROGER=`0x01` · NEGATORY=`0x0A` · CELL_ACCEPTED=`0x14` · QUEUED=`0x17` ·
INSUFFICIENT_FUNDS=`0x20` · REPAIR_COMPLETE=`0x21` · silent/no-op=`0`.

---

## 3. Active vs inactive / legacy (TS) split

### 3.1 Missions
- **Dispatched (30):** IDs 0,1,2,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,
  25,26,27,28,30,31 — each a distinct vtable slot in +0x204..+0x270 (verified switch count).
- **`3` QMove** — *no own case*; routes to the +0x204 (Sleep) slot. AircraftClass overrides
  that slot with a real QMove handler (`0x00415A50`). **Preserve this quirk.**
- **`29` (0x1D) Attack Move** — **NO case at all**; falls through, handler never runs, the
  dispatch-timer tail is not rewritten. AttackMove is resolved as a *command/queued mission*
  upstream, never executed as a `CurrentMission`. **Hard parity requirement.**
- **`14` Ambush / `21` Rescue** — base stubs, **no identified live YR trigger**. Mark
  **UNKNOWN — verify reachability before porting a handler** (do not implement speculatively).
- **No subterranean/tunnel mission exists in the enum** — the TS tunnel exclusion does not
  apply to any mission ID here.
- **Interrupt-protected missions:** `0x13` Selling (blocks all Queue/Override) and `0x1C`
  (blocks Guard-5 override). `0x1C` is named **"Wait"** in the enum table and
  **"Deliberate"** in the FOOTCLASS report — same field; the substrate calls it
  `Deliberate` and treats it as the guard-protected mission.

### 3.2 Radio messages
Full table with active/dormant per code:

| Code | Name (★ = binary-string-confirmed) | Active in YR? |
|---|---|---|
| 0x01 | ROGER ★ (response) | Yes (return value) |
| 0x02 | HELLO ★ | **Yes** — every dock/board |
| 0x03 | BREAK / OVER_AND_OUT | **Yes** |
| 0x07 | DOCKING_COMPLETE | Yes — **carryall pickup only** (not refineries) |
| 0x08 | REQUEST_DOCKING_CLEARANCE | Yes |
| 0x0B | DOCK_APPROACH | Yes (inferred name) |
| 0x0C | DOCK_ARRIVED | Live receiver, **not sent** by stock inbound refinery |
| 0x0D | anim-stop / ambient reset | Yes (ObjectClass + Building) |
| 0x0E | CAN_DOCK | **Yes** — every refinery cycle |
| 0x0F | CAN_ENTER | Yes (passenger/garrison/grinder/repair) |
| 0x10 | RESERVE_DOCK | **Dead-send** — live receiver, no live sender (the `PUSH 0x10` sites are `Queue_Mission(0x10)`, not a radio message) |
| 0x11 | IS_UNIT_LINKED (inferred) | Yes (harvester tail) |
| 0x12 | MOVE_TO_CELL | **Yes** |
| 0x13 | NEED_TO_MOVE ★ | **Yes via refinery sub-protocol** (Carryall LAND role dormant) |
| 0x14 | CELL_ACCEPTED (response) | Yes |
| 0x15 | DOCK_NOW | **Yes** — begins unload |
| 0x16 | TIMING_SYNC | **Yes** |
| 0x17 | QUEUED (response) / EVICT | Yes |
| 0x18 | ENTER_DOCK (sets +0x418) | **Yes** |
| 0x19 | LEAVE_DOCK (clears +0x418) | **Yes** |
| 0x1A/0x1B | secondary dock-lock set/clear | Yes |
| 0x1C | REPAIR_TICK | **Yes** (service depot) |
| 0x1D | aircraft helipad-reserve ack | Yes |
| 0x1E | deploy / set-nav | Yes |
| 0x1F | LINK_PASSENGER (cap check) | Yes |
| 0x20/0x21 | INSUFFICIENT_FUNDS / REPAIR_COMPLETE (responses) | Yes |
| 0x22 | IS_REPAIRING (query) | Yes |
| 0x23 | IS_OCCUPIED (query) | Yes |
| 0x24 | RADIO_WANT_RIDE ★ | **Dormant** — only [HIND] Carryall, TechLevel=-1 |

- **Inferred names** (0x07,0x0B,0x0C,0x11,0x16,0x1A,0x1B,0x1D,0x1E) are behavior-guessed,
  **not** string-confirmed — DRIFT-risk if treated as authoritative; only 0x01/0x02/0x13/
  0x24 survive as binary string literals.
- **RadioHistory (+0xD4/D8/DC)** — base only *writes* it; no base reader. Binary-wide scan
  found no external reader, no save/load. **Treat as UNCHECKED-omittable** (a future port
  may drop it) — but not *proven* inert (subclass overrides not exhaustively scanned).

### 3.3 Four distinct active dock idioms (do not conflate)
1. **Refinery (zero-link FSM):** HELLO → CAN_DOCK(0x0E) → {0x13→0x12 accepted-cell} → 0x18
   → 0x16 → unit `PerCellProcess` sends 0x15 → building `Queue_Mission(0x10,0)` →
   `Mission_Deploy_Building` 4-state deposit → state-4 BREAK. Runs with `unit+0x2E4 == 0`
   (no reciprocal link). Capacity 1 (`NumberOfDocks=1`); 2nd HELLO → NEGATORY (no evict).
2. **War factory exit (transient reciprocal contact):** `ExitObject_Main` HELLO(0x02) +
   ENTER_DOCK(0x18) + `Queue_Mission(0x10)`; the contact gates `Can_Enter_Cell`'s
   `NumberImpassableRows` skip so the vehicle can exit through the footprint.
3. **Airfield/helipad (contact-slot = pad-index):** `NumberOfDocks=4` → 4 slots; `GetDockCoord`
   uses `FindDockSlot(contact)` → `DockingOffset[slot]`; `CachedDock` revalidated by sending
   0x0F and requiring reply==1. **No FIFO** in the radio primitive.
4. **Tank bunker / service depot (reciprocal +0x2E4 link):** install writes `building+0x2E4
   = unit` and `unit+0x2E4 = building`; **three** distinct teardown helpers
   (`ReleaseDockedHarvester` normal, `UndockUnit` sell/destroy/temporal, `FUN_00459470`
   super/temporal/damage) each ending in `Transmit_Radio_ToFirst(BREAK)`.

---

## 4. Comparison against the current Rust architecture

### 4.1 Mission side — scattered `Option<T>` machines (no substrate)
| Rust machine | `GameEntity` field | ≈ gamemd mission(s) | Timer convention | advance_tick phase |
|---|---|---|---|---|
| `AircraftMission` | `aircraft_mission` (331) | Move/Attack/Guard/Enter/Unload(+ParaDrop) | **every tick**; inline `reload_timer`/`drop_cooldown` | Ph2 + Ph7 (split) |
| Miner harvest FSM | `miner` (278) | Harvest(10)/Enter(7)/Unload(16) | own `harvest_timer`/`unload_timer`/`rescan_cooldown` + **hand-inlined `+0xC8/+0xD0` mirrors** | Ph7 |
| `DockState` (depot) | `dock_state` (324) | Enter(7) servicing | own `service_timer`/`no_funds_ticks` | Ph7 |
| `DeployPhase` | `deploy_state` (430) | Unload/Construction | `ticks_remaining` in variant | Ph4.6 |
| `BuildingGateRuntime` | `building_gate` (423) | mission `0x18` | **`binary_frame` deltas** (correct model) | Ph1 |
| `OrderIntent` | `order_intent` (282) | AttackMove/Guard/Unload | none — re-derived each tick | Ph5 + Ph6 (split) |
| `NavigationState` | `navigation` (232) | FootClass NavCom (not a mission) | passive | Ph1 |
| teleport/tunnel/droppod/rocket/homing/parachute | 7 `Option<…State>` | locomotor-piggyback | every tick | Ph2 |
| `SlaveHarvester` | `slave_harvester` (280) | slave Harvest/Guard | every tick | Ph7 |
| Retaliation | bare `last_attacker_id` (246) | TechnoClass retaliation | every tick, consume-clear | Ph6 |

**Structural gaps vs gamemd:** (1) no `current_mission` selector — "what is it doing" is the
disjunction of which `Option`s are `Some`; (2) no central dispatch — ≥8 tick entry points
across different phases, ordering is emergent; (3) no shared timer — some subsystems defer
(gate/deploy/dock/miner), some run full logic every tick (order_intent/retaliation/aircraft
attack/NavCom); (4) no Queue/Suspend stack — transitions are **direct field mutation** and
every retasking command hand-enumerates teardown of every other machine (≈9 `order_intent =
None` reset sites). The only queue in the codebase is `NavigationState.nav_queue`
(destinations, not missions).

### 4.2 Radio side — membership-set surrogate (no protocol)
- `radio_contacts: Vec<u64>` + `mark/has/clear` (game_entity.rs 240, 596–610) — **unbounded,
  one-way, dense** membership set. Sole load-bearing reader: `movement_occupancy.rs:326`
  (the `Can_Enter_Cell` passability skip). Writers: war-factory spawn (one-way,
  `production_spawn.rs:213`), miner dock exit-lane (`miner_dock_sequence.rs`), and BREAK-on-
  despawn clears (`world/mod.rs:955`, crush, boarding).
- Two **separate** dock registries reproduce admission/eviction: `RefineryDockContacts`
  (capacity + FIFO `waiting_retry_queue`) and `AirfieldDocks` (pad reservation + FIFO
  promotion). Neither is a `radio_contacts` consumer.
- **No** message enum, **no** synchronous `transmit/receive`, **no** ally/alive HELLO
  guards, **no** capacity on the contact set itself, **no** slot-0 eviction, **no**
  slot-index=pad identity, **no** `RadioHistory`, **no** Radio↔Mission aircraft gate, **no**
  per-contact BREAK side-effects on despawn (only membership clear).

---

## 5. The gamemd-native behavior contract (what the substrate MUST reproduce)

### 5.1 Mission scheduler contract
1. **One dispatch per active object per tick.** Order: `ObjectClass::AI()` first → active
   gate (`+0x90 != 0`) → **due gate** → health gate (`HP > 0`) → handler → commit timer.
2. **Due gate (frame-anchored, passive):** with `start = +0xC8`, `dur = +0xD0`: due iff
   `start == -1` OR `g_CurrentFrame - start >= dur` (**inclusive**). Otherwise skip this tick.
3. **Handler return `N` = ticks to defer.** Commit: `+0xC8 = now`, `+0xD0 = N`. `N=0` ⇒
   dispatched every tick; `N=1` ⇒ next tick; larger ⇒ sleep N. The integer return **is** the
   schedule — there is no separate scheduler.
4. **Tick counter** `+0xC4` increments **every** AI tick (independent of the due gate).
5. **Active-vs-pending split:** dispatcher reads `+0xAC` (committed) directly;
   `GetCurrentMission` returns `+0xAC`, else falls back to `+0xB4` (queued) — so *queries*
   see the pending mission before *execution* switches to it.
6. **Verb semantics (exact):**
   - `Assign_Mission(m)`: force `+0xAC = m`, clear queued/`+0xB8`/substate, **reset timer**
     (`+0xC0/+0xC8 = now`, `+0xD0 = 0`). Bypasses queue. Guarded by Deliberate(0x1C)+5.
   - `Queue_Mission(m, commence)`: write `+0xB4 = m` (iff `m != -1` and not redundant) and
     clear `+0xB8`; if `commence`, call `ReadyToCommence()` (vtable +0x200) then `Commence()`.
     Guards: reject if `current==0x1C && m==5`, or `current==0x13`.
   - `Commence()`: if `+0xB4 != -1`: `+0xAC = +0xB4`, `+0xB4 = -1`, reset substate/`+0xB8`/
     timers, `+0xD0 = 0` (fires next tick), return true.
   - `Override_Mission(m)`: **if `+0xB4 != -1` → `+0xAC = m`, `+0xB0 = +0xB4` (saves the
     QUEUED; the prior current is DISCARDED; `+0xB4` is NOT cleared); else → `+0xB0 =
     +0xAC`, `+0xAC = m`.** Clear `+0xB8`. (This subtlety — discard-current-when-queued —
     is load-bearing.)
   - `Restore_Mission()`: if `+0xB0 != -1`: `+0xAC = +0xB0`, `+0xB0 = -1`, return true.
   - `ReadyToCommence()` (+0x200): **base = `return 1`** (always ready); subclasses override
     to gate promotion on type-specific readiness.
7. **No RNG in the scheduler.** All six primitives draw zero RNG; only individual *handlers*
   do (e.g. Mission_Enter's 14–16 jitter), and the RNG instance is per-handler-callsite ECX.
   Never attribute an RNG draw to the scheduler.
8. **INI-driven mission config:** per-mission `[MissionName]` block → Rate/AARate/NoThreat/
   Zombie/Recruitable/Paralyzed/Retaliate/Scatter; handlers read their own Rate to choose N.

### 5.2 Radio bus contract
1. **Synchronous RPC, no queue.** `transmit(sender, target, msg, payload) → int`: resolve by
   calling `target.receive_radio(filtered_sender, msg, payload)` inline; return its code.
   A full handshake (e.g. 0x08→0x0E→0x13→0x12→0x18→0x16→0x15→0x19→0x03) can run multiple
   round-trips inside one caller's tick.
2. **Sender RTTI filter:** the sender passed to the receiver is filtered to
   Unit/Aircraft/Building/Infantry, else NULL.
3. **HELLO(0x02) bookkeeping (receiver side):** alive gate (`+0x6C != 0`) → ally check
   (double, second gated on `AbstractFlags&1`) → idempotent (already-linked ⇒ ROGER) →
   `Capacity < 1 ⇒ NEGATORY` → free-slot insert ⇒ ROGER, else NEGATORY.
4. **HELLO (sender side, `Transmit_Radio_Impl`):** if already linked ⇒ ROGER without
   re-dispatch; if sender's own array full ⇒ **evict `Contacts[0]`** via `Transmit_Radio(BREAK)`
   (through the vtable, so subclass override fires) then use slot 0; dispatch HELLO; on ROGER
   write the contact.
5. **BREAK(0x03):** receiver nulls the **first** matching slot and returns early (no
   compaction); sender (`Transmit_Radio_Impl`) nulls **all** matching slots before forwarding.
6. **Capacity:** default 1 (non-building); buildings = `max(NumberOfDocks,1)` via grow-only
   `Set_Contact_Count`. Sparse array; null holes; next HELLO fills first null. **At most one**
   contact for a non-building.
7. **Null-target default = `Contacts[0]` only** (not first-non-null). `Transmit_Radio_ToFirst`
   targets `Contacts[0]` only.
8. **Broadcast on limbo/death:** `Broadcast_Radio_ToAll(BREAK)` sends BREAK to **every**
   non-null contact's `receive_radio`, running their teardown side-effects (e.g. 0x19 →
   clear `+0x418`; reciprocal `+0x2E4` clears). Membership-clear alone is insufficient.
9. **Dock-flag state machine:** 0x18 sets `+0x418`, 0x19 clears it. The BREAK→0x19 cascade
   fires only when **both** receiver and sender `+0x418 != 0`.
10. **Radio↔Mission coupling:** an AircraftClass whose `CurrentMission ∈ {Retreat(4),
    ParadropApproach(0x1A), ParadropOverfly(0x1B), SpyplaneApproach(0x1E), SpyplaneOverfly
    (0x1F)}` AND whose latch (`+0x294`) is 0 **drops every radio message** (returns 0) before
    dispatch. The substrate must let mission state make an object radio-deaf.
11. **Dock-pad identity:** `FindDockSlot(contact)` returns the contact-slot **index**; the
    building consumer maps that index to `DockingOffsetN`. The substrate exposes the slot
    index; pad geometry stays in the building/dock consumer.

---

## 6. Rust-native replacement boundary

**Design stance:** introduce two **substrate services inside `sim/`** that own the
*mechanism* (state representation, scheduling primitive, verb API, message dispatch). They do
**not** rewrite `advance_tick` into one monolithic loop — that would reorder the
lockstep-critical phase sequence (see §9 Risk 1). Subsystems keep their tick slots but
**delegate** their mission-state, timers, transitions, and contacts to the substrate.
`EntityStore` keeps owning storage; the substrate is plain Rust functions + two new
components, committing state in native order. This is "model the primitive" without "port
the class tree."

### 6.1 `sim/mission/` — the mission scheduler service
```
sim/mission/
  mod.rs          // MissionType, MissionCom component, the verb API
  dispatch.rs     // the per-object due-gate + substate router (called from tick slots)
  timer.rs        // MissionTimer (frame-anchored CDTimer)
  control.rs      // MissionControl table (INI Rate/AARate/flags), parsed from rules
```
- **`MissionType`** enum — the 30 active variants + `None`. Canonical mission vocabulary;
  replaces the per-subsystem ad-hoc enums as the *selector* (subsystems may keep richer
  per-mission substate, see below).
- **`MissionCom`** component on `GameEntity` (sibling to `NavigationState`):
  `current: MissionType`, `queued: Option<MissionType>`, `suspended: Option<MissionType>`,
  `substate: u8`, `timer: MissionTimer`, `tick_counter: u32`. Mirrors `+0xAC/+0xB4/+0xB0/
  +0xBC/(+0xC8,+0xD0)/+0xC4`. Hashed for lockstep once authoritative.
- **`MissionTimer { start_frame: u32, duration: u32 }`** — the single frame-anchored
  deferral primitive (the gate runtime's `binary_frame`-delta model, generalized). `due(now)
  = start == SENTINEL || now.wrapping_sub(start) >= duration`. `defer(now, n)` sets
  `start=now, duration=n`. **This replaces every bespoke countdown field** (harvest/unload/
  service/reload/drop/rescan/gate-transition/hold/deploy timers).
- **Verb API** (free fns taking `&mut MissionCom`, exact §5.1.6 semantics, incl. the two
  guards and the Override discard-current-when-queued subtlety):
  `assign_mission`, `queue_mission`, `commence`, `override_mission`, `restore_mission`,
  `get_current_mission`. `assign_mission` is the **one** place that tears down the prior
  mission's owned state — collapsing the ≈9 scattered reset sites.
- **`MissionControl`** — `BTreeMap<MissionType, MissionControlEntry>` parsed from the
  `[MissionName]` INI sections (Rate/AARate/NoThreat/Zombie/Recruitable/Paralyzed/Retaliate/
  Scatter). Handlers read their Rate to pick the deferral N. **No hardcoded mission timing.**
- **Dispatch helper** `dispatch_due(com, now) -> bool` — runs the due gate + increments the
  tick counter; subsystems call it from their existing tick slot and only run their handler
  when it returns true, then call `com.timer.defer(now, handler_return_n)`. This keeps the
  `advance_tick` phase order intact while sharing the throttle.

### 6.2 `sim/radio/` — the contact RPC bus service
```
sim/radio/
  mod.rs          // RadioMessage, RadioResponse, Contacts component, transmit()
  receive.rs      // per-category receive_radio handlers (building/unit/aircraft/infantry)
  contacts.rs     // sparse capacity-bounded Contacts array + HELLO/BREAK bookkeeping
```
- **`RadioMessage`** enum — only the active-YR codes from §3.2 (HELLO, Break, RequestClearance,
  CanDock, CanEnter, NeedToMove, MoveToCell, CellAccepted, DockNow, TimingSync, EnterDock,
  LeaveDock, RepairTick, …). Dormant codes (0x24 WantRide, 0x13-Carryall role) omitted with
  a comment; inferred-name codes carry a `// name inferred` marker.
- **`RadioResponse`** enum: `Roger`, `Negatory`, `Queued`, `CellAccepted`, `InsufficientFunds`,
  `RepairComplete`, `None`.
- **`Contacts`** component — **sparse, capacity-bounded** `slots: Vec<Option<u64>>`, where
  `capacity = max(NumberOfDocks, 1)` for buildings, else 1. Replaces `radio_contacts: Vec<u64>`
  with the correct slot model (null holes, no compaction, slot-0 eviction). `find_slot(id)` →
  the dock-pad index basis; `contains(id)` for the `Can_Enter_Cell` membership test.
- **`transmit(world, sender, target, msg, payload) -> RadioResponse`** — synchronous:
  centralizes HELLO/BREAK bookkeeping (ally + alive + idempotent + free-slot + slot-0 evict),
  applies the RTTI sender filter, then dispatches to `receive_radio`. `transmit_to_first`
  and `broadcast(msg)` for the `Contacts[0]`-only and limbo-BREAK cases — the latter runs
  **each** contact's `receive_radio(Break)` so teardown side-effects fire.
- **`receive_radio(world, this, sender, msg, payload) -> RadioResponse`** — per-category
  handlers (plain fns dispatched by `EntityCategory`), committing in native order. The
  AircraftClass radio-deaf gate reads `MissionCom.current` + the latch.
- The **four dock idioms** (§3.3) are implemented as **distinct** flows over this bus — not
  collapsed: zero-link refinery FSM, transient reciprocal war-factory contact, contact-slot=
  pad airfield, reciprocal `+0x2E4` bunker/depot with three teardown helpers. A
  `reciprocal_link: Option<u64>` field models `+0x2E4`; a `dock_entered: bool` models `+0x418`.

### 6.3 Boundary invariants
- Both services live **entirely in `sim/`** — they never reach `render/`, `ui/`, `sidebar/`,
  `audio/`, `net/` (the #1 invariant). Sound cues from radio/mission stay routed through the
  existing `sim.sound_events` queue, as today.
- All scheduling math is fixed-frame integer (`u32` frame counter), never float.
- Determinism: `MissionCom` and `Contacts` (sparse, with stable slot indices) are folded into
  the state hash; iteration stays BTreeMap-ordered.

---

## 7. Old ad-hoc Rust logic to retire (after the substrate lands)

| # | Retire | Replace with | Evidence |
|---|---|---|---|
| 1 | Manual multi-field teardown in every retasking command (≈9 `order_intent = None` + `cancel_*dock` sites) | one `assign_mission()` that clears the prior mission's owned state | world_commands.rs 144–155, 294, 332, 356, 377, 779, 873, 1020, 1116 |
| 2 | Scattered "is busy/idle" predicates (each subsystem invents its own over a different field subset) | a single `get_current_mission()` read | combat_targeting.rs:346, world_orders.rs:51, 95 |
| 3 | `order_intent` persistence as a resume side-channel (re-checked pre- AND post-combat) | the `suspended` mission stack (`override`/`restore`) | components.rs 488–495; world_orders.rs 80–113 |
| 4 | Bare `last_attacker_id` retaliation coordinated-by-convention against `order_intent` | retaliation as a mission transition arbitrated by dispatch priority | game_entity.rs:246; combat_targeting.rs 346, 394 |
| 5 | Split-phase subsystems (OrderIntent Ph5+Ph6; aircraft/airfield dock Ph2+Ph7) | one mission slot owning each; consolidate once dispatch fixes ordering | world/mod.rs phase wiring |
| 6 | Per-subsystem bespoke timers (harvest/unload/service/no_funds/reload/drop/rescan/transition/hold/deploy) | the shared frame-anchored `MissionTimer` | miner/mod.rs 265–306; building_dock.rs 46–49; deploy.rs; gate_runtime.rs 73–97 |
| 7 | The miner's hand-inlined `+0xC8/+0xD0` mirror fields (`dock_enter_retry_*`, `mission_deploy_*`) | the real `MissionTimer` on `MissionCom` | miner/mod.rs 296–306 |
| 8 | `radio_contacts: Vec<u64>` unbounded one-way membership set | sparse capacity-bounded `Contacts` (correct slot/eviction model) | game_entity.rs 240, 596–610 |
| 9 | Two parallel dock registries (`RefineryDockContacts`, `AirfieldDocks`) reproducing admission/eviction ad-hoc | the unified `transmit()` bus + per-idiom flows (keep the 4 idioms distinct) | miner_dock.rs 42–78; aircraft_dock.rs 134–193 |

**Do NOT retire:** `NavigationState` (it is NavCom/destinations, a *separate* primitive from
mission) — keep it as its own component; the substrate references it, doesn't absorb it.

---

## 8. Migration slices + acceptance tests

Sequenced so each slice is independently shippable, preserves the `advance_tick` phase order,
and is gated by a player-observable parity test. Pattern mirrors the existing `Presence`
shadow-slice precedent (add shadow → assert agreement → make authoritative).

- **Slice 0 — Substrate scaffolding (no behavior).** Add `MissionType`, `MissionControl` INI
  parse, `RadioMessage`/`RadioResponse` enums. No consumer.
  *Accept:* all `[MissionName]` sections parse; enum name↔id round-trips; **state hash
  unchanged** over a recorded skirmish.

- **Slice 1 — `MissionTimer` primitive, lowest-risk adopter.** Introduce `MissionTimer` and
  migrate the **gate runtime** (already `binary_frame`-delta) onto it.
  *Accept:* gate open/close/hold cadence **bit-identical**; hash unchanged across a gate cycle.

- **Slice 2 — `MissionCom` in shadow mode.** Add `MissionCom` to `GameEntity`; populate
  `current`/`queued` from existing state each tick; a debug assert proves it agrees with the
  mission derived from the `Option<T>` fields. Nothing reads it yet; not hashed.
  *Accept:* shadow assert never fires over a full replay; hash unchanged.

- **Slice 3 — `Contacts` slot model.** Replace `radio_contacts: Vec<u64>` with sparse
  capacity-bounded `Contacts` (capacity from `NumberOfDocks`); keep the `Can_Enter_Cell`
  membership consumer.
  *Accept:* war-factory-exit and refinery-exit passability **bit-identical**; contacts hash
  deterministic; existing miner-dock test suite green.

- **Slice 4 — RadioBus for the refinery idiom.** Implement synchronous `transmit` +
  `receive_radio` for the refinery dock choreography (HELLO→CAN_DOCK→0x12/0x13→0x18→0x16→
  0x15→`Queue_Mission(0x10)`→deposit→BREAK), replacing `RefineryDockContacts`.
  *Accept:* deposit cadence (14.4-tick whole-slot drain), pivot, and state-4 BREAK
  **bit-identical**; full miner-dock test suite green; credits-per-cycle unchanged.

- **Slice 5 — Migrate bespoke timers.** Move miner/dock/deploy/aircraft countdowns onto
  `MissionTimer`; delete the duplicated fields (retire-list #6/#7).
  *Accept:* each subsystem's cadence **bit-identical**; reload/unload/deploy durations
  unchanged to the tick.

- **Slice 6 — Verb API + dispatch adoption.** Replace the manual retasking teardown and the
  scattered "is busy" predicates with `assign_mission`/`get_current_mission` (retire-list
  #1/#2). Fold retaliation + order_intent resume into `override`/`restore` (#3/#4).
  *Accept:* retasking (Move/AttackMove/Stop while docking/attacking/guarding) clears state
  identically; guard→attack→resume and retaliation behave identically; hash unchanged.

- **Slice 7 — Per-idiom radio adoption.** One slice each: airfield (contact-slot=pad,
  `CachedDock` 0x0F revalidation, no FIFO), tank bunker (+0x2E4 reciprocal link + 3 teardown
  helpers), service depot (0x1C REPAIR_TICK loop), war-factory exit.
  *Accept (per idiom):* pad assignment / bunker install+release / repair-tick money+HP /
  factory-exit passability **bit-identical**; broadcast-BREAK on despawn clears the
  reciprocal flags (no dangling links).

- **Slice 8 — Make `MissionCom` authoritative.** Once all subsystems read `MissionCom`, drop
  the shadow asserts and the now-redundant `Option<T>` mission *selector* fields (subsystems
  keep only their genuinely-richer substate); hash `MissionCom`.
  *Accept:* full replay determinism; no mission-selector duplication remains; cross-machine
  lockstep hash stable.

**Cross-slice acceptance harness:** a recorded multi-faction skirmish replay whose
end-of-match state hash must remain stable through Slices 0–3, then change only in
documented, intended ways at each behavior-bearing slice (4–8), re-baselined per slice.

---

## 9. Confidence, open questions & risks (honest ledger)

### 9.1 Verified-from-binary this session (HIGH)
Dispatch mechanism + return-value contract (0x005B3060); the verb set + offsets
(0x005B2DA0/2FD0/3040/3570/35E0/3650/36B0); `ReadyToCommence` base = `return 1`
(0x004E0140); the 4 radio primitives + `Broadcast_Radio_ToAll` (0x0065A750/A820/A970/AAA0/
ACB0/ACE0); contact layout `+0xE4/+0xE8`; `FindDockSlot` returns slot-index (no pad map);
the AircraftClass mission-state radio gate (0x004190B0); 30 dispatched missions; ≥12
distinct handled radio codes. Current Rust facts (no `current_mission`; `radio_contacts`
consumer at movement_occupancy.rs:326; scattered timers) read this session.

### 9.2 UNCHECKED / resolve before the slice that depends on it
- **Per-subclass `ReadyToCommence` overrides** — base is `return 1`; the actual promotion
  gates (e.g. unit-at-deploy-cell) are per-type and **not decompiled**. Resolve in Slice 6.
- **AircraftClass `+0x294` latch *setter*** — the radio-deaf gate *read* is verified; *when
  the latch clears* is UNCHECKED (candidate: ParaDrop/SpyPlane handlers). Resolve in Slice 7
  (aircraft).
- **RadioHistory has no subclass reader** — claimed but not exhaustively proven; treat as
  UNCHECKED-omittable, not proven-inert.
- **FIFO eviction order vs gamemd slot-0-evict** — the Rust refinery/airfield FIFO wait
  queues have **no verified gamemd counterpart**; "who docks next" under saturation is
  unproven parity. Trace gamemd's slot-0-evict-then-receiver-rejects order before asserting.
- **Ambush(14) / Rescue(21) YR-activity** — UNKNOWN; scope out or verify reachability before
  porting a handler.
- **Inferred radio names** (0x07/0x0B/0x0C/0x11/0x16/0x1A/0x1B/0x1D/0x1E) — behavior-guessed.
- **Missions/idioms without an identified Rust home** — Capture(8), Sabotage(17), Eaten(9),
  Patrol(25), Missile(22), Selling(19), building Repair(20), ground Hunt(15)/AreaGuard(11),
  and the **bunker `+0x2E4` reciprocal link** + **service-depot REPAIR_TICK** loop. Name a
  home (or explicit NONE) per the relevant slice.
- **Rust line anchors in §4/§7** — read this session, but parallel sessions cause line
  drift; re-grep the storage fields and reset sites at the start of each implementation slice.

### 9.3 Top risks for the redesign
1. **Dispatch ordering / determinism (highest).** Unifying ≥8 per-phase tick entry points
   into one loop would reorder inter-subsystem execution and break lockstep replays. **The
   substrate must preserve the documented `advance_tick` phase order** — adopt the shared
   timer/state/verb API *within* existing slots; do not collapse phases in one step.
2. **Timer cadence model.** gamemd is `g_CurrentFrame`-anchored; Rust has BOTH frame-delta
   (gate, correct) and per-sim-tick decrement (miner/deploy/dock/aircraft). Standardize on
   the frame-anchored `MissionTimer`; migrating decrement timers will shift cadence by any
   sim-tick≠binary-frame drift — verify each per Slice 5.
3. **Radio↔Mission gate coupling.** Splitting mission state from radio state risks losing the
   aircraft radio-deaf latch; the `+0x294` setter is UNCHECKED. Keep `MissionCom` readable
   from `receive_radio` and resolve the setter before Slice 7 (aircraft).
4. **Eviction order beyond proof.** Slot-0 eviction vs FIFO is observable on saturated
   refineries/airfields (busy economy). Either trace gamemd's order or document a known DRIFT.
5. **Scale vs parity tension (the scale exception).** The 20k-unit/30-player target requires
   replacing gamemd's 8-player ally bitmask *structure* (not behavior) inside the HELLO ally
   double-check, and an unbounded contact model — but unbounded contacts **break** the slot-0-
   eviction/NEGATORY-when-full behaviors. Reconcile explicitly: keep the **sparse
   capacity-bounded** `Contacts` for parity-correct eviction, scale the **ally-check
   structure** independently.

---

## 10. Sources

**Live Ghidra this session:** `decompile_function` 0x004E0140, 0x0065ACE0; (corpus also
verified 0x005B3060/3040/35E0/3570/3650/36B0/2DA0/3A00, 0x0065A750/A820/A970/AAA0/ACB0/AD90/
AE60, 0x004190B0, 0x006F9E50) + `read_memory` raw-byte offset confirmations.
**Research docs:** `MISSIONCLASS_STATE_MACHINE.md`, `RADIOCLASS_CORE_PRIMITIVES_VERIFIED_
GHIDRA_REPORT.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, the miner radio-link/dock state-
machine reports, `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`,
`NUMBER_IMPASSABLE_ROWS_RADIO_CONTACT_VECTOR_GHIDRA_REPORT.md`, `WAR_FACTORY_EXIT_CONTACT_ROW_
SKIP_GHIDRA_REPORT.md`, `BUILDINGCLASS_MISSILE_AND_RADIO_GHIDRA_REPORT.md`.
**Rust:** `src/sim/game_entity.rs`, `src/sim/aircraft/mod.rs`, `src/sim/miner/mod.rs`,
`src/sim/movement/movement_occupancy.rs`, `src/sim/production/production_spawn.rs`,
`src/sim/docking/`, and the `advance_tick` phase wiring in `src/sim/world/mod.rs`.
