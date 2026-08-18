# Chrono Miner Post-Dump Exit Movement Swarm Trace

**Scenario:** Allied Chrono Miner (`CMIN`) has just finished dumping cargo at a standard Allied refinery (`GAREFN`) and transitions from docked/unloading to departure.  
**Concrete geometry:** `GAREFN` at `(10,10)`, foundation `4x3`, dock pad `(13,11)`.  
**Scope:** exit anchor/cell selection, `Force_Track(0x47)` bib-step, outbound walk-vs-warp decision, and visible departure behavior.  
**Date:** 2026-05-20  

> **Correction 2026-05-21 - trace invalidated for stock DockUnload**
>
> This trace is based on the now-superseded assumption that normal stock
> post-dump exit reaches `ReleaseDockedHarvester` and `Force_Track(0x47)`.
> Current verified stock path is zero-link `Mission_Deploy_Building` state 4:
> no reciprocal `+0x2E4` link, no `ReleaseDockedHarvester`, no forced track,
> and no new NavCom destination. Keep this file only as a conditional
> reciprocal-link release movement reference.
**Ghidra:** READ-ONLY live decompile used for `BuildingClass__ReleaseDockedHarvester`, `DriveLocomotionClass__Force_Track`, `FootClass__Find_Nearby_Passable_Cell`, and `UnitClass__Mission_Harvest`.

## Executive Summary

Current Rust no longer has the older missing-Force_Track and missing-sound gaps. It seeds a forced drive-track prelude (`0x47` -> raw track 15), holds normal A* until that prelude completes, and emits `RefineryExitSfx` on Departing entry.

The remaining high-visibility mismatch is the normal exit destination. gamemd computes the post-dump destination from `GetCellLocation() + (-1,+1)`, so for `GAREFN(10,10)` the destination anchor is `(9,11)` and the first-ring result is `(9,11)`. Current Rust deliberately collapses this to the refinery queue cell `(14,11)`. That makes the miner leave only to the east queue cell instead of being assigned the west-of-foundation exit cell.

## Active YR Evidence

- Superseded 2026-05-21: the older reading that `UnitClass__Mission_Deploy_Building` calls `BuildingClass__ReleaseDockedHarvester` on the normal dump-complete exit is wrong for stock zero-link DockUnload. That call belongs to the nonzero reciprocal-link branch; standard harvester/refinery completion uses state 4.
- `BuildingClass__ReleaseDockedHarvester` live decompile confirms the normal post-unload sequence: clear anim slots `0xA/0xB`, play `RulesClass+0x244`, create slots `0xC/0xD`, call active locomotor slot `+0x58`, call `Force_Track(0x47, GetCoords + -0x80,+0x80)`, set speed multiplier `1.0`, compute `GetCellLocation()+(-1,+1)`, call `FootClass__Find_Nearby_Passable_Cell`, call unit `Set_Destination`, then `SetMission(MOVE=2)`.
- `DriveLocomotionClass__Force_Track` live decompile confirms `0x47` is a drive-track index, not a facing byte; it writes the track index, resets point index, writes `head_to`, calls `Apply_Track_Delta`, sets destination/head fields, and leaves the locomotor moving.
- `FootClass__Find_Nearby_Passable_Cell` live decompile confirms ring collection order and `g_CurrentFrameCounter % count` candidate selection when a ring has multiple candidates.

## Concrete Values

- `GAREFN` foundation: `4x3`.
- Dock pad: `(13,11)` from the current Rust pad helper and prior traces.
- gamemd exit anchor: `GetCellLocation(10,10) + (-1,+1) = (9,11)`.
- gamemd exit cell: `(9,11)` because ring 0 is outside the `4x3` foundation and passable in the concrete scenario.
- Rust exit cell: `(14,11)` via `refinery_queue_cell(10,10,4,3,None)` and `refinery_exit_cell` returning the passable queue cell.
- gamemd Force_Track: turn-track index `0x47` / decimal `71`, raw track `15`, target facing `0xC0`.
- Rust Force_Track: `REFINERY_EXIT_FORCE_TRACK = 0x47`, raw track `15`, target facing `0xC0`; normal exit move is blocked while `forced_drive_track` is present.

## Stage Verdicts

1. **Phase entry:** PASS. Rust enters `RefineryDockPhase::Departing` after one unload interval, matching the active `Mission_Deploy_Building -> ReleaseDockedHarvester` dump-complete handoff closely enough for this stage; exact frame jitter was not re-measured here but both sides were verified to enter the same normal post-dump handler.
2. **Exit anchor/cell:** FAIL. gamemd computes `(9,11)`; Rust computes `(14,11)`. This is literal numeric inequality at the movement target consumed by the next movement stage.
3. **Force_Track bib-step identity:** PASS. gamemd calls `Force_Track(0x47, ...)`; Rust starts `begin_forced_turn_track(0x47, ...)` and resolves to raw track `15`, target facing `0xC0`.
4. **Force_Track timing gate:** PASS. gamemd performs `Force_Track` before `Set_Destination`; Rust returns from `phase_departing` while `forced_drive_track` is active, so normal exit movement is not issued until the forced track clears.
5. **Force_Track head-to coordinate:** UNCHECKED. gamemd writes head-to at `building.GetCoords()+(-128,+128)`. Rust uses local forced-track offsets `(0,256)`, which make track point 0 start at subcell center for the miner's current pad cell, but this trace did not compute absolute lepton equality against gamemd's `GetCoords` output.
6. **Outbound walk-vs-warp:** PASS. gamemd's `Set_Destination` is followed by `SetMission(MOVE=2)` and the active drive locomotor remains in use after `Force_Track`; Rust checks `teleport_state` and uses `movement::issue_move_command`/direct drive after forced track, with no outbound `issue_teleport_command` in Departing.
7. **Departure sound:** PASS. gamemd plays `RulesClass+0x244` (`BunkerWallsDownSound`, retail `TankBunkerDown`) at building location; Rust emits `SimSoundEvent::RefineryExitSfx { rx, ry }` on first Departing entry and the app resolves it from `[AudioVisual] BunkerWallsDownSound`.
8. **SpecialAnimThree/Four departure anims:** NOT-IMPLEMENTED. gamemd creates building anim slots `0xC` and `0xD` from `SpecialAnimThree/Four` before locomotion. Current Rust only has the bale `SpecialAnim`/slot-10 path visible in `app_building_anim.rs` and no Departing hook for slots `0xC/0xD`.
9. **Dock/contact clear timing:** PASS for this scenario's visible movement handoff. Rust now releases pad/contact reservations on Departing first entry before movement, matching the gamemd cleanup timing more closely than the older trace; deeper multi-miner contact behavior is adjacent and not traced here.

## Player-Visible Findings

1. **SUPERSEDED 2026-05-21 - Stage 2 exit destination:** This finding depended on `ReleaseDockedHarvester` as the normal stock exit. Current evidence says stock zero-link DockUnload does not use that helper or the `(9,11)` forced-release destination; keep this row only as conditional reciprocal-link release context.
2. **NOT-IMPLEMENTED - Stage 8, departure slot 0xC/0xD anims:** gamemd creates `SpecialAnimThree/Four` on departure; Rust has no Departing-side slot `0xC/0xD` building anim trigger. Player-visible difference: missing refinery bay/door departure animation if those art slots are populated. Rust search: `src/app_building_anim.rs:339`, `src/rules/art_data.rs:878`, no Departing hook. gamemd: live `BuildingClass__ReleaseDockedHarvester` anim slot creation before locomotion.

## Adjacent Findings

- The current Rust comments state the west cell is immediately overwritten by the next harvest retarget and therefore not visible. This trace did not prove that overwrite timing numerically. The verified mismatch remains at the `Set_Destination` boundary for this exact exit mechanic.
- The path around or through the south bib after the west destination is assigned depends on the drive/path state after `Force_Track` and `Set_Destination`. This trace verified the destination value but did not compute the full gamemd A* step list.

## Verdict Tally

PASS: 5 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
