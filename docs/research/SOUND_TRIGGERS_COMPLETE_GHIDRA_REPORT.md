# Sound Triggers Complete Ghidra Report

Research date: 2026-03-23
Confidence: HIGH (verified from binary decompilation)
Binary: gamemd.exe (Yuri's Revenge)

## Part 1: Core Sound Playback Functions

### Architecture Overview

The sound system has three independent playback channels:

1. **SFX Channel** -- positional sound effects via DirectSound buffers
2. **Voice Channel** -- unit voice responses via the StreamPlayer at `0x00b1d4cc`
3. **EVA Channel** -- EVA announcements via the same StreamPlayer, queued

### Function: `VocClass__PlayAtPos` (0x00750920) -- THE MAIN SFX DISPATCHER

This is the single most important sound function. It takes a VocClass index + volume/position
and plays it through a DirectSound buffer. Nearly 100 call sites across the engine.

```
int __thiscall VocClass__PlayAtPos(int vocIndex, float volume, int loopSoundEventPtr)
```

- `vocIndex`: index into the global VocClass array (at `DAT_00b1d37c`, count at `DAT_00b1d388`)
- `volume`: 0x3f800000 = 1.0f (full volume) passed as int-reinterpreted float
- `loopSoundEventPtr`: if nonzero, treated as a looping sound handle -- stops old loop if VocClass changed

Internal flow:
1. Checks `DAT_008464ac` (audio enabled flag)
2. Validates vocIndex is in range [0, DAT_00b1d388)
3. Resolves VocClass pointer: `*(*(DAT_00b1d37c + vocIndex * 4))`
4. If loopSoundEventPtr != 0, checks if an existing sound is playing with a different VocClass -- stops it
5. Calls `SoundEvent__AllocateFromPool()` (0x00405190) to get a free SoundEvent
6. Configures volume via `VocClass__CalcVolumeAndPan` (0x00750ac0) for spatial audio
7. Returns the SoundEvent handle (or 0 on failure)

### Function: `VocClass__PlayGlobal` (0x00406670) -- NON-POSITIONAL SFX

Plays a sound without spatial positioning (used for subtitle/caption sounds only).
Only 2 call sites -- both from `CaptionClass__ProcessSubtitles` (0x005ce0c0).

### Function: `VoxClass__QueueVoice` (0x00752480) -- VOICE/EVA QUEUE

Queues a VoxClass entry (EVA speech or unit voice) into the voice playback system.
Uses the StreamPlayer at `DAT_00b1d4cc` for actual audio output.

```
void __fastcall VoxClass__QueueVoice(int voxIndex, int priority, int queueSlot)
```

- If `priority == 2`, it interrupts all current playback (clears queue, stops stream)
- Otherwise, inserts into a priority queue
- Calls `VoxClass__PlayNextQueued()` (0x00752760) to advance the queue

### Function: `VoxClass__PlayEVA` (0x00752700) -- EVA ANNOUNCEMENTS

Wrapper around QueueVoice that checks if the same EVA message is already queued.

### Function: `VoxClass__PlayNextQueued` (0x00752760) -- VOICE STREAM PLAYBACK

The actual voice playback engine:
1. Checks if the StreamPlayer is idle
2. Dequeues the next voice entry
3. Selects the .aud file based on side (offset +0x2c, +0x35, +0x3e for Allied/Soviet/Yuri)
4. Appends ".aud" extension (string at `0x00844748`)
5. Calls `FUN_00407b60` to load the .aud file into the StreamPlayer
6. Sets `DAT_00b1d4d0 = 500` (500ms minimum gap between voices)

Key globals:
- `DAT_00b1d4cc` -- StreamPlayer instance for voices
- `DAT_00b1d4c4` -- currently playing VoxClass entry
- `DAT_00b1d4c8` -- current side index (0=Allied, 1=Soviet, 2=Yuri)
- `DAT_00b1d4b0` -- total VoxClass entry count
- `DAT_00b1d4a4` -- VoxClass array pointer

### SoundEvent System (Low-level)

- `SoundEvent__AllocateFromPool` (0x00405190) -- gets free SoundEvent from pool
- `SoundEvent__StartPlayback` (0x004054a0) -- initiates DirectSound buffer playback
- `SoundEvent__Stop` (0x004052f0) -- stops a playing SoundEvent, releases buffer
- `SoundEvent__UpdateState` (0x004055c0) -- per-frame state machine (states 0-4)
- `SoundEvent__AdvancePlaylist` (0x004047b0) -- advances multi-sample playlists
- `SoundEvent__SelectNextSample` (0x00404bb0) -- random/sequential sample selection
- `SoundEvent__PreparePlayout` (0x00404700) -- prepares buffer for playback
- `SoundEvent__LoadSamples` (0x004048b0) -- loads .aud samples from mix archives
- `SoundSystem__UpdateTick` (0x004041d0) -- master per-frame update for all SoundEvents
- `SoundSystem__StopAll` (0x00404e70) -- stops all sounds
- `SoundEventPool__Init` (0x00403ed0) -- initializes the SoundEvent pool

### VocClass System (Sound Definition)

- `VocClass__ReadINI` (0x00750440) -- reads soundmd.ini entries
  - INI keys: `Sounds`, `Volume`, `VShift`, `MinVolume`, `Priority`, `Attack`, `Decay`,
    `Control`, `Limit`, `Range`, `Delay`, `FShift`
- `VocClass__AddSample` (0x004064a0) -- adds a .aud sample to a VocClass
  - Strips `$` and `#` prefixes from sample names
  - Max 32 samples per VocClass (at offset +0xb4, count at +0x134)
  - Calls `AudioIndex__FindSample` to resolve sample handles
- `VocClass__FindByName` (0x007514d0) -- looks up VocClass index by name string
- `VocClass__CalcVolumeAndPan` (0x00750ac0) -- spatial volume/panning calculation
  - Converts world coords to screen coords via `TacticalClass__CoordsToClient2`
  - Computes distance-based attenuation
  - Checks fog-of-war visibility (flag 0x800: no sound if cell not revealed)

### Audio Infrastructure

- `AudioIndex__Read` (0x004018c0) -- reads .idx audio index files
- `AudioIndex__FindSample` (0x004015c0) -- finds sample in audio index
- `AudioIndex__OpenSample` (0x004016f0) -- opens sample for streaming
- `AudioIndex__GetFormat` (0x00401640) -- gets sample format info
- `AudioSystem__Init` (0x00406b10) -- initializes DirectSound
- `AudioSystem__Shutdown` (0x00406d40) -- shuts down DirectSound
- `VoiceSystem__Init` (0x00752290) -- initializes voice system
  - Creates StreamPlayer with 3000ms buffer
  - Initializes 4 voice queue slots at `0x00b1d450`-`0x00b1d480`
- `StreamPlayer__Create` (0x00407860) -- creates streaming audio player
  - Allocates 0xf8-byte structure
  - Creates a DSoundChannel for streaming
  - Sets up callback functions for buffer fill/underrun
- `DSoundBuffer__Create` (0x00402040) -- creates DirectSound buffer
- `DSoundChannel__CreateAll` (0x00403530) -- creates all DSoundChannel instances
- `DSoundChannel__FindAvailable` (0x004035f0) -- finds free channel
- `DSoundChannel__FindLowestPriority` (0x00404e20) -- evicts lowest priority sound
- `SoundThread__Init` (0x00407550) -- initializes sound processing thread

---

## Part 2: ALL Sound Trigger Categories

### 1. Weapon Report Sounds

**INI keys:** `Report=` and `DownReport=` on `[WeaponType]` sections

**Parsed in:** `WeaponTypeClass__ReadINI` (0x00772080)
- `Report` -> offset 0xcc in WeaponTypeClass (vector of VocClass indices)
- `DownReport` -> offset 0xe8 (vector, used when target is below firer)
- These are parsed via `FUN_00478720` which handles comma-separated VocClass name lists

**Triggered in:** `TechnoClass__Fire_At` (0x006fdd50)
- Lines ~700-720: After bullet creation, reads weapon's Report count at offset 0x104
- If `Report` count == 8, selects sample by facing direction (8 directional reports)
- Otherwise uses first entry: `*(*(weaponType + 0xf8))`
- Elite override: if `(*vtable + 0x400)()` returns true, uses offset 0x110 (elite report)
- If no report and unit has `field_0x82` flag, uses offset 0x118 (garrison fire sound)
- Plays via `VocClass__PlayAtPos` with the Report VocClass index

**Bullet flight sounds:**
- `BulletTypeClass__ReadINI_Part2` (0x00428319) reads:
  - `StartSound` -> offset 0x2f8 (falls back to `Report` if no StartSound)
  - `StopSound` -> offset 0x2fc

### 2. Death Sounds

**INI key:** `DieSound=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` (0x00712170)
- `DieSound` -> read at instruction address 0x007134f2 as a VocClass index

**INI key:** `BuildingDieSound=` in `[AudioVisual]` section of rules.ini

**Parsed in:** `RulesClass__ReadAudioVisual` (0x006691e0)
- `BuildingDieSound` -> RulesClass offset 0x1ba (multiplied by 4 = byte offset 0x6e8)

**Triggered in:** `TechnoClass__ReceiveDamage` (0x00701900) -- plays DieSound when health reaches 0

### 3. Selection Voices

**INI key:** `VoiceSelect=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` at address 0x00712b36
- `VoiceSelect` -> TechnoTypeClass offset 0x414 (vector of VocClass names, parsed via FUN_00478720)

**Also:**
- `VoiceSelectEnslaved` -> offset 0x430 (when unit is mind-controlled)
- `VoiceSelectDeactivated` -> offset 0x44c (when unit is deactivated/EMP'd)

**Triggered in:** `TechnoClass__Select` (0x006fbfa0)
- Calls vtable function at offset 0x360 which dispatches voice playback
- Then calls `ObjectSelection__PlayVoice` (0x00637840) which iterates selected objects
  and calls vtable function at offset 0x14c to play their selection voice
- Voice goes through the VoxClass__QueueVoice system -> StreamPlayer (voice channel)
- **Selection voices use the voice StreamPlayer, NOT normal SFX channels**
- Previous voice IS cut off (priority == 2 in QueueVoice clears the queue)

### 4. Move Order Voices

**INI key:** `VoiceMove=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` at address 0x00712acc
- `VoiceMove` -> TechnoTypeClass offset 0x468 (vector via FUN_00478720)

**Triggered in:** `SelectClass__Action` (0x006aad00)
- When player right-clicks to issue a move order
- Calls `VocClass__PlayAtPos` with VoiceMove entries
- Multiple call sites within SelectClass__Action for different action types

### 5. Attack Order Voices

**INI key:** `VoiceAttack=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` at address 0x00712c74
- `VoiceAttack` -> TechnoTypeClass offset 0x484 (vector)

**Also:**
- `VoiceSpecialAttack` -> offset 0x4a0 (for special weapon attacks)
- `VoicePrimaryWeaponAttack` -> parsed at address 0x00712db2 area
- `VoicePrimaryEliteWeaponAttack`
- `VoiceSecondaryWeaponAttack`
- `VoiceSecondaryEliteWeaponAttack`

**Triggered in:** `SelectClass__Action` (0x006aad00) when player orders an attack

### 6. Voice Feedback / Misc Voice Responses

**INI key:** `VoiceFeedback=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` at address 0x00712db2
- `VoiceFeedback` -> TechnoTypeClass offset 0x4d8 (vector)

**Other voice keys on TechnoTypeClass:**
- `VoiceDie` -> offset 0x4bc (death scream)
- `VoiceHarvest` -> parsed with the voice block
- `VoiceCapture`
- `VoiceEnter`
- `VoiceSinking`
- `VoiceCrashing`
- `VoiceFalling`
- `VoiceDeploy`
- `VoiceUndeploy`

**All voices use the same VoxClass queue -> StreamPlayer path.**

### 7. Building Ambient / Working Sounds

**INI keys:** `AmbientSound=` and `CrushSound=` on `[ObjectType]` sections

**Parsed in:** `ObjectTypeClass__ReadINI` (0x005f92d0)
- `AmbientSound` -> ObjectTypeClass offset (looked up via VocClass__FindByName)
- `CrushSound` -> ObjectTypeClass offset (looked up via VocClass__FindByName)

**Also in TechnoTypeClass__ReadINI:**
- `WorkingSound` (0x0081ab0c) -> sound when building is active/producing
- `NotWorkingSound` (0x0081aafc) -> sound when building is idle/no power

**Triggered in:** `TechnoClass__AI_Update` (0x006f9e50) at ~line 55
- Field 0x4f0 holds the ambient VocClass index (-1 = none)
- Field 0x4dc holds the playback position coords
- Calls `VocClass__PlayAtPos(0x3f800000, &field_0x4dc)` each frame when needed
- Uses loop mechanism: passes the SoundEvent handle to detect/stop changed sounds

### 8. Movement Sounds

**INI key:** `MoveSound=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI` at address 0x00713478
- `MoveSound` -> TechnoTypeClass offset 0x700 (VocClass index via VocClass__FindByName)

**Also:**
- `EnterWaterSound` (0x008259f0) -> played when entering water
- `LeaveWaterSound` (0x008259e0) -> played when leaving water

**Triggered in:** `DriveLocomotionClass__Process_Movement` (0x004b2630) at address 0x004b2e68
- When unit starts moving and `field_0x68a` flag is set
- Reads `RulesClass + 0x700` (the MoveSound VocClass index)
- Calls `VocClass__PlayAtPos(0x3f800000, 0)` -- passing 0 as non-looping

**Also triggered in:** `ShipLocomotionClass__Process_Movement` (0x006a2330)
- Same pattern for naval units at addresses 0x006a24b8 and 0x006a3118

### 9. Deploy/Undeploy Sounds

**INI keys:** `DeploySound=`, `UndeploySound=` on `[TechnoType]` sections

**Parsed in:** `TechnoTypeClass__ReadINI`
- `DeploySound` -> address 0x00713568
- `UndeploySound` -> address 0x0071359e

**Global overrides in rules.ini [AudioVisual]:**
- `SlaveMinerDeploySound` -> RulesClass offset 0x8e * 4
- `SlaveMinerUndeploySound` -> RulesClass offset 0x8f * 4

### 10. Construction / Build Placement Sounds

**INI key:** `Construction=` in `[AudioVisual]` section of rules.ini

**Parsed in:** `RulesClass__ReadAudioVisual`
- `Construction` -> RulesClass offset 0x1b2 * 4 = 0x6c8

**Also:**
- `BuildupSound` (0x0081ab58) -> per-type buildup sound
- `CreateSound` -> TechnoTypeClass (at 0x00712e84)
- `CreateUnitSound` -> RulesClass offset 0x5e * 4
- `CreateInfantrySound` -> RulesClass offset 0x5f * 4
- `CreateAircraftSound` -> RulesClass offset 0x60 * 4
- `BuildingSlam` -> RulesClass offset 0x1bb * 4 (building placement slam)
- `BuildingDrop` -> RulesClass offset 0x1f0 * 4

**Triggered in:** `HouseClass__Place_Production` (call at 0x004fb314) and
`FactoryClass__StartProduction` (call at 0x004c9d5f)

### 11. Sell Sounds

**INI key:** `SellSound=` in `[AudioVisual]` section of rules.ini

**Parsed in:** `RulesClass__ReadAudioVisual`
- `SellSound` -> RulesClass offset 0x1a9 * 4 = 0x6a4

**Triggered in:** `BuildingClass__CheckAutoSellOrCivilian` (0x00458100) at 0x004582a9

### 12. Repair Sounds

**INI key:** `BuildingRepairedSound=` in `[AudioVisual]` section of rules.ini

**Parsed in:** `RulesClass__ReadAudioVisual`
- `BuildingRepairedSound` -> RulesClass offset 0x71 * 4

**Also:** `BuildingDamageSound` -> RulesClass offset 0x1c5 * 4

### 13. Crush Sounds

**INI key:** `CrushSound=` on `[ObjectType]` sections

**Parsed in:** `ObjectTypeClass__ReadINI` at address 0x005f93a0

**Triggered in:** `DriveLocomotionClass__Process_Movement` at address 0x004b3ac9
- Plays when a vehicle crushes infantry

### 14. Explosion / Warhead Sounds

Warhead detonation sounds are handled through the animation system.
When a warhead creates an explosion animation (via `AnimClass__Constructor`),
the AnimTypeClass has its own sound entry. The weapon's `Anim=` key in
WeaponTypeClass (offset 0xf4, vector of AnimTypeClass pointers) determines
which explosion anim plays, and that anim's sound plays automatically.

### 15. UI / GUI Sounds

All parsed in `RulesClass__ReadAudioVisual`:

| INI Key | RulesClass Offset (index) |
|---------|--------------------------|
| GUIMainButtonSound | 0x62 |
| GUIBuildSound | 0x63 (99) |
| GUITabSound | 0x64 (100) |
| GUIOpenSound | 0x65 |
| GUICloseSound | 0x66 |
| GUIMoveOutSound | 0x67 |
| GUIMoveInSound | 0x68 |
| GUIComboOpenSound | 0x69 |
| GUIComboCloseSound | 0x6a |
| GUICheckboxSound | 0x6b |
| GenericClick | 0x1c3 |
| GenericBeep | 0x1c4 |
| ScoldSound | 0x1c0 |

**Also sidebar-specific:**
- `HighlightSound` (0x008242b0) -> sidebar button hover
- `SelectSound` (0x008242fc) -> sidebar button click

**Triggered in:** `SidebarClass__Action` (0x006a7820) at addresses 0x006a78bf, 0x006a78f1, 0x006a7a7d
and `CommandBar_Dispatch` (0x006d0700) at 0x006d0860, 0x006d091a

### 16. Radar Event Sounds

**INI keys in [AudioVisual]:**
- `RadarOn` -> RulesClass offset 0x1bc
- `RadarOff` -> RulesClass offset 0x1bd
- `MovieOn` -> RulesClass offset 0x1be
- `MovieOff` -> RulesClass offset 0x1bf
- `BaseUnderAttackSound` -> RulesClass offset 0x61

**Triggered in:** `RadarClass__ActivateDeactivate` (0x00656b30) at 0x00656c32, 0x00656c7d
and `RadarClass__PlayRadarMovie` at 0x00657966

### 17. Crate Pickup Sounds

All parsed in `RulesClass__ReadAudioVisual`:

| INI Key | RulesClass Offset (index) |
|---------|--------------------------|
| CrateMoneySound | 0x79 |
| CrateRevealSound | 0x7a |
| CrateFireSound | 0x7b |
| CrateArmourSound | 0x7c |
| CrateSpeedSound | 0x7d |
| CrateUnitSound | 0x7e |
| CratePromoteSound | 0x7f |
| HealCrateSound | (separate) |

**Triggered in:** Various crate pickup handler functions, all via `VocClass__PlayAtPos`.

### 18. Chronoshift / Teleport Sounds

**INI keys in [AudioVisual]:**
- `ChronoInSound` -> RulesClass offset 0x86
- `ChronoOutSound` -> RulesClass offset 0x87
- `DefaultChronoSound` -> RulesClass offset 0x74
- `LetsDoTheTimeWarpOutAgain` -> RulesClass offset 0xa1
- `LetsDoTheTimeWarpInAgain` -> RulesClass offset 0xa2

### 19. Superweapon Sounds

**INI keys in [AudioVisual]:**

| INI Key | RulesClass Offset (index) |
|---------|--------------------------|
| PsychicDominatorActivateSound | 0x93 |
| GeneticMutatorActivateSound | 0x94 |
| PsychicRevealActivateSound | 0x95 |
| MasterMindOverloadDeathSound | 0x96 |

**SuperWeaponTypeClass also reads (at 0x00772080 -- mislabeled as WeaponTypeClass__ReadINI):**
- `SpecialSound` -> SuperWeaponTypeClass offset 0xc0
- `StartSound` -> SuperWeaponTypeClass offset 0xc4

### 20. Miscellaneous Sounds (from RulesClass__ReadAudioVisual)

| INI Key | RulesClass Offset (index) | Purpose |
|---------|--------------------------|---------|
| DigSound | 0x5d | Tunnel locomotion |
| CloakSound | 0x1a8 | Cloaking/uncloaking |
| ScoreAnimSound | 0x6c | Score screen |
| CheerSound | 0x72 | Victory cheer |
| ImpactWaterSound | 0x80 | Projectile hits water |
| ImpactLandSound | 0x81 | Projectile hits land |
| SinkingSound | 0x82 | Ship sinking |
| BombTickingSound | 0x83 | Crazy Ivan bomb timer |
| BombAttachSound | 0x84 | Crazy Ivan bomb placement |
| YuriMindControlSound | 0x85 | Mind control beam |
| AddPlanningModeCommandSound | 0x76 | Planning mode waypoint |
| ExecutePlanSound | 0x77 | Execute plan |
| PlaceBeaconSound | 0x73 | Beacon placement |
| BuildingGarrisonedSound | 0x6f | Infantry enters building |
| BuildingAbandonedSound | 0x70 | Infantry leaves building |
| UpgradeVeteranSound | 0x8a | Unit promotes to veteran |
| UpgradeEliteSound | 0x8b | Unit promotes to elite |
| VoiceIFVRepair | 0x8c | IFV in repair mode |
| SlavesFreeSound | 0x8d | Slave miner destroyed |
| BunkerWallsUpSound | 0x90 | Battle bunker opens |
| BunkerWallsDownSound | 0x91 | Battle bunker closes |
| RepairBridgeSound | 0x92 | Bridge repair |
| AirstrikeAbortSound | 0x97 | Airstrike cancelled |
| AirstrikeAttackVoice | 0x98 | Airstrike ordered |
| MindClearedSound | 0x99 | Mind control broken |
| EnterGrinderSound | 0x9a | Unit enters grinder |
| LeaveGrinderSound | 0x9b | Unit leaves grinder |
| EnterBioReactorSound | 0x9c | Unit enters bio reactor |
| LeaveBioReactorSound | 0x9d | Unit leaves bio reactor |
| ActivateSound | 0x9e | Building activated |
| DeactivateSound | 0x9f | Building deactivated |
| IFVTransformSound | 0x6d | IFV weapon change |
| PsychicSensorDetectSound | 0x6e | Psychic sensor detection |
| SpySatActivationSound | 0x88 | Spy satellite on |
| SpySatDeactivationSound | 0x89 | Spy satellite off |
| DiskLaserChargeUp | 0xa3 | Disk laser charge |
| TeslaCharge | 0x1c1 | Tesla coil charging |
| TeslaZap | 0x1c2 | Tesla coil firing |
| ChuteSound | 0x1c7 | Parachute deploy |
| StopSound | 0x1c8 | Unit stop order |
| GuardSound | 0x1c9 | Unit guard order |
| ScatterSound | 0x1ca | Unit scatter |
| GateUp | 0x101 | Gate opening |
| GateDown | 0x102 | Gate closing |
| ShellButtonSlideSound | (separate) | Main menu button |
| LightningSounds | (separate) | Lightning storm |
| StormSound | (separate) | Storm ambient |
| IceCrackSounds | (separate) | Ice cracking |

### 21. Per-Unit Sounds (from TechnoTypeClass__ReadINI)

| INI Key | Purpose |
|---------|---------|
| MoveSound | Plays while moving |
| DieSound | Plays on death |
| DeploySound | Plays on deploy |
| UndeploySound | Plays on undeploy |
| CreateSound | Plays on creation |
| EnterTransportSound | Plays when entering transport |
| LeaveTransportSound | Plays when leaving transport |
| TurretRotateSound | Plays during turret rotation |
| DamageSound | Plays when taking damage |
| CrashingSound | Plays when aircraft crashes |
| AuxSound1 | Auxiliary sound 1 |
| AuxSound2 | Auxiliary sound 2 |

### 22. Map Trigger Sounds

**Trigger Action 0x13** (Play Sound Effect):
- Calls `VocClass__PlayAtPos(soundIndex, 1.0f, 0)` in `TriggerAction__Execute` (0x006dd8b0)

**Trigger Action 0x15** (Play Speech/EVA):
- Calls `VoxClass__QueueVoice(speechIndex, -1)` in `TriggerAction__Execute`

### 23. Credit Tick Sound

**INI key:** `CreditTicks=` in `[AudioVisual]`

**Parsed in:** `RulesClass__ReadAudioVisual` -> RulesClass offset 0x1b3-0x1b9 (vector)

**Triggered in:** `CreditsClass__Draw` (0x004a2480) at 0x004a2533 -- plays as credits tick up/down

### 24. Network/Multiplayer Sounds

From `RulesClass__ReadAudioVisual`:
- `GameClosed` -> 0x1aa
- `IncomingMessage` -> 0x1ab
- `MessageCharTyped` -> 0x1b1
- `SystemError` -> 0x1ac
- `OptionsChanged` -> 0x1ad
- `GameForming` -> 0x1ae
- `PlayerLeft` -> 0x1af
- `PlayerJoined` -> 0x1b0

---

## Part 3: Voice System Details -- Selection Voice Path

### When you click a unit:

1. `TechnoClass__Select` (0x006fbfa0) is called
2. It calls `ObjectClass__Select` (base class)
3. If the unit is player-controlled, calls vtable function at offset 0x360 -- this is
   the **response voice playback** function (type-specific override)
4. Then calls `ObjectSelection__PlayVoice` (0x00637840) which:
   - Gets the current selection group
   - Iterates all selected objects
   - For each, calls vtable function at offset 0x14c -- the unit's **play voice** function

### Voice channel vs SFX channel:

**Unit voices (VoiceSelect, VoiceMove, VoiceAttack, etc.) go through VoxClass__QueueVoice.**
This means they use the **StreamPlayer voice channel** at `DAT_00b1d4cc`, NOT normal SFX channels.

The flow is:
```
User clicks unit
  -> TechnoClass__Select
    -> vtable[0x360]()  (voice response handler)
      -> VoxClass__QueueVoice(vocIndex, priority)
        -> VoxClass__PlayNextQueued()
          -> FUN_00407b60()  (load .aud into StreamPlayer)
            -> StreamPlayer plays via dedicated DirectSound buffer
```

### Does it cut off the previous voice?

**YES.** When `VoxClass__QueueVoice` is called with `priority == 2`:
1. It stops the current StreamPlayer playback
2. Clears all queued voices
3. Resets `DAT_00b1d4c4` (current playing entry) to 0
4. Inserts the new voice at the head of the queue
5. Calls `VoxClass__PlayNextQueued` to start it immediately

For lower priorities, it checks if the same VoxClass is already queued and skips duplicates.

### EVA vs Unit Voices -- Same Channel

Both EVA announcements and unit voices use the **same StreamPlayer** at `DAT_00b1d4cc`.
This means:
- A unit voice will interrupt a playing EVA message if it has higher priority
- EVA messages queue behind each other
- Only one voice/EVA can play at a time

---

## Part 4: Looping Sounds

### How looping works:

The `VocClass__PlayAtPos` function has a third parameter that enables loop tracking:

```c
int __thiscall VocClass__PlayAtPos(int vocIndex, float volume, int loopSoundEventPtr)
```

When `loopSoundEventPtr != 0`:
1. It calls `FUN_00406130()` to check if a sound is currently playing at that handle
2. If playing, checks `FUN_00406310()` to get the current VocClass -- if it matches, does nothing (sound continues)
3. If the VocClass changed, calls `SoundEvent__Stop()` to stop the old sound
4. Starts the new sound and stores the SoundEvent handle via `FUN_004060f0(vocClassPtr)`

### Movement sound looping:

In `DriveLocomotionClass__Process_Movement`:
```
// At 0x004b2e68:
VocClass__PlayAtPos(
    *(RulesClass + 0x700),   // MoveSound VocClass index
    0x3f800000,               // 1.0f volume
    0                         // NOT looped -- plays once per movement step
);
```

Movement sounds are actually NOT true loops. They fire once when the unit completes a
movement step (entering a new cell). The `field_0x68a` flag is set when the unit enters
a new cell, and cleared after the sound plays.

### Ambient sound looping:

In `TechnoClass__AI_Update`:
```
// At 0x006f9ef0:
if (field_0x4f0 != -1) {
    iVar7 = FUN_00406130();  // check if already playing
    if (iVar7 == 0) {
        field_0x4f4 = field_0x4f0;  // remember current VocClass
        VocClass__PlayAtPos(0x3f800000, &field_0x4dc);
    } else if (field_0x4f4 != field_0x4f0) {
        // VocClass changed - handled at LAB_006f9f0d
    }
    field_0x4f0 = -1;  // reset for next frame
}
```

Ambient sounds are re-triggered every frame if the VocClass index at 0x4f0 is set.
The sim layer writes the VocClass index each tick; the render/update layer plays it.
The `FUN_00406130()` check prevents duplicate overlapping playback.

### DirectSound loop mechanism:

At the SoundEvent level, looping is controlled by the VocClass control flags:
- Flag 0x1 in the VocClass control field -> the sound loops (DSBPLAY_LOOPING)
- Flag 0x2 -> random selection from samples
- Flag 0x40 -> global (no spatial attenuation)

The actual DirectSound loop flag is set in `SoundEvent__StartPlayback` (0x004054a0)
and `SoundEvent__UpdateState` (0x004055c0). When a looping SoundEvent finishes its
buffer, it calls `SoundEvent__AdvancePlaylist` which either:
- Repeats (flag 0x1 set, loop count not exceeded)
- Advances to next sample in playlist
- Stops playback

---

## Part 5: TechnoTypeClass Voice/Sound Field Map

All offsets relative to TechnoTypeClass base (param_1 = `int*`, so multiply index by 4):

### Weapon references (parsed from weapons block):
- 0x898: Primary weapon type pointer
- 0x8b4: Secondary weapon type pointer
- 0xa94: Elite primary weapon type pointer
- 0xab0: Elite secondary weapon type pointer

### Voice vectors (each is a DynamicVectorClass of VocClass indices):
| Offset | INI Key |
|--------|---------|
| 0x414 | VoiceSelect |
| 0x430 | VoiceSelectEnslaved |
| 0x44c | VoiceSelectDeactivated |
| 0x468 | VoiceMove |
| 0x484 | VoiceAttack |
| 0x4a0 | VoiceSpecialAttack |
| 0x4bc | VoiceDie |
| 0x4d8 | VoiceFeedback |

### Sound indices (single VocClass index):
| Offset | INI Key |
|--------|---------|
| 0x52c+ | AuxSound1 |
| ... | AuxSound2 |
| ... | CreateSound |
| ... | DamageSound |
| ... | CrashingSound |
| 0x700 | MoveSound |
| ... | DieSound |
| ... | DeploySound |
| ... | UndeploySound |
| ... | EnterTransportSound |
| ... | LeaveTransportSound |
| ... | TurretRotateSound |

---

## Functions Labeled in This Session

| Address | Old Name | New Name |
|---------|----------|----------|
| 0x00750920 | CreditUpDown_Sound | VocClass__PlayAtPos |
| 0x005ce0c0 | FUN_005ce0c0 | CaptionClass__ProcessSubtitles |
| 0x006dd8b0 | FUN_006dd8b0 | TriggerAction__Execute |
| 0x00428319 | FUN_00428319 | BulletTypeClass__ReadINI_Part2 |
| 0x00637840 | FUN_00637840 | ObjectSelection__PlayVoice |
| 0x007265c0 | FUN_007265c0 | TriggerActionEntry__PlayVoiceForObjects |
| 0x007264c0 | FUN_007264c0 | TriggerActionEntry__EvaluateConditions |
| 0x004a71a0 | FUN_004a71a0 | BulletAnimTracker__Register |

### Previously labeled (confirmed in this session):
| Address | Name |
|---------|------|
| 0x00406670 | VocClass__PlayGlobal |
| 0x004064a0 | VocClass__AddSample |
| 0x00750ac0 | VocClass__CalcVolumeAndPan |
| 0x007514d0 | VocClass__FindByName |
| 0x00750440 | VocClass__ReadINI |
| 0x00752290 | VoiceSystem__Init |
| 0x00752480 | VoxClass__QueueVoice |
| 0x00752700 | VoxClass__PlayEVA |
| 0x00752760 | VoxClass__PlayNextQueued |
| 0x00407860 | StreamPlayer__Create |
| 0x004054a0 | SoundEvent__StartPlayback |
| 0x004052f0 | SoundEvent__Stop |
| 0x004055c0 | SoundEvent__UpdateState |
| 0x004047b0 | SoundEvent__AdvancePlaylist |
| 0x00404bb0 | SoundEvent__SelectNextSample |
| 0x00404700 | SoundEvent__PreparePlayout |
| 0x004048b0 | SoundEvent__LoadSamples |
| 0x00405190 | SoundEvent__AllocateFromPool |
| 0x004041d0 | SoundSystem__UpdateTick |
| 0x00404e70 | SoundSystem__StopAll |
| 0x00403ed0 | SoundEventPool__Init |
| 0x006691e0 | RulesClass__ReadAudioVisual |
| 0x00772080 | WeaponTypeClass__ReadINI (actually SuperWeaponTypeClass) |
| 0x006fdd50 | TechnoClass__Fire_At |
| 0x006fbfa0 | TechnoClass__Select |
| 0x006f9e50 | TechnoClass__AI_Update |
| 0x006aad00 | SelectClass__Action |
| 0x004b2630 | DriveLocomotionClass__Process_Movement |
| 0x006a2330 | ShipLocomotionClass__Process_Movement |

---

## Summary for Implementation

### Three playback paths to implement:

1. **VocClass__PlayAtPos** -- for ALL positional SFX:
   - Takes VocClass index + volume + optional loop handle
   - Resolves to .aud sample(s) via VocClass sample array
   - Applies spatial volume/pan via CalcVolumeAndPan
   - Uses SoundEvent pool for concurrent playback

2. **VoxClass__QueueVoice** -- for ALL voice responses:
   - Takes VoxClass index + priority + queue slot
   - Routes through StreamPlayer (single voice channel)
   - Priority 2 = interrupt everything
   - Side-dependent .aud file selection (Allied/Soviet/Yuri)

3. **VoxClass__PlayEVA** -- for EVA announcements:
   - Wrapper around QueueVoice with duplicate checking
   - Uses same StreamPlayer as unit voices

### Key data flow:
```
soundmd.ini -> VocClass__ReadINI -> VocClass array [DAT_00b1d37c]
                                         |
rules.ini -> RulesClass__ReadAudioVisual -> VocClass indices in RulesClass fields
           -> TechnoTypeClass__ReadINI   -> VocClass indices in type fields
           -> WeaponTypeClass__ReadINI   -> Report vectors in weapon fields
                                         |
                                    VocClass__PlayAtPos (positional SFX)
                                    VoxClass__QueueVoice (voice/EVA)
```
