# Movement Facing Direction Mapping Trace

Scenario: Grizzly-style vehicle at cell `(50,50)` receives adjacent move steps east and southeast on flat terrain. This trace is limited to direction ids, 8-bit body-facing bytes, helper conversions, and the current Rust movement setup/tick surfaces that consume those values.

## Verdict Tally

PASS: 6 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

## Pipeline

1. Trigger: move command targets adjacent cell.
2. Data: `[MTNK]` in `ini/rulesmd.ini` is Grizzly Battle Tank, `Speed=7`, `ROT=5`, drive locomotor `{4A582741-9839-11d1-B709-00A024DDAFD1}`.
3. Direction mapping: adjacent cell delta maps to gamemd direction id and facing byte.
4. Rust helper conversion: `facing_from_delta`, `dir_to_cell_delta`, `facing_to_movement`.
5. Movement target setup: path first step becomes `new_facing`, `facing_target`, and lepton move direction.
6. Movement tick: vehicle rotation or drive-track logic consumes the target facing before or during advancement.
7. Screen result: vehicle body points east or southeast while moving to the adjacent cell.

## Concrete Values

### Stage 1 - gamemd Adjacent Direction Table

Rust surface: `src/util/fixed_math.rs:313`, `src/util/fixed_math.rs:330`

Input:
- East step from `(50,50)` to `(51,50)` -> delta `(1,0)`.
- Southeast step from `(50,50)` to `(51,51)` -> delta `(1,1)`.

gamemd:
- Active YR startup table `g_DirectionOffsets @ 0x0089F688`, verified in `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`.
- Direction id `2` = `(1,0)` = East = facing byte `64`.
- Direction id `3` = `(1,1)` = Southeast = facing byte `96`.
- Active in standard YR: yes, report marks the direction table and movement/render consumers active.

Our output:
- Same table order is encoded by `dir_to_cell_delta`: `64 -> (1,0)`, `96 -> (1,1)`.

Verdict: PASS.

### Stage 2 - `facing_from_delta`

Rust surface: `src/util/fixed_math.rs:280`, public wrapper `src/sim/movement/mod.rs:181`

Formula:
- `facing = ((atan2(dx, -dy) / tau) * 256) as i32 mod 256`.

Concrete output:
- East `(dx=1, dy=0)`: `atan2(1,0)=pi/2`; `pi/2 / tau * 256 = 64`; Rust output `64`.
- Southeast `(dx=1, dy=1)`: `atan2(1,-1)=3pi/4`; `3pi/4 / tau * 256 = 96`; Rust output `96`.

gamemd:
- Existing Ghidra report verifies the same `0=N`, `64=E`, `96=SE` facing-byte convention and active vector-to-facing paths.

Verdict: PASS.

### Stage 3 - `dir_to_cell_delta`

Rust surface: `src/util/fixed_math.rs:330`

Formula:
- `dir = ((facing + 16) / 32) & 7`, then table lookup.

Concrete output:
- `facing=64`: `(64+16)/32 = 2`, direction id `2`, delta `(1,0)`.
- `facing=96`: `(96+16)/32 = 3`, direction id `3`, delta `(1,1)`.

gamemd:
- `0..7 = N,NE,E,SE,S,SW,W,NW`; East id `2`, Southeast id `3`.

Verdict: PASS.

### Stage 4 - `facing_to_movement`

Rust surface: `src/util/facing_table.rs:88`

Formula:
- Rust: `dx = sin(facing) * speed`, `dy = -cos(facing) * speed`.
- Existing gamemd report verifies the same sine/cosine convention for active facing-to-vector consumers.

Concrete output:
- East `64`: Rust table has `sin(64)=1`, `cos(64)=0`; output `(speed, 0)`.
- Southeast `96`: Rust output is approximately `(0.7071*speed, 0.7071*speed)`.

gamemd:
- Formula and signs match verified active YR code.
- Exact raw trig-table rounding for this ground vehicle step was not recomputed from gamemd in this trace.

Verdict: PASS for axis/sign convention; UNCHECKED for exact diagonal fixed-point magnitude.

### Stage 5 - Initial Movement Command Setup

Rust surface: `src/sim/movement/movement_commands.rs:153`, `src/sim/movement/movement_commands.rs:338`, `src/sim/movement/movement_commands.rs:350`, `src/sim/movement/movement_commands.rs:379`

Concrete output:
- East path `[ (50,50), (51,50) ]` computes `new_facing=64`; `move_dir=(256,0)`, `move_dir_len=256`.
- Southeast path `[ (50,50), (51,51) ]` computes `new_facing=96`; `move_dir=(256,256)`, `move_dir_len=362.038`.
- For a Grizzly-style vehicle with `ROT=5`, current Rust stores `entity.facing_target = Some(new_facing)` instead of instantly writing `entity.facing`.

gamemd:
- Direction id and facing byte match the verified active YR table.
- Exact adjacent diagonal lepton length and speed progression were not recomputed from gamemd in this trace.

Verdict: PASS for target facing bytes; UNCHECKED for exact lepton-length/timing parity.

### Stage 6 - Path Transition / Tick Update Facing

Rust surface: `src/sim/movement/movement_step.rs:70`, `src/sim/movement/movement_step.rs:87`, `src/sim/movement/movement_step.rs:131`, `src/sim/movement/movement_tick.rs:816`

Concrete output:
- On transition to a next path cell, Rust recomputes `new_face = facing_from_delta(ndx, ndy)`.
- For East `ndx=1, ndy=0`, this is `64`.
- For Southeast `ndx=1, ndy=1`, this is `96`.
- Drive-track chaining also compares `next_face` against the active track target facing.

gamemd:
- Direction ids/facing bytes match the active table.
- Exact per-tick body-facing sequence depends on the vehicle's starting facing and chosen drive track; the scenario does not provide starting facing, and this trace did not execute gamemd frame-by-frame.

Verdict: PASS for recomputed target facing bytes; UNCHECKED for exact per-tick body-facing timeline.

### Stage 7 - Drive Track Direction Quantization

Rust surface: `src/sim/movement/drive_track.rs:3467`, `src/sim/movement/drive_track.rs:3547`

Formula:
- `facing_to_dir(facing) = ((facing + 16) / 32) % 8`.

Concrete output:
- `64 -> 2`, East.
- `96 -> 3`, Southeast.

gamemd:
- Same direction ids in the verified active YR direction table.

Verdict: PASS.

## Failures

None found for the concrete East/Southeast direction ids and facing bytes.

## Not Implemented

None found in this trace.

## Unchecked Items

- Exact diagonal `facing_to_movement(96, speed)` fixed-point magnitude against gamemd's raw trig table.
- Exact `move_dir_len=362.038` diagonal lepton-length parity against gamemd for this ground vehicle step.
- Exact per-tick body-facing timeline for a drive locomotor, because the scenario does not specify the vehicle's starting facing and no runtime gamemd tick trace was performed.

## Adjacent Findings

- `src/util/fixed_math.rs:284` comments describe `(dx=1,dy=0)` as NE and `(dx=1,dy=1)` as E, but the code and verified gamemd mapping produce East `64` and Southeast `96`. This is a stale comment, not a player-visible mismatch.
- A read-only Ghidra decompile attempt for the documented addresses failed with "Function not found" in the current MCP session, so this trace relies on `FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md` for binary evidence and current Rust source inspection for our outputs.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Sources

- `docs/research/FACING_BYTE_VS_DIRECTION_INDEX_GHIDRA_REPORT.md`
- `ini/rulesmd.ini:6603`
- `src/util/fixed_math.rs:280`
- `src/util/fixed_math.rs:330`
- `src/util/facing_table.rs:88`
- `src/sim/movement/mod.rs:181`
- `src/sim/movement/movement_commands.rs:153`
- `src/sim/movement/movement_step.rs:70`
- `src/sim/movement/movement_tick.rs:816`
- `src/sim/movement/drive_track.rs:3467`

Status: COMPLETE
