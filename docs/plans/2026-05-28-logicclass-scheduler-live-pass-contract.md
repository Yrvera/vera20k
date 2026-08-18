# LogicClass Scheduler — Live Count-Reload Pass Contract

Date: 2026-05-28
Contract: #1 (LogicClass Scheduler), driven paired with #6 (Projectile/Anim
same-tick) per the foundational scheduler roadmap.

Status of phases for this slice: **ALL 8 COMPLETE.**
- PHASE 1 (verify evidence): DONE — independent Ghidra pass, 9/10 claims VERIFIED
  (one clarification: vector lives on the LogicClass singleton passed as `this`).
- PHASE 2 (this doc): the behavioral spec. DONE.
- PHASE 3 (prove premise): DONE — see "Premise"; backed by the discriminating test
  `logic_scheduler_snapshot_walk_misses_same_pass_append`.
- PHASE 4 (adversarial review): DONE — second independent agent re-verified the
  binary from scratch and returned PASS (tail-append + compacting-skip confirmed;
  surfaced the sorted-insert-flag refinement now recorded below).
- PHASE 5 (implement): DONE — `Simulation::for_each_live_object` (`src/sim/world/mod.rs`).
- PHASE 6 (tests): DONE — 4 tests in `src/sim/snapshot.rs` (`logic_scheduler_*`).
- PHASE 7 (verify): DONE — 3266 passed incl. the 4 new; 10 failures = known
  baseline (movement×4, ai×1, ore_growth×1, production×4); determinism untouched.
- PHASE 8 (determinism review): DONE — second independent agent returned PASS
  (no HashMap/float/RNG/pointer dependence; serde order verbatim; native-order
  faithful on all four sub-cases; `as_slice()[i]` proven panic-safe under shrink).

## What already exists (working tree, uncommitted)

The *order primitive* is built and wired:

- `src/sim/world/logic_vector.rs` — `LogicVector`: insertion-ordered `Vec<u64>`,
  `push` (tail), `remove` (order-preserving `retain`, never swap), serde as the
  inner `Vec`.
- `Simulation.logic: LogicVector` (`src/sim/world/mod.rs:298`).
- `register_live_object` (`mod.rs:622`) — `+0x98`-equivalent idempotent guard
  (`GameEntity::in_logic_vector`) then tail-append. Matches gamemd `0x0055BAA0`.
- `unregister_live_object` (`mod.rs:631`) — flag-gated, then compacting remove.
  Matches gamemd remover `0x0055BAE0`.
- `despawn_entity` (`mod.rs:713`) unregisters before freeing the store slot —
  matches `ObjectClass::Conceal 0x005F4D30` → remover ordering.
- `live_object_order_snapshot` (`mod.rs:643`) — returns `logic.snapshot()` (a
  `Vec` clone), no sorted fallback.
- Consumers: `passenger.rs:355` (garrison-owner timing), `world_spawn.rs:260/438`,
  `drop_payload.rs:238`.

## The gap this contract closes

`live_object_order_snapshot()` returns a **snapshot**. Every consumer that does
`for &id in &order { … }` therefore CANNOT see an object appended during the pass,
and CANNOT observe a compacting removal shifting a successor into the current
index. The research explicitly forbids treating a pass-entry snapshot as scheduler
parity (SAME_TICK report, "Negative Facts": *"Do not snapshot all candidate
objects at pass entry and call that parity; native reloads live count and removals
compact the vector."*).

The native contract is a **live forward walk that re-reads count after every
object body** and never repairs the index.

## Verified binary behavior to reproduce (PHASE 1 evidence)

All from `LogicClass::PerTickUpdate @ 0x0055AFB0`, independently re-verified via
Ghidra MCP `disassemble_function`/`get_function_callers` this session:

1. The main object loop walks `items` at `this+0x04` with count at `this+0x10`,
   where `this` = the LogicClass singleton `0x0087F778` passed by
   `Main_Tick @ 0x0055D360` (`0055dc99 MOV ECX,0x87f778; CALL 0x0055afb0`).
   (Verified via `disassemble_function 0x0055AFB0`; the `EDI` used earlier in the
   function is ScenarioClass, NOT this vector — corrected from the report prose.)
2. Loop body: `items[i]` load (`0055b608`), `CALL [EDX+0x5c]` (`0055b610`),
   `INC ESI` (`0055b616`), **count RE-READ** `MOV EAX,[EDI+0x10]` (`0055b613`),
   `CMP ESI,EAX`, `JL 0055b608`. Count is re-read every iteration, not snapshotted.
3. No null guard on the item before the `vtable+0x5C` call (`0055b608..0055b610`).
   (Contrast: the HouseClass loop at `0x0055B698` *does* null-guard with
   `TEST ECX,ECX; JZ` — relevant to Contract 5, not here.)
4. Registration `0x0055BAA0`: `MOV AL,[ESI+0x98]; TEST AL,AL; JNZ` early-return
   (idempotent) → else `CALL DynamicVector__Insert 0x005519B0` → on success
   `MOV [ESI+0x98],1`.
5. `DynamicVector__Insert 0x005519B0`: reads old count `[ESI+0x10]`, writes object
   at `items[old_count]`, stores `count+1` — TAIL append. (PHASE 4 refinement,
   verified via `decompile_function 0x005519B0`: this function takes a third
   `flag` param that, when set, routes to a *sorted* insert; but the LogicClass
   registration helper `0x0055BAA0` pushes only 2 args, so the active-object path
   is ALWAYS a plain tail append. The sorted variant is never on this path.)
6. Remover `0x0055BAE0`: `DEC ECX; MOV [ESI+0x10],ECX` then left-shift
   `MOV EDX,[ECX+EAX*4]; MOV [ECX+EAX*4-4],EDX` — compacting, NOT swap;
   `MOV [EAX+0x98],0` clears membership. No tail-slot zero, no index repair.
7. Live in stock YR: `Main_Game @ 0x0048CCC0 → Main_Tick → PerTickUpdate`. Not a
   TS-only/debug path.

## The contract (output semantics, not C++ internals)

A live pass over the active-object order MUST produce these observable outcomes:

| Event during the pass | Required outcome | gamemd evidence |
|---|---|---|
| Object at index `< i` appends a new object at the tail | The new object IS visited later in the same pass. | count re-read `0055b613`; tail insert `0055519B0` |
| Object appends a new object, but the cursor already passed the new tail position (i.e. appended object lands at an index `<= i`) — impossible for tail append since tail is always `>= i` | n/a (tail append always lands at `len`, which is `> i`) | — |
| Re-register an object already a member | No-op: order unchanged, one membership entry, body runs once. | `+0x98` guard `0055baa5` |
| The current object (`index == i`) unregisters itself | Its successor shifts into index `i`; cursor still does `i += 1`, so that successor is SKIPPED this pass. | remover left-shift `0055bb11`; no repair `0055b610..0055b619` |
| An already-visited object (`index < i`) is unregistered | No effect on the remaining walk. | left-shift below `i` |
| A not-yet-visited later object (`index > i`) is unregistered | Successors shift left; because the cursor advances by 1 with no repair, exactly one later object may be skipped. | same |
| Iteration order | Ascending index from 0, insertion order (no sort). | `xor esi,esi 0055b5ff`; tail insert |

Non-goals for this slice: the other (copied-count scratch, reverse) loops inside
`0x0055AFB0`; class-specific `vtable+0x5C` bodies; save/load reconstruction beyond
the already-passing verbatim-order serde tests.

## Rust-native realization

Add to `Simulation` a live driver that mirrors the loop exactly:

```rust
/// Native LogicClass::PerTickUpdate-style forward pass. Re-reads the live
/// length after each body call, so a tail-append (register_live_object) made
/// before the cursor reaches it is visited in the SAME pass; a compacting
/// unregister shifts successors left while the cursor still advances by one,
/// reproducing the no-index-repair skip. The body must tolerate an id whose
/// entity is absent (no native null guard); despawn always unregisters first,
/// so store/order stay consistent.
fn for_each_live_object<F: FnMut(&mut Simulation, u64)>(&mut self, mut body: F) {
    let mut i = 0;
    while i < self.logic.len() {        // count RE-READ each iteration
        let id = self.logic.as_slice()[i];
        body(self, id);                 // may push/remove self.logic
        i += 1;                         // no index repair
    }
}
```

This is fixed-point-clean (only `usize`/`u64` index math, no float), deterministic
(order is the serde-stable `LogicVector`), and `sim/`-internal (no render/ui/audio/net).
`live_object_order_snapshot()` is retained for read-only consumers but is NOT the
scheduler-parity path; consumers that must observe same-pass membership changes use
`for_each_live_object`.

## Premise (PHASE 3)

The drift is real and current Rust gets it wrong: `live_object_order_snapshot()` is
a `Vec` clone, so `passenger.rs:355`'s `for &id in &order` (and every other
snapshot consumer) cannot visit a same-pass tail append nor observe a compacting
skip. The *player-visible* realization is the SAME_TICK report's named cases
(AAHeatSeeker2 same-pass first bullet AI; garrison muzzle-flash first-AI guard;
two-miners/one-refinery ordering). Those consumers are not yet routed through a
live pass (combat applies instant damage; projectiles are not authoritative
entities), so the visible fix lands when a consumer is migrated — the C6 vertical
specced below. This slice proves the premise with a discriminating test: a snapshot
walk MISSES a same-pass append while `for_each_live_object` catches it.

## Acceptance tests (PHASE 6)

Roadmap-named:
- `logic_scheduler_append_during_pass_ticks_new_tail_same_tick`
- `logic_scheduler_duplicate_registration_is_idempotent`
- `logic_scheduler_self_unregister_uses_compacting_index_semantics`
Plus a discriminating snapshot-vs-live test backing the premise.

## C6 consumer vertical (next, scoped — NOT in this slice)

To realize a player-visible same-tick fix, migrate ONE consumer onto
`for_each_live_object` where an object's body appends another logic object:
- **Option A — authoritative projectiles**: combat-fire spawns a `BulletClass`-style
  entity, registers it, and its first AI runs same-pass. Large: changes combat from
  instant-damage to authoritative bullets — high regression surface, needs its own
  premise/contract/tests. Recommended as a dedicated follow-up.
- **Option B — anim first-AI guard**: model the garrison muzzle flash as a logic
  object whose first same-pass AI clears the first-AI guard WITHOUT advancing the
  frame (SAME_TICK §5). Smaller, but current flashes are app-layer; needs a sim-side
  anim runtime to avoid crossing the sim→render boundary.

Both require their own Phase 1–4 before implementation.

## Do-not-do (from research)

- Do not snapshot count at pass entry for the live path.
- Do not `swap_remove`; removal is order-preserving compaction.
- Do not repair the index after a body call (the skip is the contract).
- Do not treat `BTreeMap` stable-id order as active order.
- Do not force newly registered objects to wait a tick by default.
- Do not generalize this contract to the other loops inside `0x0055AFB0`.
