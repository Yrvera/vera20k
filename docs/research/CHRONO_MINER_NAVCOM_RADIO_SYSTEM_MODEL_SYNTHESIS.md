# Chrono Miner NavCom And Radio System Model Synthesis

Date: 2026-05-22  
System: stock YR `CMIN/HARV -> GAREFN/NAREFN` refinery return, NavCom assignment, radio admission, dock arrival, and unload-entry handoff.

Update: 2026-05-22 follow-ups completed. This synthesis now points to
`STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`,
`CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`, and
`miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`
as canonical for stock unload reachability, contact saturation, and close-return
timing deltas.

## Scope

Included surfaces:

- `Mission_Harvest` close-return contact timing.
- `Mission_Enter` / building `CAN_DOCK(0x0E)` admission.
- `FootClass::Receive_Radio(0x12)` NavCom assignment and retry reset.
- `TechnoClass::Receive_Radio(0x18/0x19)` dock/contact byte.
- `UnitClass::PerCellProcess` dock-pad arrival and radio `0x15`.
- `Mission_Deploy_Building` stock unload exit relation to `Force_Track(0x47)`.

Explicit non-scope:

- Full queue eviction/contact saturation behavior.
- Runtime frame capture for exact rendered first frame after arrival or exit.
- Non-refinery enter systems, Bunker occupant flow except where it distinguishes `+0x2E4`.
- Rust patch planning or implementation.

Output type: `model-synthesis`. There are stale and RED/YELLOW source claims, but the primary stock path has newer binary-backed corrections.

## Evidence-Ranked Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Stock CMIN is a harvester with `Dock=NAREFN,GAREFN`, `Teleporter=yes`, teleport locomotor | `rulesmd.ini:7351,7361,7364,7396,7398` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| GAREFN/NAREFN are stock DockUnload refineries | `rulesmd.ini:11722,11726-11729`, `12515,12519-12521`; `artmd.ini:1706,1709`, `1763,1766` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Close return state 2 sends only radio `0x02` HELLO, then writes harvest substate 3 on `ROGER(1)` | `MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING...`; Ghidra `0x0073E5E0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| State 3 queues mission `7`; it does not itself send `0x0E` | same report, `0x0073EE8D..0x0073EE93` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `Mission_Enter` sends building-directed `0x0E CAN_DOCK` | `0x004D9290`; close-return report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Accepted stock refinery `0x0E` computes payload cell as building NW `+(3,1)` | Ghidra spot-check `0x0043C2D0`; `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Accepted `0x0E` does not use art `QueueingCell=4,1` | same plus `artmd.ini:1716,1773` | confirmed | high | conditional data exists | IMPLEMENTATION_SAFE |
| `0x12` sets Foot/NavCom `+0x5A4` via `FootClass::Set_Destination_Internal`, then zeroes dispatch duration `+0xD0` | `FOOTCLASS_RECEIVE_RADIO_0X12_MOVE_FIELDS_NAVCOM...`; Ghidra spot-check `0x004D94B0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `+0xCC` in case `0x12` is target coord Y local, not a chrono-miner timing field | `FOOTCLASS_RADIO_MOVE_FIELDS...`; `FOOTCLASS_RECEIVE_RADIO_0X12...` | confirmed | high | yes | DOC_PATCH_READY |
| Building sends `0x18` then `0x16` only after `0x12` returns `0x14 ALREADY_THERE` | Ghidra spot-check `0x0043C2D0`; close-return report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x18/0x19` toggle byte `+0x418`; they do not write `+0x2E4` | Ghidra spot-check `0x006F4AB0`; `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM...` | confirmed | high | yes/conditional for `0x19` | IMPLEMENTATION_SAFE |
| Radio `0x10` is not part of the standard CMIN refinery path; `0x10` here is mission id queued by `0x15` | `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM...`; `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL...`; Ghidra spot-checks `0x006F4AB0`, `0x00739EC0`, `0x0043C2D0` | confirmed | high | no standard sender | IMPLEMENTATION_SAFE for stock path |
| Pad arrival order is `FootClass::PerCellProcess(2)` -> radio `0x15` -> locomotor slot `+0x5C` | Ghidra spot-check `0x00739EC0`; arrival reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Building receiver case `0x15` queues sender mission `0x10` for `DockUnload=yes` | Ghidra spot-check `0x0043C2D0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock refinery arrival does not create reciprocal unit/building `+0x2E4` links | `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING...`; Ghidra spot-check `0x00739EC0` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Reciprocal `+0x2E4` writer is Bunker-gated `FUN_00458E50`, not stock refinery DockUnload | same report; `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY...` | confirmed | high | conditional, not GAREFN/NAREFN | IMPLEMENTATION_SAFE |
| `ReleaseDockedHarvester` `Force_Track(0x47)` is conditional on nonzero reciprocal link; not normal stock zero-link unload exit | Ghidra spot-check `0x0073D630`; `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP...` | confirmed | high | stock path no; conditional path yes | IMPLEMENTATION_SAFE |
| Exact frame separation from state 3 queue to first `Mission_Enter` dispatch | close-return report | unknown | medium | yes | NEEDS_REINVESTIGATE |
| Full queue/contact saturation behavior for multi-miner refinery contention | `BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE...`; traces | partial | medium | yes | NEEDS_REINVESTIGATE |
| Stock state-4 unload reachability and `PathType::Has_Valid_Steps` polarity | `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Receiver-side full HELLO behavior and sender-side HELLO eviction target | `CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION...` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Current Rust close-return timing/threshold mismatches | `CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE...` | confirmed/partial | medium | yes | NEEDS_IMPLEMENTATION_PLAN |

## Current Model

The stock chrono miner close-return path starts in `Mission_Harvest` state 2. If the miner is close enough to its selected refinery (`ChronoHarvTooFarDistance=50` cells for CMIN), it sends radio `0x02` HELLO to establish radio contact. On `ROGER(1)`, it advances to harvest substate 3, which queues mission `7` (`Mission_Enter`).

`Mission_Enter` is the first standard point that sends `0x0E CAN_DOCK` to the refinery. For stock DockUnload refineries, `BuildingClass::Receive_Radio(0x0E)` accepts by computing an inline move target at building NW cell `+(3,1)`, sends `0x12 MOVE_TO_CELL` with that `CellClass*`, and only if the unit is already at that cell (`0x12` returns `0x14`) sends `0x18 ENTER_DOCK` followed by `0x16` timing/facing sync.

If `0x12` is not already-there, `FootClass::Receive_Radio(0x12)` sets the Foot/NavCom destination (`+0x5A4`) through `FootClass::Set_Destination_Internal`, resets movement retry fields, and writes mission dispatch timer storage with `+0xD0=0`. The load-bearing effect is the destination/retry reset, not a separate chrono timing field.

The live dock/contact byte for stock refinery admission is `+0x418`. `TechnoClass::Receive_Radio(0x18)` sets it and propagates `0x18`; `0x19` clears it when that conditional message is used. This is distinct from reciprocal `+0x2E4` links.

Physical pad arrival is detected in `UnitClass::PerCellProcess @ 0x00739EC0`: the branch checks mission/destination/cell/locomotor gates, calls `FootClass::PerCellProcess(2)`, sends radio `0x15`, then calls locomotor slot `+0x5C`. The refinery receives `0x15` and queues sender mission `0x10`; that `0x10` is the unload mission id, not radio `0x10`.

The normal stock ore unload FSM then runs through `UnitClass::Mission_Deploy_Building @ 0x0073D630` with `unit+0x2E4 == 0`. The stock state-4 exit clears `+0x6D1`, may stop if still moving, and returns to harvest/mission scheduling. It does not call `ReleaseDockedHarvester`, does not `Force_Track(0x47)`, and does not install a fresh NavCom exit destination. `ReleaseDockedHarvester` and `Force_Track(0x47)` remain valid for conditional reciprocal-link paths.

## Implementation-Safe Facts

- Stock CMIN/GAREFN/NAREFN refinery docking must model a `0x02` contact step before the `0x0E` admission path.
- Accepted stock refinery `0x0E` target is NW `+(3,1)`, not `QueueingCell=4,1`.
- `0x12` owns the accepted-cell NavCom write and retry reset.
- `+0x418` is the stock radio/contact dock flag toggled by `0x18/0x19`.
- `+0x2E4` reciprocal links are not normal stock refinery arrival state.
- Successful pad arrival sends `0x15`; the receiver queues mission id `0x10`.
- Do not implement standard CMIN refinery exit as unconditional `ReleaseDockedHarvester` or `Force_Track(0x47)`.

## Doc-Patch-Ready Facts

- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` audit findings are safe to patch narrowly: `+0x418` is not destination state, `+0x2E4` is not written by `0x18/0x19`, stock chain includes `0x18`, and case `0x15` queues the sender/harvester mission `0x10`.
- Older `BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md` and `HARVESTER_DOCK_UNLOAD.md` wording that frames `UndockUnit`/`ReleaseDockedHarvester` as the normal stock refinery exit should be narrowed to conditional reciprocal-link paths.
- Any overview that says `QueueingCell=4,1` is the accepted dock cell should distinguish waiting/fallback staging from accepted `0x0E` NW `+(3,1)`.

## Stale Or Superseded Claims

- `+0x2E4` as the normal stock refinery dock link is superseded by `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING...`, `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM...`, and the `0x006F4AB0`/`0x00739EC0` spot-checks.
- Radio `0x10` as standard CMIN reserve-dock traffic is superseded by `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE...`, `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM...`, and `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL...`.
- Unconditional `Force_Track(0x47)` for stock post-unload exit is superseded by `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP...` and the `0x0073D630` spot-check.
- `FootClass +0xB4` as a team id and `+0xCC` as a chrono miner timer are superseded by the two FootClass radio move-field reports.

## Cross-Doc Conflicts

- `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` is useful as a map but currently YELLOW after audit; use the audit corrections and newer focused reports for implementation.
- `CHRONO_MINER_SYSTEM_OVERVIEW.md` is RED as of 2026-05-22; do not use it as implementation authority for warp/timer or Set_Destination summaries without a targeted patch pass.
- `HARVESTER_DOCK_UNLOAD.md` is YELLOW and contains wrong/misleading exit and acceptance details; use focused Ghidra reports for load-bearing behavior.

## Follow-Up Status

- Completed: `/re-investigate stock Mission_Deploy_Building refinery unload entry and exit reachability after 2026-05-22 audit`.
- Completed: `/re-investigate chrono miner refinery contact saturation and queue eviction edge cases`.
- Completed: `/trace-action chrono miner full cargo close return exact mission dispatch frame timing`.

Remaining bounded unknowns:

- exact live frame when the next miner takes contact after state-4 unload exit;
- exact mission `7` and mission `0x10` first-dispatch frame after queueing;
- contrived already-at-accepted-cell behavior while refinery `Contacts[]` is full;
- non-refinery factory/repair/bunker `0x17` queue semantics.

## Do-Not-Implement Notes

- Do not treat `QueueingCell=4,1` as the accepted `CAN_DOCK` move cell.
- Do not add stock radio `0x10` to CMIN refinery admission.
- Do not model `+0x418` as the NavCom destination; destination is `+0x5A4`.
- Do not model stock refinery docking as reciprocal `unit/building +0x2E4`.
- Do not use unconditional `Force_Track(0x47)` as the stock post-unload exit until a caller-specific path proves the reciprocal-link condition is active.

## Source Ledger

- Ghidra spot-checks in this synthesis: `0x0073D630`, `0x006F4AB0`, `0x004D94B0`, `0x00739EC0`, `0x0043C2D0`.
- Core docs: `MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`, `CHRONO_MINER_ACCEPTED_REFINERY_DOCK_ANCHOR_GHIDRA_REPORT.md`, `FOOTCLASS_RECEIVE_RADIO_0X12_MOVE_FIELDS_NAVCOM_GHIDRA_REPORT.md`, `TECHNOCLASS_RECEIVE_RADIO_DOCK_CASES_NAVCOM_GHIDRA_REPORT.md`, `UNITCLASS_PERCELLPROCESS_DOCK_ARRIVAL_00739EC0_NAVCOM_GHIDRA_REPORT.md`, `CHRONO_MINER_DOCK_ARRIVAL_LINK_TIMING_GHIDRA_REPORT.md`, `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`.
- Supporting docs: `FOOTCLASS_RADIO_MOVE_FIELDS_0XB4_0XCC_GHIDRA_REPORT.md`, `BUILDING_RECEIVE_RADIO_0X08_CLEARANCE_QUEUE_GHIDRA_REPORT.md`, `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`, `RADIO_0x10_RESERVE_DOCK_SENDER_TRACE_GHIDRA_REPORT.md`, `BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD_NAVCOM_GHIDRA_REPORT.md`.
- Follow-up canonical reports: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_REACHABILITY_GHIDRA_REPORT.md`, `CHRONO_MINER_REFINERY_CONTACT_SATURATION_QUEUE_EVICTION_GHIDRA_REPORT.md`, `miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`.
- Audit sources: `AUDIT_LOG.md` entries dated 2026-05-20 and 2026-05-22; `.swarm-claims.md` chrono miner nav/radio entries dated 2026-05-20/2026-05-21.
- INI data: `ini/rulesmd.ini` `[General] ChronoHarvTooFarDistance`, `[CMIN]`, `[HARV]`, `[GAREFN]`, `[NAREFN]`; `ini/artmd.ini` `[GAREFN]`, `[NAREFN]` foundation/QueueingCell/pad-open cells.
