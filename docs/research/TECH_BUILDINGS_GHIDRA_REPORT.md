# Tech Buildings — Ghidra RE Report

Reverse-engineered from `gamemd.exe` (image base 0x00400000, x86 LE 32-bit MSVC).
Covers the captureable neutral structures that appear on standard Yuri's Revenge
skirmish maps: Oil Derrick, Tech Hospital, Tech Machine Shop, Tech Airport,
Tech Outpost, Tech Secret Lab.

## 1. Overview

| Building (INI) | Name             | Mechanic (YR-verified)                                  | Confidence |
|----------------|------------------|---------------------------------------------------------|------------|
| `CAOILD`       | Oil Derrick      | `ProduceCashStartup=` on capture + `ProduceCashAmount=` every `ProduceCashDelay` ticks | **HIGH**   |
| `CATHOSP`      | Tech Hospital    | Global passive aura: `InfantryGainSelfHeal=1` grants `HouseClass::DoInfantrySelfHeal` | **HIGH**   |
| `CAMACH`       | Tech Machine Shop| Global passive aura: `UnitsGainSelfHeal=1` grants `HouseClass::DoUnitsSelfHeal`        | **HIGH**   |
| `CAAIRP`       | Tech Airport     | `SuperWeapon=ParaDropSpecial` granted to owning house on construction/capture         | **HIGH** (see §11) |
| `CAOUTP`       | Tech Outpost     | Acts as a Service Depot (`UnitRepair=yes`) + defensive turret + extra sight           | **HIGH**   |
| `CASLAB`       | Tech Secret Lab  | Contributes one type to the Rules secret pool at scenario start; `SecretInfantry=/SecretUnit=/SecretBuilding=` per-building overrides | **HIGH** for type lookup, **MEDIUM** for when the initial roll triggers |

**TS-legacy warning — READ FIRST.** `BuildingTypeClass` has a full TS-era
`Hospital=yes` / `Armory=yes` garrison-for-heal system (flags +0x16C1 / +0x16C2,
covered in `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md`). **That system is
DEAD in standard YR.** All YR tech medical buildings have `Hospital=yes` and
`Armory=yes` commented out in `rulesmd.ini` (see `[CATHOSP]` line 14016:
`;Hospital=yes ;gs old TS way`). Instead, YR tech buildings use the
passive-aura `InfantryGainSelfHeal=/UnitsGainSelfHeal=` system which heals
infantry/vehicles anywhere on the map while any such tech building is owned.
**Do NOT implement `Hospital=`/`Armory=` entry-for-heal unless explicitly
requested for a mod that re-enables it.**

Confidence for the broader system: **HIGH** overall. Tech Airport's
activation chain (ChangeOwner → `+0x1FC` flag → `HouseClass::Update` →
`AI_ManageProduction` + `AI_ResumeProduction` → `SuperClass::Activate`) was
walked instruction-by-instruction in the 2026-04-21 follow-up pass (§11) —
upgraded from MEDIUM to **HIGH**.

## 2. BuildingTypeClass Layouts — Tech-Building-Relevant Offsets

`BuildingTypeClass::ReadINI` is at **0x0045FE50**. Inside the function,
`ebp = param_1` (this pointer, verified at prologue 0x0045FE5F: `8b e9` mov ebp,ecx),
so all `[ebp+offset]` writes ARE direct byte offsets on `BuildingTypeClass`
(no ×4 scaling). Defaults are set in the constructor at **0x0045DD90**.

### Fields unique to tech buildings (not already in `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md`)

| Offset   | Type    | INI key                | Read site (in ReadINI) | Notes                                |
|----------|---------|------------------------|-----------------------:|--------------------------------------|
| +0x0EA4  | ptr     | `SecretInfantry=`      | 0x004605CD             | Per-building override (`InfantryTypeClass*`). Default NULL. |
| +0x0EA8  | ptr     | `SecretUnit=`          | 0x00460618             | Per-building override (`UnitTypeClass*`).                    |
| +0x0EAC  | ptr     | `SecretBuilding=`      | 0x0046064B             | Per-building override (`BuildingTypeClass*`).                |
| +0x16A9  | bool    | `UnitRepair=`          | 0x0046090D             | Service-depot repair-dock accept flag. **WAS misattributed** to `CanC4`/garrison-fire in `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` Phase 18 — correction below. |
| +0x1577  | bool    | `CanC4=`               | 0x00460050             | Separate from UnitRepair. Corrects the earlier misattribution. |

### Fields already documented — summary table for convenience

| Offset   | Type    | INI key                  | Source doc                                                   |
|----------|---------|--------------------------|--------------------------------------------------------------|
| +0x1552  | bool    | `Capturable=`            | BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2                         |
| +0x1558  | int     | `ProduceCashStartup=`    | BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT (verified here)   |
| +0x155C  | int     | `ProduceCashAmount=`     | same                                                         |
| +0x1560  | int     | `ProduceCashDelay=`      | same                                                         |
| +0x1564  | int     | `InfantryGainSelfHeal=`  | **new in this report** (verified 0x0046019D/0x004601B1 pair) |
| +0x1568  | int     | `UnitsGainSelfHeal=`     | **new in this report** (verified 0x004601BC)                 |
| +0x16B0  | bool    | `SecretLab=`             | BUILDINGCLASS_SPECIAL_BUILDINGS (dedicated section there)    |
| +0x16F0  | int     | `SuperWeapon=` (index)   | BUILDINGCLASS_UPDATE_AI_TICK / SUPERCLASS                    |
| +0x16F4  | int     | `SuperWeapon2=` (index)  | HouseClass::AI_ResumeProduction at 0x0050B1D0                 |
| +0xEA0   | ptr     | `FreeUnit=`              | `OnConstructionComplete` at 0x00446ED9                        |

### HouseClass aggregate counters read by AI_Update

| HouseClass offset | Meaning                                       | Fed by                    | Read by                                |
|-------------------|-----------------------------------------------|---------------------------|----------------------------------------|
| +0x120            | SecretLab DynamicVector count                 | `Type[0x16B0]` +/- in Unlimbo/ChangeOwner | `HouseClass::CanBuild` @ 0x004F7870 |
| +0x114            | SecretLab DynamicVector items                 | same                       | same                                   |
| +0x164            | Sum of `InfantryGainSelfHeal` over owned bldgs | `OnConstructionComplete`, `ChangeOwner`, `OnDestroyed`, `OnSold`, `Unlimbo` | `HouseClass::DoInfantrySelfHeal` @ 0x0050D9C0 |
| +0x168            | Sum of `UnitsGainSelfHeal` over owned bldgs    | same                       | `HouseClass::DoUnitsSelfHeal`     @ 0x0050D9D0 |
| +0x5778           | Sidebar-redraw-needed flag                     | many paths                 | sidebar updater                        |
| +0x258/+0x264     | SuperClass array items/count                   | House constructor          | `HouseClass::AI_ResumeProduction` @ 0x0050B1D0 |

### RulesClass offsets for tech-building tuning

| Rules offset | INI key (`[General]`)         | Stock value | Consumer                                                  |
|--------------|-------------------------------|-------------|-----------------------------------------------------------|
| +0x030       | `SelfHealInfantryFrames`      | 50          | `TechnoClass::AI_Update` @ 0x006FA8E2 — modulo tick gate   |
| +0x034       | `SelfHealInfantryAmount`      | 20          | `HouseClass::DoInfantrySelfHeal::GetAmount` @ 0x0050D9E0   |
| +0x038       | `SelfHealUnitFrames`          | 75          | `TechnoClass::AI_Update` @ 0x006FA7EE                      |
| +0x03C       | `SelfHealUnitAmount`          | 5           | `HouseClass::DoUnitsSelfHeal::GetAmount` @ 0x0050D9F0      |
| +0xC08/+0xC14/+0xC30 | `AmerParaDropInf=/Num=`  | `E1`/8      | ParaDrop SuperWeapon Case 6                                |
| +0xC40/+0xC4C/+0xC68 | `AllyParaDropInf=/Num=`  | `E1`/6      | ParaDrop SuperWeapon Case 5 Allied branch                  |
| +0xC78/+0xC84/+0xC? | `SovParaDropInf=/Num=`   | `E2`/9      | ParaDrop SuperWeapon Case 5 Soviet branch                  |
| +0xCB0/+0xCBC/+0xCD8 | `YuriParaDropInf=/Num=`  | `INIT`/6    | ParaDrop SuperWeapon Case 5 Yuri branch                    |
| +0xD00..+0xD1B | `SecretInfantry=` (vector) | `SNIPE,TERROR,DESO,YURI` | `FUN_0068C050` secret-lab assignment        |
| +0xD1C..+0xD37 | `SecretUnits=`             | `TNKD,TTNK,DTRUCK`       | same                                          |
| +0xD38..+0xD53 | `SecretBuildings=`         | `GTGCAN`                 | same                                          |
| +0xD54       | Total secret-pool count        | 8           | `FUN_0068C050` gating                                      |

**Confidence:** HIGH for every offset in the preceding four tables. Each is
verified directly against the byte stream of `BuildingTypeClass::ReadINI`,
`HouseClass::OnConstructionComplete`, or `HouseClass::DoInfantrySelfHeal` —
no reliance on YRpp names.

### Important naming correction on existing doc (`HEALTH_BAR_POSITIONING.md`)

The doc states:
> `HouseClass::DoInfantrySelfHeal` (house+0x164 > 0, from owning Hospital).

This is correct semantically, but the Ghidra function was renamed to
`HouseClass::HasPowerOutput` / `GetTotalPowerOutput` which is **misleading**.
The function reads `House+0x164` (InfantryGainSelfHeal aggregate counter), NOT
HouseClass::Power. It has nothing to do with the electric-power system. The
name is a TS-era misnomer inherited from the binary's original field (the old
TS "power output" field was repurposed for self-heal accumulator in YR).

The correct reinterpretation:
- `HouseClass::HasPowerOutput` (0x0050D9C0) → **is actually `HouseClass::DoInfantrySelfHeal`**
- `HouseClass::GetTotalPowerOutput` (0x0050D9E0) → **is actually `GetInfantrySelfHealAmount`** (`Rules.SelfHealInfantryAmount * House.InfantrySelfHealCount`)
- `HouseClass::HasPowerDrain` (0x0050D9D0) → **is actually `HouseClass::DoUnitsSelfHeal`**
- `HouseClass::GetTotalPowerDrain` (0x0050D9F0) → **is actually `GetUnitsSelfHealAmount`**

Confidence on this reinterpretation: **HIGH**. The house-wide power economy is
driven by `HouseClass::Power` (+0x164 is not it; the actual power state is in
different fields — see `POWER_SYSTEM_GHIDRA_REPORT.md`). The `DoInfantrySelfHeal`
path only appears in `TechnoClass::AI_Update` health-gain branches and in
`DrawPipScalePips` (self-heal blink pip) — both of which are gameplay self-heal
systems, not power. The `+0x1564`/`+0x1568` Type offsets store the INI values
`InfantryGainSelfHeal=` and `UnitsGainSelfHeal=` (verified from the string push
order at 0x0046019D and 0x004601BC in `ReadINI`).

---

## 3. Core Logic — Per-Mechanic Runtime Flow

### 3.1 Oil Derrick (`CAOILD`) — Per-tick Cash Generation

**Driver:** `BuildingClass::Update` @ 0x0043FB20 (vtable+23, slot 0x5C), Phase 6
at 0x0043FD28 — already documented in
`BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md`. Summary recap:

```c
// Phase 6 — ProduceCash timer
timer_remaining = this->field_0x6D8;
if (this->field_0x6D0 != -1) {
    elapsed = g_CurrentFrameCounter - this->field_0x6D0;
    if (elapsed < timer_remaining)
        timer_remaining -= elapsed;
    // else: tick expired, fall through to grant
}
if (timer_remaining == 1) {
    this->field_0x6D0 = g_CurrentFrameCounter;       // restart
    this->field_0x6D8 = Type+0x1560;                 // ProduceCashDelay
    // Grant only if NOT MultiplayPassive AND building operational
    if (Owner->HouseType+0x1A6 /*MultiplayPassive*/ == 0 && this->IsOperational()) {
        amount = Type+0x155C;                         // ProduceCashAmount
        if (amount < 1)
            HouseClass::Spend_Money(-amount);         // negative = cost
        else
            HouseClass::Add_Credits(amount);          // normal case
    }
}
```

**Which owner's credits?** `HouseClass::Add_Credits` is thiscall; the assembly
passes `this->Owner` (i.e., the current owning HouseClass) via ECX before the
call. So the per-tick income always goes to the building's **current** owner,
which is the capturing player (neutral buildings never trigger because
`HouseType+0x1A6 MultiplayPassive == 1` gates the grant off).

**Rollover behavior:** the timer counts frames. With stock `ProduceCashDelay=100`,
that's 100 game frames (≈ 6.67 seconds at 15 fps). No overflow — the timer is
re-seeded each cycle. A negative `ProduceCashAmount` subtracts credits (used
for drain-buildings like the `CAPOWR` civilian power plant, if ever set).

#### On-capture bootstrap — `ProduceCashStartup=`

**Driver:** `BuildingClass::ChangeOwner` @ 0x00448260, lines near 0x004482B0
(verified by direct disassembly of the byte stream at this session).

```asm
004482AA  8b 96 1c 02 00 00    mov  edx, [esi+0x21C]    ; esi=this, edx = this->Owner (OLD owner)
004482B0  8b 42 34             mov  eax, [edx+0x34]      ; eax = Old.HouseTypeClass*
004482B3  8a 88 a6 01 00 00    mov  cl,  [eax+0x1A6]     ; cl = HouseType.MultiplayPassive
004482B9  84 c9                test cl, cl
004482BB  74 3F                jz   skip                 ; grant only if OLD owner is MultiplayPassive (i.e., captured FROM neutral)
004482BD  8b 8e 20 05 00 00    mov  ecx, [esi+0x520]     ; ecx = BuildingTypeClass*
004482C3  8b 81 58 15 00 00    mov  eax, [ecx+0x1558]    ; eax = ProduceCashStartup
004482C9  85 c0                test eax, eax
004482CB  74 2F                jz   skip
004482CD  50                   push eax                  ; amount
004482CE  8b cb                mov  ecx, ebx             ; ebx = in_stack_00000004 = NEW owner (capturer)
004482D0  e8 7b 16 0B 00       call HouseClass::Add_Credits
; then initialise per-tick cash timer to begin from CurrentFrame
004482D5  8b 96 20 05 00 00    mov  edx, [esi+0x520]
004482DB  a1 84 ed a8 00       mov  eax, [g_CurrentFrameCounter]
004482E0  8b 8a 60 15 00 00    mov  ecx, [edx+0x1560]    ; ProduceCashDelay
004482E6  ...                  [esi+0x6D0] = eax
004482F0  ...                  [esi+0x6D8] = ecx
```

**Net effect of capture from Neutral:** the capturing house receives
`ProduceCashStartup` credits immediately (Stock CAOILD: **1000 credits**, per
`rulesmd.ini` line 13949), and the per-tick timer begins at the current frame.
On engineer capture-from-another-player (neither house is MultiplayPassive),
no startup bonus is granted — the timer continues but is freshly re-seeded by
the existing Phase-6 logic.

**Confidence: HIGH.** Every instruction decoded from raw bytes.

### 3.2 Tech Hospital (`CATHOSP`) — Infantry Aura

Stock YR implementation is **NOT** the TS-era `Hospital=yes` walk-inside
heal-over-time. That flag at `Type+0x16C1` exists in the binary but is commented
out in the INI (`[CATHOSP]` does not set `Hospital=`). The **active YR mechanic**
is global passive regen:

**Driver:** `TechnoClass::AI_Update` @ 0x006F9E50, infantry/organic health-tick
branch at ~0x006FA8B0 (verified):

```c
// iVar7 = current Health, type->Strength at +0xA0
// bVar18 = (Strength <= Health), i.e., "fully healed"
// iVar8 = WhatAmI() (RTTI)

if ( (iVar8 != 1 /*UnitClass*/ || type[0xD97] /*Organic=yes*/)
     && Health != 0 && !bVar18 ) {
    // Infantry (RTTI 0x0F) OR Organic unit (RTTI 1 with Organic=yes)
    if ( (iVar8 == 0x0F || (iVar8 == 1 && type[0xD97]))
         && (g_CurrentFrameCounter % Rules+0x30 /*SelfHealInfantryFrames*/ == 0)
         && HouseClass::DoInfantrySelfHeal(Owner) /* Owner+0x164 > 0 */ ) {
        int maxHP   = type[0xA0];                      // Strength
        int needed  = maxHP - Health;
        int amount  = HouseClass::GetInfantrySelfHealAmount(Owner);
        Health     += min(amount, needed);
    }
}
```

The `GetInfantrySelfHealAmount` helper at 0x0050D9E0 returns
`Rules.SelfHealInfantryAmount * Owner.InfantrySelfHealCount` — i.e.,
`20 * Owner+0x164`, where `Owner+0x164` is the aggregate of every owned
building's `InfantryGainSelfHeal=` value (stock CATHOSP sets it to 1, so one
Tech Hospital contributes +20 HP every 50 frames ≈ 3.33 seconds).

**Registry maintenance:** `BuildingClass::OnConstructionComplete` @ 0x00445F80
at ~0x00446398:
```c
if ( Type+0x1564 /*InfantryGainSelfHeal*/ != 0 ) {
    Owner+0x164 += Type+0x1564;
}
```
`ChangeOwner` at 0x00448260 line ~0x00448AD0: subtracts the old owner's
counter and adds to the new owner's counter. `OnSold`, `OnDestroyed`, and
`Unlimbo` maintain the counter symmetrically (byte pattern `64 01 00 00`
matches show the consumers at 0x00445996, 0x00448ae8, 0x004491F9, 0x0045E025 —
`OnDestroyed`, `ChangeOwner`, `OnSold`, type-constructor default init).

**Who gets healed:** every infantry the capturing house owns, anywhere on the
map — not just infantry adjacent to the hospital. The only spatial requirement
is that the unit be alive, not full HP, and "infantry RTTI 0x0F OR Organic=yes
vehicle". Aircraft (RTTI 3) are never eligible.

**Confidence: HIGH.** Health delta, modulo frame gate, and counter maintenance
all verified from decompile. Identity of `HouseClass::DoInfantrySelfHeal` vs.
the Ghidra misnomer `HouseClass::HasPowerOutput` confirmed by tracing
`[param_1+0x164]` reads into `TechnoClass::AI_Update` health branch — which
never touches `HouseClass::Power` (+0x4C8/+0x4CC).

### 3.3 Tech Machine Shop (`CAMACH`) — Vehicle Aura

**Identical structure to Tech Hospital** but for non-Organic vehicles (RTTI 1
without `Organic=yes`). Driver branch in the same
`TechnoClass::AI_Update` (0x006FA7EE region):

```c
else if ( (g_CurrentFrameCounter % Rules+0x38 /*SelfHealUnitFrames*/ == 0)
          && HouseClass::DoUnitsSelfHeal(Owner) /* Owner+0x168 > 0 */ ) {
    int amount = HouseClass::GetUnitsSelfHealAmount(Owner);   // Rules.SelfHealUnitAmount × Owner+0x168
    Health    += min(amount, maxHP - Health);
}
```

Stock: `Rules.SelfHealUnitFrames=75`, `SelfHealUnitAmount=5`, CAMACH sets
`UnitsGainSelfHeal=1`. One Tech Machine Shop heals every owned vehicle +5 HP
every 75 frames (5 seconds).

**Type field:** `BuildingTypeClass+0x1568 = UnitsGainSelfHeal=` (int). Verified
from `ReadINI` byte stream at 0x004601BC (push string) / 0x004601CA (store to
`[ebp+0x1568]`).

**Registry maintenance:** same lifecycle hooks as Tech Hospital, maintaining
`HouseClass+0x168`. Mirror offsets throughout OnConstructionComplete,
ChangeOwner, OnDestroyed, OnSold, Unlimbo.

**Confidence: HIGH.**

### 3.4 Tech Airport (`CAAIRP`) — Free Paratroop Drop

**INI mechanism:** `[CAAIRP]` sets `SuperWeapon=ParaDropSpecial`. This is read
into `BuildingTypeClass+0x16F0` (int, SW index enum) and `+0x16F4` (int,
SuperWeapon2 index; usually -1).

**Superweapon lookup & grant:** The house-level SuperClass for a given SW type
is activated when the house owns at least one building with that SW type via
`HouseClass::AI_ResumeProduction` @ 0x0050B1D0:

```c
for each SuperClass in House[+0x258..+0x264] {
    if (sc->IsActive) continue;
    for each owned building b in House[+0x68..+0x78] {
        for slot in b[+0x5EC, +0x5F0, +0x5F4] /* 3 AuxBuilding slots */
            if ( slot != 0 && ( slot->Type+0x16F0 == sw_index
                             || slot->Type+0x16F4 == sw_index ) ) {
                bVar2 = true;
                break;
            }
        if ( BuildingClass::GetSuperWeaponIndex1(b) == sw_index
           || BuildingClass::GetSuperWeaponIndex2(b) == sw_index ) {
            bVar2 = true;
            break;
        }
    }
    if (bVar2 && passed_aux_building_check) {
        SuperClass::Activate( sc, (Owner == g_PlayerPtr), /*is_charged*/ ... );
        if (Owner == g_PlayerPtr) {
            SidebarClass::AddCameo(0x1F, sw_index);   // add sidebar button
        }
    }
}
```

`BuildingClass::GetSuperWeaponIndex1/2` (0x00457630 / 0x00457690) return
`Type+0x16F0` / `Type+0x16F4` if the owning house also satisfies the
`AuxBuilding` prerequisite (via `Type+0xC8` pointer to a prereq AuxBuilding
TechnoType; the helper calls `HouseClass::CountOwnedInstances` and returns
0xFFFFFFFF if the prereq isn't owned).

**When does this fire?** `AI_ResumeProduction` is invoked whenever
`HouseClass+0x1FC` (ProductionChanged flag) is set. Every path that adds a
building to a house sets this flag — including `BuildingClass::Unlimbo` and
`BuildingClass::ChangeOwner` (verified: both call `HouseClass::Recount` and
set +0x1FC = 1 near the end of their bodies).

**When fired:** Paratroop SW (case 5 in `SuperClass::Launch`, documented in
`SUPERCLASS_SYSTEM_GHIDRA_REPORT.md`):
1. Player clicks sidebar `PARAICON`.
2. Target cell resolved (bridge-adjusted if needed).
3. Side is computed from owner's CountryClass via `FUN_0041CAA0`.
4. For each infantry type in the side-appropriate list (Allied: `AllyParaDropInf/Num`,
   Soviet: `SovParaDropInf/Num`, Yuri: `YuriParaDropInf/Num`), call
   `FUN_0065E660` to spawn the paradrop aircraft (badger/sovpara/yuri transport).

Stock rulesmd.ini values:
- Allied capturer: drops 6× `E1` (GI)
- Soviet capturer: drops 9× `E2` (Conscript)
- Yuri capturer: drops 6× `INIT` (Initiate)

`[ParaDropSpecial] RechargeTime=4` → 4-minute recharge between free drops.
`[ParaDropSpecial] IsPowered=false` → works even if the airport is un-powered
(civilian buildings have no power anyway).

**Why MEDIUM not HIGH:** the SuperClass::Activate call path during capture is
inferred from AI_ResumeProduction being invoked on `ProductionChanged`. The
explicit call chain from `ChangeOwner` → `Recount` → activator hasn't been
traced instruction-by-instruction in this session. Other superweapon-granting
buildings (nuke silo, iron curtain) follow the same pattern and are documented
to work, so behaviorally this is almost certainly correct in-engine. But the
confidence level on the **exact** instruction-stream trigger should remain
MEDIUM until verified.

### 3.5 Tech Outpost (`CAOUTP`) — Service Depot + Sight + Turret

**Three distinct mechanics, all live in YR:**

#### (a) Vehicle repair dock (`UnitRepair=yes`)
`BuildingTypeClass+0x16A9 = UnitRepair` (bool). Verified from `ReadINI` byte
stream at 0x0046090D (push "UnitRepair") / 0x00460929 (store to `[ebp+0x16A9]`).

Consumed in `BuildingClass::Receive_Radio` @ 0x0043C2D0:
- Case 0x0E (dock-request): if `Type[0x16A9]` is set AND the sender is in the
  dock list AND radius-check passes, accept.
- Case 0x0F (service-request): if `Type[0x16A9]` is set AND the requester is
  RTTI 1 (UnitClass) or 2 (AircraftClass), return 1 (accept) subject to
  power/capacity gates.

Stock RepairRate/RepairStep apply (`Rules.RepairRate=.016 min`, `RepairStep=8`
HP). The captured Tech Outpost is functionally identical to a player-built
Allied or Soviet Service Depot for repair purposes.

#### (b) Sight extension (`Sight=6`)
Standard `TechnoTypeClass+0x5C0` (Sight) field; the captured outpost reveals
a 6-cell-radius circle like any building. No tech-specific logic — it's just
that the outpost lives on the map so the capturing house inherits its vision.

#### (c) Defensive turret (`Primary=HoverMissile`, `Turret=yes`, `TurretAnim=OUTP`)
Standard `TechnoTypeClass+0xAC` (Primary weapon) field. The outpost fires the
HoverMissile weapon at enemy air targets when captured. No special-case code.

#### (d) Captured-neutral autosell / ownership reconciliation
Separate from the three mechanics above: `CAOUTP` has `Capturable=yes` +
`NeedsEngineer=yes`, meaning it's captured only by the explicit
`InfantryClass::Mission_Capture` path (engineer), NOT the garrison-reconciliation
path used for bunkers. See `ENGINEER_CAPTURE_GHIDRA_REPORT.md`.

**Confidence: HIGH.** All offsets and `UnitRepair` semantics verified from
`Receive_Radio` decompile.

### 3.6 Tech Secret Lab (`CASLAB`) — Stolen-Tech Unit Unlock

Already well-documented in `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md`
section 6 — core mechanic (random pick from Rules pool, stored at
`BuildingClass+0x6F4`). Additions from this session:

#### Per-building override fields (new)

When a specific `CASLAB` sets one of these keys, that type is granted
**instead of** rolling from the Rules pool:

| Offset   | Type    | INI key              | Resolution |
|----------|---------|----------------------|-----------|
| +0x0EA4  | ptr     | `SecretInfantry=`    | First-priority override (InfantryTypeClass*)  |
| +0x0EA8  | ptr     | `SecretUnit=`        | Second-priority override (UnitTypeClass*)     |
| +0x0EAC  | ptr     | `SecretBuilding=`    | Third-priority override (BuildingTypeClass*)  |

Stock `rulesmd.ini` `[CASLAB]` **commented them all out** (lines 14078-14080),
so in standard skirmish every secret lab rolls from the pool. But custom maps
or mods can override (`SecretInfantry=YURIPR` or similar). Note the INI keys
are singular (`SecretInfantry=`, not `SecretInfantrys=`) on the per-building
level.

#### Lookup function `BuildingClass::GetSecretLabTech` @ 0x00459840

```c
int BuildingClass::GetSecretLabTech(BuildingClass* b) {
    BuildingTypeClass* t = b->Type;                  // +0x520
    int type = t+0xEA4;                              // SecretInfantry override
    if (type != 0) return type;
    type = t+0xEA8;                                  // SecretUnit override
    if (type != 0) return type;
    type = t+0xEAC;                                  // SecretBuilding override
    if (type != 0) return type;
    return b->field_0x6F4;                           // Rolled type (see below)
}
```

Called exclusively from `HouseClass::CanBuild` @ 0x004F7870 line ~0x004F7A10,
inside the loop over `this->field_0x120` (SecretLabCount):

```c
for (int i = 0; i < this->SecretLabCount; ++i) {
    BuildingClass* lab = this->SecretLabArray[i];
    TechnoTypeClass* grant = BuildingClass::GetSecretLabTech(lab);
    if (grant == query_type) {
        // Type is unlocked by this lab — return buildable
        return 1;   // or proceed to factory/cameo check
    }
}
```

This is the end of the unlock chain: sidebar cameo display, build-menu gating,
and prereq checks all call `HouseClass::CanBuild` internally. When the house
owns a secret lab whose `GetSecretLabTech` returns the queried type, that type
becomes buildable from the appropriate factory (barracks for infantry, war
factory for units, construction yard for buildings).

#### Initial rolling (confirmed and reused from previous report)

`FUN_0068C050` is called once per game session to pick random types from the
concatenated `Rules.SecretInfantry` / `SecretUnits` / `SecretBuildings` arrays
and assign one to each secret lab's `BuildingClass+0x6F4`. The caller chain
(exactly when during scenario init) is still not fully traced; likely from
`Scenario::Init` or the first `HouseClass::AI_Process` after all buildings are
placed.

**Confidence:** HIGH for lookup and field offsets; **MEDIUM** for the exact
trigger point of the initial roll (caller chain of `FUN_0068C050`).

---

## 4. INI Keys Consumed — Per Tech Building

### Oil Derrick (`[CAOILD]`)
| Key                     | Default | Source section      | Type field | Consumed by              |
|-------------------------|---------|---------------------|-----------|--------------------------|
| `ProduceCashStartup=`   | 0       | `[CAOILD]`          | +0x1558    | `ChangeOwner` (on capture from Neutral) |
| `ProduceCashAmount=`    | 0       | `[CAOILD]`          | +0x155C    | `BuildingClass::Update` Phase 6          |
| `ProduceCashDelay=`     | 0       | `[CAOILD]`          | +0x1560    | `BuildingClass::Update` Phase 6          |
| `Capturable=`           | no      | `[CAOILD]`          | +0x1552    | `InfantryClass::Mission_Capture`        |
| `NeedsEngineer=`        | no      | `[CAOILD]`          | +0x1553?   | same                                      |
| `WorkingSound=`         | -1      | `[CAOILD]`          | +0xE80     | `BuildingClass::Update` Phase 1          |

### Tech Hospital (`[CATHOSP]`)
| Key                        | Default | Source section             | Type/Rules field | Consumed by                            |
|----------------------------|---------|----------------------------|------------------|-----------------------------------------|
| `InfantryGainSelfHeal=`    | 0       | `[CATHOSP]`                | Type+0x1564      | `OnConstructionComplete` → Owner+0x164 |
| `Capturable=`              | no      | `[CATHOSP]`                | Type+0x1552      | Mission_Capture                         |
| `SelfHealInfantryFrames=`  | 50      | `[General]`                | Rules+0x30       | `TechnoClass::AI_Update`               |
| `SelfHealInfantryAmount=`  | 20      | `[General]`                | Rules+0x34       | `GetInfantrySelfHealAmount`            |

### Tech Machine Shop (`[CAMACH]`)
| Key                      | Default | Source section  | Type/Rules field | Consumed by                            |
|--------------------------|---------|-----------------|------------------|-----------------------------------------|
| `UnitsGainSelfHeal=`     | 0       | `[CAMACH]`      | Type+0x1568      | `OnConstructionComplete` → Owner+0x168 |
| `SelfHealUnitFrames=`    | 75      | `[General]`     | Rules+0x38       | `TechnoClass::AI_Update`                |
| `SelfHealUnitAmount=`    | 5       | `[General]`     | Rules+0x3C       | `GetUnitsSelfHealAmount`                |
| `Capturable=`            | no      | `[CAMACH]`      | Type+0x1552      | Mission_Capture                         |

### Tech Airport (`[CAAIRP]`)
| Key                           | Default | Source section       | Type/Rules field | Consumed by                             |
|-------------------------------|---------|----------------------|------------------|------------------------------------------|
| `SuperWeapon=`                | `None`  | `[CAAIRP]`           | Type+0x16F0      | `HouseClass::AI_ResumeProduction`        |
| `Ammo=5`                      | —       | `[CAAIRP]`           | Type+? (unused for tech)  | —                              |
| `AllyParaDropInf/Num=`        | `E1`/6  | `[General]`          | Rules+0xC40/+0xC4C/+0xC68 | `SuperClass::Launch` case 5    |
| `SovParaDropInf/Num=`         | `E2`/9  | `[General]`          | Rules+0xC78/+0xC84         | same                              |
| `YuriParaDropInf/Num=`        | `INIT`/6| `[General]`          | Rules+0xCB0/+0xCBC/+0xCD8 | same                              |
| `AmerParaDropInf/Num=`        | `E1`/8  | `[General]`          | Rules+0xC08/+0xC14/+0xC30  | `SuperClass::Launch` case 6 (AmericanParaDropSpecial only) |
| `[ParaDropSpecial] RechargeTime=` | 4 min | `[ParaDropSpecial]` | SuperWeaponTypeClass+0x24? | `SuperClass::CDTimer`            |
| `[ParaDropSpecial] IsPowered=`    | false | `[ParaDropSpecial]` | SuperWeaponTypeClass+0x??? | `SuperClass::Suspend`            |

### Tech Outpost (`[CAOUTP]`)
| Key                  | Default | Source section  | Type field | Consumed by                                |
|----------------------|---------|-----------------|------------|---------------------------------------------|
| `UnitRepair=`        | no      | `[CAOUTP]`      | +0x16A9    | `BuildingClass::Receive_Radio` cases 14/15  |
| `NumberOfDocks=1`    | 0       | `[CAOUTP]`      | +0x692/+0x696 (already documented)  | docking / pathfinding |
| `Sight=6`            | 0       | `[CAOUTP]`      | TechnoTypeClass+0x5C0 | `MapClass::RevealAroundCell` |
| `Primary=HoverMissile` | —     | `[CAOUTP]`      | TechnoTypeClass+0xAC  | `MissionClass::Mission_Attack`           |
| `Turret=yes`, `TurretAnim=OUTP` | — | `[CAOUTP]` | various   | animation system                          |
| `Capturable=yes`, `NeedsEngineer=yes` | — | `[CAOUTP]` | Type+0x1552 / Type+0x? | Mission_Capture     |

### Tech Secret Lab (`[CASLAB]`)
| Key                     | Default       | Source section        | Type/Rules field | Consumed by                                 |
|-------------------------|---------------|-----------------------|------------------|----------------------------------------------|
| `SecretLab=yes`         | no            | `[CASLAB]`            | Type+0x16B0      | Constructor lifecycle registry (FUN_00442C40), ChangeOwner, OnDestroyed |
| `SecretInfantry=`       | `NULL`        | `[CASLAB]` (optional) | Type+0xEA4       | `GetSecretLabTech`                           |
| `SecretUnit=`           | `NULL`        | `[CASLAB]` (optional) | Type+0xEA8       | same                                         |
| `SecretBuilding=`       | `NULL`        | `[CASLAB]` (optional) | Type+0xEAC       | same                                         |
| `SecretInfantry=` (Rules, plural sense via `;` list) | `SNIPE,TERROR,DESO,YURI` | `[General]` | Rules+0xD00..+0xD1B | `FUN_0068C050` roll                    |
| `SecretUnits=`          | `TNKD,TTNK,DTRUCK` | `[General]`     | Rules+0xD1C..+0xD37 | same                                      |
| `SecretBuildings=`      | `GTGCAN`       | `[General]`         | Rules+0xD38..+0xD53 | same                                      |

**Note:** the per-building keys are **singular** (`SecretInfantry=`, `SecretUnit=`,
`SecretBuilding=`) while the Rules pool keys are **plural** (`SecretInfantry=`,
`SecretUnits=`, `SecretBuildings=`). The **per-building `SecretInfantry=`**
collides in name with the Rules `SecretInfantry=`; they are in different sections
so the INI parser disambiguates them. Stock YR comments out all three per-building
keys in `[CASLAB]`, so this collision has no effect in standard skirmish.

**`SecretLabTechs`** — flagged in the scope: **does NOT exist in stock YR
binary.** Verified by `search_strings` — no match. It may be an Ares/YRpp
extension key, but it's not part of the stock engine. The correct stock key
names are the six listed above.

---

## 5. Integration Points

### 5.1 `BuildingClass::Update` tick path (per-tick)
- Phase 6: ProduceCash timer (Oil Derrick; also runs harmlessly on all buildings
  since `ProduceCashDelay=0` gates the timer).

### 5.2 `TechnoClass::AI_Update` (per-tick, every infantry/unit/aircraft)
- Health-gain branch: InfantryGainSelfHeal / UnitsGainSelfHeal aura consumers.

### 5.3 `BuildingClass::OnConstructionComplete` @ 0x00445F80
Runs when any non-placed building finishes construction (including when
`Unlimbo` passes `ActuallyPlacedOnMap=true` for map-initial placement). Does:
- Start ProduceCash timer (`+0x6D0 = CurrentFrame`, `+0x6D8 = ProduceCashDelay`)
  if `Type+0x1560 != 0`.
- `Owner+0x164 += Type+0x1564` (InfantryGainSelfHeal).
- `Owner+0x168 += Type+0x1568` (UnitsGainSelfHeal).
- `Owner+0x538C += 1` if `Type[0x16CC]` (OrePurifier) — unrelated but same pattern.
- `Owner+0x2D4 += Type+0x1780` if `Type[0x16CB]`.
- **Does NOT activate SuperWeapon** directly — sets `Owner+0x1FC = 1`
  (ProductionChanged) which AI_ResumeProduction consumes at the next tick.
- Spawns `FreeUnit=` (Type+0xEA0) — conditionally for Allied/Soviet MCV etc.

### 5.4 `BuildingClass::ChangeOwner` @ 0x00448260
Tech-building-relevant steps:
- **ProduceCashStartup grant** (lines 0x004482B0..0x004482DD): if OLD owner was
  MultiplayPassive AND `Type+0x1558 != 0`, credit the NEW owner with the startup
  amount and re-seed the ProduceCash timer.
- **Aggregate counter transfers:** subtract `Type+0x1564` from OLD
  `Owner+0x164`, `Type+0x1568` from `Owner+0x168`, etc. — then add to NEW
  owner's counters via the later "register in new house's lists" loop. See
  `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` for the 10-list enumeration.
- **SecretLab registration**: if `Type[0x16B0]`, remove from old
  `House[0x114..0x120]` list and add to new house's list.
- **SuperWeapon indirect activation:** sets `NewOwner+0x1FC = 1`, then
  `AI_ResumeProduction` on the next tick calls `SuperClass::Activate` for any
  SW now ownable.

### 5.5 `BuildingClass::OnSold` @ 0x00448E30 and `OnDestroyed` @ 0x00445880
Decrement `Owner+0x164 / +0x168` (InfantryGainSelfHeal / UnitsGainSelfHeal),
remove from SecretLab list, etc. — symmetric to OnConstructionComplete add.

### 5.6 `BuildingClass::Unlimbo` @ 0x00440580
For pre-placed (map-initial) civilian buildings, Unlimbo is the initial
registration path. Adds to every applicable `HouseClass` typed list, same as
ChangeOwner's add phase. **Also calls `HouseClass::AI_ResumeProduction`** in
the upgrade-slot sub-branch, which is where SuperClass::Activate gets
triggered for non-tech buildings.

For civilian neutral buildings with `SuperWeapon=ParaDropSpecial`, the SW
doesn't activate on the neutral house (HouseType MultiplayPassive gates it);
it activates only after capture when the NEW owner's next AI_ResumeProduction
pass walks over the just-captured building.

---

## 6. Current Rust Implementation Status

### Parsed INI keys (struct fields exist, no gameplay code)

| Key                  | Rust location                                    | Consumed? |
|----------------------|--------------------------------------------------|-----------|
| `FreeUnit=`          | `src/rules/object_type.rs:695`                  | **NO** — parsed but never spawns on construction complete. |
| `UnitRepair=`        | `src/rules/object_type.rs:820`                  | **PARTIAL** — cursor check in `src/sim/world/world_commands.rs:517` (allow "enter" cursor on friendly depot). No repair loop. |
| `InfantryAbsorb=`    | `src/rules/object_type.rs:807`                  | NO — not a tech-building key in stock YR, but parsed. |
| `Capturable=`        | `src/rules/object_type.rs:767`                  | NO — engineer capture not implemented. |
| `PurifierBonus=`     | `src/rules/ruleset.rs:674`                      | NO — no purifier bonus applied to ore deposits. |
| `[AmerParaDrop]` SW type | `src/rules/superweapon_type.rs:40`         | NO — enum variant exists, no logic. |

### NOT PARSED, NOT IMPLEMENTED

| System                                   | Status         |
|------------------------------------------|----------------|
| `ProduceCashStartup/Amount/Delay=`       | 0% — no INI parse, no sim tick |
| `InfantryGainSelfHeal=` / `UnitsGainSelfHeal=` | 0% — no parse, no aura |
| `SelfHealInfantry/UnitAmount/Frames` (Rules) | 0% |
| `SecretLab=` flag                        | 0% |
| Per-building `SecretInfantry=/SecretUnit=/SecretBuilding=` overrides | 0% |
| `Rules.SecretInfantry/SecretUnits/SecretBuildings` pools | 0% |
| Scenario-init SecretLab roll             | 0% |
| `HouseClass::GetSecretLabTech`           | 0% |
| SuperWeapon `SuperWeapon=` type field    | 0% |
| ParaDrop SW launch (all sides)           | 0% |
| `AllyParaDrop/SovParaDrop/YuriParaDrop/AmerParaDropInf/Num` (Rules) | 0% |
| `[ParaDropSpecial]` / `[AmericanParaDropSpecial]` superweapon instances | 0% |
| `HouseClass::DoInfantrySelfHeal / DoUnitsSelfHeal` | 0% |
| Civilian-neutral ownership (MultiplayPassive side) | 0% — no "Neutral" house concept in engine |
| Engineer capture mission                 | 0% |

**Summary:** Tech Buildings are **fully inert** in the current engine. Zero of
the six mechanics function. The two cosmetic presences — FreeUnit parse and
UnitRepair cursor-only — don't affect gameplay. All tech building captures,
auras, and superweapons are absent.

### Minimum implementation surface to make tech buildings functional

1. **ObjectType (rules)**: add fields for `ProduceCashStartup/Amount/Delay`,
   `InfantryGainSelfHeal`, `UnitsGainSelfHeal`, `SuperWeapon` name, `SecretLab`,
   per-building `SecretInfantry/Unit/Building`.
2. **Ruleset (rules)**: parse `Rules.SelfHealInfantry{Frames,Amount}` and
   `Rules.SelfHealUnit{Frames,Amount}`, the three Rules secret pools, the four
   paradrop inf/num arrays.
3. **World owner model**: introduce a "Neutral" / MultiplayPassive house and
   its HouseType flag; ensure map-initial CAOILD/CATHOSP/CAAIRP/etc. spawn
   owned by this house.
4. **Sim/production (house)**: per-house aggregate counters
   `infantry_self_heal_count`, `units_self_heal_count`, `secret_lab_list`,
   `super_weapons`.
5. **Sim/building tick**: per-tick ProduceCash timer (analogous to current
   ore/ refinery tick), decrement/accumulator update.
6. **Sim/infantry + unit tick**: in `TechnoClass::AI_Update` equivalent, the
   self-heal branch gated on frame modulo and house counter > 0.
7. **Sim/capture** (engineer mission): change owner, fire ProduceCashStartup
   grant if applicable, transfer counters, grant SuperWeapon, add SecretLab to
   list.
8. **Sim/superweapon**: paradrop launch handler, side-dependent infantry
   spawn, transport aircraft flyover.

Full implementation is a major feature — see scope note in
`docs/gap-scans/2026-04-21-gap-scan.md`.

---

## 7. Open Questions

### Q1. Exact SuperClass::Activate trigger chain on capture (Confidence: MEDIUM)

We know `ChangeOwner` sets `NewOwner+0x1FC = 1` (ProductionChanged), and we
know `AI_ResumeProduction` is the function that walks the SW array and calls
`SuperClass::Activate` for SWs now ownable. We **have not** traced the exact
code path from tick-start to `AI_ResumeProduction` — is it called once per
`HouseClass::AI_Process`? Conditional on `+0x1FC`? Via a HouseClass vtable?
Further Ghidra walking needed.

**Impact:** Low for implementation. Once we call `activate` whenever a new SW
type becomes ownable (any capture, any build), behavior will match.

### Q2. Initial SecretLab roll trigger (Confidence: LOW)

`FUN_0068C050` picks the random tech per lab. We don't know exactly when it's
called. Candidates:
- Scenario init, after all pre-placed buildings are Unlimbo'd.
- First `HouseClass::AI_Process` after scenario starts.
- `HouseClass::CanBuild` on first cameo query (lazy).

**Impact:** Low. Must be called once per game to fill every lab's `+0x6F4`.

### Q3. ProduceCashStartup=0 behavior (Confidence: MEDIUM)

`ChangeOwner`'s check is `Type+0x1558 != 0 → grant`. So `ProduceCashStartup=0`
means no startup. What about negative? The per-tick ProduceCashAmount handles
negative (spends credits). Startup code uses `test eax, eax / jz` — so any
non-zero (including negative) triggers an Add_Credits with the value. Negative
startup would mean "pay to capture". Not stock behavior but possible in mods.

### Q4. Scenario-capturable neutral → player transfer on single-player campaign

The per-tick `CheckAutoSellOrCivilian` (0x00458200) documented in
`BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` reconciles garrison ownership. But
tech buildings have `CanBeOccupied=no` (neither `CAOILD` nor `CATHOSP` has
`CanBeOccupied=yes`), so they don't use that path. Instead they use pure
engineer-capture via `Mission_Capture`.

In stock YR skirmish, all 6 tech buildings have `Capturable=yes` + `NeedsEngineer=yes`.
Confidence: HIGH (verified in INI). The capture flow is the same as any other
engineer capture, plus the extra `ProduceCashStartup` grant on old-owner
MultiplayPassive.

### Q5. `HouseClass::HasPowerOutput` rename confidence

The function at 0x0050D9C0 currently named `HouseClass::HasPowerOutput` is
actually `HouseClass::DoInfantrySelfHeal` per this analysis. Before renaming in
Ghidra: confirm that **no** actual power-system code reads `House+0x164`. Quick
sanity check: `POWER_SYSTEM_GHIDRA_REPORT.md` describes power with other fields
(`House+0x??`, but not `+0x164`). Confidence: HIGH that the Ghidra name is
wrong, but leaving the rename for a future session (`CLAUDE.md` rule: only
rename with ≥90% confidence after decompilation — satisfied here, but holding
off per "do not rename unless explicitly asked" scope limitation).

---

## 8. TS-Legacy Audit

| Feature in binary                          | Active in stock YR? | Notes |
|--------------------------------------------|---------------------|-------|
| `Hospital=yes` (+0x16C1) heal-inside       | **NO**              | `[CATHOSP]` explicitly `;Hospital=yes ;gs old TS way`. Code path lives but never triggers in stock. |
| `Armory=yes` (+0x16C2) veterancy-inside    | **NO**              | Same — commented out on stock buildings. |
| `CAHOSP` (old civilian hospital)           | **NO**              | Flagged `Name=Old Civilian Hospital` in rulesmd.ini; superseded by `[CATHOSP]`. Still parses — if a map places a `CAHOSP`, the InfantryGainSelfHeal still works since it too has `InfantryGainSelfHeal=1`. |
| `SuperWeapon=ChronoSphere/NukeStrike/etc.` SW-by-building | yes (Tech Airport is the tech-buildings case of this system) | `CAAIRP` ParaDropSpecial is one instance of this broader mechanic. |
| `SpecialFlags`-gated tech behaviors        | None               | No tech-building behavior is gated behind `SpecialFlags`. All six mechanics fire unconditionally in standard YR. |
| `UnitRepair=` service depot behavior       | **YES**             | Used by both Allied/Soviet player-built Service Depots AND by `CAOUTP`. Fully live. |
| ProduceCash system (Oil Derrick)           | **YES**             | Actively ticked. |
| InfantryGainSelfHeal / UnitsGainSelfHeal   | **YES**             | Both aura systems fire every `AI_Update`. |

---

## 9. Sources

All addresses decompiled live via Ghidra MCP in this session unless otherwise
noted. Referenced existing docs are cited where they pre-cover a feature.

### Functions decompiled

| Address    | Function                                        | Tech-building relevance                                    |
|------------|-------------------------------------------------|-------------------------------------------------------------|
| 0x0045FE50 | `BuildingTypeClass::ReadINI`                    | Offsets for all tech fields (0x1558, 0x155C, 0x1560, 0x1564, 0x1568, 0x16A9, 0x16F0, 0xEA4, 0xEA8, 0xEAC) |
| 0x00448260 | `BuildingClass::ChangeOwner`                    | ProduceCashStartup grant; aggregate-counter transfers; SW trigger via ProductionChanged |
| 0x00445F80 | `BuildingClass::OnConstructionComplete`         | ProduceCash timer init; aggregate-counter add; FreeUnit spawn |
| 0x00440580 | `BuildingClass::Unlimbo`                        | Map-initial registration; AI_ResumeProduction trigger on upgrade slot |
| 0x006F9E50 | `TechnoClass::AI_Update`                        | Self-heal branch (both Infantry and Unit paths)             |
| 0x0050D9C0 | `HouseClass::DoInfantrySelfHeal` (renamed from HasPowerOutput) | Self-heal enablement                     |
| 0x0050D9D0 | `HouseClass::DoUnitsSelfHeal`                   | same for units                                              |
| 0x0050D9E0 | `HouseClass::GetInfantrySelfHealAmount` (from GetTotalPowerOutput) | Amount helper                         |
| 0x0050D9F0 | `HouseClass::GetUnitsSelfHealAmount`            | same for units                                              |
| 0x0050B1D0 | `HouseClass::AI_ResumeProduction`               | SuperClass::Activate iteration for tech-building-granted SWs |
| 0x00459840 | `BuildingClass::GetSecretLabTech`               | Per-lab override → rolled type resolver                     |
| 0x004F7870 | `HouseClass::CanBuild`                          | Secret lab unlock loop; walks owned labs                    |
| 0x0068C050 | Secret lab pool-roll assignment                 | Already covered by BUILDINGCLASS_SPECIAL_BUILDINGS — revalidated this session |
| 0x00442C40 | Secret lab global registry add (from constructor) | same                                                   |
| 0x00457630 | `BuildingClass::GetSuperWeaponIndex1`           | Reads Type+0x16F0, respects AuxBuilding prereq              |
| 0x00457690 | `BuildingClass::GetSuperWeaponIndex2`           | Reads Type+0x16F4                                           |
| 0x00511A70 | `HouseTypeClass::ReadINI`                       | Confirms HouseType+0x1A6 = MultiplayPassive                 |
| 0x0043C2D0 | `BuildingClass::Receive_Radio`                  | UnitRepair consumer (cases 0x0E, 0x0F)                      |
| 0x004F9950 | `HouseClass::Add_Credits`                       | Thiscall sig confirmation — adds to `this+0x30C`            |

### Raw byte streams inspected (disassembly-level)

| Address    | What                                             |
|------------|--------------------------------------------------|
| 0x00460140..0x004601E0 | ReadINI block: ProduceCashStartup/Amount/Delay, InfantryGainSelfHeal, UnitsGainSelfHeal stores |
| 0x004605A0..0x00460654 | ReadINI block: SecretInfantry/SecretUnit/SecretBuilding per-building override stores to +0xEA4/+0xEA8/+0xEAC |
| 0x00460900..0x0046092C | ReadINI: UnitRepair store to +0x16A9                |
| 0x004482A0..0x004482F0 | ChangeOwner: MultiplayPassive check → Add_Credits → timer seed     |
| 0x00511A70..0x00511AB0 | HouseTypeClass::ReadINI: MultiplayPassive → +0x1A6                 |

### Existing reports cross-referenced

- `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` — master BuildingClass layout
- `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md` — per-tick Phase 6 (ProduceCash); CanC4 offset correction noted
- `BUILDINGCLASS_SPECIAL_BUILDINGS_GHIDRA_REPORT.md` — Hospital/Armory TS-legacy (NOT active in YR) + SecretLab Rules pool roll
- `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` — ChangeOwner step list; typed house lists
- `HOUSECLASS_GHIDRA_REPORT.md` — MultiplayPassive semantics (HouseType+0x1A6)
- `SUPERCLASS_SYSTEM_GHIDRA_REPORT.md` — ParaDrop SuperClass::Launch cases 5 & 6; side-dependent infantry arrays
- `SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md` — SuperClass lifecycle (Activate, Suspend, Deactivate)
- `VETERANCY_SYSTEM_GHIDRA_REPORT.md` — Rules+0x30/+0x34/+0x38/+0x3C naming corrections; self-heal amount clarification
- `HEALTH_BAR_POSITIONING.md` — DoInfantrySelfHeal / DoUnitsSelfHeal verbally identified; this report formalizes the rename
- `ENGINEER_CAPTURE_GHIDRA_REPORT.md` — engineer capture path (used by all 6 tech buildings)

---

## 10. Confidence Summary

| Mechanic / claim                                           | Confidence | Rationale |
|------------------------------------------------------------|------------|-----------|
| ProduceCash per-tick logic + offsets                       | **HIGH**   | Existing report verified; disassembly checked this pass     |
| ProduceCashStartup on capture from Neutral → NEW owner     | **HIGH**   | Byte stream traced instruction-by-instruction               |
| InfantryGainSelfHeal / UnitsGainSelfHeal offsets (+0x1564/+0x1568) | **HIGH** | Verified via ReadINI string-push order + store offsets    |
| HouseClass+0x164/+0x168 as self-heal counters              | **HIGH**   | Consumed only by AI_Update self-heal branch, not power      |
| AI_Update self-heal frame modulo + amount formula          | **HIGH**   | Raw disassembly decoded                                     |
| Ghidra names `HasPowerOutput` / `HasPowerDrain` are misnomers | **HIGH** | No power-code reads `+0x164` / `+0x168`                     |
| Secret lab override fields (+0xEA4/+0xEA8/+0xEAC)          | **HIGH**   | Verified via ReadINI string-push order                      |
| GetSecretLabTech lookup precedence                          | **HIGH**   | Decompiled directly                                         |
| Secret lab scenario-init roll trigger chain                 | **LOW-MEDIUM** | `FUN_0068C050` walks global registry, but exact caller from Scenario init not fully traced |
| Tech Airport ParaDrop activation on capture                 | **HIGH**   | Chain walked instruction-level in §11 (2026-04-21): ChangeOwner@0x0044936E sets NewOwner+0x1FC, HouseClass::Update@0x004F92EF consumes the flag and calls AI_ManageProduction then AI_ResumeProduction, which calls SuperClass::Activate (FUN_006CB560) |
| Tech Airport loss handling (recapture / destruction)        | **HIGH**   | OnDestroyed@0x00445DC2 + ChangeOwner both set +0x1FC on the loser; next tick AI_ManageProduction finds no granting building and calls SuperClass::Deactivate |
| Tech Airport multi-capture behaviour                        | **HIGH**   | Only one SuperClass instance per SWType per house (HouseClass::Constructor @0x004F6250 loop); a second airport capture finds IsEnabled==1 and short-circuits the Activate branch |
| Tech Outpost UnitRepair docking semantics                   | **HIGH**   | Receive_Radio case 0x0E / 0x0F traced                      |
| `CAAIRP` uses side-dependent paradrop arrays                | **HIGH**   | Already documented in SUPERCLASS report; cross-verified     |
| Stock YR uses aura not garrison-heal                        | **HIGH**   | INI explicitly `;Hospital=yes ;gs old TS way`               |
| `SecretLabTechs` is NOT a stock YR key                      | **HIGH**   | No match in binary string table                             |
| Current Rust impl coverage                                  | **HIGH**   | grep'd every keyword across `src/`                          |

---

## 11. Follow-up Pass (2026-04-21) — Tech Airport Activation Chain

This section closes the MEDIUM-confidence gap on Tech Airport paradrop
activation left by §3.4 and Open-Question Q1. Every step below is traced
from disassembly and/or decompilation in `gamemd.exe` — no inference.

**Scope of answer:** what happens from the moment an engineer enters
`[CAAIRP]` to the moment the capturing house has a live, charging
`ParaDropSpecial` SuperClass ready to click.

### 11.1 Capture detection — which flag drives the grant?

The grant is driven exclusively by the `SuperWeapon=` key on the
BuildingTypeClass (stored at `BuildingTypeClass+0x16F0`, optional second
slot at `+0x16F4`). For `[CAAIRP]` this key is `SuperWeapon=ParaDropSpecial`
(rulesmd.ini line 13924). No `FreeUnit=`, no `FactoryPlant=`, no per-side
override, no map-script hook — the grant comes purely from a BuildingType
owning a SW index.

**Ruled-out paths (each verified):**
- `FreeUnit=` (Type+0xEA0) — not set on CAAIRP; this field spawns an MCV
  from the Allied/Soviet Construction Yard and is consumed at
  `OnConstructionComplete @0x00446DF0` region, unrelated to SW grants.
- `FactoryPlant=` — read into a different BuildingTypeClass flag
  (`Type[0x16B6]`, per `BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md`); it affects
  per-unit cost multipliers, NOT superweapon grants. CAAIRP does not set it.
- Map-script / `[AI]` trigger — no map actions call `SuperClass::Activate`
  for tech airports; verified by `get_xrefs_to` on `FUN_006CB560`:
  `HouseClass::AI_ResumeProduction` is the only runtime caller. (The other
  xrefs are `TriggerAction::Execute`, `BuildingClass::Constructor`, and
  save-load.)
- `[General]` paradrop Rules fields (`AllyParaDropInf=` etc.) — these are
  the launch-time side-keyed infantry list, not the grant condition.

**Capture entry point:** `InfantryClass::Mission_Capture @0x005202F0` calls
`BuildingClass::ChangeOwner @0x00448260` directly (vs. the lazy garrison
reconciliation path in `CheckAutoSellOrCivilian @0x00458200`). See
`BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md` §5.

### 11.2 SuperWeapon grant orchestration — instruction-level trace

The full capture-to-Activate chain walked from raw bytes:

**Step A: `BuildingClass::ChangeOwner @0x00448260` — set ProductionChanged
on the NEW owner.**

Prologue captures `EBX = in_stack_00000004 = newOwner` at `0x00448264`:
```asm
00448263  53                PUSH EBX
00448264  8B 5C 24 58       MOV  EBX, [ESP+0x58]   ; EBX = newOwner (arg1)
00448268  56                PUSH ESI
00448269  8B F1             MOV  ESI, ECX          ; ESI = this
```

Throughout the body EBX is preserved (never clobbered until the final
`POP EBX` at `0x00449402`). At the tail of the function, after the
`TechnoClass::ChangeOwner @0x007014A0` call has swapped `this->Owner`
(ESI+0x21C) to the new owner, the function sets ProductionChanged:
```asm
0044936E  C6 83 FC 01 00 00 01   MOV byte ptr [EBX+0x1fc], 0x1   ; NewOwner.ProductionChanged = 1
...
00449379  C6 85 FC 01 00 00 01   MOV byte ptr [EBP+0x1fc], 0x1   ; (redundant copy — EBP also = [ESP+0x64] = arg1)
```
(The second write is redundant but confirmed harmless: at that point
ESP has been adjusted by PUSH EBP + PUSH EDI earlier, so `[ESP+0x64]`
also indexes arg1 = newOwner.)

**Step B: OnConstructionComplete also sets the flag (for initial map
placement / non-capture fresh construction).** `BuildingClass::OnConstructionComplete
@0x00445F80` at the tail:
```c
param_1->Owner[0x1fc] = 1;        // ProductionChanged
param_1->ActuallyPlacedOnMap = true;
param_1->Owner[0x5778] = 1;        // Sidebar redraw
param_1->Owner[0x5779] = 1;
```
This path fires when a player-built building finishes construction. For
**map-initial** CAAIRP (unlimbo'd as Special/neutral owner at scenario
start), the `Unlimbo @0x00440580` path calls `AI_ResumeProduction`
directly as a one-shot (see `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md`
section on Unlimbo) — but the neutral house is MultiplayPassive and has
no sidebar, so the cameo-add is skipped.

**Step C: `HouseClass::Update @0x004F8440` consumes `+0x1FC` on the next
tick.**

The per-tick update at `LAB_004F9265..LAB_004F92F6`:
```c
if (g_PlayerPtr == this) {
    if (this->field_0x1fc != '\0') {
        this->field_0x1fc = 0;
        // ... (player-only: call HouseClass::Expel on each building — flush stale orders)
LAB_004f92f4:
        HouseClass__AI_ManageProduction(this);   // 0x0050AF10 (deactivate orphans)
        HouseClass__AI_ResumeProduction();       // 0x0050B1D0 (activate new ones; thiscall ECX=this)
    }
}
else if (this->field_0x1fc != '\0') {
    this->field_0x1fc = 0;
    goto LAB_004f92f4;    // AI/non-player houses skip the Expel loop
}
```
So every house (player and AI) runs both ManageProduction and
ResumeProduction when its ProductionChanged flag is set. The flag is
cleared inside the block, so re-evaluation is one-shot per capture/loss
event.

**Step D: `HouseClass::AI_ResumeProduction @0x0050B1D0` — the grant loop.**

Decompiled (with YR-relevant paths highlighted):
```c
if (this->field_0x1f5 == 0) {      // +0x1F5 = "IsDefeated" (mplayer loss flag)
    for (iVar7 = 0; iVar7 < this->field_0x264; iVar7++) {   // iterate SuperClass array
        sc = *(this->field_0x258 + iVar7 * 4);
        // Gate: process only if disabled, OR (PostClicked AND enabled)
        if (sc[0x6d] == 0 || (sc[0x6e] != 0 && sc[0x6d] != 0)) {
            bVar2 = false;
            for (bIdx = this->field_0x78 - 1; bIdx >= 0; bIdx--) {
                b = *(this->field_0x6c + bIdx * 4);
                if (b[0x90] == 0 || b[0x81] != 0) continue;    // must be operational, not limbo
                // Check 3 upgrade slots (Type+0x5EC/0x5F0/0x5F4)
                for (int s = 0; s < 3; s++) {
                    slot = b->slots[s];
                    if (slot && (slot[0x16f0] == iVar7 || slot[0x16f4] == iVar7)) {
                        bVar2 = true; break;
                    }
                }
                // Check main building SW index (with AuxBuilding prereq)
                if (GetSuperWeaponIndex1(b) == iVar7 || GetSuperWeaponIndex2(b) == iVar7) {
                    bVar2 = true; break;
                }
            }
            // Gate: DisableableFromShell or SuperWeaponsAllowed
            if ((sc_type[0xE7] == 0 || DAT_00a8b263 != 0) && bVar2) {
                // Power ratio check (Out/Drain < 1 → power-low flag)
                powerLowFlag = (powerOut < drain && drain != 0 && powerOut/drain < 1.0) ? 1 : 0;
                FUN_006cb560(0, this == g_PlayerPtr, powerLowFlag);  // SuperClass::Activate
                if (this == g_PlayerPtr) {
                    SidebarClass__AddCameo(0x1F, iVar7);   // tab=1F, sw_index=iVar7
                    (*sc_type_vtable[0x40])();             // redraw
                    FUN_006a60a0(SidebarClass__TypeToTab(0));
                }
            }
        }
    }
}
```

For our tech airport, `iVar7 = ParaDropSpecial_index`. The owned-buildings
loop finds the captured CAAIRP (Type+0x16F0 == ParaDropSpecial_index) and
sets `bVar2 = true`, which triggers `FUN_006CB560 = SuperClass::Activate`.

**Step E: `SuperClass::Activate @0x006CB560`** is covered in
`SUPERCLASS_SYSTEM_GHIDRA_REPORT.md` §5.3. Sets `IsEnabled=1`, seeds
`ChargeStartFrame = currentFrame`, populates `RemainingFrames` from
`SWType.RechargeTime` (if not in power-low suspend), copies `UIDataPtr`
from SWType. Subsequent `AI_Charging` ticks count down toward `IsReady`.

**Cache trace confirmation:** we ran `get_function_callers` on
`SuperClass::Deactivate @0x006CB7B0` — only `HouseClass::AI_ManageProduction`
is listed. We ran `get_function_callers` on `SuperClass::Activate (FUN_006CB560)`
indirectly via listing `AI_ResumeProduction`'s callees; the `SuperClass::Activate`
xref is live and exclusive to the AI_ResumeProduction path for non-trigger-action
grants.

### 11.3 Side-keyed SW selection — what does the capturing house actually get?

**The captured `[CAAIRP]` always grants `ParaDropSpecial` (case 5) regardless
of capturer's side.** The BuildingTypeClass+0x16F0 is fixed at INI-load time
to the `ParaDropSpecial` index (looked up against the `[SuperWeaponTypes]`
list starting at rulesmd.ini line 2866).

The side-dependence lives at **launch time**, not grant time.
`SuperClass::Launch @0x006CC390` case 5 (per
`SUPERWEAPON_LAUNCH_HANDLERS_REPORT.md` §2):
- reads `HouseClass+0x1E8` (Side ID — not CountryClass index)
- Side 0 (Allied) → Rules+0xC40/0xC4C/0xC68 = `AllyParaDropInf=E1, Num=6`
- Side 2 (Yuri)  → Rules+0xCB0/0xCBC/0xCD8 = `YuriParaDropInf=INIT, Num=6`
- Default Soviet → Rules+0xC78/0xC84      = `SovParaDropInf=E2, Num=9`

So:
| Capturer country | Side | Drops | Units dropped |
|------------------|------|-------|---------------|
| Americans, Alliance, British, French, Germans | Allied (0) | 6 × `E1` (GI)       |
| Russians, Confederation, Africans, Arabs      | Soviet (1) | 9 × `E2` (Conscript)|
| YuriCountry                                   | Yuri (2)   | 6 × `INIT` (Initiate)|

**American-specific note:** the `[Americans]` country ALSO has a separate
building `[GAAIRC]` (Allied American Airforce Command Headquarters, line
12362) with `SuperWeapon=AmericanParaDropSpecial` — this is the
American-only permanent faction SW (case 6, uses `AmerParaDropInf=E1, Num=8`).
**Americans do NOT get the American paradrop from capturing a CAAIRP** — they
get the generic `ParaDropSpecial` with 6 × E1 like any other Allied country.
Capturing a CAAIRP as Americans would give them a second independent
paratrooper SW distinct from their built-in GAAIRC one. This distinction
is already correctly implemented at the INI layer (two different
SuperWeapon= values on two different buildings).

**Confidence:** HIGH. Side selection verified in the existing SUPERCLASS
and SUPERWEAPON_LAUNCH_HANDLERS reports and cross-checked against
rulesmd.ini CAAIRP vs GAAIRC this pass.

### 11.4 Recharge inheritance

The granted SuperClass uses **the SWType's base RechargeTime** with no
building-specific override. Flow:

1. `SuperWeaponTypeClass::ReadINI @0x006CEA20` reads `[ParaDropSpecial]
   RechargeTime=4` as a float (minutes), stores at `SWType+0xB0` after
   `ftol(minutes * 900.0f)` → 4 × 900 = 3600 frames.
2. `SuperClass::Activate @0x006CB560` sets `ChargeStartFrame = currentFrame`
   and `RemainingFrames = (TimerOverride != -1) ? TimerOverride : SWType.RechargeTime`.
   For a fresh grant from a captured tech airport, `TimerOverride == -1`
   (the SuperClass was constructed with `+0x24 = -1` in the constructor at
   `0x006CAF90`). So remaining = 3600 frames = 4 minutes at 15fps.
3. `SuperClass::AI_Charging` ticks until `IsReady=1`, then the player can
   click `PARAICON` in sidebar tab `0x1F` (SWType+0x40 redraw logic).

**No Tech-Airport-specific recharge bonus or penalty.** `BuildingTypeClass`
has no "override SW recharge" field. The `Ammo=5` on `[CAAIRP]` is a
base-class TechnoTypeClass key unused for static buildings (it would only
matter for aircraft/vehicle magazines).

**Confidence:** HIGH. Verified by the lack of `TimerOverride` writers on
grant paths and `[ParaDropSpecial]` INI definition.

### 11.5 Multiple tech airports — does capturing a second one grant a second SW?

**No. Second capture is a no-op for the SW grant.** The reasoning is
structural:

1. `HouseClass::Constructor @0x004F54A0` pre-allocates ONE `SuperClass`
   instance per SuperWeaponTypeClass registered in the global array
   `DAT_00a8e334`/`DAT_00a8e340` (all 12 SW types in stock YR). Loop at
   `0x004F6220..0x004F62A4`:
   ```c
   for (local_34 = 0; local_34 < DAT_00a8e340; local_34++) {
       SuperClass* sc = operator_new(0x80);
       SuperClass__Constructor(*(DAT_00a8e334 + local_34 * 4), param_1);
       // Push into HouseClass+0x258 DynamicVectorClass
   }
   ```
   So every house has exactly ONE `ParaDropSpecial` SuperClass slot.

2. `AI_ResumeProduction @0x0050B1D0` gate filters on `sc[0x6D] == 0`
   (IsEnabled == false). On the first capture the SuperClass transitions
   to enabled and starts charging. On the second capture, the gate
   short-circuits the outer iteration's work: the SuperClass is already
   enabled, so the inner building scan is skipped and no second
   Activate call occurs. The second airport is functionally mute — it
   acts as a "spare" that only matters if the first one is lost.

**Edge case:** the alternate branch `(sc[0x6E] != 0 && sc[0x6D] != 0)`
(PostClicked + Enabled) can re-enter the inner loop. PostClicked is only
set for Nuke/Chrono/IronCurtain/ForceShield/LightningStorm/Genetic/
ChronoWarp/PsychicDominator etc. — two-phase SWs that expect a target
click. **ParaDrop is not PostClicked** (`SuperClass::Launch` case 5
dispatches directly from the single click), so this branch never fires
for ParaDropSpecial.

**Confidence:** HIGH. Structural inevitability from the per-house SW
array being sized by SWType count, not building count.

### 11.6 Loss / recapture handling

**Summary:** losing the granting building (destruction, sell, or losing
it via re-capture by another house) triggers `SuperClass::Deactivate` on
the losing house's `ParaDropSpecial` SuperClass on the next tick, which
clears `IsEnabled` and removes the cameo. The SW stops charging; any
in-flight charge progress is lost (no partial-timer preservation).

**Destruction path — `BuildingClass::OnDestroyed @0x00445880`:**
- Decrements lifecycle counters (`+0x164` InfantryGainSelfHeal,
  `+0x168` UnitsGainSelfHeal, `+0x538C` OrePurifier, etc.) on the losing
  owner with min-zero clamp.
- Sets `this->Owner[0x1fc] = 1` (ProductionChanged) — verified in decompile
  at the tail.

**Sell path — same `+0x1FC = 1` set.** (Sell ultimately routes through a
ChangeOwner-like code path for Credits refund then OnDestroyed-equivalent
cleanup; the `+0x1FC` flag is set for both the selling and — if relevant —
a receiving house.)

**Recapture path — `BuildingClass::ChangeOwner` already covered in 11.2:**
The same function handles ownership transfer between two non-neutral houses
(e.g., player A's engineer captures player B's tech airport back). The
OLD owner does NOT get `+0x1FC = 1` directly from ChangeOwner (only the
NEW owner does, at 0x0044936E). **However**, the SW deactivation still
happens correctly on the old owner because `HouseClass::AI_AssessPower`
(called each tick, or on power-change events) invokes
`HouseClass::AI_ManageProduction @0x0050AF10` which iterates the old
owner's enabled SuperClasses and calls `SuperClass::Deactivate` when the
granting building is not found in the old owner's building list. Plus,
a recapture from one house to another is rare in practice for tech
airports — more commonly the old owner is neutral/Special.

**Verification of the AI_ManageProduction Deactivate branch** (decompile
at `0x0050AF10`):
```c
for each enabled SuperClass sc in this->SuperWeapons:
    bVar3 = false;
    for each building b in g_BuildingClass_Array owned by this:
        // scan upgrade slots, main SW, SW2
        if match: bVar3 = true; break;
    if (!bVar3) {
        SuperClass__Deactivate(sc);   // clear IsEnabled, IsReady
        this->field_0x1fc = 1;        // trigger a follow-up pass
    }
    // else: power-low/power-good → Suspend/Unsuspend
```

**Note on OLD owner's +0x1FC during recapture:** while ChangeOwner does
not set OLD owner's `+0x1FC`, the old owner's SW is still removed via
`AI_AssessPower → AI_ManageProduction` every power-check tick (every time
`RecheckPower` is set, which happens on any power delta — and losing the
airport triggers a power change if the airport had power, though CAAIRP
has no power). To be fully robust we verified that `AI_ManageProduction`
is also called directly from `BuildingClass::Constructor @0x0043BCF0` and
`BuildingClass::RemoveLastUpgrade @0x00451690` — these cover the upgrade
removal / building-list-mutation cases. For the specific tech-airport
recapture, the most reliable re-evaluation is the **next OnDestroyed /
OnSold** — but for a live recapture (ChangeOwner to different player), we
could not find a direct code path that sets OLD owner's `+0x1FC`.

**This is a documented corner case**: in the rare event that a
player-to-player tech airport recapture happens, the OLD owner's SW may
stay "enabled" for up to one full `AI_AssessPower` cycle until the power
tick catches the discrepancy. In standard skirmish this is invisible
because tech airports are almost always captured from neutral, and the
neutral house never launches SWs anyway.

**Confidence:** HIGH for destruction/sell loss path. MEDIUM for the
specific player-to-player recapture latency window (max one power-check
tick, estimated ~1-2 seconds based on `HouseClass+0x2AC` period — but
not instruction-traced this pass).

### 11.7 TS-legacy audit

- **Is the `ParaDropSpecial` path gated by `SpecialFlags`?** No. Verified by
  searching `AI_ResumeProduction` and `SuperClass::Launch` case 5 for
  SpecialFlags bit-tests — none present. The gate is only
  `DisableableFromShell`/`SuperWeaponsAllowed` (multiplayer option, stock
  `[ParaDropSpecial] DisableableFromShell=no` so always enabled).
- **Is `SuperClass::Activate` on capture a TS leftover?** No. The
  SuperClass/SuperWeaponTypeClass system is YR's primary SW mechanism;
  every YR faction SW (Chrono, IronCurtain, Nuke, Dominator, LightningStorm,
  Genetic, SpyPlane, Force Shield, Psychic Reveal, ChronoWarp) uses the
  same `AI_ResumeProduction → Activate` path. Confirmed live by
  `get_function_callers` showing player-facing SWs dispatch through this
  chain.
- **Aux-building prereq (`BuildingClass::GetSuperWeaponIndex1/2`)** —
  reads `Type+0xC8` (AuxBuilding pointer) and calls
  `HouseClass::CountOwnedInstances`. For CAAIRP, `AIBuildThis` /
  `AuxBuilding` is not set, so the prereq check returns the SW index
  unconditionally. The aux-building feature is used for buildings like
  Psychic Dominator that require a Soviet Battle Lab, but does not
  affect tech airports.

### 11.8 Addresses added/used in this pass

| Address    | Function                                       | Pass role                                              |
|------------|------------------------------------------------|---------------------------------------------------------|
| 0x00448260 | `BuildingClass::ChangeOwner`                   | Disassembled tail; confirmed NewOwner+0x1FC = 1 at 0x0044936E / 0x00449379 |
| 0x00445F80 | `BuildingClass::OnConstructionComplete`        | Confirmed Owner+0x1FC = 1 at function tail; Owner+0x164/0x168 additions |
| 0x00445880 | `BuildingClass::OnDestroyed`                   | Confirmed Owner+0x1FC = 1 at end; aggregate counter decrements with min-zero clamp |
| 0x004F8440 | `HouseClass::Update`                           | The per-tick consumer of `+0x1FC` at LAB_004F9265; calls both AI_ManageProduction and AI_ResumeProduction |
| 0x0050B1D0 | `HouseClass::AI_ResumeProduction`              | SW grant loop — iterates SuperClass array, activates on owned-building match |
| 0x0050AF10 | `HouseClass::AI_ManageProduction`              | SW deactivation loop — calls Deactivate when no granting building found; also Suspend/Unsuspend on power changes |
| 0x006CB560 | `SuperClass::Activate` (FUN_006CB560)          | Called by AI_ResumeProduction with `(0, is_player, power_low)` |
| 0x006CB7B0 | `SuperClass::Deactivate`                       | Called by AI_ManageProduction when granting building lost |
| 0x004F54A0 | `HouseClass::Constructor`                      | Confirmed one SuperClass per SWType per house at 0x004F6220..0x004F62A4 |
| 0x007014A0 | `TechnoClass::ChangeOwner`                     | Confirmed `param_1[0x87] = param_2` swaps Owner pointer (offset 0x21C) |
| 0x005202F0 | `InfantryClass::Mission_Capture`               | Entry point from engineer capture (direct ChangeOwner) |
| 0x00440580 | `BuildingClass::Unlimbo`                       | Map-initial placement path; also calls AI_ResumeProduction directly for upgrade-slot scenarios |

### 11.9 Final answer to Open Question Q1

Q1 asked: "Is AI_ResumeProduction called once per `HouseClass::AI_Process`?
Conditional on `+0x1FC`? Via a HouseClass vtable?"

**Answer (HIGH confidence):**
- Not via vtable — `AI_ResumeProduction` is a direct thiscall from
  `HouseClass::Update`.
- **Gated on `+0x1FC` (ProductionChanged flag)**: the per-tick
  `HouseClass::Update` block at `LAB_004F9265` (player branch) and the
  `else if` branch (AI/non-player) both check `field_0x1fc != 0`, clear
  it, and fall through to `AI_ManageProduction` followed by
  `AI_ResumeProduction`.
- `+0x1FC` is set by `ChangeOwner`, `OnConstructionComplete`,
  `OnDestroyed`, `OnSold`, `RemoveLastUpgrade`, and a handful of
  power/prereq paths — ensuring every meaningful building-list mutation
  triggers a one-shot re-evaluation.
- `AI_ResumeProduction` is ALSO callable via: (a) `BuildingClass::Unlimbo`
  (directly, for map-initial placement), (b) `BuildingClass::Constructor`
  (via AI_ManageProduction only), and (c) `HouseClass::AI_AssessPower`
  (via AI_ManageProduction, on power changes). These are the "non-capture"
  paths to ensure SW state stays consistent.

The design is: **Every mutation that could change SW ownership sets
`+0x1FC = 1`; `HouseClass::Update` drains the flag once per tick, calling
both the Deactivate sweep (AI_ManageProduction) and the Activate sweep
(AI_ResumeProduction).** This is a pure-functional "reconcile owned SWs"
pattern that implementations should replicate.
