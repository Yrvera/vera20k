# Direction 8 Sentinel And Invalid Direction Bytes Trace

**Scenario:** Verify `gamemd.exe` behavior for direction byte `8` as a tube-step sentinel, distinguish it from normal compass direction ids, and compare known/unknown invalid non-8 byte behavior against Rust `ResolvedTerrainGrid::step_coord_by_direction` / `direction_offset`.

**Report path:** `C:/Users/enok/Documents/ra2-rust-game-docs/traces/DIRECTION_8_SENTINEL_INVALID_BYTES_TRACE.md`

## Verdict

PASS: 4 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Scope Lock

This report traces one helper contract only:

- Direction `8` in the active path-coordinate helper.
- Invalid non-8 values `9..=255` only as far as the same helper path exposes them.

Adjacent path smoothing, full A* edge costs, full TubeClass producer validity, bridge marker geometry, and direction/facing rendering are out of scope.

## Evidence Inputs

- Read-only Ghidra decompile: `MapCoord_Step_By_Direction @ ram:0042D490`.
- Read-only Ghidra decompile: `Path_walk_directions_to_cell @ ram:00429780`.
- Read-only Ghidra decompile: `PathfinderClass__UpdateBridgePassability @ ram:0042ACF0`.
- Existing research: `C:/Users/enok/Documents/ra2-rust-game-docs/CANONICAL_DIRECTION_ENCODING_GHIDRA_REPORT.md`.
- Existing research: `C:/Users/enok/Documents/ra2-rust-game-docs/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- Existing trace: `C:/Users/enok/Documents/ra2-rust-game-docs/traces/DIRECTION8_TUBE_STEP_REFERENCE_TRACE.md`.
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/map/resolved_terrain.rs:298`.
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/map/resolved_terrain.rs:1153`.
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/map/bridge_facts.rs:260`.
- Rust code: `C:/Users/enok/Documents/ra2-rust-game/src/map/tubes.rs:68`.

## Active YR Confirmation

The checked `gamemd.exe` paths are active in standard Yuri's Revenge:

- `MapCoord_Step_By_Direction @ 0x0042D490` is the generic map-coordinate step helper and is cited by the canonical direction report as active YR behavior.
- `Path_walk_directions_to_cell @ 0x00429780` replays direction arrays over the same contract.
- `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` is called by live A* under `PathfinderClass+0x3C != 0`; existing research marks this active in YR, not TS-only legacy.

No TS-only dormant path is used as evidence.

## Pipeline

1. Trigger: a path/helper consumer receives current cell coordinate plus direction byte.
2. Branch: if the byte is exactly `8`, use the tube branch.
3. Direction-8 valid tube: read current `CellClass+0x116`, index `g_TubeArray`, and return `TubeClass+0x28`.
4. Direction-8 missing tube: when `CellClass+0x116 == -1`, return packed coord `0`, i.e. `(0,0)`.
5. Non-8 branch: any byte other than `8` indexes `g_DirectionOffsets` directly.
6. Rust comparison: `step_coord_by_direction` special-cases `8`, then calls `direction_offset`; `direction_offset` masks with `direction & 7`.

## Stage Trace

### Stage 1 - Direction 8 With Valid Tube

- Concrete input: `coord=(1,0)`, `direction=8`, current cell has tube index `0`, tube exit is `(2,0)`.
- Rust output: `ResolvedTerrainGrid::step_coord_by_direction` takes the `direction == 8` branch at `src/map/resolved_terrain.rs:299`, `tube_at_cell` returns the tube, and line `302` returns `Some((2,0))`.
- Rust fixture: `src/map/resolved_terrain.rs:1568` asserts `grid.step_coord_by_direction((1,0), 8) == Some((2,0))`.
- gamemd output: `MapCoord_Step_By_Direction @ 0x0042D490` checks `param_3 == 8`, reads `CellClass+0x116`, indexes `g_TubeArray`, and writes `TubeClass+0x28`; for this concrete tube, output is `(2,0)`.
- Timing/order: same helper call; current cell is read before tube array destination; no rounding; no compass-table lookup.
- Verdict: PASS. Both outputs are `(2,0)`.

### Stage 2 - Direction 8 Without Valid Tube

- Concrete input: `coord=(0,0)`, `direction=8`, current cell has no tube index.
- Rust output: line `299` takes the direction-8 branch; `tube_at_cell(0,0)` returns `None`; line `302` returns `Some((0,0))`.
- Rust fixture: `src/map/resolved_terrain.rs:1590` asserts `grid.step_coord_by_direction((0,0), 8) == Some((0,0))`.
- gamemd output: `MapCoord_Step_By_Direction @ 0x0042D490` checks `CellClass+0x116 == -1` and writes packed coord `0`; numeric cell is `(0,0)`.
- Timing/order: same helper call; missing-tube fallback does not consult `g_DirectionOffsets`.
- Verdict: PASS. Both outputs are `(0,0)`.

### Stage 3 - Direction 8 Is Not A Compass Direction

- Concrete input: any current coordinate with `direction=8`.
- Rust output: `step_coord_by_direction` checks `direction == 8` before `direction_offset`, so byte `8` never becomes `8 & 7 == 0` in this helper.
- gamemd output: checked helpers branch on exact `8`; byte `8` never indexes `g_DirectionOffsets[0]`.
- Numeric distinction: if `8` were incorrectly wrapped to `0`, a no-tube input `(10,10)` would step north to `(10,9)`. The verified `gamemd.exe` and current Rust no-tube output is `(0,0)`.
- Verdict: PASS. Both treat `8` as a tube sentinel, not `N`.

### Stage 4 - Path Replay `[2, 8]`

- Concrete input: start `(0,0)`, directions `[2,8]`, cell `(1,0)` has tube exit `(2,0)`.
- Rust output: `walk_directions_from` at `src/map/resolved_terrain.rs:314` steps `2` to `(1,0)`, then direction `8` returns the tube exit `(2,0)`.
- Rust fixture: `src/map/resolved_terrain.rs:1586` asserts final output `Some((2,0))`.
- gamemd output: `Path_walk_directions_to_cell @ 0x00429780` applies non-8 table step for `2`, then exact-8 tube branch to `TubeClass+0x28`; final output is `(2,0)`.
- Timing/order: sequential replay order; the direction-8 branch reads the post-step current coordinate.
- Verdict: PASS. Both final outputs are `(2,0)`.

### Stage 5 - Invalid Non-8 Value `9` In The Generic Helper

- Concrete input: `coord=(10,10)`, `direction=9`, no boundary interaction.
- Rust output: `direction != 8`, then `direction_offset(9)` masks `9 & 7 == 1`; line `1156` returns `(1,-1)`, so `step_coord_by_direction((10,10), 9)` would return `Some((11,9))`.
- gamemd branch contract: `MapCoord_Step_By_Direction @ 0x0042D490` and `Path_walk_directions_to_cell @ 0x00429780` do not mask invalid non-8 values; they index `g_DirectionOffsets + direction * 4` directly.
- gamemd numeric output for `direction=9`: not computed in this trace, because it depends on runtime memory beyond the eight-entry `g_DirectionOffsets` table.
- Verdict: UNCHECKED. Rust's wrapping output is known, and the binary's no-mask contract is known, but the exact gamemd numeric output for `9` was not computed; therefore this cannot be marked PASS or FAIL under the trace-swarm contract.

### Stage 6 - Invalid Non-8 Values `10..=255`

- Rust output: the same `direction & 7` mask makes every invalid non-8 byte map to one of the eight compass offsets. Examples: `10 -> 2 -> (1,0)`, `15 -> 7 -> (-1,-1)`, `255 -> 7 -> (-1,-1)`.
- gamemd branch contract: the checked active helpers treat every value other than exact `8` as a direct table index. No scoped upper-bound check or `& 7` mask was present in the decompiled helper paths.
- gamemd numeric outputs: not computed for `10..=255`; some values would read outside the verified eight-entry direction table into adjacent globals or invalid memory depending on runtime addressability.
- Verdict: UNCHECKED. This is a high-risk helper policy mismatch, but exact player-visible outputs were not computed.

## Failures

None claimed. The invalid-byte policy differs structurally, but exact gamemd numeric outputs for invalid non-8 bytes were not computed, so this report does not mark them as FAIL.

## Not Implemented

None in this scoped helper trace.

## Adjacent Findings

- `src/map/bridge_facts.rs:260` has a separate local `direction_offset` that also masks with `direction & 7`. The scoped binary bridge-direction consumers often gate or intentionally use `(dir - 4) & 7`; this report did not trace invalid bridge authoring values.
- `src/map/tubes.rs:68` rejects authored tube path steps outside `0..=7`. That is consistent with direction `8` being an internal replay/tube sentinel, not a normal map-authored step, but this report did not verify `gamemd.exe` map parser behavior for invalid tube path-step bytes.
- `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` also treats peer path entry `8` as the same tube branch and non-8 entries as direct `g_DirectionOffsets` indices. Its marker-toggling behavior is covered by the bridge marker report and was not re-traced here.
- Positive out-of-range `CellClass+0x116` tube indices are not upper-bound checked in the checked binary consumer before indexing `g_TubeArray`; Rust safely validates tube ids through `ResolvedTerrainGrid::tube`. Normal parity fixtures should use valid tube metadata and document this as a safety boundary.

## Player-Visible Impact

For valid helper inputs, direction `8` parity is good: a path/helper using direction `8` jumps to the tube exit or `(0,0)` exactly like `gamemd.exe`, and it is not confused with north.

For invalid non-8 values, current Rust wraps bytes into plausible compass moves while the checked `gamemd.exe` helpers index directly without masking. If malformed or future unsanitized path bytes ever reach this helper, Rust may move to a clean adjacent cell where `gamemd.exe` would use a different out-of-table value, corrupt path coordinate, or crash. No standard-YR producer in this trace was proven to emit those invalid bytes.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None.

## Status

COMPLETE
