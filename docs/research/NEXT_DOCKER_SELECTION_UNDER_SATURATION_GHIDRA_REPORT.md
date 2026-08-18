# Next Docker Selection Under Saturation — Ghidra Research Report

**Addresses:** `0x0043C2D0`, `0x004D9290`, `0x005B3060`, `0x0041BBD0`, `0x0065A820`,
`0x0065A970`, `0x0065ADF0`, `0x0041BC17`, `0x0041B660`, `0x00419C80`, `0x0055AFB0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** "Who docks next" under saturation for (a) one-dock refinery (CMIN/HARV →
GAREFN/NAREFN) and (b) multi-pad airfield/helipad (ORCA/BEAG → GAAIRC/AMRADR). CAN_DOCK
(`0x0E`) retry cadence, building-side admission decision, and whether any ordered wait
structure exists in the binary.
**Non-Scope:** full unload/deposit/exit state machine, airfield reload cadence, `Force_Track`,
production queues.
**Confidence:** High for both (a) and (b) — no queue structure found; selection is emergent.
**Active in YR:** Yes for all stock CMIN/HARV and ORCA/BEAG paths.

---

## Investigation Contract

### Target Question

Does gamemd store any ordered wait-list (FIFO or priority) on either the building side or the
unit side to determine which waiting unit docks next? Or is selection purely emergent from
independent per-unit mission-timer re-probes arriving at a building that holds open RadioClass
contact slots?

### Non-goals

- Full unload state machine (documented in sibling reports).
- Airfield reload cadence (`ReloadRate`, radio messages `0x1D/0x1F`).
- `Force_Track`, `ReleaseDockedHarvester`, or non-stock harvester types.

### Evidence Needed to Mark COMPLETE

1. Proof that building side stores no queue at case `0x0E`.
2. Proof that aircraft side stores no queue at `FindBuildingToDock` or `AircraftClass::Mission_Enter`.
3. Proof of per-unit mission-timer re-probe as the only retry mechanism.
4. Confirmation that `0x0F` revalidation for aircraft is a live/capacity check, not queue position.
5. Rust handoff: what to replace the FIFO with.

### Stop Conditions

Achieved when all five evidence requirements are met and the implementation handoff is written.
No mutating Ghidra calls were made.

---

## Executive Summary

gamemd has **no stored wait queue** for dock admission — for either refineries or airfields.
Admission is purely emergent: each waiting unit independently re-probes via its own
`Mission_Enter` dispatch (gated by `MissionClass+0xC8/+0xD0` timer), and whichever unit's
probe arrives first at a building with a free RadioClass contact slot wins. For aircraft, the
same applies via `AircraftClass::Mission_Enter`, which calls `CAN_DOCK (0x0E)` through
`FindBuildingToDock`. The building side holds only RadioClass contact slots — an unordered
sparse array, not a FIFO — and the refinery/airfield never calls back to notify waiting units
that a slot freed. Selection is therefore **distance-timing-emergent**, not distance-sorted:
the winner is whichever unit's per-unit mission timer fires next after a slot opens.

The `DockReservations` and `RefineryDockContacts::waiting_retry_queue` / `AirfieldDocks::queues`
fields in the Rust port are **DRIFT** from this mechanism. They impose FIFO ordering that does
not exist in the binary.

---

## Verified Binary Findings

### Finding 1 — Refinery CAN_DOCK (0x0E): No stored queue on the building side

**Active in YR:** Yes, for DockUnload/Refinery buildings.

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E`:

```text
// Call TechnoClass base
TechnoClass__Receive_Radio(param_2, param_3, param_4)
// Guard: power check
if param_1->HasPower == false: return 10

// Non-DockUnload, non-Weeder path (factories, helipads):
//   calls FUN_0065adf0 (FindFreeContactSlot) — accepts if free, else
//   iterates param_1->field_0xe8 (contact count) and pings existing contacts
//   with 0x22; if they answer 10 sends 0x17 to them, then re-checks for
//   free slot; returns 10 if still full.

// DockUnload/Refinery path (stock refinery — the path relevant here):
cVar2 = DynamicVectorClass__Contains();   // is sender already in contacts?
if cVar2 == '\\0':
    cVar2 = FUN_0065adf0();               // FindFreeContactSlot
    if cVar2 != '\\0':
        transmit HELLO (0x02) to sender   // link contact
// then:
cVar2 = DynamicVectorClass__Contains();   // is sender in contacts now?
if cVar2 != '\\0' && (DockUnload || Weeder):
    compute payload cell (NW+3,NW+1)
    send 0x12 to sender
    if 0x12 reply != 0x14: return 1
    send 0x18 to sender
    send 0x16 to sender
    return 1
// If not in contacts (full): return 1 (no explicit queue write)
```

The building side **never writes to any queue**. A full refinery (contacts saturated at
`NumberOfDocks=1`) simply does not emit `0x12`/`0x18`/`0x16` to the new probe, and returns `1`
without storing the probe anywhere. Evidence: `verified via decompile_function 0x0043C2D0`.

### Finding 2 — FootClass::Mission_Enter: per-unit timer is the sole retry gate

**Active in YR:** Yes for stock CMIN/HARV entering GAREFN/NAREFN.

`FootClass::Mission_Enter @ 0x004D9290` sends `CAN_DOCK (0x0E)` on every dispatch. The only
gate before dispatch is `MissionClass::Mission_Dispatch @ 0x005B3060` checking
`elapsed = g_CurrentFrameCounter - MissionClass+0xC8 >= MissionClass+0xD0`. The timer value
returned by Mission_Enter for `[Enter]` is `ftol(0.016 * 900.0) + RandomRanged(0,2)` = 14..16
frames. Evidence: `verified via decompile_function 0x004D9290`; `verified via decompile_function
0x005B3060`; sibling doc `WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK_GHIDRA_REPORT.md`.

No unit-side queue is consulted. The unit simply fires its timer, sends `0x0E`, and accepts or
re-arms the timer. If `0x0E` reply is not `1` and `+0x418 == 0`, it sends `BREAK(3)` and
re-queues mission `0`.

### Finding 3 — RadioClass: contact slots are unordered; no FIFO

**Active in YR:** Yes.

`RadioClass::Receive_Radio @ 0x0065A820` accepts HELLO into any null slot in `Contacts[0..E8]`.
`FUN_0065ADF0` (FindFreeContactSlot) scans linearly for the first null or already-present slot.
There is no ordering metadata or insertion-time stamp. Evidence: `verified via sibling doc
BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md` (functions
`0x0065A820`, `0x0065ADF0` decompiled); confirmed unreachable queue path for stock refinery.

### Finding 4 — AircraftClass: CachedDock is a revalidated pointer, not a queue position

**Active in YR:** Yes for AirportBound stock aircraft.

`AircraftClass::FindBuildingToDock @ 0x0041BBD0`:

```c
if (Type->AirportBound && this->CachedDock != NULL) {
    if (this->Transmit_Radio(0x0F, this->CachedDock) == 1) {
        return this->CachedDock;   // reused
    }
    this->CachedDock = NULL;       // cleared on rejection
}
this->CachedDock = FootClass::Find_Docking_Bay(...);  // new search
return this->CachedDock;
```

`CachedDock` at `AircraftClass+0x6CC` is only a nullable pointer to the last-known dock
building. Revalidation sends radio `0x0F` (not `0x0E`) to check liveness and capacity. There is
no queue slot or position number in `CachedDock`. Evidence: `verified via decompile_function
0x0041BBD0`. `FootClass::Find_Docking_Bay` is the only caller (`verified via
get_function_callers FootClass__Find_Docking_Bay`).

### Finding 5 — BuildingClass::Receive_Radio 0x0F: liveness/capacity check, not queue

**Active in YR:** Yes for helipad buildings.

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0F` gates on:
- ally check
- building mission state != `0x12` and `!= 0x13`
- `building+0x534 != 0` (not-powered gate)
- `!MapEditorMode` && `FUN_0065ADF0` (free contact slot check) && no exclusionary type flags

For `Helipad=yes` (`+0x16CB`): returns `1` if aircraft `->something == 2` (landed/ready),
else returns `10`. No queue position is recorded. Evidence: `verified via decompile_function
0x0043C2D0` case `0x0F`.

### Finding 6 — AircraftClass::Mission_Enter: CAN_DOCK retry is also timer-gated, no queue

**Active in YR:** Yes for AirportBound aircraft.

`AircraftClass::Mission_Enter @ 0x00419C80` calls `CAN_DOCK (0x0E)` via `vtable+0x274` when
`param_1[0x169] == 0` (no current destination) and the CAN_DOCK answer is not `1`. Like
`FootClass::Mission_Enter`, this is dispatched by `MissionClass::Mission_Dispatch` and is
therefore timer-gated. No queue structure is consulted. Evidence: `verified via
decompile_function 0x00419C80`.

### Finding 7 — Release does not notify waiters (refinery and airfield)

**Active in YR:** Yes.

`UnitClass::Mission_Deploy_Building @ 0x0073D630` state 4 (A releases refinery contact) calls
`RadioClass::Transmit_Radio_Impl @ 0x0065A970` with `BREAK(3)`, which removes the sender from
the receiver's `Contacts[]`. It does not iterate waiting units or send them any message. For
airfields, `AircraftClass::Detach @ 0x0041B660` clears `CachedDock` via pointer expiry but
also does not notify waiters. Evidence: sibling doc `TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`
(function `0x0073D630`); sibling doc `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`
(function `0x0041B660`).

---

## INI Keys

| Key | Value | Effect | Active in YR |
|-----|-------|--------|--------------|
| `[GAREFN/NAREFN] NumberOfDocks` | `1` | Contact capacity = 1; one miner at a time | Yes |
| `[GAAIRC/AMRADR] NumberOfDocks` | `4` | Four contact slots; up to 4 aircraft at once | Yes |
| `[Enter] Rate` | `.016` | Mission Enter timer base: 14..16 frames | Yes |

---

## Integration Points

| Function | Role | Evidence | Active in YR |
|----------|------|----------|--------------|
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | case `0x0E`: admits to free contact slot, no queue write | decompile | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | case `0x0F`: liveness/capacity gate for aircraft | decompile | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | per-unit CAN_DOCK probe; returns 14..16 frame timer | decompile | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | timer gate for Mission_Enter | decompile | Yes |
| `AircraftClass::FindBuildingToDock @ 0x0041BBD0` | revalidates CachedDock with 0x0F; no queue | decompile | Yes |
| `AircraftClass::Mission_Enter @ 0x00419C80` | aircraft's CAN_DOCK retry; timer-gated | decompile | Yes |
| `RadioClass::FUN_0065ADF0` (FindFreeContactSlot) | linear scan for null slot; no ordering | sibling docs | Yes |
| `LogicClass @ 0x0055AFB0` | per-tick live-object iteration; determines who probes first | sibling doc | Yes |

---

## Implementation Handoff

### 1 — Refinery: Delete `waiting_retry_queue`; replace with per-miner mission timer

**Verified behavior:** A full refinery returns from case `0x0E` without storing the probe.
Each waiting miner's `FootClass::Mission_Enter` re-probes independently every 14..16 frames.
The first miner whose timer fires after the slot frees wins — determined by live-object
iteration order and individual timer state.

**Rust delta:** `RefineryDockContacts::waiting_retry_queue: BTreeMap<u64, VecDeque<u64>>`
imposes FIFO order that does not exist. `hello_or_wait` enforces FIFO by requiring the front
of the queue to match before granting admission.

**Affected surfaces:**
- `src/sim/miner/miner_dock.rs` — `RefineryDockContacts`, `hello_or_wait`, `release_contact`
- `src/sim/miner/miner_dock_sequence.rs` — phase that calls `hello_or_wait`
- `src/sim/world/world_hash.rs` — hashes `waiting_retry_queue`
- `DockReservations` (same file) — also has FIFO queue/promote; used by older code path

**Required delta:** Remove `waiting_retry_queue` from `RefineryDockContacts` and the FIFO
gate in `hello_or_wait`. A miner denied access simply returns `Waiting`; the next retry
call from whichever miner's mission timer fires next wins the slot. Add per-miner mission
retry timer (`u16` countdown, initialized to 14..16 frames via RNG on refused `0x0E`) to
the miner's dock state, not to a shared building-keyed queue.

**Acceptance scenario:** Two miners A (id=1) and B (id=2) at the same refinery. B's
first probe is refused. A finishes and frees the slot. B must probe again before being
admitted — but so must any miner C (id=3) that arrives after A frees. If C's timer fires
before B's, C wins, not B. Test: `refinery_contention_winner_is_first_probe_not_fifo`.

**Risk:** `world_hash.rs` hashes `waiting_retry_queue`. Removing the field is a breaking
hash change for any in-flight save; this is acceptable since the FIFO is confirmed drift.

### 2 — Airfield: Delete `AirfieldDocks::queues` and automatic `release`-time promotion

**Verified behavior:** `CachedDock` is a revalidated pointer, not a queue reservation.
`BuildingClass::Receive_Radio` case `0x0F` admits based on liveness and contact capacity,
not queue position. `AircraftClass::Mission_Enter` re-fires its own CAN_DOCK when it needs
a dock (timer-gated). No `release`-time promotion was found.

**Rust delta:** `AirfieldDocks::queues: BTreeMap<u64, VecDeque<u64>>` and the
`release`-promotes-next-from-queue logic in `AirfieldDocks::release` and `cancel` are
not binary-backed.

**Affected surfaces:**
- `src/sim/docking/aircraft_dock.rs` — `AirfieldDocks`, `try_reserve`, `release`, `cancel`,
  `cleanup_dead`
- `tick_aircraft_docks` — `WaitForDock` phase that calls `try_reserve` (correctly probes;
  keep this, just remove queue semantics from `try_reserve`)

**Required delta:** Remove `queues` from `AirfieldDocks`. `try_reserve` returns `None` when
all slots are full; the aircraft stays in `WaitForDock` and re-probes next tick (no timer
needed — the `tick_aircraft_docks` already handles this via the phase check). Keep `slots`
as the contact-slot equivalent. The pad-slot identity rule (`NumberOfDocks` → contact-capacity;
slot index → `DockingOffsetN`) is already binary-backed and must be kept.

**Acceptance scenario:** Four aircraft at a 4-pad GAAIRC. Fifth aircraft waits. When pad 2
frees, all four waiting aircraft compete; whichever probe fires first in that tick wins pad 2
(first-free-scan on `try_reserve`). The FIFO cannot guarantee aircraft 5 wins over aircraft
6 that arrived later. Test: `airfield_full_waiter_admitted_by_probe_not_fifo`.

### 3 — No takeover callback needed on release

**Verified behavior:** A releases refinery contact via `BREAK(3)` → `RadioClass::Receive_Radio`
clears the contact. Zero notification to B. Similarly, airfield `Detach` only clears
`CachedDock` on the aircraft side. No building-side "this slot is now free, alert waiters"
logic was found. Evidence: sibling docs for both.

**Required delta:** Rust `release` must NOT automatically promote a FIFO waiter. The slot
simply becomes empty; the next probe wins it.

---

## Negative Facts / Do Not Do

1. **Do not keep `RefineryDockContacts::waiting_retry_queue`.** The refinery building stores
   no queue. Verified: case `0x0E` of `BuildingClass::Receive_Radio @ 0x0043C2D0` — no queue
   write on admission failure. Evidence: `verified via decompile_function 0x0043C2D0`.

2. **Do not keep `AirfieldDocks::queues` or automatic `release`-time FIFO promotion.**
   `BuildingClass::Receive_Radio` case `0x0F` and `AircraftClass::FindBuildingToDock @
   0x0041BBD0` contain no queue; verified zero FIFO path. Evidence: `verified via
   decompile_function 0x0041BBD0`, `0x0043C2D0` case `0x0F`.

3. **Do not implement "nearest unit wins" as a distance-sort.** The binary selects based on
   which unit's independent mission timer fires first while a slot is free. "Nearest" is an
   emergent correlation (closer units have their `Mission_Harvest` return path trigger HELLO
   sooner), not a sort. Evidence: `verified via decompile_function 0x004D9290`,
   `0x005B3060`, `0x0055AFB0` (sibling doc).

4. **Do not implement release-time notification.** Neither `UnitClass::Mission_Deploy_Building`
   state 4 nor `AircraftClass::Detach` sends any message to probing waiters. Evidence: sibling
   docs `TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`,
   `AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`.

5. **Do not treat `DockReservations` FIFO as a separate non-drift structure.** The same
   binary evidence covers it: building-side contact slots are unordered; the FIFO "promotes"
   on release is not in gamemd. Evidence: same as #1.

---

## Remaining Uncertainty

- The exact live-object iteration order for a concrete two-miner or multi-aircraft replay
  is runtime-only. Static analysis proves the mechanism (emergent from timer + object order);
  the winner in a specific scenario requires a runtime debugger trace.
- `AircraftClass::Mission_Enter @ 0x00419C80` uses a flight-phase state machine (`param_1[0x2f]`
  = 0..7) before reaching the CAN_DOCK probe. The exact delay between the aircraft's WaitForDock
  equivalent and its next `0x0E` probe depends on locomotor phase. This is not timer-table
  driven in the same way as `FootClass::Mission_Enter`; it appears to loop at rate `3` (state 6
  returns `3`). This does not affect the core finding (no queue), but the exact airfield retry
  cadence (aircraft side) has a different timer source than the miner side.
- `DockReservations` (the older, separate structure) also has FIFO semantics. Whether it is
  still reachable in live code paths needs a caller scan before deletion.

---

## Current Rust Status

| Surface | DRIFT verdict | Basis |
|---------|---------------|-------|
| `RefineryDockContacts::waiting_retry_queue` | **DRIFT** | No queue on building side |
| `hello_or_wait` FIFO gate (front-of-queue check before admission) | **DRIFT** | Any probe can win |
| `DockReservations::queues` + promote-on-release | **DRIFT** | Same mechanism |
| `AirfieldDocks::queues` + release-time FIFO promotion | **DRIFT** | No queue on airfield side |
| `AirfieldDocks::slots` (contact-slot identity) | **Aligned** | NumberOfDocks → RadioClass capacity; slot = DockingOffsetN index |
| Per-miner mission retry timer (14..16 frames) | **Missing** | Needs addition if exact frame parity is targeted |

---

## Sources

- Ghidra `decompile_function 0x0043C2D0` — `BuildingClass::Receive_Radio`
- Ghidra `decompile_function 0x004D9290` — `FootClass::Mission_Enter`
- Ghidra `decompile_function 0x005B3060` — `MissionClass::Mission_Dispatch`
- Ghidra `decompile_function 0x0041BBD0` — `AircraftClass::FindBuildingToDock`
- Ghidra `decompile_function 0x00419C80` — `AircraftClass::Mission_Enter`
- Ghidra `get_function_callers FootClass__Find_Docking_Bay` — only caller is `0x0041BBD0`
- Sibling docs (all read and reconciled):
  - `docs/research/miner/BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`
  - `docs/research/miner/WAITING_MINER_MISSION_TIMER_AFTER_BUSY_CANDOCK_GHIDRA_REPORT.md`
  - `docs/research/miner/TWO_CMIN_ONE_REFINERY_TAKEOVER_TIMING_GHIDRA_REPORT.md`
  - `docs/research/miner/TWO_CMIN_TAKEOVER_FRAME_ORDER_RETRY_GHIDRA_REPORT.md`
  - `docs/research/miner/TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_FRAME_ORDER_GHIDRA_REPORT.md`
  - `docs/research/AIRFIELD_RADIO_CACHEDDOCK_CONTACT_LIFETIME_GHIDRA_REPORT.md`
  - `docs/research/DOCKING_QUEUE_EXIT_REFERENCE_POINTS_GHIDRA_REPORT.md`
- Rust source scan: `src/sim/miner/miner_dock.rs`, `src/sim/docking/aircraft_dock.rs`,
  `src/sim/world/world_hash.rs`
- INI: `ini/rulesmd.ini`
