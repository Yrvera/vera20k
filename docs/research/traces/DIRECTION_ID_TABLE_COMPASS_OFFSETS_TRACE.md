# Direction Id Table Compass Offsets Trace

**Scenario:** Verify `gamemd.exe` canonical direction ids `0..7` map to compass names and cell deltas:
`0=N (0,-1)`, `1=NE (1,-1)`, `2=E (1,0)`, `3=SE (1,1)`, `4=S (0,1)`, `5=SW (-1,1)`, `6=W (-1,0)`, `7=NW (-1,-1)`.

**Report path:** `C:/Users/enok/Documents/ra2-rust-game-docs/traces/DIRECTION_ID_TABLE_COMPASS_OFFSETS_TRACE.md`

## Verdict

PASS: 5 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Active YR Confirmation

The `gamemd.exe` evidence is active in standard YR:

- `Foundation_direction_table_init` is a startup constructor reached before `WinMain`, per `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- `MapCoord_Step_By_Direction` is the generic coordinate-step helper and is used by live YR path/pathfinding helper paths, per `CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`.
- A read-only decompile in this trace rechecked both functions. No Ghidra mutation was performed.
- No TS-only dormant path is used as evidence here.

## Pipeline

1. Trigger: a helper receives a cell coordinate and direction id `0..7`.
2. `gamemd.exe` data: startup initializes `g_DirectionOffsets @ 0x0089F688`.
3. `gamemd.exe` step: non-8 direction ids index `g_DirectionOffsets` directly and add signed `(dx,dy)` to the current cell.
4. Rust helpers: current tables/enums return or name the same signed deltas.
5. Player result: movement/path/bridge helper steps land in the same adjacent cell for valid ids `0..7`.

## Stage Trace

### Stage 1 - gamemd.exe Canonical Table Values

- gamemd source: `Foundation_direction_table_init`, read-only decompile.
- Computed gamemd decoded table:
  - `0`: dword `0xFFFF0000` -> `(0,-1)` -> N
  - `1`: dword `0xFFFF0001` -> `(1,-1)` -> NE
  - `2`: dword `0x00000001` -> `(1,0)` -> E
  - `3`: dword `0x00010001` -> `(1,1)` -> SE
  - `4`: dword `0x00010000` -> `(0,1)` -> S
  - `5`: dword `0x0001FFFF` -> `(-1,1)` -> SW
  - `6`: dword `0x0000FFFF` -> `(-1,0)` -> W
  - `7`: dword `0xFFFFFFFF` -> `(-1,-1)` -> NW
- Timing/order: constructor writes the table before normal gameplay; no INI/theater/map input participates.
- Verdict: PASS. The binary table exactly matches the scenario's requested compass offsets.

### Stage 2 - gamemd.exe Non-8 Coordinate Step

- gamemd source: `MapCoord_Step_By_Direction`, read-only decompile.
- Computation: for `param_3 != 8`, the helper reads X from `g_DirectionOffsets + dir*4` and Y from `g_DirectionOffsets + dir*4 + 2`, then adds both signed shorts to the current packed cell coordinate.
- Concrete input from `(10,10)`:
  - `0 -> (10,9)`, `1 -> (11,9)`, `2 -> (11,10)`, `3 -> (11,11)`,
  - `4 -> (10,11)`, `5 -> (9,11)`, `6 -> (9,10)`, `7 -> (9,9)`.
- Timing/order: one helper call; no rounding, no clamp, no branch among `0..7`.
- Verdict: PASS. The live step helper consumes the same table and produces the exact expected adjacent cells.

### Stage 3 - Rust `resolved_terrain::direction_offset`

- Rust site: `C:/Users/enok/Documents/ra2-rust-game/src/map/resolved_terrain.rs:1153`.
- Rust output for valid `0..7`: `(0,-1)`, `(1,-1)`, `(1,0)`, `(1,1)`, `(0,1)`, `(-1,1)`, `(-1,0)`, `(-1,-1)`.
- gamemd output: Stage 1 table values.
- Verdict: PASS for valid ids `0..7`. Literal pairs match.

### Stage 4 - Rust `bridge_facts::direction_offset`

- Rust site: `C:/Users/enok/Documents/ra2-rust-game/src/map/bridge_facts.rs:274`.
- Rust output for valid `0..7`: `(0,-1)`, `(1,-1)`, `(1,0)`, `(1,1)`, `(0,1)`, `(-1,1)`, `(-1,0)`, `(-1,-1)`.
- gamemd output: Stage 1 table values.
- Verdict: PASS for valid ids `0..7`. Literal pairs match.

### Stage 5 - Rust Named Direction Enum and Path-Smooth Deltas

- Rust named enum site: `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs:199`.
- Rust enum discriminants: `N=0`, `NE=1`, `E=2`, `SE=3`, `S=4`, `SW=5`, `W=6`, `NW=7`.
- Rust enum offsets: `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs:212` returns the same eight `(dx,dy)` pairs.
- Rust path-smooth table: `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/path_smooth.rs:31` uses the same eight pairs, and `direction_between` returns the matching index for each adjacent delta at lines 44-52.
- gamemd output: Stage 1 table values.
- Verdict: PASS. Rust named ids and path-smoothing ids match the active YR table exactly.

## Failures

None in this scoped valid-direction table trace.

## Not Implemented

None in this scoped valid-direction table trace.

## Adjacent Findings

- Invalid non-8 directions are outside this scenario. `gamemd.exe` `MapCoord_Step_By_Direction` indexes the direction table directly for any `param_3 != 8`; current Rust helpers in `resolved_terrain.rs` and `bridge_facts.rs` mask with `direction & 7`, so values `9..=255` wrap instead of being rejected or direct-indexed. Do not count that behavior as parity without a separate invalid-direction trace.
- Direction `8` is outside this scenario. Existing traces verify it is a tube sentinel, not a ninth compass offset.
- Current Rust duplicates the same table in multiple files. That is not a parity failure for valid `0..7`, but it is a maintenance risk for future changes.

## Player-Visible Impact

For valid direction ids `0..7`, scoped Rust helpers step to the same neighboring cells as `gamemd.exe`. A player-visible mismatch was not found for the requested compass table.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Sources

- Read-only Ghidra decompile: `Foundation_direction_table_init`.
- Read-only Ghidra decompile: `MapCoord_Step_By_Direction`.
- Existing verified report: `C:/Users/enok/Documents/ra2-rust-game-docs/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- Existing verified report: `C:/Users/enok/Documents/ra2-rust-game-docs/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/map/resolved_terrain.rs`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/map/bridge_facts.rs`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/bridge_state/mod.rs`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/sim/pathfinding/path_smooth.rs`.

## Status

COMPLETE
