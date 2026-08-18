# Chrono Miner Stock Zero-Link Refinery Exit Trace

Date: 2026-05-21
Scenario: stock `[CMIN]` unloads at stock `[GAREFN]` placed with NW/foundation origin `(10,10)`, then completes `UnitClass::Mission_Deploy_Building` state 4 through the zero-link branch (`unit+0x2E4 == 0`).

## Result

The stock gamemd path does not use `BuildingClass::ReleaseDockedHarvester`, does not issue `Force_Track(0x47)`, and does not install a new passable-cell/NavCom destination during state 4.

VERA20k's current `phase_departing` models the conditional reciprocal-link helper as if it were the stock refinery unload exit. It starts forced turn track `0x47`, caches an explicit exit destination, and drives to the queue/exit cell `(14,11)` before returning to `SearchOre`.

That is a parity gap for the stock unload branch.

## Concrete GAREFN(10,10) Values

| Value | gamemd stock zero-link | VERA20k current |
|---|---:|---:|
| Building NW/foundation origin | `(10,10)` | `(10,10)` |
| Accepted dock/pad cell | `(13,11)` | `(13,11)` |
| State-4 refinery lookup cell | `(12,11)` from current unit cell plus `(-1,0)` | not used |
| Explicit state-4 exit destination | none installed | `(14,11)` |
| `Force_Track(0x47)` | not called | called |
| Exit sound in this branch | not found | `RefineryExitSfx` emitted |
| Next mission | Harvest / mission `0x0A` scheduling | `SearchOre` after reaching cached exit |

## gamemd Evidence

Fresh Ghidra decompile of `UnitClass__Mission_Deploy_Building @ 0x0073D630` verifies:

- top-level stock branch checks `param_1[0xB9] == 0` (`unit+0x2E4 == 0`);
- state 4 uses unit `Get_Cell_Packed()` plus `g_refinery_unload_adjacent_lookup_dx/dy`;
- state 4 checks `building->Type+0x16BB` and `building+0x57C`;
- state 4 clears `unit+0x6D1`;
- state 4 does not call `BuildingClass__ReleaseDockedHarvester`, locomotor `Force_Track`, or unit destination setter `+0x480`.

Supporting reports:

- `DAT_0089F6A0_RUNTIME_SOURCE_AND_VALUE_GHIDRA_REPORT.md`: `DAT_0089F6A0 = (-1,0)`, from the global 8-neighbor direction table, active in stock DockUnload.
- `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`: stock CMIN/HARV -> GAREFN/NAREFN unload is the zero-link branch; reciprocal `+0x2E4` release helpers are conditional.
- `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`: `Force_Track(0x47)` is not the normal stock CMIN post-unload exit.
- `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM_GHIDRA_REPORT.md`: `building+0x57C` is slot-8 `ProductionAnim`; stock GAREFN/NAREFN do not wait on it.

## VERA20k Evidence

- `src/sim/miner/miner_dock_sequence.rs`: `refinery_exit_cell` anchors on `refinery_queue_cell`, so GAREFN `(10,10)` yields `(14,11)` when passable.
- `src/sim/miner/miner_dock_sequence.rs`: `phase_departing` caches `exit_cell`, starts `REFINERY_EXIT_FORCE_TRACK = 0x47`, then issues movement to the cached exit.
- `src/sim/miner/miner_tests.rs`: `chrono_departing_starts_force_track_0x47_before_exit_move` asserts `exit_cell == Some((14,11))` and forced track `0x47`.
- `src/sim/miner/miner_tests.rs`: `chrono_departing_force_track_runs_before_normal_exit_move` asserts the miner eventually reaches `(14,11)`.

## Stage Verdicts

| Stage | Verdict | Notes |
|---|---|---|
| Stock branch reachability | PASS | gamemd zero-link branch is active for stock CMIN/HARV -> GAREFN/NAREFN. |
| Anchor identity | PASS | both sides use building NW/foundation origin for building anchor. |
| State-4 refinery rediscovery | FAIL in Rust shape | gamemd uses `(12,11)` lookup only; Rust computes movement destination `(14,11)`. |
| Slot-8 depart wait | PASS for stock | stock refineries do not create `ProductionAnim`; modded behavior remains unimplemented/unchecked. |
| Forced turn track | FAIL | gamemd stock state 4 does not call `Force_Track(0x47)`; Rust does. |
| Explicit exit destination | FAIL | gamemd stock state 4 does not set a new destination; Rust does. |
| Exit sound | FAIL/UNCHECKED | no stock state-4 sound source was found; Rust emits `RefineryExitSfx`. |

## Verification

Ran in `.`:

```text
cargo test chrono_departing -- --nocapture
```

Result: passed 2 targeted tests. This verifies current VERA20k behavior, but those tests encode the stale conditional-helper model: forced track `0x47` and explicit exit cell `(14,11)`.

## Follow-Up

Implementation should split stock zero-link refinery unload departure from reciprocal-link/interruption departure:

1. Stock zero-link `Departing`: no `Force_Track(0x47)`, no cached queue-cell destination, no `RefineryExitSfx` unless a separate stock-state sound source is verified. Clear/release the dock latch/reservation and hand directly to ore-search/Harvest-equivalent scheduling.
2. Conditional reciprocal-link release/interruption: keep the `ReleaseDockedHarvester`/`UndockUnit` style forced-track path where the binary actually reaches it.
3. Replace or rename the existing `chrono_departing_*0x47*` tests so they cover the conditional helper path, not stock GAREFN/NAREFN unload completion.
