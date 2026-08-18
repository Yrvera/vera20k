# Global Sound Definitions — RulesClass::ReadAudioVisual

Research date: 2026-03-23
Confidence: HIGH (verified from binary decompilation + INI cross-reference)
Binary: gamemd.exe (Yuri's Revenge 1.001)
Function: `RulesClass__ReadAudioVisual` at `0x006691e0` (1168 lines decompiled)

## Overview

`RulesClass__ReadAudioVisual` reads the `[AudioVisual]` section from rules.ini into the
global `RulesClass` singleton (instance at `g_RulesClass` = `DAT_008871e0`). It parses:

- **74 individual sound entries** (VocClass index, resolved via `VocClass__FindByName`)
- **3 sound list entries** (comma-separated VocClass names into DynamicVectorClass)
- **2 animation list entries** (TreeFire, OnFire -- comma-separated AnimType names)
- **~10 animation entries** (AnimTypeClass references)
- **~30 non-sound visual/gameplay settings** (ints, doubles, bools, colors)

This report covers the sound entries only.

## How Sound References Work

### INI -> VocClass Resolution

Every sound entry in [AudioVisual] follows the same pattern:

```c
// 1. Read string from INI
int len = CCINIClass__ReadString("AudioVisual", "SellSound", "", buffer, 128);

// 2. If non-empty, look up in global VocClass array
if (len == 0 || (index = VocClass__FindByName(buffer)) == -1) {
    index = previous_value;  // keep default / previously loaded value
}

// 3. Store VocClass index in RulesClass
rules->SellSound = index;   // -1 means "no sound"
```

### VocClass__FindByName (0x007514d0)

Linear search through the global VocClass array:
- Array pointer: `DAT_00b1d37c` (pointer to array of VocClass pointers)
- Array count: `DAT_00b1d388`
- Compares input string against `VocClass__GetName(entry)` which reads name at entry+0x6C
- Returns array index (0-based), or -1 if not found
- The returned index is what gets stored in RulesClass fields

### VocClass Data Structure

Each VocClass entry is loaded from the `[SoundList]` section of rules.ini.
Per-entry properties (read in `VocClass__ReadINI` at `0x00750440`):
- `Sounds` -- comma-separated .aud filenames
- `Volume` -- playback volume (double, default from global)
- `VShift` -- volume shift (int)
- `MinVolume` -- minimum volume (double)
- `Priority` -- "NORMAL" default, controls DSP priority
- `Attack` -- attack envelope (int, ms)
- `Decay` -- decay envelope (int, ms)
- `Control` -- control flags (comma-separated)
- `Limit` -- max simultaneous instances
- `Range` -- audible range
- `Delay` -- playback delay
- `FShift` -- frequency shift

### Sound Playback

Sounds are played via:
- `VocClass__PlayAtPos(vocIndex, volume, loopSoundPtr)` at `0x00750920` -- positional SFX
- `VocClass__PlayGlobal(name, volume)` at `0x00406670` -- non-positional SFX (rare)

## Complete Sound Table

`param_1` in the decompilation is `undefined4 *` (pointer to 4-byte values).
Byte offset = array index * 4.

### GUI Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 1 | GUIMainButtonSound | 0x62 | 0x188 | MenuClick |
| 2 | GUIBuildSound | 0x63 (99) | 0x18C | MenuClick |
| 3 | GUITabSound | 0x64 (100) | 0x190 | MenuTab |
| 4 | GUIOpenSound | 0x65 | 0x194 | MenuACBOpen |
| 5 | GUICloseSound | 0x66 | 0x198 | MenuACBClose |
| 6 | GUIMoveOutSound | 0x67 | 0x19C | MenuSlideOut |
| 7 | GUIMoveInSound | 0x68 | 0x1A0 | MenuSlideIn |
| 8 | GUIComboOpenSound | 0x69 | 0x1A4 | MenuACBOpen |
| 9 | GUIComboCloseSound | 0x6A | 0x1A8 | MenuACBClose |
| 10 | GUICheckboxSound | 0x6B | 0x1AC | MenuClick |

### Score / Misc UI Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 11 | ScoreAnimSound | 0x6C | 0x1B0 | ScoreEmblemSoundLoop |
| 12 | GenericClick | 0x1C3 | 0x70C | MenuClick |
| 13 | GenericBeep | 0x1C4 | 0x710 | GenericBeep |
| 14 | ScoldSound | 0x1C0 | 0x700 | MenuScold |
| 15 | ShellButtonSlideSound | 0x1D4 | 0x750 | (empty) |

### Building Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 16 | BuildingDieSound | 0x1BA | 0x6E8 | BuildingGenericDie |
| 17 | BuildingSlam | 0x1BB | 0x6EC | PlaceBuilding |
| 18 | BuildingDamageSound | 0x1C5 | 0x714 | BuildingDamaged |
| 19 | BuildingDrop | 0x1F0 | 0x7C0 | PlaceBuilding |
| 20 | BuildingGarrisonedSound | 0x6F | 0x1BC | BuildingGarrisoned |
| 21 | BuildingAbandonedSound | 0x70 | 0x1C0 | (empty) |
| 22 | BuildingRepairedSound | 0x71 | 0x1C4 | BuildingRepaired |
| 23 | Construction | 0x1B2 | 0x6C8 | Dummy |

### Sell / Economy Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 24 | SellSound | 0x1A9 | 0x6A4 | SellBuilding |

### Radar / Movie Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 25 | RadarOn | 0x1BC | 0x6F0 | RadarOn |
| 26 | RadarOff | 0x1BD | 0x6F4 | RadarOff |
| 27 | MovieOn | 0x1BE | 0x6F8 | MovieOn |
| 28 | MovieOff | 0x1BF | 0x6FC | MovieOff |

### Base / Alert Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 29 | BaseUnderAttackSound | 0x61 | 0x184 | BaseUnderAttackSiren |

### Unit Creation Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 30 | CreateUnitSound | 0x5E | 0x178 | (empty) |
| 31 | CreateInfantrySound | 0x5F | 0x17C | (empty) |
| 32 | CreateAircraftSound | 0x60 | 0x180 | (empty) |

### Command Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 33 | StopSound | 0x1C8 | 0x720 | CommandBar |
| 34 | GuardSound | 0x1C9 | 0x724 | CommandBar |
| 35 | ScatterSound | 0x1CA | 0x728 | CommandBar |

### Crate Pickup Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 36 | CrateMoneySound | 0x79 | 0x1E4 | CrateMoney |
| 37 | CrateRevealSound | 0x7A | 0x1E8 | CrateReveal |
| 38 | CrateFireSound | 0x7B | 0x1EC | CrateFirePower |
| 39 | CrateArmourSound | 0x7C | 0x1F0 | CrateArmor |
| 40 | CrateSpeedSound | 0x7D | 0x1F4 | CrateSpeed |
| 41 | CrateUnitSound | 0x7E | 0x1F8 | CrateFreeUnit |
| 42 | CratePromoteSound | 0x7F | 0x1FC | CratePromoted |

### Impact / Environment Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 43 | ImpactWaterSound | 0x80 | 0x200 | ExplosionWaterLarge |
| 44 | ImpactLandSound | 0x81 | 0x204 | (empty) |
| 45 | SinkingSound | 0x82 | 0x208 | GenLargeWaterDie |
| 46 | DigSound | 0x5D | 0x174 | NukeSiren |
| 47 | ChuteSound | 0x1C7 | 0x71C | ParachuteDrop |
| 48 | StormSound | 0x1CC | 0x730 | WeatherIntro |

### Chrono / Teleport Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 49 | ChronoInSound | 0x86 | 0x218 | ChronoMinerTeleport |
| 50 | ChronoOutSound | 0x87 | 0x21C | ChronoMinerTeleport |
| 51 | DefaultChronoSound | 0x74 | 0x1D0 | (empty) |
| 52 | LetsDoTheTimeWarpOutAgain | 0xA1 | 0x284 | ChronoScreenSound |
| 53 | LetsDoTheTimeWarpInAgain | 0xA2 | 0x288 | ChronoScreenSoundAgain |

### Bomb / Special Weapon Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 54 | BombTickingSound | 0x83 | 0x20C | CrazyIvanBombTick |
| 55 | BombAttachSound | 0x84 | 0x210 | CrazyIvanAttack |
| 56 | YuriMindControlSound | 0x85 | 0x214 | YuriMindControl |
| 57 | MindClearedSound | 0x99 | 0x264 | MindCleared |
| 58 | TeslaCharge | 0x1C1 | 0x704 | TeslaCoilPowerUp |
| 59 | TeslaZap | 0x1C2 | 0x708 | (empty) |

### Stealth Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 60 | CloakSound | 0x1A8 | 0x6A0 | NavalUnitEmerge |

### Promotion Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 61 | UpgradeVeteranSound | 0x8A | 0x228 | UpgradeVeteran |
| 62 | UpgradeEliteSound | 0x8B | 0x22C | UpgradeElite |
| 63 | CheerSound | 0x72 | 0x1C8 | Cheer |

### Planning Mode Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 64 | StartPlanningModeSound | 0x75 | 0x1D4 | PlanningModeStart |
| 65 | EndPlanningModeSound | 0x78 | 0x1E0 | PlanningModeEnd |
| 66 | AddPlanningModeCommandSound | 0x76 | 0x1D8 | PlanningModeAdd |
| 67 | ExecutePlanSound | 0x77 | 0x1DC | (empty) |

### Beacon Sound

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 68 | PlaceBeaconSound | 0x73 | 0x1CC | BeaconPlaced |

### Multiplayer / Network Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 69 | GameClosed | 0x1AA | 0x6A8 | GameClosed |
| 70 | IncomingMessage | 0x1AB | 0x6AC | MessageText |
| 71 | MessageCharTyped | 0x1B1 | 0x6C4 | TextBleep |
| 72 | SystemError | 0x1AC | 0x6B0 | GenericBeep |
| 73 | OptionsChanged | 0x1AD | 0x6B4 | OptionsChanged |
| 74 | GameForming | 0x1AE | 0x6B8 | NewGame |
| 75 | PlayerLeft | 0x1AF | 0x6BC | (empty) |
| 76 | PlayerJoined | 0x1B0 | 0x6C0 | PlayerJoined |

### Spy Satellite Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 77 | SpySatActivationSound | 0x88 | 0x220 | SpyUplinkOn |
| 78 | SpySatDeactivationSound | 0x89 | 0x224 | SpyUplinkOff |

### Yuri's Revenge Mission Disk Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 79 | VoiceIFVRepair | 0x8C | 0x230 | IFVMove |
| 80 | SlavesFreeSound | 0x8D | 0x234 | SlaveWorkerLiberated |
| 81 | SlaveMinerDeploySound | 0x8E | 0x238 | SlaveMinerDeploy |
| 82 | SlaveMinerUndeploySound | 0x8F | 0x23C | SlaveMinerDeploy |
| 83 | BunkerWallsUpSound | 0x90 | 0x240 | TankBunkerUp |
| 84 | BunkerWallsDownSound | 0x91 | 0x244 | TankBunkerDown |
| 85 | RepairBridgeSound | 0x92 | 0x248 | BridgeRepaired |
| 86 | PsychicDominatorActivateSound | 0x93 | 0x24C | PsychicDominatorActivate |
| 87 | GeneticMutatorActivateSound | 0x94 | 0x250 | GeneticMutatorActivate |
| 88 | PsychicRevealActivateSound | 0x95 | 0x254 | PsychicRevealActivate |
| 89 | MasterMindOverloadDeathSound | 0x96 | 0x258 | MasterMindOverloadVoice |
| 90 | AirstrikeAbortSound | 0x97 | 0x25C | MIGMissionAborted |
| 91 | AirstrikeAttackVoice | 0x98 | 0x260 | MIGMove |
| 92 | EnterGrinderSound | 0x9A | 0x268 | GrinderGrinding |
| 93 | LeaveGrinderSound | 0x9B | 0x26C | (empty) |
| 94 | EnterBioReactorSound | 0x9C | 0x270 | BioReactorEnter |
| 95 | LeaveBioReactorSound | 0x9D | 0x274 | BioReactorEnter |
| 96 | ActivateSound | 0x9E | 0x278 | (empty) |
| 97 | DeactivateSound | 0x9F | 0x27C | (empty) |

### Spy Plane / Disk Laser Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 98 | SpyPlaneCamera | 0xA0 | 0x280 | SpyPlaneSnapshot |
| 99 | DiskLaserChargeUp | 0xA3 | 0x28C | FloatingDiscChargeUp |

### Gate Sounds

| # | INI Key | RulesClass Index | Byte Offset | Default (rulesmd.ini) |
|---|---------|-----------------|-------------|----------------------|
| 100 | GateUp | 0x101 | 0x404 | Dummy |
| 101 | GateDown | 0x102 | 0x408 | Dummy |

## Sound Lists (DynamicVectorClass of VocClass indices)

These entries are comma-separated lists of sound names, stored as DynamicVectorClass
objects rather than single VocClass indices. They are read by `CCINIClass__ReadSoundList`
at `0x00525430`.

### CreditTicks

| Field | RulesClass Index | Byte Offset | Purpose |
|-------|-----------------|-------------|---------|
| DynamicVectorClass base | 0x1B3 | 0x6CC | vtable pointer |
| Data pointer | 0x1B4 | 0x6D0 | pointer to int array |
| (internal) | 0x1B5 | 0x6D4 | (capacity/state) |
| (internal) | 0x1B6 | 0x6D8 | (capacity/state) |
| Count | 0x1B7 | 0x6DC | number of entries |
| Capacity | 0x1B8 | 0x6E0 | allocated capacity |
| GrowthStep | 0x1B9 | 0x6E4 | growth increment |

INI default: `CreditTicks=CreditUp,CreditDown`
- Element [0] = CreditUp (money gained)
- Element [1] = CreditDown (money spent)

### LightningSounds

| Field | RulesClass Index | Byte Offset | Purpose |
|-------|-----------------|-------------|---------|
| DynamicVectorClass base | 0x1CD | 0x734 | vtable pointer |
| Data pointer | 0x1CE | 0x738 | pointer to int array |
| Count | 0x1D1 | 0x744 | number of entries |
| Capacity | 0x1D2 | 0x748 | allocated capacity |
| GrowthStep | 0x1D3 | 0x74C | growth increment |

INI default: `LightningSounds=WeatherStrike`

### IceCrackSounds

| Field | RulesClass Index | Byte Offset | Purpose |
|-------|-----------------|-------------|---------|
| DynamicVectorClass base | 0x192 | 0x648 | vtable pointer |
| Data pointer | 0x193 | 0x64C | pointer to int array |
| Count | 0x196 | 0x658 | number of entries |
| Capacity | 0x197 | 0x65C | allocated capacity |
| GrowthStep | 0x198 | 0x660 | growth increment |

INI default: `IceCrackSounds=` (empty)

## Non-Sound Integer Entry

| INI Key | RulesClass Index | Byte Offset | Default | Description |
|---------|-----------------|-------------|---------|-------------|
| SpyPlaneCameraFrames | 0xA4 | 0x290 | 16 | Frames between spy plane camera sound plays |

## Credit Tick Sound System (Deep Dive)

### Where CreditTicks Sounds Are Triggered

`CreditsClass__Draw` at `0x004a2370` plays the credit tick sound.

**Assembly at 0x004a24F4-0x004a2533:**

```asm
004a24f4: MOV AL, byte ptr [EBP + 0xa]     ; this->animating
004a24fc: TEST AL, AL
004a24fe: JZ skip_sound                      ; skip if not animating
004a2500: MOV EAX, [0x008871e0]             ; g_RulesClass
004a2505: CMP dword ptr [EAX + 0x6dc], 2   ; CreditTicks.Count >= 2?
004a250b: JL skip_sound
004a250d: MOV CL, byte ptr [EBP + 0x9]     ; this->counting_up
004a2512: PUSH 0x0                           ; loopSoundPtr = 0
004a2519: PUSH 0x3f000000                    ; volume = 0.5f
004a251e: JZ use_down_sound
; counting_up == true:
004a2520: MOV EAX, [EAX + 0x6d0]           ; CreditTicks data pointer
004a2526: MOV ECX, [EAX]                     ; ECX = CreditTicks[0] (CreditUp)
004a2528: JMP play
; counting_up == false:
004a252a: MOV ECX, [EAX + 0x6d0]           ; CreditTicks data pointer
004a2530: MOV ECX, [ECX + 0x4]              ; ECX = CreditTicks[1] (CreditDown)
play:
004a2533: CALL VocClass__PlayAtPos
```

### Sound Selection Logic

```c
if (credits->animating && RulesClass->CreditTicks.Count >= 2) {
    int sound_index;
    if (credits->counting_up)
        sound_index = RulesClass->CreditTicks[0];   // "CreditUp"
    else
        sound_index = RulesClass->CreditTicks[1];   // "CreditDown"

    VocClass__PlayAtPos(sound_index, 0.5f, NULL);
}
```

### Tick Rate

- The sound plays **every frame** that `animating` is set (no throttle)
- At default game speed (15 fps): up to 15 sound triggers per second
- Volume: 50% (0x3F000000 = 0.5f in IEEE 754)
- Non-positional (no spatial panning)

### Counting Animation (from CreditsClass__AI at 0x004a2600)

```c
diff = target - displayed;
step = abs(diff) >> 3;     // geometric decay: 1/8 of remaining gap
step = clamp(step, 1, 143);
if (target < displayed) step = -step;
displayed += step;

if (step != 0) {
    animating = true;
    counting_up = (step > 0);
}
```

The counting uses geometric decay (7/8 ratio per frame), clamped to [1, 143] credits
per frame, ensuring the distinctive "fast start, slow finish" feel.

## VocClass System Architecture

### Global Array

- `DAT_00b1d37c` -- pointer to array of VocClass pointers
- `DAT_00b1d388` -- count of VocClass entries
- Populated from `[SoundList]` section of rules.ini

### VocClass Entry Layout

| Offset | Field | Description |
|--------|-------|-------------|
| +0x00 | name_index | Internal name ID (0 = invalid) |
| +0x6C | name_string | Sound name string (e.g., "CreditUp") |

### Resolution Flow

```
INI string (e.g., "CreditUp")
    |
    v
VocClass__FindByName(string)    @ 0x007514d0
    |  -- linear search through DAT_00b1d37c array
    |  -- compares against entry+0x6C (name string)
    v
Returns index (0-based) or -1
    |
    v
Stored in RulesClass field as int
    |
    v
At playback time: VocClass__PlayAtPos(index, volume, loopPtr)  @ 0x00750920
    |  -- validates index in [0, count)
    |  -- resolves VocClass* from array
    |  -- allocates SoundEvent from pool
    |  -- configures DirectSound buffer
    v
Sound plays
```

### Related Functions

| Address | Name | Description |
|---------|------|-------------|
| 0x007514d0 | VocClass__FindByName | String -> index lookup (returns int) |
| 0x00751520 | VocClass__FindPtrByName | String -> VocClass* pointer lookup |
| 0x007515c0 | VocClass__FindIndexByPtr | VocClass* -> index reverse lookup |
| 0x00405170 | VocClass__GetName | Gets name string at VocClass+0x6C |
| 0x00750920 | VocClass__PlayAtPos | Play sound by index with volume + position |
| 0x00406670 | VocClass__PlayGlobal | Play sound non-positionally (rare) |
| 0x004064a0 | VocClass__AddSample | Add .aud sample to a VocClass entry |
| 0x00750440 | VocClass__ReadINI | Read per-sound properties from [SoundList] |
| 0x00525430 | CCINIClass__ReadSoundList | Read comma-separated sound list from INI |

### Labels Applied This Session

| Address | New Label |
|---------|-----------|
| 0x00751520 | VocClass__FindPtrByName |
| 0x007515c0 | VocClass__FindIndexByPtr |
| 0x00405170 | VocClass__GetName |
| 0x00525430 | CCINIClass__ReadSoundList |

(Previously labeled: RulesClass__ReadAudioVisual, VocClass__FindByName, VocClass__PlayAtPos,
VocClass__PlayGlobal, VocClass__AddSample, VocClass__ReadINI, CreditsClass__AI,
CreditsClass__Draw, CreditsClass__Init)

## Notes

1. **DeploySound** appears in `[AudioVisual]` in rulesmd.ini (line 709) but is NOT read by
   `ReadAudioVisual` -- it is only read per-type in `TechnoTypeClass__ReadINI` at 0x00713568.
   The INI entry is effectively dead/ignored as a global setting.

2. **VeinGrowthRate** at `0x0083a2dc` is also in this section but is a double, not a sound.

3. Sound entries with empty defaults in rulesmd.ini (e.g., ImpactLandSound, TeslaZap,
   ExecutePlanSound) will remain at -1 (no sound) unless a mod provides a value.

4. The function also reads several **AnimTypeClass** references (DropPodPuff, VeinAttack,
   AtmosphereEntry, Dig, Smoke, SmallFire, LargeFire) and **animation lists** (TreeFire,
   OnFire) which are not sounds but are in the same function.

5. All 101 sound entries (74 individual + 3 lists with variable elements) are stored directly
   in the RulesClass singleton. There is no separate "audio settings" sub-object.
