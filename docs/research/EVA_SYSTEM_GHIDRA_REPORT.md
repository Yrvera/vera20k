# EVA Announcement System - Ghidra Reverse Engineering Report

**Date:** 2026-03-23
**Binary:** gamemd.exe (Yuri's Revenge)
**Confidence:** HIGH - all findings verified from decompiled binary code

## Overview

The EVA (Electronic Video Agent) system handles all voice announcements in the game:
"Construction complete", "Unit ready", "Insufficient funds", "Our base is under attack", etc.

The system consists of:
1. **VoxClass** - data structure for each EVA event (loaded from evamd.ini)
2. **VoxClass static functions** - dispatch, queue management, playback
3. **StreamPlayer** - dedicated DirectSound audio channel for EVA (separate from SFX and speech/taunts)
4. **RadarEvent** - rate-limiting system for combat announcements

## Key Addresses

### Global Variables

| Address | Type | Name | Description |
|---------|------|------|-------------|
| `0x00b1d4a4` | `VoxClass**` | VoxArray_Data | Pointer to array of VoxClass pointers |
| `0x00b1d4a8` | `int` | VoxArray_Capacity | Allocated capacity of VoxClass array |
| `0x00b1d4b0` | `int` | VoxArray_Count | Number of loaded VoxClass entries |
| `0x00b1d4b8` | `QueueNode*` | PendingImmediate | Single pending immediate-play entry |
| `0x00b1d4bc` | `int` | SystemEnabled | 1 when EVA system is active |
| `0x00b1d4c0` | `int` | SequenceCounter | Monotonic counter for queue ordering |
| `0x00b1d4c4` | `VoxClass*` | CurrentlyPlaying | VoxClass currently being played (or 0) |
| `0x00b1d4c8` | `int` | CurrentSide | 0=Allied, 1=Russian, 2=Yuri |
| `0x00b1d4cc` | `StreamPlayer*` | EVAStreamPlayer | Dedicated audio stream for EVA playback |
| `0x00b1d4d0` | `int` | InterAnnouncementDelay_Lo | Low 32 bits of 64-bit delay (set to 500) |
| `0x00b1d4d4` | `int` | InterAnnouncementDelay_Hi | High 32 bits of delay |
| `0x00b1d4d8` | `StreamPlayer*` | SpeechStreamPlayer | Separate stream for taunts (NOT EVA) |
| `0x00b1d3d8` | `int` | SuspendCounter | Nested counter; EVA suppressed when > 0 |
| `0x00b1d428` | `int` | PauseCounter | Nested counter; EVA paused when > 0 |
| `0x00b1d480` | `int` | TauntLock | Prevents taunts during EVA |

### Priority Queue Memory Layout

The EVA system uses multiple linked-list queues at fixed addresses:

| Address | Queue | Description |
|---------|-------|-------------|
| `0x00b1d3c8` | InterruptQueue | For Type=QUEUED_INTERRUPT (Type 3) |
| `0x00b1d3f0` | CriticalQueue | For Priority=CRITICAL with certain types |
| `0x00b1d450` | PriorityQueue[0] | For Type=QUEUE, priority level 0 |
| `0x00b1d45c` | PriorityQueue[1] | For Type=QUEUE, priority level 1 |
| `0x00b1d468` | PriorityQueue[2] | For Type=QUEUE, priority level 2 |
| `0x00b1d474` | PriorityQueue[3] | For Type=QUEUE, priority level 3 |

Each queue is a 12-byte linked list head structure (3 pointers).

### Functions (all labeled in Ghidra)

| Address | Name | Signature | Description |
|---------|------|-----------|-------------|
| `0x00752700` | VoxClass__PlayEVA | `void __thiscall(char* evaName, int type)` | Main dispatch: looks up EVA event by name string, calls QueueVoice |
| `0x00752480` | VoxClass__QueueVoice | `void __fastcall(int index, int type, int priority)` | Queues a VoxClass by index into appropriate queue |
| `0x00752760` | VoxClass__PlayNextQueued | `void(void)` | Dequeues next EVA and plays it via StreamPlayer |
| `0x00752590` | VoxClass__InsertIntoQueue | `void __fastcall(int voxPtr, int type, int priority)` | Creates queue node and inserts into correct queue |
| `0x00752680` | VoxClass__FindInQueues | `int __fastcall(int voxPtr)` | Searches all queues for a given VoxClass |
| `0x00752370` | VoxClass__ClearAllQueues | `void(void)` | Empties all priority queues and pending |
| `0x00753000` | VoxClass__ReadEVAINI | `void(void)` | Parses [DialogList] section, creates VoxClass entries |
| `0x00752db0` | VoxClass__ReadINI | `int __fastcall(VoxClass* this)` | Reads Type, Priority, Volume, Allied/Russian/Yuri from INI |
| `0x007531a0` | VoxClass__ClearAllEntries | `void(void)` | Destroys all VoxClass objects and frees memory |
| `0x007532d0` | VoxClass__FindByName | `VoxClass* __fastcall(char* name)` | Looks up VoxClass by name string (filters `<none>`) |
| `0x00752460` | VoxClass__GetByIndex | `VoxClass* __fastcall(int index)` | Returns VoxClass pointer by array index |
| `0x00752a40` | VoxClass__RemoveFromQueues | `void __fastcall(int index)` | Removes all queued instances of a VoxClass |
| `0x007534e0` | VoxClass__SetSide | `void __fastcall(int side)` | Sets CurrentSide (0=Allied, 1=Russian, 2=Yuri) |
| `0x00753570` | VoxClass__SuspendEVA | `void(void)` | Increments suspend counter (suppresses new EVA) |
| `0x00753580` | VoxClass__ResumeEVA | `void(void)` | Decrements suspend counter |
| `0x007535b0` | VoxClass__PauseEVA | `void(void)` | Increments pause counter (freezes playback) |
| `0x00753620` | VoxClass__UnpauseEVA | `void(void)` | Decrements pause counter |
| `0x007535d0` | VoxClass__ResetAll | `void(void)` | Clears queues, resets suspend/pause, stops playback |
| `0x007533f0` | VoxClass__LoadFromSave | `int(IStream*)` | Restores EVA queue state from savegame |
| `0x00752290` | VoiceSystem__Init | `int(void)` | Initializes queues, creates EVA StreamPlayer at 0xb1d4cc |
| `0x00752ad0` | SpeechSystem__Init | `int(void)` | Initializes separate speech StreamPlayer at 0xb1d4d8 |
| `0x00752b70` | SpeechSystem__PlayTaunt | `int __fastcall(uint tauntId)` | Plays multiplayer taunts via SpeechStreamPlayer |
| `0x00752b40` | SpeechSystem__Shutdown | `void(void)` | Destroys SpeechStreamPlayer |
| `0x004f93e0` | HouseClass__BaseUnderAttack | `void __thiscall(TechnoClass*)` | Rate-limited "base under attack" EVA trigger |

## VoxClass Struct Layout (size = 0x54 = 84 bytes)

```
Offset  Size  Type       Field              Description
------  ----  ---------  -----------------  -----------
0x00    40    char[40]   Name               EVA event name (e.g., "EVA_ConstructionComplete")
0x28    4     float      Volume             Playback volume (default 1.0)
0x2C    9     char[9]    YuriSound          Yuri faction sound file (e.g., "cyur048")
0x35    9     char[9]    RussianSound       Russian faction sound file (e.g., "csof048")
0x3E    9     char[9]    AlliedSound        Allied faction sound file (e.g., "ceva048")
0x47    1     pad        -                  padding
0x48    4     int        Priority           0=LOWEST, 1=LOW, 2=NORMAL/HIGH, 3=CRITICAL
0x4C    4     int        Type               0=STANDARD, 1=QUEUE, 2=INTERRUPT, 3=QUEUED_INTERRUPT
0x50    4     int        PlayState          0=playing, 1=queued, 2=done/free
```

## Queue Node Layout (size = 0x20 = 32 bytes)

```
Offset  Size  Type       Field              Description
------  ----  ---------  -----------------  -----------
0x00    4     Node*      prev               Linked list previous pointer
0x04    4     Node*      next               Linked list next pointer
0x08    4     ???        unknown            (list head back-pointer?)
0x0C    4     VoxClass*  VoxEntry           Pointer to the VoxClass being queued
0x10    4     ???        unknown2
0x14    4     int        Priority           Priority level for this queued entry
0x18    4     int        Type               Type of this queued entry
0x1C    4     int        SequenceNum        Counter % 100 for ordering
```

## EVA Event Flow

### 1. Dispatch (VoxClass__PlayEVA at 0x00752700)

```
PlayEVA(evaName, type):
    if evaName == NULL: return
    for i in 0..VoxArray_Count:
        if stricmp(evaName, VoxArray[i].Name) == 0:
            QueueVoice(index=i, type=type, priority=-1)
            return
    QueueVoice(index=-1, type=type, priority=-1)  // not found case
```

The function is `__thiscall` where ECX = EVA event name string, EDX = type parameter.
Most callers pass `type = -1` (0xFFFFFFFF) meaning "use default type from the VoxClass INI definition".

### 2. Queue Insertion (VoxClass__QueueVoice at 0x00752480)

```
QueueVoice(index, type, priority):
    // Guard checks
    if EVAStreamPlayer == NULL: return
    if index < 0 or index >= VoxArray_Count: return
    if SuspendCounter > 0: return           // EVA is suspended
    vox = VoxArray[index]
    if vox == CurrentlyPlaying: return       // don't re-queue current

    // Default from INI if -1
    if type == -1: type = vox.Type
    if priority == -1: priority = vox.Priority

    // INTERRUPT type (2) clears everything first
    if CurrentlyPlaying != NULL and type == 2:
        // Flush all queues
        // Stop current playback
        // Clear InterAnnouncementDelay

    // Check if already queued with same type
    existing = FindInQueues(vox)
    if existing != NULL and existing.type == type: return  // already queued

    // Insert into appropriate queue
    InsertIntoQueue(vox, type, priority)

    // Try to play immediately
    PlayNextQueued()
```

### 3. Queue Routing (VoxClass__InsertIntoQueue at 0x00752590)

Based on the Type and Priority values:

| Type | Name | Queue Target |
|------|------|-------------|
| 3 | QUEUED_INTERRUPT | InterruptQueue (0xb1d3c8) - highest precedence |
| 1 | QUEUE | PriorityQueue[priority] (0xb1d450 + priority * 12) |
| 3 (alt path) | CRITICAL priority | CriticalQueue (0xb1d3f0) |
| 0, 2 | STANDARD / INTERRUPT | PendingImmediate (0xb1d4b8) - only if all queues empty AND no existing pending or this has higher priority. Otherwise DISCARDED. |

**Key insight:** STANDARD and INTERRUPT types are fire-and-forget. If anything is already queued or playing, they are silently dropped. Only QUEUE and QUEUED_INTERRUPT types actually queue up.

### 4. Playback (VoxClass__PlayNextQueued at 0x00752760)

```
PlayNextQueued():
    if GamePaused: return
    if AudioSystemOff: return
    if EVAStreamPlayer == NULL: return
    if StreamIsCurrentlyPlaying: return
    if PauseCounter > 0: return

    // Wait for inter-announcement delay
    currentTime = GetPerformanceCounter()
    lastEndTime = StreamPlayer.EndTime
    if currentTime < lastEndTime + InterAnnouncementDelay: return

    // Stop any currently playing
    if CurrentlyPlaying != NULL:
        CurrentlyPlaying.PlayState = DONE
        CurrentlyPlaying = NULL

    // Dequeue next entry (priority order):
    //   1. InterruptQueue (0xb1d3c8)
    //   2. CriticalQueue (0xb1d3f0)
    //   3. PendingImmediate (0xb1d4b8) - checked with special fallback
    //   4. PriorityQueues[3..0] (highest priority number first)

    // Build filename based on faction
    switch CurrentSide:
        case 0 (Allied):  soundName = vox.AlliedSound   (offset 0x3E)
        case 1 (Russian): soundName = vox.RussianSound   (offset 0x35)
        case 2 (Yuri):    soundName = vox.YuriSound      (offset 0x2C)

    filename = soundName + ".WAV"      // e.g., "ceva048.WAV"

    // Play through EVA StreamPlayer
    success = StreamPlayer_Play(EVAStreamPlayer, filename, fromMix=1)
    if success:
        InterAnnouncementDelay = 500   // 500ms gap before next EVA
        CurrentlyPlaying = vox
        vox.PlayState = PLAYING
```

**The ".WAV" extension** is appended at 0x00844768, but the actual files in the MIX archives are .aud format. The StreamPlayer handles both WAV and AUD formats transparently.

## INI Parsing

### File Loading

EVAMD.INI is loaded during game initialization in the main init function at 0x0052ba60.
The loading sequence:
1. Open EVAMD.INI from MIX archives
2. Call `VoxClass__ReadEVAINI` (0x00753000)
3. For each entry in `[DialogList]`:
   - Parse the section name (e.g., "EVA_ConstructionComplete")
   - Allocate a VoxClass (0x54 bytes)
   - Copy the section name to offset 0x00
   - Set defaults: Priority=1 (NORMAL), Type=0 (STANDARD), PlayState=2 (DONE)
   - Volume is set to 1.0 inside VoxClass__ReadINI, not in the constructor
   - Call `VoxClass__ReadINI` (0x00752db0) to read per-section data
   - Add to VoxArray

### Per-Section INI Keys

| Key | Offset | Type | Values | Default |
|-----|--------|------|--------|---------|
| `Volume` | 0x28 | float | 0.0 - 1.0 | 1.0 |
| `Type` | 0x4C | enum | STANDARD=0, QUEUE=1, INTERRUPT=2, QUEUED_INTERRUPT=3 | 0 (STANDARD) |
| `Priority` | 0x48 | enum | LOW=0, NORMAL=1, IMPORTANT=2, CRITICAL=3 | 1 (NORMAL) |
| `Yuri` | 0x2C | char[9] | Sound filename stem | "" |
| `Russian` | 0x35 | char[9] | Sound filename stem | "" |
| `Allied` | 0x3E | char[9] | Sound filename stem | "" |

**String addresses for INI keys:**
- "Volume" = 0x00846568
- "Type" = 0x00824314
- "Priority" = 0x0084301c
- "Yuri" = 0x00846798
- "Russian" = 0x00846790
- "Allied" = 0x00846788

**Type string comparisons (in order at 0x00752e31-0x00752e8f):**
1. "QUEUE" (0x008467cc) -> Type = 1
2. "STANDARD" (0x008467c0) -> Type = 0
3. "INTERRUPT" (0x00816120) -> Type = 2
4. "QUEUED_INTERRUPT" (0x008467ac) -> Type = 3

**Priority string comparisons (in order at 0x00752ed4-0x00752f32):**
1. "LOW" (0x008161dc) -> Priority = 0
2. "NORMAL" (0x008161d4) -> Priority = 1
3. "IMPORTANT" (0x008467a0) -> Priority = 2
4. "CRITICAL" (0x008161c0) -> Priority = 3

**Default when not specified in INI:** Priority = 1 (NORMAL), Type = 0 (STANDARD).
Most EVA events in evamd.ini explicitly set Priority=LOW and Type=QUEUE, overriding the defaults.

### Faction Side Resolution

`VoxClass__SetSide` (0x007534e0) is called from `InitSideMixFiles` (0x00534fa0) at game start.
The side index determines which sound file column to use:

| Side | Value | Sound Field | Example |
|------|-------|-------------|---------|
| Allied | 0 | offset 0x3E | ceva048 |
| Russian | 1 | offset 0x35 | csof048 |
| Yuri | 2 | offset 0x2C | cyur048 |
| Default (-1) | -> 0 | offset 0x3E | (Allied) |

## Rate Limiting and Suppression

### 1. Inter-Announcement Delay (500ms)

After each EVA plays, a 500-unit delay is set (`DAT_00b1d4d0 = 0x1F4`). The next EVA
will not play until `currentTime >= lastPlayEndTime + 500`. This prevents rapid-fire
announcements.

### 2. Duplicate Suppression

In `VoxClass__QueueVoice`, before inserting:
```
existing = FindInQueues(vox)
if existing != NULL and existing.type == type: return  // skip duplicate
```
This prevents the same EVA event from being queued multiple times with the same type.

### 3. Same-As-Current Check

```
if vox == CurrentlyPlaying: return
```
An EVA that is currently playing will not be re-queued.

### 4. STANDARD/INTERRUPT Discard Policy

STANDARD (type 0) and INTERRUPT (type 2) events are NOT queued in a linked list.
They only go to the `PendingImmediate` slot (0xb1d4b8), and ONLY if:
- InterruptQueue is empty AND
- CriticalQueue is empty AND
- Either PendingImmediate is NULL, or this entry has higher priority

Otherwise, the event is **silently discarded**. This means most non-QUEUE events are
fire-and-forget: if anything else is happening, they disappear.

### 5. Radar Event Rate Limiting (for "Base Under Attack")

`HouseClass__BaseUnderAttack` (0x004f93e0) calls `CreateRadarEvent` (0x0065fa70) before
playing the EVA. CreateRadarEvent has distance-based rate limiting:

```
for each existing radar event of same type:
    distance = sqrt((x2-x1)^2 + (y2-y1)^2)
    if distance < threshold:
        return 0  // suppress - too close to existing event
return 1  // allow
```

The radar event config table at 0x007f09a4 defines thresholds per event type (typically
0xC8 = 200 cells minimum distance). This prevents "base under attack" spam when multiple
units attack in the same area.

### 6. Suspend/Resume (Nested Counter)

`VoxClass__SuspendEVA` (0x00753570): increments `SuspendCounter` at 0xb1d3d8
`VoxClass__ResumeEVA` (0x00753580): decrements `SuspendCounter`

When `SuspendCounter > 0`, `QueueVoice` returns immediately without queuing.

Called by:
- `RadarClass__PlayRadarMovie` - suspends during radar movies
- `FUN_0053b460` - suspends during queued event processing

### 7. Pause/Unpause (Nested Counter)

`VoxClass__PauseEVA` (0x007535b0): increments `PauseCounter` at 0xb1d428, also stops StreamPlayer
`VoxClass__UnpauseEVA` (0x00753620): decrements `PauseCounter`, resumes StreamPlayer

When `PauseCounter > 0`, `PlayNextQueued` returns immediately (freezes playback but keeps queue).

### 8. INTERRUPT Type Override

When a Type=2 (INTERRUPT) event fires and something is currently playing:
```
// Flush ALL queues
// Stop current playback immediately
// Clear inter-announcement delay
// Then queue the new event
```
This is for critical announcements that must play NOW (e.g., mission-critical events).

## Audio Architecture

### Two Separate StreamPlayers

The game creates TWO independent audio stream channels:

1. **EVA StreamPlayer** (`0x00b1d4cc`, created in VoiceSystem__Init at 0x00752290)
   - Buffer size: 3000 bytes
   - Used exclusively for EVA announcements
   - Plays .WAV/.AUD files from MIX archives (e.g., "ceva048.WAV")

2. **Speech StreamPlayer** (`0x00b1d4d8`, created in SpeechSystem__Init at 0x00752ad0)
   - Buffer size: 3000 bytes
   - Used for multiplayer taunts
   - Plays from "taunts/" directory (e.g., "taunts/tauam01.wav")

Both are separate from the regular SFX system (which uses DirectSound secondary buffers
via Audio::PlaySoundEffect). The StreamPlayers use streaming playback with double-buffering
for WAV/AUD data.

### StreamPlayer Internals

`FUN_00407b60` (called from PlayNextQueued) is the StreamPlayer file-play function:
1. Opens the sound file (first as RawFile, falls back to CCFileClass for MIX lookup)
2. Parses WAV header to get format info
3. Creates DirectSound secondary buffers (double-buffered)
4. Begins streaming playback with CriticalSection synchronization
5. Returns 1 on success, 0 on failure

## Complete EVA Event List

From evamd.ini [DialogList] section - 120 active events (plus ~200+ mission-specific dialog entries):

### Superweapon Events
| Event | Trigger |
|-------|---------|
| EVA_NuclearSiloDetected | Enemy nuclear silo construction complete |
| EVA_NuclearMissileLaunched | Nuclear missile super fired |
| EVA_NuclearMissileReady | Player's nuclear missile charged |
| EVA_IronCurtainDetected | Enemy iron curtain construction complete |
| EVA_IronCurtainActivated | Iron curtain super fired |
| EVA_IronCurtainReady | Player's iron curtain charged |
| EVA_ChronosphereDetected | Enemy chronosphere construction complete |
| EVA_ChronosphereActivated | Chronosphere super fired |
| EVA_ChronosphereReady | Player's chronosphere charged |
| EVA_WeatherDeviceReady | Weather device charged |
| EVA_LightningStormCreated | Lightning storm super fired |
| EVA_LightningStormReady | Player's lightning storm charged |
| EVA_PsychicDominatorDetected | Enemy psychic dominator complete |
| EVA_PsychicDominatorActivated | Psychic dominator super fired |
| EVA_PsychicDominatorReady | Player's psychic dominator charged |
| EVA_GeneticMutatorDetected | Enemy genetic mutator complete |
| EVA_GeneticMutatorActivated | Genetic mutator super fired |
| EVA_GeneticMutatorReady | Player's genetic mutator charged |
| EVA_ForceShieldReady | Force shield charged |
| EVA_PsychicRevealReady | Psychic reveal charged |
| EVA_SpyPlaneReady | Spy plane ready |
| EVA_ReinforcementsReady | Paratroop reinforcements ready |

### Production Events
| Event | Trigger |
|-------|---------|
| EVA_ConstructionComplete | Building finished (StripClass__AI) |
| EVA_NewConstructionOptions | New build options available (FUN_006a87f0) |
| EVA_Building | Player clicks build button (SelectClass__Action) |
| EVA_Training | Player clicks train button (SelectClass__Action) |
| EVA_UnitReady | Unit production complete (HouseClass__Place_Production) |
| EVA_OnHold | Production paused |
| EVA_Canceled | Production canceled |
| EVA_UnableToComply | Cannot build (SelectClass__Action) |
| EVA_CannotDeployHere | Deploy blocked (UnitClass__Deploy) |
| EVA_SelectTarget | Super weapon target select mode |
| EVA_InsufficientFunds | Not enough money (HouseClass__Update, BuildingClass__MissionRepairAndProduce) |

### Combat Events
| Event | Trigger |
|-------|---------|
| EVA_OurBaseIsUnderAttack | Player buildings attacked (HouseClass__BaseUnderAttack + RadarEvent rate limit) |
| EVA_OurAllyIsUnderAttack | Allied player buildings attacked |
| EVA_OreMinerUnderAttack | Harvester attacked |
| EVA_UnitLost | Unit destroyed (FootClass__Evaluate_Target_Threat area) |

### Building Events
| Event | Trigger |
|-------|---------|
| EVA_BuildingOnLine | Building powered up (BuildingClass__GoOnline) |
| EVA_BuildingOffLine | Building powered down (BuildingClass__GoOffline) |
| EVA_LowPower | Power drops below demand (HouseClass__Update) |
| EVA_StructureSold | Building sold |
| EVA_Repairing | Building repair started |
| EVA_PrimaryBuildingSelected | Primary building selected |
| EVA_NewRallyPointEstablished | Rally point set (BuildingClass__SetRallyPoint) |
| EVA_StructureGarrisoned | Building garrisoned (BuildingClass__AddGarrisonOccupant) |
| EVA_StructureAbandoned | Garrison cleared (BuildingClass__CheckAutoSellOrCivilian) |

### Infiltration Events
| Event | Trigger |
|-------|---------|
| EVA_BuildingInfiltrated | Spy enters building (BuildingClass__OnSpyInfiltrate) |
| EVA_BuildingInfCashStolen | Spy steals cash |
| EVA_BuildingInfRadarSabotaged | Spy sabotages radar |
| EVA_BuildingInfTechStolen | Spy steals technology |
| EVA_TechnologyStolen | Tech stolen notification |
| EVA_CashStolen | Cash stolen notification |
| EVA_PowerSabotaged | Power sabotaged notification |
| EVA_RadarSabotaged | Radar sabotaged notification |
| EVA_EnemyBasePoweredDown | Enemy base powered down by spy |
| EVA_NewTechnologyAcquired | New tech acquired |

### Capture Events
| Event | Trigger |
|-------|---------|
| EVA_BuildingCaptured | Building captured by engineer (BuildingClass__ChangeOwner) |
| EVA_TechBuildingCaptured | Tech building captured |
| EVA_TechBuildingLost | Tech building lost |

### Alliance Events
| Event | Trigger |
|-------|---------|
| EVA_AllianceFormed | Alliance established (HouseClass__MakeAlly) |
| EVA_AllianceBroken | Alliance broken (HouseClass__BreakAlliance) |
| EVA_AllianceRequested | Alliance request received |
| EVA_RequestingAlliance | Requesting alliance |
| EVA_EnemyAllianceFormed | Enemy alliance formed |

### Victory/Defeat Events
| Event | Trigger |
|-------|---------|
| EVA_MissionAccomplished | Mission victory |
| EVA_MissionFailed | Mission failure |
| EVA_YouAreVictorious | Multiplayer victory (HouseClass__Flag_To_Win) |
| EVA_YouHaveLost | Multiplayer defeat (HouseClass__Flag_To_Lose) |
| EVA_PlayerDefeated | Another player defeated (HouseClass__MPlayer_Defeated) |
| EVA_BattleControlTerminated | Game ended |

### Upgrade Events
| Event | Trigger |
|-------|---------|
| EVA_UnitPromoted | Unit promoted (TechnoClass__AI_Update via SuperClass area) |
| EVA_UnitFirePowerUpgraded | Firepower upgrade crate |
| EVA_UnitArmorUpgraded | Armor upgrade crate |
| EVA_UnitSpeedUpgraded | Speed upgrade crate |
| EVA_UnitRepaired | Unit repaired |
| EVA_UnitSold | Unit sold |

### Misc Events
| Event | Trigger |
|-------|---------|
| EVA_BridgeRepaired | Bridge repaired |
| EVA_BeaconDetected | Beacon detected |
| EVA_BeaconPlaced | Beacon placed (DisplayClass__BandBox_LeftUp) |
| EVA_IncomingTransmission | Incoming transmission |
| EVA_RobotTanksOffline | Robot control center lost |
| EVA_RobotTanksBackOnline | Robot control center regained |
| EVA_BattlefieldControlOnline | Game start |

## Trigger Site Summary

All 37 callers of `VoxClass__PlayEVA` (0x00752700):

| Caller | Address | EVA Events |
|--------|---------|------------|
| BuildingClass__OnConstructionComplete | 0x00445f80 | Superweapon detected events (via jump table) |
| BuildingClass__ChangeOwner | 0x00448260 | EVA_BuildingCaptured |
| BuildingClass__MissionRepairAndProduce | 0x0044b780 | EVA_InsufficientFunds |
| BuildingClass__GoOnline | 0x00452260 | EVA_BuildingOnLine |
| BuildingClass__GoOffline | 0x00452360 | EVA_BuildingOffLine |
| BuildingClass__OnSpyInfiltrate | 0x004571e0 | Multiple infiltration events |
| BuildingClass__CheckAutoSellOrCivilian | 0x00458200 | EVA_StructureAbandoned |
| BuildingClass__ReadFromINI | 0x0044f820 | Various |
| BuildingClass__SetRallyPoint | 0x00443860 | EVA_NewRallyPointEstablished |
| BuildingClass__AddGarrisonOccupant | 0x00522910 | EVA_StructureGarrisoned |
| DisplayClass__BandBox_LeftUp | 0x004ab9b0 | EVA_BeaconPlaced/Detected |
| HouseClass__BaseUnderAttack | 0x004f93e0 | EVA_OurBaseIsUnderAttack, EVA_OurAllyIsUnderAttack |
| HouseClass__Update | 0x004f8440 | EVA_InsufficientFunds, EVA_LowPower |
| HouseClass__MakeAlly | 0x004f9b70 | Alliance events |
| HouseClass__BreakAlliance | 0x004f9f90 | EVA_AllianceBroken |
| HouseClass__Place_Production | 0x004fb0e0 | EVA_UnitReady |
| HouseClass__Flag_To_Win | 0x004fc9e0 | EVA_YouAreVictorious |
| HouseClass__Flag_To_Lose | 0x004fcbd0 | EVA_YouHaveLost |
| HouseClass__MPlayer_Defeated | 0x004fc0b0 | EVA_PlayerDefeated |
| HouseClass__Removed_From_Game | 0x005025f0 | Various |
| InfantryClass__Mission_Enter | 0x005196a0 | Infiltration events |
| StripClass__AI | 0x006a8b30 | EVA_ConstructionComplete |
| FUN_006a87f0 (StripClass__AddEntry) | 0x006a87f0 | EVA_NewConstructionOptions |
| SelectClass__Action | 0x006aad00 | EVA_Building, EVA_Training, EVA_UnableToComply |
| SidebarClass__AddCameo | 0x006a6300 | Various |
| SuperClass__Launch | 0x006cc390 | Superweapon activated events |
| TechnoClass__AI_Update | 0x006f9e50 | EVA_UnitPromoted |
| TemporalClass__InitiateWarp | 0x0071af20 | Various |
| UnitClass__Deploy | 0x007393c0 | EVA_CannotDeployHere |
| FUN_00430ba0 | 0x00430ba0 | Various |
| FUN_0050e010 | 0x0050e010 | Various |
| FUN_0050e0e0 | 0x0050e0e0 | Various |
| FUN_0053a6c0 | 0x0053a6c0 | Various |
| FUN_00686570 | 0x00686570 | Various |
| FUN_006cbca0 | 0x006cbca0 | Superweapon ready events |
| FUN_006cc080 | 0x006cc080 | Superweapon ready events |

## Implementation Notes for Rust Engine

### Key Design Points

1. **VoxClass array is the central registry.** Load evamd.ini [DialogList] to build a
   `Vec<VoxEntry>`. Each entry holds the event name, volume, 3 faction sound names,
   type, and priority.

2. **Dispatch is by string name.** `play_eva("EVA_ConstructionComplete")` does a linear
   scan (case-insensitive) of the VoxClass array. Consider using a `HashMap<String, usize>`
   for O(1) lookup in the Rust implementation.

3. **Queue system has 4 priority tiers + 2 special queues.** Use a struct with:
   - `interrupt_queue: VecDeque<QueueEntry>` (highest priority)
   - `critical_queue: VecDeque<QueueEntry>`
   - `priority_queues: [VecDeque<QueueEntry>; 4]`
   - `pending_immediate: Option<QueueEntry>`

4. **Playback is sequential** with a 500ms gap between announcements. Use rodio's
   Sink or a dedicated audio thread.

5. **Faction selection** maps side index to sound name field. Set once at game start
   based on player's faction.

6. **The ".WAV" extension is misleading.** The actual files in the MIX archives are
   typically .aud format (IMA ADPCM compressed). The engine's StreamPlayer handles
   both formats. In our engine, use the existing .aud parser.

7. **Rate limiting for "base under attack"** is done at the call site via RadarEvent,
   NOT in the EVA system itself. The EVA system only has duplicate suppression and
   the inter-announcement delay.

8. **Suspend/Pause are separate mechanisms:**
   - Suspend: blocks new events from being queued (e.g., during movies)
   - Pause: freezes playback but preserves queue (e.g., game pause)

9. **STANDARD type events are mostly dropped.** The original engine rarely uses
   STANDARD type - most events default to QUEUE. STANDARD events only play if
   nothing else is happening, making them unreliable for important announcements.
