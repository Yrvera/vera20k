# ReadyToCommence (vtable+0x200) — Per-Subclass Overrides Ghidra Report

**Date:** 2026-06-02
**Confidence:** HIGH for all four override addresses and their primary predicate logic
(vtable slot identities verified via `read_memory`; function bodies verified via
`decompile_function`; struct offsets verified via `get_assembly_context`).

---

## Investigation Charter

**Target question:** Which concrete TechnoClass subclasses override vtable+0x200
(`ReadyToCommence`), and what readiness predicate does each override check before
allowing `Commence()` to promote the queued mission?

**Non-goals:** Re-decoding the whole mission dispatch switch, individual mission handler
bodies, Queue_Mission guards unrelated to ReadyToCommence, or the Commence() body.

**Evidence needed to mark COMPLETE:** vtable+0x200 slot address verified by
`read_memory` for each class; function at that slot decompiled; primary predicate(s)
identified with struct offsets read from assembly.

**Stop conditions:** All four concrete leaf classes (UnitClass, InfantryClass,
AircraftClass, BuildingClass) verified; FootClass confirmed as non-overrider.

---

## Settled Facts (Pre-Existing, Not Re-Derived)

- Base `ReadyToCommence` @ `0x004E0140` = `return 1` (always ready).
  Verified via `decompile_function 0x004E0140`.
- vtable slot = byte offset **+0x200** from the vtable base pointer stored at `*(this)`.
- Queue_Mission (`0x005B35E0`) calls `ReadyToCommence()` then `Commence()` when
  `commence_now` is true. Verified in MISSIONCLASS_STATE_MACHINE.md §Queue_Mission.
- Commence (`0x005B3570`) promotes `+0xB4` → `+0xAC`, resets timer (+0xD0=0).
- `AircraftClass__Queue_Mission_Override` (`0x0041BA90`) filters certain mission changes
  for approach/overfly states, then calls the base `MissionClass__Queue_Mission`.

---

## Vtable Base Addresses (Verified)

| Class | Vtable Base | Evidence |
|---|---|---|
| FootClass | `0x007E8C94` | from FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md |
| UnitClass | `0x007F5C70` | `get_assembly_context 0x007353da`: `MOV dword ptr [ESI],0x7f5c70` in constructor |
| InfantryClass | `0x007EB058` | `get_assembly_context 0x00517acc`: `MOV dword ptr [ESI],0x7eb058` in constructor |
| AircraftClass | `0x007E22A4` | `get_assembly_context 0x00413d87`: `MOV dword ptr [ESI],0x7e22a4` in constructor |
| BuildingClass | `0x007E3EBC` | `get_assembly_context 0x0043b71f`: `MOV dword ptr [ESI],0x7e3ebc` in constructor |

---

## Vtable+0x200 Slot Reads (Verified)

| Class | Vtable+0x200 Address | Slot Contents | Override? |
|---|---|---|---|
| FootClass | `0x007E8E94` | `0x004E0140` (base stub) | **NO** |
| UnitClass | `0x007F5E70` | `0x00744270` | **YES** |
| InfantryClass | `0x007EB258` | `0x00521B60` | **YES** |
| AircraftClass | `0x007E24A4` | `0x0041B5E0` | **YES** |
| BuildingClass | `0x007E40BC` | `0x00454250` | **YES** |

All five verified via `read_memory` at the respective slot addresses.

---

## Override Detail — UnitClass (0x00744270)

**Active in YR:** Yes — called for all ground vehicles with a queued mission.

**Ghidra label:** `UnitClass__ShouldIdle` (stale / likely mislabeled; this function is at
vtable+0x200 which is the ReadyToCommence slot; the label does not match the slot role).

**Verified via:** `decompile_function 0x00744270`, `read_memory 0x007F5E70`,
`get_assembly_context 0x007353da`.

**Predicate summary (returns 1 = ready, 0 = not ready):**

```
if CurrentMission(+0xAC) == 6 (Sticky) OR == 21 (Rescue) → return 0
if byte(this+0x6E1) != 0 → return 0   // locomotor sub-state flag
if byte(this+0x6E2) != 0 → return 0   // locomotor sub-state flag
if byte(this+0x6D1) != 0 → return 0   // locomotor occupancy flag

if QueuedMission(+0xB4) != 7 (Enter):
    locoIface = this+0x674 (param_1[0x19d])
    if locoIface == NULL → assert, crash
    isMoving = locoIface.vtable[0x80]()
    if isMoving != 0:
        height = this.vtable[0x1C8]()  // ObjectClass::GetHeight
        if height < 0 → return 0        // underground/invalid height
        curMission = this.vtable[0x184]()  // GetCurrentMission
        if curMission == 5 (Guard) → return 0
        if curMission == 1 (Attack) AND QueuedMission(+0xB4) == 0 → return 0
        if byte(this+0xB8) (IsCommenced) != 0 → return 0
    // ... additional spyplane approach pad check via FUN_004a51d0 ...

return 1
```

Key struct offsets (verified from assembly via `get_assembly_context 0x007353da` and
`decompile_function 0x00744270`):

| Offset | Meaning | Evidence |
|---|---|---|
| `+0xAC` | CurrentMission (`param_1[0x2b]*4`) | `MOV EAX,[ESI+0xAC]` at `0x00744277` |
| `+0xB4` | QueuedMission (`param_1[0x2d]*4`) | decompile `param_1[0x2d]` |
| `+0x6D1` | UnitClass locomotor occupancy flag | `MOV AL,[ESI+0x6D1]` in constructor init |
| `+0x6E1` | UnitClass sub-state flag 1 | `MOV byte[ESI+0x6E1],BL` in constructor |
| `+0x6E2` | UnitClass sub-state flag 2 | `MOV byte[ESI+0x6E2],BL` in constructor |
| `+0x674` | Locomotor interface pointer | `MOV EAX,[ESI+0x674]` at `0x00744290` |

**Additional note:** A secondary branch involving `FUN_004a51d0` and a "spyplane pad"
check (checking the building type at the unit's current cell for `+0x16BD` flag)
can also cause return 0. This path fires when there is an active ParaDrop/Spyplane
target cell and the unit is at or approaching it. This is YR-active but fires only
in specific ParaDrop/SpyPlane scenarios (not standard ground vehicle scenarios).

---

## Override Detail — InfantryClass (0x00521B60)

**Active in YR:** Yes — called for all infantry with a queued mission.

**Ghidra label:** `FUN_00521b60` (unlabeled).

**Verified via:** `decompile_function 0x00521B60`, `read_memory 0x007EB258`,
`get_assembly_context 0x00521b60`.

**Predicate summary:**

```
if CurrentMission(+0xAC) == 6 (Sticky) OR == 21 (Rescue) → return 0
if byte(this+0x68D) != 0 → return 0   // infantry-specific doing-action flag
if byte(this+0x8D) != 0 → return 0    // ObjectClass flag (in-transit/hidden state)

locoIface = this+0x674
isMoving = locoIface.vtable[0x80]()
if isMoving != 0:
    curMission = this.vtable[0x184]()   // GetCurrentMission
    if curMission == 5 (Guard) → goto checkType
    if curMission == 15 (0xF, SpyplaneApproach?) → goto checkType
    if curMission == 1 (Attack) AND dword(this+0x2B4) == 0 → return 0

checkType:
    typeIndex = dword(this+0x6C4)   // InfantryTypeClass index
    if typeIndex == -1 → return 1
    if g_InfantryTypeHasIdleSeq[typeIndex] != 0 → return 1
    else → return 0
```

Key struct offsets (verified from `get_assembly_context 0x00521b60` showing
`MOV AL,[ESI+0x68D]`, `MOV AL,[ESI+0x8D]`, `MOV EAX,[ESI+0x674]`,
`MOV EAX,[ESI+0x2b4]`, `MOV ESI,[ESI+0x6c4]`):

| Offset | Meaning | Evidence |
|---|---|---|
| `+0xAC` | CurrentMission | `MOV EAX,[ESI+0xAC]` at `0x00521B63` |
| `+0x68D` | Infantry "doing action" / panic/fear in-progress flag | `MOV AL,[ESI+0x68D]` at `0x00521B7B` |
| `+0x8D` | ObjectClass field — "occupied/limbo" state byte | `MOV AL,[ESI+0x8D]` at `0x00521B89` |
| `+0x674` | Locomotor interface pointer | `MOV EAX,[ESI+0x674]` at `0x00521B93` |
| `+0x2B4` | Infantry combat-lock counter (prevent mission switch mid-attack) | `MOV EAX,[ESI+0x2B4]` at `0x00521BE7` |
| `+0x6C4` | InfantryTypeClass index | `MOV ESI,[ESI+0x6C4]` at `0x00521BF1` |

**Type table:** `g_InfantryTypeHasIdleSeq` at base `0x007EAFF7C`, accessed as
`byte[typeIndex*4 + 0x7EAFF7C]` — a per-type flag. Infantry types returning 0 from
this table are permanently not-ready (infantry without idle sequences).
Verified via assembly `MOV AL,byte ptr [ESI*0x4 + 0x7eaf7c]` at `0x00521BFC`.

**Note on mission 15 (0xF):** In the MissionClass state machine doc, 0xF = Hunt.
Mission 15 as a guard condition here causes `checkType` rather than return 0 — meaning
infantry on Hunt mission with a queued re-tasking will skip the Attack-combat-lock
check and go directly to the type check.

---

## Override Detail — AircraftClass (0x0041B5E0)

**Active in YR:** Yes — all aircraft use this to gate mission changes.

**Ghidra label:** `AircraftClass__Is_Ready` (labeled; verified correct).

**Verified via:** `decompile_function 0x0041B5E0`, `read_memory 0x007E24A4`,
`get_assembly_context 0x0041b5e0`.

**Predicate summary:**

```
curMission = dword(this+0xAC)
if curMission == 6 (Sticky) OR == 21 (Rescue) → return 0
if byte(this+0x6D2) != 0 AND curMission != 30 (0x1E, SpyplaneApproach):
    → return 0   // abort-in-progress gate, bypassed only during Spyplane approach
return byte(this+0x6D4) != 0   // aircraft is "on airstrip" / landed-ready flag
```

Assembly verified via `get_assembly_context 0x0041b5f0`:
```
CMP EAX,0x6 / JZ return0
CMP EAX,0x15 / JZ return0
MOV DL,[ECX+0x6D2] / TEST DL,DL / JZ skip
CMP EAX,0x1E / JNZ return0
skip: MOV AL,[ECX+0x6D4] / TEST AL,AL / SETNZ AL / RET
```

Key struct offsets:

| Offset | Meaning | Evidence |
|---|---|---|
| `+0xAC` | CurrentMission | `MOV EAX,[ECX+0xAC]` at `0x0041B5E0` |
| `+0x6D2` | Aircraft "abort/weapon-fire in progress" flag (init=0 in constructor) | `MOV byte[ESI+0x6D2],0x0` at `0x00413D6D`; read at `0x0041B5F0` |
| `+0x6D4` | Aircraft "landed/ready" flag (init=1 in constructor) | `MOV byte[ESI+0x6D4],AL` (AL=1) at `0x00413D7B`; read at `0x0041B5FF` |

**Semantic note for +0x6D4:** Initialized to 1 in the AircraftClass constructor
(`0x00413D7B`). This means aircraft start "ready". The flag is cleared when the aircraft
departs its airstrip and restored when it lands. The ReadyToCommence gate is essentially:
"aircraft is currently landed and not mid-abort."

**Spyplane exception:** If `+0x6D2` is set AND `CurrentMission == 30`
(SpyplaneApproach/0x1E), the abort flag is ignored — the aircraft can still be re-tasked
mid-spyplane-approach.

---

## Override Detail — BuildingClass (0x00454250)

**Active in YR:** Yes — called for all buildings with a queued mission.

**Ghidra label:** `FUN_00454250` (unlabeled).

**Verified via:** `decompile_function 0x00454250`, `read_memory 0x007E40BC`,
`get_assembly_context 0x00454250`.

**Predicate summary:**

```
return byte(this+0x6DD) != 0
```

Assembly: `MOV AL,[ECX+0x6DD]` / `TEST AL,AL` / `SETNZ AL` / `RET`
at `0x00454250–0x0045425B`.

**Setter:** `BuildingClass__OnConstructionComplete` (`0x00445F80`) sets
`param_1->field_0x6DD = 1` near function end (after the `ActuallyPlacedOnMap` block).
Verified via `decompile_function 0x00445F80`: the literal line
`param_1->field_0x6dd = 1;` appears at the final assignment before the power-on /
EVA path.

**Semantic note:** `+0x6DD` is a "construction completed and placed" flag — distinct
from `ActuallyPlacedOnMap` (which is checked separately at the top of
`OnConstructionComplete`). A building without this flag set (e.g., a pre-placed map
editor building that never ran `OnConstructionComplete`, or a building still running its
construction animation) returns 0 from ReadyToCommence. Once `OnConstructionComplete`
runs and sets `+0x6DD = 1`, the building gates open permanently.

**Key offset:**

| Offset | Meaning | Evidence |
|---|---|---|
| `+0x6DD` | "ConstructionComplete/ready-to-operate" flag | `MOV AL,[ECX+0x6DD]` at `0x00454250`; set to 1 in `OnConstructionComplete` `0x00445F80` |

---

## Caller Path Verification

`Queue_Mission` at `0x005B35E0` is directly called by:
- `AircraftClass__Queue_Mission_Override` (`0x0041BA90`) — verified via
  `get_function_callers 0x005B35E0`.

`Commence` at `0x005B3570` is directly called by:
- `AircraftClass__Override_Mission` (`0x0041B870`) — verified via
  `get_function_callers 0x005B3570`.

Both are vtable-dispatched in normal operation. The `AircraftClass__Queue_Mission_Override`
intercept adds a filter: if `CurrentMission` is one of {4, 26, 27, 30, 31}
(Retreat/ParadropApproach/ParadropOverfly/SpyplaneApproach/SpyplaneOverfly) AND
`this+0x294` == 0, and the new mission is NOT one of those same five — it skips the
base Queue_Mission call entirely. Otherwise it falls through to `MissionClass__Queue_Mission`.

`this+0x294` identity: from the sibling report `AIRCRAFT_RADIO_DEAF_LATCH_0X294_LIFECYCLE_GHIDRA_REPORT.md`,
`+0x294` is an `AirstrikeClass*` pointer — null in stock YR. The `Queue_Mission_Override`
filter at `+0x294 == 0` therefore **always passes** in stock YR (pointer is always null),
meaning the filter reduces to: if in approach/overfly and re-tasking to non-approach/overfly,
skip the Queue_Mission call.

**Active in YR:** Yes. The AircraftClass Queue_Mission override fires for all aircraft.

---

## Unverified (YELLOW)

**UnitClass secondary branch (FUN_004a51d0 / spyplane pad):** The code path in
`UnitClass__ReadyToCommence` at `0x00744270` that calls `FUN_004a51d0()` and then
checks the building at the unit's cell for a `+0x16BD` type flag (and compares cell
coordinates with a building's anchor) was decompiled but the exact semantic of
`FUN_004a51d0` could not be fully resolved from callers alone. Callers include
`BuildingClass_DrawBody`, `UnitClass__AI`, `UnitClass__Mission_Guard`,
`UnitClass__Mission_Hunt_Override`, `UnitClass__Mission_Move` — likely a "is the map
in a specific draw/paradrop delivery mode" global mode flag. This path does not affect
normal ground-vehicle tasking and fires only under SpyPlane/ParaDrop conditions.
Insufficient to mark DRIFT for normal use.

**InfantryClass +0x8D meaning:** Identified as an ObjectClass field (byte at `+0x8D`).
Its precise semantic within ObjectClass was not decompiled this session. Likely a
"limbo/hidden" or "in-transport" byte. Low priority — it only blocks ReadyToCommence
when non-zero, and non-zero represents some invalid/in-transit state that would make
mission-change physically impossible anyway.

**InfantryClass mission-15 condition:** `GetCurrentMission == 0xF (15)` triggers the
same early-exit-to-checkType as mission 5 (Guard). From the mission table, 15 = Hunt.
Why Hunt gets the same "skip the attack-combat-lock check" treatment as Guard is
behaviorally reasonable but not traced further — the assembly is clear, the reason is
inferred.

---

## Implementation Handoff

### Rust delta for `MissionCom.ready_to_commence()`

The Rust `queue_mission(m, commence_now)` verb (§5.1.6 in the design doc) must call
a `ready_to_commence()` predicate before `Commence()`. The current design doc notes
base = `return true`. The per-class overrides now provide the concrete predicates.

**Four verified behaviors → Rust delta → affected surface → acceptance scenario →
proposed test → risk:**

---

**1. BuildingClass override — `+0x6DD` construction-complete flag**

Verified behavior: `BuildingClass::ReadyToCommence` returns `*(bool*)(this+0x6DD)`.
Set to `true` by `OnConstructionComplete`. Initially `false`.

Rust delta: Add a `construction_complete: bool` field to `GameEntity` (or the
building-specific component). `ready_to_commence()` for buildings returns this flag.
`on_construction_complete()` sets it `true`.

Affected surface: building mission queue in `world_commands.rs` / the new
`queue_mission` verb on `MissionCom`.

Acceptance scenario: A freshly placed building that has not run
`OnConstructionComplete` should NOT accept a queued mission until construction
finishes. A fully placed building (past construction) should accept immediately.

Proposed test: `test_building_readytocommence_gates_on_construction_complete` —
create a building entity, verify `queue_mission(Guard, commence=true)` does not promote
before `construction_complete=true`, then does promote after.

Risk: LOW for buildings — the field has a single setter and a single reader. No
race conditions in the sim tick order.

---

**2. AircraftClass override — `+0x6D4` landed-ready flag**

Verified behavior: `AircraftClass::ReadyToCommence` returns `byte(this+0x6D4) != 0`,
subject to the Sticky/Rescue/abort-flag checks.

Rust delta: Add `is_landed: bool` to aircraft entity (or `AircraftState`). Initialize
to `true` (matches constructor). `ready_to_commence()` for aircraft returns
`is_landed && current_mission != Sticky && current_mission != Rescue`.

Affected surface: `aircraft_dock.rs` takeoff/landing state transitions; the new
`MissionCom` verb API.

Acceptance scenario: Aircraft assigned a new mission while in-flight (landed=false)
must NOT promote until `is_landed` becomes true.

Proposed test: `test_aircraft_readytocommence_requires_landed` — set aircraft
`is_landed=false`, call `queue_mission(Attack, commence=true)`, verify
`current_mission` stays at previous value; set `is_landed=true`, retry, verify
promotes.

Risk: MEDIUM — `is_landed` setter must be wired to the flight phase transitions
in the mission handlers (Mission_Move, Mission_Guard, etc.). Getting this wrong
produces aircraft that ignore re-tasking or change missions mid-flight.

---

**3. InfantryClass override — locomotor-moving + type-has-idle-seq gate**

Verified behavior: Infantry returns ready if: not in Sticky/Rescue, no doing-action
flag, locomotor not moving (or if moving: not in mid-attack-commitment), and the
infantry type has idle sequences.

Rust delta: `ready_to_commence()` for infantry checks:
- `current_mission != Sticky && current_mission != Rescue`
- `!doing_action_flag` (existing "panic/doing-action" bool)
- infantry type has `Sequence=` data (`has_idle_seq: bool` from INI)

The locomotor-moving check and `+0x2B4` combat-lock counter are secondary — they
affect *when* during movement the promotion fires, but the type flag is the final
gate. In practice the locomotor `IsMoving` returning false allows the type check
to run immediately.

Affected surface: infantry mission assignment in the new `MissionCom` verb.

Acceptance scenario: Infantry type with no `Sequence=` line (e.g., some civilian
types) should never gate through ReadyToCommence for mission promotion.

Proposed test: `test_infantry_readytocommence_type_flag` — set
`has_idle_seq=false`, verify `queue_mission` with `commence=true` does not promote;
set `has_idle_seq=true`, verify it does.

Risk: LOW for the type-flag path. MEDIUM for the locomotor/combat-lock path — the
`+0x2B4` counter's full lifecycle (what sets it, what clears it) is not yet
investigated. Do NOT invent this counter; leave it unchecked and allow promotion
when `+0x2B4` is zero (the common case) until the counter's lifecycle is documented.

---

**4. UnitClass override — locomotor + mission-state composite gate**

Verified behavior: UnitClass::ReadyToCommence is the most complex: it blocks on
multiple locomotor flags, current-mission state (Guard/Attack), and IsCommenced.
Returns 1 (ready) when all flags are clear and locomotor/mission are in quiescent
state.

Rust delta: `ready_to_commence()` for ground vehicles checks:
- `current_mission != Sticky && current_mission != Rescue`
- No locomotor sub-state flags set (`+0x6D1`, `+0x6E1`, `+0x6E2`)
- If not entering (queued != Enter): locomotor.is_moving() and height check; then
  current mission not Guard/mid-Attack-without-queued

This is the most entangled with the locomotor substrate. **Implement conservatively**:
start with the missions-6-21 check and the `is_commenced` check; add locomotor
flags as the locomotor substrate matures.

Affected surface: ground vehicle mission assignment.

Proposed test: `test_unit_readytocommence_sticky_rescue_block` — verify Sticky and
Rescue current-missions block promote.

Risk: HIGH if locomotor flag lifecycle is not known. The `+0x6E1`/`+0x6E2`/`+0x6D1`
flags do not yet have Rust analogs. Start with the outer mission-enum checks only;
the locomotor flags are secondary refinements.

---

### Negative Facts / Do Not Do

1. **Do not implement FootClass::ReadyToCommence as an override.** FootClass vtable+0x200
   = `0x004E0140` (base stub, `return 1`). FootClass does not override this slot.
   Verified via `read_memory 0x007E8E94`.

2. **Do not assume `+0x6DD` is `ActuallyPlacedOnMap`.** The two are separate fields.
   `OnConstructionComplete` sets `+0x6DD` only inside `if (!ActuallyPlacedOnMap)` block
   AND only at the END, meaning there is a phase where `ActuallyPlacedOnMap == false`
   AND `+0x6DD == 0`. Do not conflate them.
   Verified via `decompile_function 0x00445F80`.

3. **Do not skip the ReadyToCommence call even when `commence_now=false`.** ReadyToCommence
   is only called when `commence_now=true` in Queue_Mission. There is no pre-check path
   in the base class. Do not add a defensive always-check.

4. **Do not assume aircraft start NOT-landed.** Constructor at `0x00413D7B` sets `+0x6D4=1`.
   Aircraft are born ready.
   Verified via `get_assembly_context 0x00413d74`.

5. **Do not implement `AircraftClass__Queue_Mission_Override`'s filter as a
   ReadyToCommence concern.** The filter (approach/overfly states blocking re-task) is
   in Queue_Mission, not ReadyToCommence. They are independent gates.
   Verified via `decompile_function 0x0041BA90`.

---

## Remaining Uncertainty

1. **UnitClass `+0x6E1`, `+0x6E2`, `+0x6D1` flag lifecycle:** Confirmed as UnitClass
   sub-state flags that block ReadyToCommence. Their setters/clearers (likely in
   locomotor or deploy handlers) have not been traced. The Deploy handler
   (`UnitClass__Deploy`) is a candidate setter for `+0x6E1`/`+0x6E2`.

2. **InfantryClass `+0x2B4` counter lifecycle:** The attack-combat-lock counter at
   `+0x2B4` is read in ReadyToCommence (mission=Attack blocks if zero). What increments
   and decrements it is unverified. Likely set by `Mission_Attack` and cleared on target
   loss/damage-cycle completion.

3. **InfantryClass `+0x8D` ObjectClass field:** Identified as a byte in ObjectClass
   territory that blocks infantry ReadyToCommence when non-zero. Its exact semantic
   (limbo, in-transport, etc.) is unverified this session.

4. **UnitClass secondary branch (FUN_004a51d0):** The spyplane-pad geographic check
   inside UnitClass ReadyToCommence. Not load-bearing for standard YR skirmish ground
   vehicle behavior. Defer until spyplane/paradrop simulation.

---

## Stale-Doc Replacement

**MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md §9.2** states:
> Per-subclass ReadyToCommence overrides — base is return 1; the actual promotion
> gates (e.g. unit-at-deploy-cell) are per-type and not decompiled. Resolve in Slice 6.

Replace the description with:

> Per-subclass ReadyToCommence overrides — RESOLVED. Base = return 1 (0x004E0140).
> Overrides: UnitClass(0x00744270) = locomotor+mission-state composite gate;
> InfantryClass(0x00521B60) = locomotor-moving + type-has-idle-seq gate;
> AircraftClass(0x0041B5E0) = landed-ready flag (+0x6D4, init=1) gate;
> BuildingClass(0x00454250) = construction-complete flag (+0x6DD) gate.
> FootClass = no override (base stub). See READYTOCOMMENCE_VTABLE_0X200_SUBCLASS_OVERRIDES_GHIDRA_REPORT.md.

---

## Cross-References

- Sibling doc: `docs/research/MISSIONCLASS_STATE_MACHINE.md` — verified Queue_Mission
  and Commence bodies, slot offsets.
- Sibling doc: `docs/research/FOOTCLASS_MISSION_HANDLERS_GHIDRA_REPORT.md` — FootClass
  vtable base.
- Sibling doc: `docs/research/AIRCRAFT_RADIO_DEAF_LATCH_0X294_LIFECYCLE_GHIDRA_REPORT.md`
  — AircraftClass +0x294 is AirstrikeClass*, always null in stock YR.
- Design doc: `docs/research/MISSION_RADIO_SUBSTRATE_SERVICE_DESIGN.md` §9.2.
