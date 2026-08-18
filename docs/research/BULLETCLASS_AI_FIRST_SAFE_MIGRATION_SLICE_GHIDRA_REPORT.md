# BulletClass AI First Safe Migration Slice - Ghidra Research Report

**Address(es):** `BulletClass::AI @ 0x004666E0`; `BulletClass::Fire @ 0x00468670`; `LogicClass::PerTickUpdate @ 0x0055AFB0`; `ObjectClass::Reveal @ 0x005F4EC0`; `FUN_0055BAA0 @ 0x0055BAA0`; `FUN_0055BAE0 @ 0x0055BAE0`; `BulletClass::BulletDetonation @ 0x00468D80`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** first safe Rust migration slice for `BulletClass::AI` under the native live-object scheduler: scheduler membership, same-pass first AI timing, detonation/removal implications, and current Rust projectile surfaces.  
**Non-Scope:** full projectile movement math, full WarheadType damage math, full AnimClass/VoxelAnimClass migration, and full NUKE listener implementation.  
**Confidence:** High for scheduler/membership/removal order; Medium-High for the first-slice recommendation because final Rust ownership boundaries are still implementation work.  
**Active in YR:** Yes. The covered path is standard for fired projectiles; stock `[GGI] Secondary=MissileLauncher -> [MissileLauncher] Projectile=AAHeatSeeker2 -> [AAHeatSeeker2] ROT=60` is one active example (`ini/rulesmd.ini:3863`, `3868`, `22569`, `22574`, `25678`, `25687`).

---

## 0. Working Notes Gate

**Target question:** What is the first safe Rust migration slice for `BulletClass::AI` under the native live-object scheduler?

**Non-goals:** Do not implement Rust; do not re-derive full projectile math; do not expand into full TechnoClass/AnimClass migration; do not rewrite stale docs outside the allowed new report/shared claims file.

**Evidence needed to mark COMPLETE:** scheduler membership proof, same-pass first AI timing proof, detonation/removal ordering proof, current Rust projectile surface scan, and at least one implementation handoff with a concrete test-name proposal.

**Stop conditions:** stop after the first migration slice is defined and blockers are explicit; defer full math, full nuke listener, and full child-object AI migration to follow-up slices.

---

## 1. Overview

The first safe migration slice is not projectile math. It is a BulletClass runtime/scheduler slice: make bullets real live objects whose AI is dispatched from the native-style live vector, with Fire/Reveal registration and inline UnInit/removal semantics. Native `BulletClass::AI` is reached through `LogicClass::PerTickUpdate` `vtable+0x5C`, not through a projectile-specific global movement phase.

Rust already has the needed `LogicVector` primitives (`register_live_object`, `unregister_live_object`, `for_each_live_object`), but `advance_tick` still runs `rocket_movement` and `homing_movement` as staged sorted-entity passes before combat, while combat often applies damage directly and emits render-side `fire_events` instead of creating authoritative BulletClass objects.

---

## 2. Scheduler Membership And First-AI Timing

| Finding | Active in YR | Evidence | Migration implication |
|---|---|---|---|
| `BulletTypeClass` instances are logic-enabled by default. | Yes; constructor runs during rules load. | `BulletTypeClassConstructorDefaults @ 0x0046BBC0` stores byte `+0x234 = 1` as `*(undefined1 *)(param_1 + 0x8d) = 1`. | Bullet entities must be eligible for `LogicClass` scheduling when revealed. |
| A constructed bullet is not in the live scheduler until Fire/Reveal. | Yes; every fired projectile goes through allocate/init/fire. | Prior report `BULLETCLASS_CONSTRUCTION_POOL_REGISTRY_MEMBERSHIP_INIT_RESWARM_20260528.md`: Object constructor leaves `Object+0x98=0`; `g_BulletClass_Array` is a save/load registry, not the scheduler. | EntityStore ownership is not scheduler membership. Do not tick all stored bullets. |
| `BulletClass::Fire` calls `ObjectClass::Reveal` at the top of Fire, then fills velocity/source/target fields and submits display. | Yes; standard weapon-fire path. | Decompile `BulletClass::Fire @ 0x00468670`; assembly `0x00468684 CALL 0x005F4EC0`, then field copies begin at `0x00468694..0x004686A0`; arm setup at `0x00468A3F..0x00468A63`. | Registration is inside Fire, not constructor/init. Fire runs inside the current object's AI call, so the new bullet cannot receive AI until Fire returns, but can still run later in the same scheduler pass. |
| `ObjectClass::Reveal` appends logic-enabled objects to `LogicClass`. | Yes in active YR game modes. | Assembly `ObjectClass::Reveal @ 0x005F5038..0x005F5040`: pushes object, sets `ECX=0x87F778`, calls `FUN_0055BAA0`. `FUN_0055BAA0 @ 0x0055BAB5..0x0055BAC6` calls dynamic-vector insert then sets `Object+0x98=1`. | Bullet registration must be a tail append with membership guard, not sorted insertion. |
| `LogicClass::PerTickUpdate` dispatches `vtable+0x5C` and reloads live count after each object. | Yes; this is the main active-object loop. | Decompile `LogicClass::PerTickUpdate @ 0x0055AFB0`; assembly `0x0055B608` loads item pointer, `0x0055B610 CALL [EDX+0x5C]`, `0x0055B613 MOV EAX,[EDI+0x10]`, `0x0055B617 CMP ESI,EAX`, `0x0055B619 JL 0x0055B608`. | A bullet fired by an object earlier in the pass can run `BulletClass::AI` later in that same pass if appended before the cursor exits. |

**First-AI timing verdict:** Active in YR: Yes. A fired bullet is not class-design next-frame queued. It is same-pass eligible when Fire/Reveal happens during the live-vector pass and the cursor has not reached the new tail.

---

## 3. Detonation And Removal Implications

| Finding | Active in YR | Evidence | Migration implication |
|---|---|---|---|
| Normal detonation work completes before bullet self-removal. | Yes; standard detonation path. | `BulletClass::AI` decompile calls `BulletClassBulletDetonationImpactDamage(local_1a0)` before dispatching `(*vtable+0xF8)()`. Assembly `0x00467FA2 CALL 0x00468D80`, then `0x00467FAF MOV EAX,[EBP]`, `0x00467FB4 CALL [EAX+0xF8]`. | Damage/effects/sub-bullets must be emitted before the bullet unregisters. |
| `BulletClass::BulletDetonation` calls `WarheadTypeClass::Detonate`; Airburst differs from Cluster. | Yes, conditional on BulletType flags. | Decompile `0x00468D80`: non-Airburst loops `WarheadTypeClass__Detonate()` up to `BulletType+0x2AC` while bullet alive; Airburst calls `WarheadTypeClass__Detonate()` once. | Do not model Cluster as spawning child BulletClass instances; child bullets come from Airburst inside Warhead detonation. |
| Bullet removal is live-vector compaction, not swap-remove, and happens inside AI. | Yes. | `FUN_0055BAE0 @ 0x0055BAE0`; assembly `0x0055BB09..0x0055BB21` decrements count and shifts later entries left; `0x0055BB27` clears `Object+0x98`. Prior report verifies Bullet vtable+0xF8 is `ObjectClass::UnInit @ 0x005F65F0`. | Inline removal skips the shifted immediate successor this pass. Snapshot projectile phases cannot reproduce this. |
| Detonation children are appended before parent bullet UnInit and can be same-pass eligible. | Yes, conditional on warhead/Anim/BulletType flags. | Prior sibling report `BULLETCLASS_DETONATION_SAMEPASS_CHILD_SPAWN_ORDER_RESWARM_20260528.md`: assembly `0x00467FA2..0x00467FB4`, decompile `0x00468D80`, Warhead detonation child spawn sites. | First BulletClass slice must not post-process children in a late batch and call that native-equivalent. |
| The NUKE delayed-detonation branch keeps the bullet alive in the scheduler while waiting on an anim pointer. | Conditional on bullet type name `"NUKE"`; stock YR path exists. | Prior report `BULLETCLASS_DELAYED_DETONATION_ANIM_LISTENER_PATH_RESWARM_20260528.md`; fresh `BulletClass::AI @ 0x004666E0` decompile shows top branch `+0x158` flag and `+0x154` anim pointer early-return. | Do not use nuke projectiles as the first simple migration fixture. |

---

## 4. Current Rust Implementation Status

| Rust surface | Status | Evidence | Delta from native |
|---|---|---|---|
| Live scheduler primitive | Present but underused for object AI. | `src/sim/world/mod.rs:679..770` implements tail append, compacting unregister, and `for_each_live_object` with count reload. | Good foundation. The migration should use this instead of sorted projectile snapshots. |
| `advance_tick` projectile movement | Staged before combat. | `src/sim/world/mod.rs:1601..1607` runs `rocket_movement::tick_rocket_movement` and `homing_movement::tick_homing_movement` in Phase 2 before Phase 5 combat. | Native BulletClass AI runs inside the live-vector pass and can be appended during Techno AI. |
| Homing missiles | Snapshot over sorted entity IDs. | `src/sim/movement/homing_movement.rs:379..387` collects `entities.keys_sorted()`; returns detonated IDs to caller. | Cannot model same-pass append, inline removal, or shifted-successor skip. |
| Rockets | Snapshot over sorted entity IDs. | `src/sim/movement/rocket_movement.rs:133..141` collects `keys_sorted()`; detonation is pushed at `219..226`. | Same scheduler drift as homing. |
| Combat fire/damage | Often direct damage/effect emission, not BulletClass object creation. | `src/sim/combat/mod.rs:1530..1543` accumulates damage/fire/effects; `1967..1980` pushes direct damage events; `2028..2039` emits warhead effects; `2053..2062` emits `SimFireEvent`. | Native visible projectiles are live BulletClass objects; direct damage bypasses first-AI and detonation timing. |
| Explosion/warhead visuals | `WorldEffect` retained late, not live objects. | `src/sim/world/mod.rs:1467..1482` ages `world_effects` with `retain_mut`; `1880..1902` pushes combat explosion effects. | Native AnimClass children can be live-vector objects with same-pass AI; current effects cannot. |
| Direct entity removal | Still exists inside combat. | `src/sim/combat/mod.rs:1003..1010` clears occupancy/radio and calls `entities.remove(dead_id)` directly. | Outside this slot's target, but it blocks exact inline UnInit semantics when bullet detonation kills objects. |

---

## 5. First Safe Migration Slice

**Recommended slice:** implement a BulletClass AI scheduler shell before replacing projectile math.

Required scope:

1. Represent authoritative bullet entities as stored objects separate from live-vector membership. They enter storage at allocation-equivalent time but enter `LogicVector` only at Fire/Reveal-equivalent time.
2. Dispatch bullet entities from `Simulation::for_each_live_object` or an equivalent vtable-style per-object branch, with an `is_bullet`/projectile-kind branch calling one-bullet AI.
3. At first, allow the AI shell to delegate limited movement to existing `homing_state`/`rocket_state` behavior, but call it one bullet at a time from the live scheduler, not from `keys_sorted()` staged sweeps.
4. Inline the detonation/removal timing contract in the shell: detonation side effects before `uninit`; `uninit` compacts the live vector during the AI call.
5. Exclude nuke listener, Airburst sub-bullet spawning, and full AnimClass/VoxelAnimClass live AI from the first fixture unless their child/scheduler semantics are explicitly modeled. Use AAHeatSeeker2-style non-nuke homing as the first active-YR fixture.

---

## 6. Negative Facts / Do Not Do

| Negative fact | Active in YR | Evidence |
|---|---|---|
| Do not treat `g_BulletClass_Array`/EntityStore iteration as the AI scheduler. | Yes. | `BULLETCLASS_CONSTRUCTION_POOL_REGISTRY_MEMBERSHIP_INIT_RESWARM_20260528.md`; `ObjectClass::Reveal @ 0x005F5038..0x005F5040` appends to `0x87F778`, not `g_BulletClass_Array`. |
| Do not create a next-tick projectile queue for bullets fired during object AI. | Yes. | `LogicClass::PerTickUpdate` assembly `0x0055B610..0x0055B619` reloads count after `vtable+0x5C`; `BulletClass::Fire @ 0x00468670` calls Reveal. |
| Do not run BulletClass movement from `EntityStore::keys_sorted()`. | Yes. | Rust `homing_movement.rs:386..387` and `rocket_movement.rs:140..141` use sorted snapshots; native `FUN_0055BAA0` tail-appends and `FUN_0055BAE0` compacts left. |
| Do not remove a detonating bullet before damage/effects/sub-bullets are emitted. | Yes. | `0x00467FA2 CALL 0x00468D80` precedes `0x00467FB4 CALL [EAX+0xF8]`. |
| Do not model Cluster as real BulletClass child objects. | Yes, conditional on `Airburst=no`. | `BulletClass::BulletDetonation @ 0x00468D80` loops `WarheadTypeClass__Detonate()` and random scatter for Cluster; real sub-bullets are the Airburst path per prior airburst report. |
| Do not use NUKE as the first simple BulletClass fixture. | Conditional on `"NUKE"` bullet type. | Prior nuke listener report and `BulletClass::AI @ 0x004666E0` decompile show anim-listener delay via `+0x154/+0x158`. |

---

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| BulletType logic-enabled default | verified | `BulletTypeClassConstructorDefaults @ 0x0046BBC0`, store to `+0x234` | none |
| Bullet construction vs scheduler membership | verified by prior report | `BULLETCLASS_CONSTRUCTION_POOL_REGISTRY_MEMBERSHIP_INIT_RESWARM_20260528.md` | no redo; accepted settled fact |
| Fire/Reveal registration gate | verified | `BulletClass::Fire @ 0x00468670`; assembly `0x00468684`, `0x00468694`; `ObjectClass::Reveal @ 0x005F5038..0x005F5040` | exact observability of `+0x98` during the rest of Fire is deferred |
| LogicClass count-reload same-pass rule | verified | `LogicClass::PerTickUpdate @ 0x0055AFB0`; assembly `0x0055B608..0x0055B619` | runtime cursor index for a match scenario needs debugger |
| Normal BulletClass AI detonation/removal order | verified | `BulletClass::AI @ 0x004666E0`; assembly `0x00467FA2..0x00467FB4` | full non-impact exit census covered by sibling doc |
| Rust projectile staged passes | verified by source scan | `src/sim/world/mod.rs:1601..1607`; `homing_movement.rs:379..387`; `rocket_movement.rs:133..141` | no implementation in this report |
| Rust direct combat damage/fire event path | verified by source scan | `src/sim/combat/mod.rs:1967..1980`, `2028..2039`, `2053..2062` | exact replacement design belongs to implementation contract |

---

## 8. Open Questions - Final State

- `[RESOLVED] OQ-BULLET-SLICE-001 - Where does BulletClass get scheduler membership? -> Fire/Reveal, not constructor/registry.` (evidence: `0x00468670`, `0x005F5038..0x005F5040`, prior construction report)
- `[RESOLVED] OQ-BULLET-SLICE-002 - Can the first AI run in the same native tick? -> Yes, when Fire appends during the live-vector pass and the cursor later reaches the tail.` (evidence: `0x0055B610..0x0055B619`)
- `[RESOLVED] OQ-BULLET-SLICE-003 - Does detonation happen before self-removal? -> Yes; `0x00468D80` is called before `vtable+0xF8`.` (evidence: `0x00467FA2..0x00467FB4`)
- `[RESOLVED] OQ-BULLET-SLICE-004 - Can current Rust staged projectile passes model same-pass append/remove? -> No; homing/rocket use sorted snapshots and return detonation IDs.` (evidence: `src/sim/movement/homing_movement.rs:379..387`, `src/sim/movement/rocket_movement.rs:133..141`)
- `[RESOLVED] OQ-BULLET-SLICE-005 - Is `WorldEffect` a native AnimClass scheduler substitute? -> No; it is retained and aged late, not a live object appended through Reveal.` (evidence: `src/sim/world/mod.rs:1467..1482`, `1880..1902`; native child report)
- `[DEFERRED] OQ-BULLET-SLICE-006 - Exact BulletClass movement math for ROT<=0 and ROT>0.` (category: out-of-scope; reason: target is first safe migration slice, not full math; next-step-if-pursued: implementation contract from `BULLET_CLASS_AI_GHIDRA_REPORT.md` and AAHeatSeeker2 exact math docs)
- `[DEFERRED] OQ-BULLET-SLICE-007 - Full Airburst child bullet migration.` (category: requires-different-system-context; reason: needs WarheadType/AnimClass child scheduler ownership; next-step-if-pursued: combine this report with Airburst and AnimClass first-slice reports)
- `[DEFERRED] OQ-BULLET-SLICE-008 - NUKE listener exact callback and anim lifetime fixture.` (category: requires-different-system-context; reason: nuke path depends on AnimClass remove-listener roster; next-step-if-pursued: use `BULLETCLASS_DELAYED_DETONATION_ANIM_LISTENER_PATH_RESWARM_20260528.md`)

---

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Bullet Fire/Reveal tail-appends a logic-enabled bullet, and `LogicClass::PerTickUpdate` can dispatch its `vtable+0x5C` later in the same pass. Active in YR: Yes. | Decompile `0x00468670`; assembly `0x00468684`, `0x005F5038..0x005F5040`, `0x0055B610..0x0055B619`; BulletType `+0x234=1` at `0x0046BBC0`. | Rust has `for_each_live_object`, but `advance_tick` uses staged projectile snapshots and combat direct damage/events. | `src/sim/world/mod.rs:679..770`, `1508..1777`; `src/sim/combat/mod.rs`; future bullet entity spawn path. | Register bullet at Fire-equivalent reveal and dispatch bullet AI from live-vector iteration, not from `keys_sorted()` projectile phases. | Logic order `[techno, observer]`; techno AI fires bullet; bullet tail-appends and receives its first bullet AI later in the same scheduler pass if cursor reaches the tail. | `bullet_fire_tail_append_runs_first_ai_same_pass` | High. A next-tick queue adds one frame of projectile/damage latency for common missile shots. |
| Bullet detonation emits detonation effects/warhead work before `vtable+0xF8`; UnInit then compacts the live vector during the AI call. Active in YR: Yes. | Assembly `0x00467FA2..0x00467FB4`; `FUN_0055BAE0 @ 0x0055BAE0`, assembly `0x0055BB09..0x0055BB27`; prior vtable+0xF8 report. | Rust homing/rocket return detonation IDs; combat death/effects are handled by caller-side batches and direct `entities.remove` remains in combat. | `src/sim/movement/homing_movement.rs:379..569`; `src/sim/movement/rocket_movement.rs:133..260`; `src/sim/combat/mod.rs:1003..1010`; `src/sim/world/mod.rs:817..845`. | A bullet AI call must perform detonation side effects first, then call the centralized uninit/despawn path during the live-vector pass. | Active vector `[A, bullet, B, C]`; bullet detonates, emits effect, uninitializes; B shifts into bullet's old slot and is skipped, C remains reachable according to native compaction/count reload. | `bullet_ai_detonation_uninit_skips_shifted_successor` | High. Post-pass despawn cannot reproduce cursor effects or same-pass child ordering. |
| Explosion AnimClass/Airburst children are spawned before parent bullet UnInit and may be same-pass eligible, but they require their own live-object representation. Active in YR: Conditional on warhead/BulletType. | `BulletClass::BulletDetonation @ 0x00468D80`; sibling child-spawn report; native scheduler assembly `0x0055B613`. | Rust `WorldEffect` is retained late and Airburst sub-bullets are not implemented. | `src/sim/components.rs:823..923`; `src/sim/world/mod.rs:1467..1482`, `1880..1902`; future AnimClass/BulletClass child spawn surfaces. | First slice may record child append requests but must not claim native same-pass child AI until AnimClass/sub-bullet live objects exist. | Bullet detonates with AirburstWeapon: parent emits sub-bullet spawn requests before uninit; follow-up migration asserts sub-bullets append to tail and can receive AI same pass. | `bullet_airburst_children_append_before_parent_uninit` | Medium-High. Implementing children as a late batch changes chain timing; full fix belongs to AnimClass/Airburst follow-up. |

---

## 10. Stale Docs / Follow-up Docs

Found stale wording relevant to future BulletClass math contracts:

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BULLET_CLASS_AI_GHIDRA_REPORT.md`: replace Key Constants row `RulesClass+0x5A0 | FlightLevel | Max altitude for lost-target detonation` with `RulesClass+0x5A0 | MissileSafetyAltitude | lost-target homing detonation threshold; FlightLevel is a separate Rules field used by aircraft flight AI`.
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`: replace "RulesClass+0x5A0 = FlightLevel" and "target coordinate is sentinel and bullet height is at least Rules.FlightLevel" with "RulesClass+0x5A0 = MissileSafetyAltitude; sentinel/lost-target homing detonation compares GetHeight() against MissileSafetyAltitude."

No stale-doc file was modified in this slot.

---

## Sources

- Ghidra read-only decompile/assembly:
  - `LogicClass::PerTickUpdate @ 0x0055AFB0`; assembly `0x0055B608..0x0055B619`
  - `BulletTypeClass::Constructor @ 0x0046BBC0`
  - `BulletClass::Fire @ 0x00468670`; assembly `0x00468684`, `0x00468A3F..0x00468A63`
  - `ObjectClass::Reveal @ 0x005F4EC0`; assembly `0x005F5038..0x005F5040`
  - `FUN_0055BAA0 @ 0x0055BAA0`; assembly `0x0055BAB5..0x0055BAC6`
  - `FUN_0055BAE0 @ 0x0055BAE0`; assembly `0x0055BB09..0x0055BB27`
  - `BulletClass::AI @ 0x004666E0`; assembly `0x00467FA2..0x00467FB4`
  - `BulletClass::BulletDetonation @ 0x00468D80`
- Prior reports:
  - `BULLETCLASS_CONSTRUCTION_POOL_REGISTRY_MEMBERSHIP_INIT_RESWARM_20260528.md`
  - `AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
  - `BULLETCLASS_DETONATION_SAMEPASS_CHILD_SPAWN_ORDER_RESWARM_20260528.md`
  - `BULLETCLASS_VTABLE_F8_TEARDOWN_REMOVAL_PATH_RESWARM_20260528.md`
  - `BULLETCLASS_DELAYED_DETONATION_ANIM_LISTENER_PATH_RESWARM_20260528.md`
  - `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`
- INI:
  - `ini/rulesmd.ini:3863`, `3868`, `22569`, `22574`, `22575`, `25678..25687`
  - `ini/artmd.ini:14755..14760`
- Rust source scanned:
  - `src/sim/world/mod.rs`
  - `src/sim/world/logic_vector.rs`
  - `src/sim/movement/homing_movement.rs`
  - `src/sim/movement/rocket_movement.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/entity_store.rs`
