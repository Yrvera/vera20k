# Direct Removal / Same-Pass Mutation Blockers - Ghidra Research Report

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`, main object loop `0x0055B608..0x0055B619`, active remover `FUN_0055BAE0 @ 0x0055BAE0`, `ObjectClass::Conceal @ 0x005F4D30`, `ObjectClass::Destructor @ 0x005F3B80`
**Investigation Mode:** coverage-map
**Claimed Scope:** direct Rust `EntityStore::remove` paths that still bypass `Simulation::unregister_live_object` / `despawn_entity`, plus the same-pass spawn/remove cases current staged Rust cannot model under the native live-vector contract.
**Non-Scope:** re-proving the settled live-vector contract except read-only spot checks, implementing Rust, broad Bullet/Anim/Techno AI migration, every class-specific `vtable+0x5C` body, and INI/asset behavior not needed for the removal handoff.
**Confidence:** High for the native scheduler/remover facts and Rust direct-removal inventory; Medium for class-specific same-pass examples inherited from prior docs rather than re-drained here.
**Active in YR:** Yes for the scheduler/remover/lifecycle contract. Conditional for specific class examples as noted below.

## 0. Working Notes

**Target question:** Which Rust entity removal and same-pass mutation paths still block migration to the native `LogicClass` live-vector contract?

**Non-goals:** Re-proving settled `PerTickUpdate` cursor semantics, implementing Rust changes, broad Bullet/Anim/Techno AI migration, or auditing unrelated lifecycle behavior.

**Evidence needed to mark COMPLETE:** Current Rust direct-removal inventory with file:line evidence; read-only binary/doc evidence for native compacting removal, same-pass append visitation, and AI-pass removal behavior; implementation handoff with concrete acceptance scenarios.

**Stop conditions:** Stop after the scoped Rust inventory plus native mutation-contract evidence are sufficient for handoff, or if Ghidra/read-only evidence is unavailable and the report explicitly marks the affected claims as PARTIAL.

## 1. Overview

The native object AI spine is a live forward walk over `LogicClass+0x04/+0x10`, not a pass-entry snapshot. Active in YR: Yes, because `Main_Tick` calls `LogicClass::PerTickUpdate` with `ECX=0x87F778` at `0x0055DC99..0x0055DC9E`.

Rust now has a `LogicVector`, membership bit, `register_live_object`, `unregister_live_object`, `despawn_entity`, and `for_each_live_object`; the local primitive is close to the native contract. The blockers are remaining direct `EntityStore::remove` callsites and broad `advance_tick` staged passes that do not route class AI through one live mutable vector.

## 2. Native Contract Spot Checks

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| Main object loop loads `items[i]`, calls `vtable+0x5C`, reloads count, increments index, then compares. | Assembly context `0x0055B608..0x0055B619`; decompile `0x0055AFB0` | High | Yes |
| Tail appends can be visited in the same pass because count is reloaded after each object AI call. | `0x0055B613..0x0055B619`; prior registration helper report for `FUN_0055BAA0` tail append | High | Yes |
| Active removal is membership-byte gated on `Object+0x98`. | `FUN_0055BAE0` assembly `0x0055BAE7..0x0055BAEF` | High | Yes |
| Valid active removal decrements count and shifts later entries left; it does not swap-remove. | `0x0055BB09..0x0055BB21` | High | Yes |
| The membership byte is cleared after a flagged removal attempt, including not-found/invalid-index cases. | `0x0055BB23..0x0055BB27` | High | Yes |
| `ObjectClass::Conceal` calls the remover before setting `InLimbo` (`+0x81`). | xref assembly `0x005F4DCD..0x005F4DD3`; decompile `0x005F4D30` | High | Yes, conditional on game active and logic-enabled type gate |
| `ObjectClass` destructor has an active-vector fallback if `+0x98` is still set. | `0x005F3D65..0x005F3D75`; decompile `0x005F3B80` | High | Yes |

## 3. Current Rust Direct-Removal Inventory

| Rust surface | Current behavior | Delta | Active in YR relevance |
|---|---|---|---|
| `src/sim/world/mod.rs:817..839` / `despawn_entity` | `uninit` clears occupancy/radio, calls `conceal(stable_id)`, then `entities.remove(stable_id)`. | This is the safe path; it routes through live-vector unregister before store free. | Yes: mirrors native conceal-before-free ordering at the Rust lifecycle boundary. |
| `src/sim/combat/mod.rs:1003..1010` | Non-animated deaths clear occupancy/radio then call `entities.remove(dead_id)` directly. | Bypasses `unregister_live_object`; if the entity was live, `LogicVector` retains a dangling ID. Also models death as batched combat-phase cleanup, not synchronous `ReceiveDamage`/AI removal. | Yes for ordinary structure/vehicle lethal hits; verified by target-death reports and `UnitClass::ReceiveDamage @ 0x00737C90`. |
| `src/sim/movement/movement_tick.rs:1584..1590` | Crush kills set victim HP to 0, clear radio, then call `entities.remove(victim_id)`. | Bypasses `unregister_live_object`; also cannot model victim removal as an in-object live-vector mutation by the crusher/movement AI. | Conditional: crush deaths are live YR gameplay; exact native crusher call chain not re-drained in this slot. |
| `src/sim/world/mod.rs:1142..1158` | `remove_wall_entity_at` removes the wall-backed `GameEntity` directly. | Bypasses `unregister_live_object`; if wall entities are ever live-registered, this leaves stale live membership. | Conditional: wall overlay destruction is active YR; whether Rust wall backing entities should be live members is an implementation question for wall/object modeling. |
| `src/sim/production/production_sell.rs:687..714` | Sell ejects survivors/occupants, undocks miners, clears radio, then calls `sim.entities.remove(stable_id)`. | Bypasses `despawn_entity`; sold structures can remain in `LogicVector` if registered. | Yes for player building sell; native `SellBuilding`/`BuildingClass::Sell` are active paths, but this slot did not re-drain sell-to-Conceal call order. |
| `src/sim/passenger.rs:1088`, `src/sim/passenger.rs:2085` | Test-only helper removes a destroyed garrison building directly. | Test-only bypass; not a production runtime blocker, but tests can mask missing unregister behavior. | No for runtime; yes as test-surface risk. |
| `src/sim/entity_store.rs:58..60` | `EntityStore::remove` only frees BTreeMap storage. | Correct as a low-level primitive, but it is not lifecycle-safe for live objects. | N/A as library primitive. |

## 4. Same-Pass Mutation Cases Rust Still Cannot Model Globally

| Case | Verified native behavior | Current Rust blocker | Active in YR |
|---|---|---|---|
| Self-remove during object AI | Current object can call `+0xF8`/`UnInit`, `Conceal` compacts the vector, and the immediate shifted successor can be skipped. | `Simulation::for_each_live_object` models this locally, but `advance_tick` does not run Bullet/Anim/Techno/Infantry AI through it. | Yes; scheduler/remover verified, concrete common cases in `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`. |
| Same-pass tail append | Revealed objects appended before the cursor reaches the new tail can run later in the same `PerTickUpdate`. | Many Rust spawns are `EntityStore::insert` plus staged follow-up; they do not become live object AI participants in the same global pass unless manually special-cased. | Yes; stock bullets/anims/engineer/garrison/miner reports cite this contract. |
| Projectile detonation and target removal | `BulletClass::AI @ 0x004666E0` can detonate, damage targets, then call `+0xF8`; killed UnitClass vehicles remove synchronously inside `ReceiveDamage`. | Rust combat applies damage/effects in a combat phase, then `handle_entity_deaths` removes in a batch; no mid-pass target compaction or bullet self-unregister skip. | Yes for standard projectile combat and ground vehicle deaths; Crashable paths are conditional. |
| Anim lifecycle expiry | `AnimClass::AI @ 0x00423AC0` can call `+0xF8` on ordinary lifecycle exits; first-AI/tail-append cases are order-sensitive. | Rust world/app effects are retained in separate lists, not live `LogicVector` objects. | Yes for revealed `AnimClass`; specific anim type gates are conditional. |
| Engineer/capture/bridge consumption | Engineer paths consume the engineer inside infantry AI via `+0xF8`; consecutive engineers depend on compacting skip. | Bridge repair has a local sorted-ID skip surrogate; capture and other infantry paths still use staged/snapshot order, not active-vector order. | Yes for stock engineers, capturable buildings, and CABHUT bridge repair. |
| Sell/destruction survivor spawns | Native survivor/unlimbo and anim/debris append timing can place newly created objects at vector tail during the same pass. | Rust survivor/effect creation is split across command/combat/production phases; direct removals bypass live unregister. | Conditional: active for Crewed/building death/sell cases, but exact sell call order deferred. |

## 5. Current Rust Implementation Status

`src/sim/world/logic_vector.rs:1..31` has an insertion-ordered list and order-preserving remove, but `remove` uses `retain`, which removes all matching IDs. Under the intended membership invariant this is fine; under a corrupted/duplicate list it is not byte-equivalent to native's single found-index remove. Active in YR: Yes for the native single-entry compaction rule; Rust duplicate state should remain impossible or be tested as an invariant.

`src/sim/world/mod.rs:679..697` has membership-gated register/unregister. One mismatch remains in the absent-entity branch: if the entity is already gone from `EntityStore`, Rust still scrubs the order (`:696..697`), while native `FUN_0055BAE0` dereferences the object pointer and relies on conceal before free. Active in YR: Yes for native caller contract; Rust's forgiving branch is acceptable only as defensive cleanup, not a reason to remove storage first.

`src/sim/world/mod.rs:763..769` implements the native live-pass cursor shape. But `advance_tick` begins at `src/sim/world/mod.rs:1508` and runs movement, air movement, gates, combat, production, ore, spawners, and effects as separate phases. Active in YR: Rust-facing blocker; native per-object surfaces should migrate into a central live pass where proven.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Native main object loop | verified | `0x0055B608..0x0055B619`, `0x0055DC99..0x0055DC9E` | none |
| Native active remover | verified | `0x0055BAE7..0x0055BB27` | none |
| Conceal/destructor remover reachability | verified | `0x005F4DD3`, `0x005F3D75` | none |
| Rust `despawn_entity` safe path | verified-source-scan | `src/sim/world/mod.rs:817..845` | none |
| Rust direct store removals | verified-source-scan | `rg "entities.remove"` and file:line table above | future code fix |
| Combat target-death same-pass removal | verified-via-docs-and-spot-check | target-death reports, `0x00737C90` entry spot-check | full target class matrix deferred |
| Movement crush exact native call chain | touched-not-exhausted | Rust source plus known live gameplay | Ghidra trace of native crusher removal path |
| Wall entity live membership | touched-not-exhausted | Rust source plus wall overlay docs | decide whether wall backing `GameEntity` is logic-enabled |
| Production sell exact native active-vector call order | touched-not-exhausted | Rust source plus SellBuilding docs | focused sell/Conceal trace if needed |
| Global migration readiness | touched-not-exhausted | `advance_tick` source, scheduler docs | sibling slot phase partition and class slices |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the native object pass live rather than snapshotted? -> Yes; count is reloaded after each `vtable+0x5C`.` (evidence: `0x0055B608..0x0055B619`; Active in YR: Yes)
- `[RESOLVED] OQ-02 - Does native removal compact and skip shifted successors? -> Yes; remover shifts left and scheduler increments without repair.` (evidence: `0x0055BB09..0x0055BB21`, `0x0055B616`; Active in YR: Yes)
- `[RESOLVED] OQ-03 - Does current Rust have a safe lifecycle path? -> Yes, `despawn_entity` calls `uninit`, which conceals before store removal.` (evidence: `src/sim/world/mod.rs:817..845`; Active in YR: Rust-facing)
- `[RESOLVED] OQ-04 - Which production Rust removals bypass unregister? -> combat deaths, crush kills, wall entity removal, building sell, and a test-only passenger helper.` (evidence: `src/sim/combat/mod.rs:1009`, `src/sim/movement/movement_tick.rs:1589`, `src/sim/world/mod.rs:1158`, `src/sim/production/production_sell.rs:714`, `src/sim/passenger.rs:2085`; Active in YR: mixed, table above)
- `[RESOLVED] OQ-05 - Can Rust's local `for_each_live_object` model append/remove same pass? -> Yes locally; snapshot tests demonstrate same-pass append and self-unregister skip.` (evidence: `src/sim/snapshot.rs:413..423`, `src/sim/snapshot.rs:445..466`; Active in YR: Rust-facing)
- `[RESOLVED] OQ-06 - Does `advance_tick` actually use that live pass for current AI work? -> No; it remains staged phases beginning at `src/sim/world/mod.rs:1508`.` (evidence: Rust source scan; Active in YR: Rust-facing)
- `[DEFERRED] OQ-07 - Exact native crusher removal chain?` (category: `requires-different-system-context`; reason: this slot audited Rust blocker and scheduler contract, not the crush implementation; next-step-if-pursued: trace vehicle/infantry crush damage to `ReceiveDamage`/`UnInit`)
- `[DEFERRED] OQ-08 - Exact native player-sell active-vector call order?` (category: `requires-different-system-context`; reason: sell docs prove active sell/garrison behavior but not a full conceal/unregister ladder for this slot; next-step-if-pursued: focused `BuildingClass::Sell` / sell command lifecycle trace)
- `[DEFERRED] OQ-09 - Whether Rust wall-backed entities should ever be live-registered?` (category: `requires-different-system-context`; reason: depends on wall entity modeling slice; next-step-if-pursued: compare wall overlay destruction with any native ObjectClass wall representation)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Storage removal is not active-vector unregister; native Conceal unregisters before free. | `0x005F4DD3`; `0x0055BAE0`; `src/sim/world/mod.rs:817..839` | Direct `EntityStore::remove` callsites bypass `unregister_live_object`. | `src/sim/combat/mod.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/world/mod.rs::remove_wall_entity_at`, `src/sim/production/production_sell.rs` | Route live object removals through `despawn_entity` or a same-order lifecycle API; never free store first for live objects. | Spawn/register a vehicle/building, kill/sell/crush it through each path, then assert no dangling ID remains in `live_object_order_snapshot`. | `direct_removal_paths_unregister_logic_vector_before_store_free` | Do not call `entities.remove` directly for objects that may be live members. |
| Combat target death removes vehicles/buildings synchronously inside the damaging AI path; compaction can affect same-pass successor visitation. | `UnitClass::ReceiveDamage @ 0x00737C90` reports; scheduler `0x0055B608..0x0055B619`; remover `0x0055BB09..0x0055BB21` | Rust batches `handle_entity_deaths` in combat and removes with `entities.remove`; no live-pass compaction. | Combat damage/death dispatch, future BulletClass/TechnoClass AI pass | Make lethal damage dispatch capable of unregistering targets during the current live object AI call once those classes migrate. | Logic order `[bullet, victim, successor]`; bullet AI kills victim; victim unregisters before bullet returns, and successor is skipped if native cursor math dictates. | `combat_lethal_target_remove_compacts_live_logic_and_skips_shifted_successor` | Do not treat a post-combat dead queue as parity-equivalent. |
| Tail-appended live objects can run in the same pass; snapshots miss them. | `0x0055B613..0x0055B619`; `src/sim/snapshot.rs:469..505` | Current `advance_tick` phases often insert/spawn into store/effects but do not let new objects participate in one global live pass. | Bullet, Anim, survivor/eject, production spawn, garrison/engineer surfaces | Use the central live scheduler for migrated object AI; same-pass appends must be visible before pass end. | Active object A spawns/registers object C while B is old tail; visit order becomes A, B, C in same pass. | `object_ai_tail_append_runs_same_pass_in_global_spine` | Do not enforce universal "new objects tick next frame" behavior. |
| Local bridge skip surrogate is not a global substitute for native order. | Common mid-pass report; Rust `world_orders.rs` local skip docs; scheduler/remover evidence | Bridge repair has local stable-ID skip, capture and many other paths remain staged/snapshot. | `src/sim/world/world_orders.rs`, future infantry/engineer AI slice | Migrate consumption/removal semantics to active-vector order, then remove local stable-ID approximations only after tests cover native ordering. | Consecutive engineers in live order target a repair/capture case; first self-unregisters, second waits until next pass. | `engineer_consumption_uses_live_logic_order_not_stable_id_snapshot` | Do not generalize sorted stable IDs as native object order. |

### Negative Facts / Do Not Do

- Do not use `EntityStore::remove` as a lifecycle API for live objects. Evidence: native active membership is `Object+0x98` and is cleared by `FUN_0055BAE0`, not by memory/free/store removal. Active in YR: Yes.
- Do not snapshot object AI at pass entry. Evidence: native main loop reloads count at `0x0055B613` and jumps at `0x0055B619`. Active in YR: Yes.
- Do not swap-remove or "repair" the cursor after active-vector compaction. Evidence: native shifts left at `0x0055BB11..0x0055BB21` and scheduler only increments at `0x0055B616`. Active in YR: Yes.
- Do not assume a dangling Rust `LogicVector` ID is harmless because `entities.get(id)` returns `None`. Evidence: native caller contract conceals before free; current Rust debug invariant expects order length to match `in_logic_vector` flags (`src/sim/world/mod.rs:719..737`). Active in YR: Rust-facing.
- Do not treat app/render effects as substitutes for `AnimClass`/debris active objects where same-pass AI or removal order matters. Evidence: `AnimClass::AI @ 0x00423AC0` and `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`. Active in YR: Yes for revealed anims.

### Remaining Uncertainty

- Exact native crusher removal call chain and whether any special crush cases bypass normal `ReceiveDamage`/`UnInit`.
- Exact native sell-to-Conceal/unregister ordering for every building sell path; current handoff only needs the Rust direct-removal blocker.
- Whether Rust wall-backed `GameEntity` values are intended to be live logic objects or storage-only companions for overlays.
- Concrete retail object-vector indices for combined bullet/target/anim/survivor cases require runtime logging; static evidence proves mechanism, not a map-specific index distribution.

### Stale Docs / Follow-up Docs

- `docs/research/TARGETDEATH_RECEIVEDAMAGE_DEATH_DISPATCH_REMOVAL_TIMING_RESWARM_20260528.md` lines 234..237 should replace "Rust `advance_tick` is phased; the logic vector is iterated with a sorted snapshot in `live_object_order_snapshot`" with: "Rust `advance_tick` is still phased and many systems snapshot or stage work; `live_object_order_snapshot` is now verbatim LogicVector order, while `for_each_live_object` models native live append/remove locally. The gap is integration: current object AI phases do not run through that live pass globally."
- `docs/research/ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md` implementation handoff rows that say current Rust has "no membership byte" are stale in older sections; use the report's 2026-05-29 correction wording: Rust now has `GameEntity::in_logic_vector`, `LogicVector`, and byte-gated register/unregister, but direct `EntityStore::remove` callsites still bypass it.

## Sources

- Direct read-only Ghidra spot checks: `0x0055AFB0`, `0x0055BAE0`, `0x005F4D30`, `0x005F3B80`, assembly contexts `0x0055B608..0x0055B619`, `0x0055BAE7..0x0055BB27`, xref contexts `0x005F4DD3`, `0x005F3D75`, `0x0055DC99..0x0055DC9E`.
- Prior docs: `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`, `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`, `SAME_TICK_SPAWNED_OBJECT_BEHAVIOR_GHIDRA_REPORT.md`, `COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md`, `TARGETDEATH_UNITCLASS_VEHICLE_DEATH_ACTIVE_VECTOR_TIMING_RESWARM_20260528.md`, `TARGETDEATH_BUILDINGCLASS_DESTRUCTION_REMOVAL_OWNER_RESWARM_20260528.md`, `TARGETDEATH_RECEIVEDAMAGE_DEATH_DISPATCH_REMOVAL_TIMING_RESWARM_20260528.md`.
- Rust source scanned read-only: `src/sim/world/mod.rs`, `src/sim/world/logic_vector.rs`, `src/sim/entity_store.rs`, `src/sim/combat/mod.rs`, `src/sim/movement/movement_tick.rs`, `src/sim/production/production_sell.rs`, `src/sim/passenger.rs`, `src/sim/snapshot.rs`.

Status: COMPLETE for the scoped blocker inventory and Rust handoff; deferred items are follow-up slices, not blockers for this report.
