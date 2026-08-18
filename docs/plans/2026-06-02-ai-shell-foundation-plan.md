# AI Shell Migration — Plan 1: Foundation (small-first)

**Status:** DRAFTED — not approved
**Date:** 2026-06-02
**Rule:** Rust-native structure, gamemd-native semantics. (Translate the verified gamemd behavior contract into idiomatic Rust — `match category` + capability fields + `Option<T>`, never the C++ class tree, `dyn`/vtable, or COM plumbing.)
**Companions:** `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (the design doc; §7.1/§7.3/§8/§9-S0/§9-S1/§10.2) + the in-flight mission/radio substrate plan (the `MissionCom`/`Contacts`/`RadioBus` slices that landed `refresh_mission_shadow`, `MissionTimer`, the substrate, and the live-object scheduler this plan builds on).

## Overview

This is the prerequisite scaffold every later AI-shell plan plugs into. It lands the per-object AI dispatch spine (a `match category` shell walked in live LogicVector order) inert and proven hash-neutral, then takes the first behavior-bearing step — observing, in shadow, that mission dispatch precedes the locomotor `Process` for one bounded moving-`UnitClass` scenario — and finally closes two isolated lifecycle/parity fixes that are independent of the shell but worth landing on this foundation. Nothing observable moves until a future plan explicitly flips an authority: S0 is a strict no-op, S1 is a read-only `#[cfg(debug_assertions)]` shadow, and the FIX slice is lifecycle-correctness plus a doc/verdict cleanup. Every slice is shadow-first per invariant #4 — the scaffold itself lands shadowed (debug-only visit trace, agreement asserts, no hashed-state mutation) before any per-leaf authority is absorbed in S2+. The slices below are reproduced verbatim from their individual review passes; only this overview and the gating/index/recap sections are connective tissue.

## Dependency order & gating

```
S0  (no-op shell)  ─────►  S1  (first UnitClass ordering slice, shadow)  ─────►  [S2+ in later plans]
                                                                                    (authority flips)
FIX (isolated lifecycle/parity fixes) ── independent of S0/S1; lands on this foundation, either order
```

- **S0 → S1 is a hard ordering.** S1's whole meaning ("dispatch precedes Process inside one object pass") requires the per-object shell pass S0 establishes. S1 collapses the *narrow* S0 skeleton it needs (the `techno_ai.rs` module + a single scoped step) into itself if S0 has not landed separately; if a parallel session lands the full S0 passthrough first, S1 folds `unit_ai_shadow_step` into it rather than duplicating the module. The design doc sequences S0→S1→S2 around one `world/techno_ai.rs` (table at design `:905`).
- **S0 lands before any S1+ "absorb" slice.** S0 builds the dispatch + ordering scaffold whose four arms (`Unit | Infantry | Structure | Aircraft`) are strict no-ops; later slices fill one arm at a time. S0 carries no RNG-position or frame-anchored-timer obligations — those land with the S2+ slices that absorb combat/mission dispatch.
- **FIX is independent.** Neither fix depends on the mission/radio slices, S0, or S1. FIX (1) depends only on the already-landed `uninit`/`flush_pending_delete` chokepoint; FIX (2) depends on nothing. Both are independent of each other and may land in either order / separate commits. They are bundled into this foundation plan because they are small, isolated, and clear the deck before the authority-flipping plans.
- **No authority flips in this plan.** Every slice here is no-op (S0), shadow (S1), or lifecycle-correctness/cleanup (FIX). The first authoritative flip (routing scoped movement through the shell + the `+0xC4` increment + `SNAPSHOT_VERSION` bump) is S2, deferred to a later plan.

The gating tripwire across S0/S1 is the per-tick `debug_assert` (order proof in S0; dispatch<process + scope-consistency in S1) plus the named no-hash-change tests — a regression in any of them fails immediately in any replay/lockstep test because the stage fires every tick.

---

## Slice S0 — Instrumented No-Op Object-AI Shell

> **Review notes (what I corrected/confirmed against the live tree):**
> - **All `file:line` claims re-verified and correct.** The draft already corrected the FACTS-block drift; the live tree confirms: `refresh_mission_shadow` def `mod.rs:895`, `live_object_order_snapshot` `:929`, `for_each_live_object` `:947`, `set_logic_order_for_test` `:958`, wire-in `self.refresh_mission_shadow();` `:2391`, `Simulation` struct `:271`, `mod` decl block `:13-20`. `EntityCategory::Structure` (NOT `Building`) `entities.rs:25`; `category` `game_entity.rs:181`; `dying` `:371`; `Presence{Limbo,InCell,Dying}` `:144`; precedent test `world_tests.rs:565`. No pre-existing `techno_ai`/`object_ai_stage` (clean slate).
> - **CORRECTED test #4 (`skips_dying_object`):** `set_logic_order_for_test` (`mod.rs:958-965`) forces `presence = Presence::InCell` and `in_logic_vector = true` for every id but does **NOT** touch `dying`. The test must set `dying = true` on the entity **AFTER** calling `set_logic_order_for_test`, or the skip won't trigger. Spelled out below.
> - **CONFIRMED placement is post-drain (new, load-bearing):** `flush_pending_delete` runs at `mod.rs:1719` *inside* `run_late_region`, which returns at `:2383` — before the `:2391` wire-in. So `object_ai_stage` runs after all corpses are freed; the "tolerate absent id" path is a defensive guard inheriting `for_each_live_object`'s contract, not a hot path this slice. Documented so a future slice that moves the call earlier knows the corpse-window changes.
> - **CONFIRMED `EntityStore::get` takes a bare `u64`** (`entity_store.rs:96`), so `sim.substrate.entities.get(id)` (not `get(&id)`) is correct — matches the precedent test's `entities.get(1)`.
> - **CONFIRMED `test_default` signature** is `test_default(stable_id: u64, type_ref: &str, owner: &str, rx: u16, ry: u16)` (`game_entity.rs:730`) — positional order in the draft is right.
> - Invariants (1)-(8) all honored; details in §7-§8.

**Substrate:** TechnoClass/FootClass object-AI dispatch scaffold
**Status:** Implementation plan (read-only research complete; no Rust written this session). Source of truth: `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §7.1/§9-S0 + the live tree.
**Date:** 2026-06-02

### 1. Approach choice

Two siting options exist. **Chosen:** one new file `src/sim/world/techno_ai.rs` with `Simulation::object_ai_stage(&mut self)` + a free `techno_ai_shell(...)`, wired into `advance_tick` next to `refresh_mission_shadow`. **Rejected alternative:** a `src/sim/ai/` *directory* (`ai/mod.rs::object_ai_stage` + per-leaf files).

**Why `world/techno_ai.rs`:** `src/sim/ai.rs` already exists as a single-file skirmish-AI command producer (`pub mod ai;` at `sim/mod.rs`, imported at `world/mod.rs:39` as `use crate::sim::ai::{self, AiPlayerState};` and called in `run_late_region` `:1653`). Creating `sim/ai/mod.rs` would require first restructuring `ai.rs → ai/mod.rs` — a rename touching `sim/mod.rs` and every `crate::sim::ai::` consumer — for zero S0 benefit and real blast radius. The §9-S0 harness names `world/techno_ai.rs`; it sits beside the substrate it consumes (`ObjectSubstrate`, `for_each_live_object`, `LogicVector`). The `sim/ai/` directory is the S1+ horizon, explicitly deferred. S0 builds the dispatch + ordering scaffold the later "absorb" slices plug into; it moves **nothing observable**.

### 2. Goal

Introduce a per-`EntityCategory` object-AI dispatch stage that:
- walks substrate **live LogicVector order** via the proven same-pass re-read loop (`for_each_live_object`, `mod.rs:947`),
- dispatches each live, non-dying object through a `match category` shell whose four arms (`Unit | Infantry | Structure | Aircraft`) are **strict no-ops** this slice,
- records a **debug-only** per-object visit trace and asserts it matches the frozen live order,
- is wired as a new `advance_tick` stage that provably changes **no observable state** — `state_hash` is bit-identical with and without it.

No behavior moves. No phase reorders.

### 3. Files / surfaces (exact `file:line`, verified this session)

| Surface | Path:line | Role in S0 |
|---|---|---|
| **NEW** stage file | `src/sim/world/techno_ai.rs` (create) | hosts `object_ai_stage` + 4-arm shell + S0 tests |
| Module decl | `src/sim/world/mod.rs:13-20` (`mod` block) | add `mod techno_ai;` |
| Same-pass re-read loop | `src/sim/world/mod.rs:947` `for_each_live_object` | the walk S0 reuses (re-reads `logic.len()` each iter; tolerates absent id) |
| Frozen order snapshot | `src/sim/world/mod.rs:929` `live_object_order_snapshot` | the debug assert compares against this |
| Wire-in point | `src/sim/world/mod.rs:2391` `self.refresh_mission_shadow();` | call `object_ai_stage(self)` immediately before this (after `run_late_region` returns `:2383`, before mission shadow + `state_hash` `:2394`) |
| Hash boundary | `src/sim/world/world_hash.rs:33` `state_hash` | S0 must leave every input here untouched |
| LogicVector | `src/sim/world/logic_vector.rs:13` | authoritative + hashed (`world_hash.rs:51-56`); S0 must not mutate |
| Substrate | `src/sim/world/substrate.rs:48` `ObjectSubstrate` | `logic` / `entities` accessed read-only |
| Category enum | `src/map/entities.rs:19` `EntityCategory` = `{Unit, Infantry, Structure, Aircraft}` | the `match` discriminant — **`Structure`, NOT `Building`** |
| Per-object reads | `game_entity.rs:181` `category`, `:371` `dying`, `:144` `Presence{Limbo,InCell,Dying}` | live-active gate inputs |
| EntityStore accessor | `src/sim/entity_store.rs:96` `get(stable_id: u64) -> Option<&GameEntity>` | read category/dying inside the visit (bare `u64`, not `&u64`) |
| Precedent stage | `src/sim/world/mod.rs:895` `refresh_mission_shadow` | hash-neutral side-walk pattern to mirror |
| Precedent test | `src/sim/world/world_tests.rs:565` `mission_shadow_does_not_change_state_hash` | no-hash-change test pattern to mirror |
| Test helpers | `mod.rs:958` `set_logic_order_for_test` (`#[cfg(test)]`); `GameEntity::test_default(stable_id, type_ref, owner, rx, ry)` (`game_entity.rs:730`) | seed live order + entities in tests |
| Flush boundary (note) | `mod.rs:1719` `flush_pending_delete` (inside `run_late_region`) | proves S0 runs post-drain — see §4 Task 3 |

### 4. Step-by-step tasks

#### Task 1 — Create `src/sim/world/techno_ai.rs`

```rust
//! Per-object AI dispatch scaffold (TechnoClass/FootClass spine, Slice S0).
//!
//! Walks the substrate's live object order and dispatches each live object
//! through a per-`EntityCategory` shell. THIS SLICE the shell is a strict
//! no-op: it visits every live object exactly once in live order, records a
//! debug-only visit trace, and changes nothing the lockstep hash observes.
//! Later slices replace the no-op arms with the absorbed per-leaf behavior
//! (movement, turret, combat, mission dispatch) one at a time.
//!
//! Depends on: `world::Simulation` (substrate live order + entity store).
//! Must NOT depend on render/ui/sidebar/audio/net (sim invariant #1).
//! Dispatch is `match category` only — no trait object / dyn / vtable
//! (invariant #2). No RNG, no hashed-state mutation, no phase reorder.

use super::Simulation;
use crate::map::entities::EntityCategory;

impl Simulation {
    /// Object-AI stage (Slice S0: instrumented no-op).
    ///
    /// Iterates the live LogicVector order via `for_each_live_object` — the
    /// same-pass re-read contract the native scheduler uses — and dispatches
    /// each live, non-dying object through `techno_ai_shell`. The shell does
    /// nothing behavior-bearing this slice; the stage exists to pin the
    /// dispatch + ordering scaffold and prove hash-neutrality.
    pub(crate) fn object_ai_stage(&mut self) {
        // Debug-only visit trace; never read in release, never hashed.
        #[cfg(debug_assertions)]
        let mut visit_order: Vec<u64> = Vec::new();

        self.for_each_live_object(|sim, id| {
            // Tolerate an absent id (the loop's documented contract). In S0 the
            // stage runs AFTER end-of-tick flush_pending_delete, so the order
            // should not reference a freed slot — but inherit the guard anyway.
            let Some(entity) = sim.substrate.entities.get(id) else {
                return;
            };
            // Live "is active" gate — the corpse/teardown skip. A dying object
            // is mid death-animation and is not dispatched.
            if entity.dying {
                return;
            }
            let category = entity.category;
            #[cfg(debug_assertions)]
            visit_order.push(id);

            techno_ai_shell(sim, id, category);
        });

        // Order proof: the same-pass re-read visit order must equal the frozen
        // live-order snapshot taken AFTER the pass. They diverge only if the
        // shell tail-appended/removed a live object mid-pass — which a no-op
        // shell never does. First regression tripwire for any future arm that
        // accidentally mutates the live order (invariant #3 guard).
        //
        // NOTE: this equality holds because the dying-skip filters the VISIT
        // trace but NOT the snapshot; in S0 tests we keep all snapshot ids live
        // except where a test explicitly probes the skip (that test compares the
        // visit list against an expected subset, not the raw snapshot).
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            visit_order,
            self.live_object_order_snapshot(),
            "object_ai_stage visit order diverged from live LogicVector order"
        );
    }
}

/// Per-category dispatch shell. Slice S0: every arm is a strict no-op.
///
/// `match category` — NO trait/dyn (invariant #2). `sim`/`id` are threaded so
/// later slices can fill an arm with the absorbed behavior without changing
/// this signature. `#[allow(unused_variables)]` until the arms do work.
#[allow(unused_variables)]
fn techno_ai_shell(sim: &mut Simulation, id: u64, category: EntityCategory) {
    match category {
        EntityCategory::Unit => {}      // S1+: absorb movement/turret/combat/mission
        EntityCategory::Infantry => {}  // S1+: absorb fear/scatter/sequence
        EntityCategory::Structure => {} // S1+: absorb building gate/anim dispatch
        EntityCategory::Aircraft => {}  // S1+: absorb aircraft-mission dispatch
    }
}
```

**Important on the order-proof assert:** the `debug_assert_eq!(visit_order, snapshot)` is exact **only when every id in the live order is non-dying** (because the visit trace skips `dying` but the snapshot does not). That is true in the normal hash-neutrality test and in `advance_tick` (corpses already flushed at `:1719`). The dedicated dying-skip test (#4 below) therefore must seed the dying entity such that it is the only divergence and assert against an explicit expected subset — NOT against the raw snapshot. If a future slice needs the assert to survive a dying member in the order, change the assert to filter the snapshot by `!dying` too; for S0 keep it as-is (the simplest tripwire) and shape the test around it.

**Notes binding to verified facts:**
- `for_each_live_object` (`mod.rs:947`) already encodes the native scheduler contract (re-reads `logic.len()` each iter; documented to tolerate an absent id at `:944-946`) — S0 reuses it.
- `EntityCategory::Structure` (NOT `Building`; `map/entities.rs:25` confirms).
- The `entity.dying` skip (`game_entity.rs:371`) is the closest live `IsActive` analogue today. No `Simulation::category(id)`/`is_active(id)` accessor exists, so the visit reads `entities.get(id)` fields inline — do **not** add new accessors this slice.
- `entities.get(id)` takes a bare `u64` (`entity_store.rs:96`).
- The `match` is exhaustive over the 4 real variants — no `_` arm, so a future enum addition is a compile error (intentional).

#### Task 2 — Declare the module in `world/mod.rs`

Add `mod techno_ai;` to the `mod` block at `mod.rs:13-20` (e.g. after `mod substrate;`). The impl block in `techno_ai.rs` extends `Simulation`, so a bare `mod techno_ai;` suffices — no `pub` re-export (`object_ai_stage` is `pub(crate)` on `Simulation`).

#### Task 3 — Wire the stage into `advance_tick`

In `world/mod.rs`, immediately **before** `self.refresh_mission_shadow();` (`:2391`, which runs after `run_late_region` returns at `:2383`):

```rust
        // Object-AI stage (Slice S0): instrumented no-op walk over the live
        // object order. Runs after all phases (incl. the end-of-tick
        // flush_pending_delete drain inside run_late_region) and before the
        // mission shadow + state_hash, so its no-op-ness is observable in this
        // tick's hash. Later slices absorb per-leaf behavior into it WITHOUT
        // moving this call site.
        self.object_ai_stage();
        self.refresh_mission_shadow();
```

**Why here, not inside the Phase 1-7 snapshot passes:** movement / special-movement / retaliation iterate a *frozen* `live_object_order_snapshot`; S0 demonstrates the *same-pass re-read* loop, which belongs in the late region beside the other read-only side-walk (`refresh_mission_shadow`). Placing it just before `state_hash` makes the no-change claim observable in the very next hash. The stage runs **after** `flush_pending_delete` (`:1719`, inside `run_late_region`), so all corpses are already freed — the absent-id guard is defensive, not a hot path this slice. **No other phase moves** (invariant #3).

### 5. What becomes authoritative / shadow

- **Nothing becomes authoritative.** S0 introduces a stage that owns no state; all four arms are no-ops.
- **The visit trace is shadow-only** in the strongest sense: a `#[cfg(debug_assertions)]` local `Vec<u64>`, never stored on `Simulation`, never serialized, never hashed; it exists solely for the per-tick `debug_assert`.
- **All existing authority is untouched:** LogicVector order stays authoritative + hashed; `MissionCom`/`Presence` stay shadow as before; the legacy per-leaf systems (movement, turret, combat, missions) remain the sole authority.
- This is the shadow-first landing of the *dispatch scaffold itself* (invariant #4): the scaffold lands inert and proven hash-neutral before any slice flips a per-leaf authority into it.

### 6. Named acceptance tests (`techno_ai.rs` `#[cfg(test)]`)

Per design §9-S0:

1. **`techno_ai_shell_is_passthrough_no_hash_change`** — mirror `mission_shadow_does_not_change_state_hash` (`world_tests.rs:565`): `Simulation::new()`; insert `GameEntity::test_default(id, type_ref, owner, rx, ry)` across all four categories via `sim.substrate.entities.insert(...)`; register them in the live order with `set_logic_order_for_test(vec![...])`; capture `state_hash()`; call `object_ai_stage()`; capture again; `assert_eq!(before, after)`. Proves the stage is hash-neutral.

2. **`techno_ai_shell_membership_matches_phase_snapshot`** — seed a known live order via `set_logic_order_for_test(vec![..])` (`mod.rs:958`); run `object_ai_stage()`; assert (via the in-test visit recorder — see note) the visited ids equal `live_object_order_snapshot()` exactly — every live object visited **once**, in live order. Run on a tick with no mid-stage spawn so same-pass re-read == frozen snapshot (the no-op shell guarantees this). The per-tick `debug_assert` inside `object_ai_stage` already enforces this in debug builds; this test makes it an explicit, named gate.

3. **`techno_ai_shell_preserves_advance_tick_phase_order`** — drive several full `advance_tick` calls on a small fixture and capture the per-tick `state_hash` sequence; assert it is bit-identical to a baseline. **Implementation note:** since `object_ai_stage` cannot be conditionally compiled out cleanly in this test, capture the baseline as a pinned golden `Vec<u64>` of per-tick hashes recorded from the same fixture (regenerate the golden only when an intentional behavior slice lands). This confirms the new stage perturbs no phase and no surrounding ordering.

Regression guards (same file):

4. **`object_ai_stage_skips_dying_object`** — insert two live entities + register the order via `set_logic_order_for_test`, **then** set `dying = true` on one of them (`sim.substrate.entities.get_mut(id).unwrap().dying = true`) — this must happen **AFTER** `set_logic_order_for_test`, because that helper resets `presence` to `InCell` and `in_logic_vector` to `true` but does **not** touch `dying`. Use an in-test visit recorder (or assert via a `#[cfg(test)]` hook) to confirm the dying id is **not** in the visited list while the live id is, and the loop does not panic. Because the dying id remains in the snapshot, this test compares the visit list against the explicit expected `[live_id]`, NOT against `live_object_order_snapshot()` (see §4 assert note).

5. **`object_ai_stage_tolerates_absent_id_in_order`** — force the live order to include a stable id that has no entity in the store (call `self.substrate.logic.set_order_for_test(vec![absent_id, live_id])` directly, since `set_logic_order_for_test` only flips flags on ids that exist), with a real entity for `live_id`. Run `object_ai_stage()`; assert it skips the absent id without panic and still visits `live_id`.

**Visit-recorder for tests:** since the `visit_order` `Vec` is a private local, tests #2/#4 need a way to observe it. Cleanest option within S0 scope: add a `#[cfg(test)]`-only `pub(crate)` field `last_object_ai_visit: Vec<u64>` on `Simulation` written at the end of `object_ai_stage` under `#[cfg(test)]` (NOT serialized, NOT hashed — `#[cfg(test)]` fields are absent from the release struct and from `state_hash`, which hashes named fields explicitly). Alternatively, refactor the body so a `#[cfg(test)]` helper returns the visit `Vec`. Pick whichever keeps `state_hash`/serde untouched; do not add a non-test field.

### 7. Determinism / hash notes

S0 is hash-neutral iff the stage: (a) consumes **zero RNG** from any of the three streams (`scenario_rng`/`main_rng`/`mapgen_rng`, hashed in that fixed order at `world_hash.rs:43-47`); (b) mutates **no** entity field feeding `hash_entities` (`:67`); (c) does not touch `logic` order (hashed `:51-56`), `next_stable_entity_id`/`next_occupancy_enter_order` (`:48-49`), or `pending_delete`; (d) does not call any `tick`/`binary_frame`/`total_sim_ms` mutator. The implementation satisfies all four: a read-only walk (`entities.get` + field reads) plus a `#[cfg(debug_assertions)]`-local trace. No RNG handle is even in scope (invariant #7 satisfied trivially — zero draws, so per-object RNG position is unchanged). The `mission_shadow_does_not_change_state_hash` precedent (`world_tests.rs:565`) is the exact proof shape. **No `SNAPSHOT_VERSION` bump** — the serialized/hashed surface is unchanged.

The order-proof `debug_assert` equality holds only because (i) the no-op shell never tail-appends/removes mid-pass and (ii) in `advance_tick` and the hash-neutrality test, every order member is non-dying (corpses flushed at `:1719`). It is the first regression tripwire for any future arm that mutates the live order (invariant #3 guard). See §4 for the dying-member caveat.

### 8. Dependencies + sequencing + risk + do-not-do

**Dependencies / sequencing:** none external — S0 is the foundation slice. It depends only on already-landed substrate (`for_each_live_object` `:947`, `live_object_order_snapshot` `:929`, `LogicVector`, `EntityCategory`). It must land **before** any S1+ "absorb" slice (those fill the four shell arms). No dependency on the in-flight Mission/Radio slices, the RadioBus authority flip, MissionControl consumption, or the commence-gate — S0 neither reads nor writes any of those. (The S1+ slices that absorb combat/mission dispatch will carry the RNG-position and frame-anchored-timer obligations; S0 carries none.)

**Risk:** LOW. Fires every tick (the stage runs in `advance_tick`) but does nothing observable — the no-hash-change test is the gate. The only realistic failure modes are (a) an arm accidentally non-empty, or (b) a stray field mutation; the per-tick order `debug_assert` + the three named hash tests catch both. Frequency-of-fire is every tick, so a regression would be immediately visible in any replay/lockstep test.

**Do NOT:**
- Do **not** create `src/sim/ai/` or touch `src/sim/ai.rs` (file collision; restructuring out of scope — §1).
- Do **not** add `Simulation::category(id)`/`is_active(id)` accessors (aspirational §7.2 sketch; read fields inline).
- Do **not** move, reorder, or duplicate any existing `advance_tick` phase (invariant #3) — only insert the one call before `:2391`.
- Do **not** move the stage **before** `flush_pending_delete` (`:1719`) — doing so changes the corpse window the absent-id guard sees.
- Do **not** use a `match` `_` wildcard arm — keep it exhaustive over the four real variants.
- Do **not** introduce any trait/`dyn`/vtable for dispatch (invariant #2) — `match category` only.
- Do **not** store the visit trace on `Simulation` outside `#[cfg(test)]`/`#[cfg(debug_assertions)]`, serialize it, or hash it.
- Do **not** put any behavior in the arms this slice — every arm is `{}`.
- Do **not** consume RNG or mutate `logic`/counters/`pending_delete`/timers/`binary_frame`.
- Do **not** call `set_logic_order_for_test` *after* setting `dying` in test #4 — it would reset `presence` (though not `dying`); set `dying` last to be safe and explicit.

**Build/test:** `cargo check -p vera20k`, then `cargo test -p vera20k techno_ai` (confirm `-p vera20k`; wrong `-p` exits 101 without running). Read the literal `test result:` line before reporting pass/fail.

**Relevant files (absolute):**
- NEW: `src/sim/world/techno_ai.rs`
- `src/sim/world/mod.rs` (add `mod techno_ai;` at `:13-20`; wire `object_ai_stage()` before `:2391`)
- `src/sim/world/world_hash.rs` (hash boundary `:33-67`; untouched)
- `src/sim/world/world_tests.rs` (precedent test `:565`)
- `src/sim/world/substrate.rs` (`ObjectSubstrate` `:48`)
- `src/sim/entity_store.rs` (`get(u64)` `:96`)
- `src/map/entities.rs` (`EntityCategory` `:19`, `Structure` `:25`)
- `src/sim/game_entity.rs` (`category :181`, `dying :371`, `Presence :144`, `test_default :730`)
- Design doc: `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (§7.1, §9-S0)

---

## Slice S1 — First UnitClass behavior-bearing slice: mission-dispatch-before-locomotor (one scenario, shadow)

> **Review notes — what I corrected in the draft (all re-verified against the live tree this session):**
> 1. **Scope is Move-only, not "Move or Guard."** `derived_mission()` (`game_entity.rs:482`) returns `(MissionType::Move, 0)` for any in-scope ground unit with `movement_target.is_some()` (`:506`), and `(MissionType::None, 0)` for an idle one (`:509`). **There is NO `Guard` derivation path for a ground `UnitClass`** — only aircraft `Idle`/`Guard`/`DockedIdle` map to `MissionType::Guard` (`:489/:492/:495`), and aircraft are excluded from scope. The design doc's "`Mission_Move`/`Mission_Guard`" (§9 S1, :726) is gamemd-verb phrasing; the Rust machine for a moving ground unit only yields `Move`. Scope predicate and `mission` field corrected accordingly.
> 2. **`process_drive_locomotion_shell` is a presence check, not a movement computation.** It returns `Processed` iff `entity.drive_locomotion.is_some()` (`drive_locomotion.rs:27-32`) — it does NOT compute or compare a post-tick position. The draft's Task 4 "compare drive-process disposition to live movement output" overclaimed. Corrected: S1's agreement assert pins the *ordering* and that the shell would process the *same* unit Phase 1 already moved (identity, since there is exactly one Phase-1 mover pass and the shadow recomputes nothing). A read-only per-object position recompute does not exist and is explicitly S2+ work.
> 3. **Line numbers re-verified.** `advance_tick` `mod.rs:1742` ✓; Phase-1 snapshot `:1777` + call `:1778` ✓; `refresh_mission_shadow` def `:895`, call `:2391`, assert `:2393`, `state_hash` call `:2394` ✓; `live_object_order_snapshot` `:929` ✓; `for_each_live_object` `:947` ✓; `debug_assert_mission_shadow_consistent` `:908` ✓; `derived_mission` `:482` ✓; `MissionCom` field `mission` `game_entity.rs:456` ✓; `EntityCategory::Unit` `map/entities.rs:21` ✓; `process_drive_locomotion_shell` `pub(super)` `drive_locomotion.rs:27` ✓; `world_hash.rs` `state_hash` on `Simulation` `:33`, only building-gate `mission_18_active`/`mission_state` folds at `:500/:502` (no `entity.mission`) ✓. The receiver type is **`Simulation`** (`mod.rs:271`), not `World` — no `World` alias exists.
> 4. **Field name is `mission` (type `MissionCom`), accessed `entity.mission.current`** (`game_entity.rs:456`, written by `refresh_mission_shadow` `:898`). Confirmed no `techno_ai`/`ShellTrace`/`unit_ai_shadow` symbols exist anywhere in `src/` — net-new module.
> 5. The three headline test names are verbatim-correct against §9 S1 (:735–737). Supporting test `s1_shadow_preserves_advance_tick_phase_order` mirrors S0's real test name `techno_ai_shell_preserves_advance_tick_phase_order` (:720) ✓.

#### 1. Approach (brainstorm step)

Two ways to land the shadow: **(A)** land a narrow S0-skeleton shell (`techno_ai.rs` with one scoped per-object step) and hang the S1 shadow on it; **(B)** a free-floating inline shadow pass with no shell entry.

**Choice: (A), landing only the S0 skeleton S1 needs — not the full S0 passthrough.** S1's whole meaning ("dispatch precedes Process *inside one object pass*") requires a per-object pass to exist, so a bare inline pass (B) gives the later authoritative flip (S2) nothing to grow into and would be rewritten — the doc explicitly sequences S0→S1→S2 around one `techno_ai.rs` (table at :905, S1 depends on S0). But the *full* S0 passthrough (re-walking logic order, dispatching all four categories) carries the same-pass membership re-read risk the doc defers to the C9 caveat (:713). So this plan lands the **minimum shell**: `techno_ai.rs` + a single read-only `unit_ai_shadow_step` scoped to one moving `UnitClass`. It does **not** adopt the `for_each_live_object` re-read model, does **not** build the four-category match, and moves **no** phase. If a parallel session lands the full S0 first, fold `unit_ai_shadow_step` into it instead of duplicating the module.

#### 2. Goal

For one bounded scenario — a `UnitClass` vehicle with `movement_target.is_some()`, no `miner`/`dock_state`/`attack_target`/`aircraft_mission`, deriving `(MissionType::Move, 0)` — observe the mission decision **then** the locomotor `Process` marker **inside one shell pass**, proving `dispatch_seq < process_seq`. Land it as a **shadow**: read-only, `#[cfg(debug_assertions)]`, never serialized, never hashed. The named test `unit_ai_mission_dispatch_precedes_locomotor_process` pins the ordering.

#### 3. Files / surfaces (live-tree-verified this session)

| Surface | file:line | Role in S1 |
|---|---|---|
| `Simulation::advance_tick` | `src/sim/world/mod.rs:1742` | host; shadow pass invoked at tail near `refresh_mission_shadow` |
| Phase-1 movement snapshot + call | `mod.rs:1777` (`live_object_order_snapshot()`), `:1778` (`tick_movement_with_grids`) | the live ordering S1 compares against — **READ-ONLY this slice** |
| Per-object locomotor advance (the real mutator) | `src/sim/movement/movement_tick.rs:820` (`tick_movement_with_grids` fn) | position-mutating advance; S1 must NOT call it |
| Drive-process *presence* marker | `src/sim/movement/drive_locomotion.rs:27` (`process_drive_locomotion_shell`, `pub(super)`, returns `Processed` iff `drive_locomotion.is_some()`) | read-only "is a drive unit" check reused as the Process-invoked ordinal marker |
| `refresh_mission_shadow` | `mod.rs:895` (def), call `:2391` | refreshes `entity.mission.current`; shadow reads it AFTER this |
| `debug_assert_mission_shadow_consistent` | `mod.rs:908` (`#[cfg(debug_assertions)]`) | assert-discipline pattern to mirror (tick/id in message, no silent equalize) |
| `derived_mission` | `game_entity.rs:482` | Move at `:506`; idle→`(None,0)` at `:509`; aircraft-only Guard at `:489/:492/:495` |
| `mission: MissionCom` field | `game_entity.rs:456` (`#[serde(default)]`) | the shadow mission selector S1 reads; NOT hashed |
| `EntityCategory::Unit` | `src/map/entities.rs:21` | scope discriminant (variant is `Unit`; no `Vehicle`) |
| `live_object_order_snapshot` / `for_each_live_object` | `mod.rs:929` / `:947` | snapshot (no sort) — S1 walks this; re-read model is the deferred C9 caveat |
| `state_hash` | `src/sim/world/world_hash.rs:33` (on `Simulation`) | no `entity.mission` fold (only building-gate `:500/:502`) — shadow leaves hash unmoved |
| **NEW** shell module | `src/sim/world/techno_ai.rs` | does not exist; created this slice (`mod techno_ai;` in `world/mod.rs`) |

Implementer MUST re-Read each line before editing — a parallel session has grown `world/mod.rs` and numbers drift.

#### 4. Step-by-step tasks

**Task 1 — Create `src/sim/world/techno_ai.rs`** (new). `//!` header: purpose (per-object AI shell harness; S1 = mission-dispatch-before-locomotor shadow for one moving-`UnitClass` scenario), deps (reads `Simulation`/`GameEntity::derived_mission`/the read-only drive presence marker; writes NOTHING to live state or hash), invariant note (sim-only; no render/ui/audio/net; no dyn/vtable — dispatch is `match category` + field reads). Declare `mod techno_ai;` near the other `world/` submodule decls in `mod.rs`.

Scope predicate (Move-only — corrected):
```rust
/// The bounded S1 scenario: a moving UnitClass vehicle on a pure Move mission,
/// with no combat, miner, dock, or aircraft concern. `derived_mission()` yields
/// exactly (MissionType::Move, 0) for this set. Anything else returns false.
fn is_s1_scoped_move_unit(e: &GameEntity) -> bool {
    e.category == EntityCategory::Unit
        && e.movement_target.is_some()
        && e.miner.is_none()
        && e.dock_state.is_none()
        && e.attack_target.is_none()
        && e.aircraft_mission.is_none()   // Units never carry it; belt-and-suspenders
}
```

`ShellTrace` (read-only data, never committed):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShellTrace {
    dispatch_seq: u32,    // ordinal where the mission decision was observed this pass
    process_seq: u32,     // ordinal where the locomotor Process marker was observed
    mission: MissionType, // must be MissionType::Move for an in-scope unit
    is_drive: bool,       // entity.drive_locomotion.is_some() (the Process marker's basis)
}
```

**Task 2 — `unit_ai_shadow_step`: decision THEN process, read-only.**
```rust
/// S1 shadow: for one in-scope moving Unit, observe the post-dispatch locomotor
/// ordering inside a single object pass and return the trace. READ-ONLY — takes
/// &Simulation, mutates nothing (no entity, no occupancy, no hash). `seq` is a
/// shared monotonic counter across the pass; dispatch_seq < process_seq by
/// construction proves dispatch precedes Process.
pub(crate) fn unit_ai_shadow_step(sim: &Simulation, id: u64, seq: &mut u32) -> Option<ShellTrace>;
```
Body:
1. `entity = sim.substrate.entities.get(id)?`; return `None` if `!is_s1_scoped_move_unit(entity)`.
2. **Mission dispatch (decision) first.** `let mission = entity.mission.current;` (refreshed by `refresh_mission_shadow` at `:898`, asserted equal to `derived_mission().0` at `:908`). Record `let dispatch_seq = *seq; *seq += 1;`. No mutation — this is the "decision ran" marker.
3. **Locomotor Process second.** `let outcome = drive_locomotion::process_drive_locomotion_shell(entity);` (widen its visibility — see Task 6). Record `let process_seq = *seq; *seq += 1;`.
4. Return `Some(ShellTrace { dispatch_seq, process_seq, mission, is_drive: matches!(outcome, DriveProcessOutcome::Processed) })`.

The ordinal counter makes the ordering proof mechanical: `dispatch_seq < process_seq` for every in-scope unit by construction.

**Task 3 — Wire the shadow pass into `advance_tick` tail (read-only).** In `mod.rs`, AFTER `refresh_mission_shadow()` (`:2391`) and `debug_assert_mission_shadow_consistent()` (`:2393`), BEFORE `state_hash()` (`:2394`), add a `#[cfg(debug_assertions)]` block that: (1) walks `self.live_object_order_snapshot()` (`:929`, no sort — NOT `for_each_live_object`, that re-read model is the deferred C9 caveat, out of S1 scope per :713); (2) maintains a `let mut seq = 0u32;`; (3) for each id calls `techno_ai::unit_ai_shadow_step(self, id, &mut seq)`; (4) for each `Some(trace)` runs the Task-4 asserts. Rationale: tail placement reads settled post-tick state (mission shadow refreshed, Phase-1 movement applied), so the shadow's observation is compared against the live tick's actual output — the §9 S1 contract (:730). It sits before `state_hash()` only for borrow-scope cleanliness; it writes nothing the hash reads.

**Task 4 — Agreement `debug_assert` (mirror `debug_assert_mission_shadow_consistent:908`).** Per in-scope unit:
1. **Ordering invariant (headline proof):** `debug_assert!(trace.dispatch_seq < trace.process_seq, "S1: tick {} unit {}: dispatch_seq {} must precede process_seq {}", self.tick, stable_id, trace.dispatch_seq, trace.process_seq);`
2. **Scope-consistency invariant:** `debug_assert_eq!(trace.mission, MissionType::Move, ...)` (an in-scope unit must derive `Move`) and `debug_assert!(trace.is_drive, ...)` (a moving vehicle in this scope has a drive locomotor). Message names tick + `stable_id`; a divergence is asserted/logged, **never** silently equalized.

**What S1 can and cannot assert (the honest agreement story — corrected):** S1 does **not** recompute movement. `process_drive_locomotion_shell` is a *presence* check (`is_some()`), not a position computation, and no read-only per-object Process exists yet. So `unit_move_dispatch_then_process_shadow_agrees` is satisfied by **identity, not recomputation**: there is exactly one Phase-1 mover pass per tick, and the shell would dispatch-then-process the *same* in-scope unit Phase 1 already moved, to the same final position — because in this bounded scope nothing between Phase 1 (`:1778`) and the tail shadow mutates the mover (no combat, no docking, no second movement pass). The test asserts the shell's would-be ordering imposes no observable change here (the live entity's post-tick position/drive state is the one Phase 1 produced, and the shell observes that same unit). The 1-tick movement-start slip that flipping the ordering authoritative will introduce (S2) is **zero in this steady-state-moving scenario** — S1 exists to confirm the divergence count is zero before S2 expands scope to the first-tick case. Extracting a pure per-object position recompute to do a *real* position comparison is S2+ work; do NOT add it here.

**Task 5 — Tests (see §6).** Co-locate `#[cfg(test)]` in `techno_ai.rs` (Grep for an existing `world/` test module first; prefer co-location).

**Task 6 — Widen the drive-process marker visibility.** `process_drive_locomotion_shell` is `pub(super)` (`drive_locomotion.rs:27`, i.e. visible only inside `movement`). The shadow lives in `world/`, so widen to `pub(crate)` (or add a thin `pub(crate)` re-export). `DriveProcessOutcome` (`:19`) likewise needs `pub(crate)` for `matches!`. This is the only edit to `drive_locomotion.rs` and is behavior-neutral.

**Concrete vs deferred.** Concrete (written above): scope predicate, `ShellTrace`, `unit_ai_shadow_step` sig+body, wiring point, the two asserts, the visibility widen. NOT in S1 (future slices, not speculated): the four-category `match` (S0-full/S2), the `+0xC4` increment (S2), `techno_common_pre/post` (S4), the post-Foot Fire→Facing→Harvest→Ammo→Spawn fold (S3), any phase move, any real per-object position recompute.

#### 5. What is authoritative / shadow / flips later

- **Authoritative (unchanged):** the `Option<T>` machines drive Phase-1 movement (`mod.rs:1778`); global phase order (movement P1 → … → combat P5 → … → AI/cleanup → hash) untouched; `MissionCom.current/substate` re-derived each tick (`refresh_mission_shadow:895`).
- **Shadow (added):** the per-object dispatch-then-process *ordering observation* in `techno_ai::unit_ai_shadow_step`. `#[cfg(debug_assertions)]`-only at the call site, read-only `ShellTrace`, asserts ordering + scope-consistency, never serialized/hashed. Layers on the existing `MissionCom` shadow (`#[serde(default)]`, also unhashed) — same discipline, second layer.
- **Flips later (NOT here):** S2 promotes the ordering authoritative for the scoped `UnitClass` path (routes scoped movement through the shell, adds the `+0xC4` increment, bumps `SNAPSHOT_VERSION`, fresh golden).

#### 6. NAMED acceptance tests (exact fn names)

1. **`unit_ai_mission_dispatch_precedes_locomotor_process`** (§9 S1 :735) — one in-scope moving Unit; advance one tick; call `unit_ai_shadow_step`; assert `trace.dispatch_seq < trace.process_seq`. Headline proof.
2. **`unit_move_dispatch_then_process_shadow_agrees`** (§9 S1 :736) — multi-tick scoped Move scenario: zero `debug_assert` failures across ticks (every in-scope unit derives `Move`, `is_drive`, dispatch<process); plus a deliberately-divergent fixture (e.g. an entity forced into a non-`Move` mission while `movement_target` set) asserted to produce a tick+id-tagged message, **not** a silent equalize.
3. **`s1_no_hash_change_shadow`** (§9 S1 :737) — full-replay golden over a fixed skirmish seed: `state_hash` per tick bit-identical with the shadow pass enabled vs the pre-S1 baseline (shadow not hashed).
4. **`s1_shadow_skips_non_scoped_units`** (supporting) — a miner, a docking unit, an attacking unit, and an aircraft each return `None` from `unit_ai_shadow_step` (scope-guard correctness; prevents over-claiming).
5. **`s1_shadow_preserves_advance_tick_phase_order`** (supporting; mirrors S0's `techno_ai_shell_preserves_advance_tick_phase_order` :720) — assert the phase sequence around the shadow is unchanged (movement P1 → … → `refresh_mission_shadow` → shadow pass → `state_hash`); the shadow inserts only between mission-refresh and hash.

Run: `cargo test -p vera20k techno_ai` (or `cargo test -p vera20k s1_`). Confirm `-p vera20k` (wrong `-p` exits 101 — memory `project_cargo_package_name`); read the literal `test result:` line before reporting (memory `feedback_no_premature_result_reporting`); run verification as a separate bounded pass, not a background job (memory `feedback_cargo_separate_verify_pass`).

#### 7. Determinism / hash notes

- **No hash movement.** `world_hash.rs:33` folds no `entity.mission` and no shell trace (only building-gate `mission_18_active`/`mission_state` at `:500/:502`, unrelated). `ShellTrace` is a local `#[cfg(debug_assertions)]` value. `s1_no_hash_change_shadow` pins bit-identical replay. (Invariant 4: shadow-first.)
- **No RNG consumed.** Shadow reads `entity.mission.current` and `process_drive_locomotion_shell` (which only checks `drive_locomotion.is_some()`); no `scenario_rng`/`main_rng`/`mapgen_rng` draw. RNG position unchanged (invariant 7).
- **No phase reorder** (invariant 3): inserted strictly between `refresh_mission_shadow` (`:2391`) and `state_hash` (`:2394`); `s1_shadow_preserves_advance_tick_phase_order` pins it. Phase-1 movement stays at `:1778`.
- **No death / no slot-free** (invariant 6): shadow never calls `uninit`/`conceal`/`flush_pending_delete`.
- **Frame-anchored timers untouched** (invariant 5): reads no timer, never decrements `MissionTimer`.
- **Iteration order:** walks `live_object_order_snapshot()` (`:929`, no sort) — the same snapshot Phase 1 uses (`:1777`) — NOT the `for_each_live_object` re-read model (deferred §7.2/C9, :713).

#### 8. Dependencies, risk, do-not-do

**Preconditions / sequencing.** S0 is the doc's hard precondition (table :905) and is collapsed into this slice as the *narrow skeleton* (module + single scoped step) per §1. No `techno_ai.rs`/`ShellTrace`/`unit_ai_shadow_step` exists today (Grep: zero hits). If a parallel session lands full S0 first, fold `unit_ai_shadow_step` into it rather than duplicating. `sim.category(id)` does NOT exist — use `entity.category == EntityCategory::Unit` (field read; existing pattern `mod.rs:823,2057`). No standalone per-object locomotor `Process` exists — the mutator is inlined in `tick_movement_with_grids`' movers loop (`movement_tick.rs:820+`); S1 uses the read-only presence marker `process_drive_locomotion_shell` and must NOT call the mutating loop.

**Risk.** Near-zero — shadow + asserts, no behavior/hash change. The only real risk is the shadow accidentally mutating live state (would move the hash, failing `s1_no_hash_change_shadow`) — mitigated by `&Simulation` (not `&mut`) on `unit_ai_shadow_step` and `&GameEntity` on the drive check. The 1-tick movement-start slip lands in S2, not here.

**Do NOT:**
- Move/collapse/reorder any `advance_tick` phase (invariant 3) — Phase-1 movement stays at `:1778`; shadow only observes.
- Fold combat, turret, facing, fire, docking, or the `+0xC4` increment into the shell (S2/S3/S4).
- Add a real per-object position recompute or call the mutating `movers` loop / `uninit` / `flush_pending_delete` from the shadow (S2+).
- Add a trait/`dyn`/vtable (invariant 2) — `match category` + field reads only; this slice touches only the `Unit` arm.
- Hash `MissionCom`, `ShellTrace`, or any shell field; do NOT bump `SNAPSHOT_VERSION` (S2).
- Adopt the `for_each_live_object` same-pass re-read model (deferred §7.2/C9).
- Add a `Guard` scope branch for ground units — `derived_mission` never yields `Guard` for a `UnitClass` (only aircraft do); a moving ground unit is always `Move`.
- Silently equalize a divergence — assert/log with tick+id (mirror `:908`).
- Touch the untracked `src/sim/radio/receive.rs` (RadioBus) — not on the S1 path.

**Files the implementation touches (absolute):** `src/sim/world/techno_ai.rs` (NEW), `src/sim/world/mod.rs` (declare module + wire shadow at tail near `:2391`–`:2394`), `src/sim/movement/drive_locomotion.rs` (widen `process_drive_locomotion_shell` + `DriveProcessOutcome` to `pub(crate)`). Design doc: `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` (§7.3 :700–703, §9 S0 :707–720, §9 S1 :724–737).

---

## Slice FIX — Two Isolated Lifecycle/Parity Fixes

> **Review notes (what I corrected/confirmed in the draft):**
> - **CONFIRMED both headline verdicts against the live tree.** FIX (1): the bypass `self.substrate.entities.remove(id);` is real at `world/mod.rs:1380` inside `remove_wall_entity_at` (`:1365`). FIX (2): `AirfieldDocks` (`docking/aircraft_dock.rs:114`) has **only** `slots` + `aircraft_to_pad` — no `queues`/`VecDeque`/wait-list. The FIFO retire is already done; the eight non-FIFO tests (`:643`–`:758`) exist. The draft is right to close it as already-satisfied.
> - **All draft line numbers re-verified and correct** against the live tree: `uninit` `:1010`, `despawn_entity` `:1050`, `flush_pending_delete` `:1060` (3 late-region drains: `:1719`/`:1770`/`:2254`), `reveal` `:802`, `conceal` `:808`, `refresh_fog` `:1395`, `apply_wall_damage_events` `:1313` invoked at `:2096`, stale doc-comment `:86`. The FACTS-block numbers were stale; the draft's corrections match the live tree.
> - **ADDED a missed test correctness bug:** `GameEntity::test_default` creates `EntityCategory::Unit` (`game_entity.rs:743`), and `new()` defaults `presence=Limbo, in_logic_vector=false` (`:562-563`). The new leak test MUST set `entity.category = EntityCategory::Structure` before `unlimbo`, or `uninit` decrements `owned_unit_count` and the `owned_building_count` assertion is vacuous. Draft omitted this.
> - **CLARIFIED why the fix is safe on the existing raw-insert fixtures:** `uninit`→`conceal`→`unregister_live_object` short-circuits on `!in_logic_vector` (`:782-784`) *before* its `Presence::InCell` debug_assert (`:786`), so a raw-inserted Limbo wall conceals as a no-op (no panic). And `decrement_owned_count` uses `saturating_sub` (`:991`), so the never-incremented raw fixture can't underflow. These are load-bearing for "the modified test stays green."
> - **CONFIRMED `iter_sorted()` returns `(u64, &GameEntity)`** (`entity_store.rs:134`) — the `find_map` yields `id` directly; finder logic unchanged.
> - Everything else in the draft (approach choice, hash/golden-rebaseline call-out, distance-tiebreak DRIFT surfaced not implemented, do-not-do list) is correct and retained.

**Status:** AUTHORED PLAN — read-only research complete this session; no Rust written. Source of truth: live tree (every `file:line` below verified this session) + `TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §8/§10.2. Default verdict on unproven equivalence is DRIFT.

**Headline:**
- **FIX (1) is real and small.** `remove_wall_entity_at` calls `self.substrate.entities.remove(id)` at `world/mod.rs:1380`, bypassing the `uninit` lifecycle chokepoint. For a wall spawned with occupancy+count (a real map/production wall), this leaks owned-building-count, foundation occupancy, and a dangling logic-vector id on every combat wall death. One-call swap + test work.
- **FIX (2) is ALREADY DONE.** The non-native `AirfieldDocks.queues` FIFO the slice asks to retire **does not exist** in the live tree. The struct already holds only `slots` + `aircraft_to_pad`, already implements on-demand re-probe admission (saturated→refuse, first-free scan, no waiter promotion), and already carries the exact "no FIFO / re-probe wins" tests. Actionable residue: one stale doc-comment, plus a separately-scoped distance-tiebreak DRIFT for the user to decide.
- **Path correction:** the live path is `src\sim\docking\aircraft_dock.rs` (not top-level `sim/`).

#### 1. Approach choice

**FIX (1) — route through `uninit`, do not hand-roll a subset.** A wall is `EntityCategory::Structure`, so it participates in owned-building-count, foundation occupancy, the logic vector, radio contacts, and the deferred-death window exactly like every other structure. `uninit` (`:1010`) is the existing teardown chokepoint: it reads the original `dying` once (count-decrement exactly-once even if already mid-death, `:1019-1022`), removes occupancy (`:1023`), clears radio contacts (`:1025`), conceals → leaves the logic vector via the compacting `unregister_live_object` (`:1026`, `:780`), marks `Dying`, and enqueues to `pending_delete` (`:1045`). Hand-picking "just occupancy + count" is the approximation red-flag CLAUDE.md warns about — it re-creates the bypass class the moment a wall grows new lifecycle state. The only cost is the one-tick `Dying` corpse window, which is the identical window every combat death uses.

**FIX (2) — verdict-first.** Verdict: the structural retire requested **does not exist to retire** — it was completed in a prior slice. Scope collapses to: (a) close FIX (2) as already-satisfied (cite the existing tests), (b) fix one stale doc-comment, (c) surface the unmodeled distance-tiebreak as a separately-scoped DRIFT — do **not** write a phantom "remove the FIFO" task against absent code.

#### 2. Goal

1. **FIX (1):** Route wall-entity removal through `uninit` so a destroyed wall's owned-building-count, foundation occupancy, logic-vector membership, and radio contacts are torn down in native order, and the slot is freed via the deferred `pending_delete` flush — eliminating a per-combat-wall-death leak. Pin it with a properly-spawned-wall leak test.
2. **FIX (2):** Confirm and document that the airfield-dock FIFO retire is already complete; correct the one stale `WaitForDock` doc-comment; surface the "distance-then-deterministic" concurrent-waiter tiebreak as a scoped DRIFT decision (do not ship an admission-order change without sign-off — it perturbs the lockstep hash).

#### 3. Files / surfaces (exact file:line, verified live this session)

**FIX (1)**

| Surface | file:line (live) | Role |
|---|---|---|
| `remove_wall_entity_at` (fn) | `src\sim\world\mod.rs:1365` | private `fn(&mut self, rx, ry, rules)`; finder at `:1366-1377` (KEEP) |
| **Bypass call to fix** | `src\sim\world\mod.rs:1380` | `self.substrate.entities.remove(id);` ← swap target |
| Sole caller (loop) | `src\sim\world\mod.rs:1354-1356` | inside `apply_wall_damage_events` (`:1313`) over `destroyed_cells` |
| Mid-tick invocation | `src\sim\world\mod.rs:2096` | combat-results block (gated `overlay_registry.is_some()`) |
| `uninit` (chokepoint) | `src\sim\world\mod.rs:1010` | count↓ (`:1021`, reads orig `dying`) → occupancy-remove (`:1023`) → radio-clear (`:1025`) → conceal (`:1026`) → mark Dying + enqueue (`:1037`/`:1045`) |
| `despawn_entity` (alias→`uninit`) | `src\sim\world\mod.rs:1050` | equivalent route; either acceptable |
| `flush_pending_delete` | `src\sim\world\mod.rs:1060` | frees slots; 3 late-region drains `:1719`/`:1770`/`:2254` |
| `conceal`→`unregister_live_object` | `:808` → `:780` | short-circuits on `!in_logic_vector` (`:782-784`) **before** its `InCell` debug_assert (`:786`) — safe no-op on a Limbo entity |
| `increment/decrement_owned_count` | `:969` / `:983` | Structure→`owned_building_count`; decrement is `saturating_sub` (`:991`) |
| Existing wall test (BREAKS, fix in 1.2a) | `combat\combat_tests.rs:1657` `wall_warhead_damages_and_destroys_wall_overlay` | counts walls via `iter_sorted()` (no `dying` filter), so a `Dying` corpse pre-flush would fail `remaining==0` |
| Raw-insert fixture | `combat\combat_tests.rs:1629` `build_minimal_sim_with_gawall` | raw `entities.insert` (`:1651`), `EntityCategory::Unit`, `presence=Limbo` — no reveal/occupancy/count |
| Row fixture (used by chain test) | `combat\combat_tests.rs:1711` | raw insert (`:1733`); chain test `:1743` asserts overlay state (unaffected) |
| Determinism test (UNAFFECTED) | `combat\combat_tests.rs:1794` `wall_damage_deterministic_across_replays` | reads overlay grid, not entity count |
| Active spawn path (for new fixture) | `world\world_spawn.rs:554` `unlimbo` → `place_spawned(active=true)` (`:527`) | `insert → reveal → increment_owned_count → add_entity_occupancy` (`:531-535`) |
| `iter_sorted` shape | `entity_store.rs:134` | `impl Iterator<Item=(u64, &GameEntity)>` — `find_map` yields `id` directly |
| `entity_store::remove` | `entity_store.rs:69` | returns `Option<GameEntity>`, maintains owner index |

**FIX (2)**

| Surface | file:line (live) | Role |
|---|---|---|
| `AirfieldDocks` struct | `docking\aircraft_dock.rs:114-120` | `slots` + `aircraft_to_pad` only — **no `queues`** |
| `try_reserve` (re-probe admission) | `:139` | idempotent check `:148-152`; first-free scan `:155-163`; saturated→`None` `:166` |
| `release` / `cancel` / `cleanup_dead` | `:172` / `:198` / `:209` | empties slot, no waiter promotion |
| **Stale doc-comment to fix** | `:86` | `WaitForDock`: `"waiting for a dock slot (FIFO queue)"` — contradicts the correct struct docstring (`:107-112`, "no wait queue… NOT a FIFO and NOT a distance sort") |
| Existing non-FIFO tests | `:643` `airfield_docks_basic_reserve`; `:653` `airfield_release_does_not_pin_freed_pad_index`; `:672` `airfield_full_waiter_admitted_by_probe_not_fifo`; `:689` `airfield_docks_cancel`; `:703` `airfield_docks_cleanup_dead`; `:718` `airfield_docks_idempotent_reserve`; `:725` `airfield_docks_four_pad_allocation_order`; `:737` `airfield_docks_single_pad_helipad`; `:748` `airfield_docks_pad_assignment_is_deterministic` | already pin the contract the slice asked to add |
| Snapshot owner of dock state | `sim.production.airfield_docks` (hashed via `world_hash`) | untouched by this slice |

#### 4. Step-by-step tasks

**FIX (1) — Task 1.1: route wall removal through `uninit`.** File `world\mod.rs`, fn `remove_wall_entity_at` (`:1365`). Keep the finder (`:1366-1377`). Change only the removal at `:1379-1383`:

```rust
if let Some(id) = to_remove {
    // Route through the lifecycle chokepoint, not a raw store remove: a wall is
    // an EntityCategory::Structure, so it owns owned-building-count, foundation
    // occupancy, logic-vector membership, and any radio contacts. uninit tears all
    // of those down in native order, marks the entity Dying, and enqueues the slot
    // for the end-of-tick pending_delete flush (the same deferred-death window every
    // combat death uses). A direct entities.remove leaks count/occupancy and leaves a
    // dangling id in the active order.
    self.uninit(id);
} else {
    log::warn!("apply_wall_damage_events: no wall entity at ({rx}, {ry})");
}
```

Lifecycle/timing note (assert in test): wall removal runs in the Phase-5 combat-results block at `:2096`. Vision (`refresh_fog` `:1395`, called `:1967`) and power (`tick_power_states` `:1973`) are **Phase 3 / Phase 4 — they already ran earlier this tick and observed the wall ALIVE (pre-removal)**, so there is NO "Dying wall survives vision/power" window — do not assert one. After removal the `Dying` corpse persists only through the remainder of Phase 5 and is freed at the **Phase-5 `flush_pending_delete` (`:2254`)** — the first drain after the `:2096` removal. This matches the deferred-death contract (invariant #6); tick phase order is unchanged (invariant #3) — removal stays at its `:2096` site.

**FIX (1) — Task 1.2: fix the breaking test + add a real-spawn leak test.** File `combat\combat_tests.rs`.

**(a) Fix `wall_warhead_damages_and_destroys_wall_overlay` (`:1657`).** After `uninit`, the wall is `Presence::Dying` but stays in the store until `flush_pending_delete`; the `remaining==0` count at `:1694-1706` uses `iter_sorted()` (no `dying` filter), so it would see the corpse and fail. Preferred fix: call `sim.flush_pending_delete();` immediately after `apply_wall_damage_events` (`:1681`), keep the `remaining==0` count — this proves the slot is actually freed. (This stays green even though the fixture is raw-inserted/Limbo: `uninit`→conceal short-circuits on `!in_logic_vector` before its `InCell` assert, and `decrement_owned_count` is `saturating_sub`, so no panic/underflow.)

**(b) Add a properly-spawned-wall leak test** `wall_destruction_routes_through_uninit_no_leak`:
1. Build a sim with a 10×10 overlay grid + `wall_test_ini()` (`:1615`, carries `[GAWALL] Strength=400 Armor=concrete Wall=yes`).
2. Place the GAWALL overlay at (5,5). Build the wall `GameEntity` via `test_default`, then **set `entity.category = EntityCategory::Structure`** (test_default makes it `Unit` — required for the `owned_building_count` assertion), set `owner`/`type_ref` through `sim.interner`, and spawn via `sim.unlimbo(entity)` (NOT raw insert) so it gets reveal + occupancy + an incremented `owned_building_count`. Record baseline `owned_building_count`.
3. Assert pre-destruction: wall id in `sim.substrate.logic.as_slice()`; occupancy at (5,5) contains the id; `owned_building_count == baseline+1`; `presence == InCell`.
4. Forced destroy: `sim.apply_wall_damage_events(&[WallDamageEvent{rx:5,ry:5,damage:u16::MAX}], &rules, &registry)`.
5. Assert post-`uninit`/pre-flush: wall id NOT in `logic.as_slice()`; occupancy at (5,5) no longer contains the id; `owned_building_count == baseline` (decremented exactly once); entity still resolvable as `Presence::Dying`.
6. `sim.flush_pending_delete();` then assert `sim.substrate.entities.get(id).is_none()` (slot freed).

This is the **named acceptance test** pinning the leak fix: it fails on the current direct-`remove` code (count/occupancy/logic-membership all leak) and passes after the swap.

**FIX (2) — Task 2.1: correct the stale doc-comment.** File `docking\aircraft_dock.rs:86`. Change `/// At/near the airfield, waiting for a dock slot (FIFO queue).` to `/// At/near the airfield, re-probing each tick for a free dock slot (no wait queue — see [AirfieldDocks]).` Comment-only, hash-neutral, no test.

**FIX (2) — Task 2.2: close the retire + surface the distance-tiebreak DRIFT (no code).**
- **Close "retire FIFO":** struct has no `queues`/`VecDeque`/wait-list; `try_reserve` (`:139`) already does saturated-refuse + first-free re-probe; `release`/`cancel`/`cleanup_dead` already empty-without-promotion. The nine existing tests (`:643`–`:758`) already pin "no FIFO, re-probe wins, no freed-pad pinning, deterministic" — these ARE the coverage the slice asked to add. Mark the design doc (lines 37/197/938) **DONE**.
- **Surface the residual DRIFT (user decides — do NOT auto-implement):** design §10.2 specifies that among multiple same-owner aircraft simultaneously in `WaitForDock` for one saturated airfield when a pad frees the same tick, the winner should be **distance-then-deterministic**. The live admission winner is instead **iteration-order (BTreeMap id-ascending) deterministic** — the FSM snapshot is built from `entities.values()` (id-ascending) and `try_reserve` is a pure first-free pad-index scan with no distance input. Per CLAUDE.md burden-of-proof this is **DRIFT**, not internal-only. It is a separate optional follow-up because: (1) modeling distance-then-deterministic admission **changes the lockstep state hash** (admission order → pad assignment → per-pad descent cells) → needs a `SNAPSHOT_VERSION` bump + rebaselined golden + a named acceptance test; (2) the distance metric (lepton vs cell) and post-distance tiebreak key are sourced from §10.2 prose, **not a fresh Ghidra trace this session** — exactly the direction-bug-prone arithmetic CLAUDE.md flags, so verify the gamemd winner rule in the binary before implementing. If approved it becomes its own slice. Out of scope here.

#### 5. What becomes authoritative / shadow

- **FIX (1):** No shadow phase — this is lifecycle-correctness, not a new authority. Wall removal already *was* authoritative; the fix changes the *path* (route through `uninit`) so teardown is complete. Post-fix state is *more* correct (count/occupancy no longer leak), so the replay hash for a scenario that destroys a real (occupancy+count) wall **changes** — see §7.
- **FIX (2):** Nothing changes authority. The doc-comment edit is cosmetic. The distance-tiebreak, if ever adopted, is a separate authoritative change with its own shadow-first treatment.

#### 6. Named acceptance tests

**FIX (1):**
- `wall_destruction_routes_through_uninit_no_leak` (NEW, 1.2b) — spawned-via-`unlimbo`, category `Structure`; asserts logic-membership leave, occupancy clear, `owned_building_count` decrement-once, `Dying` corpse pre-flush, slot-free post-flush. Fails on current code, passes after the swap.
- `wall_warhead_damages_and_destroys_wall_overlay` (`combat_tests.rs:1657`, MODIFIED, 1.2a) — add `flush_pending_delete()` before the `remaining==0` count.
- Keep-green (unchanged): `concrete_wall_chain_reaction_runs_without_panic` (`:1743`, overlay-state assert), `wall_damage_deterministic_across_replays` (`:1794`, overlay-grid replay).

**FIX (2):** No new test required — coverage already exists at `aircraft_dock.rs:643/653/672/689/703/718/725/737/748`. IF the distance-tiebreak is later approved, it needs a NEW test e.g. `airfield_concurrent_waiters_admitted_by_distance_then_deterministic` — explicitly NOT part of this slice.

**Build/test (separate bounded pass, not background):** `cargo check -p vera20k`; `cargo test -p vera20k wall` (FIX 1); `cargo test -p vera20k airfield` (FIX 2 keep-green). Confirm `-p vera20k` (wrong `-p` exits 101 without running). Read the literal `test result:` line before reporting.

#### 7. Determinism / hash notes

- **FIX (1) IS hash-affecting for real walls.** Routing through `uninit` decrements `owned_building_count` and clears occupancy the direct `remove` leaked. For any replay/golden in which a combat-destroyed wall was spawned with occupancy+count (a production/map wall, not a raw-insert test wall), the corrected state changes — count is now lower, occupancy clear, no dangling logic id. **Requires a `SNAPSHOT_VERSION` bump + rebaselined golden** if any committed golden covers wall destruction. This is the correct gamemd-direction (a destroyed wall is no longer counted/occupying); do NOT "fix" a failing golden by reverting the behavior. The one-tick `Dying` corpse uses the same deferred-death window every combat death uses (invariant #6): created at the `:2096` removal, freed at the Phase-5 `flush_pending_delete` (`:2254`). Vision (`:1967`) and power (`:1973`) are Phase 3/4 and run *before* `:2096`, so they observe the wall alive — never the corpse; no consumer sees the Dying wall.
- **Raw-insert test fixtures are NOT a hash concern** — they never had count/occupancy/logic membership; the only delta is the `Dying`-corpse-until-flush window (handled by 1.2a's flush).
- **FIX (2) is hash-neutral as written** (comment-only). The distance-tiebreak, if later adopted, **is** hash-affecting (admission order → pad assignment → descent cells) and carries its own `SNAPSHOT_VERSION` bump + golden rebaseline + Ghidra-verified winner rule.
- **RNG (invariant #7 holds):** neither fix consumes or repositions RNG. `uninit` draws none; wall-damage RNG is consumed in `damage_wall_overlay` (`:1337`, `scenario_rng`) *before* `remove_wall_entity_at` and is unchanged.
- **Iteration order (invariant #3 holds):** finder still uses `iter_sorted()` (`:1366`); `uninit`'s conceal uses the compacting `LogicVector::remove`, same as every other death. Wall removal stays at its `:2096` mid-tick site.

#### 8. Dependencies + risk / do-not-do

**Dependencies:** none on the in-flight mission/radio slices. FIX (1) depends only on the landed `uninit`/`flush_pending_delete` chokepoint. FIX (2) depends on nothing (retire already landed). Both independent; either order / separate commits.

**Risk:**
- FIX (1): LOW mechanism (single-call swap), **MEDIUM golden-rebaseline** — if a committed replay golden covers combat wall destruction the corrected count/occupancy moves the hash; rebaseline (don't revert). Trigger frequency: every match where a unit shoots through a wall — common, so the leak is not a rare edge case.
- FIX (2): NONE for the comment edit. Distance-tiebreak, if pursued: MEDIUM (hash rebaseline + needs Ghidra verification); trigger frequency LOW-but-real (≥2 concurrent same-owner waiters on one saturated airfield, pad frees same tick).

**Do-NOT-do:**
- Do **not** hand-roll a "walls only need occupancy + count" subset in place of `uninit` (approximation red-flag — re-creates the bypass class).
- Do **not** write a "remove the AirfieldDocks FIFO" task — the field does not exist; that's the stale-FACTS trap.
- Do **not** silently implement the distance-then-deterministic admission tiebreak — it changes the lockstep hash and is sourced from doc prose, not a fresh binary trace. Surface for the user's decision; if approved, verify the winner rule in Ghidra first, give it its own `SNAPSHOT_VERSION` bump + named test.
- Do **not** re-introduce any wait-list/queue when touching `aircraft_dock.rs` — the no-queue re-probe model is the verified contract (design §10.2).
- Do **not** move wall removal off its `:2096` mid-tick site or change tick phase order (invariant #3).
- Do **not** forget `entity.category = EntityCategory::Structure` in the new test fixture — `test_default` defaults to `Unit`, which would silently make the `owned_building_count` assertion vacuous.

---

## Acceptance test index

Every named test in this plan, in dependency order. Run with `cargo test -p vera20k <filter>` (confirm `-p vera20k`; wrong `-p` exits 101 without running; read the literal `test result:` line before reporting).

**S0 — `src/sim/world/techno_ai.rs` (`#[cfg(test)]`)**
1. `techno_ai_shell_is_passthrough_no_hash_change` — stage is hash-neutral (mirrors `mission_shadow_does_not_change_state_hash`).
2. `techno_ai_shell_membership_matches_phase_snapshot` — every live object visited once, in live order.
3. `techno_ai_shell_preserves_advance_tick_phase_order` — multi-tick `state_hash` sequence bit-identical to a pinned golden.
4. `object_ai_stage_skips_dying_object` — dying id not visited; live id is; no panic (set `dying` AFTER `set_logic_order_for_test`).
5. `object_ai_stage_tolerates_absent_id_in_order` — absent id in order skipped without panic; live id still visited.

**S1 — `src/sim/world/techno_ai.rs` (`#[cfg(test)]`)**
6. `unit_ai_mission_dispatch_precedes_locomotor_process` — headline: `dispatch_seq < process_seq`.
7. `unit_move_dispatch_then_process_shadow_agrees` — zero divergences over a multi-tick scoped Move scenario; deliberate-divergence fixture produces a tick+id-tagged message (no silent equalize).
8. `s1_no_hash_change_shadow` — per-tick `state_hash` bit-identical with shadow enabled vs pre-S1 baseline.
9. `s1_shadow_skips_non_scoped_units` — miner / docking / attacking / aircraft each return `None`.
10. `s1_shadow_preserves_advance_tick_phase_order` — phase sequence unchanged around the shadow.

**FIX — `src/sim/combat/combat_tests.rs`**
11. `wall_destruction_routes_through_uninit_no_leak` (NEW) — logic-membership leave, occupancy clear, `owned_building_count` decrement-once, `Dying` corpse pre-flush, slot-free post-flush. Fails pre-fix, passes post-fix.
12. `wall_warhead_damages_and_destroys_wall_overlay` (MODIFIED) — add `flush_pending_delete()` before the `remaining==0` count.
13. `concrete_wall_chain_reaction_runs_without_panic` (KEEP-GREEN) — overlay-state assert, unaffected.
14. `wall_damage_deterministic_across_replays` (KEEP-GREEN) — overlay-grid replay, unaffected.

**FIX (2) — `src/sim/docking/aircraft_dock.rs` (already present; keep-green)**
15. `airfield_docks_basic_reserve`, `airfield_release_does_not_pin_freed_pad_index`, `airfield_full_waiter_admitted_by_probe_not_fifo`, `airfield_docks_cancel`, `airfield_docks_cleanup_dead`, `airfield_docks_idempotent_reserve`, `airfield_docks_four_pad_allocation_order`, `airfield_docks_single_pad_helipad`, `airfield_docks_pad_assignment_is_deterministic`.
16. (DEFERRED, NOT in this plan) `airfield_concurrent_waiters_admitted_by_distance_then_deterministic` — only if the distance-tiebreak DRIFT is later approved (own slice, own `SNAPSHOT_VERSION` bump, Ghidra-verified winner rule).

---

## Cross-cutting invariants recap

The eight hard invariants every slice in this plan respects:

1. **sim/ purity** — `sim/` never depends on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`. The new `world/techno_ai.rs` imports only `Simulation` + sim types; no upward dependency.
2. **No C++ class tree / no dyn / no vtable / no COM** — dispatch is `match category` (S0's four-arm shell, S1's `Unit` arm) + CapabilityFlags/field reads + `Option<T>`. No trait object anywhere.
3. **advance_tick phase order PRESERVED** — until a slice explicitly changes it (none here). S0 inserts one call before `:2391`; S1 inserts a `#[cfg(debug_assertions)]` observation block between `:2393` and `:2394`; FIX (1) leaves wall removal at its `:2096` mid-tick site. Phase-1 movement stays at `:1778`. The `*_preserves_advance_tick_phase_order` tests pin it.
4. **Shadow-first** — new authority lands shadowed (serde-skip, not hashed, debug_assert agreement) before the authority flips. S0's visit trace is `#[cfg(debug_assertions)]`-local; S1's `ShellTrace` + asserts are `#[cfg(debug_assertions)]`-only and unhashed. No `SNAPSHOT_VERSION` bump in this plan. The first authority flip is S2 (later plan).
5. **Frame-anchored timers never decrement** — `MissionTimer` (start_frame + duration) is read, never decremented. No slice here touches a timer.
6. **Deferred death** — enqueue `pending_delete`, synchronous conceal/unmark/detach, deferred slot-free. FIX (1) routes wall removal through `uninit` (synchronous conceal/unmark/count↓/occupancy-remove + enqueue) → late-region `flush_pending_delete` frees the slot; the one-tick `Dying` corpse window matches every other combat death. S0/S1 never call death paths.
7. **RNG consumed at the same per-object position/gate** — S0 and S1 consume ZERO RNG (read-only walks; no handle in scope). FIX (1)/(2) consume/reposition no RNG (`uninit` draws none; wall-damage RNG is drawn in `damage_wall_overlay` *before* removal, unchanged).
8. **Every behavior-moving slice has a NAMED acceptance test pinning gamemd order before it flips** — S0: the three hash/order tests. S1: `unit_ai_mission_dispatch_precedes_locomotor_process` pins dispatch-before-process. FIX (1): `wall_destruction_routes_through_uninit_no_leak` pins the lifecycle teardown (fails pre-fix). The FIX-(2) distance-tiebreak, being deferred, carries its named test (`airfield_concurrent_waiters_admitted_by_distance_then_deterministic`) into its own future slice, NOT this plan.
