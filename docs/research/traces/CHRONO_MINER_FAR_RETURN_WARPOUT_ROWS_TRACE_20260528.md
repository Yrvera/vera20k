# Chrono Miner Far-Return WarpOut Rows Trace

**Date:** 2026-05-28  
**Scenario:** CMIN far-return teleport to a refinery staging cell.  
**Scope:** Verify current Rust visual-row emission after the generic `AnimClassSpawnDescriptor` change: exactly two `WarpOut` rows, departure and arrival, with constructor fields `type`, `coords`, `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=false`, and no extra chrono-miner ad-hoc `WorldEffect` duplicate.  
**Non-scope:** Close-return refinery radio path, dock admission, audio ordering, full `AnimClass` AI/render lifecycle, secondary `WARPOUT;WAKE2` composition, Chronosphere phases, temporal weapon visuals.

## Verdict Summary

PASS: 6 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

No FAIL or NOT-IMPLEMENTED findings in the requested row-emission surface. Coordinate equality is UNCHECKED because this batch prompt did not supply literal departure/staging cell numbers and I did not run a gamemd runtime capture for this concrete map state.

## Evidence Read

- `docs/research/ANIMCLASS_WARP_CHRONO_RUNTIME_SPAWNS_GHIDRA_REPORT.md`
- `docs/research/TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md`
- `src/sim/movement/teleport_movement.rs`
- `src/sim/world/mod.rs`
- `src/sim/miner/miner_system.rs`
- `src/sim/components.rs`

Research-index brief was also queried for the mechanic and anchors `0x73e5e0` / `UnitClass::Mission_Harvest`.

## Active-YR Gamemd Baseline

The inspected gamemd references are active in standard YR for Teleport locomotor units, including CMIN far-return self-teleport:

- `TeleportLocomotionClass__StateMachineTick / InitiateWarp` self-teleport path is active when an active Teleport locomotor has a non-null destination different from current coordinates.
- CMIN far return reaches this path through `UnitClass::Mission_Harvest` state 2 when the refinery is beyond `ChronoHarvTooFarDistance`, causing a teleport to a refinery staging/queueing cell rather than a close radio docking path.
- The harvester special case is active for CMIN and happens after the first `WarpOut` row, so it does not suppress the departure row.
- The second self-teleport `WarpOut` row is created after relocation at the arrival/current coordinates.
- The constructor rows read `[General] WarpOut` at `RulesClass+0x33C`; `WarpIn`, `WarpAway`, and `ChronoSparkle1` are not used for these TeleportLocomotion rows.

Gamemd row arguments for both departure and arrival are:

`AnimClass::Constructor(type=Rules+0x33C WarpOut, coords=current, delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`.

## Rust Path

Pipeline:

`miner_system::try_issue_chrono_far_return_teleport` -> `movement::set_destination_for_teleporter_entity(..., destination_has_building=false)` -> `teleport_movement::issue_active_teleport_head_to_coord` -> next `Simulation::advance_tick` movement phase builds `TeleportVisuals` from `rules.general.warp_out` -> `tick_teleport_movement` Relocate phase -> two `spawn_warp_out` calls -> `WorldEffect::from_anim_spawn`.

Key Rust observations:

- `src/sim/miner/miner_system.rs:951` gates CMIN far-return teleport to chrono miners beyond the too-far threshold and resolves the staging cell.
- `src/sim/miner/miner_system.rs:978` issues the destination through the teleporter bridge with `is_harvester=true`, `is_teleporter=true`, and `destination_has_building=false`.
- `src/sim/movement/movement_commands.rs:177` would choose Drive piggyback for building cells, but the far-return staging cell path reaches `issue_active_teleport_head_to_coord` at `src/sim/movement/movement_commands.rs:212`.
- `src/sim/world/mod.rs:1281` interns `rules.general.warp_out.name` and passes it into `TeleportVisuals`.
- `src/sim/movement/teleport_movement.rs:291` spawns the departure row before position mutation.
- `src/sim/movement/teleport_movement.rs:299` spawns the arrival row after entity position is set to the target cell.
- `src/sim/movement/teleport_movement.rs:64` through `src/sim/movement/teleport_movement.rs:68` writes the constructor fields onto `AnimClassSpawnDescriptor`.
- `src/sim/components.rs:864` preserves the descriptor inside `WorldEffect::from_anim_spawn`.
- `src/sim/miner/miner_system.rs:1044` now emits only chrono sound events; no miner-specific `WorldEffect` duplicate remains in this file.

## Stage Table

| Stage | Check | Our value | Gamemd value | Verdict |
|---|---|---:|---:|---|
| 1 | Far-return visual entry count for one Relocate tick | `2` rows from two `spawn_warp_out` calls | `2` rows from self-teleport departure + arrival constructors | PASS |
| 2 | Row type source | `rules.general.warp_out.name` from `src/sim/world/mod.rs:1281` | `[General] WarpOut`, `RulesClass+0x33C` | PASS |
| 3 | Constructor scalar fields | `delay=0`, `loop_count=1`, `draw_flags=0x600`, `z_adjust=0`, `reverse=false` | `delay=0`, `loop=1`, `drawFlags=0x600`, `zAdjust=0`, `reverse=0` | PASS |
| 4 | Descriptor preservation into emitted visual row | `WorldEffect.anim_spawn=Some(row)` | Global `AnimClass` stores constructor fields including draw flags and z-adjust | PASS for row metadata; full global AnimClass lifecycle is non-scope |
| 5 | Departure coordinate equality | code uses old `rx,ry,z` plus cell-center subcoords | gamemd uses current `TechnoClass` coords before relocation | UNCHECKED: no literal scenario coordinates or runtime capture |
| 6 | Arrival coordinate equality | code uses target `rx,ry` after position set plus existing `z` | gamemd uses current `TechnoClass` coords after relocation | UNCHECKED: no literal staging coordinates or runtime capture |
| 7 | Extra chrono-miner ad-hoc visual duplicate | `0` extra `WorldEffect` pushes in `miner_system.rs`; only sound events remain | `0` extra chrono-miner ad-hoc duplicate; visuals are TeleportLocomotion `AnimClass` rows | PASS |
| 8 | Active standard-YR status of gamemd baseline | active CMIN TeleportLocomotion self-teleport path per cited reports | active CMIN TeleportLocomotion self-teleport path | PASS |

## Adjacent Findings

- Audio order was not traced here. Current Rust emits chrono sound events when far-return is issued, while gamemd sound calls occur inside the teleport state-machine tick around relocation.
- Generic `WorldEffect` playback is not a complete global `AnimClass` object model. This report only verifies the descriptor row count and stored constructor fields requested by the batch scenario.
- Secondary `WarpOut=WARPOUT;WAKE2` visual composition was not traced in this run.

## Top Player-Visible Findings

No FAIL or NOT-IMPLEMENTED findings in the requested surface.

## Status

COMPLETE
