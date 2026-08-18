# Paradrop Carrier Edge Selection Trace

Scenario: paradrop target `(50,20)` on a `100x100` Rust map. Trace only carrier edge selection for valid east/south `waypoint_edge` values, blocked-ground-edge behavior, and invalid `waypoint_edge=255` fallback.

## Sources Used

- Rust current implementation: `src/sim/superweapon/paradrop.rs`, `src/sim/world/edge_cell.rs`, `src/sim/house_state.rs`, `src/sim/superweapon/paradrop_tests.rs`.
- Verified gamemd research: `docs/research/PDPLANE_SPAWNER_EDGE_SILENT_PATH_GHIDRA_REPORT.md`, `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`, `LOCAL_GRID_CELL_SKEW_TRANSFORMS_GHIDRA_REPORT.md`.
- Ghidra MCP attempted read-only decompile for `0x004AA440`, `0x004AAB30`, `0x0050DA80`, `0x0065E660`; tool returned function-not-found, so no fresh decompile was available in this slot.

## Active YR Confirmation

The spawner path is active in standard Yuri's Revenge. Stock `rulesmd.ini` binds `[ParaDropSpecial] Type=ParaDrop` and `[AmericanParaDropSpecial] Type=AmerParaDrop`; verified `SuperClass::Launch` cases 5/6 call `FUN_0065E660`, which reads the house edge and calls `FUN_004AA440` for the carrier spawn edge.

## Pipeline

`launch` -> resolve `HouseState.waypoint_edge` -> `Edge::from_index` or north fallback -> `find_paradrop_carrier_edge_cell` -> `spawn_pdplane` at returned edge cell.

Gamemd equivalent: `SuperClass::Launch` cases 5/6 -> `FUN_0065E660` -> read `House+0x1E0` -> if invalid call `FUN_0050DA80` reading fallback `House+0x577C`, then default `0` if invalid -> call `FUN_004AA440(MapClass, edge, sentinel, sentinel, 4, 1, 0)`.

## Stage Verdicts

| Stage | Rust output for scenario | gamemd output/evidence | Verdict |
|---|---:|---|---|
| Live entry path | `launch(... ParaDropKind::American ...)` uses current Rust SW handler | Active YR cases 5/6 call `FUN_0065E660`; stock `Type=ParaDrop`/`AmerParaDrop` | PASS |
| Valid east edge decode | `waypoint_edge=1` -> `Edge::East` | Edge encoding `0=N,1=E,2=S,3=W` | PASS |
| East carrier cell | Rust computes `(99,20)` from `x=99`, closest `y=20` to target | gamemd call uses sentinel/sentinel, `LocalSize` playfield and `FUN_004AA440`; exact east output for this scenario was not recomputed in this slot | UNCHECKED |
| Valid south edge decode | `waypoint_edge=2` -> `Edge::South` | Edge encoding `0=N,1=E,2=S,3=W` | PASS |
| South carrier cell selection | Rust computes `(9,99)`: first 10 candidates `(0..9,99)`, closest to target `(50,20)` | gamemd south mode builds up to 10 candidates, but with spawner sentinel/sentinel arguments chooses a random candidate rather than closest-to-target; exact candidate depends on RNG state | FAIL |
| Blocked ground edge | Rust ignores supplied `PathGrid`; blocked-grid launch still spawns carrier. Test expects `(99,20)` for east | gamemd criterion `4` fast-accepts candidates in `FUN_004AAB30`, bypassing ordinary passability/object checks | PASS for no-abort behavior; exact cell still covered by east/south stages |
| Invalid `waypoint_edge=255` fallback edge | Rust has no secondary edge; invalid primary logs and falls directly to `Edge::North` | gamemd invalid primary calls `FUN_0050DA80`; valid `House+0x577C` is used, otherwise defaults to `0` north | NOT-IMPLEMENTED for valid-secondary fallback; north default only matches both-invalid case |
| Invalid fallback cell | Rust computes `(50,0)` for north using target-biased rectangular scan | gamemd north cell from `FUN_004AA440(MapClass, 0, sentinel, sentinel, 4, 1, 0)` not recomputed numerically in this slot | UNCHECKED |

## Findings

1. South-edge carrier spawn is not YR-parity. Rust deterministically picks the candidate closest to the target within the first ten south-edge candidates, so target `(50,20)` on `100x100` produces `(9,99)`. The verified gamemd spawner passes sentinel alternate cells, so south mode chooses from its candidate list by RNG instead of target closeness. The player-visible effect is the cargo plane entering from a predictable south-edge cell instead of the original RNG-selected one.

2. Invalid primary edge fallback is only partially modeled. Rust can default invalid `waypoint_edge=255` to north, matching gamemd only when the secondary/fallback edge is also invalid. gamemd first reads `House+0x577C` via `FUN_0050DA80`; `HouseState` has no separate fallback field, so a valid fallback edge cannot be represented.

3. East and north exact cell parity remains unchecked. Rust uses the full `sim.fog.width/height` rectangle and target-biased closest-cell selection. Verified gamemd evidence says `FUN_004AA440` operates on the playable `LocalSize` edge and the paradrop spawner passes sentinel/sentinel rather than the target. The blocked-edge no-abort behavior is fixed, but exact numerical carrier position is not yet proven equal.

## Adjacent Findings

- `src/sim/aircraft/paradrop_mission.rs` still uses `find_passable_at_edge` for opposite-edge exit resolution; that is outside this slot's carrier-spawn scenario.
- Bridge target replacement is a separate launch-target trace, not part of carrier edge selection.

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Implementation Handoff

- Split carrier spawn edge selection from target-biased edge selection. For carrier spawn, model `FUN_004AA440(MapClass, edge, sentinel, sentinel, 4, 1, 0)` semantics directly.
- Add a `HouseState` field or equivalent source for the verified secondary fallback edge if the engine wants to match invalid-primary fallback, then default to `0` only when that fallback is also invalid.
- Add focused tests for `waypoint_edge=2` south spawn with controlled RNG once the gamemd RNG input state is known, plus a LocalSize-backed east/north case.

## Status

PARTIAL: current Rust and verified docs were traced, but fresh Ghidra decompile by address was unavailable and exact east/north gamemd numeric cells were not recomputed.
