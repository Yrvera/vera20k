# Terrain Cost System — gamemd.exe Factsheet

Verified via live Ghidra MCP decompilation of `gamemd.exe` (YR 1.001).
Each fact cross-referenced against at least two independent sources
(decompiled code + INI data, or two different functions in the binary).

---

## 1. LandType Enum (12 values)

Stored at `CellClass + 0xEC` (int). Recalculated by `CellClass::RecalcAttributes` (`0x0047d2b0`).
(corrected 2026-05-29: was "Recalculated by `FUN_00483c80`"; `FUN_00483c80` is `CellClass::RecalcZoneType` and writes ZoneType to `CellClass+0x4C`, not LandType; LandType at +0xEC is written by `RecalcAttributes` — verified via `get_function_by_address 0x00483c80` and `decompile_function 0x0047d2b0` — RTTI_LABEL_DRIFT)

| Index | Name     | INI Section | Notes |
|-------|----------|-------------|-------|
|   0   | Clear    | [Clear]     | Default open ground |
|   1   | Road     | [Road]      | Paved surfaces |
|   2   | Water    | [Water]     | Open water |
|   3   | Rock     | [Rock]      | Cliffs, impassable |
|   4   | Wall     | [Wall]      | Man-made obstacles |
|   5   | Tiberium | [Tiberium]  | Ore/gem fields |
|   6   | Beach    | [Beach]     | Sandy shoreline |
|   7   | Rough    | [Rough]     | Rocky/uneven terrain |
|   8   | Ice      | [Ice]       | Frozen surfaces |
|   9   | Railroad | [Railroad]  | Train tracks |
|  10   | Tunnel   | [Tunnel]    | Underground passages |
|  11   | Weeds    | [Weeds]     | Veinhole weed patches |

**Source:** Pointer table at `0x00839d68` (12 string pointers), verified by
reading each string address. Matches INI sections in `rulesmd.ini:30191-30330`.

---

## 2. SpeedType Enum (8 values)

Stored at `TechnoTypeClass + 0x67C` (int). Parsed from `SpeedType=` in rules.ini.

| Index | Name       | Notes |
|-------|------------|-------|
|   0   | Foot       | Infantry |
|   1   | Track      | Tanks, miners |
|   2   | Wheel      | Wheeled vehicles |
|   3   | Hover      | Jumpjet-style hover |
|   4   | Winged     | Aircraft (always 1.0, hardcoded) |
|   5   | Float      | Naval vessels |
|   6   | Amphibious | Land + water |
|   7   | FloatBeach | Hover + beach access |

**Source:** String constants at `0x0081dba0–0x0081dbdc`, confirmed by
`FUN_00674000` (table populator) which reads INI keys in this exact order.
Default: Track (index 1).

---

## 3. TerrainSpeedTable (g_SpeedType_LandType_Table)

**Address:** `0x0089ea40` (BSS — populated at runtime from INI)
**Size:** 432 bytes = 12 LandTypes × 9 entries × 4 bytes/float
**Named symbol:** `g_SpeedType_LandType_Table`

### Layout

Each row = one LandType (36 bytes = 9 floats):

```
Row[LandType] = {
  [0] Foot,        [1] Track,     [2] Wheel,
  [3] Hover,       [4] Winged,    [5] Float,
  [6] Amphibious,  [7] FloatBeach,[8] Buildable (bool as float)
}
```

### Index Formula

```c
speed = g_SpeedType_LandType_Table[SpeedType + LandType * 9];
```

**Verified at:** `DriveLocomotionClass::Process_Movement` (address `0x004b3ca3`):
```c
local_48 = (double)(float)(&g_SpeedType_LandType_Table)[*(int *)(iVar8 + 0x67c) + iVar6 * 9];
```
Where `iVar8 + 0x67c` = TechnoTypeClass::SpeedType and `iVar6` = CellClass::LandType.

### Values from rulesmd.ini (% of full speed, 0 = impassable)

| LandType    | Foot | Track | Wheel | Hover | Winged | Float | Amph | FltBch | Build |
|-------------|------|-------|-------|-------|--------|-------|------|--------|-------|
| Clear       | 100  | 100   | 100   |  50   |  100   |   0   |  80  |    0   |  yes  |
| Road        | 100  | 100   | 100   |  75   |  100   |   0   | 100  |    0   |  yes  |
| Water       |   0  |   0   |   0   | 100   |  100   | 100   | 100  |  100   |  no   |
| Rock        |   0  |   0   |   0   |   0   |  100   |   0   |   0  |    0   |  no   |
| Wall        |   0  |   0   |   0   |   0   |  100   |   0   |   0  |    0   |  no   |
| Tiberium    |  90  |  70   |  50   |  50   |  100   |   0   |  50  |    0   |  no   |
| Beach       |   0  |   0   |   0   |  75   |  100   |   0   |  60  |  100   |  no   |
| Rough       | 100  | 100   | 100   |  50   |  100   |   0   |  80  |    0   |  yes  |
| Ice         |  50  |  80   |  50   | 100   |  100   |   0   |  50  |    0   |  no   |
| Railroad    |  90  | 100   |  50   | 100   |  100   |   0   |  50  |    0   |  no   |
| Tunnel      | 100  | 100   | 100   | 100   |  100   |   0   | 100  |    0   |  no   |
| Weeds       |  50  |  70   |  50   | 100   |  100   |   0   |  50  |    0   |  no   |

**Note:** Winged is hardcoded to 1.0 (100%) for all LandTypes in
`FUN_00674000` — never read from INI: `pfVar4[3] = 1.0;`

**Note:** Values are stored as floats (0.0–1.0), not integers. The INI
uses percentage format (`100%` → 1.0, `50%` → 0.5, `0%` → 0.0).

---

## 4. Speed Computation in DriveLocomotionClass

**Function:** `DriveLocomotionClass::Process_Movement` at `0x004b2630`
(corrected 2026-05-29: was `0x004b3c80`; that address is within the function body, not the entry point; entry confirmed at `0x004b2630` via `get_function_by_address 0x004b3c80` — GHIDRA_ADDRESS_SHIFT)

### Algorithm (verified from decompilation, lines ~790–830)

```
1. BASE_SPEED = TerrainSpeedTable[unit.SpeedType + cell.LandType * 9]
2. Clamp: if BASE_SPEED > 1.0 → BASE_SPEED = 1.0

3. SLOPE MODIFIER (only for ground units, vtable check == 1):
   - Going UPHILL (ground height increasing):
     if SpeedType == 1 (Track):
       speed *= RulesClass[0x768]   (SlopeClimb for tracked)
     else:
       speed *= RulesClass[0x778]   (SlopeClimb for others)
   - Going DOWNHILL:
     if SpeedType == 1 (Track):
       speed *= RulesClass[0x770]   (SlopeDescend for tracked)
     else:
       speed *= RulesClass[0x780]   (SlopeDescend for others)

4. Fallback: if speed == 0.0 → speed = 0.5
   (applied AFTER slope modifiers, not before; emergency minimum for units on impassable terrain)
   (corrected 2026-05-29: was step 3 before slope; binary shows `if (local_48 == 0.0) { local_48 = 0.5; }` after both slope blocks — verified via `decompile_function 0x004b2630` — OPERATOR_OR_ORDER_DRIFT)

5. HEALTH PENALTY:
   if healthRatio <= RulesClass[0x1700]:
     speed *= DamageSpeedMultiplier (hardcoded 0.75 at `0x007e7fc0`)

6. FORMATION:
   if unit is in formation with speed < 0x40:
     store raw speed
   else if speed differs from unit's MaxSpeed:
     adjust via vtable call
```

**Source for step 3 (0.5 fallback):** Line ~825 in decompiled output:
`if (local_48 == _FLOAT_007e2800) { local_48 = 0.5; }`

---

## 5. Pathfinding Cost Function

**Function:** `FUN_00429830` (PathfinderClass::ComputeMoveCost)

### Parameters
- `param_1`: pathfinder `this`
- `param_2`: source cell pointer
- `param_3`: destination cell pointer
- `param_4`: diagonal flag
- `param_5`: pathfinding mode (integer cast to float by decompiler)

### Base Cost Table at `0x0081870c`

| Mode | Value    | Purpose |
|------|----------|---------|
|  0   |    1.0   | Normal ground cost |
|  1   | 1000.0   | Very high cost |
|  2   |    1.0   | Bridge-aware pathing (triggers bridge logic) |
|  3   |    1.0   | Normal |
|  4   |   60.0   | Medium-high cost |
|  5   |   20.0   | Medium cost |
|  6   |    8.0   | Low-medium cost |
|  7   | 10000.0  | Near-impassable cost |

### Terrain does NOT affect pathfinding cost

The A* step cost function does **not** reference the TerrainSpeedTable, LandType,
or any cell terrain classification. All passable cells have equal pathfinding cost
(modulo bridge multipliers and diagonal penalties). The pathfinder finds the
**shortest-distance** path, not the fastest path.

Roads, rough terrain, tiberium, etc. all have identical A* cost. The
TerrainSpeedTable only affects **runtime movement speed** in
`DriveLocomotionClass::Process_Movement` — how fast a unit traverses a cell
it's already committed to, not which path the planner chooses.

### Bridge Cost Handling (the only terrain-based cost modifier)

When cell flags at `cell + 0x140` include `0x40000`:
```
cost *= 4.0   (bridge passthrough multiplier at 0x007e37bc)
```

When diagonal flag is set and bridge cell (flag `0x100`):
```
- Bridge approach (one side): cost *= 1.0    (0x007e2ac8)
- Bridge-to-bridge:           cost *= 2.0    (0x007e37b4)
- Non-bridge diagonal:        cost *= 10.0   (0x007e37b8)
```

---

## 6. MovementZone Enum (13 values)

Stored at `TechnoTypeClass + 0x5B4` (int). Parsed from `MovementZone=` in rules.ini.
**MovementZone IS the speed class** — the enum value is used directly as the row
index into the passability matrix. There is no separate mapping.

| Index | Name                | Notes |
|-------|---------------------|-------|
|   0   | Normal              | Default ground movement |
|   1   | Crusher             | Can crush infantry/walls |
|   2   | Destroyer           | Can destroy obstacles |
|   3   | AmphibiousDestroyer | Water + destroy |
|   4   | AmphibiousCrusher   | Water + crush |
|   5   | Amphibious          | Land + water |
|   6   | Subterannean        | Underground (note: misspelled in binary) |
|   7   | Infantry            | Foot soldiers |
|   8   | InfantryDestroyer   | Infantry + destroy |
|   9   | Fly                 | Aircraft |
|  10   | Water               | Ships only |
|  11   | WaterBeach          | Ships + beach |
|  12   | CrusherAll          | Crushes everything |

**Source:** String pointer table at `0x0081ba88` (13 entries), parsed by
`FUN_00474e40` (MovementZone_FromString).

---

## 7. Passability Matrix (Zone System)

**Address:** `0x0082a594` (initialized data, .rdata)
**Size:** 416 bytes = 13 rows × 8 columns × 4 bytes/int
**Purpose:** Zone flood-fill connectivity — determines which zones can
connect. Used for instant unreachability detection, NOT individual cell cost.

### Values: 1 = passable, 2 = blocked, 3 = destroyable

**Rows** = MovementZone enum (index 0–12).
**Columns** = first 8 LandTypes (Clear, Road, Water, Rock, Wall, Tiberium, Beach, Rough).

```
Idx  MovementZone        Clear Road Water Rock Wall  Tib Beach Rough
─────────────────────────────────────────────────────────────────────
 0   Normal              [ 1,   2,   2,   2,   2,   2,   2,   3 ]
 1   Crusher             [ 1,   1,   2,   2,   2,   2,   2,   3 ]
 2   Destroyer           [ 1,   1,   1,   2,   2,   2,   2,   3 ]
 3   AmphibDestroyer     [ 1,   1,   1,   1,   1,   1,   2,   3 ]
 4   AmphibCrusher       [ 1,   1,   2,   1,   1,   2,   2,   3 ]
 5   Amphibious          [ 1,   2,   2,   1,   1,   2,   2,   3 ]
 6   Subterannean        [ 1,   1,   1,   2,   2,   2,   1,   3 ]
 7   Infantry            [ 1,   2,   2,   2,   2,   1,   2,   3 ]
 8   InfantryDestroyer   [ 1,   1,   1,   2,   2,   1,   2,   3 ]
 9   Fly                 [ 1,   1,   1,   1,   1,   1,   1,   3 ]
10   Water               [ 2,   2,   2,   2,   1,   2,   2,   3 ]
11   WaterBeach          [ 2,   2,   2,   1,   1,   2,   2,   3 ]
12   CrusherAll          [ 1,   1,   1,   2,   2,   2,   2,   3 ]
```

**Indexing:** `matrix[movementZone * 8 + landType]`
Verified in `FUN_0042c290` at `0x0042c2a7` and in `FUN_005840c0` (zone flood fill):
```c
(&DAT_0082a594)[param_4 * 8 + iVar18] == 1
(&DAT_0082a594)[movementZone * 8 + *(int *)(cell + 0x4c)]
```

Only value 1 is treated as passable. Value 3 = destroyable (walls — units
that can destroy walls pathfind through them).

---

## 7. Cell Flags Relevant to Terrain Cost

**CellClass + 0x140** (bitfield):
- `0x0100` — Bridge cell (has bridge overlay)
- `0x0800` — NS bridge (vs EW)
- `0x0400` — Bridge ramp cell
- `0x40000` — Bridge passthrough flag (increases pathfinding cost ×4)

**CellClass + 0xEC** — LandType (int, 0–11)
**CellClass + 0x11B** — Cell height level (char, 0–14)

---

## 8. Bridge System and SpeedType Offset

When a unit is on a bridge, `+4` is added to the SpeedType for terrain
lookup purposes. This creates a "bridge variant" of each SpeedType.

From `DriveLocomotionClass::Process_Movement`:
```c
iVar12 = (-(uint)(*(char *)(iVar8 + 0x8c) != '\0') & 4) + (int)*(char *)(iVar12 + 0x11b);
```
The `0x8c` flag indicates "on bridge", adding 4 to the height level used
for slope calculations. This effectively skips the underlying water terrain.

---

## 9. Can_Enter_Cell — Per-Unit Passability Check

Called via **vtable offset +0x1AC** on the unit. Returns codes 0–7.

| Code | Name | Meaning | Locomotor Response |
|------|------|---------|-------------------|
| 0 | PASSABLE | Cell is free | Proceed with movement |
| 1 | BLOCKED_BY_UNIT | Friendly/neutral blocks | Scatter blocker, repath or stop |
| 2 | BLOCKED_REPATH | Temporarily blocked | Set delay timer, call pathfinder |
| 3 | REDIRECT | Bridge ramp transition | Handle bridge ramp, re-enter loop |
| 4 | OCCUPIED_TEMP | Occupied but will clear | Clear path + retry |
| 5 | OCCUPIED_CRUSHABLE | Crushable/attackable occupant | Attempt crush or attack |
| 6 | CLIFF_EDGE | Height difference too steep | Check JumpJet; try bridge transition |
| 7 | IMPASSABLE | Cannot enter at all | Full stop, clear path |

**Key function:** `UnitClass::Can_Enter_Cell` at `0x0073f0a0` (3238 bytes).

### Override Rules
- **Crusher flag** (`TypeClass+0xC94`): codes < 7 → 0 (can enter)
- **AttackMove flag** (`TypeClass+0xD28`): codes 4/5 → 0 if cell has no overlay
- **JumpJet**: codes < 7 → 0

### Terrain Check (inside FUN_004834a0)

```
speed = g_SpeedType_LandType_Table[SpeedType + LandType * 9]
if speed == 0.0 AND not on bridge → return BLOCKED (code 7)
```

### Occupancy Bitfields

**CellClass + 0x124** (ground level) / **+ 0x128** (bridge level):

| Bit | Mask | Meaning |
|-----|------|---------|
| 0-3 | 0x0F | Infantry sub-cell 0–3 occupied |
| 4   | 0x10 | Overlay/building placed |
| 5   | 0x20 | Vehicle/unit occupying cell |

**Blocked-by-terrain vs blocked-by-occupant:**
- Terrain (code 7): `SpeedTable[idx] == 0.0`, cliff height, indestructible wall
- Occupant (codes 1-5): friendly/enemy unit, crushable wall, gate building

---

## 10. CellClass::RecalcZoneType Algorithm

**Function:** `CellClass::RecalcZoneType` at `0x00483c80`
**Writes to:** `CellClass + 0x4C` (ZoneType, int 0–7 — NOT LandType)
**Reads from:** `CellClass::LandType` at `CellClass + 0xEC` (set by caller `RecalcAttributes`)

Called by `CellClass::RecalcAttributes` (`0x0047d2b0`) whenever tile/overlay changes.

(corrected 2026-05-29: was titled "RecalcLandType" and wrote "computed LandType"; binary label is `CellClass__RecalcZoneType`, it computes ZoneType (0–7) not LandType (0–11), and LandType itself is written directly by `RecalcAttributes` — verified via `get_function_by_address 0x00483c80` and `decompile_function 0x00483c80` — RTTI_LABEL_DRIFT)

### Algorithm (pseudocode)

Note: this function computes **ZoneType** (0–7), not LandType.
ZoneType values: 0=Ground, 1=Road, 2=Wall, 3=Beach, 4=Water, 5=Building, 6=Impassable, 7=OOB.

(corrected 2026-05-29: prior version described LandType values and wrong overlay flag names; corrected to ZoneType and verified flag offsets — verified via `decompile_function 0x00483c80` — RTTI_LABEL_DRIFT / INFERENCE_HARDENED)

```
1. If cell outside map bounds → ZoneType = 7 (OOB)

2. If overlay present (cell+0x44 != -1):
   a. overlay+0x22D (IsCrate) != 0       → ZoneType = 1 (Road)
   b. overlay+0x2A8 (IsWall) != 0        → ZoneType = 2 (Wall)     [!]
   c. SpeedTable[overlay+0x298 (Land)].Wheel == 0.0 → ZoneType = 6 (Impassable)
   d. overlay+0x2B5 (IsGate) != 0        → ZoneType = 6 (Impassable)
   e. overlay+0x2B4 (IsRubble) != 0      → ZoneType = 0 (Ground)

3. Check CellClass::LandType (at cell+0xEC):
   a. LandType == 2 (Water tile)  → ZoneType = 4 (Water)       [!]
   b. LandType == 6 (Beach tile)  → ZoneType = 3 (Beach)       [!]
   c. SpeedTable[LandType].Wheel <= 0.01 → ZoneType = 6 (Impassable)

4. Scan objects on cell (linked list at cell+0xE4):
   a. Building (rtti==6): LaserFence flag + living owner → ZoneType = 6
   b. Building (rtti==6): gate connection mask check   → ZoneType = 6
   c. Building (rtti==0x24): various wall checks       → ZoneType = 5 or 2

5. Default → ZoneType = 0 (Ground)
```

### Non-obvious mappings (marked [!] above)
- **IsWall overlay** → ZoneType **Wall** (2)
- **Water LandType tile** → ZoneType **Water** (4), not ZoneType 2
- **Beach LandType tile** → ZoneType **Beach** (3), not ZoneType 6

### Key OverlayTypeClass fields

| Offset | INI Key | Field | ZoneType effect |
|--------|---------|-------|-----------------|
| 0x298  | `Land=` | LandType enum | used for speed table check → Impassable |
| 0x22D  | (IsCrate flag) | IsCrate | → Road (1) |
| 0x2A8  | `Wall=` | IsWall flag | → Wall (2) |
| 0x2B4  | `IsRubble=` | IsRubble flag | → Ground (0) |
| 0x2B5  | (IsGate flag) | IsGate | → Impassable (6) |

(corrected 2026-05-29: prior table had wrong flag names — 0x22D is IsCrate not Wall, 0x2A8 is IsWall not Tiberium, 0x2B5 is IsGate not IsARock; no Tiberium flag entry was used in this function — verified via `decompile_function 0x00483c80` — INFERENCE_HARDENED)

**Tunnel (10) and Weeds (11)** LandType values are set by `RecalcAttributes` from
the overlay/tile's `Land=` value before calling `RecalcZoneType`.

---

## 11. Key Addresses Summary

| Address      | Name / Purpose |
|-------------|----------------|
| `0x0089ea40` | `g_SpeedType_LandType_Table` — 12×9 float terrain speed table (BSS) |
| `0x0082a594` | Passability matrix — 13×8 int zone connectivity (rdata) |
| `0x0081870c` | Pathfinding base cost table — 8 floats (rdata) |
| `0x00839d68` | LandType name pointer table — 12 pointers (rdata) |
| `0x0081ba88` | MovementZone string pointer table — 13 pointers (rdata) |
| `0x00474e40` | `MovementZone_FromString()` — INI parser |
| `0x00674000` | `TerrainSpeedTable_Init()` — populates speed table from INI |
| `0x00429830` | `PathfinderClass::ComputeMoveCost()` — A* step cost |
| `0x0073f0a0` | `UnitClass::Can_Enter_Cell()` — per-unit passability (3238 bytes) |
| `0x00483c80` | `CellClass::RecalcZoneType()` — computes ZoneType (0–7) into CellClass+0x4C (corrected 2026-05-29: was RecalcLandType — RTTI_LABEL_DRIFT) |
| `0x0047d2b0` | `CellClass::RecalcAttributes()` — sets LandType at +0xEC, then calls RecalcZoneType |
| `0x004834a0` | `CellClass::CheckCellPassability()` — core terrain check |
| `0x007e37bc` | Bridge pathfinding cost multiplier = 4.0 |
| `0x007e37b4` | Bridge-to-bridge diagonal cost = 2.0 |
| `0x007e37b8` | Non-bridge diagonal cost = 10.0 |
| `0x007e2ac8` | Single-bridge approach cost = 1.0 |
| `CellClass + 0x4C`  | ZoneType (0–7, written by RecalcZoneType; corrected 2026-05-29: was "Computed LandType" — RTTI_LABEL_DRIFT) |
| `CellClass + 0xEC`  | Base terrain LandType (from tile/overlay) |
| `CellClass + 0x124` | Ground-level occupancy bitfield |
| `CellClass + 0x128` | Bridge-level occupancy bitfield |
| `CellClass + 0x140` | Cell flags (bridge bits) |
| `CellClass + 0x11B` | Cell height level |
| `TechnoTypeClass + 0x5B4` | MovementZone field |
| `TechnoTypeClass + 0x67C` | SpeedType field |
| `RulesClass + 0x768` | SlopeClimb (Track) |
| `RulesClass + 0x770` | SlopeDescend (Track) |
| `RulesClass + 0x778` | SlopeClimb (Other) |
| `RulesClass + 0x780` | SlopeDescend (Other) |

---

## 12. Comparison with Our Implementation

### What we get right
- SpeedType enum variants (all 8)
- Basic LandType classification (Clear, Road, Water, Rock, Rough)
- SpeedType → terrain passability (Track can't cross water, etc.)
- Cost values as percentages (0=blocked, 100=normal, 120=road)

### What differs from gamemd.exe
- **Missing LandTypes:** We have 8 (Clear, Road, Rough, Beach, Water,
  Tiberium, Railroad, Rock). Original has 12 (adds Wall, Ice, Tunnel, Weeds).
- **Speed values are percentages, not integers:** The original stores
  0.0–1.0 floats. Our `TerrainCostGrid` uses u8 (0–255). This loses
  precision on values like 90% (Foot on Tiberium) → we'd round to 90.
- **SpeedType order:** Original is Foot(0), Track(1), Wheel(2), Hover(3),
  Winged(4), Float(5), Amphibious(6), FloatBeach(7). Ours is different
  enum order (no index significance in Rust, but matters for tables).
- **Slope modifiers:** We don't implement slope-based speed penalties.
  Original has separate climb/descend multipliers for Track vs others.
- **Bridge +4 offset:** Original adds 4 to SpeedType index for bridge cells.
  We handle bridges separately in `TerrainCostGrid::from_resolved_terrain`.
- **0.5 fallback:** Original gives stuck units 50% speed on impassable
  terrain. We block movement entirely (cost=0).
- **Pathfinding cost ≠ movement speed:** Original has separate tables for
  A* pathfinding costs vs runtime movement speed. We use a single
  `TerrainCostGrid` for both.
- **Missing zone passability matrix:** Our zone system uses a simpler
  passability check. Original has a full 13×8 matrix for zone connectivity.
- **Can_Enter_Cell return codes:** Original has 8 return codes (0–7) with
  distinct meanings (wait, crush, repath, cliff, etc.). We have simpler
  blocked/unblocked logic.
- **RecalcZoneType non-obvious mappings:** Original maps Water LandType tiles → ZoneType 4 (Water),
  Beach LandType tiles → ZoneType 3 (Beach), IsWall overlay → ZoneType 2 (Wall).
  (corrected 2026-05-29: was "RecalcLandType" with wrong LandType target values — RTTI_LABEL_DRIFT)
- **MovementZone:** We don't implement the full 13-value MovementZone enum
  or use it as the speed class for zone passability checks. Original uses
  MovementZone directly as the passability matrix row index.
- **Two LandType fields:** Original has both base terrain LandType (cell+0xEC,
  from tile) and computed LandType (cell+0x4C, after overlay/object checks).
  We have a single LandType per cell.
