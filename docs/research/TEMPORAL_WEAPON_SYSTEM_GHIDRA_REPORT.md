# Temporal Weapon System (Chrono Legionnaire Erase) — Ghidra Research Report

**Date:** 2026-04-04
**Binary:** gamemd.exe (Yuri's Revenge 1.001)
**Confidence:** High (~90%) — all findings verified from live decompilation

---

## 1. Overview

The Temporal weapon system implements the Chrono Legionnaire's "erase" mechanic. A unit
with a Temporal warhead fires a beam that gradually erases the target from existence over
time. The system is split across three main components:

1. **WarheadTypeClass** — `Temporal=yes` flag triggers the temporal path in detonation
2. **TemporalClass** — Dedicated object managing the erase lifecycle per attacker
3. **WarpAttachClass** — State machine driving the visual animation phases (warp-in beam,
   oscillation, erase, teleport-away)

The weapon chain for the Chrono Legionnaire:
- `[DVDP]` (Chrono Legionnaire) → `Primary=NeutronRifle`
- `[NeutronRifle]` → `Damage=8`, `ROF=120`, `Warhead=ChronoBeam`, `IsRadBeam=yes`
- `[ChronoBeam]` → `Temporal=yes`

Elite variant `[NeutronRifleE]` has `Damage=16` (double speed erasure).

---

## 2. INI Keys

### WarheadTypeClass
| Key | Offset | Type | Description |
|-----|--------|------|-------------|
| `Temporal=` | `0x15A` | bool | Marks warhead as temporal (erase-over-time) |

### TechnoTypeClass
| Key | Offset | Type | Description |
|-----|--------|------|-------------|
| `Warpable=` | `0xD3A` | bool | Whether this unit can be targeted by temporal weapons. Default appears to be `yes` for most units |

### RulesClass `[General]`
| Key | Offset | Type | Description |
|-----|--------|------|-------------|
| `WarpIn=` | `0x338` | AnimType* | Animation when a chrono unit warps in (default: `WARPIN`) |
| `WarpOut=` | `0x33C` | AnimType* | Animation when a chrono unit warps out (default: `WARPOUT`) |
| `WarpAway=` | `0x340` | AnimType* | Animation when target is erased (default: `WARPAWAY`) |
| `OpenToppedWarpDistance=` | `0xF60` | int (cells) | Max distance before temporal link breaks when attacker is in open-topped transport (default: 7) |

### RulesClass `[AudioVisual]`
| Key | Offset | Description |
|-----|--------|-------------|
| `LetsDoTheTimeWarpInAgain=` | — | EVA event sound for warp-in |
| `LetsDoTheTimeWarpOutAgain=` | — | EVA event sound for warp-out |

---

## 3. TemporalClass Struct Layout

**Size:** ~0x58 bytes (based on field usage; `int` param type, offsets are direct byte offsets)

| Offset | Size | Type | Field Name | Description |
|--------|------|------|------------|-------------|
| `0x00` | 4 | void* | vtable | Primary vtable (`TemporalClass`) |
| `0x04` | 4 | void* | vtable_secondary_4 | Secondary vtable |
| `0x08` | 4 | void* | vtable_secondary_8 | Secondary vtable |
| `0x0C` | 4 | void* | vtable_secondary_12 | Secondary vtable |
| `0x10`–`0x23` | | | (inherited AbstractClass fields) | Includes UniqueID, etc. |
| `0x24` | 4 | TechnoClass* | Owner | The attacking unit (Chrono Legionnaire) |
| `0x28` | 4 | TechnoClass* | Target | The unit being erased |
| `0x2C` | 4 | int | (padding/unknown) | |
| `0x30`–`0x34` | | | (unknown) | |
| `0x38` | 4 | int | TimerStart | Frame counter when current timer started (`CDTimerClass`-style) |
| `0x3C` | 4 | int | TimerAux | Timer auxiliary value |
| `0x40` | 4 | TemporalClass* | PrevInChain | Previous temporal in the linked list targeting same unit |
| `0x44` | 4 | TemporalClass* | NextInChain | Next temporal in the linked list / also used as AnimClass* for warp beam anim |
| `0x48` | 4 | int | WarpHP / AnimState | In Update path: remaining HP before erasure. In AI path: animation state index (0–4+) |
| `0x4C` | 4 | int | DamagePerTick | Cached weapon damage value per tick |
| `0x50` | 4 | int | SubFrameCounter | Sub-frame counter for animation stepping |
| `0x54` | 1 | bool | IsInGlobalArray | Whether this TemporalClass is registered in the global tracking array |

### Global arrays
- **TemporalClass instances array:** base at `DAT_00b0ec64`, count at `DAT_00b0ec70`, capacity at `DAT_00b0ec68`
- **Active warp-attach tracking array:** base at `DAT_00b0f5bc`, count at `DAT_00b0f5c8`

---

## 4. TechnoClass Fields Related to Temporal

| Offset | Size | Type | Field Name | Description |
|--------|------|------|------------|-------------|
| `0x270` | 1 | bool | IsBeingWarpedOut | Set to 1 when temporal warp begins, cleared on detach |
| `0x274` | 4 | TemporalClass* | OwnTemporal | This unit's own TemporalClass (for units that have temporal weapons) |
| `0x278` | 4 | TemporalClass* | TemporalTargetingMe | Pointer to the head of the TemporalClass linked list attacking this unit |
| `0x328` | 4 | float | WarpFactor | Visual warp/translucency factor used for rendering (0.0 = normal, varies during erase) |
| `0x198` | 4 | int | VisualTimer_StartFrame | For `ScaleByTemporalVisualPhase` |
| `0x1A0` | 4 | int | VisualTimer_Duration | Duration of current visual phase |
| `0x1A4` | 4 | int | VisualPhaseState | Visual phase state machine index (0–10) |
| `0x1A8`(`0x6A0`) | 4 | int | LastActionFrame | Frame of last temporal action (for target tracking) |
| `0x1A9`(`0x6A4`) | 4 | int | LastActionAux | Auxiliary data for last action |
| `0x1AA`(`0x6A8`) | 4 | int | WarpVisualDuration | Visual duration multiplier (set to `ROF * 3` on detach) |

---

## 5. Weapon Fire → Temporal Initiation

### Flow: TechnoClass::Fire_At → WarheadTypeClass::Detonate → TemporalClass::InitiateWarp

**Step 1: Fire_At** (`0x006fdd50`)

In `TechnoClass::Fire_At`, the weapon type is resolved. The code checks a cascade of
weapon type flags. For IsRadBeam weapons (`0x154` on WeaponTypeClass), it calls
`FUN_006fd620` which creates the visual beam. The key temporal check is:

```c
// At offset ~line 790 of Fire_At
if (*(char *)(weaponType + 0x154) == '\0') {  // not IsRadBeam
    ...
} else if (*(int *)(weaponType + 0xac) == 0 ||
           *(char *)(*(int *)(weaponType + 0xac) + 0x15a) == '\0') {
    FUN_006fd620(target, 0);  // normal rad beam
} else {
    FUN_006fd620(target, 1);  // temporal beam (purple warp visual)
}
```

The `weaponType + 0xac` is the Warhead pointer. `warhead + 0x15a` is the `Temporal=` flag.
When firing a temporal beam, `FUN_006fd620` is called with param=1, which selects the
purple/temporal beam color from `g_RulesClass_Instance + 0x1866` (3 bytes of RGB).

**Step 2: Detonate** (`0x004690b0`)

When the projectile detonates, `WarheadTypeClass::Detonate` checks the warhead flags in
a cascading if-else chain:

```
0x155 → MindControl (CaptureManagerClass)
0x157 → Parasite
0x158 → ElectricAssault
0x159 → Sonic (knockback)
0x15a → Temporal ← THIS ONE
0x15b → IsLocomotor
0x16c → ??? (another flag)
```

When `Temporal=yes` (offset `0x15A`):
```c
// Before calling InitiateWarp, checks if target is infantry heading to a
// Grinding building, and stops that action
if (target is infantry with dock && heading to grinder) {
    TechnoClass::StopAction(attacker);
}
TemporalClass::InitiateWarp(target);
```

**Step 3: InitiateWarp** (`0x0071af20`)

```c
void TemporalClass::InitiateWarp(TechnoClass* target) {
    // Kill spawned units on target
    if (target->SpawnManager != NULL)
        SpawnManagerClass::Kill_All_Spawns();

    // Free mind-controlled slaves
    if (target->CaptureManager != NULL)
        CaptureManagerClass::FreeAll();

    // If our owner already has a temporal target, detach first
    if (this->Owner->OwnTemporal != NULL &&
        this->Owner->OwnTemporal->Target != NULL)
        TemporalClass::DetachFromTarget();

    // Check if we can warp this target
    if (!TemporalClass::CanWarpTarget(target))
        return;

    // Don't allow if our own owner is already being warped
    if (this->Owner->TemporalTargetingMe != NULL)
        return;

    this->Target = target;

    if (target->TemporalTargetingMe == NULL) {
        // FIRST temporal targeting this unit — we're the head of the chain
        target->TemporalTargetingMe = this;

        // Initialize WarpHP = target Strength * 10
        int typeClass = target->GetTypeClass();
        this->WarpHP = typeClass->Strength * 10;

        // Notify if target is a building owned by local player
        if (target is building && target->Owner == g_PlayerPtr) {
            CreateRadarEvent(target->GetCoords());
            VoxClass::PlayEVA(-1);  // "Unit under attack" EVA
        }

        // Building-specific: mark house for recalc, start decloaking
        if (target is building) {
            target->Owner->NeedsRecalc = true;
            BuildingClass::StartCloaking();  // actually UNcloaks
            target->Owner->NeedsRebuild = true;
        }
    } else {
        // STACKING: Another temporal already targeting this unit
        // Insert into doubly-linked list
        TemporalClass* existing_head = target->TemporalTargetingMe;
        this->PrevInChain = existing_head;
        this->NextInChain = existing_head->NextInChain;
        existing_head->NextInChain = this;
        if (this->NextInChain != NULL)
            this->NextInChain->PrevInChain = this;
    }

    // Mark target as being warped out
    target->IsBeingWarpedOut = true;

    // If attacker is gattling type, update gattling stage
    if (attacker->TypeClass->IsGattling)
        TechnoClass::UpdateGattlingStage(1);

    // Building-specific handling
    if (target is building) {
        target->Owner->NeedsRecalc = true;
        BuildingClass::StartCloaking();
        target->Owner->NeedsRebuild = true;
    }

    // Force target to update visual (draw as partially erased)
    target->UpdateVisual(2);

    // If target has its own temporal weapon, detach it from ITS target
    if (target->OwnTemporal != NULL && target->OwnTemporal->Target != NULL)
        TemporalClass::DetachFromTarget();

    // Update fog/shroud for player
    if (g_PlayerPtr != NULL)
        target->UpdateFog();
}
```

---

## 6. Erase-Over-Time Mechanic (TemporalClass::Update)

**Address:** `0x0071a760`

This function is called each game tick for the **head** temporal in the chain (the first
one targeting the unit). It handles the HP countdown.

### Algorithm

```c
void TemporalClass::Update() {
    TechnoClass* target = this->Target;

    // SAFETY CHECK: If target's pointer back to us is wrong AND we have a
    // PrevInChain, we're not the head — clear and bail
    if (target != NULL && target->TemporalTargetingMe == this && this->PrevInChain != NULL) {
        target->TemporalTargetingMe = NULL;
        target->IsBeingWarpedOut = false;
        TemporalClass::ClearLinkedList();
        return;
    }

    // RANGE CHECK for open-topped transport
    if (this->Owner != NULL && this->Owner->IsOpenTopped) {
        int distance = Sqrt_Approx(distance_squared(owner_pos, target_pos));
        if (distance > RulesClass->OpenToppedWarpDistance * 256) {
            TemporalClass::DetachFromTarget();
            return;
        }
    }

    // SUM CHAIN DAMAGE: Accumulate damage from all temporals in the chain
    int chainDamage = 0;
    if (this->NextInChain != NULL) {
        chainDamage = TemporalClass::SumChainDamage();
    }

    // Get THIS temporal's weapon damage
    WeaponTypeClass* weapon = owner->GetWeapon(owner->GetCurrentWeaponIndex());
    int myDamage = weapon->Damage;  // offset 0xa4 on WeaponTypeClass
    this->DamagePerTick = myDamage;

    // DECREMENT: Subtract (myDamage + chainDamage) from WarpHP
    this->WarpHP -= (myDamage + chainDamage);

    if (this->WarpHP < 1) {
        // === ERASURE COMPLETE ===
        if (this->Target == NULL) {
            // Target already gone — just clean up
            clear_all_fields();
            owner->StopAction(0, 1);
        } else {
            // Play WarpAway animation at target location
            AnimClass::Constructor(RulesClass->WarpAway, target_coords, ...);

            // Experience transfer (if weapon has it enabled)
            if (owner->TypeClass->field_0xc8e) {  // some experience flag
                // Transfer experience from target to owner
            }

            // BUILDING-SPECIFIC COMPLETION
            if (target is building) {
                // Spawn parachuting units if building had them
                if (building->OccupantCount > 0)
                    SpawnUnitsWithParachute(0);

                // Release all linked objects
                // (undock, clear factory queue, etc.)

                // Suspend any super weapon charging
                SuperClass::Suspend(0);

                // Undock any unit
                BuildingClass::UndockUnit();

                // Kill target
                target->ReceivedDamage(owner);  // vtable 0x3b8
                target->Destroy(owner);         // vtable 0xe0
                target->Remove();               // vtable 0xf8
                target->Owner->NeedsRebuild = true;
            } else {
                // NON-BUILDING: Remove linked locomotor
                FootClass* foot = target->GetLocomotor();
                if (foot != NULL) {
                    foot->CYCAIDA();
                    foot->Destroy();
                }

                // Kill target
                target->ReceivedDamage(owner);
                target->Destroy(owner);
                target->Remove();
            }

            owner->StopAction(0, 1);
        }

        // Clean up all fields
        this->Target = NULL;
        this->NextInChain = NULL;
        this->PrevInChain = NULL;
        this->TimerAux = NULL;
        this->TimerStart = NULL;
        owner->StopAction(0, 1);
    }
}
```

### Erasure Speed Formula

- **WarpHP** = `target->TypeClass->Strength * 10`
- **Damage per tick** = Sum of all attacking temporals' weapon Damage values
- **Ticks to erase** = `(Strength * 10) / total_damage`

For a standard Chrono Legionnaire (`Damage=8`) vs a Rhino Tank (`Strength=400`):
- WarpHP = 400 * 10 = 4000
- Ticks to erase = 4000 / 8 = 500 ticks (~33 seconds at 15 FPS)

Two Chrono Legionnaires stacking:
- Ticks to erase = 4000 / 16 = 250 ticks (~17 seconds)

Elite variant (`Damage=16`):
- Ticks to erase = 4000 / 16 = 250 ticks (~17 seconds)

---

## 7. Stacking (Multiple Temporals on Same Target)

The system uses a **doubly-linked list** to track multiple temporals targeting the same unit.

### Data Structure

```
Target->TemporalTargetingMe → [Temporal_Head]
                                  ↕ PrevInChain / NextInChain
                               [Temporal_2]
                                  ↕
                               [Temporal_3]
                                  ...
```

- The **head** (first attacker) is stored at `target->TemporalTargetingMe` (offset `0x278`)
- Each TemporalClass has `PrevInChain` (offset `0x40`) and `NextInChain` (offset `0x44`)
- Only the **head** runs `TemporalClass::Update` for the HP countdown
- `TemporalClass::SumChainDamage` recursively walks the chain (up to 51 deep, capped at
  `0x33` to prevent infinite recursion) summing each temporal's weapon damage

### SumChainDamage (`0x0071ab10`)

```c
int TemporalClass::SumChainDamage(int depth) {
    int sum = 0;
    if (this->NextInChain != NULL && depth < 51) {
        sum = this->NextInChain->SumChainDamage(depth + 1);
    }
    WeaponTypeClass* weapon = owner->GetWeapon(owner->GetCurrentWeaponIndex());
    int myDamage = weapon->Damage;
    this->DamagePerTick = myDamage;
    return myDamage + sum;
}
```

**Result:** Yes, multiple Chrono Legionnaires targeting the same unit stack additively,
proportionally speeding up the erasure.

---

## 8. WarpAttachClass AI — Visual State Machine

**Address:** `0x006297f0` (TemporalClass__AI, called from WarpAttachClass__UpdateAttack)

The visual animation of the erase is driven by a 5-state machine at offset `0x48` of
the TemporalClass (reused as animation state when in the AI path):

| State | Description | Behavior |
|-------|-------------|----------|
| 0 | **Init** | Set `target->WarpFactor = 0`, find and play initial warp anim, transition to state 1 |
| 1 | **Beam Establish** | Advance anim frames (10 sub-frames per step, 10 steps). On completion, randomly pick state 2 or 3 |
| 2 | **Oscillate Negative** | Cosine wave visual with negative direction (`dVar12 = -1.0`) |
| 3 | **Oscillate Positive** | Cosine wave visual with positive direction (`dVar12 = 1.0`). Sets `target->WarpFactor = cos(phase) * constant` |
| 4 | **Erase / Final** | Spawn 3 warp-away particle anims. Check if unit should be erased vs warped. If erasable: destroy target, play effects, clean up. If survived: back to state 2/3 for more oscillation |

### State 4 Detail (Completion)

In state 4, the code checks `warhead + 0x174` (the `Culling` flag on WarheadTypeClass).
If Culling is enabled, AND certain conditions are met (game mode checks), the target is:

1. Experience transferred to attacker (if enabled)
2. `WarpAttachClass::Detach()` called — this teleports the CL away
3. Target marked with `field_0x3CD = 1` (some death flag)
4. Target receives damage and is removed

If Culling is NOT enabled (or conditions fail), the target takes normal damage via
`target->ReceiveDamage(weaponDamage, ...)` and if it survives, the cycle repeats
(back to state 2/3).

### Animation Frame Advance (`0x00629720`)

The `TemporalClass__AdvanceAnimFrame` function handles per-frame animation stepping:

- Uses a **sub-frame counter** at offset `0x50` and a **frame counter** at offset `0x4C`
- Sub-frame rates vary by state: state 0/1 = 3 sub-frames, state 2 = 4, others = 4
- After 10 frame steps, returns true (phase complete)
- Updates the beam animation's SHP frame based on `(RateTimer + state_offset * 10 + frame)`
- Frame calculation: `anim_frame = frame_counter + (facing & 7) * 10 + state_base_offset`

### Warp-in Beam Animation Spawning (`0x00629e90`)

`WarpAttachClass::SpawnWarpAnims` creates 3 random warp spark animations around the
target using `RulesClass + 0x94` AnimType (likely `WARPINWP` or similar), with random
offsets in range [-180, +180] X and [-64, +64] Y.

---

## 9. Interruption / Detach

**Address:** `0x0071abc0` (TemporalClass::DetachFromTarget)

When the attacker dies or the link is broken, `DetachFromTarget` is called.

### Cases

**Case A: Head of chain, no next in chain (sole attacker)**
```c
target->TemporalTargetingMe = NULL;
target->IsBeingWarpedOut = false;
// Building: trigger recalc, stop cloaking anim
// Call target->UpdateVisual(2) to restore normal appearance
```
The target **snaps back instantly** to normal. There is no recovery timer or gradual
restoration — the `IsBeingWarpedOut` flag is simply cleared and the visual warp factor
resets.

**Case B: Head of chain, has next in chain**
```c
target->TemporalTargetingMe = this->NextInChain;
this->NextInChain->PrevInChain = NULL;
// Transfer remaining WarpHP to the new head
this->NextInChain->WarpHP = this->WarpHP;
```
The next temporal in the chain becomes the new head and inherits the remaining WarpHP.
The erasure continues seamlessly.

**Case C: Middle of chain**
```c
this->NextInChain->PrevInChain = this->PrevInChain;
this->PrevInChain->NextInChain = this->NextInChain;
```
Standard doubly-linked list removal.

### ClearLinkedList (`0x0071ade0`)

Recursively walks both directions of the chain, clearing all links and resetting
`target->IsBeingWarpedOut` and `target->TemporalTargetingMe` to 0. Called when the
system detects an inconsistent state.

### Open-Topped Range Break

In `TemporalClass::Update`, if the attacker is in an open-topped transport (e.g., Battle
Fortress), the distance between attacker and target is checked each tick. If it exceeds
`OpenToppedWarpDistance * 256` leptons (default: 7 * 256 = 1792), `DetachFromTarget` is
called and the temporal link breaks.

---

## 10. WarpAttachClass::Detach — Completion Teleport

**Address:** `0x0062a4a0`

When the erase completes and the target is supposed to be removed, `WarpAttachClass::Detach`
handles the Chrono Legionnaire's post-erase behavior:

1. Gets the attacker's TypeClass to check `0xCCE` flag (likely `Teleporter=`)
2. Reads current facing direction
3. Finds a valid cell to place the attacker at (using `CellClass::CheckCellPassability`)
4. If a valid cell is found:
   - Moves the unit there
   - Sets ghost cell state
   - Stops current action
   - Updates visibility
   - Sets `target->WarpVisualDuration = ROF * 3` (warp-out visual duration)
5. If no valid cell: just removes the unit from its current position
6. Clears `target->TemporalTargetingMe`
7. Resets attacker's visual state

---

## 11. Target Restrictions (CanWarpTarget)

**Address:** `0x0071ae50`

```c
bool TemporalClass::CanWarpTarget(TechnoClass* target) {
    if (target == NULL) return false;

    // Check Warpable= flag on target's type class
    TypeClass* type = target->GetTypeClass();
    if (!type->Warpable)  // offset 0xD3A
        return false;

    // Check if target is Iron Curtained
    if (target->IsInvulnerable())  // vtable 0x160
        return false;

    // Special check: infantry heading to a Grinding building
    if (target is infantry) {
        TechnoClass* dest = FootClass::GetDestination();
        if (dest is building && dest->TypeClass->field_0x16BD != 0) {
            // Check if infantry is ON the grinder cell
            CellClass* cell = GetCellAt(target->coords);
            BuildingClass* bldg = LookupBuildingInCell(cell);
            if (bldg == dest)
                return false;  // Don't warp infantry being ground up
        }
    }

    return true;
}
```

### Summary of immunities:
- **Warpable=no** units (set on TypeClass) — explicitly immune
- **Iron Curtained** units — immune while invulnerable
- **Infantry entering a Grinder** — immune while on the grinder cell
- **Mind-controlled units** — NOT immune (but mind control link is freed on InitiateWarp)
- **Buildings** — CAN be temporal'd (special handling for occupants, factory queues, etc.)
- **Units already being temporal'd by our owner** — detaches first and re-targets

---

## 12. Visual Rendering — Temporal Phase

### UpdateTemporalVisual (`0x0070e5a0`)

Called on the **target** unit each frame to update its visual warp state. This is a
10-state machine stored at `TechnoClass + 0x1A4`:

| Phase | Duration (frames) | Visual Effect |
|-------|-------------------|---------------|
| 0 | instant | Init → Phase 1 |
| 1 | 6 | Fade-in flicker |
| 2 | 4 | Brief hold |
| 3 | 15-25 (random) | Main "being erased" state |
| 4 | 8 | Intensity build |
| 5 | 16 | Hold at intensity, check if erase timer < 0x36 |
| 6 | variable | Near completion, wait for timer < 0x1F |
| 7 | 6 | Final fade sequence |
| 8 | 4 | Brief hold |
| 9 | 20 | Final shimmer |
| 10 | — | Complete (unit fully erased visually) |

Phases 5→6 and 6→7 are gated by `CDTimerClass::Remaining()` checks (values `0x36` and
`0x1F`), tying the visual progression to the actual erase countdown.

### ScaleByTemporalVisualPhase (`0x0070e380`)

This function modifies the sprite draw scale/intensity based on the current visual phase.
It's called during rendering to apply translucency:

| Phase | Scale Formula | Effect |
|-------|--------------|--------|
| 1 | `(12 - remaining) * 256 / 6` | Fade in |
| 2, 8 | `512` (constant) | 50% translucent |
| 3 | `(remaining * 461 + 1020) / 20` | Gradual shimmer |
| 4 | `(remaining * -77 + 1024) / 8` | Decreasing intensity |
| 5 | `(remaining * 77 + 816) / 16` | Increasing intensity |
| 6 | `51` (constant) | Very translucent |
| 7 | `(remaining * -461 + 3072) / 6` | Fade out |
| 9 | `(remaining + 20) * 256 / 20` | Final restoration |

The value is used to scale the sprite's alpha: `result = (phase_scale * input) >> 8`,
clamped to max 2000.

The rendering pipeline uses specialized blitters with "Warp" in their names:
- `BlitTransLucent25ZReadWarp`
- `BlitTransLucent50ZReadWarp`
- `BlitTransLucent75ZReadWarp`
- `RLEBlitTransLucent*Warp` variants

These apply the warp visual effect (color shifting + translucency) to the target sprite.

---

## 13. Key Addresses Summary

| Address | Function |
|---------|----------|
| `0x0071a450` | `TemporalClass::Constructor` (no params) |
| `0x0071a4e0` | `TemporalClass::Constructor` (with init) |
| `0x0071b1b0` | `TemporalClass::Destructor` |
| `0x0071a660` | `TemporalClass::Load` (save/load serialization) |
| `0x0071a720` | `TemporalClass::GetClassID` |
| `0x0071a760` | `TemporalClass::Update` (HP countdown, erase logic) |
| `0x0071af20` | `TemporalClass::InitiateWarp` (start erasing target) |
| `0x0071ae50` | `TemporalClass::CanWarpTarget` (immunity checks) |
| `0x0071abc0` | `TemporalClass::DetachFromTarget` (break link) |
| `0x0071ade0` | `TemporalClass::ClearLinkedList` (recursive cleanup) |
| `0x0071acd0` | `TemporalClass::ClearWarpingOutOnTarget` (clear flags) |
| `0x0071ab10` | `TemporalClass::SumChainDamage` (recursive chain sum) |
| `0x006297f0` | `TemporalClass::AI` (visual state machine) |
| `0x00629720` | `TemporalClass::AdvanceAnimFrame` (animation stepping) |
| `0x00629e90` | `WarpAttachClass::SpawnWarpAnims` (spark effects) |
| `0x00629fd0` | `WarpAttachClass::UpdateAttack` (per-tick update caller) |
| `0x0062a4a0` | `WarpAttachClass::Detach` (completion/teleport) |
| `0x0062ab40` | `WarpAttachClass::CanPlaceAtTarget` (placement check) |
| `0x0070e380` | `TechnoClass::ScaleByTemporalVisualPhase` (render scaling) |
| `0x0070e5a0` | `TechnoClass::UpdateTemporalVisual` (visual state update) |
| `0x0075d590` | `WarheadTypeClass::ReadINI` (parses Temporal= at 0x15A) |
| `0x004690b0` | `WarheadTypeClass::Detonate` (triggers InitiateWarp) |

### RTTI
- `TemporalClass` RTTI at `0x008445d0`
- Vtable at address stored in constructor
- ClassFactory RTTI at `0x00841298`
- DynamicVectorClass for TemporalClass* at `0x00844598`

---

## 14. Implementation Notes for Rust Engine

### Core data structures needed:
1. **TemporalState** per attacking unit: owner, target, warp_hp, damage_per_tick, chain pointers
2. **Temporal target state** per target unit: is_being_warped, temporal_head pointer
3. **Visual warp state** per target: 10-phase state machine with timers

### Key behaviors:
- WarpHP = `Strength * 10`, decremented by sum of all attackers' weapon Damage per tick
- Linked list for stacking (multiple attackers on same target)
- When head attacker dies, next in chain takes over with SAME remaining WarpHP
- Interruption = instant recovery (no gradual restoration)
- Iron Curtain = immune; Warpable=no = immune
- Buildings: special handling for occupants, factory, super weapons
- OpenToppedWarpDistance range check each tick for transport passengers
- Visual phases are purely cosmetic — the HP countdown is the real timer

### Fixed-point considerations:
- WarpHP, Damage are integers — no float needed for sim logic
- Visual warp factor (`0x328`) is float — render-only, can use f32
- Distance checks use integer leptons (256 per cell)
- Cosine/sine lookups for oscillation are render-only

### TS legacy warning:
The `Culling` warhead flag (`0x174`) is checked in the AI state 4 completion path.
This appears to be active in YR (the ChronoBeam warhead could theoretically have it).
However, standard YR warheads don't set `Culling=yes`. Verify before implementing
whether any YR warhead actually uses this path — it may be TS-only dead code.
