# Power= INI Parsing and Building Power Lifecycle

> **Companion doc.** The authoritative power system reference is
> [POWER_SYSTEM_GHIDRA_REPORT.md](POWER_SYSTEM_GHIDRA_REPORT.md).
> If any detail here conflicts with the main report, the main report is correct.

Research from live Ghidra MCP decompilation of `gamemd.exe`. All addresses and offsets
verified against the binary.

---

## 1. Power= INI Parsing (BuildingTypeClass::ReadINI @ 0x45FE50)

### Assembly at 0x461060–0x4610D8

The `Power=` key is parsed at address 0x461073. The code reads a single signed integer
from INI, then splits it into separate output/drain fields:

```asm
; --- Compute default value to pass to INI reader ---
; If current PowerOutput > 0, default = PowerOutput
; If current PowerOutput <= 0, default = -(current PowerDrain)
00461060: MOV  EAX, [EBP+0xEE0]     ; EAX = current PowerOutput
00461066: CMP  EAX, EDI             ; EDI = 0 (set at 0x461054)
00461068: JG   0x461072             ; if PowerOutput > 0, use it as default
0046106A: MOV  EAX, [EBP+0xEE4]    ; else EAX = current PowerDrain
00461070: NEG  EAX                  ; negate it (drain stored positive, pass as negative)

; --- Read "Power" key from INI ---
00461072: PUSH EAX                  ; default value
00461073: PUSH 0x81938C             ; "Power" string
00461078: PUSH EBX                  ; section name
0046107B: CALL 0x5276D0             ; INI::ReadInt(section, key, default)

; --- Split result into output/drain ---
00461080: CMP  EAX, EDI             ; compare result with 0
00461082: MOV  [EBP+0xEE0], EAX    ; store raw value in PowerOutput initially
00461088: JGE  0x46109A             ; if value >= 0, jump to "positive" path

; NEGATIVE PATH: Power= is negative (building drains power)
0046108A: NEG  EAX                  ; make positive
0046108C: MOV  [EBP+0xEE4], EAX    ; PowerDrain = abs(Power)
00461092: MOV  [EBP+0xEE0], EDI    ; PowerOutput = 0
00461098: JMP  0x4610A0             ; continue to ExtraPower

; POSITIVE PATH: Power= is positive or zero (building produces power)
0046109A: MOV  [EBP+0xEE4], EDI    ; PowerDrain = 0
                                    ; PowerOutput already set at 0x461082
```

### Pseudocode

```c
// Default value reconstruction: if previously had output, use it;
// if previously had drain, pass as negative
int default_val;
if (this->PowerOutput > 0)
    default_val = this->PowerOutput;
else
    default_val = -(this->PowerDrain);

int power = INI_ReadInt(section, "Power", default_val);

if (power >= 0) {
    this->PowerOutput = power;   // +0xEE0
    this->PowerDrain  = 0;       // +0xEE4
} else {
    this->PowerOutput = 0;       // +0xEE0
    this->PowerDrain  = -power;  // +0xEE4 (stored as positive)
}
```

**Confirmed: The hypothesis was exactly right.** Positive Power= goes to +0xEE0, negative
Power= is negated and stored in +0xEE4, the other field is zeroed.

### ExtraPower= Parsing (0x4610A0–0x4610D8)

Identical logic for `ExtraPower=`:

```asm
004610A0: MOV  EAX, [EBP+0xEE8]    ; current ExtraPowerBonus
004610A6: CMP  EAX, EDI
004610A8: JG   0x4610B2
004610AA: MOV  EAX, [EBP+0xEEC]    ; current ExtraDrainBonus
004610B0: NEG  EAX

004610B2: PUSH EAX                  ; default
004610B3: PUSH 0x81A7B0             ; "ExtraPower" string
004610BB: CALL 0x5276D0             ; INI::ReadInt

004610C0: CMP  EAX, EDI
004610C2: MOV  [EBP+0xEE8], EAX
004610C8: JGE  0x4610D8

; Negative path:
004610CA: NEG  EAX
004610CC: MOV  [EBP+0xEEC], EAX    ; ExtraDrainBonus = abs(ExtraPower)
004610D2: MOV  [EBP+0xEE8], EDI    ; ExtraPowerBonus = 0

; Positive path:
004610D8: ...                       ; ExtraDrainBonus = 0
```

### Summary of BuildingTypeClass Power Fields

| Offset | Field | Source |
|--------|-------|--------|
| +0xEE0 | PowerOutput | `Power=` (positive part, or 0 if negative) |
| +0xEE4 | PowerDrain | `Power=` (abs of negative part, or 0 if positive) |
| +0xEE8 | ExtraPowerBonus | `ExtraPower=` (positive part) |
| +0xEEC | ExtraDrainBonus | `ExtraPower=` (abs of negative part) |

---

## 2. Building Under Construction and Power

### Does a building under construction produce/drain power?

**No.** Two independent mechanisms prevent it:

#### Mechanism 1: InLimbo flag (vtable+0x1D4 @ 0x70C5B0)

`GetPowerOutput` and `GetPowerDrain` both begin with:
```c
cVar1 = (**(code **)(*param_1 + 0x1D4))();  // InLimbo check
if (cVar1 != '\0') return 0;                 // In limbo → zero power
```

The vtable+0x1D4 function for BuildingClass reads field +0x270:
```c
// BuildingClass vtable[0x1D4] at 0x70C5B0
return *(byte *)(this + 0x270);  // InLimbo flag
```

While a building is being constructed in the factory (before placement), it is in limbo
(+0x270 = 1), so both GetPowerOutput and GetPowerDrain return 0.

#### Mechanism 2: IsOnline flag (+0x660)

`GetPowerOutput` additionally checks:
```c
if (base_power > 0 && IsOnline(+0x660))
    return ftol(base_power * GetHealthRatio());
return 0;
```

`GetPowerDrain` checks:
```c
if (InLimbo() || !IsOnline(+0x660)) return 0;
```

The IsOnline flag (+0x660) is initialized to 1 in the constructor, so placed buildings
start online.

#### Mechanism 3: AI_AssessPower skip conditions

In `HouseClass::AI_AssessPower` (0x508C30), the building iteration skips:
- `NULL` pointers
- `IsBeingDestroyed` (+0x81 != 0)
- `!IsAlive` (+0x74 == 0)

There is **no** explicit "IsBeingBuilt" skip in AI_AssessPower. The InLimbo check inside
GetPowerOutput/GetPowerDrain handles buildings that haven't been placed yet.

#### What about the deployment animation?

Once placed on the map, InLimbo is cleared (+0x270 = 0). The building enters mission
0x12 (Construction). During this phase:

- **GetPowerOutput**: InLimbo is false, so it proceeds. IsOnline is true (default).
  The function returns `ftol(PowerOutput * GetHealthRatio())`.
- **GetPowerDrain**: InLimbo is false, IsOnline is true. Returns full drain.

**Therefore: a building under construction (deploying) DOES contribute to power
calculations.** It produces power scaled by its current health (which starts low and
grows as construction completes) and drains the full rated amount.

**IsOperational** (vtable+0x350 @ 0x4555D0) checks:
```c
if (GetMission() == 0x12 || GetMission() == 0x13) return false;
```

So the building is NOT operational during construction — it can't fire, produce units,
or provide special effects (gap, cloak, etc.) — but it **does** affect the power
balance.

---

## 3. Mission IDs 0x12 and 0x13

### Correction to Prior Documentation

The prior POWER_SYSTEM_GHIDRA_REPORT.md incorrectly identified these as:
- 0x12 = "Selling"
- 0x13 = "MissilePrep"

**Verified from the mission string pointer table at 0x816CAC:**

| ID | Hex | Name |
|----|-----|------|
| 0x00 | 0 | Sleep |
| 0x01 | 1 | Attack |
| 0x02 | 2 | Move |
| 0x03 | 3 | QMove |
| 0x04 | 4 | Retreat |
| 0x05 | 5 | Guard |
| 0x06 | 6 | Sticky |
| 0x07 | 7 | Enter |
| 0x08 | 8 | Capture |
| 0x09 | 9 | Eaten |
| 0x0A | 10 | Harvest |
| 0x0B | 11 | Area Guard |
| 0x0C | 12 | Return |
| 0x0D | 13 | Stop |
| 0x0E | 14 | Ambush |
| 0x0F | 15 | Hunt |
| 0x10 | 16 | Unload |
| 0x11 | 17 | Sabotage |
| **0x12** | **18** | **Construction** |
| **0x13** | **19** | **Selling** |
| 0x14 | 20 | Repair |
| 0x15 | 21 | Rescue |
| 0x16 | 22 | Missile |
| 0x17 | 23 | Harmless |
| 0x18 | 24 | Open |
| 0x19 | 25 | Patrol |
| 0x1A | 26 | Paradrop Approach |
| 0x1B | 27 | Paradrop Overfly |
| 0x1C | 28 | Wait |
| 0x1D | 29 | Attack Move |
| 0x1E | 30 | Spyplane Approach |
| 0x1F | 31 | Spyplane Overfly |

**Confidence: HIGH** — decoded directly from the mission string pointer table at
0x816CAC (32 entries, each a 4-byte pointer to a null-terminated string). The strings
at 0x816D2C–0x816E72 spell out the full mission names.

---

## 4. Building Sell and Power

### When does a building stop contributing to power?

**Not when selling starts — only when health reaches 0 or the building is destroyed.**

The sell process:
1. Player issues sell command → building enters mission 0x13 (Selling)
2. Building plays sell animation, health decreases
3. Eventually health reaches 0 → building is destroyed

During the selling process:
- **GetPowerOutput** still runs normally: InLimbo is false, IsOnline is true.
  Output = `ftol(PowerOutput * GetHealthRatio())`. As health drops during selling,
  power output **gradually decreases** proportionally.
- **GetPowerDrain** also runs: returns full drain value until the building is gone.
- **IsOperational** returns false (mission == 0x13), so the building cannot fire,
  produce, etc. But it still affects power.

### When NeedsPowerRecalc triggers during sell

In `BuildingClass__Update` (0x43FB20), around address 0x440055:
```c
if (this->Health != this->CachedHealth) {
    House->NeedsPowerRecalc = 1;     // +0x5778
    House->PowerRecalcDone = 1;      // +0x5779
    this->CachedHealth = this->Health;
}
```

Every time the building's health changes (which happens each tick during selling),
NeedsPowerRecalc is triggered, causing the house to recalculate total power. The
power output drops gradually as the building loses HP.

### `BuildingClass::SellBuilding` (0x457DE0)

This function handles the actual sell placement (finding where to put infantry survivors,
etc.). It does NOT directly set NeedsPowerRecalc. Power changes are handled by the
health-change detection in BuildingClass::Update.

---

## 5. Building Capture (Engineer / Mind Control)

### The ownership transfer function: vtable+0x3D4 (0x448260)

When a building is captured (by engineer or mind control), the engine calls
`BuildingClass::ChangeOwner` (vtable+0x3D4) at 0x448260. This function:

1. **Saves old owner**: `iVar13 = param_1[0x87]` (old HouseClass pointer)
2. **Removes building from old owner's lists**: Iterates through ~12 different house
   building lists (radar, cloak generator, gap generator, sensor array, etc.) and
   removes this building from each
3. **Calls the general ownership transfer** `FUN_007014a0(new_house, 1)` which:
   - Sets `param_1[0x87] = new_house` (the actual house pointer change)
   - Adds to new owner's unit counts
   - Handles visibility/shroud updates
4. **Re-adds building to new owner's lists**: Adds to the new house's radar list,
   cloak generator list, etc.
5. **Sets power flags on new owner**:
   ```c
   *(byte *)(param_1[0x87] + 0x5778) = 1;  // NeedsPowerRecalc (new owner)
   *(byte *)(param_1[0x87] + 0x5779) = 1;  // PowerRecalcDone (new owner)
   ```
6. **Sets flag on old owner**:
   ```c
   *(byte *)(iVar13 + 0x1FC) = 1;  // NeedsUIUpdate on old owner
   ```

### Does NeedsPowerRecalc fire for BOTH owners?

**Yes, but through different mechanisms:**

- **New owner**: NeedsPowerRecalc is explicitly set at the end of `ChangeOwner`
  (0x448260) on the new house.
- **Old owner**: NeedsPowerRecalc is NOT explicitly set in `ChangeOwner`. However:
  - The building is removed from the old owner's building list
  - Next time `AI_AssessPower` runs for the old owner, the building will NOT be
    in its iteration list, so the old owner's power totals will naturally exclude it
  - The health-change detection in `BuildingClass::Update` may also trigger it if
    health changed during the same tick

**Note**: The old owner's NeedsPowerRecalc appears to be set indirectly through
`BuildingClass::Update`'s health-change check (the building is still alive when
captured, but the cached health comparison triggers a recalc).

### CloakGenerator special handling

If the captured building has `CloakGenerator=yes`:
```c
if (Type->CloakGenerator) {
    *(byte *)(new_house + 0x56F8) = 1;  // flag on new owner
}
```

The cloak generator is deactivated for the old owner and reactivated for the new
owner within the same function.

### Mind-control via CaptureManagerClass

`CaptureManagerClass::CaptureUnit` at 0x471D40 calls vtable 0x3D4 (ChangeOwner).
`CaptureManagerClass::ReleaseUnit` at 0x471FF0 calls vtable 0x3D4 again to restore
the original owner. Both trigger the same power recalculation chain.

---

## 6. ForceShield and Power

### No dedicated ForceShield power drain INI key

Searched the binary for "ForceShieldPower" and similar strings — **none found**.
The ForceShield superweapon does NOT have a configurable power drain penalty.

### ForceShield causes power BLACKOUT, not drain

The ForceShield activates through `FUN_0050BC90`:
```c
void HouseClass::StartSpyBlackout(int duration) {
    this->NeedsPowerRecalc = 1;                    // +0x5778
    this->SpyBlackoutStartFrame = g_CurrentFrame;  // +0x2A4
    this->SpyBlackoutDuration = duration;          // +0x2AC
}
```

This function is called from:
1. **Spy infiltration** (`BuildingClass::OnSpyInfiltrate` at 0x4571E0) — when a spy
   enters an enemy power plant
2. **Superweapon activation** (`FUN_006CC390` case handler) — ForceShield activation

### How the blackout works in AI_AssessPower (0x508C30)

After summing all building power, AI_AssessPower checks the blackout timer:
```c
int remaining = SpyBlackoutDuration;  // +0x2AC
if (SpyBlackoutStartFrame != -1) {    // +0x2A4
    int elapsed = g_CurrentFrame - SpyBlackoutStartFrame;
    if (elapsed < remaining)
        remaining = remaining - elapsed;
    else
        remaining = 0;
}

if (remaining > 0 && HasOccupiedPowerPlant) {
    // Blackout still active AND player has power plants
    // (skip zeroing if no power plants exist)
} else {
    PowerOutput = 0;  // ZERO all power output!
}
```

**The blackout completely zeros power output** for its duration. This is NOT a drain
— it sets `PowerOutput = 0` at +0x53A4, causing all Powered=yes buildings to fail
their IsOperational check.

### Duration

The blackout duration comes from:
- **Spy infiltration**: `RulesClass->ForceShieldBlackoutDuration` (parsed from
  `[General] ForceShieldBlackoutDuration=` in rulesmd.ini)
- **ForceShield superweapon**: same duration field

---

## 7. All NeedsPowerRecalc (+0x5778) Trigger Points

Compiled from byte-pattern search for `C6 80 78 57 00 00 01` and `C6 81 78 57 00 00 01`:

| Address | Function | Trigger |
|---------|----------|---------|
| 0x440055 | `BuildingClass::Update` | Health changed (power output scales with HP) |
| 0x43BF11 | Building destructor (0x43BCF0) | Building destroyed |
| 0x44FC57 | `BuildingClass::ReadFromINI` | Map loading (initial placement) |
| 0x454BCD | `BuildingClass::UpdateGapAndSpecialEffects` | Operational state changed |
| 0x452260 | `BuildingClass::GoOnline` | Player toggled power ON |
| 0x452360 | `BuildingClass::GoOffline` | Player toggled power OFF |
| 0x448260+ | `BuildingClass::ChangeOwner` (0x448260) | Building captured (new owner) |
| 0x701900 | `TechnoClass::ReceiveDamage` (2 refs) | Building took damage |
| 0x70FE50 | `BuildingClass::ExitTransport` | Unit exited building |
| 0x70FD70 | `BuildingClass::EnterTransport` | Unit entered building |
| 0x71ABC0 | Docking/undocking helper | Ship/unit docked/undocked |
| 0x71ACD0 | Docking/undocking helper | Ship/unit docked/undocked |
| 0x71AF20 | Docking/linking function | Link established with building |
| 0x50BC90 | `HouseClass::StartSpyBlackout` | Spy sabotage / ForceShield activation |

---

## Summary: Power Lifecycle of a Building

| Phase | InLimbo | IsOnline | Mission | Produces Power? | Drains Power? | IsOperational? |
|-------|---------|----------|---------|-----------------|---------------|----------------|
| In factory (not placed) | true | true | — | No (InLimbo) | No (InLimbo) | N/A |
| Under construction | false | true | 0x12 (Construction) | **Yes** (scaled by HP) | **Yes** (full) | **No** |
| Fully built | false | true | 0x05 (Guard) | Yes (scaled by HP) | Yes (full) | Yes |
| Selling | false | true | 0x13 (Selling) | **Yes** (decreasing with HP) | **Yes** (full) | **No** |
| Toggled off by player | false | **false** | — | **No** (IsOnline=0) | **No** (IsOnline=0) | No |
| Captured (new owner) | false | true | varies | Yes | Yes | Yes |
| Destroyed | — | — | — | No (removed) | No (removed) | N/A |
