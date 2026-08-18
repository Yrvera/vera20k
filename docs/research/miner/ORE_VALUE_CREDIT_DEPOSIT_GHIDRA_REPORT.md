# Ore Value Calculation & Credit Deposit System — Ghidra Research Report

**Date:** 2026-03-23
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH — all findings from direct decompilation and verified assembly

---

## 1. StorageClass Struct Layout

**StorageClass** is a simple struct of 4 floats (16 bytes), one per tiberium type:

```
Offset  Type    Field
0x00    float   Amount[0]   (Riparius / Ore)
0x04    float   Amount[1]   (Cruentus / Gems)
0x08    float   Amount[2]   (Vinifera)
0x0C    float   Amount[3]   (Aboreus)
```

**4 tiberium types** are supported, matching the `[Tiberiums]` section in rulesmd.ini.

Each float tracks the number of "bails" of that type currently stored. The epsilon
constant for "empty" comparisons is `0.0f` at address `0x007e1748`.

### StorageClass Functions

| Function | Address | Signature | Description |
|----------|---------|-----------|-------------|
| `GetAmount` | `0x006C9680` | `float GetAmount(int tibType)` | Returns `this->Amount[tibType]` |
| `AddAmount` | `0x006C9690` | `void AddAmount(float amount, int tibType)` | `this->Amount[tibType] += amount` |
| `Remove` | `0x006C96B0` | `float Remove(float amount, int tibType)` | Subtracts from slot; if slot < amount, zeroes it. Returns amount remaining in slot on FPU stack |
| `GetTotal` | `0x006C9650` | `int GetTotal()` | Sums all 4 float slots with ftol truncation; returns total as int |
| `GetTotalValue` | `0x006C9600` | `int GetTotalValue()` | Like GetTotal but only counts slots > 0 (confirmed from asm) |
| `FindFirstNonEmpty` | `0x006C9820` | `int FindFirstNonEmpty()` | Iterates slots 0-3, returns first index where amount > 0.0. Returns -1 if all empty |

### StorageClass::Remove — Corrected Assembly (0x006C96B0)

The decompiler shows confusing code; the assembly is clearer:

```asm
; ECX = this, [ESP+4] = amountToRemove (float), [ESP+8] = tibType (int)
006c96b0: MOV EDX, [ESP+8]          ; tibType
006c96b4: FLD [ECX + EDX*4]         ; load current amount
006c96b7: FCOMP [ESP+4]             ; compare current < amountToRemove?
006c96bb: FNSTSW AX
006c96bd: TEST AH, 0x1
006c96c0: JZ short_path             ; if current >= amountToRemove
; current < amountToRemove: zero the slot
006c96c2: FLD [ECX + EDX*4]
006c96c5: FLD [ECX + EDX*4]
006c96c8: FSUB ST0, ST1             ; result = current - current = 0.0
006c96ca: FSTP [ECX + EDX*4]        ; store 0.0
006c96cd: RET 8                     ; returns old value on FPU (ST0)
; current >= amountToRemove: subtract normally
006c96d0: FLD [ESP+4]               ; amountToRemove
006c96d4: FLD [ECX + EDX*4]         ; current
006c96d7: FSUB ST0, ST1             ; current - amountToRemove
006c96d9: FSTP [ECX + EDX*4]        ; store result
006c96dc: RET 8                     ; returns remaining on FPU (ST0)
```

Key: Remove returns the **remaining** amount in the slot on the FPU stack (ST0).

---

## 2. Tiberium Base Values

### TiberiumClass Value Field

**TiberiumClass+0xB8** stores the `Value` (int), parsed from `[TiberiumN] > Value=` in rulesmd.ini.

Parsed at `0x00721AFB` in TiberiumClass ReadINI:
```c
Value = CCINIClass::ReadInt(sectionName, "Value", this->Value);  // offset 0xB8
```

**Note:** TiberiumClass uses `int*` as param_1 type in the constructor, so field indices
multiply by 4. Offset 0xB8 = index 0x2E.

Default values from rulesmd.ini `[Tiberiums]`:
| Tiberium | INI Name | Value |
|----------|----------|-------|
| 0 (Riparius) | Ore | 25 |
| 1 (Cruentus) | Gems | 50 |
| 2 (Vinifera) | - | 25 |
| 3 (Aboreus) | - | 25 |

### Global Tiberium Array

The global `TiberiumClass*` array is at `0x00B0F4EC`. Access pattern:
```c
TiberiumClass* tib = ((TiberiumClass**)0x00B0F4EC)[tibType];
int value = tib->Value;  // offset 0xB8
```

---

## 3. HouseClass::DepositOreCredits (0x004F9610)

### Assembly (full function)

```asm
; __thiscall: ECX = HouseClass*, stack: float amount, int tibType
004f9610: FLD  [ESP+4]               ; load amount
004f9614: FMUL [0x007eaa00]          ; multiply by 5.0 (constant)
004f961a: PUSH ESI
004f961b: MOV  ESI, ECX              ; ESI = this (HouseClass*)
004f961d: FIADD [ESI+0x54E8]        ; add to HarvestedCredits (int)
004f9623: CALL Math__ftol            ; truncate to int
004f9628: MOV  [ESI+0x54E8], EAX    ; store updated HarvestedCredits
004f962e: MOV  EAX, [ESP+0xC]       ; tibType
004f9632: MOV  ECX, [0x00B0F4EC]    ; TiberiumClass array
004f9638: MOV  EDX, [ECX+EAX*4]     ; TiberiumClass* for this type
004f963b: MOV  EAX, [ESI+0x34]      ; HouseTypeClass* (this->Type)
004f963e: FILD [EDX+0xB8]           ; load TiberiumClass->Value as float
004f9644: FMUL [EAX+0x148]          ; multiply by HouseTypeClass->IncomeMult
004f964a: FMUL [ESP+8]              ; multiply by amount
004f964e: FIADD [ESI+0x30C]         ; add to Balance (int)
004f9654: CALL Math__ftol            ; truncate to int
004f9659: FLD  [ESP+8]              ; return amount on FPU
004f965d: MOV  [ESI+0x30C], EAX     ; store updated Balance
004f9663: POP  ESI
004f9664: RET  8
```

### Credit Formula

```
HarvestedCredits += (int)(amount * 5.0 + HarvestedCredits_old)

Balance += (int)(TiberiumClass[tibType]->Value * HouseTypeClass->IncomeMult * amount + Balance_old)
```

**Key constants and fields:**
| Symbol | Address/Offset | Value | Description |
|--------|---------------|-------|-------------|
| Constant 5.0 | `0x007EAA00` | `5.0f` | Harvested credits multiplier (statistics tracking) |
| `TiberiumClass->Value` | TibClass+0xB8 | int (25/50) | Per-bail credit value |
| `HouseTypeClass->IncomeMult` | HouseType+0x148 | float (default 1.0) | Difficulty/country income multiplier |
| `HouseClass->Balance` | House+0x30C | int | Current credits (what you spend from) |
| `HouseClass->HarvestedCredits` | House+0x54E8 | int | Cumulative total (for score/statistics) |
| `HouseClass->Type` | House+0x34 | HouseTypeClass* | Pointer to country type |

---

## 4. HouseClass::DepositWeedCredits (0x004F9700)

### Assembly

```asm
; __thiscall: ECX = HouseClass*, stack: int count, int tibType
004f9700: PUSH EBX
004f9701: PUSH ESI
004f9702: MOV  ESI, [ESP+0xC]       ; count
004f9706: PUSH EDI
004f9707: TEST ESI, ESI
004f9709: JLE  done
004f970b: MOV  EBX, [ESP+0x14]      ; tibType
004f970f: LEA  EDI, [ECX+0x314]     ; StorageClass* at HouseClass+0x314
loop:
004f9715: MOV  EAX, [0x008871E0]    ; g_RulesClass_Instance
004f971a: MOV  ECX, EDI
004f971c: FILD [EAX+0x17D0]        ; RulesClass->TiberiumStorageLimit (int->float)
004f9722: FSTP [ESP+0x10]
004f9726: CALL StorageClass__GetTotal
004f972b: FCOMP [ESP+0x10]          ; compare total < limit?
004f972f: FNSTSW AX
004f9731: TEST AH, 0x1
004f9734: JZ   done                  ; if total >= limit, stop
004f9736: PUSH EBX                   ; tibType
004f9737: PUSH 0x3F800000            ; 1.0f
004f973c: MOV  ECX, EDI
004f973e: CALL StorageClass__AddAmount
004f9743: DEC  ESI
004f9744: TEST ESI, ESI
004f9746: FSTP ST0                   ; discard return
004f9748: JG   loop
done:
004f974a: POP  EDI
004f974b: POP  ESI
004f974c: POP  EBX
004f974d: RET  8
```

### Logic

Deposits `count` bails of weed (tiberium) into `HouseClass+0x314` (a second StorageClass),
one bail at a time. Stops if total storage reaches `RulesClass+0x17D0` (TiberiumStorageLimit/silo capacity).

| Field | Offset | Description |
|-------|--------|-------------|
| `HouseClass->WeedStorage` | House+0x314 | StorageClass (16 bytes) for weed/tiberium deposit |
| `RulesClass->TiberiumStorageLimit` | Rules+0x17D0 | int: maximum bails across all types |

---

## 5. BuildingClass::DepositOreFromStorage (0x00522D50)

This is the **main ore dump function** called when a harvester/slave deposits ore at a refinery.

### Pseudocode (verified from assembly)

```c
void BuildingClass::DepositOreFromStorage(BuildingClass* building) {
    // this = building, storage at this+0x33C
    StorageClass* storage = &this->OreStorage;  // offset 0x33C
    bool deposited = false;
    int tibType = storage->FindFirstNonEmpty();  // 0x006C9820

    while (tibType != -1) {
        HouseClass* owner = this->Owner;         // building+0x21C -> HouseClass*
        int storageCapacity = owner->StorageCapacity;  // HouseClass+0x538C

        // AI difficulty bonus: only for AI players in multiplayer
        if (!owner->IsHuman && g_GameMode != 0) {  // HouseClass+0x1EC, 0x00A8B238
            int* bonusTable = RulesClass->AIVirtualPurifiers.Data;  // Rules+0x1324
            int difficulty = owner->AIDifficulty;  // HouseClass+0x184
            storageCapacity += bonusTable[difficulty];
        }

        float currentAmount = storage->GetAmount(tibType);

        // PurifierBonus credit calculation
        float creditValue = (float)storageCapacity
                          * RulesClass->PurifierBonus     // Rules+0xF3C (float)
                          * currentAmount;

        // Remove all stored ore of this type
        float remaining = storage->Remove(currentAmount, tibType);

        if (remaining > 0.0) {
            deposited = true;
            // Deposit the base ore value (amount * TibValue * IncomeMult)
            owner->DepositOreCredits(remaining, tibType);

            // Deposit the purifier bonus credits (if any)
            if (creditValue > 0.0) {
                owner->DepositOreCredits(creditValue, tibType);
            }
        }

        tibType = storage->FindFirstNonEmpty();
    }

    if (deposited) {
        this->vtable[0x468/4]();  // UpdateSiloDisplay or similar
    }
}
```

### Key Insight: Two DepositOreCredits Calls

Each dump cycle calls DepositOreCredits **twice**:

1. **Base deposit**: `DepositOreCredits(remainingAmount, tibType)` — credits = amount * TibValue * IncomeMult
2. **Purifier bonus**: `DepositOreCredits(creditValue, tibType)` — credits = storageCapacity * PurifierBonus * amount * TibValue * IncomeMult

The purifier bonus is proportional to the house's total storage capacity, the PurifierBonus
percentage, and the amount being deposited.

---

## 6. AI Difficulty Bonus: AIVirtualPurifiers

### RulesClass+0x1320: AIVirtualPurifiers DifficultyControl Vector

This is a `TypeList<int>` (DifficultyControl) struct at RulesClass offset 0x1320 (0x1C bytes).
The data pointer at **Rules+0x1324** points to an `int[]` array indexed by difficulty level.

Parsed in `FUN_00672AE0` (RulesClass AI configuration reader) at address `0x0067054C`:
```asm
0067054c: LEA  EBX, [ESI+0x1320]    ; AIVirtualPurifiers vector
00670552: ...
0067055f: PUSH 0x83C154             ; "AIVirtualPurifiers" string
```

**INI definition** (from rulesmd.ini):
```ini
AIVirtualPurifiers=4,2,0  ; hard,medium,easy — number of virtual purifiers
```

### How It Works

The `AIVirtualPurifiers` array is indexed by `HouseClass+0x184` (AIDifficulty):
- Index 0 = Hard (AI gets 4 virtual purifiers)
- Index 1 = Medium (AI gets 2 virtual purifiers)
- Index 2 = Easy (AI gets 0 virtual purifiers)

Each "virtual purifier" adds to the effective storage capacity used in the PurifierBonus
calculation. Since PurifierBonus defaults to 0.25, 4 virtual purifiers effectively give
the AI a `4 * 0.25 = 100%` bonus on harvested ore at hard difficulty.

### Guard Condition

The bonus only applies when:
1. `HouseClass+0x1EC == 0` (IsHuman is false — this is an AI player)
2. `g_GameMode != 0` (at address `0x00A8B238` — not campaign solo mode)

---

## 7. HouseTypeClass Difficulty Multipliers

### IncomeMult (HouseTypeClass+0x148)

Parsed at `0x00511D05` in `HouseTypeClass::ReadINI` (0x00511850):
```c
IncomeMult = CCINIClass::ReadDouble(section, "IncomeMult", (double)this->IncomeMult);
*(float*)(this + 0x148) = (float)result;
```

**Default: 1.0f** (set in constructor at `param_1[0x52] = 0x3F800000`)

This is a per-country multiplier applied to ALL ore deposits. In standard RA2/YR it's 1.0
for all countries, but the difficulty system can override it for AI players.

### Full HouseTypeClass Difficulty Multiplier Table

All initialized to 1.0f in the constructor (offsets 0x100-0x14C):

| Offset | INI Key | Type |
|--------|---------|------|
| 0x100 | ArmorInfantryMult | float |
| 0x104 | ArmorUnitsMult | float |
| 0x108 | ArmorAircraftMult | float |
| 0x10C | ArmorBuildingsMult | float |
| 0x110 | ArmorDefensesMult | float |
| 0x114 | CostInfantryMult | float |
| 0x118 | CostUnitsMult | float |
| 0x11C | CostAircraftMult | float |
| 0x120 | CostBuildingsMult | float |
| 0x124 | CostDefensesMult | float |
| 0x128 | SpeedInfantryMult | float |
| 0x12C | SpeedUnitsMult | float |
| 0x130 | SpeedAircraftMult | float |
| 0x134 | BuildTimeInfantryMult | float |
| 0x138 | BuildTimeUnitsMult | float |
| 0x13C | BuildTimeAircraftMult | float |
| 0x140 | BuildTimeBuildingsMult | float |
| 0x144 | BuildTimeDefensesMult | float |
| **0x148** | **IncomeMult** | **float** |

---

## 8. PurifierBonus (RulesClass+0xF3C)

Parsed in `RulesClass::ReadGeneral` at `0x0066FC5C`:
```asm
0066fc5c: FLD  [ESI+0xF3C]          ; current PurifierBonus
0066fc62: SUB  ESP, 8
0066fc65: MOV  ECX, EDI
0066fc67: FSTP [ESP]                 ; push as double arg
0066fc6a: PUSH 0x83C62C             ; "PurifierBonus" string
0066fc6f: PUSH EDX                   ; INI ptr
0066fc70: CALL CCINIClass__ReadDouble
0066fc75: FSTP [ESI+0xF3C]          ; store result as float
```

| Field | Offset | Type | INI Key | Default |
|-------|--------|------|---------|---------|
| PurifierBonus | Rules+0xF3C | float | `PurifierBonus` | 0.25 |

From rulesmd.ini: `PurifierBonus=.25` — 25% bonus per purifier.

---

## 9. HouseClass Key Money Fields

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| 0x34 | Type | HouseTypeClass* | Pointer to country/house type definition |
| 0x184 | AIDifficulty | int | Difficulty index (0=hard, 1=medium, 2=easy) |
| 0x1EC | IsHuman | bool (byte) | True if human player |
| 0x30C | Balance | int | Current spendable credits |
| 0x314 | WeedStorage | StorageClass (16B) | Weed/tiberium deposit storage |
| 0x33C | OreStorage | StorageClass (16B) | Main ore storage (used by refinery dump) |
| 0x538C | StorageCapacity | int | Total silo capacity in bails |
| 0x54E8 | HarvestedCredits | int | Cumulative credits harvested (statistics) |
| 0x2DC | SpentCredits | int | Total credits spent (set in SpendMoney) |

---

## 10. Credits Display Counter (CreditsClass)

### CreditsClass Struct Layout

Located at `0x0089F950` (global instance). Struct size: 16 bytes (0x10).

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00 | int | ActualCredits | Real credit value to display |
| 0x04 | int | DisplayedCredits | Currently displayed (animated) value |
| 0x08 | bool | NeedsRedraw | Flag to trigger sidebar redraw |
| 0x09 | bool | IsIncreasing | True if credits going up |
| 0x0A | bool | SoundPlaying | Flag for tick-up/down sound |
| 0x0C | int | AnimationDelay | Cooldown counter between display steps |

### CreditsClass::Init (0x004A2350)

```c
void CreditsClass::Init() {
    this->ActualCredits = 0;
    this->DisplayedCredits = 0;
    this->NeedsRedraw = false;
    this->IsIncreasing = false;
    this->SoundPlaying = false;
    this->AnimationDelay = 0;
}
```

### CreditsClass::AI (0x004A2600) — Animation Logic

Called every frame. Two code paths:

**Observer mode** (PlayerPtr == observer): Displays game timer as HH:MM:SS.

**Normal mode**: Smoothly animates `DisplayedCredits` toward `ActualCredits`:

```c
void CreditsClass::AI(bool forceUpdate) {
    // Calculate actual credits from HouseClass storage
    ActualCredits = house->vtable[0x18/4](house+0x24);  // GetAvailableCredits
    if (ActualCredits < 0) ActualCredits = 0;

    if (DisplayedCredits == ActualCredits && !forceUpdate)
        return;

    if (forceUpdate) {
        SoundPlaying = false;
        DisplayedCredits = ActualCredits;
    } else {
        // Smooth interpolation
        if (AnimationDelay > 0) AnimationDelay--;

        int diff = ActualCredits - DisplayedCredits;
        // Direction: 1 if increasing, 3 if decreasing
        AnimationDelay = (diff > 0) ? 1 : 3;

        // Step size: |diff| / 8, clamped to [1, 143]
        int step = abs(diff) >> 3;
        if (step < 1) step = 1;
        else if (step > 143) step = 143;  // 0x8F

        if (ActualCredits < DisplayedCredits) step = -step;

        int oldDisplayed = DisplayedCredits;
        DisplayedCredits += step;

        if (oldDisplayed != DisplayedCredits) {
            SoundPlaying = true;
            IsIncreasing = (step > 0);
        }
    }

    NeedsRedraw = true;
    FUN_004F42F0(0);  // trigger sidebar redraw
    g_SidebarNeedsRedraw = 1;
}
```

### Key Animation Parameters

- **Step size**: `abs(actual - displayed) / 8`, clamped to range [1, 143]
- **Direction delay**: 1 frame for increasing, 3 frames for decreasing
- Credits tick **faster** when the difference is larger (proportional stepping)
- Maximum step per frame is **143 credits** to prevent jarring jumps
- A "credit tick" sound plays when `SoundPlaying` flag is set and `RulesClass+0x6DC > 1`

### CreditsClass::Draw (0x004A2370)

Draws the credit counter on the sidebar surface. In observer mode, shows a timer.
In normal mode, formats DisplayedCredits as `"$%d"` and draws with the credit
tick-up/down sound effect.

---

## 11. HouseClass::SpendMoney (0x004F9790)

Reverse of deposit — deducts credits:

```c
void HouseClass::SpendMoney(int amount) {
    int oldTotal = StorageClass::GetTotal(&this->OreStorage);  // save for silo check
    int balance = this->Balance;  // +0x30C

    if (balance >= amount) {
        // Enough cash in balance
        this->Balance -= amount;
    } else {
        // Not enough cash, need to sell ore from silos
        int deficit = amount - balance;
        this->Balance = 0;

        if (deficit > 0 && StorageClass::GetTotal() > 0) {
            // Iterate buildings to find silos with ore
            for each building in house->Buildings {
                if (building != NULL && building->Storage.GetTotal() > 0) {
                    while (deficit > 0) {
                        int tibType = StorageClass::FindFirstNonEmpty();
                        while (StorageClass::GetAmount(tibType) > 0 && deficit > 0) {
                            StorageClass::Remove(1.0, tibType);
                            int value = ftol(removed_amount);  // value of 1 bail
                            deficit -= value;
                            amount += value;
                            if (deficit < 0) {
                                amount += deficit;  // refund overpayment
                                this->Balance -= deficit;
                                deficit = 0;
                            }
                        }
                    }
                }
            }
        }
    }

    HouseClass::UpdateSiloDisplays(oldTotal);  // 0x004F9970
    this->SpentCredits += amount;  // HouseClass+0x2DC
}
```

---

## 12. Summary: Complete Ore Value Pipeline

```
1. Harvester gathers ore from map cell
   Cell stores: overlay type (ore/gems) + density (0-11 bails)
   CellClass::Get_Tiberium_Value = TiberiumClass[type]->Value * (density + 1)

2. Harvester returns to refinery, deposits into BuildingClass+0x33C (StorageClass)
   Each bail adds 1.0 to the appropriate tiberium type slot

3. BuildingClass::DepositOreFromStorage (0x00522D50) runs:
   For each non-empty tiberium type in storage:

   a. storageCapacity = HouseClass+0x538C
      If AI: storageCapacity += AIVirtualPurifiers[difficulty]

   b. currentAmount = StorageClass::GetAmount(tibType)
      purifierCredit = storageCapacity * PurifierBonus * currentAmount

   c. StorageClass::Remove(currentAmount, tibType)
      remaining = amount left after removal (returned on FPU)

   d. If remaining > 0:
      CALL 1: DepositOreCredits(remaining, tibType)
        Balance += remaining * TibValue * IncomeMult
        HarvestedCredits += remaining * 5.0

      CALL 2: DepositOreCredits(purifierCredit, tibType)  [if purifierCredit > 0]
        Balance += purifierCredit * TibValue * IncomeMult
        HarvestedCredits += purifierCredit * 5.0

4. CreditsClass::AI animates displayed credits toward actual
   Step = |diff| / 8, clamped [1, 143] per frame
```

### Example: 1 bail of ore deposited, no purifiers, normal difficulty

- TibValue = 25 (ore), IncomeMult = 1.0, storageCapacity = 0, PurifierBonus = 0.25
- purifierCredit = 0 * 0.25 * 1.0 = 0 (no bonus)
- Balance += 1.0 * 25 * 1.0 = **25 credits**
- HarvestedCredits += 1.0 * 5.0 = 5

### Example: 1 bail of ore, AI on Hard with 2000 silo capacity

- storageCapacity = 2000 + 4 (AIVirtualPurifiers[0]) = 2004
- purifierCredit = 2004 * 0.25 * 1.0 = 501.0
- Base: Balance += 1.0 * 25 * 1.0 = 25
- Bonus: Balance += 501.0 * 25 * 1.0 = **12,525 credits**
- Total per bail: **12,550 credits** (massive AI advantage)

---

## 13. Related RulesClass Offsets

| Offset | INI Key | Type | Description |
|--------|---------|------|-------------|
| 0xF3C | PurifierBonus | float | Multiplier per purifier (default 0.25) |
| 0x1320 | AIVirtualPurifiers | TypeList<int> (0x1C bytes) | AI bonus purifiers by difficulty |
| 0x1324 | (data ptr of above) | int* | Pointer to int[3] array |
| 0x17D0 | (TiberiumStorageLimit) | int | Max bails in weed storage |
| 0x17E3 | CompEasyBonus | bool | Whether AI gets easy bonus (parsed, not used in ore calc) |

---

## 14. Ghidra Labels Applied

| Address | New Name |
|---------|----------|
| 0x006C9650 | StorageClass__GetTotal |
| 0x006C9690 | StorageClass__AddAmount |
| 0x006C9600 | StorageClass__GetTotalValue |
| 0x00522D50 | BuildingClass__DepositOreFromStorage |
| 0x004F9970 | HouseClass__UpdateSiloDisplays |
| 0x00511850 | HouseTypeClass__ReadINI |

Previously labeled (confirmed):
| Address | Name |
|---------|------|
| 0x006C9680 | StorageClass__GetAmount |
| 0x006C96B0 | StorageClass__Remove |
| 0x006C9820 | StorageClass__FindFirstNonEmpty |
| 0x004F9610 | HouseClass__DepositOreCredits |
| 0x004F9700 | HouseClass__DepositWeedCredits |
| 0x004F9790 | HouseClass__SpendMoney |
| 0x004A2600 | CreditsClass__AI |
| 0x004A2370 | CreditsClass__Draw |
| 0x004A2350 | CreditsClass__Init |
