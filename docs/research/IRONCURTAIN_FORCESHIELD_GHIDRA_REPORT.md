# IronCurtain & ForceShield Invulnerability System -- Ghidra Report

Confidence: HIGH (verified from binary, all offsets confirmed via assembly + INI cross-reference)

## Overview

IronCurtain (SuperWeapon Type=1) and ForceShield (SuperWeapon Type=10) share the same
underlying invulnerability mechanism. Both apply a timed invulnerability effect to units
via the virtual function at vtable+0x154 (`TechnoClass::IronCurtain`). ForceShield adds
a power blackout on the owning house and only targets allied buildings within a radius.

---

## 1. INI Fields and Rules Offsets

### From [General] (RulesClass::ReadGeneral @ 0x0066d530)

| INI Key                        | Rules Offset | Type | Default (rulesmd.ini) | Notes |
|-------------------------------|-------------|------|----------------------|-------|
| ForceShieldRadius             | +0x17B8     | int  | 4                    | In cells |
| ForceShieldDuration           | +0x17BC     | int  | 500                  | In frames, how long buildings stay invulnerable |
| ForceShieldBlackoutDuration   | +0x17C0     | int  | 1000                 | In frames, power blackout duration (longer than shield!) |
| ForceShieldPlayFadeSoundTime  | +0x17C4     | int  | 75                   | Frames before shield expires when fade sound plays |
| IronCurtainInvokeAnim         | +0x348      | AnimType* | -               | Animation played at IronCurtain target location |
| ForceShieldInvokeAnim         | +0x34C      | AnimType* | -               | Animation played at ForceShield target location |
| MutateExplosion               | +0x17C8     | bool | -                    | (Adjacent field, not ForceShield-related) |

### From [CombatDamage] (FUN_0066bbb0 / RulesClass::ReadCombatDamage)

| INI Key              | Rules Offset | Type | Default (rulesmd.ini) | Notes |
|---------------------|-------------|------|----------------------|-------|
| IronCurtainDuration | +0xFE8      | int  | 750                  | In frames (50 seconds at 15fps) |

### From [AudioVisual] (RulesClass::ReadAudioVisual @ 0x006691e0)

| INI Key           | Rules Offset | Type | Default | Notes |
|------------------|-------------|------|---------|-------|
| IronCurtainColor | +0x18A8     | int  | -       | Palette index for IronCurtain tint (ReadInt) |
| ForceShieldColor | +0x18B0     | int  | -       | Palette index for ForceShield tint (ReadInt) |
| BerserkColor     | +0x18AC     | int  | -       | (Adjacent, for reference) |

The color values are palette indices passed into the sprite drawing pipeline. In
TechnoClass::Draw and TechnoClass_DrawSHP, when `IsIronCurtainActive()` returns true,
the rendering applies a color tint. The `is_force_shield` flag at TechnoClass+0x1C4
determines whether IronCurtainColor or ForceShieldColor is used.

---

## 2. TechnoClass Invulnerability Fields

All offsets relative to TechnoClass `this` pointer:

| Offset | Size | Field Name            | Description |
|--------|------|-----------------------|-------------|
| +0x18C | 4    | IronCurtainStartFrame | Frame when IronCurtain was applied. -1 = never set. |
| +0x190 | 4    | (padding/unused)      | Part of timer struct, not meaningfully used |
| +0x194 | 4    | IronCurtainDuration   | Duration in frames |
| +0x1A4 | 4    | (cleared to 0)        | Reset when IronCurtain is applied |
| +0x1C4 | 4    | IsForceShield         | 1 = ForceShield, 0 = IronCurtain. Controls color tint. |

---

## 3. TechnoClass::IronCurtain (vtable+0x154)

### Base: TechnoClass::IronCurtain @ 0x0070e2b0

```
void __thiscall TechnoClass::IronCurtain(int duration, int source_house, int is_force_shield)
```

Assembly-verified behavior:
```
this+0x18C = g_CurrentFrameCounter    // Record start frame
this+0x190 = (stack local)            // Timer struct padding
this+0x1A4 = 0                        // Clear (unknown field) -- written BEFORE 0x194
this+0x194 = duration                 // Store duration in frames
if (is_force_shield)
    this+0x1C4 = 1                    // Mark as ForceShield
else
    this+0x1C4 = 0                    // Mark as IronCurtain
```
<!-- corrected 2026-05-29: doc had 0x194 written before 0x1A4; binary shows 0x1A4=0 written first, then 0x194=duration; ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT; via decompile_function 0x0070E2B0 -->

### Override: BuildingClass::IronCurtain @ 0x00457c90

Extra logic before calling the base:
```
if (this+0x6DF != 0) {               // If building has pre-existing IC state flag
    this+0x6DF = 0                    // Clear flag
    this+0x528 = CurrentFrame         // Reset building-specific timer
    this+0x52C = (stack local)        // Timer padding
    this+0x530 = 0                    // Zero duration
    this+0x540 = 0                    // Clear counter
}
TechnoClass::IronCurtain(duration, source_house, is_force_shield)
```

The building override resets a building-specific previous-IC-state before applying the
new one. Field +0x6DF appears to be a "had active IronCurtain" flag that needs cleanup.

---

## 4. TechnoClass::IsIronCurtainActive (vtable+0x160)

**Address:** 0x0041bf40 (shared by all TechnoClass derivatives via vtable)

```c
bool TechnoClass::IsIronCurtainActive() {
    int duration = this->IronCurtainDuration;  // +0x194
    if (this->IronCurtainStartFrame != -1) {   // +0x18C
        int elapsed = CurrentFrame - this->IronCurtainStartFrame;
        if (elapsed < duration)
            duration = duration - elapsed;      // remaining frames
        else
            duration = 0;                       // expired
    }
    return duration > 0;
}
```

This is the core invulnerability check. Called by:
- **TechnoClass::ReceiveDamage** (0x00701900) -- if true, damage is set to 0 and
  a spark/flash effect is played. The `is_force_shield` flag at +0x1C4 determines
  which flash type (IronCurtain=1, ForceShield=6) is passed to the effect function
  at 0x0048a620.
- **TechnoClass::Draw / TechnoClass_DrawSHP** -- applies color tint when active
- Various AI and state-check functions

---

## 5. Invulnerability Expiry

The IronCurtain effect is **purely timer-based**. There is no explicit "remove" call.
When `CurrentFrame - StartFrame >= Duration`, `IsIronCurtainActive()` returns false
and the unit becomes vulnerable again.

**There is no early-removal mechanism** in the base game code. The effect persists
for its full duration. (Temporal/Chrono weapons do NOT remove IronCurtain -- they
operate independently via different fields at +0x280 range.)

---

## 6. SuperClass::Launch -- Case 1 (IronCurtain)

**Address:** Case 1 starts around 0x006ccece in SuperClass::Launch (0x006cc390)

### Sequence:
1. Get target cell, compute world coordinates
2. Create IronCurtainInvokeAnim (`AnimClass::Constructor(Rules+0x348, coords)`) with
   flags: loop=1, owner=0x600, z_adjust=+5
3. Play EVA announcement if not in observer mode
4. Create radar event
5. Iterate all cells in the standard 3x3 area (table at 0x00B0C038):
   - For each cell, get the object linked list (CellClass+0xE4 or +0xE8 for bridges)
   - For each object in cell:
     - Skip if not a valid techno (`(flags >> 2) & 1 == 0`) or if `field_0x27C != 0`
     - Skip if object is a building with `NoForceShield` or similar immunity flag
     - Call `object->IronCurtain(Rules->IronCurtainDuration, super->Owner, 0)`
       - The 3rd argument is 0 = NOT ForceShield

### IronCurtain call parameters (verified from assembly at 0x006cd008-0x006cd01f):
```
PUSH 0                              // is_force_shield = false
PUSH [EBX + 0x2c]                  // source_house = super->Owner
PUSH [g_RulesClass + 0xFE8]        // duration = IronCurtainDuration
MOV ECX, ESI                        // this = target unit
CALL [EDX + 0x154]                  // vtable IronCurtain
```

---

## 7. SuperClass::Launch -- Case 10 (ForceShield)

**Address:** Case 10 starts around 0x006cd0ce in SuperClass::Launch

### Sequence:
1. Get target cell, compute world coordinates
2. Create ForceShieldInvokeAnim (`AnimClass::Constructor(Rules+0x34C, coords)`) with
   flags: loop=1, owner=0x600, z_adjust=+5
3. **Set ForceShield fade sound timer:**
   ```
   SuperClass+0x50 = ForceShieldDuration - ForceShieldPlayFadeSoundTime
                   = Rules+0x17BC - Rules+0x17C4
                   = 500 - 75 = 425 frames (default)
   ```
4. Store target coordinates in SuperClass: +0x54 (X), +0x58 (Y), +0x5C (Z)
5. If SuperWeaponTypeClass+0xC4 (SpecialSound) != -1, play sound at launch
6. **Call HouseClass::SpyPowerSabotage** on the owning house:
   ```
   MOV ECX, [EBX + 0x2c]              // this = super->Owner (HouseClass*)
   MOV EAX, [Rules + 0x17C0]          // ForceShieldBlackoutDuration
   PUSH EAX
   CALL HouseClass::SpyPowerSabotage
   ```
7. Iterate all buildings (BuildingClass array), for each allied building within
   `ForceShieldRadius * 256` leptons distance (3D) from the target:
   ```
   PUSH 1                              // is_force_shield = true
   PUSH [EBX + 0x2c]                  // source_house = super->Owner
   PUSH [Rules + 0x17BC]              // duration = ForceShieldDuration
   MOV ECX, ESI                        // this = building
   CALL [EAX + 0x154]                 // building->IronCurtain(...)
   ```

### Key differences from IronCurtain (case 1):
- Only affects **allied buildings** within radius (not all units in cells)
- Uses `ForceShieldDuration` (Rules+0x17BC) not `IronCurtainDuration` (Rules+0xFE8)
- `is_force_shield` parameter = 1 (affects color tint and damage flash type)
- Triggers power blackout via SpyPowerSabotage
- Sets the SuperClass fade sound timer

---

## 8. HouseClass::SpyPowerSabotage -- Power Blackout

**Address:** 0x0050bc90

```
void __thiscall HouseClass::SpyPowerSabotage(int blackout_duration)
```

### What it does (assembly-verified):
```
this+0x5778 = 1 (RecheckPower = true)
this+0x2A4  = g_CurrentFrameCounter    // Blackout start frame
this+0x2A8  = (stack local / padding)  // Timer struct padding
this+0x2AC  = blackout_duration        // Duration parameter
```

### Timer struct at HouseClass+0x2A4:
| Offset | Field | Description |
|--------|-------|-------------|
| +0x2A4 | StartFrame | Frame when blackout started. -1 = inactive. |
| +0x2A8 | (padding) | Timer struct padding, not meaningfully used |
| +0x2AC | Duration | Blackout duration in frames |

### Callers:
1. **SuperClass::Launch case 10** (ForceShield): passes `Rules+0x17C0` (ForceShieldBlackoutDuration)
2. **BuildingClass::OnSpyInfiltrate** (entry: 0x004571e0): called when a spy enters a power plant
   <!-- corrected 2026-05-29: was 0x004572e8 (an address within the function body, not the entry); actual entry 0x004571e0 via get_function_by_address 0x004572e8 returning entry 0x004571e0; ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

### How the blackout works:

In **HouseClass::AI_AssessPower** (0x00508C30), after iterating all buildings and
summing PowerOutput:

```c
// Check blackout timer
int duration = this->field_0x2AC;
if (this->field_0x2A4 != -1) {
    int elapsed = CurrentFrame - this->field_0x2A4;
    if (elapsed < duration) {
        duration = duration - elapsed;  // remaining > 0, still in blackout
    } else {
        duration = 0;  // blackout expired
    }
}

// If blackout is active (remaining > 0), zero power output.
// Edge case: if timer never started (field_0x2A4==-1) but duration>0, also zeros power.
// Special case: if timer has expired, power is also zeroed if a deploying-power-building
//   exists (local_d flag). This is a niche scenario unrelated to normal ForceShield usage.
if (remaining > 0) {
    this->PowerOutput = 0;   // ZEROES all power output during blackout!
}
// corrected 2026-05-29: doc had "duration > 0 || !has_power_buildings" — WRONG;
// binary: zeroes power when remaining>0 (active timer) OR when timer-not-started but
// duration!=0; expired-timer path gates on local_d (deploying power building), not
// absence of power buildings; ROOT_CAUSE: INFERENCE_HARDENED; via decompile_function 0x00508C30
```

**During blackout, PowerOutput is forced to 0 every tick.** PowerDrain remains
unchanged, so the house is in a full low-power state. This affects:
- All production speeds (factories slow down)
- Base defenses go offline
- Radar goes dark
- Any power-dependent systems

In **HouseClass::Update** (0x004F8440), the blackout timer is also checked:
```c
// When blackout timer expires (remaining reaches 0):
if (remaining == 0) {
    // Timer naturally expires, no explicit reset needed
    // RecheckPower was set = true at start, so next assess recalculates properly
}
```

When the timer expires, `AI_AssessPower` stops zeroing PowerOutput and the
house's actual power output from buildings is restored on the next tick.

### Blackout duration (default):
- ForceShieldBlackoutDuration = 1000 frames = ~66.7 seconds at 15fps
- ForceShieldDuration = 500 frames = ~33.3 seconds at 15fps
- The blackout lasts TWICE as long as the shield itself!

---

## 9. ForceShield Countdown Timer and Fade Sound

### SuperClass+0x50: Fade Sound Countdown

Set at launch:
```
SuperClass+0x50 = ForceShieldDuration - ForceShieldPlayFadeSoundTime
                = 500 - 75 = 425 frames (default)
```

In **SuperClass::AI_Ready** (0x006CBCA0), each tick:
```c
if (this->field_0x50 > 0)
    this->field_0x50--;

if (this->field_0x50 == 0) {
    this->field_0x50 = -1;              // Mark as played (won't trigger again)
    VocClass::PlayAt(SpecialSound, 0);  // Play ForceShieldFading sound
}
```

So the ForceShieldFading sound plays exactly `ForceShieldPlayFadeSoundTime` frames
(75 frames = 5 seconds) before the shield effect expires, warning the player.

Note: There are TWO sound events for ForceShield:
1. **At launch:** SuperWeaponTypeClass+0xC4 (SpecialSound) is played immediately
   if != -1. This is the "ForceShield activated" sound.
2. **Near expiry:** The countdown timer at SuperClass+0x50 triggers the same
   SpecialSound (ForceShieldFading) when it reaches 0, which is 75 frames before
   the shield expires.

### SuperClass+0x54/0x58/0x5C: Effect Coordinates

Stored at launch for use by the AI_Ready tick logic (animation tracking, etc.).

---

## 10. Damage Rejection When Invulnerable

In **TechnoClass::ReceiveDamage** (0x00701900):

```c
bool is_invulnerable = this->IsIronCurtainActive();  // vtable+0x160
if (is_invulnerable && !force_damage && !is_healing) {
    // Determine flash type based on ForceShield flag
    int flash_type;
    if (this->IsForceShield == 1)   // +0x1C4
        flash_type = 6;             // ForceShield flash
    else
        flash_type = 1;             // IronCurtain flash

    // Play invulnerability spark/flash at unit location
    FUN_0048a620(this->X, this->Y, this->Z, 1, flash_type);

    *damage = 0;    // Nullify all damage
    return 0;       // No damage result
}
```

---

## 11. Complete Invulnerability Lifecycle

### IronCurtain (Type=1):
1. Player fires IronCurtain super at target cell
2. IronCurtainInvokeAnim (Rules+0x348) created at target
3. All units/buildings in 3x3 cell area get `IronCurtain(IronCurtainDuration, house, 0)`
4. Each target: StartFrame=now, Duration=750, IsForceShield=0
5. For 750 frames, `IsIronCurtainActive()` returns true
6. All damage blocked, IronCurtainColor tint applied to sprites
7. At frame 750, timer expires, unit becomes vulnerable again

### ForceShield (Type=10):
1. Player fires ForceShield super at target cell
2. ForceShieldInvokeAnim (Rules+0x34C) created at target
3. **Power blackout:** `SpyPowerSabotage(ForceShieldBlackoutDuration=1000)` on owner
4. **Fade sound timer:** `SuperClass+0x50 = 500 - 75 = 425`
5. All allied buildings within `ForceShieldRadius * 256` leptons get
   `IronCurtain(ForceShieldDuration=500, house, 1)`
6. Each building: StartFrame=now, Duration=500, IsForceShield=1
7. For 500 frames, buildings are invulnerable with ForceShieldColor tint
8. At frame 425, ForceShieldFading sound plays (75 frames before expiry)
9. At frame 500, shield effect expires on buildings
10. At frame 1000, power blackout expires, power output restored
11. Net effect: buildings protected for 500f, but power down for 1000f

---

## 12. Function Address Summary

| Function | Address | Notes |
|----------|---------|-------|
| SuperClass::Launch | 0x006CC390 | Main switch for all super types |
| TechnoClass::IronCurtain | 0x0070E2B0 | Base invulnerability application (vtable+0x154) |
| BuildingClass::IronCurtain | 0x00457C90 | Override with building-specific reset (vtable+0x154) |
| TechnoClass::IsIronCurtainActive | 0x0041BF40 | Timer check (vtable+0x160) |
| HouseClass::SpyPowerSabotage | 0x0050BC90 | Starts power blackout timer |
| HouseClass::AI_AssessPower | 0x00508C30 | Zeros PowerOutput during blackout |
| HouseClass::Update | 0x004F8440 | Checks blackout timer expiry |
| SuperClass::AI_Ready | 0x006CBCA0 | Decrements fade sound timer, plays sound |
| TechnoClass::ReceiveDamage | 0x00701900 | Damage rejection + flash effect |
| RulesClass::ReadGeneral | 0x0066D530 | ForceShield INI fields, invoke anims |
| RulesClass::ReadCombatDamage | 0x0066BBB0 | IronCurtainDuration INI field |
| RulesClass::ReadAudioVisual | 0x006691E0 | IronCurtainColor, ForceShieldColor |
| InfantryClass::IronCurtain | 0x00522600 | Override: kills infantry instead of protecting |
| TemporalClass::CanWarpTarget | 0x0071AE50 | Checks IC active, blocks temporal if true |
| CaptureManagerClass::CanCapture | 0x00471C90 | Checks IC active, blocks mind control if true |
| TechnoClass::CanCrushCheck | 0x005F6CD0 | Checks IC active, blocks crushing if true |
| TechnoClass::IsWarpingOut | 0x0070C5B0 | vtable+0x1D4, separate invuln check in ReceiveDamage |

---

## 13. Iron Curtain Kills Infantry

**Address:** InfantryClass::IronCurtain @ 0x00522600 (vtable+0x154 override)

In the InfantryClass vtable at 0x007EB058, offset 0x154 points to 0x00522600 instead
of the base TechnoClass::IronCurtain (0x0070E2B0). This is the famous "Iron Curtain
kills infantry" mechanic.

**Confidence:** HIGH -- verified from binary, vtable entry confirmed.

```c
void __thiscall InfantryClass::IronCurtain(int duration, int source_house, int is_force_shield)
{
    int damage = this->TypeClass->Strength;  // TypeClass+0xA0 = full HP
    this->ReceiveDamage(&damage, 0, Rules->C4Warhead, 0, true, 0, source_house);
    // vtable+0x16C = ReceiveDamage
    // Args: damage=full_hp, distance=0, warhead=C4Warhead, source=0,
    //       force_damage=true, unknown=0, source_house
}
```

### Key details:
- **Warhead used:** C4Warhead (Rules+0xFA8) -- the same warhead used for C4/demolition
- **Damage amount:** The infantry unit's full Strength (instant kill)
- **force_damage parameter:** `true` (1) -- this bypasses ALL damage reduction checks,
  including any invulnerability that might already be on the unit
- The infantry is **not** made invulnerable -- it is simply killed outright
- This applies to ALL infantry, regardless of type. There is no INI key to opt out.
- The source_house parameter is passed through for kill attribution

### InfantryClass vtable layout at 0x007EB058:
- vtable+0x154 = 0x00522600 (InfantryClass::IronCurtain -- kills infantry)
- vtable+0x158 = 0x0070E340 (TechnoClass -- shared)
- vtable+0x160 = 0x0041BF40 (TechnoClass::IsIronCurtainActive -- shared)

**NOTE:** The `InfDeath` animation for iron curtain death is controlled by the
C4Warhead's `InfDeath=` key, not by a separate IronCurtain-specific animation.
The string "InfDeath" is found at 0x00847D88.

---

## 14. Warhead and Special Weapon Interactions

### What Iron Curtain protects against (verified from binary):

**All of these check `IsIronCurtainActive()` (vtable+0x160) and refuse to act if true:**

| Interaction | Function | Address | How it's blocked |
|------------|----------|---------|-----------------|
| Normal damage | TechnoClass::ReceiveDamage | 0x00701900 | Damage set to 0, spark effect played |
| Temporal/Chrono erase | TemporalClass::CanWarpTarget | 0x0071AE50 | Returns 0 (cannot warp) if IC active |
| Mind control | CaptureManagerClass::CanCapture | 0x00471C90 | Returns 0 (cannot capture) if IC active |
| Crushing | TechnoClass::CanCrushCheck | 0x005F6CD0 | Returns 0 (cannot crush) if IC active |

### Temporal weapon interaction (TemporalClass::CanWarpTarget @ 0x0071AE50):

```c
bool TemporalClass::CanWarpTarget(TechnoClass* target) {
    if (target == NULL) return false;
    if (!target->TypeClass->Warpable)        // TypeClass+0xD3A
        return false;
    if (target->IsIronCurtainActive())        // vtable+0x160
        return false;                         // IC blocks temporal!
    // ... additional checks for building-docked units ...
    return true;
}
```

### Mind control interaction (CaptureManagerClass::CanCapture @ 0x00471C90):

```c
bool CaptureManagerClass::CanCapture(TechnoClass* target) {
    if (target == NULL) return false;
    if (target->AbstractClass type matches) return false;
    if (target->TypeClass->ImmuneToPsionics)  // TypeClass+0xD35
        return false;
    if (target is infantry in building) return false;
    if (target is being warped) return false;
    if (target->field_0x2CC != 0) return false;  // already captured?
    if (target->IsIronCurtainActive())            // vtable+0x160
        return false;                             // IC blocks mind control!
    // ... capacity checks ...
    return true;
}
```

### Crushing interaction (TechnoClass::CanCrushCheck @ 0x005F6CD0):

```c
bool TechnoClass::CanCrushCheck(TechnoClass* crusher) {
    // Check Crushable flag on victim's TypeClass
    if (!this->TypeClass->Crushable) return false;  // TypeClass+0xD29
    if (crusher == NULL || !crusher->Exists) return false;
    if (!crusher->TypeClass->Crusher) return false;  // TypeClass+0xD2A

    // First crush check: non-building, non-allied, non-IC
    if (crusher is not building && !IsAllied(this, crusher)) {
        if (!this->IsIronCurtainActive())       // vtable+0x160
            return true;                         // Can crush if NOT IC'd
    }

    // Second check: OmniCrush (TypeClass+0x22D)
    if (this->TypeClass->OmniCrushable && !IsAllied(this, crusher)) {
        if (!this->IsIronCurtainActive())       // vtable+0x160
            return true;
    }

    return false;  // IC active = cannot crush
}
```

### What Iron Curtain does NOT protect against:

- **Negative damage (healing):** In ReceiveDamage, the IC check is skipped if
  `damage < 0` (bVar13 flag). Healing always works on IC'd units.
- **force_damage=true calls:** The IC check in ReceiveDamage also skips if
  `in_stack_00000014 != '\0'` (the "ignore defenses" parameter). This is how
  InfantryClass::IronCurtain kills through any existing IC state.
- **IsWarpingOut state:** Units being warped out (vtable+0x1D4 = IsWarpingOut @
  0x0070C5B0) also get damage set to 0 in ReceiveDamage, but this is a SEPARATE
  check from IronCurtain.

### The `IronCurtain.Modifier` warhead key:

**NOT present in vanilla YR.** Searching for "IronCurtain.Modifier" in gamemd.exe
yields no results. This is an Ares/Phobos extension, not original game behavior.

---

## 15. Visual Effects and Rendering

### Color tinting:

When `IsIronCurtainActive()` is true during rendering, the drawing pipeline applies
a color tint from one of two RulesClass fields:

| Condition | RulesClass Offset | INI Key | Section |
|-----------|------------------|---------|---------|
| IronCurtain (field_0x1C4 == 0) | +0x18A8 | IronCurtainColor | [AudioVisual] |
| ForceShield (field_0x1C4 == 1) | +0x18B0 | ForceShieldColor | [AudioVisual] |

These are palette index values (ints) read via `CCINIClass::ReadInt` from [AudioVisual].

### In TechnoClass_DrawSHP (0x00705E00):

At the rendering stage, the code checks `IsIronCurtainActive()` (vtable+0x160).
When true, visual phase scaling is applied:
```c
if (IsIronCurtainActive() || /* building-specific override check */) {
    remap = TechnoClass__ScaleByTemporalVisualPhase(remap);
    remap = TechnoClass__ScaleByWarpInVisualPhase(remap);
}
```

The remap parameter passed to `CC_Draw_Shape` controls the sprite tinting.
The color index (IronCurtainColor or ForceShieldColor) determines which
palette remap table is used for the tint effect.

### Damage spark/flash effect:

In ReceiveDamage, when IC blocks damage, `FUN_0048a620` is called with:
- flash_type=1 for IronCurtain (blue sparks)
- flash_type=6 for ForceShield (red/orange sparks)

---

## 16. TechnoTypeClass INI Flags

### NoForceShield

String at 0x0081BC6C, referenced at vtable data 0x007E4D68.

This flag on TechnoTypeClass controls whether a building can receive ForceShield
protection. Buildings with `NoForceShield=yes` are skipped during the ForceShield
launch area iteration (case 10).

### IronCurtain (as superweapon type name)

The string "IronCurtain" at 0x0081BE54 is used as the superweapon type enum name
(type index = 1) in the SuperWeaponTypeClass type name lookup table at 0x008425C0.

---

## 17. SuperWeapon Type Enum (for reference)

Decoded from the string pointer table at 0x008425C0:

| Index | Type Name | Notes |
|-------|-----------|-------|
| 0 | MultiMissile | Nuclear missile |
| 1 | IronCurtain | Unit invulnerability |
| 2 | LightningStorm | Weather control |
| 3 | ChronoSphere | Unit teleportation |
| 4 | ChronoWarp | Chrono warp effect |
| 5 | ParaDrop | Paradrop |
| 6 | AmerParaDrop | American paradrop |
| 7 | PsychicDominator | Mass mind control |
| 8 | SpyPlane | Spy plane reveal |
| 9 | GeneticConverter | Genetic mutator |
| 10 | ForceShield | Building invulnerability |
| 11 | PsychicReveal | Psychic reveal |

---

## 18. Summary of Key Behavioral Rules

1. **Iron Curtain and ForceShield share the same timer mechanism** on TechnoClass
   (fields +0x18C, +0x194, +0x1C4). The `IsForceShield` flag only affects visuals.

2. **Timer is passive/implicit**: no per-tick decrement. `IsIronCurtainActive()` checks
   `CurrentFrame - StartFrame < Duration` on demand.

3. **Iron Curtain kills all infantry** via InfantryClass::IronCurtain override at
   0x00522600. Uses C4Warhead (Rules+0xFA8) with force_damage=true for instant kill.

4. **Iron Curtain blocks**: normal damage, temporal warping, mind control, crushing.

5. **Iron Curtain does NOT block**: healing (negative damage), force_damage=true calls.

6. **ForceShield only affects allied buildings** within ForceShieldRadius, while
   IronCurtain affects all objects in a fixed 3x3 cell grid.

7. **ForceShield triggers a power blackout** via SpyPowerSabotage. The blackout
   (default 1000f) lasts TWICE as long as the shield itself (default 500f).

8. **No IronCurtain.Modifier warhead key in vanilla YR** -- this is an Ares extension.
