# BuildingClass Special Building Flags — Ghidra RE Report

Investigation of the eight gameplay-unique `BuildingTypeClass` boolean flags that drive
"special behavior" buildings in Yuri's Revenge. Verified directly against
`gamemd.exe` (image base 0x00400000, x86 LE 32-bit MSVC).

## Source: BuildingTypeClass flag layout

All eight flags are single-byte booleans on `BuildingTypeClass`, confirmed via
`BuildingTypeClass::ReadINI` at **0x0045FE50** (param_1 is `int`, offsets are
direct byte offsets — no ×4 scaling).

| Offset | Flag (INI key)          | Location in ReadINI |
|--------|-------------------------|---------------------|
| +0x16AC | `Cloning=`             | 0x0046095B          |
| +0x16AD | `Grinding=`            | 0x00460971          |
| +0x16AE | `UnitAbsorb=`          | near 0x0046099E     |
| +0x16AF | `InfantryAbsorb=`      | 0x004609A9          |
| +0x16B0 | `SecretLab=`           | 0x004609BF          |
| +0x16C1 | `Hospital=`            | 0x00460AE1          |
| +0x16C2 | `Armory=`              | 0x00460AF7          |
| +0x16CC | `OrePurifier=`         | 0x004604ED          |
| +0x16CD | `FactoryPlant=`        | 0x00460507          |
| +0x16D0..+0x16E0 | `InfantryCostBonus/UnitsCostBonus/AircraftCostBonus/BuildingsCostBonus/DefensesCostBonus` (5 × float) | 0x00460510..0x0046056E |

All flags default to 0 (false). FactoryPlant cost-bonus floats default to 1.0.

Active-in-YR status: **all eight are live in standard YR skirmish.** No
`SpecialFlags` gating and none are TS-only holdovers.

## HouseClass per-flag registry

When a building with one of these flags is built (`BuildingClass::Unlimbo` at
**0x00440580**) or captured (`BuildingClass::ChangeOwner` at **0x00448260**)
the building pointer is inserted into one of several `DynamicVector<BuildingClass*>`
on its owning `HouseClass`. `BuildingClass::OnSold` at **0x00448E30** removes the
building. Each vector is 0x18 bytes (`items_ptr, count, capacity, alloc_flag,
grow_step`) but in these call sites only the first three fields matter.

| Flag          | HouseClass items ptr | count  |
|---------------|----------------------|--------|
| Grinding      | +0x9C                | +0xA8  |
| Unit/Inf Absorb (+0x16AE/+0x16AF) | +0xB4 | +0xC0 |
| Bunker (+0x16AB, not special) | +0xCC    | +0xD8 |
| Cloning       | +0xFC                | +0x108 |
| SecretLab     | +0x114               | +0x120 |
| FactoryPlant  | +0x144               | +0x150 |
| OrePurifier counter (scalar int) | n/a    | +0x538C |

Adding a FactoryPlant additionally calls **HouseClass::RecalcBonuses**
(0x0050BF60). OrePurifier uses a plain int counter, decremented by
`BuildingClass::Limbo` at **0x00445880** (corrected 2026-05-29: was labelled
`BuildingClass::OnDestroyed`; binary shows function name is `BuildingClass__Limbo` via
get_function_by_address 0x00445880 — ROOT: RTTI_LABEL_DRIFT). Limbo is the correct
trigger: it fires on sell, destruction, and ownership-transfer removal from the map.

---

## 1. Cloning (+0x16AC) — Yuri Cloning Vats

**Mechanic:** When any Barracks belonging to this House finishes training an
infantry unit, each Cloning Vat on that House also produces an identical copy
of the infantry (for free).

**Code path:** `BuildingClass::ExitObject_Main` at **0x00443C60** handles
producing the trained unit from a factory. The mirror loop is at
**0x004449FB** (approximately line 705 of its decompilation):

```c
if (Type[0xEB8] == 0x10 /* BuildingType kind == Barracks */
    && Type[0x16AC] == 0 /* this building is NOT itself a Cloning Vat */) {
    TechnoTypeClass* exitedUnit = param_2->GetType();   // vtable+0x84
    for (int i = 0; i < House[0x108] /*Cloning list count*/; ++i) {
        BuildingClass* vat = House->ClonesArray[0xFC][i];
        // vtable+0x100 on the vat spawns a duplicate copy at its exit;
        // the argument is a location obtained from the just-exited unit.
        vat->vtable[0x100]( vat->GetFoundationCell(...) );
    }
}
```

- Guards: the originating factory must be a Barracks (`BuildingTypeClass+0xEB8
  == 0x10`, the "Barracks" enum) and must not itself be a Cloning Vat (avoid
  double-mirroring).
- Each vat in the `HouseClass+0xFC` list is walked in order.
- `vtable+0x100` on the vat is the per-Cloning-Vat exit path; it produces a
  fresh copy of the same `InfantryTypeClass` the barracks just produced. No
  cost is deducted, no queue time consumed.
- Only infantry are cloned — Barracks is the only factory type that triggers
  the mirror (`eb8 == 0x10`).

**Relevant INI keys:** none beyond the flag itself — the Barracks vs Cloning
Vat distinction is handled through `[BuildingType] Cloning=` and the Barracks
enum.

**Active in YR:** Yes. Confidence: **high** (verified in
`BuildingClass::ExitObject_Main`).

---

## 2. Grinding (+0x16AD) — Yuri Grinder

**Mechanic:** When a unit enters a Grinder (Mission=Enter), the unit is
destroyed and the owning House is credited the unit's refund value. Any
passengers and mind-controlled units are also refunded and destroyed.

**Code paths:**
- `UnitClass::Mission_Enter` at **0x00739EC0** (vehicle grinding)
- `InfantryClass::Mission_Enter` at **0x005196A0** (infantry grinding)

The core sequence when the enterer reaches the building (around
`UnitClass::Mission_Enter` lines 82–108 of its decompilation):

```c
credits = unit->vtable[0x2BC]();                 // GetRefundValue()
HouseClass::Add_Credits( credits );

// Refund every passenger recursively
for (passenger in unit->Cargo /* field_0x118 */) {
    credits = passenger->vtable[0x2BC]();
    HouseClass::Add_Credits(credits);
    passenger->vtable[0xF8]();                  // UnInit / destroy
}

// Refund mind-controlled slave if any
if (unit->MindControlledSlave) {
    HouseClass::Add_Credits( slave->GetRefundValue() );
    WarpAttachClass::Detach();
}

// Play ambient grinder sounds (Type+0x4CC = ExtraSounds1, Type+0x520 = ExtraSounds2)
```

**Refund value formula:** `vtable+0x2BC` on `TechnoClass` is `GetRefundValue`.
It returns `TechnoTypeClass.Soylent` (INI `Soylent=`, stored at
`TechnoTypeClass+0x614`, read in `TechnoTypeClass::ReadINI` at **0x007146CD**)
if that field is nonzero; otherwise it falls back to the default
half-cost refund used by the sell logic. `param_1` in `TechnoTypeClass::ReadINI`
is `int *`, so `param_1[0x185]` = byte offset `0x185 × 4 = 0x614`.

**Player-side enable (cursor):** `InfantryClass::What_Action_OnObject` at
**0x0051E3B0** gates the Enter cursor on `Type[0x16AD] != 0` — returns action
code `0x20` (Enter) when set, `0x1D` (Attack) otherwise. Same pattern in
`UnitClass::What_Action_OnObject` at **0x0073FD50**.

**Relevant INI keys:** `[TechnoType] Soylent=<int>` per unit/infantry.

**Active in YR:** Yes. Confidence: **high**.

---

## 3. Hospital (+0x16C1) — Allied Hospital

**Mechanic:** A garrisoned infantry unit is healed to full HP over time.
After healing it automatically exits the Hospital.

**Code path:** `BuildingClass::MissionRepairAndProduce` at **0x0044B780**,
Hospital branch at lines 68–142 of the decompilation.

Fields used on `BuildingClass`:
- `+0x620` accumulator (running heal-timer credit)
- `+0x624` "tick active this frame" flag
- `+0x628` frame-counter of last tick
- `+0x62C/+0x630/+0x634` bookkeeping for tick duration/step
- `+0x638` per-tick increment
- `+0x6DD` produce/state bit

Per-tick logic (runs while the Hospital has an occupant):
```c
if (field_0xBC == 0) {
    // First entry — initialise timer slot
    field_0xBC   = 2;
    field_0x6DD  = 0;
    field_0x620  = 0;
    field_0x628  = CurrentFrame;
    field_0x630  = 1;
    field_0x634  = 1;
    field_0x2FC -= 1; if (field_0x2FC < 0) field_0x2FC = 0;  // "next tick" timer
}
else if (field_0xBC == 2) {
    if (CDTimer.Remaining() == 0 && field_0x634 != 0) {
        field_0x624 = 1;
        field_0x620 += field_0x638;           // accumulate heal credit
        field_0x628  = CurrentFrame;
        field_0x630  = field_0x634;
    } else {
        field_0x624 = 0;
    }

    // Trigger heal+eject when accumulator >= IRepairRate * 900.0
    if ( Rules.IRepairRate (+0x16F0, double) * 900.0 <= field_0x620 ) {
        field_0x6DD = 0;
        field_0x620 = 0;

        // vtable+0x274 returns an "exit decision" code; 0x20 = normal exit,
        // 0x21 = ejected with radar ping + EVA voice (AI captured player)
        int exitCode = this->vtable[0x274]();
        if (exitCode == 0x20) {
            // corrected 2026-05-29: only vtable+0x100 is called for exitCode 0x20;
            // vtable+0x1E8(5,0) is NOT called on this path (decompile_function 0x0044B780
            // — for 0x20 branch: only `(**(code**)(puVar2+0x100))(uVar7); return 1;`)
            // ROOT: OPERATOR_OR_ORDER_DRIFT
            this->vtable[0x100]( FUN_00473430() );    // eject occupant
            return 1;
        }
        if (exitCode == 0x21) {
            if (HouseClass::IsHumanPlayer()) {
                CreateRadarEvent(); VoxClass::PlayEVA(...);
            }
        }
        // vtable+0x100 + vtable+0x1E8(5,0) both called for exitCode 0x21 and other non-0x20
        this->vtable[0x100]( FUN_00473430() );    // eject occupant
        this->vtable[0x1E8](5, 0);                 // mission SELECT
    }
}
```

**Heal rate formula (authoritative):**
```
trigger when   field_0x620 >= Rules.IRepairRate * 900.0
```

- `Rules.IRepairRate` is the general INI key `[General] IRepairRate=` — read in
  `RulesClass::ReadGeneral` at 0x00670xxx (look for string `IRepairRate` at
  0x0083BDB8). Stored as `double` at `RulesClass+0x16F0`.
- The constant **900.0** is loaded from `_DAT_007E27F8`
  (bytes: `00 00 00 00 00 20 8C 40` = 0x408C200000000000 double).
- At 15 fps this is 60 seconds of "timer equivalent"; combined with
  `IRepairRate` (fractional seconds) it produces the per-heal interval.
- `field_0x638` (increment per tick) is set elsewhere from the infantry's
  `BuildingTypeClass+0x684` structure (a per-building "refill slot" copy).

**Which infantry qualify:** any infantry that is allowed to garrison via
radio/enter. The Hospital's enter action is enabled by the `0x16C1` flag in
`InfantryClass::What_Action_OnObject` (flag makes action code `0x20` Enter
at line 78; action 0x1D at line 363). `BuildingClass::Receive_Radio` at
**0x0043C2D0** (lines 255–316 of its decompilation) accepts radio handshakes
when `puVar8[0x16C2] || puVar8[0x16C1]` and the entering unit's RTTI type
matches (Unit type 0x0F for Yuri-style, Infantry for allied). It also auto-
accepts regardless of player-control state in `ExitObject_Main` at 0x00444D28.

**Relevant INI keys:** `[General] IRepairRate=<seconds-per-HP-fraction>` in
rules(md).ini; per-building `Hospital=yes`.

**Active in YR:** Yes. Confidence: **high** for the timer formula and
trigger, **medium** for the precise semantic of `field_0x638` (confirmed to
be the accumulator increment but its initial value source was not fully
traced; likely `BuildingTypeClass+0x684` per decompile context).

---

## 4. Armory (+0x16C2) — Yuri Armory (veterancy promotion)

**Mechanic:** A garrisoned infantry unit is promoted one veterancy level per
cycle (Rookie → Veteran, or Veteran → Elite), then ejected.

**Code path:** `BuildingClass::MissionRepairAndProduce` at **0x0044B780**,
Armory branch at lines 144–195 (same structure as Hospital, but the terminal
action is a veterancy bump instead of a heal):

```c
if (field_0xBC == 0) {
    /* initialisation mostly matches Hospital, but the Armory branch does NOT have the
       `if (Type+0x684 != -1)` guard before decrementing field_0x2fc — it always
       decrements unconditionally (corrected 2026-05-29: was "identical initialisation to
       Hospital"; decompile_function 0x0044B780 shows the guard is absent in the Armory
       bc==0 path — ROOT: OPERATOR_OR_ORDER_DRIFT) */
}
else if (field_0xBC == 2) {
    /* accumulator tick logic matches Hospital,
       using field_0x620 / field_0x638 */

    if ( Rules.IRepairRate (+0x16F0) * 900.0 <= field_0x620 ) {
        int* dest = FootClass::GetDestination();
        if ( VeterancyStruct::IsRookie() ) {
            VeterancyStruct::SetVeteran();
        } else {
            VeterancyStruct::SetElite();
        }
        this->vtable[0x100]( FUN_00473430() );   // eject occupant
        this->vtable[0x1E8](5, 0);
    }
}
```

**Promote formula:** the same `Rules.IRepairRate * 900.0` timer reused. So in
stock YR, Hospital heal-time and Armory promote-time are both gated on the
SAME general-purpose Rule tuner (`IRepairRate`). That is the verified truth
in the binary — there is **no dedicated ArmoryRate or PromoteTime INI key**
in YR, despite community-wiki speculation.

Result:
- Rookie → Veteran (SetVeteran at 0x004???? — unnamed helper)
- Veteran/Elite → Elite (SetElite)
- Elite already → SetElite (idempotent)

After promotion the occupant is ejected via `vtable[0x100]` (building-exit).

**Receive_Radio acceptance:** same gating block as Hospital in
`BuildingClass::Receive_Radio` (lines 255–316 of `0x0043C2D0`) — both flags
share the acceptance branch.

**Relevant INI keys:** `[General] IRepairRate=` (shared with Hospital);
per-building `Armory=yes`.

**Active in YR:** Yes. Confidence: **high**.

---

## 5. InfantryAbsorb (+0x16AF) — Bio Reactor (and UnitAbsorb +0x16AE)

**Mechanic:** Occupants boost the building's power output by a fixed
per-occupant amount. Used by Yuri Bio Reactor with infantry.

**Code path:** `BuildingClass::GetPowerOutput` at **0x0044E7B0**.

```c
int BuildingClass::GetPowerOutput()
{
    int power = Type.Power;                      // +0xEE0
    if (vtable[0x1D4]() /* IsOnBridge or similar negative gate */ )
        return 0;

    if (HasExtraPowerBonus)
        power += Type.ExtraPower;                // +0xEE8

    // *** InfantryAbsorb / UnitAbsorb boost ***
    if ((Type[0x16AE] /*UnitAbsorb*/ || Type[0x16AF] /*InfantryAbsorb*/)
        && Type.ExtraPower > 0
        && this->OccupantCount /* field_0x114 */ > 0) {
        power += Type.ExtraPower * this->OccupantCount;
    }

    // Upgrades (+0xEE0 on each upgrade type)
    for (upgrade in this->Upgrades_0..2)
        power += upgrade.Type.Power;

    if (power > 0 && HasPower)
        return int( power * GetHealthRatio() );  // Math.ftol
    return 0;
}
```

**Power formula:**
```
power = Type.Power
      + (HasExtraPowerBonus ? Type.ExtraPower : 0)
      + (Absorb && Type.ExtraPower > 0 && occupants > 0
             ? Type.ExtraPower * occupants : 0)
      + sum(upgrade.Type.Power for each of 3 upgrade slots)
then scaled by GetHealthRatio() if HasPower, else 0
```

- `Type+0xEE0` = `Power=`
- `Type+0xEE8` = `ExtraPower=`
- `BuildingClass+0x114` = garrison/absorb occupant count (shared with
  Hospital/Armory queue; NOT the garrison gunner array used by Pillbox — that
  is `+0x1A0`/`+0x691` etc.)
- `Type+0x5E0` = `MaxNumberOccupants` — used as capacity gate in
  `Receive_Radio` line 224: `field_0x114 + 1 <= Type+0x5E0`.

**Note on `+0x66C DynamicVector`:** the task-brief mention of
`+0x66C` in the original Bio Reactor research is actually **0x670/0x67C**, a
different DynamicVector used for **upgrade attachments**, not absorb
occupants. `BuildingClass::PowerCheck_Upgrade` at **0x00450590** iterates
this vector (items @ +0x670, count @ +0x67C) to validate at-most-3
upgrades and toggle the "upgrade powered" flag at `+0x661`. It does **not**
read the Absorb flag; absorb is handled entirely inside `GetPowerOutput`
using the scalar `field_0x114`.

**UnitAbsorb (+0x16AE)** uses the exact same power-boost formula; the two
flags differ only in which RTTI type is allowed to enter. In
`Receive_Radio` at 0x0043C2D0 lines 192–220: UnitAbsorb accepts vehicles
(mission code 1), InfantryAbsorb accepts infantry (mission code 0x0F). The
power boost applies to whichever occupants made it inside.

**Relevant INI keys:** `[BuildingType] Power=`, `ExtraPower=`,
`MaxNumberOccupants=`, `InfantryAbsorb=yes`, `UnitAbsorb=yes`.

**Active in YR:** Yes. Confidence: **high**.

---

## 6. SecretLab (+0x16B0) — Secret Tech Lab

**Mechanic:** Each SecretLab grants one random unit drawn from the three
Rules tech-pool lists (`SecretInfantry`, `SecretUnits`, `SecretBuildings`).
The chosen unit's cameo becomes buildable by the House while the lab stands.

**Registry path:** The secret-lab registry add is called from `BuildingClass::Constructor`
at **0x0043B740** (call site **0x0043BB29**) and appends the lab pointer to the global
array at `DAT_008B41E4` (items) / `DAT_008B41F0` (count). This is a
process-wide registry of all secret labs that have ever been built.
`FUN_00442C40` is `BuildingClass::Init_Managers` (corrected 2026-05-29: was listed as
the "secret-lab registry add" — get_function_by_address 0x00442C40 shows it is
`BuildingClass__Init_Managers`, not the registry helper — ROOT: RTTI_LABEL_DRIFT).
The actual registry-add helper address inside the Constructor call at 0x0043BB29 was
not separately verified in this pass.

**Selection path:** `FUN_0068C050` — the secret-assignment routine.

```c
void AssignSecrets()
{
    if (SecretLabCount == 0) return;
    int poolTotal = Rules.SecretBuildings.TotalCount  // +0xD54
    if (poolTotal < SecretLabCount) return;

    // Build a candidate index list of size poolTotal
    for (int i = 0; i < poolTotal; ++i) candidates[local_8++] = i;

    // For each SecretLab, draw one candidate by index without replacement
    for (int lab = 0; lab < SecretLabCount; ++lab) {
        int pick = Random__RandomRanged(0, local_8 - 1);
        // compact candidates array (remove selected)
        local_8--;
        for (int j = pick; j < local_8; ++j) candidates[j] = candidates[j+1];

        // Resolve candidate index against the three concatenated lists
        TechnoTypeClass* pickedType;
        if (pick + 1 <= Rules.SecretInfantry.Count /*+0xD10*/) {
            pickedType = Rules.SecretInfantry.Items[pick];  // +0xD04
        } else {
            pick -= Rules.SecretInfantry.Count;
            if (pick + 1 <= Rules.SecretUnits.Count /*+0xD2C*/) {
                pickedType = Rules.SecretUnits.Items[pick];  // +0xD20
            } else {
                pick -= Rules.SecretUnits.Count;
                if (pick + 1 <= Rules.SecretBuildings.Count /*+0xD48*/) {
                    pickedType = Rules.SecretBuildings.Items[pick]; // +0xD3C
                }
            }
        }

        if (pickedType)
            SecretLabArray[lab]->field_0x6F4 = pickedType;
    }
}
```

**Rules field layout (all DynamicVector of TypeClass*; each instance is
28 bytes, field_offsets: +0x00 vtable, +0x04 items, +0x08 count, +0x0C cap,
+0x10 redundant/cached count):**

| Offset     | Field                      |
|------------|----------------------------|
| +0xD00..+0xD1B | `Rules.SecretInfantry`  (INI `[General] SecretInfantry=`) |
| +0xD1C..+0xD37 | `Rules.SecretUnits`     (INI `[General] SecretUnits=`)    |
| +0xD38..+0xD53 | `Rules.SecretBuildings` (INI `[General] SecretBuildings=`)|
| +0xD54        | `Rules.TotalSecretCount` / poolTotal (read inside ReadGeneral at line ~1090 of that function) |

Lists are populated by `RulesClass::ReadGeneral` at **0x0066D530**, lines
1084/1087/1090 of its decompile, via the TypeList helpers
`FUN_0067BB10` (infantry), `FUN_0067B720` (units), `FUN_0067B550`
(buildings).

**Granted-type storage:** `BuildingClass+0x6F4` — stores the chosen
TechnoTypeClass pointer. Downstream code (sidebar cameo gating and
`HouseClass::Can_Build`) queries this field to decide whether the secret
unit is buildable.

**When is AssignSecrets called?** The registry is populated as labs spawn
and is walked once the selection routine is triggered (typically the first
time any secret-lab-owning House requests a cameo refresh, or at scenario
start). Detailed trigger tracing of `FUN_0068C050` was not completed — its
caller chain is through the production/sidebar update path.

**Relevant INI keys:**
- `[General] SecretInfantry=` — comma list of InfantryType IDs
- `[General] SecretUnits=` — comma list of UnitType IDs
- `[General] SecretBuildings=` — comma list of BuildingType IDs
- per-building `SecretLab=yes`

**Active in YR:** Yes — standard for the Allied "Secret Lab" civilian
building. Confidence: **high** for the selection math and the 0x6F4 storage
slot; **medium** for when exactly AssignSecrets fires (caller chain from
`FUN_0068C050` not fully traced).

---

## 7. OrePurifier (+0x16CC) — Allied Ore Purifier

**Mechanic:** For every OrePurifier a House owns, a bonus percentage of each
raw-ore deposit is added on top of the base credit value.

**Registry:**
- `BuildingClass::OnConstructionComplete` at **0x00445F80** line ~200:
  `House[+0x538C] += 1` on Type[0x16CC] != 0.
- `BuildingClass::Limbo` at **0x00445880** (corrected 2026-05-29: was `OnDestroyed`;
  get_function_by_address 0x00445880 confirms `BuildingClass__Limbo` — ROOT: RTTI_LABEL_DRIFT):
  `House[+0x538C] -= 1` (clamped to 0).
- `BuildingClass::ChangeOwner` / `Unlimbo` also include a `+0x538C++` path
  when the ownership transition causes re-registration (verified in
  `0x00440580` line ~456 / `0x00448260`).

So `HouseClass+0x538C` is an int counter: `NumOrePurifiers`.

**Deposit formula:** `BuildingClass::DepositOreFromStorage` at **0x00522D50**:

```c
void BuildingClass::DepositOreFromStorage(BuildingClass* b)
{
    while ( (slot = StorageClass::FindFirstNonEmptySlot()) != -1 ) {
        HouseClass* house = b->Owner;

        // Effective purifier count; AI gets extra difficulty-scaled purifiers
        int purifiers = house->NumOrePurifiers;   // +0x538C
        if (!house->IsPlayerControl && g_GameMode != 0) {
            purifiers += Rules[+0x1324][ house->AI_Difficulty ];
        }

        float amount = StorageClass::GetAmount(slot);
        float bonus  = (float)purifiers
                     * Rules.PurifierBonus              // +0xF3C, float
                     * amount;

        StorageClass::RemoveAmount(amount, slot);
        if (amount > epsilon /* 0x007E1748 */) {
            HouseClass::Add_Tiberium_Credits(amount, slot);
            if (bonus > epsilon) {
                HouseClass::Add_Tiberium_Credits(bonus, slot);
            }
        }
    }
    // vtable+0x468 = "deposit finished" callback (refinery anim etc.)
    // corrected 2026-05-29: callback fires only when at least one ore slot was
    // deposited (bVar3 == true), not unconditionally (decompile_function 0x00522D50
    // shows `if (bVar3) { (**(code **)(*param_1 + 0x468))(); }`) — ROOT: OPERATOR_OR_ORDER_DRIFT
    if (any_ore_was_deposited) b->vtable[0x468]();
}
```

**Formula:**
```
bonus_credits = NumOrePurifiers * Rules.PurifierBonus * raw_ore_amount
credits_added = raw_ore_amount + bonus_credits
```

`Rules.PurifierBonus` at `RulesClass+0xF3C` is read in
`RulesClass::ReadGeneral` at 0x0066FC6A from `[General] PurifierBonus=`
(default 0.25 → +25% per purifier).

AI bonus: `Rules+0x1324` is a **pointer** to an array (corrected 2026-05-29: was
described as "a 3-int array" directly at that offset; binary shows
`*(int *)(g_RulesClass_Instance + 0x1324)` is dereferenced as a pointer and then indexed
by `*(int *)(iVar1 + 0x184) * 4` for the difficulty level — decompile_function 0x00522D50
— ROOT: MISLEADING). Extra "virtual purifiers" awarded to AI players proportional to
difficulty. INI key for this field not traced in this pass.

**Relevant INI keys:**
- `[General] PurifierBonus=<float>` (e.g. 0.25)
- per-building `OrePurifier=yes`

**Active in YR:** Yes. Confidence: **high**.

---

## 8. FactoryPlant (+0x16CD) — Industrial Plant

**Mechanic:** Each FactoryPlant multiplies one of five cost categories by a
per-building float. Multiple FactoryPlants stack multiplicatively.

**Per-building cost floats (set by `BuildingTypeClass::ReadINI` lines
352–371 of 0x0045FE50):**

| Offset   | INI key                 | Default |
|----------|-------------------------|---------|
| +0x16D0  | `InfantryCostBonus=`   | 1.0     |
| +0x16D4  | `UnitsCostBonus=`      | 1.0     |
| +0x16D8  | `AircraftCostBonus=`   | 1.0     |
| +0x16DC  | `BuildingsCostBonus=`  | 1.0     |
| +0x16E0  | `DefensesCostBonus=`   | 1.0     |

(All stored as `float`, read as `double` via `CCINIClass::ReadDouble` then
down-cast.)

**House accumulated multipliers** (computed by
`HouseClass::RecalcBonuses` at **0x0050BF60**):

| House offset | Category   |
|--------------|------------|
| +0x5390      | infantry   |
| +0x5394      | units      |
| +0x5398      | aircraft   |
| +0x539C      | buildings  |
| +0x53A0      | defenses   |

```c
void HouseClass::RecalcBonuses(HouseClass* h)
{
    h->+0x5390 = 1.0f; ... h->+0x53A0 = 1.0f;    // reset all five to 1.0
    for (i = 0; i < h->FactoryPlantCount /*+0x150*/; ++i) {
        BuildingClass*  plant = h->FactoryPlantArray[+0x144][i];
        BuildingTypeClass* t   = plant->Type;                // +0x520 (inside BuildingClass)
        h->+0x5390 *= t[+0x16D0];
        h->+0x5394 *= t[+0x16D4];
        h->+0x5398 *= t[+0x16D8];
        h->+0x539C *= t[+0x16DC];
        h->+0x53A0 *= t[+0x16E0];
    }
}
```

Invoked from `BuildingClass::Unlimbo`/`ChangeOwner` immediately after the
plant is registered in the `+0x144` list. Also re-invoked on
`OnSold`/`OnDestroyed` (via the same call site after the list is updated).

**Multipliers are applied at build-cost lookup time:**
`HouseClass::GetAccumulatedBonus` at **0x0050BEB0** takes a
`TechnoTypeClass*` and returns the matching cost multiplier from the five
fields by dispatching on RTTI-kind `vtable+0x2C`:

```c
float HouseClass::GetAccumulatedBonus(HouseClass* h, TechnoTypeClass* t)
{
    switch ( t->WhatAmI() ) {     // vtable+0x2C
        case 0x10: return h->+0x5390;  // InfantryTypeClass
        case 0x28: return h->+0x5394;  // UnitTypeClass
        case 0x03: return h->+0x5398;  // AircraftTypeClass
        case 0x07:                       // BuildingTypeClass
            if (t[+0x382] == 5 /*IsDefense*/) return h->+0x53A0;
            else                              return h->+0x539C;
        default: return 1.0f;
    }
}
```

Cost helpers `FUN_00711F00` and `FUN_00711F60` (cost getters) call both
`HouseClass::GetCostBonus` (the House's personal multiplier used for various
cost tuners) and `HouseClass::GetAccumulatedBonus`, then multiply the raw
`TechnoTypeClass.Cost` (vtable+0xAC) by both factors.

**Formula:**
```
effective_cost = Type.Cost * House.GetCostBonus() * House.GetAccumulatedBonus(Type)
```

Two FactoryPlants with `InfantryCostBonus=0.75` each → infantry cost ×0.5625.

**Relevant INI keys:** the five `*CostBonus` per-building floats listed above;
per-building `FactoryPlant=yes`. No per-house or per-side INI tuning is
required.

**Active in YR:** Yes. Confidence: **high**.

---

## Summary table

| Flag             | Runtime owner                     | Key fn address | Key formula / data                                 |
|------------------|-----------------------------------|----------------|----------------------------------------------------|
| Cloning          | `ExitObject_Main`                 | 0x00443C60     | Iterate House[+0xFC] Cloning list; vtable+0x100 spawns copy |
| Grinding         | `Unit/Infantry::Mission_Enter`    | 0x00739EC0, 0x005196A0 | `Add_Credits(unit->GetRefundValue())` via vtable+0x2BC; + passengers + slave |
| Hospital         | `MissionRepairAndProduce`         | 0x0044B780     | Trigger heal when `field_0x620 >= Rules.IRepairRate * 900.0` |
| Armory           | `MissionRepairAndProduce`         | 0x0044B780     | Same timer as Hospital; bumps VeterancyStruct one step       |
| InfantryAbsorb / UnitAbsorb | `GetPowerOutput`        | 0x0044E7B0     | `power += Type.ExtraPower * occupants`             |
| SecretLab        | `FUN_0068C050` (+ registry via Constructor @0x0043B740) | 0x0068C050 | Random pick across Rules.Secret{Infantry,Units,Buildings}; stored in `BuildingClass+0x6F4` |
| OrePurifier      | `DepositOreFromStorage`           | 0x00522D50     | `bonus = NumPurifiers * Rules.PurifierBonus * amount`; NumPurifiers at `HouseClass+0x538C` |
| FactoryPlant     | `RecalcBonuses` + `GetAccumulatedBonus` | 0x0050BF60, 0x0050BEB0 | Per-type multipliers at `Type+0x16D0..+0x16E0` stack multiplicatively into `HouseClass+0x5390..+0x53A0` |

## Active-in-YR status

All eight mechanics are **active by default in standard YR skirmish**. None
are gated behind `SpecialFlags` bits. None appear to be dormant Tiberian Sun
code — the code paths are reached unconditionally by the relevant missions
(`Mission_Enter`, `MissionRepairAndProduce`, `ExitObject_Main`,
`DepositOreFromStorage`) which fire in normal play. No TS-only caller gating
was observed.

## Confidence & caveats

- **High confidence:** Hospital/Armory timer formula; Bio Reactor power
  formula; OrePurifier deposit formula; FactoryPlant cost multiplier chain;
  Cloning mirror path; Grinder refund path.
- **Medium confidence:** exact initial value of `BuildingClass+0x638`
  (Hospital/Armory tick increment) — set somewhere during Unlimbo; the trigger
  point for SecretLab's `FUN_0068C050` assignment call (caller chain not
  fully walked).
- **To verify if needed:** the AI difficulty-purifier table at
  `RulesClass+0x1324`, specifically its INI-key name and default values.

## File addresses quick reference

| Address     | Function                                                      |
|-------------|---------------------------------------------------------------|
| 0x0045FE50  | `BuildingTypeClass::ReadINI` (param_1 is `int`, direct offsets) |
| 0x0043B740  | `BuildingClass::Constructor` (secret-lab registry call site 0x0043BB29) |
| 0x00440580  | `BuildingClass::Unlimbo` (registers building in House lists)  |
| 0x00443C60  | `BuildingClass::ExitObject_Main` (Cloning mirror path)        |
| 0x00445880  | `BuildingClass::Limbo` (OrePurifier counter decrement; corrected 2026-05-29: was labelled "OnDestroyed" — binary shows `BuildingClass__Limbo` via get_function_by_address 0x00445880 — ROOT: RTTI_LABEL_DRIFT) |
| 0x00445F80  | `BuildingClass::OnConstructionComplete` (OrePurifier increment) |
| 0x00448260  | `BuildingClass::ChangeOwner` (re-register on capture)         |
| 0x00448E30  | `BuildingClass::OnSold` (de-register)                          |
| 0x0044B780  | `BuildingClass::MissionRepairAndProduce` (Hospital + Armory)  |
| 0x0044E7B0  | `BuildingClass::GetPowerOutput` (InfantryAbsorb/UnitAbsorb)   |
| 0x00450590  | `BuildingClass::PowerCheck_Upgrade` (*upgrade slot* gating)   |
| 0x00522D50  | `BuildingClass::DepositOreFromStorage` (OrePurifier bonus)    |
| 0x00519 6A0 | `InfantryClass::Mission_Enter` (infantry grinder/absorb)      |
| 0x00739EC0  | `UnitClass::Mission_Enter` (vehicle grinder)                  |
| 0x0051E3B0  | `InfantryClass::What_Action_OnObject` (cursor grinder/hospital) |
| 0x0073FD50  | `UnitClass::What_Action_OnObject`                             |
| 0x0043C2D0  | `BuildingClass::Receive_Radio` (enter-acceptance for all 8)   |
| 0x0050BEB0  | `HouseClass::GetAccumulatedBonus` (FactoryPlant cost readback)|
| 0x0050BF60  | `HouseClass::RecalcBonuses` (FactoryPlant multiplier rebuild) |
| 0x0068C050  | SecretLab random-secret assignment                            |
| 0x00442C40  | `BuildingClass::Init_Managers` (corrected 2026-05-29: was "SecretLab Constructor-time registry add" — get_function_by_address shows Init_Managers — ROOT: RTTI_LABEL_DRIFT; actual registry-add helper inside Constructor call at 0x0043BB29 not separately verified) |
| 0x0066D530  | `RulesClass::ReadGeneral` (Secret{Infantry/Units/Buildings}, PurifierBonus) |
| 0x00712170  | `TechnoTypeClass::ReadINI` (Soylent at `param_1[0x185]` = +0x614) |
| 0x007E27F8  | `double 900.0` — Hospital/Armory timer constant               |
