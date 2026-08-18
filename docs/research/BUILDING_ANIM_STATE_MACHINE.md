# Building Animation State Machine — Ghidra Deep Dive

Source: Direct decompilation of gamemd.exe via Ghidra MCP
Functions: `0x451EE0`, `0x4545D0`, `0x4521C0`, `0x452210`, `0x451890`, `0x451E40`,
`0x451750`, `0x4509D0`, `0x450630`, `0x4547C0`, `0x454730`

---

## Architecture: 21 Fixed Slots

Every `BuildingClass` instance has a fixed array of 21 `AnimClass*` pointers starting
at offset `+0x55C`, spaced 4 bytes apart (21 × 4 = 84 bytes, `0x55C..0x5AF`).

Every `BuildingTypeClass` has a parallel array of 21 slot definitions starting at
offset `+0xF4C`, spaced `0x44` (68) bytes apart (21 × 68 = 1,428 bytes).

### Slot Layout in BuildingTypeClass

Each slot occupies `0x44` bytes with this structure:

| Relative Offset | Field | Description |
|-----------------|-------|-------------|
| `+0x00` (+0xF4C) | char[16] | **Undamaged** anim type name (e.g., "GAREFN_A") |
| `+0x10` (+0xF5C) | char[16] | **Damaged** anim type name (e.g., "GAREFN_AD") |
| `+0x20` (+0xF6C) | char[16] | **Firing** anim type name |
| `+0x30` (+0xF7C) | int | AnimTypeClass* (resolved at load from name) |
| `+0x34` (+0xF80) | int | X,Y pixel offsets for draw position |
| `+0x38` (+0xF84) | int | ZAdjust (draw offset) |
| `+0x3C` (+0xF88) | int | LoopCount / YSort |
| `+0x40` (+0xF8C) | byte | `XXXPowered` — Flag A: anim linked to weapon visual; stays alive on power-off but gets detached (0x19E=1); re-attached on power-on |
| `+0x41` (+0xF8D) | byte | `XXXPoweredLight` — Flag B: anim is power-conditional; destroyed+recreated on power-off (if building WasPowered); destroyed on power-on |
| `+0x42` (+0xF8E) | byte | `XXXPoweredEffect` — Flag C: anim tracks charge state; charge flag 0x5B0[slot] cleared on power-off, set on power-on |
| `+0x43` (+0xF8F) | byte | `XXXPoweredSpecial` — Flag D: special/turret-trigger anim; plays during PoweredSpecial activation (0x454730) |

### Slot Assignments (verified from INI parsing at 0x45FE50)

| Slot | Index | TypeClass Base | art.ini Key | Purpose |
|------|-------|---------------|-------------|---------|
| 0 | `0x00` | `+0xF4C` | PowerUp1Anim | Upgrade slot 1 anim |
| 1 | `0x01` | `+0xF90` | PowerUp2Anim | Upgrade slot 2 anim |
| 2 | `0x02` | `+0xFD4` | PowerUp3Anim | Upgrade slot 3 anim |
| 3 | `0x03` | `+0x1018` | ActiveAnim | Activity overlay (refinery arm, crane) |
| 4 | `0x04` | `+0x105C` | ActiveAnimTwo | Second activity overlay |
| 5 | `0x05` | `+0x10A0` | ActiveAnimThree | Third activity overlay |
| 6 | `0x06` | `+0x10E4` | ActiveAnimFour | Fourth activity overlay |
| 7 | `0x07` | `+0x1128` | PreProductionAnim | Pre-production warmup anim |
| 8 | `0x08` | `+0x116C` | ProductionAnim | Factory production animation |
| 9 | `0x09` | `+0x11B0` | TurretAnim | Turret-facing-driven anim |
| 10 | `0x0A` | `+0x11F4` | SpecialAnim | Turret-quarter-mapped anim / ActiveAnim swap target |
| 11 | `0x0B` | `+0x1238` | SpecialAnimTwo | Second special anim |
| 12 | `0x0C` | `+0x127C` | SpecialAnimThree | Third special anim |
| 13 | `0x0D` | `+0x12C0` | SpecialAnimFour | Fourth special anim |
| 14 | `0x0E` | `+0x1304` | SuperAnim | Super weapon charge indicator |
| 15 | `0x0F` | `+0x1348` | SuperAnimTwo | Second charge indicator |
| 16 | `0x10` | `+0x138C` | SuperAnimThree | Third charge indicator |
| 17 | `0x11` | `+0x13D0` | SuperAnimFour | Fourth charge indicator |
| 18 | `0x12` | `+0x1414` | IdleAnim | Always-looping (flags, smokestacks) |
| 19 | `0x13` | `+0x1458` | LowPower | Low-power state anim (PoweredSpecial) |
| 20 | `0x14` | `+0x149C` | SuperLowPower | Super-low-power state anim |

**Loop termination**: `0x594 / 0x44 = 0x15 = 21` — the loops iterate with `iVar += 0x44` until `>= 0x594`.

---

## 1. Damaged Art Switching (`SetDamagedState` — `0x451EE0`)

Called when health crosses the **ConditionYellow** threshold (default 50%).

### Decompiled Logic

```c
void BuildingClass::SetDamagedState(this, bool is_damaged)
{
    // Early out if no state change
    if (this->isDamaged_0x6E6 == is_damaged) return;

    this->isDamaged_0x6E6 = is_damaged;

    // Iterate all 21 anim slots
    for (slot = 0; slot < 21; slot++) {
        AnimClass* anim = this->animSlots_0x55C[slot];
        if (anim == NULL) continue;

        // Select undamaged or damaged art name
        char* art_name;
        if (is_damaged) {
            art_name = &this->type->slotDefs[slot].damaged_name;  // +0xF5C
        } else {
            art_name = &this->type->slotDefs[slot].undamaged_name; // +0xF4C
        }

        if (art_name != NULL && art_name[0] != '\0') {
            CreateAnimForSlot(art_name, slot, is_damaged, 0, 0);
        }
    }
}
```

### Key Details

- **Health ratio function** (`0x5F5C60`): `return this->health_0x6C / (float)this->type->Strength_0xA0`
- **ConditionYellow threshold**: stored at `Rules+0x1700` as a `double` (default `0.5`)
- The comparison is: `GetHealthRatio() > ConditionYellow` → undamaged; `<=` → damaged
- When switching, it **replaces** each existing anim with its damaged/undamaged variant
- The slot array offsets: undamaged at `+0xF4C + slot*0x44`, damaged at `+0xF5C + slot*0x44`

### When It's Called

1. **During repair** (`0x450630`): After each heal tick, recheck health ratio and swap all 21 anims if threshold crossed
2. **On damage received**: When health drops below ConditionYellow
3. **On placement**: Initial state set based on starting health

---

## 2. Power State Anim Switching (`0x4545D0` / `0x4547C0`)

Called every tick from `UpdateGapAndSpecialEffects` (`0x4549B0`) when
`TypeClass+0x1573` (Powered=yes) is set AND `TypeClass+0xEE4` (Drain) > 0.

- `OnPowerOff` (`0x4545D0`): called when building **HAS power** — restores normal anims
- `OnPowerOn` (`0x4547C0`): called when building **LACKS power** — shows low-power anims

### The Four Flag Bytes Per Slot (INI key suffixes)

Each of the 21 anim slots has 4 control flags at slot offsets `+0x40..+0x43`:

| Flag | Slot Offset | INI Suffix | Default | HasPower (OnPowerOff) | NoPower (OnPowerOn) |
|------|------------|------------|---------|----------------------|---------------------|
| A | `+0x40` | `Powered` | 1 (!) | Anim made visible: `0x19E=0` via `0x425270` | Anim made invisible: `0x19E=1` via `0x425260` |
| B | `+0x41` | `PoweredLight` | 0 | If slot empty + WasPowered: **create** idle replacement anim | If anim exists: **destroy** it (slot 10 special: create ActiveAnim in slot 3) |
| C | `+0x42` | `PoweredEffect` | 0 | If chargeFlag set: clear it + create static replacement | If anim exists: set chargeFlag + destroy (slot 16 special: create in slot 20) |
| D | `+0x43` | `PoweredSpecial` | 0 | (handled by orchestrator: create Flag D anims) | (handled by orchestrator: destroy Flag D anims + show LowPower) |

**IMPORTANT**: Flag A defaults to 1 in the constructor! This means every slot that does
NOT explicitly set `XXXPowered=no` in art.ini will have this flag set. Most building types
override these defaults to 0 for slots they don't use (names are empty strings).

### Decompiled Logic (Power-Off: `0x4545D0`)

```c
void BuildingClass::OnPowerOff(this)
{
    ClearAnimSlot(0x14);  // Always clear slot 20 (SuperLowPower) first

    for (slot = 0; slot < 21; slot++) {
        BuildingTypeClass* type = this->type_0x520;
        byte flagA = *(byte*)(type + 0xF8C + slot * 0x44);  // Powered
        byte flagB = *(byte*)(type + 0xF8D + slot * 0x44);  // PoweredLight
        byte flagC = *(byte*)(type + 0xF8E + slot * 0x44);  // PoweredEffect

        if (flagA != 0) {
            // POWERED anim: keep alive but detach from building weapon system
            if (this->animSlots[slot] != NULL) {
                FUN_00425270();  // sets anim->0x19E = 0 (was: = 1 in earlier analysis)
                // NOTE: 0x425270 sets 0x19E=0, 0x425260 sets 0x19E=1
                // On power-off, the anim stays but loses its "active" weapon visual
            }
        }
        else if (flagB != 0) {
            // POWERED_LIGHT: create idle replacement if slot is empty + was powered
            if (this->animSlots[slot] == NULL
                && this->wasPowered_0x6E4 != 0) {
                // Skip if slot 10 (SpecialAnim) AND IsAnimDelayedFire (0x16A7) is set
                if (type->isAnimDelayedFire_0x16A7 == 0 || slot != 10) {
                    bool isDamaged = GetHealthRatio() <= ConditionYellow;
                    char* art = isDamaged
                        ? &type->slotDefs[slot].damaged   // +0xF5C
                        : &type->slotDefs[slot].undamaged; // +0xF4C
                    if (art[0] != '\0')
                        CreateAnimForSlot(art, slot, isDamaged, 0, 0);
                }
            }
        }
        else if (flagC != 0) {
            // POWERED_EFFECT: if charge flag was set, clear it and create replacement
            if (this->chargeFlags_0x5B0[slot] != 0) {
                this->chargeFlags_0x5B0[slot] = 0;
                bool isDamaged = GetHealthRatio() <= ConditionYellow;
                char* art = isDamaged
                    ? &type->slotDefs[slot].damaged
                    : &type->slotDefs[slot].undamaged;
                if (art[0] != '\0')
                    CreateAnimForSlot(art, slot, isDamaged, 0, 0);
            }
        }
    }
}
```

### The +0x5B0 Array: Charge Flags (21 bytes)

`BuildingClass+0x5B0` is an array of 21 bytes (one per anim slot), used exclusively
by the PoweredEffect (Flag C) system. When a slot has Flag C set:
- **On power-off** (`OnPowerOff`): `chargeFlags[slot]` is cleared to 0, meaning
  "this slot's powered effect is no longer active"
- **On power-on** (`OnPowerOn`): `chargeFlags[slot]` is set to 1, meaning
  "this slot's powered effect is re-enabled"

These flags act as a latch: they remember whether the effect was active before
a power state change, so only slots that were actually running get toggled.

### Power-On (`0x4547C0`)

The inverse operation when power is restored:

```c
void BuildingClass::OnPowerOn(this)
{
    for (slot = 0; slot < 21; slot++) {
        int typeBase = this->type_0x520 + slot * 0x44;
        byte flagA = *(byte*)(typeBase + 0xF8C);  // Powered
        byte flagB = *(byte*)(typeBase + 0xF8D);  // PoweredLight
        byte flagC = *(byte*)(typeBase + 0xF8E);  // PoweredEffect

        if (flagA != 0) {
            // POWERED anim: re-attach (reverse the power-off detach)
            if (this->animSlots[slot] != NULL) {
                FUN_00425260();  // sets anim->0x19E = 1
            }
        }
        else if (flagB != 0) {
            // POWERED_LIGHT: destroy the idle replacement that was created on power-off
            if (this->animSlots[slot] != NULL) {
                ClearAnimSlot(slot);

                // SPECIAL: slot 10 (SpecialAnim) + IsAnimDelayedFire + ActiveAnim defined
                // → replace SpecialAnim with ActiveAnim in slot 3
                if (slot == 10
                    && type->isAnimDelayedFire_0x16A7 != 0
                    && type->activeAnim_0x1058 != 0) {  // ActiveAnimPowered flag
                    bool isDamaged = GetHealthRatio() <= ConditionYellow;
                    char* art = isDamaged
                        ? &type->activeAnimDamaged_0x1028
                        : &type->activeAnim_0x1018;
                    if (art[0] != '\0')
                        CreateAnimForSlot(art, 3, isDamaged, 0, 0);
                        //                    ^ slot 3 = ActiveAnim
                }
            }
        }
        else if (flagC != 0) {
            // POWERED_EFFECT: set charge flag, destroy anim, possibly create SuperLowPower
            if (this->animSlots[slot] != NULL) {
                this->chargeFlags_0x5B0[slot] = 1;
                ClearAnimSlot(slot);

                // SPECIAL: slot 16 (SuperAnimThree) → create SuperLowPower in slot 20
                if (slot == 0x10) {
                    bool isDamaged = GetHealthRatio() <= ConditionYellow;
                    char* art = isDamaged
                        ? &type->superLowPowerDamaged_0x14AC  // slot 20 damaged
                        : &type->superLowPower_0x149C;        // slot 20 undamaged
                    if (art[0] != '\0')
                        CreateAnimForSlot(art, 0x14, isDamaged, 0, 0);
                        //                      ^ slot 20 = SuperLowPower
                }
            }
        }
    }
}
```

---

## 3. Special Anim Trigger (`0x454730`) — Flag D: PoweredSpecial

Triggered by turret/special state changes. Clears slot 19 (LowPower, 0x13) first,
then checks each slot's **Flag D** (`+0xF8F`, the `XXXPoweredSpecial` flag):

```c
void BuildingClass::TriggerSpecialAnims(this)
{
    ClearAnimSlot(0x13);  // Clear slot 19 (LowPower anim)

    for (slot = 0; slot < 21; slot++) {
        if (type->slotDefs[slot].flag_0xF8F != 0) {  // PoweredSpecial flag
            bool is_damaged = GetHealthRatio() <= ConditionYellow;
            char* art = is_damaged ? slotDef.damaged : slotDef.undamaged;
            if (art[0] != '\0') {
                CreateAnimForSlot(art, slot, is_damaged, 0, 0);
            }
        }
    }
}
```

## 3b. UpdateGapAndSpecialEffects (`0x4549B0`) — The Orchestrator

This is the master function that decides WHEN to call OnPowerOff/OnPowerOn.
Called from the building's update tick. The key flow:

```c
void BuildingClass::UpdateGapAndSpecialEffects(this)
{
    bool isPowered = this->vtable->IsPowered();  // vtable+0x350

    if (isPowered) {
        // === BUILDING HAS POWER ===

        // Gap generator activation
        if (type->gapGeneratorRadius_0x40C != 0 && !this->gapActive_0x662) {
            this->gapActive_0x662 = 1;
            ActivateGapGenerator();
        }

        // CloakGenerator, SensorArray, etc...

        // KEY: If Powered=yes flag AND Drain > 0, call OnPowerOFF
        // (this is the "powered building just came online" path --
        //  it switches FROM powered anims to normal anims)
        if (type->isPowered_0x1573 && type->drain_0xEE4 > 0) {
            OnPowerOff();  // 0x4545D0
        }

        // PoweredSpecial handling (flag at 0x1574):
        if (type->isPoweredSpecial_0x1574) {
            ClearAnimSlot(0x13);  // Clear LowPower slot
            // Create anims for all slots with Flag D (PoweredSpecial)
            for (slot = 0; slot < 21; slot++) {
                if (slotDefs[slot].flag_0xF8F != 0) {
                    CreateAnimForSlot(art, slot, isDamaged, 0, 0);
                }
            }
        }

    } else {
        // === BUILDING HAS NO POWER ===

        // Deactivate gap generator, cloak, sensor...

        // KEY: If Powered=yes flag AND Drain > 0, call OnPowerON
        if (type->isPowered_0x1573 && type->drain_0xEE4 > 0) {
            OnPowerOn();  // 0x4547C0
        }

        // PoweredSpecial handling:
        if (type->isPoweredSpecial_0x1574) {
            // Timer-based: check if enough time has elapsed
            // If so, create LowPower anim in slot 19
            bool isDamaged = GetHealthRatio() <= ConditionYellow;
            char* art = isDamaged
                ? &type->lowPowerDamaged_0x1468
                : &type->lowPower_0x1458;
            if (art[0] != '\0')
                CreateAnimForSlot(art, 0x13, isDamaged, 0, 0);

            // Also clear any slots with Flag D (PoweredSpecial)
            for (slot = 0; slot < 21; slot++) {
                if (slotDefs[slot].flag_0xF8F != 0)
                    ClearAnimSlot(slot);
            }
        }
    }
}
```

**NAMING vs BEHAVIOR** (the names are counterintuitive -- focus on what they DO):

| | `OnPowerOff` (0x4545D0) | `OnPowerOn` (0x4547C0) |
|--|------------------------|----------------------|
| **Called when** | Building HAS power (`IsPowered()==true`) | Building LACKS power (`IsPowered()==false`) |
| **Flag A (Powered)** | Make anim visible (`0x19E=0`) | Make anim invisible (`0x19E=1`) |
| **Flag B (PoweredLight)** | Create idle replacement in empty slots | Destroy idle replacement anims |
| **Flag C (PoweredEffect)** | Clear charge flag + create static anim | Set charge flag + destroy anim |
| **First action** | Clear slot 20 (SuperLowPower) | (nothing) |
| **Net effect** | Building looks "powered on" | Building looks "powered off" |

The names appear to reference the PREVIOUS state being left:
- `OnPowerOff` = "leaving the power-off state" = restoring to powered appearance
- `OnPowerOn` = "leaving the power-on state" = switching to unpowered appearance

**Recommended Rust names:**
- `OnPowerOff` -> `enforce_powered_anims()` or `restore_normal_anims()`
- `OnPowerOn` -> `enforce_unpowered_anims()` or `show_lowpower_anims()`

---

## 4. Cloaking / Uncloaking (`0x4521C0` / `0x452210`)

### Start Cloaking (`0x4521C0`)

```c
void BuildingClass::StartCloaking(this)
{
    this->isPowered_0x660 = 0;  // Power off visual state

    for (i = 0; i < 21; i++) {  // loop counter = 0x15
        AnimClass* anim = this->animSlots[i];
        if (anim != NULL) {
            anim->isTranslucent_0x11A = 1;   // Enable translucency
            anim->isShrinking_0x11B  = 0;    // Not shrinking (fading)
            anim->fadeTarget_0x11C   = anim->currentAlpha_0xAC;  // Preserve current opacity
            anim->isCloaking_0x119   = 1;    // Mark as cloaking
        }
    }
}
```

### Stop Cloaking (`0x452210`)

```c
void BuildingClass::StopCloaking(this)
{
    this->isPowered_0x660 = 1;  // Power on visual state

    for (i = 0; i < 21; i++) {
        AnimClass* anim = this->animSlots[i];
        if (anim != NULL) {
            anim->isTranslucent_0x11A = 0;   // Disable translucency
            anim->isShrinking_0x11B  = 1;    // Expanding back to full opacity
            anim->isCloaking_0x119   = 0;    // Clear cloaking flag
        }
    }
}
```

### AnimClass Fields for Cloaking

| Offset | Type | Field | Cloaking | Uncloaking |
|--------|------|-------|----------|------------|
| `+0x119` | byte | `isCloaking` | 1 | 0 |
| `+0x11A` | byte | `isTranslucent` | 1 | 0 |
| `+0x11B` | byte | `isExpanding` | 0 | 1 |
| `+0x11C` | int | `fadeTarget` | copy from `+0xAC` | (unchanged) |
| `+0xAC` | int | `currentAlpha` | (read) | (unchanged) |

---

## 5. DamageFireAnims — Fire/Smoke on Damaged Buildings

These are **NOT** stored in the 21-slot system. They use a completely separate mechanism.

### How They Work

**Source data**: `DamageFireOffset0=X,Y` through `DamageFireOffset7=X,Y` in art.ini per building.
These are pixel offsets from the building's draw origin where fire/smoke overlays appear.

**Anim types used**: `FIRE01`, `FIRE02`, `FIRE03` (defined in artmd.ini with `LoopCount=-1`,
`Rate=450`, `StartSound=BuildingFireBig/BuildingFireMed`).

Additionally, `DamageParticleSystems=SparkSys,SmallGreySSys` in rules.ini provides
particle effects (sparks + grey smoke) that play alongside the fire anims.

### When They Spawn

DamageFireAnims spawn when health drops below **ConditionYellow** (50%) and are managed
by the building's render/update cycle — they are standalone `AnimClass` objects attached
to the building's cell position + the DamageFireOffset, not part of the 21-slot array.

The engine creates 1–3 fire anims depending on how many `DamageFireOffset` entries exist:
- **ConditionYellow** (50%): 1 fire anim at offset 0
- **ConditionRed** (25%): additional fire anims at remaining offsets
- Each fire anim is an independently looping AnimClass (FIRE01/02/03 randomly selected)

### art.ini Format

```ini
[GAWEAP]
DamageFireOffset0=-26,27    ; pixel offset for fire anim #1
DamageFireOffset1=-2,-57    ; pixel offset for fire anim #2
DamageFireOffset2=22,50     ; pixel offset for fire anim #3
; Up to DamageFireOffset7 (8 max positions)
```

---

## 6. CreateAnimForSlot (`0x451890`) — The Workhorse

This is the central function that populates an anim slot. Called by 18+ other functions.

### Decompiled Logic

```c
void BuildingClass::CreateAnimForSlot(this, char* animName, int slotIndex,
                                       bool isDamaged, bool isFiring, int extra)
{
    // 1. If damage state changed, update ALL 21 slots
    if (this->isDamaged_0x6E6 != isDamaged) {
        this->isDamaged_0x6E6 = isDamaged;
        for (i = 0; i < 21; i++) {
            if (this->animSlots[i] != NULL) {
                SetAnimSlotImage(i, isDamaged, 0, 0);
            }
        }
    }

    // 2. Look up AnimType from global AnimType array
    int animTypeIndex = FindAnimType(animName);  // FUN_00427CB0
    if (animTypeIndex == -1) return;

    // 3. Compute spawn position from slot definition offsets
    CoordStruct pos;
    GetAnimPosition(&pos, &this->type->slotDefs[slotIndex].xOffset);

    CoordStruct buildingPos;
    this->GetPosition(&buildingPos);  // vtable+0xAC

    // 4. Allocate new AnimClass (0x1C8 bytes)
    AnimClass* newAnim = new AnimClass(
        AnimTypes[animTypeIndex],  // DAT_008B4154[index]
        pos,
        extra,      // delay/layer
        1,          // loop
        0x1600,     // flags (visible, shadow)
        0,          // facing
        0           // z-adjust
    );

    // 5. Copy draw offsets from type definition
    newAnim->xDrawOffset_0x100 = this->type->slotDefs[slotIndex].xDrawOffset;
    newAnim->yDrawOffset_0x104 = this->type->slotDefs[slotIndex].yDrawOffset;
    newAnim->isAttached_0x118 = 1;

    // 6. Propagate veterancy to anim
    if (this->isElite_0x6E7 != 0) {
        newAnim->isElite_0x199 = 1;
    }

    // 7. Propagate gap generator shroud level to all 21 anims
    byte shroudLevel = this->gapRadiusCounter_0x6ED;
    if (shroudLevel == 0x0F && this->GetOwnerHouse()->IsPlayerControl() == 5) {
        shroudLevel = 0x10;  // max level for player-controlled
    }
    for (i = 0; i < 21; i++) {
        if (this->animSlots[i] != NULL) {
            this->animSlots[i]->shroudLevel_0x178 = shroudLevel;
        }
    }

    // 8. If slot already had an anim, transfer owner and destroy old one
    if (this->animSlots[slotIndex] != NULL) {
        newAnim->ownerObject_0xAC = this->animSlots[slotIndex]->ownerObject_0xAC;
        AnimClass* old = this->animSlots[slotIndex];
        this->animSlots[slotIndex] = NULL;
        old->vtable->Destroy(1);  // vtable+0x20
    }

    // 9. Store new anim in slot
    this->animSlots[slotIndex] = newAnim;

    // 10. If building is active + powered + TypeClass says "powered anim":
    //     disable weapon anim visual
    if (this->isActive_0x6EA && this->IsPowered() &&
        this->type->isPowered_0x1573 &&
        this->type->slotDefs[slotIndex].flag_0xF8C) {
        DisableWeaponAnim();  // +0x19E = 0
    }

    // 11. Special case: slot 9 (ProductionAnim) + HasBarrel → set barrel rotation flag
    if (slotIndex == 9 && this->type->hasBarrel_0x16C6 != 0) {
        newAnim->hasBarrelRotation_0x19D = 1;
    }

    // 12. If cloaking, set translucency on new anim
    if (this->IsCloaked()) {
        newAnim->isTranslucent_0x11A = 1;
    }
}
```

---

## 7. SetAnimSlotImage (`0x451750`) — Art Variant Selection

Selects between undamaged/damaged/firing art per slot:

```c
void BuildingClass::SetAnimSlotImage(this, int slot, bool isDamaged, bool isFiring)
{
    char* artName;

    if (!isDamaged) {
        if (!isFiring) {
            artName = &type->slotDefs[slot].undamaged;  // +0xF4C
        } else {
            artName = &type->slotDefs[slot].firing;     // +0xF6C
        }
    } else {
        artName = &type->slotDefs[slot].damaged;        // +0xF5C
    }

    if (artName != NULL && artName[0] != '\0') {
        CreateAnimForSlot(artName, slot, isDamaged, isFiring, 0);
    }
}
```

### Art Variant Offsets Per Slot

| Variant | Base Offset | Formula |
|---------|------------|---------|
| Undamaged | `+0xF4C` | `TypeClass + 0xF4C + slot * 0x44` |
| Damaged | `+0xF5C` | `TypeClass + 0xF5C + slot * 0x44` |
| Firing | `+0xF6C` | `TypeClass + 0xF6C + slot * 0x44` |

---

## 8. ClearAnimSlot (`0x451E40`)

```c
void BuildingClass::ClearAnimSlot(this, int slot)
{
    if (slot == -2) {
        // Special: clear ALL 21 slots
        for (i = 0; i < 21; i++) {
            AnimClass* anim = this->animSlots[i];
            if (anim != NULL) {
                this->animSlots[i] = NULL;
                anim->vtable->Destroy(1);
            }
        }
    } else {
        // Clear single slot
        AnimClass* anim = this->animSlots[slot];
        if (anim != NULL) {
            this->animSlots[slot] = NULL;
            anim->vtable->Destroy(1);
        }
    }
}
```

Magic value: `-2` (or `0xFFFFFFFE`) = "clear all".

---

## 9. Update Tick — The Animation State Machine (`0x4509D0`)

This is the 2,387-byte monster called every game tick. Key sections:

### Production Frame Counter
```c
if (this->productionFrameDelay_0x10C != 0) {
    this->needsRedraw_0xFC = 1;
    this->productionFrame_0xF8 += this->productionDirection_0x110;
    // ... update timers, check bounds
}
```

### Repair Depot Active Anim (Slot 0x0C)
```c
if (type->isRepairDepot_0x16A9 && GetMission() != 0x14
    && (hasDockingUnit || hasQueuedUnit) && !type->invisibleAnim_0xCCE) {

    bool isDamaged = GetHealthRatio() <= ConditionYellow;
    char* art = isDamaged ? type->repairDamagedAnim : type->repairUndamagedAnim;
    CreateAnimForSlot(art, 0x0C, isDamaged, 0, 0);
    ClearAnimSlot(8);   // clear idle anim 2
    ClearAnimSlot(0x0B); // clear damage fire 0
}
```

### Turret-Driven Anim (Slot 10 / 0x0A)
```c
if (type->hasTurretAnim_0x16A8) {
    int turretFacing = GetTurretFacing();  // 0..3+
    int frameIndex = (turretFacing << 2) / type->turretFrameCount;

    if (frameIndex == 0) {
        ClearAnimSlot(10);
    } else {
        if (this->animSlots[10] == NULL) {
            CreateAnimForSlot(turretAnim, 10, isDamaged, 0, 0);
        }
        this->animSlots[10]->currentFrame_0xAC = frameIndex;
    }
}
```

### SpecialAnim — Turret Facing Mapped to Slots 3–6
```c
if (type->hasSpecialAnim_0x16BB) {
    int turretFacing = GetTurretFacing();
    int slotIndex = (turretFacing << 2) / type->turretFrameCount;
    // Maps to slots 3, 4, 5, 6 based on turret quarter

    // Clear old slot, create new one
    ClearAnimSlot(oldSlotIndex);
    CreateAnimForSlot(specialArt, slotIndex + 3, isDamaged, 0, 0);
}
```

### Superweapon Charge Bar (Slots 14–20)
```c
if (type->superWeaponType_0x16F0 != -1 && !isSelling && !isConstructing) {
    float chargeThreshold = type->chargeThreshold_0x16E8;

    // Find matching super weapon
    for (sw in house->superWeapons) {
        if (sw->type == type->superWeaponType) {
            int remaining = sw->remainingTime;
            float chargeRatio = remaining * CHARGE_SCALE;

            // Map charge ratio to slot pair (14/15, 16/17, 18/19, 20)
            if (chargeRatio > chargeThreshold) {
                // Swap between indicator levels
                ClearAnimSlot(currentIndicator);
                CreateAnimForSlot(nextIndicatorArt, nextSlot, isDamaged, 0, 0);
            }
        }
    }
}
```

### Shadow Direction Update
```c
// After all anim updates, sync shadow direction for all non-NULL slots
if (this->animSlots[shadowSlot] != NULL) {
    uint facing = GetFacing();
    this->animSlots[shadowSlot]->shadowFrame_0xAC =
        DAT_007F4890[(facing >> 10) + 1 >> 1 & 0x1F];
    this->animSlots[shadowSlot]->shadowOffset_0xC4 = 0;
}
```

---

## 10. Repair Tick and Damage State Transitions (`0x450630`)

915 bytes. Handles auto-repair AI and manual repair orders.

### Auto-Repair Trigger
```c
// House must have enough buildings (Rules+0x1444 threshold)
// Building must not be selling (mission != 0x12) or constructing (0x13)
// Must be on map (vtable+0x94 returns true)

int healthPercent = house->GetHealthPercent();  // house+0x24, vtable+0x18
if (healthPercent < Rules->repairHealthCap_0x1758) {
    // Check: multiplayer OR auto-repair flag set
    // Check: house has enough credits (Rules+0x145C threshold)
    // Check: random chance (0..50 < house+0x1D4)
    // Check: not naval factory
    // Check: health < ConditionRed threshold (Rules+0x1708)

    this->vtable->StartSelfHeal(1);  // vtable+0x1A0
}
```

### Repair Heal Tick
```c
if (this->isRepairing_0x6B8) {
    int gameSpeed = GetGameSpeed();  // FUN_007C5F00
    int tickInterval = GameFrame / gameSpeed;

    if (GameFrame % gameSpeed == 0) {
        // Toggle wrench icon visibility
        this->showWrench_0x6DE = !this->showWrench_0x6DE;

        // Get repair amount from type
        int healAmount = type->vtable->GetRepairStep();    // vtable+0xB0
        int healCost   = type->vtable->GetRepairCost();    // vtable+0xB4

        // Check if we can afford it
        int currentHealth = this->health_0x6C;
        int maxHealth = type->Strength_0xA0;

        if (currentHealth + healAmount >= maxHealth) {
            this->health = maxHealth;
            this->isRepairing = false;
        } else {
            House::SpendMoney(healCost);
            this->health += healAmount;
        }

        // CHECK DAMAGE STATE TRANSITION
        float healthRatio = GetHealthRatio();
        double conditionYellow = Rules->conditionYellow_0x1700;
        bool wasInDamagedState = this->isDamaged_0x6E6;
        bool nowInDamagedState = healthRatio <= conditionYellow;

        if (wasInDamagedState != nowInDamagedState) {
            // TRANSITION: swap all 21 anim slots to new art variant
            this->isDamaged_0x6E6 = nowInDamagedState;
            for (slot = 0; slot < 21; slot++) {
                if (this->animSlots[slot] != NULL) {
                    char* art = (healthRatio > conditionYellow)
                        ? type->slotDefs[slot].undamaged  // +0xF4C
                        : type->slotDefs[slot].damaged;   // +0xF5C
                    if (art[0] != '\0') {
                        CreateAnimForSlot(art, slot, nowInDamagedState, 0, 0);
                    }
                }
            }
        }

        // If transitioning OUT of damaged state, destroy rubble overlay
        if (healthRatio > conditionYellow && this->rubbleAnim_0x310 != NULL) {
            this->rubbleAnim->vtable->Expire();
        }
    }
}
```

---

## Summary: What Our Rust Engine Needs

### Currently Implemented
- `ActiveAnim` (×4): parsed from art.ini, rendered as overlays
- `IdleAnim` (×2): looping via global timer
- `ProductionAnim` (×1): plays during factory production
- `SuperAnim`, `SpecialAnim`: parsed but not triggered

### Missing Systems

| Feature | Priority | Complexity | Notes |
|---------|----------|-----------|-------|
| **DamageFireAnims** | High | Low | Separate from 21-slot system. Parse `DamageFireOffset0..7` from art.ini. Spawn FIRE01/02/03 AnimClass at offsets when health < ConditionYellow. Add DamageParticleSystems (SparkSys, SmallGreySSys). |
| **Damaged art switching** | High | Medium | On health crossing ConditionYellow, swap all active anim overlays to their `*D` damaged variants. Need to parse damaged art names from art.ini (convention: append "D" to anim type name, or use separate key). |
| **Power-off anim toggle** | Medium | Medium | 3 flag bytes per slot determine behavior. Flag A: weapon link. Flag B: destroy/recreate. Flag C: disable charge. Most buildings just stop their ActiveAnim. |
| **Cloaking transparency** | Low | Low | Set translucency flag on all 21 anims. Rendering already supports alpha blending. |
| **Turret-facing anims** | Medium | Medium | Slot 10: anim frame driven by turret facing angle. Slot 3–6: SpecialAnim mapped by turret quarter. |
| **Superweapon charge bar** | Low | Medium | Slots 14–20: progressive charge indicator anims. Needs super weapon timer integration. |
| **Repair depot anim** | Medium | Low | Slot 12: plays while unit is being repaired. Already have dock sequence. |
