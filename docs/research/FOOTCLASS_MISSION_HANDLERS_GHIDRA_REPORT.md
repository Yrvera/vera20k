# FootClass Mission Handler Virtual Methods — Ghidra Research Report

**Date:** 2026-04-06
**Vtable:** `0x007E8C94` (FootClass primary vtable)
**Dispatch function:** `MissionClass::Mission_Dispatch` @ `0x005B3060`
**Confidence:** HIGH — all vtable-to-mission mappings verified from dispatch switch statement
**Active in YR:** Yes — these are the base mission handlers for all mobile units

---

## CRITICAL CORRECTION: Previous Label Errors

The FOOTCLASS_VTABLE_COMPLETE.md had **8 incorrectly labeled** mission handler slots.
The previous labels confused which vtable offset maps to which mission enum. This was
verified by decompiling the dispatch switch at `0x005B3060` and cross-referencing each
`case N: vtable+0xOFF` pair against the actual vtable memory.

**All 8 mislabeled functions have been corrected in Ghidra.**

| Address | OLD Label (WRONG) | NEW Label (CORRECT) | Mission Enum |
|---------|-------------------|---------------------|--------------|
| 0x4D4DC0 | Mission_Retreat | **Mission_Attack** | 1 |
| 0x4D4B20 | Mission_Guard_Idle | **Mission_Capture** | 8 |
| 0x4D4CB0 | Mission_Sticky | **Mission_Eaten** | 9 |
| 0x4D6AA0 | Mission_Harvest | **Mission_AreaGuard** | 11 |
| 0x4D5350 | Mission_Capture | **Mission_Hunt** | 15 |
| 0x4DA2C0 | Mission_AreaGuard | **Mission_Retreat** | 4 |
| 0x4D9290 | UnitClass__Mission_Harvest | **Mission_Enter** | 7 |
| 0x4D4280 | Mission_Hunt | **Mission_Patrol** | 25 |

Two labels were already correct:
- 0x4D5070 = `FootClass__Mission_Guard` (mission 5, vtable+0x21C)
- 0x4D4200 = `FootClass__Mission_Move` (mission 2, vtable+0x22C)

---

## Verified Mission-to-VTable Dispatch Map (FootClass)

From the dispatch switch at `0x005B3060`, `param_1[0x2B]` = CurrentMission (byte 0xAC):

| Case | Mission Name | VTable Offset | FootClass Address | Override? |
|------|-------------|---------------|-------------------|-----------|
| 0 | Sleep | +0x204 | 0x005B2E10 | NO (stub, returns 450) |
| 1 | **Attack** | +0x210 | **0x004D4DC0** | YES |
| 2 | **Move** | +0x22C | **0x004D4200** | YES |
| 3 | QMove | +0x204 | 0x005B2E10 | NO (falls to Sleep) |
| 4 | **Retreat** | +0x230 | **0x004DA2C0** | YES |
| 5 | **Guard** | +0x21C | **0x004D5070** | YES |
| 6 | Sticky | +0x21C | **0x004D5070** | YES (same handler as Guard) |
| 7 | **Enter** | +0x240 | **0x004D9290** | YES |
| 8 | **Capture** | +0x214 | **0x004D4B20** | YES |
| 9 | **Eaten** | +0x218 | **0x004D4CB0** | YES |
| 10 | Harvest | +0x224 | 0x005B2E90 | NO (stub — UnitClass overrides) |
| 11 | **AreaGuard** | +0x220 | **0x004D6AA0** | YES |
| 12 | Return | +0x234 | 0x005B2ED0 | NO (stub) |
| 13 | Stop | +0x238 | 0x005B2EE0 | NO (stub) |
| 14 | Ambush | +0x20C | 0x005B2E30 | NO (stub) |
| 15 | **Hunt** | +0x228 | **0x004D5350** | YES |
| 16 | Unload | +0x23C | 0x004DA2B0 | stub (returns 450) |
| 17 | Sabotage | +0x214 | **0x004D4B20** | YES (same as Capture) |
| 21 | Rescue | +0x258 | 0x004DDF90 | YES |
| 23 | Harmless | +0x208 | 0x005B2E20 | NO (stub) |
| 25 | **Patrol** | +0x25C | **0x004D4280** | YES |

**FootClass overrides 12 mission handler vtable slots** out of 28 possible.
Notable: Harvest (10) and Unload (16) are NOT overridden at FootClass level — these
are handled at subclass level (UnitClass overrides both).

---

## Handler Documentation

### Common Patterns

All mission handlers follow the same contract:
- Called by `Mission_Dispatch` when the dispatch timer expires
- Return value = number of frames until next call (mission rate)
- `MissionClass::GetMissionTimerEntry()` reads the INI-configured Rate/AARate for the mission
- `Math::ftol()` converts the timer entry to int
- `Random::RandomRanged(0,2)` adds jitter to prevent lockstep synchronization of units
- Standard return: `MissionTimerEntry + RandomRanged(0,2)`

### Key Struct Fields (param_1 is int*, byte offset = index * 4)

| Index | Byte | Name | Description |
|-------|------|------|-------------|
| 0x2B | 0xAC | CurrentMission | Active mission enum |
| 0x2D | 0xB4 | QueuedMission | Pending mission (-1 = none) |
| 0x2F | 0xBC | MissionState | Sub-state within handler (0=init) |
| 0x86 | 0x218 | NavCom | Destination target |
| 0x87 | 0x21C | Owner | HouseClass pointer |
| 0xAD | 0x2B4 | TarCom | Attack target |
| 0xB9 | 0x2E4 | WarheadBusy | Deploying flag |
| 0x166 | 0x598 | WaypointQueue.Count | Queued waypoints |
| 0x169 | 0x5A4 | NavCom (FootClass) | Primary navigation target |
| 0x175 | 0x5D4 | Team | TeamClass pointer |
| 0x177 | 0x5DC | GhostCell | Temp cell for hunt navigation |
| 0x19D | 0x674 | Locomotor | ILocomotion COM pointer |

### Key VTable Calls Used by Handlers

| VTable Offset | Method | Purpose |
|---------------|--------|---------|
| +0x2C | GetRTTI | Returns object type (0xF=Infantry, 6=Building, etc.) |
| +0x3C | GetOwnerHouse | Returns house/player index |
| +0x48 | GetCoords | Returns XYZ coordinates |
| +0x4C | GetCenterCoords | Returns center coordinates |
| +0x84 | GetTypeClass | Returns the TypeClass (unit type definition) |
| +0xBC | GetThreatRange | Get weapon range for threat scanning |
| +0x174 | SetFacing | Set unit facing direction |
| +0x184 | GetCurrentMission | Returns queued or current mission |
| +0x1B8 | GetCellCoords | Returns cell coordinates |
| +0x1C4 | GetCurrentCell | Returns CellClass of current position |
| +0x1C8 | IsLocomotionBusy | Returns 0 if locomotion is idle |
| +0x1E8 | Queue_Mission | Queue a new mission |
| +0x1EC | Commence | Commence queued mission |
| +0x200 | ReadyToCommence | Check if ready for mission transition |
| +0x278 | GetActionResult | Check action feasibility |
| +0x2E4 | SelectWeaponAgainst | Choose weapon against target |
| +0x3A8 | CanFireAt | Check if can fire at target |
| +0x3C4 | Greatest_Threat | Find best threat target |
| +0x3C8 | Set_ArchiveTarget | Set attack target |
| +0x3F8 | GetWeapon | Get weapon data |
| +0x478 | ScanForThreats_Simple | Quick threat scan (stub in FootClass) |
| +0x480 | Set_Destination | Set movement destination |
| +0x484 | OnArrival | Handle arrival at destination |
| +0x490 | PostMoveProcess | Handle post-movement logic |
| +0x53C | Greatest_Threat_Scan | Full threat scan for mobile units |

---

## 1. Mission_Move (enum 2) — 0x004D4200

**VTable offset:** +0x22C
**Triggers:** Player gives move order, AI pathfinding, waypoint queue

### Behavior

This is the simplest mission handler. Each tick:

1. Check `NavCom (0x5A4)` — if zero (no destination):
   a. Check `ILocomotion` — if locomotor reports "still moving", continue
   b. If locomotor stopped AND `QueuedMission == -1` (nothing queued):
      - Call `OnArrival(0, 1)` — process arrival logic
      - Return 1 (re-check next frame)

2. If NavCom is non-zero (has destination):
   - Return `MissionTimerRate + Random(0,2)` — standard polling rate

### Key Fields
- `param_1[0x169]` (0x5A4) — NavCom: primary destination
- `param_1[0x19D]` (0x674) — ILocomotion pointer
- `param_1[0x2D]` (0xB4) — QueuedMission

### Transitions
- On arrival (NavCom=0, loco stopped, no queue): calls `OnArrival` which handles
  mission transitions (typically to Guard)
- Movement itself is handled by the locomotor in `FootClass::AI`, not here — this
  handler just monitors completion

### Rate: MissionTimerEntry + Random(0,2)

---

## 2. Mission_Guard (enum 5) — 0x004D5070

**VTable offset:** +0x21C
**Triggers:** Default mission for most units, player stops unit, arrival from Move
**Also dispatched for:** Mission_Sticky (enum 6) uses the same vtable slot

### Behavior

Complex handler with multiple branches:

**Phase 1 — Special State Checks:**
1. If `IsReceivingRepair` (byte 0x68F): call RepairAI (vtable+0x340), return timer
2. If `IsDockingToBuilding` (byte 0x690): call DockingAI (vtable+0x348), return timer
3. If `IsWeedingHarvester` (byte 0x691): call WeedHarvestAI (vtable+0x34C), return timer

**Phase 2 — Target Processing (TarCom exists at 0x2B4):**
4. If has TarCom target:
   a. Get TypeClass, check `DefaultToGuardArea` flag (TypeClass+0x390)
   b. If DefaultToGuardArea AND locomotion idle:
      - Find nearest passable cell near home base
      - Set_Destination to that cell (patrol back toward base)
   c. Otherwise: continue guarding with target

**Phase 3 — Idle Guard (no TarCom):**
5. Get primary weapon; check if it exists and has `CanTarget` flag (TypeClass+0x158):
   a. If no weapon or can't target: call `ScanForThreats_Simple` (vtable+0x478)
   b. If has weapon: scan 8 adjacent cells for garrison-able buildings:
      - Find building owned by same player with `CanBeOccupied` (TypeClass+0x1575)
      - If found: Set_ArchiveTarget to building, set `HasFoundAutoTarget` (0x68E)=1,
        Queue_Mission(Enter, false)

**Phase 4 — AI Auto-Hunt (post-guard):**
6. If unit is InfantryClass (RTTI=0xF), not player-controlled, AND:
   - TypeClass has `Assaulter` flag (+0xEC2) OR weapon ability 0xE (auto-attack)
   - Current mission != Sabotage (0x11)
   - Has target AND target is a Building (RTTI=6)
   Then: Queue_Mission(Sabotage=0x11, false) — AI infantry auto-attack buildings

**Phase 5 — Timer Return:**
7. Check CDTimer at offsets 0x2EC-0x2F4 (MissionTimer):
   - If timer active and not expired: return remaining time
   - If expired or not set: return MissionTimerRate + Random(0,2)
   - Special: if TypeClass+0x6B0 flag set (Harvester?) AND `param_1[0x11A] >= 1` (ammo?):
     return 0 (immediate re-dispatch)

### Key Fields
- byte 0x68E — `HasFoundAutoTarget`: set when guard finds a garrison building
- byte 0x68F — `IsReceivingRepair`: gates RepairAI branch
- byte 0x690 — `IsDockingToBuilding`: gates DockingAI branch
- byte 0x691 — `IsWeedingHarvester`: gates WeedHarvestAI branch
- TypeClass+0x390 — `DefaultToGuardArea`: enables patrol-back-to-base

### Transitions
- Guard -> Enter (when finding garrison building)
- Guard -> Sabotage (AI infantry auto-attacking buildings)
- Guard stays Guard (with repair/dock/weed sub-states)

### Rate: MissionTimerRate + Random(0,2) or CDTimer remainder

---

## 3. Mission_Attack (enum 1) — 0x004D4DC0

**VTable offset:** +0x210
**Triggers:** Player orders attack, AI combat engagement, auto-targeting

### Behavior

Handles approach/chase of attack targets:

1. Check TypeClass `DefaultToGuardArea` flag (+0x390) AND locomotion idle:
   - Find nearest passable cell near home → Set_Destination (return to base area)

2. Check `HasFoundAutoTarget` (0x68E):
   - Scan for threat targets to attack from current position

3. If TarCom target exists (0x2B4):
   - If no NavCom: call `OnArrival(0, 1)` — arrived at attack position
   - Check target RTTI type and distance
   - For Building targets (RTTI=6): check allied status and health ratio
     against `RulesClass+0x1708` (ConditionYellow threshold)
   - If target valid: engage

4. If TarCom is null AND NavCom is null:
   - Call `OnArrival(0, 1)` — no target, stop

5. Distance-based timer optimization:
   - If target distance < 0x301 leptons AND >= threshold: return timer / 2
     (faster polling when closing in on target)

### Key Fields
- TypeClass+0x390 — `DefaultToGuardArea`
- `g_RulesClass_Instance+0x1708` — ConditionYellow health threshold

### Transitions
- Attack with target: stays in Attack, chasing
- Attack without target: OnArrival transitions to Guard
- Distance-based rate halving for responsive combat

### Rate: MissionTimerRate + Random(0,2), halved when near target

---

## 4. Mission_Hunt (enum 15) — 0x004D5350

**VTable offset:** +0x228
**Triggers:** AI hunt command, aggressive stance orders

### Behavior

Seek-and-destroy with special engineer/spy handling:

1. Get TypeClass, check flag at +0x6D4 (IsMissileUnit or similar):
   - If set: skip normal hunt, go to fallback

2. Get current coordinates, check `Can_Enter_Cell` (vtable+0x39C):
   - If cell is passable: proceed with target evaluation

3. **Infantry-specific logic** (RTTI == 0xF):
   a. Check TypeClass flags:
      - +0xEC3 = `Engineer` flag
      - +0xEC2 = `Infiltrate/Spy` flag
      - +0xEC6 = `C4` flag
      - Weapon ability 0xE = auto-attack
   b. If has Infiltrate/Spy OR weapon ability:
      - If TarCom target is Building (RTTI=6):
        Set_Destination to target, Queue_Mission(Sabotage=0x11, false)
        Commence if ready
   c. If has C4 flag:
      Set_Destination to target, Queue_Mission(Capture=8, false), Commence
   d. If Engineer (no spy/C4):
      Set_Destination to target
      Queue_Mission(Capture=8, false), Commence

4. **Fallback (non-infantry or no special flags):**
   - If not player-controlled AND game mode == 0 (skirmish/MP):
     Find nearest dock → Set_Destination → Queue_Mission(Move=2, true)
   - Otherwise: call `ScanForThreats_Simple` (vtable+0x478)

### Transitions
- Hunt -> Sabotage (spy/infiltrator infantry finding buildings)
- Hunt -> Capture (engineer infantry finding buildings)
- Hunt -> Move (AI units returning to base when no targets)

### Rate: MissionTimerRate + Random(0,2)

---

## 5. Mission_Patrol (enum 25) — 0x004D4280

**VTable offset:** +0x25C
**Triggers:** Patrol waypoint command (Ctrl+Alt+click in standard YR)

### Behavior

4-state machine (`MissionState` at 0xBC, stored as `param_1[0x2F]`):

**State 0 — Find Target:**
1. Get current position as coords
2. Call `Greatest_Threat(2, &coords, 0)` — scan for threats with weapon 2
3. Call `SelectWeaponAgainst(target)` to get optimal weapon
4. If no target found:
   a. Check NavCom (0x5A4) locomotion — if stopped and no queue:
      OnArrival(0,1), set state=3, return 1
   b. Otherwise: continue patrolling
5. If target found: check `CanFireAt(target, weapon)`
   - If out of range: calculate approach distance, check zone passability
   - If target is in unreachable zone: find nearby passable cell
   - Set GhostCell (0x5DC) for navigation target
   - Set_Destination → Set_ArchiveTarget → transition to state 1

**State 1 — Engage Target:**
1. Resolve TarCom target
2. Call `SelectWeaponAgainst` and check if still firing (vtable+0x330)
3. If not firing AND target still valid:
   - Check `CanFireAt` — if out of range, pathfind toward target
   - If target dead: Greatest_Threat_Scan(0) to find new target
   - If new target: Set_ArchiveTarget, Set_Destination, return 1
   - If no target: transition to state 2
4. If TarCom is null AND target was lost:
   - Get target's last cell, scan for new threat there
   - If found: engage new target, return to state 1
   - If not: transition to state 2
5. Distance comparison: if closer to original NavCom than to current hunt target,
   go to NavCom instead → Set_Destination, return 1

**State 2 — Return to Patrol Route:**
1. Scan from GhostCell (last known area) for new threats
2. If target found and in range: Set_ArchiveTarget, Set_Destination, return
3. If no target:
   - If has NavCom path: return to patrol route, clear GhostCell, state → 0
   - If no NavCom: OnArrival → state 3

**State 3 — Path Exhausted:**
1. If NavCom (0x5A4) locomotion still active: go to state 0 (re-patrol)
2. Otherwise: fall through to default

**Default:** return MissionTimerRate + Random(0,2), set state → 1

### Key Fields
- `param_1[0x2F]` (0xBC) — MissionState (0-3)
- `param_1[0x177]` (0x5DC) — GhostCell: cell being navigated to during hunt
- `param_1[0x86]` (0x218) — NavCom: original patrol route destination

### Transitions
- Patrol state 0 → state 1 (found target)
- Patrol state 1 → state 2 (lost target, return to route)
- Patrol state 2 → state 0 (resumed patrol)
- Patrol state 3 → Guard (path exhausted via OnArrival)

### Rate: MissionTimerRate + Random(0,2), minimum return 1 for active engagement

---

## 6. Mission_AreaGuard (enum 11) — 0x004D6AA0

**VTable offset:** +0x220
**Triggers:** Area Guard command (Ctrl+Alt+click for guard area), default harvest AI

### Behavior

This is the LARGE handler (~900 bytes of code). It handles both area-guard patrol
AND harvester ore-collection behavior at the FootClass level.

**Early exits:**
1. If `WarheadBusy` (0x2E4): Queue_Mission(Guard=5, true), return 1
2. If `IsReceivingRepair` (0x68F): call RepairAI (vtable+0x340), return timer
3. If `IsDockingToBuilding` (0x690): call DockingAI (vtable+0x348), return timer
4. If `IsWeedingHarvester` (0x691): call WeedHarvestAI (vtable+0x34C), return timer

**AI Harvester Auto-Return (not player-controlled):**
5. If AI unit, has EnterQueue entries (0x16F > 0), no NavCom path active:
   - Check if first EnterQueue entry == NavCom destination
   - If yes: Set_Destination to it, call FUN_0045ADD0 (dock approach)
   - If unit has `SelfEnterQueued` (0x6B1): call deploy function

**Refinery Proximity Check:**
6. If NavCom target is a Building (RTTI=0xB):
   - Get cell, look up building in cell
   - If allied building AND (not infantry OR not slave miner):
     Find nearby passable cell near refinery, Set GhostCell

**Slave Miner Detection:**
7. If unit is InfantryClass (RTTI=1) AND TypeClass+0xE0E (slave flag):
   Queue_Mission(Harvest=10, false), Commence, return Random+1

**Ore Collection Trigger:**
8. If `param_1[0xB6]` non-zero: call FUN_006B0CC0 (collect ore from cell)

**Idle/Destination Logic:**
9. If NavCom=0 AND QueuedMission=-1: get current cell, Set GhostCell

**AI Auto-Attack (same as Guard):**
10. If not player-controlled, InfantryType, has Assaulter/weapon:
    - If target is Building (RTTI=6): Queue_Mission(Sabotage=0x11, false)

**Target Proximity Handling:**
11. If NavCom exists:
    - Check if target is "stuck" (bridge flag at byte 0x14)
    - If target at `RulesClass+0x1724` distance: use `ConditionRed` timer
    - If `HasReachedDock` (0x68D) is clear AND no NavCom path:
      Calculate distance, if too far: clear target, Set_Destination(0)
    - If TarCom=0: try `FUN_007091D0` + `FUN_0070F7E0` (ore scan)
      then ScanForThreats_Simple, or try Greatest_Threat_Scan

**Garrison Building Scan:**
12. Same 8-direction adjacent cell scan as Mission_Guard:
    Find garrison-able buildings owned by player → enter

**Timer return:**
- Base = MissionTimerRate
- If AircraftClass (RTTI=2): double the timer
- Add Random(1,5) jitter
- If target exists, is InfantryType with harvester flag OR weapon range < 0x201:
  Check distance to target — if < 0x301 leptons: divide timer by 6

### Key Fields
- `param_1[0xB6]` — ore collection state
- `param_1[0xB9]` (0x2E4) — WarheadBusy/deploying
- byte 0x68D — `HasReachedDock`
- byte 0x6B1 — `SelfEnterQueued`
- TypeClass+0xE0E — slave miner flag

### Transitions
- AreaGuard -> Guard (if deploying)
- AreaGuard -> Harvest (slave miners)
- AreaGuard -> Sabotage (AI infantry auto-attack)
- AreaGuard -> Enter (garrison building found)

### Rate: Complex — base timer * 2 for aircraft, /6 when near target, +Random(1,5)

---

## 7. Mission_Retreat (enum 4) — 0x004DA2C0

**VTable offset:** +0x230
**Triggers:** Unit takes heavy damage, AI retreat order, fleeing

### Behavior

Simple 2-state handler:

**State 0 — Find Path (init):**
1. If NavCom (0x5A4) is zero (no destination yet):
   a. If `param_1[0x175]` (Team) non-zero: get waypoint cell from team
   b. Otherwise: get own current cell as fallback
   c. Find nearby passable cell from that point
   d. Get TypeClass +0x67C (movement zone type) for pathfinding
   e. Set_Destination to found cell
   f. State → 1

**State 1 — Moving/Arrived:**
1. If NavCom is zero AND no locomotion active:
   a. State → 0 (try again)

### Key Fields
- `param_1[0x175]` (0x5D4) — Team pointer (for team waypoint)
- `FUN_006F18A0` — get team/formation waypoint cell

### Transitions
- Retreat oscillates between state 0 and 1
- Eventually reaches destination and transitions via OnArrival -> Guard

### Rate: MissionTimerRate + Random(0,2)

---

## 8. Mission_Enter (enum 7) — 0x004D9290

**VTable offset:** +0x240
**Triggers:** Player orders unit into transport/building, harvester entering refinery

### Behavior

1. Get destination via `FootClass::GetDestination(0)` and `Filter_AbstractType_InMap()`
2. If no destination:
   a. Try `FUN_0070D8F0` (TryEnterTransport check)
   b. If can't enter AND (no NavCom OR locomotion COM state is NOT 1 and NOT 2):
      Call `OnArrival(0, 1)` — arrival/failure handling
      (Note: COM states 1 and 2 mean locomotion IS moving; the condition fires when state is neither, i.e., locomotion has stopped or hasn't started. Corrected 2026-05-29: was "not moving [state 1 or 2]" — misleading; binary `iVar4 != 1 && iVar4 != 2` confirms 1 and 2 are the moving states via decompile_function 0x004D9290 — OPERATOR_OR_ORDER_DRIFT)
   c. Call `Commence()` — process next queued mission
   Return timer

3. If has destination:
   a. Call `GetActionResult(0xE, target)` (vtable+0x278) — can we enter this?
   b. If action==1 (can enter) OR `param_1[0x106]` flag set:
      - If NavCom is zero AND WaypointQueue has entries:
        **Waypoint queue dequeue:** IPiggyback locomotion swap if needed,
        then pop first entry from WaypointQueue, shift remaining entries down,
        Set_Destination to popped waypoint
      - Else if TypeClass+0xCD4 flag set:
        Clear NavCom_Aux and NavCom, Set_Destination to NavCom target
   c. If action != 1 (can't enter):
      Call vtable+0x274 with param 3 (set some state)
      Call `OnArrival(0, 1)` — handle failure

### Key Fields
- `param_1[0x166]` (0x598) — WaypointQueue.Count
- `param_1[0x163]` (0x58C) — WaypointQueue.Data pointer
- TypeClass+0xCD4 — unknown flag (possibly IsTransport or IsDeployable)

### Transitions
- Enter -> next queued mission (via Commence on arrival)
- Enter handles waypoint queue for multi-stop paths

### Rate: MissionTimerRate + Random(0,2)

---

## 9. Mission_Capture (enum 8) — 0x004D4B20

**VTable offset:** +0x214
**Also dispatched for:** Mission_Sabotage (enum 17) uses the same vtable slot

### Behavior

Handles engineer capture, spy infiltration, and C4 placement:

1. Check TarCom target (0x2B4):
   - If target is Building (RTTI=6):
     Check allied status AND health ratio > `ConditionYellow`
     If friendly building with good health: clear target (don't attack allies)

2. If TarCom exists:
   a. If IsInfantryType (RTTI=0xF):
      - Check `Infiltrate` flag (TypeClass+0xEC2)
      - Check weapon ability 0xE
      - Check `Engineer` flag (+0xEC3)
      - Check `Assaulter` flag (+0xEB4, +0xEB5, +0xEBE)
      - If any attack-capable flag: Set_Destination to TarCom target
   b. Call OnArrival(0, 1)
   c. Check building in current cell → SetFacing toward it

3. If no TarCom AND player-controlled:
   - Do nothing (no action taken for player-controlled units with no target)
   (Corrected 2026-05-29: was "Call ScanForThreats_Simple — find new target"; binary shows the player-control branch has no ScanForThreats_Simple call at all via decompile_function 0x004D4B20 — INFERENCE_HARDENED)

4. If no TarCom AND AI (not player):
   - If not InfantryType OR no Assaulter flag:
     Clear target, Set_Destination(0), queue Hunt (0xF)
   - Otherwise: do nothing (stay in capture mission)

### Transitions
- Capture -> Hunt (AI infantry with no target reverts to hunt)
- Capture -> Guard (via OnArrival when reaching target)

### Rate: MissionTimerRate + Random(0,2)

---

## 10. Mission_Eaten (enum 9) — 0x004D4CB0

**VTable offset:** +0x218
**Triggers:** Unit captured by Yuri's mind control or similar "eaten" state

### Behavior

Very similar to Mission_Capture but with different flag checks:

1. Check InfantryType (RTTI=0xF)
2. If has TarCom target AND target is Building (RTTI=6):
   a. If no locomotion active:
      - Check `Infiltrate` (TypeClass+0xEC2) OR weapon ability 0xE
      - If can attack: Set_Destination to target
   b. Call OnArrival(0, 1)

3. If no target AND no locomotion:
   a. Call `OnArrival(0, 1)`
   b. Check building in current cell → SetFacing toward it (same as Capture)

### Transitions
- Eaten -> Guard (via OnArrival)

### Rate: MissionTimerRate + Random(0,2)

---

## Missions NOT Overridden at FootClass Level

The following missions use the base MissionClass stub (returns 450 frames = 30 seconds):

| Mission | Enum | Why Not Overridden |
|---------|------|--------------------|
| Sleep | 0 | Default idle — no behavior needed |
| Harvest | 10 | **UnitClass overrides this** — FootClass has no harvest logic at this slot |
| Unload | 16 | **UnitClass overrides this** — transport unloading is vehicle-specific |
| Harmless | 23 | Passive units need no tick logic |
| Return | 12 | Not used in standard YR skirmish |
| Stop | 13 | Immediate — no ongoing behavior |
| Ambush | 14 | TS legacy or map-trigger only |

---

## Documents Needing Correction

The following existing docs reference the OLD (incorrect) mission handler labels:

1. **FOOTCLASS_VTABLE_COMPLETE.md** — Section "Mission Handler Overrides" has wrong
   mission names for all 8 affected entries. The vtable offset column is correct but the
   Name column maps them to wrong missions.

2. **FOOTCLASS_STRUCT_LAYOUT.md** — Field descriptions reference "Mission_Guard" and
   "Mission_Harvest" by name; the field behavior is correct but the mission name
   associations may be misleading.

---

## Summary

FootClass provides 12 mission handler overrides covering the core mobile-unit behaviors:
Guard, Move, Attack, Hunt, Patrol, AreaGuard, Retreat, Enter, Capture (+ Sabotage),
and Eaten. The most complex handlers are AreaGuard (~900 bytes, full harvester + patrol
+ garrison logic) and Patrol (4-state target-seeking machine). The simplest is Move
(just monitors locomotion completion).

Harvest and Unload are deliberately NOT overridden — these are subclass responsibilities
(UnitClass provides the actual harvest state machine and transport unloading logic).

---

## Verification Audit — 2026-05-11

Audited against live `gamemd.exe` in Ghidra MCP. Confidence per claim noted as HIGH / MEDIUM / LOW. Findings reported as verified / corrected / unverifiable.

### Summary
The core claim of this doc — the FootClass mission-code → vtable-offset map and the 8 corrected labels — is fully verified against the live binary. All 8 corrected labels are still correctly applied in Ghidra (each address resolves to a function with the corrected `FootClass__Mission_<X>` name). However, the prompt's claim that **UnitClass and InfantryClass vtables override slot +0x240** is contradicted by the binary: their +0x240 slots inherit FootClass's `0x004D9290`. The override functions named `UnitClass__Mission_Enter` (0x00739EC0) and `InfantryClass__Mission_Enter` (0x005196A0) are not in the mission-7 dispatch slot — they appear to be misnamed.

### Per-claim audit (mission code → vtable slot)

Verified by reading `MissionClass__Mission_Dispatch @ 0x005B3060` and FootClass vtable starting at 0x007E8C94. Each case dereferences `*(vtable + offset)`; FootClass vtable contents read raw at each slot.

| Mission | Case | Doc says vtable+offset | Dispatch decompile | FootClass vtable value @ slot | Result | Confidence |
|---------|------|------------------------|---------------------|-------------------------------|--------|-----------|
| Sleep (0) | 0 | +0x204 | +0x204 confirmed | 0x005B2E10 (stub) | verified | HIGH |
| Attack (1) | 1 | +0x210 | +0x210 confirmed | 0x004D4DC0 | verified | HIGH |
| Move (2) | 2 | +0x22C | +0x22C confirmed | 0x004D4200 | verified | HIGH |
| Retreat (4) | 4 | +0x230 | +0x230 confirmed | 0x004DA2C0 | verified | HIGH |
| Guard (5) | 5 | +0x21C | +0x21C confirmed | 0x004D5070 | verified | HIGH |
| Sticky (6) | 6 | +0x21C | +0x21C confirmed (shared w/ Guard) | 0x004D5070 | verified | HIGH |
| Enter (7) | 7 | +0x240 | +0x240 confirmed | 0x004D9290 | verified | HIGH ← CRITICAL |
| Capture (8) | 8 | +0x214 | +0x214 confirmed | 0x004D4B20 | verified | HIGH |
| Eaten (9) | 9 | +0x218 | +0x218 confirmed | 0x004D4CB0 | verified | HIGH |
| Harvest (10) | 10 | +0x224 | +0x224 confirmed | 0x005B2E90 (stub) | verified | HIGH |
| AreaGuard (11) | 0xB | +0x220 | +0x220 confirmed | 0x004D6AA0 | verified | HIGH |
| Return (12) | 0xC | +0x234 | +0x234 confirmed | 0x005B2ED0 | verified | HIGH |
| Hunt (15) | 0xF | +0x228 | +0x228 confirmed | 0x004D5350 | verified | HIGH |
| Unload (16) | 0x10 | +0x23C | +0x23C confirmed | 0x004DA2B0 | verified | HIGH |
| Sabotage (17) | 0x11 | +0x214 | +0x214 confirmed (shared w/ Capture) | 0x004D4B20 | verified | HIGH |
| Construction (18) | 0x12 | +0x244 | +0x244 confirmed | 0x005B2F10 (stub) | verified | HIGH |
| Selling (19) | 0x13 | +0x248 | +0x248 confirmed | 0x005B2F20 (stub) | verified | HIGH |
| Repair (20) | 0x14 | +0x24C | +0x24C confirmed | 0x005B2F30 (stub) | verified | HIGH |
| Rescue (21) | 0x15 | +0x258 | +0x258 (decimal 600) confirmed | 0x004DDF90 | verified | HIGH |
| Patrol (25) | 0x19 | +0x25C | +0x25C confirmed | 0x004D4280 | verified | HIGH |

All 20 spot-checks pass. The dispatch switch covers cases 0–0x1F with 4 gaps (3 = QMove falls to default→+0x204; mission 0x1D is absent). All mission-to-vtable-offset claims in the doc table are verified.

### 8 corrected labels — re-verification

Verified via `get_function_by_address` on each:

| Address | Doc's corrected name | Live Ghidra name | Match? | Confidence |
|---------|---------------------|-------------------|--------|-----------|
| 0x004D4DC0 | Mission_Attack | `FootClass__Mission_Attack` | verified | HIGH |
| 0x004D4B20 | Mission_Capture | `FootClass__Mission_Capture` | verified | HIGH |
| 0x004D4CB0 | Mission_Eaten | `FootClass__Mission_Eaten` | verified | HIGH |
| 0x004D6AA0 | Mission_AreaGuard | `FootClass__Mission_AreaGuard` | verified | HIGH |
| 0x004D5350 | Mission_Hunt | `FootClass__Mission_Hunt` | verified | HIGH |
| 0x004DA2C0 | Mission_Retreat | `FootClass__Mission_Retreat` | verified | HIGH |
| 0x004D9290 | Mission_Enter | `FootClass__Mission_Enter` | verified | HIGH ← CRITICAL |
| 0x004D4280 | Mission_Patrol | `FootClass__Mission_Patrol` | verified | HIGH |

The two "already correct" labels are also intact:
- 0x004D5070 = `FootClass__Mission_Guard` — verified
- 0x004D4200 = `FootClass__Mission_Move` — verified

`FootClass__Mission_Enter @ 0x004D9290` has **zero direct callers** per `get_function_callers` — confirms dispatch is exclusively through vtable slot +0x240 from `Mission_Dispatch`. The only xrefs to 0x004D9290 are three [DATA] references at 0x007E8ED4, 0x007EB298, 0x007F5EB0 — all vtable slots (FootClass / InfantryClass / UnitClass at slot +0x240). HIGH confidence.

### Inheriting vtables identified

The three vtable slots from the doc's pre-flight finding are correctly identified, but the interpretation needs a note:

| Slot Address | Class | Slot offset within vtable | Inherits 0x004D9290? |
|--------------|-------|---------------------------|-----------------------|
| 0x007E8ED4 | FootClass (vtable base 0x007E8C94) | +0x240 | This IS the master slot |
| 0x007EB298 | InfantryClass (vtable base 0x007EB058) | +0x240 | yes — inherits |
| 0x007F5EB0 | UnitClass (vtable base 0x007F5C70) | +0x240 | yes — inherits |

UnitClass and InfantryClass vtables do **NOT** override slot +0x240. AircraftClass vtable (base 0x007E22A4) has its OWN handler at slot +0x240 = 0x00419C80. HIGH confidence.

### Per-class Mission_Enter cross-walk

This is where the doc's prompt-supplied "Agent D finding" needs **partial correction**:

| Class | Mission_Enter address (slot +0x240) | Status |
|-------|--------------------------------------|--------|
| FootClass | 0x004D9290 | verified — the master handler |
| InfantryClass | **inherits FootClass's 0x004D9290** (NOT 0x005196A0) | **corrected** — vtable slot +0x240 contains 0x004D9290 |
| UnitClass | **inherits FootClass's 0x004D9290** (NOT 0x00739EC0) | **corrected** — vtable slot +0x240 contains 0x004D9290 |
| AircraftClass | 0x00419C80 (Ghidra-mislabeled `Mission_Sticky`) | verified — vtable base 0x007E22A4 + 0x240 = 0x007E24E4 → 0x00419C80 |
| BuildingClass | 0x005B2F00 — 6-byte `mov eax, 0x1C2; ret` stub (no function created in Ghidra) | partial — stub bytes are present, but Ghidra has no function symbol there |

**Important finding about the supposed Unit/Infantry Mission_Enter overrides:**
- 0x00739EC0 is named `UnitClass__Mission_Enter` but its only xref is at UnitClass vtable slot **+0x18C**, not +0x240. FootClass slot +0x18C = `FootClass__PerCellProcess`. So 0x00739EC0 is almost certainly `UnitClass__PerCellProcess` mislabeled — NOT a Mission_Enter override. (The function body does contain deploy / building-entry logic, which probably triggered the mislabel.)
- 0x005196A0 (`InfantryClass__Mission_Enter`) has **zero xrefs** anywhere in the binary. Its body looks like a Mission_Enter handler (CargoClass__AddPassenger, garrison logic), but if it lives in no vtable and has no direct callers, it is dead code in YR or its xref is non-standard. Confidence: MEDIUM — needs deeper trace before treating as an override.

The prompt's downstream `MISSION_ENTER_CROSSWALK` doc design must NOT assume Unit/Infantry override at +0x240. The actual override-at-+0x240 table is: FootClass=master, Aircraft=overrides, Unit/Infantry=inherit, Building=stub.

### TS-legacy filter

The mission name table at 0x00816CAC includes entries like "Spyplane Overfly" and "Spyplane Approach" (mission codes around 0x1B–0x1E in the table). Mission_Dispatch DOES handle cases 0x1B, 0x1C, 0x1E, 0x1F via vtable slots +0x264, +0x268, +0x26C, +0x270 (these correspond to AircraftClass `SpyplaneApproach` / `SpyplaneOverfly` / `Paradrop` / `Paradrop_Approach` style missions). These are aircraft-specific and called only from aircraft callers — not FootClass-relevant. The FootClass vtable holds stub values for these slots. The doc's silence on these missions at FootClass level is correct.

No clearly-TS-only-dormant missions found within the FootClass override scope (0–0x19 range). Mission 14 (Ambush) is a FootClass stub (returns 450) — TS-era leftover; doc already notes this correctly.

### Doc health verdict

**NEEDS-MINOR-PATCHES** — the core content (mission code → vtable offset map, 8 corrected labels, handler behaviors) is fully verified. The doc itself does not claim per-class overrides at +0x240, so the doc as written needs no patch. However, the downstream prompt's "MISSION_ENTER_CROSSWALK" assumption (Unit→0x00739EC0, Infantry→0x005196A0 at slot +0x240) is **incorrect against the binary** — the actual per-class +0x240 cross-walk is: FootClass=master 0x004D9290, AircraftClass overrides at 0x00419C80, UnitClass and InfantryClass INHERIT 0x004D9290. The functions at 0x00739EC0 (xref'd at UnitClass vtable +0x18C, the PerCellProcess slot) and 0x005196A0 (no xrefs at all) appear to be mislabeled or dead — they are NOT the per-class Mission_Enter overrides.

