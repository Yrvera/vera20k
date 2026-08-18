# FootClass__Find_Nearby_Passable_Cell — Decode Doc

**Proposed Ghidra label:** `FootClass__Find_Nearby_Passable_Cell`

## Summary

`FootClass__Find_Nearby_Passable_Cell` (`0x0056DC20`) is a general-purpose spiral-search
utility that finds the nearest passable cell(s) around a seed cell coordinate. It is the
fallback cell selector for the chrono-miner far-return path in `UnitClass__Mission_Harvest`
state 2 — when the miner is too far from the refinery to use the radio-accept (close) path.
The seed cell is derived from the refinery's `QueueingCell` art entry (`BuildingType+0x1618`
/ `+0x161C`). The function populates an output cell packed as a `uint32` `(y << 16 | x)`.

The search expands outward from the seed, collecting up to 24 candidate cells that pass
passability, occupancy, obstacle, and zone checks. From those candidates it selects the one
nearest the provided hint coordinate `param_14`.

## Active in YR

**Yes — active in standard YR skirmish, called frequently.** Verified by:
- `get_function_callers 0x0056DC20` returned 48 callers including `UnitClass__Mission_Harvest`
  (`0x0073E5E0`), `TechnoClass__Set_Destination` (`0x00741970`),
  `TeleportLocomotionClass__Process` (`0x00718B70`), `BuildingClass__ReleaseDockedHarvester`
  (`0x004595C0`), `ChronoSphere__WarpUnitsAtCell` (`0x0065EC30`), and many others — all
  mainstream YR gameplay systems.
- No TS-only gate detected. The function contains no `SpecialFlags` or feature-flag checks.

## Decompilation Excerpt

```c
// from decompile_function 0x0056DC20

void __thiscall FootClass__Find_Nearby_Passable_Cell(
    int        param_1,      // 'this' ptr (not used directly in body; only field accesses at param_1+0xF4/+0xF8)
    undefined4 *param_2,     // OUT: output cell packed (y << 16 | x)
    short      *param_3,     // IN:  seed cell [x, y] pair (short[2])
    undefined4  param_4,     // IN:  SpeedType (passability filter)
    int         param_5,     // IN:  zone_id (-1 or 0xFFFF = any zone; disables zone check)
    undefined4  param_6,     // IN:  zone_type / zone_mode (passed to CellRect__CheckPassability)
    undefined4  param_7,     // IN:  passOnTeleport flag (1=teleport mode — skips approach-angle subtest)
    undefined4  param_8,     // IN:  checkOccupancy (1=use CellRect__CheckOccupancy)
    undefined4  param_9,     // IN:  reserved (unused in search loop body)
    undefined4  param_10,    // IN:  extra passability flag
    char        param_11,    // IN:  allow elevation delta ≤ 1 (terrain-level tolerance check)
    char        param_12,    // IN:  check IsCurrentCellObstacleFree
    char        param_13,    // IN:  allow on-bridge cells (0 = reject bridge overlay)
    short      *param_14,    // IN:  hint coordinate [x, y] — nearest-to-hint selection
    char        param_15,    // IN:  mirrored search (adds ±mirror candidates each ring)
    char        param_16     // IN:  check CellRect__CheckOccupancy (separate from param_8)
)
{
    // Compute max search radius from 'this':
    local_1c0 = param_1[+0xF4] + param_1[+0xF8];  // speed-type movement capacity
    if (local_1c0 > 0x20) local_1c0 = 0x20;       // cap at 32 rings

    // Start: seed cell X in local_1b4, Y in local_1c4
    // Expand outward ring by ring (iVar14 = delta from -ring to +ring)
    // For each candidate cell at (seed_x + delta_x, seed_y + delta_y):
    //   1. Look up CellClass ptr in g_CellArray_Base
    //   2. TechnoClass__IsOnScreen check (must be visible)
    //   3. CellRect__CheckPassability(cell, SpeedType, ..., zone_id, zone_type)
    //   4. If param_11: elevation delta vs seed cell must be < 2
    //   5. If param_12: TechnoClass__Is_Current_Cell_Obstacle_Free
    //   6. If !param_13: reject cells with bridge overlay (CellClass+0x140 bit 8)
    //   7. If param_16: CellRect__CheckOccupancy
    //   If all pass → store in local candidate array local_120[up to 24 entries]
    //   If param_7 == '\0': also run FUN_006d6410 approach-angle subtest;
    //     set local_1d5 flag if cell passes approach check
    //   Stop collecting at 24 candidates

    // Selection phase:
    //   Partition candidates into "approach-clear" (local_c0) and "blocked" (local_60)
    //   If no hint (param_14 == invalid sentinel):
    //     pick random candidate by g_CurrentFrameCounter % count
    //   Else:
    //     pick candidate with minimum Euclidean distance to hint coord
    *param_2 = best_cell;
    // On empty result: *param_2 = DAT_00abd480 (invalid sentinel, FFFF/FFFF)
}
```

## Behavioral Analysis

### Search algorithm

The function performs a diamond/ring expansion from the seed cell, testing each cell in
order of increasing distance. For each ring (radius 0 … `local_1c0` ≤ 32), all cells at
Manhattan distance equal to the ring radius are tested in two passes:

- **Pass 1** (diagonal arm, param_15 == '\0'): cells at `(seed_x + delta, seed_y - delta)`
- **Pass 2** (if param_15 || delta > -ring): cells at `(seed_x + delta, seed_y + delta)`,
  plus side arms for non-mirrored mode

This is a standard spiral/diamond search used throughout the codebase. The cap of 24
candidates (`local_1d4 == 0x18`) terminates the search early if enough cells are found.

### Passability filters (per-candidate)

Applied in order, short-circuiting on first failure:

1. **Bounds check**: cell index `= y * 0x200 + x` must be in `[0, 0x3FFFF]`.
   Out-of-bounds cells use the sentinel cell `DAT_00abdc50` (safe fallback).
2. **TechnoClass__IsOnScreen**: cell must be visible (i.e., on the map, not off-screen).
3. **CellRect__CheckPassability**: passes `SpeedType` (param_4), zone_id (param_5),
   zone_type (param_6). If `param_5 == 0xFFFF` (-1 as int16), the zone check is
   effectively disabled (any zone passes). This is how `UnitClass__Mission_Harvest`
   state 2's far path bypasses zone isolation.
4. **Elevation delta** (param_11 only): `|cell.elevation - seed.elevation| < 2`.
   Field `CellClass+0x11B` = elevation byte. Bridge overlay `CellClass+0x140 bit 8`
   adds 4 to the delta if the seed cell is a bridge (`uVar1 >> 0xC & 1`).
5. **Obstacle free** (param_12 only): `TechnoClass__Is_Current_Cell_Obstacle_Free`.
6. **Bridge reject** (param_13 == '\0' only): reject if `CellClass+0x140 & 0x100` != 0
   (bridge overlay).
7. **Occupancy** (param_16 only): `CellRect__CheckOccupancy(cell, 0xFFFFFFFF)`.

### Approach-angle subtest (param_7 == '\0')

When NOT in teleport mode (`param_7 == '\0'`), each passing candidate is also run through
`FUN_006d6410` (`0x006D6410`). That function converts the candidate's lepton-center
(`cell * 0x100 + 0x80`) to a drive-approach cell by walking uphill from an offset position
and returning the landing cell. If the resulting cell matches the candidate itself, the
candidate is flagged as "approach-clear" (`local_1d5 = 1`). In the selection phase,
approach-clear candidates are preferred over approach-blocked ones.

When in teleport mode (`param_7 != '\0'`, e.g., chrono miner), the approach-angle subtest
is skipped entirely — teleport locomotors don't follow drive tracks.

### Candidate selection (nearest-to-hint)

After collecting up to 24 candidates:
- Partition into `local_c0[]` (approach-clear) and `local_60[]` (approach-blocked, last 24
  slots).
- If `param_14` is the invalid sentinel (`DAT_00abd480`, FFFF/FFFF):
  pick `g_CurrentFrameCounter % count` (random frame-based selection).
- Otherwise: iterate all candidates (prefer approach-clear if any), compute Euclidean 2D
  distance to `param_14` (hint cell), pick minimum. Uses `Sqrt_Approx` for distance.

### How UnitClass__Mission_Harvest calls this (verified in decode-1)

In state 2 far-return path, `UnitClass__Mission_Harvest` calls:
```c
FootClass__Find_Nearby_Passable_Cell(
    auStack_3c,         // output cell
    &uStack_54,         // seed = QueueingCell pos (refinery_NW + art_QueueingCell_offset)
    2,                  // SpeedType = Track (verified from call site)
    0xffffffff,         // zone_id = -1 → any zone (disables zone check)
    0,                  // zone_type = 0
    0,                  // pass teleport? → no (use approach subtest)
    1,                  // check occupancy
    1,                  // reserved
    0, 0, 0, 1,         // various flags
    &uStack_4c,         // hint coord (initially 0 → invalid, random pick)
    0, 0
);
```
The zone_id `-1` (= `0xFFFFFFFF` cast to `int`) suppresses zone isolation, which is the
mechanism that lets the miner find a passable cell near the refinery even when separated
by impassable terrain or a different zone.

### Return value / output

Output cell is packed as `uint32` in `*param_2`:
- bits 0..15 = X cell coordinate (short)
- bits 16..31 = Y cell coordinate (short)

On failure (no candidates found): `*param_2 = DAT_00abd480` which is the sentinel value
`(DAT_00b1cfb8 | DAT_00b1cfba << 16)` = `(0xFFFF, 0xFFFF)` packed. Callers check
`(short)*result == 0xFFFF && (short)(*result >> 16) == 0xFFFF` to detect failure.

## Struct Field Accesses

`param_1` is `int` (not `int *` — note: this is a `__thiscall` but Ghidra reconstructed it
as plain `int`). Fields accessed via `param_1 + byte_offset`:

| Field | Byte offset from param_1 | Meaning |
|-------|--------------------------|---------|
| `param_1+0xF4` | `FootClass+0xF4` | speed-type contribution 1 (summed with +0xF8 for radius cap) |
| `param_1+0xF8` | `FootClass+0xF8` | speed-type contribution 2 |

CellClass fields (via `puVar5 = g_CellArray_Base[cell_index]`):

| Field | Byte offset | Meaning | Frame |
|-------|-------------|---------|-------|
| `CellClass+0x11B` | +0x11B | Elevation byte | Terrain-level (0=sea, higher=hill) |
| `CellClass+0x140` | +0x140 | Overlay flags (uint32) | Bit 8 = bridge overlay; bit 12 = isViaduct |

Seed coordinate system:
- `param_3` = `short[2]` = `{cell_x, cell_y}` in cell-grid frame (+X = east, +Y = south)
- Internally converted to lepton centers: `lepton = cell * 0x100 + 0x80`

## Globals Referenced

| Global | Meaning | Verified |
|--------|---------|---------|
| `g_CellArray_Base` | Base pointer of cell-object pointer array; cell index = `y * 0x200 + x` | via decompile_function 0x0056DC20 |
| `g_CurrentFrameCounter` | Current game frame — used for random candidate pick | same |
| `DAT_00abd480` | Invalid/sentinel cell return value (FFFF/FFFF packed) | same |
| `DAT_00abdc50` | Out-of-bounds cell sentinel object (safe fallback for off-map coords) | same |
| `DAT_00abdc74` | Scratch storage for out-of-bounds cell coord (written before using sentinel) | same |

## Out-of-Scope Refs

- `CellRect__CheckPassability` (`0x0056E7C0`) — passability predicate; own decode scope
- `CellRect__CheckOccupancy` (`0x00586780`) — occupancy predicate; own decode scope
- `TechnoClass__IsOnScreen` (`0x00578540`) — render scope
- `TechnoClass__Is_Current_Cell_Obstacle_Free` (`0x00486FF0`) — combat/obstacle scope
- `FUN_006d6410` (`0x006D6410`) — approach-angle sub-cell finder; referenced by this doc,
  not separately tasked; documented inline above
- `MapClass__GetZoneID` — zone system scope (called from `FUN_00703590`)
- `BuildingClass__ReleaseDockedHarvester` (`0x004595C0`) — caller; decode scope separate
- `TechnoClass__Set_Destination` (`0x00741970`) — caller; task #2
- `TeleportLocomotionClass__Process` (`0x00718B70`) — caller; locomotion scope

## Unverified Claims (YELLOW)

- The exact meaning of `FootClass+0xF4` and `+0xF8` as "speed-type contributions" for
  max-radius is inferred from the sum being capped at 32; the actual field names are unknown.
- `param_1` signature: Ghidra shows `int param_1` rather than `int *` — the `__thiscall`
  calling convention is confirmed, but `param_1` type shown is non-pointer. Fields are
  accessed via `*(int *)(param_1 + offset)` not `param_1[N]`, so offsets are direct bytes.
- The `TechnoClass__IsOnScreen` inclusion check: semantics may be "is cell within map
  bounds" rather than "is visible on screen." The check fires per-candidate before the
  passability test. Exact semantics unverified beyond that it accepts valid in-map cells.
- `DAT_00abd480` = FFFF/FFFF packed sentinel: confirmed structurally (checked against it
  as failure sentinel), but the raw memory value was not read directly in this session.
