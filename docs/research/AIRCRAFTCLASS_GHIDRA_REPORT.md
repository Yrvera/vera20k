# AircraftClass — Ghidra Research Report

**Primary Address Range:** `0x00413D20`–`0x0041CC20` (AircraftClass + AircraftTypeClass)
**VTable:** `0x007E22A4` (primary), 4 secondary vtables
**Confidence:** High (decompiled from binary, cross-referenced with INI + docs)
**Active in YR:** Yes — all findings are live in standard YR skirmish

## 1. Overview

AircraftClass is the runtime instance class for all airborne units in gamemd.exe — fighters
(Harrier, Black Eagle), transports (Nighthawk, Flak Track helicopter), airships (Kirov),
spawned sub-units (Hornet, Osprey, V3 Rocket, Dreadnought Missile), and paradrop cargo planes.

It inherits from FootClass (→ TechnoClass → RadioClass → MissionClass → ObjectClass → AbstractClass)
and adds a small number of aircraft-specific fields (only ~24 bytes beyond FootClass). The bulk
of aircraft behavior is implemented through vtable overrides of mission handlers, AI, drawing,
radio protocol, and idle-mode logic.

**Class hierarchy:**
```
AbstractClass
  └── ObjectClass
        └── MissionClass
              └── RadioClass
                    └── TechnoClass
                          └── FootClass (~0x6C0 bytes)
                                └── AircraftClass (0x6D8 = 1752 bytes)
```

## 2. AircraftClass Instance Layout

**Total size:** 0x6D8 (1752 bytes)
**Base class (FootClass) ends at:** ~0x6C0

### AircraftClass-Specific Fields (offsets from `this`)

| Offset | Type | Name | Initial | Description |
|--------|------|------|---------|-------------|
| 0x6C0 | void* | SecondaryVTable | vtable ptr | Secondary vtable (IPersistStream COM interface) |
| 0x6C4 | AircraftTypeClass* | Type | from ctor | Pointer to the type class (rules.ini definition) |
| 0x6C8 | bool | HasFired | false | Set to true when weapon fires during attack run; cleared on RTB. Used to decrement ammo exactly once per attack pass. |
| 0x6C9 | bool | IsFromFactory | false | Set to 1 in Unlimbo when the aircraft has an owner building (was produced, not spawned/paradropped). Used by some movement checks. |
| 0x6CA | bool | Unknown_6CA | false | Unknown flag, only cleared in constructor |
| 0x6CC | Building* | CachedDock | nullptr | Last-known helipad/airfield for this aircraft. Cached to speed up FindBuildingToDock — revalidated each call via radio check. Cleared in Detach() when the building is destroyed. |
| 0x6D0 | bool | Unknown_6D0 | false | Unknown flag |
| 0x6D1 | bool | Unknown_6D1 | false | Unknown flag |
| 0x6D2 | bool | IsStrafe | false | Set during strafing attack runs (states 6–9 in Mission_Attack). Controls whether the aircraft continues forward after firing. Checked in AI() for map-boundary kill. |
| 0x6D3 | byte | PayloadCount | 5 | Paradrop retry counter. Initialized to 5 in constructor. Decremented in Mission_Open on each drop approach; reset to 5 on successful drop in Drop_Payload. When it reaches 0, aircraft stops dropping and exits (see §26, §32). Verified: `decompile_function 0x004158E0` decrements `*(char *)((int)param_1 + 0x6d3)`. |
| 0x6D4 | bool | WantsToLand | true | Whether aircraft should descend to land. Set true initially and when RTB. Cleared (false) in Mission_Sticky state 7 when locomotor reports Is_On_Floor. |
| 0x6D5 | bool | IsIdling | true | Set after Enter_Idle_Mode completes or after RTB finishes. Cleared when aircraft starts an attack pass. |

### Key Inherited Fields Used by AircraftClass

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| 0x0AC | int | MissionState | `param_1[0x2B]` — Current mission enum |
| 0x0BC | int | SubState | `param_1[0x2F]` — State within current mission's state machine |
| 0x2B4 | AbstractClass* | TarCom | `param_1[0xAD]` — Target combatant |
| 0x2FC | int | Ammo | `param_1[0xBF]` — Current ammo count |
| 0x5A4 | AbstractClass* | NavCom | `param_1[0x169]` — Navigation destination |
| 0x5D4 | TeamClass* | Team | `param_1[0x175]` — Team membership |
| 0x674 | ILocomotion* | Locomotor | `param_1[0x19D]` — COM locomotion interface |

## 3. AircraftTypeClass Layout

**Constructor:** `0x0041C8B0`
**ReadINI:** `0x0041CC20`
**VTable:** Uses separate vtable from AircraftClass

### AircraftTypeClass-Specific Fields (from ReadINI)

| Offset | Type | INI Key | Section | Default | Description |
|--------|------|---------|---------|---------|-------------|
| 0xDFC | bool | Carryall | Rules | false | Can pick up and transport other units |
| 0xE00 | AnimTypeClass* | Trailer | Art | nullptr | Trail animation played behind aircraft |
| 0xE04 | int | SpawnDelay | Art | 3 | Delay in frames between Trailer spawns |
| 0xE08 | bool | Rotors | Art | false | Has rotor blades (drawn as overlays) |
| 0xE09 | bool | CustomRotor | Art | false | Uses custom rotor animation |
| 0xE0A | bool | Landable | Rules | false | Can land on the ground |
| 0xE0B | bool | FlyBy | Rules | false | Fly-by behavior (approach, fire, keep going) |
| 0xE0C | bool | FlyBack | Rules | false | After fly-by, reverse course |
| 0xE0D | bool | AirportBound | Rules | false | Must dock at helipad/airfield; crashes if none available |
| 0xE0E | bool | Fighter | Rules | false | Fighter aircraft — affects targeting behavior |

### Key Inherited TypeClass Fields Used

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| 0x684 | int | Ammo | Max ammo count (-1 = unlimited) |
| 0x680 | int | InitialAmmo | Starting ammo override (-1 = use Ammo) |
| 0x5E0 | int | Passengers | Max passenger count (for Carryall/transports) |
| 0xD68 | bool | BalloonHover | Stays airborne permanently (Kirov) |
| 0xC95 | bool | ConsideredAircraft | Game treats this as aircraft for targeting/tab purposes |

## 4. VTable — Key Overridden Methods

All offsets relative to vtable base at `0x007E22A4`.

### Core Lifecycle

| VTable Offset | Address | Name | Size | Purpose |
|---------------|---------|------|------|---------|
| +0x000 | 0x00414290 | Destructor | 95B | Cleanup and remove from global arrays |
| +0x024 | 0x00413F80 | InitFromType | — | Initialize fields from AircraftTypeClass: ammo, veterancy, facing, speed |
| +0x05C | 0x00414BB0 | **AI** | 1544B | Per-tick update: trail anims, crash handling, damage smoke, map bounds check, Carryall logic |
| +0x114 | 0x004144B0 | **Draw_It** | 1052B | Voxel rendering with locomotor matrix, shadow, rotor overlay, altitude offset |
| +0x484 | 0x004176F0 | **Enter_Idle_Mode** | 1210B | Decides next mission after completing current one (RTB, guard, attack, hunt) |

### Mission Handlers (called via MissionClass::Mission_Dispatch)

| VTable Offset | Mission ID | Address | Name | Size | Description |
|---------------|-----------|---------|------|------|-------------|
| +0x210 | 1 (Attack) | 0x00417FE0 | **Mission_Attack** | 3445B | 11-state attack state machine — approach, fire, strafe, RTB |
| +0x21C | 5/6 (Guard) | 0x0041A5C0 | **Mission_Guard** | 886B | Idle/guard: RTB when low ammo, hunt if AI, delegate to FootClass |
| +0x228 | 15 (Ambush) | 0x00414A80 | Mission_Ambush | — | Ambush behavior (brief) |
| +0x22C | 2 (Move) | 0x004166C0 | **Mission_Move** | 1031B | 5-state move: special Carryall path or standard fly-to-destination |
| +0x230 | 4 (QMove) | 0x00415A50 | Mission_QMove | — | Quick move variant |
| +0x23C | 16 (Hunt) | 0x004151E0 | **Mission_Hunt** | 929B | AI hunting: find target, approach, land at target cell, Carryall pickup |
| +0x240 | 7 (Sticky) | 0x00419C80 | Mission_Sticky | — | Docking/entering helipad state |
| +0x25C | 25 | 0x00417300 | Mission_SpyPlane | — | Spy plane overfly behavior |
| +0x260 | 26 (Open) | 0x004158E0 | Mission_Open | — | Open (paradrop prep) |
| +0x264 | 27 (Rescue) | 0x00415960 | Mission_Rescue | — | Rescue mission |
| +0x26C | 30 (ParaDropApproach) | 0x004155F0 | **Mission_ParaDropApproach** | 461B | Fly toward drop zone, reveal fog, transition to overfly |
| +0x270 | 31 (ParaDropOverfly) | 0x004157C0 | **Mission_ParaDropOverfly** | 274B | Drop payload, then fly off map edge |

### Communication & Docking

| VTable Offset | Address | Name | Description |
|---------------|---------|------|-------------|
| +0x278–0x27C | (inherited) | Radio_Send/Receive | Standard radio protocol |
| +0x194 | 0x004190B0 | **Receive_Radio** | Aircraft-specific radio handler (see Section 5) (corrected 2026-07-12: was documented as +0x284; live vtable read shows +0x284 holds `0x0041BEE0` — an unnamed function shared across 5 different vtables (xrefs from 0x007e2528, 0x007e8f18, 0x007eb2dc, 0x007f4be4, 0x007f5ef4), not Receive_Radio. The xref chain to `0x004190B0` traces to vtable data at `0x007e2438`, which is base+0x194. Verified via `read_memory 0x007E22A4 len=1332` (bulk vtable dump) + `get_xrefs_to 0x004190B0` — GHIDRA_ADDRESS_SHIFT) |
| +0x480 | 0x0041AA80 | UnitClass__EnterBuildingOrDock (inherited; AircraftClass does NOT override) | Inherited UnitClass slot — handles entering a building or initiating dock; AircraftClass has no Assign_Destination override at this slot (verified: `get_function_by_address 0x0041AA80`; `search_functions AircraftClass__Assign` returns no match) |
| +0x528 | 0x0041BBD0 | **FindBuildingToDock** | Find helipad/airfield for landing (see Section 6) |

## 5. Radio Protocol (AircraftClass::Receive_Radio — 0x004190B0)

Aircraft override the radio handler for these specific commands:

| Radio Cmd | Hex | Meaning | Aircraft Behavior |
|-----------|-----|---------|-------------------|
| RADIO_CAN_LOAD | 0x08 | "Can I load aboard you?" | Carryall: returns 10 (negative) if already docked at destination |
| RADIO_NEED_REPAIR | 0x0E | "I need to be repaired/reloaded" | If TypeClass.Passengers > 0 and current passengers < max: accept. Checks locomotor stopped, then initiates docking protocol. |
| RADIO_CAN_I_LOAD | 0x0F | "Can I dock at your pad?" | Returns 1 (positive) if passengers < max, 10 (negative) if full |
| RADIO_MOVE_HERE | 0x12 | "Move to this location" | If target is building (RTTI 6): enters Sticky mission (dock). Otherwise enters Move mission. |
| RADIO_DOCKING_REQUEST | 0x13 | "Requesting dock clearance" | If AirportBound and destination is a building: returns 1 (clear). Returns 10 otherwise. |
| RADIO_UNLOADED | 0x15 | "Passenger unloaded" | Decrements passenger count. If building has Helipad flag and not loading, enters QMove. |
| RADIO_MOVE_AWAY | 0x17 | "Move away from here" | Enters Move mission, picks random nearby destination. |
| RADIO_RELOAD_CHECK | 0x1D | "Do you need reloading?" | Returns `(ammo != maxAmmo) ? 10 : 1`. If has target, returns 10 (busy). |
| RADIO_RELOAD_AMMO | 0x1F | "Here's one ammo" | If ammo >= maxAmmo/2 AND has target: returns 1 (keep reloading). Otherwise delegates to FootClass (standard reload protocol). |
| RADIO_AMMO_STATUS | 0x21 | "What's your ammo status?" | Returns `(ammo != maxAmmo) ? 10 : 1` |

**RTB ammo threshold logic (from Receive_Radio 0x1F):**
- If aircraft has ammo >= half of max AND has a target (TarCom != 0): return 1 (accept reload but will re-engage)
- Otherwise: use FootClass default (which does the actual ammo increment)

## 6. FindBuildingToDock — Helipad Search (0x0041BBD0)

```
function FindBuildingToDock(technoType, unused, unused2):
    if AircraftTypeClass.AirportBound AND CachedDock != null:
        if Radio_Send(CAN_I_LOAD, CachedDock) == POSITIVE:
            return CachedDock    // reuse last pad
        CachedDock = null        // pad is full/gone, search again

    result = FootClass::Find_Docking_Bay(technoType, 0, unused2)
    CachedDock = result
    return result
```

**Key insight:** Aircraft cache their last-used helipad at offset 0x6CC. This avoids
re-searching every tick. The cache is invalidated when the radio check fails (pad full,
destroyed, or owner changed).

## 7. Mission_Attack State Machine (0x00417FE0)

The attack mission is the most complex aircraft behavior — an 11-state state machine:

```
State 0: INIT
    Clear HasFired, IsStrafe flags
    → State 3 (if has target) or State 10 (no target, RTB)

State 1: APPROACH_FIRE
    If HasFired: decrement ammo (HasFired acts as one-shot guard)
    If target valid and ammo > 0:
        Call FUN_004197c0() (set facing toward target)
        Assign_Destination(target)
        → State 3 (if spawner weapon) or State 10 (if no spawner)
    Else: → State 10 (RTB)

State 3: IN_RANGE_CHECK
    If HasFired: decrement ammo
    Check if weapon is spawner or target in range
    If spawner target ready: → State 4 (fire)
    If distance > weapon range: → State 4 (close in)
    If very close (< 0x200 leptons) and has spawner: → State 4 with fire
    Else if very close: set facing, return

State 4: FIRE_WEAPON
    Select weapon, call Fire_At
    switch(Fire_At result):
        FIRE_OK (0):
            Set HasFired = true
            Fire burst (loop weapon burst count times)
            Scatter objects at target
            If spawner: → State 6 (wait for spawned units)
            If has ammo: → State 1 or 10 based on FlyBy rules
            Else: → State 10 (RTB)
        OUT_OF_RANGE (2):
            If has ammo: approach or re-engage
        FACING_WRONG (default):
            If has ammo: re-approach
        SCATTER (9):
            Call Scatter()

State 5: STRAFE_FIRE (secondary fire pass)
    Similar to State 4 but for continued strafing

States 6, 7, 8: STRAFE_PASS_N (multi-pass strafing)
    Each state fires one burst, scatters target, moves to next state
    Return delay = weapon ROF
    If ammo runs out at any point: → State 10

State 9: STRAFE_FINAL
    Last strafe pass. Fire, scatter, → State 3 (re-evaluate)
    Return delay = (typeROF + 0x400) / typeSpeed

State 10: RETURN_TO_BASE
    Clear HasFired, IsStrafe
    If HasFired was set: decrement ammo
    If ammo == 0:
        If not Spawned and player-controlled: clear target
    Find helipad, move toward it
    Set IsLanding = false
    If has archive target and ammo > 0: re-enter attack
    Else: Enter_Idle_Mode → Guard
```

**Critical ammo mechanic:** Ammo is NOT decremented in the Fire_At call itself. Instead,
`HasFired` (offset 0x6C8) is set to true when a weapon fires. Ammo is decremented at
the START of the next state transition (States 1, 3, or 10). This means one ammo point
is consumed per complete attack pass, not per individual shot in a burst.

**Strafing logic:** States 6–9 implement multi-pass strafing (e.g., Harrier making
multiple passes). Each pass fires one burst and the `IsStrafe` flag (0x6D2) is set
to prevent the aircraft from entering RTB until all passes complete.

## 8. Mission_Guard — Idle & RTB Logic (0x0041A5C0)

```
function Mission_Guard():
    currentAlt = GetAltitude()
    cruiseAlt = TypeClass.GetHeight()

    if currentAlt == cruiseAlt:  // AT CRUISE HEIGHT — landed or hovering
        if has Team:
            if has NavCom: set WantsToLand = true, enter Move mission (2)  (corrected 2026-07-18: was "enter Attack mission"; decompile shows `(**(code**)(*param_1+0x1e8))(2,0)` — vtable+0x1e8 is Queue_Mission_Override (0x0041BA90, confirmed via get_function_by_address), and argument 2 is the Move mission ID per the §4 dispatch table, not Attack (1) — verified via `decompile_function 0x0041A5C0` — INFERENCE_HARDENED)
            return (mission-timer-derived delay)
        if has weapon:
            set WantsToLand = true  (corrected 2026-07-18: was "set IsLanding = true"; the byte written is offset 0x6D4, which this doc's own §2 table names WantsToLand, not IsLanding — verified via `decompile_function 0x0041A5C0` writing `*(undefined1*)(param_1+0x1b5)=1` i.e. byte 0x1b5*4=0x6D4 — INFERENCE_HARDENED)
            Enter_Idle_Mode → Guard
        else:
            move to own cell, enter Move mission (2)  (corrected 2026-07-18: was "enter Attack mission"; same Queue_Mission(2,0) call as above, confirmed via `decompile_function 0x0041A5C0` — INFERENCE_HARDENED)
        return

    // IN FLIGHT — check ammo for RTB decision
    // Three ammo threshold modes (from RulesClass global settings):
    if RulesClass.ReturnFire_Mode1:        // offset 0x889ECC
        RTB if ammo == 0
    elif RulesClass.ReturnFire_Mode2:      // offset 0x889ECD
        RTB if ammo < maxAmmo / 2
    else:                                  // default
        RTB if ammo < maxAmmo

    if should_RTB and not already pathing:
        dock = FindBuildingToDock(type + 1000)
        if dock found:
            Enter Mission_Sticky (7) — dock at helipad
            Assign_Destination(dock)
            Clear target
            return

    // Half-ammo navigation check:
    if ammo != -1 and ammo < maxAmmo/2 and currently pathing:
        if destination is building with UnitReload: keep going (already RTB)

    // Combat re-engagement:
    if has TarCom: enter Mission_Attack
    if AI-controlled: find nearby enemy, enter Attack
    if player-controlled and idle: delegate to FootClass::Mission_Guard
```

## 9. Enter_Idle_Mode — Decision Tree (0x004176F0)

This function determines what mission to assign when the aircraft has nothing to do.
Called after completing a mission, after RTB, or when created.

```
function Enter_Idle_Mode():
    if IsTeamMember:
        handle team-specific mission (script-driven)
        return

    default_mission = 5 (Guard)
    if !PlayerControlled and !HasTeam:
        if HasWeapon: default_mission = 11 (Hunt)

    locomotor_height = locomotor.GetHeight()
    current_altitude = GetAltitude()

    if altitude <= locomotor_height OR BalloonHover:
        // ON GROUND or BALLOON — minimal logic
        if Spawned:
            if no owner building: QMove (4)
            elif has team: Hunt (16) or Guard (5)
        else:
            clear destination, clear target
            if !PlayerControlled: default Hunt/Guard
        → assign mission

    else:
        // IN FLIGHT — complex decision tree
        if no owner building:
            if ammo > 0 and has target and IsAttacking: → Attack (1)
            if has weapon and IsOnMap:
                dock = FindBuildingToDock()
                if dock found and accepts radio: → Sticky (7, dock)
                if AirportBound and no dock: → self-destruct (crash)
            else: → Move to random cell (2)

        elif has team and Spawned:
            → Hunt (16) with random destination

        else: → Move to random cell (2)

    // Final RTB check:
    if ammo == 0 and has weapon and not already pathing:
        dock = FindBuildingToDock()
        if found: → Sticky (7, dock)
        if AirportBound and no dock: → self-destruct

    // Assign the chosen mission
    Queue_Mission(chosen_mission)
```

**Self-destruct on no helipad:** When `AirportBound=yes` and no dock is available,
the aircraft calls `TakeDamage(0)` which destroys it. This is the "Harrier crash"
behavior when all airfields are destroyed.

## 10. Mission_Move (0x004166C0)

```
function Mission_Move():
    if AircraftTypeClass.Carryall:
        return Carryall_Move()    // FUN_00416D50 — separate state machine

    switch SubState:
        0: INIT
            if no NavCom: Enter_Idle_Mode
            else: Assign_Destination(target), → 1

        1: SET_COURSE
            Get target coordinates from NavCom
            Set locomotor destination
            → 2

        2: IN_FLIGHT
            Check NavCom distance: if < 0xFF (255 leptons): land/arrive
            if locomotor stopped: → 3 (arrived)
            if at destination cell: → 3
            Check if destination is still valid; if not: → 0 (restart)
            → 4 (continue)

        3: WAITING_ARRIVAL
            if locomotor still moving: wait
            else: Enter_Idle_Mode

        4: COURSE_CORRECTION
            if locomotor still moving: check cell match, re-route if needed
            if stopped: → 3
```

**Arrival threshold:** 0xFF (255) leptons — approximately 1 cell width.

## 11. AI Function (0x00414BB0)

Called every game tick for each AircraftClass instance. Key behaviors:

1. **Trail animation:** If `AircraftTypeClass.Trailer` is set, spawns the trail AnimClass
   every `SpawnDelay` frames at the aircraft's position.

2. **Crash handling:** If the aircraft is crashing (`field_0x151` set), applies gravity,
   spawns smoke every 4 frames with random offset (±0xAA leptons), checks altitude < -400
   to trigger destruction.

3. **Map bounds check:** If not FlyBy and not FlyBack, checks if aircraft is within the
   playfield. If outside bounds and IsStrafe set, destroys aircraft (prevents infinite flight
   off-map during strafing).

4. **Locomotor tick:** Calls `ILocomotion::Process()` if locomotor exists — this is where
   FlyLocomotionClass/JumpjetLocomotionClass actually moves the aircraft.

5. **Landing target validity:** If ground target has become invalid (building destroyed),
   clears destination and target.

6. **Damage smoke:** When health < RulesClass.ConditionYellow (0x1708) and altitude > 0,
   randomly spawns smoke/fire animation (probability increases with lower HP).

7. **Carryall position sync:** If `Carryall=yes`, copies position data to the carried
   unit's Location fields (6 dwords = 24 bytes at offset +0x388 and +0x3A0).

## 12. INI Keys

### AircraftTypeClass-Specific (parsed in ReadINI at 0x0041CC20)

| Key | Type | Default | Section | Offset | Description |
|-----|------|---------|---------|--------|-------------|
| Landable | bool | false | Rules | 0xE0A | Can land on ground |
| AirportBound | bool | false | Rules | 0xE0D | Must dock at helipad; crashes without one |
| Fighter | bool | false | Rules | 0xE0E | Fighter aircraft classification |
| Carryall | bool | false | Rules | 0xDFC | Can pick up and transport units |
| Rotors | bool | false | Art | 0xE08 | Has visible rotor blades |
| CustomRotor | bool | false | Art | 0xE09 | Uses custom rotor animation |
| Trailer | AnimType | none | Art | 0xE00 | Trail animation behind aircraft |
| SpawnDelay | int | 3 | Art | 0xE04 | Frames between trail spawns |
| FlyBy | bool | false | Rules | 0xE0B | Strafing fly-by attack pattern |
| FlyBack | bool | false | Rules | 0xE0C | Return after fly-by (reverse course) |

### Constructor Defaults (from AircraftTypeClass::Constructor at 0x0041C8B0)

| Offset | Default | Likely Key | Notes |
|--------|---------|-----------|-------|
| 0xD38 | true | (inherited) | Set in constructor; possibly ConsideredAircraft default |
| 0xD96 | true | (inherited) | Unknown |
| 0xC8D | false | (inherited) | Unknown |
| 0x718 | 32 (0x20) | ROT | Rate of Turn default for aircraft types |

### Key Inherited TechnoTypeClass Keys

| Key | Offset | Description |
|-----|--------|-------------|
| Ammo | 0x684 | Max ammo (-1 = unlimited) |
| InitialAmmo | 0x680 | Starting ammo (-1 = use Ammo value) |
| Passengers | 0x5E0 | Passenger capacity |
| BalloonHover | 0xD68 | Never lands (Kirov) |
| ConsideredAircraft | 0xC95 | Treated as aircraft for game mechanics |
| Locomotor | 0x34C | CLSID for locomotion COM object |

### Global RulesClass Keys

| Key | Offset | Description |
|-----|--------|-------------|
| FlightLevel | 0x7B4 | Default cruise altitude |
| WakeTrail | 0x344 | Default trail animation for powered flight |
| ReturnFire modes | 0x889ECC-D | Ammo threshold for RTB decision (3 modes) |
| FlyByRules | 0x17E1 | Controls fly-by strafing behavior |

## 13. Integration Points

### Who creates AircraftClass?
- **FactoryClass** production queue → spawns at helipad/airfield
- **SpawnManagerClass** → spawns Hornets, Ospreys, missiles for carriers/destroyers/V3s
- **SuperWeaponTypeClass** → spawns paradrop cargo planes (PDPLANE, CARGOPLANE)

### What calls AircraftClass::AI?
- **LogicClass::Process** iterates all objects in the logic layer, calling `AI()` each tick

### Tick ordering (from World::advance_tick):
1. Ground movement
2. **Air movement** (FlyLocomotionClass::Process, JumpjetLocomotionClass::Process)
3. Vision
4. Combat (Fire_At → weapon burst → ammo tracking)
5. **AircraftClass::AI** (trail, crash, bounds, smoke)
6. Mission dispatch → Mission_Attack/Guard/Move/etc state machines

### Locomotor interaction:
- `ILocomotion::Process()` (vtable+0x5C) — called from AI, drives actual position change
- `ILocomotion::Is_Moving()` (vtable+0x10) — checked by mission handlers
- `ILocomotion::Move_To()` (vtable+0x44) — called when setting destination
- `ILocomotion::Get_Height()` (vtable+0x30) — used for altitude checks
- `ILocomotion::Draw_Matrix()` (vtable+0x28/0x2C) — used for rendering orientation

### Three locomotor types for aircraft:
| CLSID | Class | Users |
|-------|-------|-------|
| {4A582746-9839-11d1-B709-00A024DDAFD1} | FlyLocomotionClass | Harrier, Black Eagle, Hornet, Osprey, PDPLANE |
| {92612C46-F71F-11d1-AC9F-006008055BB5} | JumpjetLocomotionClass | Nighthawk, Flak Track heli, Kirov |
| {B7B49766-E576-11d3-9BD9-00104B972FE8} | RocketLocomotionClass | V3 Rocket, Dreadnought Missile |

## 14. Current Rust Implementation Status

The Rust engine has substantial aircraft support already implemented:

### Implemented
- **Ammo tracking & docking** (`src/sim/docking/aircraft_dock.rs`): 5-phase dock tick, AirfieldDocks reservation system, multi-slot support, reload timer
- **Air movement** (`src/sim/movement/air_movement.rs`): Altitude state machine (Ascending/Cruising/Hovering/Descending/Landed), horizontal movement, arrival detection
- **Jumpjet movement** (`src/sim/movement/jumpjet_movement.rs`): Acceleration/deceleration (1.5x ratio confirmed), turn rate, wobble, crash descent
- **Locomotor framework** (`src/sim/movement/locomotor.rs`): LocomotorKind enum (Fly/Jumpjet/Rocket), AirMovePhase enum, altitude/climb fields
- **Type parsing** (`src/rules/object_type.rs`): Ammo, ConsideredAircraft, Dock, UnitReload, Helipad, NumberOfDocks
- **Production** (`src/sim/production/`): Aircraft production category, airfield dock tracking
- **Entity integration** (`src/sim/game_entity.rs`): AircraftAmmo component, EntityCategory::Aircraft
- **Tests**: Comprehensive test suites for docking, altitude, acceleration

### Missing / Incomplete
- **Mission state machines** — The original has 11-state Mission_Attack, 5-state Mission_Move, complex Mission_Guard with RTB logic. The Rust engine likely uses simpler equivalents. The original's multi-pass strafing (states 6–9) is particularly complex.
- **HasFired ammo guard** — Original uses a per-attack-pass boolean to ensure exactly one ammo is consumed per pass. Rust currently deducts on burst completion which may differ.
- **CachedDock field** — Original caches last helipad at offset 0x6CC for efficiency
- **Radio protocol** — Original uses 10+ radio commands for docking negotiation; Rust uses a simpler reservation system
- **Carryall logic** — Original has dedicated Carryall movement handler (FUN_00416D50) and position sync in AI
- **Paradrop missions** — Original has ParaDropApproach and ParaDropOverfly mission handlers
- **Enter_Idle_Mode decision tree** — Original has complex decision tree for post-mission behavior; includes self-destruct when AirportBound and no pad available
- **Fly-by strafing** — FlyBy/FlyBack flags and multi-pass strafing states
- **Trail animations** — Trailer/SpawnDelay from AircraftTypeClass
- **Crash behavior** — Gravity, smoke spawning, altitude-based destruction
- **Rotor rendering** — Rotors/CustomRotor overlays on helicopter aircraft
- **AI hunting** — AI-controlled aircraft auto-hunt behavior in Mission_Guard and Mission_Hunt

## 15. FlyLocomotionClass — Flight Physics (0x004CD600)

**Constructor:** `0x004CC9A0`
**Process (main tick):** `0x004CD600` (526 lines decompiled, 3 pages)

### FlyLocomotionClass Struct Layout

| Offset | Type | Initial | Name | Description |
|--------|------|---------|------|-------------|
| 0x00 | void* | vtable | IUnknown_vtable | COM IUnknown interface |
| 0x04 | void* | vtable | ILocomotion_vtable | COM ILocomotion interface |
| 0x08 | int | — | RefCount | COM reference count |
| 0x0C | TechnoClass* | — | LinkedObject | Owner aircraft pointer |
| 0x18 | byte | 0 | IsMoving | Whether actively flying toward destination |
| 0x1C | int | NullCoord | Destination_X | Target position X (leptons) |
| 0x20 | int | NullCoord | Destination_Y | Target position Y (leptons) |
| 0x24 | int | NullCoord | Destination_Z | Target position Z (leptons) |
| 0x28 | int | NullCoord | OldPos_X | Previous position X |
| 0x2C | int | NullCoord | OldPos_Y | Previous position Y |
| 0x30 | int | NullCoord | OldPos_Z | Previous position Z |
| 0x34 | byte | 0 | Unknown_34 | |
| 0x38 | int | 0 | TargetAltitude | Target flight altitude (leptons) |
| 0x40 | double | 0.0 | TargetSpeedFraction | Speed fraction target (0.0–1.0) |
| 0x48 | double | 0.0 | CurrentSpeedFraction | Current speed fraction (ramps toward target) |
| 0x50 | byte | 0 | IsDescending | Aircraft should descend |
| 0x51 | byte | 0 | IsLanding | Aircraft is in landing approach |
| 0x52 | byte | 0 | Unknown_52 | |
| 0x53 | byte | 0 | Unknown_53 | |
| 0x54 | int | 0 | Unknown_54 | |
| 0x58 | int | 0 | AltitudeCounter | Descent counter — subtracted from Z each tick |
| 0x5C | byte | 0 | JustArrived | Cleared when entering Mission_Sticky |

### Flight Physics Algorithm (FlyLocomotionClass::Process)

**Per-tick processing (every game frame):**

```
1. UPDATE CELL TRACKING
   Get aircraft's current cell from owner's Get_Cell()
   If cell changed from last tick:
       Update occupancy map (remove from old cell, add to new cell)

2. ALTITUDE MANAGEMENT (when no destination / not moving)
   If altitude > 0 and not moving:
       AltitudeCounter += 1 (descending) or += 3 (fast descend when no health)
       Aircraft Z = Type.Z - AltitudeCounter

   If altitude reaches 0 (crashed to ground):
       if has Strength (health > 0):
           Explode: select warhead animation based on terrain type
           Apply_area_damage at crash site with RulesClass.CrashDamage
           return (destroyed)
       else:
           Play crash sound (water vs land based on cell terrain)
           Destroy aircraft
           return

3. GUARD MODE AUTO-ENGAGE
   If idling and has NavCom and mission is Guard:
       If no queued mission and not already landing:
           Enter Mission_Move

4. HEADING CALCULATION (when moving toward destination)
   delta_x = destination.X - current.X
   delta_y = destination.Y - current.Y
   heading = atan2(delta_y, delta_x)  // game direction format

   new_position.X = current.X + speed * cos(heading)
   new_position.Y = current.Y + speed * sin(heading)
   new_position.Z = destination.Z  // vertical handled separately

5. MAP BOUNDARY HANDLING
   If new position is outside map bounds:
       If aircraft is FlyBy type: deflect by ±0x80 leptons
       Else: find safe scatter position via FUN_0049F420
       If still out of bounds: set position and continue

6. ALTITUDE ADJUSTMENT
   If owner is on a bridge cell (cell flags & 0x100):
       Altitude = max(altitude, BridgeHeight)
       Reduce distance-to-target accordingly

7. SPEED CONTROL
   distance = sqrt(delta_x² + delta_y²)

   If distance < TargetAltitude (approaching):
       Arrive: clear destination, set IsOnMap flag
       If has Strength: trigger proximity event

   If distance > TargetAltitude (far away):
       Ascent/descent rate per tick:
           Normal: min(distance/20, 50)  // clamped to 20-50 range
           Landing approach: min(6, distance)  // slow final approach
           When target alt == cruise alt and far: min(16, distance)

   Acceleration:
       _DAT_007e3860 = acceleration step per tick (small float)
       CurrentSpeed ramps toward TargetSpeed by ±step each tick

8. SPEED FRACTION CALCULATION
   If aircraft is NOT OmniFire (TypeClass+0xD27):
       speed_fraction = min(1.0, distance / Type.Speed)
       If fraction < 0.1 threshold:
           If distance < 86 leptons: speed *= 0.3 (brake hard)
           Else: fraction = 0.1 (minimum)
       If distance < current_speed: current_speed = distance
       If speed = 0 and current_speed = 0 and distance > 0:
           current_speed = 0.05 (minimum creep)

   If OmniFire:
       If descending AND has target: fraction = 1.0
       Else: fraction = 0.0 (hover)

9. PITCH ANIMATION (visual only, no gameplay effect)
   If moving and distance < Type.Speed:
       pitch_factor = 1.0 - (distance - speed * 0.6) / (speed * 0.4)   (corrected 2026-07-12: constants were "0.375"/"0.25"; the backing addresses 0x007E3558/0x007E3550 read 0.6/0.4 live — see corrected Key Constants table above — INFERENCE_HARDENED)
       pitch = clamp(pitch_factor * Type.PitchSpeed, Type.PitchSpeed)
       owner.PitchAngle = pitch  (offset 0x2E8 in TechnoClass, float)

10. AUTO-LAND ON ARRIVAL
    If at destination cell AND speed = 0 AND (no target OR ammo = 0):
        Call FUN_004CFA70() → begin landing sequence
```

### Key Constants (from binary)

| Address | Value | Purpose |
|---------|-------|---------|
| 0x007E3860 | 0.1 | Speed acceleration step per tick (corrected 2026-07-12: was "~0.05"; `read_memory 0x007E3860 len=8` → `9a9999999999b93f` LE = IEEE754 `0x3FB999999999999A` = 0.1, matching §24 — INFERENCE_HARDENED, an earlier approximation was never reconciled against the later verified §24 read) |
| 0x007E1738 | 0.5 | Braking multiplier (close to target) (verified: `read_memory 0x007E1738 len=8` → `000000000000e03f` = IEEE 754 LE `0x3FE0000000000000` = 0.5; §24 is consistent) |
| 0x007E3558 | 0.6 | Pitch calculation near-range fraction (corrected 2026-07-12: was "0.375"; `read_memory 0x007E3558 len=8` → `333333333333e33f` LE = IEEE754 `0x3FE3333333333333` = 0.6, matching §24 — INFERENCE_HARDENED) |
| 0x007E3550 | 0.4 | Pitch calculation denominator fraction (corrected 2026-07-12: was "0.25"; `read_memory 0x007E3550 len=8` → `9a9999999999d93f` LE = IEEE754 `0x3FD999999999999A` = 0.4, matching §24 — INFERENCE_HARDENED) |
| 0x007E2810 | -π/32768 | Angle-to-radians conversion factor (corrected 2026-07-12: sign was omitted — doc stated "π/32768" (positive); `read_memory 0x007E2810 len=8` → `575e9f982d2219bf` LE = IEEE754 `0xBF19222D989F5E57` ≈ -9.5877e-5 = -(π/32768), a NEGATIVE constant — OPERATOR_OR_ORDER_DRIFT) |
| DAT_008B3CAC | — | Bridge height constant |
| RulesClass+0xFA8 | — | CrashDamage warhead index |

## 16. Airspace Deconfliction (0x00419B00)

**AircraftClass::Is_Cell_Free_For_Landing** — checks if a cell is safe for this aircraft.

```
function Is_Cell_Free_For_Landing(cell, include_self):
    if cell is outside playfield: return true (don't block out-of-bounds checks)

    // Homing missile check
    if TypeClass.IsHoming (offset 0xD54):
        nearestObj = Find_Nearest_Object_In_Cell(cell)
        if nearestObj exists and nearestObj.TarCom != 0: return true (occupied)
        if nearestObj.TypeClass.IsHoming: return true (occupied)

    // AirportBound aircraft always pass — they dock, not land on ground
    if TypeClass.AirportBound: return true

    // Carryall check: if Carryall and NavCom is an aircraft
    is_carryall_to_aircraft = (TypeClass.Carryall AND NavCom != null AND NavCom.RTTI == Aircraft)

    // Check all aircraft in global array for collisions
    for each aircraft in g_AircraftArray (at DAT_008B3DC4):
        if aircraft is null: skip
        if is_carryall_to_aircraft and aircraft == NavCom: skip (our pickup target)
        if !include_self and aircraft == this: skip
        if aircraft is limbo'd: skip
        if aircraft is not on map: skip

        aircraft_cell = aircraft.Get_Cell()
        if aircraft_cell == cell OR aircraft.NavCom == cell_object:
            return false  // OCCUPIED — another aircraft is here or heading here

    return true  // Cell is free
```

**Critical insight:** This is a global deconfliction system that prevents multiple aircraft
from landing at the same cell. It iterates the ENTIRE aircraft array every time. The Rust
engine's AirfieldDocks reservation system is a reasonable approximation but doesn't cover
non-airfield landing (e.g., Carryall pickup spots, carrier return).

## 17. Target Cell Selection (0x00418E20 + 0x004197C0)

### Find_Attack_Cell (0x00418E20)

Called to find a valid cell near a target for attack approach.

```
function Find_Attack_Cell(target):
    if target is null or (has team and team is active): return target as-is

    // Try target's cell directly
    if target is in weapon range:
        target_cell = target.Get_Cell()
        if Is_Cell_Free_For_Landing(target_cell): return target

    // Spiral search around target
    target_coord = target.Get_Location()
    for ring = 0 to RulesClass.GuardAreaRadius (offset 0x1478) / 256:
        start_dir = Random(0, 7)  // random start direction
        for dir = 0 to 7:
            angle = ((start_dir + dir) * 0x2000) - 0x3FFF
            cell = target + (cos(angle) * ring, sin(angle) * ring)

            if cell is in playfield
               AND (is AI or is Spawned or cell is revealed)
               AND target is in weapon range from cell
               AND Is_Cell_Free_For_Landing(cell):
                return cell

    return target  // fallback
```

### Find_Approach_Cell (0x004197C0)

Called from Mission_Attack to find a cell to fly to within weapon range of the target.

```
function Find_Approach_Cell(target):
    if target is null: return null
    if locomotor is still moving: return target (don't recalculate mid-flight)

    weapon_range = Get_Weapon_Range(0)
    target_dest = target.NavCom ? target.NavCom.Location : (0,0,0)

    // Decreasing-radius ring search
    for range = weapon_range; range > 256; range -= 256:
        best_distance = -1
        for dir = 0 to 255 step 16:  // 16 directions per ring
            angle = (dir << 8) - 0x3FFF
            cell = this.Location + (cos(angle) * range, sin(angle) * range)

            if cell is in playfield
               AND (is AI or is Spawned or cell is revealed)
               AND Is_Cell_Free_For_Landing(cell):
                distance_to_target_dest = Distance3D(cell, target_dest)
                if distance_to_target_dest < best_distance:
                    best_distance = distance_to_target_dest
                    best_cell = cell

        if best_distance != -1:
            // 50% chance to use best, 50% to use second-best (variety)
            if Random(0, 99) < 50: return best_cell
            else: return second_best_cell

    return null
```

## 18. Carryall System (0x00416D50 + 0x00416AF0)

### Carryall_Move State Machine (0x00416D50)

Called from Mission_Move when `AircraftTypeClass.Carryall` is true.

```
State 0: VALIDATE
    if no NavCom: Enter_Idle_Mode, return

    // Get unit to carry
    carryTarget = Get_Carry_Target()  // FUN_0040DD70
    if carryTarget exists AND not carrying AND target is allied:
        if target.IsAlive: verify target is a FootClass (RTTI == 1)

        if target is FootClass:
            // Check if we've already been assigned to it
            dest = GetDestination(0)
            if dest != carryTarget: Radio_Send(ROGER, carryTarget)

            // Request pickup permission
            result = Radio_Send(DOCKING_REQUEST, carryTarget)
            if result != POSITIVE:
                clear destination, Enter_Idle_Mode
                return

            // Check if target accepts carry
            result = Radio_Send(0x24, carryTarget)  // CARRY_REQUEST
            if result != POSITIVE:
                Radio_Send(ROGER, carryTarget)
                Enter_Idle_Mode
                return

            Radio_Send(STICKY, carryTarget)

    Assign_Destination(carryTarget)
    Set team convoy target
    → State 1

State 1: FLY_TO_TARGET
    Get target coordinates
    Set locomotor destination
    → State 2

State 2: IN_FLIGHT
    Debug log: "Carryall - FLY_TO"

    if NavCom exists and NavCom is not a building (RTTI != 0xB):
        if destination changed: → State 0 (restart)
    if NavCom is a building and target not reachable: → State 0

    if locomotor Is_On_Floor:
        if NavCom is an aircraft (RTTI == 1):
            Check if at same cell — if not: → State 0 (overshot)
        WantsToLand = false  // signal we've landed

    if locomotor not moving:
        → State 3 (landed)
    return

State 3: LANDED_PICKUP
    Debug log: "Carryall - LAND"

    if carrying a unit:
        if unit is being repaired (offset 0x140): wait

        // Already carrying — do drop-off
        Disable map updates
        Call Carryall_Pickup() to drop the unit
        Enable map updates
        → State 0

    // No unit carried — try to pick up
    Check cell for building at our position
    if building found AND building == our destination:
        Debug log: "Carryall - LAND - at destination"
        result = Radio_Send(DOCKING_REQUEST, building)
        if POSITIVE:
            Set building's IsOnMap = false
            Set building.IsSurrendered = true
            Attach building to us (FUN_004733A0)
            return

    → State 0 (reset)
    Radio_Send(ROGER)  // disconnect
    Enable map updates
    WantsToLand = true
    Enter_Idle_Mode
```

### Carryall_Pickup (0x00416AF0)

Handles the physical act of picking up or dropping off a carried unit.

```
function Carryall_Pickup(aircraft):
    carried = Get_Carried_Unit()  // FUN_00473430

    // Get ground height at carrier position
    groundHeight = CellClass::GetGroundHeight(carried_pos)
    cell = Get_Cell_At(carried_pos)

    // Set carried unit's bridge flag based on cell
    if cell has bridge (flags & 0x100):
        carried.IsOnBridge = true
    else:
        carried.IsOnBridge = false

    // RELEASE OLD LOCOMOTOR — destroy the carried unit's flight locomotor
    old_loco = carried.Locomotor
    old_loco->Release()  // COM release
    carried.Locomotor = null

    // CREATE NEW LOCOMOTOR — restore ground locomotor for the dropped unit
    typeClass = carried.GetTypeClass()
    clsid = typeClass + 0x34C  // Locomotor CLSID from type
    CoCreateInstance(clsid, ...) → new_loco
    carried.Locomotor = new_loco
    new_loco->Link_To_Object(carried)

    // Place the unit on the ground
    if carried.CanDeploy():
        Clear fire flags
        Clear surrender flag
        Update fog of war (reveal carried.Sight ± 3 cells)

        find_cell = Find_Nearby_Passable_Cell()

        if carried's destination == carrier:
            Radio_Send(UNLOADED, carried)
            Radio_Send(ROGER, carried)

        if find_cell is invalid:
            Assign_Destination(null)  // nowhere to go
        else:
            Assign_Destination(find_cell)  // move to drop point
    else:
        Attach to carrier (FUN_004733A0)
```

**Key insight:** The Carryall system physically swaps the carried unit's locomotor!
The original ground locomotor (Drive/Walk/etc.) is destroyed and replaced with the
carrier's flight locomotor during carry. On drop-off, a new ground locomotor is
CoCreateInstance'd from the unit's TypeClass CLSID. This means the carried unit
literally "flies" while being carried, then gets a fresh ground locomotor on release.

## 19. ReceiveDamage — Death & Crash (0x004165C0)

```
function ReceiveDamage(damage_args...):
    result = FootClass::ReceiveDamage(damage_args)

    if result == 4 (DESTROYED):
        // Stop the locomotor (freeze in place)
        StopLocomotion()

        // Spawn random explosion animation from type's explosion list
        if TypeClass.ExplosionCount > 0:
            index = Random() % ExplosionCount
            anim = TypeClass.Explosions[index]
            Create AnimClass at aircraft position

        // Try to crash-land (enter crash state rather than instant death)
        if Limbo_Aircraft(damage_source):
            return  // now crashing — will be destroyed when hits ground
        else:
            Destroy()  // instant destruction (no crash anim)

    elif result == 5 (NOTHING — no effect):
        return 5

    return result
```

**Crash sequence:** When destroyed, the aircraft doesn't immediately vanish. Instead,
`Limbo_Aircraft` (vtable+0x3DC) puts it in a "crashing" state where it descends with
gravity in the AI function (altitude -= 1 or 3 per tick), trailing smoke every 4 frames.
When altitude reaches -400, it explodes for real. This is why you see Harriers spiral
down when shot — they're in the crash physics loop.

## 20. Detach — Cleanup on Object Removal (0x0041B660)

```
function Detach(object, flags):
    FootClass::Detach(object, flags)

    if object == CachedDock (0x6CC):
        CachedDock = null  // helipad destroyed, clear cache

    if object == Type (0x6C4):
        Type = null  // type class removed (shouldn't happen in normal gameplay)
```

## 21. Unlimbo — Place on Map (0x00414310)

```
function Unlimbo(coords, facing):
    position = coords

    if NOT BalloonHover:
        if Spawned:
            // Check if on bridge — adjust Z
            if Is_On_Bridge(coords):
                position.Z = GetGroundHeight(coords)
        else:
            // Normal aircraft: set to cruise altitude
            groundZ = GetGroundHeight(coords)
            cruiseZ = TypeClass.GetHeight()
            position.Z = groundZ + cruiseZ

    if NOT Parent::Unlimbo(position, facing):
        return false

    // Mark as spawned if Landable and weapon isn't arcing
    if TypeClass.HasShadow AND TypeClass.Landable:
        weapon = GetPrimaryWeapon()
        if weapon.Projectile.IsArcing: skip
        else: this.IsSpawned = true

    // Factory tracking
    if HasOwnerBuilding:
        this.IsFromFactory = true  // offset 0x6C9

    // Set facing and animation state
    FacingClass::UpdateFacing(facing)
    this.AnimFrame = CurrentFrame
    this.AnimState = 1 (Y), 1 (X)
    this.DrawOffset = 0

    // Speed based on altitude
    cruiseAlt = TypeClass.GetHeight()
    currentAlt = GetAltitude()
    if currentAlt == cruiseAlt:
        SetSpeedFraction(1.0)  // full speed at cruise
    else:
        SetSpeedFraction(0.0)  // stopped on ground

    return true
```

## 22. Find_Nearest_Friendly_Airfield (0x0041A160)

This function finds the nearest friendly building that this aircraft can dock at.
Called from Enter_Idle_Mode and other places when the aircraft needs to RTB.

```
function Find_Nearest_Friendly_Airfield(randomize):
    best_distance = -1
    best_cell = null

    // Phase 1: Search owner's buildings
    if TypeClass.WeaponCount > 0 (has weapons, offset 0x3F8):
        building_count = House.BuildingCount (offset +0x78 in HouseClass array)
        for each building in owner's building list:
            if building is limbo'd: skip

            distance = Distance3D(this, building)

            // Prefer buildings of same type as our production building
            if building.TypeClass == TypeClass.FactoryBuilding:
                distance /= 4  // 4x preference!

            if distance < best_distance:
                // Find passable cell near building
                cell = Find_Nearby_Passable_Cell(near building, radius 3)
                if cell is valid:
                    best_distance = distance
                    best_cell = cell

        // If very close (< 256 leptons): search own cell instead
        if best_distance < 0x100:
            cell = Find_Nearby_Passable_Cell(near self, radius 1)
            return cell

        if found: return best_cell

    // Phase 2: Search all aircraft of same owner
    for each aircraft in g_AircraftArray:
        if aircraft is limbo'd: skip
        if aircraft.House != this.House: skip
        if aircraft == this: skip

        distance = Distance3D(this, aircraft)
        if distance < best_distance:
            cell = Find_Nearby_Passable_Cell(near aircraft, radius 1)
            if cell is valid:
                best_distance = distance
                best_cell = cell

    if found: return best_cell

    // Phase 3: Fallback — return own cell
    return Get_Cell_At(this.Location)
```

**Key insight:** The building preference has a 4x distance bonus for buildings matching
the aircraft's factory type. This means aircraft strongly prefer returning to their
own production building (e.g., Allied Air Force Command) over random allied buildings.

## 23. Open Questions

| Question | Why Unknown | Priority |
|----------|------------|----------|
| What is field 0x6CA? | Only cleared in constructor; no writes seen in any decompiled function | Low |
| What is field 0x6D0? | Only cleared in constructor | Low |
| What is field 0x6D1? | Only cleared in constructor | Low |
| Exact RTB ammo threshold globals (0x889ECC, 0x889ECD)? | Need to trace these to their INI keys in RulesClass::ReadINI | Medium |
| SpawnManagerClass ↔ AircraftClass interaction? | SpawnManagerClass creates aircraft instances and manages regen timers — not fully traced here | Medium |
| FlyLocomotionClass constants — exact float values? | Read IEEE 754 doubles at 0x007E3860, 0x007E1738, 0x007E3558, 0x007E3550 | Low |
| FUN_004CFA70 — begin landing sequence? | Called when aircraft arrives at destination with no target/ammo — likely initiates descent | Medium |
| Mission_SpyPlane (0x00417300) details? | Not decompiled — used for spy plane overfly behavior | Low |

## 24. Verified Float Constants

| Address | Hex (LE) | Value | Usage |
|---------|----------|-------|-------|
| 0x007E3860 | 3FB999999999999A | **0.1** | Speed acceleration step per tick |
| 0x007E1738 | 3FE0000000000000 | **0.5** | Braking multiplier when within 86 leptons of destination |
| 0x007E3558 | 3FE3333333333333 | **0.6** | Pitch calculation: near-range fraction |
| 0x007E3550 | 3FD999999999999A | **0.4** | Pitch calculation: denominator fraction |
| 0x007E48F0 | 3FF8000000000000 | **1.5** | Deceleration = Acceleration × 1.5 (also used by JumpjetLocomotionClass) |
| 0x007E1748 | 00000000 (float) | **0.0** | Is_Moving threshold: any PitchAngle > 0.0 counts as "moving" |

## 25. FlyLocomotionClass — ILocomotion Interface Methods

### ILocomotion VTable (at 0x007E89F4)

| VTable+Offset | Index | Address | Method | Description |
|---------------|-------|---------|--------|-------------|
| +0x00 | 0 | 0x004D0510 | QueryInterface | COM |
| +0x04 | 1 | 0x004D0520 | AddRef | COM |
| +0x08 | 2 | 0x004D0530 | Release | COM |
| +0x0C | 3 | 0x004CCA20 | Link_To_Object | Set owner pointer |
| +0x10 | 4 | 0x004CCA90 | **Is_Moving** | True if old pos set OR PitchAngle > 0.0 |
| +0x14 | 5 | 0x004CCAE0 | **Destination** | Returns dest coords (or NullCoord) |
| +0x24 | 9 | 0x004CF610 | Unlimbo | Place on map |
| +0x28 | 10 | 0x004CFB00 | Draw_Matrix | 3D rotation matrix for rendering |
| +0x30 | 12 | 0x004CF940 | Is_Powered | Check if aircraft has power |
| +0x40 | 16 | 0x004CCB40 | **Layer** | MAIN TICK: calls Process + step interpolation |
| +0x44 | 17 | 0x004CCC80 | **Move_To** (835B) | Set destination coordinates; initiates takeoff if on ground |
| +0x5C | 23 | 0x004CFD20 | Process_Wrapper | Calls FlyLocomotionClass::Process |
| +0x80 | 32 | 0x004CCAC0 | Is_Really_Moving | Stricter movement check |
| +0x84 | 33 | 0x004CFE20 | **Get_Height** | Returns TypeClass.GetHeight() as int |
| +0x90 | 36 | 0x004CFE50 | **Is_On_Floor** | Landing state check (returns 0-3) |

### Is_Moving (0x004CCA90)

```
function Is_Moving(this):
    if this.OldPos_set (offset 0x30, byte): return true
    if owner.PitchAngle (offset 0x2E8, float) > 0.0: return true
    return false
```

**Key insight:** An aircraft with ANY positive pitch angle is considered "moving".
This prevents premature landing triggers during deceleration when the aircraft is
still pitched up but has zero velocity.

### Is_On_Floor (0x004CFE50)

```
function Is_On_Floor(this):
    if this.IsLanding (offset 0x4D, byte): return 1  // ON FLOOR
    if this.IsAscending (offset 0x4C, byte): return 0  // IN AIR, ascending
    if Is_Moving(this): return 2  // APPROACHING (in flight, decelerating)
    return 3  // STOPPED ON FLOOR
```

Return value semantics:
- **0** = In air, ascending (just took off)
- **1** = Confirmed on floor (landing flag set)
- **2** = In motion (flying/approaching)
- **3** = Stopped on ground (no velocity, no pitch)

### Move_To (0x004CCC80) — Set Destination

```
function Move_To(this, dest_X, dest_Y, dest_Z):
    // Ignore if already at destination AND landing
    if dest_cell == current_dest_cell AND IsLanding: return

    // Check blocked states
    if owner.IsBeingWarped: return
    if owner.IsLocked: return
    if owner.IsCrashing: return
    if owner.IsDestroyed: return
    if !Is_On_Floor(this): return  // can't redirect mid-air? (unclear)

    if dest == NullCoord:
        // CLEAR DESTINATION → begin landing
        if altitude == 0 or IsLanding:
            destination = NullCoord
        else:
            destination = owner.Location  // land at current pos
            Begin_Landing()
        clear movement flag
        return

    // SET NEW DESTINATION
    destination.X = dest_X
    destination.Y = dest_Y
    destination.Z = dest_Z  (adjusted to groundHeight + cruiseAlt if has target+ammo)

    IsActive = true (offset 0x0C)

    // If on floor: begin takeoff
    if on_floor or altitude == 0:
        Begin_Takeoff()

    // Landing vs flight determination
    if dest.Z > groundHeight + 120: flag descending
    elif no target or no ammo: check dock status
```

### Begin_Landing (0x004CFA70)

```
function Begin_Landing(this):
    // Don't land during non-dock combat
    if owner.Mission != Sticky(7) AND in_combat(): return

    // AirportBound: verify we're at a valid helipad cell
    if owner is AircraftClass AND owner.Type.AirportBound:
        building = Get_Building_At_Cell(owner.Cell)
        if building NOT in owner's dock list:
            owner.Enter_Idle_Mode()
            return  // wrong cell, abort landing

    // Initiate descent
    this.IsDescending = false   // 0x50
    this.IsOnFloor = true       // 0x51 → Is_On_Floor returns 1 (NOT IsLanding; IsLanding is 0x4D per §25 struct table; verified: `decompile_function 0x004CFA70` writes `*(undefined1 *)(param_1 + 0x51) = 1`)
    this.Unknown_52 = false     // 0x52
    this.TargetAltitude = 0     // 0x38 → descend to ground level
```

### Begin_Takeoff (0x004CF950)

```
function Begin_Takeoff(this):
    if owner.IsBeingWarped: return
    if owner.IsLocked: return
    if owner.IsCrashing: return
    if owner.IsDestroyed: return

    // Initiate ascent
    this.IsLanding = false      // 0x51 → no longer on floor
    this.IsAscending = true     // 0x50 → ascending

    // Fix null cell reference if needed
    if owner.Cell == NullCell:
        Fix_Cell_Position(owner)

    // Set target altitude to cruise height
    typeClass = owner.GetTypeClass()
    this.TargetAltitude = typeClass.GetHeight()  // 0x38

    // Set initial facing if stationary
    if owner.GetAltitude() == 0:
        UpdateFacing(random_direction)

    // Play takeoff sound
    VocClass::PlayAt(owner.Location)
```

### FlyLocomotionClass Updated Struct Layout

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| 0x0C | byte | IsActive | Set when destination is assigned |
| 0x30 | byte | HasOldPosition | Set when aircraft has moved from prev pos |
| 0x38 | int | TargetAltitude | 0 = land, cruiseHeight = fly |
| 0x40 | double | TargetSpeedFraction | 0.0–1.0 target |
| 0x48 | double | CurrentSpeedFraction | Ramps toward target at ±0.1/tick |
| 0x4C | byte | IsAscending | Set during takeoff, cleared on landing |
| 0x4D | byte | IsLanding | Set during landing, cleared on takeoff |
| 0x50 | byte | IsDescending | Descent/ascent direction flag |
| 0x51 | byte | IsOnFloor | Set by Begin_Landing, cleared by Begin_Takeoff |
| 0x58 | int | AltitudeCounter | Incremented per tick for gravity descent |

## 26. Paradrop Mission Cycle

### Mission_Open (0x004158E0) — Prepare Payload Drop

```
function Mission_Open():
    if no TarCom: clear dest, enter QMove, return
    if no NavCom: set dest to TarCom, return

    distance = Distance_To(TarCom)
    if distance <= RulesClass.ParadropRadius (offset 0x54C):
        enter Mission_Rescue (0x1B)
        PayloadCount -= 1  ← DECREMENTS offset 0x6D3!
    return 3
```

### Mission_Rescue (0x00415960) — Execute Payload Drop

```
function Mission_Rescue():
    IsStrafe = true  (offset 0x6D2)

    if no TarCom or no owner building:
        IsStrafe = false
    else:
        distance = Distance_To(TarCom)
        if distance <= RulesClass.ParadropRadius:
            if Is_On_Bridge(position):
                return 5  // can't drop on bridge
            FUN_00415C60()  // ← ACTUAL DROP: unload one unit
            return 5

    // Check if more payload to drop
    if PayloadCount > 0 (offset 0x6D3):
        enter Mission_Open (0x1A)  // cycle back for next drop
        return 5

    // All dropped — fly off
    clear TarCom
    clear destination
    enter QMove (mission 4)
    return 5
```

### Paradrop Cycle Summary

```
Mission_ParaDropApproach → fly toward drop zone, reveal fog
    ↓ (within 0x301 leptons)
Mission_ParaDropOverfly → continue toward target, drop payload
    ↓ (reaches target area)
Mission_Open → decrement PayloadCount, approach target
    ↓ (within ParadropRadius)
Mission_Rescue → drop one unit, check PayloadCount
    ↓ (PayloadCount > 0)
Mission_Open → decrement again... (loop)
    ↓ (PayloadCount == 0)
Mission_QMove → fly off to map edge
```

**Field 0x6D3 is `PayloadCount`** — initialized to 5 in the constructor (default paradrop
load of 5 units). Decremented once per pass through Mission_Open. When it reaches 0,
the aircraft stops dropping and flies off via QMove. The actual count is set by the
superweapon/paradrop system before the mission starts.

## 27. Mission_QMove — Fly Off Map (0x00415A50)

```
function Mission_QMove():
    if no NavCom:
        // Pick a random map edge cell from owner's starting edge
        edge = House.StartingEdge (offset +0x1E0)
        if edge is invalid: edge = random(0-3)
        dest_cell = Find_Edge_Cell(edge)
        if dest_cell is valid:
            Assign_Destination(dest_cell)
        return 3

    // Already have destination — check if arrived
    current_cell = GetCell()
    if NavCom == current_cell:
        Assign_Destination(null)  // clear, we've arrived at edge
    return 3
```

Used by paradrop planes after dropping their payload, and by aircraft that need to exit
the map. Returns tick delay of 3 (fast processing).

## 28. Mission_SpyPlane (0x00417300) — Spy Plane Overfly

```
function Mission_SpyPlane():
    owner.IsCloaked = true  // offset 0x430 — spy plane is invisible

    switch SubState:
        0: INIT
            if no NavCom: Enter_Idle_Mode
            else: Find_Attack_Cell(NavCom), set destination
            → State 1

        1: SET_COURSE
            Set locomotor destination from NavCom
            → State 2

        2: IN_FLIGHT
            if ammo == 0:
                find helipad/dock → enter Sticky (dock)
                or if has ammo → find targets → enter Attack
            else:
                // Check arrival: if within 0xFF leptons of target
                if arrived: update facing, transition
                // Check locomotor moving
                if stopped: → State 3
                // Check airspace deconfliction
                if destination cell occupied: → State 0 (re-route)
                → State 4

        3: ARRIVED
            Enter_Idle_Mode

        4: CONTINUE_MONITORING
            if locomotor still moving:
                check airspace, re-route if needed
            if stopped: → State 3
```

**IsCloaked flag at offset 0x430:** The spy plane sets `param_1[0x10C]` (byte at 0x430)
to 1, making it invisible to enemies. This is the same cloaking mechanism used by
Mirage Tanks and other stealth units.

## 29. Takeoff & Landing Step Functions

### Ascent_Step (0x004CE680) — Per-Tick Takeoff

Two-phase takeoff creates realistic lift-off behavior:

```
function Ascent_Step(this):
    altitude = owner.GetAltitude()
    if on bridge: altitude -= BridgeHeight

    // Clear transition flags
    IsDescending = false
    IsAscending = false

    target = TargetAltitude  // set to cruiseHeight by Begin_Takeoff

    // Phase 1: Random wobble during initial climb (bottom 2/3)
    if altitude > target - target/3:
        Set random facing
        Update timer

    // Phase 2: Face destination at full speed (top 1/3)
    elif altitude > target/2:
        facing = atan2(dest.Y - pos.Y, pos.X - dest.X)
        UpdateFacing(facing)
        TargetSpeedFraction = 1.0  // full speed
```

### Descent_Step (0x004CE840) — Per-Tick Landing (very complex)

```
function Descent_Step(this):
    if !IsLanding: return  // not descending

    altitude = owner.GetAltitude()
    if on bridge: altitude -= BridgeHeight

    // Speed control during descent
    if NOT ConsideredAircraft: speed = 0
    elif ConsideredAircraft AND altitude == 0 AND PitchAngle > 0:
        PitchAngle -= _DAT_007e89e8  // gradually level out on ground
        clamp PitchAngle >= 0

    // Check landing cell passability
    destCell = Get_Cell_At(destination)
    if destCell is NOT passable:
        // Can't land here — abort and find new cell
        Begin_Takeoff()
        find = Find_Nearby_Passable_Cell(near owner.pos)
        if find is NullCell:
            // Nowhere to go — CRASH
            Apply_Damage(CrashDamage)
            clear destination
            return
        Assign_Destination(find)  // redirect to passable cell
        return

    // Building at landing cell: verify it's our dock
    if building found at destCell AND building is in our dock list:
        proceed  // landing at helipad

    // Landing animation trigger (altitude < 300)
    if !flag_0x52 AND altitude < 300:
        flag_0x52 = true  // only trigger once
        Get ground height at position

        if ConsideredAircraft:
            play standard landing animation
        elif Carryall type:
            play carryall-specific landing animation

        play landing sound if has Strength

    // TOUCHDOWN (altitude <= ground height)
    if altitude <= ground_height AND PitchAngle <= 0:
        // On bridge check
        if on bridge cell:
            owner.IsOnBridge = true

        // Set final position to ground level
        owner.SetAltitude(ground_height)

        // Clear all flight state
        IsLanding = false
        IsAscending = false
        CurrentSpeedFraction = 0
        TargetSpeedFraction = 0

        // UPDATE CELL OCCUPANCY (8 surrounding cells)
        // Decrements old cell's aircraft counter
        // Increments new cell's aircraft counter
        for dir in 0..7:
            old_cell = owner.OldOccupancyCell + DirectionOffsets[dir]
            CellClass.AircraftCount[old_cell] -= 1
        owner.OccupancyCell = owner.GetCell()
        for dir in 0..7:
            new_cell = owner.OccupancyCell + DirectionOffsets[dir]
            CellClass.AircraftCount[new_cell] += 1

        // Clear destination if at dock
        if at destination cell (or building == dock):
            clear flags, Move_To(NullCoord), clear destination
```

**Cell occupancy tracking:** When an aircraft lands, it marks 8 surrounding cells (in all
cardinal+diagonal directions) in the CellClass at offset 0x122 (aircraft counter byte).
This prevents other aircraft from landing in adjacent cells. On takeoff, these counters
are decremented. This is a separate system from the airspace deconfliction in
Is_Cell_Free_For_Landing.

## 30. Rendering Matrix (0x004CFB00)

```
function Render_Matrix(this, out_matrix, out_cache_key):
    // Get cell slope data
    cell = Get_Cell_At(owner.Location)
    slope = cell.SlopeType  // offset 0x11C in CellClass

    // Aircraft ignore terrain slope!
    if TypeClass.ConsideredAircraft:
        slope = 0  // flat orientation always

    // Get facing matrix from voxel rendering system
    matrix = VXL_GetFacingMatrix()
    copy matrix → local (3x4 = 12 floats)

    // Apply Z-rotation from current heading
    heading = RateTimer::Current()  // game direction 0-65535
    z_angle = ((heading >> 10) + 1 >> 1 & 0x1F) - 8
    Matrix3x4_RotateZ(z_angle * _DAT_007e4408)

    // Build cache key for voxel render caching
    if out_cache_key != null AND *out_cache_key != -1:
        *out_cache_key = ((*out_cache_key * 64 + slope) * 32)
                       | ((heading >> 10) + 1 >> 1 & 0x1F)

    copy local → out_matrix
```

**Key insight for Rust engine:** Aircraft voxel rendering uses a **32-direction quantized
heading** (5 bits from the 16-bit facing), not the full facing resolution. The slope
component is always 0 for ConsideredAircraft. The cache key combines slope (6 bits) ×
heading (5 bits) = 192 unique orientations max for aircraft.

## 31. Horizontal Movement — Dive Bombing (0x004CEFB0)

The `Step_Movement` function (224 lines) contains the critical **dive bombing** mechanic:

```
function Step_Movement(this, dest_X, dest_Y, dest_Z):
    // ... heading calculation, FlyBy handling ...

    distance = Distance_To(destination)

    // ALTITUDE ADJUSTMENT BASED ON DISTANCE (dive bombing)
    if TypeClass.ConsideredAircraft AND distance < TypeClass.Speed:
        // Aircraft within weapon range: BEGIN DIVE
        dive_ratio = 1.0 - (distance / TypeClass.Speed)  // 0.0 at max range → 1.0 at target
        dive_altitude = dive_ratio * TypeClass.CruiseHeight / 3

        TargetAltitude = dive_altitude  // descend to 1/3 of cruise height at target
    else:
        TargetAltitude = TypeClass.CruiseHeight  // stay at full altitude

    // SPEED TIERS based on distance
    if distance < 0x80 (128 leptons):
        if JustArrived flag not set AND speed low:
            Begin_Landing()
        TargetSpeedFraction = 0.0  // stop

    elif distance < 0x200 (512 leptons):
        TargetSpeedFraction = 0.5  // half speed

    elif distance < 0x300 (768 leptons):
        TargetSpeedFraction = 0.75  // 3/4 speed

    // else: keep current speed (set by other logic)

    // AI visibility: if enemy shrouded, scatter
    if owner.IsVisible AND !IsHumanControlled AND IsShrouded:
        owner.Scatter()
```

**Dive bombing formula:** When an aircraft is within its weapon range (TypeClass.Speed
field, which doubles as both speed and range for aircraft), it progressively descends
to 1/3 of its cruise altitude. This creates the characteristic Harrier/Black Eagle
dive-bomb attack pattern. At the target, the aircraft is at `CruiseHeight / 3`.

**Speed tiers (distance-based deceleration):**
| Distance to Target | Speed Fraction |
|-------------------|----------------|
| > 768 leptons | unchanged |
| 512–768 leptons | 0.75 |
| 128–512 leptons | 0.5 |
| < 128 leptons | 0.0 (stop → land) |

## 32. Paradrop Payload Drop (0x00415C60)

```
function Drop_Payload(this):
    carried = Get_Carried_Unit()
    if carried is null: return

    Ammo -= 1  // consume one payload unit

    // Alternate drop direction: even ammo = right, odd = left
    aircraft_heading = GetFacing()
    if (Ammo & 1) == 0:
        drop_dir = heading + 0x3FFF  // 90° right
    else:
        drop_dir = heading - 0x3FFF  // 90° left

    // Calculate drop position
    drop_cell = aircraft.pos + (Cos(drop_dir), Sin(drop_dir))

    // Check if drop cell is passable for infantry
    if cell is passable:
        // Find sub-cell position within the cell
        subCell = CellClass::PlaceInfantryInCell(drop_cell)

        if subCell is valid:
            // Successful drop
            if carried.Unlimbo(subCell_coords):
                PlaySound(ParadropSound, aircraft.X)
                carried.ScatterTarget = subCell  // the dropped unit moves to its sub-cell
                if aircraft has team:
                    AddToTeam(carried)

                PayloadCount = 5  // ← RESET retry counter!
                record timing data
                return SUCCESS

    // Failed drop — cell not passable
    Re_Attach(carried)  // put the unit back in the aircraft
    carried.Destroy()   // kill the unit (can't place it)
    Ammo += 1           // refund the ammo
```

**PayloadCount (0x6D3) is a retry counter, NOT a unit count.** It allows up to 5
attempts to find a valid cell for each payload unit. On successful drop it resets to 5.
If all 5 attempts fail (no passable cells), the Mission_Rescue loop exits, and the
aircraft stops dropping and flies off.

**Alternating drop sides:** Units are dropped alternately to the LEFT and RIGHT of the
aircraft's heading, creating the characteristic V-shaped paradrop pattern seen in the
original game.

## 33. AI Aircraft Production — HouseClass__AI_Choose_Aircraft (0x004FEEE0)

This is a **HouseClass method**, not a standalone function (verified: `get_function_by_address 0x004FEEE0` → `HouseClass__AI_Choose_Aircraft`). It is called on a house instance to decide which aircraft type to build next.

```
function HouseClass__AI_Choose_Aircraft(this /* HouseClass* */):
    if house.CurrentAircraftBuild != -1: return  // already building

    // Phase 1: Count aircraft pad slots per type
    counts[100] = {0}     // needed per type
    min_cost[100] = {MAX}  // cheapest per type

    for each building owned by this house:
        if building.HasAircraftPad AND building is operational:
            pad_types = GetPadAircraftTypes(building)
            for each type in pad_types:
                if type.RTTI == AircraftTypeClass (0x10):
                    counts[type.ArrayIndex]++
                    min_cost[type.ArrayIndex] = min(min_cost, building.Cost)

    // Phase 2: Subtract existing aircraft
    for each aircraft in global array:
        if aircraft.House == this house AND CanProduce(aircraft):
            type_idx = aircraft.Type.ArrayIndex  // offset 0xDF8
            counts[type_idx]--

    // Phase 3: Select best type to build
    candidates = []
    max_needed = -1
    cheapest_idx = -1

    for each AircraftType:
        if counts[type] > 0 AND house.CanBuild(type):
            if counts[type] > max_needed:
                max_needed = counts[type]
                candidates.clear()
            candidates.append(type)
            if min_cost[type] < current_cheapest:
                cheapest_idx = type

    // Phase 4: Probability-weighted selection
    weight = RulesClass.AircraftBuildWeight[house.AILevel]  // offset 0x13F4
    roll = Random(0, MAX_INT)

    if roll * probability_scale <= weight * scale2:
        // Most-needed type wins
        chosen = candidates[Random(0, candidates.len-1)]
    else:
        // Cheapest type wins
        chosen = cheapest_idx

    house.CurrentAircraftBuild = chosen  // offset 0x5654
```

**AI behavior:** The AI replaces aircraft on a per-pad basis. Each airfield building
tracks which aircraft types it can produce. The AI counts empty pads, subtracts existing
aircraft, and builds the most-needed type. A probability weight (from RulesClass, indexed
by AI difficulty level) determines whether to pick the most-needed type or the cheapest
type. This explains why AI always seems to rebuild Harriers immediately after losing them.

## 34. Fire_At — Bullet Velocity Inheritance (0x00415EE0)

AircraftClass overrides Fire_At to handle two special cases: paradrop payload and
bullet velocity adjustment for moving aircraft.

```
function Fire_At(this, target):
    // PARADROP: if carrying passengers, drop instead of firing
    if this.PassengerCount (offset 0x118) != 0:
        return Drop_Payload()

    // NORMAL FIRE: delegate to parent
    bullet = TechnoClass::Fire_At(target)
    if bullet == null: return null

    if bullet.Type.ROT == 0:  // UNGUIDED projectile (bombs, bullets)
        // VELOCITY INHERITANCE from aircraft speed
        speed = TypeClass.GetHeight()  // cruise altitude as proxy
        scale = speed / locomotor.GetCurrentSpeed()

        // Scale bullet velocity by aircraft speed factor
        bullet.VelocityX *= scale
        bullet.VelocityY *= scale
        bullet.VelocityZ *= scale

        // Apply facing correction
        facing = GetFacing()
        angle = (facing - 0x3FFF) * π/32768
        bullet.VelocityX *= Sin(angle)
        bullet.VelocityY *= Sin(angle)

        // Apply fixed depression angle (~6°)
        bullet.VelocityX *= Sin(-0.098)  // ~-5.6°
        bullet.VelocityY *= Sin(-0.098)
        bullet.VelocityZ *= Cos(-0.098)

        // Re-orient based on current heading
        heading_angle = GetFacing() → radians
        horizontal_speed = CalcHorizontalSpeed()
        bullet.VelocityX = Sin(heading) * horizontal_speed
        bullet.VelocityY = -Cos(heading) * horizontal_speed

    elif bullet.Type.ROT == 1:  // GUIDED missile
        // Direct trajectory from aircraft to target
        target_pos = target.GetLocation()
        delta = this.Location - target_pos
        heading = atan2(-delta.Y, delta.X)

        // Set bullet velocity along the heading
        horizontal_speed = CalcHorizontalSpeed()
        bullet.VelocityX = Sin(heading) * horizontal_speed
        bullet.VelocityY = -Cos(heading) * horizontal_speed

        // Vertical component
        vertical_speed = CalcVerticalSpeed()
        elev_angle = atan2(delta.Z, horizontal_distance)
        bullet.VelocityX *= Sin(elev_angle)
        bullet.VelocityY *= Sin(elev_angle)
        bullet.VelocityZ = Cos(elev_angle) * vertical_speed

        // Scale to weapon speed
        weapon_speed = TypeClass.GetWeapon(0).Speed
        scale = weapon_speed / CalcCurrentSpeed()
        bullet.VelocityX *= scale
        bullet.VelocityY *= scale
        bullet.VelocityZ *= scale

    // SHROUD REVEAL after firing
    if IsHumanPlayer:
        if position or target is shrouded:
            RevealFog(this.Location, RulesClass.ShroudRevealRadius)
```

**Bullet velocity inheritance:** Unguided projectiles (bombs) from aircraft have their
velocity adjusted by the aircraft's speed. This means a Harrier's bombs carry forward
momentum — faster aircraft throw bombs farther ahead. The fixed depression angle of ~6°
means bombs always arc slightly downward.

**Guided missiles** are launched directly at the target with full 3D trajectory calculation
including elevation angle. Weapon speed from the TypeClass scales the final velocity.

## 35. Weapon Selection — Firing Arc (0x0041A9E0)

```
function What_Weapon_Should_I_Use(target):
    fire_error = TechnoClass::GetFireError(target)
    if fire_error != CAN_FIRE: return fire_error

    // Factory-produced aircraft without payload: use Secondary weapon
    if IsFromFactory (0x6C9) AND PassengerCount == 0:
        return 1  // SECONDARY weapon

    // FIRING ARC CHECK: aircraft can only fire within ±11.25° of heading
    facing = GetFacing()      // 16-bit direction
    target_dir = CalcDirection(target)
    facing_diff = abs(short)(facing - target_dir)

    if facing_diff > 0x800:   // > 11.25° off-target
        return 2              // CANNOT FIRE — too far off angle

    return 0  // PRIMARY weapon, clear to fire
```

**±11.25° firing arc:** Aircraft have an extremely narrow forward firing cone. They can
only engage targets within ~11° of their heading. This is why aircraft must fly directly
at targets and explains the approach/attack cycle in Mission_Attack.

**Weapon selection:** Factory-produced aircraft (IsFromFactory flag at 0x6C9) that aren't
carrying payload default to their Secondary weapon. Spawned aircraft (Hornets, missiles)
use Primary. This explains why some aircraft definitions have both Primary and Secondary
weapons assigned differently.

## 36. What_Action — Cursor/Command Logic (0x00417CC0)

```
function What_Action(target, flags):
    action = FootClass::What_Action(target, flags)

    // AirportBound override: convert Dock to Move
    if action == DOCK(0x1A) AND TypeClass.AirportBound:
        action = MOVE(0)

    // Carryall: allied FootClass pickup
    if TypeClass.Carryall AND IsHumanPlayer:
        if action == DOCK or MOVE:
            if target is allied AND target is FootClass AND not carrying:
                action = CARRY(0x11)  // pickup cursor

    // Move action: get target cell, check weapon availability
    if action == MOVE(0):
        target_cell = target.GetCell()
        action = CalculateMoveAction(target_cell, flags)

    // Attack: redirect to vehicle occupant if target is on a bunker
    if action == ATTACK(2):
        if target.HasOccupant AND target.IsOnCell AND !target.Occupant.IsArcing:
            redirect to occupant

    // Guard with GuardArea target
    if action == GUARD(5):
        if target.TypeClass.GuardArea: action = ATTACK(2)

    // Helipad dock check for human player
    if IsHumanPlayer AND action == DOCK(7):
        if target is building AND target.TypeClass.UnitRepair:
            // Check if this aircraft's dock list includes this building
            for type in TypeClass.DockList:
                if type == target.TypeClass:
                    if aircraft is close AND pad is free:
                        action = DOCK(3)  // direct dock
                    break
            if not in dock list:
                action = REPAIR(0x1F)  // repair cursor instead

    // Carryall: can't dock at helipad with NoDeploy flag
    if TypeClass.Carryall AND action == CARRY(0x11):
        cell = target.GetCell()
        building = Get_Building_At(cell)
        if building has Helipad AND NoDeploy:
            return NONE(0)  // can't interact

    return action
```

## 37. Vision/Sight System (0x0041ADF0)

```
function Update_Sight(this, gap_mode, fog_layer):
    sight = TypeClass.Sight  // offset +0x5E8
    altitude = GetAltitude()

    if altitude == 0:
        sight = 1  // ON GROUND: minimal sight (1 cell)
    elif sight == 0 AND FogOfWar enabled:
        // Altitude-dependent dual-reveal
        half_flight = RulesClass.FlightLevel (0x7B4) / 2
        Reveal(position, RulesClass.ShroudRevealRadius, House,
               altitude < half_flight)  // short-range layer
        Reveal(position, RulesClass.ShroudRevealRadius, House,
               altitude >= half_flight) // long-range layer
        return
    else:
        // Normal sight reveal
        Reveal(position, sight, House, gap_mode, fog_layer)
        Reveal(position, sight, House, gap_mode, fog_layer ^ 1)  // both layers
```

**Altitude-dependent sight:** Aircraft on the ground have minimal sight (1 cell only).
In flight, if the unit has Sight=0, the engine uses FlightLevel/2 as a threshold to
determine whether to reveal the "close" or "far" shroud layer separately. This means
aircraft at different altitudes reveal different amounts of the map.

## 38. Mission Queue Protection — Paradrop Immunity

Three vtable overrides work together to prevent paradrop aircraft from being interrupted:

### Override_Mission (vtable[123] = 0x0041B870)
```
function Override_Mission():
    if current_mission != ParaDropApproach (0x1E):
        IsStrafe = false  // clear strafe flag on mission change
    Parent::Override_Mission()
```

### Assign_Mission (vtable[124] = 0x0041B9F0)
```
function Assign_Mission(new_mission):
    if current_mission in {QMove, Open, Rescue, ParaDropApproach, ParaDropOverfly}:
        if no owner building:
            if new_mission NOT in {QMove, Open, Rescue, ParaDropApproach, ParaDropOverfly}:
                return  // BLOCK — don't interrupt paradrop cycle!
    Parent::Assign_Mission(new_mission)
```

### Queue_Mission (vtable[122] = 0x0041BA90)
Same logic as Assign_Mission — blocks non-paradrop missions during paradrop operations.

**Effect:** Once a cargo plane enters the paradrop mission cycle, it CANNOT be given any
other mission (attack, move, guard, etc.) until the cycle completes naturally. Only other
paradrop-related missions can interrupt. This prevents player commands or AI decisions from
accidentally aborting a paradrop in progress.

## 39. Serialization — Save/Load (0x0041B430)

```
function Load(this):
    // Deserialize from parent
    Parent::Load()

    // Remove from global tracking array
    Remove_From_AircraftArray()

    // Deserialize aircraft-specific data
    FootClass::Load()  // loads base fields

    // Restore all 5 vtable pointers (overwritten during deserialization)
    vtable[0] = &vtable__AircraftClass
    vtable[1] = &vtable__AircraftClass__secondary_4
    vtable[2] = &vtable__AircraftClass__secondary_8
    vtable[3] = &vtable__AircraftClass__secondary_12
    vtable[0x1B0] = &vtable__AircraftClass__secondary_1728

    // Restore locomotor COM pointer (AddRef new, Release old)
    old_loco = this.Locomotor
    this.Locomotor = saved_loco
    saved_loco.AddRef()
    old_loco.Release()

    // Re-register in global array
    Register_In_AircraftArray()

    // POINTER SWIZZLE: resolve save-file pointer IDs to runtime pointers
    Swizzle(&this.Type)       // offset 0x6C4 (AircraftTypeClass*)
    Swizzle(&this.CachedDock) // offset 0x6CC (BuildingClass*)
```

**Swizzle system:** During save, pointers are replaced with unique IDs. During load,
the Swizzle function resolves these IDs back to runtime pointers. Two aircraft-specific
pointers need swizzling: the TypeClass and the CachedDock building.

## 40. SpawnManagerClass — Carrier/Spawner Integration

### Struct Layout (from Constructor at 0x006B6C90)

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| 0x00-0x0C | void*[4] | vtables | 4 COM interface vtables |
| 0x24 | TechnoClass* | Owner | Parent unit (Carrier, Destroyer, V3) |
| 0x28 | TechnoTypeClass* | SpawnType | Type of aircraft to spawn |
| 0x2C | int | MaxSpawns | SpawnsNumber from INI |
| 0x30 | int | SpawnRegenRate | Frames between regen |
| 0x34 | int | SpawnReloadRate | Frames between reloads |
| 0x38-0x4B | DynVec | SpawnControls | Array of SpawnControl structs |
| 0x4C | int | MaxActive | Max simultaneously active |
| 0x48 | int | CurrentActive | Currently active count |
| 0x50 | Timer | RegenTimer | Countdown to next regen |
| 0x58 | int | RegenDelay | Frames per regen (from SpawnRegenRate) |
| 0x5C | Timer | ReloadTimer | Countdown to next reload |
| 0x64 | int | State | Current spawn state |
| 0x68 | AbstractClass* | CurrentTarget | Active target |
| 0x6C | AbstractClass* | PendingTarget | Queued target |

### SpawnControl Sub-struct (per spawn slot, 0x18 bytes)

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| 0x00 | AircraftClass* | Aircraft | The spawned aircraft instance |
| 0x04 | int | State | 0=idle, 1=active, etc. |
| 0x08 | Timer | Timer | Regen/reload countdown |
| 0x10 | int | Unknown | |
| 0x14 | int | IsMissile | 1 for DMISL/V3ROCKET, 0 for HORNET/ASW |

### Spawn Creation (from Constructor)

```
for i in 0..MaxSpawns:
    spawn_control = new SpawnControl()
    spawn_type = SpawnType  // e.g., HORNET for Carrier

    // Create the aircraft instance
    aircraft = Owner.CreateObject(spawn_type)
    spawn_control.Aircraft = aircraft

    // Check if this is a missile type
    if SpawnType == RulesClass.V3Type (0x4E0)
       OR SpawnType == RulesClass.DMISLType (0x514)
       OR SpawnType == RulesClass.CMISLType (0x548):
        spawn_control.IsMissile = 1
    else:
        spawn_control.IsMissile = 0

    // Initialize the aircraft
    aircraft.Limbo()  // start in limbo
    aircraft.SpawnOwner = Owner  // offset 0x2D4 in TechnoClass

    // Add to spawn list
    SpawnControls.Add(spawn_control)
```

**Missile vs Aircraft spawns:** The engine distinguishes between missile spawns
(V3Rocket, DMISL, CMISL) and aircraft spawns (Hornet, ASW/Osprey) using the IsMissile
flag. Missiles use RocketLocomotionClass and fly directly to the target. Aircraft use
FlyLocomotionClass and return to the carrier after attacking.

## 41. SpawnManagerClass::AI — Full State Machine (0x006B7230)

**Size:** 2359 bytes (456 lines decompiled)
**Called from:** TechnoClass::AI_Update via vtable+0x5C, every tick
**SpawnManager stored at:** TechnoClass+0x2D0
**Tick rate:** Every 10 game frames (not every tick)
**Active in YR:** Yes — Carrier, Destroyer, Dreadnought, V3, Boomer all use this

### Corrected SpawnManagerClass Struct Layout (from constructor assembly at 0x006B6C90)

| Offset | Type | Name | Description |
|--------|------|------|-------------|
| 0x00 | void* | vtable | Primary (0x007F3650) |
| 0x04 | void* | vtable2 | Secondary 1 (0x007F3634) |
| 0x08 | void* | vtable3 | Secondary 2 (0x007F362C) |
| 0x0C | void* | vtable4 | Secondary 3 (0x007F3624) |
| 0x24 | TechnoClass* | Owner | Parent unit (Carrier, V3, etc.) |
| 0x28 | TechnoTypeClass* | SpawnType | e.g., HORNET, DMISL |
| 0x2C | int | MaxSpawns | From SpawnsNumber INI key |
| 0x30 | int | SpawnRegenRate | Frames for regen after death |
| 0x34 | int | SpawnReloadRate | Frames between launches |
| 0x38 | DynVec vtable | SpawnVec_vtable | DynVec<SpawnControl*> vtable (0x7F36B4) |
| 0x3C | SpawnControl** | SpawnArray | Pointer to array of SpawnControl* |
| 0x48 | int | SpawnCount | Active slot count |
| 0x50 | Timer | TickTimer | AI tick timer (10-frame cadence) |
| 0x58 | int | TickDelay | Always 10 frames |
| 0x5C | Timer | ReloadTimer | Cooldown between launches |
| 0x64 | int | ReloadDelay | Current reload delay |
| 0x68 | AbstractClass* | CurrentTarget | Target being attacked |
| 0x6C | AbstractClass* | PendingTarget | Queued next target |
| 0x70 | int | ManagerState | 0=Idle, 1=Launching, 2=Attacking |

### SpawnControl States (7-state machine per slot)

| State | Name | Description |
|-------|------|-------------|
| 0 | IDLE | Aircraft docked, ready to launch |
| 1 | LAUNCHED_MISSILE | Missile in flight (timer-based, awaiting retarget) |
| 2 | ACTIVE | Aircraft launched, heading to staging area |
| 3 | ATTACKING | Aircraft engaging target (Attack mission) |
| 4 | RETURNING | Aircraft heading back to carrier |
| 6 | REGENERATING | Aircraft docked after return, reloading ammo/health |
| 7 | DEAD | Aircraft destroyed, waiting for regen timer to recreate |

### Manager States (3-state outer machine)

| State | Name | Description |
|-------|------|-------------|
| 0 | IDLE | No target. Checks PendingTarget. If found, validates CanFire → state 1 |
| 1 | LAUNCHING | Waits for all spawns to be in state 2 or 7. Then sends attack commands → state 2 |
| 2 | ATTACKING | Monitors all spawns. When none in state 3 or 4 → state 0 |

### Launch Sequence (State 0 → 2)

```
Per spawn slot in IDLE state:
    1. Check CurrentTarget exists
    2. Check ReloadTimer expired
    3. Check ManagerState != 2 (not already all attacking)

    MISSILE CHECK (IsMissile == 1):
        Owner locomotor must not be Is_Moving() or Is_Busy()

    OWNER ALIVE CHECK:
        Owner must not be EMP'd (offset 0x6AD)

    SET RELOAD DELAY:
        if Owner.Type.BalloonHover: delay = 9 frames
        else: delay = 20 frames (0x14)

    ALTERNATE BURST SIDE (missiles with burst > 1):
        Owner.CurrentBurstIndex = slot_index & 1  (even=left, odd=right)

    → State 2 (ACTIVE)

    SPAWN POSITION:
        if weapon.Projectile.IsArcing == false:
            position = Owner.GetFLH(weapon)  // Fire-Launch-Height
            if SpawnType == CMISLType: apply special offset
        else:
            position = Owner.GetFireLocation()

    PLACE AIRCRAFT:
        aircraft.Unlimbo(position, Owner.Facing)
        if SpawnType == CMISLType: play launch animation

    ASSIGN MISSION:
        if IsMissile:
            aircraft.Assign_Destination(CurrentTarget)
            aircraft.Mission = Move  (missiles fly straight to target)
        else:
            aircraft.Assign_Destination(cell near carrier)
            aircraft.Mission = Move  (aircraft orbit carrier first)
```

### Return-to-Carrier Docking (State 4)

```
Per spawn slot in RETURNING state:
    if aircraft has ammo AND target exists:
        → State 3 (re-attack)

    carrier_cell = Owner.GetCell()
    aircraft_cell = aircraft.GetCell()

    if at same cell AND altitude_diff < 20 leptons:
        // DOCKED — land on carrier
        aircraft.Limbo()
        → State 6 (REGENERATING)
        start regen timer = SpawnReloadRate

    else:
        // Still flying — redirect to carrier
        aircraft.Assign_Destination(Owner)
        aircraft.ClearTarget()
        aircraft.Mission = Move
```

### Regeneration and Respawn

```
State 6 (REGENERATING) — timer = SpawnReloadRate:
    When timer expires:
        aircraft.Ammo = TypeClass.MaxAmmo  // refill
        aircraft.Health = TypeClass.MaxHealth
        → State 0 (IDLE, ready to launch again)

State 7 (DEAD) — timer = SpawnRegenRate:
    When timer expires:
        // CREATE NEW aircraft (old one was destroyed)
        new_aircraft = Owner.CreateObject(SpawnType)

        if SpawnType in {V3Type, DMISLType, CMISLType}:
            IsMissile = 1
        else:
            IsMissile = 0

        new_aircraft.Limbo()
        new_aircraft.SpawnOwner = Owner
        → State 0 (IDLE)
```

### Launch Timing Constants

| Spawn Type | RulesClass Offset | Delay Calculation |
|-----------|-------------------|-------------------|
| V3 Rocket | 0x4B0 + 0x4B4 | V3LaunchDelay + V3LaunchRandomDelay |
| Dreadnought Missile | 0x4E4 + 0x4E8 | DredLaunchDelay + DredLaunchRandomDelay |
| Cruise Missile (CMISL) | 0x548 | Special position offset |
| Aircraft (Hornet, ASW) | n/a | Standard 20-frame reload delay |
| BalloonHover carriers | n/a | 9-frame reload delay |

### RulesClass Spawn Type Pointers

| Offset | INI Key | YR Default |
|--------|---------|------------|
| 0x4E0 | V3Type | V3ROCKET AircraftTypeClass* |
| 0x514 | DMISLType | DMISL AircraftTypeClass* |
| 0x548 | CMISLType | CMISL AircraftTypeClass* |

## 42. V3/DMISL Weapon Readiness (vtable[20-21])

Two AircraftClass vtable overrides check weapon readiness specifically for missile spawns:

### Is_Weapon_Ready (vtable[20] = 0x0041B980)

```
function Is_Weapon_Ready(this):
    if Type == RulesClass.V3Type OR Type == RulesClass.DMISLType:
        // Missiles: check if locomotor is NOT busy
        return !Locomotor.Is_Busy()
    else:
        return Parent::Is_Weapon_Ready()
```

### Is_Firing_Possible (vtable[21] = 0x0041B920)

```
function Is_Firing_Possible(this):
    if Type == RulesClass.V3Type OR Type == RulesClass.DMISLType:
        // Missiles: check if locomotor IS busy (fired and in flight)
        return Locomotor.Is_Busy()
    else:
        return Parent::Is_Firing_Possible()
```

**Effect:** V3 rockets and Dreadnought missiles can only be "fired" (launched from the
SpawnManager) when their locomotor is free. Once launched, Is_Firing_Possible returns
true (the missile is "firing" = in flight). This prevents double-launching.

## 43. Paradrop NavCom Protection (vtable[125] = 0x0041BB30)

```
function Set_NavCom_Override(this, mission, target, ???):
    if current_mission in {QMove, Open, Rescue, ParaDropApproach, ParaDropOverfly}:
        if no owner building:
            if new_mission NOT in {QMove, Open, Rescue, ParaDropApproach, ParaDropOverfly}:
                return  // BLOCK

    FootClass::Set_NavCom_With_Suspend(mission, target, ???)
```

This is the FOURTH paradrop protection override (joining Queue_Mission, Assign_Mission,
Override_Mission). Together, these four functions form a complete firewall that prevents
ANY non-paradrop command from interrupting a cargo plane during its drop cycle.

## 44. Rotor System

### Asset Loading (0x0041CAF0)

```
function LoadRotorAssets():
    DAT_00889F5C = LoadFileFromMIX("RROTOR.SHP")  // right rotor blade
    DAT_00889F60 = LoadFileFromMIX("LROTOR.SHP")  // left rotor blade
```

**Rotor SHP files:** Two SHP frame sequences are loaded at game startup:
- `RROTOR.SHP` — clockwise rotor animation (right side)
- `LROTOR.SHP` — counter-clockwise rotor animation (left side)

Controlled by two AircraftTypeClass INI keys:
- `Rotors=yes` — enables standard dual-rotor rendering (Nighthawk, Hind)
- `CustomRotor=yes` — uses the type's own SHP instead of the global rotor files

**Draw queue integration:** Rotors are added to a 2D sprite overlay queue (DAT_00b0cec8,
max 500 entries) via FUN_006d9ef0. Each entry stores: {SHP_pointer, screen_x, screen_y}.
The rotor frame advances each game tick for spinning animation. Rotors are drawn ON TOP
of the voxel aircraft after the main 3D render pass.

## 45. RulesClass Aircraft Globals (confirmed offsets)

| Offset | INI Key | Type | Description |
|--------|---------|------|-------------|
| 0x7B4 | FlightLevel | int | Default cruise altitude for aircraft |
| 0x54C | ParadropRadius | int | Distance threshold for paradrop trigger |
| 0x4B0 | V3LaunchDelay | int | V3 rocket base launch delay (frames) |
| 0x4B4 | V3LaunchRandomDelay | int | V3 rocket random extra delay |
| 0x4E0 | V3Type | TypeClass* | V3 rocket aircraft type pointer |
| 0x4E4 | DredLaunchDelay | int | Dreadnought missile base launch delay |
| 0x4E8 | DredLaunchRandomDelay | int | Dreadnought random extra delay |
| 0x514 | DMISLType | TypeClass* | DMISL aircraft type pointer |
| 0x548 | CMISLType | TypeClass* | CMISL cruise missile type pointer |
| 0xFA8 | CrashDamage | WarheadType* | Warhead used for aircraft crash explosion |
| 0x13F4 | AircraftBuildWeight | int[n] | AI probability weight for aircraft production (indexed by difficulty) |
| 0x1478 | GuardAreaRadius | int | Search radius for aircraft target cell finding (leptons) |
| 0x17DC | ExtraAircraftLight | int | Extra lighting for airborne aircraft |
| 0x17E1 | AircraftFlyByRules | bool | Controls fly-by strafing behavior |

## Sources

### Ghidra Functions Decompiled (68 total)
- `0x00413D20` — AircraftClass::Constructor (variant 1)
- `0x00414080` — AircraftClass::Constructor (variant 2, with CoCreateInstance)
- `0x00413F80` — AircraftClass::InitFromType
- `0x00414290` — AircraftClass::Destructor
- `0x00414BB0` — AircraftClass::AI (1544 bytes)
- `0x004144B0` — AircraftClass::Draw_It (1052 bytes)
- `0x00417FE0` — AircraftClass::Mission_Attack (3445 bytes)
- `0x004166C0` — AircraftClass::Mission_Move (1031 bytes)
- `0x0041A5C0` — AircraftClass::Mission_Guard (886 bytes)
- `0x004151E0` — AircraftClass::Mission_Hunt (929 bytes)
- `0x004155F0` — AircraftClass::Mission_ParaDropApproach (461 bytes)
- `0x004157C0` — AircraftClass::Mission_ParaDropOverfly (274 bytes)
- `0x004176F0` — AircraftClass::Enter_Idle_Mode (1210 bytes)
- `0x004190B0` — AircraftClass::Receive_Radio
- `0x004196B0` — AircraftClass::ActionOnCell
- `0x0041BBD0` — AircraftClass::FindBuildingToDock (96 bytes)
- `0x00414310` — AircraftClass::Unlimbo (406 bytes)
- `0x0041B660` — AircraftClass::Detach (56 bytes)
- `0x00415B10` — AircraftClass::Can_Enter_Cell (336 bytes)
- `0x004165C0` — AircraftClass::ReceiveDamage (243 bytes)
- `0x00419C80` — AircraftClass::Mission_Sticky (1215 bytes) — helipad docking
- `0x00419B00` — AircraftClass::Is_Cell_Free_For_Landing — airspace deconfliction
- `0x00418E20` — AircraftClass::Find_Attack_Cell — target cell selection
- `0x004197C0` — AircraftClass::Find_Approach_Cell — weapon-range approach
- `0x0041A160` — AircraftClass::Find_Nearest_Friendly_Airfield — RTB destination
- `0x00416D50` — AircraftClass::Carryall_Move (4-state Carryall movement)
- `0x00416AF0` — AircraftClass::Carryall_Pickup — locomotor swap + drop-off
- `0x00415A50` — AircraftClass::Mission_QMove — fly off map
- `0x00417300` — AircraftClass::Mission_SpyPlane (907 bytes) — spy plane overfly
- `0x004158E0` — AircraftClass::Mission_Open — paradrop payload decrement
- `0x00415960` — AircraftClass::Mission_Rescue — paradrop unit release
- `0x004CD600` — FlyLocomotionClass::Process (526 lines) — flight physics
- `0x004CC9A0` — FlyLocomotionClass::Constructor — struct layout
- `0x004CCA90` — FlyLocomotionClass::Is_Moving — pitch angle check
- `0x004CCB40` — FlyLocomotionClass::Layer (306B) — main tick + Process call
- `0x004CCC80` — FlyLocomotionClass::Move_To_Coord (835B) — set destination + takeoff
- `0x004CFE50` — FlyLocomotionClass::Is_On_Floor — 4-state floor check
- `0x004CFE20` — FlyLocomotionClass::Get_Height — cruise altitude
- `0x004CFA70` — FlyLocomotionClass::Begin_Landing — initiate descent
- `0x004CF950` — FlyLocomotionClass::Begin_Takeoff — initiate ascent
- `0x004CFB00` — FlyLocomotionClass::Render_Matrix (265B) — voxel orientation for drawing
- `0x004CCFD0` — FlyLocomotionClass::Emergency_Relocate (711B) — find safe cell when displaced
- `0x004CE680` — FlyLocomotionClass::Ascent_Step — two-phase takeoff
- `0x004CE840` — FlyLocomotionClass::Descent_Step — full landing sequence with cell occupancy
- `0x004CEFB0` — FlyLocomotionClass::Horizontal_Step (224 lines) — per-tick movement + dive bombing
- `0x004CD2A0` — FlyLocomotionClass::Altitude_Interpolate — altitude transitions + layer changes
- `0x004CD510` — FlyLocomotionClass::Speed_Update — water crash check for QMove aircraft
- `0x00415C60` — AircraftClass::Drop_Payload — paradrop unload with alternating L/R
- `0x004FEEE0` — HouseClass::AI_Choose_Aircraft — AI aircraft build decisions
- `0x00415EE0` — AircraftClass::Fire_At — bullet velocity inheritance + paradrop
- `0x0041A9E0` — AircraftClass::What_Weapon_Should_I_Use — weapon selection + firing arc
- `0x00417CC0` — AircraftClass::What_Action (692B) — cursor/command logic
- `0x0041ADC0` — AircraftClass::Scatter — delegates to locomotor
- `0x0041ADF0` — AircraftClass::Update_Sight — altitude-dependent vision
- `0x0041BA90` — AircraftClass::Queue_Mission_Override — paradrop protection
- `0x0041B870` — AircraftClass::Override_Mission — strafe flag cleanup
- `0x0041B9F0` — AircraftClass::Assign_Mission — paradrop protection
- `0x0041B430` — AircraftClass::Load (395B) — save/load with swizzle
- `0x006B6C90` — SpawnManagerClass::Constructor — spawn slot creation
- `0x006B7B90` — SpawnManagerClass::SetTarget — target assignment
- `0x004DEBB0` — FootClass::ReceiveEMP — crash-state initiation (shared with EMP)
- `0x006B7230` — SpawnManagerClass::AI (2359B, 456 lines) — complete spawn state machine
- `0x006B7100` — SpawnManagerClass::Kill_All_Spawns — destroy all active spawns
- `0x006B6C90` — SpawnManagerClass::Constructor — assembly-level struct layout
- `0x0041B920` — AircraftClass::Is_Firing_Possible — V3/DMISL locomotor check
- `0x0041B980` — AircraftClass::Is_Weapon_Ready — V3/DMISL locomotor check (inverted)
- `0x0041BB30` — AircraftClass::Set_NavCom_Override — 4th paradrop protection
- `0x0041CAF0` — AircraftClass::LoadRotorAssets — RROTOR.SHP / LROTOR.SHP loading
- `0x006D9EF0` — Add_Sprite_To_Queue — rotor draw queue system
- `0x006F9E50` — TechnoClass::AI_Update (625 lines) — SpawnManager call traced
- `0x0041C8B0` — AircraftTypeClass::Constructor
- `0x0041CC20` — AircraftTypeClass::ReadINI
- `0x005B3060` — MissionClass::Mission_Dispatch (vtable mapping)

### VTable Reads
- Primary vtable at `0x007E22A4` (512 bytes + 256 bytes + targeted reads)
- Mission handler vtable range at `0x007E24A4` (indices 128–191)
- Docking/destination vtable at `0x007E2724` (8 entries)

### INI Files Checked
- `ini/rulesmd.ini` — AircraftTypes list, all unit sections
- `ini/artmd.ini` — Trailer, Rotors, SpawnDelay, Voxel settings

### Prior Research Documents Referenced
- `GAMEMD_ARCHITECTURE.md` — Class hierarchy, sizes
- `ADDRESS_MAP.md` — VTable addresses
- `LOCOMOTION_MATH_AND_CONSTANTS.md` — FlyLocomotionClass CLSID, FlightLevel
- `FIRE_AT_ANALYSIS.md` — Ammo handling, spawner weapons
- `MISSION_REPAIR_AND_PRODUCE_GHIDRA_REPORT.md` — Reload protocol
- `HARVESTER_DOCK_UNLOAD.md` — Radio docking protocol
- `NAVAL_SYSTEM_RESEARCH.md` — Carrier/Dreadnought spawning
- `DRAW_ORDER_DEPTH_SYSTEM.md` — Aircraft rendering layer
- `HOUSECLASS_GHIDRA_REPORT.md` — Aircraft factory pointer, count tracking
