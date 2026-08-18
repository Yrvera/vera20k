# Facing To Direction Quantization Trace

Scenario: Verify gamemd.exe quantization from an 8-bit facing byte to direction id `0..7` around compass centers and half-direction bucket boundaries. Compare against Rust `dir_to_cell_delta` and drive-track `facing_to_dir` formulas only.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Pipeline

1. Trigger: a movement/drive-track consumer needs to convert a facing to an adjacent direction bucket.
2. gamemd data contract: canonical direction ids are `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW`.
3. gamemd quantization: active drive-locomotor code rounds an 8-bit facing to the nearest direction id.
4. Rust quantization: `src/util/fixed_math.rs::dir_to_cell_delta` and `src/sim/movement/drive_track.rs::facing_to_dir` use half-bucket rounding.
5. Result: direction id chooses either a drive-track table index or a cell delta.

## Concrete Values

### Stage 1 - Canonical Direction Centers

Rust surface:
- `src/util/fixed_math.rs:330`
- `src/sim/movement/drive_track.rs:3547`

gamemd evidence:
- `Foundation_direction_table_init @ 0x0049F2F0` initializes `g_DirectionOffsets @ 0x0089F688`.
- Verified direction table:

| Facing center | gamemd dir | gamemd delta | Rust `dir_to_cell_delta` |
|---:|---:|---:|---:|
| `0` | `0` | `(0,-1)` | `(0,-1)` |
| `32` | `1` | `(1,-1)` | `(1,-1)` |
| `64` | `2` | `(1,0)` | `(1,0)` |
| `96` | `3` | `(1,1)` | `(1,1)` |
| `128` | `4` | `(0,1)` | `(0,1)` |
| `160` | `5` | `(-1,1)` | `(-1,1)` |
| `192` | `6` | `(-1,0)` | `(-1,0)` |
| `224` | `7` | `(-1,-1)` | `(-1,-1)` |

Active in standard YR: Yes. The direction table is initialized before gameplay and consumed by live path, bridge, wall, and drive-locomotor systems. Stock `[MTNK]` and many standard vehicles use drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}` in `ini/rulesmd.ini`.

Verdict: PASS. All eight center values match exactly.

### Stage 2 - gamemd Facing Bucket Formula

Rust surface:
- `src/sim/movement/drive_track.rs:3547`

gamemd evidence:
- `DriveLocomotionClass__Can_Use_Track @ 0x004B4B00` converts `g_DriveTrackDirection_Table` target facing with:

```text
dir = (((facing16 >> 12) + 1) >> 1) & 7
```

- The target facing byte is first shifted to 16-bit form, so for an 8-bit byte this is:

```text
dir = (((facing8 >> 4) + 1) >> 1) & 7
```

- `DriveLocomotionClass__Process_Movement @ 0x004B2630` uses the same `((*RateTimer_Current >> 12) + 1) >> 1 & 7` form when stepping ahead by current facing.

Active in standard YR: Yes. `DriveLocomotionClass__Process @ 0x004B0500` calls movement and drive-track processing in the standard drive locomotor path. `Can_Use_Track` is in the DriveLocomotion vtable at `0x007E7F54`.

Rust output:
- `facing_to_dir(facing) = (facing.wrapping_add(16) / 32) % 8`.

Exhaustive comparison over `0..=255`:
- gamemd formula mismatches vs Rust formula: `0`.

Verdict: PASS. The formulas are numerically identical for every 8-bit facing byte.

### Stage 3 - Boundary Samples

Boundary rule:
- Values `0..15` round to `0` (N).
- Values `16..47` round to `1` (NE).
- Values `48..79` round to `2` (E).
- Values `80..111` round to `3` (SE).
- Values `112..143` round to `4` (S).
- Values `144..175` round to `5` (SW).
- Values `176..207` round to `6` (W).
- Values `208..239` round to `7` (NW).
- Values `240..255` wrap/round to `0` (N).

Concrete samples:

| Facing | gamemd dir | Rust dir |
|---:|---:|---:|
| `15` | `0` | `0` |
| `16` | `1` | `1` |
| `47` | `1` | `1` |
| `48` | `2` | `2` |
| `79` | `2` | `2` |
| `80` | `3` | `3` |
| `111` | `3` | `3` |
| `112` | `4` | `4` |
| `143` | `4` | `4` |
| `144` | `5` | `5` |
| `175` | `5` | `5` |
| `176` | `6` | `6` |
| `207` | `6` | `6` |
| `208` | `7` | `7` |
| `239` | `7` | `7` |
| `240` | `0` | `0` |
| `255` | `0` | `0` |

Verdict: PASS. Half-direction boundaries round upward at `16 + 32n`; the northwest/north boundary wraps at `240`.

### Stage 4 - Rust `dir_to_cell_delta`

Rust surface:
- `src/util/fixed_math.rs:330`

Formula:

```text
dir = (facing.wrapping_add(16) / 32) & 7
delta = [(0,-1), (1,-1), (1,0), (1,1), (0,1), (-1,1), (-1,0), (-1,-1)][dir]
```

Comparison:
- Since Stage 2 proves exact direction-id equality for all 256 facing bytes, and Stage 1 proves the direction table equality, `dir_to_cell_delta` produces the same adjacent cell delta as gamemd for all valid 8-bit facings.
- Existing Rust tests cover centers and selected boundaries: `src/util/fixed_math.rs:818` and `src/util/fixed_math.rs:830`.

Verdict: PASS.

### Stage 5 - Rust Drive-Track `facing_to_dir`

Rust surface:
- `src/sim/movement/drive_track.rs:3467`
- `src/sim/movement/drive_track.rs:3522`
- `src/sim/movement/drive_track.rs:3547`

Formula:

```text
dir = (facing.wrapping_add(16) / 32) % 8
```

Comparison:
- Matches gamemd for all `0..=255` facing bytes.
- The helper feeds `select_drive_track(current_facing, next_facing, ...)`, which computes `turn_index = from_dir * 8 + to_dir`.
- gamemd `DriveLocomotionClass__Process_Movement @ 0x004B2630` computes drive-track index as `next_dir + current_dir * 8` for the normal 8x8 turn table. This is the same indexing layout.

Verdict: PASS.

## Failures

None.

## Not Implemented

None for this scoped mechanic.

## Unchecked Items

None inside this scenario.

## Adjacent Findings

- `src/map/resolved_terrain.rs::direction_offset` wraps invalid direction values with `direction & 7`. That is adjacent to canonical direction validation, not this facing-byte quantization trace.
- 32-way render/VXL facing bucket formulas are separate renderer questions and were not traced here.
- `src/util/fixed_math.rs` existing boundary tests do not exhaust all half-bucket boundaries, although the implementation matches the binary formula for all 256 values.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Sources

- Ghidra read-only decompile: `DriveLocomotionClass__Can_Use_Track @ 0x004B4B00`
- Ghidra read-only decompile: `DriveLocomotionClass__Process @ 0x004B0500`
- Ghidra read-only decompile: `DriveLocomotionClass__Process_Movement @ 0x004B2630`
- Ghidra xref: `DriveLocomotionClass__Can_Use_Track` vtable data reference at `0x007E7F54`
- Existing research: `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- Existing research: `docs/research/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`
- Existing research: `docs/research/DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md`
- Rust: `src/util/fixed_math.rs:330`
- Rust: `src/sim/movement/drive_track.rs:3547`
- Rust: `src/sim/movement/drive_track.rs:3467`
- INI: `ini/rulesmd.ini:6603`
- INI: `ini/rulesmd.ini:6636`

Status: COMPLETE
