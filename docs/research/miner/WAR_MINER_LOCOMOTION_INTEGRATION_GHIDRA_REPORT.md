# War Miner — Drive Locomotion Integration — Ghidra Research Report

**Date:** 2026-04-19
**Binary:** gamemd.exe
**Primary addresses:**
- `UnitClass::Mission_Harvest` — `0x73E5E0`
- `TechnoClass::Set_Destination` — `0x741970`
- `FootClass::Set_Destination_Internal` — `0x4D94B0`
- `FootClass::Search_For_Tiberium_And_Move` — `0x4DCFE0`
- `DriveLocomotionClass::Set_Destination` (vtable+0x44) — `0x4AFD40`
- `DriveLocomotionClass::Process` — `0x4B0500`
- `DriveLocomotionClass::Process_Movement` — `0x4B2630`
- `DriveLocomotionClass::Head_To_Coord` (getter, vtable+0x4C) — `0x4AFCC0`
- `BuildingClass::EnterTransport` — `0x70FD70`
- `BuildingClass::UndockUnit` — `0x4593A0`

**Confidence:** HIGH (all key paths decompiled; vtable layout cross-verified via constructor at `0x4AF540`)
**Active in YR:** YES — core gameplay; runs every War Miner tick.

---

## 1. Overview

This report documents the **integration boundary between `UnitClass::Mission_Harvest` and
`DriveLocomotionClass`** for the War Miner (HARV) — i.e., how the harvester state machine
issues movement commands to the locomotor, how the locomotor responds tick-by-tick, and
what (if any) harvester-specific branches exist on the locomotion side.

**Headline finding:** The locomotor itself is **almost entirely generic** for harvesters —
Drive treats a War Miner identically to a Rhino tank. All harvester-specific behavior lives
in (a) `Mission_Harvest` (which decides *where* to drive) and (b) `BuildingClass` dock
handlers (which take over the unit briefly during dock). The two harvester-specific branches
that *do* exist inside `DriveLocomotionClass::Process` are:

1. **Building-cell arrival auto-clear** (line ~30–40 of Process): when the nav target is a
   building (RTTI=0xB) and the unit's current cell == the building's cell, clear destination.
   This is the dock-arrival handler; not literally harvester-only but heavily exercised by
   harvesters.
2. **Tiberium spill animation** (line ~140 of Process): if the unit has cargo and the
   current cell has tiberium type 2 (Blue), spawn `Rules.TiberiumSpill` every 10 frames.
   Generic but only triggered by units with non-zero `Storage`.

Everything else — Speed=4, the `ROT=` facing-rate field, the dock-cell approach,
path-recompute on ore depletion — is either generic Drive logic with no harvester
branch, or is driven from the mission layer above the locomotor.

> **2026-05-21 correction:** This report originally described harvesters as
> effectively using `ROT=10` because `Harvester=yes` overwrote the normal `ROT=`
> field. `CMIN_RUNTIME_ROT_PARSER_OVERRIDE_GHIDRA_REPORT.md` supersedes that:
> `ROT=` is parsed into `TechnoTypeClass+0x71C` and remains the stock value
> (`5` for HARV/CMIN). The real harvester/weeder write of `10` targets a
> separate `UnitTypeClass+0x398` field, whose wider gameplay semantic is still
> deferred.

---

## 2. Set_Destination Call Chain

### 2.1 Entry from Mission_Harvest

Inside `UnitClass::Mission_Harvest` (0x73E5E0), the War Miner calls Set_Destination at
exactly four sites, all via the unit's vtable slot `+0x480` (TechnoClass::Set_Destination):

| Site | State | What is set | Purpose |
|------|-------|-------------|---------|
| `param_1[0x86]` clear, line ~State 0 | 0 | `Set_Destination(0, 1)` | Clears stale destination at start of scan |
| `Search_For_Tiberium_And_Move` → `(this+0x480)(cell, 1)` | 0/1 | Ore cell | Drive to harvest target |
| State 2 far-from-refinery | 2 | `(this+0x480)(passable_cell_near_exit)` | Drive to refinery exit when too far |
| `(this+0x480)(0, 1)` cleanup paths | 0/1/2 | NULL | Cancel current move |

Note: when the refinery is **close enough** (≤ `HarvesterTooFarDistance`*256 leptons,
default 1280), Mission_Harvest does NOT call Set_Destination. Instead it sends radio 2
(DOCK_LINK) to the refinery via `Transmit_Radio` and transitions to State 3, which queues
`Mission_Enter` (mission ID 7). The locomotion-side dock approach is then driven by
`Mission_Enter`, not `Mission_Harvest`.

### 2.2 TechnoClass::Set_Destination (0x741970)

This is a long function (~700 lines decompiled) handling many edge cases. For the **War
Miner with `Teleporter=no`**, the active code path is:

1. Early-out checks (limbo, dock-link present, suicide commando flag, etc.)
2. Detect special target types: building entry (case 7), building exit (case 0x10), etc.
3. **Skip the teleporter piggyback block** — gated by `param_1[10].vtable_INoticeSource[0xCD4]`
   (= `TechnoTypeClass+0xCD4` = `Teleporter` flag). For HARV this is 0, so the entire
   `CoCreateInstance(CLSID_DriveLocomotion)` + `Begin_Piggyback` block is skipped.
4. **The Hover-piggyback block** at the end (CLSID_HoverLocomotion check) is also skipped
   for HARV (Drive locomotor, not Hover).
5. Falls through to `FootClass::Set_Destination_Internal(unaff_retaddr, param_2)`.

Only `Teleporter=yes` units hit the piggyback creation path. For War Miner this is purely
a pass-through function: input target → output stored in `FootClass+0x5A4` and pushed to
the active locomotor's `Set_Destination`.

### 2.3 FootClass::Set_Destination_Internal (0x4D94B0)

The actual handoff from mission-level state to locomotor state. Key actions in order:

```pseudo
this.maybe_clear_path_lock_count = 0     // FootClass+0x5A0 (param_1[0x168])
if (this.is_loaded_into_transport && new_target != null) return
if (this.is_in_repair_facility && new_target != null) return
if (this.dock_link != null && new_target != null) return
if (this.is_chronowarping && new_target != null)
    BuildingClass::DeployUnit_ChronoWarp(1)

this.nav_target = new_target              // FootClass+0x5A4 (param_1[0x169])

// Cancel any in-progress radio dialogue
if (this.radio_partner != null)
    this.radio_partner.vtable+0xF8(...)   // Clear_Radio
    this.radio_partner = null

// Edge case: clearing dest with no piggyback → fall through to dest assignment

// Get target's adjusted coord (vtable+0x4C)
coord = this.nav_target.vtable+0x4C(this)

// Push into active locomotor (FootClass+0x674 = param_1[0x19d])
this.active_locomotor.vtable+0x44(coord.X, coord.Y, coord.Z)
//                          ^-- DriveLocomotionClass::Set_Destination (0x4AFD40)

this.cell_pre_arrival_marker = 0          // FootClass+0x6B7
this.path_retry_frame = current_frame
this.path_retry_count = 0
this.something_19c = RulesClass.+0x1768   // ground travel timeout
```

**Vtable slot table for ILocomotion (DriveLocomotionClass implementation):**

| Slot | Method (this report's name) | Drive impl address |
|------|------------------------------|--------------------|
| +0x44 | `Set_Destination(X, Y, Z)` | `0x4AFD40` |
| +0x4C | `Head_To_Coord()` (getter) | `0x4AFCC0` |
| +0x58 | `Stop()` | (Drive method, used by UndockUnit) |
| +0x70 | `Set_Head_To_With_Facing(facing, X, Y, Z)` | (Drive method, used by UndockUnit) |

(Slot numbering verified by reading FootClass::Set_Destination_Internal call site and
BuildingClass::UndockUnit call sites; Drive implementation addresses from
DRIVE_LOCOMOTION_CLASS.md cross-verified.)

### 2.4 DriveLocomotionClass::Set_Destination (0x4AFD40)

Trivially simple — stores the destination, with one bridge adjustment:

```pseudo
fn Set_Destination(this, X, Y, Z):
    if owner.vtable+0x37C() return        // is in some special state
    if owner.vtable+0x380() return        // is in some other special state
    if owner.vtable+0x1D4() return        // is uncontrollable
    if owner.vtable+0x1D8() return        // is being deployed
    this.destination = (X, Y, Z)          // DriveLocomotion+0x34
    if (X, Y, Z) != NullCoord:
        cell = MapClass::Get_Cell_At(X, Y)
        if cell.flags & 0x100:            // bridge cell
            this.destination.Z += g_BridgeZOffset_Drive
```

**Important:** This stores into the `destination` field at offset `+0x34`, NOT into
`head_to` at `+0x40`. The `head_to` field is updated each tick by `Process` itself
(see §3.2 below). This means a single Set_Destination call doesn't immediately start
movement — it just records *where* the unit eventually wants to go. The next `Process`
tick reads the destination, computes the next waypoint, and writes that to `head_to`.

This is also the field read by the arrival check in Process (location == destination
clears the destination).

---

## 3. DriveLocomotionClass::Process (0x4B0500) — Per-Tick Behavior

This is the per-tick entry point called by `FootClass::Locomotion_AI`. The function does
five things in order:

### 3.1 Slope-change detection (lines 1–15)

```pseudo
cell = owner.vtable+0x1BC()               // Get_Cell
new_slope = cell.SlopeIndex               // CellClass+0x11C
if new_slope != this.cached_slope_index:
    this.prev_slope = this.cached_slope_index
    this.cached_slope_index = new_slope
    CDTimerClass::Start(timer, 3)         // 3-frame interpolation
    this.slope_timer_total = 3
```

3-frame slope-blend timer for smooth visual transitions. **No harvester-specific behavior.**

### 3.2 Nav-target arrival auto-clear (lines 30–55)

This is one of two places the locomotor reads back through to `FootClass+0x5A4` (nav target):

```pseudo
// (When NOT actively on a drive track)
nav = owner.nav_target                                // FootClass+0x5A4
if nav != null and nav.RTTI == 0xB:                   // BUILDING
    cell = owner.Get_Cell_Coord_Adjusted()            // vtable+0x1B8
    if cell.X == nav.cell_X and cell.Y == nav.cell_Y:
        if owner.path_queue[0] == 0:                  // FootClass+0x5E0 == 0
            owner.Set_Destination(NULL, 1)
            return
        else:
            FootClass::Stop_Moving()
            owner.vtable+0x484(0, 1)                  // Mission_Cancel?
            return
```

**This is the dock-cell arrival handler.** When the War Miner reaches the cell of its
nav-target refinery, the locomotor self-clears the destination. The mission state
machine then transitions to State 3 (Mission_Enter) on the next Mission_Harvest tick.

A second check immediately follows for **non-building arrival** (when destination is just
a coord, not a building):

```pseudo
if owner.<offset_AC> == 5 and !this.<offset_5F> and this.destination != NullCoord:
    if owner.Location == this.destination:
        if owner.path_queue[0] == 0:
            owner.Set_Destination(NULL, 1)
        else:
            FootClass::Stop_Moving()
            owner.vtable+0x484(0, 1)
        return
```

The `<offset_AC> == 5` check is unverified; FootClass+0xAC is likely a "queued mission"
slot (mission 5 = Guard). This is the generic "arrived at destination coord" handler used
e.g., when the harvester reaches an ore cell.

### 3.3 Slope-blend hold (lines 58–75)

```pseudo
if !CDTimerClass::Remaining(timer):
    this.deploy_flag = 1
    goto end_of_function
if this.deploy_flag:
    this.deploy_flag = 0
    owner.vtable+0x18C(0)                 // Mark_All_Occupation_Bits(0)
```

Skip movement processing during the 3-frame slope blend. **No harvester branch.**

### 3.4 Re-sync head_to from nav target (lines 90–100)

This is the **critical mid-mission re-pathing hook**:

```pseudo
if !is_on_track:                          // not actively moving along a track
    if !owner.<offset_3CD>:                // not "stopped manually"
        nav = owner.nav_target
        if nav != null:
            coord = nav.vtable+0x4C(owner) // Get_Coord_Adjusted
            this.vtable+0x44(coord.X, coord.Y, coord.Z)
            //  ^-- Set_Destination — refresh from current nav target
```

**This is how mid-tick destination changes propagate.** When `Mission_Harvest` calls
`Set_Destination(new_ore_cell)`, the call sets `FootClass+0x5A4` (nav target) and pushes
the coord to the locomotor. But on subsequent ticks, `Process` re-reads the nav target's
**current** coord and pushes it again. So if the mission updates the nav target without
calling Set_Destination again, the locomotor still picks it up next tick.

For ore cells (cell objects), the coord is static, so this is a no-op. For *moving*
nav targets it would track them — but harvesters never have moving nav targets.

### 3.5 Tiberium spill animation (lines 140–155)

```pseudo
if this.vtable+0x80(this) and current_frame % 10 == 0 and !owner.is_invisible:
    cell = owner.vtable+0x1BC()                       // Get_Cell
    if cell.OverlayValue == 2 and Rules.TiberiumSpill != 0:
        // CellClass+0xEC == 2 → Blue Tiberium overlay
        spawn AnimClass(Rules.TiberiumSpill,
                       (owner.Location.X, owner.Location.Y),
                       layer=0x600)
```

(`vtable+0x80` returns true if the unit is "showing rotation" or similar — not directly
"has cargo". Effective gating for non-harvesters: they typically don't drive over Blue
Tiberium overlay tiles. For harvesters: spawns spill anim every 10 frames over Blue Tib.)

**RulesClass+0x94 = `[General] TiberiumSpill=` animation type** — this controls whether
the visual is drawn at all. Defaults to a small spill puff anim in YR.

### 3.6 Mission_Enter exclusion (line ~175)

```pseudo
in_enter_mission = owner.vtable+0x184() == 7
if !in_enter_mission:
    coord = owner.Get_Coord(...)
    delta = coord - head_to                            // distance to next waypoint
    ...
```

The Ghidra label `FootClass::Is_Mission_Harvest` (at 0x4DA2A0) is **misleading** — it
literally checks `mission == 7`, and mission 7 is `Mission_Enter`, not Harvest. Inside
Process_Movement, when the unit IS in Mission_Enter (i.e., approaching a building to dock),
the standard delta-from-head_to recomputation is skipped. Movement during the dock
approach is choreographed by `Mission_Enter` directly rather than the generic path-follower.

---

## 4. Mission_Harvest → Locomotion: State-by-State Flow

For the War Miner (`Teleporter=no`, `Harvester=yes`):

### State 0 (SCAN)

```
Mission_Harvest()
└─ if Storage full → State 2
└─ owner.IsHarvesting = 0                              // FootClass+0x6D2
└─ Query active locomotor IPiggyback CLSID
   └─ if == TeleportLocomotion AND nav_target != null:
        Set_Destination(NULL, 1)                       // edge case for Chrono Miner only
└─ Search_For_Tiberium_And_Move(scan_radius=TiberiumLongScan)
   └─ if owner.nav_target == null:
        cell = Scan_For_Tiberium(48 cells, ...)
        Set_Destination(cell, 1)                       // → 0x741970 → ... → DriveLoco.Set_Destination
        return TRUE
└─ if returned TRUE: state = 1, init RateTimer(2), step_counter = 0
└─ if returned FALSE and nav_target == null: state = 4, return 0x69 ticks
```

### State 1 (HARVEST)

```
Mission_Harvest()
└─ init RateTimer with HarvesterLoadRate frames if not running
└─ if step_counter < 9: return 1                       // wait
└─ Harvest_Ore_Tick(0x73D450)
   └─ extracts ore from current cell, returns true if ok
└─ if false (cell empty or storage full):
   └─ owner.IsHarvesting = 0
   └─ if storage == 1.0:
        state = 2                                      // RETURN
        scan TiberiumShortScan for "ghost" cell, store at WarpTarget
   └─ else:
        Search_For_Tiberium_And_Move(TiberiumShortScan=6)
        if found: state = 1 (re-target)
        else: state = 2
```

### State 2 (RETURN)

```
Mission_Harvest()
└─ refinery = vtable+0x528(...) = Find_Docking_Bay
└─ distance = sqrt((self - refinery)²)                 // 3D Euclidean leptons
└─ if distance ≤ HarvesterTooFarDistance × 256:        // 1280 leptons (5 cells)
   └─ result = Transmit_Radio(2, refinery)             // REQ_DOCK
   └─ if result == 1: state = 3
└─ else:
   └─ refinery = Find_Docking_Bay(no_pathfinding=1)    // wider search
   └─ if refinery and distance > 0x300 (768 leptons = 3 cells):
        // Calculate cell near refinery exit using ExitX/ExitY:
        target_cell = refinery.cell + (BuildingType+0x1618, BuildingType+0x161C)
        passable = FootClass::Find_Nearby_Passable_Cell(target_cell, ...)
        if passable.valid:
            Set_Destination(MapClass::Get_CellClass(passable))
        else:
            Set_Destination(NULL, 1)                   // give up, retry next tick
```

The **HarvesterTooFarDistance=5** threshold means: if the unit is within ~5 cells of any
friendly refinery, it sends a radio request to dock immediately (no further driving — the
locomotor handles the final approach via the building-cell arrival check in §3.2). If
farther, it drives to a passable cell next to the refinery's exit and then re-runs State 2.

### State 3 (DOCK / ENTER)

```
Mission_Harvest()
└─ Set_Mission(7, 0)                                   // Mission_Enter
   // Mission_Harvest is replaced by Mission_Enter on the next tick
```

`Mission_Enter` (0x739EC0) is a separate state machine outside this report's scope.
It calls `BuildingClass::EnterTransport` once the unit is on the dock cell.

### State 4 (NO ORE)

```
Mission_Harvest()
└─ if first_time_flag:
     try Set_Mission(0x14, 0)                          // Mission_Repair
     or  Set_Mission(0xF, 0)                           // Mission_Hunt fallback
└─ check current cell for refinery in path
   if blocking exit: Set_Destination to nearby cell
└─ Set_Mission(5, 0)                                   // Mission_Guard
└─ return 0x69 ticks (~7 seconds at 15Hz)
```

---

## 5. BuildingClass::EnterTransport (0x70FD70) — Locomotion State At Dock

When the harvester reaches the refinery's dock cell and Mission_Enter calls
`EnterTransport`:

```pseudo
fn EnterTransport(unit, building):
    if building == null: return
    cell = unit.vtable+0x1BC()                         // Get_Cell
    if Look_up_building_in_cell(cell) != building: return // sanity check
    building.dock_link_unit = unit                     // building+0x1D0
    unit.dock_link_building = building                 // unit+0x1CC (param_1[0x73])
    house.docked_harvester_flag = 1                    // house+0x5778
    if building.has_capture_manager:
        CaptureManagerClass::FreeAll()
    // Spawn dock animation
    coord = building.Get_Coord_Adjusted(0, 1, 0x600, 0, 0)
    anim = new AnimClass(Rules.DockAnim,               // RulesClass+0x31C
                         coord, 0, 1, 0x600, 0, 0)
    if anim: AnimClass::SetOwnerObject(building)
    if (unit.flags & 4) and unit.something_5D4 != 0:
        FUN_006EA870(unit, -1, 1)                      // unknown convoy/queue cleanup
```

**Critical:** EnterTransport does NOT touch the locomotor. The unit's locomotor
remains in whatever state it was in when arrival fired (typically `is_on_track=0`,
`destination=NULL` after the arrival auto-clear from §3.2). The unit appears stationary
on the dock pad while the dock animation plays.

The **visual model swap** (HARV → HORV) is driven by `BuildingClass::MissionRepairAndProduce`
(the building-side dock processor), NOT by EnterTransport. See HARVESTER_DOCK_UNLOAD_SEQUENCE.md.

---

## 6. BuildingClass::UndockUnit (0x4593A0) — Locomotion Hand-back

When unloading is complete, the building calls `UndockUnit` to send the harvester away:

```pseudo
fn UndockUnit(building):
    unit = building.dock_link_unit                     // building+0x2E4 (param_1[0xB9])
    if unit == null or unit.RTTI != 1 (UNIT): return
    unit.active_locomotor.vtable+0x58(unit.active_locomotor)
    //                          ^-- Stop()
    coord = building.Get_Coord_Adjusted(...)
    unit.active_locomotor.vtable+0x70(0x47,                              // facing 0x47 = ESE
                                       coord.X - 0x80,                   // -128 leptons west
                                       coord.Y + 0x80,                   // +128 leptons south
                                       coord.Z)
    unit.vtable+0x544(0, 0x3FF00000)                   // Set_Speed(1.0)
    unit.dock_link = 0
    building.dock_link_unit = 0
    building.vtable+0x274(3)                           // notify production
```

**ILocomotion::Stop (vtable+0x58)** — clears the locomotor's path/track state.
**ILocomotion::Set_Head_To_With_Facing (vtable+0x70)** — sets the head_to coord AND
explicit initial facing (without this, the unit would compute facing from the source-to-dest
vector, which would face the unit *into* the building). Facing 0x47 = 71 decimal ≈ ESE
(in the 0..255 facing convention, 0x40 = 90° = East, 0x47 ≈ 99° = ESE).

After UndockUnit, the next `FootClass::AI` tick triggers the **piggyback swap-back**
(see MINER_DOCK_GAPS_RESEARCH.md Gap 1) for Chrono Miners. War Miners have no piggyback,
so they just drive away on the standard Drive locomotor.

---

## 7. Speed and ROT — Where They Are Applied

### Speed (HARV `Speed=4`)

- Stored on `TechnoTypeClass+0x15E` as `double`.
- Read by `DriveLocomotionClass::Process_Movement` and `Process_Drive_Track` for speed
  ramping.
- Applied generically — **no harvester-specific override**. The ramping formula
  (acceleration / deceleration) is the same for HARV as for any other Drive unit:
  ```
  if current_speed < target_speed: current_speed += accel
  if current_speed > target_speed: current_speed -= accel * 1.5    // decel = 1.5×
  ```
  (Per LOCOMOTION_MATH_AND_CONSTANTS.md.)
- The "stop on arrival" check in `Process` (§3.2) is the only handshake with the
  speed system from this layer.

### ROT (HARV `ROT=5` in YR base; normal `ROT=` field is not overwritten)

- `TechnoTypeClass::ReadINI` parses `ROT=` into `TechnoTypeClass+0x71C`.
  `CMIN_RUNTIME_ROT_PARSER_OVERRIDE_GHIDRA_REPORT.md` verifies this field
  remains `5` for stock CMIN/HARV and is consumed by `UnitClass` facing setup.
- `UnitTypeClass::ReadINI` does perform a harvester/weeder write of `10`, but
  that write targets `UnitTypeClass+0x398`, not the `ROT=`-parsed `+0x71C`
  facing-rate field. The full gameplay semantic of `+0x398` is deferred.
- Applied generically — **no further harvester-specific branch in the locomotor.**

**Conclusion on Speed/ROT:** the verified normal facing-rate data path is
INI `ROT=` → `TechnoTypeClass+0x71C` → `UnitClass`/FacingClass setup. Do not
model `Harvester=yes` as an overwrite of the parsed `ROT=` value.

---

## 8. Path Recomputation on Ore Depletion / Refinery Death

### Ore-cell depletion (mid-harvest)

When the harvest tick (`Harvest_Ore_Tick`) extracts the last ore from the current cell and
returns false, Mission_Harvest State 1 calls `Search_For_Tiberium_And_Move(TiberiumShortScan=6)`:

```c
Search_For_Tiberium_And_Move:
  if owner.nav_target != null: return                  // already moving, don't re-target
  cell = scan(6 cells)                                  // diamond spiral
  if cell.valid:
    Set_Destination(cell)                              // → updates FootClass+0x5A4 + Drive.Set_Destination
    return TRUE
```

**The locomotor is NOT explicitly told "the cell is empty"** — the mission layer
makes the decision and pushes a new destination. The locomotor's per-tick re-sync (§3.4)
ensures the new destination takes effect on the next tick.

### Mid-move ore depletion (driving to ore that gets harvested by another miner)

This case is **not specially handled**. The harvester continues driving to the original
target cell. On arrival (§3.2 arrival auto-clear), it transitions to State 1 and immediately
discovers the cell is empty (`Harvest_Ore_Tick` returns false), then re-scans and moves on.

So a War Miner that sees its target cell get poached **wastes one round-trip** worth of
movement. There is no anti-poach lookahead in either the mission or the locomotor.

### Refinery destruction (mid-return or mid-dock)

- If destroyed mid-drive (State 2): the Find_Docking_Bay call next tick returns NULL or a
  different refinery, and Mission_Harvest re-issues Set_Destination to the new target.
- If destroyed mid-dock-approach (Mission_Enter): the building's death code broadcasts
  radio OVER_AND_OUT to all radio contacts, which calls `UnitClass::Receive_Radio` →
  `Set_Destination(NULL, 1)` + `Set_Mission(MISSION_GUARD)`. The unit then re-enters
  Mission_Harvest on the next mission tick. (Per MINER_DOCK_GAPS_RESEARCH.md Gap 2.)
- Locomotor side: `Set_Destination(NULL)` propagates through Drive.Set_Destination(NULL),
  which clears `+0x34` destination. Process's arrival auto-clear (§3.2) is not triggered
  (NULL == NULL is excluded by the early-out).

---

## 9. "Stuck" Behavior

There is **no harvester-specific stuck-recovery state**. Stuck handling for War Miners is
identical to all other Drive units:

- `DriveLocomotionClass::Process_Movement` increments a `path_stuck_counter`
  (FootClass+0x64C) on each `Find_Path` failure or `Can_Enter_Cell` rejection.
- After 10 consecutive failures, the path is abandoned (`path_queue[0] = -1`), the unit
  stops, and the next Mission_Harvest tick re-runs the State 0/1/2 logic which will
  re-issue Set_Destination.

For **AI-controlled** harvesters that get queued at a busy refinery, there is a
random-wander behavior in `Mission_Enter` via `FUN_00500200` (the AI wander point
generator) — see MINER_DOCK_GAPS_RESEARCH.md Gap 3. **Player-controlled harvesters do
not wander** when the dock is busy; they just stop and wait near the dock cell.

---

## 10. INI Keys Active in the Locomotion Path

| Key | Section | Address | Default | Effect on locomotion |
|-----|---------|---------|---------|----------------------|
| `Speed` | [HARV] | TechnoTypeClass+0x15E | 4 | Target speed for ramping |
| `ROT` | [HARV] | TechnoTypeClass+0x71C | 5 in YR; not overwritten by `Harvester=yes` | Body/facing rate |
| `Locomotor` | [HARV] | TechnoTypeClass | `{4A582741-...}` | DriveLocomotionClass CLSID |
| `Teleporter` | [HARV] | TechnoTypeClass+0xCD4 | no | Skips piggyback creation in Set_Destination |
| `Harvester` | [HARV] | TechnoTypeClass+0xE0E | yes | Gates Mission_Harvest paths; also triggers separate `+0x398 = 10` write |
| `Storage` | [HARV] | TechnoTypeClass+0x800 | 40 | Used by Mission_Harvest, not locomotor |
| `MovementZone` | [HARV] | — | Crusher | Used by pathfinder (FootClass::Find_Path), not Drive |
| `Crusher` | [HARV] | — | yes | Used by Process_Movement Can_Enter_Cell branch |
| `HarvesterTooFarDistance` | [General] | RulesClass+0xD78 | 5 | Direct-radio-dock vs drive-to-exit threshold |
| `ChronoHarvTooFarDistance` | [General] | RulesClass+0xD7C | 50 | Same, for Chrono Miner (not War Miner) |
| `TiberiumLongScan` | [General] | RulesClass+0x177C | 48 | State 0 scan radius |
| `TiberiumShortScan` | [General] | RulesClass+0x1778 | 6 | State 1 re-scan radius |
| `HarvesterLoadRate` | [General] | RulesClass+0x1520 | 2 | RateTimer per harvest step |
| `TiberiumSpill` | [General] | RulesClass+0x94 | (anim type) | Animation spawned over Blue Tib (§3.5) |
| `BridgeZOffset` (implied) | — | g_BridgeZOffset_Drive | — | Z adjust for bridge cells in Drive.Set_Destination |
| `ExitX` / `ExitY` | per-Refinery [art.ini] | BuildingTypeClass+0x1618 / +0x161C | — | Drive-to-exit cell offset in State 2 |
| `DockAnim` | [General] | RulesClass+0x31C | (anim type) | Spawned by EnterTransport |

Magic facing constant `0x47` (≈ ESE) and offsets `(-0x80, +0x80)` in UndockUnit are
hardcoded in the binary, not INI-driven.

---

## 11. Key Struct Offsets Used in This Path

### FootClass (param_1 is `int *` in many functions — multiply field index by 4)

| Byte offset | Field | Used by |
|-------------|-------|---------|
| 0x5A4 | nav_target (ObjectClass*) | Set_Destination_Internal, Process |
| 0x5E0 | path_queue[24] (int) | Process_Movement |
| 0x64C | path_stuck_counter (int) | Process_Movement |
| 0x674 | active_locomotor (ILocomotion*) | Set_Destination_Internal, AI |
| 0x6AD | deploy_state (byte) | Set_Destination_Internal early-out |
| 0x6AE | dest_cleared_marker (byte) | Set_Destination_Internal |
| 0x6B7 | cell_pre_arrival_marker (byte) | Set_Destination_Internal |
| 0x6D2 | is_harvesting (byte) | Mission_Harvest |

### DriveLocomotionClass (object_base; ILocomotion `this` = base+4)

| Byte offset (object_base) | Field | Used by |
|---------------------------|-------|---------|
| +0x1C | cached_slope_index | Process |
| +0x20 | slope_timer_start_frame | Process |
| +0x2C | slope_timer_total | Process |
| +0x34 | destination[3] (Coord3D, 12 bytes) | Set_Destination, Process arrival check |
| +0x40 | head_to[3] (Coord3D) | Process_Movement |
| +0x4C | residual_ticks | Process_Drive_Track |
| +0x50 | current_speed (double) | Process_Drive_Track |
| +0x58 | track_index (int) | Process_Drive_Track |
| +0x5C | point_index (int) | Process_Drive_Track |
| +0x63 | is_on_track (byte) | Process |
| +0x68 | piggybacked_locomotor (ILocomotion*) | End_Piggyback |

### TechnoTypeClass

| Byte offset | Field | INI key |
|-------------|-------|---------|
| 0x71C | ROT/facing rate (int/byte-consumed) | `ROT=`; not overwritten by `Harvester=yes` |
| 0x398 | harvester/weeder auxiliary field | default 15, written to 10 when `Harvester=yes` or `Weeder=yes`; exact semantic deferred |
| 0x15E | Speed (double) | `Speed=` |
| 0x800 | Storage (int) | `Storage=` |
| 0xCD4 | Teleporter (byte) | `Teleporter=` |
| 0xE0E | Harvester (byte) | `Harvester=` |
| 0xE0F | Weeder (byte) | `Weeder=` |

### BuildingClass / BuildingTypeClass

| Byte offset | Field | Notes |
|-------------|-------|-------|
| BuildingClass + 0x1D0 | dock_link_unit (UnitClass*) | EnterTransport sets this |
| BuildingClass + 0x2E4 | dock_link_unit (alt name in UndockUnit, same field) | UndockUnit reads this |
| BuildingClass + 0x73*4 = 0x1CC | dock_link_building (back-pointer on unit) | EnterTransport sets via unit |
| BuildingTypeClass + 0x1618 | ExitX (int, in cells) | State 2 dock-cell calc |
| BuildingTypeClass + 0x161C | ExitY (int, in cells) | State 2 dock-cell calc |

---

## 12. Current Rust Implementation Status

Based on the parallel Rust scan:

| Aspect | gamemd.exe behavior | Rust status |
|--------|--------------------|-----|-------------|
| Mission_Harvest 5-state machine | states 0..4 with flow above | implemented as `MinerState` enum: SearchOre / MoveToOre / Harvest / ReturnToRefinery / Dock / Unload / WaitNoOre / ForcedReturn — slightly different shape but covers the cases (`src/sim/miner/`) |
| Set_Destination → locomotor handoff | `Set_Destination` + `Set_Destination_Internal` + `DriveLocomotion.Set_Destination` | collapsed into `issue_move_command()` / `issue_direct_move()` (`src/sim/movement/movement_commands.rs`) — single API, no separate locomotor object |
| Per-tick locomotor `Process` | DriveLocomotionClass::Process re-syncs head_to from nav target each tick | `tick_movement_with_grids()` (`src/sim/movement/movement_tick.rs`) — different model, executes path waypoints directly without nav-target re-sync each tick |
| Building-cell arrival auto-clear | Process sees nav==BUILDING and clears on cell match | likely handled by `final_goal` arrival logic in MovementTarget — needs verification |
| Drive-to-exit (HarvesterTooFarDistance) | direct-dock if ≤5 cells, else drive-to-exit | reportedly implemented in dock FSM `phase_approach` → `phase_rotate_to_pad` |
| Tiberium spill animation over Blue Tib | every 10 frames, spawn Rules.TiberiumSpill | NOT implemented (no reference found in scan) — minor visual gap |
| Harvester `+0x398 = 10` write | Separate UnitType field, not the parsed `ROT=` facing-rate field | Do not model as `ObjectType::turret_rot = 10`; preserve raw `ROT=` and add a distinct field only if `+0x398` is proven relevant |
| Speed ramping | ramp toward target_speed, decel = 1.5× accel | implemented via `accel_factor` / `decel_factor` in MovementTarget (verified) |
| EnterTransport does not touch locomotor | true | TODO: confirm Rust dock approach doesn't issue extra movement commands when entering pad |
| UndockUnit facing 0x47 + offset (-0x80, +0x80) | hardcoded in binary | reportedly applied via `EXIT_FACING` constant in `miner_dock_sequence::phase_exit_pad` |
| Mid-tick destination change pickup | locomotor re-reads nav_target every Process tick | NOT modeled — Rust uses explicit re-issue rather than poll. Probably equivalent in practice; verify there's no "mission updates target without calling issue_move_command" edge case |

---

## 13. Open Questions

1. **`*(int *)(iVar4 + 0xac) == 5`** in Process (§3.2) — what is FootClass+0xAC and why
   does the arrival check require it == 5? Best guess: queued mission slot (5 = Guard).
   Worth one more decompilation pass to confirm.

2. **`vtable+0x80` on ILocomotion** in the spill anim check (§3.5) — what does Drive's
   implementation of this slot return, and does it actually correlate with "has cargo"
   for harvesters or with something else? Need to identify the function at Drive
   vtable[0x80].

3. **`vtable+0x484` on TechnoClass** — called twice from Drive::Process arrival paths
   alongside `Stop_Moving`. Likely `Cancel_Mission` or `Mark_Cell_Departed`. Not critical
   but should be labeled.

4. **`FootClass::Is_Mission_Harvest` (0x4DA2A0)** is **mislabeled** in Ghidra — it checks
   `mission == 7`, which is Mission_Enter, not Harvest. Recommend renaming to
   `FootClass::Is_Mission_Enter` after confirming with one more caller trace. (Per project
   rules, the rename was NOT done in this report — only labeling at ≥90% confidence.)

5. **Tiberium spill animation** is unimplemented in Rust. Trivial to add
   (`src/sim/movement/movement_tick.rs` after position update: if cargo > 0 and tick % 10 == 0
   and cell.tib_type == BlueTib, spawn anim). Cosmetic but observable.

6. **AI harvester wander when dock is busy** (FUN_00500200) — implementation in Rust not
   verified. May not be needed for current AI but should be documented.

7. [RESOLVED/SUPERSEDED 2026-05-21] The earlier assumption that `ROT=10` is set on
   the type at parse time was wrong for the verified `ROT=` facing-rate field.
   `CMIN_RUNTIME_ROT_PARSER_OVERRIDE_GHIDRA_REPORT.md` shows `ROT=` remains in
   `+0x71C` and `Harvester=yes` writes a separate `+0x398 = 10` field. Follow-up
   should investigate `+0x398` directly, not infer it from `ROT=`.

---

## Sources

- **Ghidra decompilations performed for this report:**
  - `UnitClass::Mission_Harvest` (0x73E5E0)
  - `TechnoClass::Set_Destination` (0x741970)
  - `FootClass::Set_Destination_Internal` (0x4D94B0)
  - `FootClass::Search_For_Tiberium_And_Move` (0x4DCFE0)
  - `DriveLocomotionClass::Constructor` (0x4AF540)
  - `DriveLocomotionClass::Set_Destination` (0x4AFD40)
  - `DriveLocomotionClass::Head_To_Coord` (0x4AFCC0)
  - `DriveLocomotionClass::Process` (0x4B0500)
  - `DriveLocomotionClass::Process_Movement` (0x4B2630, partial — 51KB output, scanned for
    harvester branches via grep on Is_Mission_Harvest, the only one found)
  - `BuildingClass::EnterTransport` (0x70FD70)
  - `BuildingClass::UndockUnit` (0x4593A0)
  - `FootClass::Is_Mission_Harvest` (0x4DA2A0)

- **Cross-referenced docs:**
  - WAR_MINER_REFERENCE.md
  - MISSION_HARVEST_GHIDRA_REPORT.md
  - HARVESTER_DOCK_UNLOAD_SEQUENCE.md
  - MINER_DOCK_GAPS_RESEARCH.md
  - DRIVE_LOCOMOTION_CLASS.md
  - DRIVE_LOCOMOTION_PROCESS_ANALYSIS.md
  - DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md
  - LOCOMOTION_MATH_AND_CONSTANTS.md
  - CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md

- **INI verified:**
  - `ini/rulesmd.ini` ([HARV], [General])
  - `ini/artmd.ini` ([HARV])
