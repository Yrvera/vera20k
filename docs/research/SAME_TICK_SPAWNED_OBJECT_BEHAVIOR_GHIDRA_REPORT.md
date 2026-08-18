# Same-Tick Spawned Object Behavior - Ghidra Research Report

**Date:** 2026-05-28
**Target:** `SAME_TICK_SPAWNED_OBJECT_BEHAVIOR`
**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, `BulletClass::Fire @ 0x00468670`, `BulletClass::AI @ 0x004666E0`, `ObjectClass::Conceal @ 0x005F4D30`
**Investigation Mode:** re-swarm slot 5 reconciliation
**Active in YR:** Yes. The scoped examples are standard active YR object paths: stock GGI `AAHeatSeeker2` bullets, CABHUT bridge repair engineers, civilian garrison entry, stock CMIN/HARV refinery contention, and ordinary garrison `OccupantAnim` creation.
**Status:** COMPLETE for static same-tick scheduler consequences and Rust handoff scenarios. Runtime-only concrete vector indices remain deferred.

## Target Question

Which concrete object creation/removal examples prove that native `LogicClass` live-vector order matters, and how does current Rust handle those cases?

The implementation-relevant answer is: native `LogicClass::PerTickUpdate` is a live forward object-vector walk. Tail-appended objects can run later in the same pass, and removal of the current object can compact the next object into the already-processed index so it waits until a later pass. Rust now models this locally for some slices, but it does not yet have a single native-equivalent global object scheduler.

## Non-Goals

- Do not rediscover the full global tick ladder; sibling slots own global phase order.
- Do not rederive full bullet homing math, bridge walker internals, garrison `CanDock`, refinery radio state machines, or complete `AnimClass` draw/lifecycle.
- Do not implement Rust changes or mutate Ghidra state.
- Do not runtime-trace a concrete retail replay's object-vector indices.

## Evidence Needed To Mark COMPLETE

- One direct read-only Ghidra spot-check of the live object loop or a same-tick spawned object path.
- Reconcile existing reports for the five requested examples.
- Compare current Rust handling, including cases already fixed since older reports.
- Produce acceptance scenarios that exercise append, remove/compact/skip, and order-dependent same-frame vs next-frame effects.

## Stop Conditions

- Stop when existing verified docs plus targeted spot checks establish the mechanism and handoff tests.
- Downgrade only if a requested example lacks verified binary evidence or Rust cannot be statically mapped.
- Runtime vector-index logging remains a follow-up, not a blocker, when static code already proves the rule.

## Direct Ghidra Spot Checks

Verified binary facts:

- `LogicClass::PerTickUpdate @ 0x0055AFB0` contains the main live-object loop. Direct decompile and assembly context show `items[i]` load at `0x0055B608..0x0055B60B`, `vtable+0x5C` call at `0x0055B610`, live count reload at `0x0055B613`, index increment at `0x0055B616`, compare at `0x0055B617`, and loop at `0x0055B619`. Active in YR: Yes.
- `BulletClass::Fire @ 0x00468670` calls `ObjectClass::Reveal`, normalizes/arms the projectile, and calls `DisplayClass::Submit_Object` when active. This is the standard path used by bullets created from `TechnoClass::Fire_At`. Active in YR: Yes.
- `BulletClass::AI @ 0x004666E0` begins with `ObjectClass::AI`, can create trailer anims, processes ROT/homing/proximity, calls `BulletClassBulletDetonationImpactDamage`, and then destroys/uninitializes the bullet. Active in YR: Yes.
- `ObjectClass::Conceal @ 0x005F4D30` is the normal remove-from-world path for limbo/destruction-style transitions and calls the logic-vector remover before setting `Object+0x81` in-limbo. Active in YR: Yes.

Inference from those facts: the same scheduler contract explains both same-pass tail appends and same-pass removal skips. The exact object index of a concrete retail map object is runtime state and is not inferable from static decompile alone.

## Example Reconciliation

### 1. AAHeatSeeker2 first-tick damage

Verified binary facts:

- `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md` verifies that `ObjectClass::Reveal -> FUN_0055BAA0 -> DynamicVector::Insert` appends a logic-enabled bullet, and the main loop reloads count after each object. A bullet fired by an object whose AI is still in the forward pass can therefore run `BulletClass::AI` in the same game frame.
- `BulletClass::AI` can reach real detonation/damage on its first AI call; `Arm=2` gates the proximity detector, not every earlier ROT close-hit path.

Current Rust comparison:

- Rust combat emits `SimFireEvent` and applies damage inside `combat::tick_combat_with_fog`; visible projectile flight is not authoritative for the stock `AAHeatSeeker2` path.
- This is not native-equivalent even when the visible result happens to be "damage on the firing tick." Native damage belongs to a real appended `BulletClass` object and can be same-frame, next-frame, or later depending on creation context and projectile state.

Acceptance scenarios:

- GGI fires at a legal close ground target during its object-AI turn: spawned DRAGON bullet receives first AI in the same global frame and may detonate through the verified close-hit path.
- Same fire call made after the main object vector pass: first bullet AI is deferred one frame.
- Moving target/rising aircraft case: no direct fire-tick damage unless the native `BulletClass::AI` thresholds actually detonate.

### 2. Bridge multi-engineer same-tick repair/removal

Verified binary facts:

- `BRIDGE_REPAIR_MULTI_ENGINEER_SAME_TICK_GHIDRA_REPORT.md` verifies no CABHUT duplicate latch: if a second engineer reaches the branch, SFX/EVA, repair dispatch, callbacks, and engineer disposal repeat.
- The same report verifies that engineer disposal can remove the current object from the live vector; compaction can shift the immediate successor into the current index, and the scheduler increments past it.

Current Rust comparison:

- Older wording that Rust used an unconditional prebuilt candidate snapshot is stale for current source. `tick_bridge_repair_orders` now iterates a sorted key list with `key_idx += 2` after engineer despawn to model the immediate-successor skip for this local bridge-repair slice.
- This is a local surrogate, not proof of a global `LogicClass` scheduler. It depends on stable-id order and this function's manual skip.

Acceptance scenarios:

- Consecutive engineers in logic order target one CABHUT: first engineer repairs and is consumed; the immediate successor does not emit SFX/EVA until the next pass.
- Nonconsecutive engineers in logic order target one CABHUT: both can execute in one tick, independently emitting feedback and being consumed.
- If the second branch runs, do not suppress feedback by hut id and do not require a positive repaired-cell count before consuming the engineer.

### 3. Civilian garrison ownership timing

Verified binary facts:

- `CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md` verifies infantry entry mutates the building occupant vector via `AddGarrisonOccupant @ 0x00522910`, but ownership transfer is later in the target building's `BuildingClass::Update -> CheckAutoSellOrCivilian`.
- Same-frame transfer is possible only if the infantry entry occurs before the building's update turn in the same live object pass. If the building already updated, transfer waits until its next update.

Current Rust comparison:

- Current Rust has a `live_object_order` surrogate and `tick_passenger_system` iterates `Simulation::live_object_order_snapshot()`, processing boarding and then building reconciliation per object turn.
- Focused tests now cover same-frame transfer when passenger precedes building and next-pass transfer when building precedes passenger. This is good scoped parity work, but it remains an interim surrogate over `EntityStore`, not a full native scheduler.

Acceptance scenarios:

- Passenger object before target building: boarding appends occupant, later building turn transfers owner in the same frame.
- Target building before passenger: building sees no occupant, passenger boards later, owner transfers only on the next building reconciliation turn.
- `AddGarrisonOccupant`/boarding itself must not change owner.

### 4. Two miners / one refinery ordering

Verified binary facts:

- `LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY_GHIDRA_REPORT.md` verifies append/reveal order is the miner AI order unless a later conceal/reveal path reorders it.
- Healthy CMIN return, dock, unload, and chrono teleport do not remove/reappend the miner, so the older miner normally remains earlier than the later miner.
- A release by miner A does not promote miner B through an A-side callback; B can claim only when B's own later `Mission_Enter` retry runs and its mission timer is due.

Current Rust comparison:

- Rust miner processing still uses `EntityStore::keys_sorted()` / stable-id order in `miner_system`. Tests can model A-before-B and B-before-A by choosing ids, but that is not the same as maintaining a native append-ordered live object vector.
- Current tests are useful acceptance coverage, but parity wording should remain order-dependent rather than "always same-frame" or "always next-frame."

Acceptance scenarios:

- Older/free miner A before later miner B, B timer due: A release can be observed by B later in the same frame.
- B before A: B cannot retroactively claim a contact freed later by A in the same pass.
- A's healthy dock/unload/chrono lifecycle must not despawn/reinsert A.

### 5. AnimClass registration and lifetime

Verified binary facts:

- `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md` verifies `AnimClass` constructor appends to `g_AnimClass_Array`, but ordinary per-tick AI for revealed anims is through the live `LogicClass` object vector, not a dedicated `g_AnimClass_Array` AI loop.
- `ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md` verifies a first-AI guard: a same-pass AI visit is possible, but it clears the guard and returns before frame advancement.

Current Rust comparison:

- Current garrison UC flashes use an app-layer vector, not a global `AnimClass` pool. Existing docs say this can be sufficient for ownerless ordinary garrison flashes if insertion/survivor order and lifecycle semantics are preserved.
- Lifecycle remains incomplete: first-AI guard, native `End`/loop/`Next`/`Rate=0` semantics, and generic anim fields are not yet represented. Generic attached anims still need a broader anim object/pool design later.

Acceptance scenarios:

- Two garrison shots in one fixed tick preserve creation order and survivor compaction order.
- Newly created UC flash receives an initial same-pass visit that clears first-AI guard but does not advance a frame.
- Do not treat `g_AnimClass_Array` as the ordinary scheduler.

## Implementation Handoff

| Behavior | Evidence | Current Rust status | Required effect / acceptance |
|---|---|---|---|
| Tail-appended logic objects can tick in the same pass. | `0x0055B608..0x0055B619`; AAHeatSeeker2 report; direct `BulletClass::Fire` spot-check | Not generally modeled; combat applies direct damage instead of authoritative bullets | Add/plan an authoritative live-object projectile path. Test same-context fire gets same-pass first AI, while post-loop fire waits one frame. |
| Current-object removal can skip the immediate shifted successor. | `0x0055B608..0x0055B619`; `ObjectClass::Conceal @ 0x005F4D30`; bridge report | Locally modeled for bridge repair by manual `key_idx += 2`; not global | Keep bridge tests, but do not generalize from them. Add scheduler-level tests when global live vector exists. |
| Same-frame vs next-frame effects are order-dependent, not class-phase-dependent. | Garrison and miner reports | Garrison has a scoped `live_object_order` surrogate; miners still use stable-id order | Preserve explicit A-before-B / B-before-A tests for garrison and refinery handoff; later replace stable-id surrogates with native live-object order. |
| Same-pass AnimClass visit does not imply same-pass frame advance. | Anim lifecycle report | Garrison flash runtime lacks first-AI guard | Implement first-AI guard in embedded `AnimRuntime` or future pool; update stale cadence wording. |

## Negative Facts / Do Not Do

- Do not describe native same-tick behavior as a class phase such as "all infantry before buildings" or "all bullets next frame."
- Do not snapshot all candidate objects at pass entry and call that parity; native reloads live count and removals compact the vector.
- Do not coalesce duplicate bridge-repair feedback when the second engineer branch actually runs.
- Do not add a refinery-side FIFO promotion callback on A release; B admission belongs to B's own retry.
- Do not treat `g_AnimClass_Array` as the ordinary `AnimClass::AI` scheduler.
- Do not treat Rust stable-id order as equivalent to native append/reveal order except in tests that explicitly choose ids to model a known relative order.

## Stale Docs / Replacement Wording

- `docs/research/bridges/08-traces/MULTI_ENGINEER_SAME_TICK_BRIDGE_REPAIR_TRACE.md`: replace wording that says duplicate same-tick behavior could not be live-verified with the bridge report's static rule: immediate-successor skip is verified by live-vector compaction, while a second engineer that actually runs repeats branch feedback/disposal.
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_REPAIR_MULTI_ENGINEER_SAME_TICK_GHIDRA_REPORT.md`: its "current Rust uses a prebuilt candidate snapshot" wording is stale for current source; Rust now has a local bridge skip surrogate, though not a global scheduler.
- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`: replace wording implying ordinary `AnimClass::AI` iterates `g_AnimClass_Array` with the live `LogicClass` object-vector scheduler wording from the 2026-05-27 AnimClass reswarm.
- `docs/research/traces/GARRISON_SHOT_CADENCE_POSTFIX_TRACE.md`: replace wording that same-pass AI can advance the newly spawned `OccupantAnim` frame with: same-pass AI can occur, but the first-AI guard clears and returns before frame advancement.

## Remaining Uncertainty

- Concrete retail object-vector indices for a given map/replay still require runtime logging at the live loop.
- Exact AAHeatSeeker2 first-hit frame for every target/altitude state belongs to the projectile math reports; this report only uses the scheduling consequence.
- Save/load reconstruction of live object order and mid-animation object state remains out of scope.
- Rust's local surrogates may pass the scoped tests while still diverging in interactions that combine bullets, anims, removals, production, and mission transitions in one global pass.

## Sources

- Direct Ghidra read-only spot checks: `0x0055AFB0`, `0x00468670`, `0x004666E0`, `0x005F4D30`; assembly context for `0x0055B608`, `0x0055B619`, `0x0055B5FB`.
- `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
- `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_REPAIR_MULTI_ENGINEER_SAME_TICK_GHIDRA_REPORT.md`
- `docs/research/CIVILIAN_GARRISON_OWNER_TIMING_GLOBAL_ORDER_GHIDRA_REPORT.md`
- `docs/research/miner/LIVE_OBJECT_VECTOR_ORDER_TWO_MINERS_REFINERY_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`
- `docs/research/ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`
- Rust static scan: `src/sim/world/mod.rs`, `src/sim/entity_store.rs`, `src/sim/passenger.rs`, `src/sim/world/world_orders.rs`, `src/sim/combat/mod.rs`, `src/sim/miner/miner_system.rs`.
