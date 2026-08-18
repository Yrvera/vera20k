# War Miner (HARV) — Comprehensive Reference

**Date:** 2026-04-03  
**Binary:** gamemd.exe  
**Confidence:** HIGH (verified from binary decompilation, cross-referenced with INI)  
**Active in YR:** YES — core Soviet harvester

---

## 1. Overview

The War Miner (HARV) is the Soviet harvester. It uses `DriveLocomotionClass` only (no teleport),
has a 20mmRapid turret weapon, and 2x the storage capacity of the Chrono Miner. It drives to ore,
harvests, drives back to a refinery, docks, unloads, and repeats.

Key INI identity:
- `[HARV]` section in rulesmd.ini
- `Harvester=yes`, `Teleporter=no`
- `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` (DriveLocomotionClass)
- `Storage=40` (40 bales, vs Chrono Miner's 20)
- `Primary=20mmRapid`, `Turret=yes`, `OpportunityFire=yes`
- `UnloadingClass=HORV` (visual model during unload)

---

## 2. War Miner vs Chrono Miner — Key Differences

| Aspect | War Miner (HARV) | Chrono Miner (CMIN) |
|--------|-------------------|---------------------|
| Locomotor | DriveLocomotionClass only | TeleportLocomotionClass (piggybacks Drive) |
| `Teleporter` (TechnoType+0xCD4) | `false` (0) | `true` (1) |
| Storage capacity (TechnoType+0x800) | 40 bales (1000 credits) | 20 bales (500 credits) |
| Weapon | 20mmRapid turret | None |
| Return-to-refinery | Always drives | Teleports if >50 cells, drives if <=50 |
| Distance threshold for direct dock | HarvesterTooFarDistance (5 cells) | ChronoHarvTooFarDistance (50 cells) |
| Scan radii | Same: Long=48, Short=6 | Same: Long=48, Short=6 |
| Scan function | `Scan_For_Tiberium_And_Move` | `Scan_For_Tiberium_And_Move` |
| UnloadingClass | HORV | CMON |
| Cost | 1400 | 1400 |
| Speed | 4 | 4 |

---

## 3. State Machine (UnitClass::Mission_Harvest, 0x73E5E0)

The War Miner uses the same 5-state machine as the Chrono Miner. The behavioral fork is
controlled by the `Teleporter` flag (TechnoTypeClass+0xCD4), loaded into BL at 0x73E6DE.

### State 0: SCAN FOR ORE

1. If storage full → State 2 (RETURN)
2. Clear harvesting flag (UnitClass+0x6D2 = 0)
3. Call `FootClass::Search_For_Tiberium_And_Move` (0x4DCFE0) with `TiberiumLongScan` (48 cells)
4. War Miner path: checks locomotion COM pointer, verifies it's NOT TeleportLocomotionClass
   (CLSID comparison at 0x7E9A90). If it somehow is, clears destination before scanning.
5. If ore found → State 1, init RateTimer with duration=2
6. If no ore → State 4 (NO ORE), return 105 ticks (0x69)

### State 1: HARVEST ORE

1. Wait for step counter (UnitClass+0xF8) to reach 9
   - Each step = HarvesterLoadRate frames (RulesClass+0x1520)
   - Total wait: 9 * HarvesterLoadRate frames for first bale
2. Call `Harvest_Ore_Tick` (0x73D450):
   - Gets current cell, checks tiberium overlay (CellClass+0xEC == 5)
   - Calculates remaining capacity: `Storage - GetTotalAmount()`
   - Calls `CellClass::Reduce_Tiberium` to extract ore
   - Adds to StorageClass (4 float slots indexed by tib type)
3. If harvest succeeds → stay in State 1
4. If cell empty or full:
   - If full (storage == 1.0) → State 2, scan nearby with TiberiumShortScan (6 cells) to remember a "ghost cell"
   - If not full → scan nearby with TiberiumShortScan
     - Found ore → stay in State 1
     - No ore → State 2 (return even if not full)

### State 2: RETURN TO REFINERY — War Miner Path

1. Call `Find_Docking_Bay` (0x4DF040) to find nearest friendly refinery
2. **Distance check (War Miner specific):**
   - Calculate 3D Euclidean distance in leptons
   - Compare against `HarvesterTooFarDistance * 256` (RulesClass+0xD78, default 5*256=1280 leptons)
   - If distance <= 1280 leptons: send radio 2 (DOCK_LINK) to refinery → State 3
   - If distance > 1280 leptons: second search without pathfinding constraints
     - If found AND distance > 768 leptons (3 cells): calculate cell near refinery exit
       using `BuildingTypeClass+0x1618` (ExitX) and `+0x161C` (ExitY), then drive there
     - If found AND distance <= 768 leptons: falls through (already close enough)

### State 3: DOCK/ENTER

- Queue `Mission_Enter` (mission 7)
- Mission_Enter (0x739EC0) handles the actual docking approach and unload

### State 4: NO ORE FOUND

1. If first-time flag set: search for Repair Bay → Mission_Repair
2. If standing on refinery: move away from exit
3. Queue Mission_Guard

---

## 4. Turret Behavior During Harvesting

The War Miner can fire while harvesting due to:
- `OpportunityFire=yes` — can fire at targets of opportunity
- `Turret=yes` — independent turret rotation
- ROT set to 10 (from default 15) when Harvester=yes
- `NoAutoFire` is NOT set

This means the War Miner actively seeks targets and fires its 20mmRapid weapon during all
states: scanning, moving to ore, harvesting, returning, and even while docked (though
firing while docked may be interrupted by the dock sequence).

---

## 5. Docking Sequence

### Approach and Entry

1. War Miner receives dock clearance from refinery via radio protocol
2. Drives to queue cell (`BuildingTypeClass+0x1618/0x161C` offsets)
3. When at dock cell: `BuildingClass::EnterTransport` (0x70FD70) links unit to building
4. Model swap: HARV → HORV (via `UnloadingClass` at TechnoTypeClass+0x6B8)

### Unload Loop

Handled by `BuildingClass::MissionRepairAndProduce` (0x44B780):
1. Dump progress tracked in `building+0x620`, incremented each frame by `building+0x638`
2. Completion check: `HarvesterDumpRate * 900.0 <= dump_progress`
   - HarvesterDumpRate = 0.016 (minutes per bale)
   - 900.0 = 60 sec/min * 15 fps → 14.4 frames per bale
3. Per bale: `StorageClass::FindFirstNonEmpty` → `StorageClass::GetAmount` → calculate credits
4. Credit formula: `ore_value * OreMultiplier * amount`
5. Purifier bonus: `storageFacilityCount * PurifierBonus * oreAmount`
6. AI difficulty bonus applied for non-human players

### Exit

1. `BuildingClass::UndockUnit` (0x4593A0):
   - Exit facing: 0x47 (71 decimal, ~ESE direction)
   - Exit offset: (-0x80, +0x80, 0) = (-128, +128) leptons from building center
   - Exit speed: 1.0 (full speed)
2. Model swap back: HORV → HARV
3. Sends radio 3 (OVER_AND_OUT) to building
4. Returns to Mission_Harvest State 0 (SCAN)

---

## 6. Free War Miner Spawn

When Soviet Ore Refinery (NAREFN) completes construction (`BuildingClass::OnConstructionComplete`, 0x445F80):
- `FreeUnit` type at `BuildingTypeClass+0xEA0` = HARV
- Spawns within 2 cells of building center
- Initial facing: 0xC0 (south), fallback 0xA0 (SSW)
- Immediately receives Mission 10 (Harvest)
- If no passable cell found, cost is refunded

---

## 7. Ore Scanning Algorithm

Both War Miner and Chrono Miner use the same `FootClass::Scan_For_Tiberium` (0x4DD0A0).

### Diamond Spiral Algorithm

1. Check current cell first — if it has tiberium, return immediately
2. Expand ring by ring from radius 1 to max:
   - Per ring: check 4 cells per iteration (one per quadrant of the diamond)
   - Within each ring, track the highest-value cell
   - **Early termination:** stop after the first ring containing ANY ore
3. Value ranking: `overlayType->Value * (tiberiumLevel + 1)`

### Scan Radii

| Context | Radius | INI Key | RulesClass Offset |
|---------|--------|---------|-------------------|
| State 0 initial scan | 48 cells | TiberiumLongScan | +0x177C |
| State 1 re-scan (cell depleted) | 6 cells | TiberiumShortScan | +0x1778 |
| State 1 ghost cell scan (full) | 6 cells | TiberiumShortScan | +0x1778 |

### Harvestability Checks (FootClass::Is_Cell_Harvestable)

1. Cell must be in playfield
2. Campaign + player-controlled: cell must not be shrouded
3. Cell must be reachable via zone pathfinding
4. Cell must have tiberium overlay (CellClass+0xEC == 5)
5. Cell must be enterable by the unit (Can_Enter_Cell returns 0)

---

## 8. Storage Model

### StorageClass (4-slot float array)

- Each slot indexed by tiberium type (0-3)
- `AddAmount(float, int type)`: `storage[type] += amount`
- `GetAmount(int type)`: returns `storage[type]`
- `GetTotalAmount()`: sum of all 4 slots
- `FindFirstNonEmpty()`: first index where `storage[i] > 0.0f`, or -1

### Harvest Extraction

Per `Harvest_Ore_Tick` (0x73D450):
1. Check storage not full: `Get_Storage_Percentage() < 1.0`
2. Get tib type index from cell overlay
3. Calculate remaining capacity: `Storage - GetTotalAmount()`
4. Call `CellClass::Reduce_Tiberium(remaining)` — extracts up to remaining amount
5. `StorageClass::AddAmount(extracted, tibType)`

### Cell Tiberium Levels

- CellClass+0x11E: tiberium level (0-11)
- `Reduce_Tiberium` decrements level by requested amount
- If amount >= level+1: removes ALL tiberium from cell (overlay = -1, level = 0)
- Returns actual amount extracted

---

## 9. INI Keys Reference

### [HARV] Section (rulesmd.ini)

| Key | Value | Purpose |
|-----|-------|---------|
| Harvester | yes | Enables harvester behavior |
| Storage | 40 | Bale capacity (40 * 25 = 1000 credits worth of ore) |
| Dock | NAREFN,GAREFN | Compatible refinery types |
| Speed | 4 | Movement speed class |
| Sight | 4 | Vision range |
| Turret | yes | Has rotating turret |
| Primary | 20mmRapid | Weapon |
| OpportunityFire | yes | Can fire while harvesting |
| UnloadingClass | HORV | Visual model during unload |
| Locomotor | {4A582741-...} | DriveLocomotionClass |
| ImmuneToVeins | yes | Not damaged by vein creeps (YR only) |

### [General] Section (relevant to War Miner)

| Key | RulesClass Offset | Default | Purpose |
|-----|-------------------|---------|---------|
| HarvesterTooFarDistance | +0xD78 | 5 | Cells before War Miner considers refinery "close enough" |
| HarvesterLoadRate | +0x1520 | 2 | Frames per harvest step (9 steps = 1 bale) |
| HarvesterDumpRate | +0x1528 | 0.016 | Minutes per bale during unload |
| TiberiumShortScan | +0x1778 | 6 | Short ore scan radius (cells) |
| TiberiumLongScan | +0x177C | 48 | Long ore scan radius (cells) |
| PurifierBonus | +0xF3C | 0.25 | 25% bonus per Ore Purifier |

---

## 10. Key Struct Offsets

### UnitClass (param_1 is int* in Mission_Harvest, multiply indices by 4)

| Byte Offset | Field | Description |
|-------------|-------|-------------|
| 0xBC | HarvestState | State machine (0-4) |
| 0xF8 | StepCounter | Counts timer expirations in State 1 |
| 0x100-0x10C | RateTimer | CDTimerClass for harvest timing |
| 0x218 | WarpTarget | Ghost/warp target cell |
| 0x3D0 | FirstTimeFlag | First-time-no-ore flag |
| 0x5A4 | Destination | Current destination |
| 0x6D2 | IsHarvesting | 1 when actively harvesting |

### TechnoTypeClass

| Byte Offset | Field | INI Key |
|-------------|-------|---------|
| 0x800 | Storage | `Storage=` |
| 0xCD4 | Teleporter | `Teleporter=` (false for HARV) |
| 0xE0E | Harvester | `Harvester=` |
| 0xE0F | Weeder | `Weeder=` |

---

## Sources

- Ghidra decompilation: UnitClass::Mission_Harvest (0x73E5E0), Harvest_Ore_Tick (0x73D450),
  FootClass::Scan_For_Tiberium (0x4DD0A0), CellClass::Reduce_Tiberium (0x480A80),
  Find_Docking_Bay (0x4DF040), BuildingClass::EnterTransport (0x70FD70),
  BuildingClass::UndockUnit (0x4593A0), BuildingClass::OnConstructionComplete (0x445F80)
- INI: ini/rulesmd.ini, ini/rules.ini, ini/artmd.ini
- Cross-referenced with: MISSION_HARVEST_GHIDRA_REPORT.md, HARVESTER_DOCK_UNLOAD_SEQUENCE.md,
  CHRONO_MINER_SYSTEM_OVERVIEW.md
