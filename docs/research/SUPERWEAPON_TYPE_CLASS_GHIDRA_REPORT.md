# SuperWeaponTypeClass & SuperClass -- Ghidra Research Report

## Overview

SuperWeaponTypeClass is the type definition for super weapons (Nuke, Iron Curtain, etc.).
SuperClass is the per-house runtime instance that tracks charging state, timers, and firing.

**Inheritance:** AbstractClass -> AbstractTypeClass -> SuperWeaponTypeClass
(NOT ObjectTypeClass -- SuperWeaponTypeClass inherits directly from AbstractTypeClass)

**Class size:** 0x100 (256 bytes), confirmed from RTTI size getter at 0x006CE900.

---

## 1. SuperWeaponTypeClass Struct Layout

### Inherited from AbstractClass (0x00-0x23)

| Offset | Size | Type         | Field              | Notes                           |
|--------|------|--------------|--------------------|---------------------------------|
| 0x00   | 4    | ptr          | vtable_primary     |                                 |
| 0x04   | 4    | ptr          | vtable_secondary_4 | INoticeSink                     |
| 0x08   | 4    | ptr          | vtable_secondary_8 | INoticeSource                   |
| 0x0C   | 4    | ptr          | vtable_secondary_C |                                 |
| 0x10   | 4    | int          | UniqueID           | -1 = unassigned                 |
| 0x14   | 1    | byte/flags   | AbstractFlags      | masked with 0xF8 in ctor        |
| 0x18   | 4    | int          | field_18           | 0                               |
| 0x1C   | 4    | int          | field_1C           | 0                               |
| 0x20   | 1    | bool         | field_20           | false                           |

### Inherited from AbstractTypeClass (0x24-0x97)

| Offset | Size | Type         | Field              | Notes                           |
|--------|------|--------------|--------------------|---------------------------------|
| 0x24   | 25   | char[25]     | ID                 | INI section name                |
| 0x3D   | 32   | char[32]     | UINameLabel        | CSF string label (e.g. "Name:INTCHSD")|
| 0x5D   | 3    | padding      |                    |                                 |
| 0x60   | 4    | wchar_t*     | UIName             | Resolved localized string ptr   |
| 0x64   | 49   | char[49]     | Name               | Display name from INI `Name=`   |
| 0x95   | 3    | padding      |                    |                                 |
| 0x98   | 4    | int          | ArrayIndex         | Index in global array           |

### SuperWeaponTypeClass-specific (0x9C-0xFF)

| Offset | Size | Type         | Field              | Default  | INI Key             | Notes |
|--------|------|--------------|--------------------|---------:|---------------------|-------|
| 0x9C   | 4    | ptr          | WeaponType         | 0        | WeaponType          | WeaponTypeClass* (fixup in Load) |
| 0xA0   | 4    | int          | Unknown_A0         | -1       | (none)              | See Q1 analysis below |
| 0xA4   | 4    | int          | Unknown_A4         | -1       | (none)              | See Q1 analysis below |
| 0xA8   | 4    | int          | Unknown_A8         | -1       | (none)              | See Q1 analysis below |
| 0xAC   | 4    | int          | Unknown_AC         | -1       | (none)              | See Q1 analysis below |
| 0xB0   | 4    | int          | RechargeTime       | 0x1194 (4500) | RechargeTime   | In frames (900 * minutes) |
| 0xB4   | 4    | int          | Type               | -1       | Type                | Enum index, see Type table |
| 0xB8   | 4    | ptr          | SidebarCameo       | 0        | (loaded from SHP)   | SHPStruct* loaded from mix |
| 0xBC   | 4    | int          | Action             | -1       | Action              | Enum index, see Action table |
| 0xC0   | 4    | int          | SpecialSound       | -1       | SpecialSound        | VocClass index |
| 0xC4   | 4    | int          | StartSound         | -1       | StartSound          | VocClass index |
| 0xC8   | 4    | ptr          | AuxBuilding        | 0        | AuxBuilding         | BuildingTypeClass* (fixup in Load) |
| 0xCC   | 25   | char[25]     | SidebarImage       | (from ID)| SidebarImage        | Filename for cameo SHP |
| 0xE5   | 1    | bool         | UseChargeDrain     | false    | UseChargeDrain      | |
| 0xE6   | 1    | bool         | IsPowered          | true     | IsPowered           | |
| 0xE7   | 1    | bool         | DisableableFromShell| false   | DisableableFromShell| |
| 0xE8   | 4    | int          | FlashSidebarTabFrames| 0      | FlashSidebarTabFrames| |
| 0xEC   | 1    | bool         | AIDefendAgainst    | false    | AIDefendAgainst     | |
| 0xED   | 1    | bool         | PreClick           | false    | PreClick            | |
| 0xEE   | 1    | bool         | PostClick          | false    | PostClick           | |
| 0xF0   | 4    | int          | PreDependent       | -1       | PreDependent        | SuperWeapon Type enum index |
| 0xF4   | 1    | bool         | ShowTimer          | false    | ShowTimer           | |
| 0xF5   | 1    | bool         | ManualControl      | false    | ManualControl       | |
| 0xF8   | 4    | float        | Range              | 0.0      | Range               | Float, read via ReadDouble |
| 0xFC   | 4    | int          | LineMultiplier     | 0        | LineMultiplier      | |

---

## 2. Question Resolutions

### Q1: Offsets 0xA0-0xAC -- Four int fields defaulting to -1

**Resolution: Unused/vestigial fields. Confidence: ~85%.**

Evidence:
- Set to -1 in constructor (0x006CE5B0), never written by ReadINI (0x006CEA20)
- NOT read by AbstractTypeClass::ReadINI (0x00410A60) either
- NOT read by any known SuperClass method (AI_Charging, AI_Ready, Launch, etc.)
- ARE included in ComputeChecksum (0x006CE910) via CRCEngine::AddData
- ARE saved/loaded as part of the binary blob (Save at 0x006CE8D0 delegates to parent)
- No byte pattern match for writes to these offsets was found

The fact that they're checksummed suggests they were intended to be gameplay-relevant.
The -1 default matches the pattern of sound/voice indices (like SpecialSound, StartSound).
They may have been planned EVA voice indices (e.g., ReadyVoice, DetectedVoice,
ActivatedVoice, DeactivatedVoice) that were never wired up -- the actual EVA voices
for super weapons are hardcoded per-Type in the switch statements inside
SuperClass::AI_Charging and SuperClass::AI_Ready.

**Recommendation:** Initialize to -1, include in checksum, but don't expose INI keys.
They will always be -1 in standard YR.

### Q2: RechargeTime conversion factor

**Resolution: Multiplication factor is exactly 900.0f. Verified from binary.**

Assembly at 0x006CED5C-0x006CED8B:
```asm
006ced5c: PUSH 0x0                        ; default = 0.0 (double)
006ced5e: PUSH 0x0
006ced60: PUSH 0x842634                    ; "RechargeTime"
006ced65: PUSH ESI                         ; section name
006ced66: MOV ECX,EBX
006ced68: CALL CCINIClass::ReadDouble      ; returns double in ST(0)
006ced73: FCOM double ptr [0x007e2800]     ; compare with 0.0
006ced79: FNSTSW AX
006ced7b: TEST AH,0x40                     ; check if equal to 0.0
006ced7e: JNZ 0x006ced93                   ; if zero, skip (keep default)
006ced80: FMUL float ptr [0x007f4100]      ; multiply by 900.0f
006ced86: CALL Math__ftol                  ; convert to int
006ced8b: MOV [EBP + 0xb0],EAX            ; store as RechargeTime
```

- Constant at 0x007E2800 = 0x0000000000000000 (double 0.0, used for comparison)
- Constant at 0x007F4100 = 0x44610000 (float 900.0)
- Formula: `RechargeTime_frames = (int)(RechargeTime_minutes * 900.0f)`
- 900 = 60 seconds * 15 fps
- Default 4500 frames = 5.0 minutes (set in constructor as 0x1194)

### Q3: Second constructor at 0x006CE800

**Resolution: This is the Load/Deserialize constructor (from save games).**

Function: `SuperWeaponTypeClass::Load` at 0x006CE800

Behavior:
1. Calls `AbstractClass::Load(param_1, param_2)` to deserialize the binary blob
2. If successful and `param_1 != NULL`:
   - Calls `AbstractTypeClass::Constructor` to re-initialize base (restores default state)
   - Clears `param_1[0x2E]` (offset 0xB8 = SidebarCameo) to 0
   - Re-applies vtable pointers
3. Calls `FUN_006CF240` (pointer fixup registration) on:
   - `param_1 + 0x27` (offset 0x9C = WeaponType pointer)
   - `param_1 + 0x32` (offset 0xC8 = AuxBuilding pointer)
4. Rebuilds SidebarImage filename from `param_1 + 0x33` (offset 0xCC)
5. Reloads SHP from MIX into `param_1[0x2E]` (offset 0xB8)

### Q4: Action enum table at 0x7E4C50

**Resolution: Complete table decoded. 73 entries (indices 0-72).**

Table address: 0x007E4C50 through 0x007E4D74 (exclusive)
Read by: `CCINIClass__ReadAction` at 0x00474EE0 (loop: `ppuVar2 < 0x7E4D74`)

| Index | Name             | Index | Name              |
|------:|------------------|------:|-------------------|
|     0 | None             |    37 | IronCurtain       |
|     1 | Move             |    38 | LightningStorm    |
|     2 | NoMove           |    39 | ChronoSphere      |
|     3 | Enter            |    40 | ChronoWarp        |
|     4 | Self             |    41 | ParaDrop          |
|     5 | Attack           |    42 | PlaceWaypoint     |
|     6 | Harvest          |    43 | TibSunBug         |
|     7 | Select           |    44 | EnterWaypointMode |
|     8 | ToggleSelect     |    45 | AreaAttack        |
|     9 | Capture          |    46 | IvanBomb          |
|    10 | Eaten            |    47 | NoIvanBomb        |
|    11 | Repair           |    48 | Detonate          |
|    12 | Sell             |    49 | DetonateAll       |
|    13 | SellUnit         |    50 | DisarmBomb        |
|    14 | NoSell           |    51 | SelectNode        |
|    15 | NoRepair         |    52 | AttackSupport     |
|    16 | Sabotage         |    53 | PlaceBeacon       |
|    17 | Tote             |    54 | SelectBeacon      |
|    18 | DontUse2         |    55 | AttackMoveNav     |
|    19 | DontUse3         |    56 | AttackMoveTar     |
|    20 | Nuke             |    57 | Demolish          |
|    21 | DontUse4         |    58 | AmerParaDrop      |
|    22 | DontUse5         |    59 | PsychicDominator  |
|    23 | DontUse6         |    60 | SpyPlane          |
|    24 | DontUse7         |    61 | GeneticConverter  |
|    25 | DontUse8         |    62 | ForceShield       |
|    26 | GuardArea        |    63 | NoForceShield     |
|    27 | Heal             |    64 | Airstrike         |
|    28 | Damage           |    65 | PsychicReveal     |
|    29 | GRepair          |    66 | (next chunk)      |
|    30 | NoDeploy         |    67 | (next chunk)      |
|    31 | NoEnter          |    68 | GeneticConverter  |
|    32 | NoGRepair        |    69 | ForceShield       |
|    33 | TogglePower      |    70 | NoForceShield     |
|    34 | NoTogglePower    |    71 | Airstrike         |
|    35 | EnterTunnel      |    72 | PsychicReveal     |
|    36 | NoEnterTunnel    |       |                   |

Note: Indices 66-72 appear to have duplicate names with earlier entries. Some entries
(DontUse2-8, TibSunBug) are TS legacy / placeholders.

### Q5: SuperClass CDTimerClass at offsets 0x30-0x3B

**Resolution: CDTimerClass is a 12-byte struct with 3 int fields.**

CDTimerClass layout (verified from GetTimeRemaining at 0x00426630 and Start at 0x0046B640):

| Offset | Size | Type | Field       | Notes |
|--------|------|------|-------------|-------|
| +0x00  | 4    | int  | StartFrame  | Frame counter when timer started; -1 = inactive |
| +0x04  | 4    | int  | Reserved    | Not written by Start(), not read by GetTimeRemaining() |
| +0x08  | 4    | int  | Duration    | Total duration in frames |

**GetTimeRemaining logic:**
```c
if (StartFrame == -1) return Duration;  // inactive, return raw value
elapsed = CurrentFrame - StartFrame;
if (elapsed < Duration) return Duration - elapsed;
return 0;  // expired
```

**Start logic:**
```c
StartFrame = g_CurrentFrameCounter;
Duration = param;
// NOTE: Reserved field [+0x04] is NOT written
```

In SuperClass, the CDTimerClass is embedded at offset 0x30:
- SuperClass+0x30 = CDTimerClass.StartFrame
- SuperClass+0x34 = CDTimerClass.Reserved (written as unknown value from stack in AI_Charging)
- SuperClass+0x38 = CDTimerClass.Duration

### Q6: Global SuperWeaponTypeClass array

**Resolution: DynamicVectorClass at 0x00A8E328.**

| Global Address | Field              | Notes |
|----------------|--------------------|-------|
| 0x00A8E328     | DVC start          | VectorClass base |
| 0x00A8E330     | Allocator vtable   | Memory allocator pointer |
| 0x00A8E334     | Data pointer       | Pointer to array of SuperWeaponTypeClass* |
| 0x00A8E338     | Capacity           | Allocated slots |
| 0x00A8E33D     | IsAllocated flag   | Whether buffer was heap-allocated |
| 0x00A8E340     | Count              | Current number of entries |
| 0x00A8E344     | GrowthStep         | Growth increment |

Confirmed from constructor (0x006CE5B0):
```c
param_1[0x26] = DAT_00a8e340;  // ArrayIndex = Count (before increment)
index = DAT_00a8e340 * 4;
DAT_00a8e340 = DAT_00a8e340 + 1;
*(DAT_00a8e334 + index) = param_1;  // Store in array
```

FindOrAllocate (0x006CEEF0) iterates `DAT_00A8E334[0..DAT_00A8E340]` comparing
`+0x24` (ID) to find existing types, or allocates 0x100 bytes for a new one.

---

## 3. SuperWeaponType Enum Table

Table at 0x008425C0, 12 entries (loop bound: `< 0x8425F0`).

| Index | Name             | Notes |
|------:|------------------|-------|
|     0 | MultiMissile     | Nuclear missile |
|     1 | IronCurtain      | |
|     2 | LightningStorm   | |
|     3 | ChronoSphere     | |
|     4 | ChronoWarp       | Second click of Chronosphere |
|     5 | ParaDrop         | |
|     6 | AmerParaDrop     | American paradrop variant |
|     7 | PsychicDominator | YR |
|     8 | SpyPlane         | |
|     9 | GeneticConverter | YR (Genetic Mutator) |
|    10 | ForceShield      | YR |
|    11 | PsychicReveal    | YR |

---

## 4. SuperClass Struct Layout (Partial)

SuperClass inherits from AbstractClass. Total size not yet determined.

| Offset | Size | Type            | Field              | Notes |
|--------|------|-----------------|--------------------|-------|
| 0x00   | 16   | vtable ptrs     | (4 vtables)        | |
| 0x10   | 4    | int             | UniqueID           | |
| 0x14   | ...  |                 | (AbstractClass)    | |
| 0x24   | 4    | int             | CustomRechargeTime | -1 = use type default |
| 0x28   | 4    | ptr             | Type               | SuperWeaponTypeClass* |
| 0x2C   | 4    | ptr             | Owner              | HouseClass* |
| 0x30   | 12   | CDTimerClass    | RechargeTimer      | {StartFrame, Reserved, Duration} |
| 0x3C   | 4    | ptr             | field_3C           | |
| 0x48   | 4    | int             | field_48           | |
| 0x4C   | 4    | int             | field_4C           | |
| 0x50   | 4    | int             | SoundCountdown     | Decremented each tick, plays sound at 0 |
| 0x54   | ...  |                 |                    | |
| 0x60   | 1    | bool            | IsSuspendedByPlayer| Toggled by Suspend() |
| 0x62   | 4    | CellStruct      | ChronoTarget       | ChronoSphere warp target cell |
| 0x64   | 4    | int             | field_64           | |
| 0x68   | 4    | ptr             | Anim               | AnimClass* (visual effect) |
| 0x6C   | 1    | bool            | field_6C           | |
| 0x6D   | 1    | bool            | IsActive           | true if granted to house |
| 0x6E   | 1    | bool            | IsCharged          | true if one-shot charged |
| 0x6F   | 1    | bool            | IsReady            | true if ready to fire |
| 0x70   | 1    | bool            | IsSuspended        | true if suspended |
| 0x74   | 4    | int             | ReadyFrame         | Frame when became ready |
| 0x78   | 4    | int             | LastAnimStage      | Previous anim stage for change detection |
| 0x7C   | 4    | int             | ChargeDrainState   | 0=empty, 1=charging, 2=draining |

---

## 5. EVA Voices for Super Weapons (Hardcoded per Type)

The "Ready" EVA events are NOT read from INI -- they're hardcoded in a switch on Type:

| Type Index | Type Name        | Ready EVA String               |
|-----------:|------------------|---------------------------------|
|          0 | MultiMissile     | EVA_NuclearMissi(leReady)       |
|          1 | IronCurtain      | EVA_IronCurtainReady            |
|          2 | LightningStorm   | EVA_LightningStormReady         |
|          3 | ChronoSphere     | EVA_ChronosphereReady           |
|          4 | (ChronoWarp)     | (case 4 falls through to default - no EVA) |
|          5 | ParaDrop         | EVA_ReinforcementsReady         |
|          6 | AmerParaDrop     | EVA_ReinforcementsReady         |
|          7 | PsychicDominator | EVA_PsychicDominatorReady       |
|          8 | SpyPlane         | EVA_SpyPlaneReady               |
|          9 | GeneticConverter | EVA_GeneticMutatorReady         |
|         10 | ForceShield      | EVA_ForceShieldReady            |
|         11 | PsychicReveal    | EVA_PsychicRevealReady          |

Switch at 0x006CC0FC (AI_Charging) and 0x006CBDE6 (AI_Ready).

---

## 6. Key Functions Reference

| Address    | Name | Notes |
|------------|------|-------|
| 0x006CE5B0 | SuperWeaponTypeClass::Constructor | Normal constructor |
| 0x006CE800 | SuperWeaponTypeClass::Load | Save-game deserialization constructor |
| 0x006CE8D0 | SuperWeaponTypeClass::Save | Delegates to AbstractTypeClass::Save |
| 0x006CE8F0 | SuperWeaponTypeClass::GetRTTI | Returns 0x20 (32) |
| 0x006CE900 | SuperWeaponTypeClass::GetSize | Returns 0x100 (256) |
| 0x006CE910 | SuperWeaponTypeClass::ComputeChecksum | CRC of all gameplay fields |
| 0x006CEA20 | SuperWeaponTypeClass::ReadINI | Reads all INI keys |
| 0x006CEEF0 | SuperWeaponTypeClass::FindOrAllocate | Find by name or create new |
| 0x006CEF80 | SuperWeaponTypeClass::GetAction | Returns Action, special case for ForceShield |
| 0x006CEFE0 | SuperWeaponTypeClass::Destructor | |
| 0x00410A60 | AbstractTypeClass::ReadINI | Reads Name, UIName |
| 0x00474EE0 | CCINIClass::ReadAction | Reads Action enum from INI |
| 0x006CAEC0 | SuperClass::Constructor (default) | No-args |
| 0x006CAF90 | SuperClass::Constructor (full) | With Type, House params |
| 0x006CB120 | SuperClass::Destructor | |
| 0x006CB4D0 | SuperClass::Suspend | Pause/unpause timer |
| 0x006CB7B0 | SuperClass::Deactivate | Remove from house |
| 0x006CBCA0 | SuperClass::AI_Ready | Ready state tick logic |
| 0x006CBEE0 | SuperClass::AnimStage | Sidebar animation frame |
| 0x006CC080 | SuperClass::AI_Charging | Charging state tick logic |
| 0x006CC2B0 | SuperClass::NameReadiness | Readiness text for UI |
| 0x006CC390 | SuperClass::Launch | Fire the super weapon (massive switch) |
| 0x00426630 | CDTimerClass::GetTimeRemaining | |
| 0x0046B640 | CDTimerClass::Start | |

---

## 7. Global Arrays

| Address    | Contents |
|------------|----------|
| 0x00A8E328 | DynamicVectorClass\<SuperWeaponTypeClass*\> (type definitions) |
| 0x00A8E334 | -> Data pointer |
| 0x00A8E340 | -> Count |
| 0x00A83CB8 | DynamicVectorClass\<SuperClass*\> (all instances) |
| 0x00A83CBC | -> Data pointer |
| 0x00A83CC8 | -> Count |
| 0x007E4C50 | Action enum string table (73 entries, ends at 0x7E4D74) |
| 0x008425C0 | SuperWeaponType enum string table (12 entries, ends at 0x8425F0) |
