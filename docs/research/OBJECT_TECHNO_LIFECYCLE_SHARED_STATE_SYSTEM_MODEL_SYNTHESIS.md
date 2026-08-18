# Object / Techno Lifecycle Shared State - System Model Synthesis

**Date:** 2026-05-28  
**Scope:** ObjectClass reveal/register/conceal/unregister, active object order, limbo/storage separation, selected TechnoClass shared runtime fields, miner unload accumulator timing, and IronCurtain/ForceShield timer state.  
**Non-scope:** complete TechnoClass field map, all weapon/turret/cloak/temporal state, save/load serialization, rendering tint parity, and full projectile/anim lifecycle.  
**Output type:** model-synthesis with bounded unknowns.  
**Overall status:** implementation-safe for ObjectClass logic membership/order and selected Techno shared fields; partial-ready for broader TechnoClass only after more field-cluster research.

## Evidence Ladder Used

| Rank | Meaning in this synthesis |
|---|---|
| BINARY_HIGH | Ghidra report with direct function body, caller/gate/default, and active YR status |
| RESEARCH_HIGH | Recent re-swarm/re-investigate report with exact addresses and Rust handoff |
| TRACE_HIGH | Runtime trace tied to binary findings |
| DOC_SYNTHESIS | Older overview prose; not canonical when contradicted |
| INFERENCE | Unsafe unless later verified |

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Object logic membership is explicit state, guarded by `Object+0x98`. | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Reveal/register tail-appends to the LogicClass active vector; conceal/destructor unregister compacts and clears membership. | logic helper report; scheduler reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Scenario active-object order is loader section/key order plus runtime successful reveal/register order, not sorted storage ID. | `ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Limbo/storage existence does not imply active logic membership or cell occupation. | `LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Failed GACNST redeploy constructs an AMCV-like object that survives in limbo rather than rolling back construction. | `FAILED_REDEPLOY_LIMBO_UNIT_CLEANUP_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Object save/load active-vector rebuild owner is known. | active-order report | unknown | medium | yes | NEEDS_REINVESTIGATE |
| HARV/CMIN unload accumulator runs after mission dispatch, so unload-start cannot increment in the same AI pass. | `TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `+0x110` unload step is constructor-defaulted to `1`, not derived from `HarvesterDumpRate`. | `UNIT_0X110_UNLOAD_ACCUMULATOR_STEP_WRITERS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `+0x104` in the unload timer cluster is local scratch, not a semantic world coordinate. | `MISSION_DEPLOY_UNLOAD_TIMER_CLUSTER_0X104_SOURCE_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| IronCurtain and ForceShield timers are start-frame/duration fields read against `g_CurrentFrameCounter`. | `IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Full TechnoClass shared state is covered by the current evidence set. | multiple reports | unknown | low | yes | NEEDS_REINVESTIGATE |

## Current Model

Object lifecycle has separate layers:

1. Native object construction/global class-array membership.
2. Limbo versus unlimbo/cell occupation.
3. LogicClass active-object membership guarded by `Object+0x98`.
4. Owner, visibility, target/contact, and class-specific shared state.

`ObjectClass::Reveal` can register an object for logic only after type/game-mode gates pass. The helper checks the object-local membership byte, appends to the active vector on first registration, and sets the byte only after insertion succeeds. Unregistration checks the same byte, removes by compacting the active vector, and clears the byte even if the object was flagged but not found.

This means `EntityStore` membership cannot stand in for native object lifecycle state. A stored object can be in limbo, unregistered from logic, or active in logic depending on the owning native path.

The selected TechnoClass shared state currently has safe islands:

- HARV/CMIN unload accumulator fields `+0xF8..+0x110` are live, frame-driven, and ordered after mission dispatch inside `TechnoClass::AI_Update`.
- IronCurtain/ForceShield invulnerability uses Techno timer fields and a kind flag, with passive expiry and damage rejection through active checks.
- ForceShield power blackout uses a House timer start/duration shape, not a remaining-count max model.

These islands are not a complete TechnoClass port.

## Implementation-Safe Facts

- Add or preserve an object-local logic membership field separate from storage, limbo, alive/dead, and cell occupation.
- Active object order must come from native scenario-load order and successful reveal/register order.
- Limbo object creation must not automatically append to the active logic list.
- Failed deploy/redeploy paths should not assume construction rollback unless the native path proves deletion.
- HARV/CMIN unload start writes the timer cluster, then the same AI pass observes elapsed `0`; state-3 drains only from previous accumulator state.
- `HarvesterDumpRate` is the dump threshold source; `+0x110` is the accumulator increment step and defaults to `1`.
- IronCurtain/ForceShield apply/check paths must use the native frame source, not an unrelated Rust tick.
- ForceShield blackout should be represented as start frame plus duration unless a future binary check proves max-countdown semantics.

## Doc-Patch-Ready Facts

- Replace any prose equating stored entity existence with active object AI eligibility.
- Replace any "limbo object ticks because it exists" wording with explicit reveal/register membership.
- Replace any "unload step comes from HarvesterDumpRate" wording with threshold versus step separation.
- Mark invulnerability docs that use generic tick countdowns as stale unless they explicitly map to `g_CurrentFrameCounter`.

## Stale Or Superseded Claims

- Sorted stable-ID order as a fallback active-object order is not parity-safe.
- Any failed-redeploy cleanup assumption that deletes the constructed AMCV-like object is contradicted by the failed redeploy report.
- Any semantic interpretation of unload cluster `+0x104` as a world coordinate is superseded by the timer-cluster report.

## Cross-Doc Conflicts

No unresolved conflict remains for the scoped ObjectClass registration/removal model or the scoped miner unload accumulator model. Broader TechnoClass remains incomplete, not contradicted.

The strongest boundary is save/load: reports prove `ObjectClass::Save/Load` do not directly serialize or rebuild the active membership byte, but they do not prove the later reconstruction owner.

## Needs Re-Investigation

- `/re-investigate ObjectClass save load active vector rebuild owner`
  - Needed before persistence/replay order can be implemented.
- `/re-investigate direct FUN_0055BAA0 caller <address>`
  - Needed for each non-`ObjectClass::Reveal` direct registration/removal caller before generalizing activation paths.
- `/re-swarm TechnoClass shared runtime fields`
  - Needed before turning this into a full TechnoClass field contract.
- `/re-investigate TechnoClass save load accumulator invulnerability fields`
  - Needed before save/load byte parity for selected shared fields.

## Do-Not-Implement Notes

- Do not infer active logic membership from `EntityStore` presence.
- Do not use sorted ID fallback to silently activate stored but unregistered objects.
- Do not register limbo-created objects unless the native path reveals/registers them.
- Do not derive `+0x110` from `HarvesterDumpRate`.
- Do not treat ForceShield blackout as "remaining ticks = max(existing, new)" without binary proof.
- Do not claim this doc covers every TechnoClass shared field.

## Source Ledger

- `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
- `docs/research/ACTIVE_OBJECT_ORDER_SOURCE_LOAD_REVEAL_SPAWN_GHIDRA_REPORT.md`
- `docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/FAILED_REDEPLOY_LIMBO_UNIT_CLEANUP_GHIDRA_REPORT.md`
- `docs/research/SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/miner/TECHNOCLASS_AI_UPDATE_UNLOAD_ACCUMULATOR_ORDERING_GHIDRA_REPORT.md`
- `docs/research/miner/UNIT_0X110_UNLOAD_ACCUMULATOR_STEP_WRITERS_GHIDRA_REPORT.md`
- `docs/research/miner/MISSION_DEPLOY_UNLOAD_TIMER_CLUSTER_0X104_SOURCE_GHIDRA_REPORT.md`
- `docs/research/IRONCURTAIN_FORCESHIELD_GHIDRA_REPORT.md`
- `src/sim/entity_store.rs`
- `src/sim/game_entity.rs`
- `src/sim/world/mod.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/miner/mod.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/superweapon/invulnerability.rs`
- `src/sim/superweapon/force_shield.rs`
- `src/sim/superweapon/iron_curtain.rs`
- `src/sim/power_system.rs`
