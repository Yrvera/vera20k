# Radio Link Refinery Dock State Machine - Doc Conflict Audit

**Date:** 2026-05-24
**Target document:** `docs/research/miner/RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`
**Canonical comparison:** `docs/research/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
**Investigation mode:** exhaustive-slice for the requested stale-doc conflict anchors only.
**Active in YR:** Yes for all stock `CMIN/HARV -> GAREFN/NAREFN` findings below.

## Working Notes

- Target question: Which statements in `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` conflict with the current verified stock refinery dock model?
- Non-goals: Do not re-audit aircraft, repair, bunker, slave miner, service depot, carryall, 0x0F passenger, or factory queue radio messages except to mark them out of scope.
- Evidence needed to mark COMPLETE: For each conflict anchor, cite current synthesis plus fresh read-only Ghidra body evidence and an assembly range where the claim is handoff-critical.
- Stop conditions: Stop if Ghidra is unavailable, if the target expands beyond stock refinery dock/unload, or if a claim requires runtime frame ordering beyond static evidence.

## Verdict

The target report is **YELLOW/stale-in-body** for the requested scope. Its 2026-05-21/22 correction block and sections 7/11 mostly agree with the current stock model, but several older body sections remain misleading enough to break an implementation if read without the correction header.

The stale areas are:

- Section 2 overview and sections 5.1/5.2/5.3 still say or imply `Mission_Enter` sends `0x0E` "per tick" and that the `0x13 -> 0x12 -> 0x18 -> 0x16` burst is the normal single accepted reply sequence.
- Section 4, section 9, and section 10 understate `0x15` senders by listing `UnitClass::PerCellProcess @ 0x00739EC0` as the only stock sender.
- Section 4 and section 5 overstate `0x16` as "faces east/cascade sends 0x15" without the first-ordinary-`0x16` early return and without the later/already-synced gate conditions.
- Section 6 still lists `UnitClass::PerCellProcess` and `TechnoClass::Receive_Radio` as not deeply decompiled, now stale for this scope.
- Section 16 leaves OQ-8/OQ-10 as historical open questions even though both are resolved for the stock path.
- The +0x418/+0x2E4 and stock zero-link exit corrections are already present and should be preserved.

## Verified Binary Facts

1. `FootClass::Mission_Enter @ 0x004D9290` sends one `0x0E` through vtable+0x278 per mission dispatch, then returns `ftol([Enter].Rate * 900) + RandomRanged(0,2)`; it is not a literal every-tick poll unless the mission timer is due. Evidence: decompile `004D9290`; disassembly range `004D9290-004D9497`; canonical synthesis Timing and Tick Order. Active in YR: Yes, mission `7`/Enter is the stock return path.
2. `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E` sends `0x13`, then `0x12` with accepted target `NW+(3,1)` for `DockUnload/Weeder`; if the `0x12` reply is not `0x14`, it returns `1` and does not send `0x18` or `0x16`. Evidence: decompile `0043C2D0`; disassembly range `0043C2D0-0043CDBF`. Active in YR: Yes, stock GAREFN/NAREFN have `DockUnload=yes`.
3. `UnitClass::Receive_Radio @ 0x00737430` case `0x16` first delegates to `FootClass`, checks the facing timer/current value against `0x4000`, and if unsynced calls locomotor vtable+0x4C with `0x4000` then returns `1`. Only after the timer is already synced does it test idle locomotor, nonzero destination, contact flag, destination WhatAmI building, and mission `7` before sending `0x15`. Evidence: decompile `00737430`; disassembly range `00737430-00737B37`. Active in YR: Yes, emitted by building case `0x0E` after `0x18`.
4. `UnitClass::PerCellProcess @ 0x00739EC0` remains a separate `0x15` source, including the GetDockCoord-equality branch and the contact-flag adjacent-building branch; it is not the mission-7 dispatch handler and not the only stock `0x15` source. Evidence: decompile `00739EC0`; disassembly range `00739EC0-0073B30F`; `UnitClass::Receive_Radio 0x16` above. Active in YR: Yes, unit tick/per-cell path for stock harvesters.
5. `TechnoClass::Receive_Radio @ 0x006F4AB0` case `0x18` sets byte `+0x418` and propagates `0x18`; case `0x19` clears byte `+0x418` and propagates `0x19`. This is not reciprocal dock pointer `+0x2E4`. Evidence: decompile `006F4AB0`; disassembly range `006F4AB0-006F4E33`. Active in YR: Yes, building `0x0E` sends `0x18` in the stock admission path.
6. `UnitClass::Mission_Deploy_Building` containing `0x0073E277` uses the stock zero-link state-4 exit: clear `+0x6D1`, queue mission `10`/Harvest, optionally send radio `3` through the normal contact path, and return/commence. The nonzero `+0x2E4` branch calls `BuildingClass::ReleaseDockedHarvester`, but that is conditional and separate. Evidence: decompile at `0073E277`; disassembly range `0073D5B0-0073E617`; `ReleaseDockedHarvester @ 004595C0` range `004595C0-00459975`. Active in YR: Yes for stock zero-link unload; nonzero link path is conditional.

## Conflict Details

### Sequence Diagram / State Tables

The diagram's `loop per tick until dock accepted` should become a mission-dispatch loop with a 14..16-frame stock Enter delay. The current diagram correctly says no `0x18/0x16` while still moving, but it should also show that `0x16` itself may only sync facing/timer and return before any `0x15` handoff.

### Radio Catalog

`0x15` should list two source families for stock refinery docking:

- `UnitClass::Receive_Radio` case `0x16`, later/already-synced path, if idle with building destination, contact flag set, and mission `7`.
- `UnitClass::PerCellProcess @ 0x00739EC0`, through source-specific per-cell branches.

`0x16` should say "sync/facing timer; may later cascade `0x15` after its gates pass", not unconditional "cascade sends 0x15 back".

Aircraft/repair/bunker entries in the catalog are out of scope for this audit. No new verdict is made on those paths.

### 0x15 Sender/Source Wording

The statement that `PerCellProcess` sends `0x15` at pad arrival is true but incomplete. The stale part is treating `PerCellProcess` as the sender inventory's only stock source. The current model requires source-aware handoffs because `0x16 -> 0x15` and `PerCellProcess -> 0x15` have different preconditions and tick placement.

### +0x418 / +0x2E4 / Zero-Link Exit

No contradiction found in the target report's correction block, field table rows, or teardown section after the 2026-05-21/22 patches. Preserve the current wording that `+0x418` is the radio/contact-entered byte, `+0x2E4` is conditional reciprocal link state, and normal stock unload exits through `Mission_Deploy_Building` state 4 rather than `ReleaseDockedHarvester` or `Force_Track(0x47)`.

## Negative Facts / Do Not Do

- Do not model `Mission_Enter` `0x0E` as an every-frame retry; stock Enter returns 14..16 frames at default `[Enter] Rate=.016`. Evidence: `004D9290-004D9497`; Active in YR: Yes.
- Do not send `0x18/0x16` when building `0x12` returns ordinary `1`; only `0x14` already-there reaches the entered/sync messages. Evidence: `0043C2D0-0043CDBF`; Active in YR: Yes.
- Do not make first `0x16` start unload by itself; the ordinary unsynced case only calls locomotor vtable+0x4C with `0x4000` and returns. Evidence: `00737430-00737B37`; Active in YR: Yes.
- Do not require `PerCellProcess`/GetDockCoord equality as the only way to send stock `0x15`; later/already-synced `0x16` can send it without GetDockCoord. Evidence: `00737430` and `00739EC0`; Active in YR: Yes.
- Do not synthesize a normal stock reciprocal `+0x2E4` release, `ReleaseDockedHarvester`, or `Force_Track(0x47)` exit for healthy stock unload completion. Evidence: `0073D5B0-0073E617`, `004595C0-00459975`; Active in YR: Yes for the negative stock-path claim.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected surface | Required effect | Acceptance scenario | Proposed test | Risk |
|---|---|---|---|---|---|---|---|
| Enter retry is mission-dispatch gated and returns 14..16 frames | `004D9290`; canonical Timing | Rust has split MissionEnter/AwaitingAcceptedCell, but timer/RNG parity should remain pinned | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs` | No next-tick CAN_DOCK retry unless the Enter timer is due | Busy/accepted miner waits stock Enter delay before retrying | `mission_enter_candock_retry_uses_enter_timer_jitter` | High determinism/RNG risk |
| `0x12 == 1` means move/defer; only `0x12 == 0x14` sends `0x18/0x16` | `0043C2D0`; canonical One-Screen Flow | Rust already has `AwaitingAcceptedCell -> MissionEnter`; keep tests | `src/sim/miner/miner_dock_sequence.rs` | Arrival at accepted cell does not itself set contact-entered | Miner reaches NW+(3,1), remains not entered until next MissionEnter pass | `accepted_cell_arrival_rechecks_can_dock_before_entered_flag` | Frequent player-visible timing risk |
| First ordinary `0x16` can only sync facing/timer; later synced path can send `0x15` | `00737430`; canonical Claim Table | Rust has Pivoting, but `phase_linked`/pad naming should be checked against no forced NW+3->NW+2 move | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs` | Separate contact-entered, facing sync, and unload handoff; do not treat return `1` as unload | First sync leaves miner not unloading; later/synced pass enters unload | `first_0x16_syncs_without_starting_unload` | High timing and position risk |
| Normal stock exit is zero-link state 4, not reciprocal release | `0073D5B0-0073E617`, `004595C0-00459975` | Rust comments and tests already reflect no `Force_Track(0x47)` | `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs` | Departing clears dock bookkeeping without release-helper effects | Healthy full unload emits no refinery exit SFX/forced track | `stock_zero_link_departing_does_not_force_track_or_release_sfx` | High regression risk if reused for interrupts |

## Exact Stale-Doc Replacement Wording

Apply these replacements to `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md` if patching the stale body.

1. Replace section 2 overview paragraph with:

> When a harvester returns to a stock refinery with ore, the dock sequence proceeds through a mission-dispatch-gated approach, a source-aware dock handoff, a unit-side unload FSM, and a stock zero-link exit. `FootClass::Mission_Enter` sends one `CAN_DOCK(0x0E)` per due Enter dispatch and returns the stock `[Enter]` delay `ftol(.016 * 900) + RandomRanged(0,2) = 14..16` frames. `BuildingClass::Receive_Radio` case `0x0E` sends `0x13` then `0x12` with accepted target `building NW+(3,1)`; only when `0x12` replies `0x14` already-there does the building send `0x18` and `0x16`. The first ordinary `0x16` may only sync facing/timer through locomotor vtable+0x4C(`0x4000`) and return; a later/already-synced `0x16`, or separate `UnitClass::PerCellProcess` branches, can send `0x15` to make the building queue the harvester's `Mission_Deploy_Building`.

2. Replace the section 4 `0x0E` row note with:

> Primary dock admission message. `Mission_Enter` sends one `0x0E` per due dispatch, not every frame; stock Enter retry is 14..16 frames at default `[Enter] Rate=.016`.

3. Replace the section 4 `0x15` row sender/notes with:

> Sender(s): later/already-synced `UnitClass::Receive_Radio` case `0x16` and separate `UnitClass::PerCellProcess @ 0x00739EC0` branches. Notes: Building case `0x15` queues sender mission `0x10`; exact first source can be runtime-frame sensitive.

4. Replace the section 4 `0x16` row notes with:

> `UnitClass` case `0x16` first syncs facing/timer to `0x4000` and can return without `0x15`; only a later/already-synced call can send `0x15`, gated on idle locomotor, building destination, contact flag, and mission `7`.

5. Replace the section 5.1 loop label and `0x16`/`0x15` lines with:

> `loop each due Mission_Enter dispatch (stock 14..16 frame retry)`; `R->>H: 0x16 TIMING_SYNC (first call may only sync facing/timer)`; `H->>R: 0x15 DOCK_NOW (from later/synced 0x16 or PerCellProcess branch)`.

6. Replace section 10 `0x15` sender row with:

> `0x15 TIMING_SYNC_BACK / DOCK_NOW | (a) UnitClass::Receive_Radio case 0x16 later/already-synced path; (b) UnitClass::PerCellProcess @ 0x00739EC0 branches | Source-aware dock handoff to BuildingClass case 0x15 | 0x16 path plus PerCellProcess reports`

7. Replace section 16 OQ-8/OQ-10 with:

> OQ-8 RESOLVED for stock refinery: `UnitClass::PerCellProcess @ 0x00739EC0` sends `0x15` through separate per-cell branches and is not the mission-7 dispatch handler. OQ-10 RESOLVED for standard CMIN/HARV refinery docking: `TechnoClass::Receive_Radio @ 0x006F4AB0` handles `0x18/0x19` contact-state toggles and has no meaningful stock `0x10` handler.

## Remaining Uncertainty

- Exact first `0x15` source in every concrete replay frame remains runtime-sensitive; static code proves possible sources and ordering constraints, not every frame winner.
- Exact frame count from first unsynced `0x16` to the facing timer reaching `0x4000` remains locomotor/runtime-sensitive for per-unit `Rot`.
- This audit did not re-verify aircraft, repair, bunker, slave-miner, or factory queue radio semantics.

## Sources

- Ghidra decompile/read-only spot-checks: `004D9290`, `0043C2D0`, `00737430`, `00739EC0`, `0073E277`, `006F4AB0`, `004595C0`.
- Assembly ranges checked: `004D9290-004D9497`, `0043C2D0-0043CDBF`, `00737430-00737B37`, `00739EC0-0073B30F`, `0073D5B0-0073E617`, `006F4AB0-006F4E33`, `004595C0-00459975`.
- `docs/research/miner/STOCK_REFINERY_DOCK_UNLOAD_STATE_MACHINE_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`.
- Rust scan: `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/mod.rs`, `src/sim/miner/miner_tests.rs`.
