# DriveTrack Chain Lookahead Blocker Trace - 2026-05-27

**Scenario:** A standard DriveLocomotion vehicle reaches a DriveTrack chain/lookahead point. The next-next cell contains either a friendly moving blocker or bridge-sensitive occupancy that `UnitClass::Can_Enter_Cell` can classify as code `2`, `3`, or `6`.

**Scope:** One mechanic only: `Process_Drive_Track` chain/lookahead blocker return-code consumption versus current Rust DriveTrack chain handling.

**Active in standard YR:** Yes. `DriveLocomotionClass::Process @ 0x004B0500` reaches `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` for normal Drive units, and `Process_Drive_Track` calls owner vtable `+0x1AC`, which is `UnitClass::Can_Enter_Cell @ 0x0073F0A0` for standard vehicle units.

## Evidence Used

- Ghidra read-only decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- Ghidra read-only decompile: `UnitClass::Can_Enter_Cell @ 0x0073F0A0`
- `docs/research/DRIVE_PROCESS_DRIVE_TRACK_SPEED_BUDGET_RESIDUAL_GHIDRA_REPORT.md`
- `docs/research/DRIVE_PROCESS_MOVEMENT_TICK_ORDER_GHIDRA_REPORT.md`
- `docs/research/UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`
- Rust source scan: `src/sim/movement/movement_tick.rs`, `src/sim/movement/movement_step.rs`, `src/sim/movement/movement_occupancy.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/movement/bump_crush.rs`

## Pipeline

Move order/path already exists -> active DriveTrack reaches chain point -> Rust/gamemd look ahead to the next-next path cell -> `Can_Enter_Cell` classifies blocker -> chain track is installed, delayed, or side effects fire -> player sees whether the vehicle starts the next turn, waits, crushes/checks obstacle, or scatters a blocker.

## Stage Findings

### Stage 1 - Active YR Entry

`Process_Drive_Track @ 0x004B0F20` is active YR Drive locomotion code. It is called from the active Drive `Process` path and is used by stock Drive-locomotor units. Verdict: UNCHECKED for Rust equality because this stage is binary reachability only, not a computed Rust-vs-gamemd output.

### Stage 2 - gamemd Chain Lookahead Call

At the chain/lookahead branch, gamemd computes a cell one direction beyond the current head-to coordinate, gets its effective height, maps the packed cell to `CellClass`, and calls owner vtable `+0x1AC`:

```text
owner.Can_Enter_Cell(candidate_cell, direction, effective_height, arg5)
```

For standard vehicles this resolves to `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, whose active return-code taxonomy includes `2`, `3`, and `6`. Verdict: UNCHECKED for full numerical equality because the exact concrete candidate coordinate and track index were not computed for a named map cell.

### Stage 3 - Code 2 Friendly Moving Blocker

gamemd consumes code `2` in the same switch arm as code `0`. It installs the chained DriveTrack state: clears the alternate-track byte, writes the new track index, writes the point index to the selected track end minus one, clears/rebuilds head-to state, calls owner side-effect methods, and can run `Apply_Track_Delta` after crate pickup handling.

Current Rust also treats `CellEntryResult::TemporaryBlock` as chain-acceptable, but it only proceeds to `select_drive_track` and assigns `entity.drive_track = Some(new_track)` in `handle_deferred_drive_track_chain`. It does not reproduce the full gamemd state-write packet for code `2`.

Verdict: FAIL. The high-level "allow chain" decision matches for code `2`, but exact state equality is not proven and known gamemd side effects/state writes are missing.

### Stage 4 - Code 3 Obstacle/Crushable Check

gamemd code `3` does not enter the code `0/2` chain-install block. It calls `MapClass::Check_Crushable_Obstacle(owner, candidate_cell)` and then leaves the switch without installing the new chained track in that branch.

Current Rust maps `CellEntryResult::ScatterRequired` into the same accepted group as `Clear` and `TemporaryBlock`, then installs a new DriveTrack. This collapses gamemd's distinct code `3` side-effect path into ordinary chaining.

Verdict: FAIL.

### Stage 5 - Code 6 Friendly Stationary / Bridge-Sensitive Occupancy

gamemd code `6` computes a bridge-sensitive scatter flag from the candidate cell's bridge bit and height relation, then calls `CellClass::Scatter_Objects(NullCoord, 1, force/bridge_sensitive_flags)`. It does not use the code `0/2` chain-install body for that return code.

Current Rust calls `bump_crush::scatter_blocker` for `FriendlyStationary`, then continues after the match and still installs the new DriveTrack. The scatter helper also lacks the gamemd bridge-sensitive flag/height argument and instead searches adjacent cells with `PathGrid` walkability plus layer occupancy.

Verdict: FAIL.

### Stage 6 - Bridge Layer Context

Current Rust does perform a bridge traversal/layer precheck before deferred chain classification through `evaluate_runtime_can_enter_cell`, and it passes a layer context into occupied-cell classification. This is directionally closer than pure layer/walkable/empty behavior.

However, the chain result is still reduced after classification to broad buckets: accepted (`Clear`, `TemporaryBlock`, `ScatterRequired`, `FriendlyStationary` after scatter/crush handling) versus rejected (`FriendlyWall`, `OccupiedEnemy`, `Impassable`). The exact gamemd code-specific switch shape is not preserved for codes `3` and `6`.

Verdict: FAIL.

## Verdict Tally

PASS: 0 | FAIL: 4 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Top Player-Visible Failures

1. **Stage 4:** Code `3` lookahead can start a chained turn in Rust, while gamemd runs `Check_Crushable_Obstacle` and does not install the chain in that switch arm. Rust: `src/sim/movement/movement_tick.rs:658`, `src/sim/movement/movement_tick.rs:692`; gamemd: `Process_Drive_Track @ 0x004B0F20` switch case `3`.

2. **Stage 5:** Code `6` friendly stationary/bridge-sensitive blocker can scatter and still install a chained track in Rust; gamemd only scatters from that return-code arm. Rust: `src/sim/movement/movement_tick.rs:670`, `src/sim/movement/movement_tick.rs:692`; gamemd: `Process_Drive_Track @ 0x004B0F20` switch case `6`.

3. **Stage 5:** Bridge-sensitive scatter arguments are missing in Rust's blocker scatter helper, so a bridge-adjacent blocker can be scattered using different legality/direction inputs. Rust: `src/sim/movement/bump_crush.rs:612`; gamemd: code `6` branch in `Process_Drive_Track @ 0x004B0F20` computes bridge flag before `CellClass::Scatter_Objects`.

4. **Stage 3:** Code `2` is accepted by both engines, but Rust does not write the full gamemd chain state packet, including Drive track fields/head-to side effects. Rust: `src/sim/movement/movement_tick.rs:692`; gamemd: code `0/2` arm in `Process_Drive_Track @ 0x004B0F20`.

## Adjacent Findings

- `cell_entry.rs` still uses an approximate first-primary-blocker classification path in `find_primary_blocker`; full object-list order parity belongs to a separate `Can_Enter_Cell` trace.
- Exact raw DriveTrack index equality for a named map coordinate was not computed in this trace, so the accepted code `2` path remains worse than PASS even where the broad branch agrees.

## Status

COMPLETE for the requested chain/lookahead return-code trace. No Rust, INI, or published docs were modified.
