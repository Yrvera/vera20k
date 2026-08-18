# Mind Control System — Ghidra Research Report

Reverse-engineered from `gamemd.exe`. Confidence: **high** (verified from binary decompilation
and disassembly, cross-referenced with INI string xrefs).

## Overview

Mind control is implemented through `CaptureManagerClass`, a per-unit manager that tracks which
units a controller has mind-controlled. It is one of the mutually exclusive "special warhead"
effects in `WarheadTypeClass::Detonate` (0x004690B0), alongside IvanBomb, Temporal, Parasite, etc.

The system consists of:
1. **WarheadTypeClass** flag `MindControl=yes` that triggers the MC path on detonation
2. **WeaponTypeClass** fields that configure capacity and infinite mode
3. **CaptureManagerClass** that manages the list of controlled units
4. **TechnoClass** fields for the MC relationship (controller pointer, MC anim)
5. **RulesClass** global settings for visuals and AI behavior

## INI Keys and Their Binary Offsets

### WarheadTypeClass (parsed in WarheadTypeClass::ReadINI @ 0x0075D590)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `MindControl` | 0x155 | bool | Enables mind control warhead effect |

Verified: string "MindControl" at 0x0081BBC8, xref at 0x0075D7CF pushes it for
`CCINIClass::ReadBool`, result stored at `ESI + 0x155`.

The check in `Detonate` (0x00469211): `CMP byte [warheadType + 0x155], 0` — if nonzero,
branches into MC path instead of normal damage.

**Mutually exclusive chain** in Detonate (if-else cascade starting at 0x00469211):
- 0x155: MindControl
- 0x156: Poison
- 0x157: IvanBomb
- 0x158: (next special)
- 0x159: Temporal
- 0x15A: (next special)
- 0x15B: (next special)

### WeaponTypeClass (parsed in WeaponTypeClass::ReadINI @ 0x00772080)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `InfiniteMindControl` | 0x140 | bool | Unlimited number of controlled units |
| `Damage` | 0xA4 | int | Used as `maxControl` count for CaptureManager |

When `InfiniteMindControl=yes`, the controller can capture unlimited units.
Otherwise, `Damage` determines how many units can be controlled simultaneously.

### TechnoTypeClass (parsed in TechnoTypeClass::ReadINI @ 0x00712170)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `MindControlRingOffset` | 0x60C | int | Z-offset (leptons) for MC ring anim on victim |
| `LeptonMindControlOffset` | 0x3DC | int | Z-offset for MC link line endpoint on victim |
| `ImmuneToPsionics` | 0xD35 | bool | Unit cannot be mind-controlled |
| `ImmuneToPsionicWeapons` | 0xD36 | bool | Immune to psionic weapon fire |

### RulesClass (parsed in RulesClass::ReadAudioVisual @ 0x0066BBB0 and CombatDamage section)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `MindControlAttackLineFrames` | 0x310 | int | Duration (frames) the MC link line is visible |
| `ControlledAnimationType` | 0x320 | AnimType* | Anim played on victim while MC'd |
| `PermaControlledAnimationType` | 0x324 | AnimType* | Anim for permanent MC (controller dead) |
| `YuriMindControlSound` | (in audio) | — | Sound played on successful capture |

### TechnoClass Instance Fields

| Offset | Type | Description |
|--------|------|-------------|
| 0x2BC | CaptureManagerClass* | Controller's MC manager (null if not an MC unit) |
| 0x2C0 | TechnoClass* | Victim's pointer back to its controller (MindControlledBy) |
| 0x2C4 | bool | `IsMindControlled` flag (set to 1 on capture) |
| 0x2C8 | AnimClass* | MC ring anim attached to victim |

## CaptureManagerClass Struct Layout

**Size: 0x50 (80 bytes)** — confirmed by `GetSize()` returning 0x50 and `operator_new(0x50)`.
**Class ID: 0x42** — returned by `GetClassID()`.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 4 | ptr | vtable (primary) | `vtable__CaptureManagerClass` @ 0x007E4B40 |
| 0x04 | 4 | ptr | vtable (INoticeSink) | secondary vtable #1 |
| 0x08 | 4 | ptr | vtable (IRTTITypeInfo) | secondary vtable #2 |
| 0x0C | 4 | ptr | vtable (INoticeSink) | secondary vtable #3 |
| 0x10-0x20 | 16 | — | AbstractClass base fields | Inherited from AbstractClass |
| 0x24 | 4 | ptr | DynamicVector vtable | `PTR_FUN_007E4BA4` |
| 0x28 | 4 | ptr | nodes_data | Pointer to array of `MCNode*` |
| 0x2C | 4 | int | nodes_capacity | DynamicVector allocated capacity |
| 0x30 | 1 | bool | nodes_is_valid | DynamicVector valid flag |
| 0x31 | 1 | bool | (unknown flag) | Initialized to 0 |
| 0x34 | 4 | int | nodes_count | Current number of controlled units |
| 0x38 | 4 | int | nodes_grow_step | Growth increment (default: 10) |
| 0x3C | 4 | int | max_control | Max units that can be controlled |
| 0x40 | 1 | bool | infinite_mind_control | From weapon's InfiniteMindControl |
| 0x41 | 1 | bool | (spark flag) | Related to MC damage sparks |
| 0x44 | 4 | int | spark_delay | Spark visual delay counter |
| 0x48 | 4 | ptr | owner | Pointer to owning TechnoClass |
| 0x4C | 4 | int | update_timer | Countdown for next MC decay check (init: 0x1E = 30) |

### MCNode Sub-struct (Mind Control Link Node)

**Size: 0x14 (20 bytes)** — allocated via `operator_new(0x14)`.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 4 | ptr | victim | Pointer to controlled TechnoClass |
| 0x04 | 4 | ptr | original_owner | HouseClass the victim belonged to before capture |
| 0x08 | 4 | int | capture_frame | Frame number when capture occurred (-1 = permanent) |
| 0x0C | 4 | int | (unused/padding) | |
| 0x10 | 4 | int | link_visible_frames | Duration to show MC link line (from `MindControlAttackLineFrames`) |

## CaptureManagerClass Creation

**Where:** `FUN_006F3F40` (TechnoClass init helpers), called during unit creation.

**Condition:** The unit's primary weapon's warhead must have `MindControl=yes` (checked at
`warheadType + 0x155`).

**Constructor call:**
```
CaptureManagerClass::Constructor(
    owner_techno,                    // TechnoClass* that owns this manager
    weapon->Damage,                  // int maxControl (WeaponTypeClass + 0xA4)
    weapon->InfiniteMindControl      // bool (WeaponTypeClass + 0x140)
)
```

The new CaptureManager is stored at `TechnoClass + 0x2BC`.

## Key Functions

### CaptureManagerClass::CaptureUnit @ 0x00471D40

**Signature:** `bool __thiscall CaptureUnit(CaptureManagerClass* this, TechnoClass* target)`

**Called from:** `WarheadTypeClass::Detonate` @ 0x004692D0

**Flow:**
1. If `target` is null, return false
2. Cast target to FootClass (check `AbstractFlags & 1`)
3. Call `CanCapture(target)` — if false, return false
4. **Override mode** (maxControl == 1): free ALL currently controlled units first
5. Get target's current owner via `GetHouse()` and controller's owner
6. Call `target->SetOwner(controller_owner)` — transfers ownership
7. Allocate new MCNode (0x14 bytes):
   - `node->victim = target`
   - `node->original_owner = target_old_owner`
   - `node->capture_frame = g_CurrentFrameCounter`
   - `node->link_visible_frames = RulesClass->MindControlAttackLineFrames` (offset 0x310)
8. Add node to the DynamicVector (grow if needed)
9. Set `target->MindControlledBy = controller` (victim offset 0x2C0)
10. Call `DecideUnitFate(target)` for AI disposition
11. **Skip scatter** for certain building-type victims (mission 0x10=Unload, 0x13, 0x12)
12. Otherwise call `target->Scatter()` to make it move
13. Create the MC ring anim: `AnimClass::Constructor(RulesClass->ControlledAnimationType)`
14. Attach anim to victim via `FUN_00424B50`
15. If victim is a building, set anim Z-offset to `LeptonMindControlOffset` (0xFFFFFC00 = -1024)
16. Store anim pointer at `victim + 0x2C8`
17. Return true

### CaptureManagerClass::CanCapture @ 0x00471C90

**Signature:** `bool __thiscall CanCapture(CaptureManagerClass* this, TechnoClass* target)`

**Returns true if ALL of the following are met:**
1. Target is not null
2. Target's owner is different from controller's owner (can't MC own units)
3. Target's type does NOT have `ImmuneToPsionics` set (TechnoTypeClass + 0xD35)
4. Target does NOT have a Temporal warp active (`target + 0x2E4 != 0` check)
5. Target is not immune via `FUN_007105E0` (general immunity check)
6. Target does not already have a `MindControlledBy` pointer (`target + 0x2C0 == 0`)
7. Target is not currently in Iron Curtain / Force Shield (`IsIronCurtained()`)
8. **Capacity check**: one of:
   - `InfiniteMindControl` flag is set, OR
   - `nodes_count < max_control` (still have room), OR
   - `max_control == 1` (override mode — will free existing victim)
9. Target mission is not 0x13 or 0x12 (selling/certain states blocked)

### CaptureManagerClass::FreeUnit @ 0x00471FF0

**Signature:** `bool __thiscall FreeUnit(CaptureManagerClass* this, TechnoClass* victim)`

**Called when:** a specific controlled victim needs to be released (override, death, etc.)

**Flow:**
1. Iterate the node array looking for the node whose `victim` matches
2. Remove the MC ring anim from victim (`victim->MCAnim->Remove()`, clear `victim + 0x2C8`)
3. Play the "freed from MC" sound (from TechnoTypeClass, or `RulesClass + 0x264` default)
4. Restore victim's owner: `victim->SetOwner(node->original_owner, redraw=true)`
5. Call `DecideUnitFate(victim)` for AI disposition of the freed unit
6. Clear `victim->MindControlledBy` (offset 0x2C0 = 0)
7. Free the MCNode memory
8. Remove node from DynamicVector (shift remaining entries down)

### CaptureManagerClass::FreeAll @ 0x00472140

**Signature:** `void __fastcall FreeAll(CaptureManagerClass* this)`

Simple loop: iterates all nodes in reverse and calls `FreeUnit()` on each victim.

**Called from:**
- `TechnoClass::ReceiveDamage` @ 0x00702112 — when the controller dies
- `BuildingClass::UpdateGapAndSpecialEffects` @ 0x00454B47
- `BuildingClass::EnterTransport` @ 0x0070FDBD — when controller enters transport
- `TemporalClass::InitiateWarp` @ 0x0071AF48 — when controller is Chronoshifted
- `FUN_004DE5D0` @ 0x004DE5DD — related to unit removal
- `FUN_00710460` @ 0x0071046A

### CaptureManagerClass::Update @ 0x00471A50

**Signature:** `void __fastcall Update(CaptureManagerClass* this)`

**Called from:** `TechnoClass::AI_Update` @ 0x006FA730, every tick if CaptureManager exists.

**Behavior:**
- Checks `this + 0x40` (InfiniteMC flag) — if not set, does nothing special
- Decrements `spark_delay` counter (offset 0x44)
- Decrements `update_timer` (offset 0x4C)
- When `update_timer` reaches 0:
  - Looks up MC decay parameters from RulesClass tables at offsets 0xEEC..0xF24
  - Applies damage to controlled units based on MC distance/duration tables
  - Creates spark particle effects (5 particles via `ParticleSystemClass::Constructor`)
  - May cause victims to "wobble" (random heading change, +-0.015 or +-0.03 radians)
  - Resets update_timer from RulesClass tables

### CaptureManagerClass::DrawLinks @ 0x00472160

**Signature:** `void __fastcall DrawLinks(CaptureManagerClass* this)`

**Called from:** `TacticalClass::Draw` @ 0x006D47BF, when rendering the game view.

**Precondition:** `ShouldDrawLinks()` must return true.

**Flow:**
1. Iterates all nodes in reverse order
2. For each node:
   - Checks if the victim is alive/valid (offset 0x83 — IsAlive)
   - Calculates remaining link visibility: `link_visible_frames - (current_frame - capture_frame)`
   - If `capture_frame == -1`, link is permanent (always visible while duration > 0)
3. If controller or victim is alive:
   - Gets victim's 3D position (`victim->x, y, z`)
   - Adds `LeptonMindControlOffset` (TechnoTypeClass + 0x3DC) to Z coordinate
   - Gets controller's position via `GetScreenCoords()`, with per-link offset (`-1 - index % 5`)
   - Reads MC line color from `controller->House + 0x56F9` (house-specific MC line color)
   - Calls `FUN_00704E40` to draw the colored line between two 3D points

### CaptureManagerClass::ShouldDrawLinks @ 0x00472640

**Signature:** `bool __fastcall ShouldDrawLinks(CaptureManagerClass* this)`

Returns true if:
- The controller is selected (offset 0x83), OR
- The controller's transport is selected, OR
- Any controlled victim is selected, OR
- Any controlled victim has remaining link visibility time

### CaptureManagerClass::DecideUnitFate @ 0x004723B0

**Signature:** `void __thiscall DecideUnitFate(CaptureManagerClass* this, TechnoClass* victim)`

AI decision function for what to do with a newly captured (or released) unit. Uses RulesClass
tables for probability-based outcomes:

**Decision categories** (based on house power ratio and victim health):
- Category 0: Low power → lookup from `RulesClass + 0xEA0`
- Category 1: Powered but weak → lookup from `RulesClass + 0xE84`
- Category 2: Healthy → lookup from `RulesClass + 0xE68`
- Category 3: Very healthy → lookup from `RulesClass + 0xE4C`

**Outcomes** (random roll 1-100 against probability tables):
1. Join team of capturing unit
2. Scatter (`victim->Scatter()`)
3. Hunt (`victim->SetMission(Hunt)`)
5. Do nothing (keep current behavior)
Default: `victim->SetMission(0xF)` — Guard mission

Logged with debug string: `"AICapture: I think, %s, so I roll %d => %s"`.

### CaptureManagerClass::GetOriginalOwner @ 0x004722F0

Given a victim TechnoClass pointer, searches the node array and returns the stored
`original_owner` HouseClass pointer (node offset 0x04). Returns 0 if not found.

### CaptureManagerClass::SetOriginalOwner @ 0x00472330

Updates the `original_owner` field for a specific victim in the node array. Used when
a victim needs to be re-assigned to a different original owner (e.g., house changes).

## Mind Control Capture Flow (End-to-End)

### 1. Weapon fires, bullet hits target

`BulletClass::AI` calls `WarheadTypeClass::Detonate` (0x004690B0).

### 2. Detonate checks warhead specials

At 0x0046920B, checks `warheadType->MindControl` (offset 0x155). If true:
- Gets the firer's TechnoClass from `BulletClass + 0xB0`
- Gets the firer's CaptureManager from `TechnoClass + 0x2BC`
- If either is null, skips to normal damage label

### 3. Pre-capture effects

- If firer is an InfantryClass with valid house, plays the MC EVA event
- Triggers house-specific voiceline events (EVA event 6 and 0x2C)

### 4. CaptureUnit is called

`CaptureManagerClass::CaptureUnit(target)` at 0x004692D0.

### 5. Ownership transfer

Inside CaptureUnit:
- Victim's owner is changed to the controller's owner via `SetOwner()`
- The original owner is saved in the MCNode for later restoration
- MC ring anim is created and attached to the victim

### 6. Per-tick update

`TechnoClass::AI_Update` calls `CaptureManagerClass::Update` every frame at 0x006FA730.
This handles MC decay, spark effects, and victim wobble.

### 7. Visual rendering

`TacticalClass::Draw` at 0x006D47BF calls `DrawLinks()` to render the colored line
between controller and each victim. The line color comes from the controller's house.

## Controller Death / Release Mechanics

### When the controller dies

`TechnoClass::ReceiveDamage` (0x00701D40) calls `CaptureManagerClass::FreeAll()` at
0x00702112 when the unit is destroyed (health <= 0).

**FreeAll iterates all victims and for each:**
1. Removes the MC ring anim from the victim
2. Restores the victim's original owner (from the MCNode)
3. Calls `DecideUnitFate()` for AI disposition
4. Clears the victim's `MindControlledBy` pointer
5. Frees the MCNode

**Result:** All victims return to their original owners and resume independent behavior.

### When the controller enters a transport

`BuildingClass::EnterTransport` (0x0070FDBD) also calls `FreeAll()` — entering a transport
releases all MC'd units.

### When the controller is Chronoshifted

`TemporalClass::InitiateWarp` (0x0071AF48) calls `FreeAll()` — temporal warping releases
all MC'd units.

### When a specific victim dies

`UnitClass::Mission_Enter` (0x0073A2CD, 0x0073A72B) calls `FreeUnit()` to release a
specific victim when it enters a transport or dies.

## InfiniteMindControl Flag

**INI:** `InfiniteMindControl=yes` on WeaponTypeClass
**Binary offset:** WeaponTypeClass + 0x140
**CaptureManager offset:** 0x40

When set:
- `CanCapture()` always passes the capacity check (unlimited victims)
- The DynamicVector grows dynamically (step size 10) to accommodate new nodes
- `Update()` still runs decay/spark effects on all victims

When not set:
- `max_control` (from weapon `Damage`) limits the number of simultaneous victims
- If `max_control == 1`: "override mode" — capturing a new victim automatically frees
  the previous one first
- If `max_control > 1` and at capacity: `CanCapture()` returns false, MC attempt fails

## Permanent Mind Control

There is no explicit "permanent" MC flag in the INI. Permanence is handled by the
`PermaControlledAnimationType` in RulesClass:

- When a controller dies and `FreeAll()` restores victims to original owners, the game
  plays `PermaControlledAnimationType` (RulesClass + 0x324) instead of the normal
  `ControlledAnimationType` (RulesClass + 0x320) on victims that remain alive
- The MCNode's `capture_frame` field is set to -1 when the link becomes "permanent"
  (no longer has a visible duration countdown)

In the `DrawLinks` function, `capture_frame == -1` means the link line is always visible
(as long as `link_visible_frames > 0`).

## Mind Control Link Visuals

### MC Ring Anim (on victim)

- Animation type: `RulesClass->ControlledAnimationType` (offset 0x320)
- Permanent version: `RulesClass->PermaControlledAnimationType` (offset 0x324)
- Z-offset for buildings: `LeptonMindControlOffset` = -1024 leptons (0xFFFFFC00)
- Z-offset (general): `TechnoTypeClass->MindControlRingOffset` (offset 0x60C)
- Stored at: `TechnoClass + 0x2C8` (victim's anim pointer)
- Attached via `FUN_00424B50` which links anim lifecycle to the techno

### MC Link Line (between controller and victim)

- Duration: `RulesClass->MindControlAttackLineFrames` (offset 0x310) frames
- Color: House-specific, read from `HouseClass + 0x56F9`
- Drawn by: `FUN_00704E40` (generic 3D line drawing function)
- Controller endpoint: controller position with per-link index offset (`-1 - index % 5`)
- Victim endpoint: victim position + `TechnoTypeClass->LeptonMindControlOffset` (offset 0x3DC) on Z
- Only drawn when controller or victim is selected, or link duration hasn't expired

## Function Address Summary

| Address | Name | Size | Description |
|---------|------|------|-------------|
| 0x004717D0 | Constructor (full) | — | `(owner, maxControl, infiniteMC)` |
| 0x00471890 | Constructor (default) | — | Parameterless, for save/load |
| 0x00471A50 | Update | 574 | Per-tick decay and spark effects |
| 0x00471C90 | CanCapture | 176 | Checks if target can be MC'd |
| 0x00471D40 | CaptureUnit | 690 | Main capture execution |
| 0x00471FF0 | FreeUnit | 332 | Release a specific victim |
| 0x00472140 | FreeAll | 30 | Release all victims (loop) |
| 0x00472160 | DrawLinks | 314 | Render MC link lines |
| 0x004722F0 | GetOriginalOwner | 50 | Lookup original HouseClass for victim |
| 0x00472330 | SetOriginalOwner | 118 | Update original owner for victim |
| 0x004723B0 | DecideUnitFate | 566 | AI decision for captured/freed unit |
| 0x00472640 | ShouldDrawLinks | 176 | Check if links should be rendered |
| 0x00472720 | Save | 436 | Serialization |
| 0x004728E0 | Load | 127 | Deserialization |
| 0x00472960 | Detach | 56 | COM-style detach |
| 0x004729A0 | GetSize | 6 | Returns 0x50 |
| 0x004729B0 | GetClassID | 6 | Returns 0x42 |
| 0x004729C0 | Destructor | 182 | Cleanup and free nodes |

## Related Addresses

| Address | Context |
|---------|---------|
| 0x007E4B40 | CaptureManagerClass primary vtable (20 entries) |
| 0x007E4BA4 | DynamicVector vtable used for node storage |
| 0x006F3F40 | TechnoClass init — creates CaptureManager if weapon has MC warhead |
| 0x004690B0 | WarheadTypeClass::Detonate — MC dispatch at 0x00469211 |
| 0x006F9E50 | TechnoClass::AI_Update — calls Update at 0x006FA730 |
| 0x006D3D10 | TacticalClass::Draw — calls DrawLinks at 0x006D47BF |
| 0x00701D40 | TechnoClass::ReceiveDamage — calls FreeAll at 0x00702112 |
| 0x0089E0F0 | Global DynamicVector of all CaptureManagerClass instances |
| 0x00424B50 | Anim attach function — links MC ring anim to victim |
| 0x00704E40 | 3D line drawing function — renders MC link lines |
