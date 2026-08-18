# Lightning Storm Superweapon — Ghidra Research Report

**Primary Addresses:**
- `LightningStorm::Start` — `0x00539EB0`
- `LightningStorm::Process` — `0x0053A6C0`
- `LightningStorm::CreateCloudBolt` — `0x0053A140`
- `LightningStorm::GroundStrike` — `0x0053A300`
- `SuperWeaponEffects::UpdateLighting` — `0x0053C280`
- `SuperClass::Launch` case 2 — `0x006CC390`

**Confidence:** HIGH (all functions decompiled from binary)
**Active in YR:** Yes — launched via Weather Control Device (GAWEAT building)

---

## 1. Overview

The Lightning Storm is a superweapon (Type=2, `LightningStorm`) that creates a localized
electrical storm at a target cell. Bolts strike the target area over a configurable duration,
dealing area damage with the LightningWarhead. The system uses a two-phase bolt lifecycle
(cloud bolt → ground strike) and manages its own ambient lighting transition.

**Launch dispatch (Case 2) is the simplest of all 12 types:** it just calls
`LightningStorm::Start` and plays EVA. All logic is delegated to the LS state machine.

---

## 2. Global State Variables

| Address | Type | Name | Purpose |
|---------|------|------|---------|
| 0x00A9F9CC | CellStruct | LS_TargetCell | Center of the storm |
| 0x00A9F9F8 | CellStruct | LS_DefaultCell | Reset/invalid value for target |
| 0x00A9FAB4 | byte | LS_Active | 1 = storm in progress |
| 0x00A9FAB8 | int | LS_QueueCountdown | Frames until queued storm starts |
| 0x00A9FACC | ptr | LS_OwnerHouse | HouseClass* who fired |
| 0x00A9FAD0 | byte | LS_Ending | 1 = storm winding down (no new bolts) |
| 0x00827FC0 | int | LS_StartFrame | g_CurrentFrameCounter when started |
| 0x00827FC4 | int | LS_Duration | Duration in frames (-1 = infinite) |
| 0x00A9FA30 | int | LS_LastStrikeX | Lepton X of last strike (avoid duplicate) |
| 0x00A9FA34 | int | LS_LastStrikeY | Lepton Y of last strike |
| 0x00A9FA38 | int | LS_LastStrikeZ | Lepton Z of last strike |
| 0x00A9FA84 | int | LS_BridgeZOffset | Added to Z when on bridge |
| 0x00A9FA90 | int | LS_HeightPerLevel | Z offset per height level |

**NOTE:** Address `0x00A9FAB4` was mislabeled as "NukeActive" in the
PSYCHIC_DOMINATOR_SUPERWEAPON_GHIDRA_REPORT. It is the **Lightning Storm active flag**,
verified from both Start (sets it to 1) and Process (clears it on cleanup).

### Anim Tracking Arrays

Three DynamicVectorClass arrays track active animations for cleanup:

| Base Address | Count Address | Purpose |
|-------------|---------------|---------|
| 0x00A9F9D4 | 0x00A9F9E0 | Cloud bolt anims (overhead visual) |
| 0x00A9FA64 | 0x00A9FA70 | Strike anims (waiting for ground hit trigger) |
| 0x00A9FA1C | 0x00A9FA28 | Debris/explosion anims |

---

## 3. Rules Offsets

| Rules Offset | INI Key | Type | Default (YR) | Purpose |
|-------------|---------|------|--------------|---------|
| 0x1794 | `LightningDeferment` | int | 250 | Frames between announcement and storm start |
| 0x1798 | `LightningDamage` | int | 250 | Damage per strike (passed to Apply_area_damage as EDX) |
| 0x179C | `LightningStormDuration` | int | 180 | Active storm duration in frames |
| 0x17A0 | `LightningHitDelay` | int | 10 | Frames between center strikes |
| 0x17A4 | `LightningScatterDelay` | int | 5 | Frames between random strikes. INI comment: "DO NOT DECREASE -- PERFORMANCE HIT" |
| 0x17A8 | `LightningCellSpread` | int | 10 | Spread range in cells (divided by 2 for ± offset) |
| 0x17AC | `LightningSeparation` | int | 3 | Min city-block distance (manhattan) between bolts |
| 0x17B0 | `LightningPrintText` | bool | — | Enable countdown EVA/text notifications |
| 0x17B4 | `LightningWarhead` | ptr | IonWH | WarheadType* for strike damage |
| 0x02C0 | `WeatherConClouds` | DVec | WCCLOUD1-3 | Cloud darkening anim list |
| 0x02CC | (count) | int | 3 | Number of cloud anims |
| 0x02DC | `WeatherConBolts` | DVec | WCLBOLT1-3 | Lightning bolt anim list |
| 0x02E8 | (count) | int | 3 | Number of bolt anims |
| 0x0744 | `LightningSounds` | DVec | WeatherStrike | Thunder sound list |
| 0x0140 | `Scorches` | DVec | — | Fire/debris anims for destroyed terrain |
| 0x014C | (count) | int | — | Number of scorch anims |

**Duration:** `LightningStormDuration` at Rules+0x179C (default 180 frames = 12 sec at 15fps).
This is the active storm phase only; existing bolts finishing extends total visible time.

**Deferment:** `LightningDeferment` at Rules+0x1794 (default 250 frames). The countdown
between announcement (EVA warning to enemies) and the storm actually starting.

**Damage:** `LightningDamage` at Rules+0x1798 (default 250). Passed as `param_2` (EDX) to
`Apply_area_damage`. Each ground strike applies this damage via the LightningWarhead.

---

## 4. Two-Phase Bolt Lifecycle

Each lightning bolt goes through two phases:

### Phase 1 — Cloud Bolt (LightningStorm::CreateCloudBolt — 0x0053A140)

Creates a visual-only cloud darkening effect. No damage.

```
get cell at target position
calculate Z: heightLevel * LS_HeightPerLevel + bridgeOffset + terrainOffset

if coords == last strike coords:
    return  // anti-duplicate, skip exact same position

pick random anim from WeatherConClouds array (Rules+0x2C0)
    index = Random() % WeatherConClouds_Count (Rules+0x2CC)

create AnimClass at target lepton coords
track anim in cloud bolt array AND strike anim array
```

### Phase 2 — Ground Strike (LightningStorm::GroundStrike — 0x0053A300)

Triggered when a cloud bolt anim reaches **half its total frame count**.
This is the actual damaging event.

```
get cell at strike location

// Bolt anim is created FIRST, before the anti-duplicate check.
// Anti-duplicate only skips the sound+damage, NOT the bolt visual.
// (corrected 2026-05-29: was "anti-duplicate check before anim creation" —
//  binary shows AnimClass created and pushed to debris array before the
//  coordinate equality check fires; verified via decompile_function 0x0053A300
//  — OPERATOR_OR_ORDER_DRIFT)
create random bolt anim from WeatherConBolts (Rules+0x2DC)
    index = Random__Next() % WeatherConBolts_Count (Rules+0x2E8)
track in debris array

if coords == last strike coords:
    return  // anti-duplicate: bolt visual already spawned, but skip sound+damage

// Sound
if LightningSounds count > 0:
    play random thunder sound

// Visual explosion
create explosion anim via Warhead::SelectExplosionAnim (based on cell overlay)

// Pre-damage snapshot
building_before = Look_up_building_in_cell()
infantry_before = CellClass::Find_Nearest_Object() where WhatAmI == 0xF

// Damage
create explosion effect (FUN_0048A620)
Apply_area_damage(0, LightningWarhead, 1, LS_OwnerHouse)

// Post-damage check
building_after = Look_up_building_in_cell()
infantry_after = CellClass::Find_Nearest_Object()

terrain_destroyed = false
if building was destroyed OR infantry was killed OR
   cell overlay type in {Road, Concrete, ...}:
    terrain_destroyed = true

// Debris fires (only if NOT infantry hit AND terrain was destroyed)
if !infantry_hit AND terrain_destroyed:
    scorch_count = Random(2, 4)
    for i in 0..scorch_count:
        pick random scorch anim from Scorches (Rules+0x140)
        create AnimClass at strike coords
```

---

## 5. Process Tick (LightningStorm::Process — 0x0053A6C0)

Called every game tick. Manages the complete LS lifecycle.

```
// --- PD state machine transitions (shared code in same function) ---
if PD_State == 1 AND timer expired:
    PD_State = 2, set delay = 15 frames
    update lighting
if PD_State == 2 AND timer expired:
    PD_State = 0

PsychicDominator::Process()  // FUN_0053AF40
Process_QueuedEvents()

// --- Anim cleanup loops (3 arrays, reverse iteration) ---

// 1. Debris anims: remove when past half duration
for each debris anim (reverse):
    if current_frame >= total_frames / 2:
        remove from debris array

// 2. Strike anims → trigger ground strike when past half duration
for each strike anim (reverse):
    if current_frame > total_frames / 2:
        GroundStrike(anim.coords)  // Phase 2 damage!
        remove from strike array

// 3. Cloud bolt anims: remove when at last frame
for each cloud bolt (reverse):
    if current_frame >= total_frames - 1:
        remove from cloud bolt array

// --- Storm cleanup check ---
if cloud_bolt_count == 0 AND LS_Ending:
    if LS_Active:
        LS_Active = false
        LS_OwnerHouse = NULL
        LS_TargetCell = LS_DefaultCell
        UpdateLighting()  // restore normal ambient
    LS_Ending = false

// --- Active storm logic ---
if LS_Active AND NOT LS_Ending:
    // Duration check
    if LS_Duration != -1 AND CurrentFrame > LS_StartFrame + LS_Duration:
        LS_Ending = true  // begin wind-down, no new bolts

    // Center bolt: every LightningHitDelay frames
    if CurrentFrame % Rules.LightningHitDelay == 0:
        CreateCloudBolt(LS_TargetCell)

    // Random bolt: every LightningScatterDelay frames
    if CurrentFrame % Rules.LightningScatterDelay == 0:
        attempts = 3
        spread = Rules.LightningCellSpread / 2
        do:
            randomCell.X = LS_TargetCell.X + Random(-spread, +spread)
            randomCell.Y = LS_TargetCell.Y + Random(-spread, +spread)

            // Minimum distance check against existing cloud bolts
            tooClose = false
            for each cloud bolt:
                boltCell = bolt.GetCoords() >> 8  // leptons to cells
                if |randomCell.X - boltCell.X| + |randomCell.Y - boltCell.Y| < LightningSeparation:
                    tooClose = true
                    break

            if Cell_in_bounds(randomCell) AND NOT tooClose:
                CreateCloudBolt(randomCell)
                break

            attempts--
        while attempts > 0

// --- Queue countdown (storm not yet started) ---
else if LS_QueueCountdown > 0:
    LS_QueueCountdown--
    if LS_QueueCountdown == 0:
        Start(LS_TargetCell, LS_OwnerHouse)  // deferred storm begins

    // EVA countdown notifications every 225 frames (0xE1, hardcoded)
    if LS_QueueCountdown % 225 == 0 AND Rules+0x17B0 (LS_Warning):
        play EVA voice + text notification
```

---

## 6. Start (LightningStorm::Start — 0x00539EB0)

Called from `SuperClass::Launch` case 2 and from Process when a queued storm's countdown
reaches zero.

```
// Validate target cell
if LS_TargetCell == LS_DefaultCell:
    // Invalid/default target — pick random valid cell
    repeat:
        cell.X = Random(0, MapWidth)
        cell.Y = Random(0, MapHeight)
    until Cell_in_bounds(cell)

LS_TargetCell = target_cell
LS_OwnerHouse = owner_house

// Deferment > 0 AND storm NOT yet active? Queue it instead of starting.
// (corrected 2026-05-29: was "Already active? Queue instead" — binary shows
//  queuing fires when LS_Active==false AND param_2 (deferment) != 0; when
//  LS_Active==true, Start() simply returns doing nothing — no queuing;
//  verified via decompile_function 0x00539EB0 — OPERATOR_OR_ORDER_DRIFT)
if NOT LS_Active AND param_2 != 0:
    if LS_QueueCountdown == 0 OR param_2 <= LS_QueueCountdown:
        LS_QueueCountdown = param_2   // deferment frames
    LS_Duration = param_1             // store duration
    return

// LS_Active == true → Start() does nothing (returns immediately, no queuing)
if LS_Active:
    return

// --- Start fresh storm ---
CreateRadarEvent(target_cell)
LS_StartFrame = g_CurrentFrameCounter
LS_Active = true
LS_Duration = param_1

// EVA warning to all non-allied, non-defeated houses
for each house:
    if NOT allied with owner AND NOT defeated:
        play EVA warning via FUN_0050bcd0

PlayerPtr+0x5779 = 1  // flag on local player's house

UpdateLighting()  // transition to storm ambient

if Rules+0x17B0 (LS_Warning):
    play storm start sound
    display text notification
```

---

## 7. Shared Lighting System (SuperWeaponEffects::UpdateLighting — 0x0053C280)

Manages ambient lighting transitions for Lightning Storm, Nuke flash, and
Psychic Dominator. Uses a priority system:

```
ScenarioClass* scen = DAT_00a8b230;

if PD_State == 1 (flash) OR NukeFlash active:
    scen->CurrentAmbient (+0x3530) = scen->FlashAmbient (+0x3560)
    intensity = scen->FlashIntensity (+0x356C) * 1000 / 100

else if LS_Active:
    scen->CurrentAmbient = scen->LSAmbient (+0x3548)
    intensity = scen->LSIntensity (+0x3554) * 1000 / 100

else if PD_State == 0 or 5 (inactive/done):
    scen->CurrentAmbient = scen->NormalAmbient (+0x3528)
    FUN_0053AD00(-1, 0)  // restore normal lighting
    return

else (PD active, states 2-4):
    scen->CurrentAmbient = scen->PDAmbient (+0x357C)
    intensity = scen->PDIntensity (+0x3588) * 1000 / 100

FUN_0053AD00(intensity, 1)  // apply lighting change
```

**Lighting transition offsets (from ScenarioClass):**

| Offset | Purpose |
|--------|---------|
| +0x3528 | Normal ambient |
| +0x3530 | Current ambient (actively updated) |
| +0x3548 | Lightning Storm ambient |
| +0x3554 | Lightning Storm intensity |
| +0x3560 | Flash ambient (Nuke/PD flash) |
| +0x356C | Flash intensity |
| +0x357C | Psychic Dominator ambient |
| +0x3588 | Psychic Dominator intensity |

---

## 8. INI Configuration (from rulesmd.ini)

### [General] Section

```ini
LightningStormDuration=180    ; Duration in frames (12 sec at 15fps). Overridable by triggers.
LightningDeferment=250        ; Frames between announcement and commencement
LightningDamage=250           ; Damage value (applied via warhead)
LightningWarhead=IonWH        ; Warhead for strike damage
LightningHitDelay=10          ; Frames between center bolt strikes
LightningScatterDelay=5       ; Frames between random bolt strikes (DO NOT DECREASE)
LightningCellSpread=10        ; Spread range in cells (n×n square)
LightningSeparation=3         ; Min city-block distance between bolts
WeatherConClouds=WCCLOUD1,WCCLOUD2,WCCLOUD3   ; Cloud darkening anims
WeatherConBolts=WCLBOLT1,WCLBOLT2,WCLBOLT3    ; Lightning bolt anims
WeatherConBoltExplosion=EXPLOLB               ; Special bolt explosion anim
LightningSounds=WeatherStrike                 ; Thunder sound effects
StormSound=WeatherIntro                       ; Sound when storm starts
```

### [LightningStormSpecial] Section

```ini
UIName=Name:Storm
Name=Lightning Storm
IsPowered=true
RechargeTime=10               ; 10 minutes
Type=LightningStorm
Action=LightningStorm
SidebarImage=BOLTICON
ShowTimer=yes
DisableableFromShell=yes
AIDefendAgainst=yes
Range=7
LineMultiplier=2
```

**Building:** GAWEAT (Allied Weather Control Device)

---

## 9. Integration Points

- **Caller:** `SuperClass::Launch` case 2 calls `LightningStorm::Start`
- **Tick:** `LightningStorm::Process` is called every game tick (from the global event loop)
- **Lighting:** Shares `UpdateLighting` with Nuke flash and Psychic Dominator
- **Damage:** Uses `Apply_area_damage` with the configured WarheadType*
- **Queuing:** Only one LS can be active at a time; additional launches are queued

---

## 10. Open Questions

1. **LS countdown EVA interval = 225 (0xE1)** — Hardcoded constant. No INI key.

2. **WeatherConBoltExplosion** — Listed in INI but its Rules offset is not traced.
   Used by `Warhead::SelectExplosionAnim` internally.

## 11. Resolved Questions (from verification pass)

1. **Rules+0x17B0 = `LightningPrintText`** — RESOLVED. Bool flag read via
   `CCINIClass::ReadBool` in `RulesClass::ReadGeneral` at `0x0067107F`. Controls
   whether countdown EVA notifications and text announcements are displayed.
   Not found in rulesmd.ini grep because it's not set in standard YR INI — meaning
   it defaults to the constructor's default value (likely false in standard YR).

2. **LightningDeferment at Rules+0x1794** — RESOLVED. Read in ReadGeneral. This is
   the countdown stored in `LS_QueueCountdown` before the storm starts.

3. **LightningDamage at Rules+0x1798** — RESOLVED. Read in ReadGeneral. Passed as
   `param_2` (EDX register) to `Apply_area_damage`, which stores it as `local_c4`
   then passes `&local_c4` to each target's `TechnoClass::ReceiveDamage` (vtable+0x16C).

4. **LightningStormDuration at Rules+0x179C** — RESOLVED. Read in ReadGeneral. Stored
   as `LS_Duration` when storm starts.

---

## Sources

**Ghidra functions decompiled:**
- 0x00539EB0 (LightningStorm::Start — 74 lines)
- 0x0053A6C0 (LightningStorm::Process — 184 lines)
- 0x0053A140 (LightningStorm::CreateCloudBolt — 62 lines)
- 0x0053A300 (LightningStorm::GroundStrike — 120 lines)
- 0x0053C280 (SuperWeaponEffects::UpdateLighting — 27 lines)

**INI files checked:** ini/rulesmd.ini, ini/rules.ini

**Date:** 2026-04-02
