# Slice 1b — Migrate `EntityStore` into `ObjectSubstrate` Implementation Plan

> **For Claude (NEW SESSION):** Execute task-by-task. This is the large half of Slice 1 — a
> ~1,340-site mechanical refactor with NO behavior change; the state hash MUST stay
> bit-identical. **Do NOT blind-sed `entities`** — it is an extremely common parameter and
> context-struct field name; rely on the compiler to enumerate the real `Simulation` sites
> (see Task 3). Tasks land together (a field move does not compile half-done); first clean
> compile is after Task 4 — verify it with `cargo check --all-targets` (the field is also used by
> integration tests under `tests/`, which a plain `cargo check` does NOT build, so they would
> stay broken behind a green check). Recommend running `/review-plan` on this before executing.

**Goal:** Move the `entities: EntityStore` field off `Simulation` into `ObjectSubstrate`
(as `substrate.entities`), completing Slice 1's storage migration — the substrate then owns
the entity store, occupancy grid, active-object vector, and counters, instead of those being
scattered as flat `Simulation` fields.

**Architecture:** Entirely inside `sim/`. `ObjectSubstrate` (`src/sim/world/substrate.rs`)
gains the `entities` field; `Simulation` (`src/sim/world/mod.rs`) drops it and exposes
`entities()`/`entities_mut()` accessors. Internal sim/ uses the field path
`self.substrate.entities` (preserves disjoint-field borrows); the ~146 external sites route
through the accessors. No `render`/`ui`/`audio`/`net` dependency added.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](../research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md)
— §6 architecture (lines 176–182, `store: EntityStore`), Slice 1 (line 225), borrow
discipline (line 193).

**Precedent:** Slice 1a (commit `d924b20`, [2026-05-31-slice1a-occupancygrid-substrate-plan.md](2026-05-31-slice1a-occupancygrid-substrate-plan.md))
did the identical move for `OccupancyGrid`. This plan mirrors it; the **differences from 1a
are called out explicitly** below (serialization + over-reach scale).

---

## Grounding Summary

- **Design (lines 176–182, 225):** Slice 1's target has `ObjectSubstrate` own
  `store`/`logic`/`occupancy`/counters. After 1a, only `entities` (the `EntityStore`) remains
  on `Simulation` ([mod.rs:272](../../src/sim/world/mod.rs)). This plan moves it.
- **Borrow discipline (line 193):** "storage stays independently borrowable." Internal sim/
  co-mutates `entities` with `occupancy`/counters. After the move these become
  `&mut self.substrate.entities` + `&mut self.substrate.occupancy` + counter — all **disjoint
  fields of `substrate`**, which Rust's disjoint-field borrow checker accepts (same proof as
  1a, now both operands inside `substrate`). Internal sites therefore use the **field path**,
  never an accessor method.
- **⚠ DIFFERENCE FROM 1a #1 — serialization.** `entities` is NOT `#[serde(skip)]` (occupancy
  was). It is real persisted state. Moving it nests it under `substrate` in the serialized
  form. **The state hash is unaffected** (`world_hash.rs` reads fields explicitly via the new
  path), but the **snapshot byte-format changes**. The `c2b5153` commit already did this when
  it moved `logic`/counters into `substrate` (round-trip tests only, no byte-golden) and
  **still bumped `SNAPSHOT_VERSION` 14→15** — so Slice 1b MUST bump it **15→16**
  unconditionally (Task 5). **Verified during review:** `src/sim/snapshot.rs` has NO byte-golden;
  every test is round-trip (save→load→compare-hash) and passes with or without the bump, so the
  suite gives **no signal** — the bump is purely for on-disk save compatibility (old v15 saves
  must be rejected cleanly, not silently mis-deserialized under the new layout).
- **⚠ DIFFERENCE FROM 1a #2 — over-reach hazard is severe.** `entities` is a pervasive
  parameter and context-struct field name: dozens of `fn …(entities: &EntityStore)` /
  `&mut EntityStore`, plus context structs with their own `entities` field
  (`cell_rect.rs:305` `entities: Option<&EntityStore>`; `TerrainSpawnContext` —
  `terrain_spawn.rs` `self.entities = Some(entities)`). A blanket sed of `self.entities`/
  `sim.entities` **WILL corrupt these**. **Use the compiler-driven sweep ONLY** (Task 3): the
  compiler flags exactly the `Simulation`-receiver sites (E0615/E0609) and leaves local
  `EntityStore` params and other structs' `entities` fields untouched.
- **Completeness is compiler-guaranteed:** removing the field turns every missed
  `Simulation` `.entities` into a hard error (E0615 "take value of method" once the accessor
  exists, or E0609 "no field"), so the sweep cannot silently miss a site.
- **Blast radius (measured):** `.entities` ≈ 1,194 in sim/, ≈146 outside (**`src/`-only count**).
  Receiver names: mostly `self` (mod.rs, world_commands, world_orders) and `sim` (other modules
  + tests), PLUS Simulation-typed local test fixtures with other names (e.g. `stable_order`/
  `live_order` in movement_tests — 1a hit these; expect more for entities). The compiler lists
  them all. **Beyond `src/`:** integration tests under `tests/` (e.g.
  `tests/refinery_live_rules.rs:156`) also access `sim.entities` and must route through the
  **public accessor** (`substrate` is `pub(crate)`, invisible to an external test crate). These
  are only built by `cargo test` / `cargo check --all-targets`, NOT plain `cargo check`.
- **EntityStore derives — VERIFY FIRST (Task 1):** `ObjectSubstrate` derives `Debug, Clone`
  ([substrate.rs:20](../../src/sim/world/substrate.rs)); `EntityStore`
  ([entity_store.rs:33](../../src/sim/entity_store.rs)) must impl both or the move won't
  compile (this is exactly the gotcha 1a hit with `OccupancyGrid`). If missing, add the
  derives (its fields are a `BTreeMap` + `by_owner` index — should support both).
- **Unknown after grounding:** whether `snapshot.rs` has a byte-golden that needs regenerating
  vs a round-trip test that doesn't (Task 5 resolves it). Whether `EntityStore` already derives
  Debug/Clone (Task 1 resolves it).

## Key Technical Decisions

- **Field name `substrate.entities` (not `store`).** **Confidence:** medium — **Source:**
  design doc says `store: EntityStore`, but `entities` matches the current field name and the
  `entities()`/`entities_mut()` accessor names, minimizing cognitive load and churn. Flag for
  `/review-plan`: rename to `store` only if the reviewer prefers strict design-doc alignment.
- **Internal access via field path `self.substrate.entities`, external via accessors.**
  **Confidence:** high — **Source:** design line 193 + the 1a precedent (commit `d924b20`).
- **Compiler-driven sweep, NO blind sed.** **Confidence:** high — **Source:** the over-reach
  hazard above (1a's terrain_spawn over-reach + missed test-fixture bindings would scale to
  many corruptions here). The compiler is the only safe discriminator.
- **Snapshot format change is acceptable; hash stays identical; bump `SNAPSHOT_VERSION` 15→16
  unconditionally.** **Confidence:** high — **Source:** `c2b5153` (logic/counters nested under
  substrate, round-trip-only tests, still bumped 14→15) + review-verified that
  `src/sim/snapshot.rs` has no byte-golden. The bincode layout changes when a non-skip field
  moves, so the bump is mandatory **regardless of test outcome** (the round-trip suite passes
  either way and is not a safety net for this).

## Open Questions

### Resolved During Planning
- *Does the move break co-borrows?* → No. `&mut self.substrate.entities` + other
  `self.substrate.*` are disjoint fields → compiles (same proof as 1a).
- *Does it change the state hash?* → No. `world_hash` reads entity data explicitly via the
  new path; hash input is unchanged.

### Resolved During Review
- *Does `EntityStore` already derive Debug/Clone?* → **No.** [entity_store.rs:33](../../src/sim/entity_store.rs)
  has no `#[derive]`; it has **manual** `impl Serialize/Deserialize` (lines 154-161), so serde is
  already satisfied and only `Debug, Clone` are missing. Task 1 adds exactly those two.
- *Is there a snapshot byte-golden needing regeneration / a version bump?* → **No byte-golden;
  round-trip tests only.** The version bump 15→16 is **mandatory** (bincode layout change), not
  conditional on a golden. Task 5 Step 1 does it unconditionally.

### Deferred to Implementation
- *Exact rewrite-site list* → the compiler enumerates after Task 2 removes the field.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/entity_store.rs` | Add `Debug, Clone` derives to `EntityStore` if missing |
| Modify | `src/sim/world/substrate.rs` | Add `entities: EntityStore` field + init in `new()` |
| Modify | `src/sim/world/mod.rs` | Remove `entities` field + initializer; add `entities()`/`entities_mut()` accessors; rewrite compiler-flagged `self.entities`→`self.substrate.entities` |
| Modify | (compiler-flagged sim/ files) | `<sim_binding>.entities`→`<sim_binding>.substrate.entities` |
| Modify | (compiler-flagged app/render files + `tests/` integration tests, ~146 in `src/`) | `sim.entities`→`sim.entities()`/`sim.entities_mut()` |
| Modify | `src/sim/snapshot.rs` | Bump `SNAPSHOT_VERSION` 15→16 (**mandatory** — bincode layout change; no byte-golden exists) |

## Interface Changes

- **New public methods on `Simulation`:** `entities(&self) -> &EntityStore`,
  `entities_mut(&mut self) -> &mut EntityStore`. They replace direct `pub entities` field
  access for the ~146 external sites.
- **Removed:** `pub entities: EntityStore` field on `Simulation`.

## Sim Checklist

- [x] No new f32/f64 — pure field move.
- [ ] **State-hash impact: must be NONE.** Verified by the `cargo test -p vera20k` suite (Task 5)
      — the snapshot round-trip + lifecycle/determinism hash tests; there is **no standalone
      5000-tick replay harness**, so the suite is the oracle. Snapshot byte-format DOES change
      (entities nests under substrate) — that is a format change, not a behavior change; the hash
      is computed by explicit field reads and stays identical. If a hash test moves, STOP — a
      read path was missed.
- [x] No deps on render/ui/sidebar/audio/net.
- [x] Tick ordering: unaffected.
- [x] BTreeMap iteration order: unaffected (`EntityStore` is moved whole; its internal
      `BTreeMap<u64, GameEntity>` order is preserved).

## Risk Areas

- **Over-reach (HIGH) — the dominant risk.** `entities` is a pervasive param/field name. Do
  NOT sed. Only rewrite compiler-flagged `Simulation`-receiver sites. After the sweep, the
  compiler must be 100% green AND a spot-check must confirm no local `entities: &EntityStore`
  param or context-struct field (`cell_rect`, `TerrainSpawnContext`) was rewritten.
- **Borrow conflict (low):** only if a site whole-borrows `self.substrate` while also
  borrowing `self.substrate.entities`. `ObjectSubstrate` has no `&mut self` methods, so this
  should not occur; field paths are always disjoint-safe. Compiler surfaces it instantly.
- **Snapshot format / save-compat (medium):** moving a serialized field reshuffles the saved
  bytes. **Bump `SNAPSHOT_VERSION` 15→16 unconditionally** (the project tracks it; no byte-golden
  exists). The round-trip suite passes with or without the bump, so it is **not** a safety net
  here — the bump is purely for on-disk save compatibility. This is expected, not a behavior bug.
- **Missed site (eliminated):** removing the field makes every missed `Simulation` `.entities`
  a compile error.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 5 | Deterministic state hash unchanged | A storage-owner move must not alter lockstep state; any hash drift = a missed read path, not an intended change | Full `cargo test -p vera20k` green (round-trip + lifecycle/determinism hash tests — no standalone 5000-tick harness); snapshot round-trip passes; `SNAPSHOT_VERSION` bumped 15→16 |

Refactor with **no** gamemd-mechanism change — the only parity stake is that the hash and all
lifecycle/snapshot tests stay byte-for-byte unchanged in behavior.

---

## Tasks

### Task 1: Ensure `EntityStore` derives `Debug` + `Clone`

**Why:** `ObjectSubstrate` derives `Debug, Clone`; its new `entities` field must satisfy that.
1a hit this exact gotcha with `OccupancyGrid` (which lacked both, since `Simulation` derives
neither).

**Files:** Modify: `src/sim/entity_store.rs`

**Step 1 — Check.** Read the `#[derive(...)]` on `pub struct EntityStore` (~line 33). If it
already includes `Debug` and `Clone`, skip to Task 2.

**Step 2 — Add if missing.** Add `Debug, Clone` to the derive list. If a field blocks the
derive (unlikely — it is a `BTreeMap` + index), resolve that field's derive first; do not
hand-roll `Clone` unless forced.

**Step 3 — Verify compile.** Run: `cargo check -p vera20k`. Expected: clean.

### Task 2: Add `entities` to `ObjectSubstrate`; remove from `Simulation`; add accessors

**Why:** Make the substrate the owner. Adding to the substrate first lets the two fields
coexist briefly; removing from `Simulation` breaks the call sites (intended — Task 3 sweeps).

**Files:** Modify: `src/sim/world/substrate.rs`, `src/sim/world/mod.rs`

**Step 1 — Add the field to `ObjectSubstrate`** (after `occupancy`), in `substrate.rs`:
```rust
    /// Plain-struct entity storage (BTreeMap<u64, GameEntity> + by_owner index).
    /// The authoritative object store — serialized verbatim (NOT skipped).
    pub(crate) entities: EntityStore,
```
Add the import near the others: `use crate::sim::entity_store::EntityStore;`

**Step 2 — Init it in `ObjectSubstrate::new()`** (match the current `Simulation::new`
initializer — `EntityStore::new()`):
```rust
            occupancy: OccupancyGrid::new(),
            entities: EntityStore::new(),
```

**Step 3 — Remove the field from `Simulation`** at [mod.rs:271-272](../../src/sim/world/mod.rs):
```rust
    /// Plain-struct entity storage.
    pub entities: EntityStore,
```
(Remove both lines.)

**Step 4 — Remove its initializer** in `Simulation::new` (~[mod.rs:473](../../src/sim/world/mod.rs)):
```rust
            entities: EntityStore::new(),
```

**Step 5 — Add the accessors** in `impl Simulation` (next to the `occupancy()`/`occupancy_mut()`
accessors added in 1a):
```rust
    /// The entity store. Read access for systems above sim/.
    pub fn entities(&self) -> &EntityStore {
        &self.substrate.entities
    }

    /// Mutable entity-store access for above-sim callers.
    pub fn entities_mut(&mut self) -> &mut EntityStore {
        &mut self.substrate.entities
    }
```

**Step 6 — Do NOT compile yet** (Step 3 broke the internal sites; Task 3 fixes them). Keep the
`EntityStore` import in mod.rs — still used by accessor return types and any `EntityStore::`
calls.

### Task 3: Compiler-driven sweep — `self.entities`/`<sim>.entities` → `…substrate.entities`

**Why:** Re-point every `Simulation` receiver at the new owner. **DO NOT SED.** The compiler
flags exactly the Simulation sites and leaves local `entities` params + other structs' fields
alone.

**Files:** Modify: every file the compiler flags.

**Step 1 — Get the list.** Run (`--all-targets` so lib + bins + unit tests + `tests/`
integration tests are all compiled — a plain `cargo check` skips `tests/` and would hide
the integration-test sites until Task 5):
`cargo check --all-targets -p vera20k 2>&1 | rg "E0615|E0609|take value of method .entities|no field .entities" -A1`
Every flagged location is a `Simulation` `.entities` access (the receiver is typed
`&Simulation`/`&mut Simulation`/`Simulation`).

**Step 2 — Rewrite each flagged site** from `<recv>.entities` to `<recv>.substrate.entities`,
where `<recv>` is `self`, `sim`, or a Simulation-typed local binding (e.g. `stable_order`,
`live_order` in tests — 1a hit these; the compiler confirms the type). For multi-line
continuations (`recv\n    .entities`), insert `.substrate` as its own line before `.entities`.

> **DO NOT** touch: local `entities: &EntityStore` / `&mut EntityStore` parameters (they call
> `entities.foo()` directly and will NOT be flagged), or context-struct fields
> (`cell_rect`’s `entities: Option<&EntityStore>`, `TerrainSpawnContext`’s `self.entities`).
> The compiler does not flag these — if you see a `no field substrate on <SomeContext>` error,
> you over-reached: revert that site to plain `.entities`.

**Step 3 — Iterate to green.** Re-run Step 1's check; fix newly surfaced sites (errors cascade
as earlier ones resolve). Continue until zero `entities`-related errors remain in `sim/`.
(The ~146 external sites in Task 4 also block full compile — do them in parallel.)

### Task 4: Route external (above-sim) sites through the accessors

**Why:** App/render code can no longer touch the field.

**Files:** Modify: every compiler-flagged file outside `src/sim/` (~146 sites in
`src/app_*`, `src/render/`, `src/ui/`, etc.) **plus `tests/` integration tests** (e.g.
`tests/refinery_live_rules.rs:156`), which are external crate consumers and can only use the
`pub` accessor.

**Step 1 — Enumerate.** Run (with `--all-targets`; do NOT filter the path down to
app/render/ui — `tests/` and `src/bin/` consumers must surface too):
`cargo check --all-targets -p vera20k 2>&1 | rg "E0615|E0609" -A1`

**Step 2 — Rewrite each:** read access → `sim.entities()`; mutable access →
`sim.entities_mut()`. Where the old code took `&sim.entities`, drop the `&` (the accessor
already returns a reference). This includes `tests/` integration tests — they MUST use the
public accessor (`sim.substrate.entities` is `pub(crate)`, invisible across the crate
boundary). Watch for context structs / params outside sim/ that have their own `entities`
field (`app_entity_pick.rs` passes `entities: &EntityStore`; `src/bin/inspect-yro-render.rs`
uses `map.entities` on the *map* struct) — those are NOT Simulation access; leave them (the
compiler will not flag them).

**Step 3 — Compile.** Run: `cargo check --all-targets -p vera20k`. Expected: clean once
Tasks 3+4 are complete (the `--all-targets` build includes `tests/` and `src/bin/`, so a
green result here is a true clean compile — unlike a plain `cargo check`).

### Task 5: Verify hash-identity + snapshot format + full suite + commit

**Why:** Prove behavior-preserving (same hash, same tests) and handle the serialized-format
change deliberately.

**Step 1 — Bump `SNAPSHOT_VERSION` (mandatory, unconditional).** Edit `src/sim/snapshot.rs`:
`SNAPSHOT_VERSION` **15 → 16**, with a comment noting the `entities`-under-`substrate` bincode
relocation (state hash unchanged), mirroring the existing `c2b5153` 14→15 comment. This bump
is **not conditional** on anything: moving a non-`#[serde(skip)]` field into `substrate` changes
the bincode positional layout, so old v15 saves must be rejected with a clean `VersionMismatch`
rather than silently mis-deserialized under the new layout.

> **Do NOT skip the bump just because the snapshot tests are round-trip.** They are
> (save→load→compare-hash, no byte-golden — confirmed during review), so they pass whether or
> not you bump — that is exactly why the bump is easy to forget. `c2b5153` had the identical
> round-trip-only tests and still bumped 14→15. For reference: `git show c2b5153 -- src/sim/snapshot.rs`.

**Step 2 — Targeted tests.** Run:
`cargo test -p vera20k -- world_hash snapshot lifecycle occupancy entity_store`
Expected: PASS (read the literal `test result:` line). A `world_hash`/`state_hash` failure
means a read path was missed — STOP and investigate; do not regenerate a hash golden.

**Step 3 — Full suite.** Run: `cargo test -p vera20k`. Expected: **0 failed**, same pass count
as before. (There is no byte-golden to regenerate — the only snapshot change is the
`SNAPSHOT_VERSION` 15→16 bump from Step 1, and the round-trip tests pass unchanged because the
save and load paths move together.)

**Step 4 — Over-reach audit.** Run: `rg -n "\.substrate\.entities\b" src` and spot-check that
every hit's receiver is a `Simulation` (not a context struct). Then run
`rg -n "\bentities\b: ?(Option<)?&?(mut )?EntityStore" src` to confirm no param/field decl was
mangled.

**Step 5 — Commit.** `refactor(sim): move EntityStore into ObjectSubstrate (Slice 1b)` — note
in the body whether `SNAPSHOT_VERSION` was bumped and why (serialized format reshuffle, hash
identical). This completes Slice 1.

## Sources & References

- **Design doc:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md — §6 (176–182),
  Slice 1 (225), borrow discipline (193), retire-list (§7).
- **Precedent commits:** `c2b5153` (substrate introduced — logic/counters, the serialized-move
  precedent); `d924b20` (Slice 1a — OccupancyGrid, the identical move pattern).
- **Current Rust:** `src/sim/world/mod.rs` — `entities` field 271-272, init **468** (not ~473),
  accessors next to `occupancy()`/`occupancy_mut()` (mod.rs:581-587), `substrate` field
  `pub(crate)` (mod.rs:321); `src/sim/entity_store.rs:33` (`EntityStore` struct — **no `#[derive]`**;
  manual `impl Serialize/Deserialize` at 154-161, so only `Debug, Clone` are missing);
  `src/sim/world/substrate.rs:21` (target owner; derives `Debug, Clone, Serialize, Deserialize`);
  `src/sim/snapshot.rs` (snapshot tests are **round-trip only, no byte-golden**;
  `SNAPSHOT_VERSION` currently 15). External accessor sites include `tests/refinery_live_rules.rs:156`.
  Over-reach traps: `src/sim/cell_rect.rs:305`, `src/sim/terrain_spawn.rs:222/288`,
  `src/app_entity_pick.rs`, `src/bin/inspect-yro-render.rs:140` (`map.entities`) — all have their
  own `entities` — leave alone.
- **INI keys:** none. **Ghidra:** none (pure refactor).
