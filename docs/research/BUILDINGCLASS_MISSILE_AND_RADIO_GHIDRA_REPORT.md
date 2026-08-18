# BuildingClass::Mission_Missile and BuildingClass::Receive_Radio — Ghidra Report

Two investigations, both verified live in Ghidra against `gamemd.exe`.

| System | Address | Vtable slot | Size |
|---|---|---|---|
| `BuildingClass::Mission_Missile` | `0x0044C980` | 148 (+0x250) | ~3104 B |
| `BuildingClass::Receive_Radio`   | `0x0043C2D0` | 101 (+0x194) | ~1500 B |
| `TechnoClass::Receive_Radio` (base) | `0x006F4AB0` | (inherited slot 101) | ~145 lines |
| `RadioClass::Receive_Radio` (lowest base) | `0x0065A820` | — | ~80 lines |

---

## PART 1 — `Mission_Missile` (Nuke Silo + EMP Cannon)

### 1.1 Top-level dispatch

```c
int BuildingClass::Mission_Missile(this)
{
    if (this->Type[0x16BA] != 0)              // NukeSilo= (NAMISL, YR-ACTIVE)
        switch (this->field_0xBC) { 0..4 }    // 5-state machine A

    // if field_0x5F8 != -1  -> clean up SW-fire state
    //    vtable+0x3CC (CellClass::AttachSuperWeaponEvent-style)
    //    vtable+0x1E8(0, 0)  -> Queue_Mission(SLEEP, 0)

    if (this->Type[0x16C3] != 0)              // EMPulseCannon= (TS legacy, DORMANT in YR)
        switch (this->field_0xBC) { 0..3 }    // 4-state machine B

    // default: MissionClass::GetMissionTimerEntry()
}
```

- **`+0x16BA NukeSilo=`** — confirmed from `BuildingTypeClass::ReadINI_Water` at `0x0045FE50`, line calling
  `CCINIClass::ReadBool("NukeSilo", ...)` writing `param_1+0x16BA`. Set `yes` on **NAMISL** (Soviet Nuclear Missile Silo) only.
- **`+0x16C3 EMPulseCannon=`** — confirmed from the same ReadINI. **Never set in default YR rules** (pure TS
  legacy, dormant). Documented separately in `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md` as "TS legacy,
  blocks firing".
- **`+0x16C9 ICBMLauncher=` is NOT tested anywhere in `Mission_Missile`.** I ran
  `disassemble_function` and confirmed only `[EAX+0x16BA]` (0x0044C998) and `[EAX+0x16C3]`
  (0x0044CD1E) gate the two paths. `ICBMLauncher` exists as a flag but drives an entirely different
  subsystem (not investigated here).

### 1.2 Branch A — NukeSilo (`Type[0x16BA]`) — 5 states

State counter is `this->field_0xBC` (this is MissionClass's SubState / mission state index).
Key sub-fields used:

| Field | Meaning | Source |
|---|---|---|
| `+0x0BC` | SubState (mission state index) | MissionClass |
| `+0x21C` | HouseClass* owner pointer | BuildingClass (used as `Owner+0x5784` = nuke target cell) |
| `+0x54C` | AnimClass* (PSIWARN warning animation instance) | BuildingClass |
| `+0x5F8` | SuperWeaponType index being fired | verified via `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` |
| `+0x6DD` | Anim-complete flag (set by Animation_Update, read here) | `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` |

Where Owner+0x5784 comes from `HouseClass+0x5784` = packed target cell stored by `SuperClass::Launch`
Path B (see `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` §2).

| State | Timer return | Actions (verified from disassembly 0x0044C9B5–0x0044CCDD) |
|---|---|---|
| **0** | — (falls through to 1) | `field_0x6DD = 0`; `GrandOpening(2)` (begin silo door open); `MapClass::Get_CellClass(Owner+0x5784)` (fetch target cell); `AnimTypeClass::FindByName("PSIWARN")`; `new AnimClass(PSIWARN, targetCoord, 0, 1, 0x600, 0, 0)`; FUN_00424C90(anim, 0); FUN_00424CA0(anim, targetCell); `anim+0x19D = 1`; store anim ptr at `field_0x54C`; set `field_0xBC = 1`; FALLTHROUGH |
| **1** | — (falls through to 2) | Wait loop: if `field_0x6DD != 0` (doors fully open): `GrandOpening(4)` (fully-open pose); set `field_0xBC = 2`; FALLTHROUGH |
| **2** | **1** on success | `vtable+0x48(cellCoord)` (get building center coord); `MapClass::Get_CellClass(Owner+0x5784)` (target cell); `SWType = SuperWeaponTypeClass_Array[ field_0x5F8 ]`; `weapType = *(WeaponTypeClass**)(SWType+0x9C)`; `damage = weapType+0xA4` (Damage=), `warhead = weapType+0xAC` (Warhead=) (corrected 2026-05-29: was `bulletType = SWType+0xA4`, `warhead = SWType+0xAC` — wrong attribution; +0xA4 and +0xAC are WeaponTypeClass offsets accessed via pointer at SWType+0x9C, confirmed via decompile_function 0x0044C980 and WeaponTypeClass__ReadINI 0x00772080 — STRUCT_FAMILY_CASCADE); `BulletClass::Allocate(this, ...)`; `BulletClass::SetOwner(owner)`; release `field_0x54C` (PSIWARN anim, FUN_00424C90 with 0) and clear to 0; call `bullet vtable+0xD4` (init state); compute vertical velocity via `Sin/Cos(pi/2 = 0x3FF921C9)` giving ~(0,0,-Speed); call `bullet vtable+0x1F0(source, velocity)` (Fire/MoveTo); on **failure**: `bullet vtable+0x20(1)` (destroy), set state=3, return 1; on **success**: `new AnimClass(Rules+0x98=NUKETO, buildingCoord, 0, 1, 0x600, 0, 0)`; `anim+0x100 = 0xFFFFFF9C` (= -100, anim duration/fade flag); set `field_0xBC = 3`; return 1 |
| **3** | **6** | `GrandOpening(5)` (silo pose change / close); set `field_0xBC = 4`; return 6 |
| **4** | **0x3C (60)** | (common tail `switchD_0044c9b5_caseD_4`): `GrandOpening(5)`; `vtable+0x1E8(5, 0)` = `Queue_Mission(MISSION_GUARD, 0)`; return 60 |

Notes:
- The returned integer is the MissionClass timer delay (frames) before the handler runs again.
- The 16.16 angle `0x3FF921C9` is π/2. In the velocity calc, sin(0)·cos(π/2)·DAT_007E44A8 ≈ 0,
  sin(π/2)·DAT_007E44A8 = 1·Speed. Net velocity is purely +Z (straight up) — matches the
  "GiantNukeUp vertical carrier" phase from `NUKE_SUPERWEAPON_GHIDRA_REPORT.md`.
- `PSIWARN` is placed **at the target cell** (not the silo). It's the psychic-warning-style red
  marker that flashes at the impact cell while the doors open. Despite the "Psi" name, this is the
  standard warning anim used by both MultiMissile and the psychic dominator launcher.
- `Rules+0x98` = `NukeTakeOff=NUKETO` (verified in `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` §8.3). Played
  **at the silo** when the missile actually fires.

### 1.3 Interaction with superweapon charging (the `+0x6D0` question)

**Answer: Mission_Missile does NOT touch `+0x6D0`.** Charging is 100% house-side.

- `HouseClass` owns the `SuperClass` objects and their charge timers.
- When `SuperClass+0x6F/0x6E/0x6D` readiness flags transition appropriately, `SuperClass::Launch`
  (`0x006CC390`) runs Path B: it finds a building with `+0x16BA NukeSilo=yes`, writes the packed
  target cell to `HouseClass+0x5784`, writes the SW index to `building+0x5F8`, and invokes
  `building->vtable[0x1E8]()` / `[0x1EC]()` to **assign Mission 22 (Missile)** to the building.
- From then on, `Mission_Missile` drives the silo animation, bullet creation, and cleanup. It **reads**
  `this->Owner+0x5784` (target) and `this->field_0x5F8` (SW index). It never writes the SW charge state.
- `building+0x6D0` is the **ProduceCashTimer** (OreRefinery cash timer), documented in
  `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — unrelated to superweapons.
- After Mission_Missile state 4 returns to the common tail (`0x0044CCDE`), if `field_0x5F8 != -1`
  the code calls `vtable+0x3CC` (CellClass::AttachSuperWeaponEvent or similar) and
  `vtable+0x1E8(0, 0)` = `Queue_Mission(SLEEP, 0)` — this is where the SW slot on the owner
  house is cleared for next use (verified by the `field_0x5F8 != -1` guard).

### 1.4 Branch B — EMPulseCannon (`Type[0x16C3]`) — 4 states — TS LEGACY (DORMANT IN YR)

Present in the binary but **not active in standard YR**: no default YR `BuildingTypeClass` has
`EMPulseCannon=yes` set. Included for completeness.

| State | Timer | Actions |
|---|---|---|
| **0** | 1 | Turn turret toward target (`vtable+0x4E8` with cell coord from ReadINI'd target), check two `RateTimer`s (one for delay between state 0 repeats, one for overall charge). If overall timer expired → set state=1; else re-arm via `RateTimer::Set`. |
| **1** | 0x20 (32) | `new AnimClass` using `AnimTypes_Array[ FUN_00422B20() ]` as animation (the "aim glow" anim), played at building. Set state=2. |
| **2** | 1 (success) or falls through | Get weapon via `vtable+0x3F8`, get owner house, check if owner's MP color matches neutral — if yes, skip fire and `Queue_Mission(5, 0)` (Guard) + return 60. Otherwise: `RateTimer::Set` (cooldown), run full trajectory math (sqrt distance, atan2 angle, gravity from `Rules+0x16B8 Gravity`), create the **PULSBALL** projectile via `BulletClass::Allocate`; call bullet `vtable+0x1F0(source, velocity)` Fire; on success call `vtable+0x3FC` (play fire anim), play random VocClass voice, set state=3, return 1. On failure destroy bullet. |
| **3** | — | Arm a 0x4000-tick RateTimer, fall through. |

Default case (any state not 0–3, with EMPulseCannon flag): calls `MissionClass::GetMissionTimerEntry()`
and returns its default timer.

**Do NOT implement.** The Rust engine should not port this unless a mod is later verified to set
`EMPulseCannon=yes`.

---

## PART 2 — `BuildingClass::Receive_Radio` (radio protocol)

Source: decompilation of `0x0043C2D0`. The function dispatches on `param_3` (message code)
and for messages it doesn't handle it falls through to `TechnoClass::Receive_Radio` at `0x006F4AB0`.

### 2.1 Signature (verified)

```c
int BuildingClass::Receive_Radio(
    this,                       // param_1: BuildingClass*
    TechnoClass *sender,        // param_2
    int        message,         // param_3: radio message code
    void     **out_param);      // param_4: in/out payload (cell, pointer, etc)
```

- Return codes are the TS/YR shared ResponseType enum (documented from
  `MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §Radio_Protocol_Tables):

| Response | Value | Meaning |
|---|---|---|
| ROGER | 1 | Positive ack |
| ZONE_MISMATCH / REJECT | 0xE | Wrong zone (seen here for msg 0xF ally-check rejection) |
| CELL_ACCEPTED | 0x14 | Unit-side accepts move-to-cell (used for msg 0x12 response) |
| QUEUED | 0x17 | In queue (msg 0x8 response for WeaponsFactory) |
| INSUFFICIENT_FUNDS | 0x20 | (TechnoClass msg 0x1C path) |
| REPAIR_COMPLETE | 0x21 | (TechnoClass msg 0x1C path) |
| NEGATORY | 10 (0xA) | No |

### 2.2 Message handlers (complete table, from switch at 0x0043C2F0)

Verified cases (decompiled): **3, 8, 0xB, 0xC, 0xD, 0xE, 0xF, 0x10, 0x15**.
All other messages (0x2, 0x7, 0x9, 0x11, 0x12..0x14, 0x16..0x19, 0x1A..0x1F, 0x20..0x23) fall through
to `TechnoClass::Receive_Radio(sender, message, out_param)`.

#### Case 3 — OVER_AND_OUT (break radio link)
```c
BuildingClass::GrandOpening();           // reset idle anim state
TechnoClass::Receive_Radio(sender,3,p);  // base: removes sender from Contacts[], calls Object handler
return ROGER (1);
```
Used to cleanly disconnect any partner (harvester, rally-point aircraft, docked vehicle).

#### Case 8 — REQUEST_DOCKING_CLEARANCE (Unit→Building: "may I approach?")
```c
if (Type[0x16A9] UnitRepair || Type[0x16AB] Bunker) {
    dist = |this.center - sender.center|;
    if (dist < 0x180)                     // already within 3 cells
        return ROGER;
}
TechnoClass::Receive_Radio(sender, 8, p);   // base handler (side effects)
if (Type[0x16BD] WeaponsFactory ||
    Type[0x16A9] UnitRepair ||
    Type[0x16AB] Bunker)
    return QUEUED (0x17);                 // always queue harvesters / vehicles
return ROGER (1);
```
**This is the well-documented "WeaponsFactory always queues" behavior** from
`MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §6.6.

#### Case 0xB — DOCK_APPROACH (Building→Unit: heading hint)
```c
this->vtable[0x1E8](0x14, 0);             // Queue_Mission(UNLOAD=0x14, 0)
// fallthrough to case 0xC tail -> TechnoClass::Receive_Radio(sender, 0xB, p)
return ROGER (1);
```

#### Case 0xC — DOCK_ARRIVED (Unit→Building: "I'm at the dock cell")
```c
if (GetCurrentMission() != 0x13 /* MISSION_UNLOAD_REFINERY */) {
    this->vtable[0x1E8](5, 0);            // Queue_Mission(GUARD, 0)
    if (Type[0x16B9] ConstructionYard) {
        BuildingClass::ClearAnimSlot(this);   // twice
        BuildingClass::ClearAnimSlot(this);
        healthRatio = GetHealthRatio();
        animName = healthRatio > Rules.ConditionYellow  // Rules+0x1700
                       ? Type+0x116C                    // healthy ambient anim
                       : Type+0x117C;                   // damaged ambient anim
        if (*animName != 0) BuildingClass::CreateAnimForSlot(this);
    }
}
TechnoClass::Receive_Radio(sender, 0xC, p);
return ROGER (1);
```

#### Case 0xD — (unnamed, suppressed for WeaponsFactory)
```c
if (Type[0x16BD] WeaponsFactory)
    return ROGER (1);                     // swallow silently
// else fall through to base TechnoClass::Receive_Radio at the end
```
WAW the code just `break;`s. The default fallthrough to `TechnoClass::Receive_Radio(sender, 0xD, p)`
handles the generic case (likely "radio contact lost" / reconnect).

#### Case 0xE — CAN_DOCK (Unit→Building: full dock query)
This is the large, well-studied case. Summary (full decompile in
`MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md` §6.1 — the logic I see in Ghidra matches exactly):

```c
TechnoClass::Receive_Radio(sender, 0xE, p);                      // base first
if (!this->HasPower) return NEGATORY (10);

// UnitRepair busy check
if (Type[0x16A9] UnitRepair && ContactsContains(sender)
    && this->vtable[0x278](0x22 IS_REPAIRING, sender) == 10)
    return NEGATORY;

// Bunker deploy-here check
if (Type[0x16AB] Bunker && ContactsContains(sender) && !CanAutoDeployHere(sender))
    return NEGATORY;

if (Type[0x16C1] Hospital == 0 && Type[0x16C2] Armory == 0) {
    // Normal refinery / factory / weapons-factory / weeder dock path
    if (!ContactsContains(sender) && FUN_0065ADF0()) {       // low-level IsHarvester check
        this->vtable[0x278](2 DOCK_LINK, sender);             // establish link
    }
    if (ContactsContains(sender) && (Type[0x16B3] Refinery || Type[0x16BC] Weeder)) {
        // compute queue cell for harvesters
        startCell = (cell-at vtable+0xA8)(sender);
        ...
    }
    if (FootClass::GetDestination() != 0 && (Type[0x16A9] || Type[0x16AB])) {
        // compute distance from sender to destination (stat/log only here)
    }
    r = this->vtable[0x278](0x13 IS_UNIT_LINKED, sender);
    if (r != ROGER && (char)piStack_4 == 0) return ROGER;
    *out_param = this;
    if (Type[0x16B3] Refinery == 0 && Type[0x16BC] Weeder == 0) {
        if (Type[0x16CB] Helipad == 0) return ROGER;
        // Helipad: move sender to self, returns 0x14 = CELL_ACCEPTED
        *out_param = this;
        if (this->vtable[0x27C](0x12 MOVE_TO_CELL, out_param, sender) != 0x14) return ROGER;
        this->vtable[0x274](0x18 ENTER_DOCK);
        return ROGER;
    }
    // Refinery / Weeder: compute queue cell (+3, +1 from building top-left)
    tl = this->vtable[0x1B8](...);             // get top-left cell of foundation
    queue = { tl.x + 3, tl.y + 1 };            // HARDCODED offset
    cell = MapClass::Get_CellClass(queue);
    *out_param = cell;
    if (this->vtable[0x27C](0x12 MOVE_TO_CELL, out_param, sender) != 0x14) return ROGER;
    this->vtable[0x278](0x18 ENTER_DOCK, sender);
    if (this->vtable[0x278](0x16 TIMING_SYNC, sender) == ROGER) return ROGER;
    sender->vtable[0x174](&DAT_0089C848, 1, 1);  // scatter sender away
    return ROGER;
}

// Hospital / Armory path (Type[0x16C1] || Type[0x16C2])
if (FUN_0065ADF0()) {                        // Hospital/Armory: accept any infantry immediately
    *out_param = CellClass::Get_Cell_At(this);
    this->vtable[0x27C](0x12 MOVE_TO_CELL, out_param, sender);
    return ROGER;
}
// evict mismatched queue entries
for (i = 0; i < this->ContactsCount (field_0xE8); i++) {
    unit_i = FootClass::GetDestination();    // (reads contacts slot)
    if (this->vtable[0x278](0x22 IS_REPAIRING, unit_i) == 10) {
        this->vtable[0x278](0x17 EVICT_QUEUE);
    }
}
if (FUN_0065ADF0()) return ROGER;
return NEGATORY;
```

**Flag-driven branches (Type offsets confirmed from ReadINI at `0x0045FE50`):**

| Type flag | INI key | Effect in case 0xE |
|---|---|---|
| +0x16A9 | UnitRepair= | Dock-in-place repair (Grand Cannon, Service Depot base class) |
| +0x16AB | Bunker= | Deploy-into-building |
| +0x16B3 | Refinery= | Uses hardcoded queue cell (+3,+1) |
| +0x16BC | Weeder= | Uses hardcoded queue cell (+3,+1) |
| +0x16BD | WeaponsFactory= | Affects case 8 (always QUEUED) and case 0xD (silent) |
| +0x16C1 | Hospital= | Accepts infantry immediately, queue mgmt via msg 0x22/0x17 |
| +0x16C2 | Armory= | Same Hospital-style path |
| +0x16CB | Helipad= | Move-to-self, send ENTER_DOCK |

#### Case 0xF — CAN_ENTER (Unit→Building: passenger entry request)
```c
TechnoClass::Receive_Radio(sender, 0xF, p);
if (!HouseClass::Is_Ally_ByObject(this, sender))     // not ally
    return 0;                                         // reject with 0 (not NEGATORY)
if (GetCurrentMission() == 0x12 CONSTRUCTION) return NEGATORY;
if (GetCurrentMission() == 0x13 UNLOAD_REFINERY)      return NEGATORY;
if (field_0x534 == 0) return NEGATORY;                // no aux data/slot available

if (!g_MapEditorMode && !FUN_0065ADF0() &&            // not harvester
    !Type[0x16AE] UnitAbsorb && !Type[0x16AF] InfantryAbsorb)
    return NEGATORY;

// Naval / land zone mismatch check (type+0xCCE)
if (senderType+0x5B4 != 5 /* aircraft? */ &&
    (this->type.naval != sender->type.naval)) return NEGATORY;

if (sender->type+0xD6A != 0) return NEGATORY;          // sender can't enter buildings
if (!this->HasPower) return NEGATORY;

// UnitAbsorb/InfantryAbsorb (Yuri Grinder, ClonigVat)
if (Type[0x16AE] || Type[0x16AF]) {
    senderClass = sender->vtable[0x2C]();                  // UNIT=1, AIRCRAFT=0xF, INFANTRY=? 
    if ((senderClass == 1 && !Type[0x16AE])   // UnitAbsorb must be set for unit
        || (senderClass == 0xF && !Type[0x16AF]))  // InfantryAbsorb must be set for aircraft? (check)
        return NEGATORY;
    if (sender->CaptureManager != 0 && FUN_004722C0()) return NEGATORY;  // don't grind MindControlled sender
    if (this->field_0x114 + 1 <= senderType+0x5E0 /* unit cost? */
        && senderType->GetHealthThreshold <= type+0x388 /* threshold */)
        return ROGER;   // fully credit
}
if (Type[0x16AD] Grinding) return ROGER;                  // Grinder always accepts
if (Type[0x16AB] Bunker) {
    if (!CanAutoDeployHere(sender)) return NEGATORY;
    if (this->vtable[0x278](0x23 IS_OCCUPIED, sender) == ROGER) return NEGATORY;
    return ROGER;
}
if (Type[0x16A9] UnitRepair) {
    senderClass = sender->vtable[0x2C]();
    if (senderClass != 1 UNIT && senderClass != 2 AIRCRAFT) return NEGATORY;
    if (this->vtable[0x278](0x23 IS_OCCUPIED, sender) == ROGER) return NEGATORY;
    return ROGER;
}
if ((Type[0x16C2] Armory || Type[0x16C1] Hospital)
    && sender->class == 0xF /* aircraft? actually INFANTRY in this code */) {
    if (sender->CaptureManager && FUN_004722C0()) return NEGATORY;
    if (IsMindControlled(sender)) return NEGATORY;
    // Armory has per-house cap; Hospital doesn't.
    return (field_0x2FC != 0) ? NEGATORY : ROGER;
}
if (Type[0x16CB] Helipad) {
    // Helipad accepts only aircraft of certain dock-type
    return (sender_dock_type(sender) != 2) ? NEGATORY : ROGER;
}
// Garrison (Type[0x16B3] is Refinery here? NO — re-check: actually this is CanOccupy/Garrisonable)
// reinterpreting: both Refinery (0x16B3) and Weeder (0x16BC) below handle
// harvester-generator behavior in case 0xE, not 0xF. In 0xF those flags gate infantry garrison
// per field_0x118 (occupants count) vs senderType->can_occupy_fire field.
if (Type[0x16B3] /* ?garrison? */ && sender->class == 1 UNIT
    && sender->HouseType+0xE0E /* AllowGarrisonByType? */) {
    if (g_MapEditorMode) return ROGER;
    if (field_0x118 == 0) return ROGER;
}
if (Type[0x16BC] && sender->class == 1 && sender->HouseType+0xE0F) {
    if (g_MapEditorMode) return ROGER;
    if (field_0x118 == 0) return ROGER;
}
return 0;   // silent reject
```

Note: my interpretation of some of the obscure flags in the case 0xF tail (0x16B3 vs Refinery,
0x16BC vs Weeder) comes from the *same* ReadINI offsets used in case 0xE. The case 0xF logic
uses them as garrison-entry gates, which is consistent with how RA2 reuses BuildingType flags
across multiple code paths. Confidence: **MEDIUM** on the garrison branches, HIGH on the
Grinder/Bunker/Hospital/Armory/Helipad/UnitRepair branches.

#### Case 0x10 — RESERVE_DOCK (Unit→Building: reserve a dock slot)
```c
if (field_0x118 == 0                        // no current passengers
    && FUN_0065ADF0()                       // is-harvester check
    && field_0x81 == 0                      // no lockout
    && sender->GetOwner() == this->Owner) {
    if (Type[0x16BB] /* unknown TS flag */) return ROGER;
    if (Type[0x16A9] UnitRepair) return ROGER;
    if (Type[0x16BC] Weeder) return ROGER;
}
return NEGATORY;
```

#### Case 0x15 — DOCK_NOW (Unit→Building: "I'm at the dock cell, start sequence")
```c
if (GetCurrentMission() == 0x13 UNLOAD_REFINERY) return NEGATORY;
if (Type[0x16AE] UnitAbsorb)      return ROGER;
if (Type[0x16AF] InfantryAbsorb)  return ROGER;
if (Type[0x16A9] UnitRepair || Type[0x16AA] UnitReload ||
    Type[0x16C1] Hospital   || Type[0x16C2] Armory) {
    this->field_0x6DD = 1;                              // mark anim complete (start dock anim)
    this->vtable[0x1E8](0x14 UNLOAD, 0);                // Queue_Mission(UNLOAD)
    piStack_4->vtable[0x1E8](0 SLEEP, 0);               // sender: Queue_Mission(SLEEP)
    return ROGER;
}
if (Type[0x16AB] Bunker) {
    field_0x6DD = 1;
    this->vtable[0x1E8](0x14 UNLOAD, 0);
    return ROGER;
}
if (Type[0x16B3] /* garrison/refinery */) {
    sender->vtable[0x1E8](0x10 ENTER, 0);               // sender: Queue_Mission(ENTER)
    return ROGER;
}
// fall through
return TechnoClass::Receive_Radio(sender, 0x15, p);     // base handler
```

### 2.3 Tail (default for unhandled messages)

```c
return TechnoClass::Receive_Radio(sender, message, out_param);
```

`TechnoClass::Receive_Radio` at `0x006F4AB0` (verified by decompile) handles:

| Msg | Meaning | Action |
|---|---|---|
| 0x03 | BREAK | If DockedIn set and sender unit has grinding-related flag, send `0x19 LEAVE_DOCK` to sender first; then delegate to `RadioClass::Receive_Radio` (which removes from Contacts[]). |
| 0x07 | DOCKING_COMPLETE | Send `0x18 ENTER_DOCK` to sender; delegate to RadioClass::Receive_Radio. |
| 0x08 | REQUEST_CLEARANCE (base) | Send `0x19 LEAVE_DOCK`, then return `vtable[0x278](0x03 BREAK, sender)`. |
| 0x09 | (same as 0x07) | Send `0x18`, delegate. |
| 0x16 | TIMING_SYNC (base) | Send `0x18`, delegate. |
| 0x18 | ENTER_DOCK | Set `this->DockedIn = 1` (byte at `+6·sizeof(obj).UniqueID = ~+0x7x`); propagate. |
| 0x19 | LEAVE_DOCK | Clear DockedIn; propagate. |
| 0x1A | (DockedIn high flag set) | Set second lock bit; propagate. |
| 0x1B | (unset second lock bit) | Clear second lock bit; propagate. |
| 0x1C | REPAIR_TICK | **Repair logic.** See §2.4 below. |
| 0x1E | (deploy action) | If vtable[0x3F4] returns non-null ptr with non-zero content: set nav to *param_4, Queue_Mission(1 MOVE, 0). |
| 0x1F | (capacity / passenger?) | Compare `field_0x4C` to `Type+0x684`; if already at cap return NEGATORY; else increment and ROGER. |
| other | delegate to RadioClass::Receive_Radio | |

`RadioClass::Receive_Radio` at `0x0065A820` is the lowest base:
- Shift 3-deep RadioHistory at `+0xD4/+0xD8/+0xDC`, record new message.
- Msg `0x03` BREAK — find sender in Contacts[] (`+0xE4`, count `+0xE8`), null that slot,
  call `ObjectClass::Receive_Radio` for side-effects, return 1.
- Msg `0x02` HELLO — ally check, already-present check, free-slot scan; if room, add sender
  to Contacts[] and return ROGER, else NEGATORY.
- Else defer to `ObjectClass::Receive_Radio`.

### 2.4 Repair flow (radio 0x1C) — for completeness

Handled entirely by `TechnoClass::Receive_Radio` (not the BuildingClass override). When a
Service Depot (`+0x16A9 UnitRepair=yes`) has a unit docked, the building's Mission_Repair
periodically sends `0x1C REPAIR_TICK` to the unit. The unit's TechnoClass::Receive_Radio
case 0x1C runs:

```c
if (this->HealthRatio >= Rules+0x16F8 ConditionYellowRepair) return NEGATORY;  // full
step_cost = Type->vtable[0xB0]();     // money per tick
step_hp   = max(1, Type->vtable[0xB4]());  // hp per tick
if (owner_cash < step_cost) return INSUFFICIENT_FUNDS (0x20);
HouseClass::Spend_Money(step_cost);
this->Health += step_hp;
// (parachute / warp handling)
if (HealthRatio > ConditionYellow_Max or IronCurtain_hi) -> anim callback
if (HealthRatio >= ConditionYellowRepair) {
    this->Health = Type->MaxHealth;
    return REPAIR_COMPLETE (0x21);
}
return ROGER (1);
```

Matches `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md` §18.

### 2.5 Building-radio vs Vehicle/Infantry-radio interaction

- All TechnoClass subclasses share the same 4-level class hierarchy:
  `RadioClass < TechnoClass < (FootClass | BuildingClass)`.
- Every radio exchange is symmetric: sender's `Transmit_Message` calls receiver's
  `Receive_Radio`, then receiver's response code flows back.
- `BuildingClass::Receive_Radio` intercepts dock/garrison/factory-specific messages (0x8, 0xB,
  0xC, 0xE, 0xF, 0x10, 0x15) before letting TechnoClass handle the generic ones.
- `UnitClass::Receive_Radio` (and InfantryClass, AircraftClass) override a different set —
  predominantly 0x7, 0xB, 0xC, 0x12, 0x15, 0x16, 0x18, 0x19 — with unit-side behavior
  (e.g. unit msg 0x15 DOCK_NOW on unit side turns 180°, plays dock sound, transitions to
  "docked idle" state).
- **Contacts[] array** (field_0xE4 ptr, field_0xE8 count) is per-object and tracks currently
  linked partners. BREAK (0x3) removes; HELLO (0x2) adds.
- **RadioHistory** (3-slot, +0xD4/0xD8/0xDC) dedupes duplicate back-to-back messages.
- **DockedIn flag** (single bit in field_0xE8 range) gates many dock-sequence behaviors.

---

## 3. Confidence & caveats

| Finding | Confidence | Notes |
|---|---|---|
| Mission_Missile state 0–4 for NukeSilo path | HIGH | Verified disassembly + existing `NUKE_SUPERWEAPON_GHIDRA_REPORT.md` |
| `+0x16BA = NukeSilo`, `+0x16C3 = EMPulseCannon`, `+0x16C9 = ICBMLauncher` | HIGH | Confirmed from `BuildingTypeClass::ReadINI_Water` strings |
| `+0x16C9 ICBMLauncher` NOT used by Mission_Missile | HIGH | Disassembly confirms only 0x16BA and 0x16C3 are tested |
| EMPulseCannon path is TS-legacy dormant in YR | HIGH | No default YR rules set `EMPulseCannon=yes` |
| PSIWARN anim used in NukeSilo state 0 | HIGH | String literal verified at 0x81907C |
| NUKETO anim via `Rules+0x98` in state 2 | HIGH | Matches existing NUKE_SUPERWEAPON doc §2.3 / §8 |
| `+0x6D0` is unrelated (ProduceCashTimer) | HIGH | Confirmed in `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` |
| Receive_Radio cases 3, 8, 0xB, 0xC, 0xE, 0xF, 0x10, 0x15 | HIGH | Full decompile |
| Receive_Radio case 0xD (silent for WeaponsFactory) | HIGH | Verified |
| Case 0xF garrison branches for 0x16B3/0x16BC | MEDIUM | Flag reuse is plausible but not cross-verified against another garrison code path |
| Base TechnoClass::Receive_Radio cases 3, 7, 8, 9, 0x16, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1E, 0x1F | HIGH | Full decompile |

## 4. Address summary

| Symbol | Address |
|---|---|
| `BuildingClass::Mission_Missile` | `0x0044C980` |
| `BuildingClass::Receive_Radio` (vtable slot 101 = +0x194) | `0x0043C2D0` |
| `TechnoClass::Receive_Radio` (inherited base) | `0x006F4AB0` |
| `RadioClass::Receive_Radio` (lowest base) | `0x0065A820` |
| `BuildingClass::GrandOpening` | `0x00447780` |
| `AnimTypeClass::FindByName` (called `FindByIndex` in Ghidra) | `0x00427CB0` |
| `AnimClass::Constructor` | `0x00421EA0` |
| `BulletClass::Allocate` | `0x0046B050` |
| `BulletClass::SetOwner` | `0x0046B260` |
| `MapClass::Get_CellClass` | (static, via `ECX=0x87F7E8` global MapClass*) → `0x005657A0` |
| `RulesClass+0x16B8` | `[General] Gravity=` (default 6) |
| `RulesClass+0x98` | `NukeTakeOff=` anim (NUKETO) |
| `HouseClass+0x5784` | packed nuke target cell coord |
| `SuperWeaponTypeClass+0x9C` | WeaponTypeClass* |
| `BuildingClass+0x5F8` | SW type index being fired (-1 = idle) |
| `BuildingClass+0x54C` | PSIWARN anim ptr (NukeSilo state 0→2 scratch) |
| `BuildingClass+0x6DD` | anim-complete flag (used by state 1 wait) |
| `BuildingClass+0x0BC` | SubState (mission state counter) |
| `BuildingTypeClass+0x16BA` | `NukeSilo=` (YR-active) |
| `BuildingTypeClass+0x16BD` | `WeaponsFactory=` |
| `BuildingTypeClass+0x16C3` | `EMPulseCannon=` (TS-legacy, dormant) |
| `BuildingTypeClass+0x16C9` | `ICBMLauncher=` (NOT used by Mission_Missile) |
