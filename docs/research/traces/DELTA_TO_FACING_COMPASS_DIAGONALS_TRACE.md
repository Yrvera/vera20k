# Delta to Facing Compass Diagonals Trace

Scenario: verify conversion from adjacent cell deltas to 8-bit facing bytes for all eight compass steps, especially `NE=(1,-1)->32`, `SE=(1,1)->96`, `SW=(-1,1)->160`, and `NW=(-1,-1)->224`. Compare against Rust `facing_from_delta_int` and the movement wrapper.

Scope lock: one mechanic only, adjacent cell-delta to facing-byte mapping. Direction-id tables and movement consumers are used only as evidence for this mapping. Direction `8`, invalid directions, drive-track curve timing, projectile frame mapping, and VXL draw buckets are adjacent/out of scope.

Report path: `docs/research/traces/DELTA_TO_FACING_COMPASS_DIAGONALS_TRACE.md`

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Pipeline

1. Trigger: movement/path logic receives an adjacent next-cell delta.
2. gamemd reference: active YR direction/facing contract maps the delta to a compass direction and facing byte.
3. Rust helper: `facing_from_delta_int(dx, dy)` computes an 8-bit facing byte.
4. Movement wrapper: `sim::movement::facing_from_delta(dx, dy)` delegates to the helper.
5. Movement consumers: move setup and path transition store or target that facing.

## Stage Trace

### Stage 1 - gamemd Adjacent Direction / Facing Contract

Rust comparison surface: `src/util/fixed_math.rs:280`, `src/util/fixed_math.rs:330`

Concrete gamemd outputs:

| Name | Delta `(dx,dy)` | Direction id | Facing byte |
|---|---:|---:|---:|
| N | `(0,-1)` | `0` | `0` |
| NE | `(1,-1)` | `1` | `32` |
| E | `(1,0)` | `2` | `64` |
| SE | `(1,1)` | `3` | `96` |
| S | `(0,1)` | `4` | `128` |
| SW | `(-1,1)` | `5` | `160` |
| W | `(-1,0)` | `6` | `192` |
| NW | `(-1,-1)` | `7` | `224` |

gamemd evidence:
- `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` section 3.1 gives the direction table and facing equivalents above.
- `CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md` section 3 verifies `Foundation_direction_table_init @ 0x0049F2F0` writes the same signed neighbor offsets.
- Active in standard YR: yes. The cited reports mark the startup table, A*/path, bridge, wall, Fire_At, and HomingTrack facing consumers as live YR paths. This is not TS-only dormant code.

Tiny details:
- No rounding is needed for this table stage. Direction ids are exact integers and facing bytes are `direction_id * 32`.
- Direction `8` is excluded from this trace; existing direction reports verify it is a tube sentinel, not a compass delta.

Verdict: PASS. The gamemd expected outputs are literal numeric values for all eight adjacent compass deltas.

### Stage 2 - Rust `facing_from_delta_int`

Rust surface: `src/util/fixed_math.rs:280`

Formula:

```text
if dx == 0 && dy == 0: return 0
angle_rad = atan2(dx, -dy)
facing = int(angle_rad / tau * 256) mod 256
```

Concrete Rust outputs, computed from the helper formula:

| Name | Delta `(dx,dy)` | Rust output | gamemd output |
|---|---:|---:|---:|
| N | `(0,-1)` | `0` | `0` |
| NE | `(1,-1)` | `32` | `32` |
| E | `(1,0)` | `64` | `64` |
| SE | `(1,1)` | `96` | `96` |
| S | `(0,1)` | `128` | `128` |
| SW | `(-1,1)` | `160` | `160` |
| W | `(-1,0)` | `192` | `192` |
| NW | `(-1,-1)` | `224` | `224` |

Verification notes:
- Existing Rust tests assert exact cardinal outputs at `src/util/fixed_math.rs:485`.
- Existing Rust tests assert diagonal buckets at `src/util/fixed_math.rs:499`; the helper formula computes the exact expected values for these unit deltas.
- A local arithmetic check using the same formula produced exactly `0,32,64,96,128,160,192,224`.

Tiny details:
- The helper casts positive exact multiples to `i32`; the eight unit compass angles are exact facing-bucket boundaries for this formula.
- Negative angles are normalized with `rem_euclid(256)`.
- Zero delta returns `0`, but zero delta is outside this adjacent-step scenario.

Verdict: PASS. All eight Rust helper outputs are numerically equal to gamemd's facing-byte contract.

### Stage 3 - Rust Movement Wrapper

Rust surface: `src/sim/movement/mod.rs:181`

Concrete behavior:
- `facing_from_delta(dx, dy)` directly returns `facing_from_delta_int(dx, dy)`.
- Therefore the movement wrapper outputs the same eight bytes as Stage 2:
  `N=0`, `NE=32`, `E=64`, `SE=96`, `S=128`, `SW=160`, `W=192`, `NW=224`.

gamemd comparison:
- Same Stage 1 expected bytes.

Tiny details:
- No additional clamp, rounding, remap, or delay exists in this wrapper.

Verdict: PASS. The wrapper preserves the helper's numerically matching outputs.

### Stage 4 - Movement Setup / Path Transition Consumption

Rust surfaces:
- Initial command setup: `src/sim/movement/movement_commands.rs:338`
- Path transition: `src/sim/movement/movement_step.rs:70`

Concrete behavior:
- Initial move setup computes `dx = path[1].x - start.x`, `dy = path[1].y - start.y`, then `new_facing = facing_from_delta(dx, dy)`.
- Path transition computes `ndx = next.x - current.x`, `ndy = next.y - current.y`, then `new_face = facing_from_delta(ndx, ndy)`.
- For adjacent compass steps, the stored/targeted bytes are exactly:
  - `(0,-1)->0`
  - `(1,-1)->32`
  - `(1,0)->64`
  - `(1,1)->96`
  - `(0,1)->128`
  - `(-1,1)->160`
  - `(-1,0)->192`
  - `(-1,-1)->224`

gamemd comparison:
- Same Stage 1 expected bytes for adjacent cell steps.

Tiny details:
- The movement command reads path cells before setting facing.
- For drive locomotor units, Rust may store `facing_target = Some(new_facing)` instead of immediately overwriting body `facing`; this changes timing of rotation, not the target byte verified here.
- For non-drive or instant-facing paths, Rust writes the same `new_facing` byte directly.

Verdict: PASS for target facing-byte selection. The target byte for each adjacent delta equals gamemd's direction/facing contract.

### Stage 5 - Runtime Body-Facing Timeline

Rust surfaces:
- Drive-track selection: `src/sim/movement/drive_track.rs:3467`
- Movement tick consumers: `src/sim/movement/movement_tick.rs`

Concrete checked value:
- The target facing byte that enters rotation/drive-track logic is verified in Stage 4.

Unchecked value:
- The exact per-tick body-facing sequence after the target is set was not recomputed against a live gamemd runtime trace for all eight starting facings and all eight adjacent deltas.

Why unchecked:
- The requested scenario is the helper-level delta-to-facing conversion.
- Per-tick drive-track timing depends on current facing, unit type, `ROT`, selected track, and path state.

Verdict: UNCHECKED for runtime rotation timeline only. This does not invalidate the PASS result for the helper target bytes.

## Failures

None found for the scoped adjacent delta to facing-byte conversion.

## Not Implemented

None found for the scoped helper conversion.

## Unchecked Items

- Exact per-tick body-facing timeline for every drive-locomotor start-facing and adjacent step combination. The target byte is verified; the rotation timeline is a separate movement trace.

## Adjacent Findings

- `src/util/fixed_math.rs:284..288` contains stale comments that describe `(dx=1,dy=0)` as `NE` and `(dx=1,dy=1)` as `E`. The code and tests produce `E=64` and `SE=96`, matching gamemd. This is a documentation/comment issue, not a player-visible behavior mismatch.
- `src/util/fixed_math.rs:499` diagonal tests allow `<= 1` bucket tolerance even though these four adjacent diagonals currently compute exact values `32,96,160,224`.
- A read-only Ghidra MCP spot-check could not decompile the documented addresses in this session (`Function not found` for `0x0049F2F0`, `0x0042AA90`, `0x005B20F0`), and static memory at `0x0089F688` reads zero before runtime constructor execution. This report therefore relies on the existing verified Ghidra reports for binary evidence.

## Sources

- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `docs/research/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`
- `docs/research/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
- `src/util/fixed_math.rs:280`
- `src/util/fixed_math.rs:330`
- `src/sim/movement/mod.rs:181`
- `src/sim/movement/movement_commands.rs:338`
- `src/sim/movement/movement_step.rs:70`

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Status

COMPLETE
