# HARV Post-Unload Exit Path - Ghidra Research Report

**Address(es):** `0x0073D630`, `0x004595C0`, `0x004593A0`, `0x0065AE30`, `0x0065ACB0`, `0x0065A970`, `0x0065A820`, `0x006F4AB0`, `0x0043C2D0`, `0x0073E5E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** healthy stock War Miner (`[HARV]`) finishing a stock refinery unload after storage is empty, specifically the `Mission_Deploy_Building` state-3 empty-slot gate through zero-link state 4, contact cleanup, exit destination/facing/track effects, and next Harvest scheduling.  
**Non-Scope:** destroyed/sold/temporal refinery interrupts, non-stock reciprocal `+0x2E4` links, Yuri slave miner, service depots/aircraft docks, runtime frame-perfect two-miner promotion order, and full Mission_Harvest ore-selection after the state-4 handoff.  
**Confidence:** High for the bounded stock HARV state-3-empty/state-4 exit path; Medium for exact same-frame queue takeover because that requires runtime scheduler observation.  
**Active in YR:** Yes. Stock `[HARV] Harvester=yes`, `Dock=NAREFN,GAREFN`, `[NAREFN]/[GAREFN] DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1` make this path live in standard YR.

## Working Notes

- **Target question:** After a healthy War Miner empties storage at a stock refinery, does stock YR exit through zero-link `Mission_Deploy_Building` state 4 or through `ReleaseDockedHarvester` / `UndockUnit`, and what cleanup/scheduling happens?
- **Non-goals:** do not re-document the whole miner FSM, chrono teleporter return path, interrupted building teardown, or multi-miner runtime promotion timing.
- **Evidence needed to mark COMPLETE:** direct binary evidence for path liveness, state write order, contact/radio order, reciprocal-link helper reachability, default stock INI gates, and current Rust handoff surfaces.
- **Stop conditions:** stop at static state-4 handoff once all direct callees affecting contact cleanup and scheduling are verified; defer runtime-only frame ordering and object iteration questions.

## 1. Overview

Stock healthy HARV completion is the zero-`unit+0x2E4` `UnitClass::Mission_Deploy_Building` path, not `BuildingClass::ReleaseDockedHarvester` and not `BuildingClass::UndockUnit`. The final empty-storage gate in state 3 writes `unit+0xBC = 4` and direct-returns `1`; the next mission call runs state 4, waits if refinery slot-8 `ProductionAnim` is still live, clears `unit+0x6D1`, sets mission Harvest `0x0A`, optionally sends radio `3` to clear contacts, queues/advances the mission, then returns through the mission timer epilogue.

No stock state-4 instruction seeds `Force_Track(0x47)`, `BunkerWallsDownSound`, `SpecialAnimThree/Four`, or a cached queue-cell exit destination. Those effects belong to the conditional reciprocal-link helper at `0x004595C0` and interrupt-style `UndockUnit` at `0x004593A0`.

## 2. Class Layout / Key Offsets

| Offset / field | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0x2E4` / `param_1[0xB9]` | reciprocal dock-link branch selector | `0x0073D63B` checks zero; `0x0073D66D` calls release only on nonzero branch | Conditional; stock completion keeps zero |
| Unit `+0xBC` / `param_1[0x2F]` | `Mission_Deploy_Building` substate | state-3 branch at `0x0073E2BF`; state-4 branch at `0x0073E17F`; write `4` at `0x0073E51C` | Yes |
| Unit `+0x6D1` | unload-active / dock presentation flag | set at `0x0073DFDA`; cleared in state 4 at `0x0073E1F6` | Yes |
| Unit `+0xB4` / `param_1[0x2D]` | queued mission id | normal branch tests `-1` and `0x0A` at `0x0073E201..0x0073E20F` | Yes |
| Unit `+0x5A4` | destination/target override pointer | read at `0x0073E1F0` before normal state-4 branch | Yes |
| Unit `+0x33C` | carried `StorageClass` | `StorageClass__FindFirstNonEmptySlot @ 0x006C9820` from `ESI+0x33C` | Yes |
| Unit `+0xF8` | dump-rate accumulator | compared against `HarvesterDumpRate * 900.0` at `0x0073E355..0x0073E374` | Yes |
| Radio contacts `+0xE4/+0xE8` | contact array pointer/capacity | `PathType__Has_Valid_Steps @ 0x0065AE30`; `RadioClass` funcs | Yes |
| Unit/building `+0x418` | entered/contact byte, not dock link | `TechnoClass__Receive_Radio @ 0x006F4AB0`: `0x18` sets, `0x19` clears | Yes |
| Building `+0x57C` | slot-8 `ProductionAnim` pointer guard | state-4 wait at `0x0073E1DF` | Yes; stock refineries normally null |
| Building `+0x584` | slot-10 `SpecialAnim` pointer | state 3 clears slot 10 after empty transition at `0x0073E526..0x0073E534` | Yes |
| BuildingType `+0x16B3` | `DockUnload=yes` | `BuildingClass::Receive_Radio(0x15)` sends sender mission `0x10` | Yes for GAREFN/NAREFN |
| BuildingType `+0x16BB` | `Refinery=yes` | state-4 wait guard at `0x0073E1D5`; state-3 slot-8 call | Yes for GAREFN/NAREFN |
| `g_refinery_unload_adjacent_lookup_dx/dy` | signed `(-1,0)` refinery rediscovery from miner cell | use at `0x0073E181..0x0073E1A3`, prior init report | Yes |

## 3. Core Logic

### 3.1 Path liveness and stock gates

Stock `[HARV]` has `Dock=NAREFN,GAREFN`, `Harvester=yes`, `Storage=40`, and `UnloadingClass=HORV` in `rulesmd.ini`. Stock `[NAREFN]` and `[GAREFN]` have `DockUnload=yes`, `Refinery=yes`, and `NumberOfDocks=1`.

`UnitClass::Mission_Deploy_Building @ 0x0073D630` starts by testing `unit+0x2E4`. If nonzero, it attempts a building lookup and calls `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` at `0x0073D66D`. If zero, it reaches the normal deploy/refinery FSM. Later reports and the `BuildingClass::Receive_Radio(0x15)` spot-check show stock DockUnload does not set the reciprocal `+0x2E4` link, so standard HARV unload completion stays on the zero-link path.

**Finding:** active in standard YR stock War Miner unload; high confidence.

### 3.2 State 3 empty-storage gate

State 3 rediscovers the refinery at current miner cell plus `(-1,0)`, checks the dump gate, spawns per-dump effects, finds the first non-empty storage slot at `unit+0x33C`, and drains one full storage slot. If no slot exists (`FindFirstNonEmptySlot == -1`) or removal is not positive, stock code:

- calls `BuildingClass__SetAnimSlotImage(slot=8)` if `Refinery=yes` (`0x0073E4DC..0x0073E517`);
- writes `unit+0xBC = 4` (`0x0073E51C`);
- clears slot 10 if `building+0x584` is non-null (`0x0073E526..0x0073E534`);
- falls to direct `return 1` at `0x0073E5B1..0x0073E5BD`.

This means cargo-empty does **not** run state 4 in the same `Mission_Deploy_Building` call. State 4 runs on the next mission invocation.

**Finding:** active in standard YR stock War Miner unload; high confidence.

### 3.3 State 4 healthy stock exit order

For non-Weeder stock HARV, state 4 at `0x0073E17F` performs this order:

1. Re-find refinery by miner current cell plus `(-1,0)` and `Look_up_building_in_cell` (`0x0073E181..0x0073E1C6`).
2. If a building exists, `Refinery=yes`, and `building+0x57C != 0`, direct-return `1` without clearing `+0x6D1` or radio contacts (`0x0073E1CB..0x0073E1EA`).
3. Clear `unit+0x6D1 = 0` (`0x0073E1F6`).
4. If there is no overriding destination/queued mission, call vtable `+0x1E8` with mission `0x0A` and queued flag `0` (`0x0073E24D..0x0073E254`).
5. Call vtable `+0x200`; if false, skip radio and queue call and proceed to timer epilogue (`0x0073E25A..0x0073E266`).
6. Call `PathType__Has_Valid_Steps`; if true, send radio `3` via vtable `+0x274` (`0x0073E268..0x0073E279`).
7. Call vtable `+0x1EC` to queue/advance mission (`0x0073E27F..0x0073E283`).
8. Return through `MissionClass__GetMissionTimerEntry`, `Math__ftol`, and `RandomRanged(0,2)` (`0x0073E289..0x0073E2BE`).

**Finding:** active in standard YR stock War Miner unload; high confidence.

### 3.4 Contact cleanup order

`PathType__Has_Valid_Steps @ 0x0065AE30` scans contact array `+0xE4` for `+0xE8` entries and returns true when any contact slot is non-null. In state 4, that means radio `3` is only sent if the miner still has a contact.

Radio `3` cleanup is synchronous:

- `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` sends to `Contacts[0]` if present, otherwise returns `0`.
- `RadioClass__Transmit_Radio_Impl @ 0x0065A970` handles message `3` by clearing every sender-side contact slot equal to the target before forwarding the message.
- `BuildingClass__Receive_Radio @ 0x0043C2D0` case `3` calls `GrandOpening`, delegates to `TechnoClass__Receive_Radio`, and returns `1`.
- `TechnoClass__Receive_Radio @ 0x006F4AB0` case `3` can send `0x19` when both participants have `+0x418` set; case `0x19` clears `+0x418` and propagates once.
- `RadioClass__Receive_Radio @ 0x0065A820` case `3` then finds the sender in receiver contacts, calls `ObjectClass__Receive_Radio`, clears that receiver slot, and returns `1`.

Therefore state-4 contact cleanup happens after `+0x6D1` clear and after mission Harvest assignment, but before vtable `+0x1EC` mission queue/advance returns. It is not tied to physical movement off the pad.

**Finding:** active in standard YR stock War Miner unload; high confidence.

### 3.5 No stock exit destination, facing, or track

No instruction in the zero-link state-4 branch calls `Force_Track`, writes track `0x47`, plays `BunkerWallsDownSound`, creates slots 12/13, calls `Find_Nearby_Passable_Cell`, or writes a queued exit destination.

Those effects are in `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, which:

- clears anim slots 10/11;
- optionally plays `BunkerWallsDownSound`;
- creates slots 12/13;
- reads reciprocal `building+0x2E4`;
- clears `unit+0x2E4`;
- powers the locomotor, calls track `0x47` with `(-0x80,+0x80)` lepton offsets from building coords, sets speed `1.0`;
- computes a passable-cell destination from `(NW.x - 1, NW.y + 1)`;
- sets unit mission Move `2`;
- clears `building+0x2E4`, resets building state, and sends radio `3`.

`BuildingClass__UndockUnit @ 0x004593A0` has similar track `0x47` and reciprocal-link clear, but callers are `BuildingClass__ReceiveDamage`, `BuildingClass__Sell`, and `TemporalClass__Update`, not normal stock unload.

**Finding:** active helper bodies are conditional; not the normal stock HARV cargo-empty exit. High confidence.

### 3.6 Next Mission_Harvest scheduling

State 4 normal exit calls `SetMission(0x0A,0)` and `QueueMission` / vtable `+0x1EC`, then returns through the mission timer epilogue. `MissionClass__GetMissionTimerEntry @ 0x005B3A00` indexes the mission timer table by current mission id at byte `+0xAC`; the epilogue multiplies timer entry `+0x10` by `900.0`, converts with `Math__ftol`, and adds `RandomRanged(0,2)`.

`UnitClass__Mission_Harvest @ 0x0073E5E0` case 0 then resumes harvest/search behavior. If `unit+0x218` last harvest cell exists, it can set that as destination and preserve the ghost-cell archive before search. That later ore-selection path is downstream and outside this report's exact exit-path scope.

**Finding:** active in standard YR stock War Miner unload; high confidence for the state-4 handoff, medium for later search target details because this report did not exhaust Mission_Harvest.

## 4. INI Keys

| Key | YR stock value | Effect in this slice | Evidence |
|---|---|---|---|
| `[HARV] Dock` | `NAREFN,GAREFN` | stock refinery candidates | `ini/rulesmd.ini:8225` |
| `[HARV] Harvester` | `yes` | enters harvester unload path via `UnitType+0xE0E` | `ini/rulesmd.ini:8228`; `0x0073D678` |
| `[HARV] Storage` | `40` | storage capacity; state 3 drains `StorageClass` | `ini/rulesmd.ini:8236`; `unit+0x33C` |
| `[HARV] UnloadingClass` | `HORV` | unloading display override; not the state-4 exit gate | `ini/rulesmd.ini:8246` |
| `[NAREFN] DockUnload` | `yes` | radio `0x15` queues sender mission `0x10` | `ini/rulesmd.ini:12519`; `0x0043C2D0` |
| `[NAREFN] Refinery` | `yes` | state-4 `+0x57C` wait and state-3 anim calls | `ini/rulesmd.ini:12520`; `0x0073E1D5` |
| `[NAREFN] NumberOfDocks` | `1` | one contact slot | `ini/rulesmd.ini:12521` |
| `[GAREFN] DockUnload` | `yes` | same DockUnload handoff | `ini/rulesmd.ini:11726`; `0x0043C2D0` |
| `[GAREFN] Refinery` | `yes` | same state-4 wait gate | `ini/rulesmd.ini:11727`; `0x0073E1D5` |
| `[GAREFN] NumberOfDocks` | `1` | one contact slot | `ini/rulesmd.ini:11729` |
| `[General] HarvesterDumpRate` | `0.016` | state-3 gate threshold `0.016 * 900.0 = 14.4` | prior state-3 report; `0x0073E355..0x0073E374` |

## 5. Integration Points

| Function | Role | Verified details |
|---|---|---|
| `UnitClass__Mission_Deploy_Building @ 0x0073D630` | primary state machine | zero-link stock state 3 empties storage, state 4 clears `+0x6D1`, radios `3`, queues Harvest |
| `BuildingClass__Receive_Radio @ 0x0043C2D0` | refinery radio receiver | case `0x15` sets sender mission `0x10`; no reciprocal `+0x2E4` write on stock handoff |
| `PathType__Has_Valid_Steps @ 0x0065AE30` | contact-present test in state 4 | true if any contact slot is non-null |
| `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0` | sends state-4 radio `3` to first contact | returns `0` if no contact |
| `RadioClass__Transmit_Radio_Impl @ 0x0065A970` | sender-side radio cleanup | radio `3` clears sender contacts before target receive |
| `BuildingClass__Receive_Radio(3)` + `TechnoClass__Receive_Radio(3)` + `RadioClass__Receive_Radio(3)` | receiver-side cleanup | clears `+0x418` via `0x19` cascade and receiver contact slot |
| `BuildingClass__ReleaseDockedHarvester @ 0x004595C0` | conditional reciprocal-link release | only called from nonzero `unit+0x2E4` entry branch |
| `BuildingClass__UndockUnit @ 0x004593A0` | interrupt/sell/destroy ejection | callers are damage, sell, temporal; not stock healthy unload |
| `UnitClass__Mission_Harvest @ 0x0073E5E0` | downstream next mission | state 4 schedules mission `0x0A`; later harvest/search resumes here |

## 6. Current Rust Implementation Status

Relevant Rust surfaces:

- `src/sim/miner/mod.rs:86` defines `RefineryDockPhase`; `Unloading` explicitly transitions to `Departing` on the empty-slot gate, `DepositCooldown` is legacy/pass-through, and `Departing` documents the stock zero-link state-4 handoff.
- `src/sim/miner/mod.rs:286` keeps `deposit_cooldown_ticks` only for legacy states and keeps `exit_cell` as a legacy/conditional cache, not a stock completion destination.
- `src/sim/miner/miner_dock_sequence.rs:763` drains one slot per threshold crossing.
- `src/sim/miner/miner_dock_sequence.rs:847` sends empty cargo directly to `Departing` without seeding another dump-gate cooldown.
- `src/sim/miner/miner_dock_sequence.rs:870` releases pad/contact bookkeeping, clears display override, clears movement/track state, clears `reserved_refinery`, and hands back to SearchOre scheduling without `Force_Track(0x47)`.
- `src/sim/miner/miner_tests.rs:3017` covers stock Departing returning to SearchOre without a cached exit move.
- `src/sim/miner/miner_tests.rs:3053` covers no stock `Force_Track(0x47)`.
- `src/sim/miner/miner_tests.rs:3518` covers a full War Miner dock cycle ending at the pad with no queue-cell exit move.
- `src/sim/miner/miner_tests.rs:3952` covers the empty-slot gate: first tick enters `Departing` and keeps the dock occupied, next tick runs state-4 handoff and releases it.
- `src/sim/miner/miner_tests.rs:4216` keeps old `DepositCooldown` save states as pass-through to `Departing`.

Observed delta: current Rust broadly matches the verified stock zero-link handoff. The main remaining parity risks are the modded `building+0x57C` wait guard and exact same-frame two-miner promotion/object-order behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Output report existence | verified | `Test-Path` returned false before writing | none |
| Stock HARV INI gates | verified | `rulesmd.ini` lines for `[HARV]`, `[NAREFN]`, `[GAREFN]` | none |
| `unit+0x2E4` top-level split | verified | `0x0073D63B`, `0x0073D641`, `0x0073D66D` | non-stock reciprocal-link frequency out of scope |
| `ReleaseDockedHarvester` body | verified | decompile `0x004595C0`; caller xref only from `0x0073D66D` | none for stock exclusion |
| `UndockUnit` body and callers | verified | decompile `0x004593A0`; callers ReceiveDamage/Sell/Temporal | none |
| State 3 empty-slot transition | verified | `0x0073E4DC..0x0073E5BD` | none |
| State 4 `+0x57C` wait before cleanup | verified | `0x0073E1CB..0x0073E1EA` | runtime duration for modded slot-8 anim |
| State 4 `+0x6D1` clear order | verified | `0x0073E1F6` | none |
| State 4 `SetMission(0x0A,0)` order | verified | `0x0073E24D..0x0073E254` | exact vtable method naming not needed |
| State 4 radio `3` condition | verified | `0x0073E268..0x0073E279`; `0x0065AE30` | whether vtable `+0x200` is always true in all stock edge cases |
| Sender contact clear | verified | `0x0065A970` radio `3` branch | none |
| Receiver contact and `+0x418` clear | verified | `0x0043C2D0`, `0x006F4AB0`, `0x0065A820` | none |
| Timer epilogue after normal state 4 | verified | `0x0073E289..0x0073E2BE`, `0x005B3A00` | exact next tick scheduler outside slice |
| Downstream Mission_Harvest resume | touched-not-exhausted | `0x0073E5E0` decompile | full ore target selection and pathing are separate reports |
| Current Rust `Departing` handoff | verified by code scan | `src/sim/miner/miner_dock_sequence.rs:870` | focused tests not rerun in this research slot |
| Current Rust tests for HARV handoff | verified by code scan | `src/sim/miner/miner_tests.rs:3518`, `3952` | runtime binary trace still needed for frame-perfect two-miner order |

## 8. Open Questions - Final State Of The Investigation Log

- `[RESOLVED] OQ-01 - What mode applies? -> exhaustive-slice for stock healthy HARV state-3-empty through state-4 exit; runtime object scheduler questions are explicitly outside the claimed slice.` (evidence: user scope; primary function boundary `0x0073D630`)
- `[RESOLVED] OQ-02 - Does the output report already exist? -> No.` (evidence: `Test-Path C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARV_POST_UNLOAD_EXIT_PATH_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-03 - Is the normal stock path zero-link? -> Yes; `unit+0x2E4 == 0` enters the stock FSM, nonzero calls release.` (evidence: `0x0073D63B`, `0x0073D641`, `0x0073D66D`)
- `[RESOLVED] OQ-04 - Is `ReleaseDockedHarvester` normal stock cargo-empty exit? -> No; only the nonzero `+0x2E4` branch calls it.` (evidence: `0x0073D66D`; `0x004595C0` xrefs)
- `[RESOLVED] OQ-05 - Is `UndockUnit` normal healthy exit? -> No; callers are damage, sell, and temporal update.` (evidence: callers of `0x004593A0`)
- `[RESOLVED] OQ-06 - What stock INI gates make HARV live? -> `[HARV] Harvester=yes`, `Dock=NAREFN,GAREFN`; refineries have `DockUnload=yes`, `Refinery=yes`, `NumberOfDocks=1`.` (evidence: `rulesmd.ini`)
- `[RESOLVED] OQ-07 - What triggers state 3 -> state 4? -> no non-empty storage slot or no positive removal at a dump gate; code writes state 4 and direct-returns `1`.` (evidence: `0x0073E4DC..0x0073E5BD`)
- `[RESOLVED] OQ-08 - Does state 4 run in the same function call as empty detection? -> No; empty detection direct-returns `1` after setting state 4.` (evidence: `0x0073E5B1..0x0073E5BD`)
- `[RESOLVED] OQ-09 - Does state 4 wait before cleanup? -> Yes, while found refinery is `Refinery=yes` and `building+0x57C != 0`.` (evidence: `0x0073E1CB..0x0073E1EA`)
- `[RESOLVED] OQ-10 - When is unload-active `+0x6D1` cleared? -> after the `+0x57C` wait guard passes and before mission/radio cleanup.` (evidence: `0x0073E1F6`)
- `[RESOLVED] OQ-11 - What mission is scheduled next? -> Harvest mission `0x0A`, queued flag `0`, then vtable `+0x1EC`.` (evidence: `0x0073E24D..0x0073E283`)
- `[RESOLVED] OQ-12 - When does radio `3` fire? -> after Harvest assignment and successful vtable `+0x200`, only if contacts are non-empty.` (evidence: `0x0073E25A..0x0073E279`; `0x0065AE30`)
- `[RESOLVED] OQ-13 - What does `PathType__Has_Valid_Steps` mean here? -> It scans contact slots and returns true if any slot is non-null.` (evidence: `0x0065AE30`)
- `[RESOLVED] OQ-14 - What contact side clears first? -> sender-side contacts clear in `Transmit_Radio_Impl(3)` before receiver receives radio.` (evidence: `0x0065A970`)
- `[RESOLVED] OQ-15 - Does receiver contact and `+0x418` clear? -> Yes; Building delegates to Techno, Techno can cascade `0x19`, base Radio clears receiver slot.` (evidence: `0x0043C2D0`, `0x006F4AB0`, `0x0065A820`)
- `[RESOLVED] OQ-16 - Is any stock state-4 exit movement/track seeded? -> No; no `Force_Track`, destination, or exit-cell write in the zero-link state-4 branch.` (evidence: `0x0073E17F..0x0073E2BE`)
- `[RESOLVED] OQ-17 - Where do `Force_Track(0x47)` and `BunkerWallsDownSound` live? -> conditional `ReleaseDockedHarvester` and interrupt-style `UndockUnit`, not stock state 4.` (evidence: `0x004595C0`, `0x004593A0`)
- `[RESOLVED] OQ-18 - Does current Rust encode zero-link state 4? -> Yes; `phase_unloading` enters `Departing`, and `phase_departing` clears dock bookkeeping without release-helper effects.` (evidence: `src/sim/miner/miner_dock_sequence.rs:847`, `870`)
- `[RESOLVED] OQ-19 - Are there Rust acceptance tests for this? -> Yes: no exit move, no force track, full War Miner dock cycle, empty-gate next-state handoff, legacy cooldown pass-through.` (evidence: `src/sim/miner/miner_tests.rs:3017`, `3053`, `3518`, `3952`, `4216`)
- `[DEFERRED] OQ-20 - Is vtable `+0x200` always true in every stock healthy state-4 edge case?` (category: `requires-different-system-context`; reason: the branch was identified and ordered, but the virtual target was not resolved in this slot; next-step-if-pursued: resolve UnitClass vtable `+0x200` and test normal HARV/CMIN state 4)
- `[DEFERRED] OQ-21 - Does a waiting second miner enter in the same rendered frame as radio `3` contact clear?` (category: `needs-runtime-debugger`; reason: static binary proves contact clear order but not object iteration timing; next-step-if-pursued: watch two miners and refinery contacts across the exact handoff frame)
- `[DEFERRED] OQ-22 - How long does a modded slot-8 `ProductionAnim` keep `building+0x57C` non-null?` (category: `requires-different-system-context`; reason: stock refineries normally have no live slot-8 delay; next-step-if-pursued: trace anim slot 8 creation/lifetime on a modded refinery)
- `[DEFERRED] OQ-23 - Does save/load mid-state-4 preserve all radio/contact/animation transient fields identically?` (category: `out-of-scope`; reason: target is live healthy unload exit; next-step-if-pursued: save at state 3 empty gate, state 4 wait, and after radio `3`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Stock healthy HARV completion is zero-link state 4, not release helper | `0x0073D63B`, `0x0073D66D`, `0x004595C0` xref | none observed | `src/sim/miner/miner_dock_sequence.rs:870` | keep normal completion independent from reciprocal `+0x2E4` links | `full_dock_cycle_war_miner` ends without forced track or exit move | Do not call/model `ReleaseDockedHarvester` for normal stock cargo-empty completion |
| Empty storage at state 3 writes state 4 and returns `1`; cleanup runs next mission call | `0x0073E4DC..0x0073E5BD` | matches current next-tick test | `phase_unloading`, `empty_unload_gate_releases_dock_on_next_stock_state4_handoff` | do not release dock in same Rust tick that only detects empty slot if modeling function-call boundary | first tick enters Departing and dock remains occupied; next tick releases | Do not add another full dump interval after the empty-slot gate |
| State 4 waits on `building+0x57C` before clearing unload-active/contact | `0x0073E1CB..0x0073E1EA` | stock path likely OK; modded anim wait unchecked | `phase_departing`, future building anim slot state | if slot-8 `ProductionAnim` is modeled, hold the dock until it clears | modded refinery with slot-8 anim keeps miner dock-active until anim ends | Do not release contacts before this wait guard when the pointer is live |
| State 4 clears `+0x6D1`, sets mission Harvest `0x0A`, then conditionally radios `3` | `0x0073E1F6`, `0x0073E24F`, `0x0073E275` | broad match via `Departing` cleanup and SearchOre handoff | `phase_departing`, display override cleanup, dock contacts | clear unload display/bookkeeping before or with contact release and return to harvest/search scheduling | after handoff display override is gone, dock released, state is SearchOre/WaitNoOre | Do not couple display cleanup to an outbound movement target |
| Radio `3` clears sender contacts before receiver contacts and clears `+0x418` via `0x19` cascade | `0x0065A970`, `0x0043C2D0`, `0x006F4AB0`, `0x0065A820` | Rust uses explicit contact/on-pad abstraction | `src/sim/miner/miner_dock.rs`, `phase_departing` | release contact/entered bookkeeping as state-4 radio cleanup abstraction | no stale contact/entered flag after Departing | Do not confuse `+0x418` with reciprocal `+0x2E4` |
| No stock state-4 `Force_Track(0x47)`, sound, anim slots 12/13, or cached exit destination | absence in `0x0073E17F..0x0073E2BE`; presence in `0x004595C0`/`0x004593A0` | current tests cover no force track and no explicit exit move | `phase_departing`, `exit_cell`, movement fields | keep `exit_cell` empty and movement target/drive track cleared on stock handoff | `stock_departing_does_not_start_force_track_0x47`; `stock_departing_hands_directly_to_search_without_exit_move` | Do not reuse conditional release-helper track/destination for stock completion |
| Next mission is Harvest `0x0A` then timer epilogue; Mission_Harvest resumes downstream search | `0x0073E24F..0x0073E2BE`, `0x0073E5E0` | approximated by SearchOre/WaitNoOre scheduling | `phase_departing`, `SearchOre`, last-harvest archive handling | preserve archive and let normal ore search consume it after handoff | post-dock miner returns to SearchOre/WaitNoOre and can resume saved productive patch | Do not clear `last_harvest_cell` as part of dock cleanup |

### Concrete Rust Test-Name Proposals

- `war_miner_empty_slot_gate_holds_dock_until_state4_call` - keep existing behavior covered by `empty_unload_gate_releases_dock_on_next_stock_state4_handoff`.
- `war_miner_state4_waits_for_refinery_slot8_production_anim` - add when building anim slot 8 is modeled.
- `war_miner_state4_releases_contact_before_search_resumes` - assert contact/entered cleanup and post-handoff scheduling order.
- `war_miner_stock_state4_preserves_last_harvest_cell_archive` - ensure next SearchOre can reuse the pre-dock productive patch.
- `war_miner_stock_state4_never_uses_reciprocal_release_track` - keep no `Force_Track(0x47)`, no release sound, no exit-cell cache.
- `war_miner_two_miner_handoff_runtime_trace_parity` - only after debugger trace resolves same-frame object ordering.

### Negative Facts / Do Not Do

- Do not model healthy stock HARV cargo-empty completion as `ReleaseDockedHarvester`.
- Do not call `UndockUnit` for normal stock post-unload exit.
- Do not seed `Force_Track(0x47)`, facing target `0x47`, `BunkerWallsDownSound`, slot 12/13 anims, or a cached queue-cell destination on stock state 4.
- Do not release the dock on the same state-3 empty-slot function call; state 3 sets state 4 and returns `1`.
- Do not release contact/unload-active before the `building+0x57C` wait guard if that pointer is live.
- Do not treat `+0x418` as the reciprocal dock link; it is radio entered/contact state and is distinct from `+0x2E4`.
- Do not collapse art `QueueingCell` into the stock post-unload exit destination; stock state 4 does not install an outbound exit destination.

### Stale Docs / Follow-up Docs

- Replace any claim that "`ReleaseDockedHarvester` is the universal normal stock HARV/CMIN post-unload exit" with: "Stock healthy `CMIN/HARV -> GAREFN/NAREFN` unload completion normally exits through zero-link `Mission_Deploy_Building` state 4; `ReleaseDockedHarvester` is only the nonzero reciprocal `+0x2E4` branch."
- Replace any claim that "`UndockUnit` is normal healthy refinery unload exit" with: "`UndockUnit` is an interrupt/sell/destroy/temporal ejection helper for nonzero reciprocal dock links, not healthy stock cargo-empty completion."
- Replace any wording that "empty cargo immediately releases the dock" with: "State 3 empty-slot gate writes `unit+0xBC = 4` and direct-returns `1`; the next state-4 invocation performs unload/contact cleanup."
- Replace any wording that "stock post-unload exit drives through Force_Track/queue cell" with: "Stock zero-link state 4 schedules Harvest and clears radio contact; no `Force_Track(0x47)` or cached exit destination is installed."

## Remaining Uncertainty

- Exact same-frame second-miner takeover after radio `3` contact cleanup needs runtime debugger observation of object iteration order.
- Vtable `+0x200` was ordered but not resolved to a named method in this slot; if an edge case makes it false, radio `3` and queue advance are skipped for that call.
- Modded refineries with a live slot-8 `ProductionAnim` need an anim-lifetime trace to reproduce the `building+0x57C` wait length.
- Save/load behavior across state 3 empty gate, state 4 wait, and post-radio cleanup was not investigated.

## Sources

- Ghidra decompiled/read-only: `UnitClass__Mission_Deploy_Building @ 0x0073D630`, `BuildingClass__ReleaseDockedHarvester @ 0x004595C0`, `BuildingClass__UndockUnit @ 0x004593A0`, `PathType__Has_Valid_Steps @ 0x0065AE30`, `RadioClass__Transmit_Radio_ToFirst @ 0x0065ACB0`, `RadioClass__Transmit_Radio_Impl @ 0x0065A970`, `RadioClass__Receive_Radio @ 0x0065A820`, `TechnoClass__Receive_Radio @ 0x006F4AB0`, `BuildingClass__Receive_Radio @ 0x0043C2D0`, `UnitClass__Mission_Harvest @ 0x0073E5E0`, `MissionClass__GetMissionTimerEntry @ 0x005B3A00`.
- Ghidra caller/callee/xref checks: `BuildingClass__ReleaseDockedHarvester` caller only from `UnitClass__Mission_Deploy_Building`; `BuildingClass__UndockUnit` callers from `BuildingClass__ReceiveDamage`, `BuildingClass__Sell`, `TemporalClass__Update`; `UnitClass__Mission_Deploy_Building` callees include release helper, storage, radio contact helper, and timer functions.
- Prior reports read: `STOCK_MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_PATHTYPE_STATE4_GHIDRA_REPORT.md`, `TWO_MINER_ONE_REFINERY_ZERO_LINK_HANDOFF_TIMING_GHIDRA_REPORT.md`, `RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md`, `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`.
- Rust scanned: `src/sim/miner/mod.rs`, `src/sim/miner/miner_dock_sequence.rs`, `src/sim/miner/miner_tests.rs`.

**Status:** COMPLETE for the bounded healthy stock HARV post-unload exit path; runtime same-frame multi-miner ordering remains a separate debugger-trace question.
