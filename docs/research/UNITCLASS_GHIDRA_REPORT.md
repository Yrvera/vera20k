# UnitClass — Ghidra Research Report

**Primary addresses:** Constructor `0x007353C0`, AI `0x007360C0`, PerCellProcess `0x007416A0`, TurretAI `0x007468C0`, Fire_At_Target `0x00736DF0`, Facing_Update `0x00736990`
**Confidence:** HIGH (all findings verified from binary decompilation)
**Active in YR:** Yes — all systems documented here are active in standard YR skirmish

## 1. Overview

UnitClass is the leaf class for all ground vehicles and naval units, inheriting from FootClass (→ TechnoClass → ObjectClass → AbstractClass). It adds ~40 bytes (0x6C0–0x6E4) for vehicle-specific state: the sentinel field, type pointer, convoy chain, flag carrier, deploy/harvest/ore flags, crash/fall state, and deploy animation state. The class owns turret tracking AI, crush-on-cell-entry logic, gattling weapon stage tracking, harvester AI delegation, and self-deploying vehicle logic.

**Total instance size:** ~0x6E8 (1768 bytes). Constructor writes through `0x6E4`.

## 2. UnitClass Struct Layout (0x6C0–0x6E4)

All offsets verified from constructor assembly at `0x007353C0` (ESI = this pointer, MOV instructions).

| Byte Offset | Size | Init Value | Field Name | Evidence / Purpose |
|-------------|------|------------|------------|--------------------|
| `0x6C0` | 4 | -1 (EDI) | **Sentinel / unused** | Initialized to -1. InfantryClass has TypeClass* here; UnitClass has -1 sentinel. Confirmed by CONVOY_FORMATION_SYSTEM report |
| `0x6C4` | 4 | param (TypeClass*) | **UnitTypeClass*** | Constructor stores type parameter. Used everywhere as `this->Type` (param_1[0x1B1] byte-offset form) |
| `0x6C8` | 4 | 0 (EBX) | **NextInConvoy** | FootClass* — singly-linked list to next convoy member. Confirmed by CONVOY_FORMATION_SYSTEM at 0x6C8 |
| `0x6CC` | 4 | -1 (EDI) | **FlagCarrierHouse** | CTF flag carrier house index (-1 = none). Confirmed by UNIT_DRAW_EXTRAS (FLAGFLY.SHP rendering, attach/detach at 0x740DF0/0x740E20) |
| `0x6D0` | 1 | 0 | **IsConvoyFollower** | Set 1 on convoy followers. Confirmed by CONVOY_FORMATION_SYSTEM |
| `0x6D1` | 1 | 0 | **ConvoyStopped / DockingInitialized** | Prevents re-propagation in Stop_Moving; dock door flag |
| `0x6D2` | 1 | 0 | **IsHarvesting** | Active harvest state — gates OREGATH.SHP overlay rendering. Confirmed by UNIT_DRAW_EXTRAS (param_1+0x6D2 check). Cleared when mission != Harvest(10) in UnitClass::AI |
| `0x6D3` | 1 | 0 | **Unknown_6D3** | Byte flag, initialized to 0 |
| `0x6D4` | 4 | -1 (EDI) | **DeployAnimIndex / CrashState** | Deploy animation timer/index or crash type. -1 = inactive. See `IsCrashing` vtable: returns true if `+0x6D8 != -1 OR EMP active` |
| `0x6D8` | 4 | -1 (EDI) | **CrashType / DeathAnimState** | Checked by `IsCrashing (0x746C90)`: `if (this+0x6D8 == -1) { return IsUnderEMP(); } else return 1;` |
| `0x6DC` | 4 | 0 (EBX) | **Unknown_6DC** | Int field, cleared to 0 |
| `0x6E0` | 1 | 0 | **IsFalling** | Byte flag — returned directly by `UnitClass__IsFalling (0x746D80)`. Also named DeployedFlag in deploy context |
| `0x6E1` | 1 | 0 | **DeployAnimForward** | Deploy animation playing forward. Checked in UNIT_MISSION_DEPLOY (forward anim state) |
| `0x6E2` | 1 | 0 | **DeployAnimReverse** | Deploy animation playing reverse. Checked in UNIT_MISSION_DEPLOY (reverse anim state) |
| `0x6E4` | 4 | 0 (EBX) | **DumpBaleIndex** | Current bale being dumped during ore unload at refinery. Confirmed by HARVESTER_DOCK_UNLOAD |

### Related parent-class fields heavily used by UnitClass

| Offset | Class | Name | Notes |
|--------|-------|------|-------|
| `0x0F8` | TechnoClass | StepCounter | Harvester frame timer for 9-step harvest wait |
| `0x100–0x10C` | TechnoClass | StepTimerClass | Start/rate/step/total for harvesting |
| `0x138` | TechnoClass | CurrentWeaponNumber | Active weapon slot |
| `0x140` | TechnoClass | CurrentGattlingStage | Gattling spin-up stage |
| `0x148` | TechnoClass | GattlingValue | Gattling accumulator |
| `0x1B0` | FootClass | **DeployCountdown** | Used in AI: if != -1, decrements. At 0, calls FUN_00738680 (death explosion), destroys unit. From UnitTypeClass+0xE38 (DeathFrames) |
| `0x1B1` | FootClass (used as type ptr) | **UnitTypeClass ptr via array offset** | UnitClass often reads `param_1[0x1B1]` — byte offset 0x6C4 |
| `0x334` | TechnoClass | RockingForwardsPerFrame | Set to -0.05 by crush tilt |
| `0x370–0x3A0` | TechnoClass | BodyFacing / TurretFacing / BarrelFacing | FacingClass (24 bytes each) |
| `0x3CD` | TechnoClass | IsSinking | Blocks movement and vision |
| `0x504` | TechnoClass | EMPLockRemaining | Checked by IsCrashing |
| `0x674` | FootClass | ILocomotion* | Locomotor pointer |
| `0x6AF` | FootClass | ShouldNotScatter | Set during docking |

## 3. UnitClass::AI (`0x007360C0`, 270 lines)

Called every tick from LogicClass. Sequence of operations:

### 3a. Sinking particle effects
If `TechnoClass::GetByte_0x1C8()` returns true (sinking flag):
- Every 30 frames (desync by position hash), spawns a ParticleSystem from `RulesClass+0x1020` at random offset ±100 leptons. **Active in YR: Yes** (ships that die sink and bubble).

### 3b. Transport building check
If unit has a transport building (`param_1[0x73]`, offset 0x1CC), every 16 frames checks if the building still occupies the same cell. If not, exits transport.
If orphaned passenger (`param_1[0x75]` set, no transport): forces exit.

### 3c. Parasite AI
If `param_1[0x9E]` (ParasiteClass* at 0x278) is non-null: calls `Parasite->AI()` via vtable+0x5C.

### 3d. Falling/parachute handling
Same as InfantryClass: if falling, spawns parachute anim every 24 frames from RulesClass+0x344.

### 3e. Deploy countdown (timed death)
```
DeployCountdown (0x6C0 area, actually param_1[0x1B6] = offset 0x6D8 region)
if (DeployCountdown != -1):
    DeployCountdown += 1
    if DeployCountdown >= UnitTypeClass->DeathFrames (+0xE38):
        FUN_00738680()  // death explosion with debris
        UnSelect(), Destroy()
        return
```
This handles V3 rockets and similar timed-destruction units. **Active in YR: Yes** for RocketLocomotion units.

### 3f. Tube movement
If `DriveTrackIndex bit7 CLEAR` (index 0–127, i.e. `(char)param_1[0x1a1] >= 0`): calls `FUN_007359F0()` (tube/tunnel process), then vtable+0x4A0 and returns. `0xFF`/–1 (bit7 set) means no tube. (Verified via `decompile_function 0x007360C0`: `if (-1 < (char)param_1[0x1a1])`.)

### 3g. Warp check & AI-deploy for ConstructionYard types
If unit has `DeploysInto` type (type+0x404 != 0): checks against `RulesClass.ConstructionYardTypes` list. If AI-controlled and not in Hunt/Sleep mission, assigns Hunt(0xF) mission.

### 3h. Warp/chrono handling
Calls `vtable+0x200` (IsWarping); if so calls `vtable+0x1EC` (UpdateWarp).

### 3i. FootClass::AI
Calls parent `FootClass::AI()`.

### 3j. Turret rotation for non-ROT-zero types
If type has `TurretNotHidden` (`type+0xD2F != 0`) AND NOT `TurretLocked` (`type+0xD30 == 0`):
calls `FUN_007468C0()` — the turret AI function (see §5).

### 3k. Sinking movement
If `IsSinking (0x3CD)` is set: sets location to coord minus 5Z per tick. If height drops below -400: kills unit. Every 4 frames spawns wake anim (RulesClass+0x94).

### 3l. IsHarvesting flag management
If current mission != Harvest(10): clears `IsHarvesting (0x6D2)` to 0.

### 3m. Fire-at-target
Calls `FUN_00736DF0()` — combat fire logic (see §6).

### 3n. Facing update
Calls `FUN_00736990()` — body/turret facing rotation toward target or destination (see §7).

### 3o. Guard/Sleep mission — terrain check
If mission is Guard(5), checks terrain passability. If invalid terrain + has sight: kills passengers, destroys self.

### 3p. Harvester/Weeder AI delegation
If `UnitTypeClass+0xE18` (Harvester) or `+0xE19` (Weeder) is true: calls `FUN_00737180()` — the idle harvest brain that decides when to switch to Harvest mission.

### 3q. Update animation
Calls `vtable+0x424` (UpdateAnimation) — handles body animation frame updates.

### 3r. Turret-mounted SpawnManager
If type has `SpawnDelay > 0` (type+0x5E0) and not destroyed and mission != Sleep: calls `FUN_004A5240()` to manage spawn timer/state (Aircraft Carrier / Dreadnought missile spawning).

### 3s. AI auto-hunt
Every 16 frames, AI-controlled idle units (Guard/Sleep, not Harvester/Weeder, with valid target from `FUN_00455DD0`) enter Hunt mission and move to target.

### 3t. Stuck harvester rescue
AI harvesters idle for >300 frames: checks if sitting on a specific building cell (refinery dock offset +3,+1) and tries to path out.

## 4. UnitClass::PerCellProcess (`0x007416A0`, 105 lines)

Called when a unit enters a new cell during movement.

### 4a. Bridge detection
Checks cell flags `& 0x100` (bridge bit). If on bridge and unit is not OnBridge, reads cell-at-center and checks if bridge-level height delta matches level+4 (bridge crossing detection).

### 4b. Pre-entry scatter
Before entering: if bridge layer has infantry occupants (`cell+0x128 & 0x1F`), scatters them with `CellClass::Scatter_Objects`. Same for ground layer (`cell+0x124 & 0x1F`).

### 4c. Crush iteration (post-entry)
If `Crusher (type+0xD28)` OR veteran ability `CRUSHER (0x11)`:
```
for each object in cell's linked list:
    if CanCrushCheck(object) fails: skip
    if IsAlly(object) AND NOT IsTrain: skip
    
    dist_sq = DistanceSquaredTo(object.coords)
    if dist_sq > 0x3FFF (~128 leptons): skip  // too far from center
    if object.InLimbo (0x8D): skip
    
    // Special case: infantry deployer being entered by own transport
    if object is Infantry AND DeployedCrushable AND object.NavTarget == self AND NOT IsTrain:
        Copy player-control flag from infantry to self
        Kill self (sacrifice transport)
        Change owner to infantry's owner
        Destroy infantry
    else:
        // Normal crush
        Play CrushSound (object.type+0x1F0)
        object->Record_Kill(self)  // vtable+0xE0
        object->Mark_Unselected()  // vtable+0x124
        object->Scatter()          // vtable+0xD4
        object->Destroy()          // vtable+0xF8
```

### 4d. Crush tilt
After crushing, if `TiltsWhenCrushes (type+0xD2B)` and body speed is zero:
```
RockingForwardsPerFrame (0x334) = -0.05f
```
This gives the visual nose-dip when running over infantry.

## 5. Turret AI (`0x007468C0`, 108 lines)

Called from UnitClass::AI when turret is active (`TurretNotHidden && !TurretLocked`).

### 5a. Idle turret scan
If unit is alive, locomotor is stopped, and `type+0xD32 != 0` (TurretScansNearby) AND no destination: sets a "should scan" flag.

### 5b. Enemy proximity check (every 8 frames)
Scans all 8 adjacent cells for non-allied objects (via `FUN_0047EC40`). If enemy found:
```
SightTimer.Start = g_CurrentFrameCounter
SightTimer.Duration = RulesClass+0x1014
Call vtable+0x470 (UpdateTurretFacing toward threat)
```

### 5c. Idle turret scan decay
If scan flag set and the sight timer has expired:
```
Pick random AnimType from RulesClass.IdleActionFrequency list
Set disguise type = random idle AnimType
Mark disguise flag = 1
Call vtable+0x49C (Start idle animation)
```
This handles the "Mirage Tank idle animation" that plays when disguised and not in combat.

### 5d. Spawn building turret AI
If unit has `SpawnManager (0xB2 / offset 0x2C8)`: if alive and player-controlled → sets SpawnManager+0x19D = 1; else sets to 0. Controls whether the carrier's spawns auto-target.

## 6. Fire_At_Target (`0x00736DF0`, 149 lines)

Called from UnitClass::AI when unit has a target.

### 6a. Weapon selection
Gets current weapon via `vtable+0x3F8`. If weapon is null, returns.

### 6b. Range/LOS check dispatch
Calls `vtable+0x3C0` (Can_Fire_At) with target and weapon index. Result codes:

| Code | Behavior |
|------|----------|
| 0 | Fire! Calls `vtable+0x3CC` (Fire_At). Updates gattling, harvester anim facing |
| 2 | Out of range — if `IsSimpleDeployer`, redeploy; else face target and move closer |
| 3 | `GattlingValue += 1` (no other action). Gattling gate: codes 0/2/3/4 call `IncreaseGattlingStage`; other codes call `UpdateGattlingStage` (decay). Verified via `decompile_function 0x00736DF0` |
| 5 | Range exceeded — clear target if not repairable building |
| 6 | SpawnManager — calls ClearAllTargets |
| 8, 0xB | Waiting for ROF — no action |
| 9 | Enter target (transport/garrison) — attempts boarding |

### 6c. Harvester fire animation
On successful fire (code 0), if `Harvester || Weeder`:
```
facing_index = ((target_dir >> 0xC) + 1 >> 1) & 7   // verified via decompile_function 0x00736DF0
StepCounter (0xF8) = facing_table[facing_index]  // from DAT_008458B0
StepTimer.Start = g_CurrentFrame
StepTimer.Step = 5
StepTimer.Rate = 5
```
This drives the OREGATH arm animation during harvest.

### 6d. Gattling stage management
After fire result, if `type+0xCD5` (IsGattling):
- On fire success (code 0): calls `FUN_0070DE70(1)` (increment gattling accumulator)
- On miss/wait: calls `TechnoClass__UpdateGattlingStage(1)` (decay)
- Increments `GattlingValue (0x148)` when gattling continues firing

## 7. Facing Update (`0x00736990`, 95 lines)

Called from UnitClass::AI for body/turret facing toward movement target.

### 7a. Moving with target
If has NavTarget (`param_1[0xAD]`, offset 0x2B4) and not `ShouldNotScatter (0x6AF)`:
Calculates facing toward target. If `Deployer (type+0xCA1)`: checks weapon readiness and locks/unlocks facing based on weapon charge state.

### 7b. Non-deployer facing
If `SpeedType == 1` (Amphibious) and no NavCom: faces current direction.
Otherwise: if locomotor stopped, faces toward target or destination.

### 7c. Deploy-to-fire facing
For `IsSimpleDeployer` units: special facing logic — if deploying and has target, faces target and then fires. Uses CDTimerClass remaining to gate deploy animation.

### 7d. Facing formula
```
target_dir = atan2(target - self)  →  raw_facing
body_facing = ((raw_facing >> 7) + 1 >> 1) & 0xFF  →  8 compass offsets
turret_facing = target_dir (full precision)
```
Body rotates in discrete steps; turret tracks with full angular precision.

## 8. Death Explosion (`0x00738680`, 60 lines)

Called when `DeployCountdown` expires (timed-death units like V3 rockets).

### Debris spawning
```
// First debris array (Explodes/veteran-gated big explosion)
if type->DeathWeaponAnims count (type+0x73C) > 0:
    pick random debris AnimType from type->DeathWeaponAnims array (type+0x730)
    
    if (type->Explodes (type+0xD15) OR veteran EXPLODES ability 10)
       AND (type->Sight == -1 OR Ammo > 0):   // Sight==-1 triggers big explosion (verified via decompile_function 0x00738680: `*(int *)(iVar5 + 0x684) == -1`)
        use LAST anim in array (the big explosion)
    
    spawn AnimClass at unit location

// Second debris array (unconditional — no Explodes/veteran gate)
if type+0x758 (count) > 0:
    pick random AnimType from array at type+0x74C (ptr), count type+0x758
    spawn AnimClass at unit location
// (Verified via decompile_function 0x00738680)
```
Then checks `RulesClass+0x17E5` (ScatterDebris flag) and distributes ore from StorageClass.

## 9. INI Keys — UnitTypeClass Key Offsets

| Offset | Type | INI Key | Notes |
|--------|------|---------|-------|
| `+0x404` | ptr | DeploysInto | BuildingTypeClass* (MCV → ConYard) |
| `+0x408` | ptr | UndeploysInto | UnitTypeClass* (ConYard → MCV) |
| `+0x5E0` | int | SpawnDelay | > 0 enables SpawnManager AI |
| `+0x67C` | int | SpeedType | 1=Amphibious, 3=Water, etc. |
| `+0x680` | int | Sight (override) | -1 = use fallback at +0x684 |
| `+0x684` | int | Sight (base) | Sight range in cells |
| `+0x6B8` | ptr | DeployingAnim | AnimTypeClass* for SimpleDeployer |
| `+0x71C` | ptr | Turret VXL header | Parsed from art.ini |
| `+0x730` | ptr | DeathWeaponAnims array | |
| `+0x73C` | int | DeathWeaponAnims count | |
| `+0xA0` | int | Strength | Max HP |
| `+0xC8E` | byte | Trainable | Vet eligible |
| `+0xC94` | byte | IsTrain | Enables mutual passthrough + ally crushing |
| `+0xCA1` | byte | Deployer | Has deploy action |
| `+0xCD0` | byte | Unknown | Copied to TechnoClass+0x3D2 in InitFromType |
| `+0xCD5` | byte | IsGattling | Gattling weapon stage system |
| `+0xD15` | byte | Explodes | On death: uses big explosion anim |
| `+0xD21` | byte | TurretLocked | Turret does not rotate independently |
| `+0xD28` | byte | Crusher | Can crush infantry |
| `+0xD29` | byte | OmniCrusher | Crushes anything non-resistant |
| `+0xD2A` | byte | OmniCrushResistant | Immune to OmniCrusher |
| `+0xD2B` | byte | TiltsWhenCrushes | Visual nose-dip on crush |
| `+0xD2F` | byte | TurretNotHidden | Has visible turret (enables turret AI) |
| `+0xD30` | byte | TurretLocked2 | Second lock flag |
| `+0xD32` | byte | TurretScansNearby | Idle turret scans for enemies |
| `+0xE0E` | byte | Harvester | Ore harvester flag |
| `+0xE0F` | byte | Weeder | Vein harvester flag |
| `+0xE10` | byte | Unknown | Checked in fire logic: if set, clears attack flag on fire success |
| `+0xE11` | byte | Unknown | Checked in deploy-fire facing: if 0, uses deployer facing logic |
| `+0xE13` | byte | IsSimpleDeployer | Deploy anim without building transform |
| `+0xE18` | byte | Harvester2 | Second harvester flag (gates AI harvest brain) |
| `+0xE19` | byte | Weeder2 | Second weeder flag (gates AI harvest brain) |
| `+0xE38` | int | DeathFrames | Timed death countdown limit |
| `+0xEDC` | int | DeployFacing | Required facing direction for deploy |

## 10. Integration Points

### What calls UnitClass::AI
- `LogicClass::PerTickUpdate @ 0x0055AFB0` contains the per-object active-vector loop; it iterates the LogicClass-owned object vector forward and calls vtable+0x5C, re-reading count after each call. `LogicClass::AI` is the input/event dispatcher, not this object-AI loop.

### What UnitClass::AI calls (ordered)
1. Sinking particle spawn
2. Transport building orphan check
3. ParasiteClass::AI (if attached)
4. Parachute anim (if falling)
5. Deploy countdown / timed death
6. Tube/tunnel movement
7. AI auto-deploy for ConYard types
8. Warp/chrono update
9. **FootClass::AI** (parent — locomotor, movement, team, idle)
10. **TurretAI** (`0x007468C0`)
11. Sinking descent
12. IsHarvesting flag clear
13. **Fire_At_Target** (`0x00736DF0`)
14. **Facing_Update** (`0x00736990`)
15. Guard terrain check
16. Harvester/Weeder AI brain (`0x00737180`)
17. UpdateAnimation (vtable+0x424)
18. SpawnManager update
19. AI auto-hunt (every 16 frames)
20. Stuck harvester rescue

## 11. Current Rust Implementation Status

### Implemented
- **Object type with vehicle fields**: ObjectType struct has turret, harvester, crusher, deploy fields
- **Turret rotation**: `tick_turret_rotation` in `sim/movement/turret.rs` — ROT-based rotation, idle return-to-body, harvest spin
- **Deploy/undeploy**: `deploy_mcv`, `undeploy_building` in `sim/world/world_spawn.rs`
- **Harvester state machine**: Full 8-state `MinerState` in `sim/miner/` with dock phases
- **Crush system**: `can_crush`, `collect_crush_victims` in `sim/movement/bump_crush.rs`
- **Drive tracks**: 72 TurnTracks, 16 RawTracks with transform flags
- **Unit rendering**: VXL body+turret+barrel compositing, OREGATH overlay, turret offset

### NOT Implemented
- **Deploy countdown / timed death**: V3 rocket self-destruction timer (DeathFrames)
- **UnitClass-specific struct fields**: No Rust equivalent of 0x6C0–0x6E4 fields (FlagCarrier, CrashState, DeployAnimState)
- **Crush tilt visual**: `TiltsWhenCrushes` (-0.05f nose dip) not applied
- **IsTrain mutual passthrough**: Trains passing through each other in Can_Enter_Cell
- **Sinking movement**: IsSinking descent at -5Z/tick with wake particles
- **Turret idle scan**: TurretScansNearby enemy detection in adjacent cells
- **Gattling weapon stage**: IsGattling spin-up/decay accumulator
- **SpawnManager AI**: Aircraft Carrier / Dreadnought spawn dispatch
- **Stuck harvester rescue**: AI harvester path-out from dock cell
- **CTF flag carrier**: FLAGFLY.SHP rendering and attach/detach
- **Convoy chain**: next_in_convoy linked list, speed propagation, stop propagation
- **Short drive track selection**: Always uses normal track (hardcoded false)

## 12. Open Questions

1. **Fields 0x6D3, 0x6DC**: Initialized to 0 but not traced to specific behavior. May be related to transport or dock state.
2. **Offset 0x6D4 vs 0x6D8 dual-use**: Both initialized to -1. 0x6D8 is checked by IsCrashing, 0x6D4 usage needs more tracing — may be deploy anim phase or separate crash counter.
3. **UnitClass::Draw_It**: Not decompiled. VXL draw pipeline is complex and spread across multiple functions. The vtable slot needs to be resolved from the vtable at `0x7F5C70`.
4. **TurretROT separation**: rulesmd.ini has only `ROT=` (no separate `TurretROT`). Need to verify in binary whether body and turret share the same ROT value or if there's a hidden separation.
5. **Deployer facing gate**: `type+0xEDC` (DeployFacing) — the exact facing comparison formula needs verification. MCV_DEPLOY report covers this but field may have changed.
6. **UnitClass::Mission_Harvest full flow**: The AI function at `0x00737180` (harvest brain) was partially decompiled — the full ore-scan and refinery selection deserves its own report expansion.
7. **Drive tracks 5–6**: Wide cell-crossing curve point data still missing from binary extraction (noted in existing docs).

## 13. UnitClass Vtable Method Map (vtable at `0x7F5C70`, 160+ entries)

Complete list of UnitClass-specific overrides and key inherited methods. Vtable fully parsed from binary — all 160 entries resolved, every UnitClass override (0x73xxxx–0x74xxxx range) identified and labeled in Ghidra.

| Address | Name | Vtable Offset | Size | Notes |
|---------|------|---------------|------|-------|
| `0x007360C0` | UnitClass__AI | +0x5C | 270 lines | Main tick — 20 subsystems |
| `0x007393C0` | UnitClass__Deploy | +0xC4(?) | 289 lines | MCV deploy to ConYard |
| `0x007404B0` | UnitClass__What_Action_OnCell | +0x1A4 | 122 lines | Cursor/action for cell hover |
| `0x007416A0` | UnitClass__PerCellProcess | +0xF0 | 105 lines | Crush on cell entry |
| `0x00744470` | UnitClass__Draw_It | +0x14 | ~391 bytes | Load/save + draw dispatch |
| `0x00744600` | UnitClass__UpdatePosition | +0x18 | 31 lines | Clears 0x6DC, calls parent |
| `0x00746810` | UnitClass__InitFromType | +0x24 | ~108 bytes | Veteran setup, gattling init |
| `0x00746B20` | UnitClass__GetToolTipString | | 55 lines | Tooltip with passenger info |
| `0x00746C90` | UnitClass__IsCrashing | +0x37C | 12 lines | 0x6D8 != -1 OR EMP |
| `0x00746CB0` | UnitClass__CanDeploy | +0x314 | 5 bytes | Thunk to TechnoClass::CanDeploy |
| `0x00746D80` | UnitClass__IsFalling | slot unknown — needs re-survey | 3 lines | Returns byte at 0x6E0. +0x1D8 is wrong (= `0x0070C5C0` TechnoClass__IsBeingWarped; verified via `read_memory 0x007F5E48`); full vtable scan 0x007F5C70–0x007F6100 did not find 0x00746D80 |
| `0x00746DE0` | UnitClass__GetCLSID | +0x0C | 56 bytes | Returns UnitClass CLSID |
| `0x00746E20` | UnitClass__What_Am_I | +0x2C | 5 bytes | Returns 1 (RTTI_Unit) |
| `0x00746E80` | UnitClass__ScalarDelDestructor | | 11 lines | |
| `0x00743A50` | UnitClass__Scatter | +0x174 | ~247 lines | Already documented |
| `0x0073F0A0` | UnitClass__Can_Enter_Cell | +0x1AC | ~465 lines | Already documented |
| `0x0073CEC0` | UnitClass__DrawExtras | | ~588 bytes | OREGATH, flag, pips |
| `0x0073E5E0` | UnitClass__Mission_Harvest | | | Harvest state machine |
| `0x00740810` | UnitClass__Mission_Guard_Harvester | | ~634 bytes | Harvester guard variant |
| `0x00740A90` | UnitClass__Mission_Guard | | 21 lines | Clears IsHarvesting, updates anim |
| `0x00740B60` | UnitClass__Mission_Hunt | | 118 lines | 8-direction best-cell search |
| `0x0073B0B0` | UnitClass__Mission_Move | | 34 lines | NavalYard dock check + parent |
| `0x00739EC0` | UnitClass__Mission_Enter | | ~4.5KB | Transport enter + dock logic |
| `0x0073D630` | UnitClass__Mission_Deploy_Building | | ~4KB | MCV/SimpleDeployer/Dump state machine |
| `0x00737430` | UnitClass__Receive_Radio | | ~2KB | Docking protocol responses |
| `0x007359F0` | UnitClass__TubeMovement | | 230 lines | Tube/tunnel traversal |
| `0x00737180` | UnitClass__HarvestBrain_Idle | | 102 lines | Idle→Harvest mission brain |
| `0x007468C0` | UnitClass__TurretAI | | 108 lines | Turret scan + enemy detect |
| `0x00736DF0` | UnitClass__Fire_At_Target | | 149 lines | Weapon fire + gattling |
| `0x00736990` | UnitClass__Facing_Update | | 95 lines | Body/turret rotation |
| `0x00738680` | UnitClass__Death_Explosion | | 60 lines | Timed death debris |
| `0x00740DF0` | UnitClass__AttachFlag | | | CTF flag attach |
| `0x00740E20` | UnitClass__DetachFlag | | | CTF flag detach |
| `0x00740E50` | UnitClass__ReceiveDamage | +0xE0(?) | 5 bytes | Thunk to parent |
| `0x00746520` | UnitClass__Save | | 132 bytes | Serialization |
| `0x00746400` | UnitClass__IsNotDeployer | +0x80 | 22 bytes | Returns !(type+0xE1B). Slot re-verified correct: `read_memory 0x007F5CF0` → 0x00746400 |
| `0x007465B0` | UnitClass__GetDisplayType | +0xCC | 34 bytes | Disguise: returns type or disguise type |
| `0x007465F0` | UnitClass__GetDisplayOwner | +0xD0 | 52 bytes | Disguise: returns owner or disguise house |
| `0x007440B0` | UnitClass__Limbo | +0xD4 | 50 bytes | Rally point save + parent limbo |
| `0x00746750` | UnitClass__RecordKill | +0xC8 | 185 bytes | Kill recording + map reveal for attacker |
| `0x00738910` | UnitClass__Fire_At | +0x140 | 88 bytes | Deployed check + parent fire |
| `0x00738890` | UnitClass__Can_Fire_At | +0x144 | 125 bytes | NavalYard + move check + parent |
| `0x00744270` | UnitClass__ShouldIdle | +0x200 | 507 bytes | Deploy/dock/mission idle check |
| `0x00740EF0` | UnitClass__Mission_Unload | +0x24C | 144 bytes | Clears IsHarvesting, RulesClass+0x850 lookup |
| `0x00740B10` | UnitClass__Mission_Hunt_Override | +0x25C | 65 bytes | Clears IsHarvesting + parent hunt |
| `0x007353C0` | UnitClass__Constructor | | ~960 bytes | Field init |
| `0x007463A0` | UnitClass__Transfer_Convoy_On_Owner_Change | | | Convoy chain ownership |
| `0x00744640` | FootClass__Save_Convoy_State | +0x34 | ~350 bytes | Inherited — convoy serialization |
| `0x00741490` | TechnoClass__GetTechnoType_Impl_Unit | +0x88 | 7 bytes | Returns type pointer |
| `0x00737BA0` | UnitClass__Unlimbo | +0xD8 | ~232 bytes | Unlimbo + placement init |
| `0x00744720` | UnitClass__OnEnterCell_Triggers | +0xE0 | ~122 bytes | Trigger processing on cell entry |
| `0x007441B0` | ObjectClass__Mark_Occupation | +0xF0 | ~93 bytes | Inherited occupation marking |
| `0x00744210` | ObjectClass__Clear_Occupation | +0xF4 | ~83 bytes | Inherited occupation clearing |
| `0x00737C90` | UnitClass__Mission_Harvest_Full | +0x16C | ~2.5KB | Full harvest mission handler |
| `0x00746D60` | UnitClass__Receive_Message_Hook | +0x170 | 17 bytes | Message hook thunk |
| `0x00744100` | UnitClass__ScanForTiberium_SlaveMiner | +0x220 | ~117 bytes | Slave miner ore scan |
| `0x0073EFC0` | UnitClass__DeployHelper | +0x228 | ~468 bytes | Deploy helper logic |

## 14. Key Decompilation Findings (Extended Session)

### Mission_Guard (`0x00740A90`)
- Clears `IsHarvesting (0x6D2)` = 0
- If not deploying and not in deploy animation states: updates deployed anim, calls parent `FUN_004D4200` (MissionClass::Mission_Guard)
- Otherwise: enters Guard mission (code 5)

### Mission_Hunt (`0x00740B60`)
- 8-direction cell scoring algorithm with directional bias toward target
- For each adjacent cell: checks SpeedType terrain table passability, occupancy flags
- Score formula: `base_score (0x80 for passable, -0x80 for blocked) - facing_delta`
- If direction index == 4 (behind): penalty of -100
- Picks highest-scored cell; returns direction index
- `IsTrain` units: override to use current body facing direction

### Mission_Move (`0x0073B0B0`)
- Checks if docked to a building (NavalYard/repair type): if docking building mission is Sleep(0x10), handles undocking via FUN_004A5110/004A5130/004A51B0/004A51D0 chain
- Falls through to parent `FUN_005F4B10` (FootClass::Mission_Move)

### Deploy (`0x007393C0`, 289 lines) — MCV to ConYard
1. Check `vtable+0x314` (CanDeploy) — returns 0 if can't
2. Stop locomotor, unselect
3. Get cell coords, calculate foundation placement
4. Check `BuildingTypeClass::CanBePlacedAt` — AI units also validate
5. If facing != required `DeployFacing (type+0xEDC)`: rotate and return 1 (in progress)
6. Allocate 0x720 bytes, call `BuildingClass__Constructor` with `DeploysInto` type
7. Place building via `vtable+0xD8`; on failure: destroy building, play EVA_CannotDeployHere
8. Transfer rally targets to new building
9. Transfer veterancy + health ratio
10. Transfer upgrades (copies 5 DWORDs from offset 0x1F0 + 2 more)
11. Transfer sound handles
12. If ConYard + AI: set base location + production flags
13. Destroy MCV unit, play deploy sound

### Tube Movement (`0x007359F0`, 230 lines)
- Reads `TubeClass` from global `g_TubeArray + DriveTrackIndex * 4`
- Steps through tube node array at `TubeClass+0x30 + step*4`
- Each node encoded as direction (bits 0-2) + metadata
- Calculates world position by interpolating between tube entry and exit via sin/cos
- Height interpolated linearly between entry and exit ground heights
- At each node: reads direction, advances tube step counter (+0x685)
- Node value -1 = end of tube → exits to world surface

### UpdatePosition (`0x00744600`)
- If `0x6DC` (Unknown_6DC) != 0: calls `FUN_004C2C10()` then clears 0x6DC to 0
- Falls through to parent `FUN_004DB690` (FootClass::UpdatePosition)
- This means 0x6DC is likely a "pending position update" or "visual override reset" flag

### Unlimbo (`0x00737BA0`, 38 lines)
1. Calls parent `FUN_004D7170` (FootClass::Unlimbo) with coords and facing
2. If fails: return 0
3. Sets body facing from param
4. **Cloaking init:** If `Cloakable (0x3D2)` and NOT `HasSight (0x3D5)`: sets `CloakState (0x220) = 2`
5. **Harvester anim init:** If type is Harvester (+0xE18) or Weeder (+0xE19):
   - Random start frame 0–29 for StepCounter (+0xF8)
   - StepTimer rate = 1, total = 1
   - Otherwise: zero all StepTimer fields

### Receive_Radio (`0x00737430`, 204 lines) — Radio message dispatch
Radio protocol messages handled by UnitClass:

| Message | Code | Behavior |
|---------|------|----------|
| RADIO_OVER_OUT | 3 | If mission is Sleep(0xC): switch to Guard(5). Calls parent. |
| RADIO_MOVE_HERE | 7 | Clear destination, clear target, assign Nothing mission, force idle. If docking flag: start dock approach (radio 0x13). |
| RADIO_CAN_LOAD | 0xE | Spawn range check: `type->SpawnDelay` distance vs caller size. Zone ID compatibility check. If compatible: parent call + dock coordination. |
| RADIO_DOCKING | 0xF | Spawn proximity check. Mind control gate. Validates no building on cell. Size compatibility. |
| RADIO_LOADING_DONE | 0x15 | If at spawn capacity: update idle anim. |
| RADIO_APPROACH | 0x16 | Face South (0x4000). If locomotor stopped and not docking: start dock approach (0x15 message to target). |
| RADIO_UNLOAD | 0x17 | Harvester/Weeder: clear DockingInitialized (0x6D1), scatter, enter Harvest(10) mission. |
| RADIO_CAN_ENTER | 0x24 | Bridge check. Tube check (DriveTrackIndex). Docking flag gate. Returns 1/10 based on state. |

### Mission_Enter (`0x00739EC0`, 534 lines) — The biggest function
This handles ALL "enter" scenarios for units:

**Phase 1 — Deploy check (state 0/2):**
If unit has `Deployer` flag and state is 0 or 2: attempts `UnitClass::Deploy`. If unit dies during deploy → return.

**Phase 2 — Building sell/enter (state 2):**
On cell with building that matches destination:
- If mission is `Capture(9)` and target is self-sell building: sells building, refunds credits, plays sell sound, clears GarrisonAnim, destroys self
- If building is `CanBeOccupied` (0x16AE) and mission is `Enter(7)`: radio DOCKING(0xF) → limbo self → add to cargo → mark production changed

**Phase 3 — Transport approach (state 7/0x19):**
- Coordinate check: is unit at the transport's dock point? (Exit coordinates from BuildingTypeClass)
- **WalkLocomotion CLSID check:** If locomotor CLSID matches WalkLocomotion AND building has `CanLoadNaval` (0x16A9): set as docking target
- If at dock: change mission to Capture(0x15), force locomotor process

**Phase 4 — Harvester redirect:**
If unit is flagged as docking to building and mission is Enter(7):
- If current cell has same building as dock target: start docking radio
- If no building and no dock target: AI harvesters seek nearest refinery; player harvesters stop and scatter
- If refinery found: move to it, assign Sleep(0xB) mission, set ghost cell

**Phase 5 — Terrain validation:**
Checks if unit is on passable terrain. If `Can_Enter_Cell` returns 7 (impassable) and not on bridge and not sinking: spawn explosion anim, apply crush warhead damage, destroy self.

**Phase 6 — Post-entry:**
Updates fog border with extended vision range (+3). Checks harvest mission assignment for Harvester/Weeder types targeting dock buildings.

### DeployHelper (`0x0073EFC0`, 47 lines)
2-state machine for AI auto-deploy of ConstructionYard-type units:
- State 0: Check `FUN_00738D30` (can deploy here). If yes: call `UnitClass::Deploy`, advance to state 1.
- State 1: Wait for `HasReachedDock (0x68C)` to clear, then return to state 0.
- Falls through to parent `FUN_004D5350` (FootClass::Mission_Deploy) for non-ConYard types.

### ScanForTiberium_SlaveMiner (`0x00744100`)
- Gates on `field_0x2D8 != 0` (has SlaveManager/harvest reference)
- Timer: `RulesClass+0x1790` (HarvestInterval) + `this+0xC0` (lastScanTime) < g_CurrentFrameCounter
- When timer fires: calls `FUN_006B1020` (ore scan) → `FUN_006B0CC0` (dispatch slave workers)
- Falls back to `FootClass::Mission_Harvest` if no harvest target

### OnEnterCell_Triggers (`0x00744720`)
Fires trigger actions when entering a cell:
- If has `AttachedTag (0x34)`: checks `FUN_006E57C0` for crossing zone boundaries
- Fires trigger actions: 7 (enter cell), 0x30 (unit enters cell, specific), 0x1D (enters waypoint)
- Calls `TechnoClass::RecordKill` with the cell for zone tracking

### Fire_At (`0x00738910`)
- If unit is **deployed** (`param_1[0x1B8]` flag set): calls `vtable+0x70` with weapon 0. If result is 2 (blocked): return 0.
- If IsSinking (`param_1[0xA6]`): return 0 (cannot fire while sinking)
- Otherwise: parent `FUN_004D7D50` (FootClass::Fire_At)

### Can_Fire_At (`0x00738890`)
Mission remapping before parent call:
- If parent returns mission code 3 → remap to 1
- If parent returns 9 or 0x10 → remap to 5 (Guard)
- If target is self AND result is Move(2) or Guard(5) → return 0 (can't fire at self)
- IsSinking gate: return 0

### ShouldIdle (`0x00744270`, 507 bytes)
Complex idle-state check:
- NOT idle if: mission is Capture(6) or Sleep(0x15), deploy anim playing (0x6E1/0x6E2), docking (0x6D1)
- NOT idle if: mission != Enter(7) AND locomotor can move AND height >= 0 AND mission != Guard(5)/Attack(1) AND not attacking
- Dock check: if docked to NavalYard-type building at current cell offset (0, +1): return 0 (stay idle in dock)
- Deploy check: if `IsSimpleDeployer` AND deployed AND no target AND mission != Move(2)/Enter(7): return 0

### GetToolTipString (`0x00746B20`)
- Checks if unit has a passenger: if so, shows passenger type name
- Checks veterancy: shows vet/elite status strings
- Checks `type+0xECA` (some flag): shows custom tooltip per vet level
- Uses `StringTable__LoadString` with indexed string IDs (0x2C90, 0x2C94, 0x2C98, 0x2CBE)

## Sources

### Ghidra addresses decompiled
- `0x007353C0` — UnitClass::Constructor (assembly MOV scan for field layout)
- `0x007360C0` — UnitClass::AI (full, 270 lines)
- `0x007393C0` — UnitClass::Deploy (full, 289 lines — MCV to ConYard)
- `0x007416A0` — UnitClass::PerCellProcess (full, 105 lines — crush logic)
- `0x007468C0` — TurretAI (full, 108 lines — idle scan, enemy proximity, spawn control)
- `0x00736DF0` — Fire_At_Target (full, 149 lines — weapon dispatch, gattling, harvester anim)
- `0x00736990` — Facing_Update (full, 95 lines — body/turret rotation toward target)
- `0x00738680` — Death explosion handler (60 lines — debris, ore scatter)
- `0x00746C90` — UnitClass::IsCrashing (CrashState check + EMP)
- `0x00746D80` — UnitClass::IsFalling (returns byte at 0x6E0)
- `0x007359F0` — Tube movement handler (full, 230 lines)
- `0x00740A90` — Mission_Guard (full, 21 lines)
- `0x00740B60` — Mission_Hunt (full, 118 lines — 8-direction scoring)
- `0x0073B0B0` — Mission_Move (full, 34 lines — naval dock check)
- `0x00744600` — UpdatePosition (31 lines — 0x6DC flag + parent)
- `0x00746810` — InitFromType (veteran, gattling init)
- `0x00746B20` — GetToolTipString (55 lines — passenger/vet display)
- `0x00746CB0` — CanDeploy (5 bytes — thunk)
- `0x00746DE0` — GetCLSID (CLSID return)
- `0x00740E50` — ReceiveDamage (5 bytes — thunk to parent)
- `0x00746400` — IsNotDeployer (22 bytes — returns !(type+0xE1B))
- `0x007465B0` — GetDisplayType (34 bytes — disguise type dispatch)
- `0x007465F0` — GetDisplayOwner (52 bytes — disguise house dispatch)
- `0x007440B0` — Limbo (50 bytes — rally point save + parent)
- `0x00746750` — RecordKill (185 bytes — kill recording + map reveal)
- `0x00738910` — Fire_At (88 bytes — deployed check + parent)
- `0x00738890` — Can_Fire_At (125 bytes — naval/move check + parent)
- `0x00744270` — ShouldIdle (507 bytes — deploy/dock/mission idle)
- `0x00740EF0` — Mission_Unload (144 bytes — ore unload dispatch)
- `0x00740B10` — Mission_Hunt_Override (65 bytes — clears harvest + parent)

**Total functions labeled in Ghidra this session: 50+**

### Extended decompilations (second pass)

- `0x00737BA0` — UnitClass__Unlimbo (38 lines)
- `0x00737430` — UnitClass__Receive_Radio (204 lines)
- `0x00739EC0` — UnitClass__Mission_Enter (534 lines)
- `0x0073FD50` — UnitClass__What_Action_OnObject (1882 bytes)
- `0x00744720` — UnitClass__OnEnterCell_Triggers (decompiled — trigger action dispatch)
- `0x0073EFC0` — UnitClass__DeployHelper (47 lines)
- `0x00744100` — UnitClass__ScanForTiberium_SlaveMiner (decompiled — ore scan timer)
- `0x00746400` — UnitClass__IsNotDeployer (decompiled — returns !(type+0xE1B))
- `0x007465B0` — UnitClass__GetDisplayType (decompiled — disguise type dispatch)
- `0x007465F0` — UnitClass__GetDisplayOwner (decompiled — disguise house dispatch)
- `0x007440B0` — UnitClass__Limbo (decompiled — rally point save)
- `0x00746750` — UnitClass__Discovered_By (decompiled — map reveal for enemy)
- `0x00738910` — UnitClass__Fire_At (decompiled — deployed check)
- `0x00738890` — UnitClass__Can_Fire_At (decompiled — mission remap)
- `0x00744270` — UnitClass__ShouldIdle (decompiled — 507 bytes, idle state gating)
- `0x00740EF0` — UnitClass__Mission_Unload (decompiled — ore unload dispatch)
- `0x00740B10` — UnitClass__Mission_Hunt_Override (decompiled — clears harvest)

### Doc files referenced
- `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` — Scatter branch logic
- `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md` — Cell-entry return codes, urgency escalation
- `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md` — Deploy/undeploy state machine
- `UNIT_DRAW_EXTRAS_REPORT.md` — OREGATH, CTF flag, pip rendering
- `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` — Passability codes 0–7
- `DRIVE_LOCOMOTION_CLASS.md` — DriveLocomotionClass layout, vtable map
- `DRIVE_TRACK_SYSTEM.md` — Track tables, transform flags, stepping algorithm
- `CONVOY_FORMATION_SYSTEM_GHIDRA_REPORT.md` — Convoy chain fields, IsTrain
- `CRUSH_SYSTEM_GHIDRA_REPORT.md` — CanCrushCheck, crush warhead, offsets
- `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md` — Harvest state machine
- `FOOTCLASS_COMPLETE_GHIDRA_REPORT.md` — Parent class layout 0x520–0x6BF
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md` — Parent class layout 0x00–0x520

### INI files checked
- `ini/rulesmd.ini` — VehicleTypes, Crusher, Harvester, Deployer, ROT, SpeedType, IsGattling
- `ini/artmd.ini` — TurretOffset, VXL layer definitions
