# DriveLocomotion Blocked Delay Timer for Chrono Miner Ore Approach - Ghidra Research Report

**Address(es):** `0x004B0500` (`DriveLocomotionClass::Process`), `0x004B2630` (`DriveLocomotionClass::Process_Movement`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** only the blocked/no-path branches relevant to a chrono miner while a `DriveLocomotionClass` is active/piggybacked for ore/dock approach.  
**Non-Scope:** full `Process_Drive_Track`, full chrono teleport state machine, full `UnitClass::Mission_Harvest`, full `UnitClass::Can_Enter_Cell` taxonomy.  
**Confidence:** High for the timer/branch mechanics; Medium for chrono-miner-specific runtime frequency because no live retail replay/debugger run was performed.  
**Active in YR:** Yes. Evidence: `CMIN` has `Harvester=yes` and `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` in `ini/rulesmd.ini:7351-7398`; chrono miner uses drive locomotion through the verified piggyback path when the active locomotor is Drive (`CHRONO_MINER_SYSTEM_OVERVIEW.md:32-59`, `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md:1744-1759`). `DriveLocomotionClass::Process` is the Drive ILocomotion tick (`ADDRESS_MAP.md:724`, vtable `0x007E7EB0`).

## 1. Overview

The blocked-delay path does not directly re-run the harvest state machine. It is a movement-layer patience timer: when `Can_Enter_Cell` returns code `2` for a temporary moving-friendly block, `Process_Movement` sets `FootClass+0x6B7`, starts `FootClass+0x668/+0x66C/+0x670`, and later changes the `FootClass::Find_Path` urgency argument from `1` to `2` after the timer expires.

The timer is not at owner `+0x388`. In this slice, owner `+0x388` is reached as the body/facing `RateTimer` object used by `RateTimer::Current` at `0x004B3410`; the blocked-delay timer is on the owner FootClass at `+0x668/+0x66C/+0x670`.

## 2. Class Layout / Key Offsets

| Offset | Owner | Purpose in this slice | Evidence | Active in YR |
|---|---|---|---|---|
| `Drive+0x34/0x38/0x3C` | Drive loco | destination coord | `Process_Movement` reads `param_1+0x34` before pathfinding; `Set_Destination` starts at `0x004AFD40` | Yes - Drive movement path |
| `Drive+0x40/0x44/0x48` | Drive loco | head-to/intermediate coord; cleared to NullCoord on stop/give-up branches | clears at `0x004B31E4-0x004B31F8`, `0x004B3607-0x004B3649`, `0x004B4561+` | Yes |
| `Drive+0x63` | Drive loco | head-to valid/equivalent flag in this slice; cleared when head-to is reset | `MOV byte ptr [EBP+0x63],0` at `0x004B31F8`, `0x004B3642`, `0x004B3869` | Yes |
| `Drive+0x62` | Drive loco | initialized to 0; no material blocked-delay use found in scoped branches | constructor `0x004AF5B2` writes zero | Conditional/unknown - initialized, but not used by this slice |
| `Drive+0x64` | Drive loco | terrain/building overlay routing flag, not blocked-delay timer | set from `bVar4` in `Process_Movement` around path/overlay handling; constructor `0x004AF5B8` zeroes it | Yes, but not chrono-specific |
| `Foot+0x388` | owner Foot/Techno | body facing `RateTimer` base in this slice, not blocked delay | `0x004B3410: ADD ECX,0x388` followed by `RateTimer::Current @ 0x004C93D0` | Yes |
| `Foot+0x5A4` | owner Foot | NavCom/destination target pointer; `0` clears destination | `FootClass::Set_Destination_Internal @ 0x004D94B0` writes `param_1[0x169]`; `FootClass::Stop_Moving @ 0x004DF0D0` clears `+0x5A0/+0x5A4` | Yes |
| `Foot+0x5E0` | owner Foot | path queue head | empty check at `0x004B2630` start; cleared at `0x004B41BC`, `0x004B4521` | Yes |
| `Foot+0x640/+0x644/+0x648` | owner Foot | movement-delay/pathfinding rate limiter | checked at `0x004B3690-0x004B36B6`; set at `0x004B3A59-0x004B3A8C`, `0x004B4541` | Yes |
| `Foot+0x668/+0x66C/+0x670` | owner Foot | blocked-delay timer: start frame, facing snapshot, duration | set at `0x004B3663-0x004B368D`; checked at `0x004B36BC-0x004B36ED`; reset by `FootClass::Set_Destination_Internal @ 0x004D96C2-0x004D96ED` | Yes |
| `Foot+0x68A` | owner Foot | pending blocked sound flag | played/cleared at `0x004B2E47-0x004B2E70`, `0x004B3AA1-0x004B3ACE`, `0x004B4652` | Yes |
| `Foot+0x6B7` | owner Foot | blocked-by-moving-friendly flag | set at `0x004B3663`, tested at `0x004B36BC`, cleared in `FootClass::Set_Destination_Internal @ 0x004D96C2` | Yes |

## 3. Core Logic

### 3.1 Drive Process calls Process_Movement

`DriveLocomotionClass::Process @ 0x004B0500` is the Drive locomotor tick. Its blocked/no-path relevance is limited to dispatch and stop paths:

1. It calls `DriveLocomotionClass::Process_Movement @ 0x004B2630` when the Drive loco is not in a current track, or after `Process_Drive_Track` cannot continue.
2. It handles stop/scatter-style exits through owner vtable calls `+0x480` and `+0x484`, but the blocked-delay timer itself lives in `Process_Movement`.
3. Active in YR: Yes, because this is the Drive ILocomotion process slot and chrono miners can have Drive active through piggybacking.

### 3.2 Code 2 moving-friendly block starts the blocked timer

Dispatch evidence: `0x004B364D` compares the `Can_Enter_Cell` return code to `2`. If the code is not 2, execution jumps to other code handling at `0x004B3A97`.

On code 2:

1. If `Foot+0x6B7 == 0`, `0x004B3663` sets it to `1`.
2. `0x004B3678` targets `Foot+0x668`.
3. `0x004B367E` reads `RulesClass+0x1768`.
4. `0x004B3684/0x004B368A/0x004B368D` store current frame, facing snapshot, and duration into `Foot+0x668/+0x66C/+0x670`.

Active in YR: Yes. Evidence: code is inside the active ground Drive movement function, and `BlockagePathDelay` is a stock YR `[General]` key (`ini/rulesmd.ini` has default `60`, parsed as `RulesClass+0x1768` in prior verified Rules docs).

### 3.3 Movement delay gates pathfinder calls before blocked-delay urgency

After timer initialization, `0x004B3690-0x004B36B6` checks `Foot+0x640/+0x648`. If the movement-delay timer is still active, the branch skips the repath call and goes to the blocked-sound/return path at `0x004B3AA1`.

Active in YR: Yes. Evidence: unconditional code-2 branch in `Process_Movement`; timer fields are also reset from `PathDelay` at `0x004B3A59-0x004B3A8C`.

### 3.4 Blocked-delay expiration changes Find_Path urgency only

Once movement delay permits a pathfinder call, `0x004B36BC-0x004B36ED` checks `Foot+0x6B7` and `Foot+0x668/+0x670`.

If blocked delay is still running, execution enters `0x004B39D1`, where `BL` is zeroed. `0x004B39FB-0x004B3A00` computes `urgency = 1`.

If blocked delay expired, `0x004B36ED` sets `BL = 1` and jumps past the zeroing site. `0x004B39FB-0x004B3A00` computes `urgency = 2`.

`0x004B3A0E` calls `FootClass::Find_Path @ 0x004D3920` with that urgency argument. The destination used is the Drive destination coord converted from leptons to cell XY.

Active in YR: Yes. Evidence: same active code-2 branch; `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` verifies `urgency=1` maps moving-friendly cells to cost `4.0`, while `urgency=2` maps them to `1000.0`.

### 3.5 What happens after Find_Path returns

After `Find_Path`:

1. If success, `0x004B3A2D` falls through to `0x004B3A59-0x004B3A8C`, resetting the `Foot+0x640` movement-delay timer and returning `1`.
2. If failure but the owner reports it can still move (`vtable+0x2CC` nonzero), it also resets the movement delay and returns `1`.
3. If failure and the owner cannot still move, `0x004B3A3E-0x004B3A47` calls owner vtable `+0x480` with `(0,1)` and returns `0`.

Active in YR: Yes. Evidence: same active Drive movement branch.

### 3.6 No direct SetMission(None) on blocked-delay expiry

No instruction in the code-2 expiration path calls a mission setter or writes a mission ID. The visible side effect of expiration is `Find_Path(..., urgency=2)`.

The stop/give-up call seen in this slice is owner vtable `+0x480(0,1)`, verified as `FootClass::Set_Destination_Internal @ 0x004D94B0` in FootClass context, not `SetMission(None)`. With `param_2=0`, it clears `Foot+0x5A4` destination and resets path timers/`Foot+0x6B7` at `0x004D96C2-0x004D9707`.

Active in YR: Yes. Evidence: `FootClass::Set_Destination_Internal @ 0x004D94B0` decompiled in this pass; vtable+0x480 callsites at `0x004B3A47`, `0x004B3213`, `0x004B44E0`.

### 3.7 Scatter is not the code-2 timer-expiry action

`CellClass::Scatter_Objects @ 0x00481670` appears in adjacent blocker branches, especially code 6/stationary-friendly paths (`0x004B393A`, `0x004B2DC0`, `0x004B327D`). The code-2 expiry path itself goes to `Find_Path` urgency escalation; it does not call scatter as the timer-expiry action.

Active in YR: Yes for the scatter callsites, but conditional on `Can_Enter_Cell` returning the matching non-code-2 blocker code.

### 3.8 Chrono miner harvest re-evaluation is indirect

For a chrono miner driving toward ore, this movement layer can cause harvest code to see a stopped/no-destination miner on a later mission tick, because vtable `+0x480(0,1)` clears destination or because the path queue is exhausted/cleared. The blocked timer itself does not call `Mission_Harvest`, `SetMission(None)`, or a harvest-state setter. Mission re-evaluation is a consequence of normal owner/mission ticking after movement state changes.

Active in YR: Conditional. Evidence: `CMIN` is `Harvester=yes` and uses Teleport locomotor in INI; Drive is active only when piggybacked/selected for ground movement. Harvest re-evaluation is covered by `Mission_Harvest @ 0x0073E5E0` prior docs, not by a direct call in this branch.

## 4. INI Keys

| Key | Section | YR/default value | Binary field | Effect in this slice | Active in YR |
|---|---|---|---|---|---|
| `BlockagePathDelay` | `[General]` | `60` | `RulesClass+0x1768` | copied to `Foot+0x670`; code-2 timer duration before urgency becomes 2 | Yes - evidence `0x004B367E`, `ini/rulesmd.ini`, Rules docs |
| `PathDelay` | `[AI]` | `.01` | `RulesClass+0x1760` | multiplied by `900.0` and stored in `Foot+0x648` as movement-delay rate limiter | Yes - evidence `0x004B3A65-0x004B3A8C` |
| `CloseEnough` | `[General]` | `2.25` cells | `RulesClass+0x1718` | used in no-path/stationary-friendly close-enough stop paths, not the code-2 timer itself | Yes - evidence `0x004B2979`, `0x004B42D5-0x004B42E1`, `ini/rulesmd.ini:58` |
| `Locomotor` on `CMIN` | `[CMIN]` | Teleport CLSID, Drive commented | Type data | makes chrono miner primarily Teleport; Drive path applies when piggybacked/active | Conditional - `ini/rulesmd.ini:7397-7398` |
| `Harvester` on `CMIN` | `[CMIN]` | `yes` | Type data | makes chrono miner run harvester mission logic around movement | Yes - `ini/rulesmd.ini:7364` |

## 5. Integration Points

| Function / point | Role | Evidence | Active in YR |
|---|---|---|---|
| `DriveLocomotionClass::Process @ 0x004B0500` | Drive tick; invokes movement logic and track logic | decompiled this pass; callers/callees show `Process_Movement` | Yes |
| `DriveLocomotionClass::Process_Movement @ 0x004B2630` | owns code-2 blocked-delay branch and no-path stop/clear branches | decompiled this pass; assembly contexts around `0x004B3649-0x004B3A0E` | Yes |
| `FootClass::Find_Path @ 0x004D3920` | receives urgency argument `1` or `2` after code-2 block | call at `0x004B3A0E`; decompiled this pass | Yes |
| `FootClass::Set_Destination_Internal @ 0x004D94B0` | owner vtable `+0x480`; clears/sets `Foot+0x5A4` and resets timers/blocked flag | decompiled this pass; reset at `0x004D96C2-0x004D9707` | Yes |
| `FootClass::Stop_Moving @ 0x004DF0D0` | clears `Foot+0x5A0/+0x5A4` only | decompiled this pass | Yes |
| `CellClass::Scatter_Objects @ 0x00481670` | used by stationary/scatter branches, not code-2 timer expiry | callees + contexts at `0x004B393A`, `0x004B2DC0` | Conditional on non-code-2 blocker branches |

## 6. Current Rust Implementation Status

Rust already has equivalent state fields at `src/sim/components.rs:235-240`: `MovementTarget.blocked_delay` and `MovementTarget.path_blocked`. It parses `BlockagePathDelay` at `src/rules/ruleset.rs:419` and default `60` at `src/rules/ruleset.rs:620`.

Movement handling has a blocked-delay module at `src/sim/movement/movement_blocked.rs:4-181`, including urgency selection (`blocked_delay > 0 => 1`, else `2`) at lines `112-117`. Chrono miner state logic is separate in `src/sim/miner/miner_system.rs`; the binary evidence here supports keeping the blocked timer as movement-layer state, not as a direct harvest mission transition.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `DriveLocomotionClass::Process @ 0x004B0500` blocked/no-path dispatch | verified | decompile this pass; `Process_Movement` calls at `0x004B0500` | none for this slice |
| `Process_Movement` code-2 branch | verified | assembly `0x004B3649-0x004B3A0E` | none |
| `Foot+0x668/+0x670` blocked timer | verified | writes at `0x004B3663-0x004B368D`; reset at `0x004D96C2-0x004D96ED` | none |
| owner `+0x388` timer hypothesis | verified-refuted | `0x004B3410` adds `0x388` before `RateTimer::Current`; blocked timer uses `+0x668` | none |
| `Drive+0x62` was-waiting hypothesis | touched-not-exhausted | constructor zero at `0x004AF5B2`; no scoped branch use found | exact semantic outside blocked/no-path slice |
| `Find_Path` urgency effect | verified by prior doc + callsite | `0x004B39FB-0x004B3A0E`; `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` | none for this slice |
| Direct `SetMission(None)` on timer expiry | verified-absent in slice | no mission setter/write on code-2 expiry path; expiry only changes urgency | runtime mission scheduler timing outside scope |
| Scatter on timer expiry | verified-absent for code 2 | scatter callsites are code 6/other branches, e.g. `0x004B393A` | none |
| Chrono miner active Drive condition | touched-not-exhausted | INI + chrono overview docs | exact runtime frequency for ore approach would need live trace |

## 8. Open Questions - Final State

[RESOLVED] Q1 - Is the blocked-delay timer at owner `+0x388`? No. `+0x388` is used as a `RateTimer` base for facing at `0x004B3410`; blocked-delay is `Foot+0x668/+0x66C/+0x670`.  
[RESOLVED] Q2 - What starts blocked-delay? `Can_Enter_Cell` return code `2` in `Process_Movement`, first tick only when `Foot+0x6B7 == 0`; evidence `0x004B3649-0x004B368D`.  
[RESOLVED] Q3 - What happens when blocked-delay expires? `Find_Path` urgency becomes `2`; evidence `0x004B36ED` and `0x004B39FB-0x004B3A0E`.  
[RESOLVED] Q4 - Does timer expiry call `SetMission(None)`? No direct call/write in this slice; evidence code-2 expiry path only computes urgency and calls `FootClass::Find_Path`.  
[RESOLVED] Q5 - Does timer expiry scatter? No for code 2; scatter calls are adjacent code 6/scatter branches, e.g. `0x004B393A`.  
[RESOLVED] Q6 - Does the branch clear destination? Only on failure/stop paths via owner vtable `+0x480(0,1)` or `FootClass::Stop_Moving`; `FootClass::Set_Destination_Internal @ 0x004D94B0` clears `Foot+0x5A4` when `param_2=0`.  
[RESOLVED] Q7 - Is the path active in standard YR for chrono miner? Conditional: active when CMIN has Drive active/piggybacked; CMIN is a stock YR harvester with Teleport locomotor in `rulesmd.ini`.  
[DEFERRED] Q8 - Exact semantic of `Drive+0x62` outside this slice. Category: out-of-scope; reason: constructor initializes it, but blocked/no-path branches here did not consume it.

## Sources

- Ghidra: `DriveLocomotionClass::Process @ 0x004B0500`
- Ghidra: `DriveLocomotionClass::Process_Movement @ 0x004B2630`
- Ghidra assembly contexts: `0x004B3649-0x004B3A0E`, `0x004B36BC-0x004B36ED`, `0x004B39FB-0x004B3A0E`, `0x004B3410`
- Ghidra: `FootClass::Find_Path @ 0x004D3920`
- Ghidra: `FootClass::Set_Destination_Internal @ 0x004D94B0`
- Ghidra: `FootClass::Stop_Moving @ 0x004DF0D0`
- INI: `ini/rulesmd.ini` `[General] CloseEnough`, `BlockagePathDelay`; `[CMIN] Harvester`, `Locomotor`
- Prior verified reports: `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`, `UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md`, `CHRONO_MINER_SYSTEM_OVERVIEW.md`, `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
