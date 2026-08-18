# Power System Edge Cases & AI Power Management

> **Companion doc.** The authoritative power system reference is
> [POWER_SYSTEM_GHIDRA_REPORT.md](POWER_SYSTEM_GHIDRA_REPORT.md).
> If any detail here conflicts with the main report, the main report is correct.

Research from Ghidra decompilation of `gamemd.exe`. All offsets verified live.

## 1. AI Power Management

### 1a. AI Prioritizes Building Power Plants When in Deficit

**Confidence: HIGH** -- fully decompiled from FUN_004FE3E0 (AI_PickBuildItem).

When the AI processes its build queue, it performs a **power budget check** before placing
any building that would increase power drain:

```c
// FUN_004FE3E0 @ ~0x4FE720
// Check: would building this drain exceed our output?
if (PowerDrain + BuildingType->PowerDrainValue(0xEE4) > PowerOutput - PendingDrain) {
    // This building would cause a power deficit!

    // Exception 1: If building is in BuildConst list (RulesClass+0x8B0, count at +0x8BC),
    // skip the power check entirely. Construction Yards always get built.
    for (i = 0; i < RulesClass->BuildConstCount; i++) {
        if (BuildConstList[i] == buildingType) goto SKIP_POWER_CHECK;
    }

    // Exception 2: If building has zero drain (power plant itself), skip
    if (BuildingType->PowerDrain <= 0) goto SKIP_POWER_CHECK;

    // Exception 3: If base power timer hasn't expired, skip
    // (HouseClass+0x2A4/0x2AC timer pair)

    // If no exceptions apply AND HasPoweredCenter(+0x577B) is false:
    // INSERT A POWER PLANT into the build queue instead!
    if (!House->HasPoweredCenter) {
        if (House->Side == 0)       // Allied
            powerPlant = RulesClass->AlliedPower (offset 0x89C);
        else if (House->Side == 2)  // Yuri
            powerPlant = RulesClass->YuriPower (offset 0x8A8);
        else {                      // Soviet
            // Check if we have enough buildings to warrant advanced power
            // Uses FUN_00505360 to evaluate threshold
            if (enoughBuildings)
                powerPlant = RulesClass->SovietAdvancedPower (offset 0x8A4);
            else
                powerPlant = RulesClass->SovietPower (offset 0x8A0);
        }
        // Insert power plant build order BEFORE the current item
        buildQueue.Insert(powerPlantIndex, currentPosition);
    }
}
```

The `BuildPower` INI key (parsed at RulesClass offsets 0x8C8+) defines a list of power
plants. For each side, the AI selects from a fixed set of power plant types defined at
RulesClass offsets 0x89C (Allied), 0x8A0/0x8A4 (Soviet base/advanced), 0x8A8 (Yuri).

**Key insight**: The AI does NOT sell buildings when power is low. Instead, it **interrupts
its build queue** to insert a power plant build order whenever it detects that the next
building would cause a power deficit.

### 1b. AI Does NOT Sell Buildings for Power

**Confidence: HIGH** -- searched all AI functions.

There is no function in the AI code that sells low-priority buildings when power is
insufficient. The AI's only response to a power deficit is to insert power plant
construction into the build queue (as described above).

### 1c. AI Does NOT Toggle Buildings Offline

**Confidence: HIGH** -- verified in GoOnline/GoOffline and AI code.

`BuildingClass::GoOffline` (0x452360) requires:
- The building must currently be online (`+0x660 != 0`)
- The building type must have `PowerDrain > 0` OR `Powered=yes`

`BuildingClass::GoOnline` (0x452260) requires:
- The building must be offline (`+0x660 == 0`)
- **The EMP counter at +0x504 must equal 0** (cannot go online while EMPed)

The GoOnline/GoOffline functions are called from player commands (toggle power button)
and from the `TogglePowerOrGate` function (0x447110). There is no AI code that calls
these functions to strategically toggle buildings. The AI never manually takes buildings
offline to conserve power.

### 1d. AI Base Layout Includes Power Plants

**Confidence: HIGH** -- decompiled from FUN_005082C0.

The AI base planner (FUN_005082C0) uses the `BuildPower` list from RulesClass to
intersperse power plants in the base layout alongside defense structures. The planner
walks the base perimeter in 4 directions and alternates between placing defense buildings
(from `BuildDefense` list) and power buildings (from `BuildPower` list).

### 1e. AI Difficulty Affects Build Choices

The random check at the top of FUN_004FE3E0 uses:
```c
int threshold = RulesClass->DifficultyBuildPercent[House->Difficulty];
// (RulesClass offset 0xDD8 = pointer to array, indexed by House->Difficulty at +0x184)
if (Random(0,99) < threshold) { ... }
```

This means the AI's willingness to build on lower difficulties is gated by a random
percentage check per difficulty level.

## 2. Power During EMP (EMPulse)

### 2a. The +0x504 Field is the EMP Counter, NOT WarpCount

**Confidence: HIGH** -- verified in constructor, IsOperational, and Update.

The field at `TechnoClass+0x504` (accessed as `param_1[0x141]`) is the **EMP lock
remaining counter**. This was previously (incorrectly) documented as "WarpCount" in
some references.

**Evidence:**
- `TechnoClass::Constructor` (0x6F2B40): `param_1[0x141] = 0;` -- initialized to 0
- `BuildingClass::IsOperational` (0x4555D0): `if (0 < *(int*)(this + 0x504)) return false;`
- `FUN_0070EFD0` (IsUnderEMP helper): `return 0 < *(int*)(param_1 + 0x504);`
- `FUN_004C54E0` (EMPulse tick): `building[0x141] = empDuration;` -- sets the counter
- `TechnoClass::Update` (0x6F9E50): decrements and handles expiry

### 2b. EMP Disables IsOperational but Does NOT Affect Power Output/Drain

**Confidence: HIGH** -- verified in GetPowerOutput and GetPowerDrain.

`BuildingClass::GetPowerOutput` (0x44E7B0):
- Calls `vtable[0x1D4]()` which is the **InLimbo** check (returns `*(this + 0x270)`)
- If in limbo: output = 0
- Checks `HasPower` flag (offset +0x660)
- Multiplies by health ratio if positive output
- **Does NOT check +0x504 (EMP counter)**

`BuildingClass::GetPowerDrain` (0x44E880):
- Calls `vtable[0x1D4]()` (InLimbo check)
- If in limbo: drain = 0
- Checks `HasPower` flag (offset +0x660)
- **Does NOT check +0x504 (EMP counter)**

**Result**: An EMPed building **still produces and drains power** normally. The EMP
effect makes `IsOperational()` return false, which disables the building's functionality
(gap generator, cloak generator, weapons, production, etc.), but the building continues
to contribute to the house's power totals.

**Why**: The critical distinction is the `IsOnline` flag at `+0x660`:
- `GoOffline` (0x452360) sets `+0x660 = 0` -- this causes GetPowerOutput/GetPowerDrain
  to return 0 (they both check `+0x660`).
- `EMPulse tick` (FUN_004C54E0) calls `FUN_00452480` (shutdown effects: stops cloak,
  sensors, weapons, laser fences) but does **NOT** clear `+0x660`. The building
  remains "online" from the power system's perspective.
- So EMPed buildings have `+0x660 = 1` (still counted for power) but `+0x504 > 0`
  (makes IsOperational return false).

This is consistent with the original game's behavior: EMPing a Tesla Reactor does NOT
reduce your power output, but EMPing a Battle Lab makes it non-operational (so you can't
build tech units even though the power is fine).

### 2c. EMP Counter Tick-Down

**Confidence: HIGH** -- decompiled from TechnoClass::Update at 0x6FAE60.

```c
// Near end of TechnoClass::Update (0x6F9E50)
if (this->EMPLockRemaining > 0) {
    this->EMPLockRemaining--;
    if (this->EMPLockRemaining == 0) {
        // EMP just expired
        if (IsBuilding()) {
            // Building: call GoOnline-like recovery
            if (!type->IsImmune) {
                FUN_00452410();  // Recovery animation
                if (type->HasRadar) {
                    house->NeedsEffectsUpdate = 1;
                }
            }
        } else {
            // Non-building techno: clear temporal/animation state
            if (this->TemporalTarget != 0) {
                // Clean up temporal weapon effects
            }
            // Clear sparkle animations from EMPulse
        }
    }
}
```

### 2d. EMPulse Application (FUN_004C54E0)

**Confidence: HIGH** -- fully decompiled.

The EMPulse effect iterates all technos within the pulse radius. For each:
- If it's a non-building unit: sets `techno[0x141] = empDuration` and creates sparkle anim
- If it's a building at the epicenter cell: calls `FUN_00452480()` (GoOffline-like) and
  sets `building[0x141] = empDuration`, then marks house for power recalc

The EMP duration comes from `EMPulseClass+0x30` (the Duration field of the EMP instance).

## 3. Power During Chronoshift

### 3a. Chronoshift Does NOT Apply to Buildings

**Confidence: HIGH** -- verified from game mechanics.

The Chronosphere super weapon only targets **units** (vehicles, infantry). Buildings
cannot be chronoshifted. Therefore, the question "does a chronoshifted building still
produce/drain power?" is moot -- it cannot happen in normal gameplay.

The Chrono Legionnaire's temporal weapon (erasing effect) is managed by `TemporalClass`,
which is a separate system from the Chronosphere. Temporal weapons erase units over time
and do not interact with building power at all.

### 3b. No WarpCount Field Exists at +0x504

The field at +0x504 was previously hypothesized to be "WarpCount" but is confirmed to
be the **EMP counter** (see section 2a above). There is no separate "warp count" field
that affects building power.

## 4. Power During Iron Curtain

### 4a. Iron Curtain Does NOT Affect Power

**Confidence: HIGH** -- verified in IsOperational (0x4555D0).

The `IsOperational` function checks these conditions:
1. HasPower / IsOnline (+0x660)
2. EMP counter (+0x504) > 0
3. Health (+0x6C) == 0
4. Powered + PowerDrain + ratio check
5. PoweredSpecial + timer check
6. NeedsEngineer check
7. Mission != SELLING/DECONSTRUCTING

**Iron Curtain is NOT checked in IsOperational.** An Iron Curtained building remains
fully operational and continues to produce/drain power normally. The Iron Curtain only
makes the building invulnerable to damage -- it has no effect on power or functionality.

### 4b. Iron Curtain Timer Fields

The Iron Curtain timer is stored at TechnoClass offsets:
- +0x4FC (`param_1[0x13F]`): Timer start frame (initialized to `g_CurrentFrameCounter`)
- +0x500 (`param_1[0x140]`): Timer duration (initialized to 0)

These are adjacent to but separate from the EMP counter at +0x504.

## 5. Power and Crates

### 5a. No Power Crate Exists

**Confidence: HIGH** -- verified from crate debug strings.

The crate system supports these effects (from debug strings at 0x81CE5C-0x81D0A4):
- Firepower, Speed, Armor, ICBM, Base Healing, Veterancy, Cloaking Device,
  Napalm, Explosives, Money, Unit, Reveal, Shroud, Tiberium, Poison Gas

**There is no "Power" crate.** No crate effect grants a power bonus or modifies
power output/drain.

## 6. The +0x504 Field (Corrected Documentation)

### Previous (Incorrect) Assumption
The field at BuildingClass+0x504 was assumed to be "WarpCount" based on the
`GoOnline` function's check `if (param_1[0x141] == 0)`.

### Correct Identification: EMP Lock Counter
The field is the **EMP lock remaining counter** (TechnoClass+0x504):

| Aspect | Detail |
|--------|--------|
| Class | TechnoClass (inherited by BuildingClass) |
| Offset | +0x504 (index [0x141]) |
| Type | int (countdown timer) |
| Init | 0 (in TechnoClass::Constructor at 0x6F2B40) |
| Written by | EMPulseClass tick (FUN_004C54E0): sets to EMP duration |
| Decremented by | TechnoClass::Update (0x6F9E50): decrements by 1 each tick |
| Read by | IsOperational (0x4555D0): if > 0, returns false |
| Read by | GoOnline (0x452260): if != 0, refuses to go online |
| Read by | FUN_0070EFD0 (IsUnderEMP): returns (value > 0) |

### Why GoOnline Checks This Field
`GoOnline` checks `+0x504 == 0` because a building cannot be manually toggled
online while under an EMP effect. The EMP must expire (count down to 0) first.
When the counter reaches 0 in TechnoClass::Update, the building automatically
recovers (plays recovery animation and marks house for power recalculation).

## 7. Powered=yes with Positive Power= Output

### 7a. The Logic in IsOperational

**Confidence: HIGH** -- directly verified in decompilation.

```c
// From IsOperational (0x4555D0):
if (Type->Powered(+0x1573) && Type->PowerDrain(+0xEE4) > 0) {
    double ratio = House->GetPowerRatio();
    if (ratio < 1.0 && UpgradeLevel < 2) return false;
}
```

The check requires BOTH conditions:
1. `BuildingType->Powered` (offset 0x1573) is true
2. `BuildingType->PowerDrain` (offset 0xEE4) is greater than 0

### 7b. What Happens with Positive Power= and Powered=yes

If a building has `Power=200` (positive):
- `BuildingType->PowerOutput` (offset 0xEE0) = 200
- `BuildingType->PowerDrain` (offset 0xEE4) = **0** (positive Power= goes to output only)

Therefore: `Type->PowerDrain > 0` evaluates to **false** (0 > 0 is false).

**Result**: The power-ratio check is **skipped entirely**. A building with positive
`Power=` and `Powered=yes` will NEVER be disabled by low power. The `Powered=yes`
flag is effectively ignored because the drain check gates the power ratio evaluation.

This is presumably intentional: a power plant that's also `Powered=yes` would create
a circular dependency (need power to produce power). The engine avoids this by requiring
both `Powered=yes` AND positive drain before applying the power check.

### 7c. Negative Power= and Powered=yes

If a building has `Power=-100` (negative):
- `BuildingType->PowerOutput` (offset 0xEE0) = 0
- `BuildingType->PowerDrain` (offset 0xEE4) = 100

In this case: `Type->PowerDrain > 0` is **true** (100 > 0), so the full power ratio
check applies. If the house power ratio < 1.0 and UpgradeLevel < 2, the building
becomes non-operational.

This is the normal case for buildings like the Ore Purifier, Psychic Sensor, etc.
that consume power and stop working during low power.

## Summary of Edge Cases

| Scenario | +0x660 (IsOnline) | +0x504 (EMP) | Power Output | Power Drain | IsOperational |
|----------|-------------------|--------------|-------------|-------------|---------------|
| Normal building | 1 | 0 | Normal | Normal | Normal |
| EMPed building | **1** (unchanged) | **> 0** | **Still active** | **Still active** | **FALSE** (EMP check) |
| Iron Curtained building | 1 | 0 | Normal | Normal | Normal (IC not checked) |
| Building in Limbo | 1 | 0 | 0 (limbo check) | 0 (limbo check) | N/A |
| Toggled offline (GoOffline) | **0** | 0 | **0** (+0x660 check) | **0** (+0x660 check) | Depends on other checks |
| Powered=yes + Power=200 | 1 | 0 | 200 | 0 | Normal (drain check skipped) |
| Powered=yes + Power=-100 | 1 | 0 | 0 | 100 | False when low power |
