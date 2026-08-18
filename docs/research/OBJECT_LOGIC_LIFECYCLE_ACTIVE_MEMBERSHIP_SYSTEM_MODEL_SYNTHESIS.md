# Object / Logic Lifecycle Active Membership System Model Synthesis

**Output type:** model-synthesis  
**Date:** 2026-05-28  
**Scope:** `ObjectClass` / `LogicClass` active membership, reveal, conceal, uninit, save/load active vector persistence, listener cleanup, and direct non-Reveal registration callers.  
**Non-scope:** full render-layer ordering, full FactoryClass production semantics, radio/miner state machines, anim drawing internals, and Rust implementation design. FactoryClass appears only as an adjacent scheduler/persistence ordering warning.  
**Safety verdict:** Core active-membership and lifecycle ordering is implementation-safe at API-contract level. Listener body census/mutation, exact class-specific AI self-removal cases, and a few special transient anim persistence corners still need reinvestigation before fine-grained implementation.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---:|---:|---:|---|
| `LogicClass::PerTickUpdate` ticks a live pointer vector at singleton `0x0087F778`, not an entity-store snapshot. It uses pointer array `+0x04`, count `+0x10`, forward index, vtable `+0x5C`, and reloads count after each object AI call. | `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`; Ghidra spot-check `0x0055DC99 -> 0x0055AFB0`; disassembly `0x0055B5FB..0x0055B619`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Tail appends during the live vector pass can tick in the same pass; compacting current/earlier removals can skip shifted successors. | `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`; `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`; disassembly `0x0055B608..0x0055B619`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `FUN_0055BAA0` uses `Object+0x98` as the active `LogicClass` membership guard; ordinary reveal/direct callers pass flag 0 and append at old count. | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`; `.swarm-claims.md` 2026-05-21/28 entries. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `FUN_0055BAE0` returns without lookup when `Object+0x98` is clear; if set, it find-indexes, stable-compacts the vector, clears `+0x98` even on not-found/out-of-range, and does not zero the stale tail. | `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `Object+0x98` is distinct from `InLimbo +0x81`, `IsAlive +0x90`, native `UniqueID +0x10`, and Rust stable IDs. | `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`; `UNIQUE_ID_MINUS_TWO_PRODUCERS_CONSUMERS_RESWARM_20260528.md`; Ghidra spot-check full Object ctor writes `+0x98=0` at `0x005F398D`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `ObjectClass::Reveal` gates before mutation on all-zero coords, game-active, `InLimbo==1`, `IsMarked==0`, and `CanEnter(...) == 0`. Nonzero `CanEnter` rejects with no side effects. | `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`; `REVEAL_DERIVED_CANENTER_RETURN_CODES_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Reveal clears limbo/redraw and writes transformed raw coords before `Mark(MARK_PUT)`. If `Mark` fails, only limbo is restored; raw coords remain changed. | `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Reveal registers into `LogicClass` only after successful Mark, only if alive, after display submit, gated by `ObjectType+0x234`, game mode, and `UniqueID==-2` sentinel rules, then alpha/trail side effects follow. | `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`; `REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md`; `OBJECTTYPE_REVEAL_SIDE_EFFECT_FLAGS_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `UniqueID==-2` skips register/unregister only in non-0/non-5 game modes. Modes 0 and 5 bypass that sentinel check; replay is not mode 5. | `REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md`; `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`. | confirmed | high | conditional | IMPLEMENTATION_SAFE |
| The live producer for native `UniqueID==-2` is the non-0/non-5 move-click feedback anim path, produced through temporary scenario counter `-3` plus normal `AssignUniqueID`, not by hardcoding the object field. | `UNIQUE_ID_MINUS_TWO_PRODUCERS_CONSUMERS_RESWARM_20260528.md`. | confirmed | high | conditional | IMPLEMENTATION_SAFE for gate; NEEDS_REINVESTIGATE for persistence |
| Direct non-Reveal registrations exist and must share the same future active-list API: BuildingLight and BFRT/OpenTopped passenger cases. WaveClass binding is UNVERIFIED — see correction below. | `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`; `DIRECT_FUN_0055BAA0_NON_REVEAL_CALLERS_RESWARM_20260528.md`; `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`; re-verified 2026-05-29 via `get_function_callers 0x0055BAA0`. | confirmed (WaveClass UNVERIFIED) | high (WaveClass: low) | conditional/stock-live | IMPLEMENTATION_SAFE |
| `ObjectClass::UnInit` order is bomb defuse, passenger/EMP hook, `Detach_From_All_Lists`, virtual Conceal, write `Object+0x90=0`, append pending-delete vector. | `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `ObjectClass::Conceal` unregisters from `LogicClass` under the same type/mode/UniqueID gate, then finishes alpha/line-trail and limbo/redraw writes. | `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`; `UNIQUE_ID_MINUS_TWO_PRODUCERS_CONSUMERS_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `Detach_From_All_Lists` is a pre-conceal listener broadcast, not a blind Rust-side pointer wipe. It clears UI/current pointers, then dispatches listener vtable `+0x28(expiring, removal_flag)` over `DAT_00B0F724`, with post-loop helper cleanup afterward. | `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`; `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`; Ghidra spot-check `0x007258D0`, including `0x0072593E..0x0072595F`. | confirmed | high | yes | IMPLEMENTATION_SAFE for ordering/registry shape; NEEDS_REINVESTIGATE for every callback body |
| The listener roster includes Object-derived instances and non-object observers such as House, Team, Factory, AlphaShape, and ParticleSystem. Object construction appends; destruction compact-removes. | `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`; Ghidra spot-check full Object ctor appends to `DAT_00B0F724` at `0x005F3A3B..0x005F3A8B`. | confirmed | high | yes | IMPLEMENTATION_SAFE for registry requirement |
| Standard save/load serializes the `LogicClass` active vector directly through `FUN_00551B20`/`FUN_00551B90`; it is not rebuilt from object storage or sorted IDs. | `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`; `OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Post-load swizzle fixes pointer slots only; there is no generic `FUN_0055BAA0` pass to reconcile `Object+0x98` with the active vector. | `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`; `OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `Object+0x98` is raw-streamed with the object, but its final byte after load is class-specific: vtable-only Object ctor load bodies preserve the raw byte; full Foot/Building/Techno constructor chains clear it. | `OBJECT_98_SAVE_LOAD_FINAL_BYTE_PROVENANCE_RESWARM_20260528.md`; Ghidra spot-check full ctor clears `+0x98` while vtable-only ctor `0x005F3B50` does not touch it. | confirmed | high | yes | IMPLEMENTATION_SAFE per class-family, not yet a blanket invariant |
| Replay startup uses normal scenario init, not a savegame active-vector restore or re-register-all pass. | `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `FUN_00551A30` is a render `LayerClass` adjacent-swap pass using vtable `+0xB8` and `ECX=0x008A0390`; it is not a `LogicClass` prepass or active-order helper. | `FUN_00551A30_ACTIVE_ORDER_PREPASS_RESWARM_20260528.md`; Ghidra spot-check callsite `0x0055DBC3..0x0055DBC8` and decompile of `FUN_00551a30`. | confirmed | high | yes | DOC_PATCH_READY / negative IMPLEMENTATION_SAFE |
| FactoryClass top-level save/load and PerTick order are global array order, not owner/category map order. This is adjacent evidence for scheduler/persistence parity, not part of ObjectClass membership. | `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`; `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER_RESWARM_20260528.md`. | confirmed | high | yes | IMPLEMENTATION_SAFE when production order is touched |

## Current Model

Native storage, construction, active logic membership, render-layer membership, and listener registration are separate systems. An object can exist in global class arrays and broad listener registries without being in the `LogicClass` active vector.

The central gameplay AI pass is `LogicClass::PerTickUpdate` on singleton `0x0087F778`. The active vector is live: the loop reads the object pointer immediately before vtable `+0x5C`, then reloads count after the call. This makes same-pass tail append possible and makes compacting removals observable.

`Object+0x98` is the native active-membership byte for the `LogicClass` vector. Add and remove helpers must be modeled as side-effecting vector operations with that byte as the guard, not as derived state from limbo, alive, object storage, or Rust stable ID presence.

Reveal is the main activation gate. It rejects without side effects until all entry gates pass. It then updates limbo/redraw/raw coords, attempts map Mark, and only after Mark success, alive status, display submit, type logic flag, game mode, and native UniqueID checks does it call the active-list add helper. Alpha image and line trail work follows active registration.

Conceal is the main active-removal gate, but full death/uninit does more. `ObjectClass::UnInit` broadcasts pointer expiry through `Detach_From_All_Lists` before Conceal, before alive clear, and before pending-delete append. This pre-conceal phase is where bullets, transports, capture managers, radio/contact holders, disk lasers, spawn managers, tactical pointers, and non-object observers get a chance to handle the expiring object.

Save/load preserves the active vector directly and preserves or resets `Object+0x98` according to the concrete load construction path. There is no generic post-load "rebuild active membership from all objects" pass. Replay is normal scenario initialization, not save-vector restoration.

`FUN_00551A30` must stay out of the active-object model. It operates on the ground display `LayerClass` and performs one adjacent-swap pass by Y-sort. It cannot justify sorting, filtering, or repairing `LogicClass` order.

## Implementation-Safe Facts

- Future Rust active membership needs a native-style vector plus per-object membership byte equivalent, with append-on-add, stable compaction-on-remove, stale-tail irrelevance, and live count-reload tick semantics.
- Reveal/Conceal APIs must own active-vector membership transitions; direct storage insertion/removal is not enough.
- Add/remove gating must include `ObjectType+0x234`, mode 0/5 bypass behavior, and the non-0/non-5 `UniqueID==-2` sentinel skip.
- Non-Reveal register callers must share the same active-list API: BuildingLight and OpenTopped passenger surfaces. (WaveClass binding UNVERIFIED — see Unverified Claims.)
- Uninit/despawn must run pointer-expiry listeners before Conceal, alive clear, and pending-delete/removal semantics.
- Save/load must preserve active vector stream order instead of rebuilding from sorted storage or `EntityStore`.
- Production/Factory work that touches scheduler order must use native global FactoryClass array order, not owner/category `BTreeMap` traversal.

## Doc-Patch-Ready Facts

- Replace any "`g_GameMode==5` is replay" claim with "mode 5 is offline Skirmish in these ObjectClass gates; replay playback is `DAT_00A8D5F8 & 2`."
- Replace any "`FUN_00551A30` is an active-object/order prepass" claim with "it is a render `LayerClass` adjacent-swap pass on `ECX=0x008A0390`; `LogicClass` tick later uses `ECX=0x0087F778`."
- Replace any "`Object+0x98` is on-map/limbo/alive" wording with "it is the `LogicClass` active-membership guard byte."
- Replace any "`ObjectClass__Save @ 0x005F6250` is normal stream save" wording with "that surface is CRC/checksum-style; IPersist stream save goes through `FUN_0065AC40 -> AbstractClass::Save`."
- Replace broad "post-load rebuilds active object order" wording with "standard save/load serializes the active vector directly and does not run generic re-registration."

## Stale Or Superseded Claims

- Older timing docs that label `g_GameMode==5` as replay are superseded for these gates by `REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md` and `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`.
- Older main-tick or active-order prose that treats `FUN_00551A30` as a `LogicClass` helper is superseded by `FUN_00551A30_ACTIVE_ORDER_PREPASS_RESWARM_20260528.md` and the callsite spot-check.
- The earlier post-load `Object+0x98` runtime-only uncertainty is superseded by `OBJECT_98_SAVE_LOAD_FINAL_BYTE_PROVENANCE_RESWARM_20260528.md`.
- Earlier FactoryClass top-level save/load partials are superseded by `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER_RESWARM_20260528.md`.

## Cross-Doc Conflicts

No broad conflict blocks the core Object/Logic lifecycle model. The remaining conflicts are doc-staleness issues: replay mode labeling, `FUN_00551A30` ownership, and save/load active-order rebuild assumptions.

## Unverified Claims (YELLOW)

- **WaveClass as a direct non-Reveal `FUN_0055BAA0` caller — UNVERIFIED (2026-05-29).** `get_function_callers 0x0055BAA0` returns exactly five callers: `BuildingLightClass__Constructor` (0x435820), `ObjectClass__Reveal` (0x5f4ec0), `TechnoClass__SetInOpenTransport` (0x710470, OpenTopped), and two unnamed functions `FUN_00437050` and `FUN_0075f8b0`. Decompiling both unnamed callers (`decompile_function 0x00437050`, `decompile_function 0x0075f8b0`): `FUN_00437050` is a thin Reveal-then-register wrapper (`if (Reveal(...)) { FUN_0055baa0(this,0); }`); `FUN_0075f8b0` is an un-limbo / Mark-then-register path (clears `+0x81` limbo, calls Mark via vtable `+0x1ac`, `Submit_Object`, then registers). Neither is identifiable as WaveClass, and no WaveClass RTTI/vtable evidence appears among the callers. The original "BuildingLight, WaveClass, and BFRT/OpenTopped" claim cited only the prior re-swarm docs; treat the WaveClass binding as unproven until a WaveClass registration path is traced directly in the binary.

## Needs Re-Investigation

- Runtime mutation rules for `DAT_00B0F724` while listener callbacks are executing.
- Exact bodies and side effects for Tactical `+0x28`, SpawnRetreat, and any listener callback still represented only by a broad roster name.
- Class-specific `vtable+0x5C` AI self-removal and same-pass removal cases, ranked by normal-game frequency.
- Save/load persistence behavior for already-live special `UniqueID==-2` feedback anims, if multiplayer feedback anim parity becomes implementation scope.
- Direct non-Reveal caller class fields and tests, when BuildingLight or OpenTopped passenger work becomes implementation scope.
- Identity of the two unnamed direct `FUN_0055BAA0` callers `FUN_00437050` and `FUN_0075f8b0` (re-verified present 2026-05-29 via `get_function_callers 0x0055BAA0`); `FUN_00437050` is a thin Reveal-then-register wrapper and `FUN_0075f8b0` is an un-limbo/Mark-then-register path. Neither was identifiable as WaveClass; confirm whether WaveClass registers at all and through which path.

## Do-Not-Implement Notes

- Do not sort or repair `live_object_order` using `FUN_00551A30`.
- Do not infer active membership from `EntityStore`, alive, limbo, occupancy, or display submission.
- Do not post-load re-register all objects to reconstruct the active vector.
- Do not clear alive or remove the entity before `Detach_From_All_Lists` listener dispatch.
- Do not replace listener dispatch with blind pointer nulling; several callbacks compute fallback pointers, cache cells, or notify non-object observers.
- Do not model native `UniqueID==-2` as `Object+0x98`, and do not hardcode the object field in the verified producer.

## Rust Touchpoints

- `src/sim/world/mod.rs`: the active-membership surfaces now match native semantics (re-verified 2026-05-29 by reading `src/sim/world/mod.rs:666-769`, `src/sim/world/logic_vector.rs`, `src/sim/game_entity.rs:172`). A dedicated `LogicVector` (`src/sim/world/logic_vector.rs`) backs the `logic: LogicVector` field (`mod.rs:319`); `register_live_object` (`mod.rs:680`) is a membership-guarded tail-append gating on `GameEntity::in_logic_vector` (`game_entity.rs:172`); `unregister_live_object` (`mod.rs:689`) is a flag-gated order-preserving compacting remove (`LogicVector::remove` = `retain(|&x| x != id)`); `live_object_order_snapshot` (`mod.rs:745`) returns `self.logic.snapshot()` verbatim with no sorted-ID fallback (the comment marks the old fallback as "was DRIFT"); `for_each_live_object` (`mod.rs:763`) re-reads `self.logic.len()` every iteration for native same-pass append/compaction semantics. The earlier "`Vec::contains`, unconditional `retain`, sorted fallback, post-storage removal cleanup" drift is RESOLVED. Remaining gap: the add gate-chain is still simplified — `reveal`/`register_live_object` do NOT enforce the native Reveal preconditions (Mark(MARK_PUT) success, `IsAlive`, `ObjectType+0x234`, game-mode, and `UniqueID==-2` sentinel), so registration in Rust is unconditional once `in_logic_vector` is clear.
- `src/app_sim_tick.rs`, `src/sim/world/world_orders.rs`, and `src/sim/passenger.rs`: current snapshot/fixed-phase ordering should not be treated as native `LogicClass` live-vector order.
- `src/sim/production/production_types.rs` and `src/sim/production/production_queue.rs`: owner/category `BTreeMap` production traversal is not native global FactoryClass array order.
- `src/sim/entity_store.rs` and `src/sim/game_entity.rs`: radio/contact/capture/passenger pointer cleanup exists, but not as a central pre-conceal listener broadcast with non-object observers.

## Source Ledger

- `LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`
- `LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`
- `ACTIVE_VECTOR_REMOVE_HELPER_FUN_0055BAE0_RESWARM_20260528.md`
- `OBJECTCLASS_REVEAL_EXACT_ORDERING_RESWARM_20260528.md`
- `REVEAL_DERIVED_CANENTER_RETURN_CODES_RESWARM_20260528.md`
- `REVEAL_GAMEMODE_OWNER_STATUS_GATE_RESWARM_20260528.md`
- `OBJECTTYPE_REVEAL_SIDE_EFFECT_FLAGS_RESWARM_20260528.md`
- `UNIQUE_ID_MINUS_TWO_PRODUCERS_CONSUMERS_RESWARM_20260528.md`
- `DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`
- `DIRECT_FUN_0055BAA0_NON_REVEAL_CALLERS_RESWARM_20260528.md`
- `BUILDINGLIGHT_HASSPOTLIGHT_REGISTRATION_RESWARM_20260528.md`
- `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`
- `DETACH_FROM_ALL_LISTS_LISTENER_EFFECTS_RESWARM_20260528.md`
- `DETACH_FROM_ALL_LISTS_LISTENER_ROSTER_CENSUS_RESWARM_20260528.md`
- `SAVE_LOAD_ACTIVE_VECTOR_RECONSTRUCTION_OWNER_RESWARM_20260528.md`
- `OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`
- `POST_LOAD_OBJECT_98_OWNER_RECONCILIATION_RESWARM_20260528.md`
- `OBJECT_98_SAVE_LOAD_FINAL_BYTE_PROVENANCE_RESWARM_20260528.md`
- `REPLAY_ACTIVE_VECTOR_RESTORE_CORNER_RESWARM_20260528.md`
- `FUN_00551A30_ACTIVE_ORDER_PREPASS_RESWARM_20260528.md`
- `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`
- `FACTORYCLASS_TOP_LEVEL_SAVE_LOAD_RESTORE_ORDER_RESWARM_20260528.md`
- Ghidra spot-checks in `gamemd.exe`: `0x0055DBC3..0x0055DC9E`, `FUN_00551a30`, `0x0055AFB0`, `0x007258D0`, `0x005F3900`, `0x005F3B50`.
