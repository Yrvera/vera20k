# TechnoClass Systems — Ghidra Research Report
# Veterancy, Cloaking, Iron Curtain, Temporal, Mind Control, EMP, Transport, Storage

Reverse-engineered from `gamemd.exe`. Confidence: **HIGH** — all offsets verified from binary
decompilation, constructor initialization, and INI string cross-references.

**Note:** Mind control, cloaking visual pipeline, sensor detection, and radiation/EMP already
have dedicated reports. This document covers the remaining systems comprehensively and
consolidates field maps into a single reference.

---

## 1. Veterancy / Experience / Promotion System

### 1.1 VeterancyStruct

The veterancy data is a **single float** stored in each TechnoClass instance.

| TechnoClass Offset | Type | Field | Description |
|---------------------|------|-------|-------------|
| +0x150 | float | Veterancy | Current experience level (0.0 = rookie, 1.0 = veteran, 2.0 = elite) |

**Note:** `param_1` in TechnoClass constructor is `int*`, so `param_1[0x54]` = byte offset 0x150.
The constructor calls `FUN_0074ff30()` which initializes this field (sets to 0.0).

### 1.2 Promotion Thresholds (hardcoded constants in .rdata)

| Address | Float Value | Rank |
|---------|-------------|------|
| 0x007E2AC8 | 1.0f (0x3F800000) | Veteran threshold |
| 0x007E37B4 | 2.0f (0x40000000) | Elite threshold |

**VeterancyClass::IsVeteran** (0x0074FF90):
```c
bool IsVeteran(float *veterancy) {
    return (*veterancy >= 1.0f && *veterancy < 2.0f);
}
```

**VeterancyClass::IsElite** (0x00750010):
```c
bool IsElite(float *veterancy) {
    return (*veterancy >= 2.0f);
}
```

**VeterancyStruct::SetVeteran** (0x00750090): Sets value to 1.0f (0x3F800000) if true, 0.0 if false.
**VeterancyStruct::SetElite** (0x007500B0): Sets value to 2.0f (0x40000000) if true, 0.0 if false.

### 1.3 Experience Gain (FUN_0074FF50)

**Address:** 0x0074FF50

```c
void AddExperience(float *veterancy, int victim_cost, int xp_amount) {
    float gained = (float)xp_amount / ((float)victim_cost * RulesClass->VeteranRatio);
    *veterancy += gained;
    // Binary uses >=, not >: `if (VeteranCap <= fVar1)` (verified via decompile_function 0x0074FF50)
    if (RulesClass->VeteranCap <= *veterancy) {
        *veterancy = RulesClass->VeteranCap;
    }
}
```

Called from `TechnoClass::RecordKill` (0x00702D40) and `TemporalClass::Update` (0x0071A760)
when the target is destroyed.

### 1.4 Kill Credits and XP Multipliers (TechnoClass::RecordKill @ 0x00702D40)

When a unit kills another:
1. Gets the victim's cost via `TechnoTypeClass::GetCost()` (vtable+0x84 chain)
2. **XP multiplier based on KILLER's rank:**
   - Rookie: xp = base_cost (1x)
   - Veteran: xp = base_cost * 2
   - Elite: xp = base_cost * 3
3. **Allied kills give 0 XP** (checked via `HouseClass::IsAlly`)
4. **DontScore units** (TechnoTypeClass + 0xC9F) give no XP at all

**Kill credit routing** (who gets the XP):
- If the victim is a spawned unit (`victim + 0x82 != 0` and `victim[0x47] != 0`),
  and the spawn manager's owner is `Trainable`, XP goes to the **spawn manager owner**
- If the killer itself is `Trainable` (TechnoTypeClass + 0xC8E), XP goes directly to killer
- If the killer is a **passenger** (`Passengers` flag, TechnoTypeClass + 0xD68), and the
  transport (`victim[0xB5]`) is `Trainable`, XP goes to the **transport**
- If the killer is an aircraft-type (WhatAmI == 6) and is an active spawned aircraft,
  XP goes to the **carrier/spawn manager**

**HouseClass statistics updated:**
- `HouseClass + 0x548C`: last killed house index
- `HouseClass + 0x54E8`: total cost of units killed (accumulates xp_amount)
- `HouseClass + 0x5434`: total units destroyed count
- `HouseClass + 0x53E4 + houseIndex * 4`: per-house units destroyed array
- `HouseClass + 0x5438 + houseIndex * 4`: per-house buildings destroyed array

### 1.5 RulesClass Veterancy Settings ([General] section)

All stored as `double` in RulesClass. Read in `RulesClass::ReadGeneral` (0x0066D530+;
corrected 2026-07-18: was `0x0066BBB0` — that address is a different function, currently
labeled `RulesClass__ReadCombatDamage` by Ghidra (reads Drain/cloak-adjacent fields, not
veterancy); the actual `RulesClass::ReadGeneral` entry point was cross-confirmed via two
independent string xrefs landing inside it — `VeteranRatio` (0083c9d4, xref at 0066eea3)
and `SelfHealInfantryFrames`/`SelfHealUnitFrames`/`SelfHealUnitAmount` (xrefs at
0066e6eb/0066e71f/0066e738) — all resolve to the function at `0x0066D530` via
`get_function_by_address`; the offset table below is unaffected, only the function
address citation was wrong — RTTI_LABEL_DRIFT).

| INI Key | RulesClass Offset | Type | Default | Description |
|---------|-------------------|------|---------|-------------|
| `VeteranRatio` | +0x668 | double | 3.0 | Kill cost multiple of self-value to gain 1 level |
| `VeteranCombat` | +0x670 | double | 1.1 | Veteran firepower multiplier |
| `VeteranSpeed` | +0x678 | double | 1.2 | Veteran speed multiplier |
| `VeteranSight` | +0x680 | double | (parsed) | Veteran sight range multiplier |
| `VeteranArmor` | +0x688 | double | 1.5 | Veteran armor multiplier (damage divisor) |
| `VeteranROF` | +0x690 | double | 0.6 | Veteran rate-of-fire multiplier (lower = faster) |
| `VeteranCap` | +0x698 | double | (parsed) | Maximum veterancy value (caps experience) |

### 1.6 VeteranAbilities / EliteAbilities (TechnoTypeClass)

Parsed in `TechnoTypeClass::ReadINI` (0x00712170) at around 0x007154A3.

Each is a comma-separated list of ability names. Stored as boolean arrays:

| TechnoTypeClass Offset | Size | Field |
|------------------------|------|-------|
| +0x29C | BYTE[18] | VeteranAbilities (boolean flags) |
| +0x2AE | BYTE[18] | EliteAbilities (boolean flags) |

**Ability Enum** (string table at 0x008463B8, parser at 0x00477640):

| Index | Name | Vet Offset | Elite Offset | Effect |
|-------|------|------------|--------------|--------|
| 0 | FASTER | +0x29C | +0x2AE | Speed bonus |
| 1 | STRONGER | +0x29D | +0x2AF | Armor bonus |
| 2 | FIREPOWER | +0x29E | +0x2B0 | Damage bonus |
| 3 | SCATTER | +0x29F | +0x2B1 | Auto-scatter from threats |
| 4 | ROF | +0x2A0 | +0x2B2 | Rate of fire bonus |
| 5 | SIGHT | +0x2A1 | +0x2B3 | Increased vision range |
| 6 | CLOAK | +0x2A2 | +0x2B4 | Gains cloaking ability |
| 7 | TIBERIUM_PROOF | +0x2A3 | +0x2B5 | Immune to tiberium damage |
| 8 | VEIN_PROOF | +0x2A4 | +0x2B6 | Immune to vein damage |
| 9 | SELF_HEAL | +0x2A5 | +0x2B7 | Gradual HP regeneration |
| 10 | EXPLODES | +0x2A6 | +0x2B8 | Explodes on death |
| 11 | RADAR_INVISIBLE | +0x2A7 | +0x2B9 | Hidden from radar |
| 12 | SENSORS | +0x2A8 | +0x2BA | Gains sensor ability |
| 13 | FEARLESS | +0x2A9 | +0x2BB | Immune to morale effects |
| 14 | C4 | +0x2AA | +0x2BC | Gains C4 demolition ability |
| 15 | TIBERIUM_HEAL | +0x2AB | +0x2BD | Heals from tiberium exposure |
| 16 | GUARD_AREA | +0x2AC | +0x2BE | Enhanced guard behavior |
| 17 | CRUSHER | +0x2AD | +0x2BF | Can crush infantry |

**HasWeaponAbility** (0x0070D0D0) — checks if a unit has a specific ability at its current rank:
```c
bool HasWeaponAbility(TechnoClass *this, int ability_index) {
    if (!IsVeteran() && !IsElite()) return false;
    int typeClass = this->GetType();
    if (IsVeteran() && *(char*)(typeClass + 0x29C + ability_index)) return true;
    if (IsElite() && (*(char*)(typeClass + 0x29C + ability_index) ||
                       *(char*)(typeClass + 0x2AE + ability_index))) return true;
    return false;
}
```

**Key observation:** Elite units inherit ALL veteran abilities plus their own elite abilities.
The check is cumulative, not replacement.

---

## 2. Cloaking / Stealth System

(See also: CLOAKING_VISUAL_PIPELINE.md, CLOAKING_INTERACTIONS_REPORT.md, SENSOR_CLOAK_DETECTION.md)

### 2.1 TechnoClass Cloaking Fields

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x220 | DWORD | CloakState | 0=Uncloaked, 1=Cloaking, 2=Cloaked, 3=Uncloaking |
| +0x224 | DWORD | CloakProgress | Animation progress counter |
| +0x228 | BYTE | CloakDirty | Set when CloakProgress changes |
| +0x22C | CDTimer(12) | CloakStepTimer | Controls tick rate for CloakProgress advancement |
| +0x238 | DWORD | CloakingSpeed | Copied from TechnoTypeClass+0x310 |
| +0x23C | DWORD | CloakStepDelta | +1 when cloaking, -1 when uncloaking |
| +0x240 | CDTimer start | SecondaryCloakGateTimer.start | Secondary CDTimer gate checked in `CanAutoCloak` (`param_1[0x90]`); distinct from ReCloakDelayTimer at +0x2EC/+0x2F4; verified via `decompile_function 0x006FBDC0` |
| +0x248 | CDTimer dur | SecondaryCloakGateTimer.duration | Duration field for secondary cloak gate timer (`param_1[0x92]`); timer must have expired for `CanAutoCloak` to return true; verified via `decompile_function 0x006FBDC0` |
| +0x2EC | CDTimer start | ReCloakDelayTimer.start | Recloak delay timer start frame (`param_1[0xBB]`); verified via `decompile_function 0x006FBDC0` |
| +0x2F4 | CDTimer dur | ReCloakDelayTimer.duration | Recloak delay timer duration (`param_1[0xBD]`); verified via `decompile_function 0x006FBDC0` |
| +0x269 | BYTE | CloakShroudActive | Gap generator shroud active flag |
| +0x26C | DWORD | CloakShroudRadius | Cached gap radius |
| +0x3D2 | BYTE | HasStealthAbility | Runtime cloak flag |

### 2.2 Cloak/Decloak Triggers

**StartCloaking** (0x00703770):
- Only from state 0 (Uncloaked) or 3 (Uncloaking)
- Sets state to 1 (Cloaking), progress to 0
- Copies CloakingSpeed from TechnoTypeClass+0x310
- Plays CloakSound
- If not owned by player, removes from screen

**StartUncloaking** (0x007036C0):
- Transitions to state 3 (Uncloaking)
- Plays uncloak sound

**CloakingTick** (0x006FB740) — called every game tick:
- State 0 (Uncloaked): checks CanAutoCloak conditions, if met starts recloak timer
- State 1 (Cloaking): advances CloakProgress until fully cloaked (state 2)
  - At 10% health or below, 10% random chance per tick to abort cloaking
- State 2 (Cloaked): checks ShouldUncloak conditions
  - If health < ConditionYellow (RulesClass+0x1708), 4% random chance to decloak
- State 3 (Uncloaking): runs uncloak animation down to state 0
  - Then handles detection/reveal of nearby units under cloak shroud

### 2.3 CanAutoCloak (0x006FBDC0)

Returns true if the unit can re-enter cloak. Checks (verified via `decompile_function 0x006FBDC0`):
1. Has cloaking ability (Cloakable flag or veteran/elite CLOAK ability)
2. CloakState != 2 (`param_1[0x88] == 2`, byte offset +0x220 = CloakState field; not "mission state" — this is "already fully cloaked", return false immediately)
3. Recloak delay timer has expired (CDTimer at `param_1[0xBB]`/`param_1[0xBD]`, byte offsets **+0x2EC/+0x2F4** — NOT +0x240; see §2.1 note below)
4. Not currently targeting something in certain conditions
5. Secondary cloak gate timer at `param_1[0x90]`/`param_1[0x92]` (byte offsets **+0x240/+0x248**) has expired — this is a **separate CDTimer gate** distinct from the ReCloakDelayTimer at +0x2EC/+0x2F4; both must have expired for `CanAutoCloak` to return true; verified via `decompile_function 0x006FBDC0`
6. Not a building that is in "offline" mode
7. GetDamageState() < 1 (not heavily damaged or dead)

### 2.4 ShouldUncloak (0x006FBC90)

Returns true (= should NOT auto-recloak) when:
1. Unit is NOT cloakable AND NOT has stealth ability from veteran/elite
2. Unit's cell is NOT visible to its own house (enemy can't see it anyway)

Returns false (= OK to auto-cloak) when:
- Unit has native cloakable ability or veteran/elite CLOAK
- Cell is visible to own house

### 2.5 Key INI Keys

| Object | INI Key | Offset | Description |
|--------|---------|--------|-------------|
| TechnoTypeClass | `Cloakable` | +0xCD0 | Main cloakable flag |
| TechnoTypeClass | `CloakingSpeed` | +0x310 | Frames between cloak steps |
| TechnoTypeClass | `CloakStop` | +0xC93 | Can't cloak while moving |
| WeaponTypeClass | `DecloakToFire` | +0x133 | Must uncloak to fire |
| RulesClass | `CloakingStages` | +0x628 | Number of cloak visual stages (default: 9) |
| RulesClass | `CloakDelay` | +0x1410 | Delay before auto-recloak (minutes) |

---

## 3. Iron Curtain / Force Shield

### 3.1 TechnoClass Iron Curtain Fields

From `TechnoClass::IronCurtain` (0x0070E2B0) and `IsIronCurtainActive` (0x0041BF40):

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x18C | CDTimer start | IronCurtainStartFrame | Frame when IC was applied |
| +0x190 | DWORD | (uninitialized) | corrected 2026-07-12: NOT meaningful timer state. `TechnoClass::IronCurtain` (0x0070E2B0) writes this field from an **uninitialized local stack slot** (`SUB ESP,0xC` reserves scratch space that is never written before `MOV EAX,[ESP+0x8]` / `MOV [ESI+0x4],EAX` copies it to +0x190) — verified via `disassemble_function 0x0070E2B0`. The constructor (`disassemble_function 0x006F2B40`) never initializes +0x190 either. `IsIronCurtainActive` (0x0041BF40) only reads +0x18C/+0x194, never +0x190, so this garbage value is never consumed — INFERENCE_HARDENED |
| +0x194 | DWORD | IronCurtainDuration | Duration in frames |
| +0x1A4 | DWORD | (unknown) | Set to 0 by IronCurtain |
| +0x1C4 | DWORD | IsForceShield | 1 if applied by Force Shield, 0 if Iron Curtain |

### 3.2 TechnoClass::IronCurtain (0x0070E2B0)

```c
void TechnoClass::IronCurtain(int duration, int source_house, int is_force_shield) {
    this->IronCurtainStartFrame = g_CurrentFrameCounter;  // +0x18C
    // +0x190 is written from an uninitialized stack slot, not a real value (corrected 2026-07-12,
    // verified via disassemble_function 0x0070E2B0 — see §3.1 note)
    this->field_0x1A4 = 0;
    this->IronCurtainDuration = duration;                  // +0x194
    if (is_force_shield) {
        this->IsForceShield = 1;                           // +0x1C4
    } else {
        this->IsForceShield = 0;
    }
}
```

### 3.3 IsIronCurtainActive (0x0041BF40)

```c
bool TechnoClass::IsIronCurtainActive() {
    int duration = this->IronCurtainDuration;  // +0x194
    if (this->IronCurtainStartFrame != -1) {   // +0x18C
        int elapsed = g_CurrentFrameCounter - this->IronCurtainStartFrame;
        if (elapsed < duration) {
            duration = duration - elapsed;
        } else {
            duration = 0;
        }
    }
    return duration > 0;
}
```

Standard CDTimer pattern: start frame + duration, elapsed check.

### 3.4 InfantryClass::IronCurtain Override (0x00522600)

**Infantry are KILLED by Iron Curtain, not protected!**
```c
void InfantryClass::IronCurtain(int duration, int source_house) {
    int maxHealth = this->GetType()->Strength;  // TechnoTypeClass+0xA0
    this->ReceiveDamage(&maxHealth, 0, RulesClass->C4Warhead, 0, true, 0, source_house);
}
```
This matches the original game behavior: Iron Curtain kills infantry.

### 3.5 BuildingClass::IronCurtain Override (0x00457C90)

Clears the building's sell animation state if selling (`+0x6DF` flag), then calls
the base `TechnoClass::IronCurtain`.

### 3.6 RulesClass Iron Curtain / Force Shield Settings

| INI Key | RulesClass Offset | Type | Description |
|---------|-------------------|------|-------------|
| `IronCurtainDuration` | (read in AudioVisual) | int | Default IC duration in frames |
| `ForceShieldDuration` | (read in ReadGeneral ~0x6710BE) | int | Force shield duration |
| `ForceShieldBlackoutDuration` | (read in ReadGeneral ~0x6710DE) | int | Blackout after FS |
| `ForceShieldRadius` | (read in ReadGeneral ~0x67109F) | int | Radius of FS effect |

### 3.7 TechnoTypeClass

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `ImmuneToPsionics` | +0xD35 | bool | Immune to mind control |
| `ImmuneToPsionicWeapons` | +0xD36 | bool | Immune to psionic weapon fire |
| `ImmuneToRadiation` | +0xD37 | bool | Immune to radiation damage |

---

## 4. Temporal / Chrono Warp Effects

### 4.1 TemporalClass Struct Layout

**Size: 0x54 bytes** (from constructor field initialization pattern).

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00-0x0F | ptr[4] | vtables | Primary + 3 secondary vtables |
| 0x10-0x23 | | AbstractClass base | Inherited fields |
| 0x24 | ptr | Owner | TechnoClass* that owns this temporal weapon |
| 0x28 | ptr | Target | TechnoClass* currently being warped |
| 0x2C | CDTimer | WarpTimer | Timer fields (start frame) |
| 0x30 | CDTimer | WarpTimer aux | |
| 0x34 | CDTimer | WarpTimer duration | |
| 0x38 | ptr | LinkedListPrev? | For chained temporal attacks |
| 0x3C | ptr | SuperWeaponRef | SuperClass reference for building targets |
| 0x40 | ptr | LinkedListNext? | Next in temporal chain |
| 0x44 | ptr | LinkedListAlt? | Alternative linked list pointer |
| 0x48 | int | TargetHP | Remaining HP before warp kill |
| 0x4C | int | DamagePerTick | Last damage applied per tick |

### 4.2 TemporalClass::InitiateWarp (0x0071AF20)

When a temporal weapon fires at a target:
1. Kills all spawned units on the target (`SpawnManager->KillAll`)
2. Frees all mind-controlled units (`CaptureManager->FreeAll`)
3. If already warping something, detaches first
4. Checks `CanWarpTarget()` — target must be valid and not already at head of a warp chain
5. Sets `target->IsWarpingOut` (+0x270) = 1
6. Calculates initial HP pool: `target->GetType()->Strength * 10` (at offset +0x48)
7. If target is a building, marks the building as under temporal attack and stops cloaking
8. For chained temporals (multiple chrono legionnaires on same target):
   - Links into the existing temporal chain via linked list pointers at +0x38/+0x40/+0x44
9. Plays EVA notification if target is player's building

### 4.3 TemporalClass::Update (0x0071A760)

Called every tick while a warp is active (verified via `decompile_function 0x0071A760`):
1. **Chain-orphan cleanup** (early-return guard): if `target != null AND target[0x9E] == this AND (this+0x40) != 0`, clear WarpingOut on target and call `ClearLinkedList`, then return.
   - **Doc was wrong on both counts**: the original said "target[0x9E] != this → detach" — the condition is inverted (`==`, not `!=`), and the semantics differ. `this+0x40` is the `LinkedListNext` pointer; a non-null value here means this temporal has a chain successor. The combination (`target still records this temporal AND this temporal has a successor`) indicates a stale chain link — the chain head has moved forward but the target field still references this node. This is chain-orphan cleanup, not rival-temporal detection.
   - **Open question**: the exact lifecycle scenario that leaves the chain in this state (e.g., a second temporal taking over as chain head mid-warp) is not fully traced. The guard fires before the distance check and short-circuits the entire tick.
2. Distance check: if owner and target are further than `RulesClass+0xF60 * 0x100` leptons, call `DetachFromTarget` and return (`* 0x100` = `* 256`; both notations are equivalent)
3. Calculates chain damage: sums damage from all linked temporals (via `+0x44` chain pointer, if non-null)
4. Gets weapon damage: `Owner->GetWeapon()->Damage` (WeaponTypeClass +0xA4)
5. Subtracts `(weapon_damage + chain_damage)` from TargetHP (+0x48) each tick
6. When TargetHP reaches 0:
   - Creates warp-out animation (`RulesClass+0x340`)
   - For buildings (WhatAmI == 6): spawns parachuting survivors **only if occupant count > 0** (`(*piVar2+0x408)() > 0`), undocks any docked unit, removes building
   - For non-buildings: removes the unit
   - Awards experience to the temporal weapon owner (if Trainable, `TechnoTypeClass+0xC8E`)
   - Clears all temporal chain pointers

### 4.4 TechnoClass Temporal Fields

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x270 | BYTE | IsWarpingOut | Set to 1 when temporal warp begins |
| +0x271 | BYTE | IsWarpingIn | Set by TeleportLocomotionClass on chrono arrival |
| +0x274 | ptr | TemporalTargetingMe | TemporalClass* that is currently warping this unit |
| +0x278 | ptr | TemporalChainHead | Head of temporal chain targeting this unit |

### 4.5 Temporal Visual State Machine (0x0070E5A0)

TechnoClass::UpdateTemporalVisual runs a 10-state visual animation:
- State 0: Start, delay 6 frames
- State 1: Phase 1, delay 4 frames
- State 2: Phase 2, delay 15-25 frames (random +-5)
- State 3: Phase 3, delay 8 frames
- State 4: Phase 4, delay 16 frames
- State 5: Check if warp almost complete (< 0x36 remaining), loop to state 4 if not
- State 6: Transition when < 0x1F remaining
- State 7: delay 6 frames
- State 8: delay 4 frames
- State 9: delay 20 frames
- State 10+: Final state (warp complete visual)

Field at +0x1A4 (param_1[0x69]) stores the current visual state.

---

## 5. Mind Control

(See: MIND_CONTROL_GHIDRA_REPORT.md for comprehensive details)

### 5.1 TechnoClass Mind Control Fields

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x2BC | ptr | CaptureManager | CaptureManagerClass* (null if not an MC unit) |
| +0x2C0 | ptr | MindControlledBy | TechnoClass* — who controls this unit (victim field) |
| +0x2C4 | BOOL | IsMindControlled | Flag set on captured units |
| +0x2C8 | ptr | MindControlAnim | AnimClass* — MC ring anim on victim |

### 5.2 Ownership Change (TechnoClass::ChangeOwner @ 0x007014A0)

Full ownership transfer flow:
1. If old owner is player, remove from screen
2. Clear all targets and orders
3. Kill all spawned units (SpawnManager)
4. Remove from old house tracking arrays
5. Transfer cost value to new house's `TotalKilledValue` (+0x54E8)
6. Add to new house tracking arrays (unit count by type)
7. Set `this->Owner` (offset +0x21C) = new_house
8. Set `IsDiscoveredByPlayer` (+0x41A) based on new house
9. Re-acquire targets for new owner
10. If building entering dock, preserve dock state

### 5.3 Key Immunity Checks

**IsMindControlled** (0x007105E0):
```c
bool TechnoClass::IsMindControlled() {
    return (this->MindControlledBy != 0) || (this->IsMindControlledFlag != 0);
}
```

**FreeAllMindControlCaptures** (0x00710460):
```c
void TechnoClass::FreeAllMindControlCaptures() {
    if (this->CaptureManager != 0) {
        CaptureManagerClass::FreeAll(this->CaptureManager);
    }
}
```

---

## 6. EMP Effects

(See: RADIATION_EMP_GHIDRA_REPORT.md for comprehensive details)

### 6.1 TechnoClass EMP Field

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| +0x504 | int | EMPLockRemaining | Frames remaining under EMP. Decremented in AI_Update. |

### 6.2 EMP Application (FootClass::ReceiveEMP @ 0x004DEBB0)

1. Check `GetDamageState() >= 1` — only apply to units that can be affected
2. If unit has parasites (param_1[0x1B] > 0), triggers parasite exit events
3. Sets `+0x425` = 1 (EMP flag)
4. Calls `RemoveFromRadar()` (vtable+0x274)
5. Stops locomotor (vtable+0x3A0)
6. Recursively EMPs all passengers (`FootClass::EMPPassengers` @ 0x00707CB0)
7. Sets random rocking angles for visual wobble
8. Calls `Detach_From_All_Lists` (0x007258D0) (corrected 2026-07-12: this is NOT a sparkle-effect creator — it is a generic list-detachment utility called from ~50+ unrelated constructors/destructors across the codebase, e.g. `AircraftClass::Destructor`, `BuildingClass::Destructor`, `ObjectClass::UnInit`; verified via `get_function_by_address 0x007258D0` + `get_xrefs_to 0x007258D0` — INFERENCE_HARDENED, the "likely sparkle effect" label was an unverified guess against an unnamed `FUN_` address)

### 6.3 EMP Recovery (in TechnoClass::AI_Update)

When `EMPLockRemaining` decrements to 0:
- **Buildings:** calls `BuildingClass::RestoreOnlineEffects()`, re-enables all systems
- **Foot units:** restarts locomotor, clears EMP anim references

---

## 7. Parasites

### 7.1 ParasiteClass Struct Layout

**Size: ~0x58 bytes** (estimated from constructor field writes).

| Offset | Type | Field | Description |
|--------|------|-------|-------------|
| 0x00-0x0F | ptr[4] | vtables | Primary + 3 secondary |
| 0x10-0x23 | | AbstractClass base | |
| 0x24 | ptr | Owner | TechnoClass* that owns this parasite weapon |
| 0x28 | ptr | Victim | TechnoClass* currently being parasitized |
| 0x2C | CDTimer | DamageTimer | Timer for periodic damage |
| 0x34 | int | (unknown) | |
| 0x38 | CDTimer | LifeTimer | Timer for total parasite duration |
| 0x40 | int | field_40 | |
| 0x44 | int | (unused) | Init to 0 |
| 0x48 | int | (unused) | Init to 0 |
| 0x4C | int | field_4C | Init to 0 |
| 0x50 | int | field_50 | Init to 0 |
| 0x54 | BYTE | field_54 | Init to 0 |

### 7.2 TechnoTypeClass Parasite INI Keys

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `Parasiteable` | (read ~0x714F86) | bool | Unit can be targeted by parasites |
| `ImmuneToPoison` | (read from string 0x00843704) | bool | Immune to poison/parasite |

### 7.3 WarheadTypeClass Flags

| Offset | Type | Description |
|--------|------|-------------|
| +0x155 | bool | MindControl |
| +0x156 | bool | Poison (parasite) |
| +0x157 | bool | IvanBomb |
| +0x159 | bool | Parasite (corrected 2026-07-18: added, was missing; a distinct INI key from Poison at +0x156 — `WarheadTypeClass::ReadINI` (0x0075D590) writes the `"Parasite"` string result (string @0081717c) to +0x159, immediately before the Temporal write; `Parasite=yes` confirmed present in `ini/rulesmd.ini` — INFERENCE_HARDENED) |
| +0x15A | bool | Temporal (corrected 2026-07-18: was +0x159; binary shows the `"Temporal"` string (@00817168) write targets +0x15A, not +0x159 — verified byte-by-byte via `disassemble_function 0x0075D590`: EMEffect(+0x154)/MindControl(+0x155)/Poison(+0x156)/IvanBomb(+0x157)/ElectricAssault(+0x158)/Parasite(+0x159)/Temporal(+0x15A)/IsLocomotor(+0x15B) — OFFSET_RETYPED_WRONG) |

These are the mutually exclusive special warhead effects checked in
`WarheadTypeClass::Detonate` (0x004690B0, confirmed via `get_function_by_address`).

---

## 8. Self-Healing

### 8.1 Veteran Self-Heal in AI_Update

Self-healing is checked via vtable+0x294 (the `ShouldSelfHeal` virtual). In
`TechnoClass::AI_Update` (0x006F9E50):

```c
if (this->ShouldSelfHeal()) {  // vtable+0x294
    this->Health += 1;
    float ratio = GetHealthRatio();
    if (ratio > RulesClass->ConditionYellow || GetDamageState() < -10) {
        // Remove damage fire anim if exists
        if (this->DamageFireAnim != 0) {
            DamageFireAnim->Remove();
        }
    }
}
```

The `ShouldSelfHeal` virtual is overridden per class type (Infantry, Unit, Building) and
checks whether the SELF_HEAL ability is active at the unit's current veteran/elite rank.
The heal rate is 1 HP per tick, gated by the modulo timer.

### 8.2 RulesClass Self-Heal Settings

| INI Key | RulesClass Offset | Type | Default | Description |
|---------|-------------------|------|---------|-------------|
| `SelfHealInfantryFrames` | struct +0x30 (read at ~0x66E6EB, corrected 2026-07-18: was `~0x66E705` — that address belongs to `SelfHealInfantryAmount`; the actual `"SelfHealInfantryFrames"` string xref (@0083cc5c) is at code address 0x0066e6eb, writing RulesClass+0x30; verified via `disassemble_function 0x0066D530` — OFFSET_RETYPED_WRONG) | int | 50 | Frames between infantry self-heal ticks |
| `SelfHealInfantryAmount` | struct +0x34 (read at ~0x66E705; address itself was already correct — string @0083cc44 confirmed `"SelfHealInfantryAmount"` via `read_memory`, writing RulesClass+0x34) | int | 20 | HP healed per tick (infantry) |
| `SelfHealUnitFrames` | struct +0x38 (read at ~0x66E71F, confirmed) | int | 75 | Frames between unit self-heal ticks |
| `SelfHealUnitAmount` | struct +0x3C (read at ~0x66E738, confirmed) | int | 5 | HP healed per tick (vehicles) |

### 8.3 TechnoTypeClass

| INI Key | Offset (byte) | Type | Description |
|---------|---------------|------|-------------|
| `SelfHealing` | +0xD14 (param_1[0x345]) | bool | Unit has innate self-healing |

### 8.4 Building Self-Repair in AI_Update

Buildings have a separate repair system (not veteran self-heal):
- **Infantry repair** (WhatAmI == 1, type has `Hospital` flag at +0xD97):
  - Ticks every `RulesClass+0x38` frames
  - Heals up to `PowerDrain` HP per tick
- **Building repair** (WhatAmI == 0xF, or infantry-hospital):
  - Ticks every `RulesClass+0x30` frames
  - Requires house to have power
  - Heals up to min(PowerOutput, MaxHealth - CurrentHealth) HP per tick

---

## 9. Ammo System

### 9.1 TechnoTypeClass Ammo Fields

All accessed as `param_1[index]` where param_1 is `int*`, so byte offset = index * 4.

| INI Key | Array Index | Byte Offset | Type | Description |
|---------|-------------|-------------|------|-------------|
| `InitialAmmo` | 0x1A0 | +0x680 | int | Starting ammo count |
| `Ammo` | 0x1A1 | +0x684 | int | Maximum ammo capacity |
| `IFVMode` | 0x1A2 | +0x688 | int | IFV weapon mode |
| `Reload` | 0x1A6 | +0x698 | int | Frames between reloads |
| `EmptyReload` | 0x1A7 | +0x69C | int | Frames for full reload from empty |
| `ReloadIncrement` | 0x1A8 | +0x6A0 | int | Ammo gained per reload tick |
| `ManualReload` | 0x349 (byte) | +0xD24 | bool | Must return to base to reload |

### 9.2 Ammo Tracking in TechnoClass

The ammo tick system is in TechnoClass::AI_Update. The reload timer is a CDTimer
at TechnoClass fields around +0x100/+0x108/+0x10C/+0x110:

```c
// Ammo reload tick (from AI_Update, non-building units)
if (WhatAmI() != Building) {
    if (ReloadTimer.HasExpired()) {
        if (ReloadIncrement != 0) {
            this->ReloadDirty = 1;        // +0xFC
            this->CurrentAmmo += ReloadIncrement;  // +0xF8
            ReloadTimer.Start(ReloadDelay);
        }
    } else {
        this->ReloadDirty = 0;
    }
}
```

---

## 10. Passengers / Transport

### 10.1 CargoClass (Embedded in FootClass/TechnoClass)

The cargo system uses a `CargoClass` embedded struct, NOT a separate allocated object.

**CargoClass::AddPassenger** (0x004733A0):
```c
void CargoClass::AddPassenger(CargoClass *this, TechnoClass *passenger) {
    if (passenger == NULL) return;
    passenger->Limbo(false);  // vtable+0xD4, remove from map
    // Walk linked list to find insertion point
    // Count starts from NextObject linked list
    this->FirstPassenger = passenger;  // Store at index [1]
    // Recount total passengers
    this->PassengerCount = 0;
    TechnoClass *p = this->FirstPassenger;
    while (p != NULL && p->IsInLimbo) {
        this->PassengerCount++;
        p = p->NextObject;  // offset 0x30 in ObjectClass
    }
}
```

### 10.2 TechnoTypeClass Transport Fields

| INI Key | Array Index | Byte Offset | Type | Description |
|---------|-------------|-------------|------|-------------|
| `Passengers` | 0x178 | +0x5E0 | int | Maximum passenger capacity |
| `SizeLimit` | 0xE2 (double) | +0x388 | double | Max size of a single passenger |
| `PhysicalSize` | 0xDE (double) | +0x378 | double | Physical size of this unit |
| `Weight` | 0xDC (double) | +0x370 | double | Weight of this unit |

### 10.3 Enter/Exit Logic

**Entering:** Passengers are removed from the map via `Limbo(false)`, linked into the
cargo list. The linked list uses `ObjectClass::NextObject` (+0x30) as the next pointer.
`AbstractFlags & 4` (bit 2) is set to indicate "in transport".

**Exiting:** Passengers are placed back on the map via `Unlimbo()`. Cloakable units
that exit a transport are instantly set to CloakState=2 (fully cloaked), bypassing
the normal cloaking animation. (See CLOAKING_INTERACTIONS_REPORT.md).

---

## 11. Tiberium Storage (Ore)

### 11.1 StorageClass

A simple struct of 4 floats, one per ore/tiberium type.

**Size: 16 bytes** (4 x float)

```c
struct StorageClass {
    float amounts[4];  // One per tiberium type (0-3)
};
```

**StorageClass::Constructor** (0x006C95E0): Zeros all 4 floats.
**StorageClass::AddAmount** (0x006C9690): `amounts[type] += value`
**StorageClass::GetTotalAsInt** (0x006C9600): Sums all 4 floats, returns as int.
**StorageClass::RemoveAmount** (0x006C96B0): Subtracts from specified type.

### 11.2 Where Storage Lives

- **TechnoClass:** Has an embedded StorageClass for unit-level ore storage (used by harvesters).
  Initialized in constructor via `StorageClass::Constructor()`.
- **HouseClass:** Has aggregate storage for the player's total ore reserves.

### 11.3 TechnoTypeClass

| INI Key | Byte Offset (approx) | Type | Description |
|---------|----------------------|------|-------------|
| `Storage` | (read ~0x713130) | int | Maximum ore storage capacity |

### 11.4 Related Functions

- `HouseClass::Add_Tiberium_To_Storage` (0x004F9700)
- `HouseClass::Get_Storage_Fraction` (0x004F9750)
- `BuildingClass::DepositOreFromStorage` (0x00522D50)
- `UnitClass::Get_Storage_Percentage` (0x007414A0)

---

## 12. Power Consumption/Generation

(See: POWER_SYSTEM_GHIDRA_REPORT.md, POWER_INI_PARSING_AND_LIFECYCLE.md for full details)

### 12.1 Overview

Power is tracked at the HouseClass level, not per-unit. Buildings contribute to or drain
from the house power pool.

### 12.2 Key TechnoClass AI_Update Power Logic

In AI_Update, there is a "power drain" mechanic for units with `Drains` ability:
```c
if (this->DrainTarget != 0 && type->HasDrainAbility && 
    g_CurrentFrameCounter % RulesClass->DrainMoneyFrameDelay == 0) {
    int amount = min(RulesClass->DrainMoneyAmount, target->House->GetCredits());
    target->House->SpendMoney(amount);
    this->House->AddCredits(amount);
}
```

Fields:
| TechnoClass Offset | Type | Field |
|---------------------|------|-------|
| +0x1D0 | ptr | DrainTarget |
| +0x1CC | ptr | DrainSource (who is draining me) |
| +0x1D4 | ptr | DrainAnim |

RulesClass fields:
| Offset | INI Key | Type |
|--------|---------|------|
| +0x314 | DrainMoneyFrameDelay (corrected 2026-07-18: was `DrainFrames` — no such key exists; binary string xref at 0066c97f in `RulesClass__ReadCombatDamage` (0x0066BBB0) resolves to `"DrainMoneyFrameDelay"` (@0083af5c), writing +0x314; `ini/rulesmd.ini:856` confirms `DrainMoneyFrameDelay=30` — offset itself was already correct, only the key name was fabricated — INFERENCE_HARDENED) | int |
| +0x318 | DrainMoneyAmount (corrected 2026-07-18: was `DrainAmount` — no such key exists; binary string xref at 0066c99f resolves to `"DrainMoneyAmount"` (@0083af48), writing +0x318; `ini/rulesmd.ini:857` confirms `DrainMoneyAmount=30` — offset already correct, only the key name was fabricated — INFERENCE_HARDENED) | int |

---

## 13. TechnoClass Master Field Map (Constructor-Verified)

All offsets from `TechnoClass::Constructor` (0x006F2B40). param_1 is `int*`,
so `param_1[N]` = byte offset `N * 4`.

| Array Index | Byte Offset | Init Value | Likely Field |
|-------------|-------------|------------|--------------|
| 0x3C | +0xF0 | 0 | DamageFlags |
| 0x40 | +0x100 | CurrentFrame | CDTimer (reload?) |
| 0x42 | +0x108 | 0 | CDTimer duration |
| 0x44 | +0x110 | 1 | |
| 0x46 | +0x118 | 0 | FirstPassenger (cargo) |
| 0x47 | +0x11C | 0 | Transport pointer |
| 0x48 | +0x120 | -100 (0xFFFFFF9C) | |
| 0x4E | +0x138 | 0 | |
| 0x4F | +0x13C | -1 | DamageCategory |
| 0x50 | +0x140 | 0 | |
| 0x53 | +0x14C | Owner HouseClass | House (set from param_2) |
| 0x54 | +0x150 | 0.0 (via FUN_0074ff30) | **Veterancy** |
| 0x56-0x57 | +0x158 | 1.0 (double) | SpeedMultiplier |
| 0x58-0x59 | +0x160 | 1.0 (double) | FirepowerMultiplier |
| 0x5A | +0x168 | CurrentFrame | CDTimer (weapon 0 ROF?) |
| 0x5D | +0x174 | CurrentFrame | CDTimer |
| 0x60 | +0x180 | CurrentFrame | CDTimer |
| 0x62 | +0x188 | 0x2D (45) | Timer duration (45 frames) |
| 0x63 | +0x18C | CurrentFrame | **IronCurtainStartFrame** |
| 0x65 | +0x194 | 0 | **IronCurtainDuration** |
| 0x66-0x68 | +0x198 | CDTimer | Temporal visual timer |
| 0x69 | +0x1A4 | 10 | **TemporalVisualState** (init 10 = not active) |
| 0x6A-0x6C | +0x1A8 | CDTimer | |
| 0x71 | +0x1C4 | 0 | **IsForceShield** |
| 0x73-0x75 | +0x1CC | 0 | DrainSource, DrainTarget, DrainAnim |
| 0x87 | +0x21C | Owner (param_2) | **HouseClass pointer** |
| 0x88 | +0x220 | 0 | **CloakState** |
| 0x89 | +0x224 | 0 | **CloakProgress** |
| +0x269 | | 0 | CloakShroudActive |
| +0x270 | | 0 | IsWarpingOut |
| +0x271 | | 0 | IsWarpingIn |
| +0x272 | | 0 | (unknown chrono flag) |
| +0x27C | | 0 | ChronoInTransit |
| 0x9E | +0x278 | 0 | TemporalChainHead |
| 0xA2-0xA4 | +0x288 | NullCoord | ChronoDestCoord |
| 0xAC | +0x2B0 | 0 | SpawnManager |
| 0xAD | +0x2B4 | 0 | (target?) |
| 0xAF | +0x2BC | 0 | **CaptureManager** |
| 0xB0 | +0x2C0 | 0 | **MindControlledBy** |
| +0x2C4 | | 0 | **IsMindControlled** |
| 0xB2 | +0x2C8 | 0 | **MindControlAnim** |
| 0xB4 | +0x2D0 | 0 | **SpawnManagerClass pointer** |
| 0xB5 | +0x2D4 | 0 | Transport (for spawns) |
| 0xB6 | +0x2D8 | 0 | SlaveManagerClass pointer |
| +0x3D2 | | 0 | **HasStealthAbility** |
| +0x41A | | 0 | IsDiscoveredByPlayer |
| +0x425 | | 0 | EMPFlag |
| +0x504 | | 0 | **EMPLockRemaining** (corrected 2026-07-12: IS zeroed in the constructor — `006f3112: MOV dword ptr [ESI + 0x504],EBX`; the prior "not in constructor" note was wrong, verified via `disassemble_function 0x006F2B40` — INFERENCE_HARDENED) |

---

## 14. Summary of INI Key Cross-Reference

### TechnoTypeClass (parsed in ReadINI @ 0x00712170)

| INI Key | Byte Offset | Type | Section |
|---------|-------------|------|---------|
| `VeteranAbilities` | +0x29C (array) | BYTE[18] | Veterancy |
| `EliteAbilities` | +0x2AE (array) | BYTE[18] | Veterancy |
| `CloakingSpeed` | +0x310 | int | Cloaking |
| `Cloakable` | +0xCD0 | bool | Cloaking |
| `CloakStop` | +0xC93 | bool | Cloaking |
| `SensorsSight` | +0x5F0 | int | Detection |
| `Invisible` | +0xC9A | bool | Visibility |
| `Sensors` | +0xC9D | bool | Detection |
| `DontScore` | +0xC9F | bool | Scoring |
| `Trainable` | +0xC8E | bool | Veterancy |
| `SelfHealing` | +0xD14 | bool | Self-heal |
| `Passengers` | +0x5E0 | int | Transport |
| `SizeLimit` | +0x388 (double) | double | Transport |
| `PhysicalSize` | +0x378 (double) | double | Transport |
| `Weight` | +0x370 (double) | double | Transport |
| `Storage` | (at ~0x713130) | int | Ore |
| `InitialAmmo` | +0x680 | int | Ammo |
| `Ammo` | +0x684 | int | Ammo |
| `Reload` | +0x698 | int | Ammo |
| `EmptyReload` | +0x69C | int | Ammo |
| `ReloadIncrement` | +0x6A0 | int | Ammo |
| `ManualReload` | +0xD24 | bool | Ammo |
| `Parasiteable` | (at ~0x714F86) | bool | Parasite |
| `ImmuneToRadiation` | +0xD37 | bool | Radiation |
| `ImmuneToPsionics` | +0xD35 | bool | Mind Control |
| `ImmuneToPsionicWeapons` | +0xD36 | bool | Mind Control |
| `MindControlRingOffset` | +0x60C | int | Mind Control |
| `LeptonMindControlOffset` | +0x3DC | int | Mind Control |

### RulesClass (parsed in ReadGeneral/ReadAudioVisual)

| INI Key | RulesClass Offset | Type | Section |
|---------|-------------------|------|---------|
| `VeteranRatio` | +0x668 | double | Veterancy |
| `VeteranCombat` | +0x670 | double | Veterancy |
| `VeteranSpeed` | +0x678 | double | Veterancy |
| `VeteranSight` | +0x680 | double | Veterancy |
| `VeteranArmor` | +0x688 | double | Veterancy |
| `VeteranROF` | +0x690 | double | Veterancy |
| `VeteranCap` | +0x698 | double | Veterancy |
| `CloakingStages` | +0x628 | int | Cloaking |
| `CloakDelay` | +0x1410 | double | Cloaking |
| `ConditionYellow` | +0x1708 | double | Health states |
| `SelfHealUnitFrames` | +0x38 (at ~0x66E71F) | int | Self-heal |
| `SelfHealUnitAmount` | +0x3C (at ~0x66E738) | int | Self-heal |
| `SelfHealInfantryFrames` | +0x30 (at ~0x66E6EB, corrected 2026-07-18: was `~0x66E705`, see §8.2) | int | Self-heal |
| `SelfHealInfantryAmount` | +0x34 (at ~0x66E705) | int | Self-heal |
| `DrainMoneyFrameDelay` | +0x314 | int | Power drain (corrected 2026-07-18: key name was `DrainFrames`, does not exist; see §12.2, `ini/rulesmd.ini:856`) |
| `DrainMoneyAmount` | +0x318 | int | Power drain (corrected 2026-07-18: key name was `DrainAmount`, does not exist; see §12.2, `ini/rulesmd.ini:857`) |
