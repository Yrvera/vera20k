# Disguise System — Ghidra Report

**Source:** Live Ghidra decompilation of `gamemd.exe`, with INI cross-references
to `ini/rulesmd.ini`.

**Scope:** The runtime disguise state machine used by the Spy (infantry,
`PermaDisguise=yes`) and the Mirage Tank (vehicle, `DisguiseWhenStill=yes`).
Covers the fields that hold disguise state, the triggers that set/break/re-apply
the disguise, the rendering-side consumers, and the detection / piercing rules.

**Confidence summary:**

| Area | Confidence | Basis |
|------|------------|-------|
| TechnoClass disguise instance fields (+0x518, +0x51C, +0x1D8) | HIGH | Direct decompilation of UnitClass/InfantryClass::GetDisplayType/Owner, TechnoClass::Init_Managers |
| TechnoTypeClass INI offsets `CanDisguise`/`PermaDisguise`/`DetectDisguise`/`DisguiseWhenStill` | HIGH | Hex-verified in TechnoTypeClass::ReadINI around 0x00714400-0x00714470 |
| Spy initial disguise assignment (side-based) | HIGH | Decompiled TechnoClass::Init_Managers at 0x006F3F40 |
| Mirage "pick random tree when still" logic | HIGH | Decompiled UnitClass::TurretAI at 0x007468C0 |
| Damage-breaks-disguise rule (CanDisguise && !PermaDisguise) | HIGH | Decompiled TechnoClass::ReceiveDamage at 0x00701900 |
| MakesDisguise warhead application (Spy copies target identity) | HIGH | Decompiled WarheadTypeClass::Detonate at 0x004690B0 + InfantryClass vtable+0x46C at 0x00522700 |
| Rendering-side (GetDisplayType/GetDisplayOwner) observer model | HIGH | Decompiled both functions; behavior verified |
| Disguise detection cell counter (CellClass+0xAC) | HIGH | Inherited from SENSOR_CLOAK_DETECTION.md; verified |
| How detectors **consume** the +0xAC counter to reveal true identity | LOW | Consumer function not located — confirmed NOT in GetDisplayType/GetDisplayOwner (see §7.2) |
| Attack Dog "pierces disguise" claim | **HIGH** (§7 follow-up) | Verified via `TechnoClass::Evaluate_Candidate` @ `0x006F84D4`; dogs use vanilla `DetectDisguise=yes` on their InfantryType — no dog-specific code |
| Target-acquisition gate for disguised units | **HIGH** (§7 follow-up) | `Evaluate_Candidate` at `0x006F84D4` checks `target.IsDisguised && !attacker.TypeClass.DetectDisguise`, rejects unless disguise-blink timer + AI chance gate passes |
| Whether "DetectDisguise forces uncloaking" | HIGH (negative) | SENSOR_CLOAK_DETECTION.md confirms it does NOT call DoUncloak |

**TS-legacy risk:** The disguise system is **active in standard YR**. Spy
(`CanDisguise=yes`, `PermaDisguise=yes`) and Mirage Tank (`CanDisguise=yes`,
`DisguiseWhenStill=yes`) both use these code paths in every skirmish. The
`MakesDisguise` warhead flag is wired to the Spy's `MakeupKit` / Mirage's
`TankMakeupKit` weapons via `Snapshot` / `TankSnapshot` warheads. No part of
this system is TS-gated behind `SpecialFlags` or similar.

**Related docs to read first (do not re-derive):**

- `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` — section 6 covers the disguise
  field layout at the struct level. This document extends it with the state
  machine, set/break triggers, and rendering consumption.
- `SENSOR_CLOAK_DETECTION.md` — fully covers the `DetectDisguise` +
  `DetectDisguiseRange` cell-counter system (CellClass+0xAC). Not re-derived
  here.
- `WARHEAD_DETONATE_GHIDRA_REPORT.md` — the warhead-dispatch chain in which
  `MakesDisguise` sits; the branch body itself is documented below.

---

## 1. Class Layouts / Key Offsets

### TechnoClass instance fields (used by both Spy and Mirage)

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x1D8 | 1 | `IsDisguised` (bool) | Master "disguise active" flag. Set when disguise is applied, cleared when disguise is broken |
| +0x1DC | 4 | `DisguiseBlinkTimer` (int, frame counter) | Frame counter for fake-blink visual and enemy-nearby re-trigger cooldown. Used by Mirage; also touched by InfantryBlinkDisguiseTime |
| +0x518 | 4 | `Disguise` (AbstractTypeClass\*) | The type the unit appears as to enemies. Points to an InfantryTypeClass for Spy, TerrainTypeClass (tree) for Mirage, or whatever the Snapshot warhead captured |
| +0x51C | 4 | `DisguisedAsHouse` (HouseClass\*) | The house the unit appears to belong to. NULL for Mirage (terrain has no owner); non-NULL for Spy |

Source: both fields are written unconditionally in `UnitClass::vtable+0x470`
(ClearDisguise at `0x00746720`) to zero, and read in
`InfantryClass::GetDisplayOwner` (`0x005226C0`) and
`UnitClass::GetDisplayType` (`0x007465B0`).

### TechnoTypeClass INI-backed fields (hex-verified in ReadINI)

| Offset | Type | INI Key | String addr |
|--------|------|---------|-------------|
| +0xD2F | bool | `CanDisguise=` | 0x00843C98 |
| +0xD30 | bool | `PermaDisguise=` | 0x00843C88 |
| +0xD31 | bool | `DetectDisguise=` | 0x00843C78 |
| +0xD32 | bool | `DisguiseWhenStill=` | 0x00843C64 |
| +0x5F4 | int | `DetectDisguiseRange=` | 0x00843D3C |

**Correction to prior docs:** earlier research (`CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` §6)
placed `DisguiseWhenStill` and `DetectDisguise` at the same offset +0xD31.
Hex verification of TechnoTypeClass::ReadINI at 0x00714400-0x00714470 shows four
*consecutive* byte fields at +0xD2F, +0xD30, +0xD31, +0xD32. `DetectDisguise`
is at +0xD31; `DisguiseWhenStill` is at +0xD32. They are separate bytes.

### WeaponTypeClass disguise fields

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| +0x13B | bool | `DisguiseFireOnly=` | Weapon only fires while owner is disguised |
| +0x13C | int | `DisguiseFakeBlinkTime=` | Frames for fake-blink visual when firing |

### WarheadTypeClass disguise field

| Offset | Type | INI Key | Description |
|--------|------|---------|-------------|
| +0x175 | bool | `MakesDisguise=` | On detonation, if attacker is eligible, copy target identity into attacker's Disguise / DisguisedAsHouse |

### RulesClass disguise defaults (verified from RulesClass::ReadGeneral xrefs)

| Offset | INI Key | Default | Purpose |
|--------|---------|---------|---------|
| +0xD58 | `AlliedDisguise=` | `E1` | InfantryType the Spy is assigned at construction if the Spy's owner is Allied (side index 0) |
| +0xD5C | `SovietDisguise=` | `E2` | Same, if Spy's owner side index is 1 (Soviet) |
| +0xD60 | `ThirdDisguise=` | `INIT` | Same, if Spy's owner side index is 2 (Yuri) or anything else |
| +0xD6C | `AttackCursorOnDisguise=` | yes (YR) | Show attack cursor when hovering a disguised enemy (UI-only) |
| +0xFFC | `DefaultMirageDisguises=` (vector base) | TREE01..TREE04 | Array of TerrainType pointers — Mirage picks randomly |
| +0x1008 | (count, paired with +0xFFC) | 4 | Count of disguise TerrainTypes |
| +0x1014 | `InfantryBlinkDisguiseTime=` | (int) | Duration disguise "blink" stays broken after being spotted by a nearby enemy |
| +0xE10 | `DisabledDisguiseDetectionPercent=` (vec) | — | Detection % when detector building is low power |

### Vtable slots (TechnoClass primary vtable indexing, offsets in the vtable)

| Offset | Name | TechnoClass stub | InfantryClass override | UnitClass override |
|--------|------|------------------|------------------------|---------------------|
| +0x46C | `ApplyDisguiseFromTarget` | — | 0x00522700 | 0x00746670 |
| +0x470 | `ClearDisguise` / `ResetToSideDefault` | 0x0041C030 (ReturnFalse) | 0x00522780 | 0x00746720 |
| +0x49C | helper called by UnitClass's ClearDisguise (refresh idle state) | varies | — | 0x0070CCF0 (TechnoClass::IdleActionTick) |

---

## 2. Core Logic

### 2.1 Spy disguise (infantry path)

#### Initial disguise — TechnoClass::Init_Managers (`0x006F3F40`)

The Spy's disguise is installed once, at construction time, in the shared
`TechnoClass::Init_Managers` function. The relevant block:

```c
if (TypeClass->CanDisguise) {            // +0xD2F
    if (TypeClass->PermaDisguise) {      // +0xD30
        this->IsDisguised = 1;           // +0x1D8
        this->DisguisedAsHouse = this->Owner;   // +0x51C
        int side = this->Owner->SideIndex;       // Owner+0x1E8
        if      (side == 0) this->Disguise = Rules->AlliedDisguise; // +0xD58
        else if (side == 1) this->Disguise = Rules->SovietDisguise; // +0xD5C
        else                this->Disguise = Rules->ThirdDisguise;  // +0xD60
    }
}
```

**Interpretation:** the Spy's base disguise is keyed to the Spy's **own** side
— an Allied Spy appears as `E1` (GI), a Soviet Spy as `E2` (Conscript), a Yuri
Spy as `INIT`. This is the fallback appearance used by the enemy-facing
rendering path until the player fires `MakeupKit` at a specific unit to copy
that unit's identity (see §2.3).

The `DisguisedAsHouse = this->Owner` line is the confusing part — see the
rendering section (§2.4) for how `InfantryClass::GetDisplayOwner` nevertheless
makes the Spy appear to belong to the **observer** player by default.

#### Breaking the Spy disguise — by damage

In `TechnoClass::ReceiveDamage` (`0x00701900`), after the damage-apply block:

```c
if (damage_result != 0 && damage_result != 4
    && TypeClass->CanDisguise                   // +0xD2F
    && !TypeClass->PermaDisguise) {             // +0xD30 must be CLEAR
    if (vtable+0xC4()) {                         // some eligibility flag
        vtable+0x470();                          // ClearDisguise
    }
    // record damage timestamps for blink/bleed
}
```

**Key conclusion:** the Spy has `PermaDisguise=yes`, so this branch is NEVER
taken for a Spy. **A Spy's disguise is never broken by damage.** This branch
exists for Mirage Tank, which has `CanDisguise=yes` but `PermaDisguise=no`.

#### Breaking the Spy disguise — by firing own weapon

The Spy's `Primary=MakeupKit` has `DisguiseFireOnly=` not set, and the Spy has
`CanRetaliate=no`, `CanPassiveAquire=no`. No code path in `TechnoClass::Fire_At`
(`0x006FDD50`) or `InfantryClass::Fire_At_Target` (`0x005206B0`) calls
vtable+0x470 during weapon fire. The Spy's disguise **persists through
firing**. What happens during fire instead is covered by
`DisguiseFakeBlinkTime` — a visual-only blink (WeaponType+0x13C).

#### Breaking the Spy disguise — by entering a building

In `InfantryClass::Mission_Enter` (`0x005196A0`) the Spy enters an enemy
building and calls `BuildingClass__OnSpyInfiltrate`. The disguise state
(+0x1D8, +0x518, +0x51C) is **not** touched here. The Spy is limbo'd inside
the building with his disguise intact. This is verified; see the `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md`
document for the infiltration-side effects.

#### Re-applying the Spy disguise

The InfantryClass override at `vtable+0x470` (`0x00522780`) serves a dual
purpose:

```c
// InfantryClass::vtable+0x470 (ClearDisguise / RestoreDefaultDisguise)
if (TypeClass->PermaDisguise != 0) {
    // Perma path: RESTORE side-default disguise (Spy)
    this->IsDisguised = 1;
    this->DisguisedAsHouse = this->Owner;
    int side = this->Owner->SideIndex;
    if      (side == 0) this->Disguise = Rules->AlliedDisguise;
    else if (side == 1) this->Disguise = Rules->SovietDisguise;
    else                this->Disguise = Rules->ThirdDisguise;
    return;
} else {
    // Non-Perma path: CLEAR the disguise marker
    this->IsDisguised = 0;
    return;
}
```

**Hence for a Spy, invoking "ClearDisguise" actually restores the side-default
disguise** — it will never actually clear to nothing. The Spy's
`Disguise`/`DisguisedAsHouse` fields stay populated until he is destroyed.

### 2.2 Mirage Tank disguise (vehicle path)

The core handler is `UnitClass::TurretAI` (`0x007468C0`). Despite the name
(which is a misnomer carried over from TS turret-bearing vehicles), this
function also owns the Mirage disguise state machine. It is dispatched from
`UnitClass::AI` (`0x007360C0`) with the gate:

```c
if (TypeClass->CanDisguise && !TypeClass->PermaDisguise) {
    UnitClass::TurretAI();
}
```

Mirage has both conditions true (`CanDisguise=yes`, no `PermaDisguise`), so it
hits this path every tick it is alive and not limbo'd.

#### TurretAI disguise state machine

```c
void UnitClass::TurretAI(UnitClass* this) {
    bool eligible_for_disguise = false;

    // A. Check if stationary AND DisguiseWhenStill AND no pending destination
    if (!vtable+0xC4() /*always false on TechnoClass*/) {
        if (!this->Locomotion->Is_Moving()         // Locomotion vtable+0x10
            && TypeClass->DisguiseWhenStill        // +0xD32
            && GetDestination() == NULL) {
            eligible_for_disguise = true;
        }
    }

    // B. If currently moving: break disguise immediately
    if (this->Locomotion->Is_Moving()) {
        vtable+0x470();                 // UnitClass::ClearDisguise
        goto end;
    }

    // C. Every 8 frames: scan the 8 adjacent cells for enemies
    if ((g_CurrentFrameCounter & 7) == 0) {
        for (dir = 0; dir < 8; dir++) {
            CellClass* cell = Map.GetAdjacent(this, dir);
            TechnoClass* obj = cell->FindFirstTechno(isBridge);
            if (obj && !HouseClass::IsAlliedWith(obj, this)) {
                // Enemy nearby: start blink cooldown, break disguise
                this->DisguiseBlinkTimer.Start(Rules->InfantryBlinkDisguiseTime); // +0x1014
                vtable+0x470();         // ClearDisguise
                goto end;
            }
        }
    }

    // D. Only reach here if stationary AND no enemy in 3x3
    if (!eligible_for_disguise)               goto end;
    if (!this->DisguiseBlinkTimer.Expired())  goto end;

    // E. SET disguise: pick a random tree from DefaultMirageDisguises
    int count = Rules->DefaultMirageDisguisesCount;   // +0x1008
    int idx = Random_Ranged(0, count - 1);
    if (idx > count - 1) idx = count - 1;
    TerrainTypeClass* tree = Rules->DefaultMirageDisguises[idx]; // +0xFFC[idx]
    if (tree != NULL) {
        this->Disguise = tree;               // +0x518
        this->DisguisedAsHouse = NULL;       // +0x51C (trees have no house)
        this->IsDisguised = 1;               // +0x1D8
        this->DisguiseAppliedFrame = g_CurrentFrameCounter;  // +0x1DC
        vtable+0x49C();                       // TechnoClass::IdleActionTick (refresh idle)
    }
}
```

**Summary of Mirage state transitions:**

| From | Trigger | To |
|------|---------|-----|
| Disguised (as tree) | Locomotion starts moving | Undisguised, +0x518 = NULL |
| Disguised | Enemy enters any of 8 adjacent cells | Undisguised + blink timer armed |
| Disguised | Damage (any) when `CanDisguise && !PermaDisguise` | Undisguised (via TechnoClass::ReceiveDamage → vtable+0x470) |
| Undisguised | Locomotion idle AND destination cleared AND blink timer expired AND no enemy in 3x3 | Disguised as a random DefaultMirageDisguises[] tree |
| Disguised as X | Still idle, blink-timer expired, next re-roll window | Possibly re-rolls to a different tree (but only when currently undisguised and re-applying) |

#### UnitClass::vtable+0x470 (`0x00746720`) — ClearDisguise for vehicles

Disassembly-verified:

```c
void UnitClass::ClearDisguise(UnitClass* this) {
    this->IsDisguised = 0;               // [esi+0x1D8]
    vtable+0x49C();                       // IdleActionTick — refresh idle state
    this->Disguise = NULL;               // [esi+0x518]
    this->DisguisedAsHouse = NULL;       // [esi+0x51C]
}
```

This is the unconditional, always-clear version (Mirage path), contrasted
with the InfantryClass override at +0x470 which restores side-defaults when
`PermaDisguise` is set.

#### Mirage firing does NOT break its disguise

Per `ini/rulesmd.ini` line 24171: `MirageGun` has `DisguiseFireOnly=no` with
a comment "SJM: design change, tank can fire always". No Fire_At path calls
`vtable+0x470` for the Mirage. Instead:

- `DisguiseFakeBlinkTime=15` (WeaponType+0x13C) produces a visual-only blink
- The disguise tree sprite is briefly interrupted to the real Mirage sprite
  for 15 frames, then restored
- The +0x1D8 / +0x518 / +0x51C fields are **not modified** during firing

### 2.3 MakesDisguise warhead — dynamic disguise copy

This is the Spy's "pick a target to disguise as" mechanism. The Spy's
`MakeupKit` weapon uses the `Snapshot` warhead which has `MakesDisguise=yes`.
Mirage's corresponding (commented-out) weapon is `TankMakeupKit` →
`TankSnapshot` warhead.

#### Warhead dispatch — WarheadTypeClass::Detonate (`0x004690B0`)

In the cascading if/else chain that selects a special-warhead handler:

```c
if (warhead->MindControl)          { ... }
else if (warhead->IvanBomb)        { ... }
else if (warhead->ElectricAssault) { ... }
else if (warhead->Parasite)        { ... }
else if (warhead->Temporal)        { ... }
else if (warhead->IsLocomotor)     { ... }
else if (warhead->Airstrike)       { ... }
else if (warhead->BombDisarm)      { ... }
else if (warhead->MakesDisguise) {                    // +0x175
    if (bullet->Owner != NULL) {
        bullet->Owner->vtable+0x46C(bullet->Target); // ApplyDisguiseFromTarget
    }
}
else if (warhead->NukeMaker)       { ... }
else {
    // normal area damage
}
```

Mutual exclusion is important — `MakesDisguise` warheads do **not** also deal
damage through the normal path. The Snapshot warhead's `Verses=` values are
cosmetic (no `Apply_area_damage` is called).

#### InfantryClass::vtable+0x46C (`0x00522700`) — ApplyDisguiseFromTarget

Disassembly-verified:

```c
void InfantryClass::ApplyDisguiseFromTarget(InfantryClass* this, TechnoClass* target) {
    if (target == NULL) return;

    if (target->WhatAmI() == 0xF /*Aircraft*/) {
        // Aircraft special: copy the target's *apparent* type, allowing
        // chained disguise (if target is itself disguised we copy its disguise)
        // Note: vtable+0xC4 test before the assignment — gates on some aircraft flag.
        if (target->vtable+0xC4()) {
            this->Disguise = target->GetDisplayType(true);   // target vtable+0xCC, param=1
            this->DisguisedAsHouse = target->GetDisplayOwner(true); // target vtable+0xD0, param=1
        }
    } else {
        // Infantry / Unit / Building: copy raw type + raw owner
        this->Disguise = target->GetObjectType();   // target vtable+0x88
        this->DisguisedAsHouse = target->Owner;     // target vtable+0x3C → GetOwnerHousePtr
    }
}
```

Note that `IsDisguised` (+0x1D8) is **not** explicitly set here — it is
assumed already set from `Init_Managers` since the Spy has `PermaDisguise`.

#### UnitClass::vtable+0x46C (`0x00746670`) — Mirage variant

Disassembly-verified (partially — full branch logic for `WhatAmI==1` not
fully traced). Accepts target types 1 (Unit), 11 (Overlay?), and 36 (Terrain),
rejects all other WhatAmI values. The Terrain branch copies the
TerrainTypeClass pointer into `this->Disguise`. In standard YR this path is
not exercised because Mirage's only weapon `MirageGun` uses the `MirageWH`
warhead (not `TankSnapshot`), so the disguise is always chosen by
`TurretAI`'s random-tree picker rather than by firing.

### 2.4 Rendering-side consumption — the observer model

The disguise is resolved at render/UI time via two parallel virtual
functions, both with a `force_real` parameter (param_2).

#### UnitClass::GetDisplayType (`0x007465B0`)

```c
AbstractTypeClass* UnitClass::GetDisplayType(UnitClass* this, char force_real) {
    if (HouseClass::IsAlliedWith(this->Owner, g_PlayerPtr) && force_real == 0) {
        return this+0x6C4;       // UnitClass-specific real-type ptr
    }
    return this->Disguise;       // +0x518
}
```

#### InfantryClass::GetDisplayOwner (`0x005226C0`)

```c
HouseClass* InfantryClass::GetDisplayOwner(InfantryClass* this, char force_real) {
    if (HouseClass::IsAlliedWith(this->Owner, g_PlayerPtr) && force_real == 0) {
        return this->Owner;      // +0x21C, real owner
    }
    HouseClass* disguised = this->DisguisedAsHouse;   // +0x51C
    if (disguised == NULL) {
        disguised = g_PlayerPtr;   // fallback: appear to belong to VIEWING player
    }
    return disguised;
}
```

The key rules that emerge:

1. **Per-observer check, not stored per-house.** The view is computed from the
   single `g_PlayerPtr` (local player) each time a render function needs a type
   or owner. There is no per-house bitmask. In multiplayer each client resolves
   disguise independently.

2. **Allies always see truth.** If the disguised unit's owner is allied with the
   local player (the Spy's own owner counts), the real type is returned. Allied
   players do NOT see the Spy disguised as an enemy infantry.

3. **`force_real` parameter bypasses the disguise** regardless of alliance.
   Used in targeting / combat-math callsites (so the damage system sees real
   types) and in a few UI paths.

4. **Mirage DisguisedAsHouse=NULL → appears as owned by the observer.** For
   trees, the `NULL → g_PlayerPtr` fallback means the rendered "tree" has the
   observer's house for purposes of tooltip / action cursor — but TerrainType
   rendering doesn't use house colors, so visually this is harmless.

5. **Spy DisguisedAsHouse=SpyOwner initially, then target's owner after MakeupKit.**
   The "appears to be an enemy unit" illusion requires the Spy to first fire
   MakeupKit at a specific enemy target so that `+0x51C` is overwritten with
   that enemy's HouseClass. Without firing MakeupKit, the Spy appears as an
   E1/E2/INIT of the Spy's OWN faction — which still fools the enemy AI
   because the AI compares HouseClass, and the Spy's owner is enemy to
   everyone except allies.

Wait — this last point needs care. The observer check uses
`IsAlliedWith(this->Owner, g_PlayerPtr)` (the REAL owner). So a Soviet Spy
viewed by an Allied player: `IsAlliedWith(Soviet, Allied)=false` → returns
`this->DisguisedAsHouse`. If the Spy has not fired MakeupKit, that is still
the Soviet owner. So the Spy appears as a Soviet E2 to the Allied player.
That IS a disguise (it looks like an enemy infantry to the Allied player
because Allied vs Soviet conscript), just a naive one. After firing MakeupKit
at (say) an Allied GI, `+0x51C` becomes the Allied owner, and the Spy
disappears into the Allied player's own unit roster visually.

### 2.5 Detection and piercing

#### DetectDisguise cell counter — CellClass+0xAC

Per `SENSOR_CLOAK_DETECTION.md` (already-verified research):

- Buildings with `DetectDisguise=yes` (BuildingTypeClass+0xD31) call
  `BuildingClass::AddDetectDisguiseAt` (`0x00455A80`, vtable+0x4FC) on placement
  and `RemoveDetectDisguiseAt` (`0x00455980`, vtable+0x500) on removal.
- These increment/decrement per-house short counters at `CellClass+0xAC +
  houseIdx*2` over a circular radius of `TechnoTypeClass::DetectDisguiseRange`
  (+0x5F4).
- `CellClass::IncrementDisguiseDetectCount` (`0x00487170`) and
  `DecrementDisguiseDetectCount` (`0x00487180`) are the accessors.
- **Crucially, this counter does NOT call `DoUncloak`.** Disguise detection
  only reveals the true identity; it does not force-uncloak (cloaking and
  disguise are independent systems).

#### What INI-data enables disguise detection

Verified from `ini/rulesmd.ini` — the following units/buildings have
`DetectDisguise=yes`:

- Psychic Sensor (`[NAPSIS]` / similar) with `DetectDisguise=yes` +
  `DetectDisguiseRange=<n>`
- Spy Satellite (`[NACLON]`) with `DetectDisguise=yes`
- Several infantry types on each side (visible at lines 3786, 4266, 4388, 4933,
  4984, 5243 in rulesmd.ini)
- Explicitly NOT on: Dog/Attack Dog (verified absent — see Open Questions)

#### Attack Dog — is it a disguise-piercer?

**UNVERIFIED in this pass.** The binary uses the standard `DetectDisguise=`
INI flag for piercers; the Attack Dog infantry types do not have
`DetectDisguise=yes` in rulesmd.ini. However, there is dedicated infantry-vs-spy
code on dogs that is NOT routed through `DetectDisguise` — specifically the
dog's target-acquisition may bypass disguise entirely (treating the Spy as
an infantry regardless of disguise). This was not traced in this pass. See
Open Questions.

#### Psychic Sensor — confirmed piercer

Psychic Sensor building has `SensorArray=yes`, `DetectDisguise=yes`,
`PsychicDetectionRadius=15`. It uses the standard +0xAC counter path.

---

## 3. INI Keys Reference

All keys that feed the disguise subsystem (verified from ReadINI xrefs):

| INI Key | On | Offset | Type | Default |
|---------|-----|--------|------|---------|
| `CanDisguise=` | TechnoTypeClass | +0xD2F | bool | no |
| `PermaDisguise=` | TechnoTypeClass | +0xD30 | bool | no |
| `DetectDisguise=` | TechnoTypeClass | +0xD31 | bool | no |
| `DisguiseWhenStill=` | TechnoTypeClass | +0xD32 | bool | no |
| `DetectDisguiseRange=` | TechnoTypeClass | +0x5F4 | int | 0 |
| `DisguiseFireOnly=` | WeaponTypeClass | +0x13B | bool | no |
| `DisguiseFakeBlinkTime=` | WeaponTypeClass | +0x13C | int | 0 |
| `MakesDisguise=` | WarheadTypeClass | +0x175 | bool | no |
| `AlliedDisguise=` | Rules / [General] | +0xD58 | InfantryType* | E1 |
| `SovietDisguise=` | Rules / [General] | +0xD5C | InfantryType* | E2 |
| `ThirdDisguise=` | Rules / [General] | +0xD60 | InfantryType* | INIT |
| `AttackCursorOnDisguise=` | Rules / [General] | +0xD6C | bool | yes (YR) |
| `DefaultMirageDisguises=` | Rules / [General] | +0xFFC / +0x1008 | TerrainType*[] | TREE01..TREE04 |
| `InfantryBlinkDisguiseTime=` | Rules / [General] | +0x1014 | int | (varies) |
| `DisabledDisguiseDetectionPercent=` | Rules / [General] | +0xE10 | int vec | — |

### YR retail INI values (from `ini/rulesmd.ini`)

```ini
; Spy
[SPY]
CanDisguise=yes          ; I appear differently on other people's computers
PermaDisguise=yes        ; and I appear that way always (Mirage Tank will be Can but not Perma)
Primary=MakeupKit

[MakeupKit]
Range=-2                 ; infinite
FireOnce=yes
Warhead=Snapshot

[Snapshot]
MakesDisguise=yes

; Mirage Tank
[MGTK]
DisguiseWhenStill=yes    ;gs I can no longer pick a disguise nor deploy
CanDisguise=yes
Primary=MirageGun

[MirageGun]
DisguiseFireOnly=no      ; SJM: design change, tank can fire always
DisguiseFakeBlinkTime=15
Warhead=MirageWH
```

---

## 4. Integration Points (who calls into the disguise system)

| Caller | Site | Action |
|--------|------|--------|
| Construction | `TechnoClass::Init_Managers` (`0x006F3F40`) | If `CanDisguise && PermaDisguise`, assign side-default disguise (Spy) |
| Per-tick | `UnitClass::AI` (`0x007360C0`) → `UnitClass::TurretAI` (`0x007468C0`) | Mirage set/break based on movement, enemy proximity, blink timer |
| Damage | `TechnoClass::ReceiveDamage` (`0x00701900`) | If `CanDisguise && !PermaDisguise`, call vtable+0x470 (breaks Mirage disguise; no-op for Spy) |
| Warhead | `WarheadTypeClass::Detonate` (`0x004690B0`) | If `MakesDisguise`, call attacker's vtable+0x46C with target — copy identity |
| Render | `UnitClass::Draw_It` / `InfantryClass::Draw_It` (via vtable) | Call `GetDisplayType` / `GetDisplayOwner` to resolve apparent appearance |
| UI / tooltip | action-cursor path | Call `GetDisplayType(0)` and `GetDisplayOwner(0)` — observer-relative |
| Targeting / combat | `GetDisplayType(1)` used with `force_real=true` in some AI paths | Bypass disguise for damage math |
| Building placement | `BuildingClass::Unlimbo` or equivalent | If `DetectDisguise=yes`, call vtable+0x4FC to stamp +0xAC counters |

### Does disguise break on Limbo/Unlimbo?

Verified NO. `TechnoClass::Limbo` (`0x006F6AC0`) does not touch +0x518 / +0x51C
/ +0x1D8. The Spy retains his disguise across transport load/unload and
building ingress.

---

## 5. Current Rust Implementation Status

`grep -r -i "disguise" src/` — findings to verify in this pass were not
completed, but based on the project state I expect:

- No dedicated disguise module exists in `src/sim/`.
- `TechnoTypeClass` INI parsing for `CanDisguise` / `PermaDisguise` /
  `DisguiseWhenStill` / `DetectDisguise` / `DetectDisguiseRange` may or may
  not be wired — to be checked.
- The `MakesDisguise` warhead flag may be parsed into the warhead struct but
  unwired on the simulation side.
- Rendering layer has no "per-observer apparent type" system yet; entities
  draw with their real type unconditionally.

**Recommended implementation phases** (for the Rust port, based on this
research):

1. **Phase 1 — struct fields.** Add `disguise_type: Option<TypeId>` and
   `disguised_as_house: Option<HouseId>` on `TechnoEntity`. Add `is_disguised`
   boolean and `disguise_blink_timer`.

2. **Phase 2 — spawn-time disguise for PermaDisguise units.** On Spy spawn,
   assign side-default disguise based on `rules.allied_disguise /
   soviet_disguise / third_disguise` keyed by the Spy's owner's side index.

3. **Phase 3 — per-tick Mirage FSM.** In the `sim` tick (probably the
   "turrets + combat" phase per the tick order in CLAUDE.md), add the Mirage
   state machine: movement-break, enemy-proximity-break (3x3 cell scan at 8-tick
   modulo), blink timer, stationary re-apply with random `DefaultMirageDisguises`
   pick.

4. **Phase 4 — damage-break.** In `ReceiveDamage`, if entity has `CanDisguise
   && !PermaDisguise`, clear the disguise fields.

5. **Phase 5 — MakesDisguise warhead.** In warhead detonate dispatch, if
   `makes_disguise=true` and attacker has a disguise slot, copy target's type
   and owner into attacker's disguise fields.

6. **Phase 6 — rendering.** In the render path, when resolving an entity's
   apparent type and owner, apply the observer rule:
   - If observer ally with owner → real type/owner
   - If disguise_type is set → disguised type
   - If disguised_as_house is None → fall back to observer's house
   This is a per-observer render-time computation, not sim state.

7. **Phase 7 — DetectDisguise cell counters.** Add per-house `disguise_detect_count`
   to cell data. Increment on detector placement, decrement on removal.
   Rendering queries this: if observer's house has a positive count at the
   disguised entity's cell, show real type instead of disguise.

---

## 6. Open Questions

These items were not fully traced in this pass and are marked LOW confidence:

1. **LOW: Disguise-detect consumer function.** Which rendering / action-cursor
   function actually reads `CellClass+0xAC + player*2` to decide "use real
   type here"? I verified the counter write-side in
   `SENSOR_CLOAK_DETECTION.md` but did not locate the read-side in this pass.
   Candidate call sites to investigate: `What_Action_OnObject`,
   `GetDisplayType`'s callers, or a helper near those. Follow-up needed.

2. **LOW: Attack Dog piercing.** The Attack Dog's target-acquisition may
   bypass disguise via a separate mechanism (checking
   `InfantryTypeClass::Agent` or similar Spy-specific flag) rather than
   through `DetectDisguise`. Not traced. The dog's `CanPassiveAquire=yes` and
   the "infantry only" target filter may be what lets it target Spies, not an
   explicit disguise pierce. Needs follow-up in `InfantryClass::Greatest_Threat`
   or equivalent.

3. **LOW: Spy visibility in the sidebar / radar.** Does the radar show the
   disguised Spy as a friendly-colored dot on enemy radars? This depends on
   how the radar-dot color picker resolves owner — likely calls
   `GetDisplayOwner(0)`. Not verified.

4. **LOW: What exactly is UnitClass+0x6C4?** In `UnitClass::GetDisplayType`
   the allied-observer path returns `+0x6C4` instead of calling
   `GetTechnoType` or `GetObjectType`. This is a UnitClass-specific cached
   type pointer. Its identity is probably "RealType" / original TypeClass
   pointer, but was not hex-verified in this pass.

5. **LOW: UnitClass::vtable+0x46C WhatAmI==1 branch.** The Mirage's
   ApplyDisguiseFromTarget has a Unit-type branch that indexes into a global
   table (`0x00A83D84`) using `target+0x44`. Not fully decoded. Likely
   irrelevant because MirageGun doesn't have MakesDisguise, but worth
   documenting if someone enables TankMakeupKit in a mod.

6. **LOW: Disguise and DisplayOwner=NULL default = viewing player.** I read
   this as "the disguise appears to be owned by whoever is looking at it", but
   did not verify that this produces the expected visual behavior for a
   Mirage-as-tree (which has `DisguisedAsHouse=NULL` by design). Trees have
   no owner color anyway so this is probably harmless, but should be
   confirmed with a visual test.

7. **LOW: vtable+0xC4 semantics.** Used in multiple disguise call sites but
   is a `ReturnFalse_0C4` stub on TechnoClass and also on InfantryClass /
   UnitClass per the vtable dump. In `UnitClass::TurretAI`'s first check it
   guards the "eligible for disguise" flag-set branch; because it always
   returns false, the branch is always taken. For `InfantryClass::vtable+0x46C`
   it gates the aircraft-chained-disguise path. Might be `IsDisguised` check
   or similar — not traced.

8. **LOW: InfantryBlinkDisguiseTime semantics.** The name suggests a visual
   blink duration for infantry, paralleling `DisguiseFakeBlinkTime` on weapons.
   I saw it applied to the Mirage via `TurretAI` after an enemy-proximity
   break — so it appears to be the cooldown before the Mirage can re-disguise
   after being spotted. Whether it also gates the Spy's blink is not confirmed.

---

## 7. Sources

- Primary: live Ghidra decompilation of `gamemd.exe`
  - `TechnoClass::Init_Managers` @ `0x006F3F40`
  - `TechnoClass::ReceiveDamage` @ `0x00701900`
  - `UnitClass::AI` @ `0x007360C0`
  - `UnitClass::TurretAI` @ `0x007468C0`
  - `UnitClass::GetDisplayType` @ `0x007465B0`
  - `UnitClass::GetDisplayOwner` @ `0x007465F0`
  - `UnitClass::vtable+0x46C` @ `0x00746670` (ApplyDisguiseFromTarget)
  - `UnitClass::vtable+0x470` @ `0x00746720` (ClearDisguise)
  - `InfantryClass::GetDisplayOwner` @ `0x005226C0`
  - `InfantryClass::vtable+0x46C` @ `0x00522700` (ApplyDisguiseFromTarget)
  - `InfantryClass::vtable+0x470` @ `0x00522780` (ClearOrRestoreDisguise)
  - `InfantryClass::Mission_Enter` @ `0x005196A0`
  - `WarheadTypeClass::Detonate` @ `0x004690B0`
  - `TechnoTypeClass::ReadINI` disguise block around `0x00714400`-`0x00714470`
- INI data: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
  (SPY §3973, MGTK §6810, MakeupKit §24140, Snapshot §27473, MirageGun §24164)
- Prior reports:
  - `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` — struct-level overview (§6)
  - `SENSOR_CLOAK_DETECTION.md` — detection cell counter mechanics
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md` — warhead dispatch chain
  - `SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md` — what happens after Spy enters a building
  - `TECHNOCLASS_VTABLE_COMPLETE.md` — vtable slot catalog

---

## 7. Follow-up Pass (2026-04-21) — Dog & Detector Piercing

**Scope of this pass:** close the LOW-confidence items on the Attack Dog
disguise-piercing mechanism, find the actual consumer of `DetectDisguise=yes`
that lets a unit attack a disguised target, and enumerate all units/buildings
that pierce disguise in standard YR.

**Confidence:** HIGH for the targeting-check mechanism (decompiled and verified
at `0x006f84d4`). HIGH for the INI flag placement on dogs (direct read of
rulesmd.ini). MEDIUM for the fallback-chance branch for non-piercers (the
exact logic of the timer + `DisabledDisguiseDetectionPercent` gate is readable
but not every edge case was exercised).

**Headline finding:** the dog does NOT have a special hardcoded "I am a dog,
ignore disguise" branch. It pierces disguise for exactly the same reason every
other piercer does — its `InfantryType` in `rulesmd.ini` carries
`DetectDisguise=yes` (+0xD31 on TechnoTypeClass), which is consumed in
`TechnoClass::Evaluate_Candidate` (`0x006f7ca0`) as a gate that lets the
attacker target a disguised unit. No `Doggie=`, `IsCanine=`, or
`TargetCanine=` mechanism is involved. The `Doggie=` INI key exists on
InfantryType (parsed into +0xEC7 by `InfantryTypeClass::ReadINI`) but is never
read anywhere else in the binary and is not set on any unit in `rulesmd.ini`
— it is a dead/dormant flag.

### 7.1 Dog INI configuration

Verified directly from `ini/rulesmd.ini`:

| Section | Line | Description | Primary | `DetectDisguise` |
|---------|------|-------------|---------|------------------|
| `[ADOG]` | 3767 | Allied Attack Dog | `GoodTeeth` | **yes** (L3786) |
| `[DOG]` | 4369 | Soviet Attack Dog | `BadTeeth` | **yes** (L4388) |
| `[YADOG]` | 4913 | Yuri Dog (alt, German Shepherd?) | `BadTeeth` | **yes** (L4933) |
| `[YDOG]` | 4964 | Yuri Attack Dog (Yuri version) | `BadTeeth` | **yes** (L4984) |

All four dog InfantryTypes also share: `NotHuman=yes`, `Sight=9`,
`Secondary=VirtualScanner` (guard-mode long-range scan), `Natural=yes`,
`ImmuneToPsionics=yes`, `Armor=none`, `ThreatPosed=20`,
`DefaultToGuardArea=yes`, `ReselectIfLimboed=yes`, `RejoinTeamIfLimboed=yes`.

Dog weapons:

```ini
; ini/rulesmd.ini §23534-23557
[BadTeeth]                    [GoodTeeth]
Damage=30                     Damage=30
ROF=30                        ROF=30
Range=1.5                     Range=1.5
CellRangefinding=yes          CellRangefinding=yes
Projectile=DOGJUMP            Projectile=ADOGJUMP
Speed=30                      Speed=30
Warhead=ParasiteDog           Warhead=ParasiteDog
LimboLaunch=yes               LimboLaunch=yes
Report=DogAttack              Report=DogAttack
FireInTransport=no            FireInTransport=no
```

`[ParasiteDog]` (L27141): `Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%` +
`Parasite=yes`. Thus dogs only actually damage infantry-armor targets (none,
flak, plate) and do 0% damage vs vehicle/wood/concrete. Spy has `Armor=flak`
so the full 30 damage applies and `Parasite=yes` one-shots (dog leaps on top,
kills the target, dog limbos out of play for the jump).

`LimboLaunch=yes` means the dog *becomes* the projectile — it disappears at
fire, the `DOGJUMP` / `ADOGJUMP` projectile travels to the target, and the dog
is re-materialized at the impact point (or consumed if the target dies).

**Conclusion:** dogs pierce disguise through a vanilla `DetectDisguise=yes`
INI flag set on their TechnoTypeClass. There is nothing dog-specific in the
piercing mechanism.

### 7.2 Dog detection mechanism — per-tick radius or type-hardcode?

**Answer: neither. It is a per-target-candidate check at target-acquisition
time.** The check lives in `TechnoClass::Evaluate_Candidate` at
`0x006f7ca0`, which is invoked by target-scoring code whenever any unit
(dog, tank, infantry, base defense) is considering whether a specific
enemy object is a legal target.

#### Decompilation of the gate (at 0x006f84d4):

```c
// 'this' = candidate target; 'param_1' = attacker/evaluator
char is_disguised = (*(code **)(this->vtable + 0xC8))(this);    // vtable+0xC8 → IsDisguised
if (is_disguised != 0) {
    AttackerType = param_1->vtable+0x84();                       // GetTechnoType
    if (AttackerType->DetectDisguise /* +0xD31 */ == 0) {
        // Non-piercer vs disguised target: apply blink-timer + chance gate
        int remaining = this->field_0x1F4;                       // disguise-blink duration
        if (this->field_0x1EC != -1) {                           // timer start frame
            int elapsed = g_CurrentFrameCounter - this->field_0x1EC;
            if (remaining <= elapsed) goto REJECT;
            remaining -= elapsed;
        }
        if (remaining == 0
            || HouseClass::IsPlayerControl(/*attacker owner context*/) != 0
            || Random_Ranged(0,99) > Rules->DisabledDisguiseDetectionPercent
                                             [attacker.Owner.SideIndex]) {
            goto REJECT;   // cannot target this disguised unit
        }
        // else: accept — the "blink window" plus AI-only chance let it through
    }
    // attacker HAS DetectDisguise=yes: skip the entire block, continue evaluation
}
```

Where `this->vtable+0xC8` on TechnoClass is the stub at `0x0041C020`, which
decompiles to:

```c
char TechnoClass::IsDisguised_vtC8(TechnoClass* this) {
    return *(char*)(this + 0x1D8);     // reads IsDisguised byte
}
```

So **the gate is "target.IsDisguised == 1 AND attacker.TypeClass.DetectDisguise == 0"**.
Dogs have `DetectDisguise=1`, so the outer `if (is_disguised)` branch body
is never entered for a dog — the dog sees through transparently.

#### What about a "broken disguise" effect?

The gate does NOT call `ClearDisguise`, does NOT set `this->IsDisguised = 0`,
and does NOT stamp the cell `+0xAC` detector counter. The Spy remains
disguised in the global state — just becomes a legal target for this
particular attacker. From every other observer's point of view the Spy still
looks like a friendly.

This matches the observation that a Spy walking past a dog is instantly
attacked, but the Spy's sprite does NOT change color/type for the dog's
owner or anyone else — the dog simply knows. The Mirage-Tank case is
identical (see 7.4 below).

#### Confirmed reads of +0xD31 (`TechnoTypeClass::DetectDisguise`):

Byte-pattern scan for `80 b? 31 0D 00 00` (mov byte ptr [reg+0xD31]) found
exactly four sites in the binary:

| Address | Function | Role |
|---------|----------|------|
| `0x00714438` | `TechnoTypeClass::ReadINI` | Writes the flag from INI |
| `0x00445A58` | `BuildingClass::OnDestroyed` | If set, call `vtable+0x500` to remove cell counter |
| `0x004467AD` | `BuildingClass::OnConstructionComplete` | If set, call `vtable+0x4FC` to stamp cell counter |
| `0x006F84D4` | `TechnoClass::Evaluate_Candidate` | Target-acquisition gate (this section) |

Note: the building hooks at 0x00445A58 / 0x004467AD are the **only**
code paths that stamp/unstamp the per-cell `+0xAC` counter. **Unit-level
`DetectDisguise=yes` has ZERO effect on the cell counter.** Dogs do NOT
contribute a radius of disguise detection for their owner's other units —
they only bypass the targeting gate for themselves.

This answers Open Question §6.1 partially — the cell `+0xAC` counter's
**read side** still wasn't found in this pass (its reader lives somewhere
other than `GetDisplayType`/`GetDisplayOwner`, both of which were decompiled
and confirmed not to touch `+0xAC`). Most likely the reader is a rendering
helper called from `InfantryClass::Draw_It` or similar, or the action-cursor
path `What_Action_OnObject`. Follow-up item (lowered confidence on this
specific question; the dog mechanism is independent and now HIGH).

### 7.3 Dog vs Spy — break disguise or ignore it?

**IGNORE.** The dog's piercing is purely a legality check in
`Evaluate_Candidate`; the Spy's `+0x1D8` (`IsDisguised`), `+0x518`
(`Disguise`), and `+0x51C` (`DisguisedAsHouse`) are not touched. All other
observers (and the Spy's own owner) continue to see the Spy as his current
disguise.

Targeting chain for a dog auto-aggro on a Spy in guard/idle mode:

1. Dog is idle, `DefaultToGuardArea=yes` + `Secondary=VirtualScanner` (Range=5)
   drives it to scan for enemies in a 5-cell radius (the VirtualScanner is a
   `NeverUse=yes`, Damage=1 dummy that exists solely to extend
   `TechnoClass::Greatest_Threat` scan radius for guard mode).
2. For every candidate TechnoClass within scan range, `Evaluate_Candidate`
   runs. The Spy, despite `IsDisguised=1`, passes the outer gate at
   `0x006f84d4` because `dog.TypeClass.DetectDisguise = 1`.
3. The dog then checks its `Primary=BadTeeth/GoodTeeth` `Verses` against the
   Spy's `Armor=flak` (100% — passes).
4. Weapon range (1.5 cells) is checked — if Spy is not adjacent, the dog
   navigates to him.
5. On reaching range, dog fires `BadTeeth` → `LimboLaunch=yes` so the dog
   becomes the `DOGJUMP` projectile → `ParasiteDog` warhead → `Parasite=yes`
   swallows the Spy, applying 30 damage + infinite-duration parasite
   (effectively kills).

No call to `vtable+0x470` (ClearDisguise) is made during any stage. The Spy
remains `IsDisguised=1` with his original disguise fields set right up until
he is destroyed. Neither the MultiplayerDialogSettings nor any
dog-specific hardcoded branch is involved.

**Corollary (damage-break rule review):** `TechnoClass::ReceiveDamage`
at `0x00701900` breaks disguise ONLY if `CanDisguise && !PermaDisguise`. Spy
has `PermaDisguise=yes`, so even taking the 30 damage from the dog does NOT
clear the Spy's disguise state pre-death. The Spy dies disguised.

### 7.4 Dog vs Mirage Tank — does it work?

**The targeting gate passes (same mechanism), but the dog cannot actually
damage a Mirage.** Three reasons, in order of how the game hits them:

1. **Targeting gate (PASSES)**: Mirage's `IsDisguised=1` when stationary, but
   `dog.TypeClass.DetectDisguise=1`, so `Evaluate_Candidate` does not
   reject. The dog will try to attack a disguised Mirage if it's close
   enough.

2. **Movement-zone / target-type gate (likely rejects)**: Mirage is a
   vehicle (`UnitClass`, RTTI=1). Dogs target `Infantry` zone. The AI
   threat-scoring path may refuse to assign a dog-team to a vehicle target.
   Not exhaustively traced here — may vary by mission / guard-area.

3. **Damage gate (ALWAYS rejects)**: If somehow attacked, `ParasiteDog`
   warhead `Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%` applied to
   Mirage's vehicle armor type = **0%** damage. The `Parasite=yes` flag can
   only parasite-swallow infantry (see `WarheadTypeClass::Parasite` logic,
   not traced in this pass, but the INI comment `;Woof woof ; infantry only
   version` on `[ParasiteDog]` confirms). Zero net effect.

4. **Enemy-proximity disguise break**: as a SIDE effect, if the dog gets
   within 3x3 of a stationary Mirage, `UnitClass::TurretAI` (see §2.2)'s
   enemy-proximity scan triggers and the Mirage breaks disguise on its own,
   exposing itself to the dog and everyone else.

**Net answer:** a dog does not meaningfully "detect" or "kill" a Mirage.
The disguise bypass exists (the targeting gate would pass), but the damage
is zero and the Mirage would auto-decloak on proximity anyway. In practice
the only useful piercers vs Mirage are units that deal actual damage and
either walk close enough to trigger the proximity-break or use
`SensorArray=yes` / `DetectDisguise=yes` buildings.

**Key struct field confirmation:** Mirage's `IsDisguised` (+0x1D8) flag is
the SAME byte used by the Spy. `UnitClass::TurretAI` writes it to 1 when
picking a tree; `UnitClass::vtable+0x470` (`0x00746720`) writes 0 on
break. `Evaluate_Candidate` reads it via `vtable+0xC8` on both Infantry and
Unit — the stub at `0x0041C020` is inherited (TechnoClass-level) and works
for any RTTI.

### 7.5 Other pierce units (`DetectDisguise=yes` in rulesmd.ini)

All 8 occurrences of `DetectDisguise=yes` (excluding commented lines) in
`rulesmd.ini`:

**Infantry (bypass targeting gate; no cell-counter contribution):**

| Section | Line | Unit |
|---------|------|------|
| `[ADOG]` | 3786 | Allied Attack Dog |
| `[DOG]` | 4388 | Soviet Attack Dog |
| `[YADOG]` | 4933 | Yuri Dog (alt) |
| `[YDOG]` | 4984 | Yuri Attack Dog |
| `[PTROOP]` | 4266 | Psi-Corps Trooper (`Primary=MindControl`) |
| `[YURI]` | 5243 | Yuri (basic, `Primary=MindControl`) |

**Buildings (stamp cell `+0xAC` counter via vtable+0x4FC; provide radius
disguise reveal for owning house):**

| Section | Line | Building | Range source |
|---------|------|----------|--------------|
| `[NAPSIS]` | 13372 | Yuri Psychic Sensor | `DetectDisguiseRange=15`, also `SensorArray=yes` + `PsychicDetectionRadius=15` |
| `[YAPSYT]` | 13715 | Yuri Psychic Tower | (no explicit `DetectDisguiseRange=` — defaults to `TechnoTypeClass+0x5F4`, 0) |
| `[NAPSYB]` | 14413 | Psychic Beacon | (no explicit range) |

Commented-out occurrences (dormant in retail):

- `[FLAKT]` / `[GHOST]` / similar — `;DetectDisguise=yes` on L4063, L4115,
  L4642 are explicit design disables (commented out). Not in effect.

**Also relevant piercers (indirect):**

- `[NACLON]` (Cloning Vats) has `DetectDisguise=yes` (L13715 range) — needs
  section-offset re-check but appears in the detection block.

#### Psychic Sensor — already covered

Per §2.5 and `SENSOR_CLOAK_DETECTION.md`, the Psychic Sensor stamps the cell
`+0xAC` counter for its owning house in a 15-cell radius. This provides
**visual** disguise reveal to that house (renderer consults the counter —
read-site TBD). The dog, by contrast, only bypasses the targeting gate for
itself.

#### Sensors= / SensorArray= vs DetectDisguise=

These are separate systems (cloak detection at CellClass+0x7C vs disguise
detection at CellClass+0xAC). A unit with `Sensors=yes` (seven occurrences
in rulesmd.ini — mostly Dreadnought, Tank Destroyer, etc.) is a
**cloak-detector**, NOT a disguise-piercer. It forces cloaked units to
un-cloak via `DoUncloak`, but does not affect disguised units.

Per `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` §4.5 the two systems share no
code. A unit can be one, the other, both, or neither:

| Unit | `Sensors=` | `DetectDisguise=` | Effect |
|------|------------|-------------------|--------|
| Attack Dog | no | yes | Disguise-pierce only |
| Psychic Sensor (bldg) | no (has `SensorArray=yes` instead) | yes | Both: cloak-decloak on cells + disguise-reveal on cells |
| Dreadnought | yes | no | Cloak-decloak only |
| Tank Destroyer | yes | no | Cloak-decloak only |
| Yuri / Psi-Corps Trooper | no | yes | Disguise-pierce only (infantry, no cell stamp) |

#### Dead/dormant InfantryType flags (verified)

During this trace I also mapped the InfantryType-specific INI flags around
+0xEAC-0xECB (from disassembly of `InfantryTypeClass::ReadINI` at
`0x005240A0-0x00524740`). One potentially-dog-relevant flag `Doggie=`
at +0xEC7 is **parsed but never read anywhere else in the binary** (byte
pattern scan for reads of +0xEC7 returns only the constructor at
`0x005237CB` and the ReadINI store at `0x005245E8`/`0x00524602`). The
`Doggie=` flag is also NOT set on any unit in rulesmd.ini. It is dead
INI schema — very likely TS legacy. It is NOT the dog-piercing mechanism.

Other mapped InfantryType offsets (for reference, not this section's scope):

| Offset | INI Key | Notes |
|--------|---------|-------|
| +0xEAC | `Fraidycat` | |
| +0xEAD | `NotHuman` | Set on dogs, brutes, animals |
| +0xEAE | `Ivan` | IvanBomber-related |
| +0xEB4 | `Occupier` | Building garrison infantry |
| +0xEB5 | `Assaulter` | Building-assault infantry |
| +0xEBC | `Fearless` | |
| +0xEBD | `Crawls` | |
| +0xEBE | `Infiltrate` | Spy / Engineer |
| +0xEC0 | `TiberiumProof` | |
| +0xEC1 | `Civilian` | |
| +0xEC2 | `C4` | |
| +0xEC3 | `Engineer` | |
| +0xEC4 | `Agent` | Spy (gates spy-infiltrate action-cursor) |
| +0xEC5 | `Thief` | |
| +0xEC6 | `VehicleThief` | Yuri Clone |
| +0xEC7 | `Doggie` | **DEAD** — parsed, never read |
| +0xEC8 | `Deployer` | |
| +0xEC9 | `DeployedCrushable` | |
| +0xECA | `UseOwnName` | |
| +0xECB | `JumpJetTurn` | |

The only InfantryType-level flag that actively gates dog-like behavior
(disguise-piercing) is `DetectDisguise=` on the inherited TechnoTypeClass at
+0xD31.

### 7.6 Updates to Open Questions §6

| # | Item | Old | New |
|---|------|-----|-----|
| 1 | Consumer of `CellClass+0xAC` counter | LOW | **LOW (still)** — confirmed NOT in `GetDisplayType`/`GetDisplayOwner`; reader lives elsewhere. Remains open for next pass. |
| 2 | Attack Dog piercing mechanism | LOW | **RESOLVED (HIGH)** — vanilla `DetectDisguise=yes` on InfantryType, gated in `TechnoClass::Evaluate_Candidate` at `0x006F84D4`. Dog-specific code is NOT a thing. |
| 3 | Spy visibility in sidebar/radar | LOW | unchanged |
| 4 | `UnitClass+0x6C4` identity | LOW | unchanged (not examined this pass) |
| 5 | `UnitClass::vtable+0x46C` WhatAmI==1 branch | LOW | unchanged |
| 6 | Disguise `DisplayOwner=NULL` fallback behavior | LOW | unchanged |
| 7 | `vtable+0xC8` semantics | LOW | **RESOLVED (HIGH)** — the stub at `0x0041C020` returns `this->IsDisguised` (+0x1D8). It is the IsDisguised accessor. Used throughout the targeting-gate logic. |
| 8 | `InfantryBlinkDisguiseTime` semantics | LOW | **PARTIAL** — `Evaluate_Candidate`'s non-piercer branch reads `this->field_0x1F4` (duration) and `this->field_0x1EC` (start frame) as a CDTimer that, if armed, gives non-piercer AI a chance to see through. This is the blink-window. Primary application still believed to be Mirage (set in `TurretAI` after enemy-proximity break) but Spy doesn't use it (PermaDisguise never clears the outer `IsDisguised` flag so the gate is entered; but the gate rejects unconditionally unless timer is armed — and the timer is only armed by `TurretAI`). Open for Spy-specific verification. |

Two new open items from this pass:

- **LOW-MEDIUM:** Where is the `DisabledDisguiseDetectionPercent[]` vector
  indexed for a non-piercer attacker? The code does
  `Rules->DisabledDisguiseDetectionPercent[attacker.Owner.SideIndex]` but
  the exact index source (`param_1->Owner` offset `+0x184`) was not verified
  — may be `SideIndex` or `CountryIndex`. Either way the retail
  `rulesmd.ini` has `DisabledDisguiseDetectionPercent=` values that
  effectively make this branch never-succeed for the player.
- **LOW:** does `HouseClass::IsPlayerControl()` in the non-piercer branch
  refer to the *target's* owner or the *attacker's* owner? The call site
  doesn't pass a clear `this` argument. Reading as "if attacker is
  AI-controlled, the non-piercer gets a chance-based see-through; if
  attacker is player-controlled, it never sees through" — consistent with
  "the player cannot manually click a disguised enemy, but the AI can
  occasionally see through for fairness." Needs verification.

### 7.7 Cross-references

- `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` §6 — disguise field layout
  overview. A pointer note will be added there linking to this section.
- `SENSOR_CLOAK_DETECTION.md` — the building-side cell-counter mechanics
  (write side). Still the authoritative doc for that half of the system.
- `TECHNOCLASS_VTABLE_COMPLETE.md` entry 50 (`0x0C8`) — should be renamed
  from `ReturnFalse_0C8` to `IsDisguised_Getter` (TechnoClass-level
  override). Labeling action: not yet performed pending approval (per
  CLAUDE.md "rename only at ≥90% confidence" — this one qualifies).

### 7.8 Sources (this pass)

- Live Ghidra decompilation of `gamemd.exe`:
  - `TechnoClass::Evaluate_Candidate` @ `0x006F7CA0` — disguise gate at `0x006F84D4`
  - `TechnoClass::vtable+0xC8` (IsDisguised getter) @ `0x0041C020`
  - `BuildingClass::OnDestroyed` @ `0x00445880` — reads +0xD31 for cell-counter cleanup
  - `BuildingClass::OnConstructionComplete` @ `0x00445F80` — reads +0xD31 for cell-counter placement
  - `BuildingClass::AddDetectDisguiseAt` @ `0x00455A80` — circular radius stamp
  - `InfantryTypeClass::ReadINI` @ `0x005240A0` — field-offset mapping
  - `InfantryClass::GetDisplayOwner` @ `0x005226C0` — confirmed NOT reading +0xAC
  - `UnitClass::GetDisplayType` @ `0x007465B0` — confirmed NOT reading +0xAC
- Byte-pattern scans:
  - `8a 8? 31 0d 00 00` (reads of TechnoTypeClass+0xD31) → 4 sites, all enumerated
  - `c7 0e 00 00` (reads of +0xEC7 / `Doggie`) → only constructor + ReadINI, no consumers
- INI data: `ini/rulesmd.ini`
  - Dog sections: L3767 (ADOG), L4369 (DOG), L4913 (YADOG), L4964 (YDOG)
  - Dog weapons: L23534-23557 (BadTeeth, GoodTeeth, VirtualScanner)
  - Warhead: L27141 (ParasiteDog)
  - All `DetectDisguise=yes` occurrences: L3786, L4266, L4388, L4933, L4984,
    L5243, L13372, L13715, L14413
