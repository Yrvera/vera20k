# PsychicDominator Superweapon System — Ghidra Report

## Overview

The PsychicDominator (PD) is superweapon type 7 (`SuperWeaponTypeClass+0xB4 == 7`). It is
launched from `SuperClass::Launch` case 7, which calls the PD Start function. A per-tick state
machine advances the animation phases, and at a configurable percentage into the first animation,
the actual mind-control + damage effect fires. Ambient lighting gradually shifts to a red tint
during the effect and fades back to normal afterward.

**Confidence: HIGH** — all offsets and flow verified directly from Ghidra decompilation of
gamemd.exe. INI defaults verified against in-repo rulesmd.ini.

---

## 1. Global Variables (SuperWeapon Effect State)

These globals are shared across Lightning Storm, Psychic Dominator, and Nuke superweapon
effects. They live in BSS at addresses 0x00A9F9xx–0x00A9FAxx.

| Address       | Type  | Name                     | Description |
|---------------|-------|--------------------------|-------------|
| `0x00A9FA48`  | int   | PD_TargetCell            | CellStruct (packed X/Y) of PD target |
| `0x00A9FAC0`  | int   | PD_State                 | State machine: 0=inactive, 1–5=active phases |
| `0x00A9FAC4`  | ptr   | PD_CurrentAnim           | Pointer to current AnimClass (first or second) |
| `0x00A9FAC8`  | ptr   | PD_OwnerHouse            | HouseClass* of the player who fired the PD |
| `0x00A9FACC`  | ptr   | PD_DamageOwnerHouse      | HouseClass* used for area damage attribution |
| `0x00A9FAD0`  | byte  | (serialized flag)        | Related to lightning/NW effect state |
| `0x00A9FAB4`  | byte  | NukeActive               | Nuke superweapon active flag |
| `0x00A9FABC`  | int   | LightningStorm_State     | Lightning storm state machine |
| `0x00A9FAB0`  | int   | LightningStorm_Active    | Lightning storm active flag |
| `0x00A9FAB8`  | int   | (related SW state)       | Additional superweapon state |
| `0x00A9F9CC`  | int   | (reset target cell)      | Saved target cell state |
| `0x00A9F9F8`  | int   | InvalidCell              | "Zero" cell used to reset PD_TargetCell |
| `0x00A9FA70`  | int   | MC_PendingList_Count     | Count of pending mind-controlled units |
| `0x00A9FA64`  | ptr   | MC_PendingList_Array     | Array of pending mind-controlled unit ptrs |
| `0x00A9FA28`  | int   | MC_QueuedList_Count      | Queued events list count |
| `0x00A9FA1C`  | ptr   | MC_QueuedList_Array      | Queued events list array |
| `0x00827FC0`  | int   | LS_Timer1                | Lightning storm timer value 1 |
| `0x00827FC4`  | int   | LS_Timer2                | Lightning storm timer value 2 |
| `0x00827FC8`  | int   | LS_StartFrame            | Lightning storm start frame |
| `0x00827FCC`  | int   | LS_Duration              | Lightning storm duration |

### Serialization Order (Save/Load)

From FUN_00539890 (save) and FUN_00539ae0 (load), the globals are serialized in this order:
1. `DAT_00A9FAB4` (1 byte) — NukeActive
2. `DAT_00827FC0` (4 bytes) — LS_Timer1
3. `DAT_00827FC4` (4 bytes) — LS_Timer2
4. `DAT_00A9FAB8` (4 bytes)
5. `DAT_00A9FAD0` (1 byte)
6. `DAT_00827FC8` (4 bytes) — LS_StartFrame
7. `DAT_00827FCC` (4 bytes) — LS_Duration
8. `DAT_00A9FABC` (4 bytes) — LightningStorm_State
9. `DAT_00A9FAC0` (4 bytes) — PD_State
10. `DAT_00A9FAC4` (4 bytes) — PD_CurrentAnim
11. `DAT_00A9F9CC` (4 bytes) — saved target cell
12. `DAT_00A9FA48` (4 bytes) — PD_TargetCell
13. `DAT_00A9FACC` (4 bytes) — PD_DamageOwnerHouse
14. `DAT_00A9FAC8` (4 bytes) — PD_OwnerHouse
15. `DAT_00A9FA70` (4 bytes) — MC_PendingList count, then array
16. `DAT_00A9F9E0` (4 bytes) — second list count, then array
17. `DAT_00A9FA28` (4 bytes) — queued list count, then array

---

## 2. Rules INI Offsets (RulesClass)

Parsed in `RulesClass::ReadGeneral` (0x0066e000+) from the `[General]` section:

| Rules Offset | Type         | INI Key                   | Default (rulesmd.ini)  |
|-------------|--------------|---------------------------|------------------------|
| `+0x2F8`   | WarheadType* | DominatorWarhead          | DominatorWH            |
| `+0x2FC`   | AnimType*    | DominatorFirstAnim        | PDFXCLD                |
| `+0x300`   | AnimType*    | DominatorSecondAnim       | PDFXLOC                |
| `+0x304`   | int          | DominatorFireAtPercentage | 20                     |
| `+0x308`   | int          | DominatorCaptureRange     | 1                      |
| `+0x30C`   | int          | DominatorDamage           | 1000                   |

### Related Rules Offsets (CombatDamage section, parsed in FUN_0066bbb0)

| Rules Offset | Type         | INI Key                      | Default      |
|-------------|--------------|------------------------------|--------------|
| `+0x320`   | AnimType*    | ControlledAnimationType      | MINDANIM     |
| `+0x324`   | AnimType*    | PermaControlledAnimationType | MINDANIMR    |
| `+0x310`   | int          | MindControlAttackLineFrames  |              |

### PsychicDominatorActivateSound (AudioVisual section)

Parsed in `RulesClass::ReadAudioVisual` (param_1 is `int*`, so multiply index by 4):

| Rules Offset | Type | INI Key                         | Default                    |
|-------------|------|---------------------------------|----------------------------|
| `+0x24C`   | int  | PsychicDominatorActivateSound   | PsychicDominatorActivate   |

The sound is played in `SuperClass::Launch` case 7 via `VocClass::PlayAtCoord()` after
calling `FUN_0053ae50` (PD Start).

---

## 3. ScenarioClass Lighting Offsets

Parsed in FUN_00689e90 (ScenarioClass map init) from the `[Lighting]` section of each map.
These control the red ambient shift during the PD effect.

| Scenario Offset | Type | INI Key                    | Purpose |
|----------------|------|----------------------------|---------|
| `+0x3528`      | int  | (NormalAmbient)            | Baseline ambient level |
| `+0x352C`      | int  | (NormalAmbientTarget)      | Ambient fade-to target |
| `+0x3530`      | int  | (CurrentAmbientTarget)     | Active target ambient (set by FUN_0053c280) |
| `+0x3548`      | int  | NukeAmbient target         | Nuke lighting values |
| `+0x3554`      | int  | NukeAmbientChangeRate      | Nuke fade speed |
| `+0x3560`      | int  | LightningStorm ambient     | Storm lighting target |
| `+0x356C`      | int  | LightningStormChangeRate   | Storm fade speed |
| `+0x357C`      | int  | DominatorAmbient           | PD ambient target |
| `+0x3580`      | int  | DominatorRed               | PD red tint |
| `+0x3584`      | int  | DominatorGreen             | PD green tint |
| `+0x3588`      | int  | DominatorBlue              | PD blue tint |
| `+0x358C`      | int  | DominatorGround            | PD ground lighting |
| `+0x3590`      | int  | DominatorLevel             | PD level lighting |
| `+0x3594`      | int  | DominatorAmbientChangeRate | PD ambient fade speed |

### ScenarioClass Timing Offsets

| Scenario Offset | Type  | Purpose |
|----------------|-------|---------|
| `+0x1248`      | int   | PD activation start frame (g_CurrentFrameCounter) |
| `+0x124C`      | int   | PD target cell ground height (for sound/anim) |
| `+0x1250`      | int   | PD active marker (set to 1) |

---

## 4. Full Lifecycle

### Phase 0: Launch

**SuperClass::Launch case 7** (at ~0x006CD497):
1. Checks `param_1+0x6F != 0` (super is ready/enabled)
2. Calls `CreateRadarEvent()` to ping the minimap
3. Calls **FUN_0053ae50** (PD Start function)
4. If player is local and not observer, plays `VoxClass::PlayEVA("EVA_PsychicDominatorActivated")`
5. Gets target cell coords, plays `VocClass::PlayAtCoord()` with PsychicDominatorActivateSound
6. If `param_3` is set, resets sidebar and removes EVA from queue

### Phase 1: PD Start (FUN_0053ae50 at 0x0053ae50)

**Preconditions**: Rules `DominatorFirstAnim` (+0x2FC) and `DominatorSecondAnim` (+0x300) must
both be non-null. If either is null, the function returns immediately doing nothing.

**Actions**:
1. Stores `param_2` → `PD_TargetCell` (0x00A9FA48)
2. Stores `param_1` (owner house) → `PD_OwnerHouse` (0x00A9FAC8)
3. Gets CellClass at target cell, gets its 3D coordinates
4. Creates AnimClass at target with `DominatorFirstAnim` (the giant head), flags=0x600, loop=1
5. Stores anim pointer → `PD_CurrentAnim` (0x00A9FAC4)
6. Sets `PD_State = 1` (0x00A9FAC0)
7. Stores `g_CurrentFrameCounter` → `ScenarioClass+0x1248`
8. Stores cell height → `ScenarioClass+0x124C`
9. Sets `ScenarioClass+0x1250 = 1`
10. Calls **FUN_0053c280** (lighting change function) to start ambient shift

### Phase 2–5: Per-Tick State Machine (FUN_0053af40 at 0x0053af40)

Called every tick from `LightningStorm__Process` (actually "SuperWeaponEffects__Process").

```
State 1 → State 2    (immediate, one tick delay)
State 2 → State 3    (when first anim reaches DominatorFireAtPercentage% of its frames)
                      Condition: animFrame/totalFrames >= FireAtPercentage * 0.01
                      Action: calls FUN_0053b080 (the actual PD effect)
State 3 → State 4    (when first anim has < 11 frames remaining)
State 4 → State 5    (when first anim has < 2 frames remaining)
                      Action: resets target cell to InvalidCell, clears anim ptr,
                      calls FUN_0053c280 to start ambient fade-back
State 5 → State 0    (when ScenarioClass+0x3530 equals ScenarioClass+0x352C)
                      i.e., when ambient lighting has fully returned to normal
```

**Animation frame tracking:**
- `PD_CurrentAnim + 0xAC` = current frame index of the anim
- `PD_CurrentAnim + 0xC8` = AnimTypeClass pointer
- `AnimTypeClass->vtable[0x9C/4]()` returns SHP image; `*(short*)(image + 6)` = total frame count

### Phase 3: PD Effect (FUN_0053b080 at 0x0053b080)

This is the core function that applies mind control and damage. Called once when transitioning
from state 2 to state 3.

**Step 1: Create visual ring**
1. Allocates a new object via FUN_0053cb10 at the target cell coords (adds to a global vector)
2. Sets object+0x10 = 1 (marks it active)
3. Creates a new AnimClass with `DominatorSecondAnim` (ground ring) at the target cell

**Step 2: Apply area damage**
```c
Apply_area_damage(
    0,                                          // no source object
    *(WarheadType*)(Rules + 0x2F8),             // DominatorWarhead
    1,                                          // damage flag
    PD_DamageOwnerHouse                         // owner house for damage
);
```
The DominatorWarhead's CellSpread (7 in rulesmd.ini) determines the damage radius.
DominatorDamage (Rules+0x30C = 1000) is applied through the warhead system.

**Step 3: Mind control iteration**

Uses the CellSpread offset table at `0x007ED3D0` which maps radius → cell count:
```
Radius 0: 1 cell, Radius 1: 9 cells, Radius 2: 21 cells, ... Radius 10: 253 cells
```

The cell offset pairs at `0x00ABD490` (populated at startup) give the X/Y deltas for each cell
in the spread pattern.

For `DominatorCaptureRange` (Rules+0x308, capped to max 10), iterates all cells in that radius:
```
for each cell in CellSpread[DominatorCaptureRange]:
    cell_xy = target_cell + CellOffsetTable[i]
    for each object in cell (via CellClass::FindNearestObject linked list):
        apply filter checks
        if passes: mind-control the unit
```

**Filter checks for each object (all must pass):**

| Check | Vtable/Field | Meaning |
|-------|-------------|---------|
| `WhatAmI() != 6` | vtable+0x2C | Skip buildings (RTTI_Building=6) |
| `GetTypeClass()->ImmuneToPsionics == 0` | TypeClass+0xD35 | Not immune to psionics |
| `vtable+0x160 returns 0` | (likely IsIronCurtained) | Not under Iron Curtain protection |
| `GetTypeClass()->BalloonHover == 0` | TypeClass+0xD6A | Not a balloon unit |
| `vtable+0x54 returns 0` | (likely InLimbo check) | Not in limbo state |

**Mind control application for each qualifying unit:**

1. If unit already has a CaptureManager (`unit[0xB0] != 0` → byte offset 0x2C0):
   - Call `CaptureManagerClass::FreeUnit()` to release from existing mind control

2. Call `unit->SetOwner(PD_OwnerHouse, 1)` via vtable+0x3D4
   - Changes unit owner to the PD firing player
   - Second arg `1` = permanent transfer (not temporary MC)

3. Set `unit[0xB1] = 1` (byte offset 0x2C4) — marks unit as permanently mind-controlled

4. Get `MindControlRingOffset` from type class (TypeClass+0x60C)

5. Create AnimClass with `PermaControlledAnimationType` (Rules+0x324 = MINDANIMR) at unit
   position + MindControlRingOffset height

6. Store anim pointer in `unit[0xB2]` (byte offset 0x2C8) — the MC ring anim

7. If anim created, call `AnimClass::SetOwnerObject(unit)` to attach it

8. Add unit pointer to a local collection for post-processing

**Post-processing (non-player-controlled units):**

After iterating all cells, if the PD's owner is NOT the local player
(`HouseClass::IsPlayerControl()` returns false), iterate the collected units and call
`vtable+0x1E8` with args `(0xF, 0)` — this is likely `TechnoClass::Scatter(0xF, 0)` to
scatter the newly captured units.

---

## 5. FUN_0053c280 — Ambient Lighting Control (0x0053c280)

This shared function controls the ambient lighting transitions for all three super weapon
effects (Lightning Storm, Nuke, and Psychic Dominator).

**Logic:**

```
if (LightningStorm_State == 1 || LightningStorm_Active != 0):
    // Lightning storm lighting
    CurrentAmbientTarget = ScenarioClass+0x3560 (StormAmbient)
    changeRate = (ScenarioClass+0x356C * 1000) / 100

else if (NukeActive == 0):
    if (PD_State == 0 || PD_State == 5):
        // Normal: fade back to baseline
        CurrentAmbientTarget = ScenarioClass+0x3528 (NormalAmbient)
        call FUN_0053ad00(-1, 0)  // reset palette, no fade
        return
    else:
        // PD is active (states 1-4)
        CurrentAmbientTarget = ScenarioClass+0x357C (DominatorAmbient)
        changeRate = (ScenarioClass+0x3588 * 1000) / 100

else:
    // Nuke lighting
    CurrentAmbientTarget = ScenarioClass+0x3548 (NukeAmbient)
    changeRate = (ScenarioClass+0x3554 * 1000) / 100

call FUN_0053ad00(changeRate, 1)  // apply palette shift at given rate
```

**Priority**: Lightning Storm > Nuke > PsychicDominator > Normal

FUN_0053ad00 applies the ambient/palette change to all surfaces and color schemes at the
given rate. When rate=-1 and flag=0, it resets to normal.

---

## 6. Reset Function (FUN_00539760 at 0x00539760)

Clears ALL superweapon effect state. Called on game reset/new scenario.

Sets to zero: PD_TargetCell, PD_State, PD_CurrentAnim, PD_OwnerHouse, PD_DamageOwnerHouse,
LightningStorm_State, LightningStorm_Active, NukeActive, all timer globals.
Frees the mind-control pending/queued arrays.
Resets ambient to NormalAmbient and calls FUN_0053ad00(-1, 0).

---

## 7. DominatorWH Warhead (from rulesmd.ini)

```ini
[DominatorWH]
CellSpread=7          ; damage radius in cells
PercentAtMax=.2       ; 20% damage at edge
Verses=0%,0%,0%,0%,0%,0%,100%,100%,6%,0%,0%
```

The warhead does 0% damage to most armor types — effectively only damages certain armor
classes (100% to types 6 and 7, 6% to type 8). This means infantry/vehicles largely survive
the damage and get mind-controlled instead. Buildings (filtered out of MC iteration) take
the actual warhead damage.

---

## 8. TechnoClass Instance Offsets (relevant to PD)

All offsets below are byte offsets into TechnoClass instances (`int*` pointer with *4 math):

| Byte Offset | int* Index | Type  | Name |
|-------------|-----------|-------|------|
| `0x2C0`     | `[0xB0]`  | ptr   | CaptureManager* (null if not mind-controlled) |
| `0x2C4`     | `[0xB1]`  | bool  | IsPermanentlyMindControlled (set to 1 by PD) |
| `0x2C8`     | `[0xB2]`  | ptr   | MindControlRingAnim* (AnimClass for the MC ring) |

### TechnoTypeClass Offsets Used in Filter

| Byte Offset | Type | INI Key |
|-------------|------|---------|
| `0xD35`     | bool | ImmuneToPsionics |
| `0xD36`     | bool | ImmuneToPsionicWeapons |
| `0xD6A`     | bool | BalloonHover |
| `0x60C`     | int  | MindControlRingOffset |

---

## 9. EVA Events

| String Address | EVA Event | When Triggered |
|---------------|-----------|----------------|
| `0x00842470` | EVA_PsychicDominatorReady | SuperClass::AI_Ready, when PD finishes charging |
| `0x00842584` | EVA_PsychicDominatorActivated | SuperClass::Launch, when PD is fired |
| `0x00818EB0` | EVA_PsychicDominatorDetected | (detected by enemy — from Launch) |

---

## 10. Key Differences from Normal Mind Control

The PsychicDominator's mind control is **permanent transfer**, not standard CaptureManager
mind control:

1. **No CaptureManager link**: The PD calls `SetOwner(house, 1)` directly instead of going
   through CaptureManager::CaptureUnit. The `1` flag makes it a permanent owner change.

2. **Permanent flag**: Sets `TechnoClass+0x2C4 = 1` (IsPermanentlyMindControlled), which
   uses `PermaControlledAnimationType` (MINDANIMR — red ring) instead of the normal
   `ControlledAnimationType` (MINDANIM — blue ring).

3. **Existing MC freed first**: If the unit was already under normal mind control, the PD
   calls `CaptureManagerClass::FreeUnit()` before applying permanent capture.

4. **No range link**: Unlike normal MC which breaks at range, PD capture is permanent and
   has no controlling unit to break the link.

---

## 11. CellSpread Table (0x007ED3D0)

Pre-computed cumulative cell counts per radius level:

| Radius | Total Cells | Value at 0x007ED3D0 |
|--------|------------|---------------------|
| 0      | 1          | 0x00000001          |
| 1      | 9          | 0x00000009          |
| 2      | 21         | 0x00000015          |
| 3      | 37         | 0x00000025          |
| 4      | 61         | 0x0000003D          |
| 5      | 89         | 0x00000059          |
| 6      | 121        | 0x00000079          |
| 7      | 161        | 0x000000A1          |
| 8      | 205        | 0x000000CD          |
| 9      | 253        | 0x000000FD          |
| 10     | 309        | 0x00000135          |

DominatorCaptureRange is capped to max 10 in FUN_0053b080:
```c
if (captureRange > 9) captureRange = 10;  // clamp
```

The cell offset table at `0x00ABD490` is populated at runtime and contains the (X,Y) delta
pairs for each cell position in the spread pattern.

---

## 12. Function Address Summary

| Address      | Name | Purpose |
|-------------|------|---------|
| `0x0053AE50` | PsychicDominator::Start | Initializes PD effect, creates first anim, sets state=1 |
| `0x0053AF40` | PsychicDominator::Process | Per-tick state machine (states 1-5) |
| `0x0053B080` | PsychicDominator::Fire | Applies area damage + mind control effect |
| `0x0053B400` | PsychicDominator::IsActive | Returns PD_State != 0 |
| `0x0053B410` | PsychicDominator::ShowMessage | Displays "Psychic Dominator Active" message |
| `0x0053C280` | SuperWeaponEffects::UpdateLighting | Shared lighting control for LS/Nuke/PD |
| `0x0053AD00` | SuperWeaponEffects::ApplyPalette | Applies palette/ambient changes to all surfaces |
| `0x00539760` | SuperWeaponEffects::Reset | Clears all superweapon effect globals |
| `0x00539890` | SuperWeaponEffects::Save | Serializes all effect state to save game |
| `0x00539AE0` | SuperWeaponEffects::Load | Deserializes effect state from save game |
| `0x0053CB10` | (helper) | Creates visual effect object, adds to global vector |
| `0x006CC390+` | SuperClass::Launch case 7 | Entry point: launches PD from sidebar |
| `0x0053A6C0` | LightningStorm::Process | Actually "SuperWeaponEffects::Process" — calls PD tick |
| `0x0055AFB0` | LogicClass::PerTickUpdate | Game logic tick — calls the above |

---

## 13. Timing and Sequencing Summary

```
Frame N:     Player fires PD at target cell
             → SuperClass::Launch case 7
             → PD::Start: state=0→1, first anim created, lighting starts shifting
Frame N+1:   PD::Process: state=1→2 (one frame delay)
Frame N+?:   PD::Process: first anim reaches FireAtPercentage (20%) of its frames
             → state=2→3
             → PD::Fire: area damage + mind control applied (ONE-SHOT)
             → second anim (ground ring) created
Frame N+??:  PD::Process: first anim has < 11 frames left → state=3→4
Frame N+???: PD::Process: first anim has < 2 frames left → state=4→5
             → target cell reset, anim ptr cleared
             → lighting starts fading back to normal
Frame N+?+:  PD::Process: ambient has fully returned to normal → state=5→0 (done)
```

The mind control effect fires ONCE at the FireAtPercentage point. It is not reapplied.
After firing, the state machine just waits for the animation to finish playing.
