# Garrison (Building Occupation) System -- Ghidra Report

Reverse-engineered from `gamemd.exe`. Covers how infantry garrisons civilian/military
buildings: eligibility checks, entry flow, occupant storage, weapon selection,
damage/ROF modifiers, range override, round-robin firing, and visual rendering.

## Overview

The garrison system allows infantry with `Occupier=yes` to enter buildings with
`CanBeOccupied=yes`. Garrisoned infantry fire from the building using their
`OccupyWeapon` (or primary weapon as fallback), with damage/ROF/range governed
by global `[CombatDamage]` multipliers. Occupants cycle round-robin per shot.

---

## 1. INI Configuration

### BuildingTypeClass (rules.ini)

| Offset   | INI Key              | Type | Default | Description |
|----------|----------------------|------|---------|-------------|
| `0x157B` | `CanBeOccupied`      | bool | false   | Infantry can garrison this building |
| `0x157C` | `CanOccupyFire`      | bool | false   | Garrisoned infantry can fire from building |
| `0x1580` | `MaxNumberOccupants` | int  | 0       | Max infantry that can garrison simultaneously |
| `0x1584` | `ShowOccupantPips`   | bool | false   | Show pip indicators for occupants |
| `0x1588` | `MuzzleFlash0..N`    | 2xint| —       | Fire port pixel offsets (X,Y) per occupant slot |
| `0x15D8` | `DamageFireOffset0..7` | 2xint | —    | Damage smoke positions (up to 8) |

`MuzzleFlash` positions are read as `MuzzleFlashN=X,Y` (N = 0..MaxNumberOccupants-1),
stored as two ints (8 bytes) per entry starting at `+0x1588`.

### BuildingTypeClass (art.ini)

| Offset   | INI Key              | Type   | Description |
|----------|----------------------|--------|-------------|
| `0xEF4`  | `Height`             | int    | Building height in z-levels |
| `0xEF8`  | `OccupyHeight`       | int    | Z-levels the occupation extends |
|          | `ActiveAnimGarrisoned`| string | Animation played when building is garrisoned |
|          | `AddOccupy0..N`      | cell offsets | Cells marked occupied when garrisoned |
|          | `RemoveOccupy0..N`   | cell offsets | Cells unmarked when garrisoned |

### InfantryTypeClass (rules.ini)

| Offset   | INI Key            | Type            | Description |
|----------|--------------------|-----------------|-------------|
| `0xEB4`  | `Occupier`         | bool            | Can garrison `CanBeOccupied` buildings |
| `0xEB5`  | `Assaulter`        | bool            | Can assault enemy buildings |
| `0xE04`  | `OccupyWeapon`     | WeaponTypeClass*| Weapon used when garrisoned (resolved by name) |
| `0xE20`  | `EliteOccupyWeapon`| WeaponTypeClass*| Elite version of garrison weapon |
| `0xEAC`  | `OccupyPip`        | —               | Pip shown for this infantry when garrisoned |

### RulesClass [CombatDamage] (rules.ini)

| Offset   | INI Key                    | Type  | Description |
|----------|----------------------------|-------|-------------|
| `0x0F40` | `OccupyDamageMultiplier`   | float | Damage multiplier for garrison fire |
| `0x0F44` | `OccupyROFMultiplier`      | float | ROF divisor for garrison fire |
| `0x0F48` | `OccupyWeaponRange`        | int   | Weapon range in cells (replaces weapon range) |
| `0x0F4C` | `BunkerDamageMultiplier`   | float | Damage multiplier for bunker passengers |
| `0x0F50` | `BunkerROFMultiplier`      | float | ROF divisor for bunker passengers |
| `0x0F54` | `BunkerWeaponRangeBonus`   | int   | Range bonus for bunker passengers |
| `0x0F58` | `OpenToppedDamageMultiplier`| float| Damage multiplier for open-topped passengers |
| `0x0F5C` | `OpenToppedRangeBonus`     | int   | Range bonus for open-topped passengers |
| `0x0F60` | `OpenToppedWarpDistance`    | int   | Warp distance for open-topped |

### RulesClass [General] (rules.ini)

| Offset   | INI Key              | Type | Description |
|----------|----------------------|------|-------------|
|          | `ThreatPerOccupant`  | int  | Threat value per occupant (default=10) |
|          | `BuildingGarrisonedSound` | sound | Sound played when building garrisoned |

---

## 2. BuildingClass Runtime Layout (Occupant Storage)

Occupants are stored in a `DynamicVectorClass<InfantryClass*>` starting at
BuildingClass byte offset `0x684`:

| Offset   | Field         | Type              | Description |
|----------|---------------|-------------------|-------------|
| `0x0684` | vtable        | ptr               | DynamicVectorClass vtable (0x007E43C8) |
| `0x0688` | Items         | InfantryClass**   | Heap-allocated array of infantry pointers |
| `0x068C` | Capacity      | int               | Allocated array capacity |
| `0x0690` | IsAllocated   | bool              | Heap allocation flag |
| `0x0694` | Count         | int               | **Current number of occupants** |
| `0x0698` | GrowStep      | int               | Growth increment (initialized to 10) |
| `0x069C` | CurrentFireIdx| int               | **Round-robin fire index** |

There is a second DynamicVectorClass at `0x066C` which is likely for YR's
`Bunker=yes` system (separate from garrison).

### Other BuildingClass fields:

| Offset   | Field         | Description |
|----------|---------------|-------------|
| `0x0520` | Type          | Pointer to BuildingTypeClass |
| `0x0702` | FirePortCount | Number of active fire port slots (byte) |
| `0x05EC` | FirePortSlots | Array of infantry pointers for fire ports |

---

## 3. Function Reference

### Entry/Exit

| Address      | Name | Description |
|-------------|------|-------------|
| `0x00457CE0` | `BuildingClass::CanDock` | **Primary garrison eligibility check** |
| `0x00522910` | `BuildingClass::AddGarrisonOccupant` | Adds infantry to occupant vector |
| `0x004D9290` | `FootClass::Mission_Enter` | Infantry mission handler for entering buildings (corrected 2026-05-28: was `0x005196A0` / `InfantryClass::Mission_Enter`; `0x005196A0` is mid-body of `InfantryClass__PerCellProcess` which starts at `0x519630`; verified via `get_function_by_address 0x005196A0` and `search_functions Mission_Enter` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| `0x00457DE0` | `BuildingClass::SellBuilding` | **Actual occupant ejection** (also runs on destruction) |
| `0x004575B0` | `BuildingClass::EjectOccupants` | Refunds upgrade-slot credits only — **does NOT eject occupants** despite the name |
| `0x0070FD70` | `BuildingClass::EnterTransport` | Generic transport entry (single-occupant link) |
| `0x0070FE50` | `BuildingClass::ExitTransport` | Generic transport exit |

### Combat

| Address      | Name | Description |
|-------------|------|-------------|
| `0x006FDD50` | `TechnoClass::Fire_At` | Shared fire function (garrison damage/ROF applied here) |
| `0x006FCFA0` | `TechnoClass::GetROF` | ROF calculation with garrison division |
| `0x004526F0` | `BuildingClass::GetWeapon` | Returns OccupyWeapon from current occupant |
| `0x006F7220` | `TechnoClass::InRange` | Range check using OccupyWeaponRange |
| `0x006F8DF0` | `TechnoClass::Greatest_Threat` | Target scanning with garrison range override |

### Query

| Address      | Name | Description |
|-------------|------|-------------|
| `0x00458DD0` | `BuildingClass::IsOccupied` | CanBeOccupied && CanOccupyFire && Count > 0 |
| `0x004581F0` | `BuildingClass::GetOccupantCount` | Returns `*(this + 0x694)` |
| `0x00458E00` | `BuildingClass::GetHalfFoundationSize` | `min(width, height) / 2` |
| `0x004525F0` | `BuildingClass::CanGarrison` | **Misleading name**: actually gate passability check |

### Rendering

| Address      | Name | Description |
|-------------|------|-------------|
| `0x0043E7B0` | `BuildingClass::UpdateGarrisonFire` | Draws muzzle flash sprites at fire ports |

---

## 4. Garrison Entry Flow

### Step 1: Action Determination

Player right-clicks infantry on a `CanBeOccupied` building. The action system
(vtable `What_Action`) recognizes the infantry has `Occupier=yes` and the target
building has `CanBeOccupied=yes`, assigning the "Enter" action with the garrison cursor.

### Step 2: Eligibility Check -- `BuildingClass::CanDock` (0x00457CE0)

```
Conditions checked (all must pass):
1. BuildingTypeClass+0x157B (CanBeOccupied) != 0
2. Building mission != 0x12 (Construction) and != 0x13 (Selling)
3. Building is in valid map bounds and not cloaked

Then checks infantry type:
  IF Occupier (InfantryTypeClass+0xEB4):
    - Same owner OR building is MultiplayPassive (neutral civilian)
    - Occupant count < MaxNumberOccupants
    - Building is not at red HP
  IF Assaulter (InfantryTypeClass+0xEB5):
    - Infantry is NOT allied with building owner
    - Building HAS occupants (GetOccupantCount() != 0 — you clear an existing
      enemy garrison; you cannot assault an empty building) (corrected
      2026-07-18: was "Building has space", which is the opposite of the
      actual gate; contradicted this doc's own §13a two sections later.
      Binary shows `CALL [EDX+0x408]` (GetOccupantCount) then `TEST EAX,EAX;
      JZ <fail>` at 0x00457dc7-0x00457dd3 in `BuildingClass::CanDock` —
      verified via `disassemble_function 0x00457CE0` — ROOT_CAUSE:
      OPERATOR_OR_ORDER_DRIFT)
```

### Step 3: Infantry Navigation

Infantry receives Mission "Enter" (7) and pathfinds to the building. During
pathfinding, `InfantryClass::Can_Enter_Cell` (0x0051BF90) checks gate state
and other building flags for traversal cost.

### Step 4: Entry -- `InfantryClass::Mission_Enter` (0x005196A0)

When infantry arrives at the building cell:
1. Verifies `CanBeOccupied` on building type
2. Calls `BuildingClass::AddGarrisonOccupant`

### Step 5: Occupant Registration -- `BuildingClass::AddGarrisonOccupant` (0x00522910)

```
1. Verify infantry has Occupier=yes (InfantryTypeClass+0xEB4)
2. Remove infantry from map (Limbo via vtable+0xD4)
3. Add infantry pointer to DynamicVectorClass at BuildingClass+0x684
4. Increment occupant count at BuildingClass+0x694
5. Recalculate building power (FUN_0070f6e0)
6. If first occupant:
   - Set building mission via vtable+0x124
   - Play radar event (EVA_StructureGarrisoned)
   - Play BuildingGarrisonedSound (if local human player)
7. If infantry's owner has HouseClass+0x1EC set:
   - Clear infantry+0x691 and +0x690 (two status bytes)
```

**Note:** AddGarrisonOccupant does **NOT** transfer building ownership. Ownership
transfer for civilian/neutral buildings happens lazily on the next tick via
`BuildingClass::CheckAutoSellOrCivilian` (0x00458200), which is called
unconditionally from `BuildingClass::Update`. That function detects the
"occupants > 0 && owner == civilian" state and calls `ChangeOwner(Items[0]->Owner, 0)`.
See `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` for the full reconciliation flow.

If infantry has `Assaulter=yes` instead, a different path triggers (parachute spawn
of assault units rather than garrison entry).

### Step 6: Ejection -- `BuildingClass::EjectOccupants` (0x004575B0)

Triggered on building destruction or sell:
1. Iterates occupant vector backwards
2. Unlimbos each infantry near the building
3. Clears the occupant vector
4. Resets building ownership if needed

---

## 5. Garrison Fire Mechanics

### Weapon Selection -- `BuildingClass::GetWeapon` (0x004526F0, vtable+0x3F8)

```
1. Check fire port slots (BuildingClass+0x5EC, count at +0x702)
   - If fire port infantry exists, return its weapon

2. Check IsOccupied():
   - CanBeOccupied && CanOccupyFire && OccupantCount > 0

3. Get current occupant from round-robin index:
   - infantry = Items[CurrentFireIdx]  (at BuildingClass+0x688)
   - CurrentFireIdx at BuildingClass+0x69C

4. Check infantry veterancy (InfantryClass+0x150, float >= 2.0 = elite):
   - NOT elite: use InfantryTypeClass+0xE04 (OccupyWeapon)
   - Elite:     use InfantryTypeClass+0xE20 (EliteOccupyWeapon)

5. If OccupyWeapon is NULL: fall back to infantry's primary weapon (GetWeapon(0))

6. If not occupied: fall back to TechnoClass::GetWeapon (0x0070E140)
```

### Damage Calculation -- `TechnoClass::Fire_At` (0x006FDD50)

The damage chain for garrison fire:

```
baseDamage = WeaponTypeClass+0xA4 (Damage)
damage = baseDamage * HouseClass+0x188 (FirepowerMultiplier)

IF IsOccupied():
    damage *= RulesClass+0xF40 (OccupyDamageMultiplier)

Assembly proof at 0x006FE3FA:
    CALL [EDX + 0x400]          ; IsOccupied()
    FILD dword ptr [ESP + 0x2c] ; load damage as float
    MOV EAX, [0x008871E0]       ; g_RulesClass_Instance
    FMUL float ptr [EAX + 0xF40]; * OccupyDamageMultiplier
    CALL ftol                   ; back to int
```

### ROF Calculation -- `TechnoClass::GetROF` (0x006FCFA0)

```
baseROF = WeaponTypeClass+0xB0 (ROF)
rof = baseROF * HouseClass+0x1A8 (ROFMultiplier)
rof = apply_veterancy_bonus(rof)

IF IsOccupied():
    rof /= GetOccupantCount()                     ; MORE occupants = FASTER fire
    IF OccupyROFMultiplier (RulesClass+0xF44) > 0.0:
        rof = (int)((float)rof / OccupyROFMultiplier)

Assembly proof at 0x006FD176:
    IDIV ECX                    ; ROF / occupantCount
    FLD [ECX + 0xF44]          ; load OccupyROFMultiplier
    FCOMP [0x007E1748]         ; compare with 0.0f
    ; if > 0.0:
    FILD [ESP + 0x14]          ; load ROF as float
    FDIV [ECX + 0xF44]         ; ROF / OccupyROFMultiplier
    CALL ftol
```

**Key insight**: ROF is divided by occupant count first, THEN divided by the
multiplier. More occupants = proportionally faster aggregate fire rate.

### Round-Robin Cycling -- `TechnoClass::Fire_At` (0x006FDD50)

After each successful shot:

```
IF IsOccupied() AND GetWhatAmI() == 6 (Building):
    CurrentFireIdx++                    ; BuildingClass+0x69C
    CurrentFireIdx %= GetOccupantCount()
```

Each shot cycles to the next occupant, so all garrisoned infantry take turns firing.

### Weapon Range -- `TechnoClass::InRange` (0x006F7220)

```
IF IsOccupied():
    halfFoundation = GetHalfFoundationSize()  ; min(width,height)/2
    weaponRange = (halfFoundation + OccupyWeaponRange) * 256  ; in leptons
```

The garrison range **completely replaces** the weapon's normal range. It is NOT
additive. `OccupyWeaponRange` is a global value from `[CombatDamage]`.

For target scanning in `Greatest_Threat` (0x006F9190):
```
scanRadius = halfFoundation + 1 + OccupyWeaponRange  ; in cells
```

---

## 6. Garrison Visual Rendering

### Garrison Shot Muzzle Flash

Ordinary occupied-garrison shot flashes are produced by `TechnoClass::Fire_At`
using `WeaponType+0x110` (`OccupantAnim`) and the current fire port coordinate.

The older claim that `BuildingClass::Update` creates a passive 24-frame garrison
flash at each fire port is stale. The branch at `0x004403D4..0x0044055D` is
chrono/temporal sparkle rendering gated by warp flags `+0x270/+0x271`; it uses
`[General] ChronoSparkle1` at `RulesClass+0x344` and only reuses `MuzzleFlashN`
as sparkle anchors.

### Not the muzzle flash function: `0x0043E7B0`

Ghidra's label `BuildingClass::UpdateGarrisonFire` at `0x0043E7B0` is a
misnomer. That function actually draws a **FactoryClass construction-cameo
sprite** via `CC_Draw_Shape` — it's gated on `this->Factory` being non-null
and has nothing to do with garrison muzzle flashes or `+0x1588`. Do not
implement garrison rendering from this address.

### ActiveAnimGarrisoned (art.ini)

A named animation (e.g., `CAWA19_AG`) that plays on the building while it has
occupants. Selected by `FUN_00458330` (anim-variant dispatcher) based on
occupancy and health state — one of three variants per anim slot (healthy-empty
/ damaged / healthy-garrisoned). Runs from `CheckAutoSellOrCivilian` on
ownership transitions. See `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` §9.2
for the full slot table.

---

## 7. Related Systems (Bunker vs OpenTopped vs Garrison)

The engine has THREE distinct passenger-fire systems with separate multipliers:

| System | Flag | Damage Mult | ROF Mult | Range | Storage |
|--------|------|-------------|----------|-------|---------|
| **Garrison** | `CanBeOccupied` | `OccupyDamageMultiplier` (+0xF40) | `OccupyROFMultiplier` (+0xF44) | `OccupyWeaponRange` (replaces) | DVec at +0x684 |
| **Bunker** | `Bunker=yes` | `BunkerDamageMultiplier` (+0xF4C) | `BunkerROFMultiplier` (+0xF50) | `BunkerWeaponRangeBonus` (adds) | DVec at +0x66C (?) |
| **OpenTopped** | `OpenTopped=yes` | `OpenToppedDamageMultiplier` (+0xF58) | — | `OpenToppedRangeBonus` (adds) | Passenger list |

Garrison is for buildings only (civilian structures like gas stations, restaurants).
Bunker is YR's battle bunker system. OpenTopped is for vehicles like IFVs and
Flak Tracks where passengers fire independently.

---

## 8. Note on `BuildingClass::CanGarrison` (0x004525F0)

**Despite the name, this function is NOT a garrison check.** It checks gate
passability:

```
IF Gate (BuildingTypeClass+0x16B7) is false:
    return 1  (no gate = passable)
IF mission == 0x18 (Open):
    return FUN_004a51b0()  (check gate animation is in open state)
return 0  (gate closed = not passable)
```

Called by `CellClass::PlaceInfantryInCell`, `MapClass::Check_Crushable_Obstacle`,
`InfantryClass::Can_Enter_Cell`, and `UnitClass::Can_Enter_Cell` for gate traversal.

---

## 9. Confidence Assessment

| Finding | Confidence | Basis |
|---------|------------|-------|
| BuildingTypeClass garrison offsets (0x157B-0x1588) | **95%** | Direct ReadINI with string literals |
| InfantryTypeClass OccupyWeapon offsets (0xE04/0xE20) | **95%** | Assembly verification at 0x00524117 |
| BuildingClass occupant DVec at 0x684 | **90%** | Constructor + GetWeapon usage patterns |
| OccupyDamageMultiplier in Fire_At | **95%** | Assembly FMUL at 0x006FE3FA |
| ROF division by occupant count | **95%** | Assembly IDIV at 0x006FD176 |
| Round-robin cycling in Fire_At | **95%** | Decompiled code with modulo |
| Range replacement (not additive) | **90%** | Decompiled InRange with clear constant refs |
| CanGarrison is actually gate check | **95%** | Decompiled + callers all gate-related |
| CanDock is the real garrison check | **90%** | Decompiled logic matches garrison semantics |
| Bunker DVec at 0x66C | **70%** | Structural inference, not fully verified |

**2026-07-18 audit note (label drift, not content errors):** three functions this
doc names by role currently carry no matching name in live Ghidra — the
address and decompiled behavior are independently re-confirmed and unchanged,
only the display label is stale/absent:
- `TechnoClass::Fire_At` (0x006FDD50) is currently labeled `TechnoClassFireAtSpawnsBullet`.
- `TechnoClass::GetROF` (0x006FCFA0) is currently anonymous (`FUN_006fcfa0`).
- `BuildingClass::GetFireCoords` (0x00453840) is currently anonymous (`FUN_00453840`).

Both addresses and all formulas/offsets cited under these names in this doc
were re-verified this session (`decompile_function` / `disassemble_function`
on each address) and are CONFIRMED correct. ROOT_CAUSE: RTTI_LABEL_DRIFT.
Not corrected in-place throughout the doc (would require a full-doc reflow
for a cosmetic label with no behavioral change); flagged here for the
next labeling pass.

---

## 10. Verified Combat Details (Second Research Pass)

### Projectile Origin -- `BuildingClass::GetFireCoords` (0x00453840, vtable+0xB0)

Garrisoned buildings use **fire port positions**, NOT building center:

```
IF CanOccupyFire AND OccupantCount > 0:
    pixel_offset = BuildingTypeClass+0x1588[currentFirePortIndex * 8]  ; MuzzleFlash X,Y
    world_offset = IsometricPixelToWorld(pixel_offset)
    result = GetCoords() + world_offset                               ; building center + offset
    RETURN result
ELSE:
    fall through to normal FLH system
```

The fire port index is the same round-robin index at BuildingClass+0x69C that
advances in Fire_At after each shot.

### Target Acquisition -- Garrison-Specific Scan Range Override

`TechnoClass::Greatest_Threat` (0x006F8DF0) **does** have a garrison-specific
branch. After computing the default scan range, it checks `IsOccupied` (vtable+0x400)
and if true overwrites the scan radius with the garrison formula. See §15a for
the verified formula and assembly reference:

```
scanRange_cells = GetHalfFoundationSize() + 1 + OccupyWeaponRange
```

This replaces (not adds to) the default scan range. Weapon selection in
`GetWeapon` still returns OccupyWeapon for garrisoned buildings, and target
evaluation uses generic `Evaluate_Candidate` / `Scan_Cell_For_Target`. But the
scan radius itself is garrison-specific.

### Veterancy Source -- OCCUPANT's, Not Building's

Assembly at 0x45275B confirms the occupant infantry's veterancy is checked:

```asm
MOV ECX, [ESI + 0x688]        ; building->occupantArray
MOV ESI, [ECX + EAX*4]        ; ESI = occupant infantry pointer
LEA ECX, [ESI + 0x150]        ; ECX = &infantry->veterancy
CALL IsElite                   ; check occupant's vet, NOT building's
MOV EAX, [ESI + 0x6c0]        ; EAX = infantry->InfantryTypeClass
```

**Confidence: HIGH** -- direct assembly verification.

### Retaliation -- Standard Path, No Garrison-Specific Code

- `BuildingClass::ReceiveDamage` (0x442230) calls `TechnoClass::ReceiveDamage` (0x701900)
- Retaliation check (0x7087C0) uses `GetWeapon` → returns OccupyWeapon for garrisoned buildings
- If attacker is in OccupyWeapon range, building retaliates normally
- No garrison-specific retaliation logic exists

---

## 11. Implementation Plan for Rust Engine

### Architecture: Building is the Attacker

In the original engine, the **building** fires on behalf of occupants. The building
owns the attack target, cooldown, and round-robin index. Infantry inside are passive
weapon providers. This means:

- The building entity needs `attack_target` set when it has occupants and detects enemies
- Infantry inside do NOT fire independently (they stay blocked at the transport gate)
- Weapon selection on the building checks its occupant list for OccupyWeapon

### What Already Works (in our Rust engine)

- `PassengerCargo` tracks occupant IDs (`passengers: Vec<u64>`)
- `EnterTransport` command + boarding state machine
- Buildings already enter the combat tick if they have `attack_target`
- `GarrisonRules` parsed from [CombatDamage] (OccupyDamageMultiplier, etc.)
- `ObjectType` has `occupier`, `occupy_weapon`, `can_occupy_fire`, etc.
- `ArtEntry` has `muzzle_flash_positions`

### What Needs Implementation

1. **Garrison auto-scan**: Garrisoned buildings with `can_occupy_fire` and occupants
   need to periodically scan for targets (like guard-mode units). Currently
   `attack_target` only gets set via Attack command, retaliation, or OrderIntent.
   Add a garrison scan pass in the combat tick or world_orders.

2. **Garrison weapon selection**: `select_weapon_with_ifv()` in `combat_weapon.rs`
   needs a garrison path. When the attacker is a building with `can_occupy_fire`
   and occupants, look up the current occupant's `occupy_weapon` (or fall back to
   their primary weapon). Use occupant's veterancy to choose elite variant.

3. **Round-robin index on PassengerCargo**: Add `current_fire_index: u32` to
   `PassengerCargo`. After each shot, advance: `index = (index + 1) % count`.

4. **Damage multiplier**: In the damage calculation (combat.rs ~line 449), if the
   attacker is an occupied building, multiply damage by `garrison_rules.occupy_damage_multiplier`.

5. **ROF formula**: In the ROF/cooldown update (combat.rs ~line 489), if occupied:
   `rof = rof / occupant_count / occupy_rof_multiplier`.

6. **Range override**: In the range check (combat.rs ~line 402), if occupied:
   use `(half_foundation + occupy_weapon_range) * 256` leptons instead of weapon range.

7. **Entry validation fixes** (in `can_enter_transport` / command validator):
   - Check `Occupier=yes` on infantry for garrison entry
   - Allow neutral civilian building entry (ownership transfer)
   - Block entry when building at red HP
   - Block entry during construction/selling

### Files to Modify

| File | Change |
|------|--------|
| `sim/combat.rs` | Damage multiplier, ROF formula, range override |
| `sim/combat_weapon.rs` | Garrison weapon selection path |
| `sim/combat_targeting.rs` | No changes (standard scan works) |
| `sim/passenger.rs` | Add `current_fire_index` to PassengerCargo, entry validation |
| `sim/world_orders.rs` | Add garrison auto-scan pass |
| `sim/world_commands.rs` | Entry validation for Occupier check |

---

## 12. Verified Decompiled Code — Fire_At Garrison Paths (Second Research Pass)

All code from `TechnoClass::Fire_At` (0x006FDD50) in file
`129_006fc0b0_007077c0.c`, confirmed via decompilation.

### 12a. Round-Robin Advancement (lines 1926-1933)

After a projectile is successfully created, the building advances its fire index:

```c
// At address 0x006FF035-0x006FF085 in Fire_At (corrected 2026-07-18: was
// "~0x006FF680" — that address is unrelated code (a GetCoords call, vtable+0x48)
// nearly 1.6KB further into the function. The actual sequence verified via
// disassemble_function 0x006FDD50 / grep on the listing:
//   006ff035: CALL [EAX+0x400]      ; IsOccupied()
//   006ff065: MOV EAX,[EDI+0x69c]   ; CurrentFireIdx
//   006ff06e: INC EAX; MOV [EDI+0x69c],EAX   ; ++CurrentFireIdx
//   006ff074: CALL [EDX+0x408]      ; GetOccupantCount()
//   006ff083: IDIV ECX              ; CurrentFireIdx / count
//   006ff085: MOV [EDI+0x69c],EDX   ; CurrentFireIdx = remainder (idx %= count)
// ROOT_CAUSE: OFFSET_RETYPED_WRONG):
cVar4 = (**(code **)(*param_1 + 0x400))();  // IsOccupied()
if (((cVar4 != '\0') && (param_1 != (int *)0x0)) &&
   (iVar8 = (**(code **)(*param_1 + 0x2c))(), iVar8 == 6)) {  // Is building?
    piVar18 = (int *)(~-(uint)(iVar8 != 6) & (uint)param_1);
    piVar18[0x1a7] = piVar18[0x1a7] + 1;                       // ++CurrentFireIdx (Building+0x69C)
    iVar8 = (**(code **)(*piVar18 + 0x408))();                  // GetOccupantCount()
    piVar18[0x1a7] = piVar18[0x1a7] % iVar8;                   // Wrap: idx %= count
}
```

**Confidence: 95%** — piVar18[0x1a7] = byte offset 0x69C, matching the garrison report.
This runs after EVERY shot, guaranteeing occupants cycle. The logic/formula was
independently re-confirmed byte-for-byte in the live binary; only the address
citation was wrong.

### 12b. Garrison Damage Multiplier (lines 1631-1634)

```c
// At address ~0x006FE3FA in Fire_At:
cVar4 = (**(code **)(*param_1 + 0x400))();  // IsOccupied()
if (cVar4 != '\0') {
    uVar17 = FUN_007c5f00();  // Applies OccupyDamageMultiplier from RulesClass+0xF40
    uStack_a4 = uVar17;       // Updated damage result stored for projectile creation
}
```

`FUN_007c5f00` performs float multiplication: `(float)damage * *(float*)(RulesClass + 0xF40)`.
This is applied AFTER base damage and veterancy bonuses, BEFORE projectile creation.

### 12c. Garrison Muzzle Flash Selection (lines 2016-2022)

```c
// At address 0x006FF31F-0x006FF329 in Fire_At (corrected 2026-07-18: was
// "~0x006FF7C0" — that address is unrelated code, a byte-flag/fire-port test
// with no IsOccupied() call or +0x110 read nearby. Verified via
// disassemble_function 0x006FDD50 / grep on the listing:
//   006ff31f: CALL [EDX+0x400]      ; IsOccupied()
//   006ff325: TEST AL,AL; JZ 0x006ff32f
//   006ff329: MOV EDI,[EBX+0x110]   ; WeaponType+0x110
// ROOT_CAUSE: OFFSET_RETYPED_WRONG):
cVar4 = (**(code **)(*param_1 + 0x400))();  // IsOccupied()
if (cVar4 != '\0') {
    iVar9 = *(int *)(uVar17 + 0x110);       // WeaponType+0x110 = garrison muzzle flash anim
}
```

When garrisoned, the weapon's `+0x110` field (OccupyAnim/garrison-specific muzzle flash
animation type) is used instead of the standard muzzle flash at `+0x104`.

### 12d. Garrison Weapon Lookup from Occupant Type (lines 4286-4293)

This is in a weapon-image/facing resolver called from Fire_At's projectile setup:

```c
// At address ~0x007030A0:
cVar1 = (**(code **)(*unaff_retaddr + 0x400))();  // IsOccupied()
if ((cVar1 != '\0') && (GetType() == 6)) {          // Is building?
    uVar7 = building_ptr;
    // Read current occupant from Items[CurrentFireIdx]:
    //   Items = *(int*)(building + 0x688)
    //   CurrentFireIdx = *(int*)(building + 0x69C)
    //   occupant = Items + CurrentFireIdx * 4
    //   InfantryTypeClass = *(occupant + 0x6C0)
    uVar5 = (**(code **)(**(int **)(*(int *)(uVar7 + 0x688) +
                                    *(int *)(uVar7 + 0x69c) * 4)
                          + 0x6c0) + 0x84))
                        (param_1[0x87], iVar3);  // Call GetImage on InfantryTypeClass
}
```

This confirms: weapon/image data comes from the **current round-robin occupant's
InfantryTypeClass**, not from the building's own type. The occupant's type at
`+0x6C0` is dereferenced, then vtable+0x84 (GetImage/GetWeapon) is called.

### 12e. Fire Gating for Empty Garrison Buildings (FUN_007091D0)

**Correction 2026-07-18:** `FUN_007091D0` is NOT a garrison-specific gate — it is
`TechnoClass::CanAcquireTarget`, a general multi-condition "can this unit target
anything at all" predicate (disabling-state check via vtable+0x1DC, a
`+0x2DC` flag, `TechnoType+0xD99`, capture-management, player-control,
weapon-equipped via vtable+0x2AC). The garrison `CanBeOccupied` +
`GetOccupantCount()==0` block quoted below is real and is exactly one gate
among several inside this broader predicate, not the function's sole purpose.
Verified via `decompile_function 0x007091d0` (Ghidra's own plate comment,
dated 2026-06-11, already documents the fuller role and post-dates this
doc's original write-up). ROOT_CAUSE: INFERENCE_HARDENED — the garrison
snippet was correctly extracted but its containing function's purpose was
narrowed to only what was visible in that one branch.

```c
// At address 0x007091D0 (TechnoClass::CanAcquireTarget), garrison-specific branch only:
iVar3 = (**(code **)(*param_1 + 0x2c))();           // GetType
if (((iVar3 == 6) &&                                 // Is building?
    (*(char *)(param_1[0x148] + 0x157b) != '\0')) && // CanBeOccupied=yes?
   (iVar3 = (**(code **)(*param_1 + 0x408))(),       // GetOccupantCount
    iVar3 == 0)) {
    return 0;  // CANNOT FIRE — CanBeOccupied building with 0 occupants
}
```

**Critical:** A building with `CanBeOccupied=yes` but no occupants **cannot fire at all**,
even if the building has its own Primary weapon. This is a fire-gating rule:
garrisonable buildings are defenseless when empty.

### 12f. CanOccupyFire vs CanBeOccupied

From BuildingTypeClass:
- `+0x157B` = **CanBeOccupied** (infantry can enter this building)
- `+0x157C` = **CanOccupyFire** (can garrisoned infantry shoot from this building)

Verified via `IsOccupied` at `0x00458DD0` which reads both offsets directly:
`*(char*)(Type + 0x157B)` and `*(char*)(Type + 0x157C)`.

These are SEPARATE flags. A building can have `CanBeOccupied=yes` (infantry can enter)
but `CanOccupyFire=no` (they can't shoot from it). The `IsOccupied()` query (vtable+0x400)
returns true only if BOTH `CanBeOccupied` AND `CanOccupyFire` AND occupant count > 0.

---

## 13. Full Decompilation — CanDock, GetWeapon, IsOccupied (Third Research Pass)

Live Ghidra decompilation of the three core garrison functions.

### 13a. BuildingClass::CanDock (0x00457CE0) — Full Eligibility Check

```c
int __thiscall BuildingClass__CanDock(ObjectClass *this, int infantry_ptr)
{
    // Gate 1: infantry must exist, CanBeOccupied must be set
    if (infantry_ptr == 0) return 0;
    if (BuildingTypeClass->CanBeOccupied == 0) return 0;       // +0x157B

    // Gate 2: building mission must not be Construction (0x12) or Selling (0x13)
    if (mission == 0x12 || mission == 0x13) return 0;

    // Gate 3: building must be in valid map bounds (FUN_005785f0)
    if (!IsInMapBounds(GetCoords())) return 0;

    // Gate 4: building must not be deploying (vtable+0x1D4)
    if (IsDeploying()) return 0;

    // Branch on infantry type flags:
    InfantryTypeClass *infType = *(infantry + 0x6C0);

    if (infType->Occupier == 0) {  // +0xEB4
        // NOT Occupier — check Assaulter
        if (infType->Assaulter != 0               // +0xEB5
            && !HouseClass::Is_Ally(infantry)      // Must NOT be allied
            && GetOccupantCount() != 0) {          // Building must have enemy occupants
            return 1;  // CAN ASSAULT this enemy-garrisoned building
        }
        return 0;
    }

    // IS Occupier — check garrison entry conditions:
    HouseClass *buildingHouse = this->Owner;         // +0x21C
    HouseClass *infantryHouse = *(infantry + 0x21C);

    // Condition 1: Same owner OR building house is MultiplayPassive
    // MultiplayPassive is at CountryTypeClass+0x1A6
    //   accessed via: buildingHouse->CountryType(+0x34)->MultiplayPassive(+0x1A6)
    if (buildingHouse != infantryHouse
        && *(char *)(buildingHouse->CountryType + 0x1A6) == 0) {
        return 0;  // Different owner AND not MultiplayPassive → blocked
    }

    // Condition 2: Not full — count != MaxNumberOccupants
    if (GetOccupantCount() == BuildingTypeClass->MaxNumberOccupants) {  // +0x1580
        return 0;  // Full
    }

    // Condition 3: Not at red HP
    if (ObjectClass::IsRedHP(this)) return 0;

    // Condition 4: Not mind-controlled (TechnoClass::IsMindControlled at 0x007105e0)
    //   Returns 1 if entity+0x2C0 (mind-control link ptr) or entity+0x2C4 (flag) is set
    if (IsMindControlled()) return 0;

    return 1;  // CAN GARRISON
}
```

**New findings:**
- **MultiplayPassive** is NOT directly on HouseClass. It's at `HouseClass->CountryType(+0x34)->+0x1A6`.
  In rulesmd.ini, this is the `MultiplayPassive=true` flag on the house section.
- **Assaulter path** requires the building to HAVE occupants (enemy garrison). You can't
  assault an empty building — assault is specifically "enter building to clear enemy garrison".
- **Mind-control check** (`TechnoClass::IsMindControlled` at 0x007105e0): checks two
  fields at +0x2C0 (mind-control link pointer) and +0x2C4 (mind-control flag byte).
  If either is non-zero, the building is mind-controlled and cannot be entered.

### 13b. BuildingClass::GetWeapon (0x004526F0) — Garrison Weapon Selection

```c
int * __thiscall BuildingClass__GetWeapon(int *this, int weapon_index)
{
    // PHASE 1: Check fire port infantry (Building+0x5EC array, count at +0x702)
    //          Fire ports are separate from the occupant DynamicVectorClass.
    if (firePortCount > 0) {  // *(byte*)(this + 0x702)
        int *portSlots = this + 0x17B;  // byte offset 0x5EC (0x17B * 4)
        for (int i = 0; i < firePortCount; i++) {
            if (portSlots[i] != 0) {  // Port has infantry assigned
                int *weapon = FUN_007177c0(weapon_index);  // Standard weapon lookup
                if (*weapon != 0) return weapon;
            }
        }
    }

    // PHASE 2: Garrison occupant weapon selection
    if (!IsOccupied() || occupant_count <= garrison_fire_index) {
        // NOT occupied OR fire index out of bounds → building's own weapon
        return TechnoClass::GetWeapon(weapon_index);  // FUN_0070E140
    }

    // Get current round-robin occupant
    int *infantry = Items[garrison_fire_index];  // *(*(this+0x688) + fire_idx*4)

    bool isElite = FUN_00750010();  // Check occupant veterancy
    int *infType = infantry[0x1B0];  // InfantryTypeClass at Infantry+0x6C0

    if (!isElite) {
        // NORMAL: try OccupyWeapon at InfantryTypeClass+0xE04
        if (*(int *)(infType + 0xE04) != 0) {
            return (int *)(infType + 0xE04);  // Return OccupyWeapon pointer
        }
        // OccupyWeapon is NULL → fall back to infantry's primary weapon
        return infantry->GetWeapon(0);  // vtable+0x3F8 with index 0
    } else {
        // ELITE: try EliteOccupyWeapon at InfantryTypeClass+0xE20
        if (*(int *)(infType + 0xE20) != 0) {
            return (int *)(infType + 0xE20);  // Return EliteOccupyWeapon pointer
        }
        // EliteOccupyWeapon is NULL → fall back to infantry's primary weapon
        return infantry->GetWeapon(0);
    }
}
```

**New findings:**
- **Fire port slots** (Building+0x5EC, count +0x702) are checked FIRST, before the
  garrison DynamicVectorClass. Fire ports are a separate mechanism used by some
  buildings (likely bunkers). If a fire port has infantry, its weapon is used.
- **Safety check:** `occupant_count <= garrison_fire_index` prevents out-of-bounds.
  If the index is invalid, falls back to the building's own weapon.
- **Veterancy check** uses `FUN_00750010` on the occupant infantry — confirmed as
  veterancy/elite status, not "IsYRMode".
- **Return value** is a pointer directly to the WeaponTypeClass in the InfantryTypeClass
  struct, NOT a copy. OccupyWeapon and EliteOccupyWeapon are inline pointers.

### 13c. BuildingClass::IsOccupied (0x00458DD0) — Three-Way Boolean

```c
int __fastcall BuildingClass__IsOccupied(int *this)
{
    if (*(char *)(this[0x148] + 0x157B) != 0     // CanBeOccupied
        && *(char *)(this[0x148] + 0x157C) != 0)  // CanOccupyFire
    {
        if (GetOccupantCount() > 0) {
            return 1;  // OCCUPIED AND CAN FIRE
        }
    }
    return 0;
}
```

Simple but critical: all three conditions required.
`CanBeOccupied` alone is NOT enough — `CanOccupyFire` must also be true.

---

## 14. Full Decompilation — Fire Coordinates & Ejection

### 14a. BuildingClass::GetFireCoords (0x00453840)

How garrison buildings compute projectile/muzzle origin:

```c
int * __thiscall GetFireCoords(int *this, int *output, int weapon_index)
{
    if (CanBeOccupied && GetOccupantCount() > 0) {
        // GARRISON PATH:
        // MuzzleFlash position = BuildingTypeClass+0x1588 + CurrentFireIdx * 8
        // Each entry = 8 bytes = two ints (pixel X, pixel Y in isometric screen space)
        int *pixel_offset = BuildingTypeClass + 0x1588 + garrison_fire_index * 8;

        // Convert screen-space pixel offset to 3D world coordinates
        int *world_offset = IsometricPixelToWorld(pixel_offset);

        // Add to building's center coordinates (vtable+0xAC = GetCenterCoords)
        int *center = GetCenterCoords();
        output[0] = center[0] + world_offset[0];  // X
        output[1] = center[1] + world_offset[1];  // Y
        output[2] = center[2];                      // Z = building's Z (no height offset)
        return output;
    }

    // NON-GARRISON PATHS:
    //   1. No custom fire offset → check turret → use FLH
    //   2. Custom fire offset at TypeClass+0xE44 → IsometricPixelToWorld + FLH or center
    //   3. TypeClass+0x16C5 flag → add additional pixel offset from +0x11E0
    // (standard building fire coordinate computation)
}
```

**Key detail:** The pixel-to-world conversion uses `IsometricPixelToWorld` which
transforms screen-space (X,Y) pixel offsets into the 3D isometric world coordinate
system. The same function is used for both fire coordinates AND muzzle flash rendering.

### 14b. TechnoClass::IsMindControlled (0x007105e0) — used in CanDock

```c
int __fastcall IsMindControlled(int entity)
{
    // entity+0x2C0 = mind-control link pointer (non-zero while controlled)
    // entity+0x2C4 = mind-control flag byte
    if (*(int *)(entity + 0x2C0) == 0 && *(char *)(entity + 0x2C4) == 0) {
        return 0;  // NOT mind-controlled
    }
    return 1;  // IS mind-controlled — blocks garrison entry
}
```

Prevents garrisoning a building that is currently under Yuri/psychic mind-control.
Note: Ghidra's official annotation names this function `TechnoClass::IsMindControlled`;
the fields at +0x2C0 / +0x2C4 are the classic mind-control link and flag, not
chrono/warp state.

### 14c. BuildingClass::SellBuilding Occupant Ejection (0x00457DE0)

Full occupant ejection flow when a garrisoned building is sold or destroyed:

```c
void __thiscall SellBuilding(BuildingClass *this)
{
    // Reset round-robin fire index to 0
    this->CurrentFireIdx = 0;  // Building+0x69C = 0

    int count = GetOccupantCount();
    if (count == 0) return;  // No occupants to eject

    // STEP 1: Find exit cell
    // Search deterministic foundation perimeter order:
    // east column SE->NE, south row SE->SW, north row west->east,
    // then west row north->south.
    // Uses occupant slot 0 Can_Enter_Cell(cell,-1,-1,0,1).
    // If ALL fail, fallback depends on caller arguments.

    // STEP 2: Eject occupants (iterate BACKWARDS through occupant array)
    for (int i = occupant_count - 1; i >= 0; i--) {
        int *infantry = Items[i];  // *(*(Building+0x688) + i*4)

        // Try to unlimbo infantry at the exit cell
        bool unlimboed = infantry->Unlimbo(exitCoords, 0);  // vtable+0xD8

        if (!unlimboed) {
            // NO ROOM → DESTROY the infantry
            infantry->Destroy();  // vtable+0xF8
        } else {
            // Successfully placed on map
            // Clear house radar flag if applicable
            if (infantry->House->HasRadar) {  // HouseClass+0x1EC
                *(byte*)(infantry + 0x691) = 0;
                *(byte*)(infantry + 0x690) = 0;
            }
            // Clear archive target via vtable+0x3C8(0)
            infantry->ClearArchiveTarget();
            // Direct Scatter with building center coordinate and true/true flags.
            // Infantry Scatter may queue mission 2 and set a destination.
            infantry->Scatter(building->GetCoords(), true, true);
            // Later mission 0xF block exists but is first-argument gated;
            // direct callers checked by the 2026-05-27 swarm pass 0.
        }
    }

    // STEP 3: Clear occupant vector
    DynamicVector_Clear();
    DynamicVector_Resize(originalCapacity, 0);

    // STEP 4: Recalculate building power
    RecalcPower();
}
```

**Key findings:**
- **Iterates BACKWARDS** (high index to low) — LIFO order, not FIFO
- **First resets CurrentFireIdx to 0** before ejection begins
- **Exit cell search** uses the verified east/south/north/west perimeter order
  and probes only occupant slot 0 via `Can_Enter_Cell(cell,-1,-1,0,1)`
- **If no exit cell found at all:** destruction/red-HP callers pass zero and
  take `SpawnUnitsWithParachute(0)`'s null remove branch; normal player sell
  uses an inside-foundation fallback coordinate
- **Unlimbo failure → DESTROY** — if the chosen exit cell can't fit the infantry, it dies
- **Successful unlimbo:** infantry is placed on map, archive target is cleared,
  then the occupant Scatter virtual is called with the building coordinate
- **Infantry Scatter** uses scenario `RandomRanged(0,4)` after Scatter gates,
  not an immediate raw `% 8` ejection draw

### 14d. GetOccupantCount (0x004581F0) — Trivial Getter

```c
int __fastcall GetOccupantCount(int building) {
    return *(int *)(building + 0x694);  // Direct field read
}
```

### 14e. GetHalfFoundationSize (0x00458E00)

```c
int GetHalfFoundationSize() {
    int height = GetFoundationHeight();
    int width = GetFoundationWidth();
    if (width < height) {
        return GetFoundationWidth() / 2;  // Use smaller dimension
    }
    return GetFoundationHeight() / 2;
}
```

Used in garrison range calculation: `(halfFoundation + OccupyWeaponRange) * 256` leptons.

### 14f. Chrono Sparkle Spawning in BuildingClass::Update (0x0043FB20)

Correction from `CONTINUOUS_GARRISON_MUZZLE_FLASH_CADENCE_GHIDRA_REPORT.md`:
this branch is not a continuous occupied-garrison muzzle-flash path. It is gated
by `TechnoClass::IsWarpingOut` / `IsBeingWarped` (`+0x270/+0x271`) and spawns
`[General] ChronoSparkle1` from `RulesClass+0x344`. When `BuildingType+0x1580`
is nonzero, `MuzzleFlashN` offsets are reused as chrono sparkle anchors.

```c
// In BuildingClass::Update:
if (IsWarpingOut() || IsBeingWarped()) {
    if (MaxNumberOccupants == 0 || MuzzleFlashAnimType == 0) {
        // No port positions -> spawn at building location every 24 frames
        if (g_CurrentFrameCounter % 24 == 0 && MuzzleFlashAnimType != 0) {
            AnimClass::Create(MuzzleFlashAnimType, buildingCoords, flags=0x600);
        }
    } else {
        // HAS port positions -> iterate and stagger chrono sparkles
        for (int port = 0; port < MaxNumberOccupants; port++) {
            // Each port sparkles at a different time in the 24-frame cycle
            if ((g_CurrentFrameCounter + port) % 24 == 0 && MuzzleFlashAnimType != 0) {
                // Convert MuzzleFlash[port] pixel offset to world coords
                int *offset = IsometricPixelToWorld(TypeClass + 0x1588 + port * 8);
                int *center = GetCenterCoords();
                coords = center + offset;

                AnimClass *anim = AnimClass::Create(
                    MuzzleFlashAnimType, coords, 0, 1, 0x600, 0, 0);
                anim->ZOffset = -200;  // Depth sort: in front of building
            }
        }
    }
}
```

**Key finding:** ordinary occupied garrison shot flashes are still actual
`Fire_At` events using `WeaponType+0x110` (`OccupantAnim`). Do not add a normal
24-frame ambient garrison flash from this `Update` branch.

`RulesClass+0x344` is `[General] ChronoSparkle1` (`CHRONOSK` in stock YR), not
a garrison muzzle flash animation.

---

## 15. Target Acquisition & Kill Attribution (Fourth Research Pass)

### 15a. Greatest_Threat — Garrison Scan Range Override

In `TechnoClass::Greatest_Threat` (0x006F8DF0), the scan range for garrisoned
buildings is computed separately from normal weapons:

```c
// After normal scan range calculation:
cVar2 = IsOccupied();  // vtable+0x400
if (cVar2 != '\0') {
    iVar6 = GetHalfFoundationSize();   // vtable+0x404
    iStack_34 = iVar6 + 1 + *(int *)(RulesClass + 0xF48);  // +OccupyWeaponRange
}
// iStack_34 is now the scan radius in CELLS
```

**Garrison scan range formula:**
```
scanRange_cells = GetHalfFoundationSize() + 1 + OccupyWeaponRange
```

Note the `+1` — the scan range is **1 cell larger** than the firing range. This
ensures the building detects targets that are approaching but not yet in range.

Compare to the **firing range** from `InRange` (0x006F7220):
```
firingRange_leptons = (GetHalfFoundationSize() + OccupyWeaponRange) * 256
```

The scan radius is in cells, the firing range is in leptons (256 per cell).

### 15b. Greatest_Threat — Building Scan Mode

Buildings (type 6) use **cell-based scanning** (`param_2 & 4` flag). The scan
iterates all cells within `scanRange_cells` radius around the building's
position, calling `Scan_Cell_For_Target` for each cell.

Each candidate is evaluated by `TechnoClass::Evaluate_Candidate` which checks:
- Target alive and valid
- Not cloaked (unless sensor detects)
- Warhead Verses check (0% = blocked, 1% = suppressed = no passive acquire)
- Distance comparison (closer = better score)
- Weapon compatibility

### 15c. InRange — Target Foundation Bonus

When the target is a building, its foundation size adds to the effective range:

```c
if (target.GetType() == 6) {  // Target is building
    int height = BuildingTypeClass::GetFoundationHeight();
    int width = BuildingTypeClass::GetFoundationWidth();
    range += (height + width) * 64;  // 64 leptons per foundation dimension sum
}
```

This ensures weapons can hit large buildings from the apparent same distance
regardless of building size.

### 15d. Kill Attribution — Experience Goes to Occupant

From `TechnoClass::RecordKill` (0x00702D40) (corrected 2026-05-28: was `RegisterDestruction`; Ghidra label is `TechnoClass__RecordKill` — ROOT_CAUSE: RTTI_LABEL_DRIFT):

```c
// Check if attacker is a garrisoned building
cVar1 = IsOccupied();  // vtable+0x400
if (cVar1 != '\0' && GetType() == 6) {
    // Get current round-robin occupant
    int *infantry = Items[CurrentFireIdx];          // Building+0x688, +0x69C
    InfantryTypeClass *type = infantry->TypeClass;   // +0x6C0

    // Award kill credit to the OCCUPANT's type, not the building
    type->AwardKillBounty(victim_house, bounty_value);
}
```

**Key finding:** Kill credit and veterancy experience goes to the **infantry that
fired the shot** (the current round-robin occupant), NOT to the building. This
means garrisoned infantry can earn promotions through garrison kills.

### 15e. GetWeaponRange — Average Range for Scan

`TechnoClass::GetWeaponRange` (0x006F3970, corrected 2026-07-18: was `0x006F6F20`;
that address is mid-body of an unrelated function, `TechnoClass::Unlimbo`
(body `0x6F6CA0`-`0x6F6F32`) — verified via `search_functions WeaponRange` ->
`TechnoClass__GetWeaponRange @ 006f3970`, cross-checked with
`get_function_by_address 0x006F6F20` returning `TechnoClass__Unlimbo` —
ROOT_CAUSE: OFFSET_RETYPED_WRONG; same failure class this doc already caught
once for `Mission_Enter`) with `index == -1`:
- Returns the **average** of primary and secondary weapon ranges
- For garrisoned buildings, `GetWeapon(0)` returns the garrison weapon
- Range values at WeaponType+0xA4 and +0x98 are summed (likely Damage range + Burst range?)

### 15f. Buildings Run Standard TechnoClass AI

`BuildingClass::Update` calls `TechnoClass::AI_Update()`, which includes:
- Mission dispatch (Guard → scan for targets → attack)
- Retaliation handling
- Idle scanning via `Greatest_Threat`

Buildings in Guard mission periodically call `Greatest_Threat` to find hostile
targets within scan range. When a target is found, the building transitions
to attacking. This is the **standard TechnoClass behavior** — no garrison-specific
AI code is needed for target acquisition.

### 15g. Garrison Occupants Are NOT Harmed by Building Damage

No code path was found that passes damage to garrison occupants when the building
takes hits. Occupants are safe inside the building until it is destroyed. They
only die/eject when:
1. Building is destroyed → EjectOccupants (unlimbo or kill)
2. Building is sold → SellBuilding (unlimbo or parachute)

### 15h. House::ReplenishGarrison (AI-Only)

`FUN_0050D320` — The AI has a dedicated function to replenish garrison structures.
For vehicle-type buildings (e.g., tech structures with preset defenders), if the
expected garrison unit is missing, the AI creates a replacement and sends it to
the building. This is AI-only behavior — not relevant for player-controlled garrison.

---

## 16. Summary of ALL Garrison Function Addresses

| Address | Name | Size | Role |
|---------|------|------|------|
| 0x00457CE0 | `BuildingClass::CanDock` | 168 | Garrison eligibility check |
| 0x004526F0 | `BuildingClass::GetWeapon` | 128 | Weapon selection (garrison/fire-port/standard) |
| 0x00458DD0 | `BuildingClass::IsOccupied` | 40 | Three-way check: CanBeOccupied + CanOccupyFire + count>0 |
| 0x004581F0 | `BuildingClass::GetOccupantCount` | 7 | Returns *(building+0x694) |
| 0x00458E00 | `BuildingClass::GetHalfFoundationSize` | 40 | min(width,height)/2 |
| 0x00453840 | `BuildingClass::GetFireCoords` | 220 | Fire port pixel→world coords, or FLH fallback |
| 0x00522910 | `BuildingClass::AddGarrisonOccupant` | 318 | Adds infantry to occupant vector |
| 0x00457DE0 | `BuildingClass::SellBuilding` | 1029 | **Actual occupant ejection** (backwards, unlimbo/kill/parachute) |
| 0x004575B0 | `BuildingClass::EjectOccupants` | 112 | **Refunds upgrade-slot credits only — misleading name; does NOT eject occupants** |
| 0x00458200 | `BuildingClass::CheckAutoSellOrCivilian` | 200 | Per-tick ownership reconciliation |
| 0x00448260 | `BuildingClass::ChangeOwner` | 4517 | Full ownership transfer (vtable+0x3D4) |
| 0x0043B150 | `BuildingClass::IsTargetInRange` | 865 | Range check with garrison awareness |
| 0x0043E7B0 | `FUN_0043E7B0` | 200 | **Draws factory cameo — NOT garrison muzzle flash** (Ghidra label `UpdateGarrisonFire` is misleading) |
| 0x0043FB20 | `BuildingClass::Update` | 2650 | Main tick (calls AI_Update, spawns fire port muzzle flashes) |
| 0x006FDD50 | `TechnoClass::Fire_At` | 7167 | Projectile creation, damage mult, round-robin advance |
| 0x006FCFA0 | `TechnoClass::GetROF` | 350 | ROF / occupant_count / OccupyROFMultiplier |
| 0x006F7220 | `TechnoClass::InRange` | 500 | (halfFoundation + OccupyWeaponRange) * 256 |
| 0x006F8DF0 | `TechnoClass::Greatest_Threat` | 3400 | Target scanning (scan range = halfFound+1+range) |
| 0x006F3970 | `TechnoClass::GetWeaponRange` | 200 | Average range across weapons (corrected 2026-07-18: was `0x006F6F20`, mid-body of `TechnoClass::Unlimbo` — verified via `search_functions WeaponRange` — ROOT_CAUSE: OFFSET_RETYPED_WRONG) |
| 0x007091D0 | `TechnoClass::CanAcquireTarget` | 120 | General target-acquire predicate; ONE of its gates blocks empty CanBeOccupied buildings from firing (corrected 2026-07-18: doc previously implied this function's sole purpose was garrison fire-gating — verified via `decompile_function 0x007091d0` — ROOT_CAUSE: INFERENCE_HARDENED) |
| 0x007105E0 | `TechnoClass::IsMindControlled` | 37 | Mind-control check (blocks CanDock) — checks `+0x2C0` (link ptr) and `+0x2C4` (flag byte) (corrected 2026-05-28: was "IsBeingWarped check"; binary decompile confirms mind-control fields, not warp state; Ghidra label `TechnoClass__IsMindControlled` via `get_function_by_address 0x007105E0`; §14b already correct — ROOT_CAUSE: INFERENCE_HARDENED in §16 table only) |
| 0x00702D40 | `TechnoClass::RecordKill` | 1215 | Kill credit → round-robin occupant (corrected 2026-05-28: was `TechnoClass::RegisterDestruction`; Ghidra label is `TechnoClass__RecordKill` via `get_function_by_address 0x00702D40` — ROOT_CAUSE: RTTI_LABEL_DRIFT) |
| 0x0050D320 | `House::ReplenishGarrison` | 200 | AI: auto-replace missing garrison units |
