# Drive Process Drive Track Speed Budget / Residual - Ghidra Research Report

**Address(es):** `0x004B0F20` primary; caller `0x004B0500`; speed helpers `0x004DB1A0`, `0x004D3710`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` speed-fraction consumer, per-tick movement budget, residual carry, and point-step consumption.
**Non-Scope:** full `Process_Movement` path selection, full `Apply_Track_Delta`, NavCom/queue arrival, collision recovery, tube movement, and full body-facing cadence except where the speed-budget branch directly touches point stepping.
**Confidence:** High for the scoped budget/residual path; Medium for upstream target-speed producer details because `Process_Movement` was used as context only.
**Active in YR:** Yes. `DriveLocomotionClass::Process @ 0x004B0500` reaches `Process_Drive_Track` on live DriveLocomotion ticks, and stock YR units use DriveLocomotion CLSID `{4A582741-9839-11d1-B709-00A024DDAFD1}`.

## Working Notes Gate

- Target question: How does `Process_Drive_Track @ 0x004B0F20` turn current speed fraction/top-speed/terrain target into an integer per-tick movement budget, how is residual carried, and how do track points consume it?
- Non-goals: Do not re-investigate NavCom, queue arrival, facing beyond point-step coupling, full `Process_Movement`, or full collision/bridge/tube behavior.
- Evidence needed to mark COMPLETE: decompile plus assembly context for `+0xDBD` branch, `SetSpeedFraction`, `GetCurrentSpeed`, retry mask, residual add/store, `7` step consumption, and current Rust surfaces.
- Stop conditions: Stop after budget/residual mechanics are resolved and Rust handoff is concrete; record upstream target producer and full `GetCurrentSpeed` internals as context/deferred if not fully drained.

## 1. Overview

`Process_Drive_Track` does not move Drive units by directly applying `Speed=` leptons each tick. It first synchronizes the owner's current speed fraction, then calls `FootClass::GetCurrentSpeed`, masks out the fresh speed contribution for same-tick retry calls, adds `DriveLocomotion+0x4C` residual, and spends the resulting integer budget in exact chunks of `7` per drive-track point.

The leftover budget is written back to `DriveLocomotion+0x4C`. If a valid track and positive residual remain, the function interpolates visible position toward the next track point using `residual * (1/7)`; this interpolation does not itself consume another point.

## 2. Class Layout / Key Offsets

| Offset | Owner | Type | Meaning for this slice | Active in YR |
|---:|---|---|---|---|
| `+0x4C` | DriveLocomotion | int | residual integer movement budget after point loop | Yes; read at `0x004B1284`, written at `0x004B1F64` |
| `+0x50` | DriveLocomotion | double | current Drive target speed fraction produced upstream, consumed by no-ramp branch | Yes; pushed to `SetSpeedFraction` at `0x004B1261..0x004B1269` |
| `+0x58` | DriveLocomotion | int | active drive track index; `-1` invalid; `<0x40` normal speed-ramp branch | Yes; read throughout `0x004B0F20` |
| `+0x5C` | DriveLocomotion | int | current track point index | Yes; incremented after each consumed point at `0x004B1F53` |
| `+0x60` | DriveLocomotion | byte | selects normal vs short/reversed raw-track byte | Yes; residual branch re-reads it at `0x004B1F83..0x004B1F92` |
| `+0x63` | DriveLocomotion | byte | active track/head-to flag | Yes; early guard at `0x004B0F20` |
| `+0xDBD` | TechnoType | bool | `Accelerates`; `0` skips ramp, `1` runs ramp/brake branch | Yes; reader evidence `0x00715402..0x00715416`, consumer `0x004B0F74..0x004B0F81` |
| `+0x578` | TechnoClass | double | current speed fraction clamped by `SetSpeedFraction` | Yes; helper `0x004D3710` |
| vtable `+0x538` | Foot/Techno owner | method | `FootClass::GetCurrentSpeed`, returns integer budget contribution | Yes; call `0x004B126F..0x004B1274` |
| vtable `+0x544` | Techno owner | method | `TechnoClass::SetSpeedFraction(double)` | Yes; false branch and true branch join use it |

## 3. Core Logic

### 3.1 Speed-fraction branch

Active in YR: Yes. Evidence: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` decompile; assembly `0x004B0F69..0x004B0F81`.

The function gets the owner type with vtable `+0x84`, reads `byte [type+0xDBD]`, tests it, and jumps to `0x004B1261` when zero:

```text
type = owner.GetType()
if type.Accelerates == false:
    owner.SetSpeedFraction(loco.target_speed_fraction_at_+0x50)
else:
    run ramp/brake branch before SetSpeedFraction
```

This corrects older wording that named `+0xDBD` as a formation-leader flag. `+0xDBD` is `Accelerates`, default true, read from `Accelerates=`. Active in YR: Yes. Evidence: `GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, and fresh consumer assembly `0x004B0F74..0x004B0F81`.

When `Accelerates=false`, assembly `0x004B1261..0x004B1269` pushes the double from `DriveLocomotion+0x50` and calls owner vtable `+0x544`. There is no distance/deceleration math on this branch. Active in YR: Yes.

When `Accelerates=true`, the branch can run distance-to-destination, braking, and acceleration/deceleration math before the same `SetSpeedFraction` join. The branch reads destination `+0x34/+0x38/+0x3C`, owner coords `+0x9C/+0xA0/+0xA4`, owner current speed fraction `+0x578`, `SlowdownDistance` at type `+0x2F8`, `DeaccelerationFactor` at type `+0x300`, and `AccelerationFactor` at type `+0x308`. Active in YR: Yes. Evidence: `0x004B0F87..0x004B1211` decompile plus prior Grizzly report.

### 3.2 Speed fraction clamp/store

Active in YR: Yes. Evidence: `TechnoClass::SetSpeedFraction @ 0x004D3710` decompile; assembly contexts `0x004D3710..0x004D3768`.

`SetSpeedFraction` clamps its double argument:

- `input >= 1.0` writes exactly `1.0` to owner `+0x578/+0x57C` (low dword = `0`, high dword = `0x3FF00000`). (corrected 2026-05-29: prior wording said "writes exactly `1.0` to owner `+0x578`", omitting the high-dword write to `+0x57C`; binary at `0x004D3710` writes both dwords for the `>= 1.0` case via `decompile_function 0x004D3710` — MISLEADING)
- `input <= 0.0` writes exactly `0.0` to both `+0x578` and `+0x57C`.
- otherwise stores the double verbatim at `+0x578/+0x57C`.

This means the no-ramp branch means "assign current target speed fraction now", not "ignore terrain/health/slope and force raw full speed".

### 3.3 Integer budget contribution

Active in YR: Yes. Evidence: `0x004B126F..0x004B1295` assembly and primary decompile.

After speed-fraction synchronization, the function calls owner vtable `+0x538`:

```text
speed = owner.GetCurrentSpeed()
budget = (retry_param ? 0 : speed) + loco.residual_budget
```

The retry mask is literal x86 integer logic at `0x004B127A..0x004B128D`: load retry byte, `NEG`, `SBB`, `NOT`, `AND EDX,EAX`. When `param_2 != 0`, the mask zeroes the fresh `speed`; when `param_2 == 0`, the full `GetCurrentSpeed` integer is kept. The residual add is `ADD EDX,EDI` at `0x004B1295`, where `EDI` was loaded from `DriveLocomotion+0x4C`.

`FootClass::GetCurrentSpeed @ 0x004DB1A0` is an integer-return helper. In the inspected decompile/assembly, it composes type speed/current speed factors and returns `iVar3`, halved for UnitClass when owner `+0x6CC != -1`. Full internals of house bonus, veteran/ability modifiers, and `Math__ftol` rounding were not exhausted in this slot. Active in YR: Yes; called directly by live Drive at `0x004B1274`.

### 3.4 Point-step consumption and residual write

Active in YR: Yes. Evidence: primary decompile and assembly contexts `0x004B159D`, `0x004B1F50..0x004B1F64`.

When `budget > 7`, the function enters the track step loop. Each complete point costs exactly `7` budget units. The subtract happens before the point body (`SUB EDI,0x7` at `0x004B159D`), and the loop continues while leftover budget is still greater than `7` (`CMP EAX,0x7`; `JG 0x004B158F` at `0x004B1F50..0x004B1F56`).

After the loop exits, the current point index is incremented (`INC ECX`; `MOV [EBP+0x5C],ECX` around `0x004B1F4F..0x004B1F53`), and leftover budget is stored to `DriveLocomotion+0x4C` (`MOV [EBP+0x4C],EAX` at `0x004B1F64`).

Important boundary: the loop condition is `budget > 7`, not `budget >= 7`. A residual of exactly `7` is stored and does not consume the next point until a later call adds more budget. Active in YR: Yes. Evidence: assembly `CMP EAX,0x7`; `JG`.

### 3.5 Residual interpolation

Active in YR: Yes. Evidence: primary decompile residual branch after `0x004B1F64`.

If the stored residual is `< 1`, the function returns. If the track index is negative, it returns. Otherwise, it re-reads the current track point and computes an interpolated coordinate toward the next point:

```text
interp_delta = Transform(next_point_delta) * (residual * (1/7))
candidate = saved_position + interp_delta
use candidate if candidate cell is saved cell or full-step cell, or residual > 3
else use full-step coordinate
```

The constant is `_DAT_007E7FA8 = 1/7`. The safety gate includes a strict `residual > 3` trust window. Active in YR: Yes. Evidence: decompile calls `CoordStruct__ScaleByFactor(&delta, (float)*(this+0x4C) * _DAT_007e7fa8)` and tests `3 < *(int *)(this+0x4C)`.

This interpolation updates position/cell marking as needed, but it does not increment `+0x5C` again and does not spend another `7` budget units. Active in YR: Yes.

## 4. INI Keys

| Key | Default / stock relevance | Binary reader / consumer | Active in YR |
|---|---|---|---|
| `Accelerates=` | default true; many stock vehicles override false | read into TechnoType `+0xDBD` at `0x00715402..0x00715416`; consumed at `0x004B0F74..0x004B0F81` | Yes |
| `Speed=` | raw type top speed; feeds `GetCurrentSpeed` path after current fraction is set | type speed path via owner helpers; `GetCurrentSpeed @ 0x004DB1A0` called at `0x004B1274` | Yes |
| `AccelerationFactor=` | ramp increment for `Accelerates=true` branch | type `+0x308` read in `0x004B0F87..0x004B1211` | Yes |
| `DeaccelerationFactor=` | ramp/braking decrement for `Accelerates=true` branch | type `+0x300` read in `0x004B0F87..0x004B1211` | Yes |
| `SlowdownDistance=` | distance threshold for destination braking | type `+0x2F8` read in `0x004B0F87..0x004B1211` | Yes |

## 5. Integration Points

`DriveLocomotionClass::Process @ 0x004B0500` is the live caller. It can call `Process_Drive_Track(0)` before `Process_Movement`, then after movement selection it calls `Process_Drive_Track(uVar11)`, where `uVar11` is `1` in the active-track-then-movement path. Active in YR: Yes. Evidence: `0x004B0500` decompile.

`Process_Movement @ 0x004B2630` is the upstream producer for `DriveLocomotion+0x50` target speed fraction from terrain/slope/health context, but its full producer formula is outside this slot. Active in YR: Yes. Evidence: prior `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md` and current decompile context.

`TechnoClass::SetSpeedFraction @ 0x004D3710` clamps/writes owner `+0x578`, and `FootClass::GetCurrentSpeed @ 0x004DB1A0` turns the current speed state into the integer budget contribution used by `Process_Drive_Track`. Active in YR: Yes.

## 6. Current Rust Implementation Status

Current Rust has `DriveLocomotionRuntime` fields for `target_speed_fraction`, `current_speed_fraction`, `residual_budget`, and `drive_delay` in `src/sim/components.rs`.

Rust also has DriveTrack residual and sub-step interpolation in `src/sim/movement/drive_track.rs`: `advance_drive_track` computes `budget = (speed * dt).to_num::<i32>() + state.residual`, spends `TRACK_STEP_COST`, stores `state.residual`, and `interp_sub_step` uses `residual / 7` with a `residual > 3` trust gate.

The remaining mismatch is ownership and input source: current Drive movement still computes `cell_speed_mod`, stores it in `DriveLocomotionRuntime`, then advances through generic `MovementTarget.current_speed * cell_speed_mod` in `src/sim/movement/movement_tick.rs`. For `Accelerates=true`, `store_drive_speed_fraction` intentionally preserves `current_speed_fraction` and does not implement the gamemd ramp. `DriveLocomotionRuntime.residual_budget` is not the active budget consumed by movement; `DriveTrackState.residual` is.

This is not a full miss: Rust now models the `7` point cost and residual interpolation in `DriveTrackState`. But exact gamemd ownership is still drift because `Process_Drive_Track` owns residual at DriveLocomotion `+0x4C` and gets its budget through the owner's current speed fraction helper after the `Accelerates` branch.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Process_Drive_Track @ 0x004B0F20` speed branch | verified | decompile; assembly `0x004B0F69..0x004B1269` | none |
| `Accelerates` identity for `+0xDBD` | verified | `0x00715402..0x00715416`; `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`; consumer `0x004B0F74` | none |
| `SetSpeedFraction` clamp/store | verified | decompile `0x004D3710`; assembly `0x004D3710..0x004D3768` | none |
| retry mask / no double-speed contribution | verified | assembly `0x004B127A..0x004B128D`; caller `0x004B0500` | none |
| residual add and writeback | verified | assembly `0x004B1284..0x004B1295`, `0x004B1F50..0x004B1F64` | none |
| point cost and loop bound | verified | `SUB ...,0x7`; `CMP ...,0x7; JG` at `0x004B159D`, `0x004B1F50..0x004B1F56` | none |
| residual interpolation safety gate | verified | decompile residual branch; `_DAT_007E7FA8`; `residual > 3` | none for branch shape |
| full `GetCurrentSpeed @ 0x004DB1A0` internals | touched-not-exhausted | decompile and assembly `0x004DB1A0..0x004DB240` | dedicated speed helper slice for exact rounding/bonus inputs |
| upstream `Process_Movement` target-speed producer | touched-not-exhausted | `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`; decompile context | sibling swarm slots |
| NavCom/arrival/queue behavior | deferred | out of scope | already covered by sibling docs/current implementation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x004B0F20` live in standard YR? -> Yes, `DriveLocomotionClass::Process @ 0x004B0500` calls it for active Drive ticks.` (evidence: `0x004B0500` decompile; Active in YR: Yes)
- `[RESOLVED] OQ-02 - What is TechnoType `+0xDBD`? -> `Accelerates`, not formation leader.` (evidence: reader `0x00715402..0x00715416`, `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`, consumer `0x004B0F74`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - What happens when `Accelerates=false`? -> Direct `SetSpeedFraction(loco+0x50)` with no ramp math.` (evidence: `0x004B1261..0x004B1269`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - What happens when `Accelerates=true`? -> Ramp/brake branch runs before `SetSpeedFraction`.` (evidence: `0x004B0F87..0x004B1211`; Active in YR: Yes)
- `[RESOLVED] OQ-05 - Does `SetSpeedFraction` clamp? -> Yes, to `[0.0, 1.0]` at owner `+0x578`.` (evidence: `0x004D3710..0x004D3768`; Active in YR: Yes)
- `[RESOLVED] OQ-06 - What is the fresh budget source? -> owner vtable `+0x538` / `FootClass::GetCurrentSpeed`.` (evidence: `0x004B126F..0x004B1274`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Does retry call add speed again? -> No, nonzero `param_2` masks the speed contribution to zero.` (evidence: `0x004B127A..0x004B128D`; Active in YR: Yes)
- `[RESOLVED] OQ-08 - Where is residual read and written? -> read `+0x4C` before budget add; written `+0x4C` after loop exit.` (evidence: `0x004B1284..0x004B1295`, `0x004B1F64`; Active in YR: Yes)
- `[RESOLVED] OQ-09 - What is one track point's cost? -> Exactly `7` budget units.` (evidence: `0x004B159D`; Active in YR: Yes)
- `[RESOLVED] OQ-10 - Is the loop `> 7` or `>= 7`? -> Strictly `> 7`; residual `7` does not consume a point.` (evidence: `0x004B1F50..0x004B1F56`; Active in YR: Yes)
- `[RESOLVED] OQ-11 - Does residual interpolation consume a point? -> No, it scales toward next point after storing residual and returns without `+0x5C` increment.` (evidence: residual branch after `0x004B1F64`; Active in YR: Yes)
- `[RESOLVED] OQ-12 - Does Rust already implement the `7` residual interpolation? -> Partly yes in `DriveTrackState`, not in `DriveLocomotionRuntime` ownership.` (evidence: `src/sim/movement/drive_track.rs`; Active in YR: N/A Rust comparison)
- `[DEFERRED] OQ-13 - Full exact `GetCurrentSpeed` helper formula and every rounding step.` (category: out-of-scope; reason: target is Process_Drive_Track budget path; next-step-if-pursued: dedicated `FootClass::GetCurrentSpeed` report)
- `[DEFERRED] OQ-14 - Full upstream target speed fraction producer in `Process_Movement`.` (category: out-of-scope; reason: sibling swarm target covers tick order/rules fields; next-step-if-pursued: reconcile with `DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-15 - Full `Apply_Track_Delta` point residual semantics.` (category: out-of-scope; reason: sibling slot owns it; next-step-if-pursued: use slot 4 report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Drive budget is `(retry ? 0 : owner.GetCurrentSpeed()) + DriveLocomotion+0x4C`, and retry calls do not add a fresh speed contribution. | `0x004B126F..0x004B1295`; caller `0x004B0500` | mismatch/partial: Rust feeds `effective_speed = MovementTarget.current_speed * cell_speed_mod` into `DriveTrackState` | `src/sim/movement/movement_tick.rs`, `src/sim/movement/drive_track.rs`, `src/sim/components.rs` | Make normal Drive movement budget derive from DriveLocomotion-owned current speed fraction and residual, with a same-tick retry/no-double-count path. | Two same-tick Drive passes with residual 6 and speed contribution 4 consume at most one point, not two fresh-speed budgets -> `drive_track_retry_call_uses_residual_without_new_speed` | Do not add `speed * dt` twice when `Process` calls track before and after movement selection. |
| Each consumed drive-track point costs exactly `7`; loop is strict `budget > 7`; residual `7` is carried. | `0x004B159D`, `0x004B1F50..0x004B1F64` | partial: Rust uses `TRACK_STEP_COST`, but budget lives on `DriveTrackState`, not `DriveLocomotionRuntime` | `src/sim/movement/drive_track.rs`, `src/sim/components.rs` | Preserve strict `> 7` semantics and store leftover in DriveLocomotion-owned residual for normal Drive. | Starting residual 7 with zero fresh speed does not advance point_index; residual remains 7 -> `drive_track_budget_equal_seven_does_not_consume_point` | Do not convert this to `>= 7`; it creates one-point cadence drift. |
| `Accelerates=false` assigns `loco+0x50` to owner current speed fraction immediately; `Accelerates=true` runs ramp/brake before budget. | `0x004B0F74..0x004B1269`; `0x004D3710..0x004D3768`; `0x00715402..0x00715416` | mismatch: Rust parses/stores `accelerates`, but true-ramp branch in `DriveLocomotionRuntime` is not implemented and generic ramp still controls movement | `src/sim/movement/drive_locomotion.rs`, `src/sim/movement/movement_tick.rs`, `src/rules/object_type.rs` | Drive runtime should own `target_speed_fraction` and `current_speed_fraction`; false copies target to current before budget; true applies verified ramp/clamps before budget. | MTNK/Grizzly (`Accelerates=false`) starts at computed target fraction on first Drive tick, while AMCV default true starts below target and ramps -> `drive_accelerates_false_assigns_target_before_budget` / `drive_accelerates_true_ramps_before_budget` | Do not model the flag by zeroing `AccelerationFactor`; the binary uses a distinct bool at `+0xDBD`. |
| Residual interpolation uses `residual * 1/7` with a saved/full-cell safety gate and `residual > 3` trust window. | primary decompile residual branch; `_DAT_007E7FA8`; `3 < residual` test | mostly implemented in `interp_sub_step`; needs ownership reconciliation after Drive runtime owns residual | `src/sim/movement/drive_track.rs`, render-visible position update callers | Keep existing interpolation shape while moving the residual source to DriveLocomotion ownership; do not drop the `>3` gate. | Residual 3 outside saved/full cell falls back to full step, residual 4 uses interpolation -> `drive_residual_interp_trust_window_matches_gt_three` | Do not make interpolation purely `residual / 7` without the cell membership fallback. |

## Negative Facts / Do Not Do

- Do not call `+0xDBD` a formation leader flag. Evidence: `TechnoTypeClass::ReadINI` reads `Accelerates` into `+0xDBD`, default true; `Process_Drive_Track` consumes that byte at `0x004B0F74`.
- Do not make `Accelerates=false` bypass DriveTrack movement. Evidence: the false branch joins before `GetCurrentSpeed` and `7`-unit track stepping at `0x004B126F..0x004B1F64`.
- Do not treat `Accelerates=false` as raw full `Speed=` ignoring terrain/slope/health. Evidence: false branch assigns `DriveLocomotion+0x50`, a current target speed fraction, then `SetSpeedFraction` clamps it.
- Do not consume a point on budget exactly `7`. Evidence: loop test is strict `JG` after `CMP EAX,0x7`.
- Do not keep two residual authorities for exact Drive parity. Evidence: gamemd stores residual on DriveLocomotion `+0x4C`; Rust currently stores the consumed residual on `DriveTrackState`.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/PROCESS_DRIVE_TRACK_DECOMPILATION.md`: replace `TechnoType+0xDBD = is_formation_leader` with "`TechnoType+0xDBD` is `Accelerates`; false jumps to direct `SetSpeedFraction(DriveLocomotion+0x50)`, true enters the Drive ramp/brake branch. Convoy/follower propagation is separate and uses owner `+0x6C8` after the speed-fraction write."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/DRIVE_LOCOMOTION_CLASS.md`: replace the `+0xDBD` field label `is_formation_leader` with "`Accelerates` bool; default true; active Drive consumer at `0x004B0F74..0x004B0F81`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`: current Rust status is stale. Replacement wording: "Rust now parses `ObjectType::accelerates` and carries Drive runtime speed-fraction scaffold fields, but `Accelerates=true` Drive speed ramp and DriveLocomotion-owned residual budget are still not the active movement authority; normal Drive still advances from generic `MovementTarget.current_speed * cell_speed_mod` plus `DriveTrackState.residual`."

## Sources

- Ghidra decompile: `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- Ghidra assembly contexts: `0x004B0F69..0x004B0F81`, `0x004B1261..0x004B1295`, `0x004B159D`, `0x004B1F50..0x004B1F64`
- Ghidra decompile: `DriveLocomotionClass::Process @ 0x004B0500`
- Ghidra decompile/context: `FootClass::GetCurrentSpeed @ 0x004DB1A0`
- Ghidra decompile/context: `TechnoClass::SetSpeedFraction @ 0x004D3710`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/GRIZZLY_ACCELERATES_FALSE_SEMANTICS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/DRIVE_PROCESS_MOVEMENT_GHIDRA_REPORT.md`
- Current Rust scan: `src/sim/components.rs`, `src/sim/movement/drive_locomotion.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/movement/drive_track.rs`, `src/rules/object_type.rs`
