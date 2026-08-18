# Common Mid-Pass Unregister / Despawn Cases - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, remover `0x0055BAE0`, `ObjectClass::Conceal @ 0x005F4D30`, `ObjectClass::UnInit @ 0x005F65F0`, `BulletClass::AI @ 0x004666E0`, `BulletClass::BulletDetonation @ 0x00468D80`, `AnimClass::AI @ 0x00423AC0`, `AnimClass::Destroy @ 0x004255B0`, `VoxelAnimClass::AI @ 0x00749F30`, `InfantryClass::AI @ 0x0051BAB0`, `InfantryClass::DoType_Sequencer @ 0x00520AE0`, `InfantryClass::Mission_Capture @ 0x005202F0`
**Investigation Mode:** coverage-map
**Claimed Scope:** concrete common active-YR `vtable+0x5C` object-AI bodies that can unregister/despawn themselves during the live `LogicClass::PerTickUpdate` main object-vector pass, plus the scheduler-visible consequence of current/self removal. Covered examples are bullets, `AnimClass`, `VoxelAnimClass`, infantry corpse/special sequence cleanup, engineer capture/bridge-repair consumption, and garrison-entry limbo evidence from existing Ghidra-backed reports.
**Non-Scope:** exhaustive census of every class `vtable+0x5C`, every target-destruction side effect of warhead damage, full building destruction call graph, pending-delete drain timing, save/load vector reconstruction, and runtime measurement of concrete object indices on a retail map.
**Confidence:** High for scheduler/remover/Conceal/UnInit mechanics and the four self-removing AI families directly decompiled; Medium for target-object removal during bullet damage because this slot did not drain every warhead/ReceiveDamage target class.
**Active in YR:** Yes. The scheduler is called from standard `Main_Tick`; the scoped object classes and INI/data cases are stock YR gameplay paths.

## 0. Investigation Contract

**Target question.** Which common active-YR `vtable+0x5C` AI bodies unregister/despawn themselves, or can remove other active objects, during the main live `LogicClass::PerTickUpdate` object-vector pass, and what order consequence must Rust preserve?

**Non-goals.** Do not exhaust every class. Do not implement Rust. Do not mutate Ghidra. Do not rewrite existing docs except this report and `.swarm-claims.md`. Do not treat a deferred target-damage call graph as proved.

**Evidence needed to mark COMPLETE.**

- Direct scheduler evidence: live forward object-vector loop, no post-call index repair, and compacting remover.
- Direct lifecycle evidence: `UnInit -> Conceal -> remover` for logic-enabled objects.
- At least three frequent self-removing `vtable+0x5C` bodies verified by decompile plus scheduler composition.
- At least one infantry/engineer/garrison-related common case.
- Rust-facing line references showing current snapshot/phased/removal behavior.

**Stop conditions.**

- Stop after bullets, anims, voxel anims/debris, infantry/engineer/garrison are covered enough to justify scheduler semantics.
- Stop if a candidate requires draining a whole damage or building-destruction subsystem; record it as deferred.
- Stop if a Ghidra path is TS-only or stock YR-inactive; report as negative.

## 1. Overview

`LogicClass::PerTickUpdate` walks its main object vector live. When a currently visited object calls a removal path that compacts that vector, the scheduler increments the index after the AI call and can skip the object shifted into the just-processed slot. This is not theoretical: common active-YR bullets, anims, voxel anims, infantry death/special sequences, and engineers all have normal `vtable+0x5C` paths that call `vtable+0xF8`/`UnInit` or equivalent limbo/removal during their own AI.

The key implementation consequence is that "collect IDs, process each, despawn later" is not a parity-neutral rewrite for these cases. It changes whether the immediate successor runs in the same native pass.

## 2. Scheduler And Removal Mechanics

| Mechanism | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Main object pass | The loop loads `items[i]`, calls `vtable+0x5C`, increments `i`, then reloads live count at `LogicClass+0x10`. | `0x0055B608..0x0055B619` decompile/disassembly in scheduler report; fresh `0x0055AFB0` decompile in this slot. | Yes |
| No index repair | After object AI returns, scheduler does not compare the old pointer, find the new index, or retry shifted entries. | `0x0055B610..0x0055B619`. | Yes |
| Remover compacts | `FUN_0055BAE0` checks `Object+0x98`, finds index, decrements count, shifts later pointers left, and clears `Object+0x98`; no stale-tail zeroing. | Decompile `0x0055BAE0`; assembly `0x0055BB09..0x0055BB27`. | Yes |
| Conceal unregisters | `ObjectClass::Conceal` calls `FUN_0055BAE0` when the object's type is logic-enabled (`type+0x234`) and then sets `InLimbo` at `+0x81`. | Decompile `0x005F4D30`, call site before `+0x81=1`. | Yes, conditional on normal game-active and logic-enabled type gates |
| UnInit reaches Conceal | `ObjectClass::UnInit` calls `Detach_From_All_Lists`, virtual `+0xD4`, clears alive at `+0x90`, then appends to pending-delete. For normal object paths, `+0xD4` reaches `ObjectClass::Conceal`/derived limbo wrappers. | Decompile `0x005F65F0`; prior lifecycle docs. | Yes |

**Scheduler-visible consequence.** If vector is `[A, B, C]`, scheduler is at index `0`, and `A` unregisters itself through `Conceal`, removal shifts `[B, C]` into indices `[0, 1]`. Scheduler then increments to `1`, so `C` runs next and `B` waits until a later pass. If the removed object is before the current index, a different shifted object can be skipped or the pass can terminate earlier depending on count.

## 3. Concrete Common Cases

### 3.1 `BulletClass::AI` self-removes on detonation

`BulletClass::AI @ 0x004666E0` calls `ObjectClass::AI` first, then handles movement/homing/proximity. On a detonation path it calls `BulletClass::BulletDetonation @ 0x00468D80`, then dispatches `this->vtable+0xF8` and returns. A separate delayed-nuke path stores an anim listener, but after the delayed anim completes the same AI removes the listener, calls detonation, then dispatches `+0xF8`.

| Detail | Evidence | Active in YR |
|---|---|---|
| Detonation exits through `vtable+0xF8` inside bullet AI. | `0x004666E0` decompile: `BulletClassBulletDetonationImpactDamage(...)` followed by `(**(code **)(*param_1 + 0xf8))();`. | Yes |
| Bullet detonation can spawn damage/effects before self-removal. | `0x00468D80` decompile calls `WarheadTypeClass__Detonate`; AAHeatSeeker2 reports verify stock GGI missile path. | Yes |
| Stock projectile path is common. | `[GGI] Secondary=MissileLauncher`, `[MissileLauncher] Projectile=AAHeatSeeker2`, and prior AAHeatSeeker2 reports. | Yes |

**Order consequence.** A projectile that detonates during the live object pass can unregister itself before the scheduler increments. The immediate successor in the logic vector can be skipped if it shifts into the projectile's old index.

### 3.2 `AnimClass::AI` self-removes when lifecycle ends or hidden/deletion state is set

`AnimClass::AI @ 0x00423AC0` has multiple ordinary exits that call `vtable+0xF8`. `AnimClass::Destroy @ 0x004255B0` then releases owner/sound side effects and calls `ObjectClass::UnInit`, which reaches conceal/unregister.

| Detail | Evidence | Active in YR |
|---|---|---|
| Bouncer/landing branch, `HideIfNoOre`/inactive branch, no-`Next` end branch, and make-infantry completion branch all dispatch `+0xF8`. | Decompile `0x00423AC0`; calls at the bouncer landing tail, `LAB_00424B38`, and make-infantry completion. | Yes |
| Destroy calls `ObjectClass::UnInit`. | `AnimClass::Destroy @ 0x004255B0` decompile. | Yes |
| Ordinary revealed anims run through the live LogicClass vector, not a separate class-array tick. | `ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`; `0x0055B608..0x0055B619`. | Yes |
| Stock garrison muzzle flashes and explosion/debris anims use `AnimClass`. | `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`, `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, stock `OccupantAnim=UCFLASH` traces. | Yes |

**Order consequence.** Expiring muzzle flashes, explosion anims, and metallic-debris anims can compact the same live object vector from inside their AI. They are not safe to process by a separate post-pass retain list if the goal is object-order parity.

### 3.3 `VoxelAnimClass::AI` self-removes on duration expiry / landing

`VoxelAnimClass::AI @ 0x00749F30` is the voxel/debris AI body at `vtable+0x5C`. It removes itself at entry when its removal byte is set, and removes itself after duration/landing effects. It can also spawn child voxel anims/anim effects before removal.

| Detail | Evidence | Active in YR |
|---|---|---|
| Entry removal flag dispatches `+0xF8` and returns. | Decompile `0x00749F30`, early `(char)param_1[0x44] != 0` branch. | Yes |
| Duration/landing path always ends at `+0xF8` after optional splash, damage, debris, tiberium, radar/dirty work. | Decompile `0x00749F30`, tails at `0x0074A6FC` and after 8-neighbor tiberium loop. | Yes |
| Warhead debris and barrel/death debris spawn `VoxelAnimClass` in stock/modded YR paths. | `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`, unit dossiers with `DebrisTypes=`. | Yes, conditional on debris-producing warhead/unit data |

**Order consequence.** Voxel debris cleanup can skip the immediate successor exactly like bullet/anim cleanup. Since debris often spawns during deaths and impacts, this can interact with same-frame projectile/anim ordering.

### 3.4 `InfantryClass::AI` reaches self-removal through sequence cleanup

`InfantryClass::AI @ 0x0051BAB0` calls `InfantryClass::DoType_Sequencer @ 0x00520AE0` late in the AI body after fire/fear logic. The sequencer dispatches `vtable+0xF8` when death sequences finish and for several special completed sequences.

| Detail | Evidence | Active in YR |
|---|---|---|
| Death sequence cases `0x0B..0x0F` spawn optional death anim, then call `+0xF8`. | Decompile `0x00520AE0`, cases `0x0B..0x0F`. | Yes |
| Special cases `0x14`, `0x15`, `0x24` call `+0xF8` when animation completes. | Decompile `0x00520AE0`. | Yes |
| `InfantryClass::AI` calls `DoType_Sequencer` on the active live AI path and returns if the object is no longer alive. | Decompile `0x0051BAB0`; `GI_GHIDRA_REPORT.md`. | Yes |

**Order consequence.** Infantry corpse cleanup is a common visible case where a dead infantry object can unregister during its own AI turn. A later unit immediately after it in the logic vector can be skipped that pass.

### 3.5 Engineers: capture and bridge repair consume the engineer inside infantry AI paths

Two common engineer paths consume the engineer via `vtable+0xF8` while the infantry is executing the live object pass.

| Detail | Evidence | Active in YR |
|---|---|---|
| Building capture: `Mission_Capture` changes the target building owner and then calls engineer `vtable+0xF8`, returning `1` to `InfantryClass::AI`. | Decompile `0x005202F0`; call after `ChangeOwner` and before `return 1`. | Yes for stock engineers and capturable buildings |
| Bridge repair: `PerCellProcess` CABHUT branch always calls engineer `vtable+0xF8` after sound/observer/bridge dispatch, regardless of repaired-cell count. | `BRIDGE_REPAIR_MULTI_ENGINEER_SAME_TICK_GHIDRA_REPORT.md`, assembly `0x0051A010..0x0051A02E`. | Yes for stock CABHUT engineer-enter path |
| Same-tick bridge repair explicitly depends on live vector compaction. | Same bridge report section 3.3 plus scheduler/remover evidence. | Yes |

**Order consequence.** Consecutive engineers can differ by vector order. If engineer A is current and self-removes, engineer B immediately after A shifts into A's old slot and can be skipped until the next pass. If B already ran earlier, both effects can happen in one pass.

### 3.6 Garrison entry limbos the entering infantry, but ownership transfer is building-update order

`BuildingClass::AddGarrisonOccupant @ 0x00522910` is reached from infantry arrival and appends the infantry pointer to the building's occupant vector. Prior Ghidra-backed reports verify it calls infantry limbo and does not change owner; ownership reconciliation happens later in `BuildingClass::Update`.

| Detail | Evidence | Active in YR |
|---|---|---|
| Garrison entry calls infantry limbo/appends occupant; no `ChangeOwner` in `AddGarrisonOccupant`. | `CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`, fresh report cites `0x00522910`. | Yes |
| Building ownership transfer happens when target building later runs `vtable+0x5C` and calls `CheckAutoSellOrCivilian`. | `GARRISON_OWNER_LIVE_OBJECT_ORDER_POSTFIX_TRACE.md`; `0x0043FB20`, `0x00458200`. | Yes |
| Whether transfer is same global frame or next depends on live vector relative order. | Garrison owner order traces and scheduler evidence. | Yes |

**Order consequence.** Garrison entry is a common player-visible example of "mutate/remove current infantry now, reconcile target building later if its vector slot is still ahead." This report does not re-decompile the full `CanDock` path because that is covered by prior garrison reports.

## 4. Earlier / Not-Yet-Visited Target Removal

This slot found strong self-removal proof and partial target-removal proof.

| Candidate | Status | Evidence | Active in YR | What remains |
|---|---|---|---|---|
| Bullet detonation destroys target or nearby objects | touched-not-exhausted | `BulletClass::AI -> BulletDetonation -> WarheadTypeClass__Detonate`; AAHeatSeeker2 detonation reports. | Yes | Drain `ReceiveDamage`/`UnInit` for the most common building/infantry/unit target deaths and record exact target-vector compaction timing. |
| Engineer capture changes target building owner, then runs building lifecycle, then consumes engineer | touched-not-exhausted | `Mission_Capture @ 0x005202F0` capture branch (corrected 2026-05-29 via `decompile_function 0x005202F0`): vcall order is building `+0x274(3)` (owner-change) FIRST, then building `+0xDC(0)`, then building `+0x3d4(...)`, then engineer `+0xF8()`, then `return 1` — ROOT_CAUSE: prior row inverted the owner-change/`+0xDC` order and omitted `+0x274`/`+0x3d4`. | Yes | Confirm whether the target building unregister/re-register changes its position in the active vector for every captured-building subtype. |
| Destroyed buildings from their own `BuildingClass::Update` | deferred | No common self-removing `BuildingClass::Update @ 0x0043FB20` destruction exit was drained here. | Conditional | Separate building death/destruction-effects investigation. Most structure death is driven by damage from other AI/weapon paths, not necessarily the building's own AI turn. |

## 5. Current Rust Implementation Status

| Rust surface | Current behavior | Evidence | Delta |
|---|---|---|---|
| `Simulation::advance_tick` | Fixed phases, many subsystem-specific snapshots, binary frame advanced at tick start. | `src/sim/world/mod.rs:1508` (corrected 2026-05-29 via `grep "fn advance_tick"`: was 1402, but 1402 is the `run_late_region` return tuple; `pub fn advance_tick` signature is at 1508 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT from file growth + region-extraction refactor). | No central live LogicClass object-vector pass. |
| Homing projectiles | `tick_homing_movement` snapshots `keys_sorted()` and returns detonated IDs for caller-side damage/despawn. | `src/sim/movement/homing_movement.rs:380`, `src/sim/movement/homing_movement.rs:386` (corrected 2026-05-29: was 379, 534; current lines are 380 and 536 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT from minor file edits), `src/sim/movement/homing_movement.rs:536`, `src/sim/movement/homing_movement.rs:569`. | Does not self-unregister inside a live object pass. |
| Garrison muzzle flashes | App-layer list spawns from pending events, extends list, then `retain_mut`s after elapsed fixed ticks. | `src/app_building_anim.rs:702`, `src/app_building_anim.rs:756`, `src/app_building_anim.rs:764`. | Models lifecycle fields better than older code, but not native object-vector removal/skip semantics. |
| Engineer capture | Captures are collected into a `Vec` before processing; engineer is despawned later in loop. | `src/sim/world/world_orders.rs:179`, `src/sim/world/world_orders.rs:244`. | Snapshot can process successors that native compaction may skip. |
| Bridge repair | Current code intentionally simulates current-removal skip by incrementing `key_idx += 2` after despawning engineer. | `src/sim/world/world_orders.rs:265` (`tick_bridge_repair_orders` signature), `src/sim/world/world_orders.rs:414` (`key_idx += 2`) (corrected 2026-05-29 via `grep "key_idx += 2"`: was 269/394/398; the actual skip increment is at 414 — 394/398 are unrelated zone-graph code — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT from file edits). | This local fix matches the bridge-repair skip shape for stable-ID order, but still uses stable IDs rather than native active-vector order. |
| Death/final removal | Many Rust paths mark `dying` or remove from `EntityStore`; app tick later despawns finished death anims. | `src/sim/animation.rs:395`, `src/app_sim_tick.rs:306`, `src/sim/world/mod.rs:675`. | Native object may unregister during its own AI; Rust often defers physical removal. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Main scheduler live loop | verified | `0x0055AFB0`, `0x0055B608..0x0055B619` | none |
| Remover compaction | verified | `0x0055BAE0`, assembly `0x0055BB09..0x0055BB27` | none |
| `ObjectClass::Conceal` unregister gate | verified | `0x005F4D30` | exact game-mode sentinel names remain inherited from helper report |
| `ObjectClass::UnInit` pending delete and limbo call | verified | `0x005F65F0` | pending-delete drain timing out of scope |
| `BulletClass::AI` self-removal | verified | `0x004666E0`, `0x00468D80` | target damage/removal chain not exhausted |
| `AnimClass::AI` self-removal | verified | `0x00423AC0`, `0x004255B0` | exact object-pool pending-delete destruction drain out of scope |
| `VoxelAnimClass::AI` self-removal | verified | `0x00749F30` | child-spawn ordering relative to current vector append is slot-1/3 territory |
| `InfantryClass::DoType_Sequencer` corpse/special removal | verified | `0x0051BAB0`, `0x00520AE0` | exact sequence inventory already in GI report |
| Engineer capture self-removal | verified | `0x005202F0` | target building vector-position effects after capture still partial |
| Bridge repair engineer self-removal and skip | verified | bridge repair report, `0x0051A010..0x0051A02E` | concrete retail object index runtime examples still need debugger |
| Garrison entry limbo/order | touched-not-exhausted | `0x00522910` reports and garrison traces | full CanDock/PerCellProcess re-decompile not repeated |
| Destroyed building self-removal from building AI | deferred | none in this slot | separate `BuildingClass::Update`/destruction effects investigation |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-CMU-001 - Is the main object pass live-count, not snapshot? -> Yes; count reloads after each vtable call.` (evidence: `0x0055B608..0x0055B619`)
- `[RESOLVED] OQ-CMU-002 - Does remover compact instead of swap-remove? -> Yes; later entries shift left and count decrements.` (evidence: `0x0055BB09..0x0055BB27`)
- `[RESOLVED] OQ-CMU-003 - Does Conceal unregister logic-enabled objects? -> Yes under type/game-mode gates before setting InLimbo.` (evidence: `0x005F4D30`)
- `[RESOLVED] OQ-CMU-004 - Does UnInit reach Conceal and pending delete? -> Yes; it calls virtual +0xD4, clears alive, queues pending delete.` (evidence: `0x005F65F0`)
- `[RESOLVED] OQ-CMU-005 - Can BulletClass::AI self-remove? -> Yes after detonation/delayed-detonation branch.` (evidence: `0x004666E0`)
- `[RESOLVED] OQ-CMU-006 - Is bullet path active in stock YR? -> Yes for ordinary projectiles, including GGI AAHeatSeeker2.` (evidence: AAHeatSeeker2 reports, stock rules)
- `[RESOLVED] OQ-CMU-007 - Can AnimClass::AI self-remove? -> Yes through multiple lifecycle exits to Destroy/UnInit.` (evidence: `0x00423AC0`, `0x004255B0`)
- `[RESOLVED] OQ-CMU-008 - Can VoxelAnimClass::AI self-remove? -> Yes on removal flag and duration/landing tails.` (evidence: `0x00749F30`)
- `[RESOLVED] OQ-CMU-009 - Can InfantryClass AI self-remove after death sequence? -> Yes through `DoType_Sequencer` death and special sequence exits.` (evidence: `0x0051BAB0`, `0x00520AE0`)
- `[RESOLVED] OQ-CMU-010 - Can engineer capture consume the engineer inside infantry AI? -> Yes; `Mission_Capture` calls engineer `+0xF8` and returns true.` (evidence: `0x005202F0`)
- `[RESOLVED] OQ-CMU-011 - Can CABHUT repair consume the engineer and cause immediate-successor skip? -> Yes; prior bridge report verifies `+0xF8` and scheduler composition.` (evidence: `0x0051A010..0x0051A02E`; scheduler/remover)
- `[RESOLVED] OQ-CMU-012 - Is garrison entry same-frame ownership based on object order? -> Yes; AddGarrisonOccupant mutates occupant vector, building update reconciles later when its vector turn occurs.` (evidence: garrison owner traces, `0x00522910`, `0x0043FB20`, `0x00458200`)
- `[RESOLVED] OQ-CMU-013 - Does current Rust globally model this as a live object vector? -> No; `advance_tick` is phased and several systems snapshot `keys_sorted`.` (evidence: `src/sim/world/mod.rs:1187`, `src/sim/movement/homing_movement.rs:386`)
- `[DEFERRED] OQ-CMU-014 - Which target-object deaths during bullet detonation unregister earlier/not-yet-visited objects?` (category: `requires-different-system-context`; reason: requires draining `WarheadTypeClass::Detonate`, `ReceiveDamage`, and target class UnInit paths; next-step-if-pursued: target ground unit/infantry/building death from a projectile hit.)
- `[DEFERRED] OQ-CMU-015 - Does BuildingClass::Update itself commonly self-remove destroyed buildings?` (category: `bounded-cost-too-high`; reason: building destruction system is broad and often damage-driven by other objects; next-step-if-pursued: focused `BuildingClass::Update` and destruction-effects pass.)
- `[DEFERRED] OQ-CMU-016 - What are concrete object-vector indices for stock replay scenarios?` (category: `needs-runtime-debugger`; reason: static Ghidra proves mechanics, not runtime vector positions; next-step-if-pursued: instrument retail map with two adjacent engineers/projectiles/anims.)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Common objects can unregister during their own `vtable+0x5C` AI; current/self removal compacts the live vector and can skip the immediate successor. | `0x0055B608..0x0055B619`, `0x0055BAE0`, `0x005F4D30`, `0x005F65F0`, `0x004666E0`, `0x00423AC0`, `0x00749F30`, `0x00520AE0` | Missing globally: Rust phases/snapshots IDs in many systems. | Future sim-level active object scheduler; `src/sim/world/mod.rs::advance_tick`; projectile/anim/infantry cleanup surfaces. | A central logic pass must reproduce append/remove/skip semantics for logic-enabled objects, not just final state. | Three active objects A,B,C in vector order; A's AI self-UnInit's; B is not called until next pass, C is called next this pass. | Do not process a pass-entry snapshot and despawn after the pass for parity-sensitive objects. Proposed test: `logic_vector_self_uninit_skips_shifted_successor`. |
| Bullet AI detonates and immediately calls `+0xF8`; damage/effects happen before bullet unregisters. | `0x004666E0`, `0x00468D80` | Partial/missing: homing movement snapshots keys and returns detonated IDs to caller. | `src/sim/movement/homing_movement.rs`, future projectile damage/despawn dispatch. | Projectile detonation should be an in-AI event in live object order, with self-unregister before post-call scheduler increment. | A missile at index i detonates; object originally at i+1 is shifted and skipped if missile unregisters current. | Do not model projectile detonation as a separate end-of-phase batch when order affects later objects. Proposed test: `projectile_detonation_uninit_uses_live_logic_skip`. |
| Anim and voxel-anim lifecycle cleanup calls `+0xF8` from AI, then `UnInit`/Conceal unregisters. | `0x00423AC0`, `0x004255B0`, `0x00749F30` | Partial/missing: garrison flashes are app-layer retain lists, world effects are not logic-vector objects. | `src/app_building_anim.rs`, future generic `AnimRuntime`/world-effect scheduler. | Expiring anims/debris must remove at their vector turn and can affect same-pass successor order. | Spawn two UCFLASH-like anims followed by an infantry; first flash expires at its AI turn, second immediate successor waits, infantry runs next if shifted order dictates. | Do not use render/app retain order as proof of native logic order. Proposed test: `anim_expiry_compacts_logic_vector_before_next_ai`. |
| Engineer capture/repair consumes engineer via `+0xF8` inside infantry paths; bridge repair already has explicit skip shape but uses stable-ID order. | `0x005202F0`; bridge report `0x0051A010..0x0051A02E`; Rust `world_orders.rs:177` (`tick_capture_orders`), `world_orders.rs:265` (`tick_bridge_repair_orders`), `world_orders.rs:414` (`key_idx += 2`) (corrected 2026-05-29 via grep: was 269/394 — see §5 row) | Partial: bridge repair simulates skip locally; capture uses collected Vec and stable IDs. | `src/sim/world/world_orders.rs::tick_capture_orders`, `tick_bridge_repair_orders`, future active-object order source. | Engineer-consumption order should derive from active logic-vector order; capture should not process an immediate shifted successor if native would skip it. | Two engineers consecutive in native vector reach capturable/repair target; first consumes itself; second waits until next pass. | Do not generalize the bridge local `key_idx += 2` over sorted stable IDs as full parity. Proposed tests: `engineer_capture_self_uninit_skips_shifted_successor`, `bridge_repair_consecutive_engineers_second_skipped_by_live_logic_order`. |

### Negative Facts / Do Not Do

- Do not snapshot the active object list at pass entry for object AI. Active in YR: Yes; evidence `0x0055B613..0x0055B619`.
- Do not swap-remove active logic objects. Active in YR: Yes; evidence `0x0055BB11..0x0055BB21`.
- Do not delay all `+0xF8` removals until a post-pass cleanup and call the shifted successor in the same pass. Active in YR: Yes; evidence scheduler/remover composition.
- Do not treat app-layer render effects as equivalent to native `AnimClass`/`VoxelAnimClass` object AI when removal order can affect gameplay/render timing. Active in YR: Yes; evidence `0x00423AC0`, `0x00749F30`.
- Do not claim destroyed-building AI self-removal is covered by this report. Active in YR: Conditional/unchecked; building death requires its own damage/destruction pass.

### Proposed Rust Tests

- `logic_vector_self_uninit_skips_shifted_successor`
- `projectile_detonation_uninit_uses_live_logic_skip`
- `anim_expiry_compacts_logic_vector_before_next_ai`
- `voxel_debris_expiry_compacts_logic_vector_before_next_ai`
- `engineer_capture_self_uninit_skips_shifted_successor`
- `bridge_repair_consecutive_engineers_second_skipped_by_live_logic_order`
- `garrison_entry_building_reconciliation_uses_live_object_order`

### Remaining Uncertainty

- Full target-object removal during bullet/warhead damage is not drained; this matters for earlier/not-yet-visited target objects.
- Building destruction from `BuildingClass::Update` vs damage-driven paths remains a separate investigation.
- Static Ghidra proves mechanics, but concrete vector index distributions need runtime debugging on stock scenarios.
- Exact pending-delete destructor drain timing after `UnInit` is out of scope; the report only proves active-list unregistration and skip semantics.

### Stale Docs / Follow-up Docs

- Replace any statement that "current Rust collecting sorted IDs is equivalent to gamemd live object AI order" with: "gamemd's main object AI pass is a live LogicClass vector. Common AI bodies can call `+0xF8`/`UnInit` during their own turn; compacting removal can skip the shifted immediate successor. Sorted-ID snapshots are deterministic but not parity-equivalent unless a specific subsystem proves the same order and removal timing."
- Replace any statement that "anim/muzzle flash expiry is only render-side cleanup" with: "native `AnimClass` and `VoxelAnimClass` are logic objects when revealed; their AI lifecycle calls `+0xF8` and unregisters through `ObjectClass::UnInit`/`Conceal`, affecting same-pass object-vector order."
- Replace any bridge/engineer wording that says "both engineers in the same command window process in one tick" with: "two engineers process according to live object-vector order; if the first current engineer removes itself and the second was the immediate successor, the second is shifted into the old index and skipped until a later pass."

## Sources

- Ghidra read-only decompiled:
  - `LogicClass::PerTickUpdate @ 0x0055AFB0`
  - `FUN_0055BAE0 @ 0x0055BAE0`
  - `ObjectClass::Conceal @ 0x005F4D30`
  - `ObjectClass::UnInit @ 0x005F65F0`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::BulletDetonation @ 0x00468D80`
  - `AnimClass::AI @ 0x00423AC0`
  - `AnimClass::Destroy @ 0x004255B0`
  - `VoxelAnimClass::AI @ 0x00749F30`
  - `InfantryClass::AI @ 0x0051BAB0`
  - `InfantryClass::DoType_Sequencer @ 0x00520AE0`
  - `InfantryClass::Mission_Capture @ 0x005202F0`
- Ghidra read-only assembly context:
  - `FUN_0055BAE0` around `0x0055BAE0` and `0x0055BB11`
- Prior reports referenced:
  - `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
  - `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
  - `docs/research/AAHEATSEEKER2_FIRST_TICK_DAMAGE_LATENCY_GHIDRA_REPORT.md`
  - `docs/research/ANIMCLASS_GLOBAL_OBJECT_REGISTRATION_LIFETIME_RESWARM_20260527.md`
  - `docs/research/ANIMCLASS_AI_LIFECYCLE_EXACT_SUBSET_RESWARM_20260527.md`
  - `docs/research/VOXELANIMCLASS_GHIDRA_REPORT.md`
  - `docs/research/GI_GHIDRA_REPORT.md`
  - `docs/research/bridges/05-damage-collapse-repair-cabhut/BRIDGE_REPAIR_MULTI_ENGINEER_SAME_TICK_GHIDRA_REPORT.md`
  - `docs/research/CIVILIAN_GARRISON_OWNERSHIP_TRANSFER_TIMING_GHIDRA_REPORT.md`
  - `docs/research/traces/GARRISON_OWNER_LIVE_OBJECT_ORDER_POSTFIX_TRACE.md`
- Rust source scanned:
  - `src/sim/world/mod.rs`
  - `src/sim/world/world_orders.rs`
  - `src/sim/movement/homing_movement.rs`
  - `src/app_building_anim.rs`
  - `src/sim/animation.rs`

Status: COMPLETE for the requested bounded coverage-map; not exhaustive for every class or target-damage path.
