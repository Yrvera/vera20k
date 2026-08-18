# MCV Deployment — gamemd.exe Ghidra Analysis

Complete reverse-engineering of the two MCV deployment paths in Yuri's Revenge.

---

## Overview

There are **two deployment paths** for an MCV:

| Path | Direction | Trigger |
|------|-----------|---------|
| **Unit Deploy** | MCV unit → Construction Yard building | Player deploy command (D key / button) |
| **Building Undeploy** | Construction Yard → MCV unit | Player undeploy command (multiplayer only, requires MCVRedeploys=ON) |

Both paths destroy the source object and create a replacement at the same cell,
transferring health, ownership, veterancy, and waypoints.

---

## INI Configuration

### Unit Types (rulesmd.ini)

| Key | AMCV (Allied) | SMCV (Soviet) | YMCV (Yuri) |
|-----|---------------|---------------|-------------|
| `DeploysInto=` | GACNST | NACNST | YACNST |
| `Deployer=` | (not set) | (not set) | yes |

### Building Types (rulesmd.ini)

| Key | GACNST | NACNST | YACNST |
|-----|--------|--------|--------|
| `UndeploysInto=` | AMCV | SMCV | (not set in base rulesmd) |
| `ConstructionYard=` | yes | yes | yes |

### TechnoTypeClass Field Offsets (from ReadINI at `0x00712170`) <!-- corrected 2026-05-28: was 0x00713280; binary entry point is 00712170 via get_function_by_address — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

| Byte Offset | Field Size | INI Key | Description |
|-------------|-----------|---------|-------------|
| `+0x404` | 4 (pointer) | `DeploysInto` | Pointer to BuildingTypeClass this unit deploys into |
| `+0x408` | 4 (pointer) | `UndeploysInto` | Pointer to UnitTypeClass this building undeploys into |
| `+0x6BC` | 4 (pointer) | `DeployingAnim` | Deploy animation type pointer (AnimTypeClass*) | <!-- corrected 2026-05-28: was +0x5AC; binary shows param_1[0x1af] = iVar4 after s_DeployingAnim_00819490 ReadString call → 0x1af*4=0x6BC via TechnoTypeClass__ReadINI @0x00712170 — ROOT_CAUSE: OFFSET_RETYPED_WRONG -->
| `+0x56C` | 4 (int) | `DeploySound` | Deploy sound index (used in UnitClass::Deploy cleanup) | <!-- corrected 2026-05-28: was +0x5A8 "undeploy anim index"; +0x5A8 (param_1[0x16a]) is ActivateSound, not undeploy anim; +0x56C is DeploySound confirmed in UnitClass__Deploy vtable+0x84 read → *(iVar11+0x56c) — ROOT_CAUSE: OFFSET_RETYPED_WRONG -->
| `+0x5EC` | 1 (bool) | (deploy target flag) | Deploy-to-target mode flag |
| `+0x16B9` | 1 (bool) | `ConstructionYard` | Marks building as a ConYard (controls undeploy restrictions) |
| `+0xCA1` | 1 (bool) | `Turret` | **NOT Deployer** — Ghidra plate comment in TechnoTypeClass__ReadINI confirms this is the `Turret=` INI key; `Deployer` field offset is UNVERIFIED in this session | <!-- corrected 2026-05-28: was "+0xCA1 = Deployer"; binary plate comment @0x00712170 explicitly labels [this+0xCA1] = Turret (INI key "Turret=") — ROOT_CAUSE: OFFSET_RETYPED_WRONG -->
| `+0xE13` | 1 (bool) | `IsSimpleDeployer` | Simple deployer (no building conversion) |

### Instance Field Offsets (TechnoClass/UnitClass/BuildingClass)

| Byte Offset | Field | Description |
|-------------|-------|-------------|
| `+0x72` (x4=`0x1C8`) | deploying flag | Set to 1 during deploy animation |
| `+0x19D` (x4=`0x674`) | COM deploy interface | ILocomotion-style deploy interface pointer |
| `+0x1A3` (x4=`0x68C`) | deploy state | Tracks deploy/undeploy progress |
| `+0x1A5` (x4=`0x694`) | attached object | Link to attached/garrisoned object |
| `+0x6AD` | deploy/locomotor-piggyback active guard | Participates in deploy/piggyback active state and blocks non-null Set_Destination_Internal calls; "deploy complete" is too narrow for movement parity. |
| `+0x6B6` | perform deploy complete | Set in PerformDeploy |
| `+0xAB` (x4=`0x2AC`) | owner link | Back-reference from deployed building to deployer |
| `+0xB0` (x4=`0x2C0`) | power link | Attached powered unit (blocks undeploy if set) |
| `+0x2F` (x4=`0xBC`) | deploy sub-state | Sub-state within deploy mission |

---

## Path 1: MCV → Construction Yard (Unit Deploy)

### Trigger

Player selects MCV and presses **D** key or clicks the **Deploy** button.
This sends a network event (event type `0x1E` = Deploy) to the lockstep engine.

### Key Functions

| Address | Name | Role |
|---------|------|------|
| `0x006AFD60` | UnitClass::Mission_Deploy handler | State machine driving the deploy sequence | <!-- corrected 2026-05-28: was 0x006AFF60; binary entry point is 006afd60 via get_function_by_address — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->
| `0x007393C0` | UnitClass::Deploy | Core conversion: creates building, transfers state, destroys unit |
| `0x0070FC90` | TechnoClass::OnDeployBegin | Initiates deploy animation, resets turret |
| `0x0070FBE0` | TechnoClass::OnUndeployComplete | Cleanup after deploy animation completes |
| `0x0070FB50` | TechnoClass::CanAutoDeployHere | Validates cell for auto-deploy capable units |
| `0x00710000` | TechnoClass::PerformDeploy | Alternative deploy path (COM interface based, for building→building) |
| `0x0043B740` | BuildingClass::Constructor | Creates the new Construction Yard |
| `0x00465D70` | (facing calculator) | Computes deploy facing direction |

### State Machine (`FUN_006AFD60`) <!-- corrected 2026-05-28: was FUN_006AFF60 — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT -->

The deploy mission runs a multi-state loop at `this+0x5C`:

<!-- corrected 2026-05-28: doc previously showed only 6 states (0–5); binary has 7 states (0–6) via decompile of FUN_006AFD60; also RTTI limbo check is piVar1[0x169] where piVar1 is int* → byte offset 0x5A4, not "entity[0x169] == 0" literally; failure in state 3 → state 1, not a new call chain — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT -->

```
State 0: Initial check
  - If target is a building (RTTI == 6) with mission != 0x13 and != 0x12, go to state 5

State 1: Move to deploy position
  - Get current cell coordinates via vtable+0x338
  - If already at target cell: reset to state 0
  - Find valid deploy cell via SlaveManagerClass__FindDeployCell
  - Move unit to deploy cell via vtable+0x480
  - Set mission via vtable+0x1e8
  - Transition to state 2

State 2: Execute deploy
  - Verify unit is still a UnitClass (RTTI check == 1)
  - Check unit is not limbo'd (piVar1[0x169] != 0 → unit+0x5A4 byte offset)
  - Call UnitClass__Deploy (0x007393C0)
  - On success → state 4
  - On failure → state 3 (with 30-frame retry delay, `entity+0x58 = 0x1E`)

State 3: Retry deploy
  - Verify still a unit (RTTI == 1)
  - Call UnitClass__Deploy again
  - On success → state 4
  - On failure → state 1 (go back to find new position)

State 4: Post-deploy (building placed)
  - Monitor the newly created building (RTTI == 6)
  - If building has production capability (`building+0x534 != 0`), go to state 5
  - If unit reappears (RTTI == 1) with deploy_state clear and mission == 5, return to state 2

State 5: Monitor deploy target distance
  - If unit type is back (RTTI == 1), reset to state 0
  - Otherwise compare current cell to deploy cell via SlaveManagerClass__FindDeployCell
  - If distance from deploy target > RulesClass threshold (g_RulesClass_Instance+0x1788),
    trigger rally movement and transition to state 6
  - Call SlaveManagerClass__DeploySlaves

State 6: Rally movement in progress
  - Wait for unit RTTI to return to 1 (unit)
  - Recall idle slaves via SlaveManagerClass__RecallIdleSlaves
  - Transition to state 1
```

### Core Deploy Function (`FUN_007393C0` — 0x007393C0)

This is the heart of MCV deployment. Decompiled logic:

```
1. VALIDATE
   - Check vtable+0x314 (CanDeploy virtual) returns true
   - Verify not already deploying (entity[0x169] == 0)
   - Query COM deploy interface (entity[0x19D]) — confirm not locked
   - Verify TypeClass has DeploysInto set (type+0x404 != 0)

2. PREPARE
   - Lock deploy interface (vtable+0x124, pass 0)
   - Set deploy interface to not-deployable (vtable+0x9C, pass 0)
   - Calculate deploy position from current coordinates (vtable+0x1B8)

3. CHECK FACING
   - Calculate required deploy facing via FUN_00465D70
   - Get current facing from FUN_004C93D0
   - If facing doesn't match:
     a. Check COM interface facing lock (vtable+0x80)
     b. Set new facing (vtable+0x4C)
     c. Set deploy direction (vtable+0x274, pass 3)
     d. Set deploy_state flag (entity+0x1A3 = 1)
     e. Return 1 (in progress — come back next tick)

4. CREATE BUILDING
   - Facing matches — proceed
   - Lock deploy interface (vtable+0x124, pass 0)
   - Set interface not-deployable (vtable+0x9C, pass 0)
   - Allocate new BuildingClass: operator_new(0x720)
   - Construct building: FUN_0043B740(DeploysInto_type, owner_house)
   - Set building mission to Deploying (vtable+0x1E8, mission 0x12)

5. PLACE BUILDING
   - Calculate cell coordinates: cell_x * 256 + 128, cell_y * 256 + 128
   - Call building->Place(coords) via vtable+0xD8
   - If placement FAILS:
     a. Destroy the building (vtable+0x20, pass 1)
     b. Re-enable deploy interface
     c. Return 0 (failure)

6. TRANSFER STATE (on successful placement)
   - Transfer all rally-bound units to the new building:
     For each TechnoClass in g_TechnoClass_Array:
       If unit.owner == this_MCV:
         Redirect unit.target to new_building (vtable+0x3C8)
   - Copy veterancy: building[0x85] = MCV[0x85]
   - Copy experience: building[0x54] = MCV[0x54]
   - Handle fog/shroud reveal for local player
   - Transfer health:
     health_ratio = MCV.GetHealthRatio()
     building.health = ftol(health_ratio * building.max_health)
     building.health = max(building.health, 1)
   - Transfer upgrade slots: copy entity[0x137..0x13D]
   - Transfer power-up links if present (FUN_006B0D10, FUN_006AF580)

7. CLEANUP
   - Play EVA announcement if applicable
   - Destroy the MCV unit (vtable+0x3A0)
   - Remove from any team/group
   - Play deploy animation if type has one (type+0x56C)
   - Mark deploy complete (vtable+0xF8)
   - Return 1 (success)
```

---

## Path 2: Construction Yard → MCV (Building Undeploy)

### Prerequisites

A ConYard can only undeploy back into an MCV when ALL of these are true:

| Condition | Check Location | Detail |
|-----------|---------------|--------|
| Has `UndeploysInto` set | `type+0x408 != 0` | GACNST→AMCV, NACNST→SMCV |
| Is `ConstructionYard=yes` | `type+0x16B9 != 0` | Non-ConYard deployers always allowed |
| Multiplayer mode | `DAT_00A8B238 != 0` | **Single player ConYards CANNOT undeploy** |
| `MCVRedeploys` option ON | `DAT_00A8B320 != 0` | Game lobby checkbox at dialog ID `0x66B` |
| Player is human | `FUN_0050B730()` | AI ConYards follow different rules |
| No power link | `entity+0x2C0 == 0` | No attached powered unit |
| Production queue empty | `entity[0x141] <= 0` | Cannot undeploy while producing |

### Gate Functions

**FUN_0044F5C0 — "Should show deploy button" (`0x0044F5C0`)**

Controls whether the undeploy button appears in the UI:
```
if production_queue_count > 0:     return false   // busy producing
if already_deployed_state:         return true    // show undeploy option
if type.ConstructionYard:
    if NOT human_controlled:       return false
    if NOT multiplayer:            return false
    if NOT MCVRedeploys:           return false
    if has_power_link:             return false
return type.UndeploysInto != 0
```

**FUN_00449BC0 — "Can undeploy ConYard" (`0x00449BC0`)**

Runtime validation when deploy command is issued:
```
if type.UndeploysInto == 0:        return false
if NOT type.ConstructionYard:      return true    // non-ConYard always OK
// ConYard-specific checks:
if NOT multiplayer:                return false
if NOT has_owner:                  return false
if NOT human_controlled:           return false
if NOT MCVRedeploys:               return false
if has_power_link:                 return false
return true
```

### Key Functions

| Address | Name | Role |
|---------|------|------|
| `0x0044F5C0` | BuildingClass::ShouldShowDeployButton | UI button visibility |
| `0x00449BC0` | BuildingClass::CanUndeployMCV | Runtime deploy capability check |
| `0x0073D630` | UnitClass::Mission_Deploy_Building | State machine for building deploy/undeploy | <!-- corrected 2026-05-28: was "BuildingClass::Mission_Deploy handler"; binary label is UnitClass__Mission_Deploy_Building via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
| `0x0073EFC0` | UnitClass::DeployHelper | Calls UnitClass::Deploy for ConYard conversion | <!-- corrected 2026-05-28: was "BuildingClass::DeployHelper"; binary label is UnitClass__DeployHelper via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT -->
| `0x007393C0` | UnitClass::Deploy | Same function as Path 1 — creates the target unit |

### Building Undeploy State Machine (`FUN_0073D630`)

When a ConYard has `DeploysInto` (or more precisely, it processes through
the undeploy path when it has a `DeploysInto` type):

```
State 0: Check deploy interface
  - Query COM deploy interface: is it ready?
  - If on a bridge (cell overlay == 2), find new cell
  - If not deployable, check facing and proceed

State 1: Wait for deploy ready
  - Check COM deploy interface readiness (vtable+0x10 → IsMoving?)
  - Call FUN_007393C0 to attempt conversion
  - On success:
    - If single player (DAT_00A8B238 == 0): set mission Guard (0xF)
    - If multiplayer: set mission Guard (5)
  - On failure: enter sub-state 2

State 2: Retry
  - If deploy_state flag (entity+0x1A3) is clear:
    - Cancel deploy (vtable+0x484)
    - Resume normal operation (vtable+0x1EC)
  - Otherwise call FUN_007393C0 again

State 3: Deploy in progress (deploy animation / unit placement)
  - Wait for deploy sequence to complete
  - Handle production output placement for factories
  - Monitor completion via cell placement checks

State 4: Deploy complete
  - Clear deploy flag (entity+0x6D1 = 0)
  - If has queued mission, execute it
  - Otherwise return to Guard mission
```

### Undeploy Flow Summary

```
Player clicks Deploy on ConYard
  → FUN_0044F5C0 checks button visibility (MCVRedeploys, etc.)
  → Network event 0x1E (Deploy) sent to lockstep
  → FUN_00449BC0 validates capability
  → BuildingClass::Mission_Deploy (FUN_0073D630) activates
  → FUN_0073EFC0 helper called
  → FUN_007393C0 creates MCV unit at ConYard's cell
  → Building destroyed, MCV unit placed
  → Health/veterancy/upgrades transferred
```

---

## Game Start Auto-Deploy (`ScenarioClass__Generate_Random_Units` at `0x006886B0`) <!-- corrected 2026-05-28: doc implied this was a dedicated "Auto-Deploy" function; binary label is ScenarioClass__Generate_Random_Units via get_function_by_address — ROOT_CAUSE: RTTI_LABEL_DRIFT -->

At game initialization (when `Bases=yes` lobby option is enabled, stored as
`DAT_00A8B258`), the engine places starting units:

```
For each player house:
  1. Pick starting position from available spawn points
  2. If Bases=yes (DAT_00A8B258):
     a. Look up ConYard type from RulesClass (offset +0xB20)
     b. Create MCV unit: UnitClass__Constructor at 0x007353C0 (ConYardType, House)
     c. Place at starting cell coordinates: cell_x*256+0x80, cell_y*256+0x80
     d. Call Place(coords) — vtable+0xD8
     e. If placement fails: FUN_00688ED0 (find alternate cell)
     f. Set as house's primary construction yard (iVar13+0x53dc, iVar13+0x53e0)
     g. If MCVDeploy special flag is ON (bit 4/0x10 of ScenarioClass flags): <!-- corrected 2026-05-28: was "bit 8"; binary: (*g_ScenarioClass_Instance & 0x10) != 0 via ScenarioClass__Generate_Random_Units decompile -->
        Call Force_MCV_Deploy at 0x004FC060
  3. Create remaining starting units (UnitCount option)
```

The starting ConYard is created directly — it does NOT go through the
MCV→ConYard deploy animation path. The MCV is created and placed as a unit,
then the game forces the deploy.

---

## Game Options Affecting MCV Deploy

### SpecialFlags (bit flags at `DAT_00A8B230`)

| Bit | Flag | Effect |
|-----|------|--------|
| 4 (`0x10`) | `MCVDeploy` | When ON, starting MCVs are forced to deploy at game start | <!-- corrected 2026-05-28: was "Bit 8"; binary check in ScenarioClass__Generate_Random_Units is `(*g_ScenarioClass_Instance & 0x10) != 0` → bit 4 (0-indexed), not bit 8 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT -->

### MultiplayerDialogSettings

| Offset | Setting | Variable | Dialog ID | Effect |
|--------|---------|----------|-----------|--------|
| `+0x14B8` | `MCVRedeploys` | `DAT_00A8B320` | `0x66B` | Enables ConYard → MCV undeploy |
| `+0x14AC` | `BridgeDestruction` | `DAT_00A8B260` | - | (unrelated but parsed alongside) |
| `+0x14AF` | `Bases` | `DAT_00A8B258` | - | Enables starting ConYard placement |

### WDT (Weight/Decision Table) Entries

| String | Address | Description |
|--------|---------|-------------|
| `WDT:RedeployableMCV` | `0x0084845C` | AI weight when MCVRedeploys=ON |
| `WDT:NoRedeployMCV` | `0x00848448` | AI weight when MCVRedeploys=OFF |

---

## EVA Voice Lines

| String | Address | Trigger |
|--------|---------|---------|
| `EVA_CannotDeployHere` | `0x0082012C` | Deploy attempted at invalid location |

Referenced from:
- `HouseClass__Place_Production` (`0x004FB372`) — placement failure
- `FUN_004AB9B0` (`0x004ABC7B`) — deploy command failure
- `FUN_007393C0` (`0x00739502`) — MCV deploy placement failure

---

## Key Differences Between the Two Paths

| Aspect | MCV → ConYard | ConYard → MCV |
|--------|---------------|---------------|
| Available in single player | Yes | **No** |
| Available in multiplayer | Yes | Only if MCVRedeploys=ON |
| Creates | BuildingClass (0x720 bytes) | UnitClass (via FUN_007353C0) |
| Facing requirement | Must face deploy direction | No facing requirement |
| Animation | Deploy animation (type+0x5AC) | Undeploy animation (type+0x5A8) |
| Blocked by | Invalid cell, bridge, occupied | Production queue, power link |
| AI behavior | AI auto-deploys via Mission handler | AI uses WDT weights |
| Sound | `DeploySound` | `UndeploySound` |
| Voice | `VoiceDeploy` | `VoiceUndeploy` |

---

## Summary

The MCV deploy system uses a **shared core function** (`FUN_007393C0` at `0x007393C0`)
for both directions. This function:

1. Validates deployment conditions
2. Checks/adjusts unit facing
3. Creates the target object (building or unit)
4. Attempts placement at the current cell
5. Transfers health, veterancy, experience, upgrades, and ownership links
6. Destroys the source object
7. Updates house production/sidebar state

The **critical gate** for ConYard→MCV undeploy is the `MCVRedeploys` game option
(`DAT_00A8B320`), which is only available in multiplayer games. Single-player
ConYards can never undeploy — this is hardcoded, not an INI setting.
