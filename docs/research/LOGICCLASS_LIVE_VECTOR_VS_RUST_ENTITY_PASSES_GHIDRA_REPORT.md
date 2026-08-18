# LogicClass Live Vector vs Rust Entity Passes - Ghidra Report

**Target question:** Does current Rust have an equivalent to the active YR `LogicClass+0x04/+0x10` live appendable/removable forward object-AI vector, and which Rust passes are highest-risk because they snapshot/sort entities or split work by subsystem instead of native object-AI membership?

**Status:** COMPLETE for the scheduler-vs-Rust reconciliation requested here. This report does not prove every class-specific `vtable+0x5C` body or full global `LogicClass::PerTickUpdate` phase order.

**Address(es):** `LogicClass::PerTickUpdate @ 0x0055AFB0`; main object loop `0x0055B5FB..0x0055B619`; registration helper `0x0055BAA0`; remover `0x0055BAE0`; `DynamicVector__Insert @ 0x005519B0`.

**Active in YR:** Yes. Existing scheduler research verifies `Main_Tick` calls `LogicClass::PerTickUpdate` with `ECX=0x87F778` at `0x0055DC99..0x0055DC9E`, and this slot rechecked the live object loop in Ghidra.

## Non-goals

- Full global subsystem ordering inside `LogicClass::PerTickUpdate`.
- Frame-counter/pre-increment timing.
- `FactoryClass`/`HouseClass` late-loop ordering.
- Same-tick projectile examples except as supporting scheduler consequences.
- Exhaustive class-specific self-removal/destruction paths inside each `vtable+0x5C` body.

## Evidence Needed To Mark COMPLETE

- Reconfirm native main object loop from live Ghidra decompile and assembly context.
- Reconfirm append and removal helper mechanics that affect same-pass scheduling.
- Inspect Rust `Simulation::advance_tick`, `EntityStore`, and high-risk entity passes for a LogicClass-equivalent live vector or snapshot/sorted substitutes.
- Produce a Rust handoff with acceptance scenarios that distinguish live-vector semantics from sorted/snapshot phase semantics.

## Stop Conditions

- Stop before mutating Ghidra state. No rename/comment/save tools were used.
- Stop before modifying Rust or INI files.
- Stop after one report plus optional `.swarm-claims.md` update.

## Verified Binary Facts

| Fact | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| The main object-AI loop walks a LogicClass-owned pointer vector forward, using `LogicClass+0x04` as the item array and `LogicClass+0x10` as count. | Decompile of `0x0055AFB0`; assembly context `0x0055B5FB..0x0055B619`: `MOV EAX,[EDI+0x10]`, `MOV EAX,[EDI+0x4]`, `MOV ECX,[EAX+ESI*4]`. | High | Yes |
| The per-object method is `vtable+0x5C`, and the loop reloads count after every call rather than snapshotting count at pass entry. | Assembly context `0x0055B610..0x0055B619`: `CALL [EDX+0x5c]`, `MOV EAX,[EDI+0x10]`, `INC ESI`, `CMP ESI,EAX`, `JL 0x0055B608`. | High | Yes |
| Ordinary registration is membership-gated and tail-appending. | Assembly context `0x0055BAA0..0x0055BAC6` checks `Object+0x98`; `DynamicVector__Insert @ 0x00551A0A..0x00551A1D` reads old count, stores count+1, writes object to `items[old_count]`, returns success. | High | Yes |
| Removal compacts the vector and clears `Object+0x98`; the scheduler does not repair the current index after the object call. | Remover assembly context `0x0055BB09..0x0055BB27` decrements count, shifts later entries left, clears byte `+0x98`; scheduler post-call context is only count reload, index increment, compare. | High | Yes |
| The main object loop has no item null guard in the scheduler path. | Assembly context `0x0055B608..0x0055B610` loads item pointer, then vtable pointer, then calls `+0x5C` directly. | High | Yes |

## Rust Findings

| Rust surface | Finding | Evidence | Classification |
|---|---|---|---|
| `EntityStore` | Stores all entities in `BTreeMap<u64, GameEntity>`, keyed by stable id. It is deterministic storage, not a separate active object-AI vector with membership bit/order. | `src/sim/entity_store.rs:33` (`pub struct EntityStore`), `:35` (`entities: BTreeMap<u64, GameEntity>`). Corrected 2026-05-29 (was `:23/:31/:33`, stale after file growth): re-anchored by Reading `src/sim/entity_store.rs`. | Verified Rust source |
| `EntityStore::keys_sorted` | Returns a newly collected `Vec<u64>`. Any pass using it snapshots membership for that pass. Tail inserts during the pass cannot be discovered by that pass unless the pass explicitly re-queries. | `src/sim/entity_store.rs:109` (`pub fn keys_sorted`), body `:110` (`self.entities.keys().copied().collect()`). Corrected 2026-05-29 (was `:102/:107`): re-anchored by Reading `src/sim/entity_store.rs`. | Verified Rust source |
| `Simulation::advance_tick` | Uses fixed subsystem phases: movement, air/special movement, vision, power, superweapons, deploy/fear, combat, particles, retaliation/passengers, production/repairs/docks/ore, AI, defeat, building/world-effect animation. | `src/sim/world/mod.rs:1508` (`advance_tick`); phases now split by a SPINE-region refactor — Phases 1-7 inline at `1534` (ground movement), `1567` (air/special), `1690` (vision), `1704` (power), `1714` (superweapons), `1721`/`1730` (deploy/fear), `1732` (combat+turret), `1966` (particles), `1970` (retaliation/passengers), `1976` (production/repairs/docks/ore); Phases 8/8.5/9 (AI, defeat, building anims) extracted into `run_late_region` at `1418`/`1451`/`1458`. Corrected 2026-05-29 (was `:1187` + flat phase list `1245/1276/1422/1646/1655/1757/1790/1799`, all stale after file growth + spine-region extraction): re-anchored by Reading `src/sim/world/mod.rs` and grepping `fn advance_tick` and `// --- Phase`. | Verified Rust source |
| High-risk per-entity passes | Many object-like passes snapshot sorted ids before mutation: movement, air, rocket, homing, teleport, tunnel, droppod, parachute, deploy, infantry fear, turret rotation, combat, animation, passenger. | Examples: `src/sim/movement/movement_tick.rs:816`, `air_movement.rs:204`, `rocket_movement.rs:140`, `homing_movement.rs:386`, `teleport_movement.rs:221`, `tunnel_movement.rs:187`, `droppod_movement.rs:106`, `parachute_descent.rs:93`, `src/sim/deploy.rs:81`, `src/sim/infantry.rs:135`, `src/sim/movement/turret.rs:95`, `src/sim/combat/mod.rs:1212`, `src/sim/animation.rs:396`, `src/sim/passenger.rs:623`. | Verified Rust source |
| LogicClass-equivalent live vector | No Rust equivalent was found in the scanned surfaces: no scheduler-owned active object pointer/id vector, no membership bit equivalent to `Object+0x98`, and no generic forward object-AI pass that reloads vector count after each object body. | Source scan of `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, and `src/sim/**` `keys_sorted`/phase callers. | Rust-facing inference from static scan |

## Reconciliation

The current Rust tick is deterministic, but deterministic stable-id order is not the native contract. Native YR does not simply iterate all stored entities in sorted id order. It iterates a LogicClass active-object vector whose membership is separate from object storage, whose order is insertion order, and whose count is live after each object AI call.

The most important consequence is same-pass membership change. A logic-enabled object appended by an object already processed can still run later in the same native pass if it is appended before the forward loop reaches the new tail. A Rust pass that starts with `let keys = entities.keys_sorted()` cannot see that append in the same pass. Likewise, compacting removal in native preserves vector order and can shift entries relative to the already-incremented index; sorted `BTreeMap` iteration or swap-like removal would not reproduce that behavior.

## Highest-Risk Rust Surfaces

| Priority | Surface | Why it is high risk |
|---:|---|---|
| 1 | Combat/projectile bodies: `tick_combat_with_fog`, homing/rocket movement | Projectile creation, detonation, death/despawn, turret/fire decisions, and first-tick latency are directly sensitive to same-pass scheduling. |
| 2 | Movement/locomotor bodies: ground, air, teleport, tunnel, droppod, parachute | Native object AI membership cuts across locomotor type; Rust splits movement into many type-specific sorted passes. |
| 3 | Deploy/fear/turret/animation per-entity passes | These are object-state bodies that currently run as separate sorted snapshots, not as one membership-ordered object AI pass. |
| 4 | Passenger/docking/miner/production-adjacent updates | These create/remove/link entities and contacts while reading global state; sorted snapshots risk next-tick-only behavior where native may see live membership. |
| 5 | Despawn/conceal/unlimbo/reveal helpers | Native scheduler semantics depend on separate active-list registration/removal; Rust entity insertion/removal alone is not enough. |

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Native object AI uses a live forward active vector and reloads count after every `vtable+0x5C`. | Rust has fixed subsystem passes and many `keys_sorted()` snapshots; a newly inserted entity usually cannot be seen by the same pass that collected keys. | Future sim-level LogicClass-equivalent scheduler; combat/projectile/movement integration. | Entity A's AI registers logic-enabled entity B at tail; same scheduler pass calls B if A was before the old end. | `logic_scheduler_append_during_pass_ticks_new_tail_same_tick` | High: first-tick projectile/anim/unit AI latency drift. |
| Native registration is idempotent by object membership bit and tail-appends on first registration. | Rust storage membership is `EntityStore` membership; no separate active-logic membership/order was found. | `EntityStore`, `GameEntity`, future reveal/unlimbo/conceal/despawn scheduler hooks. | Register A twice; active list contains A once, and one scheduler pass calls A once. | `logic_scheduler_duplicate_registration_is_idempotent` | High: duplicate AI calls or missing active objects. |
| Native removal compacts by shifting left and the current scheduler index is not repaired after the object call. | Rust removal semantics for any future active list are undefined; sorted map storage does not encode native compacting-index behavior. | Future active-list remove/unregister/despawn handling. | Register A,B,C; B unregisters itself during its object body; remaining same-pass behavior matches compacting remove plus post-call `INC index`. | `logic_scheduler_self_unregister_uses_compacting_index_semantics` | Medium-high: skipped/double-ticked object drift in death/conceal/self-delete paths. |

## Negative Facts / Do Not Do

- Do not treat `BTreeMap<u64, GameEntity>` sorted iteration as equivalent to the native LogicClass active-object vector. The native vector is separate, insertion-ordered, and membership-gated.
- Do not snapshot active object count at pass entry for the LogicClass-equivalent object loop.
- Do not force newly spawned/revealed logic-enabled objects to wait until the next tick by default.
- Do not use `swap_remove` semantics for a LogicClass-equivalent list; native remover shifts later entries left.
- Do not generalize this live-vector contract to every loop inside `LogicClass::PerTickUpdate`; the function also has copied-count and reverse-array loops per the existing scheduler report.

## Remaining Uncertainty

- Which common class-specific `vtable+0x5C` bodies remove themselves or earlier vector entries during the pass remains a separate trace target.
- Save/load and replay reconstruction of active-list membership/order was not traced.
- This report identifies Rust surfaces at risk, but does not prescribe whether the final design should be one central scheduler, compatibility shims around selected object types, or staged migration. The parity constraint is the observable live-list semantics, not a required Rust shape.

## Stale-doc Wording

- No stale wording was found inside `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`; it already states the live-vector contract and the Rust `keys_sorted()` mismatch.
- If older docs still say `LogicClass::AI() tick loop -> iterates all entities -> calls AI on each`, replace with: `LogicClass::PerTickUpdate @ 0x0055AFB0 contains the active per-object scheduler loop. It walks the LogicClass-owned object vector forward, calls vtable+0x5C, and re-reads count after each call; LogicClass::AI is not this object-AI loop.`

## Sources

- Live Ghidra decompile: `LogicClass::PerTickUpdate @ 0x0055AFB0`.
- Live Ghidra assembly context: `0x0055B5FB`, `0x0055B601`, `0x0055B608`, `0x0055B610`, `0x0055B613`, `0x0055B616`, `0x0055B619`, `0x0055BAA0`, `0x0055BAE0`, `0x00551A0A`, `0x00551A13`, `0x00551A1D`, `0x0055BB09`, `0x0055BB11`, `0x0055BB21`, `0x0055BB27`.
- Existing report: `docs/research/LOGICCLASS_PERTICKUPDATE_SCHEDULER_GHIDRA_REPORT.md`.
- Rust source: `src/sim/entity_store.rs`, `src/sim/world/mod.rs`, `src/sim/movement/*`, `src/sim/combat/mod.rs`, `src/sim/deploy.rs`, `src/sim/infantry.rs`, `src/sim/animation.rs`, `src/sim/passenger.rs`.
