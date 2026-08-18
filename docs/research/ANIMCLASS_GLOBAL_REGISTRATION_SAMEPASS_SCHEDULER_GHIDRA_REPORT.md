# AnimClass Global Registration / Same-Pass Scheduler - Ghidra Report

**Date:** 2026-05-28  
**Investigation mode:** exhaustive-slice  
**Target:** `AnimClass` global registration and same-pass scheduler behavior for children constructed during `AnimClass::AI`.  
**Primary addresses:** `AnimClass::Constructor @ 0x00421EA0`, `AnimClass::AI @ 0x00423AC0`, trailer constructor call `0x0042431D`, bouncer `ExpireAnim` constructor call `0x00423E70`, `ObjectClass::Reveal @ 0x005F4EC0`, `FUN_0055BAA0 @ 0x0055BAA0`, `LogicClass::PerTickUpdate @ 0x0055AFB0`, `FUN_0055BAE0 @ 0x0055BAE0`, `AnimClass::Destroy @ 0x004255B0`, `ObjectClass::UnInit @ 0x005F65F0`.

## Working Notes Gate

`Target question`: How are newly constructed `AnimClass` children registered into the live global traversal, and can trailer / bouncer / expire children appended during an `AnimClass::AI` visit receive `AI` in the same global tick?

`Non-goals`: Do not re-investigate constructor row arguments, `TrailerSeperation` modulo, `Next=` in-place mutation, bouncer impact return gates, bouncer damage formulas, draw ordering, or audio timing. Those are settled by sibling reports.

`Evidence needed to mark COMPLETE`: decompile plus assembly context proving constructor registry append, reveal-time LogicClass registration, live scheduler count reload, child constructor call sites inside `AnimClass::AI`, and remove/destroy interactions for parent cleanup. Include current Rust surface comparison and concrete test-name proposals.

`Stop conditions`: Stop after proving append/cursor/remove semantics for the scoped child examples. Runtime debugger capture of concrete vector indices is allowed to remain deferred because static scheduler mechanics determine the contract.

## Summary

New `AnimClass` children created from inside `AnimClass::AI` are ordinary `AnimClass` objects. The constructor appends each object to `g_AnimClass_Array`, then for normal non-bouncer/non-meteor child types it calls `ObjectClass::Reveal`, which can append the object to the live `LogicClass` active-object vector through `FUN_0055BAA0`.

The ordinary per-tick `AnimClass::AI` dispatch is not a `g_AnimClass_Array` walk. It is the shared `LogicClass::PerTickUpdate` live-vector loop. That loop loads `items[index]`, calls `vtable+0x5C`, increments the index, then reloads the live count before the next comparison. Therefore a trailer, `BounceAnim`, or `ExpireAnim` child appended to the tail during a parent `AnimClass::AI` visit can receive `AI` in the same global tick if the scheduler cursor has not yet reached the new tail.

Same-pass eligibility is not the same as visible same-pass advancement. A trailer child is constructed with `delay=1`, so its first scheduler visit returns through delay/first-visit lifecycle before visible playback work. Delay-zero children such as `BounceAnim` and `ExpireAnim` are same-pass eligible under the same cursor rule, subject to the native first-AI guard and their own lifecycle fields.

Parent cleanup is also live-vector visible. When accepted bouncer impact later destroys the parent, `AnimClass::Destroy` calls `ObjectClass::UnInit`, which reaches conceal/removal and the pending-delete queue. The active-vector remover compacts entries left and the scheduler does not repair the index, so self-removal can skip an immediate shifted successor. That is the same global scheduler rule, not an AnimClass-specific side channel.

## Verified Findings

### Constructor appends to `g_AnimClass_Array`, but that registry is not the AI scheduler

Active in YR: Yes.

`AnimClass::Constructor @ 0x00421EA0` first runs the object base constructor and then appends `this` to the `g_AnimClass_Array` registry. Fresh Ghidra spot-check:

- `0x00422058..0x00422063`: load registry capacity/count globals.
- `0x00422067..0x0042208E`: growth gate for the dynamic vector.
- `0x00422092..0x004220A7`: read old count, increment `g_AnimClass_Array_Count`, and store `ESI` into `g_AnimClass_Array[old_count]`.
- `0x004220AA..0x004220B0`: continue initialization and set `ObjectClass+0x90` alive byte.

This proves object identity and registry order, but not tick order. The ordinary AI scheduler is the live `LogicClass` vector described below. Prior stale wording that says `AnimClass::AI` is run by iterating `g_AnimClass_Array` is wrong.

### Reveal can append newly constructed anims to the live LogicClass vector

Active in YR: Yes, conditional on normal reveal gates and `ObjectTypeClass+0x234` logic eligibility.

`AnimClass::Constructor` calls `ObjectClass::Reveal(coords, 0)` for normal child types when the type is not entering the bouncer/meteor alternate constructor branch. `ObjectClass::Reveal @ 0x005F4EC0` reaches the logic registration call after display submission and type/game-mode gates. Fresh assembly context at `0x005F5038..0x005F5040` shows:

- `PUSH 0`
- `PUSH ESI`
- `MOV ECX,0x87F778`
- `CALL 0x0055BAA0`

`FUN_0055BAA0 @ 0x0055BAA0` checks `ObjectClass+0x98`; if the byte is already set, it returns success without duplicate insertion. Otherwise it calls `DynamicVector__Insert @ 0x005519B0`; on success it writes `ObjectClass+0x98 = 1`. Evidence: decompile plus assembly context `0x0055BAA5..0x0055BAC6`.

This is the same active vector used by the tick scheduler at singleton `0x87F778`.

### The live scheduler reloads count after every object AI call

Active in YR: Yes.

`LogicClass::PerTickUpdate @ 0x0055AFB0` contains the main object loop:

- `0x0055B5FF`: index starts at zero.
- `0x0055B608..0x0055B60B`: load `LogicClass+0x04[index]`.
- `0x0055B610`: call object vtable slot `+0x5C`.
- `0x0055B613`: reload `LogicClass+0x10` live count.
- `0x0055B616..0x0055B619`: increment index, compare against the reloaded count, and loop.

Handoff-critical consequence: a child appended to the tail during a parent `AnimClass::AI` can be reached later in the same pass when `new_index >= old_count` and the live count reload observes the append before the cursor exits. If the parent runs after the cursor has already passed what becomes the appended tail, the child waits until the next pass. Static analysis proves the rule; concrete map/replay index values require runtime logging.

### Trailer children use the same registration path after their constructor call

Active in YR: Conditional on `TrailerAnim != null` and signed modulo pass.

The already-settled trailer branch constructs the child at `0x0042431D`. This slot reuses only the scheduler-relevant part: after `AnimClass::AI` allocates `0x1C8` and calls `AnimClass::Constructor`, the child enters the same constructor path above and can be appended to the live LogicClass vector through `ObjectClass::Reveal`.

Because sibling work proved the trailer row uses `delay=1`, a same-pass scheduler visit is still lifecycle-gated by the delay/first-visit behavior. Do not infer immediate visible frame advancement from same-pass insertion.

### Bouncer `BounceAnim` and `ExpireAnim` children use the same registration path, but parent destroy can alter the cursor

Active in YR: Conditional on bouncer/meteor impact branch and non-null child refs.

Sibling work settled the row arguments and gates. Scheduler-relevant facts from fresh context:

- `ProcessBounceResult` constructs `BounceAnim` before returning to the parent AI when return code is `1`; the constructor is still `AnimClass::Constructor @ 0x00421EA0`.
- `AnimClass::AI` constructs `ExpireAnim` at `0x00423E70`; assembly context shows the call to `0x00421EA0`.
- After accepted impact handling, normal parent cleanup reaches `AnimClass::Destroy @ 0x004255B0`, which calls `ObjectClass::UnInit @ 0x005F65F0`.

The children are appended before the parent is uninitialized in the accepted-impact order. If parent unregistration removes the current active-vector entry, `FUN_0055BAE0` compacts the vector and `LogicClass::PerTickUpdate` does not repair the cursor. Same-pass child reachability therefore depends on the composed append order and compacting removal, not on a special bouncer queue.

### Active-vector removal is compacting and interacts with same-pass scheduling

Active in YR: Yes.

`FUN_0055BAE0 @ 0x0055BAE0` first checks `ObjectClass+0x98`; if clear, it returns with no vector lookup. If set, it finds the pointer, decrements count, and shifts later entries left. Fresh assembly context:

- `0x0055BAE7..0x0055BAEF`: read/test `ObjectClass+0x98`.
- `0x0055BAF1..0x0055BAFD`: find index through the vector helper.
- `0x0055BB09..0x0055BB21`: decrement count and shift later entries left.

`ObjectClass::Conceal` is the ordinary lifecycle caller under object type and game-mode gates; `ObjectClass::UnInit` calls a limbo/conceal virtual before queuing pending delete. `AnimClass::Destroy @ 0x004255B0` then reaches `ObjectClass::UnInit @ 0x005F65F0`. The scheduler increments its index after the parent AI returns, so self-removal can skip the object shifted into the old current index. This is active YR behavior for live objects that unregister during their `vtable+0x5C` call.

## Current Rust Surface

Active in YR comparison relevance: Yes, this is the Rust-facing parity gap for generic AnimClass-like runtime.

- `src/sim/components.rs:769..790` has `AnimClassSpawnDescriptor` preserving constructor row fields, but this is a descriptor attached to `WorldEffect`, not a globally scheduled `AnimClass`.
- `src/sim/components.rs:823..923` ticks `WorldEffect` as a retained vector of one-shot visual effects. It has no `ObjectClass+0x98` membership byte, no live LogicClass cursor, no same-pass append semantics, and no generic trailer/bouncer child scheduling.
- `src/sim/world/mod.rs:612..620` has a scoped `live_object_order` surrogate with append and `retain` removal, but `src/sim/world/mod.rs:622..628` exposes snapshots and `advance_tick` remains a phased pipeline rather than one global live object pass.
- `src/sim/world/mod.rs:1826..1840` advances `world_effects` at the end of the fixed tick by `retain_mut`, so children appended earlier in native object AI would not naturally be visited through a native-equivalent live object cursor.
- `src/app_building_anim.rs:776..840` spawns garrison flashes then advances them immediately through an app vector; `src/app_building_anim.rs:892..903` does model a first-AI guard for that app-side runtime, but this is not a generic global AnimClass scheduler.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Revealed `AnimClass` children append to the live `LogicClass` vector through `ObjectClass::Reveal -> FUN_0055BAA0`, and the scheduler reloads count after each `vtable+0x5C`. | Generic `WorldEffect`/descriptor paths are retained visual vectors, not live object-vector members. | Future generic `AnimClass` runtime, `src/sim/components.rs`, `src/sim/world/mod.rs`. | Parent anim at active-list index 0 emits a trailer child before the scheduler reaches the old tail; child receives an AI visit in the same global tick, but delay/first-visit rules decide visible work. | `anim_scheduler_tail_appended_trailer_child_is_same_pass_eligible` | High: forcing all children to next tick changes projectile smoke, debris, and chained anim timing. |
| Bouncer impact can append `BounceAnim` and `ExpireAnim` children before parent cleanup, and cleanup can compact/remove the current parent from the same active vector. | No generic active-list cursor or compacting self-removal semantics for anims; `world_effects.retain_mut` is not equivalent. | Future bouncer runtime, active object scheduler, pending-delete/despawn surfaces. | Active order `Parent, Sibling`; parent impact emits two children then destroys itself. The scheduler applies native compacting/index semantics: the shifted successor behavior and child reachability match the live-vector rule. | `anim_bouncer_parent_destroy_uses_live_vector_compaction_semantics` | High: append-then-remove ordering changes which child/sibling receives same-frame AI. |
| `g_AnimClass_Array` preserves registry/lifetime identity but is not the ordinary AI traversal. | Current descriptors preserve rows but not object identity, owner scans, membership byte, pending delete, or live traversal. | Generic `AnimClass` pool or runtime registry design. | Two child anims created in one parent AI are stored in constructor order for registry/owner-scan purposes, but their AI order comes from active-list insertion. | `anim_registry_order_is_not_used_as_ai_scheduler_order` | Medium-high: building the scheduler over registry order would be a subtle global ordering drift. |

## Negative Facts / Do Not Do

- Do not iterate `g_AnimClass_Array` as the ordinary `AnimClass::AI` scheduler. Evidence: active scheduler loop is `LogicClass+0x04/+0x10` at `0x0055B608..0x0055B619`; constructor registry append is separate at `0x00422092..0x004220A7`.
- Do not make all spawned trailer/bouncer/expire children wait until the next tick. Evidence: live count reload at `0x0055B613` allows tail appends to be reached same pass.
- Do not claim same-pass insertion means same-pass visible frame advance. Evidence: child lifecycle still uses constructor delay, first-AI guard, and child type fields; trailer child delay is already verified as `1`.
- Do not model active-list removal as `swap_remove`. Evidence: `FUN_0055BAE0` shifts later entries left at `0x0055BB11..0x0055BB21`.
- Do not collapse `ObjectClass+0x98` active-list membership into storage membership or `InLimbo`. Evidence: add/remove helpers gate on `+0x98`; `ObjectClass+0x81` is separate limbo state in reveal/conceal reports.

## Remaining Uncertainty

- Concrete vector indices for a live stock trailer/meteor scenario require runtime logging; the static cursor rule is complete, but a replay-specific "this child ticks this frame" claim needs the parent's live index and current vector count.
- Same-pass visible work for delay-zero children depends on the exact generic first-AI guard subset and child type lifecycle; this report only proves scheduler eligibility.
- Save/load reconstruction of `AnimClass` active-list membership is not covered here.
- Attached-owner scans through `g_AnimClass_Array` are outside this slot except for the negative fact that the registry is not the AI scheduler.

## Stale Docs / Replacement Wording

- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`: replace any wording that says "for each AnimClass in `g_AnimClass_Array`" for ordinary tick AI with: "AnimClass constructor appends to `g_AnimClass_Array` for registry/lifetime/owner-scan purposes, but ordinary revealed anim AI runs through the live `LogicClass` active-object vector inserted by `ObjectClass::Reveal -> FUN_0055BAA0`; `LogicClass::PerTickUpdate @ 0x0055B608..0x0055B619` reloads live count after every `vtable+0x5C` call."
- Any doc stating "newly spawned trailer children always wait until the next tick" should be replaced with: "newly constructed trailer children append through the normal reveal/register path and are same-pass eligible under the live LogicClass cursor rule; however the trailer row's `delay=1` and the generic lifecycle guard can make the same-pass visit non-visible."
- Any doc stating "newly spawned trailer children always advance immediately" should be replaced with: "same-pass scheduler eligibility is separate from visible advancement; child delay, first-AI guard, rate, and type lifecycle fields decide whether the same-pass visit changes the rendered frame or side effects."

## Open Questions Log

- `[RESOLVED] OQ-01 - Does AnimClass constructor append to g_AnimClass_Array? -> Yes.` Evidence: `0x00422092..0x004220A7`.
- `[RESOLVED] OQ-02 - Is g_AnimClass_Array the ordinary AI scheduler? -> No.` Evidence: revealed objects register into `LogicClass` at `0x005F5038..0x005F5040`, and `LogicClass::PerTickUpdate` calls `vtable+0x5C` from `LogicClass+0x04`.
- `[RESOLVED] OQ-03 - Can an AnimClass child appended during parent AI run in the same tick? -> Yes, conditionally, when the live cursor has not passed the appended tail.` Evidence: count reload at `0x0055B613`.
- `[RESOLVED] OQ-04 - What happens when parent Destroy/UnInit removes the current object? -> Remover compacts left; scheduler does not repair index.` Evidence: `0x0055BB09..0x0055BB21` plus `0x0055B610..0x0055B619`.
- `[DEFERRED] OQ-05 - What exact stock replay indices make a trailer child visible or non-visible in the same frame?` Category: runtime-index logging.

## Sources

- Fresh read-only Ghidra decompile: `AnimClass::Constructor @ 0x00421EA0`.
- Fresh read-only Ghidra decompile: `ObjectClass::Reveal @ 0x005F4EC0`.
- Fresh read-only Ghidra decompile: `FUN_0055BAA0 @ 0x0055BAA0`.
- Fresh read-only Ghidra decompile: `LogicClass::PerTickUpdate @ 0x0055AFB0`.
- Fresh read-only Ghidra decompile: `FUN_0055BAE0 @ 0x0055BAE0`.
- Fresh read-only Ghidra decompile: `AnimClass::Destroy @ 0x004255B0`; `ObjectClass::UnInit @ 0x005F65F0`.
- Fresh read-only assembly contexts: `0x00422058`, `0x00422092`, `0x005F5038`, `0x0055BAA0`, `0x0055B608`, `0x0055BAE0`, `0x0055BB09`, `0x0042431D`, `0x00423E70`.
- Prior reports reconciled: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`, `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`, `SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`, `ANIMCLASS_AI_TRAILER_NEXT_INTERACTION_GHIDRA_REPORT.md`, `ANIMCLASS_BOUNCER_IMPACT_GATES_GHIDRA_REPORT.md`.
- Rust static scan: `src/sim/components.rs`, `src/sim/world/mod.rs`, `src/app_building_anim.rs`.

## Status

COMPLETE for the scoped `AnimClass` global registration / same-pass scheduler slice.
