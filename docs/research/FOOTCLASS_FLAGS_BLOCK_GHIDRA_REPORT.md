# FootClass Flags Block (0x684-0x6B8) - Ghidra Analysis Report

## Overview

This report documents the single-byte flags and small fields in FootClass at offsets 0x684-0x6B8.
All fields are initialized to 0 in the FootClass constructor at `0x4D31E0` unless noted otherwise.

The Save/ComputeChecksum function at `0x4DBAD0` serializes a subset of these fields. Fields NOT
in the checksum are transient (re-derived each tick or only needed for rendering/sound).

## Constructor Layout (from 0x4D31E0)

```
param_1 type = int* (multiply word offsets by 4 for byte offsets)

0x684  param_1[0x1A1] byte - TubeIndex (0xFF = not in tube)
0x685  (int)param_1+0x685  byte
0x686  (int)param_1+0x686  byte
0x687  (int)param_1+0x687  byte
0x688  param_1[0x1A2]  byte
0x689  (int)param_1+0x689  byte
0x68A  (int)param_1+0x68A  byte
0x68B  (int)param_1+0x68B  byte
0x68C  param_1[0x1A3]  byte
0x68D  (int)param_1+0x68D  byte
0x68E  (int)param_1+0x68E  byte
0x68F  (int)param_1+0x68F  byte
0x690  param_1[0x1A4]  byte
0x691  (int)param_1+0x691  byte
0x692-0x693  -- NOT explicitly initialized (padding between 0x691 and 0x694)
0x694  param_1[0x1A5]  dword (pointer, set to 0) -- TeamClass pointer
0x698  param_1[0x1A6]  dword (set to 0)
0x69C  param_1[0x1A7]  dword (set to 0)
... (additional fields after)
0x6AB  (int)param_1+0x6AB  byte (part of param_1[0x1AA])
0x6AC  param_1[0x1AB]  byte
0x6AD  (int)param_1+0x6AD  byte
0x6AE  (int)param_1+0x6AE  byte
0x6AF  (int)param_1+0x6AF  byte
0x6B0  param_1[0x1AC]  byte
0x6B1  (int)param_1+0x6B1  byte
0x6B2  (int)param_1+0x6B2  byte
0x6B3  (int)param_1+0x6B3  byte
0x6B4  param_1[0x1AD]  byte
0x6B5  (int)param_1+0x6B5  byte
0x6B6  (int)param_1+0x6B6  byte (initialized to 1, not 0)
0x6B7  (int)param_1+0x6B7  byte
0x6B8  param_1[0x1AE]  byte
```

## Checksum Serialization Order (from 0x4DBAD0)

The checksum saves these bytes from the flags block, in order:
```
0x684  (char)  via 004a1c10
0x685  (char)  via 004a1c10
0x688  (bool)  via 004a1ca0
0x689  (bool)  via 004a1ca0
0x68A  (bool)  via 004a1ca0
0x68B  (bool)  via 004a1ca0
0x68C  (bool)  via 004a1ca0
0x68D  (bool)  via 004a1ca0
-- gap: 0x68E-0x6AE NOT checksummed (transient state) --
0x6AF  (bool)  via 004a1ca0
0x6B0  (bool)  via 004a1ca0
0x6B1  (bool)  via 004a1ca0
0x6B2  (bool)  via 004a1ca0
0x6B3  (bool)  via 004a1ca0
0x6B4  (char)  via 004a1c10
```

Fields 0x686, 0x687, 0x68E-0x6AE, 0x6B5-0x6B8 are NOT checksummed.
004a1c10 = hash_add_byte (signed char value)
004a1ca0 = hash_add_bool (0 or 1)

---

## Identified Fields

### 0x685 - TubeSegmentIndex
- **Type:** byte (signed char)
- **Purpose:** Index into the current tunnel/tube's direction array. Incremented as the unit
  moves through each segment of a tube.
- **Writes:**
  - `UnitClass::TubeMovement` (0x7359F0): incremented by 1 each segment, reset to 0 on tube entry
  - `InfantryClass::TubeMovement` (0x51B350): same pattern
  - `WalkLocomotionClass::Process_Movement` (0x5B01C0): reset to 0 on tube entry
  - `DriveLocomotionClass::Process_Drive_Track` (0x4B0F20): reset to 0 on tube entry
- **Reads:** Used as index: `*(int *)(tubeData + 0x30 + offset_0x685 * 4)` to get tube direction
- **Checksummed:** Yes
- **Confidence:** 95% - clear usage pattern across all locomotion classes

### 0x686 - PathTargetWaypointID
- **Type:** byte (signed char)
- **Purpose:** The waypoint ID for the current path destination. Passed to FUN_00763980
  (waypoint lookup) to get the target cell coordinates.
- **Writes:**
  - `FootClass::SetPathIndex` (0x4DC810): set to `param_3` (the waypoint parameter),
    cleared to 0 when path index is -1
  - `FUN_004DC8C0` (FootClass path recalculation): cleared to 0 when pathfinding fails
- **Reads:**
  - `FootClass::PerCellProcess` (0x4D85D0): reads via `FUN_00763980((int)*(char *)((int)param_1 + 0x686))`,
    clears path if waypoint no longer valid
  - `FootClass::OnArrival` (0x4D8400): same read pattern, used to recalculate path on arrival
  - `FUN_004DE580`: reads to look up waypoint and recalculate path
- **Checksummed:** No (transient path state, re-derived from pathfinding)
- **Confidence:** 90%

### 0x68A - IsMovingSound / MovementSoundPending
- **Type:** bool
- **Purpose:** Flag that triggers a movement start sound effect. When set, the next movement
  processing cycle plays `VocClass::PlayAtPos()` and then clears the flag.
- **Writes:**
  - `DriveLocomotionClass::Process_Movement` (0x4B2630): cleared to 0 after playing sound,
    cleared to 0 when movement blocked or completed
  - `WalkLocomotionClass::Process_Movement` (0x5B01C0): cleared to 0 in multiple paths -
    after sound playback, on movement completion, on path failure, on idle stop
- **Reads:**
  - Both locomotion classes check `if (*(char *)(foot + 0x68a) != '\0')` before calling VocClass::PlayAtPos
- **Checksummed:** Yes (affects game state via sound event timing)
- **Confidence:** 85% - consistently paired with VocClass::PlayAtPos across locomotors

### 0x68B - BridgeRampTransition / ElevationChanged
- **Type:** bool
- **Purpose:** Set when a unit crosses between a bridge cell and a non-bridge cell (elevation
  transition). Also set when exiting a tunnel/tube.
- **Writes:**
  - `DriveLocomotionClass::Process_Movement` (0x4B2630 at 0x4B4640): set to 1 when
    `(cell->flags >> 8 & 1) != foot->0x8c` (bridge state differs from unit's bridge flag)
  - Also set to 1 at 0x4B45F0 when drive movement encounters a fully blocked path
  - `WalkLocomotionClass::Process_Movement` (0x5B01C0): set to 1 on same bridge mismatch check
  - `UnitClass::TubeMovement` (0x7359F0): set to 1 when unit exits tube
  - `InfantryClass::TubeMovement` (0x51B350): set to 1 when unit exits tube
- **Reads:** Not directly read in checked code; likely consumed by the next movement cycle
  for height adjustment decisions
- **Checksummed:** Yes
- **Confidence:** 80% - the bridge/tube-exit write pattern is clear; the read side may be
  in code not yet decompiled

### 0x68C - Unknown Bool
- **Type:** bool
- **Purpose:** Unknown. Initialized to 0 in constructor. Saved in checksum.
  No confirmed writes or reads found in decompiled FootClass or locomotion methods.
  The byte pattern `8c 06 00 00` appears in BuildingClass constructors (different class,
  different offset meaning) and some other unrelated code.
- **Checksummed:** Yes
- **Confidence:** 10% - no functional references found; may be vestigial or only written
  by specific mission/trigger logic not yet examined

### 0x692 - NOT A FOOTCLASS FIELD
- **Status:** Eliminated. This offset is NOT initialized in the FootClass constructor between
  0x691 and 0x694. It appears in decompiled code as `*(char *)(typeClass + 0x692)` -- this is
  a TechnoTypeClass field, not a FootClass instance field. Bytes 0x692-0x693 are likely
  alignment padding in FootClass between the 0x691 byte flag and the 0x694 dword pointer.
- **Confidence:** 90% that this is NOT a FootClass field

### 0x693 - NOT A FOOTCLASS FIELD
- **Status:** Same as 0x692. Padding between 0x691 and 0x694.
  The byte pattern `93 06 00 00` appears in networking code as a string table index,
  and in MechLocomotionClass dialog functions as UI control IDs.
- **Confidence:** 90% that this is NOT a FootClass field

### 0x698 - Unknown DWord (param_1[0x1A6])
- **Type:** dword (4 bytes)
- **Purpose:** Unknown. Initialized to 0 in constructor. Not in the checksum.
  No confirmed code accesses found. May be a counter or timer that was planned but
  never used, or only used in very specific edge cases (e.g., specific map triggers).
- **Checksummed:** No
- **Confidence:** 5% - no functional references found

### 0x69C - Unknown DWord (param_1[0x1A7])
- **Type:** dword (4 bytes)
- **Purpose:** Unknown. Initialized to 0 in constructor. Not in the checksum.
  No confirmed code accesses found.
- **Checksummed:** No
- **Confidence:** 5% - no functional references found

### 0x6B0 - Unknown Bool
- **Type:** bool
- **Purpose:** Unknown FootClass instance field. In the checksum.
  NOTE: The byte pattern `b0 06 00 00` also matches TypeClass field accesses (e.g., in
  `FootClass::Mission_Guard` at 0x4D530D, `*(char *)(typeClass + 0x6b0)` checks whether
  the unit type has some property). The FootClass instance field at 0x6B0 is separate.
  No confirmed writes to the FootClass instance field were found in the code examined.
- **Checksummed:** Yes
- **Confidence:** 10%

### 0x6B2 - CellProcessCleared / PerCellResetFlag
- **Type:** bool
- **Purpose:** Cleared to 0 at the start of `FootClass::PerCellProcess` when `param_2 == 2`
  (entering a new cell). This is part of the per-cell state reset along with 0x6B0 being
  cleared at the same point.
- **Writes:**
  - `FootClass::PerCellProcess` (0x4D85D0): `*(undefined1 *)((int)param_1 + 0x6b2) = 0`
    at the very start of the enter-cell handler
- **Reads:** Not directly observed in decompiled code
- **Checksummed:** Yes
- **Confidence:** 50% - we know it's reset per cell, but don't know what sets it to 1

### 0x6B5 - IsOnWall / CrushingWallDeceleration
- **Type:** bool
- **Purpose:** Set when a vehicle enters a cell containing a wall/fence that it can crush.
  When set, forces the unit's speed to a specific low value (decelerating through the wall).
  Used by DriveLocomotionClass and WalkLocomotionClass.
- **Writes:**
  - `DriveLocomotionClass::Process_Drive_Track` (0x4B0F20): set to 1 when:
    ```
    entering_cell has building AND
    unit's SpeedType == 0xC AND
    unit type has OmniCrush/WallBuster flag
    ```
    Also writes `foot->0x334 = 0xbd4ccccd` (speed factor) if type has 0xD2B flag
  - `WalkLocomotionClass::Process_Movement` (0x5B01C0): same pattern -
    set to 1 when next cell has a wall overlay, the unit has WallBuster ability,
    and the overlay has `294 == 0` (Strength 0 = crushable wall). Also writes speed factor.
- **Reads:**
  - `DriveLocomotionClass::Process_Drive_Track` (0x4B0F20): when 0x6B5 is set,
    forces speed to `_DAT_007e3548` (a specific deceleration value) instead of
    normal acceleration/deceleration logic
- **Checksummed:** No (transient per-cell state)
- **Confidence:** 85% - consistent write pattern across Drive and Walk locomotors,
  clearly related to wall-crushing deceleration

---

## Summary Table

| Offset | Size | Checksummed | Name | Confidence |
|--------|------|-------------|------|------------|
| 0x685 | byte | Yes | TubeSegmentIndex | 95% |
| 0x686 | byte | No | PathTargetWaypointID | 90% |
| 0x68A | byte | Yes | IsMovingSound | 85% |
| 0x68B | byte | Yes | BridgeRampTransition | 80% |
| 0x68C | byte | Yes | Unknown | 10% |
| 0x692 | -- | -- | NOT A FIELD (padding) | 90% |
| 0x693 | -- | -- | NOT A FIELD (padding) | 90% |
| 0x698 | dword | No | Unknown | 5% |
| 0x69C | dword | No | Unknown | 5% |
| 0x6B0 | byte | Yes | Unknown | 10% |
| 0x6B2 | byte | Yes | PerCellResetFlag | 50% |
| 0x6B5 | byte | No | IsOnWall (crush decel) | 85% |

## Additional Context Fields (already known, for reference)

| Offset | Name | Notes |
|--------|------|-------|
| 0x684 | TubeIndex | Index into global tube array, 0xFF = not in tube |
| 0x687 | PendingScatter | Checked/cleared in OnArrival, triggers scatter |
| 0x688 | (known) | |
| 0x689 | (known) | |
| 0x68D | (known) | |
| 0x68E | EnterGarrison | Set in Mission_Guard when entering garrison |
| 0x68F | (known) | Checked in Mission_Guard |
| 0x691 | (known) | |
| 0x6AD | IsLanding / Paradrop | Checked in FootClass::AI and Set_Destination |
| 0x6AE | (known) | Set in Set_Destination when clearing paradrop |
| 0x6B3 | TickArrivalProcessed | Set/cleared in FootClass::AI, prevents duplicate arrival |
| 0x6B6 | HasArrivedAtCell | Set in Drive/Walk on track completion, cleared on cell enter |
| 0x6B7 | PathBlockedFlag | Cleared in Set_Destination and on arrival |

## Methodology

- Decompiled FootClass constructor (0x4D31E0) for initialization values
- Disassembled FootClass::ComputeChecksum/Save (0x4DBAD0) for serialization order
- Searched for 4-byte little-endian patterns of each offset across .text section
- Decompiled key FootClass methods: AI, Set_Destination, OnArrival, PerCellProcess,
  Mission_Guard, SetPathIndex, and path recalculation (0x4DC8C0)
- Decompiled locomotion classes: DriveLocomotionClass::Process_Movement (0x4B2630),
  DriveLocomotionClass::Process_Drive_Track (0x4B0F20),
  WalkLocomotionClass::Process_Movement (0x5B01C0),
  UnitClass::TubeMovement (0x7359F0), InfantryClass::TubeMovement (0x51B350)
- Cross-referenced to eliminate TypeClass offset collisions (0x692 in TypeClass vs FootClass)
