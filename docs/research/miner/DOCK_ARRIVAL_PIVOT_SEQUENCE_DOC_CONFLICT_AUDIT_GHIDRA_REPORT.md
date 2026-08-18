# Dock Arrival Pivot Sequence Doc-Conflict Audit - Ghidra Research Report

**Date:** 2026-05-24  
**Address(es):** `0x00739EC0`, `0x004D9290`, `0x00737430`, `0x0073D630`, `0x004595C0`, `0x0043C2D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** audit only the stale/conflicting claims in `miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` against the current stock refinery dock model for stock `CMIN/HARV -> GAREFN/NAREFN`.  
**Non-Scope:** unrelated pivot visuals, slave miners, service/repair/aircraft docks, non-stock multi-dock mods, exact first-frame runtime winner among possible `0x15` senders.  
**Confidence:** High for the audited conflict claims.  
**Active in YR:** Yes for the stock `CMIN/HARV -> GAREFN/NAREFN` path unless marked conditional.

## Working Notes Gate

Target question: Which claims in `DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md` conflict with the current verified stock refinery dock model around `0x00739EC0`, mission enter, `0x16`, `+0x2E4`, `ReleaseDockedHarvester`/`Force_Track(0x47)`, and normal stock exit?  
Non-goals: Do not audit unrelated pivot visuals, do not re-open accepted-cell/GetDockCoord/QueueingCell beyond direct contradictions, do not implement Rust, do not mutate Ghidra.  
Evidence needed to mark COMPLETE: each scoped conflict classified against live Ghidra decompile plus function identity/xref or assembly range, stock INI/default gate evidence where needed, and current Rust/doc handoff.  
Stop conditions: all scoped open questions resolved/deferred, no Ghidra mutations, write only this report plus the shared claims file, and leave stale-doc patch wording for later.

## 1. Overview

The 2026-05-19 dock-arrival report is now a mixed-validity document. Its `0x16` negative claim ("not facing, no stop") remains directionally correct, but its top-level model still conflates `0x00739EC0` with `Mission_Enter`, over-states `GetDockCoord`/pad arrival as the ordinary entry source, and treats `ReleaseDockedHarvester`/`Force_Track(0x47)` as normal stock post-unload exit. Current Ghidra and the canonical synthesis show the stock path is: `FootClass::Mission_Enter @ 0x004D9290` drives `0x0E`; `BuildingClass::Receive_Radio @ 0x0043C2D0` sends accepted `0x12` to NW+(3,1), later `0x18` and `0x16`; `0x16` may only set rate/timer to `0x4000` on its first ordinary pass; `0x15` queues `Mission_Deploy_Building`; stock unload exits through zero-link state 4, not `ReleaseDockedHarvester`.

## 2. Audited Claim Verdicts

| Audited claim | Verdict | Active in YR | Evidence |
|---|---|---|---|
| `0x00739EC0` is `UnitClass::Mission_Enter` / mission-7 dispatch handler | DRIFT | Yes, but as a per-cell hook, not mission dispatch | `get_function_by_address(0x00739EC0)` = `UnitClass__PerCellProcess`, body `0x00739EC0..0x0073B0AE`; vtable data xref `0x007F5DFC`. `FootClass__Mission_Enter @ 0x004D9290`, body `0x004D9290..0x004D949B`, mission-table xrefs `0x007E8ED4`, `0x007EB298`, `0x007F5EB0`. |
| `UnitClass::Mission_Enter (= UnitClass__PerCellProcess)` detects normal approach-to-pad | DRIFT/MISLEADING | Conditional | `0x00739EC0` can send `0x15` from cell-entry branches, but stock mission dispatch is `0x004D9290`. Current model leaves first `0x15` source runtime-sensitive. |
| `0x16` calls `GetDockCoord`, `Set_Destination`, or writes location | CONFIRMED NEGATIVE | Yes | `UnitClass__Receive_Radio @ 0x00737430` case `0x16`, assembly `0x007376AD..0x0073778C`: calls `FootClass__Receive_Radio`, reads `+0x6AF`, calls `RateTimer__Current` at `+0x388`, conditionally calls locomotor vtable `+0x4C(0x4000)`, else may send `0x15`. No `GetDockCoord`, no `Set_Destination`, no coordinate/location write. |
| First ordinary `0x16` can only sync `+0x388`/locomotor `+0x4C(0x4000)` and return | CONFIRMED | Yes | Decompile and assembly `0x007376BF..0x00737718`: if `+0x6AF == 0` and `RateTimer::Current(+0x388) != 0x4000`, call `loco+0x4C(0x4000)` and return `1`. |
| Later/already-synced `0x16` can send `0x15` | CONFIRMED | Yes, conditional | Assembly `0x0073771B..0x0073778C`: if locomotor not moving, destination exists, contact-entered byte `+0x418` is set, destination `WhatAmI()==6`, and unit mission is `7`, call vtable `+0x278(0x15,destination)`, then return `1`. |
| `0x16` sets facing East / `0x4000` is a facing | DRIFT | Yes path, wrong meaning | `0x4000` is passed to locomotor vtable `+0x4C`; no body-facing setter appears in the case. Current Rust comments/tests still model `Do_Turn(0x4000)`/East-facing pivot and should be corrected separately. |
| `+0x2E4` is set by normal stock refinery docking before unload | DRIFT | No for normal stock path | `BuildingClass__Receive_Radio @ 0x0043C2D0` case `0x15` queues sender mission `0x10` for `DockUnload` and has no `+0x2E4` write. `UnitClass__PerCellProcess @ 0x00739EC0` arrival branch writes conditional `+0x5A4`, sends `0x15`, powers off locomotor, and does not write reciprocal `+0x2E4`. |
| `ReleaseDockedHarvester`/`Force_Track(0x47)` is normal stock CMIN/HARV unload completion | DRIFT | Conditional, not normal stock | `UnitClass__Mission_Deploy_Building @ 0x0073D630` calls `ReleaseDockedHarvester` only from the nonzero `param_1[0xB9]` branch at `0x0073D64F..0x0073D672`. Stock zero-link state 4 at `0x0073E0F0..0x0073E28F` clears `+0x6D1`, queues mission `10`/Harvest, optionally sends `BREAK(3)`, and does not call `ReleaseDockedHarvester`, `Force_Track`, or `Set_Destination`. |
| `ReleaseDockedHarvester` itself performs `Force_Track(0x47)` when the reciprocal link exists | CONFIRMED BUT CONDITIONAL | Conditional | `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, xref from `0x0073D66D`; null `building+0x2E4` branch clears `+0x718`, sets building mission `5`, returns. Non-null unit branch clears `unit+0x2E4`, powers on, calls locomotor vtable `+0x70` with track `0x47`, sets destination/mission, clears `building+0x2E4`, sends `BREAK(3)`. |
| Stock `CMIN/HARV -> GAREFN/NAREFN` active gates | CONFIRMED | Yes | `rulesmd.ini`: `[CMIN] Dock=NAREFN,GAREFN`, `[HARV] Dock=NAREFN,GAREFN`, `[GAREFN]/[NAREFN] DockUnload=yes`, `Refinery=yes`; `artmd.ini`: `[GAREFN]/[NAREFN] QueueingCell=4,1` staging only. |

## 3. Core Logic Corrections

### `0x00739EC0` identity

Active in YR: Yes, as `UnitClass__PerCellProcess`. It is not the mission-7 dispatch function. Its audited arrival branch may compare current cell against a `GetDockCoord`-derived cell and may send `0x15`, but that is a cell-entry hook. The mission dispatcher uses `FootClass__Mission_Enter @ 0x004D9290`, which sends `0x0E` to the current destination and returns the mission timer delay through `MissionClass__GetMissionTimerEntry`, `Math__ftol`, and `Random__RandomRanged(0,2)`.

### `0x16` semantics

Active in YR: Yes. First unsynced ordinary `0x16` is a rate/timer sync, not a facing or movement operation. The handler contains no `GetDockCoord`, no `Set_Destination`, and no location write. Only when already synchronized and stopped with a live building destination, `+0x418` contact-entered byte set, and mission `7`, it can send `0x15` to the destination building.

### `+0x2E4` and stock exit

Active in YR: Conditional. `+0x2E4` is a real reciprocal-link field for linked/interrupt paths, but normal stock `CMIN/HARV -> GAREFN/NAREFN` unload does not use it. The normal stock branch enters `Mission_Deploy_Building` with `unit+0x2E4 == 0`, drains cargo in state 3, and exits through state 4 with `+0x6D1` clear and Harvest scheduling. `ReleaseDockedHarvester` is a conditional reciprocal-link helper.

## 4. INI Keys

| Key | Stock value | Active in YR | Effect in this audit |
|---|---|---|---|
| `[CMIN] Dock=` | `NAREFN,GAREFN` | Yes | Makes CMIN a stock candidate for these refinery receivers. |
| `[HARV] Dock=` | `NAREFN,GAREFN` | Yes | Makes HARV a stock candidate for these refinery receivers. |
| `[GAREFN]/[NAREFN] DockUnload=` | `yes` | Yes | `BuildingClass::Receive_Radio` `0x15` queues sender mission `0x10`. |
| `[GAREFN]/[NAREFN] Refinery=` | `yes` | Yes | State-4 stock unload verifies refinery type in adjacent lookup before some guards. |
| `[GAREFN]/[NAREFN] QueueingCell=` | `4,1` | Yes, but separate | Staging/fallback; not accepted `0x12` target and not `GetDockCoord`. |
| `[Enter] Rate=` | `.016` | Yes | `FootClass::Mission_Enter` retry delay is `ftol(.016*900)+RandomRanged(0,2)=14..16`. |

## 5. Integration Points

| Function | Role | Active in YR | Evidence |
|---|---|---|---|
| `FootClass__Mission_Enter @ 0x004D9290` | Mission-7 dispatch; sends `0x0E`, handles mission timer return | Yes | Decompile plus assembly range `0x004D9290..0x004D949B`, mission vtable xrefs. |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | Building-side dock admission/handoff | Yes | Case `0x0E` accepted target hardcodes NW+(3,1); case `0x15` queues mission `0x10`. |
| `UnitClass__Receive_Radio @ 0x00737430` | Unit radio switch including `0x16` | Yes | Case `0x16` assembly `0x007376AD..0x0073778C`. |
| `UnitClass__PerCellProcess @ 0x00739EC0` | Unit cell-entry hook, including separate `0x15` branches | Yes, when unit cell-entry processing runs | Function identity and body `0x00739EC0..0x0073B0AE`. |
| `UnitClass__Mission_Deploy_Building @ 0x0073D630` | Harvester unload FSM and state-4 stock exit | Yes | Decompile plus assembly ranges `0x0073D630..0x0073D690`, `0x0073DEE0..0x0073E28F`. |
| `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` | Conditional reciprocal-link release helper | Conditional | Direct xref from `0x0073D66D`; nonzero `building+0x2E4` branch only. |

## 6. Current Rust Implementation Status

Rust already reflects several current stock-model corrections: accepted `0x12` target helper `refinery_can_dock_queue_cell` is NW+(3,1), QueueingCell is separate, state-4 `Departing` avoids stock `Force_Track(0x47)`, and tests cover no stock Force_Track. Evidence: `src/sim/miner/miner_dock_sequence.rs` and `src/sim/miner/miner_tests.rs`.

Current Rust still contains a drift against the audited `0x16` semantics: `RefineryDockPhase::Linked/Pivoting`, `dock_pivot_facing`, and tests model an East-facing pivot from `0x4000`. That should be treated as a stale implementation/doc contract, because binary `0x16` writes locomotor/rate state and can later send `0x15`; it does not set a body-facing target.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00739EC0` label conflict | verified | function identity + decompile | none for identity |
| Mission-7 dispatch owner | verified | `0x004D9290` function identity, decompile, xrefs | none for owner |
| Unit radio `0x16` first-call semantics | verified | decompile + assembly `0x007376AD..0x00737718` | exact frame count to timer convergence is runtime-sensitive |
| Unit radio `0x16 -> 0x15` later-call semantics | verified | decompile + assembly `0x0073771B..0x0073778C` | exact first `0x15` source in every replay frame remains runtime-sensitive |
| `+0x2E4` normal stock writer claim | verified negative | `0x0043C2D0`, `0x00739EC0`, existing writer inventory docs | full global writer inventory not re-opened |
| Stock zero-link unload state 4 | verified | `0x0073D630` decompile; state-4 range | exact visual stale-frame timing not in scope |
| `ReleaseDockedHarvester` behavior | verified conditional | `0x004595C0` decompile and xref `0x0073D66D` | non-refinery/bunker producer details out of scope |
| Current Rust `0x16` pivot surface | touched-not-exhausted | rg/read of `miner_dock_sequence.rs`, `miner_tests.rs` | implementation change belongs to a separate patch/contract |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Is 0x00739EC0 currently labeled/function-owned as Mission_Enter or PerCellProcess?` -> PerCellProcess, not Mission_Enter. (evidence: `get_function_by_address(0x00739EC0)`)
- `[RESOLVED] OQ2 - What is the actual mission-7 dispatch handler?` -> `FootClass__Mission_Enter @ 0x004D9290`. (evidence: function identity, vtable xrefs)
- `[RESOLVED] OQ3 - Does `0x16` call GetDockCoord, Set_Destination, or write location?` -> No. (evidence: `UnitClass__Receive_Radio` case `0x16`, assembly `0x007376AD..0x0073778C`)
- `[RESOLVED] OQ4 - What can first ordinary `0x16` do?` -> If `+0x6AF==0` and timer != `0x4000`, call locomotor `+0x4C(0x4000)` and return `1`. (evidence: `0x007376BF..0x00737718`)
- `[RESOLVED] OQ5 - What can later/already-synced `0x16` do?` -> If stopped with building destination, `+0x418`, and mission 7, send `0x15`. (evidence: `0x0073771B..0x0073778C`)
- `[RESOLVED] OQ6 - Does normal building `0x15` write `+0x2E4`?` -> No; DockUnload queues sender mission `0x10`. (evidence: `BuildingClass__Receive_Radio @ 0x0043C2D0`)
- `[RESOLVED] OQ7 - Is `ReleaseDockedHarvester` normal stock exit?` -> No; it is only called from nonzero `unit+0x2E4` branch. (evidence: `UnitClass__Mission_Deploy_Building @ 0x0073D630`, call at `0x0073D66D`)
- `[RESOLVED] OQ8 - What does stock zero-link state 4 do instead?` -> Clears unload-active `+0x6D1`, queues/continues Harvest, optionally sends `BREAK(3)` if a valid contact/path condition remains. (evidence: `0x0073E0F0..0x0073E28F`)
- `[RESOLVED] OQ9 - Are stock CMIN/HARV and GAREFN/NAREFN gates active?` -> Yes. (evidence: `rulesmd.ini` Dock/DockUnload/Refinery entries)
- `[DEFERRED] OQ10 - Which exact `0x15` source wins first in every concrete replay frame?` (category: needs-runtime-debugger; reason: static code leaves multiple source-aware gates possible; next-step-if-pursued: runtime trace around first `0x18/0x16` and cell-entry callbacks)
- `[DEFERRED] OQ11 - Exact visible body facing during dump for each map path?` (category: needs-runtime-debugger; reason: depends on path-driven arrival facing and render sampling; next-step-if-pursued: capture facing byte across accepted-cell arrival and first unload frames)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x16` does not set East facing; first ordinary call syncs timer/rate and returns | `0x00737430`, assembly `0x007376AD..0x00737718` | mismatch in comments/state/tests; likely behavior mismatch in `Pivoting` | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_tests.rs` | Replace East-facing pivot contract with source-aware `0x16` sync/wait semantics; do not make `0x4000` a body-facing target | `first_0x16_syncs_without_starting_unload_or_turning_east` | Do not preserve `DOCK_FACING_EAST` as binary proof |
| Later/already-synced `0x16` may send `0x15` without `GetDockCoord` equality | `0x0073771B..0x0073778C` | partially represented by current phases but source is not explicit | Miner dock radio/contact FSM | Split first `0x16` return from later `0x16 -> 0x15`; keep `+0x418`-like contact-entered gate | `already_synced_0x16_can_start_unload_from_accepted_cell_destination` | Do not require `GetDockCoord` equality before every `0x15` |
| Stock exit is zero-link `Mission_Deploy_Building` state 4, no normal `ReleaseDockedHarvester`/`Force_Track(0x47)` | `0x0073D630`, state-4 range, `0x004595C0` conditional branch | current Rust appears aligned for stock `Departing`; keep regression coverage | `phase_departing`, interrupt path, dock reservations | Preserve no ForceTrack/no explicit exit move/no departure SFX for healthy stock completion; keep ForceTrack only for linked interrupt | `stock_departing_does_not_start_force_track_0x47` plus no SFX test | Do not reintroduce reciprocal `+0x2E4` as stock slot |

## Negative Facts / Do Not Do

- Do not cite `0x00739EC0` as the mission-7 dispatch handler; cite `FootClass__Mission_Enter @ 0x004D9290`.
- Do not implement `0x16` as `GetDockCoord`, `Set_Destination`, a location write, a stop, or an East-facing turn.
- Do not treat `0x16` return `1` as proof that `0x15` was sent.
- Do not model normal stock `CMIN/HARV -> GAREFN/NAREFN` completion as `ReleaseDockedHarvester` or `Force_Track(0x47)`.
- Do not use reciprocal `unit/building +0x2E4` as the normal stock DockUnload slot.

## Stale Docs / Follow-up Docs

Exact replacement wording for `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`:

> This report is superseded for stock `CMIN/HARV -> GAREFN/NAREFN` dock admission and unload completion by `STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md` and `DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`. `0x00739EC0` is `UnitClass__PerCellProcess` / a cell-entry hook, not the mission-7 dispatch handler; mission 7 dispatch is `FootClass__Mission_Enter @ 0x004D9290`. Unit radio `0x16` has no `GetDockCoord`, no `Set_Destination`, and no location write; first ordinary `0x16` may only sync `+0x388`/locomotor `+0x4C(0x4000)` and return, while a later/already-synced call may send `0x15` under stopped-building-destination/contact-entered/mission-7 gates. Normal stock unload completion is zero-link `Mission_Deploy_Building` state 4 and does not call `ReleaseDockedHarvester` or `Force_Track(0x47)`; those helpers remain conditional reciprocal-link/interrupt paths only.

## Sources

- Ghidra read-only: `get_function_by_address`, `decompile_function`, `get_xrefs_to`, `get_function_callers`, `disassemble_bytes`.
- `UnitClass__PerCellProcess @ 0x00739EC0`, body `0x00739EC0..0x0073B0AE`; disassembly range generated `0x007397B0..0x0073A51F`.
- `FootClass__Mission_Enter @ 0x004D9290`, body/disassembly `0x004D9290..0x004D949B`.
- `UnitClass__Receive_Radio @ 0x00737430`, case `0x16` assembly `0x007376AD..0x0073778C`.
- `UnitClass__Mission_Deploy_Building @ 0x0073D630`, assembly ranges `0x0073D630..0x0073D690`, `0x0073DEE0..0x0073E28F`.
- `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, body `0x004595C0..0x00459839`.
- `BuildingClass__Receive_Radio @ 0x0043C2D0`, body `0x0043C2D0..0x0043CE5E`.
- `docs/research/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
- `docs/research/miner/DOCK_ARRIVAL_PIVOT_SEQUENCE_GHIDRA_REPORT.md`.
- `ini/rulesmd.ini`, `ini/artmd.ini`.

**Status:** COMPLETE
