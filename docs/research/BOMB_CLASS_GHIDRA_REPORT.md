# BombClass — Ghidra Research Report (Crazy Ivan Demolition Charge)

**Primary addresses:**
- Primary ctor (stream/load construction): `0x004385D0`
- Attach-bomb factory (the real "plant a bomb" entry point): `0x00438E70`
- Detonate: `0x00438720`
- Defuse: `0x004389B0`
- IsTimerExpired: `0x00438A70`
- GetClockFrame: `0x00438A00`
- UpdateAll (per-tick housekeeping): `0x00438BF0`
- Scalar Deleting Destructor: `0x004393F0`
- Primary vtable: `0x007E3D10`
- Instance size: **0x5C bytes** (92)

**Confidence:** HIGH. All field offsets, the full detonation/defuse/clock pipeline,
and the INI wiring verified by direct decompilation and assembly-level inspection
of gamemd.exe. A handful of sound-handle and serialization sub-fields are marked MEDIUM
where only the call signatures (not the internal struct) were inspected.

**Active in YR:** Yes. BombClass is YR-era code — Crazy Ivan/Chrono Ivan are YR units,
and `IvanBomber` is the only live caller of the attach path. No Tiberian Sun gating.
No `SpecialFlags` guard on any BombClass code. The class file-wide is live content.

The one TS-adjacent gate worth noting is `Rules->CanDetonateTimeBomb` /
`CanDetonateDeathBomb` (both default `no` in rules(md).ini). They gate the player's
double-click "detonate-own-bomb" input event (EventClass case 10 → BombClass::Detonate).
The fuse-expiry, clock overlay, and damage pipeline do not touch them.

---

## 1. Purpose

BombClass represents one Crazy Ivan demolition charge attached to a single carrier
Techno. It:
1. Tracks which Techno it is attached to (Target) and who planted it (Attacker, HouseOwner).
2. Runs a timed fuse on the current frame counter.
3. Drives the 13-frame CHRONOSK.SHP clock overlay drawn over the carrier.
4. Manages the BombTickingSound looping voc while the carrier is on-map.
5. Maintains visibility via Rules->BombSight on opposing infantry (the "BombVisible"
   flag on the target).
6. On fuse expiry, calls `Apply_area_damage(target.coords, IvanDamage, attacker,
   IvanWarhead)` and also destroys any bridge under a bombed building.
7. On carrier death (in TechnoClass::ReceiveDamage) or UnInit, detonates or defuses.

---

## 2. Struct Layout — 0x5C bytes (verified from ctor asm `new(0x5C)`)

All offsets are **byte offsets**. Fields inherited from AbstractClass (0x00–0x20) reuse
the base layout; Bomb-specific fields start at 0x24.

| Offset | Size | Type | Name | Init | Evidence | Conf |
|--------|------|------|------|------|----------|------|
| 0x00 | 4 | ptr | vtable_primary | `0x007E3D10` | Ctor `MOV [ESI],0x7E3D10` | HIGH |
| 0x04 | 4 | ptr | vtable_IRTTI | `0x007E3CF4` | Ctor `MOV [ESI+4],0x7E3CF4` | HIGH |
| 0x08 | 4 | ptr | vtable_INoticeSink | `0x007E3CEC` | Ctor `MOV [ESI+8],0x7E3CEC` | HIGH |
| 0x0C | 4 | ptr | vtable_INoticeSource | `0x007E3CE4` | Ctor `MOV [ESI+0xC],0x7E3CE4` | HIGH |
| 0x10 | 4 | int | UniqueID | -1 | AbstractClass::Constructor_Full | HIGH |
| 0x14 | 1 | byte | AbstractFlags | 0 (bits 0-2 cleared) | AbstractClass::Constructor_Full | HIGH |
| 0x15 | 3 | — | padding | — | — | HIGH |
| 0x18 | 4 | ptr | field_0x18 | 0 | AbstractClass::Constructor_Full | HIGH |
| 0x1C | 4 | int | field_0x1C | 0 | AbstractClass::Constructor_Full | HIGH |
| 0x20 | 1 | bool | Dirty | false | AbstractClass::Constructor_Full | HIGH |
| 0x21 | 3 | — | padding | — | — | HIGH |
| **0x24** | 4 | `TechnoClass*` | **Attacker** | param (Ivan) | Ctor `MOV [ESI+0x24],EBP` | HIGH |
| **0x28** | 4 | `HouseClass*` | **OwnerHouse** | attacker->vtable[0x3C]() | Ctor `CALL [EDX+0x3C]; MOV [ESI+0x28],EAX` | HIGH |
| **0x2C** | 4 | `TechnoClass*` | **Target** (carrier) | param | Ctor `MOV [ESI+0x2C],EAX` | HIGH |
| **0x30** | 4 | int | **State** (0=armed, 1=detonated) | 0 | Ctor `MOV [ESI+0x30],EBX`; IsTimerExpired tests `== 0`; GetClockFrame tests `==1`→frame 12 | HIGH |
| **0x34** | 4 | int | **StartFrame** | `g_CurrentFrameCounter` | Ctor `MOV [ESI+0x34], [g_CurrentFrameCounter]` | HIGH |
| **0x38** | 4 | int | **EndFrame** | StartFrame + IvanTimedDelay | Ctor `ADD ECX,EDX; MOV [ESI+0x38],ECX`; IsTimerExpired tests `<` vs g_CurrentFrameCounter | HIGH |
| **0x3C** | 16 | `VocClass_Handle` | **TickingSoundHandle** | zero-inited | Ctor `LEA ECX,[ESI+0x3C]; CALL 0x405BE0` (VocHandle init) | HIGH |
| — | — | — |   .slot = 0 (not playing) | | | HIGH |
| — | — | — |   .id_1 = 0 | | | MEDIUM |
| — | — | — |   .id_2 = 0 | | | MEDIUM |
| — | — | — |   .vtable = &DAT_0087E294 (VocManager) | | | HIGH |
| **0x4C** | 4 | int | field_0x4C | 0 | Ctor `MOV [ESI+0x54],EBX` covers 0x50/0x54; 0x4C is part of VocHandle | MEDIUM |
| **0x50** | 4 | `int` (sound ID) | **TickingSoundID** (=Rules->BombTickingSound @Rules+0x20C) | Rules+0x20C | Ctor `MOV EDX,[EAX+0x20C]; MOV [ESI+0x50],EDX` | HIGH |
| **0x54** | 4 | `AnimClass*` | **TickingSoundAnim** (spatial looping voc anim) | 0 | UpdateAll reads/writes `[bomb+0x54]`; Defuse/Detonate clear to 0 | HIGH |
| **0x58** | 1 | bool | **HasFired** (set once bomb has detonated or been defused) | false | Ctor `MOV byte [ESI+0x58],BL`; Detonate/Defuse set to 1; IsTimerExpired requires 0 | HIGH |
| 0x59 | 3 | — | padding | — | — | HIGH |

**Notes:**
- **0x30 vs 0x58** are two separate state flags. `0x30` is read by `IvanBomb__GetClockFrame`
  to decide whether to render frame 12 (the "explosion" frame of CHRONOSK.SHP). It is written
  by other code paths that are not part of Defuse/Detonate (neither Detonate nor Defuse writes
  0x30 to 1 — they both write 0x58 to 1). `0x30` may be set elsewhere; the only code I
  observed writing it is the ctor (to 0). This suggests the "explosion frame" code path is
  currently unreachable in stock YR — a latent visual feature. **Confidence on the 0x30→frame-12
  path being live: LOW.** (Possibly dormant TS-era clock logic.)
- `0x58` is the canonical "bomb is spent" flag. IsTimerExpired: `state==0 && endFrame<currentFrame
  && hasFired==0` → fire. Detonate and Defuse both set `hasFired=1` to prevent re-entry.
- `AttachedBomb` pointer lives on the carrier at **`TechnoClass+0x38`** (already documented
  in OBJECTCLASS and BULLET_CLASS reports). This is the back-pointer from the carrier to its
  BombClass.
- **BombVisible** flag (rendered from Rules->BombSight) lives at **`TechnoClass+0x68`**.

---

## 3. Primary Vtable — 0x007E3D10 (24 slots listed; remaining slots inherit AbstractClass)

The primary vtable has 28 slots matching AbstractClass. BombClass overrides the slots marked
`BombClass::` below; all other slots call the unmodified AbstractClass base implementation.

| Slot | Off | Address | Method | Notes |
|------|-----|---------|--------|-------|
| 0 | 0x00 | `0x00410260` | QueryInterface (base) | IUnknown / IPersistStream |
| 1 | 0x04 | `0x00410300` | AddRef (base) | ret 1 |
| 2 | 0x08 | `0x00410310` | Release (base) | ret 1 |
| 3 | 0x0C | `0x00438B00` | **BombClass::GetClassID** | Writes BombClass GUID at `0x007E98B0` into out-param |
| 4 | 0x10 | `0x00410450` | IsDirty (base) | |
| 5 | 0x14 | `0x00438B40` | **BombClass::Load** | Calls AbstractClass::Load, re-points vtables, re-inits VocHandle, swizzles Attacker/Owner/Target pointers via registration table `0x00B0C110` |
| 6 | 0x18 | `0x00438BD0` | **BombClass::Save** | Just forwards to AbstractClass::Save (Attacker/Owner/Target pointers are serialized as UniqueIDs via CRC path) |
| 7 | 0x1C | `0x004103E0` | GetSizeMax (base) | |
| 8 | 0x20 | `0x004393F0` | **BombClass::ScalarDeletingDestructor** | Resets vtables, removes self from global array at `0x0089C66C`, calls AbstractClass::Destructor_ResetVtables, optional `delete` |
| 9 | 0x24 | `0x00410470` | Init (base) | |
| 10 | 0x28 | `0x00410480` | PointerExpiredNotification (base) | no-op |
| 11 | 0x2C | `0x004393E0` | **BombClass::WhatAmI** | Returns `0x44` (RTTI_BOMB) |
| 12 | 0x30 | `0x004393D0` | **BombClass::GetSize** | Returns `0x5C` (object size) |
| 13 | 0x34 | `0x00438A90` | **BombClass::ComputeCRC** | Feeds AbstractClass CRC + UniqueIDs of Attacker (+0x24), Owner (+0x28), Target (+0x2C) via their IRTTITypeInfo::GetID vtable, plus HasFired byte (+0x58). |
| 14 | 0x38 | `0x00410490` | GetOwningHouseIndex (base) | Returns -1 (queried externally via OwnerHouse instead) |
| 15 | 0x3C | `0x004104A0` | Unknown_0x3C (base) | |
| 16 | 0x40 | `0x004104B0` | Unknown_0x40 (base) | |
| 17 | 0x44 | `0x00410440` | IsAlive (base) | Returns true (bomb has no Health concept) |
| 18 | 0x48 | `0x004104C0` | GetCoords (base) | **Returns NullCoord** — bombs report no coords. Game reads target->coords when it needs them. |
| 19 | 0x4C | `0x004104F0` | GetCenterCoords (base) | delegates to GetCoords → NullCoord |
| 20 | 0x50 | `0x00410520` | IsFallingDown (base) | |
| 21 | 0x54 | `0x00410530` | IsFallingDown_Late (base) | |
| 22 | 0x58 | `0x00410540` | GetFLH (base) | |
| 23 | 0x5C | `0x00410570` | **AI / Update (base, just RET)** | Bombs are NOT updated via vtable Update. Their per-tick work runs in `BombClass::UpdateAll` (0x00438BF0) called once per game tick from `LogicClass::PerTickUpdate`. The actual fuse-expiry check runs in `TechnoClass::AI_Update` on the *carrier*. |
| 24+ | — | — | base | |

Secondary vtables at `0x007E3CF4`, `0x007E3CEC`, `0x007E3CE4` are AbstractClass's standard
IRTTI / INoticeSink / INoticeSource thunks — not BombClass-specific.

---

## 4. Lifecycle — Placement → Countdown → (Detonate | Defuse | CarrierDeath)

### 4.1 Placement (Ivan fires IvanBomber weapon at target)

1. `InfantryClass::Fire_At` → creates a BulletClass with `Warhead=IvanBomb` (rules warhead
   with `IvanBomb=yes` at WarheadType+0x157).
2. Bullet immediately detonates (`Projectile=Invisible`, `CellRangefinding=yes`, `FireOnce=yes`,
   Damage=400/600 (Primary/Elite) — but this damage is **dead**; the IvanBomb warhead routes
   the damage path away from `Apply_area_damage`).
3. `BulletClass::Detonate` → `WarheadTypeClass::Detonate @ 0x004690B0`.
4. WarheadTypeClass::Detonate, seeing `warhead->IvanBomb @ +0x157 != 0`, branches to the
   IvanBomb handler:
   ```asm
   ; at 0x0046935D
   AND DL, 0x1           ; is attacker a Techno?
   NEG DL
   SBB EDX, EDX
   AND EAX, EDX          ; uVar22 = attacker if IsTechno else 0
   PUSH EAX              ; stack arg: attacker
   MOV EAX, [ESI+0xB0]   ; BulletClass.Target
   PUSH EAX              ; stack arg: target
   MOV ECX, 0x0087F5D8   ; this = &g_BombList
   CALL 0x00438E70       ; BombClass::Attach(target, attacker)
   ```
5. `BombClass::Attach` (0x00438E70) is `__thiscall` on the global `g_BombList` (DynamicVector
   at `0x0087F5D8`). It does:
   - Guards: target must have `WhatAmI()==0x0F` (= RTTI_BULLET? actually `0x0F == 15` —
     need to double-check but looking at ABSTRACTCLASS list this is "some specific techno
     subclass"; the check is present and must pass). Also requires attacker!=0 and
     `target->AttachedBomb == 0` (no existing bomb).
   - `operator_new(0x5C)` → the BombClass instance.
   - `AbstractClass::Constructor_Full(bomb)` sets base fields.
   - Writes vtables, clears Attacker/Target, inits VocHandle, loads `bomb->TickingSoundID =
     Rules->BombTickingSound (Rules+0x20C)`.
   - Registers the bomb into `g_BombClass_List` (the global array at `0x0089C66C`,
     size at `0x0089C678`, vector-header at `0x0089C668`) via the standard DynamicVector grow/add.
   - Fills the instance: `Attacker=attacker`, `OwnerHouse = attacker->vtable[0x3C]() //
     GetOwningHouse`, `Target=target`, `State=0`, `StartFrame=g_CurrentFrameCounter`,
     `EndFrame=StartFrame + Rules->IvanTimedDelay (Rules+0xFD0)`, `HasFired=0`.
   - Sets `target->AttachedBomb (target+0x38) = bomb` (the carrier-side back-pointer).
   - Registers the bomb into the `this`-side attach vector (EDI = `0x0087F5D8`) — the per-warhead
     vector used by `UpdateAll`. Also writes the vector's "dirty" flag at `this+0x30 = 1`.
   - **BombAttachSound (CrazyIvanAttack)**: if `Rules->BombAttachSound (Rules+0x210) != -1`
     AND `target.Owner.IsHumanPlayer()`, plays the sound positionally at target coords.
     So the "stuck!" sound is local-player-faction only, not spatial-audio-for-all.

### 4.2 Countdown (per-tick on the carrier)

**In `TechnoClass::AI_Update @ 0x006F9E50`**, every carrier checks its attached bomb each tick:

```c
if ((carrier->AttachedBomb /*+0x38*/ != 0) && (carrier->InLimbo /*+0x81*/ == 0) &&
    BombClass__IsTimerExpired(carrier->AttachedBomb)) {
    BombClass__Detonate(carrier->AttachedBomb);
}
```

**`BombClass::IsTimerExpired` at `0x00438A70`** returns true iff:
- `bomb->State (+0x30) == 0` (not marked detonated), AND
- `bomb->EndFrame (+0x38) < g_CurrentFrameCounter`, AND
- `bomb->HasFired (+0x58) == 0` (not already processed)

The bomb's *own* Update vtable slot is the AbstractClass base no-op RET. The only per-tick
work that happens *through the BombClass global list* is in `BombClass::UpdateAll`
(`0x00438BF0`) called by `LogicClass::PerTickUpdate` before it iterates AnimClass etc.
UpdateAll handles:
1. **Purge defused bombs**: iterates `g_BombList`, removes entries with `bomb->Target==0`
   (Defuse sets Target=0) by calling their scalar-deleting destructor with `delete=1`.
2. **Ticking sound anim lifecycle**: for each armed bomb with a looping-sound anim type
   (`bomb->TickingSoundID +0x50 != -1`):
   - If target is in limbo (`target+0x81 != 0`), detach the anim.
   - Else if no anim is spawned (`bomb->+0x54 == 0`), spawn the looping voc via `VocClass::PlayAt`
     at `bomb+0x3C` and set the anim pointer.
   - Else update the looping sound at the target's current coords.
3. **BombVisible refresh** (every 45 frames — constant `0x2D` at `0x00438BF0`+`0xB6`):
   iterates opposing infantry list; for each, if the infantry's `TypeClass->BombSight
   (+0x5F8)` range in cells covers the distance to the bomb's target, mark
   `target->BombVisible (+0x68) = 1`. Human players see their own bombs unconditionally.
   If `BombVisible` changed, set `target->NeedsRedraw (+0x80) = 1`.
   - The refresh interval `0x2D = 45 frames` = 3 seconds @ 15 fps — this is a hard-coded
     constant in BombClass::UpdateAll. Not INI-driven.

### 4.3 Clock Overlay Rendering

`IvanBomb__GetClockFrame` @ `0x00438A00`:

```c
int BombClass__GetClockFrame(BombClass* bomb) {
    if (bomb->State /*+0x30*/ == 1)  // "detonated" flag
        return 12;                    // explosion frame of CHRONOSK.SHP
    int elapsed = g_CurrentFrameCounter - bomb->StartFrame;
    int frame = (elapsed / (Rules->IvanTimedDelay / 6)) * 2;  // 0, 2, 4, 6, 8, 10
    if (g_CurrentFrameCounter % (Rules->IvanIconFlickerRate * 2) >= Rules->IvanIconFlickerRate)
        frame++;  // flicker odd frame (1, 3, 5, 7, 9, 11)
    return min(frame, 11);
}
```

**CHRONOSK.SHP** is 13 frames (0–11 clock positions + 12 detonation glyph). The name is a
holdover from TS where the same asset may have doubled as the chrono sparkle; the string
`"CHRONOSK.SHP"` is at `0x0083B0E0` and the loaded SHP pointer lives at `Rules+0xFE0`.
It is loaded by `RulesClass::ReadCombatDamage` at `0x0066C5FB`.

Drawn by `TechnoClass::DrawExtras` at `0x006F5190`:
```c
if ((techno->field_0x68 /*BombVisible*/ != 0) && (techno->field_0x38 /*AttachedBomb*/ != 0)) {
    int frame = BombClass__GetClockFrame(techno->AttachedBomb);
    CC_Draw_Shape(Rules->CHRONOSK_SHP /*+0xFE0*/, frame, &screen_coords, &viewport,
                  0xE00, 0, 0, 0, 1000, 0);
}
```

The overlay is drawn inline in the carrier's extras pass, NOT through the LayerClass sort
system. It piggybacks on the carrier's draw.

### 4.4 Detonate

**`BombClass::Detonate` at `0x00438720`** is called from three paths:
1. **Fuse expiry** — `TechnoClass::AI_Update @ 0x006F9E50` per-tick check.
2. **Carrier death** — `TechnoClass::ReceiveDamage @ 0x00701900` after the unit becomes
   non-alive (listed as step 10 in RECEIVE_DAMAGE_PIPELINE_VERIFICATION). Carrier death
   **does detonate** the bomb (carrier-death does NOT merely cancel it).
3. **Player double-click "detonate your own bomb"** — `EventClass::Execute` case `0x0A`
   (only reachable when `Rules->CanDetonateTimeBomb=yes` or `CanDetonateDeathBomb=yes`;
   defaults `no`). The UI gates this via `CanDetonateTimeBomb/DeathBomb` in rules; the
   event code itself just requires `selected->AttachedBomb != 0`.

Detonate logic:
```c
void BombClass__Detonate(BombClass* bomb) {
    TechnoClass* target = bomb->Target (+0x2C);
    if (!target || bomb->HasFired (+0x58)) return;

    target->AttachedBomb (+0x38) = 0;   // always break the back-pointer

    if (target->InLimbo (+0x81) == 0) {
        // Normal case — target on-map
        target->BombVisible (+0x68) = 0;
        bomb->HasFired (+0x58) = 1;

        CoordStruct coords = target->Location (+0x9C/0xA0/0xA4);
        Apply_area_damage(
            &coords,                       // impact coords
            Rules->IvanDamage (+0xFCC),    // damage = 450 (default)
            bomb->Attacker (+0x24),        // source = Ivan (for kill credit)
            Rules->IvanWarhead (+0xFC8));  // warhead = IvanWH

        // Spawn explosion anim at impact
        AnimType* anim = Warhead__SelectExplosionAnim(cellLandType, &coords);
        new AnimClass(anim, &coords, 0, 1, 0x2600, face, 0);

        // If target is a building with flag +0x16B6 set (UnsellableTransport?):
        //   Scan the foundation cells; if all cells are either on a LOW bridge tileset
        //   range OR overlay range 0x4A..0x65, destroy the LOW bridge underneath.
        //   Otherwise destroy the HIGH bridge.
        // (This is the "Ivan bombs a building on a bridge → bridge collapses" mechanic.)
        if (target->WhatAmI() == 6 /*Building*/ && target->Type->Insignificant (+0x16B6)) {
            /* bridge-destruction scan over 5x5 cell grid */
        }
    } else {
        // Target in limbo — silent: no damage, no anim
        target->BombVisible (+0x68) = 0;
        bomb->HasFired (+0x58) = 1;
    }

    bomb->Attacker (+0x24) = 0;
    bomb->Target (+0x2C) = 0;
    bomb->OwnerHouse (+0x28) = 0;
    VocHandle_Release(&bomb->TickingSoundHandle (+0x3C));  // FUN_00405FD0
    bomb->TickingSoundAnim (+0x54) = 0;
}
```

After Detonate, `bomb->Target==0`, so on the next tick `BombClass::UpdateAll` will purge the
bomb via its scalar-deleting destructor.

### 4.5 Defuse

**`BombClass::Defuse` at `0x004389B0`** is called from three paths:
1. **BombDisarm warhead hit** — `WarheadTypeClass::Detonate` IvanBomb/BombDisarm branch,
   when `warhead->BombDisarm @ +0x16E` is set AND attacker is a Techno AND
   `attacker->AttachedBomb (+0x38)` is non-zero (wait — the check is actually on the
   target's `+0x38` in the disarm branch; see `WarheadTypeClass__Detonate` at 0x00469410).
   Engineers are NOT the natural owner of this — in standard YR rules the only warhead
   carrying `BombDisarm=yes` is `[BombDisarm]` in rulesmd.ini, which is **not wired up
   to the stock Engineer weapon** (the Engineer uses the Capture mechanic, not BombDisarm).
   Mods wire BombDisarm onto specific weapons. See "Open questions" below.
2. **ObjectClass::UnInit @ 0x005F65F0** — when any object with an attached bomb is being
   cleaned up (garbage-collection path), it first calls Defuse to safely detach the bomb
   without detonation.
3. **ObjectClass::Constructor (0x005F3B80) and BuildingClass::ChangeOwner (0x00448260)** —
   structural re-init paths that clear any latent bomb state.

Defuse logic:
```c
void BombClass__Defuse(BombClass* bomb) {
    if (bomb->Target /*+0x2C*/) {
        target->AttachedBomb /*+0x38*/ = 0;
        target->BombVisible /*+0x68*/ = 0;
    }
    bomb->HasFired /*+0x58*/ = 1;       // same "spent" flag as Detonate
    bomb->Attacker /*+0x24*/ = 0;
    bomb->Target /*+0x2C*/ = 0;
    bomb->OwnerHouse /*+0x28*/ = 0;
    VocHandle_Release(&bomb->TickingSoundHandle);
    bomb->TickingSoundAnim /*+0x54*/ = 0;
}
```

The bomb itself is still in `g_BombList` after Defuse, but with `Target==0` it will be
destroyed on the next `BombClass::UpdateAll` pass.

### 4.6 Carrier Death

When the carrier's `ReceiveDamage` kills it (`TechnoClass::ReceiveDamage @ 0x00701900`),
the death cleanup calls `BombClass::Detonate` if `carrier->AttachedBomb != 0`. **Carrier
death does NOT defuse the bomb** — the bomb explodes.

This is the iconic "shoot the Ivan'd unit to spread the damage" tactic. The explosion
is applied at the carrier's death coords using `IvanWarhead` + `IvanDamage`, so the
Ivan bomb *adds* to the damage the carrier already took.

---

## 5. Attachment Mechanism

The bomb-carrier relationship is two-way:

| Side | Offset | Meaning | Written by | Cleared by |
|------|--------|---------|-----------|-----------|
| Carrier (ObjectClass) | +0x38 | `AttachedBomb: BombClass*` | BombClass::Attach (ctor) | Detonate, Defuse, UnInit (via Defuse) |
| Carrier (ObjectClass) | +0x68 | `BombVisible: bool` (sight-based render gate) | UpdateAll (BombSight scan) | Detonate, Defuse |
| Bomb | +0x24 | `Attacker: TechnoClass*` | ctor | Detonate, Defuse |
| Bomb | +0x28 | `OwnerHouse: HouseClass*` | ctor | Detonate, Defuse |
| Bomb | +0x2C | `Target: TechnoClass*` (back-pointer) | ctor | Detonate, Defuse |

**Global storage:**
- `g_BombClass_List @ 0x0089C668` — DynamicVectorClass<BombClass*>, vtable at `0x007E17CC`,
  grow-step 10. Every BombClass instance is added here by the base ctor (0x004385D0) and
  removed by `BombClass::ScalarDeletingDestructor` (0x004393F0).
- `g_BombAttachList @ 0x0087F5D8` — a second DynamicVectorClass<BombClass*> passed as the
  `this` arg to `BombClass::Attach`. Same vtable/growStep. Bombs are added here by `Attach`.
  This is the vector iterated by `BombClass::UpdateAll` (0x00438BF0). The two vectors may
  always have identical contents in a stock game — the separation likely corresponds to
  "all bomb instances (for heap tracking)" vs. "active attached bombs (for per-tick iteration)".

**Lifetime:**
- Created by `BombClass::Attach` on warhead detonate (IvanBomb path).
- Lives until Detonate or Defuse clears `Target=0`, then UpdateAll destroys it on the next tick.
- Serialized/restored by BombClass::Load/Save via IPersistStream (CLSID at `0x007E98B0`).

---

## 6. Clock Overlay Rendering (Summary)

| Aspect | Detail |
|--------|--------|
| SHP file | `CHRONOSK.SHP` (string at `0x0083B0E0`, loaded at `Rules+0xFE0`) |
| Frame count | 13 (0–11 clock positions, 12 detonation frame) |
| Frame picker | `BombClass::GetClockFrame` at `0x00438A00` |
| Frame formula | `((elapsed / (IvanTimedDelay / 6)) * 2) + flickerBit`, min 11; 12 if `State==1` |
| Flicker | Every `IvanIconFlickerRate` frames, add 1 to the computed frame. Default `IvanIconFlickerRate=8`. |
| Draw site | `TechnoClass::DrawExtras @ 0x006F5190` (inline, NOT LayerClass-sorted) |
| Gate | Only drawn when `carrier->BombVisible (+0x68) != 0` AND `carrier->AttachedBomb (+0x38) != 0` |
| Visibility driver | `BombClass::UpdateAll` updates `BombVisible` every 45 frames by scanning opposing infantry with `TypeClass->BombSight > 0` (radius in cells) |
| Layer / Z | CC_Draw_Shape with z-param `0xE00`, priority `1000` — drawn on top of the carrier sprite |

The "detonated frame 12" branch in `GetClockFrame` is present but I did not find a code path
that writes `bomb->State (+0x30) = 1` — both Detonate and Defuse write HasFired (+0x58)
instead. The frame-12 render path may be a latent (dead) visual feature. Confidence LOW on
frame 12 ever rendering in a stock YR game.

---

## 7. Defuse Mechanism

In stock rulesmd.ini, there is exactly one warhead with `BombDisarm=yes`:
```
[BombDisarm]
BombDisarm=yes
```
This is slot 38 in the `Warheads=` list. However, no stock YR weapon has
`Warhead=BombDisarm`. Searching rulesmd.ini shows `BombDisarm` is defined but not
referenced by any weapon — it is a moddable hook. So in stock YR, **there is no in-game
way to defuse a bomb via a weapon**.

The actual Defuse code paths that run in stock YR are:
- **UnInit** — when any object is being cleaned up (building sold, unit removed, etc.), its
  attached bomb is defused (not detonated) to prevent orphan-bomb state.
- **BuildingClass::ChangeOwner** — captured buildings have their bombs silently defused.
- **ObjectClass::Constructor** — defensive reset of bomb pointers.

The two INI options `CanDetonateTimeBomb=no` and `CanDetonateDeathBomb=no` (default `no`
in rulesmd.ini) gate the **player double-click detonate** event in the UI, not the Defuse
path. Per the YR rules comment:
```
CanDetonateTimeBomb=no  ; double click functionality on enemy bombs
CanDetonateDeathBomb=no ; double clicking bombs on own guys
```
When both are `no`, the UI will not emit EventClass case 10.

---

## 8. Fuse Timing

| Source | Value | Unit |
|--------|-------|------|
| `IvanTimedDelay=` in `[General]` of rulesmd.ini (line 830) | **450** | frames |
| At 15 fps game tick | 30 | seconds |
| Runtime field | `Rules+0xFD0` (int) | |
| Stored into bomb | `bomb->EndFrame (+0x38) = StartFrame + Rules->IvanTimedDelay` | |
| Clock segment length | `IvanTimedDelay / 6 = 75` | frames per clock hour |
| Flicker period | `IvanIconFlickerRate=8` frames (Rules+0xFD8) | |

The fuse is checked once per game tick in `TechnoClass::AI_Update` on the carrier. There is
no partial-second precision — it's whole-frame granularity.

---

## 9. Damage Pipeline

When the fuse expires (or carrier dies / player double-clicks), `BombClass::Detonate` calls:

```c
Apply_area_damage(
    &target_coords,                 // impact location (target's current coords)
    Rules->IvanDamage,              // = 450 (rulesmd.ini line 829)
    bomb->Attacker,                 // source — Ivan himself, for kill credit
    Rules->IvanWarhead);            // IvanWH
```

Then `Warhead__SelectExplosionAnim` + `AnimClass::Constructor` spawn the AnimList entry
from `[IvanWH]`:
```
[IvanWH]
Verses=100%,100%,100%,100%,100%,100%,100%,250%,20%,100%,100%
InfDeath=6
CellSpread=1.5
PercentAtMax=0.25
AnimList=CRIVEXP
```

So Ivan bombs:
- Deal 450 base damage with IvanWH (250% vs. armor index 7 / wood, 20% vs. index 8 / steel).
- CellSpread=1.5 — roughly a 3-cell splash.
- InfDeath=6 — the "zap/goo" infantry death anim.
- AnimList spawns CRIVEXP explosion art.

The bullet from IvanBomber (the weapon, which plants the bomb) had `Damage=400` (Primary)
or `Damage=600` (Elite), but this damage is never applied — the IvanBomb warhead short-circuits
the normal damage path in `WarheadTypeClass::Detonate`. The IvanBomber weapon damage exists
only so that the bullet system doesn't crash on zero-damage bullets; it's a placeholder.

---

## 10. INI Keys (all verified to be parsed and live)

All parsed by `RulesClass::ReadCombatDamage` (for IvanDamage/IvanTimedDelay etc. in the
`[CombatDamage]` section) and `RulesClass::ReadGeneral` (for the warhead references in
`[General]`).

| INI key | Section | Default | Rules offset | Meaning | Confidence |
|---------|---------|---------|--------------|---------|-----------|
| `IvanDamage` | `[CombatDamage]` | 450 | `Rules+0xFCC` (int) | Damage dealt by each bomb explosion | HIGH |
| `IvanTimedDelay` | `[CombatDamage]` | 450 (frames) | `Rules+0xFD0` (int) | Fuse duration in frames (30 s @ 15 fps) | HIGH |
| `IvanIconFlickerRate` | `[CombatDamage]` | 8 (frames) | `Rules+0xFD8` (int) | Clock-frame flicker toggle period | HIGH |
| `IvanWarhead` | `[CombatDamage]` | `IvanWH` | `Rules+0xFC8` (ptr) | Warhead used for the explosion (NOT the planting warhead) | HIGH |
| `BombTickingSound` | `[AudioList]` (indexed) | `CrazyIvanBombTick` | `Rules+0x20C` (int: sound index) | Looping ticking voc while bomb is active | HIGH |
| `BombAttachSound` | `[AudioList]` (indexed) | `CrazyIvanAttack` | `Rules+0x210` (int: sound index) | One-shot sploosh at bomb attach, human-player-only | HIGH |
| `CanDetonateTimeBomb` | `[General]` | no | — (UI-gate field; location not probed) | If yes, player can double-click own/enemy time bombs to detonate | MEDIUM |
| `CanDetonateDeathBomb` | `[General]` | no | — (UI-gate field) | Double-click on death-bomb (legacy, largely dead) | MEDIUM |
| `BombSight` (on InfantryTypes) | per-infantry INI | 0 | `InfTypeClass+0x5F8` (int: cells) | Per-infantry-type sight that reveals bomb clocks; set on GIs (4), etc. | HIGH |
| `IvanBomb=yes` (warhead flag) | any warhead | no | `WarheadType+0x157` (bool) | Marks the warhead as the planting warhead | HIGH |
| `BombDisarm=yes` (warhead flag) | any warhead | no | `WarheadType+0x16E` (bool) | Marks the warhead as defusing bombs | HIGH |
| `IvanBomb=yes` (infantry flag) | `[IVAN]`, `[CIVAN]` | no | `InfTypeClass+0xEBE` (bool per MouseClass research) | "This infantry can plant bombs" — gates mouse cursor + AI logic | HIGH |

**Important distinction:**
- The `[IvanBomb]` weapon warhead (used by `[IvanBomber]` / `[IvanBomberE]`) has
  `IvanBomb=yes`. This is the *planting* warhead — it triggers BombClass::Attach.
- The `[IvanWH]` warhead (referenced via `IvanWarhead=IvanWH`) has `IvanBomb=no` by default.
  This is the *explosion* warhead — it deals the actual damage via Apply_area_damage.

The two warheads are distinct and must not be conflated. See rulesmd.ini lines 27185–27193.

---

## 11. Call Graph

```
WarheadTypeClass::Detonate (0x004690B0)
  └─ [warhead.IvanBomb @ +0x157]
      └─ BombClass::Attach (0x00438E70)  — this=g_BombAttachList @ 0x0087F5D8
          ├─ operator_new(0x5C)
          ├─ AbstractClass::Constructor_Full (0x00410170)
          ├─ VocHandle_Init (FUN_00405BE0)
          ├─ register in g_BombList @ 0x0089C668
          ├─ register in g_BombAttachList @ 0x0087F5D8
          ├─ target->AttachedBomb (+0x38) = bomb
          └─ [if target.Owner.IsHuman && Rules+0x210 != -1]
              └─ VocClass::PlayAt(Rules->BombAttachSound)

LogicClass::PerTickUpdate (0x0055AFB0)
  └─ BombClass::UpdateAll (0x00438BF0)
      ├─ purge bombs with Target==0 via ScalarDeletingDestructor (0x004393F0)
      ├─ ticking-sound anim lifecycle via AnimClass::UpdateLoopingSound / VocClass::PlayAt
      └─ BombVisible refresh every 45 frames via BombSight range check

TechnoClass::AI_Update (0x006F9E50) [per-tick, per carrier]
  └─ [carrier.AttachedBomb != 0 && carrier.InLimbo==0]
      └─ BombClass::IsTimerExpired (0x00438A70)
          └─ BombClass::Detonate (0x00438720)

TechnoClass::ReceiveDamage (0x00701900) [on death]
  └─ [target.AttachedBomb != 0]
      └─ BombClass::Detonate (0x00438720)

EventClass::Execute (0x004C6CB0) [player event 0x0A]
  └─ [selected.AttachedBomb != 0]
      └─ BombClass::Detonate (0x00438720)

ObjectClass::UnInit (0x005F65F0)
  └─ [obj.AttachedBomb != 0]
      └─ BombClass::Defuse (0x004389B0)

BombClass::Detonate (0x00438720)
  ├─ clear target->AttachedBomb
  ├─ Apply_area_damage(target.coords, Rules->IvanDamage, bomb->Attacker, Rules->IvanWarhead)
  ├─ spawn AnimClass(warhead-selected explosion anim)
  └─ [target is Building with Type+0x16B6]
      └─ destroy low/high bridge under foundation

TacticalClass::DrawObjects / TechnoClass::DrawExtras (0x006F5190)
  └─ [carrier.BombVisible && carrier.AttachedBomb]
      ├─ BombClass::GetClockFrame (0x00438A00)
      └─ CC_Draw_Shape(Rules->CHRONOSK_SHP, frame, ...)

IPersistStream::Save (g_BombList iteration)
  └─ BombClass::Save (0x00438BD0) → AbstractClass::Save

IPersistStream::Load (stream)
  └─ BombClass::Constructor (0x004385D0)  [via CoCreateInstance for BombClass CLSID]
  └─ BombClass::Load (0x00438B40) → AbstractClass::Load + pointer-swizzle
```

---

## 12. Open Questions

1. **Who writes `bomb->State (+0x30) = 1`?** Neither Detonate nor Defuse writes 0x30.
   GetClockFrame tests it for the "frame 12 detonation glyph". If no code writes 0x30=1 in a
   live YR game, frame 12 never renders. Worth a second pass with a debugger-assisted byte
   pattern search (`C6 40 30 01` / `C7 40 30 01 00 00 00`) to confirm or deny.
2. **Is `g_BombList @ 0x0089C668` and `g_BombAttachList @ 0x0087F5D8` always kept in sync?**
   Both receive the bomb in BombClass::Attach, and ScalarDeletingDestructor removes from
   `g_BombList` — I did not trace whether it also removes from `g_BombAttachList`. If not,
   the attach list could accumulate stale pointers. Mild concern.
3. **The `WhatAmI()==0x0F` guard on the Attach target.** 0x0F = 15 decimal — the target must
   return this RTTI enum from its own WhatAmI slot. In the YR enum scheme, this specific
   value should be double-checked against the project's RTTI table (likely `RTTI_TECHNO`
   or similar). If the mapping is wrong, Ivan might silently refuse to bomb certain target
   types. The check is real and will gate the whole attach path.
4. **The "flicker" check in GetClockFrame** uses `g_CurrentFrameCounter % (IvanIconFlickerRate
   * 2)`, which creates a 16-frame period (at default 8). This is derived from the *global*
   frame counter, not the bomb's StartFrame, so all bombs on screen flicker in phase. This
   matches the visible game behavior.
5. **BombDisarm has no stock wiring.** `[BombDisarm]` warhead exists but no stock weapon
   references it. The defuse-by-warhead code path is implemented but unreachable in vanilla.
   This may be deliberate (no hero-engineer defusal in YR) or a mod-author hook. Worth noting
   in the Rust port: if we never allow `BombDisarm=yes` on a warhead, we can skip that
   branch in WarheadTypeClass::Detonate.
6. **Building-under-bomb bridge destruction.** The bridge-destruction scan in Detonate walks
   a 5×5 cell grid around the building's location cell, checking IsoTile OverlayTypeIndex
   ranges (0x18/0x19 for some overlays, `DAT_00abad30` tileset base for bridges). The logic
   is functional but the TypeClass flag at offset `0x16B6` that gates it was labeled
   "Insignificant" in BuildingClass docs — probably meaning the bridge scan only runs on
   structures small enough to actually be ON a bridge. This deserves its own verification
   pass when bridge systems are ported.

---

## 13. Ghidra Functions Labeled in This Session

| Address | Final label | Old label | Confidence |
|---------|-------------|-----------|------------|
| `0x00438720` | `BombClass__Detonate` | `FUN_00438720` | HIGH — 3 callers, clear damage-pipeline path |
| `0x004389B0` | `BombClass__Defuse` | `FUN_004389B0` | HIGH — 4 callers (UnInit, ChangeOwner, ObjectClass ctor, WarheadType::Detonate BombDisarm branch) |
| `0x00438A70` | `BombClass__IsTimerExpired` | `FUN_00438A70` | HIGH — sole caller is TechnoClass::AI_Update |
| `0x00438B00` | `BombClass__GetClassID` | `FUN_00438B00` | HIGH — writes CLSID out-param per IPersist contract |
| `0x00438B40` | `BombClass__Load` | `FUN_00438B40` | HIGH — calls AbstractClass::Load and pointer-swizzle table |
| `0x00438BD0` | `BombClass__Save` | `FUN_00438BD0` | HIGH — IPersistStream::Save forwarder |
| `0x00438A90` | `BombClass__ComputeCRC` | `FUN_00438A90` | HIGH — matches AbstractClass slot 13 override pattern |
| `0x00438BF0` | `BombClass__UpdateAll` | `FUN_00438BF0` | HIGH — iterates g_BombAttachList; sole caller is LogicClass::PerTickUpdate |
| `0x00438E70` | `BombClass__Attach` | `BombClass__Constructor` | HIGH — not really a ctor; it's the "plant a new bomb on target" factory method |
| `0x004393D0` | `BombClass__GetSize` | `FUN_004393D0` | HIGH — returns 0x5C (object size) |
| `0x004393E0` | `BombClass__WhatAmI` | `FUN_004393E0` | HIGH — returns 0x44 RTTI enum |
| `0x004393F0` | `BombClass__ScalarDeletingDestructor` | `FUN_004393F0` | HIGH — removes from g_BombList, resets vtables, optional delete |

Total functions labeled this session: **12**.

Plus a pre-existing label confirmed and documented: `BombClass__Constructor @ 0x004385D0`,
`IvanBomb__GetClockFrame @ 0x00438A00`.

Program saved after labeling.

---

## 14. Headline Facts (for the quick reader)

- **Fuse duration:** `IvanTimedDelay = 450 frames` (30 seconds @ 15 fps). Stored at
  `Rules+0xFD0`; used by ctor to set `bomb->EndFrame = StartFrame + IvanTimedDelay`.
- **Damage value:** `IvanDamage = 450` (rulesmd.ini line 829). Stored at `Rules+0xFCC`;
  passed to `Apply_area_damage` with `IvanWarhead = IvanWH`.
- **Clock SHP:** `CHRONOSK.SHP`, 13 frames (0–11 clock + 12 detonation glyph), loaded into
  `Rules+0xFE0`.
- **Carrier-death behavior:** **Detonate, not defuse.** Carrier death in
  `TechnoClass::ReceiveDamage` calls `BombClass::Detonate`, adding the IvanDamage splash on
  top of whatever killed the carrier.
- **Labels added:** 12 new BombClass method labels in Ghidra, all saved.

---

## 15. Follow-up investigation (round 2, 2026-04-21)

### Q2 — Who writes `bomb->State (+0x30) = 1`?

**Resolution: RESOLVED. Answer: nobody writes it to 1.** Verdict: the frame-12
detonation-glyph code path in `IvanBomb__GetClockFrame` is **dead visual logic in
stock YR**. Additionally, the previous round's confusion of `+0x30` between the
BombClass and the `g_BombAttachList` DynamicVector has been resolved.

**Method.** Byte-pattern search across the entire gamemd.exe `.text` for every
plausible encoding of a write to `[reg + 0x30] = 1`:

| Pattern (x86) | Meaning | Hits in BombClass range (0x00438xxx..0x0043944x) |
|---------------|---------|--------------------------------------------------|
| `C7 47 30 01 00 00 00` | `MOV dword [EDI+0x30], 1` | **1 hit @ 0x00438FCA** |
| `C7 46 30 01 00 00 00` | `MOV dword [ESI+0x30], 1` | 0 |
| `C7 43 30 01 00 00 00` | `MOV dword [EBX+0x30], 1` | 0 |
| `C7 45 30 01 00 00 00` | `MOV dword [EBP+0x30], 1` | 0 |
| `C7 40 30 01 00 00 00` | `MOV dword [EAX+0x30], 1` | 0 |
| `C7 44 24 30 01 00 00 00` | `MOV dword [ESP+0x30], 1` | 0 (none in BombClass range) |
| `C6 47 30 01` / `C6 46 30 01` / `C6 43 30 01` | `MOV byte [reg+0x30], 1` | 0 |
| `88 47 30` / `88 46 30` | `MOV byte [reg+0x30], reg8` | 0 |
| `C7 80..87 30 00 00 00 01 00 00 00` | `MOV dword [reg+0x30h], 1` (32-bit disp form) | 0 |

The one hit at `0x00438FCA` (pattern `C7 47 30 01 00 00 00`) lives inside
`BombClass::Attach`, but in that function EDI holds `param_1` (= the
DynamicVectorClass container `g_BombAttachList @ 0x0087F5D8`), NOT the bomb
instance (`puVar3` in decompile). The write `param_1[0xc] = 1` sets the
**vector's** `+0x30` field. This field is subsequently read and
decremented/reset inside `BombClass::UpdateAll` — verified by following the
`[param_1+0x30]` reads/writes at `0x00438D0C` and `0x00438D1B`:

```c
// In BombClass::UpdateAll (param_1 = &g_BombAttachList)
if (0 < *(int *)(param_1 + 0x30)) {
    *(int *)(param_1 + 0x30) = *(int *)(param_1 + 0x30) + -1;
    return;                                 // cooldown still counting down, skip
}
// Otherwise — BombVisible refresh tick:
local_30 = *(int *)(param_1 + 0x10);       // count of active bombs
*(undefined4 *)(param_1 + 0x30) = 0x2d;    // reload cooldown = 45 frames
// ... then the BombSight infantry scan ...
```

**So the `+0x30` referenced in the original report's §4.2 (the "refresh every 45
frames" counter) lives on the DynamicVector `g_BombAttachList`, NOT on any
BombClass instance.** The previous report's struct table lists `State` at
BombClass+0x30, but no evidence now supports that field being written or read
by anything other than `BombClass::IsTimerExpired` and `IvanBomb::GetClockFrame`,
both of which only test `== 0` / `== 1`.

**Writers to BombClass+0x30 in the bomb instance itself:**

1. **`BombClass::Attach @ 0x00438E70`** — zeroes it: `puVar3[0xc] = 0` (verified
   in decompile; corresponds to `MOV [ESI+0x30], EBX` with EBX=0 at `0x00438F8A`).
   This is the ONLY write to the field on the bomb object in the whole binary.
2. **`BombClass::Constructor @ 0x004385D0`** (stream-load ctor) — does NOT touch
   it (Load restores it from the stream via AbstractClass::Load).

**Readers of BombClass+0x30 (the field exists and is tested):**

1. `BombClass::IsTimerExpired @ 0x00438A70` — first clause is `if (*(uint*)(bomb+0x30) == 0)`.
2. `IvanBomb__GetClockFrame @ 0x00438A00` — first clause is `if (*(int*)(bomb+0x30) == 1) return 12;`.

Since no writer ever sets it to 1, the `return 12` branch is unreachable in
stock YR. The frame-12 detonation glyph from CHRONOSK.SHP **never renders**.
This is consistent with the observation that CHRONOSK.SHP may be a TS-era
asset repurposed for YR; the frame-12 render branch is legacy code (likely
used in TS for the chrono-legionnaire freeze indicator or similar), left dormant
in gamemd.exe.

**Renaming recommendation:** the field's semantic name should remain `State` or
be relabeled `WasDetonated` (value-set semantics imply: 0=armed/ticking,
1=already-exploded), but code readers should know that in YR nothing ever
transitions it to 1 — so treat it as "always 0" for behavior purposes.

**Implication for Rust port:** do not implement the `frame 12` rendering branch
in the bomb clock overlay. Clock frames 0-11 (with flicker) are the complete
visible behavior. If in the future we choose to faithfully reproduce the dead
branch for visual parity with a theoretical TS-like content set, add a no-op
path gated on a future `BombClass::DetonatedFlag` that remains permanently 0.

**No new label created** — no writer exists to label.

---

### Q3 — VocHandle sub-field offsets & types

**Resolution: RESOLVED (HIGH confidence for all four inner slots).**

The VocHandle is a **16-byte (4×4) weak-reference structure** — not a 20-byte
structure spilling into `bomb+0x4C` as suggested by the previous round's table.
Verified by decompiling the init + all five consumer functions:

| Function @ addr | New label |
|-----------------|-----------|
| `0x00405BE0` | `VocHandle__Init` |
| `0x00405C00` | `VocHandle__Detach` |
| `0x00405FD0` | `VocHandle__Stop` |
| `0x00406130` | `VocHandle__ValidateOrClear` |
| `0x00406170` | `VocHandle__GetSoundID` |

All five renamed and program saved this session.

#### Verified VocHandle layout (16 bytes)

From `VocHandle__Init @ 0x00405BE0`:
```c
void VocHandle__Init(VocHandle* h) {
    h->slot_ptr     = NULL;                    // +0x00
    h->gen_stamp    = 0;                       // +0x04
    h->sound_id     = 0;                       // +0x08
    h->magic_vtable = &DAT_0087E294;           // +0x0C
}
```

From `SoundEvent__SetLoopHandle @ 0x004060F0` — the authoritative writer (the
function that binds a VocHandle to a live SoundEvent slot):
```c
void SoundEvent__SetLoopHandle(VocHandle* h, SoundEvent* slot, int sound_id_override) {
    if (h->magic_vtable != &DAT_0087E294) return;   // not a real VocHandle, no-op
    h->slot_ptr = slot;                              // +0x00 = live SoundEvent*
    h->sound_id = sound_id_override;                 // +0x08 (if nonzero)
    if (slot != NULL) {
        h->gen_stamp = slot->field_0x138;            // +0x04 = slot's generation counter
        if (sound_id_override == 0)
            h->sound_id = slot->field_0x24;          // +0x08 = slot's sound-type ID
        slot->field_0x278 = h;                       // slot back-points to its listener handle
    }
}
```

From `VocHandle__Stop @ 0x00405FD0` and `VocHandle__Detach @ 0x00405C00` —
validity gate on every access:
```c
if (g_AudioManager_Active /* DAT_0087E2A0 */ != 0 &&
    h->magic_vtable == &DAT_0087E294 &&          // still a VocHandle
    h->slot_ptr != NULL) {
    SoundEvent* slot = h->slot_ptr;
    // Generation check: has the slot been reused for a different voice?
    if (h->sound_id  == slot->field_0x24  &&
        h->gen_stamp == slot->field_0x138 &&
        (slot->flags /* +0x18 */ & 0x20) == 0) {
        // Handle is still valid — act on slot.
        ...
    }
    h->slot_ptr = NULL;  // always clear
}
h->sound_id = 0;
// (Detach additionally zeros h->magic_vtable to +0x00)
```

#### Final struct map (HIGH confidence on all four fields)

| Offset (in handle) | Offset (in bomb) | Type | Field | Written by | Read/validated by |
|--------------------|------------------|------|-------|-----------|-------------------|
| +0x00 | bomb+0x3C | `SoundEvent*` | **slot_ptr** | Init=0; SetLoopHandle=slot; Stop/Detach=0 | All consumers — primary "is this handle live?" test |
| +0x04 | bomb+0x40 | `int` | **gen_stamp** (generation counter fingerprint) | Init=0; SetLoopHandle=slot->+0x138 | Consumers test `h->gen_stamp == slot->+0x138` to detect slot reuse |
| +0x08 | bomb+0x44 | `int` | **sound_id** (sound-type fingerprint) | Init=0; SetLoopHandle=slot->+0x24 (or override) | Consumers test `h->sound_id == slot->+0x24`; also returned by GetSoundID |
| +0x0C | bomb+0x48 | `void*` | **magic_vtable** | Init=`&DAT_0087E294` constant | Consumers gate ALL access on this — it's the "is this even a VocHandle?" stamp |

**Total handle size: 16 bytes, spanning bomb+0x3C..bomb+0x4B inclusive.**

#### Consequence: what lives at bomb+0x4C?

The VocHandle does NOT extend into bomb+0x4C. That means the 4 bytes at bomb+0x4C
are a **separate field** — and since Attach's decompile shows
`puVar3[0x13] = ???` is NEVER written (the next write is `puVar3[0x14] =
Rules->BombTickingSound`, i.e. bomb+0x50), **bomb+0x4C is uninitialized padding
or reserved**. The previous §2 entry `"field_0x4C | 4 | int | 0 | MEDIUM"` should
be **downgraded to "uninitialized slot, unused"** — the bomb's `operator_new(0x5C)`
allocates 92 bytes, but the ctor only writes 0x00-0x08 (AbstractClass + VocHandle
magic), 0x10-0x20 (AbstractClass base fields), 0x24-0x3B (bomb-specific early),
0x3C-0x48 (VocHandle), and 0x50-0x58 (bomb-specific late). Bomb+0x4C is dead
space (4 bytes of padding, likely alignment-driven between VocHandle and the
following fields).

#### Corrected §2 struct table entries

Replace the existing MEDIUM-confidence rows with:

| Offset | Size | Type | Name | Init | Confidence |
|--------|------|------|------|------|------------|
| 0x3C | 4 | `SoundEvent*` | TickingSoundHandle.slot_ptr | 0 (Init) | HIGH |
| 0x40 | 4 | `int` | TickingSoundHandle.gen_stamp | 0 (Init) | HIGH |
| 0x44 | 4 | `int` | TickingSoundHandle.sound_id | 0 (Init) | HIGH |
| 0x48 | 4 | `void*` | TickingSoundHandle.magic_vtable | `&DAT_0087E294` (Init) | HIGH |
| 0x4C | 4 | (unused) | padding / uninitialized | — | HIGH (uninitialized by ctor, no read/write found) |
| 0x50 | 4 | `int` | TickingSoundID | Rules+0x20C | HIGH (unchanged) |
| 0x54 | 4 | `AnimClass*` | TickingSoundAnim | 0 | HIGH (unchanged) |
| 0x58 | 1 | `bool` | HasFired | 0 | HIGH (unchanged) |

All four VocHandle sub-fields now at **HIGH confidence**. The "0x4C field"
question is also resolved — it's padding, not part of the handle.

#### Implication for Rust port

For a snapshot-serializable bomb, you do **not** need to serialize the VocHandle
contents. On load, reinitialize via the equivalent of `VocHandle__Init`
(slot_ptr=NULL, gen_stamp=0, sound_id=0, magic_vtable=sentinel). The looping
audio will be re-established on the first UpdateAll pass. Do not attempt to
persist the handle across save/load — the engine's own `BombClass::Load` at
`0x00438B40` does exactly this (calls `VocHandle__Init` after AbstractClass::Load).

---

### Round-2 new labels & saves

- `0x00405BE0` → `VocHandle__Init`
- `0x00405C00` → `VocHandle__Detach`
- `0x00405FD0` → `VocHandle__Stop`
- `0x00406130` → `VocHandle__ValidateOrClear`
- `0x00406170` → `VocHandle__GetSoundID`

`save_program` executed at end of session.

---

## Verification (round 3)

**Claim under review:** Nobody writes `bomb+0x30 = 1` in gamemd.exe. The frame-12
detonation glyph in CHRONOSK.SHP (returned by `IvanBomb__GetClockFrame @ 0x00438A00`
when `State+0x30 == 1`) never renders because no code path stores 1 into an
individual bomb's +0x30.

**Independent evidence:**

(a) **Vtable sweep.** Walked the full BombClass method set from the `vtable__BombClass`
slots referenced in `BombClass::Constructor @ 0x004385D0` and
`BombClass::Attach @ 0x00438E70`:
- `BombClass::Constructor @ 0x004385D0` — does not initialise +0x30 explicitly
  (zero-inherited from the parent init or allocator).
- `BombClass::Attach @ 0x00438E70` — **zeros** `puVar3[0xc]` (= +0x30) on the newly
  allocated bomb. Confirmed.
- `BombClass::Detonate @ 0x00438720` — writes +0x24, +0x28, +0x2c, +0x54, +0x58.
  **Does NOT write +0x30.**
- `BombClass::Defuse @ 0x004389B0` — writes +0x24, +0x28, +0x2c, +0x54, +0x58.
  **Does NOT write +0x30.**
- `BombClass::IsTimerExpired @ 0x00438A70` — only READS +0x30 (treats as bool).
- `BombClass::Load @ 0x00438B40` — restores serialised state via `FUN_006cf240`
  callbacks on +0x24/+0x28/+0x2c. +0x30 is not individually re-deserialised.
- `BombClass::Save @ 0x00438BD0`, `ComputeCRC`, `GetSize`, `GetClassID`, `WhatAmI`,
  `ScalarDeletingDestructor` — none write +0x30.

(b) **Byte-pattern search for every common x86 encoding of MOV [reg+0x30], 1:**
- `C7 40 30 01 00 00 00` — 0 hits
- `C7 41 30 01 00 00 00` — 0 hits
- `C7 42 30 01 00 00 00` — 0 hits
- `C7 43 30 01 00 00 00` — 0 hits
- `C7 45 30 01 00 00 00` — 0 hits
- `C7 46 30 01 00 00 00` — 1 hit at `0x004a78e7` (in `DiskLaserClass::AI`, unrelated
  to BombClass — different object type)
- `C7 47 30 01 00 00 00` — 1 hit at `0x00438fca` (inside `BombClass::Attach`). This
  encoding corresponds to `mov [edi+0x30], 1`, but inspection of the surrounding
  bytes at `0x00438fc4` shows the write is `param_1[0xc] = 1` in the decompile —
  **and `param_1` at that point in Attach is the BombListClass container
  (`g_BombList_Instance`), not the individual bomb being initialised**. The
  individual bomb is `puVar3`, and `puVar3[0xc] = 0` is the zero we already
  documented.
- `C6 40/41/42/43/45/46/47 30 01` (byte forms) — no hit inside BombClass.

(c) **Reader traceback.** `IvanBomb__GetClockFrame` is called only from
`TechnoClass::DrawExtras @ 0x006F5190`. `IsTimerExpired` is called only from
`TechnoClass::AI_Update @ 0x006F9E50`. Both readers treat a nonzero +0x30 as
"timer paused", which is consistent with +0x30 being a field that only exists in
the struct because of shared layout with something that used to write it (likely
C4 mines or an unused pause mechanic). Since no writer ever sets it to 1, the
`== 1 ? return 12` branch in `IvanBomb__GetClockFrame` is unreachable under
ordinary execution.

**Caveat:** Save/Load of a bomb's +0x30 happens through
`AbstractClass::Load → raw struct blob read`, so in theory a hand-crafted save
file with +0x30 pre-set could exercise the frame-12 path. This is not a runtime
write from any gamemd.exe code path.

**Verdict: CONFIRMED DEAD.**
No in-game code path writes `bomb+0x30 = 1`. The frame-12 detonation glyph branch
in `IvanBomb__GetClockFrame` is dead code in gamemd.exe. Round 2's finding holds.

Ghidra MCP calls: decompiled `BombClass::Attach`, `BombClass::Constructor`,
`BombClass::Detonate`, `BombClass::Defuse`, `BombClass::Load`,
`BombClass::IsTimerExpired`, `BombClass::UpdateAll`, `IvanBomb__GetClockFrame`;
byte-pattern searched all MOV [reg+0x30], 1 encodings; checked callers of both
readers.
