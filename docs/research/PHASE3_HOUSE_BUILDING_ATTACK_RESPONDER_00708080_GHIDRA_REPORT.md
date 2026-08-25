# Phase 3 House Building-Attack Responder (`0x00708080`) Ghidra Report

**Date:** 2026-08-25  
**Binary:** active retail Yuri's Revenge `gamemd.exe`  
**Mode:** exhaustive-slice  
**Status:** COMPLETE RESEARCH — implementation required  
**Scope:** the active House/base-defence response transaction entered by Building and protected-Techno damage, including callers, entry/cooldown gates, team suspension, responder pool admission and ranking, mission/target writes, RNG, persistence, retail Rules bindings, and the downstream ground `Rescue`/`Area Guard` behavior needed for a closed player loop.  
**Excluded:** the already-closed `House+0x54D8` timestamp implementation; broad House strategy; ordinary self-retaliation; Railgun; LaserDraw; Sonic Wave; destroyable cliffs; and Tiberian Sun legacy.

## Verdict

This is an active ordinary-skirmish mechanism, not dormant AI residue. When an eligible AI-owned Building is damaged by a hostile Unit or Infantry, the wrapper records the attacker's House index, runs the base-defence response immediately, suspends low-priority teams, selects at most six same-House Infantry/Units through native fire-legality, zone and threat tests, and queues `Rescue` or `Area Guard` while assigning both a protected anchor and the attacker as shoot target. The response consumes Scenario RNG in selected-list order and arms an attacker-owned cooldown only after the assigned responder cost strictly exceeds the signed response budget.

The current Rust code only preserves the adjacent `House+0x54D8` frame stamp. It has no `ToProtect`, response Rules, responder cooldown, archive anchor, raw last-attacker House index, two map recruitment bytes, team-suspension transaction, six-slot selection quirk, or ground `Rescue` handler. Its `GetFireError`, zone shortcut and target scan cannot be substituted wholesale: this helper tests exactly error `5`, has an asymmetric off-playfield reachability admit, and later `Rescue` uses the native anchored ring scan. Those are implementation requirements, not research unknowns.

## Resolved questions

Every seeded question is closed below. `EXCLUDED` means direct native evidence proves the branch cannot execute for the active supported caller/mode; it does not mean the code was merely omitted.

| ID | Status | Resolution |
|---|---|---|
| OQ-01 | RESOLVED | `FUN_00708080 @ 0x00708080..0x007087B3` is `void __thiscall(TechnoClass *victim, TechnoClass *attacker)`: victim in `ECX`, one explicit attacker argument, `RET 4`. It has no meaningful return value. |
| OQ-02 | RESOLVED | Exhaustive direct callers are Building `ReceiveDamage @ 0x004422BC`, Foot `ReceiveDamage @ 0x004D749A`, and Techno `ReceiveDamage @ 0x007027E9`. Building and generic Techno are live. The Foot call is statically unreachable: its outer route requires `Team == NULL`, while the nested helper block requires `Team != NULL`. |
| OQ-03 | RESOLVED | `HouseClass__Is_Ally_ByObject @ 0x004F9A90` rejects null; otherwise it admits same House and House-alliance-bit membership. The helper exits when attacker is allied to the victim House. |
| OQ-04 | RESOLVED | `HouseClass__IsControlledByHuman @ 0x0050B730` reads House `+0x1EC` in non-campaign modes; campaign also treats the local-player `+0x1ED` case as human. The helper only responds for a non-human victim House. |
| OQ-05 | RESOLVED | `attacker+0x81` is `ObjectClass::InLimbo`; nonzero exits. This is the same byte initialized and maintained by Object `Unlimbo/Limbo`, not an inferred gameplay flag. |
| OQ-06 | RESOLVED | In campaign only (`g_GameMode == 0`), victim vtable `+0x2AC` is the weapon-equipped predicate and a weapon-equipped victim exits. Ordinary skirmish bypasses the branch. |
| OQ-07 | RESOLVED | exact `WhatAmI` values accepted for attacker are `1 = UnitClass` and `0x0F = InfantryClass`; Aircraft, Building, Terrain and all other abstract kinds exit. |
| OQ-08 | RESOLVED | victim type `+0x232` is `ObjectType::Insignificant`, default false and bound to `Insignificant=`. Nonzero exits before any team or responder side effect. |
| OQ-09 | RESOLVED | attacker vtable `+0x2C0` returns type `Cost` (`Type+0x670` for Unit/Infantry). Signed wrapping `attackerCost * Rules+0xB18` forms the budget. `+0xB18` is `[General] ComputerBaseDefenseResponse`, constructor and retail value `3`. Native dereferences attacker for this call before its null check. |
| OQ-10 | RESOLVED | all accepted attackers are Foot-derived and carry Abstract bit 4, so cooldown always applies. `+0x650` is start frame, `+0x658` duration. Start `-1` bypasses elapsed subtraction; otherwise remaining is signed/wrapping `duration - (current-start)` while elapsed is below duration. Any nonzero remaining exits. `+0x654` is written junk and is behaviorally inert. |
| OQ-11 | RESOLVED | `FUN_006EC250` suspends victim-House teams whose signed TeamType Priority is below Rules `SuspendPriority`; it runs after entry/cooldown gates and before either responder scan, including when budget is already non-positive. Members are removed in order, Team bytes `+0x7E/+0x7D/+0x83` are set, and a `SuspendDelay*900` timer is armed. There is no return-value gate. |
| OQ-12 | RESOLVED | `0x00A83DEC/+0x00A83DF8` are Infantry pointer-array/count; `0x008B410C/+0x008B4118` are Unit pointer-array/count. Constructors append and destructors compact-shift, so order is stable construction order with no retained holes. Infantry are scanned completely before Units, subject to the signed budget gate. |
| OQ-13 | RESOLVED | `candidate+0x421` and `+0x422` are two independent, persistent recruitment-admission bytes, both constructor-default `1`. The first is read by recruitment/action helpers and this responder; the second is additionally used by Team admission policy. They are not `CanRock` (live `TechnoClass__RockingUpdate @ 0x0070B570` does not read `+0x421`). Scenario readers write both from the final two `[Aircraft]/[Infantry]/[Units]` columns, and scenario writers serialize both. Use neutral names such as `recruitable_a`/`recruitable_b` until broader Team semantics are named; their exact boolean behavior is closed. |
| OQ-14 | RESOLVED | candidate `+0x5D4` is Team pointer; Team `+0x24` is TeamType; TeamType `+0xF6` is `IsBaseDefense=`. Candidate passes when Team is null or its TeamType is base defence. The same test is repeated at assignment and forces AreaGuard for a base-defence-team member. |
| OQ-15 | RESOLVED | `MissionClass__GetMissionTimerEntry @ 0x005B3A00` returns the current MissionControl entry; byte `+6` is `Recruitable=`. Campaign requires it. Non-campaign modes bypass it, so ordinary skirmish candidates are not excluded by their current mission's Recruitable flag. |
| OQ-16 | RESOLVED | slot `+0x3BC` is the two-argument read-only wrapper over derived `GetFireError(target=attacker, weapon=0, checkCanFire=0)`. The wrapper suppresses range checking. Only exact full-int result `5` (`FIRE_ILLEGAL`) excludes; ammo `1`, busy `3`, cannot `6`, out-of-range `8`, cloaked `9`, and derived non-5 codes remain eligible. Base and Unit/Infantry override illegal branches are part of the required predicate. |
| OQ-17 | RESOLVED | slot `+0x4C` is destination coordinates. For Foot it uses NavCom destination, then an active tube exit, then current coordinates. Each lepton axis converts by signed truncation toward zero: `(v + ((v >> 31) & 255)) >> 8`, then stores signed 16-bit cell components. |
| OQ-18 | RESOLVED | slot `+0xBC` is `ShouldBeOnBridge`; Unit movement zone is reached through instance type `+0x6C4`, Infantry through `+0x6C0`, then type `+0x5B4`. Call is `MapClass__Can_Reach_Zone(candidateDestination, victimDestination, movementZone, shouldBeOnBridge(0), 0, 0)`. Zone `-1` admits. A source cell outside the playfield but inside the logical diamond also admits; the second asymmetric outside-cell shortcut is disabled because the final flag is zero. Otherwise the selected layer's two raw zone IDs must match. |
| OQ-19 | RESOLVED | `FootClass__Evaluate_Target_Threat @ 0x004D97A0` returns signed `i32`: null/other-armed-target/non-base-defence-team/Harvest/zero-cost return zero; already shooting attacker returns `-candidateCost`; otherwise it computes current-position 3D `Sqrt_Approx/ftol` distance, primary range, `base=cost<<10`, and returns base in range or `max(base / max((distance-range)/max(speed,1),1),1)` out of range, with signed wrapping operations. |
| OQ-20 | RESOLVED | victim `+0x218` is the Techno archive/guard anchor set by `Set_ArchiveTarget @ 0x0070C610`, not the shoot target (`+0x2B4`). If victim is its own anchor, nonzero Infantry scores are multiplied by wrapping `100`; this is reachable through the generic `ToProtect` caller. |
| OQ-21 | RESOLVED | Unit type `+0x5EC` is `ResourceGatherer`; candidate `+0x2E4` is its reciprocal bunker installation link; victim `+0x2DC` is SlaveOwner. Any of the three excludes the Unit candidate. A positive Unit score is multiplied by wrapping `10` when victim is self-anchored; this literal class-specific factor differs from Infantry's `100`. |
| OQ-22 | RESOLVED | One shared capacity-six list receives Infantry then Units. While count < 6, every nonnegative nonzero threat is appended; minimum starts at zero and is not updated. Once full, a new signed score must be strictly greater than minimum. Replacement replaces *every* entry equal to the old minimum with the same candidate, creating duplicates; minimum recomputation skips replaced slots and starts from the new score. A later candidate therefore exposes the first-six/minimum-zero quirk. Final sort is stable signed descending because equal scores never swap. |
| OQ-23 | RESOLVED | each selected entry draws from ScenarioClass RNG through inclusive `RandomRanged(0,99)`. `0..=65` queues Rescue for non-base-defence-team members (66%); `66..=99` queues AreaGuard (34%). Base-defence-team members still consume the draw and are forced to AreaGuard. |
| OQ-24 | RESOLVED | slot `+0x1E8` is `MissionClass__Queue_Mission`; `0x15 = Rescue`, `0x0B = Area Guard`. Argument zero means deferred queue, not immediate `Commence`. |
| OQ-25 | RESOLVED | responder `+0x218 = victim` installs the protected archive/guard anchor; slot `+0x3C8(attacker)` is `Assign_Target` and writes the shoot-at target. Both happen after mission queueing on every selected occurrence, including duplicates. |
| OQ-26 | RESOLVED | accumulated responder cost starts zero and adds each responder's signed cost with wrapping arithmetic after its writes. Processing stops only when accumulated cost is strictly greater than budget; equality continues. Negative/modded costs reduce the total. Zero cost can be assigned only through pathological list data because normal threat evaluation returns zero and is not selected. |
| OQ-27 | RESOLVED | cooldown is armed only at the strict accumulated `>` budget exit and only for the Foot-derived attacker flag (always true for valid attackers). Start becomes current signed frame and duration becomes `ftol(BaseDefenseDelay * 900.0)`. Retail `.25` produces 225 frames. No overshoot means no cooldown. |
| OQ-28 | RESOLVED | Building wrapper writes attacker House array index to victim House `+0x54DC` and initializes it to `-1` in the House constructor. No other direct displacement consumer exists. House `Save/Load` serializes the raw virtual-sized House block (`0x160B8`), so the value persists and participates in deterministic state even without a live reader. |
| OQ-29 | RESOLVED | ordinary skirmish reaches Building caller, nonhuman-House gate, noncampaign MissionControl bypass, Infantry/Unit scans, both mission outcomes, RNG, overshoot and cooldown. Campaign-only gates are explicitly identified above. `ShouldProtect +0x3CF` is serialized TS residue: constructor zero and no YR writer to one; only `ToProtect=` activates the generic caller. |
| OQ-30 | RESOLVED | Rust matches signed mission IDs, MissionControl, Scenario RNG, cost/speed/movement-zone/insignificant/resource-gatherer fields, entity construction order, deferred mission queueing, attack targets, bunker/slave links, and House snapshot/hash infrastructure. Missing or partial surfaces are enumerated in the implementation handoff below. |
| OQ-31 | RESOLVED | the helper executes synchronously once per concrete caller, not once per damage batch. Building invokes it before shared Techno immunity/death; generic Techno invokes it after Object damage/visual work and before dead-state/postlude branches. Save/load retains House index, archive/target/mission/team/cooldown state and both RNG cursors. Repeated same-frame calls observe prior mission/target writes and Scenario draws; only an armed attacker cooldown suppresses a later call. |
| OQ-32 | RESOLVED | null attacker faults before the native null guard; supported callers prove nonnull. A null victim owner is outside constructed Techno invariants. Empty pools and signed budget <= 0 assign nobody, but team suspension has already run. Candidate requires Object alive byte `+0x90`; there is no separate candidate `InLimbo` test. Every arithmetic and comparison noted above is 32-bit signed/wrapping. Pool counts are signed and loops stop when count is nonpositive. |
| OQ-33 | EXCLUDED | Foot caller contradiction and `ShouldProtect +0x3CF` writer absence are inactive compiled residue. No Railgun/Laser/Sonic/cliff behavior is called or read by this transaction. No TS-only mechanic is required to implement the active YR branch. |
| OQ-34 | RESOLVED | retail `[Rescue] Rate=.016`; ground Rescue state 0 attacks an existing shoot target, otherwise searches within `1.5 * Threat_Range(1)` around archive anchor, assigns a found target, or transitions to state 1 by choosing a nearby passable cell, setting destination and clearing anchor. State 1 queues/commences AreaGuard when NavCom clears. Cadence is `ftol(Rate*900)+RandomRanged(0,2)`. Retail `[Area Guard] Rate=.040`, `AARate=.032`, `Recruitable=yes`; the ground AreaGuard handler holds/scans around a guard post and uses its own anchored ring topology/cadence. Aircraft Rescue override is excluded because responders are Unit/Infantry only. |
| OQ-35 | RESOLVED | selection does not exclude already-Rescue/AreaGuard/gathering responders except ResourceGatherer Units. Each selected occurrence queues the new mission again, overwrites archive anchor with current victim, overwrites shoot target with current attacker, adds cost again and consumes RNG again. Duplicated top-six entries repeat all of those effects. |
| OQ-36 | RESOLVED | raw stores show cooldown `+0x654` and Team timer `+0x68` receive an uninitialized compiler stack local. Neither has a behavioral reader or CRC use; only start/duration pairs gate execution. Rust must omit the junk value rather than make it deterministic gameplay state. |
| OQ-37 | RESOLVED | RNG is consumed after selected-entry null/team recheck, before choosing mission, including team-forced AreaGuard and duplicates. Null entries draw nothing. The strict overshoot check occurs after writes/cost addition, so no draw happens for later entries once the budget has been exceeded. |
| OQ-38 | RESOLVED | smallest production validation: ordinary skirmish, one nonhuman House with an eligible Building and at least two eligible ground responders, one hostile costed Unit/Infantry attacker, and deterministic Scenario seed. Damage the Building through the normal receiver, then assert House attacker index, ordered RNG advance, selected missions/anchors/targets, team suspension, strict budget stop, attacker cooldown and post-Rescue/AreaGuard behavior through attacker loss. No debug-only dispatch is needed. |

## Exact native transaction

### Call sites and wrapper order

`BuildingClass__ReceiveDamage @ 0x00442230` first applies the no-self-damage early return. With a nonnull attacker and a victim whose vtable `+0x80` predicate is false, it writes current frame to victim House `+0x54D8`, writes attacker House array index to `+0x54DC`, and calls `0x00708080`. This precedes Building immunity, the already-dead gate, and the shared Techno receiver. Consequently even a later-nullified Building hit may mobilize defenders.

`TechnoClass__ReceiveDamage @ 0x00701900` calls the helper when type `ToProtect @ +0xC96` or instance `ShouldProtect @ +0x3CF` is set, the victim House is nonhuman, and attacker is nonnull. It does so after the Object receiver's health/visual work and before the dead-state branch and surviving hostile-hit postlude. Retail `ToProtect=yes` types are exactly:

`DEST, AEGIS, CARRIER, CMON, CMIN, HYD, DRED, HORV, HARV, SMON, SMIN, VLAD, CDEST`.

### Entry and cooldown

The literal entry order is:

1. zero six responder pointers and six signed scores; zero count, minimum and accumulated cost;
2. call attacker cost and compute wrapping budget **before** checking attacker null;
3. reject allied attacker, human-controlled victim House, attacker InLimbo, campaign weapon-equipped victim, attacker other than Infantry/Unit, or Insignificant victim;
4. evaluate attacker response cooldown;
5. suspend low-priority teams;
6. scan Infantry then Units while signed budget remains positive.

Retail bindings and constructor defaults:

| Rules input | Native field | Constructor | retail | frame/value result |
|---|---:|---:|---:|---:|
| `ComputerBaseDefenseResponse` | `+0xB18` | `3` | `3` | attacker cost × 3 |
| `BaseDefenseDelay` | `+0x14D8` | `.25` | `.25` | 225 frames |
| `SuspendPriority` | `+0x14E0` | `20` | `1` | suspend priority `< 1` |
| `SuspendDelay` | `+0x14E8` | `2.0` | `2` | 1800 frames |

### Candidate predicate

For each pointer in pool order:

- pointer nonnull and Object alive byte `+0x90` nonzero;
- same owner pointer as victim;
- no Team, or TeamType `IsBaseDefense=yes`;
- both persistent recruitment bytes `+0x421/+0x422` nonzero;
- weapon-equipped virtual true;
- campaign: current MissionControl `Recruitable=yes`; noncampaign: bypass;
- `PeekGetFireError(attacker, weapon 0) != 5`;
- destination-zone reachability succeeds;
- `Evaluate_Target_Threat(attacker) != 0`;
- Unit only: not ResourceGatherer, not installed in a bunker, and victim has no SlaveOwner.

A negative threat subtracts from the remaining response budget and is not inserted. A positive/nonnegative retained score enters the exact six-slot algorithm described in OQ-22.

### `GetFireError == 5` dependency

The peek passes `checkCanFire=0`; native therefore intentionally ignores range. The implementation must preserve the enum result and compare exactly `5`, not replace it with `can_fire == true`. Active illegal producers include null/limbo target, source/target disabling and deployment states, transport and bunker restrictions, target/object-kind restrictions, projectile-versus-target constraints, warhead Verses zero, capture eligibility, open-transport and bridge/height clauses, and Unit/Infantry override-specific target restrictions. Ammo, rearm/busy, cloaking and several inability states are non-5 and must not exclude a responder.

### Selection and assignment

The six-slot minimum bug and duplicate replacement are observable because they alter which responders consume RNG and receive missions. After stable descending sort, each retained occurrence:

1. rechecks base-defence Team status;
2. consumes `Scenario.RandomRanged(0,99)`;
3. queues Rescue for a non-team/ordinary-team responder on `0..=65`, otherwise AreaGuard;
4. writes archive anchor = victim;
5. assigns shoot target = attacker;
6. adds responder cost with signed wrap;
7. exits only when accumulated cost is strictly greater than budget.

On that exit, valid attacker cooldown becomes `(start=currentFrame, duration=ftol(BaseDefenseDelay*900))`.

## Persistence, ordering and replay

- House `+0x54DC` is a signed raw House field, constructor `-1`, preserved by raw House Save/Load.
- cooldown start/duration, both recruitment bytes, archive anchor, shoot target, mission state, Team suspension state and timers are gameplay state and must be serialized and hashed.
- the Scenario RNG draw is part of the same immediate receiver transaction. Moving it to the next tick or sorting responders by Rust ID after selection changes replay state.
- native global Infantry/Unit arrays are construction ordered; Rust's stable EntityStore order is an acceptable source only if it retains category-first construction order and does not filter limbo objects beyond the explicit alive predicate.
- damage batches must dispatch the helper per receiver in event order. Nested death/damage receivers see all earlier response writes.

## Rust disparity and implementation handoff

### Preserve

- existing Building wrapper timestamp gate/order for House `+0x54D8`;
- MissionControl IDs/rates and deferred mission queue semantics;
- Scenario RNG inclusive range primitive;
- signed object cost/speed, movement-zone, Insignificant and ResourceGatherer data;
- stable entity construction order, bunker/slave links, attack-target representation, House snapshot and hash infrastructure;
- exact current damage receiver ordering and inline world hook seam.

### Add or correct

1. Parse `ObjectType::to_protect` from `ToProtect=` with false default. Add the four signed/double response Rules inputs with native constructor defaults and retail parsing sections.
2. Preserve map/scenario trailing recruitment bytes A/B for Unit, Infantry and Aircraft, defaulting each to true when absent; copy them to persistent entity state and include them in snapshot/hash.
3. Add persistent responder archive anchor and attacker cooldown `(signed start, signed duration)` to ground Techno state. Archive anchor must accept self-reference and survive save/load/hash.
4. Add signed House `last_attacker_house_index`, constructor/default `-1`, and write it at the Building wrapper's native pre-shared-receiver point.
5. Extend the Team seam with Priority, `IsBaseDefense`, suspension latches/timer and ordered member removal. Expose candidate Team lookup and run suspension before the budget early stop. Do not invent a broader AI-team producer.
6. Implement a dedicated `base_defense_response` transaction called synchronously at both native sites: Building prelude and generic ToProtect post-Object/pre-dead-state. Preserve signed wrapping and caller order.
7. Implement exact responder peek classification that returns/compares error `5`; reuse existing represented fire legality facts, but do not collapse other error codes into illegal.
8. Add responder-specific native `Can_Reach_Zone` behavior, including destination/tube-exit coordinates, signed conversion, layer selection and the first off-playfield-in-diamond admit. Do not call the current widened `ZoneGrid::can_reach` without this wrapper.
9. Implement exact `Evaluate_Target_Threat`, Infantry/Unit exclusions and multipliers, the six-slot duplicate replacement bug, stable signed sort, RNG order, strict budget boundary and cooldown arm.
10. Implement ground Rescue states 0/1 and the protected-anchor ring scan. Reuse AreaGuard only after its guard-post/anchored scan path is exact for responders; current nearest-first acquisition and missing guard post are insufficient once the attacker disappears.
11. Add focused native-contract tests for entry gates, campaign/noncampaign difference, team suspension despite zero budget, fire error exact-5 behavior, asymmetric reachability, threat arithmetic, top-six pathology/duplicates, RNG forced-team draw, equality-vs-overshoot, repeated calls/cooldown, Building-vs-ToProtect call order, persistence/hash, and the production closed loop in OQ-38.

### Acceptance gate

The mechanism remains open if any of these are left approximate: exact-5 fire-error classification, candidate destination/zone semantics, archive/self-anchor behavior, top-six duplicate replacement, team suspension/member removal, Scenario RNG ordering, strict signed budget/cooldown behavior, ground Rescue target-loss continuation, or persistence/hash coverage. A test-only helper that bypasses the receiver or mission scheduler does not close the player loop.

## Evidence log

| Evidence | Result |
|---|---|
| live decompile and raw assembly `0x00708080..0x007087B3` | complete helper control flow, fields, pool order, ranking, RNG, writes and cooldown |
| direct xrefs/calls `0x004422BC`, `0x004D749A`, `0x007027E9` | exhaustive caller set and Foot contradiction |
| live Building/Techno receiver disassembly | exact wrapper versus shared-receiver ordering |
| live `HouseClass__Is_Ally_ByObject @ 0x004F9A90`, `HouseClass__IsControlledByHuman @ 0x0050B730` | House admission semantics |
| live `FootClass__Evaluate_Target_Threat @ 0x004D97A0` | signed threat formula and exclusions |
| live `MapClass__Can_Reach_Zone @ 0x0056D100`, `FootClass__GetDestinationCoords @ 0x004DBDF0`, `FootClass__ShouldBeOnBridge @ 0x004DDC40` | destination, cell conversion and zone contract |
| live `TechnoClass__GetFireError @ 0x006FC0B0` plus Unit/Infantry override bytes | exact wrapper arguments and result-5-only gate |
| instruction search for operand `+0x421` and live scenario read/write decompiles | constructor, recruitment readers, scenario writers and three scenario readers; stale CanRock label disproved |
| live `FUN_006EC250` and Team AI timer consumer | suspension transaction and expiry |
| live `FootClass__Mission_Rescue @ 0x004DDF90`, AreaGuard mission bodies, MissionControl reader | downstream mission loop and cadences |
| retail `rulesmd.ini` | `ComputerBaseDefenseResponse=3`, `BaseDefenseDelay=.25`, `SuspendPriority=1`, `SuspendDelay=2`, Rescue/AreaGuard entries and 13 ToProtect types |
| Rust source at branch HEAD `3cd6e5315ed54ee5f6b36b82f1f1353668137fc7` | preserved/missing surface inventory |

No Ghidra metadata was changed during this investigation.

## Annotation candidates

- name `0x00708080` as `TechnoClass__RespondToBaseAttack` with `void __thiscall(TechnoClass *victim, TechnoClass *attacker)`;
- name Techno `+0x650/+0x658` as base-defence response cooldown start/duration; retain `+0x654` as unknown/junk, not a timer component;
- replace stale `Techno+0x421 CanRock` with neutral `RecruitableA` and `+0x422 RecruitableB` pending the broader Team semantics audit;
- name House `+0x54DC` as `LastBuildingAttackerHouseIndex`;
- name Rules `+0xB18/+0x14D8/+0x14E0/+0x14E8` from their exact INI keys.
