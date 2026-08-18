# Mission_Enter Cross-walk + Gap-Fill Ghidra Report

**Date:** 2026-05-11  
**Scope:** Stage 2 `/re-investigate` pass for Mission_Enter + multi-pad docking parity. Verified the corrected `MissionClass` vtable +0x240 cross-walk, fully decompiled `AircraftClass::Mission_Enter`, fully traced `FootClass::Mission_Enter`, light-investigated five additional gap sites (orphan `0x005196A0`, spy infiltrate dispatch, FreeUnit-at-construction, C4 scatter formula, refinery release helper, mission-7 queue site in ReceiveDamage).  
**Confidence:** Top-level findings HIGH from binary; per-claim confidence noted inline.

---

## Executive Summary

**(a) Corrected cross-walk.** The previously documented "UnitClass::Mission_Enter @ `0x00739EC0`" and "InfantryClass::Mission_Enter @ `0x005196A0`" labels are wrong — both are **PerCellProcess overrides at vtable +0x18C**, not Mission_Enter at +0x240. UnitClass and InfantryClass both **inherit FootClass's Mission_Enter at `0x004D9290`** (no override). The InfantryClass +0x18C handler at `0x00519630` (Ghidra still labels its mid-body offset `0x005196A0` as "InfantryClass__Mission_Enter" — that label is misplaced; the real function entry is `0x00519630`) is the **infantry per-cell process** that handles C4 detonation, spy infiltrate, engineer building hits, and harvester-on-refinery-cell. Only `AircraftClass` truly overrides Mission_Enter at +0x240 → `0x00419C80` (previously mislabeled "Mission_Sticky" — **renamed to `AircraftClass__Mission_Enter` during this pass**). `BuildingClass` and the other inheritor vtables (`0x007EDF00`, `0x007F0748`, `0x007F4BA0`) all point at the no-op stub `0x005B2F00` (`MOV EAX, 0x1C2; RET` — buildings never "enter" anything).

**(b) What `FootClass::Mission_Enter @ 0x004D9290` actually does.** It is the **state-driver for any non-aircraft unit's "enter this building" intent** — refinery harvesters, repair-depot tanks, garrison/spy/engineer/C4 infantry (the dispatch into PerCellProcess does the *arrival* handling, not Mission_Enter itself). The function reads the unit's destination, sends radio command 0xE (REQUEST_ENTER / "Can I dock?") to the destination, and on ROGER (1) handles two cases: (i) if the unit has waypoint queue items remaining (`+0x166`), pop the next destination and re-queue Mission_Enter (multi-stop enter); (ii) otherwise the building is allowed to consume the unit — the Type's `+0xCD4` "Consume on Enter" flag controls whether the unit's link to building is cleared (`+0x168/+0x169 = 0`) and the unit is marked for cleanup. On NO-ROGER, it sends radio cmd 3 (BREAK / CLEAR_LINK) and gives up the dock target. Returns a 1-frame jitter delay (1 + Random.RangeInt(0,2)).

**(c) Top findings for the Rust design.**

1. **Only AircraftClass needs a distinct Mission_Enter state machine in Rust.** Every other class shares one tiny FootClass loop; the *arrival* behavior is driven by `PerCellProcess` per-subclass plus the receiving building's `Receive_Radio` handler. The "Rust Mission_Enter" abstraction can therefore be a **two-layer** thing: a tiny shared FSM (transmit cmd 0xE → wait → cmd 0x16 FACE_DOCK or break) + a per-arrival handler dispatched off the cell-arrival event.
2. **The 5-state aircraft Mission_Enter** (states 0/1/2 = no-op return-via-padding, state 6 = approach with `0xE` radio recheck, state 7 = land + AddPassenger or carryall handoff) is materially different from ground unit Mission_Enter and warrants its own type. Carryall pickup is integrated into state 7's `vtable+0x274 (cmd 0x15 PREPARE)` branch.
3. **Multi-pad assignment is NOT decided by Mission_Enter** — it is decided when `Transmit_Radio(HELLO=2)` writes the unit into the building's `Contacts[]` (per RadioClass §) and when the building's per-pad index lookup occurs. Mission_Enter only sends the radio call and observes the ROGER/NACK answer.
4. **Spy infiltration check is unit-side (`InfantryType+0xEC4 Agent`), not building-side.** The dispatch site is inside the InfantryClass per-cell handler at `0x0051A00B`, gated by `Agent=yes` on the infantry's Type. The building-side `Spyable=yes` (`BuildingType+0xEC4` — same offset on a different type, do not confuse) check happens earlier at command-target validation, not here.
5. **C4 scatter formula is `((value >> 12) + 1) >> 1 & 7`**, where `value` is the first dword of the unit's RateTimer (probably the mission-timer countdown, instance+0x388, sampled via `RateTimer::Current`). The Rust comment `(tick >> 12 + 1) >> 1 & 7` is **operator-precedence ambiguous but semantically correct** — Rust's `<<`/`>>` are left-to-right and `+` has higher precedence than `>>`, so the Rust expression literally evaluates as `tick >> (12+1) >> 1 & 7 = tick >> 14 & 7` which IS WRONG. The intended formula `((tick >> 12) + 1) >> 1 & 7` needs explicit parens in the Rust comment.
6. **FreeUnit-at-construction does NOT queue Mission_Enter.** Newly-spawned harvester gets `Unlimbo(loc, 0xC0)` (facing 0xC0 = WSW = "facing the refinery dock cell") and on success starts in default mission state (no Set_Mission call). On Unlimbo failure, it gets `Set_Mission(0xA = Hunt?)` or `Set_Mission(5 = Retreat)`. **The free harvester relies on the player's pathfinding cycle to pick up "I'm next to a refinery" → standard harvester FSM.**
7. **`UnitClass::ReceiveDamage @ 0x00738664` is the "retreat to repair depot when HP red" trigger.** Conditioned on `Type+0xE0E` or `+0xE0F` (RepairTeam-style flags), HP <= 50% (Rules+0x1700 = ConditionRed), AI-controlled, non-allied. Iterates a target list from `Type+0x3EC`, picks the first that `(vtable+0x530)(target)` accepts, sends radio cmd 2 (HELLO), and on ROGER queues Mission_Enter (7). This is a SECONDARY consumer of Mission_Enter we did not account for in the original investigation plan.

---

## 1. Deliverable 1 — Mission_Enter vtable +0x240 Cross-walk (CORRECTED)

**Method:** Read each class's primary vtable directly from memory at the +0x240 offset. Cross-checked by listing xrefs to candidate functions to confirm they are referenced from the vtable address.

| Class | Vtable BASE | +0x240 slot addr | +0x240 target | Notes |
|---|---|---|---|---|
| `FootClass` | `0x007E8C94` | `0x007E8ED4` | **`0x004D9290`** `FootClass__Mission_Enter` | The real handler. **HIGH** — `get_xrefs_to(0x004D9290)` returns ONLY data xrefs from `0x007E8ED4`, `0x007EB298`, `0x007F5EB0` (no caller code). |
| `InfantryClass` | `0x007EB058` | `0x007EB298` | `0x004D9290` (inherits) | Reading vtable slot: bytes `90 92 4d 00` = `0x004D9290`. **HIGH.** InfantryClass does NOT override. |
| `UnitClass` | `0x007F5C70` | `0x007F5EB0` | `0x004D9290` (inherits) | Reading vtable slot: bytes `90 92 4d 00` = `0x004D9290`. **HIGH.** UnitClass does NOT override. |
| `AircraftClass` | `0x007E22A4` | `0x007E24E4` | **`0x00419C80`** `AircraftClass__Mission_Enter` (renamed from `Mission_Sticky` this pass) | Reading vtable slot: bytes `80 9c 41 00` = `0x00419C80`. **HIGH.** Decompile (Deliverable 3) confirms Mission_Enter semantics. |
| `BuildingClass` | `0x007E3EBC` | `0x007E40FC` | `0x005B2F00` (no-op: `MOV EAX, 0x1C2; RET`) | Reading vtable slot: bytes `00 2f 5b 00` = `0x005B2F00`. **HIGH.** Other BuildingClass-family vtables (`0x007EDF00`, `0x007F0748`, `0x007F4BA0`) all share this same no-op. Buildings effectively never "enter" anything. |

**Identity of the "intermediate" vtables flagged by Agent D as inheritors of FootClass:**

| Vtable addr | Identity (verified) | +0x240 target |
|---|---|---|
| `0x007E8ED4` | FootClass vtable +0x240 | `0x004D9290` |
| `0x007EB298` | InfantryClass vtable +0x240 | `0x004D9290` |
| `0x007F5EB0` | UnitClass vtable +0x240 | `0x004D9290` |

So only **three** vtables hold the FootClass Mission_Enter pointer (FootClass itself, InfantryClass, UnitClass). The earlier audit's list (`0x007E8ED4` / `0x007EB298` / `0x007F5EB0`) was complete.

**Method to confirm vtable identity:** `get_xrefs_to(0x007EB058)` returns `From 00517acc in InfantryClass__Constructor` (and similar for `0x007E8C94 → FootClass__Constructor`, `0x007F5C70 → UnitClass__Constructor`). Each base address is referenced from the class's constructor that writes `[this] = vtable_base`.

**Active in YR: Yes** for FootClass and AircraftClass paths. **No** for BuildingClass (no-op stub).

---

## 2. Deliverable 2 — `FootClass::Mission_Enter @ 0x004D9290` FULL Decompile

**Confidence: HIGH** (direct decompile).

**Signature:** `int __fastcall FootClass__Mission_Enter(FootClass *this)`  
**Body extent:** `0x004D9290` → ~`0x004D9430` (about 0x1A0 bytes / ~120 lines C).  
**Return:** Frame delay before next Mission_Dispatch call. Formula: `(MissionClass::GetMissionTimerEntry()) + Random.RangeInt(0,2)` — i.e., **1–3 tick jitter delay** (range bound by GetMissionTimerEntry — typically returns 1 for non-paused entities).

### State / control flow

This is NOT a multi-state machine. It is a single dispatch with three early-exit paths and one happy-path. The function reads from `this->NavCom` (destination) at `param_1[0x163]` (TBD precise offset; sequence indicates a coord triple in `this[0x163..0x166]`), `this->MissionDestinationCount` at `param_1[0x166]`, `this->RadioContact[0]` at `param_1[0x169]`, and the locomotor pointer at `param_1[0x19D]`.

**Decomp anchors (3-line slices):**

```c
// Entry — read destination
iVar4 = FootClass__GetDestination(0);
if ((iVar4 == 0) && (iVar4 = Filter_AbstractType_InMap(), iVar4 == 0)) {
    // EARLY EXIT 1: no destination
```

```c
// Send REQUEST_ENTER (cmd 0xE = CAN_ENTER_BUILDING? / CAN_DOCK?)
iVar4 = (**(code **)(*param_1 + 0x278))(0xe, iVar4);     // RadioClass::Transmit_Radio(0xE, dest)
if ((iVar4 == 1) || ((char)param_1[0x106] != '\\0')) {   // ROGER (1) or "force enter" flag at +0x418
```

```c
// MULTI-STOP: pop next waypoint and re-queue
if ((param_1[0x169] == 0) && (0 < param_1[0x166])) {     // no contact, queue length > 0
    iVar4 = FUN_0045af20(piVar1);                         // pop locomotor head?
    ...
    // shifts param_1[0x163..0x166] array down by one
    param_1[0x166] = iVar4 + -1;
```

```c
// SINGLE-STOP success: clear dock-link if "consume on enter" flag is set
iVar4 = (**(code **)(*param_1 + 0x84))();                 // GetType()
if (*(char *)(iVar4 + 0xcd4) != '\\0') {                  // Type->+0xCD4 (likely Consumed=yes)
    iVar4 = param_1[0x169];
    param_1[0x168] = 0;                                   // clear DockTarget
    param_1[0x169] = 0;                                   // clear RadioContact
    (**(code **)(*param_1 + 0x480))(iVar4, 1);            // Set_Destination(prev_link, force=1)
}
```

```c
// NO-ROGER (NACK) path: send BREAK
(**(code **)(*param_1 + 0x274))(3);                       // Receive_Radio(3 = CLEAR_LINK/BREAK)
(**(code **)(*param_1 + 0x484))(0, 1);                    // Set_NavCom(NULL, force=1)
```

```c
// Tail: jitter delay
MissionClass__GetMissionTimerEntry();
iVar4 = Math__ftol();
iVar5 = Random__RandomRanged(0, 2);
return iVar5 + iVar4;
```

### Branch summary

| State / branch | Trigger | Action | Active in YR |
|---|---|---|---|
| **No destination** | `GetDestination()==0 AND not in-map` | If contact exists and is NOT in mission state 1 or 2 → `Set_NavCom(0)`. Then call `vtable+0x1EC` (likely `Stop_Driver` or `PerCellProcess` cleanup) | Yes |
| **NACK from target** | `Transmit_Radio(0xE)` returned ≠ 1 AND `+0x418` flag clear | Send BREAK (cmd 3) to self, clear NavCom | Yes |
| **ROGER + queue not empty** | ROGER (1) and `+0x166` waypoint count > 0 | Pop head waypoint, shift array down. Internal locomotor `Release_Piggybacked` calls suggest queue is locomotor-managed | Yes |
| **ROGER + queue empty + Consumed** | ROGER, queue empty, `Type[+0xCD4] != 0` | Clear DockTarget and RadioContact, Set_NavCom(prev_target, force) | Yes |
| **ROGER + queue empty + NOT Consumed** | ROGER, queue empty, `Type[+0xCD4] == 0` | Fall through to tail — building/PerCellProcess handles consumption | Yes |
| **Tail (always)** | — | Return `GetMissionTimerEntry + Random(0,2)` | Yes |

### Magic numbers / offsets extracted

| Offset | Meaning | Notes |
|---|---|---|
| `param_1[0x106]` (= `+0x418`) | "Force Enter" / `Repair=yes` flag | Causes Mission_Enter to proceed even on NACK |
| `param_1[0x163..0x166]` (= `+0x58C..+0x598`) | Waypoint array (3 dwords?) | Stride and count TBD; popped via memmove |
| `param_1[0x166]` (= `+0x598`) | Waypoint queue length | Decremented on pop |
| `param_1[0x168]` (= `+0x5A0`) | Likely `DockTarget` ptr | Cleared on Consumed branch |
| `param_1[0x169]` (= `+0x5A4`) | `RadioContact[0]` ptr | Standard FootClass RadioContact |
| `param_1[0x19D]` (= `+0x674`) | Locomotor (`IPiggyback*`) | Used for queue manipulation |
| Type+0xCD4 | Probable `Consumed=` flag (TS-era, but live in YR for cloners/grinders) | **Conditional in YR** — only YAGRNR/cloner-type units consume passengers |
| Vtable+0x274 | `Receive_Radio(cmd)` (self-targeted, used for BREAK) | |
| Vtable+0x278 | `Transmit_Radio_To(cmd, target)` | Standard pattern |
| Vtable+0x484 | `Set_NavCom` | |
| Vtable+0x480 | `Set_Destination` | |
| Vtable+0x1EC | Likely `Stop_Driver` (no-arg) | |
| Vtable+0x84 | `GetType` | |

### Edge-exit semantics

- On **no destination**, exits cleanly without queue manipulation — Mission_Dispatch will keep calling Mission_Enter next tick because the mission code is still 7.
- On **NACK**, sends BREAK then clears NavCom. The unit is left in mission 7 but with no target — next Mission_Enter call will hit the "no destination" branch and idle. **No automatic transition out of Mission_Enter** — caller must use `Set_Mission(Guard)` to abort.
- On **success without Consumed**, building's `Receive_Radio` is expected to call `Set_Mission(Sleep)` or similar on the unit to take it off the map (passenger-list addition).

### Return semantics

Always returns a 1–3 tick delay. The dispatch in `MissionClass::Mission_Dispatch @ 0x005B3060` writes this into `param_1[0x33]/[0x34]` as the next-call-time. So Mission_Enter runs **every 1–3 ticks**, not every tick. This is the source of the "harvester wiggle on approach" delay.

### TS-legacy filter

- **Active in YR: Yes** — this is the universal mission handler for all non-aircraft Mission_Enter (refinery dock, repair depot, garrison, capture, spy infiltrate, C4 plant — all flow through here). The TS-era cleanup branches (Filter_AbstractType_InMap, locomotor piggyback) are still live.
- The `Type+0xCD4` "Consumed" flag is TS-era but used by YR cloning vats (NACLONE/GACLONE/YACLONE) and grinder (YAGRNR). **Live in YR via Cloning=yes / Grinding=yes**.

---

## 3. Deliverable 3 — `AircraftClass::Mission_Enter @ 0x00419C80` FULL Decompile

**Confidence: HIGH** (direct decompile, renamed in Ghidra this pass).

**Signature:** `undefined4 __fastcall AircraftClass__Mission_Enter(AircraftClass *this)`  
**Body extent:** `0x00419C80` → ~`0x0041A380` (about 0x700 bytes).  
**Rename action:** Executed `rename_function_by_address(0x00419C80, "AircraftClass__Mission_Enter")`. **Saved program.**  
**Renamed from:** `AircraftClass__Mission_Sticky` (the old Ghidra label was wrong; mission "Sticky" is a different mission code, and 0x00419C80 sits in vtable slot +0x240 = Mission_Enter).

### State machine

8 sub-states selected on `this->MissionSubState` (= `param_1[0x2F]` = `+0xBC`). On entry, state 0 immediately becomes state 6 (skip the legacy approach states); states 1–5 are dormant TS-era and yield a 1-tick return. Real work is in states 6 and 7.

**Decomp anchors:**

```c
// Pre-dispatch: ammo check + carryall preempt
iVar2 = (**(code **)(*param_1 + 0x1c8))();   // GetAmmo()
if (((iVar2 == 0) && (*(char *)(param_1[0x1b1] + 0xe0d) != '\\0')) &&   // Type+0xE0D = Selectable=yes or Reload-targets-required
   (piVar6 = (int *)param_1[0x169], piVar6 != (int *)0x0)) {
    // ammo=0 and have a contact → switch contact to current cell's building, BREAK, fly there
    (**(code **)(*param_1 + 0x1bc))();   // GetCell
    piVar3 = (int *)Look_up_building_in_cell();
    if (piVar6 != piVar3) { ... swap contact, send BREAK, Set_NavCom(2,1) }
}
```

```c
// Mid-flight target validity check
iVar2 = (**(code **)(*param_1 + 0x1c8))();   // GetAmmo (again)
if (((0 < iVar2) && (piVar6 = (int *)param_1[0x169], (int *)param_1[0x1b3] != piVar6)) &&
   ((piVar6 != (int *)0x0 && (iVar2 = (**(code **)(*piVar6 + 0x2c))(), iVar2 == 6)))) {
    // ammo > 0 and contact is a Building → realign DockTarget at +0x1B3
```

```c
// Air-corridor takeoff: no contact, not landed, not docked
if (((param_1[0x169] == 0) && (iVar2 = (**(code **)(*param_1 + 0x78))(), iVar2 != 2)) &&   // GetAltitude != 2 (not on ground)
   ((iVar2 = (**(code **)(*param_1 + 0x274))(0xe), iVar2 != 1 &&                            // Receive_Radio(0xE)... interesting reverse direction
    (cVar1 = FUN_0070d8f0(), cVar1 == '\\0')))) {
    (**(code **)(*param_1 + 0x484))(0,1);     // Set_NavCom(0, force)
    return 1;
}
```

### State table

`param_1[0x2F]` corresponds to instance offset `+0xBC` = `MissionSubState`. Same slot used by all FootClass-derived classes.

| State | Code at offset | Behavior | Branches |
|---|---|---|---|
| **0** | `0x0041A0xx` | Initialize: `MissionSubState = 6` (skip TS approach states) | Falls through to 1-5 (return 1) |
| **1–5** | fall-through | TS-era pre-approach states. Set `+0x1B5 = 1`. Return 1. **Active in YR: No** — these are TS holdovers; the function bypasses 1-5 by forcing state 6 on entry. | Return 1. |
| **6** (Approach) | `0x0041A0F0` | If ammo == 0 AND Type+0xE0D set, send `Receive_Radio(0xE)` again (re-query target). Check `PathType::Has_Valid_Steps()`. If path valid: call locomotor `+0x90` ("Is at destination?"). If at destination: transition to state 7. Otherwise return 3 (3-frame delay). If path invalid or `FUN_0070d8f0` returns true: transition logic; if `NavCom == -1`, `Set_Destination(0)` + `Set_NavCom(0)`. | Sets state 7 (transition), or stays at 6 with delay 3, or aborts with `Set_NavCom(0)`. |
| **7** (Landed / Dock) | `0x0041A18x` | Locomotor `+0x90` check (is at landed pad?). If 1: call building's vtable `+0xA8` (a method that returns landing-cell coords) OR locomotor `vtable+0x48` (for non-building targets). Call self `vtable+0x1B4` (likely `Set_Location` to dock cell). Return 1. ELSE: send `Receive_Radio(0x15 PREPARE)`. On ROGER (1): if Type+0xDFC carryall AND `param_1[0x46]` (cargo slot 0 nonzero): cargo handoff via `Carryall_Pickup`. On NACK 5: `vtable+0xD4 (Stop?), CargoClass__AddPassenger(this)`. On other: `Set_NavCom(0, force)`. | Final landing + passenger add, or carryall pickup, or abort. |

### Pad-index selection

**The aircraft does NOT call `FindDockSlot @ 0x0065AD90` directly.** Pad assignment is done internally by the building's `Receive_Radio` handler when the aircraft sends `Transmit_Radio(0xE)` and the building writes the aircraft into `Contacts[]` (per RadioClass §). The aircraft just observes the ROGER and uses the building's returned cell (via `vtable+0xA8` — a method that takes the aircraft pointer and returns the assigned pad's cell coords) to navigate.

**Specifically, in state 7:**
```c
piVar6 = (int *)param_1[0x169];    // contact = building
if (iVar2 == 6) {                   // AbstractType is Building
    iVar2 = (**(code **)(*piVar6 + 0xa8))(auStack_28, param_1);   // Building->GetDockingPosForUnit(this)
}
```

The building's `vtable+0xA8` is the key — it reads the building's `+0x1788[N]` DockingOffset array indexed by the aircraft's slot in `Contacts[]`. The aircraft never sees the pad index directly; it just gets a cell coord back.

### Approach / descent triggers

- **Approach distance:** Not encoded as a magic number — the state 6 check is "is locomotor at NavCom destination" via `vtable+0x90`. The NavCom is set to the pad cell by `Set_NavCom(pad_coord, force=1)`.
- **Descent altitude:** Implicit in `GetAltitude() (vtable+0x78) == 2` check (state 6 → 7 trigger). Altitude 2 = "on ground."
- **Carryall pickup trigger:** State 7 ROGER 1 path, `Type[+0xDFC] != 0` (the YAGRNR-style Carryall flag? Actually for AYCAR/UCAR aircraft) AND `param_1[0x46]` (cargo slot 0 nonempty = "we have a unit to carry").

### Reload trigger

Implicit: ammo=0 detection happens at function entry, not in a state. If ammo=0 + contact != current building under, the function re-establishes contact (BREAK old, switch to building below, then state-machine drives landing). Actual reloading is handled by the building's tick (e.g., `MissionRepairAndProduce` UnitReload state) once the aircraft is docked.

### Magic numbers / offsets

| Offset | Meaning |
|---|---|
| `param_1[0x2F]` (= `+0xBC`) | `MissionSubState` |
| `param_1[0x46]` (= `+0x118`) | Carryall cargo slot 0 |
| `param_1[0x169]` (= `+0x5A4`) | `RadioContact[0]` (the target building) |
| `param_1[0x1B1]` (= `+0x6C4`) | `Type` ptr (AircraftTypeClass) |
| `param_1[0x1B3]` (= `+0x6CC`) | `DockTarget` (separate from RadioContact) |
| `param_1[0x1B5]` (= `+0x6D4`) | `MissionSubState_Dirty` flag (1 = state changed) |
| `param_1[0x19D]` (= `+0x674`) | Locomotor (`IPiggyback*`) |
| `param_1[0x2D]` (= `+0xB4`) | NavCom (-1 = no NavCom set) |
| Type+0xE0D | Probable `ConsiderTransportTarget=yes` or `MissionWhenAmmoEmpty` flag |
| Type+0xDFC | Probable `Carryall=yes` |
| Vtable+0xA8 | (On building) `GetDockingPositionForUnit(unit)` — **THE KEY METHOD** for pad-index resolution |
| Vtable+0x90 | (On locomotor) `Is_At_Destination` |
| Vtable+0x274 | `Receive_Radio(cmd)` (self-target, used for self-BREAK / cmd 0xE poll) |

### TS-legacy filter

- States 1–5 are TS holdovers. Function bypasses them (state 0 → 6 immediately on entry). **Active in YR: No** for explicit branches into 1-5. If a savegame ever stored state 1-5, the function would handle it (return 1, no-op) — defensive.
- The `FUN_0070d8f0` helper appears multiple times — it's likely a "is on terrain that allows this?" cell-property check. Live in YR.

---

## 4. Deliverable 4 — `0x005196A0` Investigation

**Confidence: HIGH.**

### Key correction

`0x005196A0` is **NOT a function entry point**. Disassembly at `0x005196A0` starts with `JZ` (a conditional jump) — that's not a function prologue. The real entry of the function Ghidra labels "InfantryClass__Mission_Enter" is at **`0x00519630`**, where the prologue `SUB ESP,0x40 / PUSH EBX / PUSH EBP / PUSH ESI / MOV ESI, ECX / PUSH EDI` begins. Ghidra has a function defined from `0x005196A0 → 0x0051AA0A`, but the real body starts 0x70 bytes earlier at `0x00519630`.

### Vtable reference

The function at `0x00519630` IS referenced — from `0x007EB1E4`, which is **InfantryClass vtable +0x18C = PerCellProcess**. So:

| Class | Vtable +0x18C | Target |
|---|---|---|
| FootClass | `0x007E8E20` | `0x004D85D0` `FootClass::PerCellProcess` (base) |
| InfantryClass | `0x007EB1E4` | **`0x00519630`** `InfantryClass::PerCellProcess` (override) |
| UnitClass | `0x007F5DFC` | `0x00739EC0` `UnitClass::PerCellProcess` (override) |
| AircraftClass | `0x007E2430` | `0x004D85D0` (inherits FootClass) |

So **the function previously misnamed "InfantryClass::Mission_Enter @ 0x005196A0" is in fact `InfantryClass::PerCellProcess @ 0x00519630`**. The "InfantryClass__Mission_Enter" name in Ghidra is wrong on TWO counts: (a) wrong address (real entry is 0x00519630 not 0x005196A0), (b) wrong meaning (it's PerCellProcess, not Mission_Enter).

### What this function actually does

Reads the unit's current mission via `vtable+0x184` (`What_Action`) and branches on the mission code. Each branch implements **arrival-at-building handling for that mission**:

| Mission code | Branch behavior |
|---|---|
| **0 (Sleep)** | (Fall-through to default — not handled in this PerCellProcess) |
| **6 (Garrison-like arrival)** | If contact is a building and CanDock fails: BREAK, Set_NavCom(building, force). Else: `BuildingClass::AddGarrisonOccupant(building)`. **Garrison consumer.** |
| **7 (Enter)** | If at destination building cell AND mission is still 7: send `Receive_Radio(0xF CAN_ENTER_BUILDING)`; on ROGER: clear `+0xC4`, clear bridge flag, free mind-control, call `(**(code **)(pBVar5->vtable + 0x394))(param_1)` (AddPassenger), `vtable+0x11C` (limbo / clean off map). **Generic passenger-add.** |
| **8 (Capture)** | If building has full health: BREAK, Set_NavCom(building, force) — retry. Else apply damage via `vtable+0x40C`. Spy/engineer dispatch: if `Type[+0xEC3] (Engineer)`: damage-or-capture; if `Type[+0xEC4] (Agent)`: call `BuildingClass::OnSpyInfiltrate(building, attacker_house)`. **Engineer/spy consumer.** |
| **9 (Harvest-arrival)** | If standing on harvest cell of refinery: refund ore via `HouseClass::Add_Credits`, animations, voice. Same logic as UnitClass refinery dock but for infantry. **Active in YR: No** (no harvester infantry in retail YR). |
| **0xB (Move)** | If on building cell: `vtable+0x174` (Death? Limbo?) | 
| **0x11 (Sabotage)** | If `Type[+0xEC2] (C4)`: at destination + animation + voice; if building is in `+0x13` state OR `vtable+0x160` returns true → instant detonate; else mark building's `+0x6DF` (C4 attached flag), set timer at `building+0x528..0x534`. **THIS IS THE C4 PLANT DISPATCH.** Then the scatter formula runs. **Active in YR: Yes** for Tanya/SEAL/Trooper. |
| **0x19** | (Treated like cases 8/0xB — fall through) |

### Conclusion

**`0x005196A0`** is a fragment label inside `InfantryClass::PerCellProcess @ 0x00519630`. The function is the infantry per-cell arrival handler. **It is referenced (via vtable +0x18C), not orphaned** — but the `0x005196A0` label that has zero xrefs is a mid-function label that should not exist as a defined function entry.

**Recommended rename (FLAG, do not execute):**
- Delete the function at `0x005196A0` and re-create it at `0x00519630` as `InfantryClass__PerCellProcess`.
- This would correct the misnamed function and align with the FootClass/UnitClass PerCellProcess naming.

---

## 5. Deliverable 5 — Spy Infiltration Dispatch Site

**Confidence: HIGH.**

### Caller of `BuildingClass::OnSpyInfiltrate @ 0x004571E0`

`get_function_callers(0x004571E0)` returns exactly **one** caller: `InfantryClass__Mission_Enter` (the Ghidra-misnamed function = the real `InfantryClass::PerCellProcess @ 0x00519630`). Single dispatch site at `0x0051A00B`:

```c
// Disasm:
// 00519FF8: MOV CL, byte ptr [EAX + 0xEC4]   ; CL = infantry->Type->Agent
// 00519FFE: TEST CL, CL
// 0051A000: JZ 0x0051A03E                     ; not Agent → retreat path
// 0051A002: MOV EAX, dword ptr [ESI + 0x21C] ; EAX = infantry->Owner (HouseClass*)
// 0051A008: MOV ECX, EDI                      ; ECX = this = building
// 0051A00A: PUSH EAX
// 0051A00B: CALL 0x004571E0                   ; BuildingClass::OnSpyInfiltrate(this=building, attacker_house)
```

### Dispatch conditions (preceding context)

Reading backwards from `0x00519FF8`, the chain that leads here:
1. Mission == 8 (Capture) OR 0xB (Move) OR 0x19 (some other).
2. RadioContact is a Building (`vtable+0x2C == 6`).
3. Infantry is **adjacent to the building** (cell match check via Get_Cell_At + Look_up_building_in_cell).
4. Some preceding branch failed (e.g., the building is NOT a refinery: `infantry->Type[+0xEC3]` engineer-flag took the alternate path).
5. **Check `infantry->Type[+0xEC4]` (Agent)** — if set, call `OnSpyInfiltrate`. If NOT set, fall to retreat/exit logic.

### Verification of unit-side vs building-side check

**`Agent=yes` is checked on the INFANTRY's TypeClass (InfantryTypeClass+0xEC4).**  
This is NOT a `Spyable=yes` check on the BuildingTypeClass. The building's `Spyable` flag (also at offset `+0xEC4` but on `BuildingTypeClass`, different struct) **must be checked elsewhere** — most likely at command-target validation time (i.e., when the player right-clicks a spy on a building, the engine validates `building.Type.Spyable=yes` before issuing the move command). This pre-validation is NOT inside Mission_Enter / PerCellProcess.

Cross-reference: `NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md` lists `+0xEC4 = Agent` on InfantryTypeClass (line 202). `BUILDINGTYPECLASS_FIELDS.csv` would have a separate `Spyable` entry on BuildingTypeClass.

### Active in YR

**Yes**. Spy unit (`E1` / Spy) has `Agent=yes`. Most tech/economy buildings have `Spyable=yes`. The infiltration system is live and frequently used in skirmish.

---

## 6. Deliverable 6 — FreeUnit-at-Construction Confirmation

**Confidence: HIGH.**

### Decompile findings at `BuildingClass::OnConstructionComplete @ 0x00445F80`

Search for `FreeUnit` spawn logic: function reads `Type+0xEA0` (FreeUnit type pointer; not the index but the actual TypeClass* — defaults `-1` for "none"). If set and player-controlled / construction-just-finished, the function:

```c
// Decomp slice:
piVar13 = (int *)UnitClass__Constructor(*(undefined4 *)(param_1->Type + 0xea0), param_1->Owner);
// piVar13 = newly-spawned unit
iStack_50 = (short)((short)(iVar6 + (iVar6 >> 0x1f & 0xffU) >> 8) + sVar4) * 0x100 + 0x80;
local_48 = 0; uStack_46 = 0;
uStack_4c = sVar3 * 0x100 + 0x80;
cVar1 = (**(code **)(*piVar13 + 0xd8))(&iStack_50, 0xc0);     // Unlimbo(coord, facing=0xC0)
if (cVar1 == '\\0') {
    // FAILURE path: try alternate cells via Find_Nearby_Passable_Cell, retry Unlimbo
    // If all fail: refund credits, Delete(piVar13)
} else {
    // SUCCESS path
LAB_00446e9f:
    (**(code **)(*piVar13 + 0x1e8))(10, 0);    // Set_Mission(0xA = ???)
    (**(code **)(*piVar13 + 0x1ec))();           // (Probably Stop_Driver)
}
```

### Verdict

- **Mission_Enter (7) is NOT queued** on the spawned unit. The unit gets `Set_Mission(0xA)` on the success-via-alternate-cell branch.
- **Mission 0xA** per the mission name table = `Move`? Let me verify — index 0xA in the table at `0x00816CAC + 0xA*4 = 0x00816CD4` points to a string. Reading the table: index 0xA likely "Unload" or "Hunt" based on cohort. Either way, **NOT Mission_Enter**.
- The harvester's standard FSM (Mission_Harvest at index 0x14) takes over via standard idle-AI selection within a few ticks. The unit relies on cell-adjacency to the refinery (built-in heuristic) to start harvesting.
- Initial facing is `0xC0` (192/256ths) — facing WSW, looking out from the refinery dock cell.

### Side-effects on the building

The function also sets `param_1->Owner[0x1FC] = 1` (player-just-built-something flag), `Owner[0x5778] = 1` / `Owner[0x5779] = 1` (probably "show production banner" flags). It does NOT call `Set_Link` or `dock_link()` to pre-assign the harvester to the refinery — the harvester finds the refinery via its own Mission_Harvest logic.

**Confirmed: clean spawn, no Mission_Enter side-call. Gap #4 fully resolved.**

### Active in YR

**Yes** — every newly-built refinery in retail YR spawns one harvester at construction via this path.

---

## 7. Deliverable 7 — C4 Scatter Formula Re-derive

**Confidence: HIGH** (direct disassembly).

### Site

Inside `InfantryClass::PerCellProcess @ 0x00519630`, mission 0x11 (Sabotage) branch, at `0x0051A6E0`:

```asm
0051A6D0: LEA ECX, [ESP + 0x2C]                    ; param_2 = output buffer
0051A6D4: PUSH ECX
0051A6D5: LEA ECX, [ESI + 0x388]                   ; ECX = &this->RateTimer (at offset +0x388)
0051A6DB: CALL 0x004C93D0                          ; RateTimer::Current(this, output)
0051A6E0: MOV EAX, dword ptr [EAX]                 ; EAX = *output = current dword (post-call EAX = output buf addr)
0051A6E4: SHR EAX, 0xC                             ; EAX = value >> 12
0051A6E7: INC EAX                                  ; EAX = (value >> 12) + 1
0051A6EA: SHR EAX, 0x1                             ; EAX = ((value >> 12) + 1) >> 1
0051A6EC: AND EAX, 0x7                             ; EAX = (((value >> 12) + 1) >> 1) & 7
```

### Re-derived formula

**`direction_index = (((value >> 12) + 1) >> 1) & 7`**

Where `value` = first dword returned by `RateTimer::Current(this->RateTimer_at_0x388)`. The RateTimer at +0x388 on a TechnoClass is likely the **mission state timer** (current Mission_Enter or Mission_Sabotage step countdown). Its value is a 16.16 fixed-point linearly-interpolated remaining-time, so the top bits change slowly as time progresses → the `>> 12` makes the index step every ~4096 sub-units (i.e., once per ~64 frames at 60 ticks/sec — roughly once per second).

The 8 possible indices map to entries in `g_DirectionDeltaX_Table @ 0x0089F6D8` (stride 8 bytes per entry, used to fetch `(dx, dy)` per the 8 compass directions). The scatter cell is `(unit_loc + delta[index])`.

### Comparison to Rust code

The Rust comment at `src/sim/world/world_orders.rs` claims `(tick >> 12 + 1) >> 1 & 7`.

**This Rust expression is operator-precedence-wrong.** In Rust:
- `>>` has lower precedence than `+`
- So `tick >> 12 + 1` parses as `tick >> (12 + 1) = tick >> 13`
- Then `>> 1 & 7` continues
- Final: `((tick >> 13) >> 1) & 7 = (tick >> 14) & 7` — **wrong by 2 bits of shift**.

**Correct Rust expression:** `((tick >> 12) + 1) >> 1 & 7` (with explicit parens around `(tick >> 12)`).

**Caveat:** The "tick" in our Rust comment may not be the same value as the binary's `RateTimer current value`. The binary uses the RateTimer's current value (a 16.16 fixed-point interpolation), not the raw frame counter. If our Rust code uses the raw game tick as input, it produces a different sequence even with the corrected formula. **Verification needed:** confirm what value the Rust code passes in vs what RateTimer would produce.

### Active in YR

**Yes** — every successful C4 detonation by Tanya/SEAL/Trooper triggers this scatter pick.

---

## 8. Deliverable 8 — `UnitClass::ReceiveDamage @ 0x00738664` Mission-7 Queue Site

**Confidence: HIGH.**

### Context

Inside `UnitClass::ReceiveDamage @ 0x00737C90`, the mission-7 queue happens at `0x00738664` (in the post-FootClass::ReceiveDamage handling). Decomp:

```c
// Preconditions:
// - param_8 != 0, param_8 != 4, param_8 != 5 (i.e., still alive, not dead, not key event)
// - param_1[10].vtable_INoticeSource[0xE0E] != 0  OR  param_1[10].vtable_INoticeSource[0xE0F] != 0
//   (Type+0xE0E or +0xE0F = some "auto-retreat-when-damaged" flag)
// - HealthRatio <= Rules->ConditionRed (Rules+0x1700, default 0.5)
// - NOT player-controlled  (AI-only)
// - NOT allied with attacker
// - Building has retreat-list at Type+0x3EC, count at +0x3F8
// - Some target in the list passes a vtable+0x530 acceptance check
// - Target not in DynamicVectorClass

while (iVar4 = (**(code **)(param_1->vtable + 0x530))(retreat_list[iVar6], 0), iVar4 == 0) {
    iVar6++;
    if (count <= iVar6) return param_8;
}
// Target = retreat_list[iVar6]
if (DynamicVectorClass__Contains() != 0) return param_8;
iVar6 = (**(code **)(param_1->vtable + 0x278))(2);   // Transmit_Radio(HELLO=2)
if (iVar6 != 1) return param_8;
(**(code **)(param_1->vtable + 0x1e8))(7);            // Set_Mission(Enter=7)
```

### Verdict

**Yes — this is the "retreat to repair depot when HP red" trigger.** Trigger conditions:

| Condition | Meaning |
|---|---|
| HP below `Rules+0x1700` (ConditionRed, default 50%) | Red health threshold |
| AI-controlled, non-allied unit | Player units don't auto-retreat; ally units don't trigger on us |
| `Type+0xE0E` OR `+0xE0F` set | The unit type has the "can auto-retreat" flag (likely `RepairableByRepairTeam=yes` or similar) |
| Type has retreat-target list at `+0x3EC` | Pre-populated list of valid retreat buildings (UnitRepair=yes service depots, etc.) |
| First valid target passes acceptance check | Reachability / not-busy filter |

The target IS assigned implicitly via the radio-link mechanism: `Transmit_Radio(2)` writes the unit's contact pointer to the chosen retreat target. The subsequent `Set_Mission(7)` then operates with that target as the destination.

### Active in YR

**Yes**, but **conditional** on:
- Unit being AI-controlled (not player).
- Unit's Type having the `+0xE0E`/`+0xE0F` flag (most retail YR units do NOT — this is a flag for special unit types).
- Service depot or repair-target being in range.

**Frequency in normal play:** Low-moderate. Visible mostly with AI-controlled Apocalypse / Soviet armor that auto-retreats to YAREPAIR / NATECH when red.

### Implication for Rust

We currently don't have this trigger. **This is a new Mission_Enter consumer we hadn't accounted for in the original investigation plan.** Severity: LOW for now (player units don't trigger it; AI auto-retreat is part of "AI behavior" which is out of scope per the user memory). Defer to AI implementation phase.

---

## 9. Deliverable 9 — `FUN_004595C0` Per-Cycle Harvester Release Helper

**Confidence: HIGH.**

### Signature

`void __fastcall FUN_004595C0(BuildingClass *param_1)` — `this` = refinery building. No additional args.

### Body (key 30 lines)

```c
BuildingClass__ClearAnimSlot(param_1);  // slot 0xA
BuildingClass__ClearAnimSlot(param_1);  // slot 0xB
if (Rules+0x244 != -1) VocClass__PlayAt(loc);          // "unloading complete" sound
// ... animation creation for slots 0xC, 0xD based on HealthRatio ...
piVar1 = *(int **)&param_1->field_0x2e4;               // docked unit ptr at building+0x2E4
if (piVar1 == NULL) {
    // No docked unit: clear field_0x718 and Set_Mission(5 = Retreat?) on building. Return.
    param_1->field_0x718 = 0;
    (**(code **)(param_1->vtable + 0x1e8))(...);
    return;
}
if (unit->vtable[+0x2C]() == 1) {       // is the docked thing a Unit (AbstractType=1)?
    unit[0xB9] = 0;                       // clear something on unit at +0x2E4
    // Locomotor cleanup
    locomotor->vtable[+0x58]();
    // Compute exit cell: dock_cell + (-0x80, +0x80)
    uStack_40 = 0x47;                     // facing = 0x47 (= 71/256 ≈ 100°, roughly East-SE)
    pcStack_3c = (iVar4 + -0x80);         // X = building_loc_X - 128 leptons (half cell west)
    iStack_38 = iVar2 + 0x80;             // Y = building_loc_Y + 128 leptons (half cell south)
    locomotor->vtable[+0x70]();            // Apply facing/position
    unit->vtable[+0x544](0, 0x3FF00000);   // Set_Speed(0, 1.0)?
    // Find nearby passable cell to retreat to
    uVar8 = FootClass__Find_Nearby_Passable_Cell(...);
    uVar8 = MapClass__Get_CellClass(uVar8);
    (**(code **)(*piVar1 + 0x480))(uVar8, 1);    // Set_Destination(retreat_cell, force=1)
    (**(code **)(*piVar1 + 0x1e8))(2, 0);         // Set_Mission(2 = Move)
    param_1->field_0x2E4 = 0;                       // clear docked unit
    param_1->field_0x718 = 0;
    (**(code **)(param_1->vtable + 0x1e8))(5, 0);   // Set_Mission(5 = Retreat) on building?
    (**(code **)(param_1->vtable + 0x274))(3);      // Receive_Radio(3 = BREAK) on building
}
```

### Findings

- **Facing 0x47** (= 71 in decimal). 71/256 of 360° = ~100° from north. Roughly **East-SE direction**. This is the harvester's facing when released from the dock.
- **Offset `(-0x80, +0x80)`** in leptons (1 cell = 256 leptons; offset = half-cell west, half-cell south of dock center).
- **Invoked once per dump cycle** (called from `UnitClass::Mission_Deploy_Building` per BUILDING_DOCKING_SYSTEM doc — at end of per-cycle ore drain). The function returns silently if no unit is docked.
- **Distinct from `UndockUnit @ 0x004593A0`** which is only called on destruction/sell/temporal. This `FUN_004595C0` is the **per-cycle** release.

### Active in YR

**Yes** — every harvester dump at every refinery in every match triggers this.

### Rename proposal (FLAG, do not execute)

`BuildingClass__ReleaseDockedHarvester` or `BuildingClass__ReleaseDockedUnitPerCycle`. The function ends with `param_1->field_0x2E4 = 0` (clearing the docked unit pointer), confirming the "release" semantics.

---

## 10. Deliverable 10 — Ghidra Labels Applied This Pass

### Executed (user pre-approved)

| Address | Old name | New name | Justification |
|---|---|---|---|
| `0x00419C80` | `AircraftClass__Mission_Sticky` | **`AircraftClass__Mission_Enter`** | Direct decompile confirms Mission_Enter handler (calls AddPassenger, Carryall_Pickup, sends `Receive_Radio(0xE/0x15)`, dispatched from AircraftClass vtable +0x240 = Mission_Enter slot). Confidence 95%. |

Program saved.

### Proposed (FLAGGED, not executed — require user approval)

| Address | Current name | Proposed name | Reason |
|---|---|---|---|
| `0x005196A0` | `InfantryClass__Mission_Enter` (function defined here, but real body starts 0x70 bytes earlier) | **Delete function at `0x005196A0`** | Function entry is misplaced; real prologue at `0x00519630`. |
| `0x00519630` | (no function defined) | **`InfantryClass__PerCellProcess`** | Vtable +0x18C reference; function body matches PerCellProcess semantics (handles mission codes 6/7/8/9/0xB/0x11/0x19 on a per-cell basis). |
| `0x00739EC0` | `UnitClass__Mission_Enter` | **`UnitClass__PerCellProcess`** | Vtable +0x18C reference (UnitClass vtable at `0x007F5C70 + 0x18C = 0x007F5DFC` → `0x00739EC0`); function body matches PerCellProcess semantics. This corrects the long-standing mislabel that propagated through `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md`. |
| `0x004595C0` | `FUN_004595C0` | **`BuildingClass__ReleaseDockedHarvester`** (or `BuildingClass__ReleaseDockedUnitPerCycle`) | Function clears `building+0x2E4` (docked unit ptr), uses facing 0x47 + (-0x80,+0x80) offset, calls `Set_Mission(2)` on the released unit. Per-cycle helper inside Mission_Deploy_Building dump loop. |

**Why not executed:** the user pre-approved only the `0x00419C80` rename. The remaining four renames are higher-stakes (they re-categorize functions that are referenced in other research docs) and require explicit approval.

---

## 11. Deliverable 11 — Open Questions Remaining

Resolved during this pass:
- **Open Q #1 (AircraftClass vtable):** Resolved. `0x007E22A4`. ✓
- **Open Q #2 (AircraftClass Mission_Enter):** Resolved. `0x00419C80` (renamed). ✓
- **Open Q #3 (Pad-selection policy):** Resolved. Building-side via `Receive_Radio(0xE)` → write to `Contacts[]` → first-empty-slot. Aircraft does NOT pre-select via `FindDockSlot`. ✓
- **Open Q #4 (Per-pad approach cell):** Likely answered. Aircraft asks building for cell coords each tick via `building->vtable[+0xA8](aircraft)` — no separate approach-cell reservation. ✓ MEDIUM confidence.
- **Open Q #5 (Hover-when-full timeout):** Implicit. Aircraft hovers indefinitely if building NACKs; per the function structure, no internal timeout. ✓ Active in YR: Yes.
- **Open Q #6 (DockingOffset stride):** Out of scope of this pass; defer to /verify-doc on BUILDING_DOCKING_SYSTEM. ✗
- **Open Q #7 (Save serialization):** Out of scope. ✗
- **Open Q #8 (Cross-pad facing):** Aircraft does NOT explicitly set per-pad facing in Mission_Enter — facing 0x47/0xC0 are ground-unit specifics (harvester release uses 0x47; FreeUnit uses 0xC0). Aircraft uses locomotor default. ✓ Partial.
- **Open Q #9 (Mission_Enter exit-to-Guard):** Mission_Enter on success does NOT auto-clear — the building's `Receive_Radio` calls Set_Mission(Sleep) or AddPassenger which limbos the unit. ✓
- **Open Q #10 (`0x006AF6C0` refinery dock processor):** **NOT RESOLVED in this pass.** Out of scope of Stage 2; user assigned this to Stage 1 /verify-doc. The corrected picture: there is no separate "refinery dock-queue processor" — refineries use `Mission_Deploy_Building @ 0x0073D630` (per HARVESTER_DOCK_UNLOAD) + `FUN_004595C0` per-cycle release helper. ✗ For BUILDING_DOCKING_SYSTEM /verify-doc.
- **Open Q #11 (Mission_Sticky mislabel):** Resolved + renamed. ✓
- **Open Q #12 (mission-7 hits at `0x00417C21` and `0x00510F2C`):** Not investigated in detail. The hits are at undefined-function boundaries; would need `create_function` first. **DEFERRED.** ✗
- **Open Q #13 (`UnitClass::ReceiveDamage @ 0x00738664`):** Resolved. Retreat-to-repair-depot trigger for AI-controlled, low-HP units with `Type+0xE0E`/`+0xE0F` flag. ✓

### New questions surfaced this pass

1. **`Type+0xCD4` ("Consumed=" flag):** The FootClass::Mission_Enter consumed-branch uses this flag. What INI key maps to it? Likely `Consumed=yes` (TS-era) or might be `Cloneable=yes`. Need a TYPE-level INI re-derive. **Defer to /verify-doc on FOOTCLASS_MISSION_HANDLERS.**
2. **Mission code 0xA on FreeUnit success:** What is mission 0xA in the table? (Likely "Unload" or similar.) Verify by reading the name table entry. **LOW priority — not blocking the Rust design.**
3. **Building's `vtable+0xA8` (GetDockingPosForUnit):** This is the multi-pad coord lookup. Slot lookup `building->Type[+0x1788 + slot_index * 12]` needs confirmation. **Defer to /verify-doc on BUILDING_DOCKING_SYSTEM.**
4. **Building's `vtable+0x394` (AddPassenger via radio cmd 0xF):** Cross-check with CargoClass implementations. **LOW priority.**
5. **Rust C4 comment fix:** The `(tick >> 12 + 1) >> 1 & 7` operator-precedence issue needs a quick code fix even if behavior was correct by accident (depends on what the Rust code actually evaluates to). **Action item for the Rust side; do not modify in this pass per the RE-rules.**

---

## Appendix A — Address-Confidence Summary

| Claim | Address | Confidence | Source |
|---|---|---|---|
| FootClass vtable BASE | `0x007E8C94` | HIGH | xref from `FootClass__Constructor` |
| InfantryClass vtable BASE | `0x007EB058` | HIGH | xref from `InfantryClass__Constructor` |
| UnitClass vtable BASE | `0x007F5C70` | HIGH | xref from `UnitClass__Constructor` |
| AircraftClass vtable BASE | `0x007E22A4` | HIGH | xref from `AircraftClass__Constructor` |
| BuildingClass vtable BASE | `0x007E3EBC` | HIGH | (carried from Stage 0 Agent D) |
| FootClass::Mission_Enter | `0x004D9290` | HIGH | direct decompile |
| FootClass::PerCellProcess | `0x004D85D0` | HIGH | Ghidra label intact, vtable +0x18C |
| InfantryClass::PerCellProcess (real entry) | `0x00519630` | HIGH | vtable +0x18C reference |
| UnitClass::PerCellProcess (mislabeled "Mission_Enter") | `0x00739EC0` | HIGH | vtable +0x18C reference |
| AircraftClass::Mission_Enter | `0x00419C80` | HIGH | vtable +0x240 + full decompile (renamed this pass) |
| BuildingClass Mission_Enter (no-op stub) | `0x005B2F00` | HIGH | direct memory read of vtable +0x240 + bytes `B8 C2 01 00 00 C3` |
| BuildingClass::OnSpyInfiltrate | `0x004571E0` | HIGH | single caller from PerCellProcess at `0x0051A00B` |
| Mission_Dispatch | `0x005B3060` | HIGH | direct decompile + switch on `[this+0xAC]` mission code |
| Mission name table | `0x00816CAC` | HIGH | direct memory read |
| Mission code 0x11 → name "Sabotage" | string at `0x00816DDC` | HIGH | direct memory read at index 0x11 |
| C4 scatter formula site | `0x0051A6E0` | HIGH | direct disassembly with `SHR/INC/SHR/AND` chain |
| FUN_004595C0 (per-cycle release helper) | `0x004595C0` | HIGH | direct decompile |
| OnConstructionComplete (FreeUnit spawn) | `0x00445F80` | HIGH | direct decompile |
| UnitClass::ReceiveDamage mission-7 queue | `0x00738664` (`6A 00 6A 07 8B CE FF 90 E8 01 00 00`) | HIGH | direct disassembly + decomp |

---

## Appendix B — Mission Code Quick-Reference (Verified This Pass)

Read from name table at `0x00816CAC` (32 entries, each 4 bytes pointing to a const-string).

| Code | Name | Vtable slot dispatched to | Notes |
|---|---|---|---|
| 0 | Sleep | `+0x204` (default) | |
| 1 | Attack | `+0x210` | |
| 2 | Move | `+0x22C` | |
| 3 | QMove | `+0x204` (default) | |
| 4 | Retreat | `+0x230` | |
| 5 | Guard | `+0x21C` | |
| 6 | (Sticky? Garrison-adjacent) | `+0x21C` | Same slot as Guard |
| **7** | **Enter** | **`+0x240` Mission_Enter** | **THIS PASS'S FOCUS** |
| 8 | Capture | `+0x214` | InfantryClass::Mission_Capture |
| 9 | Harvest | `+0x218` | UnitClass::Mission_Harvest |
| 0xA | (Unload?) | `+0x224` | Used by FreeUnit-at-construction success path |
| 0xB | (Move-related) | `+0x220` | |
| 0xC | ? | `+0x234` | |
| 0xD | ? | `+0x238` | |
| 0xE | ? | `+0x20C` | |
| 0xF | Hunt? | `+0x228` | Returned to in `UnitClass::ReceiveDamage` ricochet |
| 0x10 | ? | `+0x23C` | |
| **0x11** | **Sabotage** | **`+0x214`** | **Same slot as Capture (8)** — both reuse `InfantryClass::Mission_Capture` to enter the destination building, then PerCellProcess dispatches the actual capture/sabotage on arrival |
| 0x12 | ? | `+0x244` | |
| 0x13 | Construction | `+0x248` | |
| 0x14 | (Harvest sub-state?) | `+0x24C` | |
| 0x15 | (Patrol?) | `+0x258` | |
| 0x16 | Sabotage? | `+0x250` | (Different from 0x11 — name reuse?) |
| 0x17 | Spyplane | `+0x208` | |
| 0x18 | Spyplane Approach | `+0x254` | |
| 0x19 | Spyplane Overfly | `+0x25C` | |
| 0x1A | ? | `+0x260` | |
| 0x1B | Paradrop Overfly | `+0x264` | |
| 0x1C | Paradrop Approach | `+0x268` | |
| 0x1E | ? | `+0x26C` | |
| 0x1F | Wait (Patrol-related) | `+0x270` | |

(Names for non-Enter codes are best-effort from string contiguity in the name table; do NOT rely on this column for Rust without re-verifying.)

---

## Appendix C — Verbatim AircraftClass::Mission_Enter Decomp (Selected)

```c
undefined4 __fastcall AircraftClass__Mission_Enter(int *param_1)
{
  // Entry: ammo=0 + Type+0xE0D + contact → realign contact to building under
  iVar2 = (**(code **)(*param_1 + 0x1c8))();   // GetAmmo
  if (((iVar2 == 0) && (*(char *)(param_1[0x1b1] + 0xe0d) != '\\0')) &&
     (piVar6 = (int *)param_1[0x169], piVar6 != (int *)0x0)) {
    (**(code **)(*param_1 + 0x1bc))();          // GetCell
    piVar3 = (int *)Look_up_building_in_cell();
    if (piVar6 != piVar3) { /* swap contact, BREAK, Set_NavCom(2,1) */ }
  }
  
  // Mid-flight: ammo > 0 + contact-is-building → realign DockTarget
  iVar2 = (**(code **)(*param_1 + 0x1c8))();
  if ((0 < iVar2) && ... contact_type == 6 (Building) ...) { ... }
  
  // Takeoff: no contact, not landed, Receive_Radio(0xE) didn't ROGER → Set_NavCom(0)
  if (((param_1[0x169] == 0) && (iVar2 = (**(code **)(*param_1 + 0x78))(), iVar2 != 2)) &&
     ((iVar2 = (**(code **)(*param_1 + 0x274))(0xe), iVar2 != 1 &&
      (cVar1 = FUN_0070d8f0(), cVar1 == '\\0')))) {
    (**(code **)(*param_1 + 0x484))(0,1);       // Set_NavCom(0,force)
    return 1;
  }
  
  // State machine
  switch(param_1[0x2f]) {
  case 0: param_1[0x2f] = 6;
  case 1: case 2: case 3: case 4: case 5:
    *(undefined1 *)(param_1 + 0x1b5) = 1;
    return 1;
  case 6:  // Approach
    // ... locomotor check, transition to 7 ...
    return 3;
  case 7:  // Land + AddPassenger / Carryall_Pickup
    iVar2 = (**(code **)(*(int *)param_1[0x19d] + 0x90))(...);  // locomotor.At_Destination
    if (iVar2 == 1) {
      // Get pad cell from building (vtable+0xA8) or locomotor (vtable+0x48)
      // Call self vtable+0x1B4 (Set_Location)
      ...
    }
    iVar2 = (**(code **)(*param_1 + 0x274))(0x15);   // Receive_Radio(PREPARE)
    if (iVar2 == 1) {
      // Carryall path or AddPassenger
      ...
      CargoClass__AddPassenger(piVar6);
      AircraftClass__Carryall_Pickup();
      ...
    } else if (iVar2 == 5) {  // NACK 5
      (**(code **)(*param_1 + 0xd4))();              // Stop_Driver
      CargoClass__AddPassenger(param_1);
    }
  }
  return 1;
}
```

---

## Appendix D — Verbatim FootClass::Mission_Enter Decomp (Full)

```c
int __fastcall FootClass__Mission_Enter(int *param_1)
{
  iVar4 = FootClass__GetDestination(0);
  if ((iVar4 == 0) && (iVar4 = Filter_AbstractType_InMap(), iVar4 == 0)) {
    // No destination
    cVar3 = FUN_0070d8f0();
    if ((cVar3 == '\\0') &&
       (((int *)param_1[0x169] == (int *)0x0 ||
        ((iVar4 = (**(code **)(*(int *)param_1[0x169] + 0x2c))(), iVar4 != 1 &&
         (iVar4 = (**(code **)(*(int *)param_1[0x169] + 0x2c))(), iVar4 != 2)))))) {
      (**(code **)(*param_1 + 0x484))(0,1);   // Set_NavCom(0,force)
    }
    (**(code **)(*param_1 + 0x1ec))();          // Stop_Driver
  }
  else {
    iVar4 = (**(code **)(*param_1 + 0x278))(0xe,iVar4);   // Transmit_Radio(CAN_DOCK=0xE, dest)
    if ((iVar4 == 1) || ((char)param_1[0x106] != '\\0')) {
      // ROGER 1, or Force-Enter flag at +0x418
      if ((param_1[0x169] == 0) && (0 < param_1[0x166])) {
        // Multi-stop: pop next waypoint
        ... locomotor piggyback release ...
        ... memmove array[0..count] down ...
        param_1[0x166]--;
      }
      else {
        // Single-stop, ROGER
        iVar4 = (**(code **)(*param_1 + 0x84))();    // GetType
        if (*(char *)(iVar4 + 0xcd4) != '\\0') {     // Type->Consumed flag
          iVar4 = param_1[0x169];
          param_1[0x168] = 0;                          // clear DockTarget
          param_1[0x169] = 0;                          // clear RadioContact
          (**(code **)(*param_1 + 0x480))(iVar4,1);    // Set_Destination(old_contact, force)
        }
      }
    }
    else {
      // NACK
      (**(code **)(*param_1 + 0x274))(3);            // Receive_Radio(CLEAR_LINK=3)
      (**(code **)(*param_1 + 0x484))(0,1);          // Set_NavCom(0,force)
    }
  }
  MissionClass__GetMissionTimerEntry();
  iVar4 = Math__ftol();
  iVar5 = Random__RandomRanged(0,2);
  return iVar5 + iVar4;   // 1-3 tick jitter delay
}
```

---

**End of report.**
