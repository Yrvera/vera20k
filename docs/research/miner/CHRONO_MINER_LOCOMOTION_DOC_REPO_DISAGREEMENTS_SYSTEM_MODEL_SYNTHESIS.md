# Chrono Miner Locomotion Docs vs Repo Disagreement Synthesis

**Date:** 2026-05-25  
**Investigation Mode:** conflict-map / repo-disagreement scan  
**Scope:** existing chrono miner locomotion research and traces versus current Rust source. Includes ore approach, full-cargo return, far-return staging, dock entry, unload/departure, ROT/facing, and teleport side effects.  
**Non-Scope:** fresh Ghidra decompilation, implementation patches, full runtime frame capture.

## Current Model

Current Rust is no longer in the old 2-cell inbound-return state. The live source parses `ChronoHarvTooFarDistance`, stores it in `MinerConfig::too_far_threshold_chrono`, compares object-coordinate 3D lepton distance, and uses strict `>` for the far-return teleport path. It also separates far/refused `QueueingCell` staging from accepted `CAN_DOCK` anchor `+(3,1)`, and normal stock unload completion avoids `Force_Track(0x47)`.

The remaining doc-vs-repo disagreements are mostly in exact mechanism surfaces: dock `0x16`, ROT source, fallback nearby-cell search, unload refinery lookup, active locomotor piggyback lifecycle, and teleport side effects.

## Claim Table

| Claim | Best evidence | Status | Safe? |
|---|---|---|---|
| Current Rust still uses a hardcoded 2-cell CMIN return threshold | Older far-return report and two-miner trace | Stale; repo now uses parsed threshold | Doc-patch-ready |
| Normal stock unload completion calls `Force_Track(0x47)` | Older ForceTrack/post-dump docs | Stale/superseded; repo matches newer zero-link model | Doc-patch-ready |
| CMIN ore approach should warp, not drive | Older stuck/final-approach traces | Disputed; newer docs and repo say drive | Needs reinvestigation before implementation |
| Dock radio `0x16` is an East-facing pivot | Current Rust comments/code | Contradicted by audited doc | Implementation delta |
| Stock CMIN effective `ROT=` is 5, not harvester-overridden 10 | `CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES` | Repo delta: parser forces 10 | Implementation delta |
| Far-return fallback uses ring limit 32, 24-candidate cap, direct/indirect modulo pick | `FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH` | Repo delta: radius 16 and simplified candidates | Implementation delta |
| Unload FSM rediscovers refinery at `(miner.x-1, miner.y)` | `DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE` | Repo delta: uses `reserved_refinery` | Implementation delta |
| Teleport relocation has extra targeting/animation/mission side effects | `CHRONO_MINER_TELEPORT` | Repo likely incomplete | Needs focused audit |
| Normal CMIN Drive/Teleport piggyback lifecycle is exact | System overview / teleport docs | Repo does not model normal-drive piggyback | Needs contract |

## Stale Or Superseded Docs

- `docs/research/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md` section 6 previously said Rust still had a 2-cell inbound threshold; it now carries a 2026-05-25 repo-status supersession. Current `miner_system.rs` and `miner/mod.rs` prove that is stale for stock values.
- `docs/research/miner/traces/TWO_CHRONO_MINERS_SAME_REFINERY_FULL_CARGO_QUEUE_TAKEOVER_TRACE.md` previously repeated the same 2-cell threshold failure; it now carries a 2026-05-25 repo-status supersession. Later stages about full end-to-end timing remain useful as unchecked.
- Older ForceTrack/post-dump docs that treat normal stock unload as `ReleaseDockedHarvester`/`Force_Track(0x47)` are superseded by zero-link state-4 reports. Current `phase_departing` matches the newer no-ForceTrack/no-explicit-exit-move model.
- `docs/research/miner/traces/MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md` is already RED in `AUDIT_LOG.md`; it has stale claims about state-0 in-transit looping and blocked-delay `SetMission(None)`.

## Current Repo Deltas Against Verified Docs

1. **Dock `0x16` semantics:** `miner_dock_sequence.rs` still defines `DOCK_FACING_EAST`, creates `dock_pivot_facing`, updates body facing, and starts unload after the local East-facing gate. The audit says first ordinary `0x16` syncs locomotor/rate state and returns; it does not set body facing East.
2. **ROT parser source:** `object_type.rs` forces `turret_rot = 10` for `Harvester=yes`. The ROT parser report says stock CMIN `ROT=5` remains the effective facing/DriveLocomotion field; the harvester write is a separate `UnitType+0x398` field.
3. **Far-return fallback search:** Rust uses `EXIT_SEARCH_MAX_RADIUS = 16` and collects candidates from the first non-empty ring without the verified 32-ring effective limit, 24-candidate cap, direct/indirect split, and frame-modulo classification.
4. **Unload refinery lookup:** `phase_unloading` and `phase_departing` use `reserved_refinery`. The binary-backed doc says stock state 3/state 4 lookup uses the miner's current cell plus `(-1,0)` and a building lookup in that adjacent cell.
5. **Piggyback lifecycle:** Rust has special-movement `begin_override/end_override`, but not the normal CMIN Drive-over-Teleport piggyback lifecycle and `Is_Ok_To_End` restoration gate.
6. **Teleport side effects:** Rust teleport movement relocates, updates occupancy, and restores override. Binary docs include additional targeting/animation/mission side effects that are not proven implemented.

## Aligned Areas

- Stock full-cargo close/far return split is aligned for positive stock values: `ChronoHarvTooFarDistance=50`, 3D lepton distance, strict far `>`.
- Close HELLO, refused/far `QueueingCell`, and accepted `CAN_DOCK` `+(3,1)` are separated in Rust.
- Normal healthy stock unload completion does not run `Force_Track(0x47)` in Rust.
- Current Rust drives to ore, which matches newer ore-acquisition/drive-model docs; older warp-to-ore traces remain disputed.

## Needs Re-Investigation

- `/re-investigate chrono miner ore approach teleport-vs-drive` if the older warp-to-ore trace is still considered plausible.
- `/re-investigate chrono miner normal Drive Teleport piggyback restore lifecycle` before implementing active-locomotor parity.
- `/trace-action chrono miner far-return fallback blocked queueing cell first rendered staging frame` if player-visible fallback cell selection needs frame proof.

## Source Ledger

- `docs/research/miner/CMIN_CLOSE_FAR_RETURN_SPLIT_CHRONOHARVTOOFARDISTANCE_GHIDRA_REPORT.md`
- `docs/research/miner/FIND_NEARBY_PASSABLE_CELL_FALLBACK_SEARCH_GHIDRA_REPORT.md`
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`
- `docs/research/miner/CMIN_RUNTIME_ROT_AFTER_PARSER_OVERRIDES_GHIDRA_REPORT.md`
- `docs/research/miner/DAT_0089F6A0_REFINERY_LOOKUP_OFFSET_SOURCE_GHIDRA_REPORT.md`
- `docs/research/miner/DRIVELOCOMOTION_BLOCKED_DELAY_TIMER_CHRONO_MINER_GHIDRA_REPORT.md`
- `docs/research/miner/DRIVE_BLOCKED_DELAY_EXPIRY_MINER_RETARGET_GHIDRA_REPORT.md`
- `docs/research/miner/CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- Current source scan: `src/sim/miner/miner_system.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/movement/teleport_movement.rs`, `src/sim/movement/locomotor.rs`, `src/rules/object_type.rs`.
