# Mind Control System — Comprehensive Ghidra Research Report

Reverse-engineered from `gamemd.exe`. Confidence: **high** (verified from binary decompilation
and disassembly, cross-referenced with INI string xrefs and in-repo INI files).

This is an expanded version of the original `MIND_CONTROL_GHIDRA_REPORT.md`, with additional
findings on the Psychic Dominator permanent MC path, the Mastermind overload damage system,
interaction rules, and corrected field descriptions.

---

## 1. System Overview

Mind control in YR is implemented through `CaptureManagerClass`, a per-unit manager allocated
on any TechnoClass whose primary weapon has a `MindControl=yes` warhead. The manager tracks
all units currently under control and handles capture, release, visual links, and (for
InfiniteMindControl) the Mastermind overload damage system.

There are **two distinct mind control mechanisms** in gamemd.exe:

1. **CaptureManager-based MC** — Reversible. Controller tracks victims via MCNode linked list.
   When controller dies, all victims revert to their original owners. This is the standard
   Yuri Clone / Yuri Prime / Psychic Tower / Mastermind system.

2. **Psychic Dominator permanent MC** — Irreversible. Directly transfers ownership without any
   CaptureManager link. Sets `TechnoClass + 0x2C4` (PermanentlyMindControlled flag) to 1.
   Uses `PermaControlledAnimationType` for the ring anim. No controller pointer is stored.
   The unit belongs to the new owner permanently.

### Mutually Exclusive Warhead Specials

In `WarheadTypeClass::Detonate` (0x004690B0), the special warhead effects are checked as an
if-else cascade. Only ONE fires per detonation:

| Priority | WH Offset | INI Key | Description |
|----------|-----------|---------|-------------|
| 1 | 0x155 | `MindControl` | Mind control capture |
| 2 | 0x157 | `IvanBomb` | Ivan bomb attachment |
| 3 | 0x158 | `ElectricAssault` | Electric assault |
| 4 | 0x159 | `Temporal` | Chrono warp / erase |
| 5 | 0x15A | `Parasite` | Parasite attachment |
| 6 | 0x15B | (unknown special) | Another special WH |
| 7 | 0x16C | `IsLocomotor` | Locomotor override |
| 8 | 0x14F | (tractor beam) | Tractor beam (inf only) |
| 9 | 0x16E | `BombDisarm` | Disarm bombs |
| 10 | 0x175 | `MakesDisguise` | Force disguise |
| 11 | 0x176 | `NukeMaker` | Spawn nuke |
| — | — | (normal damage) | `Apply_area_damage()` |

---

## 2. INI Keys and Binary Offsets

### 2.1 WarheadTypeClass (ReadINI @ 0x0075D590)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `MindControl` | 0x155 | bool | Enables mind control warhead path |

String "MindControl" at 0x0081BBC8. Read at 0x0075D7CF. Stored to `ESI + 0x155`.

### 2.2 WeaponTypeClass (ReadINI @ 0x00772080)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `InfiniteMindControl` | 0x140 | bool | Unlimited MC capacity (Mastermind mode) |
| `Damage` | 0xA4 | int | Used as max control count for CaptureManager |

String "InfiniteMindControl" at 0x0084948C. Read at 0x00772218.

When `InfiniteMindControl=yes`, the weapon's `Damage` value only affects pip display,
not the actual control limit (which becomes unlimited). However, exceeding the
`OverloadCount` thresholds triggers Mastermind overload damage to the controller.

### 2.3 TechnoTypeClass (ReadINI @ 0x00712170)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `ImmuneToPsionics` | 0xD35 | bool | Unit cannot be mind-controlled |
| `MindControlRingOffset` | 0x60C | int | Z-offset (leptons) for MC ring anim on victim |
| `LeptonMindControlOffset` | 0x3DC | int | Z-offset for MC link line endpoint |
| `MindClearedSound` | 0x5B0 | VocIndex (int) | Per-type sound on MC release (-1 = use global) |

### 2.4 RulesClass — [AudioVisual] Section (ReadAudioVisual @ 0x00669360)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `YuriMindControlSound` | 0x214 | VocIndex | Sound on successful MC capture |
| `MindClearedSound` | 0x264 | VocIndex | Default sound on MC release |
| `MasterMindOverloadDeathSound` | 0x258 | VocIndex | Sound when Mastermind dies from overload |

### 2.5 RulesClass — [CombatDamage] Section (@ 0x0066BBB0)

| INI Key | Offset | Type | Description |
|---------|--------|------|-------------|
| `MindControlAttackLineFrames` | 0x310 | int | Duration (frames) MC link line is visible |
| `ControlledAnimationType` | 0x320 | AnimType* | Anim on victim while MC'd |
| `PermaControlledAnimationType` | 0x324 | AnimType* | Anim for permanent MC (Psychic Dominator) |
| `OverloadCount` | 0xEE8 | DynVector<int> | Threshold array for Mastermind tiers | (corrected 2026-05-29: was 0xEEC; binary shows `FUN_004779e0(param_1 + 0xee8)` in RulesClass__ReadCombatDamage — OFFSET_RETYPED_WRONG)
| `OverloadDamage` | 0xF04 | DynVector<int> | Damage per tick per tier | (corrected 2026-05-29: was 0xF08; binary shows `FUN_004779e0(param_1 + 0xf04)` — OFFSET_RETYPED_WRONG)
| `OverloadFrames` | 0xF20 | DynVector<int> | Tick interval per tier | (corrected 2026-05-29: was 0xF24; binary shows `FUN_004779e0(param_1 + 0xf20)` — OFFSET_RETYPED_WRONG)

**DynamicVector layout note:** Each DynVector occupies 0x1C bytes. The data pointer is at
+0x04, capacity at +0x08, valid flag (byte) at +0x0D, count at +0x10, grow_step at +0x14.
(corrected 2026-05-29: original note said "count at +0x08" — WRONG; CaptureManagerClass constructor
shows nodes_count at param_1[0xd]=offset 0x34 and DynVector base at 0x24, so count is at base+0x10;
Update() reads count from RulesClass+0xEF8 = 0xEE8+0x10 — ROOT_CAUSE: OFFSET_RETYPED_WRONG)
- `OverloadCount.data_ptr` = RulesClass + 0xEEC (= base 0xEE8 + 0x04), `.count` = 0xEF8 (base+0x10)
  (corrected 2026-05-29: was data=0xEF0/count=0xEF4; Update() reads data ptr from 0xEEC, count from 0xEF8 — OFFSET_RETYPED_WRONG)
- `OverloadDamage.data_ptr` = RulesClass + 0xF08 (= base 0xF04 + 0x04), `.count` = 0xF14 (base+0x10)
  (corrected 2026-05-29: was data=0xF0C; Update() reads data ptr from 0xF08 — OFFSET_RETYPED_WRONG)
- `OverloadFrames.data_ptr` = RulesClass + 0xF24 (= base 0xF20 + 0x04), `.count` = 0xF30 (base+0x10)
  (corrected 2026-05-29: was data=0xF28; Update() reads data ptr from 0xF24 — OFFSET_RETYPED_WRONG)

Default INI values (from rulesmd.ini):
```ini
OverloadCount=3,6,10,50
OverloadDamage=0,50,100,500
OverloadFrames=30,60,60,60
```

---

## 3. TechnoClass Instance Fields

| Offset | Type | Field Name | Description |
|--------|------|-----------|-------------|
| 0x2BC | CaptureManagerClass* | CaptureManager | MC manager (on controller). Null if not MC unit |
| 0x2C0 | TechnoClass* | MindControlledBy | Pointer to the controller (on victim). 0 if not MC'd |
| 0x2C4 | bool | PermanentlyMindControlled | Set by Psychic Dominator. Irreversible. No controller link |
| 0x2C8 | AnimClass* | MindControlAnim | MC ring animation on victim |

**Important correction:** Offset 0x2C4 is NOT a general "IsMindControlled" flag. It is
specifically the **permanent mind control** flag set only by the Psychic Dominator superweapon
(at `PsychicDominator__MindControlArea` @ 0x0053B080). Regular CaptureManager-based MC does
NOT set this flag.

The `TechnoClass__IsMindControlled` function (0x007105E0) checks BOTH paths:
```c
bool IsMindControlled(TechnoClass* this) {
    return (this->MindControlledBy != NULL) || (this->PermanentlyMindControlled != 0);
}
```

---

## 4. CaptureManagerClass Struct Layout

**Size: 0x50 (80 bytes)** — confirmed by `GetSize()` @ 0x004729A0 returning 0x50.
**Class ID: 0x42** — from `GetClassID()` @ 0x004729B0.
**Allocated with:** `operator_new(0x50)` in `TechnoClass__Init_Managers`.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 4 | ptr | vtable (primary) | `vtable__CaptureManagerClass` @ 0x007E4B40 |
| 0x04 | 4 | ptr | vtable (INoticeSink) | Secondary vtable #1 |
| 0x08 | 4 | ptr | vtable (IRTTITypeInfo) | Secondary vtable #2 |
| 0x0C | 4 | ptr | vtable (INoticeSink) | Secondary vtable #3 |
| 0x10-0x23 | 20 | — | AbstractClass fields | Inherited base class |
| 0x24 | 4 | ptr | DynVector vtable | `PTR_FUN_007E4BA4` |
| 0x28 | 4 | ptr | nodes_data | Pointer to array of MCNode* |
| 0x2C | 4 | int | nodes_capacity | DynamicVector allocated capacity |
| 0x30 | 1 | bool | nodes_is_valid | DynamicVector valid flag |
| 0x31 | 1 | bool | (unknown flag) | Initialized to 0 |
| 0x34 | 4 | int | nodes_count | Current number of controlled units |
| 0x38 | 4 | int | nodes_grow_step | Growth increment (default: 10) |
| 0x3C | 4 | int | max_control | Max simultaneous victims (from weapon Damage) |
| 0x40 | 1 | bool | infinite_mind_control | From weapon's InfiniteMindControl flag |
| 0x41 | 1 | bool | overload_spark_active | Whether overload sparks are currently playing |
| 0x44 | 4 | int | overload_spark_delay | Cooldown counter for spark visual effects |
| 0x48 | 4 | ptr | owner | Pointer to owning TechnoClass |
| 0x4C | 4 | int | overload_tick_timer | Countdown for next overload damage tick |

### MCNode Sub-struct (Mind Control Link Node)

**Size: 0x14 (20 bytes)** — allocated via `operator_new(0x14)`.

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| 0x00 | 4 | ptr | victim | Pointer to controlled TechnoClass |
| 0x04 | 4 | ptr | original_owner | HouseClass victim belonged to before capture |
| 0x08 | 4 | int | capture_frame | Frame when captured (-1 = permanent link line) |
| 0x0C | 4 | int | (reserved) | Set from an uninitialized register |
| 0x10 | 4 | int | link_visible_frames | Duration to show MC link (from MindControlAttackLineFrames) |

---

## 5. Key Functions

### 5.1 CaptureManagerClass::Constructor @ 0x004717D0

```
CaptureManagerClass* __thiscall Constructor(
    CaptureManagerClass* this,
    TechnoClass* owner,          // stored at offset 0x48
    int maxControl,              // stored at offset 0x3C (from weapon Damage)
    bool infiniteMC              // stored at offset 0x40
)
```

Called from `TechnoClass__Init_Managers` (0x006F3F40) when the unit's primary weapon's
warhead has `MindControl=yes`.

### 5.2 CaptureManagerClass::CanCapture @ 0x00471C90

**Returns true if ALL conditions are met:**

1. Target is not null
2. Target's owner differs from controller's owner (can't MC own units)
3. Target type does NOT have `ImmuneToPsionics` (TechnoTypeClass + 0xD35)
4. Target is not currently being warped (TechnoClass + 0x2E4 != 0, and GetWhatAmI == 1 for infantry check)
5. Target is not already mind-controlled (`TechnoClass__IsMindControlled()` returns false)
6. Target is not under Iron Curtain / Force Shield (`TechnoClass + 0x2CC` timer == 0)
7. Target is not immune via vtable call 0x160 (IsBeingDrained or similar)
8. **Capacity check** — one of:
   - `infinite_mind_control` is set, OR
   - `nodes_count < max_control` (room available), OR
   - `max_control == 1` (override mode — will free existing)
9. Target mission is not 0x13 (Selling) or 0x12 (certain blocked state)

**Key interaction: cannot MC an already MC'd unit.** Check 5 ensures this.
To re-MC a unit, the first controller must die or be destroyed first.

### 5.3 CaptureManagerClass::CaptureUnit @ 0x00471D40

**Flow:**

1. Validate target (null check + AbstractFlags)
2. Call `CanCapture(target)` — return false if denied
3. **Override mode** (max_control == 1): iterate all existing nodes and `FreeUnit()` each
4. Get target's current owner via `GetHouse()` (vtable 0x3C)
5. Transfer ownership: `target->SetOwner(controller_owner)` via vtable 0x3D4
6. Allocate MCNode (0x14 bytes):
   - `node->victim = target`
   - `node->original_owner = target_previous_owner`
   - `node->capture_frame = g_CurrentFrameCounter`
   - `node->link_visible_frames = RulesClass->MindControlAttackLineFrames`
7. Add node to DynamicVector
8. Set `target->MindControlledBy = controller` (victim offset 0x2C0 = controller ptr from 0x48)
9. Skip scatter for certain building-type missions (0x10=Unload, 0x13, 0x12)
10. Otherwise call `target->Scatter()` (vtable 0x3D0)
11. Call `DecideUnitFate(target)` for AI disposition
12. Create MC ring anim from `RulesClass->ControlledAnimationType` (offset 0x320)
13. Attach anim to victim, store pointer at `victim + 0x2C8`
14. If victim is building, set anim Z-offset to -1024 leptons (0xFFFFFC00)
15. Return true

### 5.4 CaptureManagerClass::FreeUnit @ 0x00471FF0

**Flow:**

1. Iterate node array looking for node with matching victim
2. Remove MC ring anim from victim (call `Remove()` on anim, clear `victim + 0x2C8`)
3. Play "freed from MC" sound:
   - Check `TechnoTypeClass + 0x5B0` (per-type MindClearedSound)
   - If -1, use `RulesClass + 0x264` (global MindClearedSound)
4. Restore victim's owner: `victim->SetOwner(node->original_owner, redraw=true)` via vtable 0x3D4
5. Call `DecideUnitFate(victim)` for AI disposition
6. Clear `victim->MindControlledBy` (offset 0x2C0 = 0)
7. Free MCNode memory
8. Remove node from DynamicVector (shift remaining entries down)

### 5.5 CaptureManagerClass::FreeAll @ 0x00472140

Simple reverse-iteration loop calling `FreeUnit()` on each victim.

**Called from:**
- `TechnoClass::ReceiveDamage` @ 0x00702112 — controller destroyed
- `BuildingClass::ReceiveDamage` @ 0x004424F9 — controller building destroyed
- `BuildingClass::EnterTransport` @ 0x0070FDBD — controller enters transport
- `TemporalClass::InitiateWarp` @ 0x0071AF48 — controller Chronoshifted/warped
- `BuildingClass::UpdateGapAndSpecialEffects` @ 0x00454B47
- `FUN_004DE5D0` @ 0x004DE5DD — unit removal

### 5.6 CaptureManagerClass::Update @ 0x00471A50

**Called from:** `TechnoClass::AI_Update`, every tick if CaptureManager exists.

**Only active when `infinite_mind_control` (offset 0x40) is true.** This is the
Mastermind overload damage system.

**Algorithm:**
1. Decrement `overload_spark_delay` (offset 0x44)
2. Decrement `overload_tick_timer` (offset 0x4C)
3. When `overload_tick_timer` reaches 0:
   a. Find the overload tier: iterate `OverloadCount[]` to find which threshold
      the current `nodes_count` falls into
   b. Read damage from `OverloadDamage[tier]`
   c. Read frame interval from `OverloadFrames[tier]`, store as new `overload_tick_timer`
   d. If damage > 0:
      - Set `overload_spark_delay = 10`
      - Apply damage to the **CONTROLLER** (not victims!) via `ReceiveDamage()`
        using `RulesClass + 0xFA8` warhead
      - Play `MasterMindOverloadDeathSound` if first spark occurrence
   e. Create 5 spark particle effects at controller location
   f. Optionally apply heading wobble to controller (+-0.015 or +-0.03 radians)

**With default INI values:**
- 0-2 victims: 0 damage (safe)
- 3-5 victims: 0 damage, 30 frame interval
- 6-9 victims: 50 damage, 60 frame interval
- 10-49 victims: 100 damage, 60 frame interval
- 50+ victims: 500 damage, 60 frame interval

### 5.7 CaptureManagerClass::DrawLinks @ 0x00472160

Renders colored lines between controller and each victim.

**Per-node logic:**
1. Check if victim is alive (offset 0x83)
2. Calculate remaining link visibility: `link_visible_frames - (current_frame - capture_frame)`
3. If `capture_frame == -1`: link is permanent (always visible while duration > 0)
4. Get victim 3D position + `TechnoTypeClass->LeptonMindControlOffset` Z-offset
5. Get controller position with per-link index offset (`-1 - index % 5`)
6. Read MC line color from `controller->House + 0x56F9`
7. Draw line via `FUN_00704E40`

### 5.8 CaptureManagerClass::DecideUnitFate @ 0x004723B0

AI decision for what to do with a newly captured or released unit.

**Decision categories** (based on house power ratio and victim health):
- Category 0: Low on money → probability table from `RulesClass + 0xEA0`
- Category 1: Low power → from `RulesClass + 0xE84`
- Category 2: Healthy → from `RulesClass + 0xE68`
- Category 3: Very healthy → from `RulesClass + 0xE4C`

**Outcomes** (random roll 1-100):
1. Join capturing unit's team
2. Scatter
3. Hunt
5. Do nothing
Default: `SetMission(Guard)` (mission 0xF)

Debug log: `"AICapture: I think, %s, so I roll %d => %s"`.

### 5.9 CaptureManagerClass::GetOriginalOwner @ 0x004722F0

Searches node array for a victim and returns its stored `original_owner` HouseClass pointer.

### 5.10 CaptureManagerClass::SetOriginalOwner @ 0x00472330

Updates the `original_owner` for a specific victim. Used when house assignments change.

### 5.11 TechnoClass__IsMindControlled @ 0x007105E0

```c
bool IsMindControlled(TechnoClass* this) {
    return (*(int*)(this + 0x2C0) != 0) || (*(char*)(this + 0x2C4) != 0);
}
```

Returns true if the unit is controlled by EITHER mechanism (CaptureManager or Psychic Dominator).

### 5.12 TechnoClass__FreeAllMindControlCaptures @ 0x00710460

```c
void FreeAllMindControlCaptures(TechnoClass* this) {
    if (this->CaptureManager != NULL) {  // offset 0x2BC
        CaptureManagerClass__FreeAll(this->CaptureManager);
    }
}
```

---

## 6. Mind Control Capture Flow (End-to-End)

### Step 1: Weapon fires

The MC unit fires its primary weapon (e.g., `MindControl` weapon with `Warhead=Controller`).
A BulletClass is created with the target.

### Step 2: Bullet hits, Detonate called

`BulletClass::AI` calls `WarheadTypeClass::Detonate` (0x004690B0).

### Step 3: MindControl warhead check

At 0x00469211, checks `warheadType->MindControl` (offset 0x155). If true, enters MC path.

### Step 4: Validate controller

Gets the firer's CaptureManager from `firer->CaptureManager` (offset 0x2BC).
If null (firer has no MC capability), skips to normal damage.

### Step 5: Pre-capture effects

If firer is controllable by a human player, plays EVA events (cell action events 6 and 0x2C).
This triggers the "Unit mind-controlled" voiceline.

### Step 6: CaptureUnit called

`CaptureManagerClass::CaptureUnit(target)` at 0x004692D0.

### Step 7: Ownership transfer

Inside CaptureUnit, `target->SetOwner(controller_owner)` transfers the unit. The original
owner is saved in the MCNode.

### Step 8: Sound effect

If capture succeeds and `RulesClass->YuriMindControlSound` (offset 0x214) is valid,
play the MC sound at the victim's location (if local player can hear).

### Step 9: Per-tick update

`TechnoClass::AI_Update` calls `CaptureManagerClass::Update` every frame.
For Mastermind (InfiniteMC), this handles overload damage.

### Step 10: Visual rendering

`TacticalClass::Draw` calls `DrawLinks()` to render colored MC link lines.
`ShouldDrawLinks()` is checked first (controller/victim selected or link timer active).

---

## 7. Controller Death / Release Mechanics

### When the controller dies

`TechnoClass::ReceiveDamage` at 0x00702112 calls `CaptureManagerClass::FreeAll()`.

**For each victim:**
1. MC ring anim removed
2. "Mind cleared" sound played
3. Original owner restored via `SetOwner()`
4. `DecideUnitFate()` called for AI disposition
5. `MindControlledBy` pointer cleared
6. MCNode freed

**Result:** All victims return to their original owners.

### When the controller enters a transport

`FreeAll()` is called — entering a transport releases all MC'd units.

### When the controller is Chronoshifted

`TemporalClass::InitiateWarp` calls `FreeAll()` — temporal warping releases all MC'd units.

### When a specific victim is removed

Various `FreeUnit()` callers handle individual victim cleanup:
- `InfantryClass::Mission_Enter` @ 0x0051A2DA, 0x0051A438
- `UnitClass::Mission_Enter` @ 0x0073A2CD, 0x0073A72B

---

## 8. InfiniteMindControl and Mastermind Overload

**INI:** `InfiniteMindControl=yes` on WeaponTypeClass
**Binary:** WeaponTypeClass + 0x140 / CaptureManager + 0x40

When set:
- `CanCapture()` always passes capacity check (unlimited victims)
- DynamicVector grows dynamically (step 10) to accommodate
- `Update()` runs the overload damage system against the CONTROLLER

When NOT set:
- `max_control` (from weapon Damage) limits simultaneous victims
- If `max_control == 1`: **override mode** — capturing new victim frees the previous
- If `max_control > 1` and at capacity: `CanCapture()` returns false, MC fails

**Overload tiers** (default INI):

| Tier | Count Range | Damage/Tick | Tick Interval |
|------|------------|-------------|---------------|
| 0 | 0-2 | 0 | 30 frames |
| 1 | 3-5 | 0 | 30 frames |
| 2 | 6-9 | 50 | 60 frames |
| 3 | 10-49 | 100 | 60 frames |
| 4 | 50+ | 500 | 60 frames |

The damage is applied to the controller itself via `ReceiveDamage()`.

---

## 9. Psychic Dominator Permanent Mind Control

**Function:** `PsychicDominator__MindControlArea` @ 0x0053B080

This is a completely separate code path from CaptureManager. The Psychic Dominator
superweapon iterates all units in an area and:

1. Skips buildings (`GetWhatAmI() == 6`)
2. Skips `ImmuneToPsionics` units (TechnoTypeClass + 0xD35)
3. Skips units immune via vtable 0x160
4. Skips units with `TechnoTypeClass + 0xD6A` set (ImmuneToPsychicDominator)
5. Skips units failing vtable 0x54 check (IsAlive/valid)
6. If target has an active CaptureManager link (`target + 0x2C0 != 0`):
   - Calls `CaptureManagerClass::FreeUnit()` to release from existing controller first
7. Transfers ownership: `target->SetOwner(dominator_house)` via vtable 0x3D4
8. Sets `target->PermanentlyMindControlled = 1` (offset 0x2C4)
9. Creates `PermaControlledAnimationType` anim (RulesClass + 0x324)
10. Uses `TechnoTypeClass->MindControlRingOffset` (offset 0x60C) for anim Z-offset
11. Stores anim at `target + 0x2C8`

**Key differences from CaptureManager MC:**
- No controller pointer stored (0x2C0 stays 0 or is cleared)
- No MCNode tracking — no way to "free" the unit
- Uses `PermaControlledAnimationType` instead of `ControlledAnimationType`
- The 0x2C4 flag is permanent and irreversible
- Does NOT use any CaptureManagerClass

---

## 10. Interaction Rules

### Mind-controlling an already MC'd unit
**Blocked.** `CanCapture()` check 5 calls `TechnoClass__IsMindControlled()` which returns
true if either 0x2C0 (CaptureManager link) or 0x2C4 (permanent flag) is set. The MC
attempt silently fails. The first controller must die first.

### Mind-controlling a mind controller
**Allowed** (if the target type doesn't have `ImmuneToPsionics=yes`). The MC'd controller
retains its own CaptureManager and all its victims. It simply changes ownership. The
victims of the now-MC'd controller continue to belong to the controller's *new* owner
(since they were already transferred).

### Iron Curtain interaction
**Blocked.** `CanCapture()` check 6: if `target + 0x2CC` (IronCurtain timer) is nonzero,
MC is denied. Iron Curtained units cannot be mind-controlled.

### Temporal weapon interaction
**Blocked.** `CanCapture()` check 4: if the target has an active temporal warp
(`target + 0x2E4 != 0`) and is infantry, MC fails. Additionally, if the CONTROLLER
is being temporally warped, `TemporalClass::InitiateWarp` calls `FreeAll()` to release
all victims.

### Force Shield / ForceShield
Same as Iron Curtain — blocked by the same timer check at offset 0x2CC.

### Units in certain missions
Targets in mission 0x12 or 0x13 (selling-related states) cannot be MC'd.

### Re-mind-controlling a freed unit
After a controller dies and `FreeAll()` restores a victim, the victim's `MindControlledBy`
is cleared to 0 and `PermanentlyMindControlled` is not set, so the unit CAN be
mind-controlled again by another MC unit.

### Psychic Dominator on an already MC'd unit
The Psychic Dominator function explicitly checks `target + 0x2C0` (MindControlledBy) and
calls `CaptureManagerClass::FreeUnit()` to cleanly release the unit from its existing
controller before permanently capturing it.

---

## 11. Function Address Summary

| Address | Name | Description |
|---------|------|-------------|
| 0x004717D0 | CaptureManagerClass::Constructor (full) | `(owner, maxControl, infiniteMC)` |
| 0x00471890 | CaptureManagerClass::Constructor (default) | For save/load deserialization |
| 0x00471A50 | CaptureManagerClass::Update | Per-tick overload damage & sparks |
| 0x00471C90 | CaptureManagerClass::CanCapture | Validates if target can be MC'd |
| 0x00471D40 | CaptureManagerClass::CaptureUnit | Main capture execution |
| 0x00471FF0 | CaptureManagerClass::FreeUnit | Release a specific victim |
| 0x00472140 | CaptureManagerClass::FreeAll | Release all victims |
| 0x00472160 | CaptureManagerClass::DrawLinks | Render MC link lines |
| 0x004722F0 | CaptureManagerClass::GetOriginalOwner | Lookup original HouseClass |
| 0x00472330 | CaptureManagerClass::SetOriginalOwner | Update original owner |
| 0x004723B0 | CaptureManagerClass::DecideUnitFate | AI decision for captured/freed unit |
| 0x00472640 | CaptureManagerClass::ShouldDrawLinks | Check if links should render |
| 0x00472720 | CaptureManagerClass::Save | Serialization |
| 0x004728E0 | CaptureManagerClass::Load | Deserialization |
| 0x00472960 | CaptureManagerClass::Detach | COM detach |
| 0x004729A0 | CaptureManagerClass::GetSize | Returns 0x50 |
| 0x004729B0 | CaptureManagerClass::GetClassID | Returns 0x42 |
| 0x004729C0 | CaptureManagerClass::Destructor | Cleanup |
| 0x004690B0 | WarheadTypeClass::Detonate | MC dispatch at 0x00469211 |
| 0x006F3F40 | TechnoClass::Init_Managers | Creates CaptureManager |
| 0x007105E0 | TechnoClass::IsMindControlled | Checks both MC paths |
| 0x00710460 | TechnoClass::FreeAllMindControlCaptures | Wrapper for FreeAll |
| 0x00710550 | TechnoClass::FreeMindControlledChain | Free chain of MC'd units |
| 0x0053B080 | PsychicDominator::MindControlArea | Permanent MC superweapon |

### Related Addresses

| Address | Context |
|---------|---------|
| 0x007E4B40 | CaptureManagerClass primary vtable |
| 0x007E4BA4 | DynamicVector vtable for node storage |
| 0x0089E0F0 | Global DynamicVector of all CaptureManagerClass instances |
| 0x00424B50 | Anim attach function (links MC ring anim to victim) |
| 0x00704E40 | 3D line drawing function (renders MC link lines) |

---

## 12. TS Legacy Warnings

- The `DecideUnitFate` function contains elaborate AI probability tables that may have
  originated from Tiberian Sun. The specific outcome categories and their probability
  distributions should be verified against actual YR gameplay if implementing AI decisions.
  The function IS called in YR but the probability tables might be vestigial.

- The `MindControlDecision` INI key found in TeamTypeClass/AI task force data
  (0x008430D8) is used in AI team composition decisions. This appears to be live YR
  code (not TS legacy) but is part of the AI system, not the core MC mechanic.

- The overload system with its tiered damage tables is definitely live in YR (Mastermind
  unit uses it), not TS legacy.

---

## 13. Confidence Summary

| Finding | Confidence | Basis |
|---------|-----------|-------|
| WarheadTypeClass::MindControl at 0x155 | Verified | String xref + disassembly |
| WeaponTypeClass::InfiniteMindControl at 0x140 | Verified | String xref + disassembly |
| WeaponTypeClass::Damage as max_control | Verified | Constructor call trace |
| TechnoClass field offsets (0x2BC-0x2C8) | Verified | Multiple function decompilations |
| 0x2C4 = PermanentlyMindControlled (not IsMindControlled) | Verified | PsychicDominator function decompilation |
| CaptureManagerClass struct layout | Verified | Constructor + multiple member functions |
| MCNode struct layout | Verified | CaptureUnit allocation + field usage |
| Mastermind overload system | Verified | Update function + RulesClass INI reads |
| CanCapture blocking conditions | Verified | Full decompilation of CanCapture |
| RulesClass offsets (sounds, anims, tables) | Verified | ReadAudioVisual/CombatDamage string xrefs |
| TechnoTypeClass::ImmuneToPsionics at 0xD35 | Verified | Disassembly at 0x00714FA7 |
| TechnoTypeClass::MindClearedSound at 0x5B0 | Verified | ReadINI string xref |
