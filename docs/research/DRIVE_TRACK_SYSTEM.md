# Drive Track System — Ghidra Research Report

## Overview

The drive track system provides **pre-computed curved movement paths** for all ground
vehicles in RA2/YR. Instead of stop-rotate-go, vehicles follow smooth arcs through
cells, producing natural-looking tank movement with gradual facing changes.

**Verdict:** The system is **fully active** in the original engine. There is no code
path where a Drive-locomotor vehicle moves without going through the turn track table.
Every cell transition for every ground vehicle reads from these tables every tick.

## Binary Evidence — Call Chain

```
FootClass::AI (0x4da530)
  └─ CALL [ECX+0x40]  (0x4da877)          ← ILocomotion::Process, every tick
       │
       ├─ DriveLocomotionClass vtable at 0x7e7eb0
       │   slot 16 (+0x40) = FUN_004b0500  ← main Drive tick
       │
       FUN_004b0500 (0x4b0500–0x4b0acd)
         ├─ if track_index != -1:
         │    └─ Process_Drive_Track (0x4b0f20, ~5860 bytes)
         │         reads g_DriveTrackIndex_Table  (5 xrefs to 0x7e7b28)
         │         reads g_DriveTrackData_Array   (4 xrefs to 0x7e7a28)
         │         calls Transform_Track_Coords   (0x4b4780)
         │         calls Apply_Track_Delta         (0x4b0ad0)
         │
         └─ if track_index == -1:
              └─ Process_Movement (0x4b2630–0x4b4766, ~8500 bytes)
                   pathfinding → pick next cell → compute speed →
                   assign track_index → read g_DriveTrackIndex_Table (0x4b4023)
```

### Key Assembly — Track Table Access

At `0x4b4023` in `Process_Movement`:
```asm
LEA  EAX, [EAX + EAX*2]                 ; EAX = track_index * 3
MOV  CL, byte ptr [EAX*4 + 0x007e7b28]  ; TurnTrack[index*12].normal_track
TEST CL, CL                              ; zero = no curve available
JNZ  +6                                  ; non-zero → use this curve
LEA  ECX, [EBX + EBX*8]                  ; fallback: direction * 9 (straight)
```

## Units That Use Drive Locomotor

**CLSID:** `{4A582741-9839-11d1-B709-00A024DDAFD1}`

| Category | Units |
|----------|-------|
| Allied tanks | Grizzly, IFV, Mirage, Prism Tank, Battle Fortress, MCV |
| Soviet tanks | Apocalypse, Rhino, Demo Truck, Flak Track, War Miner |
| Yuri tanks | Lasher, Gattling Tank, Magnetron, Mastermind |
| Civilian | School Bus, civilian vehicles |
| Naval (secondary) | Sub, Dolphin, Aegis, Carrier, Dreadnought — via `{Ship};{Drive}` |

~58 units total across both factions and civilians.

## Data Tables in Binary

### 1. TurnTrack Table — `g_DriveTrackIndex_Table` (0x7e7b28)

**72 entries, 12 bytes each.** Maps turn configurations to raw curve indices.

```
struct TurnTrackEntry {      // 12 bytes
    u8  normal_track;        // +0x00  raw track for normal speed (0 = no curve)
    u8  short_track;         // +0x01  raw track for high speed
    u8  _pad[2];             // +0x02
    i32 direction;           // +0x04  target facing (0x00–0xE0 in 0x20 steps)
    i32 flags;               // +0x08  transform + cell-crossing flags
};
```

**Indexing:** `track_index = next_direction + current_direction * 8`

- Directions 0–7 map to N, NE, E, SE, S, SW, W, NW
- Entries 0–63: standard 8×8 direction matrix
- Entries 64–71: special tracks (references raw tracks 11–15)
- When `normal_track == 0`, falls back to `current_dir * 9` (straight line)

### 2. RawTrack Table — `g_DriveTrackData_Array` (0x7e7a28)

**16 entries, 16 bytes each.** Metadata for base curve definitions.
(corrected 2026-05-29: table size was listed as 192 bytes; binary shows 16 entries × 16 bytes = 256 bytes, spanning 0x7e7a28–0x7e7b28 verified via read_memory — OFFSET_RETYPED_WRONG)

```
struct RawTrackEntry {       // 16 bytes
    int* points;             // +0x00  pointer to TrackPoint array
    i32  total_count;        // +0x04  number of points (-1 for simple tracks)
    i32  entry_index;        // +0x08  where to start following
    i32  jump_index;         // +0x0C  cell crossing point (-1 = none)
};
```

| Track | Pointer    | Count | Entry | Jump | Description |
|-------|-----------|-------|-------|------|-------------|
| 0     | NULL      | 0     | 0     | 0    | Null/empty |
| 1     | 0x7e6258  | -1    | 0     | -1   | Straight north (24 pts) |
| 2     | 0x7e6378  | -1    | 0     | -1   | Straight NE diagonal (32 pts) |
| 3     | 0x7e64f8  | 37    | 12    | 22   | 45° turn curve (55 pts) |
| 4     | 0x7e6790  | 26    | 11    | 19   | 90° turn curve (38 pts) |
| 5     | 0x7e6968  | 45    | 15    | 31   | Wide turn curve (60 pts) |
| 6     | 0x7e6c50  | 44    | 16    | 27   | Wide turn curve (56 pts) |
| 7     | 0x7e6f00  | -1    | 0     | -1   | Short curve A (22 pts) |
| 8     | 0x7e7050  | -1    | 0     | -1   | Short curve B (22 pts) |
| 9     | 0x7e7158  | -1    | 0     | -1   | Short curve C (24 pts) |
| 10    | 0x7e72d0  | -1    | 0     | -1   | Short curve D (22 pts) |
| 11    | 0x7e7420  | -1    | 0     | -1   | Special A (14 pts) |
| 12    | 0x7e74c8  | -1    | 0     | -1   | Special B (corrected 2026-05-29: entry missing; verified via read_memory 0x7e7ac8) |
| 13    | 0x7e7568  | -1    | 0     | -1   | Special C (corrected 2026-05-29: entry missing; verified via read_memory 0x7e7ad8) |
| 14    | 0x7e78a8  | -1    | 0     | -1   | Special D (corrected 2026-05-29: entry missing; verified via read_memory 0x7e7ae8) |
| 15    | 0x7e7968  | -1    | 0     | -1   | Special E (corrected 2026-05-29: entry missing; verified via read_memory 0x7e7af8) |

Point counts for tracks 1–2 derived from pointer gaps (288/12=24, 384/12=32).

### 3. TrackPoint Format

```
struct TrackPoint {          // 12 bytes in binary
    i32 x;                   // lepton offset within cell
    i32 y;                   // lepton offset within cell
    i32 facing;              // 0–255 heading (stored as i32)
};
```

**Track 1 sample** (straight north, at 0x7e6258):
```
Point  0: x=0, y=245, face=0
Point  1: x=0, y=234, face=0     (y decreases by 11 per step)
Point  2: x=0, y=223, face=0
  ...
Point 22: x=0, y=3,   face=0
Point 23: x=0, y=-8,  face=0     (sentinel-like: crosses into next cell)
```

**Track 3 sample** (45° turn, at 0x7e64f8):
```
Point  0: x=-256, y=501, face=0   ← lead-in (previous cell, straight)
  ...
Point 12: x=-256, y=363, face=0   ← entry_index: following starts here
Point 13: x=-254, y=352, face=1   ← curve begins (x/face start changing)
Point 14: x=-252, y=341, face=3
  ...
Point 22: x=???,  y=???, face=??  ← jump_index: cell crossing occurs
  ...
Point 36: x=???,  y=???, face=32  ← exit (now facing NE)
```

### 4. Transform Flags — `g_DriveTrackFlags_Table` (0x7e7b30)

Stored at byte 8 of each 12-byte TurnTrack entry. Allows ~6 base curves to
produce all 72 directional variants through mirroring/flipping.

| Bit | Effect |
|-----|--------|
| 0 (1) | Swap X↔Y, adjust facing by −0x40 |
| 1 (2) | Negate X, negate facing |
| 2 (4) | Negate Y, subtract 0x80 from facing |
| 3 (8) | Cell-crossing track — requires Can_Enter_Cell validation |

`Transform_Track_Coords` at `0x4b4780` applies these per-step.

## Stepping Algorithm (Process_Drive_Track)

```
each tick:
    budget += speed_from_terrain_and_slope
    while budget > 7:
        point = track_points[point_index]
        if point.x == 0 && point.y == 0 && point_index != 0:
            track complete → attempt chain to next track
            break
        transform point via mirror/flip flags
        apply position delta to unit
        if point_index == jump_index:
            cell crossing → validate via Can_Enter_Cell
        update facing from point.facing
        point_index += 1
        budget -= 7
    store remaining budget as residual for next tick
    interpolate visual position from residual for smooth sub-step rendering
```

## Speed System Integration

Process_Movement computes speed before assigning a track:

1. Look up `SpeedType × LandType` from `g_SpeedType_LandType_Table`
2. Apply slope multiplier (uphill/downhill from `RulesClass`)
3. Apply damaged-speed penalty if health below `ConditionYellowPct`
4. Store as double at locomotor+0x50
5. Feed to `Process_Drive_Track` as movement budget increment

## Rust Implementation Status

### What's Done

| Component | Status |
|-----------|--------|
| TurnTrack table (72 entries) | Extracted, matches binary |
| RawTrack metadata (16 entries) | Extracted, matches binary |
| Track 1 points (straight N, 24 pts) | Extracted |
| Track 2 points (straight NE, 32 pts) | Extracted |
| Track 3 points (45° curve, 55 pts) | Extracted |
| Track 4 points (90° curve, 38 pts) | Extracted |
| Track selection (`select_drive_track`) | Implemented |
| Track stepping (`advance_drive_track`) | Implemented |
| Movement budget system (cost=7) | Implemented |
| Cell crossing detection (jump_index) | Implemented |
| Track chaining at end | Implemented |
| Integration in movement.rs | Fully wired |
| GameEntity.drive_track field | Present |

### What's Missing

| Component | Priority | Notes |
|-----------|----------|-------|
| Track 5 point data (wide curve, 60 pts) | High | At 0x7e6968, cell-crossing |
| Track 6 point data (wide curve, 56 pts) | High | At 0x7e6c50, cell-crossing |
| Track 7–10 point data (short curves) | Medium | For high-speed vehicles |
| Track 11–15 point data (specials) | Low | Used by entries 64–71 only |
| Short track selection (use_short) | Medium | Hardcoded `false` in movement.rs |
| TrackPoint field sizes | Minor | Rust uses i16/i16/u8 vs binary i32/i32/i32 |

### Impact of Missing Data

Without tracks 5–6, vehicles making **wide turns (>90°)** that cross cells during
the curve will fall back to straight-line movement instead of smooth arcs. Without
tracks 7–10, high-speed vehicles won't use tighter curves. The system degrades
gracefully — missing tracks cause `select_drive_track` to return `None`, and the
vehicle falls back to stop-rotate-go movement.

## Address Reference

| Address | Label | Size |
|---------|-------|------|
| 0x4af540 | DriveLocomotionClass::Constructor | 154 bytes (corrected 2026-05-29: was 160; body 0x4af540–0x4af5d9 = 0x9A = 154 via get_function_by_address — OFFSET_RETYPED_WRONG) |
| 0x4b0500 | DriveLocomotionClass__Process (main tick) | ~1450 bytes |
| 0x4b0ad0 | Apply_Track_Delta | ~365 bytes (corrected 2026-05-29: was ~280; body 0x4b0ad0–0x4b0c3d = 0x16E = 366 via get_function_by_address — OFFSET_RETYPED_WRONG) |
| 0x4b0f20 | Process_Drive_Track | ~5860 bytes |
| 0x4b2630 | Process_Movement | ~8500 bytes |
| 0x4b4780 | Transform_Track_Coords | ~123 bytes (corrected 2026-05-29: was ~180; body 0x4b4780–0x4b47fb = 0x7C = 124 via get_function_by_address — OFFSET_RETYPED_WRONG) |
| 0x7e6258 | Track 1 point data (straight N) | 288 bytes |
| 0x7e6378 | Track 2 point data (straight NE) | 384 bytes |
| 0x7e64f8 | Track 3 point data (45° curve) | 660 bytes |
| 0x7e6790 | Track 4 point data (90° curve) | 456 bytes |
| 0x7e6968 | Track 5 point data (wide A) | 720 bytes |
| 0x7e6c50 | Track 6 point data (wide B) | 672 bytes |
| 0x7e6f00 | Track 7 point data (short A) | 264 bytes |
| 0x7e7050 | Track 8 point data (short B) | 264 bytes |
| 0x7e7158 | Track 9 point data (short C) | 288 bytes |
| 0x7e72d0 | Track 10 point data (short D) | 264 bytes |
| 0x7e7420 | Track 11 point data (special A) | 168 bytes |
| 0x7e7a28 | g_DriveTrackData_Array (RawTrack[16]) | 256 bytes (corrected 2026-05-29: was 192; 16 entries × 16 bytes = 256, spans to 0x7e7b28 verified via read_memory — OFFSET_RETYPED_WRONG) |
| 0x7e7b28 | g_DriveTrackIndex_Table (TurnTrack[72]) | 864 bytes |
| 0x7e7eb0 | DriveLocomotionClass ILocomotion vtable | 204 bytes (corrected 2026-05-29: was 96; spans 0x7e7eb0–0x7e7f7b = 51 slots before IUnknown vtable at 0x7e7f7c, verified via read_memory — OFFSET_RETYPED_WRONG) |
| 0x7e7f7c | DriveLocomotionClass IUnknown vtable | 12 bytes |

## DriveLocomotionClass Object Layout

```
+0x00  IUnknown vtable          (0x7e7f7c)
+0x04  ILocomotion vtable       (0x7e7eb0)
+0x08  ref_count
+0x0C  FootClass* linked        (the game entity)
+0x18  IPiggyback vtable        (0x7e7e8c)
+0x1C  field_1C
+0x20  field_20
+0x24  frame_counter
+0x34  destination coord (x,y,z)
+0x40  head_to coord (x,y,z)    (next cell target)
+0x4C  movement_budget          (residual from stepping loop)
+0x50  current_speed            (double, 8 bytes)
+0x58  track_index              (into TurnTrack table, -1 = none)
+0x5C  point_index              (current pos in track point array)
+0x60  use_short_track          (byte, selects short variant)
+0x61  flag
+0x62  flag
+0x63  is_on_track              (byte, set when actively following)
+0x64  on_bridge_approach       (byte)
+0x65  initialized              (byte, set to 1 in constructor)
+0x68  field_68
```
