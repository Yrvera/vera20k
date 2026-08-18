# ObjectSubstrate Slice 6 — Deferred-Delete Queue + Dying Window (Fork B) Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. This is the **Fork B,
> evidence-driven** plan: unify death through one deferred-delete queue, keep the animated
> lingering window, give immediate (structure/voxel) deaths gamemd's end-of-tick window, and
> add the gating fixes that keep whole-store scans gamemd-faithful. The `SNAPSHOT_VERSION`
> bump is **conditional on the empirical hash result** (Task 12), not assumed.

**Goal:** Make every death two-phase — synchronous detach/conceal/unmark/`dying`, then a
deferred slot-free drained at end-of-tick — so a just-killed structure stays resolvable-but-
`Dying` through the rest of the tick (matching gamemd `UnInit`→`ProcessPendingDelete`), without
changing the animated-death corpse window the port already has.

**Architecture:** Add a transient `pending_delete: Vec<u64>` to `ObjectSubstrate`; `uninit`
enqueues instead of removing; `flush_pending_delete()` drains it at two points (end of Phase 9
inside `advance_tick`; in the app layer after the anim-end despawn loop). Whole-store
`entities.values()` scans in Phases 5.5→8.5 that lacked a `dying` gate (AI, production
placement) get one so they exclude the dying structure — matching gamemd's live-vector
iteration. `sim/` gains no dependency on render/ui/audio/net.

**Design Doc:** [docs/plans/2026-06-01-slice6-deferred-delete-design.md](docs/plans/2026-06-01-slice6-deferred-delete-design.md)

---

## Grounding Summary

- **Docs:** `SLICE6_DEFERRED_DELETE_DYING_WINDOW_GHIDRA_REPORT.md` (HIGH, this session) verifies
  gamemd's two-phase death: `ObjectClass::UnInit` (`0x005F65F0`) synchronously detaches →
  Limbo/Conceal (occupancy-unmark + deselect) → `IsAlive=0` → appends to PendingDeleteList;
  `ProcessPendingDelete` (`0x00725C70`) drains at end of `Main_Tick` (`0x0055DE9F`); drain
  predicate `vtable+0x44` = `ObjectClass::IsDead` (`IsAlive==0`, always true post-UnInit) ⇒ free
  same tick. 1:1 cross-ref fields are **gated-at-use**, not nulled (§8 RESOLVED). Mind-control
  teardown + transport-pax despawn confirmed **out of scope** for the port (no mapping / known gap).
- **Ghidra re-verification:** not required this slice — the gamemd contract is fully captured in
  the evidence artifact; the work is port-side Rust plumbing.
- **Repo reality (verified this session):** the port has **two** death-exit paths — animated
  (infantry/SHP) linger as their own corpse via `dying=true`, despawned in the **app layer**
  (`app_sim_tick.rs:300-308`) after `advance_tick`; immediate (structure/voxel) are `uninit`'d
  in **Phase 5** (`world/mod.rs:1963-1965`) same-tick. `uninit` (`world/mod.rs:939-969`) already
  does counts→occupancy-unmark→radio-clear→conceal→`presence=Dying` synchronously; only the
  final `store.remove` is what defers under this slice.
- **Pattern mirrored:** the substrate-owns-transient-state pattern (occupancy is `#[serde(skip)]`
  on `ObjectSubstrate`) and the Slice 2 `Presence`/`dying` shadow.
- **Gating audit (done):** dock/miner code already gates lingering entities
  (`aircraft_dock.rs:297` `health==0||dying`; `miner_system.rs:1114` same; `building_dock.rs:81`
  alive-set `!dying`; `tick_retaliation` `combat_targeting.rs:351` `health>0` + clears
  `last_attacker_id` regardless at 400). **Ungated** structure/unit scans that Fork B would make
  wrongly count a dying structure: `ai.rs:152/190/426/486/512/1010`, `production_placement.rs:427/462`.
- **INI keys:** none — engine lifecycle plumbing.
- **Still unknown (→ Task 12):** whether the absolute state hash actually changes. After the
  gating fixes every consumer treats a dying structure identically to an absent one, so the
  expected outcome is **no hash change**; confirmed empirically by a before/after capture.

## Key Technical Decisions

- **One queue, two drain points** (Phase 9 in `advance_tick`; app layer after anim-end despawn).
  — gamemd has one end-of-`Main_Tick` drain, but the port's animated-death despawn lives in the
  app layer after the hash, so a single in-`advance_tick` drain would add a 1-tick linger to
  corpses. Two drains preserve the animated-path timing exactly. **Confidence:** high. **Source:**
  `app_sim_tick.rs:293-308` + `world/mod.rs:1539-1634` (verified).
- **Flush placed before the Phase 9 `OCCUPANCY_DEBUG` rebuild** (`world/mod.rs:1618-1622`) and
  before the tail asserts (`2259-2261`) + `state_hash` (`2262`). — `OccupancyGrid::rebuild`
  (`occupancy.rs:118-142`) iterates `entities.values()` with no `dying` filter, so an unflushed
  dying structure would be re-added and trip `debug_assert_matches`. **Confidence:** high.
  **Source:** `occupancy.rs:118-142` (verified).
- **Gating fixes EXCLUDE the dying structure** (`!dying`) from whole-store scans — gamemd's
  per-tick systems iterate the live vector, which the concealed dead object is already off.
  Excluding it keeps behavior identical to today's immediate-removal (the inert outcome).
  **Confidence:** high. **Source:** SLICE6 report §3.1 + repo gate pattern (`miner_system.rs:1114`).
- **No proactive 1:1 cross-ref nulling; radio clear only** (already in `uninit`). — RE proved
  gate-at-use, not field-null (§8 RESOLVED). **Confidence:** high.
- **`SNAPSHOT_VERSION` bump is conditional** (Task 12). — `pending_delete` is `#[serde(skip)]`
  (no layout change); determinism/saveload tests are A-vs-B (no hardcoded golden in `sim/`); the
  gating fixes keep behavior inert. Bump 16→17 + pin a golden ONLY if a real hash change appears.
  **Confidence:** medium (empirical) — **flagged for /review-plan and Task 12 evidence.**
- **AI gating fixes are defensive invariant-preservation, not AI feature work** — they preserve
  the pre-slice "present structure = alive" assumption under the new deferred-removal invariant.
  Tension with `feedback_no_ai_yet` noted; the alternative (leave ungated) ships a real
  skirmish regression (AI counting dead structures). **Confidence:** high.

## Open Questions

### Resolved During Planning
- *Does `uninit` free immediately?* No for animated deaths (linger via anim, app-layer despawn);
  yes for immediate (structure/voxel) at Phase 5. Resolved by reading `combat/mod.rs:986-1020`,
  `app_sim_tick.rs:300-308`, `world/mod.rs:1963`.
- *Which consumers break the "present = alive" assumption under deferral?* AI scans + production
  placement scans (enumerated above); docks/miner/retaliation already gate.
- *Where must the in-tick drain go?* End of Phase 9, before the `OCCUPANCY_DEBUG` block and the
  tail asserts/hash.
- *Do any `uninit` callers spawn a replacement at the same cell?* Yes (MCV→ConYard
  `world_spawn.rs:702`, SMIN↔YAREFN `slave_miner.rs:473/555`) — all run **inside a tick** (command
  application), so the dying entity is flushed at Phase 9 of the same `advance_tick`, before
  render. No ghost. (Verify no setup-time caller — Task 4 step.)

### Deferred to Implementation
- **Does the absolute state hash change?** Determined empirically in Task 12 (before/after
  capture). Expected: no change. If it changes, diagnose whether a gating site was missed or it
  is a legitimate gamemd-faithful exposure, then decide the version bump.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/world/substrate.rs:48-83` | Add `pending_delete: Vec<u64>` (`#[serde(skip)]`) + init |
| Modify | `src/sim/world/mod.rs:939-969` | `uninit`: enqueue + `dying=true` instead of `remove()` |
| Modify | `src/sim/world/mod.rs` (new method) | `flush_pending_delete()` |
| Modify | `src/sim/world/mod.rs:1588-1622` | Phase 9 drain placement |
| Modify | `src/sim/world/mod.rs:833-838` | Presence-assert comment |
| Modify | `src/sim/game_entity.rs:130-133` | `Presence::Dying` doc |
| Modify | `src/app_sim_tick.rs:300-308` | App-layer drain after anim-end despawn |
| Modify | `src/sim/ai.rs` (6 sites) | Defensive `!dying` gates on structure/unit scans |
| Modify | `src/sim/production/production_placement.rs:427,462` | `!dying` gates on placement scans |
| Modify | `src/sim/snapshot.rs:694-712`, `src/sim/world/world_tests.rs:147` | Update tests to two-phase semantics |
| Modify | `src/sim/world/mod.rs` (tests) | New Dying-window behavior tests |

## Interface Changes

- **`ObjectSubstrate.pending_delete: Vec<u64>`** — new `pub(crate)` field, `#[serde(skip)]`.
  Empty at every tick/save boundary. No serialized-layout change.
- **`Simulation::flush_pending_delete(&mut self)`** — new `pub(crate)` method. Called from
  `run_late_region` (sim) and `app_sim_tick.rs` (app). No external dependents beyond these two.
- **Behavior contract:** after `uninit(id)` and until the next drain, `store.get(id)` returns
  `Some` (a `Dying` entity), not `None`. Same-window consumers MUST gate on `dying`. Documented
  on `uninit` and `flush_pending_delete`.

## Sim Checklist

- [x] No new floating-point math (id bookkeeping only — `Vec<u64>`).
- [x] New state (`pending_delete`) **excluded** from the hash and snapshot (`#[serde(skip)]`,
      empty at every boundary) — intentional; verified by Task 11/12.
- [x] No `sim/` dependency on render/ui/sidebar/audio/net (the app-layer drain lives in
      `app_sim_tick.rs`, which already calls `sim` methods).
- [x] Tick-ordering impact: drain added at end of Phase 9, before asserts/hash. Documented.
- [x] `BTreeMap` iteration order: drain order = `Vec` insertion (death) order; deterministic.

## Risk Areas

- **Phase 9 drain ordering** — must precede `OCCUPANCY_DEBUG` rebuild and tail asserts/hash, else
  debug builds panic. Regression: existing `lifecycle_keeps_membership_invariant`,
  `debug_assert_presence_consistent`, `OCCUPANCY_DEBUG` runs.
- **Ungated whole-store scans** — any missed P5.5→P8.5 scan that counts a dying structure is
  drift (and may surface as a hash change in Task 12). Mitigation: the enumerated AI/production
  fixes + the Task 12 before/after capture as a backstop detector.
- **Same-cell respawn callers** (MCV/SMIN) — verify the lingering dying entity doesn't collide
  with the freshly-spawned replacement (new id, occupancy unmarked → expected safe; add a test).
- **Existing uninit/despawn tests** — three assert immediate store-absence; they break under
  deferral and must be updated to the two-phase semantics.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| Task 4 | In-tick drain at end of Phase 9, before hash | gamemd frees at end-of-`Main_Tick` after the object pass; a structure must be resolvable-but-`Dying` through P5.5–P8.5 then gone at hash | SLICE6 report §3.5 (`Main_Tick 0x0055DE9F`); store clean at `state_hash` |
| Task 5 | App-layer drain timing | Animated corpse must free at exactly the same frame as today (no extra tick of linger / no 1-frame ghost) | Existing animation/despawn tests green; corpse-frees-at-anim-end test |
| Task 7/8 | Whole-store scans exclude the dying structure | gamemd per-tick systems iterate the live vector (concealed dead object excluded); counting a dead structure is drift | Gate matches `miner_system.rs:1114` pattern; Task 11 cross-ref test |
| Task 3 | `uninit` order unchanged except enqueue+`dying` | Detach/conceal/unmark/`IsAlive=0` order is observable in gamemd | SLICE6 §3.1; count-decrement still reads original `dying` |
| Task 11 | Mutual same-tick death determinism | Two structures killing each other both resolve as `Dying`; replay-stable | Critic #9; A-vs-B hash equal across two seeded runs |

---

## Tasks

### Task 0 (pre-step): Capture the baseline hash on current `dev`

**Why:** Establishes H0 so Task 12 can tell empirically whether the slice changes the hash.

**Files:** none (throwaway measurement).

**Step 1:** On the current clean `dev`, add a temporary test in `src/sim/world/world_tests.rs`
that builds a small deterministic scenario killing a structure and prints the post-tick hash:
```rust
#[test]
fn slice6_baseline_hash_capture() {
    // Build a minimal sim: place a structure, apply lethal damage so it dies
    // this tick via the immediate (structure) path, advance one tick, print hash.
    // (Mirror an existing structure-death test setup in this file.)
    let mut sim = Simulation::new();
    sim.reseed_both(0x5117_6006);
    // ... place a structure + an attacker, drive one advance_tick that kills it ...
    let h = sim.state_hash();
    println!("SLICE6_BASELINE_HASH={h:#018x}");
    panic!("baseline capture only");
}
```
**Step 2:** Run `cargo test -p vera20k --lib slice6_baseline_hash_capture -- --nocapture`,
record the printed `SLICE6_BASELINE_HASH=` value in the plan/PR notes, then **delete the
temporary test**. Do not commit it.

**Step 3:** Note H0. (If building a faithful structure-death scenario here is non-trivial, use
the simplest existing structure-death test in `world_tests.rs`/`combat_tests.rs` as the basis
and capture its `state_hash()` instead — the requirement is a stable, reproducible H0.)

> If a clean H0 cannot be captured cheaply, skip Task 0 and rely on Task 12's determinism +
> saveload + targeted-behavior suite as the evidence (all A-vs-B; a missed gate would surface as
> a determinism failure or a wrong value in the new behavior test). Note the skip explicitly.

### Task 1: Add `pending_delete` to `ObjectSubstrate`

**Why:** The transient queue is the foundation everything else builds on; define it first.

**Files:** Modify `src/sim/world/substrate.rs:48-83`

**Pattern:** Mirrors the existing `#[serde(skip)] occupancy` field on the same struct.

**Step 1: Add the field** to `ObjectSubstrate` (after `entities`):
```rust
    /// Plain-struct entity storage ...
    pub(crate) entities: EntityStore,
    /// Deferred-delete queue (gamemd `PendingDeleteList`). `uninit` pushes an id
    /// here instead of freeing the store slot; `Simulation::flush_pending_delete`
    /// drains it in death order at end-of-tick. Transient: empty at every tick/save
    /// boundary, so it is `#[serde(skip)]` — not serialized, not hashed. Between
    /// enqueue and drain the entity stays in the store as a `Dying`, off-occupancy,
    /// off-logic corpse, resolvable by id (the gamemd two-phase death window).
    #[serde(skip)]
    pub(crate) pending_delete: Vec<u64>,
```

**Step 2: Initialize** in `ObjectSubstrate::new()`:
```rust
        Self {
            next_stable_entity_id: 1,
            next_occupancy_enter_order: EnterOrderCounter::new(),
            logic: LogicVector::new(),
            occupancy: OccupancyGrid::new(),
            entities: EntityStore::new(),
            pending_delete: Vec::new(),
        }
```

**Step 3: Verify** — `cargo check -p vera20k`. Expected: compiles (serde derive accepts the
skipped `Vec`; `Default` flows through `new()`).

**Step 4: Commit** — `feat(sim): add transient pending_delete queue to ObjectSubstrate (Slice 6)`.

### Task 2: Add `Simulation::flush_pending_delete()`

**Why:** The single drain primitive; both drain sites call it. Define before wiring callers.

**Files:** Modify `src/sim/world/mod.rs` (add immediately after `uninit`/`despawn_entity`, ~line 975)

**Pattern:** new `pub(crate)` method; drains a `Vec` in order via the existing
`entities.remove`.

**Step 1: Add the method:**
```rust
    /// Drain the deferred-delete queue, freeing each enqueued store slot in death
    /// (insertion) order. The gamemd `ProcessPendingDelete` end-of-tick drain. Called
    /// at the end of `run_late_region` (inside `advance_tick`, before the asserts +
    /// state hash) and in the app layer after the death-animation despawn loop. After
    /// this returns the queue is empty and no `Dying` entity remains in the store.
    pub(crate) fn flush_pending_delete(&mut self) {
        // mem::take to avoid borrowing self.substrate while removing through self.
        let queued = std::mem::take(&mut self.substrate.pending_delete);
        for id in queued {
            self.substrate.entities.remove(id);
        }
    }
```
> Use `std::mem::take` so the loop body can call `self.substrate.entities.remove` without a
> simultaneous borrow of `self.substrate.pending_delete`. `remove` of an absent id is a no-op
> (idempotent), covering any defensive double-enqueue.

**Step 2: Verify** — `cargo check -p vera20k`. Expected: compiles.

**Step 3: Commit** — `feat(sim): add flush_pending_delete drain (Slice 6)`.

### Task 3: `uninit` enqueues instead of freeing; sets `dying`

**Why:** Converts death to two-phase. The behavior pivot of the slice.

**Files:** Modify `src/sim/world/mod.rs:939-969`

**Pattern:** keep the existing synchronous teardown order; replace the final `remove()`.

**Step 1:** In `uninit`, after the `conceal` + `presence = Dying` block, replace the final
`self.substrate.entities.remove(stable_id);` (line 968) and set `dying`:
```rust
        if let Some(e) = self.substrate.entities.get_mut(stable_id) {
            debug_assert_ne!(
                e.presence,
                Presence::Dying,
                "uninit: entity {stable_id} already Dying (double teardown?)",
            );
            e.presence = Presence::Dying;
            // IsAlive-equivalent: a queued corpse is dead for all live systems.
            // Idempotent — the count-decrement above already read the original
            // `dying`, so owned-counts are still adjusted exactly once.
            e.dying = true;
        }
        // Two-phase death: enqueue instead of freeing. The slot is freed by
        // flush_pending_delete at end-of-tick (gamemd ProcessPendingDelete).
        self.substrate.pending_delete.push(stable_id);
```

**Step 2: Update the stale comment** at `world/mod.rs:957-959` (the "In this slice the slot is
freed immediately below" note) to:
```rust
        // Conceal moved presence to Limbo (or it was already Limbo for a never-
        // revealed limbo object); we then mark Dying + enqueue. The store slot is
        // NOT freed here — flush_pending_delete frees it at end-of-tick. The entity
        // stays resolvable by id as a Dying corpse until then (the gamemd window).
```

**Step 3: Verify** — `cargo check -p vera20k`. Expected: compiles. (Tests will be addressed in
Task 10; some will fail until then — that is expected and noted there.)

**Step 4: Commit** — `feat(sim): uninit enqueues to pending_delete instead of freeing (Slice 6)`.

### Task 4: Drain inside `advance_tick` (end of Phase 9, before the debug block)

**Why:** Frees the immediate (structure/voxel) + undeploy + sell + slave + engineer enqueues
before the asserts and state hash, so the store is clean at the hash and `OCCUPANCY_DEBUG`
doesn't see a dying entity.

**Files:** Modify `src/sim/world/mod.rs` in `run_late_region` (between line 1613 and 1615)

**Pattern:** a single method call at the Phase 9 cleanup tail.

**Step 1:** Insert the flush after `self.sound_events.extend(started_effect_sounds);`
(`world/mod.rs:1613`) and **before** the `#[cfg(debug_assertions)] if … OCCUPANCY_DEBUG`
block (`1615-1622`):
```rust
        self.sound_events.extend(started_effect_sounds);

        // End-of-tick deferred-delete drain (gamemd ProcessPendingDelete). Frees
        // every entity uninit'd during this advance_tick (immediate structure/voxel
        // deaths in Phase 5, tick_building_down undeploy frees, sells, slave-miner
        // conversions, engineer-capture consumption). Runs BEFORE the OCCUPANCY_DEBUG
        // rebuild (which scans all entities and would re-add an unflushed dying
        // structure) and before the tail presence/membership asserts + state_hash.
        self.flush_pending_delete();
```

**Step 2: Verify the no-setup-caller invariant.** Confirm every `uninit`/`despawn_entity`
caller runs either inside `advance_tick` (drained here) or in the app anim-end loop (Task 5):
non-test callers are `world/mod.rs:1464,1964`, `slave_miner.rs:473,555`,
`production_sell.rs:716`, `world_orders.rs:243,409`, `world_spawn.rs:702`, `app_sim_tick.rs:307`.
All except `app_sim_tick.rs:307` are reached only via command application / production / combat
inside `advance_tick`. **Grep to confirm no caller is on a map-load/setup path that runs outside
a tick** (`grep -rn "uninit\|despawn_entity" src/ | grep -v test`); if one is found, that path
must call `flush_pending_delete()` itself. Record the result.

**Step 3: Verify** — `cargo check -p vera20k`. Expected: compiles.

**Step 4: Commit** — `feat(sim): drain pending_delete at end of Phase 9 in advance_tick (Slice 6)`.

### Task 5: Drain in the app layer after the anim-end despawn loop

**Why:** Animated corpses are despawned (`uninit`) in the app layer after `advance_tick`; flush
immediately so they free at the same frame as today — no extra tick of linger.

**Files:** Modify `src/app_sim_tick.rs:300-311`

**Pattern:** a single method call after the existing despawn `for` loop.

**Step 1:** Fold the drain into the **existing** `if !death_finished.is_empty()` block at
`app_sim_tick.rs:309-311` (do NOT add a second identical conditional — review finding):
```rust
            if !death_finished.is_empty() {
                // Anim-end corpses were uninit'd above (enqueued). Drain now so they
                // free at exactly this frame — the deferred queue must not carry an
                // animated death into the next tick.
                sim.flush_pending_delete();
                refresh_after_tick = true;
            }
```

**Step 2: Verify** — `cargo check -p vera20k`. Expected: compiles (`flush_pending_delete` is
`pub(crate)`; `app_sim_tick.rs` is in-crate).

**Step 3: Commit** — `feat(app): flush pending_delete after death-animation despawn (Slice 6)`.

### Task 6: Update the presence/Dying doc comments

**Why:** The slice makes `Dying` a real mid-tick in-store state; the existing comments assert it
never persists, which is now misleading.

**Files:** Modify `src/sim/world/mod.rs:833-838`, `src/sim/game_entity.rs:130-133`

**Step 1:** In `world/mod.rs` above `debug_assert_presence_consistent`, replace the
"`Dying` is transient inside `uninit` (slot freed same call), so no in-store entity is ever
`Dying` at a tick boundary in this slice." sentence with:
```rust
    /// `Dying` entities exist in-store between `uninit`'s enqueue and the end-of-tick
    /// `flush_pending_delete`. The flush runs in Phase 9 before this assert, so no
    /// `Dying` entity remains in the store at this assert's call point.
```

**Step 2:** In `game_entity.rs:130-133`, update the `Presence::Dying` doc to:
```rust
/// FSM, Slice 2). ... `Dying` is set during teardown after conceal and persists in
/// the store until the end-of-tick deferred-delete drain frees the slot (Slice 6) —
/// during that window the entity is resolvable by id but off occupancy + off the
/// logic vector, excluded from all live systems.
```

**Step 3: Verify** `derived_presence` is not invoked on a `Dying` in-store entity before the
flush — grep `derived_presence` (`grep -rn "derived_presence" src/sim/`); confirm the only
caller is `debug_assert_presence_consistent` (runs post-flush). If another caller exists that
runs mid-tick, ensure it tolerates `Dying`. Record the result.

**Step 4: Verify** — `cargo check -p vera20k`. **Commit** — `docs(sim): update Presence::Dying for the deferred-delete window (Slice 6)`.

### Task 7: Gating fix — AI structure/unit scans exclude dying

**Why:** Under deferral a dying structure lingers through Phase 8 (AI); these ungated
`entities.values()` scans would wrongly count it (e.g. "owner still has a refinery", target a
dead enemy structure). Excluding it preserves the pre-slice behavior (gamemd live-vector).

**Files:** Modify `src/sim/ai.rs` at the six scan sites.

**Pattern:** mirror the dock gate `entity.dying || entity.health.current == 0` (`miner_system.rs:1114`).
Defensive invariant-preservation — **not** AI feature work (`feedback_no_ai_yet` respected).

**Step 1:** Add `&& !e.dying` (or `if entity.dying { continue; }` for `for` loops) to each:
- `ai.rs:190` `has_owned_structure_matching` closure — add `&& !e.dying` to the `.any(|e| …)`.
- `ai.rs:1010` `count_refineries` filter — add `&& !e.dying`.
- `ai.rs:152` deployable scan `for` loop — add `if entity.dying { continue; }` after the owner check.
- `ai.rs:426` idle-units `for` loop — add `if entity.dying { continue; }` after the owner check.
- `ai.rs:486` `find_base_center` `for` loop — add `&& !entity.dying` to the `if` condition.
- `ai.rs:512` `find_nearest_enemy_structure` `for` loop — add `if entity.dying { continue; }`
  (alongside the existing category check).

Example (190):
```rust
    sim.substrate.entities.values().any(|e| {
        !e.dying
            && e.category == EntityCategory::Structure
            && sim.interner.resolve(e.owner).eq_ignore_ascii_case(owner)
            && matches(sim.interner.resolve(e.type_ref))
    })
```

**Step 2: Verify** — `cargo check -p vera20k`. Expected: compiles.

**Step 3: Commit** — `fix(sim): AI scans exclude dying structures under deferred-delete (Slice 6)`.

### Task 8: Gating fix — production placement scans exclude dying

**Why:** A dying structure must not block placement or provide build-area adjacency (gamemd
unmarks it from cell lists synchronously). Reached when AI places in Phase 8 the same tick a
structure died in Phase 5.

**Files:** Modify `src/sim/production/production_placement.rs:427,462`

**Step 1:** `production_placement.rs:427` (footprint-block scan `entities.values().any(|e| …)`):
add `if e.dying { return false; }` at the top of the closure (before the category check).

**Step 2:** `production_placement.rs:462` (`is_within_build_area` `for e in …values()` loop):
add `if e.dying { continue; }` after the `category != Structure` continue.

> `production_placement.rs:324` and `production_queue.rs:273` are diagnostic-only
> (`[BUILD-DIAG]` log / error-detail string), not hashed — leave them, or add the same gate for
> consistency; either is acceptable. Note the choice.

**Step 3: Verify** — `cargo check -p vera20k`. **Commit** — `fix(sim): placement scans exclude dying structures (Slice 6)`.

### Task 9: Regression test — dying attacker yields the same retaliation outcome as an absent one

**Why:** Locks the parity contract that a resolvable-but-`Dying` `last_attacker_id` is treated
identically to a freed/`None` one.

**Files:** Modify `src/sim/combat/combat_tests.rs` (add a test)

**Step 1:** Add a test that: places a victim with `last_attacker_id = A`, no `attack_target`, no
`order_intent`; in one branch leaves A absent (already freed), in the other leaves A present with
`health.current = 0, dying = true`; runs `tick_retaliation` on both; asserts both produce the
same result — no retaliation issued and `last_attacker_id` cleared (`combat_targeting.rs:351`
gates on `health > 0`, line 400 clears regardless).
```rust
#[test]
fn dying_attacker_retaliation_matches_absent_attacker() {
    // ... build two sims; one with A removed, one with A dying (health 0); ...
    // run tick_retaliation on both; assert no new attack_target on the victim and
    // last_attacker_id == None in both.
}
```

**Step 2: Verify** — `cargo test -p vera20k --lib dying_attacker_retaliation -- --nocapture`.
Expected: PASS. **Commit** — `test(sim): dying attacker retaliation == absent attacker (Slice 6)`.

### Task 10: Update existing uninit/despawn tests to the two-phase semantics

**Why:** Three tests assert immediate store-absence right after `uninit`/`despawn_entity`; under
deferral the entity is `Some`+`Dying` until a flush. Update them to assert the new contract.

**Files:** Modify `src/sim/snapshot.rs:694-696,709-711`, `src/sim/world/world_tests.rs:147-149`

**Step 1:** `snapshot.rs` `uninit` test (~694) — replace the immediate-absence assert:
```rust
        sim.uninit(2);
        // Two-phase: resolvable-but-Dying until the drain, off the logic order now.
        assert!(sim.substrate.entities.get(2).is_some_and(|e| e.dying));
        assert!(sim.live_object_order_snapshot().is_empty());
        sim.flush_pending_delete();
        assert!(sim.substrate.entities.get(2).is_none());
```

**Step 2:** `snapshot.rs` `despawn_entity_delegates_to_uninit` (~709) — same pattern with id 3.

**Step 3:** `world_tests.rs:147` (`despawn_entity(1)` then `get(1).is_none()`) — insert
`sim.flush_pending_delete();` between the despawn and the `get(1).is_none()` assert, and add a
pre-flush `assert!(sim.substrate.entities.get(1).is_some_and(|e| e.dying));` to document the window.

> `world_tests.rs:60` (`uninit(10)` → occupancy cleared) stays green (occupancy unmarked
> synchronously). `world_hash.rs:935-936` stays green (both sims linger id 1 identically → A-vs-B
> equal). `snapshot.rs:728` (`lifecycle_keeps_membership_invariant`) stays green (logic order
> excludes the concealed entity). Confirm these three still pass without edits.

**Step 4: Verify** — `cargo test -p vera20k --lib` (snapshot + world tests). Expected: the three
edited tests PASS. **Commit** — `test(sim): update uninit/despawn tests for two-phase death (Slice 6)`.

### Task 11: New behavior tests — the Dying window

**Why:** Pins the core slice behavior: immediate-path window, mutual death, animated-path
unchanged.

**Files:** Modify `src/sim/world/world_tests.rs` (or `combat_tests.rs` where structure-death
fixtures exist)

**Step 1 — immediate-path window:** place a structure + a lethal attacker; drive one
`advance_tick`. Assert that *during* the tick (use a direct `uninit` + mid-tick inspection
harness, or assert post-flush state) the structure is enqueued and off occupancy/logic, and that
after the Phase 9 flush `store.get(struct_id) == None` and `store.len()` is correct. Minimal
direct-call form:
```rust
#[test]
fn immediate_structure_death_is_dying_then_flushed() {
    let mut sim = /* place a structure, reveal it */;
    let id = /* structure id */;
    sim.uninit(id);
    assert!(sim.substrate.entities.get(id).is_some_and(|e| e.dying));   // resolvable-but-Dying
    assert!(!sim.live_object_order_snapshot().contains(&id));            // off logic
    // off occupancy:
    // assert !occupancy.contains_entity(rx, ry, id) for each foundation cell
    sim.flush_pending_delete();
    assert!(sim.substrate.entities.get(id).is_none());                  // freed
}
```

**Step 2 — mutual same-tick death:** `uninit(a); uninit(b);` assert both resolvable-but-`Dying`
before the flush; flush; assert both `None`; and that two identical-seed runs of the sequence
produce equal `state_hash()` at the pre-flush point (determinism).

**Step 3 — animated path unchanged:** assert an infantry death still lingers (`dying`, in store)
across animation ticks and is freed only at anim end (exercise via the existing animation-death
test path if present; otherwise assert `uninit` on an animated entity enqueues and a single
`flush_pending_delete` frees it — confirming the app-layer drain semantics).

**Step 4: Verify** — `cargo test -p vera20k --lib` (new tests). Expected: PASS.
**Commit** — `test(sim): deferred-delete Dying-window behavior tests (Slice 6)`.

### Task 12: Full suite, hash-change determination, conditional version bump

**Why:** Evidence gate for the `SNAPSHOT_VERSION` decision and lockstep safety.

**Files:** possibly Modify `src/sim/snapshot.rs:22` (conditional)

**Step 1:** Run the full lib suite: `cargo test -p vera20k --lib`. **Read the literal
`test result:` line** before reporting (batched output arrives delayed —
`feedback_no_premature_result_reporting`). Confirm 0 failures; in particular all
determinism / `world_hash` / `saveload` tests green.

**Step 2 — hash-change determination:** re-capture the Task 0 scenario's `state_hash` (H1) the
same way and compare to H0.
- **If H1 == H0 (expected):** the window is inert — **keep `SNAPSHOT_VERSION = 16`**, do NOT add
  a golden. Record "Slice 6 is behavior/hash-preserving on the current consumer set; the Dying
  window is architecturally present but observationally latent." Report this finding to the user.
- **If H1 != H0:** a consumer observes the deferral. Diagnose: is it a **missed gating site** (a
  whole-store scan still counting the dying structure → fix it, re-test, expect H1==H0) or a
  **legitimate gamemd-faithful exposure** (a by-id consumer correctly now resolving the dying
  object)? If the latter and verified gamemd-correct (cite the SLICE6 evidence artifact, critic
  #4): **bump `SNAPSHOT_VERSION` 16→17**, add a comment line documenting the bump, and add a
  golden test pinning the new value for the representative scenario.

**Step 3:** If Task 0 was skipped, the evidence is the green A-vs-B suite + the new behavior
tests (a missed gate would surface as a determinism failure or a wrong value in Task 11). Default
to **keep 16** unless a concrete observable delta is demonstrated.

**Step 4: Verify** — re-run `cargo test -p vera20k --lib`; confirm `test result: ok`.
**Commit** — `chore(sim): Slice 6 verification + version decision` (include the H0/H1 finding in
the commit body; bump snapshot.rs:22 only if Step 2 took the bump branch).

### Task 13: Final review pass

**Why:** Catch any missed P5.5→P8.5 raw-store scan, confirm the sim/ boundary, confirm
determinism.

**Verify:**
- `grep -rn "entities.values()\|keys_sorted()" src/sim/` cross-checked against the tick phase of
  each caller — any P5.5→P8.5 caller processing `Structure`/voxel-`Unit` category without a
  `dying`/`health==0` gate is a missed site (add the gate, re-run Task 12).
- `sim/` introduces no `use` of `render`/`ui`/`sidebar`/`audio`/`net` (`flush_pending_delete`
  callers are sim + `app_sim_tick.rs` only).
- Determinism: two identical-seed runs of a structure-death scenario produce equal hashes.
- **Commit** — `chore(sim): Slice 6 final scan` (if any gate added) or fold into Task 12.

## Sources & References

- **Design doc:** docs/plans/2026-06-01-slice6-deferred-delete-design.md
- **Evidence artifact:** docs/research/SLICE6_DEFERRED_DELETE_DYING_WINDOW_GHIDRA_REPORT.md
  (`UnInit 0x005F65F0`, `Detach_From_All_Lists 0x007258D0`, `ProcessPendingDelete 0x00725C70`,
  `Main_Tick 0x0055D360`/drain call `0x0055DE9F`, `IsDead 0x005F6690`)
- **Parent design:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md §8 Slice 6,
  critics #4/#6/#9
- **Related research:** docs/research/LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md §3.9,
  COMMON_MIDPASS_UNREGISTER_DESPAWN_CASES_GHIDRA_REPORT.md
- **gamemd.exe addresses:** kept in the docs above, not in Rust comments
  (`feedback_no_engine_refs_in_comments`)
- **Current code:** src/sim/world/substrate.rs:48-83, src/sim/world/mod.rs:833-851/939-969/1463-1478/1539-1634/1963-1965/2258-2262,
  src/app_sim_tick.rs:284-313, src/sim/combat/mod.rs:840-1022, src/sim/ai.rs:152/190/426/486/512/1010,
  src/sim/production/production_placement.rs:427/462, src/sim/docking/{aircraft_dock.rs:294,building_dock.rs:73-84},
  src/sim/miner/miner_system.rs:1100-1118, src/sim/combat/combat_targeting.rs:325-401,
  src/sim/occupancy.rs:118-142, src/sim/game_entity.rs:130-149, src/sim/snapshot.rs:22
- **Prior slices (dev):** Slice 5 (EnterOrderCounter), Slice 4 (incremental by_owner),
  commit 8f7b599 (aircraft_dock alive-set !dying)
- **INI keys:** none
