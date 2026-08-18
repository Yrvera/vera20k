# Facing Byte Table Compass Names Trace

Scenario: Verify gamemd.exe 8-bit facing bytes for the eight compass names only:
`N=0`, `NE=32`, `E=64`, `SE=96`, `S=128`, `SW=160`, `W=192`, `NW=224`.
Compare against current Rust facing helper tests and consumers. Adjacent direction-id,
rounding-boundary, projectile-frame, and drive-track timing findings are out of scope.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Pipeline

1. gamemd startup direction table establishes compass order.
2. gamemd facing convention maps compass order to 8-bit facing bytes.
3. Rust `facing_from_delta_int` computes compass-name bytes from cell deltas.
4. Rust inverse/consumer helpers consume the same byte table.
5. Rust movement/combat consumers route those facing bytes into entity facing.
6. Test coverage checks the table, with one strictness caveat for diagonals.

## Stage 1 - gamemd Startup Direction Table

Rust surface for comparison: `src/util/fixed_math.rs:330`

gamemd evidence:

- Direct read-only Ghidra decompile of `Foundation_direction_table_init @ 0x0049F2F0` writes:
  - `0x0089F688 = (0,-1)`
  - `0x0089F68C = (1,-1)`
  - `0x0089F690 = (1,0)`
  - `0x0089F694 = (1,1)`
  - `0x0089F698 = (0,1)`
  - `0x0089F69C = (-1,1)`
  - `0x0089F6A0 = (-1,0)`
  - `0x0089F6A4 = (-1,-1)`
- `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` identifies this as the active standard YR adjacent-cell direction table.
- `list_data_items_by_xrefs` reports `g_DirectionOffsets @ 0x0089F688` with 518 xrefs, so the table is not dormant TS-only data.

Computed gamemd output:

| Compass | Direction index | Cell delta |
|---|---:|---:|
| N | 0 | `(0,-1)` |
| NE | 1 | `(1,-1)` |
| E | 2 | `(1,0)` |
| SE | 3 | `(1,1)` |
| S | 4 | `(0,1)` |
| SW | 5 | `(-1,1)` |
| W | 6 | `(-1,0)` |
| NW | 7 | `(-1,-1)` |

Verdict: PASS.

## Stage 2 - gamemd Facing Byte Table

Rust surface for comparison: `src/util/fixed_math.rs:280`

gamemd evidence:

- `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` states the 8-bit facing byte uses the same compass origin and clockwise order as the adjacent direction table.
- `DRIVE_TRACK_TABLES_DEEP_DECODE.md` independently decodes TurnTrack target-facing bytes as:
  `0x00=N`, `0x20=NE`, `0x40=E`, `0x60=SE`, `0x80=S`, `0xA0=SW`, `0xC0=W`, `0xE0=NW`.
- `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md` independently reduces 16-bit facing to 3-bit compass direction and lists `0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW`.
- Active in standard YR: yes. Evidence paths include unit movement/scatter, drive locomotion, projectile/facing helpers, and `g_DirectionOffsets` runtime consumers.

Computed gamemd output:

| Compass | 8-bit facing byte | Hex |
|---|---:|---:|
| N | 0 | `0x00` |
| NE | 32 | `0x20` |
| E | 64 | `0x40` |
| SE | 96 | `0x60` |
| S | 128 | `0x80` |
| SW | 160 | `0xA0` |
| W | 192 | `0xC0` |
| NW | 224 | `0xE0` |

Verdict: PASS.

## Stage 3 - Rust `facing_from_delta_int`

Rust surface: `src/util/fixed_math.rs:280`

Formula:

```text
if dx == 0 && dy == 0: facing = 0
angle = atan2(dx, -dy)
facing = trunc(angle / tau * 256) mod 256
```

Computed Rust output for the scenario's exact compass deltas:

| Compass | Input delta | Rust output | gamemd output |
|---|---:|---:|---:|
| N | `(0,-1)` | 0 | 0 |
| NE | `(1,-1)` | 32 | 32 |
| E | `(1,0)` | 64 | 64 |
| SE | `(1,1)` | 96 | 96 |
| S | `(0,1)` | 128 | 128 |
| SW | `(-1,1)` | 160 | 160 |
| W | `(-1,0)` | 192 | 192 |
| NW | `(-1,-1)` | 224 | 224 |

Verdict: PASS.

## Stage 4 - Rust Inverse Helper and Drive Quantization

Rust surfaces:

- `src/util/fixed_math.rs:330`
- `src/sim/movement/drive_track.rs:3547`

Rust formulas:

```text
dir_to_cell_delta: dir = ((facing + 16) / 32) & 7
drive facing_to_dir: dir = ((facing + 16) / 32) % 8
```

Computed Rust output at exact compass facings:

| Facing | Rust dir | Rust delta | gamemd dir/delta |
|---:|---:|---:|---:|
| 0 | 0 | `(0,-1)` | `0 / (0,-1)` |
| 32 | 1 | `(1,-1)` | `1 / (1,-1)` |
| 64 | 2 | `(1,0)` | `2 / (1,0)` |
| 96 | 3 | `(1,1)` | `3 / (1,1)` |
| 128 | 4 | `(0,1)` | `4 / (0,1)` |
| 160 | 5 | `(-1,1)` | `5 / (-1,1)` |
| 192 | 6 | `(-1,0)` | `6 / (-1,0)` |
| 224 | 7 | `(-1,-1)` | `7 / (-1,-1)` |

Verdict: PASS.

## Stage 5 - Current Rust Consumers

Rust consumers:

- Movement command setup calls `facing_from_delta`: `src/sim/movement/movement_commands.rs:118`, `:344`
- Movement step/tick recomputes path-step facing: `src/sim/movement/movement_step.rs:87`, `src/sim/movement/movement_tick.rs:328`, `:816`
- Combat target-facing writes call the same helper: `src/sim/combat/mod.rs:477`, `:543`
- Air, rocket, tube, tunnel movement use the same helper path.
- Turret body-facing expansion uses `body << 8`: `src/sim/movement/turret.rs:69`

Computed output:

- For exact compass cell deltas, all listed consumers receive the same Rust bytes from Stage 3.
- For exact compass facing bytes, drive-track quantization receives the same direction ids from Stage 4.
- gamemd has the same active compass/facing table from Stages 1-2.

Verdict: PASS for the byte table values consumed by these paths.

## Stage 6 - Current Test Coverage

Rust test surfaces:

- `src/util/fixed_math.rs:485`
- `src/util/fixed_math.rs:499`
- `src/util/fixed_math.rs:819`
- `src/sim/movement/drive_track_tests.rs:211`

Findings:

- Cardinal `facing_from_delta_int` tests assert exact equality for `N/E/S/W`.
- `dir_to_cell_delta` tests assert exact equality for all eight compass facings.
- Drive-track `facing_to_dir` tests assert exact equality for all eight compass facings.
- Diagonal `facing_from_delta_int` tests currently allow +/-1 instead of asserting exact `32/96/160/224`.

Verdict: UNCHECKED for exact diagonal test strictness. The helper output computed in Stage 3 matches gamemd, but the existing diagonal tests would not catch a one-byte drift.

## Failures

None for the scoped compass-name facing byte table.

## Not Implemented

None for the scoped compass-name facing byte table.

## Unchecked Items

- Exact diagonal `facing_from_delta_int` test strictness: current tests tolerate +/-1 for diagonal helper outputs even though gamemd's named compass bytes are exact multiples of 32.

## Adjacent Findings

- `src/util/fixed_math.rs:284` still has stale explanatory comments saying `(dx=1,dy=0)` is NE and `(dx=1,dy=1)` is E. The code and tests produce `E=64` and `SE=96`, matching gamemd. This is documentation drift, not a player-visible mismatch.
- Rounding-boundary behavior around values such as `15/16` and invalid direction bytes is intentionally outside this slot.
- Direction id `8` tube sentinel and invalid `9..255` direction-index behavior are outside this slot.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Sources

- Direct read-only Ghidra decompile: `Foundation_direction_table_init @ 0x0049F2F0`
- Direct read-only Ghidra data-xref count: `g_DirectionOffsets @ 0x0089F688`
- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `docs/research/DRIVE_TRACK_TABLES_DEEP_DECODE.md`
- `docs/research/UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`
- `src/util/fixed_math.rs`
- `src/sim/movement/drive_track.rs`
- `src/sim/movement/drive_track_tests.rs`

Status: COMPLETE
