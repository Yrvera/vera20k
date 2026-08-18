# Timer Classes, FacingClass, and Zone Map System -- Ghidra Report

Verified against `gamemd.exe` via Ghidra MCP live decompilation.
All addresses, offsets, and data layouts confirmed from binary. Confidence: ~90%.

---

## Part 1: CDTimerClass

CDTimerClass is a lightweight countdown timer used throughout the engine. It stores a
start frame and duration, then computes remaining time on demand by subtracting elapsed
frames from duration.

### Data Layout (12 bytes)

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| +0x0 | 4 | int | start_frame | Frame when timer was started. -1 = timer not started/paused |
| +0x4 | 4 | int | (field_4) | Often written alongside start_frame, purpose varies by context |
| +0x8 | 4 | int | duration | Duration in frames. 0 = expired immediately |

### Key Functions

**CDTimerClass::Start** (was: CDTimerClass::Init; Ghidra label is CDTimerClass__Start; semantics unchanged — verified via get_function_by_address 0x0046b640, 2026-05-20) -- `0x0046b640`
```c
void __thiscall CDTimerClass__Start(int *this, int duration) {
    this[0] = g_CurrentFrameCounter;  // start_frame = now
    this[2] = duration;               // set duration
}
```

**CDTimerClass::GetTimeRemaining** (simple form) -- `0x00426630`
```c
int __fastcall CDTimerClass__GetTimeRemaining(int *this) {
    int duration = this[2];
    if (this[0] != -1) {           // if timer is active
        int elapsed = g_CurrentFrameCounter - this[0];
        if (elapsed < duration)
            return duration - elapsed;
        return 0;                  // expired
    }
    return duration;               // timer paused: return raw duration
}
```

**CDTimerClass::Remaining** (bool form, used by RateTimer) -- `0x004c9480`

Embedded timer check that returns 1 if time remaining > 0, 0 otherwise. Accesses a
larger struct (offset +0x8 for start_frame, +0x10 for duration, +0x14 for rate check).
This version is part of the RateTimer/FacingClass layout.

**IMPORTANT — outer rate gate (corrected 2026-05-28: prior text said "same algorithm as
GetTimeRemaining"; binary shows an extra outer gate not present in GetTimeRemaining):**
Before any timer math, the function checks `if (0 < *(short *)(param_1 + 0x14))` (the
`rate` field). If rate ≤ 0, it returns 0 immediately — the timer is never consulted.
Only when rate > 0 does it proceed to the start_frame/duration check. This gate is
absent from `CDTimerClass__GetTimeRemaining`. (corrected 2026-05-28: was "same
algorithm as GetTimeRemaining"; binary via decompile_function 0x004c9480 shows outer
`if (0 < *(short*)(param_1+0x14))` guard — ROOT_CAUSE: INFERENCE_HARDENED)

### How It Works

- The timer never self-updates. It is purely *computed* from `g_CurrentFrameCounter`.
- `g_CurrentFrameCounter` is the global game frame count, incremented once per game tick.
- Setting start_frame to -1 effectively pauses the timer (remaining returns raw duration).
- Zero duration means immediately expired.
- The timer is NOT an object with a vtable -- it is a plain 12-byte struct.

### Usage in DriveLocomotionClass

CDTimerClass::Start is called from:
- `DriveLocomotionClass::Process` at `0x004b0536` -- slope transition timer (3 frames)
- `ShipLocomotionClass::Process` at `0x0069fc46` -- same for ships. (corrected 2026-07-12: a
  2026-05-20 pass changed this call-site address to the function's entry address 0x0069fc10 and
  mislabeled 0x0069fc46 as wrong "mid-body". get_xrefs_to 0x0046b640 (CDTimerClass::Start)
  confirms "From 0069fc46 in ShipLocomotionClass__Process" is the actual call site — the same
  kind of citation the sibling `DriveLocomotionClass::Process` bullet above uses (0x004b0536 is
  likewise a call site, confirmed via the same xref list, not that function's entry at 0x004b0500).
  ShipLocomotionClass::Process's own entry is separately confirmed at 0x0069fc10 via
  get_function_by_address, but that is not the address CDTimerClass::Start is called from. Root
  cause of the 05-20 error: address-type conflation, call-site vs. function-entry.)
- Various building/tiberium constructors

---

## Part 2: RateTimer / FacingClass

FacingClass manages smooth turn interpolation. It stores a desired facing, a saved
(start) facing, an embedded CDTimerClass for timing, and a rate value. The current
interpolated facing is computed on-demand.

### Data Layout (22 bytes)

Ghidra sees `param_1` as `short*`. All offsets in bytes:

| Offset | Size | Type | Field | Notes |
|--------|------|------|-------|-------|
| +0x0 | 2 | short | desired_facing | Target facing (set by RateTimer::Set) |
| +0x2 | 2 | short | (high_word) | Upper 16 bits when read as dword at +0x0 |
| +0x4 | 2 | short | saved_facing | Facing when turn began (start of interpolation) |
| +0x6 | 2 | short | (high_word) | Upper 16 bits when read as dword at +0x4 |
| +0x8 | 4 | int | timer.start_frame | CDTimerClass start frame (-1 = not started) |
| +0xC | 4 | int | timer.field_4 | CDTimerClass padding/unknown |
| +0x10 | 4 | int | timer.duration | CDTimerClass duration in frames |
| +0x14 | 2 | short | rate | Turn rate (steps for full turn). 0 = instant snap |

### Facing Interpolation Algorithm

**RateTimer::Current** -- `0x004c93d0`

Returns the current interpolated facing as a 4-byte value (low word = facing):

```
if rate <= 0:
    return desired_facing  (instant)

remaining = CDTimer_GetTimeRemaining()
if remaining == 0:
    return desired_facing  (turn complete)

delta = desired_facing - saved_facing
abs_delta = abs(delta)
step_size = abs_delta / rate
if step_size < 1:
    return result  (negligible turn)

current = desired_facing - (delta / step_size) * remaining
return current
```

The key insight: the facing interpolates *backward* from the desired facing. With
`remaining` frames left, the current facing is `desired - step_per_frame * remaining`.
When remaining reaches 0, current equals desired.

**RateTimer::Set** -- `0x004c9220`

Called when a new turn is requested (e.g., from `DriveLocomotionClass::Do_Turn` at `0x004b0ef0`):

1. If desired_facing already equals the new target, return 0 (no turn needed)
2. Snapshot the current interpolated facing into saved_facing
3. Set desired_facing to the new target
4. If rate > 0: compute new timer duration = abs(new_delta) / rate
5. Set timer.start_frame = g_CurrentFrameCounter

**FacingClass::UpdateFacing** -- `0x004c9300`

Called to check if the turn is complete. If the interpolated facing matches a reference
facing, it resets the timer (start_frame = now, duration = 0) and returns 0 (done).
**Rate field (`param_1[10]`, byte offset +0x14) is NOT touched — rate is preserved across
a turn-complete reset.** The done branch writes: `*(int*)(param_1+4) = g_CurrentFrameCounter`,
`*(undefined4*)(param_1+6) = local_8` (uninitialized), `param_1[8] = 0`, `param_1[9] = 0`;
`param_1[10]` is untouched. (verified via decompile_function 0x004c9300, 2026-05-20)
If not matching, it overwrites both desired and saved facings with the reference and
returns 1 (changed). **The not-matching branch performs the identical timer reset too**
(start_frame = now, duration = 0, rate untouched) in addition to overwriting the facings —
it is not a "facings only" write. (corrected 2026-07-12: doc previously described the timer
reset only under the matching/done branch, which reads as if the not-matching branch leaves
timer state alone; decompile_function 0x004c9300 shows both branches write identical
`*(int*)(param_1+4)=g_CurrentFrameCounter`, `param_1[8]=0`, `param_1[9]=0` — MISLEADING by
omission, corrected.)

### DriveLocomotionClass::Do_Turn -- `0x004b0ef0`

This is a trivial wrapper:
```c
void DriveLocomotionClass__Do_Turn(FacingClass* facing, short* new_facing) {
    RateTimer__Set(facing, new_facing);
}
```

The facing is part of the linked FootClass/TechnoClass, not the locomotor itself.

---

## Part 3: Zone Map System

The zone map is a graph-based reachability system that allows O(1) "can unit A reach
cell B?" checks. Zones are recomputed when the map loads and when bridges are
built/destroyed.

### Architecture Overview

**IMPORTANT (corrected 2026-07-12) — the "speed_type"/"SpeedType" parameter named
throughout this Part 3 section (in the ASCII diagram below, `GetZoneID`, and
`Can_Reach_Zone`) is a misnomer inherited from an earlier pass. The value callers
actually pass is the unit's **MovementZone** (`TechnoTypeClass+0x5b4`, 0-12), NOT
`SpeedType` (`TechnoTypeClass+0x67c`, 0-7). Verified via `decompile_function 0x004d3810`
(`FootClass::CanReachDestination`): it reads `*(int*)(typeclass+0x5b4)` into `iVar1`,
then calls `MapClass__Can_Reach_Zone(from, to, iVar1, ...)` — the third argument
(`speed_type` in this doc's pseudocode) is that MovementZone value, not SpeedType. This
is also why there are exactly 13 zone-ID arrays/passability rows below, not 8 — the axis
is MovementZone (13 values), matching the resolution of Open Question 1 near the end of
this doc. The variable name `speed_type` is left as-is in the code blocks below to match
the still-current Ghidra decompiler output (`param_3` is unnamed in the binary); read it
as "zone-category index (MovementZone)" wherever it appears. ROOT_CAUSE:
INFERENCE_HARDENED, compounded by STRUCT_FAMILY_CASCADE (SpeedType and MovementZone are
different TechnoTypeClass fields that this doc's earlier passes conflated).**

```
Per-cell data:
  MapClass+0x68 -> cell_data[cell_count]  (4 bytes each)
                   bytes 0-1: unknown/land info
                   bytes 2-3: zone_cluster_id (ushort)

  MapClass+0x70 -> zone_index[cell_count * 5]  (10 bytes per cell, 5 shorts)
                   Each short is a zone_cluster_id for one of 3 speed categories
                   (indices 0, 1, 2 used; 3-4 may be padding/bridge variants)

Per-SpeedType zone arrays:
  MapClass+0x18 -> zone_ids[0][cluster_count]  (ushort array, SpeedType 0)
  MapClass+0x1C -> zone_ids[1][cluster_count]  (ushort array, SpeedType 1)
  ...
  MapClass+0x48 -> zone_ids[12][cluster_count] (ushort array, SpeedType 12)

  Total: 13 zone ID arrays, one per passability row
```

### Zone Lookup: MapClass::GetZoneID -- `0x0056d230`

```
uint GetZoneID(MapClass* this, CellStruct* cell, int speed_type, char check_bridge) {
    // Bridge handling: if cell is a bridge cell, look up the bridge record
    // and potentially redirect to the bridge endpoint cell

    // Convert cell coord to linear index
    linear = (MapWidth + 1 + MapOriginX) * cell->Y + cell->X;
    linear = clamp(linear, 0, cell_count - 1);

    // Two-level lookup:
    // 1) cell_data[linear].zone_cluster_id  (ushort at offset 2 of 4-byte entry)
    cluster_id = cell_data[linear].zone_cluster_id;

    // 2) zone_ids[speed_type][cluster_id]
    return zone_ids[speed_type][cluster_id];
}
```

The two-level indirection is key:
1. Each cell maps to a **zone cluster** (shared across SpeedTypes)
2. Each zone cluster maps to a **zone ID** per SpeedType

Two cells are in the same zone (for a given SpeedType) iff their zone IDs match.

### Zone Reachability Check: MapClass::Can_Reach_Zone -- `0x0056d100`

```c
bool Can_Reach_Zone(CellStruct* from, CellStruct* to, int speed_type,
                    int from_flags, int to_flags, bool from_in_playfield) {
    if (speed_type == -1) return true;  // no speed type = always reachable

    // Edge cases: if source is outside playfield but inside map bounds, reachable
    // If destination is outside playfield (and source is inside), reachable

    // Core check:
    return GetZoneID(from, speed_type, from_flags) == GetZoneID(to, speed_type, to_flags);
}
```

### Passability Matrix -- `0x0082a594`

A 13x8 matrix of dword values. Indexed as `[row * 8 + land_type]`.

Values: 1 = passable, 2 = impassable, 3 = special (destroyable/weeds?)

The 13 rows correspond to 13 **MovementZone** values (Normal through CrusherAll).
The 8 columns correspond to the 8 **ZoneType** values (Ground, Road, Wall, Beach, Water,
Building, Impassable, OOB) — NOT LandType. ZoneType is a reduced classification computed
from LandType + overlays + objects by `CellClass::RecalcZoneType` at `0x483c80`.

**Two different TypeClass offsets are used:**

- `TechnoTypeClass+0x67c` -- **SpeedType** (0-7). Used for the speed/land-type table
  lookups (movement speed calculation). Set from `SpeedType=` INI key. Read in
  TechnoTypeClass::ReadINI at `0x007121e5`. 8 named values. Re-confirmed this session:
  `get_assembly_context 0x007121e5` shows `CALL 0x00476fc0` (`CCINIClass::ReadSpeedType`,
  itself confirmed via `decompile_function 0x00476fc0` to call `SpeedType__ToName` /
  `SpeedType__FromName`) immediately followed by `MOV dword ptr [EBP+0x67c],EAX`.

**CLARIFICATION (2026-07-18) -- resolves an apparent conflict with
`SIDEBAR_STRIPS_TABS_CAMEOS_GHIDRA.md`'s claim that `RTTI_Naval_Check` (`0x005004E0`) reads
"TechnoTypeClass+0xE08" for a naval SpeedType check:** that claim is imprecise on both the
struct and the field. `decompile_function 0x005004E0` shows `RTTI_Naval_Check` gates on RTTI
type 6/7 only, calls `RTTI_To_TypeArray(rtti_type)` (`decompile_function 0x0048dcd0`, cases
6/7 index `g_BuildingTypeClass_Array`), and returns `*(undefined4*)(iVar1+0xe08)` where
`iVar1` is a **BuildingTypeClass\*** (never a plain TechnoTypeClass\*, since RTTI 6/7 only
resolve building types) -- it performs no SpeedType/Float/Amphibious comparison itself, it is
a plain accessor. `decompile_function 0x0045fe50` (`BuildingTypeClass::ReadINI`) shows
`+0xe08` is populated by `FUN_00475060(this+0x24, s_BuildCat_0081aee4, *(this+0xe08))` --
`read_memory 0x0081aee4` confirms the pushed string is `"BuildCat"`. So `+0xE08` is
**BuildCat** (BuildCategory), a BuildingTypeClass-only field far outside the generic
TechnoTypeClass layout this doc documents (TechnoTypeClass's own fields top out around
`+0x67c`/`+0x5b4`) -- it is not SpeedType and not a shared TechnoTypeClass member. This
doc's own `+0x67c=SpeedType` claim is unaffected; the two docs describe two different
fields on two different classes, not a disparity in this doc. (Not a correction to this
doc's own claims -- added as a clarifying note per the wave-2 contradiction-resolution
task; the sibling doc's claim itself is out of scope to edit here.)

- `TechnoTypeClass+0x5b4` -- **MovementZone** (0-12), NOT a computed "ZoneSpeedCategory".
  Used for zone reachability and passability matrix row lookups. It is written directly
  by `CALL CCINIClass::ReadMovementZone (0x00474e40)` reading the `MovementZone=` INI key
  (string "MovementZone" at `0x008431c8`), with the return value stored straight to
  `[EBP+0x5b4]` at `0x00716081` -- no combining step with SpeedType exists. This
  resolves the 13-vs-8 row question below: the passability matrix's 13 rows are simply
  the 13 MovementZone values, not a derived category. (corrected 2026-07-12: doc
  previously called this a "computed combined index derived from SpeedType + MovementZone"
  with the computation function "not located" and ~75% confidence; this directly
  contradicted the already-confirmed `MOVEMENT_CLASSIFIERS_REFERENCE.md` claim
  "TechnoTypeClass+0x5B4=MovementZone" and the doc's own correct statement two paragraphs
  above ("The 13 rows correspond to 13 MovementZone values"). Verified this session via
  `get_xrefs_to 0x00474e40` -> call site `0x00716079` in `TechnoTypeClass__ReadINI`, then
  `get_assembly_context` on that call site showing `CALL 0x00474e40` immediately followed
  by `MOV dword ptr [EBP + 0x5b4],EAX`, and `read_memory 0x008431c8` confirming the pushed
  INI-key string is "MovementZone" — ROOT_CAUSE: INFERENCE_HARDENED, compounded by
  STRUCT_FAMILY_CASCADE from not cross-checking the sibling doc's confirmed offset.)

**SpeedType enum** (8 values, table at `0x0081da58`):

| Index | Name | Typical Units |
|-------|------|---------------|
| 0 | Foot | Infantry |
| 1 | Track | Tanks, heavy vehicles |
| 2 | Wheel | Light vehicles, APCs |
| 3 | Hover | Hovercraft |
| 4 | Winged | Aircraft |
| 5 | Float | Ships |
| 6 | Amphibious | Amphibious vehicles |
| 7 | FloatBeach | Amphibious near beach |

(corrected 2026-07-12: indices 6 and 7 were swapped — table previously listed 6=FloatBeach,
7=Amphibious. Binary shows the name-pointer table at `0x0081da58` entry 6 points to
`0x0081bb18` = "Amphibious" and entry 7 points to `0x0081dba0` = "FloatBeach" — verified via
read_memory 0x0081da58 (pointer table) + read_memory 0x0081bb18 / 0x0081dba0 (string bytes),
2026-07-12 — OPERATOR_OR_ORDER_DRIFT / table transcription error. All 12 LandType entries at
`0x0081da28` were also spot-checked this session and confirmed correct in the existing order.)

(Additional rows 8-12 in the passability matrix handle special cases like
AmphibiousCrusher, AmphibiousDestroyer, Destroyer, Crusher, Normal -- these
correspond to the extended SpeedType/MovementZone combinations.)

**CORRECTION (2026-04-17):** The enum below is **LandType** (12 values, table at `0x0081da28`),
NOT MovementZone. MovementZone is a separate 13-value enum (Normal, Crusher, ..., CrusherAll)
at `0x81ba88`. See ZONE_PASSABILITY_VERIFIED.md for the correct MovementZone enum.

**LandType enum** (12 values, table at `0x0081da28`):

| Index | Name |
|-------|------|
| 0 | Clear |
| 1 | Road |
| 2 | Water |
| 3 | Rock |
| 4 | Wall |
| 5 | Tiberium |
| 6 | Beach |
| 7 | Rough |
| 8 | Ice |
| 9 | Railroad |
| 10 | Tunnel |
| 11 | Weeds |

The `MovementZone_From_Name` function at `0x0048df80` parses the actual MovementZone
(Normal, Crusher, etc.) from `rules.ini` under the `MovementZone=` key on TechnoTypes.
LandType is the per-cell terrain classification (12 values); MovementZone is the per-unit
passability category (13 values). These are different enums that this document conflated.

### Zone Computation: MapClass::UpdateBridgeZonesHelper -- `0x0056c510`

This is the main zone computation function (~550 instructions). Called during map load
and bridge state changes.

Algorithm:
1. Free all 13 existing zone arrays at MapClass+0x18..0x48
2. Clear the per-cell zone cluster data at MapClass+0x68
3. Iterate all cells, assigning each to a zone cluster based on its passability type
4. Build a zone connection graph (256 hash buckets, 0x18 bytes each at MapClass+0x14)
5. For each of the 13 passability rows:
   - Allocate a new zone ID array (one ushort per cluster)
   - Flood-fill connected clusters that share the same passability value
   - Assign each connected component a unique zone ID
   - Store at MapClass+0x18 + row*4

### Zone Invalidation

**MapClass::InvalidateBridgeZones** -- `0x0056dae0`
- Called when a bridge is destroyed
- Finds the bridge record, marks it inactive (byte at +8 = 0)
- Calls RemoveBridgeZoneEdges to remove graph edges

**MapClass::ValidateBridgeZones** -- `0x0056db70`
- Called when a bridge is built/repaired
- Finds the bridge record, marks it active (byte at +8 = 1)
- Calls AddBridgeZoneEdges to add graph edges
- Checks if the bridge actually connects two different zones

**MapClass::ComputeBridgeZones** -- `0x0056d6e0`
- Scans all cells for bridges, creates bridge records (16 bytes each)
- Bridge records stored at MapClass+0x54, count at MapClass+0x60

### Hierarchical Pathfinding Integration

**PathfinderClass::UpdateHierarchicalEdges** -- `0x0042ccd0` (was: `0x0042cd80`, which is offset +0xB0 from entry; actual entry verified via get_function_by_address 0x0042ccd0, 2026-05-20; behavior confirmed)
- Called after zone recomputation
- Iterates 3 speed categories (0, 1, 2)
- For each category, calls ZoneMap::FloodFillReachableZones at `0x005840c0`
- Builds a zone adjacency graph for the A* pathfinder's hierarchical pre-check

**Zone_precheck** -- `0x0042c290`
- Called before A* search to check if destination is reachable
- Uses the hierarchical zone graph for fast rejection
- Iterates 3 speed categories (index 2, 1, 0 -- descending)
- Uses a priority queue (binary heap) for Dijkstra-like search on zone graph

### Key Data Structures in MapClass

| Offset | Type | Description |
|--------|------|-------------|
| +0x14 | ptr | Zone connection graph (hash table, 256 buckets x 0x18 bytes) |
| +0x18 | ptr[13] | Zone ID arrays, one per passability row (ushort per cluster) |
| +0x4C | int | Total zone cluster count |
| +0x54 | ptr | Bridge records array (16 bytes per bridge) |
| +0x58 | int | Bridge record capacity |
| +0x60 | int | Bridge record count |
| +0x68 | ptr | Cell data array (4 bytes per cell: 2 bytes info + 2 bytes cluster_id) |
| +0x6C | int | Total cell count |
| +0x70 | ptr | Per-cell zone index array (10 bytes per cell = 5 shorts) |
| +0x90 | ptr[3] | Zone connection graphs for 3 speed categories (24 bytes each) |
| +0xF4 | int | Map origin X |
| +0xF8 | int | Map width |

---

## Part 4: Movement Delay System

The drive locomotor uses CDTimerClass-style timers embedded in the FootClass/TechnoClass
at specific offsets to manage movement pacing and blocked-path retries.

### Movement Delay Timer (FootClass+0x640)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0x640 | 4 | int | movement_delay.start_frame |
| +0x644 | 4 | int | movement_delay.field_4 (context-dependent) |
| +0x648 | 4 | int | movement_delay.duration |

**When started:**
- Set when the unit needs to wait before its next pathfinding attempt
- `start_frame = g_CurrentFrameCounter`, `duration` = computed from speed table
- Also set when movement is blocked (Can_Enter_Cell returns 4, 5, 6, or 7) and
  `param_2` (first-attempt flag) is true

**When checked:**
- At the top of the pathfinding branch in Process_Movement (around `0x004b281c`)
- If remaining > 0, the unit skips pathfinding and returns 0 (waiting)
- When expired, a new path is computed via `FootClass::Find_Path`

**Duration computation:**
- Derived from `Math::ftol()` of the speed/slope calculation
- Represents the number of frames to wait before the next movement step

### Blocked Delay Timer (FootClass+0x668)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0x668 | 4 | int | blocked_delay.start_frame |
| +0x66C | 4 | int | blocked_delay.field_4 |
| +0x670 | 4 | int | blocked_delay.duration |

**When started:**
- Set when `Can_Enter_Cell` returns 2 (cell occupied by friendly) for the first time
- The blocked flag at `FootClass+0x6B7` is set to 1
- Duration is loaded from `g_RulesClass_Instance + 0x1768` (the global `BlockagePathDelay`
  value from `[General]` section of `rules.ini` -- see corrected key name below)

(CONFIRMED 2026-07-18, previously flagged "not re-decompiled" in the 2026-07-12 pass:
`decompile_function 0x004b2630` (`DriveLocomotionClass::Process_Movement`) shows the exact
sequence `*(int*)(iVar5+0x668) = g_CurrentFrameCounter; *(uint*)(iVar5+0x66c) = uStack_c;
*(undefined4*)(iVar5+0x670) = <Rules+0x1768 value>` gated on the first-time
`*(char*)(iVar5+0x6b7) == '\0'` check, then `*(iVar5+0x6b7) = 1` -- matches this section's
description of all four fields exactly.)

**When checked:**
- After the movement delay check, if cell is still blocked
- If blocked_delay has expired AND blocked flag is set:
  - Re-pathfind with scatter flag = 2 (more aggressive path search)
- If blocked_delay has NOT expired:
  - Re-pathfind with scatter flag = 1 (normal retry)

**Relationship to re-pathfinding:**
```
if Can_Enter_Cell == BLOCKED_BY_FRIENDLY:
    if not already_blocked:
        set blocked flag (+0x6B7) = 1
        start blocked_delay timer with Rules.BlockedDelay duration

    if movement_delay expired:
        if blocked flag set AND blocked_delay expired:
            Find_Path(dest, 0, 2)   // aggressive scatter
        else:
            Find_Path(dest, 0, 1)   // normal retry

        if path found:
            reset movement_delay timer
            return 1  // continue
```

### Other Related Offsets

| Offset | Size | Description |
|--------|------|-------------|
| +0x5E0 | 96 | Path queue (24 x 4-byte direction entries). -1 = end marker |
| +0x63C | 4 | Unknown timer/flag (set to -1 on successful track setup) |
| +0x64C | 4 | Retry counter (set to 10 when cell is out of playfield) |
| +0x688 | 1 | Flag: convoy chain needs clearing |
| +0x68A | 1 | Flag: horn/blocked sound played |
| +0x68B | 1 | Flag: bridge crossing state changed |
| +0x6B7 | 1 | Blocked flag (1 = unit is blocked by friendly) |

### Rules.ini Constants

**CORRECTED 2026-07-18** (was: "INI Key (likely)" column held descriptive glosses, not the
literal INI keys; ROOT_CAUSE: INFERENCE_HARDENED -- a plausible-sounding name was written
without checking the binary string table). Verified this session via
`search_strings`/`get_xrefs_to`/`get_assembly_context` on `RulesClass::ReadGeneral`
(`0x0066f200`-`0x0066f2c2` for the four speed fields) and `RulesClass::ReadAudioVisual`
(`0x0066b372` for ConditionYellow):

| Offset in RulesClass | INI Key (verified) | Description |
|---------------------|-------------------|-------------|
| +0x768 | `TrackedUphill` | Uphill speed multiplier for tracked (string `0x0083c8ac`, read call at `0x0066f22f`, stored via `FSTP [ESI+0x768]` at `0x0066f234`) |
| +0x770 | `TrackedDownhill` | Downhill speed multiplier for tracked (string `0x0083c89c`, read call at `0x0066f256`, stored at `0x0066f25b`) |
| +0x778 | `WheeledUphill` | Uphill speed multiplier for wheeled (string `0x0083c88c`, read call at `0x0066f27d`, stored at `0x0066f282`) |
| +0x780 | `WheeledDownhill` | Downhill speed multiplier for wheeled (string `0x0083c87c`, read call at `0x0066f2a4`, stored at `0x0066f2a9`) |
| +0x1700 | `ConditionYellow` | Health ratio threshold for slow-down (string `0x0083a370`, `FSTP double [ESI+0x1700]` at `0x0066b37f` in `RulesClass::ReadAudioVisual`) |
| +0x1718 | `CloseEnough` | Distance threshold for "arrived at destination" (string `0x0083bd84`, `MOV [ESI+0x1718],EAX` at `0x00670ef7` in `RulesClass::ReadGeneral`) |
| +0x1768 | `BlockagePathDelay` | Frames to wait before aggressive re-pathfind (string `0x0083d314`, read via `CALL 0x005276d0` int-read helper, stored via `MOV [ESI+0x1768],EAX` at `0x00673a31`) |

---

## Summary of Ghidra Labels Applied

| Address | Name |
|---------|------|
| 0x0046b640 | CDTimerClass__Start (was: CDTimerClass__Init; Ghidra label is CDTimerClass__Start; semantics unchanged — verified via get_function_by_address 0x0046b640, 2026-05-20) |
| 0x00426630 | CDTimerClass__GetTimeRemaining |
| 0x004c9480 | CDTimerClass__Remaining |
| 0x004c9220 | RateTimer__Set |
| 0x004c93d0 | RateTimer__Current |
| 0x004c9300 | FacingClass__UpdateFacing |
| 0x004b0ef0 | DriveLocomotionClass__Do_Turn |
| 0x0056d100 | MapClass__Can_Reach_Zone |
| 0x0056d230 | MapClass__GetZoneID |
| 0x0056d430 | MapClass__CoordToZoneLinearIndex (corrected 2026-07-12: doc listed "MapClass__CellCoordToLinearIndex"; live Ghidra label at this address is "MapClass__CoordToZoneLinearIndex" — verified via get_function_by_address 0x0056d430 — RTTI_LABEL_DRIFT, label was renamed since this doc's last pass) |
| 0x0056d6e0 | MapClass__ComputeBridgeZones |
| 0x0056dae0 | MapClass__InvalidateBridgeZones |
| 0x0056db70 | MapClass__ValidateBridgeZones |
| 0x0056c510 | MapClass__UpdateBridgeZonesHelper |
| 0x005840c0 | ZoneMap__FloodFillReachableZones |
| 0x0056d3f0 | ZoneMap__CellToZoneIndex |
| 0x005851b0 | MapClass__AddBridgeZoneEdges |
| 0x00584e50 | MapClass__RemoveBridgeZoneEdges |
| 0x0048df80 | MovementZone_From_Name |
| 0x0048dfd0 | MovementZone_To_Name |
| 0x0048dff0 | SpeedType_From_Name |
| 0x0048e030 | SpeedType_To_Name |
| 0x0042c290 | Zone_precheck |
| 0x0042ccd0 | PathfinderClass__UpdateHierarchicalEdges (was: 0x0042cd80 — mid-body at +0xB0; actual entry verified via get_function_by_address 0x0042ccd0, 2026-05-20) |

---

## Open Questions / Lower Confidence Areas

1. ~~**13 vs 8 passability rows**~~ -- RESOLVED (2026-07-12), see the "Two different
   TypeClass offsets" section above: TechnoTypeClass+0x5b4 is the raw **MovementZone**
   value (0-12), written directly by `CCINIClass::ReadMovementZone` (`0x00474e40`) from
   the `MovementZone=` INI key -- there is no computed "ZoneSpeedCategory" and no missing
   combination function. The 13 rows are simply the 13 MovementZone values (Normal
   through CrusherAll); SpeedType (+0x67c, 8 values) is a separate axis used only for
   `MapClass::GetZoneID`'s own speed-type parameter, not for selecting the passability
   matrix row. Verified via `get_xrefs_to 0x00474e40` + `get_assembly_context 0x00716079`
   + `read_memory 0x008431c8` ("MovementZone" string). (corrected 2026-07-12: previous
   text claimed a computed combining function existed and was unlocated at ~80%
   confidence; ROOT_CAUSE: INFERENCE_HARDENED)

2. **Cell data +0x68 field layout**: The 4 bytes per cell at MapClass+0x68 have the
   zone cluster ID at bytes 2-3. Bytes 0-1 contain land/passability information but
   the exact bit layout needs more investigation. (~80% confidence)

3. **Zone index array (+0x70) entries 3-4**: Only indices 0, 1, 2 of the 5 shorts per
   cell are used by the hierarchical pathfinder. The other 2 may be bridge-related
   variants or unused. (~60% confidence)

4. ~~**Rules+0x1768 INI key name**~~ -- RESOLVED (2026-07-18). Confirmed key is
   `BlockagePathDelay` (not the guessed "BlockagePathfindingDelay"/"BlockedDelay"). The
   2026-07-12 pass's `search_strings "BlockedDelay"` found zero matches because that is
   not the real key; this session's `search_strings "BlockagePathDelay"` found the string
   at `0x0083d314`, and `get_xrefs_to 0x0083d314` + `get_assembly_context 0x00673a18` show
   it is pushed immediately before the `CALL 0x005276d0` (int-read helper) whose result is
   stored via `MOV dword ptr [ESI+0x1768],EAX` at `0x00673a31` in `RulesClass::ReadGeneral`.
