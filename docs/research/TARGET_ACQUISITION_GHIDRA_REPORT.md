# Target Acquisition & Threat Evaluation System -- Ghidra Report

**Research date:** 2026-03-22
**Source:** Ghidra MCP live decompilation of gamemd.exe
**Confidence:** HIGH (directly decompiled from binary, cross-referenced with READINI field maps)

---

## 1. Architecture Overview

Target acquisition in YR is a multi-layered system with three main functions:

| Address | Name | Size | Role |
|---------|------|------|------|
| `0x006F8DF0` | `TechnoClass::Greatest_Threat` | 518 lines | **Main threat scanner** -- vtable+0x3C4. Iterates cells/objects, filters candidates, returns best target |
| `0x006F7CA0` | `TechnoClass::Evaluate_Candidate` | 489 lines | **Per-candidate evaluator** -- checks validity, computes threat score via `Calculate_Threat_Score` |
| `0x0070CD10` | `TechnoClass::Calculate_Threat_Score` | 121 lines | **Threat math** -- applies coefficients (strength, distance, special) to produce numeric score |
| `0x006F8960` | `TechnoClass::Scan_Cell_For_Target` | 130 lines | **Cell scanner** -- finds best target in a specific cell, calls `Evaluate_Candidate` |
| `0x004D5690` | `FootClass::Greatest_Threat_Scan` | 737 lines | **FootClass scatter/approach** -- for mobile units, finds firing position and approaches |
| `0x004D5350` | `FootClass::Mission_Hunt` | 175 lines | **Hunt mission** -- simplified delegation: checks CanFire/IsEngineer conditions then calls Greatest_Threat path or moves toward base objective (corrected 2026-05-28: was 0x004D4280; binary at that address is FootClass__Mission_Patrol, verified via `get_function_by_address 0x004D4280` and `search_functions Mission_Hunt` — ROOT CAUSE: RTTI_LABEL_DRIFT) |
| `0x004D4280` | `FootClass::Mission_Patrol` | 737 lines | **Patrol mission** -- 4-state machine that calls `Greatest_Threat` via vtable+0x3C4, validates targets, gates on path distance; previously mislabelled as Mission_Hunt in this doc (corrected 2026-05-28) |
| `0x004D97A0` | `FootClass::Evaluate_Target_Threat` | 69 lines | **Quick threat rating** -- simplified evaluation for active pursuit decisions |
| `0x006F79A0` | `TechnoClass::ThreatAvoidance_Modifier` | 100 lines | **Avoidance multiplier** -- scans nearby cells for allied buildings, reduces threat near friendlies |

### Call Graph

```
Mission_Patrol (0x004D4280)  [previously mislabelled as Mission_Hunt — corrected 2026-05-28]
  |
  +-- vtable+0x3C4 --> Greatest_Threat (0x006F8DF0)
  |     |
  |     +-- [mode: iterate all TechnoClass objects]
  |     |     +-- HouseClass::Is_Ally filter
  |     |     +-- Evaluate_Candidate (0x006F7CA0)
  |     |           +-- Calculate_Threat_Score (0x0070CD10)
  |     |           +-- ThreatAvoidance_Modifier (0x006F79A0)
  |     |
  |     +-- [mode: scan cells in expanding square]
  |           +-- Scan_Cell_For_Target (0x006F8960)
  |           |     +-- iterate cell occupants linked list
  |           |     +-- ally/enemy filtering
  |           |     +-- Evaluate_Candidate (0x006F7CA0)
  |           |
  |           +-- Cell threat fallback (0x006F8C10)
  |
  +-- vtable+0x3A8 --> Can_Fire_At (validate target)
  +-- FUN_0042D170 --> pathfind distance check

FootClass::Greatest_Threat_Scan (0x004D5690)
  [called from InfantryClass/AircraftClass wrappers]
  +-- validates current target
  +-- scans cells in directions for firing positions
  +-- FUN_006F7220 --> line-of-fire check
  +-- approaches target via locomotor
```

---

## 2. Greatest_Threat (0x006F8DF0) -- The Core Scanner

### Signature

```c
int* __thiscall TechnoClass::Greatest_Threat(
    TechnoClass* this,
    uint threat_flags,     // param_2: bitfield controlling scan behavior
    int* scan_origin,      // param_3: coordinates to scan around (leptons)
    char enemy_only        // param_4: if true, only scan specific enemy house
);
// Returns: pointer to best target (AbstractClass*), or NULL
```

### Threat Flag Bits (param_2)

| Bit(s) | Hex | Meaning |
|--------|-----|---------|
| 0 | 0x0001 | Scan mode: use weapon range (ground scan) |
| 1 | 0x0002 | Scan mode: use guard range |
| 2 | 0x0004 | Include neutral objects (not just enemies) |
| 3 | 0x0008 | Prioritize air targets (adds flag 0x8000 to evaluation) |
| 4 | 0x0010 | Include allies as targets (for repair/healing weapons) |
| 5-6 | 0x0020/0x0040 | Include special unit types |
| 8 | 0x0100 | Quick house-level scan only (returns house, not specific unit) |
| 14 | 0x4000 | Only target specific enemy house |
| Various | 0x1BA60 | Combined flags for special threat types |

### Scan Modes

The function has TWO distinct scan modes based on `(threat_flags & 3)`:

#### Mode A: Linear Array Scan (flags & 3 == 0)

When threat flags have neither bit 0 nor bit 1 set, the function performs a **flat iteration** over:

1. **All CellClass objects** (via `FUN_00412B40` cell iterator with scan radius) if flag 0x4 is set
2. **All TechnoClass instances** (`g_TechnoClass_Array`, count `g_TechnoClass_Count`) -- brute force scan of every unit on the map

This is the fallback/AI mode used when no specific range constraint applies.

#### Mode B: Expanding Cell-Square Scan (flags & 3 != 0)

When bit 0 or bit 1 is set, the function performs an **expanding square scan** centered on `scan_origin`:

1. Calculates `scan_radius` in cells from weapon range or guard range
2. Iterates outward from center, scanning 4 sides of each concentric square
3. For each cell: calls `Scan_Cell_For_Target` (0x006F8960) to find the best target in that cell
4. **Early termination**: if a target is found at 1/4 or 1/2 of the scan radius, returns immediately (optimization to avoid scanning the full area when a close target exists)

### Scan Radius Calculation

```
if (weapon_range < 0 && current_mission == GUARD):
    scan_radius = 0x200 (512 leptons = 2 cells)

if (scan_radius == 0):
    // No weapons: use sight range from TypeClass
    if (has_locomotor && TypeClass->BalloonHover):
        scan_radius = TypeClass->SightRange  // offset +0x5B8
    else:
        range = max(GetWeaponRange(0), GetWeaponRange(1))
        scan_radius = range

// Add GuardRange bonus from TypeClass (offset +0x68C)
scan_radius_cells = (scan_radius >> 8) + 1 + (TypeClass->GuardRange >> 8)

// If unit is a "Sensor" (vtable+0x400), add Rules->GuardAreaTargetingDelay (offset +0xF48)
if (Is_Sensor()):
    scan_radius_cells = GetSensorRange() + 1 + Rules->GuardAreaTargetingDelay
```

### Alliance Filtering

For each candidate target:

1. **Is_Ally check**: calls `HouseClass::Is_Ally(candidate)`
2. If ally AND weapon range >= 0 AND NOT player-controlled AND NOT AttackFriendlies:
   - Allied targets are REJECTED (goto LAB_006f894f)
   - Exception: if the ally has health below `Rules->ConditionYellow` (offset 0x16F8) AND the scanner is an infantry type, the ally may be targeted for repair
3. If ally AND weapon range < 0 (healing weapon):
   - Allied targets are ACCEPTED for healing
   - But only if they're damaged below the repair threshold
4. **AttackFriendlies** flag (TypeClass offset 0x6C0): overrides ally filtering, allows attacking friendlies
5. **Berserk mode** (field_0x298 != 0): attacks everything regardless of alliance
6. **Tethered mind control** (field_0x11C): if a mind-controller exists with CanAttack flag, targets enemies of the controller's house

### enemy_only Filter (param_4)

When `enemy_only` is true, only units belonging to a specific house are considered:
```c
if (enemy_only && target->Owner->ArrayIndex != this->Owner->EnemyHouseIndex)
    skip;
```

---

## 3. Evaluate_Candidate (0x006F7CA0) -- Per-Target Filtering

### Signature

```c
int* __thiscall TechnoClass::Evaluate_Candidate(
    TechnoClass* this,
    uint threat_flags,     // param_2
    uint internal_flags,   // param_3: derived from threat_flags
    int weapon_range,      // param_4: -1 for unlimited
    int* candidate,        // param_5: potential target (TechnoClass*)
    int** out_score,       // output: threat score
    int zone_id,           // param_7: movement zone for pathability
    int* scan_origin       // param_8: coordinates for distance calc
);
// Returns: 1 (low byte) if candidate is valid, 0 if rejected
```

### Filter Pipeline (in order)

1. **Can_Fire check**: `vtable+0x3BC` (GetFireError) -- if returns 5 (FIRE_CANT), reject. Exception: flags 0x18200 bypass this.

2. **Health check**: if `target->Health < 1` AND TypeClass has specific armor flag (offset 0xE5 == 2), reject.

3. **Warhead vs Armor check**: reads the warhead's Verses table:
   ```c
   warhead = weapon->WarheadType;
   armor_type = target->TypeClass->Armor;  // offset +0x9C
   verses = warhead->Verses[armor_type];   // array at warhead+0xA0, 8 bytes per entry
   if (verses <= 0.0):
       reject;  // weapon does zero or negative damage to this armor
   ```

4. **Submarine check**: if TypeClass has `Naval=yes` (offset 0x181 == 2) AND target is submerged (vtable+0x50) AND target's locomotor is not surfacing (vtable+0x1BC, field 0x3B != 2), reject.

5. **In limbo check**: if `target->InLimbo` (field 0x81), reject.

6. **Zero health check**: if `target->Health == 0`, reject.

7. **Cloaking check**: if `target->CloakState == 2` (fully cloaked):
   - Get cell sensor count for our house
   - If sensor count == 0 AND target belongs to different house: reject
   - If our sensors detect it: allow targeting

8. **Underground check**: if target is underground (field_0x3D5) AND timer not expired AND random check fails: reject (probabilistic detection).

9. **Distance/range check**: calculates 3D distance between scanner and target:
   ```c
   dx = target->X - this->X;
   dy = target->Y - this->Y;
   dz = target->Z - this->Z;
   distance = sqrt(dx*dx + dy*dy + dz*dz);
   if (weapon_range > 0 && distance > weapon_range):
       reject;
   if (weapon_range == 0):
       // Use TypeClass->SightRange (offset +0x5B8) as fallback
       if (distance > sight_range):
           reject;
   ```

10. **Can_Fire_At validation**: `vtable+0x3A8` -- final fire validation (checks weapon cooldown, ammo, etc.)

11. **Player visibility checks**: if target is not `Selectable` (TypeClass offset 0x231), reject. If `Insignificant` (0x232) and not mind-controllable, special handling.

12. **Type-specific filtering**:
    - **TypeClass->IsInsignificant** (0x232): skip if not mind-controllable
    - **TypeClass->LegalTarget** (0x231): must be true
    - **Civilian check** (0x1572): buildings with Civilian=yes may be skipped
    - **Occupant check**: buildings with occupants (0x800 offset) required if flag 0x40 set

13. **Bridge check**: if both attacker and target are on bridges, they must be on the SAME bridge (same OnBridge flag). This prevents units on one bridge level from targeting units on another.

14. **Zone compatibility**: if zone_id parameter is not -1, checks that target is in the same movement zone (for pathability).

---

## 4. Calculate_Threat_Score (0x0070CD10) -- The Math

### Signature

```c
float10 __thiscall TechnoClass::Calculate_Threat_Score(
    TechnoClass* this,       // the scanning unit
    ObjectClass* target,     // the candidate target
    int* reference_coords    // coordinates to measure distance from (leptons)
);
```

### Coefficient Selection

The function first selects which coefficient set to use:

```c
// Condition: *(char*)(this->Owner + 0x1fb)  [Owner = this+0x21C; field at HouseClass+0x1FB]
// False (== '\0') = NOT human-controlled (AI/no human) → use Dumb/global Rules coefficients
// True (!= '\0')  = human-controlled or per-type coefficients available → use TypeClass values
// (corrected 2026-05-28: was "if Owner->IsHumanControlled == false → AI branch uses TypeClass";
//  binary shows OPPOSITE: false branch reads RulesClass, true branch reads TypeClass.
//  Verified via decompile_function 0x0070CD10 — ROOT CAUSE: OPERATOR_OR_ORDER_DRIFT)

if (*(char *)(this->Owner + 0x1fb) == '\0') {
    // NOT human-controlled: use global "Dumb" defaults from RulesClass (4 doubles)
    Coeff_A        = Rules[0x1068]; // (double, 8 bytes)
    Coeff_B        = Rules[0x1070]; // (double, 8 bytes)
    StrengthCoeff  = Rules[0x1080]; // (double, 8 bytes)
    DistanceCoeff  = Rules[0x1088]; // (double, 8 bytes)
} else {
    // Human-controlled or unit with CanAttack typeclass: use per-type TypeClass values
    Coeff_A        = TypeClass[0x2c8]; // (double, 8 bytes)
    Coeff_B        = TypeClass[0x2d0]; // (double, 8 bytes)
    StrengthCoeff  = TypeClass[0x2e0]; // (double, 8 bytes)
    DistanceCoeff  = TypeClass[0x2e8]; // (double, 8 bytes)
}
// Note: TechnoTypeClass+0x2C0 is SpecialThreatValue (a multiplicand, NOT a coefficient)
// Note: only 4 coefficient doubles are loaded in each branch (not 5 as previously stated).
//  0x2D8 in TypeClass and 0x1078 in RulesClass are NOT read in Calculate_Threat_Score.
```

### Score Formula

```
Score = base_value + weapon_threat + special_threat + strength_factor + distance_factor

Where:
  base_value = DAT_007F4E90  (small constant, likely 0.5)

  weapon_threat = Verses[target_armor] * WarheadDamage * SpecialThreatCoeff
    -- How much damage our weapon does to this target's armor type
    -- If target is our current target: negated (preference for sticking with current)

  special_threat = target->TypeClass->SpecialThreatValue * ThreatCoeff_Unknown
    -- Per-type special threat value (e.g., superweapon buildings rated higher)
    -- Plus EnemyHouseThreatBonus (Rules+0x1090) if target belongs to our designated enemy house

  strength_factor = (weapon_range_cells + health_ratio * StrengthCoeff)
    -- Weapon range in cells as base
    -- Health ratio (0.0-1.0) scaled by strength coefficient

  distance_factor = max(0, distance_cells - weapon_range_cells) * DistanceCoeff
    -- Distance penalty: further targets get lower scores
    -- Only counts distance BEYOND weapon range (within-range targets get no penalty)
    -- distance_cells computed from 3D Euclidean distance
```

### Special Modifiers

After the base score:

1. **Prefer wounded targets**: if TypeClass has `PreferWounded` flag (offset 0x394 == 1):
   - Target health < half max health: score doubled (`*= 2`)
   - Target health == 0: score halved (`/= 2`)

2. **Enemy house override**: if scanner's owner has a specific enemy house set (offset 0x249), AND target belongs to a DIFFERENT house than the designated enemy: score forced to 1 (minimum).

3. **Base defense bonus** (flag 0x800): if target is a building (type 6) with defense value (TypeClass offset 0xEE0):
   - `score += defense_value * 1000`

4. **Power plant bonus** (flag 0x8000): if target is a building with power output (offset 0x1580):
   - `score += power * 1000`

5. **Factory bonus** (flag 0x10000): if target is a factory building (offset 0x1552):
   - `score += 1000`

6. **No defense penalty** (flag 0x1000): if target is a building with no defense capability (offset 0xEB8 == 0):
   - `score = 0` (skip defenseless buildings)

### ThreatAvoidance_Modifier (0x006F79A0)

After computing the base score, `Evaluate_Candidate` calls this modifier:

```c
float10 ThreatAvoidance_Modifier(TechnoClass* this, short* cell_coords);
```

This scans a square around the target's cell position (radius from `Rules->ThreatAvoidanceRadius`, offset 0x1430). For each cell in the square, it checks if there's an ALLIED building. Each allied building found multiplies the threat score by `DAT_007E1738` (a constant < 1.0, likely ~0.8). This means targets near our own buildings get LOWER threat scores, discouraging attacks near our base (since the enemy is "protected" by proximity to our structures -- this makes AI units prefer isolated enemy units over those near our defenses).

---

## 5. Scan_Cell_For_Target (0x006F8960) -- Cell-Level Scanning

### Signature

```c
uint __thiscall TechnoClass::Scan_Cell_For_Target(
    TechnoClass* this,
    uint threat_flags,
    uint internal_flags,
    short* cell_xy,          // packed cell coordinates
    int weapon_range,
    int** out_target,        // output: best target found
    int** out_score,         // output: best threat score
    uint zone_id
);
```

### Cell Occupant Iteration

Each cell has a linked list of occupants:
- `cell+0xE8`: first TechnoClass occupant (on the cell)
- `cell+0xE4`: fallback occupant pointer (alt list head)
- Each occupant links to next via `this+0x30`

The function iterates the linked list and for each occupant:

1. **Self-check**: skip if candidate == this
2. **Valid object check**: `field_0x14 & 1` must be set (object exists on map)
3. **Healing weapon check**: if weapon range < 0 (healing):
   - Only target allies below `Rules->ConditionYellow` (health threshold)
4. **Enemy check**: `HouseClass::Is_Ally` -- reject allies (unless special flags)
5. **AttackFriendlies override**: TypeClass offset 0x6C0
6. **Berserk override**: field_0x298
7. **Mind-control tethered override**: field_0x11C with allied enemy check
8. **Infantry engineer special**: type 0xF with garrison flags, special building capture logic
9. **Best-of-cell**: evaluates each candidate via `Evaluate_Candidate`, keeps highest score

### Zone Filtering

If `zone_id != -1`, the cell's zone is checked:
```c
cell_zone = MapClass::GetZoneID(cell, this->TypeClass->SpeedType);
if (cell_zone != zone_id):
    reject entire cell;
```

---

## 6. FootClass::Mission_Hunt (0x004D5350) and FootClass::Mission_Patrol (0x004D4280)

> **Correction 2026-05-28**: The original §6 heading and all content below described the function at `0x004D4280`, which the binary labels `FootClass__Mission_Patrol` (737-line 4-state machine). The true `FootClass__Mission_Hunt` is at `0x004D5350` (175 lines, much simpler). Both are documented below. ROOT CAUSE: RTTI_LABEL_DRIFT — verified via `get_function_by_address 0x004D4280`, `get_function_by_address 0x004D5350`, and `search_functions Mission_Hunt`.

### FootClass::Mission_Hunt (0x004D5350) — The Actual Function

Simplified delegation function. Checks `TypeClass+0x6D4` (unknown flag) first; if set, skips directly to move. Otherwise:
1. Calls `vtable+0x39c` with current coords — checks if unit can fire at something already.
2. If it can fire: checks for infantry/engineer/garrison capture conditions (`TypeClass+0xEC3`, `+0xEC2`, `+0xEC6`), then issues fire order (`vtable+0x1E8`) or advances toward target (`vtable+0x480`).
3. If it cannot fire: calls `HouseClass::IsPlayerControl()`. If not player-controlled and not campaign mode (`g_GameMode == 0`), calls `FUN_0050def0` to find a target coord, then moves there (`vtable+0x480`, mission 2). If player-controlled, calls `vtable+0x478`.

This function does NOT contain the 4-state machine. That machine lives in `Mission_Patrol`.

### FootClass::Mission_Patrol (0x004D4280) -- 4-State Machine

*(Previously mislabelled as Mission_Hunt in this document.)*

### States

| State | Description |
|-------|-------------|
| 0 | **Acquire target**: call `Greatest_Threat(2, &my_coords, 0)` via `vtable+0x3C4`. If found, validate with `Can_Fire_At` via `vtable+0x3A8`. Check pathfinding distance via `PathfinderClass::EstimateZoneCost`. Set target, transition to state 1. |
| 1 | **Pursue target**: check if target still valid. If target destroyed/moved, re-acquire. If friendly building under attack (type 6, allied, above `ConditionRed`), consider switching targets. Compare path distances to decide. |
| 2 | **Approach via waypoint**: move toward last known cell position. Re-scan for targets along the way via `vtable+0x3C4`. |
| 3 | **Reset**: if no transport, clear and restart. |

### Target Persistence

Once a target is acquired in state 0, the unit sticks with it in state 1 unless:
- Target is destroyed
- Target becomes allied (health recovery above `Rules->ConditionRed`, offset `0x1708`, for friendly-building defense switch)
- A closer or higher-scoring target is found during re-evaluation
- Pathfinding fails (distance exceeds threshold: weapon_range_cells + 6)

### Path Distance Gating

Before committing to a target, `Mission_Patrol` checks pathfinding distance via `PathfinderClass::EstimateZoneCost`:
```c
path_distance = PathfinderClass::EstimateZoneCost(my_cell, target_cell, this, zone_flags, zone, -1);
if (path_distance > weapon_range_cells + 6):
    reject target;  // path is too long/winding
// Note: doc previously cited FUN_0042D170 — actual function is PathfinderClass::EstimateZoneCost
// (corrected 2026-05-28; verified via decompile_function 0x004D4280)
```

---

## 7. Key Struct Offsets

### TechnoTypeClass Threat Fields

| Byte Offset | Field Name | Type | INI Key |
|-------------|-----------|------|---------|
| 0x09C | Armor | int | `Armor=` |
| 0x231 | LegalTarget | byte(bool) | (derived) |
| 0x232 | Insignificant | byte(bool) | (derived) |
| 0x2C0 | SpecialThreatValue | double | `SpecialThreatValue=` |
| 0x2C8 | TargetSpecialThreatCoefficientDefault | double | `TargetSpecialThreatCoefficientDefault=` |
| 0x2D0 | (second threat coeff) | double | (possibly DumbTargetSpecialThreatCoefficient) |
| 0x2D8 | TargetSpecialThreatCoefficient | double | `TargetSpecialThreatCoefficient=` |
| 0x2E0 | TargetStrengthCoefficient | double | `TargetStrengthCoefficient=` |
| 0x2E8 | TargetDistanceCoefficient | double | `TargetDistanceCoefficient=` |
| 0x394 | PreferWounded | int | (set to 1) |
| 0x5B4 | SpeedType | int | `SpeedType=` |
| 0x5B8 | SightRange | int (leptons) | `Sight=` |
| 0x5E4 | CanAttack | byte(bool) | (derived from weapons) |
| 0x670 | ThreatPosed | int | `ThreatPosed=` |
| 0x674 | Points | int | `Points=` |
| 0x68C | GuardRange | int (leptons) | `GuardRange=` |
| 0x695 | CloseRange | byte(bool) | `CloseRange=` |
| 0x6B0 | IsGattling | byte(bool) | `IsGattling=` |
| 0x6C0 | AttackFriendlies | byte(bool) | `AttackFriendlies=` |
| 0xC94 | Cloakable | byte(bool) | `Cloakable=` |
| 0xD20 | DontScore | byte(bool) | `DontScore=` |
| 0xD27 | HunterSeeker | byte(bool) | `HunterSeeker=` |
| 0xD31 | ImmuneToRadiation | byte(bool) | `ImmuneToRadiation=` |
| 0xD33 | CanBeScattered | byte(bool) | (scatter flag) |
| 0xD34 | OpportunityFire | byte(bool) | (retarget check) |
| 0xD69 | BalloonHover | byte(bool) | `BalloonHover=` |

### RulesClass Threat Globals

| Byte Offset | Field Name | Type | INI Key |
|-------------|-----------|------|---------|
| 0x1040 | TargetSpecialThreatCoefficientDefault | double | `TargetSpecialThreatCoefficientDefault=` |
| 0x1048 | (second default coeff) | double | |
| 0x1060 | TargetDistanceCoefficientDefault | double | `TargetDistanceCoefficient=` (fallback) |
| 0x1068 | Dumb_Coeff_A (human-ctrl path) | double | read at Rules+0x1068 in NOT-human-ctrl branch of Calculate_Threat_Score (corrected 2026-05-28: was labelled "TargetSpecialThreatCoefficientDefault / AI evaluation set" — duplicate label was contradictory; binary uses this offset in the false/non-human branch, verified via decompile_function 0x0070CD10 — ROOT CAUSE: INFERENCE_HARDENED) |
| 0x1070 | Dumb_Coeff_B | double | read at Rules+0x1070 in NOT-human-ctrl branch |
| 0x1080 | Dumb_StrengthCoeff | double | read at Rules+0x1080 in NOT-human-ctrl branch |
| 0x1088 | Dumb_DistanceCoeff | double | read at Rules+0x1088 in NOT-human-ctrl branch |
| 0x1090 | EnemyHouseThreatBonus | double | `EnemyHouseThreatBonus=` |
| 0x0F48 | GuardAreaTargetingDelay | int | (sensor bonus to scan radius) |
| 0x16F8 | ConditionYellow | double | `ConditionYellow=` (health threshold for repair) |
| 0x1708 | ConditionRed | double | `ConditionRed=` (health threshold) |
| 0x1430 | ThreatAvoidanceRadius | int (leptons) | (radius for avoidance scan) |

### TechnoClass Instance Fields (Runtime)

| Byte Offset | Field Name | Purpose |
|-------------|-----------|---------|
| 0x021C | Owner | HouseClass* -- owning house |
| 0x0298 | Berserk flag | nonzero = attacks everything |
| 0x02B4 | Target | AbstractClass* -- current primary target |
| 0x041A | DiscoveredByPlayer | byte |
| 0x041B | DiscoveredByCurrentPlayer | byte |
| 0x0460-0x046C | Gattling target arrays | tracking for gattling weapons |

### CellClass Fields

| Byte Offset | Field Name | Purpose |
|-------------|-----------|---------|
| 0x0E4 | FirstObject_alt | ObjectClass* -- first occupant (alt list) |
| 0x0E8 | FirstObject | ObjectClass* -- first TechnoClass occupant |
| 0x030 | NextObject | ObjectClass* -- linked list to next occupant in cell |
| 0x140 | Flags | bitfield: bit 8 = on bridge |
| 0x044 | OverlayType | int -- overlay type index |
| 0x050 | OwnerHouse | int -- house index of cell owner |

---

## 8. Special Cases

### Anti-Air Units

Aircraft (RTTI type 2) have special handling:
- `GetWeaponRange(-1)` checks if ANY weapon can target air. If < 0, the threat flags are modified to `(flags & 3) | 0x4008`, forcing air-target-only mode.
- Infantry (type 0xF) with negative weapon range similarly gets `(flags & 3) | 0x4010`.

### Gattling Weapons

If `TypeClass->IsGattling` (offset 0x6B0) is set, the function maintains separate target/score arrays for primary and secondary weapon groups (fields 0x440-0x46C). Each weapon group independently tracks its best target.

### HunterSeeker Units

Units with `HunterSeeker=yes` (TypeClass offset 0xD27) have special behavior in `Greatest_Threat_Scan`: they bypass the normal cell-scanning logic and instead move directly toward their assigned target.

### CloseRange Infantry

Infantry with `CloseRange=yes` (TypeClass offset 0x695):
- Scan range is clamped to a short distance (`_DAT_007E9248`, likely 0x14C = 332 leptons)
- They approach targets more aggressively via locomotor destination setting
- Special cell-by-cell approach logic to get within melee range

### Engineer / IFV Infantry (Type 0xF)

Infantry type 0x0F (engineers, IFVs) have special garrison and repair targeting:
- Check for `team->IsBaseDefense` flag (offset 0xEC3 in TeamTypeClass)
- Can target allied buildings below `ConditionRed` health for repair
- Can target enemy buildings for capture/garrison

### Submarines

`TypeClass->Naval` (offset 0x181 == 2): submarines that are submerged are invisible to most units. The check verifies both the `Is_Submerged` vtable call and the locomotor surfacing state before allowing targeting.

### Mind-Controlled Units

If the scanning unit has a mind controller (field 0x11C != NULL), and that controller's TypeClass has `CanAttack` (offset 0x5E4), the unit uses the controller's house for alliance checks. This means a mind-controlled unit attacks enemies of its controller, not its original owner.

---

## 9. Performance Notes

- `DAT_00A8EC34` is incremented each time `Greatest_Threat` is called (global call counter)
- The expanding-square scan has early termination at 1/4 and 1/2 radius if a target is found
- The all-TechnoClass-array scan is O(n) where n = total units on map -- this is the brute force path
- Cell-based scanning is more efficient for short-range units
- `FUN_0042D170` (pathfinding distance) is expensive; it's only called when comparing targets, not for initial filtering
