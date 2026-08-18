# Drive Blocked-Delay Expiry and Miner Retarget - Ghidra Research Report

**Address(es):** `0x004B2630`, `0x004D94B0`, `0x0073E5E0`, `0x004DCFE0`, `0x0073D450`, `0x005B2FD0`, `0x005B35E0`, `0x005B3570`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** DriveLocomotion blocked/no-path patience expiry and the immediate Mission_Harvest consequence when a stock YR Chrono Miner has its destination cleared while en route to ore/refinery.  
**Non-Scope:** full A*, ore scan ranking, full chrono teleport state machine, full refinery dock radio/undock flow, runtime replay capture, and all DriveLocomotion branches unrelated to code-2 blocked movement or give-up destination clear.  
**Confidence:** High for timer field, expiry effect, destination-clear side effects, and harvest-state consequence; Medium for exact in-match frequency on CMIN because no runtime debugger/replay observation was performed.  
**Active in YR:** Conditional. The Drive code is live in YR and CMIN is a stock `Harvester=yes` unit with Teleport locomotor; the Drive blocked branch is active when the Chrono Miner is using/piggybacking DriveLocomotion for ground approach.

## Working Notes

**Target question:** How does DriveLocomotion blocked/no-path delay expire, and what does the alleged `SetMission(None)`/mission reset do for a Chrono Miner stuck en route to ore/refinery?

**Non-goals:** Do not redo all A* or ore scan; do not revisit stock zero-link unload, dock anchor, interrupted unload cargo, or ReleaseDockedHarvester/Force_Track paths.

**Evidence needed to mark COMPLETE:** binary proof for the timer fields; binary proof for the expiry branch effect; binary proof for `Drive+0x62` or related flag semantics in this slice; binary proof for whether a mission reset call is made and with which args; binary proof for the Mission_Harvest consequence after destination clear; current Rust surfaces for handoff.

**Stop conditions:** Stop once `Process_Movement` code-2 expiry, destination-clear failure branch, mission assignment/queue helpers, `Set_Destination_Internal`, `Mission_Harvest` state 0/state 1 consequence, and current Rust surfaces are covered with no unresolved in-scope open questions.

## 1. Overview

The blocked-delay timer does not expire into `SetMission(None)`. In the live DriveLocomotion code-2 branch, expiry only changes the next `FootClass::Find_Path` urgency argument from `1` to `2`.

The mission-relevant action happens later only on no-path/give-up branches: Drive calls owner vtable `+0x480(0, 1)`, which resolves to destination clearing (`TechnoClass::Set_Destination` / `FootClass::Set_Destination_Internal`), not mission assignment. That clears `FootClass+0x5A4`, clears `Foot+0x6B7`, resets the blocked and movement timers, and leaves the current mission/substate unchanged.

For a miner already in `Mission_Harvest` state 1, the next harvest tick no longer takes the "destination exists" success shortcut in `UnitClass::Harvest_Ore_Tick`. If the miner is not on ore, state 1 falls into the short-retarget branch: `TiberiumShortScan` with zone argument `0`; success keeps substate `1`, miss plus no destination changes to substate `2` (return/refinery path).

## 2. Class Layout / Key Offsets

| Offset | Owner | Purpose in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `Drive+0x34/0x38/0x3C` | Drive loco | destination coord used to build Find_Path target cell | `0x004B39EA..0x004B3A0E` | Yes, when Drive active |
| `Drive+0x40/0x44/0x48` | Drive loco | head-to/intermediate coord, cleared on stop/give-up paths | clears around `0x004B31E4`, `0x004B3607`, `0x004B4561` | Yes |
| `Drive+0x62` | Drive loco | constructor-zeroed byte; no blocked-delay use found in scoped branch | `0x004AF5B2` writes `0` | Conditional/unknown outside this slice |
| `Drive+0x63` | Drive loco | head-to valid flag; cleared when head-to coord is cleared | `0x004B3642`, `0x004B31F8`, `0x004B3869` | Yes |
| `Foot+0x5A4` | owner Foot/Techno | destination/NavCom pointer; zero means no current destination | `0x004D94B0` writes `param_1[0x169]`; `0x0073E8C3`, `0x0073EAE4` read it | Yes |
| `Foot+0x640/+0x644/+0x648` | owner Foot | movement-delay timer (`PathDelay`) gating Find_Path calls | `0x004B3690..0x004B36B6`; reset at `0x004D96F0..0x004D9707` | Yes |
| `Foot+0x668/+0x66C/+0x670` | owner Foot | blocked-delay timer (`BlockagePathDelay`) | set `0x004B3663..0x004B368D`; checked `0x004B36BC..0x004B36ED`; reset `0x004D96C2..0x004D96ED` | Yes |
| `Foot+0x6B7` | owner Foot | blocked-by-moving-friendly flag | set `0x004B3663`, tested `0x004B36BC`, cleared `0x004D96C2` | Yes |
| `Mission+0xAC` | MissionClass | current mission id | mission dispatch `0x005B3060`; assign `0x005B2FD0` | Yes |
| `Mission+0xB4` | MissionClass | queued mission id | queue `0x005B35E0`; commence `0x005B3570` | Yes |
| `Mission+0xBC` | MissionClass / UnitClass | mission substate; `Mission_Harvest` uses 0/1/2/... | state 0/1 writes in `0x0073E5E0` | Yes |

## 3. Core Logic

### 3.1 Code-2 block starts the blocked timer

When `Can_Enter_Cell` result equals `2`, `DriveLocomotionClass::Process_Movement @ 0x004B2630` enters the moving-friendly blocked branch.

Verified order:

1. `0x004B364D` compares result to `2`; non-2 jumps away.
2. `0x004B3659..0x004B3661` checks `Foot+0x6B7`.
3. First blocked tick writes `Foot+0x6B7 = 1` at `0x004B3663`.
4. `0x004B367E` reads `RulesClass+0x1768` (`BlockagePathDelay`).
5. `0x004B3684/0x004B368A/0x004B368D` write start frame, snapshot, and duration to `Foot+0x668/+0x66C/+0x670`.

**Active in YR:** Yes. This is inside the live Drive ILocomotion process; `[General] BlockagePathDelay=60` is stock YR.

### 3.2 Expiry changes only Find_Path urgency

After movement-delay allows a pathfinder call:

1. `0x004B36BC` reads `Foot+0x6B7`.
2. `0x004B36CA..0x004B36E7` checks `Foot+0x668/+0x670`.
3. If still active, execution goes through `0x004B39D1` with `BL = 0`.
4. If expired, `0x004B36ED` sets `BL = 1`.
5. `0x004B39FB..0x004B3A00` computes `urgency = (BL != 0) + 1`.
6. `0x004B3A0E` calls `FootClass::Find_Path @ 0x004D3920` with urgency `1` before expiry and `2` after expiry.

No mission field write, `Queue_Mission`, `Assign_Mission`, `Commence`, or `Mission_Dispatch` call exists on this expiry path.

**Active in YR:** Yes. Same live Drive path; urgency effects are corroborated by existing A* cost reports.

### 3.3 No-path/give-up clears destination, not mission

After `Find_Path` returns false:

1. `0x004B3A2F..0x004B3A3C` calls owner vtable `+0x2CC`; if nonzero, Drive resets movement delay and returns.
2. If owner cannot still move, `0x004B3A43/45/47` calls owner vtable `+0x480(0, 1)`.
3. The call returns `0`; Drive does not call `+0x1E8`, `+0x1EC`, or `+0x1F0` on this path.

The same destination-clear pattern appears in adjacent no-valid-next-cell branches (`0x004B3213`, `0x004B3880`, `0x004B3A47`, `0x004B44xx`) unless `Foot+0x598`/waypoint-like state sends execution to vtable `+0x484`.

**Active in YR:** Yes for destination clear; `+0x484` is conditional and not the normal code-2 timer-expiry action.

### 3.4 `+0x480(0,1)` resolves to destination clear

`TechnoClass::Set_Destination @ 0x00741970` eventually calls `FootClass::Set_Destination_Internal @ 0x004D94B0`.

For `param_2 == 0`, `FootClass::Set_Destination_Internal`:

1. writes `Foot+0x5A4 = 0`;
2. may call active locomotor vfunc `+0x48` during clear handling;
3. writes `Foot+0x6B7 = 0` at `0x004D96C2`;
4. resets the blocked timer from `RulesClass+0x1768` at `0x004D96D4..0x004D96ED`;
5. resets the movement-delay timer triplet at `0x004D96F0..0x004D9707`;
6. does not write `Mission+0xAC`, `Mission+0xB4`, or `Mission+0xBC`.

**Active in YR:** Yes. This is the live destination setter for Foot/Techno objects.

### 3.5 What actual mission helpers would do

These helpers were checked to distinguish destination clear from mission reset:

- `MissionClass::Assign_Mission @ 0x005B2FD0` directly writes current mission `+0xAC = mission`, queued mission `+0xB4 = -1`, queued flag `+0xB8 = 0`, substate `+0xBC = 0`, and timer triplets.
- `MissionClass::Queue_Mission @ 0x005B35E0` writes queued mission `+0xB4 = mission` unless `mission == -1` or it is already current/queued; optional `param_3 != 0` can call `Commence`.
- `MissionClass::Commence @ 0x005B3570` copies queued mission `+0xB4` to current `+0xAC`, clears queue to `-1`, resets substate `+0xBC = 0`, and resets timer triplets.

None of those calls are made by blocked-delay expiry or the primary code-2 no-path clear branch.

**Active in YR:** Yes as generic mission infrastructure; not called by the scoped timer-expiry path.

### 3.6 Miner consequence after destination clear

`Mission_Harvest` state 0 first selects ore and calls `Search_For_Tiberium_And_Move`; on success it writes substate `1` and returns `1` (`0x0073E879..0x0073E8C2`). Therefore a miner already en route to ore is normally in harvest substate `1`, not state `0`.

In state 1:

1. If `UnitClass+0xF8 < 9`, it returns `1` without extraction (`0x0073E96F..0x0073E980`).
2. `UnitClass::Harvest_Ore_Tick @ 0x0073D450` returns success immediately if `Foot+0x5A4 != 0`.
3. If Drive cleared destination, that shortcut is gone. If the unit is not on ore (`CellClass+0xEC != 5`) or is full/not harvester, it resets the step timer and returns false.
4. Mission state 1 clears `Unit+0x6D2 = 0` at `0x0073E99A`.
5. For harvesters, it calls `Search_For_Tiberium_And_Move` with `RulesClass+0x1778` (`TiberiumShortScan`) and zone arg `0` at `0x0073EAA6..0x0073EAB9`.
6. If scan succeeds, or if destination is nonzero after the call (`0x0073EAE4..0x0073EAEC`), it writes substate `1` and `+0x6D2 = 1` at `0x0073EB0F/0x0073EB19`.
7. If scan misses and destination is still zero, it clears ghost/archive state and writes substate `2` at `0x0073EAF8`.

**Active in YR:** Yes. CMIN and HARV both have `Harvester=yes`; CMIN's Teleport locomotor is stock, and this state machine is UnitClass mission 10.

### 3.7 Refineries / en route to refinery

This report did not re-open the full state-2/refinery dock pipeline. The relevant scoped fact is the same: Drive's blocked-delay expiry does not assign a mission. If a no-path branch clears destination while the miner is in a refinery/return mission substate, the next `Mission_Harvest`/Mission_Enter tick observes the current mission/substate plus `Foot+0x5A4 == 0`; the mission-specific retry behavior belongs to the existing state-2/Mission_Enter dock reports.

**Active in YR:** Conditional; active when the miner is in the corresponding return/dock substate.

## 4. INI Keys

| Key | Section | Stock YR value | Binary field | Effect | Active in YR |
|---|---|---:|---|---|---|
| `PathDelay` | `[General]` / rules comments | `.01` | `RulesClass+0x1760` in prior docs | movement-delay rate limit before another Find_Path attempt | Yes |
| `BlockagePathDelay` | `[General]` | `60` | `RulesClass+0x1768` | duration copied to `Foot+0x670`; expiry changes urgency to 2 | Yes |
| `TiberiumShortScan` | `[General]` | `6` | `RulesClass+0x1778` | state-1 retarget radius after destination-clear/no ore tick | Yes |
| `TiberiumLongScan` | `[General]` | `48` | `RulesClass+0x177C` | state-0 initial ore scan, not the post-clear state-1 retarget | Yes |
| `Harvester` | `[CMIN]` | `yes` | `UnitType+0xE0E` | enables Mission_Harvest miner path | Yes |
| `Locomotor` | `[CMIN]` | Teleport CLSID | Type data | Drive branch applies when piggyback/active Drive is used | Conditional |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `DriveLocomotionClass::Process_Movement @ 0x004B2630` | owns blocked-delay, urgency escalation, and no-path destination clear | decompile + assembly contexts | Yes |
| `FootClass::Find_Path @ 0x004D3920` | receives urgency 1/2 | call at `0x004B3A0E` | Yes |
| `TechnoClass::Set_Destination @ 0x00741970` | resolves vtable `+0x480` | decompile | Yes |
| `FootClass::Set_Destination_Internal @ 0x004D94B0` | clears `+0x5A4`, resets `+0x6B7`, timers | decompile + assembly | Yes |
| `MissionClass::Assign_Mission @ 0x005B2FD0` | what direct mission reset would look like | decompile | Yes, but not called here |
| `MissionClass::Queue_Mission @ 0x005B35E0` | queued mission helper | decompile | Yes, but not called here |
| `MissionClass::Commence @ 0x005B3570` | queued mission -> current mission/substate reset | decompile | Yes, but not called here |
| `UnitClass::Mission_Harvest @ 0x0073E5E0` | miner mission consequence after destination clear | assembly contexts | Yes |
| `UnitClass::Harvest_Ore_Tick @ 0x0073D450` | destination-present shortcut in state 1 | decompile | Yes |
| `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0` | scan wrapper; no scan if destination exists at entry | decompile | Yes |

## 6. Current Rust Implementation Status

Current Rust surfaces scanned:

- `src/sim/components.rs:196-288` has `MovementTarget.blocked_delay`, `path_blocked`, `path_stuck_counter`, and comments mapping them to `Foot+0x668`, `Foot+0x6B7`, and `Foot+0x64C`.
- `src/sim/movement/movement_blocked.rs:1-181` models urgency `1` while `blocked_delay > 0` and urgency `2` after expiry. It also decrements `path_stuck_counter` on urgency-2 failures and finishes the entity when the counter reaches zero.
- `src/sim/movement/movement_tick.rs:1098-1135` handles finished movement by clearing `movement_target` and setting locomotor phase idle; it does not reset miner mission/substate.
- `src/sim/miner/miner_system.rs:382-489` currently performs per-tick ore rescan while in `MoveToOre`, can clear movement on target change, and reissues movement when no movement target exists.

Current Rust is closer to the corrected model than the older stuck-watchdog trace assumed because `MoveToOre` now has a per-tick rescan block. Remaining delta: Rust's stuck abort is represented as `movement_target = None` and miner logic rescan/reissue, while the binary's normal no-path branch clears `Foot+0x5A4` without assigning a mission; the subsequent miner behavior depends on the current harvest substate, especially state 1 short-retarget vs miss-to-state-2 behavior.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Drive code-2 blocked-delay start | verified | `0x004B364D..0x004B368D` | none |
| Drive blocked-delay expiry | verified | `0x004B36BC..0x004B36ED`, `0x004B39FB..0x004B3A0E` | none |
| Direct `SetMission(None)` on expiry | verified-absent | no mission helper/write on expiry path; decompile/assembly above | none for scoped branch |
| No-path/give-up after failed Find_Path | verified | `0x004B3A2F..0x004B3A50` | owner `+0x2CC` full semantics outside scope |
| `+0x480(0,1)` side effects | verified | `0x004D94B0`, `0x004D96C2..0x004D9707` | none |
| `Drive+0x62` blocked-delay hypothesis | touched-not-exhausted/refuted-for-slice | constructor write `0x004AF5B2`; no scoped blocked-branch use found | exact meaning outside this slice |
| Mission helper semantics | verified | `0x005B2FD0`, `0x005B35E0`, `0x005B3570` | none |
| Mission_Harvest state-0 transition to state 1 | verified | `0x0073E879..0x0073E8C2` | full state-0 scan details delegated to existing report |
| Harvest state-1 consequence after destination clear | verified | `0x0073D450`, `0x0073E96F..0x0073EB19` | none for ore-en-route consequence |
| Return/refinery substate after destination clear | touched-not-exhausted | same destination-clear fact applies | full dock retry belongs to state-2/Mission_Enter docs |
| Current Rust surfaces | verified | files/lines in section 6 | implementation work separate |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the blocked-delay timer at Techno+0x388? -> No for this slice; +0x388 is facing/rate-timer territory in Drive reports, while blocked delay uses Foot+0x668/+0x66C/+0x670.` (evidence: `0x004B3678..0x004B368D`, prior Drive report)
- `[RESOLVED] OQ-02 - What starts the blocked-delay? -> Can_Enter_Cell code 2 with Foot+0x6B7 previously clear.` (evidence: `0x004B364D..0x004B3663`)
- `[RESOLVED] OQ-03 - What expires the blocked-delay? -> Current frame elapsed check against Foot+0x668/+0x670.` (evidence: `0x004B36CA..0x004B36ED`)
- `[RESOLVED] OQ-04 - What is the expiry side effect? -> BL becomes 1, and Find_Path urgency becomes 2.` (evidence: `0x004B36ED`, `0x004B39FB..0x004B3A0E`)
- `[RESOLVED] OQ-05 - Does expiry call SetMission(None)? -> No direct mission call/write on the expiry path.` (evidence: `0x004B36ED..0x004B3A0E`)
- `[RESOLVED] OQ-06 - What happens after urgency-2 Find_Path failure? -> If owner +0x2CC says it cannot still move, Drive calls owner +0x480(0,1) and returns 0.` (evidence: `0x004B3A2F..0x004B3A50`)
- `[RESOLVED] OQ-07 - What does +0x480(0,1) do here? -> Clears destination/NavCom through Set_Destination_Internal and resets blocked/movement timers; it does not reset mission state.` (evidence: `0x004D94B0`, `0x004D96C2..0x004D9707`)
- `[RESOLVED] OQ-08 - What would real mission assignment reset? -> Assign_Mission writes current mission and substate 0; Queue/Commence use +0xB4 and may later reset substate. Those helpers are not called by the scoped Drive branch.` (evidence: `0x005B2FD0`, `0x005B35E0`, `0x005B3570`)
- `[RESOLVED] OQ-09 - Is a miner en route to ore still in state 0? -> After successful state-0 ore issue, Mission_Harvest writes substate 1 and returns 1.` (evidence: `0x0073E879..0x0073E8C2`)
- `[RESOLVED] OQ-10 - Why does destination clear matter in state 1? -> Harvest_Ore_Tick returns success immediately while +0x5A4 is nonzero; if cleared and not on ore, it returns false.` (evidence: `0x0073D450`)
- `[RESOLVED] OQ-11 - What retarget does state 1 perform after false extraction? -> TiberiumShortScan with zone arg 0; success keeps substate 1/+0x6D2, miss with no destination writes substate 2.` (evidence: `0x0073EAA6..0x0073EB19`)
- `[RESOLVED] OQ-12 - Is this active for stock CMIN? -> Conditional: CMIN is Harvester=yes and Teleport locomotor; Drive branch is active when Drive piggyback/ground approach is active.` (evidence: `rulesmd.ini [CMIN]`, Drive/chrono reports)
- `[RESOLVED] OQ-13 - Does current Rust have blocked_delay and path_blocked fields? -> Yes.` (evidence: `src/sim/components.rs:196-288`)
- `[RESOLVED] OQ-14 - Does current Rust reset miner mission when movement finishes/stuck-aborts? -> No; movement finalization clears MovementTarget/drive track and sets locomotor idle.` (evidence: `src/sim/movement/movement_tick.rs:1098-1135`)
- `[RESOLVED] OQ-15 - Does current Rust MoveToOre still never rescan? -> No; current file has a per-tick rescan block in MoveToOre.` (evidence: `src/sim/miner/miner_system.rs:419-444`)
- `[DEFERRED] OQ-16 - Exact runtime frequency for CMIN Drive piggyback blocked while en route to each ore/refinery scenario.` (category: needs-runtime-debugger; reason: static code proves branch behavior but not observed frequency; next-step-if-pursued: record retail replay/debugger trace with CMIN blocked by moving-friendly vs permanent local obstruction)
- `[DEFERRED] OQ-17 - Full state-2/refinery retry consequence after destination clear.` (category: out-of-scope; reason: this target asked blocked-delay expiry and miner retarget, not full dock retry; next-step-if-pursued: use existing Mission_Harvest state-2 and Mission_Enter dock reports)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Blocked-delay expiry changes path urgency from 1 to 2; it does not reset mission. | `0x004B36ED`, `0x004B39FB..0x004B3A0E` | mostly matches: Rust uses urgency 1 while `blocked_delay > 0`, urgency 2 after expiry | `src/sim/movement/movement_blocked.rs` | Keep blocked-delay as movement-layer urgency escalation, not a miner mission reset. | `test_blocked_drive_delay_expiry_uses_urgency_2_without_mission_reset`: after `BlockagePathDelay`, a blocked mover retries with urgency 2 and miner state is not forcibly reset to SearchOre/None by timer expiry alone. | Do not implement `SetMission(None)` on blocked-delay expiry. |
| Drive no-path/give-up clears destination via `+0x480(0,1)` and resets blocked/movement timers, not current mission/substate. | `0x004B3A43..0x004B3A50`; `0x004D96C2..0x004D9707` | partial/mismatch risk: Rust stuck abort clears `MovementTarget`; miner behavior is then governed by Rust state rather than stock destination-clear semantics | `src/sim/movement/movement_tick.rs`; `src/sim/miner/miner_system.rs` | Model stuck abort as destination-clear signal to miner logic, while preserving current miner mission/substate until the miner tick processes the cleared destination. | `test_blocked_drive_delay_expiry_restarts_miner_mission`: CMIN blocked en route to ore clears movement destination; next miner tick follows stock state-specific retarget/miss consequence rather than blindly reissuing same movement. | Do not treat movement_target removal alone as equivalent to mission assignment or full harvest FSM reset. |
| Miner en route to ore is in Mission_Harvest substate 1 after initial state-0 search; after destination clear, state 1 performs short-retarget (`TiberiumShortScan`, zone 0), or goes to substate 2 on miss/no destination. | `0x0073E879..0x0073E8C2`; `0x0073D450`; `0x0073EAA6..0x0073EB19` | current Rust has a separate `MoveToOre` state with long-radius per-tick rescan; may diverge from state-1 short-retarget-after-clear behavior | `src/sim/miner/miner_system.rs:382-489`; miner config scan radii | On a destination-clear/stuck-abort while already committed to ore, use stock-equivalent state-specific handling: destination-present shortcut gone, then short retarget if appropriate; do not assume state 0 long scan. | `test_chrono_miner_destination_clear_in_state1_short_retargets_or_returns`: blocked CMIN en route to ore with nearby short-scan ore retargets and remains harvest-continuation; with no short-scan ore and no destination, transitions toward return/refinery behavior. | Do not use TiberiumLongScan as the automatic post-clear state-1 recovery scan unless the miner is actually in state 0. |

## 10. Negative Facts / Do Not Do

- Do not say `Techno+0x388` is the blocked-delay timer for this branch; the verified blocked-delay timer is `Foot+0x668/+0x66C/+0x670`.
- Do not give `Drive+0x62` blocked-delay semantics from this evidence; it is constructor-zeroed, but no scoped blocked-delay consumer was found.
- Do not call `SetMission(None)` on blocked-delay expiry; the binary expiry path only changes `Find_Path` urgency.
- Do not conflate `+0x480(0,1)` with mission reset; in this context it clears destination/NavCom and resets movement timers.
- Do not assume a miner en route to ore is still in Mission_Harvest state 0; successful state-0 search writes substate 1.
- Do not always use long-scan retarget after a destination clear; state-1 recovery uses `TiberiumShortScan` with zone arg `0`.
- Do not reintroduce the stale claim that Rust `MoveToOre` never rescans; current active code does rescan per tick.

## 11. Remaining Uncertainty

- Runtime frequency for stock CMIN hitting the Drive code-2 branch during real ore/refinery approach remains unmeasured; static evidence proves behavior when the branch is active.
- Full return/refinery retry after destination clear was touched only at the boundary. Use the state-2 and Mission_Enter dock reports before changing refinery-retarget logic.
- `Drive+0x62` may have semantics outside the scoped branch; this report only refutes it as the blocked-delay state in the inspected blocked/no-path path.
- The exact `+0x2CC` "can still move" predicate was not decompiled here; it is a branch guard before the destination-clear call and can affect how often give-up happens.

## 12. Stale Docs / Follow-up Docs

- `miner/traces/MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md` W10 replacement wording: "DriveLocomotion code-2 `BlockagePathDelay` expiry does not call `SetMission(None)`. Expiry changes `Find_Path` urgency from 1 to 2. Only subsequent no-path/give-up branches call owner `+0x480(0,1)`, which clears destination/NavCom and resets movement timers; Mission_Harvest then observes the cleared destination on its next mission tick."
- Same trace W1/W6 wording should not state unqualified that state 0 re-scans every tick while driving to ore. Replacement: "State 0 long-scans and, on success, writes Mission_Harvest substate 1. While in state 1, an existing destination makes `Harvest_Ore_Tick` return success; if Drive clears the destination before arrival, state 1 falls into the short-retarget/miss branch."
- `DRIVELOCOMOTION_BLOCKED_DELAY_TIMER_CHRONO_MINER_GHIDRA_REPORT.md` remains directionally correct on the timer and no direct mission reset. Add the miner-state consequence above if extending it: destination clear leaves mission/substate intact, so state-specific miner logic, not a direct mission reset, drives retarget.
- Older dock docs that call `+0x480` "SetMission" should replace that wording with "`+0x480` destination setter/clear; mission queue/assign slots are `+0x1E8/+0x1F0` depending on class/vtable context."

## Sources

- Ghidra read-only decompile / assembly contexts:
  - `DriveLocomotionClass::Process_Movement @ 0x004B2630`
  - `FootClass::Set_Destination_Internal @ 0x004D94B0`
  - `TechnoClass::Set_Destination @ 0x00741970`
  - `UnitClass::Mission_Harvest @ 0x0073E5E0`
  - `UnitClass::Harvest_Ore_Tick @ 0x0073D450`
  - `FootClass::Search_For_Tiberium_And_Move @ 0x004DCFE0`
  - `MissionClass::Assign_Mission @ 0x005B2FD0`
  - `MissionClass::Queue_Mission @ 0x005B35E0`
  - `MissionClass::Commence @ 0x005B3570`
  - `MissionClass::Mission_Dispatch @ 0x005B3060`
- Prior docs:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/traces/MINER_STUCK_WATCHDOG_RETARGET_ON_UNREACHABLE_TRACE.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/DRIVELOCOMOTION_BLOCKED_DELAY_TIMER_CHRONO_MINER_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/MISSION_HARVEST_STATE0_SEEK_TIBERIUMSHORTSCAN_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/miner/HARV_HARVEST_STATE_RETARGET_VISUAL_FLAG_GHIDRA_REPORT.md`
- INI:
  - `ini/rulesmd.ini` `PathDelay=.01`, `BlockagePathDelay=60`, `[CMIN] Harvester=yes`, `[CMIN] Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}`
- Rust scanned:
  - `src/sim/components.rs`
  - `src/sim/movement/movement_blocked.rs`
  - `src/sim/movement/movement_tick.rs`
  - `src/sim/miner/miner_system.rs`
