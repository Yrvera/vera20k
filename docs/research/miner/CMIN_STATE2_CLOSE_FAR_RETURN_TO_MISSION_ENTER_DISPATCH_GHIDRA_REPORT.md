# CMIN State-2 Return To Mission Enter Dispatch - Ghidra Research Report

**Address(es):** `0x0073E5E0`, `0x004DF040`, `0x0043C2D0`, `0x004D9290`, `0x005B3060`, `0x005B35E0`, `0x005B3570`, `0x007360C0`, `0x00744270`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR `CMIN` `Mission_Harvest` state-2 return branch from full/forced return target selection through close/far dispatch and the first `Mission_Enter` `CAN_DOCK` admission attempt.  
**Non-Scope:** unload drain, two-miner queue handoff after release, destroyed/sold refinery mid-unload, `0x15` post-arrival unload handoff, and post-unload exit.  
**Confidence:** High for static ordering and scheduler boundary; Medium for exact randomized retry frame counts because the runtime mission timer table value was not read from a live process.  
**Active in YR:** Yes. Stock `[CMIN]` has `Harvester=yes`, `Teleporter=yes`, and `Dock=NAREFN,GAREFN`; stock `[GAREFN]`/`[NAREFN]` have `DockUnload=yes` and `Refinery=yes`.

## Working Notes Gate

- Target question: verify the exact standard YR `CMIN` state-2 return branch from return decision into close/far refinery approach and first `Mission_Enter` dock admission.
- Non-goals: no unload drain, no two-miner release handoff, no destroyed-refinery unload abort, no post-unload exit, no Rust edits.
- Evidence needed to mark COMPLETE: decompile plus assembly context for state-2 close/far branch, queue/commence helper, `UnitClass::AI` commence gates, and first `Mission_Enter` `0x0E` dispatch; INI proof that stock YR reaches the branch.
- Stop conditions: stop after proving selected refinery preservation/fallback target class, exact queue/commence boundary, and first `Mission_Enter` dispatch entry; record runtime-only timer table values as remaining uncertainty rather than expanding scope.

## 1. Overview

`Mission_Harvest` state 2 first chooses a refinery from the unit type's `Dock=` list. For `CMIN`, a close result sends radio `0x02` to the selected refinery object and advances only to harvest substate `3`; it does not send `CAN_DOCK(0x0E)` yet. A far or refused close result uses the second fallback dock search and sets a movement destination near `QueueingCell`.

Substate `3` then calls `Queue_Mission(7, 0)`. Because the commence flag is `0`, this writes queued mission `+0xB4` but does not dispatch `Mission_Enter` inside the harvest handler. `UnitClass::AI` has a late `ReadyToCommence`/`Commence` gate after `FootClass::AI`; that gate promotes mission `7` to current mission in the same unit AI frame, and the first `FootClass::Mission_Enter @ 0x004D9290` dispatch is on the next mission-dispatch pass for that unit, normally the next frame.

## 2. Key Offsets / Slots

| Offset / slot | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Unit `+0xBC` | `Mission_Harvest` substate; `2` return, `3` queue enter | `0x0073E5E0`, `0x0073EE68` | Yes |
| Unit `+0x5A4` | current destination/NavCom; state 2 only chooses a new return target when zero | `0x0073E5E0`; `0x004D9290` | Yes |
| Unit type `+0xCD4` | `Teleporter=yes`, selects chrono threshold/fallback behavior | `0x0073E5E0`; `rulesmd.ini:7396` | Yes for CMIN |
| Unit type `+0x3E8/+0x3F8` | `Dock=` vector and count | `0x004DF040`; `rulesmd.ini:7361` | Yes |
| Rules `+0xD7C` | `ChronoHarvTooFarDistance` in cells | `0x0073EE40`; `rulesmd.ini:294` | Yes |
| BuildingType `+0x1618/+0x161C` | `QueueingCell` X/Y, fallback only | `0x0073EEC1..0x0073EED0`; `artmd.ini:1716,1773` | Conditional |
| Mission `+0xAC` | current mission id | `0x005B3060`, `0x005B3A00` | Yes |
| Mission `+0xB4` | queued mission id | `0x005B35E0`, `0x005B3570` | Yes |
| Mission `+0xB8` | queued-mission aux byte, cleared by queue/commence | `0x005B35E0`, `0x005B3570` | Yes |
| Mission `+0xC8/+0xD0` | dispatch timer start/duration | `0x005B3060`, `0x005B3570` | Yes |
| vtable `+0x1E8` | `MissionClass::Queue_Mission @ 0x005B35E0` | decompile `0x005B35E0` | Yes |
| vtable `+0x1EC` | `MissionClass::Commence @ 0x005B3570` | decompile `0x005B3570` | Yes |
| Unit vtable `+0x200` | `UnitClass::ShouldIdle @ 0x00744270`, ReadyToCommence gate for this unit | `0x007360C0`, `0x00744270` | Yes |
| Mission id `7` slot `+0x240` | `FootClass::Mission_Enter @ 0x004D9290` | `0x005B3060` case `7` | Yes |

## 3. Core Logic

### State 2 Target Selection And Close Threshold

State 2 calls the docking-bay search through vtable `+0x528`, resolved to `FootClass::Find_Docking_Bay @ 0x004DF040`, with the `Dock=` vector and `arg3=0`. The selected object is the refinery object, preserving an explicit clicked/selected refinery if it is the current target/dock candidate from the order path.

For `CMIN`, the code computes 3D lepton distance between the miner object coordinate and the refinery object coordinate and compares it inclusively:

```text
distance <= Rules.ChronoHarvTooFarDistance * 0x100
```

Stock YR has `ChronoHarvTooFarDistance=50`, so the inclusive close threshold is `12800` leptons. Evidence: `0x0073EE35..0x0073EE4B`, `rulesmd.ini:294`. Active in YR: Yes.

### Close Branch Output

If close, state 2 sends radio `0x02` to the refinery object. Assembly context at `0x0073EE54..0x0073EE68` shows `PUSH refinery`, `PUSH 0x2`, call vtable `+0x278`, compare reply to `1`, then write `+0xBC = 3`. There is no `0x0E`, `0x12`, `0x18`, `0x16`, or `0x15` in this state-2 close branch.

After the substate write, control jumps to the normal `Mission_Harvest` timer epilogue at `0x0073EF77`: `GetMissionTimerEntry`, multiply the entry's `Rate` by `900.0`, and add `RandomRanged(0,2)`. Therefore state 3 is not necessarily dispatched on the immediately following frame; it is dispatched when mission 10's timer is next eligible. Active in YR: Yes.

### Far / Refused-Close Fallback

If the close branch does not fire, state 2 brackets a second docking-bay search with `g_MapEditorMode++` / `--` and calls the same dock search with `arg3=1`. For `CMIN`, whenever that fallback returns a refinery, the branch computes:

```text
seed = refinery_anchor + BuildingType.QueueingCell
result = Find_Nearby_Passable_Cell(seed, radius=2, ...)
Set_Destination(result CellClass*) or clear destination if invalid
```

Evidence: `0x0073EEA6..0x0073EF41`, `artmd.ini:1716`, `artmd.ini:1773`. This fallback target is the waiting/staging cell, not the later accepted `CAN_DOCK` cell. Active in YR: Conditional, when normal close reservation/admission does not happen.

### State 3 Queue Boundary

State 3 calls vtable `+0x1E8` with `(mission=7, commence=0)`. Assembly at `0x0073EE8D..0x0073EE93` shows `PUSH 0`, `PUSH 7`, call `+0x1E8`; the handler then returns `1`.

`MissionClass::Queue_Mission @ 0x005B35E0` writes `+0xB4 = 7` and clears byte `+0xB8`, but its own `Commence` call is inside `if (commence != 0)`. Because the state-3 call passes `0`, it does not dispatch `Mission_Enter` synchronously. Active in YR: Yes.

### Unit AI Commence Boundary

`UnitClass::AI @ 0x007360C0` has two commence gates. The relevant one for a mission queued during `Mission_Dispatch` is the late gate after `FootClass::AI`: assembly context `0x00736461..0x00736473` calls vtable `+0x200`, tests the result, and calls vtable `+0x1EC` if true.

`MissionClass::Commence @ 0x005B3570` copies queued mission `+0xB4` into current mission `+0xAC`, clears `+0xB4` to `-1`, sets `+0xBC=0`, sets `+0xC8=g_CurrentFrameCounter`, clears `+0xD0=0`, and clears `+0xB8`. `UnitClass::ShouldIdle @ 0x00744270` explicitly allows queued mission `7` through the readiness gate (`if queued mission != 7` then extra locomotor checks), so the normal close-return queued enter mission is promoted in that same unit AI frame.

`MissionClass::Mission_Dispatch @ 0x005B3060` has already run for that unit frame when state 3 queues mission 7. There is no second dispatch loop after late `Commence`. The first `FootClass::Mission_Enter` handler call is on the next live `Mission_Dispatch` for that unit, normally the next global frame. Active in YR: Yes.

### First Mission Enter Admission

`MissionClass::Mission_Dispatch @ 0x005B3060` dispatches current mission `7` through vtable slot `+0x240`, which is `FootClass::Mission_Enter @ 0x004D9290`. At `0x004D92B4..0x004D92BF`, Mission Enter sends directed radio `0x0E` to the target and accepts reply `1`. If the reply is not `1`, byte `+0x418` can preserve the path; otherwise it sends `BREAK(0x03)` and clears the attempted entry path.

The accepted movement cell is produced by `BuildingClass::Receive_Radio @ 0x0043C2D0` case `0x0E`, not by state 2 and not by `QueueingCell`. For stock refineries, case `0x0E` sends `0x12` with `building_anchor+(3,1)`. Assembly context around `0x0043CAB3..0x0043CADB` shows `0x12`, then only if reply is `0x14` does it send `0x18` and `0x16`. Active in YR: Yes.

## 4. INI Keys

| INI key | Stock value | Effect | Active in YR |
|---|---:|---|---|
| `[General] ChronoHarvTooFarDistance` | `50` | CMIN close direct-radio threshold, cells converted by `*0x100` | Yes |
| `[General] HarvesterTooFarDistance` | `5` | Non-chrono harvester equivalent; not selected for CMIN | Yes for HARV |
| `[CMIN] Dock` | `NAREFN,GAREFN` | Dock list searched by state 2 | Yes |
| `[CMIN] Harvester` | `yes` | Enables harvest mission behavior | Yes |
| `[CMIN] Teleporter` | `yes` | Selects chrono threshold/fallback branch | Yes |
| `[GAREFN]/[NAREFN] DockUnload` | `yes` | Enables refinery dock admission and later unload handoff | Yes |
| `[GAREFN]/[NAREFN] Refinery` | `yes` | Stock refinery identity | Yes |
| `artmd.ini [GAREFN]/[NAREFN] QueueingCell` | `4,1` | Far/refused fallback seed only | Conditional |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | owns state 2 close/far branch and state 3 queue | decompile plus assembly contexts | Yes |
| `FootClass::Find_Docking_Bay @ 0x004DF040` | iterates `Dock=` vector and returns candidate refinery | decompile | Yes |
| `MissionClass::Queue_Mission @ 0x005B35E0` | writes queued mission when `commence=0` | decompile; `0x0073EE8D..0x0073EE93` caller | Yes |
| `MissionClass::Commence @ 0x005B3570` | promotes queued mission to current and clears dispatch duration | decompile | Yes |
| `UnitClass::AI @ 0x007360C0` | late commence gate after `FootClass::AI` | decompile plus assembly `0x00736461..0x00736473` | Yes |
| `UnitClass::ShouldIdle @ 0x00744270` | readiness gate; queued mission 7 bypasses extra movement block | decompile | Yes |
| `MissionClass::Mission_Dispatch @ 0x005B3060` | mission handler switch; mission 7 -> `+0x240` | decompile | Yes |
| `FootClass::Mission_Enter @ 0x004D9290` | first `CAN_DOCK(0x0E)` admission attempt | decompile plus assembly `0x004D92B4..0x004D92BF` | Yes |
| `BuildingClass::Receive_Radio @ 0x0043C2D0` | accepted `0x12` cell and `0x18/0x16` ordering | decompile plus assembly `0x0043CAB3..0x0043CADB` | Yes |

## 6. Current Rust Implementation Status

No Rust was edited. Current surfaces scanned:

| Rust surface | Current shape | Status vs this slice |
|---|---|---|
| `src/sim/miner/miner_system.rs` | `ReturnToRefinery` / `ForcedReturn` preserve `reserved_refinery`, choose close vs far, and use `chrono_harvester_too_far_distance` | Mostly aligned conceptually; binary uses 3D lepton object distance and mission timer epilogue |
| `src/sim/miner/miner_dock_sequence.rs` | `Approach -> MissionEnter -> AwaitingAcceptedCell` models HELLO then accepted cell | Good high-level phase split; must preserve native one-dispatch boundary after HELLO/state-3 queue |
| `src/sim/miner/miner_dock_sequence.rs` | `refinery_can_dock_queue_cell` returns anchor `+(3,1)` while `refinery_queue_cell` remains separate | Matches settled binary distinction |
| `src/sim/miner/miner_tests.rs` | tests already assert HELLO then next tick MissionEnter and accepted-cell recheck | Existing tests point in the right direction; exact timer/jitter not modeled |
| `src/sim/command.rs`, `src/sim/world/world_commands.rs` | `MinerReturn { target_refinery_id }` and `reserved_refinery` preserve clicked target | Matches manual order preservation requirement |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| State-2 selected refinery preservation | verified | manual order docs; state-2 dock object passed to radio `0x02` | none for explicit clicked/refinery object path |
| CMIN close/far threshold | verified | `0x0073EE35..0x0073EE4B`; `rulesmd.ini:294` | none |
| Close branch sends only `0x02` | verified | `0x0073EE54..0x0073EE68` | none |
| State-2 close branch return delay formula | verified | `0x0073EE72 -> 0x0073EF77`; `0x005B3A00`; `RandomRanged(0,2)` | runtime table value not read |
| Far/refused fallback `QueueingCell` destination | verified | `0x0073EEA6..0x0073EF41`; `artmd.ini:1716,1773` | full passable-cell search ordering covered by sibling report |
| State 3 `Queue_Mission(7,0)` | verified | `0x0073EE8D..0x0073EE93`; `0x005B35E0` | none |
| Queue helper no immediate dispatch with flag 0 | verified | `0x005B35E0` | none |
| Late UnitClass AI commence gate | verified | `0x007360C0`; assembly `0x00736461..0x00736473`; `0x005B3570` | none |
| First `Mission_Enter` dispatch boundary | verified | `0x005B3060`, `0x004D9290` | exact global frame assumes normal one UnitClass AI call per frame |
| First `CAN_DOCK` admission sequence | verified | `0x004D92B4..0x004D92BF`; `0x0043C2D0` | queue contention details out-of-scope |
| `QueueingCell` as accepted `CAN_DOCK` target | verified negative | no state-2 close or `0x0E` accepted-cell read; sibling reports | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does state 2 directly enter Mission_Enter? -> No; close branch sends `0x02`, writes substate 3 on reply 1, then returns through the harvest timer epilogue.` (evidence: `0x0073EE54..0x0073EF9C`)
- `[RESOLVED] OQ-2 - What threshold selects CMIN close return? -> Inclusive `distance <= ChronoHarvTooFarDistance * 0x100`, stock `50 * 256 = 12800` leptons.` (evidence: `0x0073EE35..0x0073EE4B`, `rulesmd.ini:294`)
- `[RESOLVED] OQ-3 - Is `QueueingCell` part of close admission? -> No; it is only read in the far/refused fallback destination branch.` (evidence: `0x0073EEC1..0x0073EED0`; no read before close `0x02`)
- `[RESOLVED] OQ-4 - What does state 3 queue? -> mission `7` with commence flag `0`.` (evidence: `0x0073EE8D..0x0073EE93`)
- `[RESOLVED] OQ-5 - Does `Queue_Mission(7,0)` call Commence immediately? -> No; `0x005B35E0` only calls `+0x1EC` when its third argument is nonzero.` (evidence: `0x005B35E0`)
- `[RESOLVED] OQ-6 - When is the queued mission promoted? -> normally late in the same `UnitClass::AI` frame by the post-`FootClass::AI` `+0x200`/`+0x1EC` gate.` (evidence: `0x007360C0`, `0x00736461..0x00736473`, `0x005B3570`)
- `[RESOLVED] OQ-7 - Can the newly current Mission Enter dispatch in the same mission-dispatch call? -> No; the dispatch switch has already returned from the mission 10 handler, and there is no second dispatch loop after late Commence.` (evidence: `0x005B3060`, `0x007360C0`)
- `[RESOLVED] OQ-8 - What is the first Mission_Enter admission message? -> directed radio `0x0E` to the contact/fallback target.` (evidence: `0x004D92B4..0x004D92BF`)
- `[RESOLVED] OQ-9 - What movement target does accepted refinery admission use? -> building anchor `+(3,1)` through `0x12`, not `QueueingCell=4,1`.` (evidence: `0x0043C2D0`; sibling accepted-anchor reports)
- `[RESOLVED] OQ-10 - Is manual partial-cargo clicked refinery preserved into this transition? -> Yes for stock object-click path: command stores the clicked refinery target, state-2 close branch radios the selected refinery object, and only generic/fallback selection may choose another refinery.` (evidence: `MANUAL_PARTIAL_CARGO_RETURN_ORDER_AND_VOICE_GHIDRA_REPORT.md`; `0x0073E5E0`)
- `[DEFERRED] OQ-11 - Exact numeric state-2-to-state-3 delay after the harvest epilogue` (category: `needs-runtime-debugger`; reason: formula is verified, but runtime mission timer table value and `RandomRanged(0,2)` draw were not read; next-step-if-pursued: run debugger at `0x0073EF77` / `0x005B3A00`)
- `[DEFERRED] OQ-12 - First rendered pixel movement after accepted `0x12` destination` (category: `needs-runtime-debugger`; reason: requires matched runtime position logging; next-step-if-pursued: break on `0x004D8FB0` case `0x12` and next locomotor update)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Close CMIN state 2 sends `HELLO(0x02)` to the selected refinery object and advances only to harvest substate 3; `CAN_DOCK` is not sent in state 2. | `0x0073EE54..0x0073EE68` | mostly addressed by current `Approach -> MissionEnter`, but exact timer/jitter is not modeled | `src/sim/miner/miner_system.rs`; `src/sim/miner/miner_dock_sequence.rs` | Preserve clicked/selected refinery, issue HELLO before accepted-cell movement, and keep a dispatch boundary before `CAN_DOCK`. | `cmin_close_return_hello_success_defers_can_dock_until_next_mission_enter_dispatch` | Do not collapse HELLO, `CAN_DOCK`, and accepted-cell movement into one return tick. |
| Far/refused close fallback uses `QueueingCell` plus nearby-passable search; accepted `CAN_DOCK` later uses anchor `+(3,1)`. | `0x0073EEC1..0x0073EF41`; `0x0043C2D0` | Rust has separate helpers and tests | `src/sim/miner/miner_system.rs`; `src/sim/miner/miner_dock_sequence.rs` | Keep staging/waiting cell distinct from accepted cell even when both are adjacent. | `cmin_refused_close_return_stages_at_queueingcell_but_can_dock_moves_to_anchor_3_1` | Do not use `QueueingCell=4,1` as the stock accepted dock target. |
| `Queue_Mission(7,0)` writes queued mission, late UnitClass AI promotes it same unit frame, and first Mission Enter handler dispatch is the next mission-dispatch pass. | `0x0073EE8D..0x0073EE93`; `0x005B35E0`; `0x005B3570`; `0x00736461..0x00736473`; `0x005B3060` | Rust has once-per-miner-tick phase progression, which can represent this boundary if phases do not recurse | `src/sim/miner/miner_dock_sequence.rs`; miner tick scheduler | Model one handler-dispatch boundary: after successful HELLO/state-3 queue, MissionEnter logic should not run until the next miner tick/dispatch. | `cmin_state3_queue_promotes_enter_but_first_can_dock_is_next_tick` | Do not call MissionEnter logic recursively from the same phase that records HELLO acceptance. |

## 10. Negative Facts / Do Not Do

- Do not redispatch `Mission_Enter` synchronously from `Queue_Mission(7,0)`; the commence flag is zero and the dispatch switch has already run. Evidence: `0x005B35E0`, `0x005B3060`.
- Do not treat state-2 close acceptance as `CAN_DOCK`; it is radio `0x02` only. Evidence: `0x0073EE54..0x0073EE68`.
- Do not use `QueueingCell` for the accepted `0x12` target; it is fallback/waiting staging only. Evidence: `0x0073EEC1..0x0073EED0`, `0x0043C2D0`.
- Do not require full cargo for manual clicked-refinery return before entering this flow; prior binary evidence shows object-click mission `7` preserves clicked refinery and does not check full cargo.
- Do not hardcode CMIN threshold as 2 cells or accepted-cell distance; stock close threshold is `ChronoHarvTooFarDistance=50` cells in lepton-space object distance.

## 11. Remaining Uncertainty

- Exact numeric state-2 close-success delay to the state-3 queue depends on the runtime MissionControl rate for current mission 10 plus `RandomRanged(0,2)`. Static formula is verified; live table value was not read because the debugger server was unavailable.
- Exact first rendered locomotor pixel displacement after `0x12` needs runtime coordinate logging.

## 12. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/CHRONO_MINER_FULL_CARGO_CLOSE_RETURN_MISSION_DISPATCH_TIMING_TRACE.md`: replace the stale DQ-1/DQ-2 wording with: "Resolved by `CHRONO_MINER_CLOSE_RETURN_SCHEDULER_FRAME_TRACE.md` and `CMIN_STATE2_CLOSE_FAR_RETURN_TO_MISSION_ENTER_DISPATCH_GHIDRA_REPORT.md`: `Queue_Mission(...,0)` is promoted by the late `UnitClass::AI` commence gate in the same unit AI frame, but the promoted mission's handler first dispatches on the next mission-dispatch pass, normally the next frame."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`: replace "state 3 queues mission 7" with "state 3 calls `Queue_Mission(7,0)`; this queues mission `7`, late `UnitClass::AI` normally promotes it in the same unit AI frame, and first `Mission_Enter` dispatch is the next mission-dispatch pass."
- `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_MANUAL_ORDER_PARTIAL_CARGO_TO_REFINERY_TRACE.md`: retain the prior replacement from `MANUAL_PARTIAL_CARGO_RETURN_ORDER_AND_VOICE_GHIDRA_REPORT.md`: stock owned available refinery resolves to action `3`, mission `7`, `VoiceEnter`, and target-preserving Mission Enter; do not use action `0x1A` for that stock path.

## Sources

- Ghidra read-only decompile: `UnitClass::Mission_Harvest @ 0x0073E5E0`; `FootClass::Find_Docking_Bay @ 0x004DF040`; `BuildingClass::Receive_Radio @ 0x0043C2D0`; `FootClass::Mission_Enter @ 0x004D9290`; `MissionClass::Mission_Dispatch @ 0x005B3060`; `MissionClass::Queue_Mission @ 0x005B35E0`; `MissionClass::Commence @ 0x005B3570`; `UnitClass::AI @ 0x007360C0`; `UnitClass::ShouldIdle @ 0x00744270`; `MissionClass::GetMissionTimerEntry @ 0x005B3A00`.
- Ghidra read-only assembly contexts: `0x0073EE54..0x0073EE68`, `0x0073EE8D..0x0073EE93`, `0x0073EF77..0x0073EF95`, `0x004D92B4..0x004D92BF`, `0x0043CAB3..0x0043CADB`, `0x00736461..0x00736473`.
- Existing reports: `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_MISSION_HARVEST_STATE2_RETURN_BRANCH_COORDS_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/miner/CHRONO_MINER_FAR_RETURN_FALLBACK_DESTINATION_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_HARVEST_STATE2_CLOSE_RETURN_RADIO_TIMING_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/CHRONO_MINER_CLOSE_RETURN_SCHEDULER_FRAME_TRACE.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MANUAL_PARTIAL_CARGO_RETURN_ORDER_AND_VOICE_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/UNIT_MISSION_ENTER_REFINERY_RETRY_QUEUE_LOOP_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/art.ini`.
- Rust scan only: `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/mod.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_system.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_dock_sequence.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/miner/miner_tests.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/sim/command.rs`.
