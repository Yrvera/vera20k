# Building Systems — Complete Ghidra Decompilation Report

Source: Ghidra decompilation of `gamemd.exe` (Yuri's Revenge 1.001)
Reports referenced: 015, 016, 017, 018, 037, 048, 060, 089, 114, 115, 116, 122, 129, 130, 131

---

## Object Sizes

| Object | Size | Description |
|--------|------|-------------|
| BuildingClass | `0x720` (1,824 bytes) | Runtime building instance |
| BuildingTypeClass | `0x1798` (6,040 bytes) | Type definition parsed from rules.ini |
| FactoryClass | `0x74` (116 bytes) | Production queue per factory |
| AnimClass | `0x1C8` (456 bytes) | Animation instance (building anims, effects) |

---

## 1. Construction & Production Pipeline

### Production Step Accumulator

Production uses a **54-step accumulator** (`0x36` steps). Each step's delay is computed as
`lepton_distance / 54`, clamped to `[1, 255]` frames.

### House Factory Pointers

| House Offset | Factory Type |
|---|---|
| `+0x53AC` | Infantry factory |
| `+0x53B0` | Aircraft factory |
| `+0x53B4` | Building factory (primary) |
| `+0x53B8` | Building factory (naval) |
| `+0x53BC` | Unit factory (primary) |
| `+0x53CC` | Unit factory (naval) |

### Production Flow

```
House::Begin_Production (0x4FA350, 1222 bytes)
  ├─ Allocates FactoryClass (0x74 bytes)
  ├─ Stores factory pointer by RTTI type into house+0x53AC..0x53CC
  ├─ Calls FactoryClass::StartProduction (0x4C9C70, 405 bytes)
  │    ├─ Validates queue capacity and house production limits (Rules+0xF0)
  │    └─ Creates the producing object via vtable+0x8C
  └─ Updates sidebar for local player via FUN_006A6140

FactoryClass::CompletionStep (0x4C9EA0, 272 bytes)
  ├─ Called each frame while production is active
  ├─ Advances step counter toward 0x36 (54)
  └─ Computes delay: lepton_distance / 54, clamped [1, 255]

House::Place_Production (0x4FB0E0, 1426 bytes)
  ├─ Places completed production at target cell
  ├─ Handles building placement, factory exit for vehicles/infantry
  ├─ Deploy logic for auto-deploying buildings
  └─ EVA voices: "EVA_CannotDeployHere", "EVA_UnitReady"
```

### FactoryClass Layout (`0x74` bytes)

| Offset | Type | Field |
|--------|------|-------|
| `+0x24` | int | Production step counter (0 → 0x36 for completion) |
| `+0x38` | int | Production delay/timer |
| `+0x44` | ptr | Queued items array pointer |
| `+0x48` | int | Max queue size |
| `+0x50` | int | Current queue count |
| `+0x58` | ptr | Currently-producing object pointer |
| `+0x60` | int | Cost already paid |
| `+0x68` | int | Suspended production object ID |
| `+0x6C` | ptr | Owning house pointer |
| `+0x70` | byte | "Ready" flag |
| `+0x71` | byte | "Can place" flag |

### Key Factory Functions

| Address | Function | Size | Purpose |
|---------|----------|------|---------|
| `0x4C9C70` | StartProduction | 405 | Begin production of an object |
| `0x4C9E60` | Suspend | — | Suspend production, set ready/can-place flags |
| `0x4C9EA0` | CompletionStep | 272 | Advance production step |
| `0x4C9FF0` | AbandonProduction | 301 | Cancel, refund cost, reset house fields |
| `0x4CA5A0` | StartNextQueued | — | Pop first queue item and begin |
| `0x4CA620` | RemoveFromQueue | — | Remove item by shifting array |
| `0x4CA6B0` | IsInQueue | — | Check if type exists in queue |
| `0x4CA6E0` | UpdateAllStepDelays | — | Recompute delays when cost multiplier changes |

### AI Build Strategy

```
House::AI_Building_Strategy (0x4FD500, 1074 bytes)
  ├─ Selects nearest non-allied house as enemy
  ├─ Tracks threat response: 0=normal, 1=low power, 3=under attack
  ├─ Calls AI_Check_Build_Need (0x4FD9A0, 819 bytes)
  │    └─ If no buildings owned, looks for ConYard via rules 0x8E4, 0x938
  ├─ Calls AI_Manage_Build_Queue (0x4FDD10, 1736 bytes)
  │    ├─ Iterates build queue (house+0x5708, 16 bytes/entry)
  │    ├─ Checks if building can be placed or should be abandoned
  │    └─ Applies upgrades if available
  └─ Calls AI_Choose_Building (0x4FE3E0, 1653 bytes)
       ├─ Reads build-priority queue
       ├─ Evaluates cost vs credits
       ├─ Handles defense placement (checks naval flag)
       └─ Stores chosen type at house+0x564C
```

---

## 2. Placement & Foundation

### Core Placement Function

**BuildingClass::Place / OccupyMap** (`0x441F60`, 718 bytes):
- Gets foundation cell data from `FUN_0045F160`
- Iterates foundation cells (terminated by `0x7FFF` sentinel)
- For each cell: sets overlay flags (`cell+0x44 = 0xEF`), recalculates passability
- If wall type (`TypeClass+0x1767`): calls `FUN_0056BEC0` with height adjustment `0x14`

### Cell Ownership

**BuildingTypeClass::SetOwnerAndOccupy** (`0x543330`, 1050 bytes):

| Mode | Action |
|------|--------|
| 0 | Remove: clears cell owner to `0xFFFF`, subtracts height |
| 1 | Place: sets cell owner to building type ID, increments height |
| 3 | Place (variant): same as mode 1 |

Cell fields affected:
- `cell+0x38` = owner building type index
- `cell+0x11A` = sub-position byte (0–9 for multi-cell buildings)
- `cell+0x11B` = cumulative height level (15 px per level)

Bridge overlay handling: all overlays with indices `0x1A–0x25` (bridge-related) are destroyed during placement.

After placement, updates pathfinding in 4 cardinal directions via `FUN_0047CA80`/`FUN_00481810`, then `FUN_0047D2B0(-1)` for full passability recalc.

### Foundation Lookup

Foundation dimensions are stored in lookup tables indexed by foundation type enum:
- Width: `DAT_008192B8[foundation_type]`
- Height: `DAT_00819310[foundation_type]` (adds 1 if HasBib at `TypeClass+0x1570`)

### Placement Validation

**BuildingTypeClass::CanBePlacedAt** (`0x45EE70`):
- Gets foundation cell list via `vtable+0x90`
- For each foundation cell: checks in-bounds (`FUN_00568300`), zone match, occupant, wall adjacency, overlay placement
- Returns: 0 = ok, 1 = partial, 2 = blocked

---

## 3. The 21-Slot Animation System

Every building has **21 animation slots**, each `0x44` bytes (68 bytes) in BuildingTypeClass starting at offset `0xF4C`. Each slot can hold an independent AnimClass instance.

### Central Animation Update

**UpdateAnimation** (`0x4509D0`, 2387 bytes) — called per-tick:

1. **Production frame counter** (`[0x3E–0x44]`): `[0x43]` = delay, `[0x44]` = direction
2. **Facing/turret rotation**: `FUN_00456FB0`, `FUN_00451F60`/`FUN_00452170`
3. **Active anim slot 0xC**: based on ammo/reload state (`FUN_00473460`)
4. **Turret anim** (`TypeClass+0x16A8`): maps turret facing to 4+ frame indices
5. **Special anim** (`TypeClass+0x16BB`): turret-driven
6. **Superweapon charge bar**: checks charge accumulation vs `TypeClass+0x16E8` threshold
7. **Production frame cycling**: checks frame bounds, sets completion flag `[0x6DD]`
8. Sets NeedRedraw flag (`[0x20]`), updates shadow anim facing via `DAT_007F4890` lookup

### Slot Management Functions

| Address | Function | Purpose |
|---------|----------|---------|
| `0x451890` | CreateAnimForSlot | Workhorse — called by 18 functions; creates AnimClass for slot |
| `0x451E40` | ClearAnimSlot | Clears one slot or all 21 (param = `0xFFFFFFFE`) |
| `0x451EE0` | SetDamagedState | Updates all slots when health crosses ConditionYellow |
| `0x451F60` | UpdateAnimFacingAndDirection | Updates all 21 with facing + shadow direction |
| `0x452000` | UpdateAllAnimFacings | Recalculates facing from scratch for all 21 |
| `0x452170` | SetAnimRemap | Sets remap/palette on all 21 anims |
| `0x4521C0` | StartCloaking | Sets all 21 anims translucent, shrinking |
| `0x452210` | StopCloaking | Sets all 21 anims opaque, expanding |

### Anim Image Selection

**SetAnimSlotImage** (`0x451750`):
- param_3: 0 = undamaged, 1 = damaged
- param_4: 1 = firing
- Reads from TypeClass offsets `0xF4C`/`0xF5C`/`0xF6C` + `(slot * 0x44)`

### Animation State Change Handlers

| Address | Trigger | Action |
|---------|---------|--------|
| `0x4545D0` | Power off | Clear slot 0x14; for each slot check flags 0xF8C/0xF8D/0xF8E |
| `0x454730` | Special anim start | Clear slot 0x13; create anims where flag 0xF8F set |
| `0x4547C0` | Power on | Detach/recreate anims based on flag bytes |

---

## 4. Power System

### Power Output Calculation

**GetPowerOutput** (`0x44E7B0`):
```
base = TypeClass+0xEE0
if veteran:  base += TypeClass+0xEE8
if upgrades: base += TypeClass+0xEE8 * upgrade_count  (from [0x17B..0x17D])
return base  (only if [0x198] set = operational)
```

### Power Drain Calculation

**GetPowerDrain** (`0x44E880`):
```
base = TypeClass+0xEE4
if flag [0x669]: base += TypeClass+0xEEC
base += sum(upgrade drains)
return base  (0 if not operational)
```

### Power State Transitions

**GoOnline** (`0x452260`):
- Sets `[0x660] = 1` (powered)
- Creates production anim (`FUN_00554A60`)
- Updates wall connectivity (`FUN_004533A0`)
- Attaches weapon anims (`FUN_00425270`)
- Updates radar (`FUN_00509140`)
- Plays EVA "BuildingOnLine"
- Sets house flags `+0x5778/0x5779/0x1FC`

**GoOffline** (`0x452360`):
- Clears `[0x660]`
- Destroys production anim, firewall anim, weapon anims
- Deactivates gap generator, radar jammer
- Plays EVA "BuildingOffLine"

---

## 5. Wall / Laser Fence / Firewall Connectivity

### Connection Algorithm

**ConnectWalls** (`0x452A40`, on placement):
1. Iterate 4 cardinal directions using `DAT_0089F688` offset table
2. For each neighbor cell: look up building (`FUN_0047C520`)
3. Check: is laser fence (`+0x16BF`), same owner (`vtable+0x38`), matching orientation
4. OR connection bitmask into `[0x186]` using `DAT_00818CA0` table
5. Call `FUN_00453060` for each connected neighbor

### Full Recalculation

**RecalculateWallConnections** (`0x4533A0`, 1151 bytes):
- For all 4 cardinal directions: finds neighboring wall posts
- Determines connection type based on power state checks:
  - `vtable+0x350` (IsPowered) AND `[0x6EA]` (IsActive) AND not selling
- Sets `[0x186]` to overlay frame index:

| Frame | Topology |
|-------|----------|
| 0–2 | Endpoint variants |
| 3–5 | Straight + powered/unpowered |
| 6 | T-junction |
| 7–8 | Cross + powered/unpowered |

### Wall Segment Auto-Creation

**ExtendWallInDirection** (`0x452DC0`, 660 bytes):
- Searches for nearest fence post (`FUN_00452BB0`)
- If found and not selling, creates wall segments between posts
- Each segment: allocates `0x720` bytes, calls BuildingClass constructor

### Wall Destruction Propagation

**OnWallDestroyed** (`0x453240`):
- Finds connected segments
- Destroys or reconnects depending on direction flags

---

## 6. Gap Generator

### 4-State Radius Animation

State machine at building offset `[0x220]`:

| State | Name | Behavior |
|-------|------|----------|
| 0 | Collapsed | Off; optionally creates locomotor anim |
| 1 | Expanding | Counter 0→15 in steps; redraws at steps 1/6/11 |
| 2 | Full | Active shroud; checks `vtable+0x2A4` |
| 3 | Contracting | Counter 15→0; redraws at 0/5/10 |

### Implementation

**UpdateGapGenerator_Tick** (`0x454DB0`, 2076 bytes):
- Propagates shroud level to 21 sub-objects (`[0x157..0x16B]` writing to `+0x178`)
- Per-cell shroud bitfield: `cell+0xDC |= (1 << player_index)` (from `HouseClass+0x30`)
- Updates HouseClass reveal bounds (`+0x5754/0x5758/0x575C/0x5760`)
- Proximity-based gap propagation: checks `(dx² + dy² < (range+2)² * 4)` for nearby buildings

**UpdateGapAndSpecialEffects** (`0x4549B0`) — per-tick update:
- Powered ON: starts ambient sound, activates gap generator, radar jammer
- Powered OFF: stops ambient, collapses gap, deactivates jammer

---

## 7. Garrison System

### Enter/Exit

**EnterTransport** (`0x70FD70`, 207 bytes):
```
transport+0x1D0 = this_unit     // occupant pointer
this[0x73]      = transport     // back-link
transport->cell+0x5778 = 1     // garrisoned cell flag
```
- Allocates AnimClass (`0x1C8` bytes) with type `0x600`
- Updates animation link

**ExitTransport** (`0x70FE50`, 86 bytes):
- Clears `transport+0x1D0`, `this+0x1CC`
- Clears garrison cell flag

**EjectOccupants** (`0x4575B0`):
- Loops while `[0x702] > 0` (occupant count)
- For each: gets infantry pointer, calls `vtable+0xB8` (exit), updates cash, cleanup

### Garrison Fire

**UpdateGarrisonFire** (`0x43E7B0`):
- Checks visibility (`[0x83]`)
- Checks garrison present
- Fires bullet via `FUN_004AED70` with flags `0xE00`, range 1000

### Building Occupancy Limits

- MaxNumberOccupants: `TypeClass+0x1580`
- CanBeOccupied flag: `TypeClass+0x157B`
- CanOccupyFire flag: `TypeClass+0x157C`
- ShowOccupantPips: `TypeClass+0x157D`
- NumberImpassableRows: `TypeClass+0x1620`

---

## 8. Spy Infiltration

**OnSpyInfiltrate** (`0x4571E0`, 965 bytes):

| TypeClass Flag | Effect | EVA |
|---|---|---|
| `+0x16A4` | Reset radar | "EVA_RadarSabotaged" |
| `+0xEE0 > 0` | Steal technology (sets `target+0x2BD/2BE/2BC/1FC`) | "EVA_TechnologyStolen" / "EVA_NewTechnologyAcquired" |
| `+0x16F0 != -1` | Infiltrate weapon (calls `FUN_006CE0B0`) | — |
| `+0x800 > 0` | Steal cash (calls spend/add money) | "EVA_CashStolen" |
| `+0xEB8 == 0x28` | Power sabotage (sets `[0x2C0]`) | "EVA_EnemyBasePoweredDown" |
| `+0xEB8 == 0x10` | Radar sabotage (sets `[0x2BF]`) | "EVA_BuildingInfRadarSabotaged" |

Checks `[0x87] == param_2` to prevent self-infiltration.

---

## 9. Sell / Undeploy

### Can Sell Check

**CanSellOrUndeploy** (`0x4555D0`):
- Returns false if: in construction, in limbo, no health, Unsellable (`+0x1552`), mission is Construction (`0x12`) or Selling (`0x13`)
- Also checks PoweredSpecial timer conditions

### Sell Sequence

**SellBuilding** (`0x457DE0`, 1029 bytes):
1. Find exit cell by iterating adjacent cells (passability check)
2. Iterate stored occupants (`[0x688]` with count `[0x694]`)
3. For each: eject via `vtable+0xF8` or send to sell point
4. Call `FUN_0070F6E0` for sell visual effects
5. Fallback: `SpawnUnitsWithParachute` (`0x4585C0`) if no ground path

### Civilian Transfer

**CheckAutoSellOrCivilian** (`0x458200`, 301 bytes):
- If no occupants and not civilian: plays "EVA_StructureAbandoned"
- Transfers to Civilian house via `vtable+0x3D4`
- If occupied and player-owned: transfers to first occupant's house

### Survivor Spawning

**SpawnSurvivors** (`0x442D90`, 1651 bytes):
- Engineers/technicians from `TypeClass+0x16AE/0x16AF` flags
- Spawns random infantry survivors at foundation cells
- Spawns debris particles via `FUN_006B59A0`/`FUN_006B5C90` (50/50 random)
- String ref: "Creating survivor type '%s' from building type '%s'"

---

## 10. Repair & Production State Machine

### The Mega Function

**MissionRepairAndProduce** (`0x44B780`, **4605 bytes** — largest building function):

| Building Type Flag | Behavior |
|---|---|
| IsBaseDefense (`0x16B9`) | Weapon stage + anim play |
| IsCloning (`0x16C1`) | Accumulator check: `GameSpeed * 0x16F0 * _DAT_007E27F8`; on complete → `vtable+0x274` |
| IsGrinder (`0x16C2`) | Accumulator; calls cost refund functions |
| IsRepairDepot (`0x16A9`) | 3-phase: setup → active repair → completion; EVA "Repairing"/"UnitRepaired"/"InsufficientFunds" |
| IsBarracks (`0x16AA`) | Unit exit with direction assignment |

### Auto-Repair Logic

**UpdateRepairAndPower** (`0x450630`, 915 bytes):
- Checks health against thresholds: `Rules+0x1444`, `Rules+0x1758`
- If health low and conditions met: triggers self-heal (`vtable+0x1A0`)
- When repairing (`[0x1BA]`): periodically heals with TypeClass repair amount
- Switches anims at ConditionYellow threshold

---

## 11. Superweapon Deployment from Buildings

### Mission_Missile State Machine

**FUN_0044C980** (`3105 bytes`):

#### Psychic Dominator Path (`TypeClass+0x16BA`)

| State | Action |
|---|---|
| 0 | Create PSIWARN animation |
| 1 | Wait for anim completion |
| 2 | Fire superweapon with sin/cos targeting; create PULSBALL projectile |

#### Nuclear Missile Path (`TypeClass+0x16C3`)

| State | Action |
|---|---|
| 0 | Face toward target (`FUN_004C9220`) |
| 1 | Create launch animation |
| 2 | Launch missile with trajectory math (distance, angle, velocity normalization) |
| 3–4 | Cleanup |

Timer returns: 1, `0x20` (32), or `0x3C` (60) frames between states.

---

## 12. Docking System (Slave Miner / Aircraft)

### 7-State State Machine

Stored at building offset `[0x718]`:

| State | Action | Details |
|---|---|---|
| 0 | Check docking unit | Scan foundation cells for obstructions, push away |
| 1 | Compute approach | Scan cells; if clear, compute approach angle (`FUN_004CAE30`) |
| 2 | Move to dock | Send locomotor move (`vtable+0x70`); set speed 1.0 (`0x3FF00000`) |
| 3 | Wait arrival | Start 180° turn (`0x8000`) |
| 4 | Wait turn | Play deploy sound from TypeClass |
| 5 | Link | Bidirectional: `[0xB9] = unit`, `unit[0xB9] = building`; set unit idle |
| 6 | Complete | Docked; play map event |

### Undock

**UndockUnit** (`0x4593A0`, 208 bytes):
- Detach locomotor (`ptr[0x19D]→vtable+0x58`)
- Reposition (-0x80, +0x80) facing `0x47`
- Set speed 1.0, clear dock refs

---

## 13. Building Combat

### Weapon Firing

Buildings fire through the standard TechnoClass weapon system:

**TechnoClass::Fire_At** (`0x6FDD50`, 7167 bytes):
- Gets weapon via `vtable+0x3F8`
- Special weapon types: IsNuke (`+0x131`), IsDominator (`+0x142`), IsGatling (`+0x691`)
- Creates BulletClass projectile with scatter/accuracy
- Visual effects: muzzle flash, ElectricBolt, Laser, Beam, RadBeam, scatter particles

### Damage Receiving

**TechnoClass::ReceiveDamage** (`0x701900`, 5154 bytes):
- Applies veterancy defense modifier
- Warhead checks: IsOrganic, IsConventional, NoVsBuildings
- Result codes: 0=survived, 1=wounded, 2=half, 3=quarter, 4=destroyed, 5=obliterated
- On death: spawns debris (MaxDebris from `type+0x5BC`), destroys parasites, removes from team

### Building Destruction

**OnDestroyed** (`0x445880`, 1478 bytes):
- Destroys 8 anim slots
- Adjusts HouseClass counters: PowerOutput (`+0x2D4`), SpyPlanePower (`+0x538C`)
- Handles super weapon reset (`+0x16BE`)
- Clears adjacent cells: walls use 8-directional offsets, bridges use axis-aligned ±1/±3
- Plays destruction sound
- Calls `HouseClass::Recount` (`FUN_004FF980`) and sidebar update

---

## 14. VXL Turret Rendering

**BuildVXLTurretMatrix** (`0x458810`, 432 bytes):
- Constructs 4×4 transformation matrix
- 3 pivot points from TypeClass: `+0x1730/0x1734/0x1738`, `+0x173C/0x1740/0x1744`, `+0x1748/0x174C/0x1750`
- Scale factor from `TypeClass+0x1728` (double)
- Angle conversion: `((*puVar2 >> 10) + 1 >> 1 & 0x1F) - 8` converts palette remap to signed tilt
- Outputs 48-byte (12 float) matrix

**GetTurretDrawPosition** (`0x453BF0`):
- Reads turret offsets: forward (`TypeClass+0x1754`), lateral (`+0x1758`), vertical (`+0x175C`)
- Applies rotation matrix (`FUN_00458810`), matrix×vector (`FUN_005AFB80`)
- Adds building base position, converts to screen coords

---

## 15. Building Frame Selection (SHP)

**GetCurrentFrame** (`0x43EF90`, 484 bytes):
- Checks TypeClass flags: IsGate (`0x16BF`), IsLaserFence (`0x16C0`), HasActiveAnim (`0x16B7`), IsCharged (`0x157B`)
- Returns frame from `[0x186]` (gate frame) or `[0x187]` (laser fence)
- Computes frame from weapon index `[0x14D]` and damage state

---

## 16. Tactical Rendering Order

From the main render pass (`FUN_006D3D10`, 3643 bytes):

| Layer | Function | What |
|---|---|---|
| 1 | `FUN_006D2B60` | Base blit + special cells |
| 2 | `FUN_006D3660` | Shroud edges + icons |
| 3 | `FUN_006D2DE0` | Terrain shadows |
| 4 | `FUN_006D3470` | Base terrain cells |
| 5 | `FUN_006D3290` | Smudges/craters |
| 6 | **`FUN_006D3AC0`** | **Building overlays** (RTTI `0x24`) |
| 7 | `FUN_006D3040` | Overlays (walls, etc.) |
| 8 | `FUN_006D3870` | Animations |

Building rendering (`FUN_006D3AC0`, 319 bytes):
- Iterates object render list (layer `DAT_008A0394`, count `DAT_008A03A0`)
- Filters for RTTI == `0x24`, not invisible
- Clips bounding box vs dirty rect
- Calls `vtable+0x104` (Draw), clears `+0x20` flag

---

## 17. Sidebar Construction UI

### Layout

- 4 strips (tabs), stride `0xF94` bytes each
- 75 item slots per strip, 52 bytes per slot
- 2 columns × N rows, icon size 60×48 pixels, row height 50 pixels
- Active tab stored at `[0x539C]`

### Build Queue UI

**Add new construction option** (`0x6A6300`, 777 bytes):
- Maps RTTI to tab: buildings→tab 3, defense→tab 2, units→tab 0/1, supers→tab 1
- Max 76 items per strip
- Sorted insertion via comparator (`0x6A8420`, 748 bytes):
  1. Super weapons sort by cost, then name
  2. Same factory match
  3. Upgrade status
  4. Tech level
  5. Build cost
  6. Name (wcsicmp)

### Build Progress Rendering

**DrawStrip** (`0x6A9540`, 4210 bytes — largest sidebar function):
- Per slot: draws icon via `FUN_004AED70`
- Unavailable: overlays DARKEN.SHP (semi-transparent flag `0x401`)
- Flashing (completed): blinks frame 8–15 of 16-frame cycle
- Progress bar: GCLOCK2.SHP at current progress frame (0–52)
- Queued count badge: "x3" rendered top-right
- "Ready" / "Hold" text overlay

### Assets Loaded

~25 SHPs: GCLOCK2.SHP, SELL.SHP, REPAIR.SHP, TAB00-03.SHP, R-DN/R-UP.SHP, SIDE1/2/3.SHP, ADDON.SHP, country flags (OBSALLI, OBSSOVI, OBSYURI, etc.)

---

## 18. Rally Points

**SetRallyPoint** (`0x443860`, 806 bytes):
- Reads factory type (`TypeClass+0xEB8`): 3=aircraft, 7=naval
- Validates via pathfinding (`FUN_0056D230`/`FUN_0056DC20`)
- Network event type `0x1E` into ring buffer (`DAT_00A802D4`, max 128, 0x6F bytes each)
- Plays "EVA_NewRallyPointEstablished"

House rally pointers:
- Rally point object: `house+0x53DC`
- Rally point cell: `house+0x53E0`

---

## 19. Gate & Deploy Logic

**ToggleGate** (`0x443B90`, 207 bytes):
- Checks mission `0x13` (Guard)
- Validates factory occupant state
- Handles IsLaserFenceGate (`0x16C4`), IsFirestormWall (`0x16CA`)

**TogglePowerOrGate** (`0x447110`, 247 bytes):
- Checks `[0x6E9]` (deployed/active)
- Checks TypeClass autofire (`0x5B0`)
- Checks `[0x6DF]` (IsSelling)

---

## 20. Save/Load Serialization

### Map Loading

**ReadFromINI** (`0x44F820`, 1651 bytes):
- Parses `[Structures]` INI section
- Format: `owner,type,health,cellX,cellY,direction,tag,upgrade1,upgrade2,upgrade3,flags...`
- Old format (< v4): packed cell coordinates
- New format: separate X,Y
- Creates BuildingClass (0x720 bytes), places via `vtable+0xD8` (Unlimbo)
- Caps health to TypeClass Strength (`+0xA0`)

### Map Saving

**SaveToINI** (`0x44FEC0`):
- Format string: `"%s,%s,%d,%d,%d,%d,%s,%d,%d,%d,%d,%d"`
- Fields: owner house, building type ID, health, cell coords, direction, tag, upgrade names, state flags

---

## BuildingClass Field Map

| Offset | Type | Field |
|--------|------|-------|
| `+0x080` | byte | NeedRedraw flag |
| `+0x0BC` | int | Mission state |
| `+0x148` | ptr | BuildingTypeClass pointer (alias for `+0x520`) |
| `+0x149` | ptr | FactoryClass pointer (alias for `+0x524`) |
| `+0x14D` | int | Weapon stage index |
| `+0x186` | int | Wall/gate connection bitmask (frame index) |
| `+0x187` | int | Laser fence frame |
| `+0x198` | byte | Is operational |
| `+0x199` | byte | Veterancy level |
| `+0x19B` | — | Production timer |
| `+0x1A1` | — | Production timer 2 |
| `+0x1BA` | byte | Is repairing |
| `+0x1BB` | int | Flash rate |
| `+0x1C6` | int | Docking sequence state (0–6) |
| `+0x1D0` | int | Queued action countdown |
| `+0x1D4` | int | Queued action type |
| `+0x20C` | int | Screen Y position |
| `+0x21C` | ptr | HouseClass pointer (owner) |
| `+0x220` | int | Gap generator state (0–3) |
| `+0x2B4` | ptr | Current target |
| `+0x2C0` | int | Rally target |
| `+0x3D3` | byte | Is primary factory |
| `+0x504` | int | Sell/limbo timer |
| `+0x520` | ptr | BuildingTypeClass pointer |
| `+0x524` | ptr | FactoryClass pointer |
| `+0x534` | — | Upgrade count area |
| `+0x55C–0x5AF` | ptr[] | AnimClass* slots [21] |
| `+0x5B0–0x5C4` | byte[] | Anim charging flags [21] |
| `+0x5E8` | ptr | Upgrade building pointers |
| `+0x5FC` | int | Barrel rotation index (0–2, suffixes B/C/D) |
| `+0x614` | ptr | ProductionAnim pointer |
| `+0x618` | int | WallConnectionMask |
| `+0x660` | byte | IsPowered/Online flag |
| `+0x661` | byte | IsDocking flag |
| `+0x66C–0x680` | — | Docking VectorClass (vtable, data, capacity, count) |
| `+0x688` | ptr | Stored occupant array |
| `+0x694` | int | Stored occupant count |
| `+0x6C9` | byte | IsSelling |
| `+0x6CA` | byte | IsNominal |
| `+0x6CB` | byte | IsReadyToCommence |
| `+0x6DD` | byte | AnimComplete flag |
| `+0x6DF` | byte | Sell flag |
| `+0x6E3` | byte | IsElite |
| `+0x6E4` | byte | WasPowered |
| `+0x6E6` | byte | IsDamaged state |
| `+0x6E9` | byte | IsDeployed |
| `+0x6EA` | byte | IsActive (anims enabled) |
| `+0x6EB` | byte | Gap generator active / light flash value |
| `+0x6ED` | byte | Facing/remap index / gap radius counter (0–15) |
| `+0x702` | byte | Garrison occupant count / upgrade count |
| `+0x704–0x714` | — | Superweapon charge data |
| `+0x718` | int | Docking state (0–6) |

---

## BuildingTypeClass Key Offsets

| Offset | Field |
|--------|-------|
| `+0x0024` | Short ID string (e.g. "GAWEAP") |
| `+0x0064` | Full UI name |
| `+0x00A4` | Main SHP pointer |
| `+0x00B8–C4` | Turret/Barrel VXL+HVA (4 pointers) |
| `+0x01F8` | Art section name |
| `+0x022C` | IsTheaterSpecific |
| `+0x0E00` | Theater SHP pointer |
| `+0x0E58` | ToTile set pointer (rubble) |
| `+0x0EA0` | Super overlay type |
| `+0x0EB4` | Adjacent range |
| `+0x0EB8` | FactoryType |
| `+0x0ED4` | ExitCell |
| `+0x0EE0` | Power output |
| `+0x0EE4` | Power drain |
| `+0x0EE8` | Veteran power bonus |
| `+0x0EF0` | Foundation type enum |
| `+0x0F4C` | Anim slot definitions (21 × 0x44) |
| `+0x11B0` | Turret image name |
| `+0x11E0–E8` | Turret draw offsets |
| `+0x1518` | BibShape SHP |
| `+0x1524` | AntiAirValue |
| `+0x1528` | AntiArmorValue |
| `+0x152C` | AntiInfantryValue |
| `+0x1552` | Unsellable |
| `+0x1558` | ProduceCashStartup |
| `+0x155C` | ProduceCashAmount |
| `+0x1560` | ProduceCashDelay |
| `+0x1570` | HasBib |
| `+0x1572` | Capturable |
| `+0x1573` | Powered |
| `+0x1574` | PoweredSpecial |
| `+0x1579` | ClickRepairable |
| `+0x157B` | CanBeOccupied |
| `+0x1580` | MaxNumberOccupants |
| `+0x1620` | NumberImpassableRows |
| `+0x16A4` | Radar |
| `+0x16A5` | SpySat |
| `+0x16A9` | IsRepairDepot |
| `+0x16AA` | IsBarracks |
| `+0x16B7` | HasActiveAnim / IsUndeployable |
| `+0x16B8` | IsHelipad |
| `+0x16B9` | IsBaseDefense |
| `+0x16BA` | IsPsychicDominator |
| `+0x16BB` | HasSpecialAnim |
| `+0x16BD` | IsWeaponsFactory |
| `+0x16BE` | IsWall (super weapon wall) |
| `+0x16BF` | IsGate / IsLaserFence |
| `+0x16C0` | IsLaserFence (alt) |
| `+0x16C1` | IsCloning |
| `+0x16C2` | IsGrinder |
| `+0x16C3` | IsNukeSilo / IsJuggernaut |
| `+0x16C4` | IsLaserFenceGate |
| `+0x16C5` | HasTurret |
| `+0x16C6` | HasBarrel |
| `+0x16C7` | GapGenerator |
| `+0x16CA` | IsFirestormWall |
| `+0x16F0` | SuperWeaponType index |
| `+0x16FC` | Upgrade limit |
| `+0x1707` | GapRadiusInCells / Height value |
| `+0x1728` | VXL scale factor (double) |
| `+0x1730–1750` | VXL turret pivot points (3 × XYZ) |
| `+0x1754–175C` | Turret draw offset (forward/lateral/vertical) |
| `+0x1767` | IsWall |
| `+0x1768` | HasDamagedArt |
| `+0x1780` | Power / HitPoints |

---

## Global Data Addresses

| Address | Purpose |
|---------|---------|
| `DAT_008871E0` | RulesClass singleton pointer |
| `DAT_00A8EB44` / `DAT_00A8EB50` | BuildingClass global array / count |
| `DAT_00A83C6C` / `DAT_00A83C78` | BuildingTypeClass global array / count |
| `DAT_00A8ED84` | Current game frame counter |
| `DAT_00A8B238` | Game mode flag (0=campaign, nonzero=MP) |
| `DAT_0089F688` | Cell direction offset table (8 entries × 2 shorts) |
| `DAT_008192B8` | Foundation width lookup table |
| `DAT_00819310` | Foundation height lookup table |
| `DAT_00818CA0` | Wall connection bitmask table (4 entries) |
| `DAT_007F4890` | Shadow direction lookup table (32 entries) |
| `DAT_00A802D4` | Network event ring buffer (128 × 0x6F bytes) |
| `DAT_0088731C` | Primary DSurface pointer |
| `DAT_00887314` | DSurface vtable pointer (rendering) |
| `DAT_0089DDBC` | BUILDNGZ.SHA Z-buffer SHP |
| `DAT_0089DDC4` | POWEROFF.SHP pointer |
| `DAT_0089DDC8` | WRENCH.SHP pointer |

---

## RulesClass Offsets (Building-Related)

| Rules Offset | Purpose |
|---|---|
| `+0x0F0` | Max production queue size |
| `+0x86C–87C` | Bridge/wall BuildingType pointers |
| `+0xB5C` | Bridge type pair |
| `+0xDF8` | Team leash distance |
| `+0xF70` | Building threat rating |
| `+0xFA8` | Wall move speed / guard range |
| `+0x1444` | Auto-repair health threshold |
| `+0x14F8–1500` | Shroud/gap range values |
| `+0x1700` | ConditionYellow threshold (double) |
| `+0x1708` | ConditionRed threshold |
| `+0x16E8` | RepairRate |
| `+0x16F0` | GameSpeed |
| `+0x1758` | Repair health cap |
| `+0x178C` | Patrol distance threshold |
| `+0x18B8` | TurretDragFactor |
