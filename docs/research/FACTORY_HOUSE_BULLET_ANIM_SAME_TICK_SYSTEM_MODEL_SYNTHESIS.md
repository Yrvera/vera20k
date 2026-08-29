# Factory / House / Bullet / Anim Same-Tick Tail - System Model Synthesis

> **2026-08-29 correction:** this synthesis's `defeat/scatter` shorthand is
> stale for `0x004FC6D0`. That House tail calls the live-Techno destruction/C4
> receiver sweep documented in
> `docs/gap-scans/2026-08-29-disparity-scan-action-119-house-destruction.md`,
> not movement Scatter. Its independent scheduler-order findings are unchanged.

**Date:** 2026-05-28  
**Scope:** PerTick tail ordering for tactical/factory/house work, production completion versus house update, and same-tick projectile/animation lifecycle effects through the live object scheduler.  
**Non-scope:** complete FactoryClass insertion/rebuild order, full HouseClass AI formulas, exhaustive projectile family timing, all AnimClass constructor caller rows, and final draw-layer pixel ordering.  
**Output type:** conflict-aware model-synthesis.  
**Overall status:** implementation-safe for global tail order and common live-vector same-tick removal mechanics; partial-ready for production/projectile/anim implementation because several family-specific mappings remain blocked.

## Evidence Ladder Used

| Rank | Meaning in this synthesis |
|---|---|
| BINARY_HIGH | Direct Ghidra function body with active YR caller/gate/default |
| RESEARCH_HIGH | Recent focused report or re-swarm with exact addresses and handoff |
| TRACE_HIGH | Runtime trace tied to binary evidence |
| DOC_SYNTHESIS | Older overview prose; unsafe when contradicted |
| INFERENCE | Plausible but not implementation-safe |

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| PerTick tail order is tactical, global factories, global houses, then last-ref-object. | `PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`; `FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Factory progress runs before HouseClass update. | factory/house order report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Native global factory loop is not equivalent to Rust per-owner category queue iteration. | factory/house order report | confirmed | high | n/a | IMPLEMENTATION_SAFE as mismatch fact |
| HouseClass update includes per-house superweapon ready/low-power and defeat/AI management timing; LightningStorm process is a separate earlier global call. | factory/house and global-order reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Bullet and Anim objects can unregister during their own AI; compacting removal can skip shifted successors. | `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`; scheduler reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| A tail-appended logic object can run later in the same object scheduler pass. | `SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`; scheduler reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| AnimClass has first-AI no-advance guard and exact frame/rate/loop fields. | `ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md` | confirmed | high | yes | IMPLEMENTATION_SAFE for generic anim lifecycle subset |
| Current Rust app-layer world effects and flashes are native `AnimClass` equivalents. | anim lifecycle and global registration reports | disputed/unknown | medium | yes | NEEDS_REINVESTIGATE |
| Exact first-hit timing is known for every projectile/altitude family. | bullet reports | unknown | low | yes/conditional | NEEDS_REINVESTIGATE |
| Exact global FactoryClass array insertion/reconstruction order is complete. | factory/house order report | unknown | medium | yes | NEEDS_REINVESTIGATE |

## Current Model

The PerTick tail is a global order contract, not just a rough "production before AI" shape. After the main object scheduler and later wave/alpha/crate/tactical work, native iterates the global `FactoryClass` array, then the global `HouseClass` array. Factory AI advances production before any house update for that PerTick pass.

House work is not simply Rust high-level AI. Native HouseClass update covers defeat/scatter checks, superweapon ready/low-power handling, and AI production management in that house's tail position. Active LightningStorm processing is not inside the house tail; it is an earlier global PerTick call before the main object vector.

Bullet and Anim same-tick behavior is governed by the same live object scheduler described in the timing model. A projectile or animation that unregisters during its own `vtable+0x5C` turn mutates the active vector immediately. This can make the object that shifts into the current index wait until the next pass. Conversely, a logic object appended to the tail before the scheduler reaches the tail can receive its first AI in the same global frame.

AnimClass also has native lifecycle details independent of scheduler order: constructor fields, delay handling, first-AI guard, frame step, loop byte/sentinel, `Next` in-place transition, and `Rate=0` no-advance semantics. App-layer visual lists cannot be treated as native AnimClass parity without mapping these fields and scheduler membership.

## Implementation-Safe Facts

- Preserve tactical -> factories -> houses -> last-ref-object order for any native-mapped tail work.
- Production completion/progress must be visible to later house updates in the same tail pass.
- Do not run HouseClass-equivalent superweapon ready/AI management before global factory work.
- Keep LightningStorm process separate from per-house superweapon ready work.
- Projectile detonation and Anim expiry that are native object AI should occur inside live object order, not as end-of-phase batches.
- Use compacting active-list removal for projectile/anim/engineer self-uninit paths.
- Implement AnimClass frame advancement only after the first-AI guard and native delay checks; delay-zero construction does not mean same-visit frame advance.
- Preserve `Rate=0`, loop byte/sentinel, `Next` in-place transition, and signed frame-step behavior for generic AnimClass runtime work.

## Doc-Patch-Ready Facts

- Replace "FactoryClass/HouseClass ordering is only production-before-AI" with the full tail placement.
- Replace wording that puts LightningStorm or EMP after houses; native active LightningStorm/EMP placement is earlier than object/tactical/factory/house tail work.
- Replace "spawned effects advance immediately because delay is zero" with first-AI guard language for AnimClass.
- Mark current app-layer visual effect lists as Rust implementation details, not native AnimClass proof.

## Stale Or Superseded Claims

- Older FactoryClass build-speed docs that imply production tick order without the global tail context are stale.
- Older HouseClass docs that compress AI/update/superweapon placement into a broad high-level AI pass are stale for ordering-sensitive work.
- Any projectile/anim implementation plan that batches destruction after the pass is superseded for live object AI paths.

## Cross-Doc Conflicts

The tail-order reports and scheduler reports agree on the high-level order. The main unresolved conflict is representational: Rust currently splits production, AI, superweapon, projectile movement, damage, world effects, and app animations into separate phases/lists. That split is not proven equivalent to native live-vector plus tail semantics.

The AnimClass docs also create an implementation boundary: they verify generic lifecycle fields and some constructor rows, but not every current `WorldEffect` producer's exact native constructor call, owner mutation, and draw-layer role.

## Needs Re-Investigation

- `/re-investigate FactoryClass global array insertion reconstruction order`
  - Needed before same-frame multi-factory completion ordering can be claimed complete.
- `/re-investigate HouseClass MPlayer_Defeated scatter production-management tail side effects`
  - Needed before moving all HouseClass tail behavior into Rust with confidence.
- `/re-investigate projectile first-hit timing by projectile altitude family`
  - Needed for exhaustive bullet same-tick latency outside already verified cases.
- `/re-swarm AnimClass constructor rows for current WorldEffect producers`
  - Needed before replacing app effects with a generic native AnimClass runtime.

## Do-Not-Implement Notes

- Do not use per-owner/category production queue order as proof of global FactoryClass array order.
- Do not collapse LightningStorm process into per-house superweapon ready work.
- Do not process projectile detonation or anim expiry solely as a later cleanup batch when native object AI unregistration matters.
- Do not treat a local bridge/engineer skip fix over stable IDs as a global scheduler replacement.
- Do not play all SHP frames for omitted `End`; native fills from SHP frame count only for explicit `End=-1`.
- Do not consume RNG for AnimClass random rate/loop delay unless the relevant INI keys enable those paths.

## Source Ledger

- `docs/research/PERTICKUPDATE_FULL_ORDERING_LADDER_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_GLOBAL_SUBSYSTEM_ORDER_0055AFB0_GHIDRA_REPORT.md`
- `docs/research/FACTORY_HOUSE_AI_ORDER_VS_RUST_PRODUCTION_AI_GHIDRA_REPORT.md`
- `docs/research/SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `docs/research/LOGICCLASS_LIVE_VECTOR_VS_RUST_ENTITY_PASSES_GHIDRA_REPORT.md`
- `docs/research/BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`
- `docs/research/BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`
- `docs/research/ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`
- `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- `src/sim/world/mod.rs`
- `src/sim/production/production_queue.rs`
- `src/sim/ai.rs`
- `src/sim/movement/homing_movement.rs`
- `src/sim/animation.rs`
- `src/app_building_anim.rs`
