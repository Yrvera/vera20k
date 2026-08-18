# TemporalClass Lifecycle & Warp Draw Pipeline -- Ghidra Deep-Dive

## Summary

Full reverse-engineering of the TemporalClass (chrono-erase weapon system) and its
integration with the draw pipeline. Covers lifecycle from creation through per-tick
update, visual phase state machines, blitter selection for warp rendering, and the
shared base class between TemporalClass and ParasiteClass.

Confidence: **HIGH (90%+)** for all struct layouts and lifecycle. Confidence is
**MODERATE (75%)** for the exact naming of the shared base class (called
"WarpAttachClass" here as a working name; it may be unnamed in the original source).

---

## 1. TemporalClass Struct Layout

**Size: 0x50 bytes** (based on constructor initialization range)

TemporalClass inherits from AbstractClass via INoticeSink.

| Offset | Size | Type | Field Name | Evidence |
|--------|------|------|-----------|----------|
| 0x00 | 4 | ptr | vtable (primary) | Constructor: `*param_1 = &vtable__TemporalClass` at 0x7F5180 |
| 0x04 | 4 | ptr | vtable (INoticeSink) | Constructor: secondary_4 at 0x7F5164 |
| 0x08 | 4 | ptr | vtable (IPersist) | Constructor: secondary_8 at 0x7F515C |
| 0x0C | 4 | ptr | vtable (IStream) | Constructor: secondary_12 at 0x7F5154 |
| 0x10-0x23 | 20 | | AbstractClass base fields | Inherited |
| 0x24 | 4 | TechnoClass* | **Owner** (the chrono legionnaire) | Constructor 0x71a4e0: `param_1[9] = param_2` |
| 0x28 | 4 | TechnoClass* | **Target** (the unit being erased) | InitiateWarp: `*(param_1 + 0x28) = param_2` |
| 0x2C | 4 | int | CDTimer.StartFrame | Constructor: `= g_CurrentFrameCounter` |
| 0x30 | 4 | int | CDTimer.Ticks | |
| 0x34 | 4 | int | CDTimer.Duration | Constructor: `= 0` |
| 0x38 | 4 | | Cleared on detach | DetachFromTarget: zeroed |
| 0x3C | 4 | ptr | **SuperWeapon** ptr (for buildings) | Update: `SuperClass__Suspend(0)` called, then zeroed |
| 0x40 | 4 | TemporalClass* | **PrevInChain** (linked list) | InitiateWarp/DetachFromTarget |
| 0x44 | 4 | TemporalClass* | **NextInChain** (linked list) | InitiateWarp/DetachFromTarget |
| 0x48 | 4 | int | **WarpPoints** (HP remaining) | InitiateWarp: `= target->Strength * 10`; Update decrements |
| 0x4C | 4 | int | **DamagePerTick** | SumChainDamage: `= weapon->Damage` |

### Vtable at 0x7F5180 (primary, 20 entries)

Key entries (byte offsets into vtable):
- +0x0C: 0x0071A720 = TemporalClass::GetClassID
- +0x20: 0x0071A660 = TemporalClass::Load
- +0x24: generic Save

### Global Array

TemporalClass instances are tracked in a global `DynamicVectorClass<TemporalClass*>`:
- Array pointer: `DAT_00b0ec64`
- Count: `DAT_00b0ec70`
- Capacity: `DAT_00b0ec68`

---

## 2. TemporalClass Full Lifecycle

### 2a. Creation (0x006f3f40 -- TechnoClass Init)

When a TechnoClass is created, if its primary weapon's warhead has `Temporal=yes`
(TypeClass flag at offset +0x15A from WarheadTypeClass), a TemporalClass is allocated
(size 0x50) and stored at **TechnoClass+0x274** (`param_1[0x9d]`):

```
param_1[0x9d] = TemporalClass__Constructor(this_unit);
// Then: temporal->DamagePerTick = weapon->Damage
*(param_1[0x9d] + 0x4C) = weapon->Damage;
```

### 2b. Initiating a Warp (0x0071AF20 -- TemporalClass::InitiateWarp)

Called when the chrono legionnaire fires at a target.

**Steps:**
1. If target has a SpawnManager (+0x2D0), call FUN_006b7100 (release spawns)
2. If target has a CaptureManager (+0x2BC), call CaptureManagerClass::FreeAll
3. If owner already has an active temporal, detach from current target
4. Validate target with CanWarpTarget (0x0071AE50):
   - Target must have `Temporal=yes` on its TypeClass (+0xD3A)
   - Target must NOT already be warping out (vtable +0x160 check)
   - If infantry moving into a WarpAway building, reject
5. If target already has temporals attacking it (`target+0x278 != 0`):
   - **Insert into linked list** via +0x40/+0x44 chain
   - Copy existing head's remaining time
6. If first temporal on target:
   - Set `target+0x278 = this` (TemporalTarget pointer on victim)
   - Set WarpPoints = `target->Strength * 10` (TypeClass offset +0xA0)
   - If target is a building (WhatAmI==6), set cloaking flags and notify house
   - Trigger radar event if target is player-owned infantry
7. Set `target+0x270 = 1` (IsBeingWarped flag)
8. If target's TypeClass has `ImmuneToTemporalOnWarp` (+0xCD5), apply immediate temporal damage
9. Call `target->Limbo(2)` -- begin removal from map layers

### 2c. Per-Tick Update (0x0071A760 -- TemporalClass::Update)

Called from the global TemporalClass array iteration (not from TechnoClass::AI).

**Steps:**
1. **Range check**: If owner is still alive and target exists, compute distance.
   If distance > `Rules->TemporalRange * 256`, call DetachFromTarget and abort.
2. **Sum chain damage**: Recursively walk the NextInChain (+0x44) linked list,
   accumulating weapon damage values from each temporal in the chain.
   Each temporal's DamagePerTick is read from `owner->GetWeapon(slot)->Damage`.
3. **Decrement WarpPoints**: `WarpPoints -= (myDamage + chainDamage)`
4. **If WarpPoints <= 0** (unit erased):
   - Spawn warp-out animation (Rules+0x340 AnimType)
   - If owner has `WarpCombatExperience` flag, grant veterancy
   - **Erase the target**:
     - For buildings: release spawn units with parachutes, remove all occupants,
       suspend super weapons, undock units, notify house
     - For non-buildings: detach attached objects, release SlaveManager
     - Call `target->Registered_For_Kill(owner)` and `target->Remove()`
   - Clear all linked list pointers and owner references

### 2d. Detach From Target (0x0071ABC0)

Handles three cases based on position in linked list:
1. **Head of list** (PrevInChain == 0, NextInChain != 0): Promote next to head,
   transfer remaining WarpPoints, update `target+0x278`
2. **Head of list, only temporal**: Clear `target+0x278`, clear `target+0x270`,
   rebuild building vision, call `target->Limbo(2)`
3. **Middle/end of list**: Unlink from doubly-linked list

Always clears: +0x28 (target), +0x44, +0x40, +0x3C, +0x38.

### 2e. The Linked List (+0x40 / +0x44)

Multiple chrono legionnaires can target the same unit simultaneously.
The TemporalClass instances form a **doubly-linked list**:

```
target->TemporalTarget (+0x278) --> Temporal_A
    Temporal_A->PrevInChain (+0x40) = 0 (head)
    Temporal_A->NextInChain (+0x44) = Temporal_B
    Temporal_B->PrevInChain (+0x40) = Temporal_A
    Temporal_B->NextInChain (+0x44) = Temporal_C
    ...
```

The head temporal "owns" the WarpPoints counter. When summing damage,
the chain is walked recursively (max depth 51 to prevent infinite loops).

---

## 3. FUN_0062A4A0 Identification -- Shared Base Class Detach

**Address:** 0x0062A4A0, body ends at 0x0062A8D9 (1081 bytes)

### What class does param_1 belong to?

param_1 is a **TemporalClass\*** OR **ParasiteClass\***. Both classes inherit from
the same base (AbstractClass <- INoticeSink) and share identical field layouts at:
- +0x24 = Owner (TechnoClass*)
- +0x28 = Target (TechnoClass*)
- +0x2C/+0x30/+0x34 = CDTimer

The function is a **non-virtual method** shared by both classes. It is NOT in either
vtable. Working name: **WarpAttachClass::Detach** (detaches owner from target and
teleports/places the owner at the target's location).

### Callers (xrefs TO 0x0062A4A0)

| Caller | Context |
|--------|---------|
| TemporalClass::AI (0x006297f0) | Called 3x during temporal warp update phases |
| TeleportLocomotionClass::InitiateWarp (0x7195cf) | Called via `unit->field_0x694->field_0x69C` |
| FootClass::ReceiveDamage (0x4d735f, 0x4d740e) | Called via `unit->field_0x694->field_0x69C` |
| TechnoClass::StartFidget (0x4deb6e) | Same path |
| TechnoClass::PerformDeploy (0x710058) | Same path |
| FUN_006f4ab0 (0x6f4da6) | TechnoClass fire/command handler |
| SuperClass::Launch (0x6cc7b7) | Chronoshift super weapon |
| UnitClass::Mission_Enter (0x73a18d) | Unit entering transport |

### What it does

1. Reads target's TypeClass flag at +0xCCE to determine if this is a chrono/temporal warp
2. Gets current map cell position (via RateTimer or target coords)
3. Checks cell passability for the owner unit
4. If cell is valid and different from current position:
   - Tests CanPlaceAtTarget (FUN_0062AB40)
   - Moves owner to target location
   - Calls `owner->SetGhostCell(0)`, `owner->Unlimbo()`, `owner->Scatter()`
   - Sets up warp-in visual timer: `owner[0x1A8..0x1AA]` with duration = weapon->Speed * 3
5. Clears the attachment: sets `target->field_0xCA = 0`, resets target's visual timer
6. Removes self from the global tracking array if registered

### Field Layout Accessed

| Access | Offset | Meaning |
|--------|--------|---------|
| param_1[9] | +0x24 | Owner TechnoClass* |
| param_1[10] | +0x28 | Target TechnoClass* |
| param_1[0xb] | +0x2C | Timer start frame |
| param_1[0xd] | +0x34 | Timer duration |
| param_1[0x11] | +0x44 | NextInChain / linked anim |
| param_1[0x12] | +0x48 | State counter |
| param_1[0x13] | +0x4C | DamagePerTick |
| param_1[0x14] | +0x50 | Sub-state counter |
| param_1[0x15] | +0x54 | IsInGlobalArray flag |

---

## 4. TechnoClass+0x694 -- The Parasite/Temporal Attacker Pointer

**FootClass offset 0x694** is a pointer to the **TechnoClass that is parasitically
attached to or chrono-warping this unit**. For Terror Drone victims, this points to
the Terror Drone. For chrono-warp targets, this points to the Chrono Legionnaire.

The access pattern is always:
```c
attacker = unit->field_0x694;      // TechnoClass* of the attaching unit
parasite = *(attacker + 0x69C);    // ParasiteClass* owned by the attacker
WarpAttachClass__Detach(parasite); // detach and place at target
```

**TechnoClass+0x69C** (`param_1[0x1A7]`) is the **ParasiteClass\*** pointer,
allocated in TechnoClass init (0x006f3f40) when the unit's weapon warhead has
the `Parasite` flag (+0x159 on WarheadTypeClass):

```c
iVar1 = ParasiteClass__Constructor(this_unit);
param_1[0x1A7] = iVar1;  // byte offset 0x69C
```

### Related Fields

| Offset | Field | Set Where |
|--------|-------|-----------|
| +0x274 (0x9D*4) | **TemporalClass\*** | TechnoClass init, if weapon has Temporal |
| +0x278 | **TemporalClass\* (attacking me)** | TemporalClass::InitiateWarp |
| +0x294 (0xA5*4) | **AirstrikeClass\*** | TechnoClass init, if type has Airstrike |
| +0x2BC (0xAF*4) | **CaptureManagerClass\*** | TechnoClass init, if weapon has MindControl |
| +0x2D0 (0xB4*4) | **SpawnManagerClass\*** | TechnoClass init, if type has Spawns |
| +0x2D8 (0xB6*4) | **SlaveManagerClass\*** | TechnoClass init, if type has Slaves |
| +0x694 (0x1A5*4) | **TechnoClass\* (attacker/parasite host)** | Set by parasite/temporal attach logic |
| +0x69C (0x1A7*4) | **ParasiteClass\*** | TechnoClass init, if weapon has Parasite |

---

## 5. TechnoClass::UpdateTemporalVisual (0x0070E5A0)

Called from **TechnoClass::AI_Update** every tick. Drives the 10-phase visual
state machine for a unit being chrono-erased.

**Fields used:**
- `this+0x198` (param_1[0x66]) = Visual timer start frame
- `this+0x19C` (param_1[0x67]) = Visual timer ticks (unused/stored only)
- `this+0x1A0` (param_1[0x68]) = Visual timer duration
- `this+0x1A4` (param_1[0x69]) = **Visual phase** (0-10)

**Entry condition:** vtable+0x160 returns true (IsBeingWarped/IsWarpingOut).
If false, phase is reset to 0.

### Phase State Machine

| Phase | Duration (frames) | Visual Effect | Next Phase |
|-------|-------------------|---------------|------------|
| 0 | (instant) | Init | -> 1 |
| 1 | 6 | Fade in | -> 2 |
| 2 | 4 | Hold | -> 3 |
| 3 | Random(15..25) | Shimmer (random +/- 5 frames) | -> 4 |
| 4 | 8 | Pulse out | -> 5 |
| 5 | 16 | Pulse in | -> 4 or 6 |
| 6 | (conditional) | Wait for WarpPoints < 54 | -> 7 |
| 7 | 6 | Final fade | -> 8 |
| 8 | 4 | Final hold | -> 9 |
| 9 | 20 | Final shimmer | -> 10 (default) |
| 10 | - | Fully warped | (terminal) |

**Phase 5 -> 4/6 transition:** Calls `CDTimerClass::Remaining()` on the temporal's
warp timer. If remaining < 0x36 (54), advances to phase 6 (endgame). Otherwise
loops back to phase 4 (pulsing continues).

**Phase 6 -> 7 transition:** Waits until `CDTimerClass::Remaining() < 0x1F` (31),
then advances to phase 7 (final fade sequence).

---

## 6. TechnoClass::ScaleByWarpInVisualPhase (0x0070E4B0)

Called in the draw path to scale sprite intensity during warp-IN (teleportation
arrival). Uses fields:

- `this+0x1B4` = Warp-in timer start frame
- `this+0x1B8` = Warp-in timer ticks (unused stored)
- `this+0x1BC` = Warp-in timer duration (= remaining frames)
- `this+0x1C0` = **Warp-in visual phase**

Set in WarpAttachClass::Detach: `owner[0x1A8..0x1AA]` -> duration = weapon->Speed * 3.

### Phase Math (scale factor, 8.8 fixed-point, 0x100 = 1.0)

| Phase | Formula | Range | Effect |
|-------|---------|-------|--------|
| 1 | `(12 - remaining) * 256 / 6` | 0x000..0x200 | Linear fade in, 6 frames |
| 2 | `0x200` (constant) | 2.0x | Bright flash hold |
| 3 | `(remaining + 20) * 256 / 20` | 0x200..0x100 | Fade from 2x to 1x, 20 frames |
| 4 | `(128 - remaining) * 256 / 64` | ~0x100..0x200 | Subtle pulse out, 8 frames |
| 5 | `(remaining + 64) * 256 / 64` | ~0x200..0x100 | Subtle pulse in, 16 frames |
| 6 | `0x100` (constant) | 1.0x | Normal brightness |
| 7 | `(12 - remaining) * 256 / 6` | Same as phase 1 | Mirror of phase 1 |
| 8 | `0x200` (constant) | Same as phase 2 | Mirror of phase 2 |
| 9 | `(remaining + 20) * 256 / 20` | Same as phase 3 | Mirror of phase 3 |
| default | passthrough | | No scaling |

Result is clamped: `scale * input_value >> 8`, max 2000.

---

## 7. TechnoClass::ScaleByTemporalVisualPhase (0x0070E380)

Same structure as ScaleByWarpInVisualPhase but uses the temporal-out fields:

- `this+0x198` = Timer start frame
- `this+0x19C` = Timer ticks
- `this+0x1A0` = Timer duration
- `this+0x1A4` = Phase

### Phase Math

| Phase | Formula | Approx Scale |
|-------|---------|-------------|
| 1 | `(12 - rem) * 256 / 6` | 0..0x200 (fade in) |
| 2 | `0x200` | 2.0x (bright) |
| 3 | `(rem * 0x1CD + 0x3FC) / 20` | ~0x200..0x100 |
| 4 | `(rem * -0x4D + 0x400) / 8` | ~0x100..fluctuating |
| 5 | `(rem * 0x4D + 0x330) / 16` | fluctuating |
| 6 | `0x33` | Very dim (~20%) |
| 7 | `(rem * -0x1CD + 0xC00) / 6` | Rapid dim |
| 8 | `0x200` | 2.0x (bright flash) |
| 9 | `(rem + 20) * 256 / 20` | 0x200..0x100 |
| default | passthrough | |

---

## 8. Scale Application in Draw Path

Both `TechnoClass_DrawSHP` (0x00705D20) and `TechnoClass::Draw` (0x007065D0)
apply temporal scaling via the wrapper at **FUN_0070E360**:

```c
void FUN_0070e360(int value) {
    value = TechnoClass__ScaleByTemporalVisualPhase(value);
    value = TechnoClass__ScaleByWarpInVisualPhase(value);
    return value;
}
```

The condition for applying these scales is:
```c
if (IsWarpingOut() ||                           // vtable +0x160
    (this is a building &&                       // WhatAmI == 6
     this->AirstrikePtr != NULL &&               // +0x294 != 0
     airstrike->Target == this))                 // building self-bomb
{
    intensity = ScaleByTemporalVisualPhase(intensity);
    intensity = ScaleByWarpInVisualPhase(intensity);
}
```

The scaled intensity value is then passed to:
- **CC_Draw_Shape** for SHP sprites
- **VXL_CacheBlit** for voxel models

---

## 9. CC_Draw_Shape Warp Flag Integration (0x004AED70)

### Flag bits relevant to warp rendering

| Bit | Hex | Meaning |
|-----|-----|---------|
| 0x0002 | Transparency 25% | CloakState == 1 |
| 0x0004 | Transparency 50% | CloakState 2/3 or cloaked |
| 0x0006 | Transparency 75% | Cloaked building on bridge |
| 0x0010 | Z-buffer enable | Set if z-buffer ptr is non-null |
| 0x0020 | Shadow/ghost | Infantry crawling |
| 0x0200 | Center sprite | Always set (0x600 = 0x200 + 0x400?) |
| 0x0800 | Remap colors | Always OR'd in draw path |
| 0x1000 | **Warp blitter (low)** | Part of 0x3000 mask |
| 0x2000 | **Warp blitter (high)** | Set when z-buffer frame exists |
| 0x3000 | **Warp blitter mask** | Selects ZReadWarp blitter family |
| 0x4000 | **Alpha blitter** | Set when `param_6 != 0` (alpha flag) |

The 0x2000 flag is set in TechnoClass_DrawSHP when `param_5 != -1` (z-buffer
frame index is valid). The 0x4000 flag is set when `param_6 != 0` (alpha mode).

In TechnoClass::Draw (voxel path), the base flags start at 0x2000 and are OR'd
with transparency based on cloak state.

---

## 10. Blitter Selector (0x00490B90)

The blitter selector takes `(surface_context, flags)` and returns a blitter
object pointer. The surface_context object at param_1 has ~50 pre-allocated
blitter instances at known offsets.

### Flag Decision Tree (simplified)

```
if (flags & 0x10):          // Z-buffer enabled
    if (flags & 0x4000):    // Alpha mode
        if (flags & 0x800): return [+0xC0]  // ZReadWrite+Alpha+Remap
        else:               return [+0x58]  // ZRead+Alpha
    if (flags & 0x3000):    // Warp mode
        if (flags & 0x800): return [+0x9C]  // ZReadWarp+Remap
        else:               return [+0x30]  // ZReadWarp
    if (flags & 0x800):     return [+0x70]  // ZRead+Remap
    else:                   return [+0x14]  // ZRead plain

if ((flags & 6) == 2):     // 25% transparency
    if (flags & 0x4000):    // Alpha
        ...
    if (flags & 0x3000):    // Warp
        if (flags & 8):     // + darken
            if (flags & 0x800): return [+0xB4]
            else:               return [+0x48]
        if (flags & 0x800): return [+0xA8]
        else:               return [+0x3C]
    ...

if ((flags & 6) == 4):     // 50% transparency
    ... (same pattern with different blitter offsets)

if ((flags & 6) == 6):     // 75% transparency
    ... (same pattern)
```

### ZReadWarp Blitter Variants (the 0x3000 mask)

The RTTI class names from the binary reveal 12 ZReadWarp blitter types:

| Class Name | What it does |
|-----------|-------------|
| `BlitTransLucent25ZReadWarp<unsigned_short>` | 25% transparent + warp Z-read |
| `BlitTransLucent25AlphaZReadWarp<unsigned_short>` | 25% + alpha + warp |
| `BlitTransLucent50ZReadWarp<unsigned_short>` | 50% transparent + warp |
| `BlitTransLucent50AlphaZReadWarp<unsigned_short>` | 50% + alpha + warp |
| `BlitTransLucent75ZReadWarp<unsigned_short>` | 75% transparent + warp |
| `BlitTransLucent75AlphaZReadWarp<unsigned_short>` | 75% + alpha + warp |

The "Warp" blitters differ from normal ZRead blitters in how they handle the
Z-buffer comparison: instead of the standard less-than depth test, warp
blitters use a **write-through** mode that allows the warping unit's sprite
to render on top of terrain while still participating in the Z-buffer for
correct ordering with other sprites. The warp effect itself (the shimmering
distortion) comes from the intensity scaling applied BEFORE the draw call,
not from the blitter.

---

## 11. TemporalClass::AI (0x006297F0) -- The Full State Machine

This is the main per-tick function called on each active TemporalClass from
the global array. param_1 fields (int* type, so indices are dword-based):

- +0x24 = Owner TechnoClass*
- +0x28 = Target TechnoClass*
- +0x44 = Anim pointer (warp effect)
- +0x48 = **State** (0-4, the warp attack phase)
- +0x4C = Sub-counter
- +0x50 = Sub-counter 2

### States

| State | Name | What happens |
|-------|------|-------------|
| 0 | Init | Creates warp anim (from Rules+0x340 AnimType), starts tracking, -> state 1 |
| 1 | Charging | Calls FUN_00629720 (advance warp timer). When timer expires, randomly pick state 2 or 3, spawn warp sparks (FUN_00629E90) |
| 2 | Oscillate Left | Compute cos(counter * rate) -> building rotation offset at +0x328 |
| 3 | Oscillate Right | Same but with -1.0 multiplier on angle |
| 4 | Fire/Damage | Clear rotation. Create 3 random small anims (from Rules RubbleAnims array at +0xBC4, count at +0xBD0). If target has WarpAway flag (+0x174) and warhead grants experience, attempt full erase via WarpAttachClass::Detach. Otherwise apply damage via vtable+0x16C (ReceiveDamage). If target survives, return to state 2/3. |

### State 4 detail -- The Erase

When the temporal determines the target should be erased (flag +0x174 on
TypeClass, plus experience conditions):
1. Grant veterancy to owner (if applicable)
2. Call **WarpAttachClass::Detach** on the target's attached parasite if any
3. Set `target+0x3CD = 1` (mark for removal)
4. Call `target->RegisterKill(owner)` and `target->Scatter()`
5. Clean up all state, remove from global array

---

## 12. Update Call Chain Clarification

There are TWO update paths for TemporalClass:

### Path A: TemporalClass::Update (0x0071A760) -- Virtual, from vtable

Called through the vtable (entry in vtable at 0x7F51DC). This is called from the
global TemporalClass array iteration during the main game loop. It handles:
- Range checking (detach if owner is too far from target)
- Damage accumulation from the linked list chain
- WarpPoints decrement
- Target erasure when WarpPoints reach 0

### Path B: WarpAttachClass::UpdateAttack (0x00629FD0) -> TemporalClass::AI (0x006297F0)

Called from the **shared base class update** that dispatches between temporal and
parasite behavior. The dispatch condition:

```c
if (owner->TypeClass->field_0xCCE != 0   // Temporal flag
    && owner->TypeClass->field_0xD97 != 0)  // WarpAway flag
{
    TemporalClass__AI();  // 5-state attack animation machine
}
else
{
    // Parasite attack: particle effects, direct damage
}
```

TemporalClass::AI handles the visual/animation state machine (oscillation, spark
effects, final erase attempt with experience). It calls WarpAttachClass::Detach
when the erase completes.

### How they relate

- **TemporalClass::Update** (0x0071A760) = the per-tick WarpPoints countdown and
  target erasure (called from global array, handles the "health bar" of the warp)
- **TemporalClass::AI** (0x006297F0) = the visual attack state machine with
  oscillation and spark anims (called from the shared base class update)
- **WarpAttachClass::Detach** (0x0062A4A0) = teleport/place the owner unit at
  the target location when the warp completes

---

## 13. Ghidra Labels Applied (This Session)

| Address | Old Name | New Name |
|---------|----------|----------|
| 0x0071A760 | FUN_0071A760 | TemporalClass__Update |
| 0x0071ADE0 | FUN_0071ADE0 | TemporalClass__ClearLinkedList |
| 0x0071AB10 | FUN_0071AB10 | TemporalClass__SumChainDamage |
| 0x006297F0 | FUN_006297F0 | TemporalClass__AI |
| 0x0062A4A0 | FUN_0062A4A0 | WarpAttachClass__Detach |
| 0x0062AB40 | FUN_0062AB40 | WarpAttachClass__CanPlaceAtTarget |
| 0x00629E90 | FUN_00629E90 | WarpAttachClass__SpawnWarpAnims |
| 0x0071AE50 | FUN_0071AE50 | TemporalClass__CanWarpTarget |
| 0x00629FD0 | FUN_00629FD0 | WarpAttachClass__UpdateAttack |

Previously labeled (already existed):
- 0x0071AF20 = TemporalClass__InitiateWarp
- 0x0071ABC0 = TemporalClass__DetachFromTarget
- 0x0071ACD0 = TemporalClass__ClearWarpingOutOnTarget
- 0x0071B1B0 = TemporalClass__Destructor
- 0x0071A450 = TemporalClass__Constructor (no args)
- 0x0071A4E0 = TemporalClass__Constructor (with owner)
- 0x0071A660 = TemporalClass__Load
- 0x0070E5A0 = TechnoClass__UpdateTemporalVisual
- 0x0070E4B0 = TechnoClass__ScaleByWarpInVisualPhase
- 0x0070E380 = TechnoClass__ScaleByTemporalVisualPhase
- 0x0070E000 = TechnoClass__ApplyTemporalDamage

---

## 14. Implementation Notes for Rust Engine

### Key structures needed

1. **TemporalClass** with doubly-linked list support and CDTimer
2. **Warp visual phase state machine** (10 phases for warp-out, ~9 phases for warp-in)
3. **Scale pipeline** in the draw path: chain ScaleByTemporalVisualPhase and ScaleByWarpInVisualPhase
4. **Blitter flag system**: the 0x3000 mask selects warp-mode Z-buffer blitters

### Warp rendering in GPU pipeline

The original game's "warp" visual is NOT a special shader effect. It's simply:
- **Intensity scaling** via the phase state machines (makes the unit pulse/fade)
- **Z-buffer write mode** via the ZReadWarp blitters (allows correct Z-ordering during warp)
- The "shimmer" comes from the phase 3/5 oscillation between bright and dim

For the wgpu renderer, this translates to:
- Multiply sprite color by the scale factor (8.8 fixed point, where 0x100 = 1.0)
- Use the standard Z-buffer pipeline (no special warp Z mode needed with modern GPU)
- The pulsing/fading effect is purely an intensity modulation per frame
