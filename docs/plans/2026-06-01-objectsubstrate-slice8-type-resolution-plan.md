# ObjectSubstrate Slice 8 — Type-Resolution Boundary — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

## STATUS — 2026-06-02 (core complete, Task 4 deferred)

Slice 8 acceptance is **met and verified**. Commits on `dev`:
- `f32479ba` Task 1 — index-addressable object storage + `TypeHandle`
- `dcd455f9` Task 2 — `TypeHandleTable` (one-hop)
- `73d9edeb` Task 3 — wired into `Simulation` + `object_type` resolver
- `4fc2a998` Task 5 — retired `Owner(InternedId)`
- `02799812` Task 6 — collapsed duplicate intern helpers
- `b6ef1de1` Task 7 — completeness + casing acceptance tests

Hash-identity oracle: full lib suite **3521 passed / 0 failed**; `type_handles`
excluded from `state_hash`. Casing was already CI before this slice (commit
`845ea12b`), so that subtask was a no-op confirmed by the guard test.

**DEFERRED → Slice 8b:** Task 4, the call-site sweep (re-census: **81 sites / 26
sim/ files**, hash-neutral perf cleanup). Deferred by user decision to avoid
clobbering the concurrent mission/radio-substrate session active in the same
files. Run it as a focused follow-up once sim/ is quiet. The one-hop path
(`object_type`) already exists and is wired; the sweep only swaps existing
two-hop call sites onto it. Migration recipe + grouping: see Task 4 below.

---

**Goal:** Collapse entity→type resolution to one precomputed hop via a `TypeHandle`
index and a sim-side `TypeHandleTable`, and retire the leftover ad-hoc type/owner
plumbing — without changing any resolved type identity (state-hash identical).

**Architecture:** `RuleSet` (rules/) gains integer-indexable object storage and a
case-insensitive `type_handle(&str) -> Option<TypeHandle>` + `object_by_handle`. A new
`TypeHandleTable` in sim/ maps `InternedId → TypeHandle`, built at the `intern_all_ids`
init seam (rules/ → sim/ one-way preserved). `GameEntity` type resolution goes
`type_ref → table → object_list[idx]` (two array indexes, no string alloc, no hash).

**Design Doc:** `docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` §8 (+ §6
TypeHandleTable, §7 item 11, C13/C14, critic #8).

---

## Grounding Summary

- **Design (§8 / §6 / C13):** add `RuleSet::type_handle(&str)` (always
  case-insensitive) + `TypeHandleTable` (`InternedId→TypeHandle(index)`) built at
  `intern_all_ids`; entity→`&ObjectType` in one hop; retire `Owner(InternedId)`
  (components.rs:80) + duplicate `intern_owner`/`intern_type` (app_commands.rs).
- **Ghidra (pre-accepted, do NOT re-decompile):** shared matcher `FUN_007C8D20` is
  case-INSENSITIVE (OR-0x20 fold). So all name→type resolution must be CI.
- **⚠ Premise already partly satisfied in code (commit `845ea12b`, 2026-05-31):**
  `StringInterner` (intern.rs) uppercase-normalizes keys → `intern("htnk")` and
  `intern("HTNK")` already return the same `InternedId`. `RuleSet::object/weapon/
  warhead/projectile` already route through `lookup_ci` (ruleset.rs:1724) → already
  case-insensitive. No raw case-sensitive `.objects.get()` exists outside the
  accessors (verified by grep). **Therefore "deprecate the case-sensitive raw gets"
  is a no-op rename/cleanup, and the casing regression test is a guard/confirmation
  test, not a bug fix.** This is surfaced, not papered over.
- **Repo patterns mirrored:** `resolve_bridge_warheads` (ruleset.rs:1688, called at
  app_init.rs:626) is the precedent for a "pre-resolve IDs against the interner at
  init" method — the table build copies its shape. `StringInterner` (intern.rs) is
  the existing `index → Vec` + `key → id` pattern the `object_index`/`object_list`
  split mirrors.
- **INI keys:** none — this is an internal resolution-boundary refactor; no new keys.
- **Storage today:** `objects: HashMap<String, ObjectType>` (private, ruleset.rs:1302),
  inserted with raw registry casing (`objects.insert(id.clone(), obj)`, ruleset.rs:1415).
  Only ~5 internal `self.objects` sites + `collect_weapon_refs(&objects)` read it.
- **Resolution surface:** `entity.type_ref: InternedId` (game_entity.rs:176); the
  two-hop `rules.object(interner.resolve(e.type_ref))` appears in **42 sim/ files**
  and many app/ files. `Owner(InternedId)` is used at exactly ONE site
  (render/minimap_tests.rs:119).
- **Still unknown after grounding (→ Deferred):** whether the INI registry actually
  contains any two type ids differing only by case (would change the hash on the
  uppercase-key collapse — the self-check). Resolved empirically by Task 7.

## Key Technical Decisions

- **`TypeHandle(u32)` = index into a `Vec<ObjectType>` (`object_list`).** A true
  one-hop deref needs integer-indexable storage; a HashMap has no stable index.
  — **Confidence:** high — **Source:** §6 "TypeHandleTable(index)"; storage blast
  radius measured (5 internal sites, private field).
- **Restructure `objects: HashMap<String,ObjectType>` → `object_list: Vec<ObjectType>`
  + `object_index: HashMap<String, TypeHandle>` (uppercase key).** Uppercase key means
  exact O(1) CI lookup, retiring `lookup_ci`'s scan fallback for objects. — **Confidence:**
  high — **Source:** repo pattern (StringInterner), measured blast radius.
- **Uppercase-key collapse is MORE faithful, but may move the hash IF the INI holds
  two object ids differing only by case.** gamemd's find-or-allocate (`FUN_007C8D20`)
  merges such pairs to one type; our current per-String-key HashMap keeps both. If the
  hash moves, that is a real missing-type finding to investigate (per acceptance), not
  papered over. — **Confidence:** high (mechanism) / unknown (whether any such pair
  exists) — **Source:** C13; resolved by Task 7.
- **`TypeHandleTable` lives on `Simulation` (sim/), not `ObjectSubstrate`.** It is a
  rules-derived cache bridging interner+rules, serde-skip, rebuilt at init/load. The
  substrate owns store/logic/occupancy; the table is a sibling. — **Confidence:** high
  — **Source:** §6; `Simulation` owns `interner` (mod.rs:272), `RuleSet` is threaded as
  `&RuleSet` (mod.rs:644), so a handle deref needs `&RuleSet` at the call.
- **Handle build at the `intern_all_ids` seam.** Add `Simulation::resolve_type_handles(
  &mut self, rules: &RuleSet)` called immediately after `intern_all_ids` at every seam
  (app_init.rs:620 + 4 test sites), mirroring `resolve_bridge_warheads`. — **Confidence:**
  high — **Source:** repo pattern.
- **Call-site migration scope = sim/ layer only this slice; app/ (render/UI) deferred
  to a follow-up (Slice 8b).** Sim/ is the hash-relevant + hot-path surface (42 files);
  app/ resolution is not hashed and is high-churn/low-value. *Flagged for user choice
  at presentation.* — **Confidence:** medium (scope judgment) — **Source:** §8 acceptance
  centers on the handle existing + GameEntity one-hop + hash identity, not a full sweep.

## Open Questions

### Resolved During Planning

- *Is the casing DRIFT still present?* No — fixed in `845ea12b`. Slice 8 reduces to the
  handle-table boundary + Owner/dup-helper cleanup + a confirmation test. (Source: code
  read of intern.rs + ruleset.rs:1724-1759.)
- *Where does the table live / how is a handle dereffed?* On `Simulation`; deref needs
  `&RuleSet` (rules is threaded, not owned). (Source: mod.rs:270-467, 644.)
- *Does any code bypass the accessors with a case-sensitive map get?* No. (grep.)
- *Where is the build seam?* `intern_all_ids` call sites: app_init.rs:620 + 4 tests.

### Deferred to Implementation

- *Does the hash move?* Only observable by running the 5000-tick replay after Task 1+7.
  If it moves, Task 7 investigates which id pair collapsed (do NOT bump version to hide
  it — the acceptance treats drift as a found bug).
- *Exact count of orphan `type_ref`s* (registry ids with no `[section]`): produced by
  the Task 7 completeness test; expected small, logged not failed.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | `TypeHandle`, `object_list`/`object_index`, `type_handle`, `object_by_handle`; rework `from_ini` insert + 5 internal sites |
| Create | `src/sim/type_handle_table.rs` | `TypeHandleTable` (`InternedId→TypeHandle`), build-from-rules |
| Modify | `src/sim/mod.rs` | register `type_handle_table` module |
| Modify | `src/sim/world/mod.rs` | `Simulation.type_handles` field (serde-skip), `resolve_type_handles`, one-hop `object_type` helper, load rebuild |
| Modify | `src/app_init.rs:620` | build table after `intern_all_ids` |
| Modify | `src/sim/production/production_queue_tests.rs` (×4) | build table after `intern_all_ids` |
| Modify | 42 sim/ files (Task 4) | two-hop → `sim.object_type(e.type_ref, rules)` |
| Modify | `src/sim/components.rs:78-80` | delete `Owner(InternedId)` |
| Modify | `src/render/minimap_tests.rs:119` | drop `Owner` usage |
| Modify | `src/app_commands.rs:16-33,*` | inline `intern_owner`/`intern_type` |
| Modify | `src/rules/ruleset.rs` (tests) | casing-confirm + completeness tests |

## Interface Changes

- **New public:** `rules::ruleset::TypeHandle(pub u32)` (Copy/Eq/Hash, serde);
  `RuleSet::type_handle(&str) -> Option<TypeHandle>`; `RuleSet::object_by_handle(
  TypeHandle) -> &ObjectType`. Consumed by `TypeHandleTable` (sim/) and the new
  `Simulation::object_type` helper.
- **New public:** `sim::type_handle_table::TypeHandleTable`;
  `Simulation::resolve_type_handles(&mut self, &RuleSet)`;
  `Simulation::object_type(&self, InternedId, &RuleSet) -> Option<&ObjectType>`.
- **Removed:** `sim::components::Owner` (1 consumer); `app_commands::intern_owner`,
  `app_commands::intern_type` (call sites inlined).
- **Unchanged signature, reworked internals:** `RuleSet::object(&str)` now delegates to
  `type_handle` + `object_by_handle` (still `Option<&ObjectType>`, still CI). All 42+
  existing `rules.object(&str)` callers keep compiling.

## Sim Checklist

- [x] No new floating point — `TypeHandle`/indices are `u32`; no game math added.
- [x] New state NOT hashed — `type_handles` is a derived cache (serde-skip, rebuilt);
      it must NOT enter `state_hash` (it is a function of rules+interner, already hashed
      via type identity). Assert this in Task 7.
- [x] No deps on render/ui/sidebar/audio/net — `type_handle_table.rs` imports only
      `rules::` (one-way) + `sim::intern`.
- [x] Tick ordering unaffected — table is built at init, read-only during ticks.
- [x] BTreeMap iteration order irrelevant — resolution is per-entity by `type_ref`.

## Risk Areas

- **Highest blast radius: Task 1** (RuleSet storage restructure). Mitigation: private
  field, ~5 internal sites, `object()` signature preserved; gate on `cargo test
  -p vera20k --lib` (rules tests) before proceeding.
- **Hash movement: Task 1+7.** If uppercase-key collapse changes resolved identity, the
  replay hash moves. Mitigation: Task 7 runs the replay and, on drift, prints the
  collapsed id pair for investigation BEFORE any version bump (none expected).
- **Build-seam coverage: Task 3.** A test that interns ids but skips
  `resolve_type_handles` would hit an empty table. Mitigation: `object_type` falls back
  to the two-hop when the table is empty/unbuilt (logged once), so no test breaks; the
  4 known test seams get the explicit build call.
- **Migration sweep: Task 4.** 42 files; mechanical but wide. Mitigation: single
  transform recipe + per-subsystem grouping + hash-identity gate after each group.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1, 7 | Case-insensitive name→type resolution | An INI ref `htnk` vs section `[HTNK]` must resolve to the unit; gamemd is CI (`FUN_007C8D20`). A miss = a unit/weapon silently absent every match. | Casing-confirm test (Task 7) + hash-identity replay |
| 1 | Resolved ObjectType identity per name unchanged | Any change to *which* type a name resolves to is player-visible (wrong unit stats). | 5000-tick replay hash bit-identical (Task 7) |
| 7 | `intern_all_ids` completeness | An interned `type_ref` with no resolvable type → an entity with no stats/art. | Completeness test enumerates orphan `type_ref`s |

---

## Tasks

### Task 1: Integer-indexable object storage + `TypeHandle` in `RuleSet`

**Why:** A one-hop deref needs index-addressable object storage. This is the foundation
every later task builds on. Behavior-preserving (resolved identity unchanged).

**Files:**
- Modify: `src/rules/ruleset.rs` (field decl ~1302; `from_ini` insert ~1408-1432;
  `object`/`lookup_ci` ~1724-1739; internal sites 1883, 1992, 2022; test 3462)

**Pattern:** Mirrors `StringInterner` (`to_str: Vec<String>` + `to_id: map`).

**Step 1: Define the handle type** (near the top of `ruleset.rs`, after imports):
```rust
/// O(1) index into `RuleSet`'s object list. Resolved once from a name (or an
/// interned id via the sim `TypeHandleTable`) and then dereferenced directly,
/// avoiding a per-call string round-trip and hash lookup.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct TypeHandle(pub u32);
```

**Step 2: Replace the `objects` field** (ruleset.rs:1301-1302):
```rust
    /// All game objects in registry insertion order. Indexed by `TypeHandle`.
    object_list: Vec<ObjectType>,
    /// Uppercase type ID → handle. Uppercase key gives O(1) case-insensitive
    /// resolution, matching the original engine's case-insensitive find-or-allocate.
    object_index: HashMap<String, TypeHandle>,
```

**Step 3: Rework the `from_ini` build** — replace the `let mut objects: HashMap<...>`
declaration (1392) and the insert loop body (1413-1422). Use a local builder so
existing `collect_weapon_refs(&objects)` keeps working via a view:
```rust
        let mut object_list: Vec<ObjectType> = Vec::new();
        let mut object_index: HashMap<String, TypeHandle> = HashMap::new();
        // ...inside the `for id in &ids` loop, replacing `objects.insert(...)`:
                if let Some(section) = ini.section(id) {
                    let obj: ObjectType = ObjectType::from_ini_section(id, section, category);
                    let key = id.to_ascii_uppercase();
                    // Find-or-allocate: a case-duplicate id reuses its slot (last
                    // definition wins), matching the engine's single-type-per-name.
                    match object_index.get(&key) {
                        Some(&TypeHandle(idx)) => object_list[idx as usize] = obj,
                        None => {
                            let h = TypeHandle(object_list.len() as u32);
                            object_list.push(obj);
                            object_index.insert(key, h);
                        }
                    }
                } else {
                    log::trace!("Object '{}' listed in [{}] but has no section", id, registry_name);
                }
```
Then update `collect_weapon_refs(&objects)` (1435) to iterate the new list — change its
signature to take `&[ObjectType]` and pass `&object_list` (see Step 6). Update the
struct initializer to set `object_list, object_index` instead of `objects`.

**Step 4: Rewrite the accessors** (replace `lookup_ci`/`object`/`object_case_insensitive`,
1724-1739):
```rust
    /// Resolve a type name to its handle, case-insensitively (gamemd parity).
    pub fn type_handle(&self, id: &str) -> Option<TypeHandle> {
        self.object_index.get(&id.to_ascii_uppercase()).copied()
    }

    /// Dereference a handle to its object. Handles only come from this `RuleSet`.
    #[inline]
    pub fn object_by_handle(&self, handle: TypeHandle) -> &ObjectType {
        &self.object_list[handle.0 as usize]
    }

    /// Look up a game object by ID (case-insensitive, gamemd parity).
    pub fn object(&self, id: &str) -> Option<&ObjectType> {
        self.type_handle(id).map(|h| self.object_by_handle(h))
    }

    /// Deprecated alias retained so existing call sites compile; identical to `object`.
    pub fn object_case_insensitive(&self, id: &str) -> Option<&ObjectType> {
        self.object(id)
    }
```
Keep `lookup_ci` only if `weapon`/`warhead`/`projectile` still use it (they do — leave
those maps and `lookup_ci` untouched this slice).

**Step 5: Fix the 3 internal sites:**
- 1883 `self.objects.values_mut()` → `self.object_list.iter_mut()`
- 1992 `self.objects.len()` → `self.object_list.len()`
- 2022 `self.objects.values()` → `self.object_list.iter()`

**Step 6: Fix `collect_weapon_refs`** (fn at ruleset.rs:2078) — change param from
`&HashMap<String, ObjectType>` to `&[ObjectType]` and iterate `.iter()` instead of
`.values()`. Update its one call site (1435) to `&object_list`.

**Step 7: Fix the rules test** (3462 `check_ci(&rules.objects, ...)`) — replace with an
iteration over `rules.object_list` keyed via `object_index`, or delete if it only tested
the now-removed `lookup_ci` scan path (decide by reading 3450-3470).

**Step 8: Verify**
Run: `cargo test -p vera20k --lib rules`  → read the literal `test result:` line.
Expected: PASS (object resolution unchanged).

**Step 9: Commit** — `refactor(rules): index-addressable object storage + TypeHandle (Slice 8)`

---

### Task 2: `TypeHandleTable` in sim/

**Why:** The `InternedId → TypeHandle` table is the one-hop seam. Pure data + a build
function; no Simulation wiring yet (keeps it independently testable).

**Files:**
- Create: `src/sim/type_handle_table.rs`
- Modify: `src/sim/mod.rs` (add `pub mod type_handle_table;`)

**Pattern:** Dense `Vec<Option<_>>` indexed by `InternedId::index()`, mirroring the
interner's `to_str` indexing.

**Step 1: Module**
```rust
//! Precomputed `InternedId → TypeHandle` map for one-hop entity→type resolution.
//!
//! Built once at sim init from a `RuleSet` + the populated `StringInterner`
//! (after `intern_all_ids`). Read-only during ticks; a derived cache, never
//! serialized — rebuilt on load.
//!
//! ## Dependency rules
//! - Part of sim/; depends on rules/ (one-way) + sim::intern. NEVER on
//!   render/ui/sidebar/audio/net.

use crate::rules::ruleset::{RuleSet, TypeHandle};
use crate::sim::intern::{InternedId, StringInterner};

/// Maps an interned type id to its object handle. Dense by `InternedId` index;
/// `None` = interned but no resolvable object (orphan type_ref).
#[derive(Debug, Clone, Default)]
pub struct TypeHandleTable {
    by_interned: Vec<Option<TypeHandle>>,
}

impl TypeHandleTable {
    /// Build from every string currently in the interner. Each id that resolves
    /// to an object (case-insensitively) gets its handle; the rest stay `None`.
    pub fn build(rules: &RuleSet, interner: &StringInterner) -> Self {
        let mut by_interned = Vec::with_capacity(interner.len());
        for idx in 0..interner.len() as u32 {
            // Safe: idx < interner.len(), so resolve() is in-bounds.
            let name = interner.resolve(InternedId::from_index(idx));
            by_interned.push(rules.type_handle(name));
        }
        Self { by_interned }
    }

    /// Resolve an interned id to its handle, if it names an object.
    #[inline]
    pub fn handle_for(&self, id: InternedId) -> Option<TypeHandle> {
        self.by_interned.get(id.index() as usize).copied().flatten()
    }

    /// True if no handles were built (e.g. table not yet resolved).
    pub fn is_empty(&self) -> bool {
        self.by_interned.is_empty()
    }

    /// Count of interned ids that did NOT resolve to an object (orphans).
    pub fn orphan_count(&self) -> usize {
        self.by_interned.iter().filter(|h| h.is_none()).count()
    }
}
```

**Step 2: Add `InternedId::from_index`** (sim/intern.rs, near `index()`):
```rust
    /// Reconstruct an id from a raw index. Only valid for indices this interner produced.
    #[inline]
    pub fn from_index(idx: u32) -> Self {
        Self(idx)
    }
```

**Step 3: Test** (in `type_handle_table.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_for_resolves_case_insensitively() {
        // Build a minimal ruleset with one object, intern a lowercased ref.
        let rules = RuleSet::test_with_object("HTNK"); // helper — see note
        let mut interner = StringInterner::new();
        let id = interner.intern("htnk");
        let table = TypeHandleTable::build(&rules, &interner);
        assert_eq!(table.handle_for(id), rules.type_handle("HTNK"));
        assert_eq!(table.orphan_count(), 0);
    }

    #[test]
    fn unknown_interned_id_is_orphan() {
        let rules = RuleSet::test_with_object("HTNK");
        let mut interner = StringInterner::new();
        let bogus = interner.intern("NOSUCHTYPE");
        let table = TypeHandleTable::build(&rules, &interner);
        assert_eq!(table.handle_for(bogus), None);
        assert_eq!(table.orphan_count(), 1);
    }
}
```
Note: if no `RuleSet::test_with_object` helper exists, add a minimal `#[cfg(test)]`
constructor to `ruleset.rs` that pushes one `ObjectType::from_ini_section` over a tiny
in-memory section, OR build the ruleset from a fixture INI already used by rules tests
(check `ruleset.rs` tests ~3300-3470 for an existing fixture builder and reuse it).

**Step 4: Verify** — `cargo test -p vera20k --lib type_handle_table` → PASS.

**Step 5: Commit** — `feat(sim): TypeHandleTable for one-hop type resolution (Slice 8)`

---

### Task 3: Wire the table into `Simulation` + one-hop resolver + load rebuild

**Why:** Makes the table live and gives callers a single one-hop API. After this, the
two-hop is mechanically replaceable.

**Files:**
- Modify: `src/sim/world/mod.rs` (field ~272; init; resolver; load rebuild)
- Modify: `src/app_init.rs:620`
- Modify: `src/sim/production/production_queue_tests.rs` (4 seams: 28, 101, 171, 506)

**Step 1: Field** (in `pub struct Simulation`, near `interner`, mod.rs:272):
```rust
    /// Derived: interned-id → object handle, rebuilt at init/load. NOT hashed.
    #[serde(skip)]
    pub type_handles: crate::sim::type_handle_table::TypeHandleTable,
```
Add to the constructor initializer (mod.rs:467 area): `type_handles: Default::default(),`.

**Step 2: Build method** (impl Simulation, near other init helpers):
```rust
    /// Build the interned-id→type-handle table. Call once at sim init AFTER
    /// `RuleSet::intern_all_ids`, mirroring `resolve_bridge_warheads`. Idempotent.
    pub fn resolve_type_handles(&mut self, rules: &RuleSet) {
        self.type_handles =
            crate::sim::type_handle_table::TypeHandleTable::build(rules, &self.interner);
    }
```

**Step 3: One-hop resolver** (impl Simulation):
```rust
    /// Resolve an entity's type to its `ObjectType` in one precomputed hop.
    /// Falls back to the name path if the table is unbuilt (test setups that
    /// skip `resolve_type_handles`), so no caller observes a stale empty table.
    #[inline]
    pub fn object_type<'r>(
        &self,
        type_ref: InternedId,
        rules: &'r RuleSet,
    ) -> Option<&'r ObjectType> {
        match self.type_handles.handle_for(type_ref) {
            Some(h) => Some(rules.object_by_handle(h)),
            None if self.type_handles.is_empty() => rules.object(self.interner.resolve(type_ref)),
            None => None,
        }
    }
```
(Ensure `ObjectType` is imported in mod.rs; add `use crate::rules::ruleset::ObjectType;`
if absent.)

**Step 4: Init seam** — at `app_init.rs:620`, after `ruleset.intern_all_ids(&mut sim.interner);`
and before/after `resolve_bridge_warheads`, add:
```rust
        sim.resolve_type_handles(&ruleset);
```

**Step 5: Test seams** — at each of the 4 `rules.intern_all_ids(&mut sim.interner);`
lines in `production_queue_tests.rs`, add immediately after:
```rust
    sim.resolve_type_handles(&rules);
```

**Step 6: Load rebuild** — find `rebuild_caches_after_load` (mod.rs ~1001) and confirm
where rules is available post-load. If load reloads `RuleSet` from INI and re-runs
`intern_all_ids`, add `sim.resolve_type_handles(&rules)` at that same point. If load does
NOT have rules in scope, rely on the Step-3 empty-table fallback and add a doc note that
the table is rebuilt at the next `resolve_type_handles`. (Read the load path before
choosing; do not guess.)

**Step 7: Verify** — `cargo test -p vera20k --lib` → read `test result:`. Expected: PASS.

**Step 8: Commit** — `feat(sim): wire TypeHandleTable into Simulation + one-hop object_type (Slice 8)`

---

### Task 4: Migrate sim/ two-hop call sites to `object_type` (scope: sim only)

**Why:** Delivers the actual one-hop resolution on the hash-relevant + hot-path surface
and removes the per-call `to_ascii_uppercase` String allocation. App/ (render/UI) sites
are deferred to Slice 8b (not hashed, high churn).

**Files:** the 42 sim/ files matching the two-hop (enumerate fresh before editing):
`grep -rln "interner.resolve(.*type_ref)" src/sim/`

**Transform recipe (apply uniformly):**
- Before:
  ```rust
  let obj = rules.object(sim.interner.resolve(entity.type_ref));
  // or: rules.and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
  ```
- After:
  ```rust
  let obj = sim.object_type(entity.type_ref, rules);
  // or, when rules is Option: rules.and_then(|r| sim.object_type(e.type_ref, r))
  ```
- Where the code only needs the **name string** (e.g. art lookup, `is_refinery_type`,
  debug text), LEAVE `interner.resolve(type_ref)` as-is — those are not type→ObjectType
  resolutions and the table doesn't help them.
- Where `self` is the `Simulation` (sim-internal methods), call `self.object_type(...)`;
  borrow-check note: `object_type` takes `&self` and returns a borrow tied to `rules`,
  so it does NOT conflict with later `self.store.get_mut(...)` (the returned ref borrows
  `rules`, not `self`). If a specific site fights the borrow checker, bind the resolved
  fields into locals before the `&mut self` use.

**Execution — do it in subsystem groups, building + hash-checking after each:**
1. movement (`movement/*`) → build+test
2. combat (`combat/*`) → build+test
3. production (`production/*`) → build+test
4. miner/docking (`miner/*`, `docking/*`) → build+test
5. remainder (`world/*`, `aircraft/*`, `superweapon/*`, `ai.rs`, `passenger.rs`,
   `infantry.rs`, `animation.rs`, `slave_miner.rs`, `gate_runtime.rs`,
   `power_system.rs`, `trigger_runtime.rs`, `terrain_*`, `scatter.rs`,
   `bump_crush.rs`) → build+test

**Step N (each group): Verify** — `cargo test -p vera20k --lib` → `test result:` PASS.

**Step (after all groups): Commit** — `refactor(sim): one-hop type resolution via object_type across sim/ (Slice 8)`
(Commit per group if a group is large, to keep diffs reviewable.)

---

### Task 5: Retire `Owner(InternedId)` wrapper

**Why:** Dead wrapper (§7 item 11) with a single test-only consumer.

**Files:**
- Modify: `src/sim/components.rs:78-80` — delete the `Owner` struct + its doc comment.
- Modify: `src/render/minimap_tests.rs:119` — replace `let owner = Owner(test_intern("British"));`
  with the raw id: `let owner = test_intern("British");` and update the downstream use
  (the field it feeds is an `InternedId`; drop the `.0` if any).

**Step 1:** Delete the struct. **Step 2:** Fix the one call site (read minimap_tests.rs
~110-130 to see how `owner` is consumed; it sets `GameEntity.owner: InternedId`).

**Step 3: Verify** — `cargo test -p vera20k --lib` → PASS (and `cargo test -p vera20k`
to catch the render integration test). Read `test result:`.

**Step 4: Commit** — `refactor(sim): remove dead Owner(InternedId) wrapper (Slice 8)`

---

### Task 6: Retire duplicate `intern_owner` / `intern_type` helpers

**Why:** §7 item 11 — thin duplicates of `interner.intern`; inline them.

**Files:**
- Modify: `src/app_commands.rs:16-33` (helper defs) + all call sites (65, 88-89, 112,
  134, 153, 168-169, 243-244, 304, 445-446).

**Step 1:** Decide replacement. Both helpers return `InternedId::default()` when
`state.simulation` is `None`. To preserve that exactly, replace with a single private
helper that keeps the guard but is named for what it does, OR inline at each site:
```rust
// each `intern_owner(state, &owner)` / `intern_type(state, type_id)` becomes:
state.simulation.as_mut().map(|s| s.interner.intern(&owner)).unwrap_or_default()
```
Prefer keeping ONE helper `fn intern_in_sim(state: &mut AppState, s: &str) -> InternedId`
(merging the two identical bodies) and routing all call sites through it — that removes
the *duplication* (the §7 complaint) without scattering the `None` guard across 12 sites.
Delete `intern_owner` and `intern_type`; add `intern_in_sim`; update call sites.

**Step 2: Verify** — `cargo test -p vera20k --lib` + `cargo check -p vera20k`. Expected:
PASS / clean.

**Step 3: Commit** — `refactor(app): collapse duplicate intern helpers (Slice 8)`

---

### Task 7: Acceptance tests — casing confirm, completeness, hash identity

**Why:** The slice is self-checking. These tests encode the §8 acceptance criteria.

**Files:**
- Modify: `src/rules/ruleset.rs` (tests) and/or `src/sim/type_handle_table.rs` (tests).
- Use an existing real-INI-backed test fixture if one exists (check rules tests
  ~3300-3470 and `production_queue_tests.rs` setup for a loaded `RuleSet`).

**Step 1: Casing-confirm + quantify test** (rules tests) — proves `htnk` → `[HTNK]` and
counts how many registry ids would have failed a case-SENSITIVE exact lookup (the
"affected references" quantity the acceptance asks for):
```rust
#[test]
fn type_handle_is_case_insensitive_and_quantifies_casing_refs() {
    let rules = load_test_ruleset(); // real rulesmd-backed fixture
    // htnk vs [HTNK]
    assert_eq!(rules.type_handle("htnk"), rules.type_handle("HTNK"));
    assert!(rules.object("htnk").is_some(), "lowercased ref must resolve");
    // Quantify: ids whose lowercased form != stored uppercase key — i.e. refs that a
    // case-sensitive exact map.get() would have missed. Surfaced, not hidden.
    let affected = rules
        .object_ids_for_test() // add a small test accessor returning &[String] of section ids
        .iter()
        .filter(|id| rules.type_handle(&id.to_ascii_lowercase()).is_some())
        .count();
    println!("casing-affected references resolvable CI: {affected}");
    assert!(affected > 0);
}
```

**Step 2: `intern_all_ids` completeness test** (sim or rules test) — no orphan
`type_ref` beyond known section-less registry ids:
```rust
#[test]
fn intern_all_ids_has_no_unexpected_orphans() {
    let rules = load_test_ruleset();
    let mut interner = StringInterner::new();
    rules.intern_all_ids(&mut interner);
    let table = TypeHandleTable::build(&rules, &interner);
    // Orphans are registry ids listed without a [section]; print them for audit.
    let orphans = table.orphan_count();
    println!("orphan type_refs (registry id without section): {orphans}");
    // Assert the table covers every id that DOES have a section.
    for id in rules.object_ids_for_test() {
        assert!(rules.type_handle(id).is_some(), "section id {id} must have a handle");
    }
}
```

**Step 3: Hash-identity replay** — run the existing 5000-tick replay/world-hash test
(world_hash / world_tests). Confirm the recorded golden hash is **unchanged**.
```
cargo test -p vera20k --lib world_hash
cargo test -p vera20k --lib -- replay   # or the project's replay-hash test name
```
- If the hash is **identical** → acceptance met.
- If the hash **moved** → DO NOT bump `SNAPSHOT_VERSION` to hide it. Add a temporary
  debug print in Task 1's `from_ini` that logs any uppercase-key collision
  (`object_index.get(&key).is_some()` on insert), re-run map load, and report which two
  ids collapsed. That pair is the missing-type the acceptance warns about — investigate
  and report to the user before proceeding.

**Step 4: Assert table not hashed** — grep `world_hash.rs` to confirm `type_handles` is
absent from the hash inputs; add a one-line comment at the field that it is derived/not
hashed.

**Step 5: Verify** — `cargo test -p vera20k --lib` full → read `test result:`. Expected:
PASS, hash unchanged.

**Step 6: Commit** — `test(sim): Slice 8 casing-confirm, completeness, hash-identity (Slice 8)`

---

### Task 8: Final sweep + clippy + slice close-out

**Why:** Confirm the whole slice is green and the boundary is the single resolution path.

**Step 1:** `cargo test -p vera20k` (full, incl. integration) → read `test result:`.
**Step 2:** `cargo clippy -p vera20k` → no new warnings on touched files.
**Step 3:** Grep `src/sim/` for any remaining `rules.object(*.interner.resolve(*type_ref))`
two-hops; confirm only intentional name-string uses remain.
**Step 4:** Confirm no `Owner(` / `intern_owner` / `intern_type` references remain
(`grep -rn`).
**Step 5: Verify** the design §8 boxes: (a) `type_handle` CI ✓, (b) `TypeHandleTable` at
`intern_all_ids` ✓, (c) GameEntity one-hop ✓, (d) Owner + dup helpers retired ✓,
(e) hash identical ✓, (f) casing test ✓, (g) completeness ✓.
**Step 6: Commit** — `chore(sim): close ObjectSubstrate Slice 8 (type-resolution boundary)`

---

## Sources & References

- **Design doc:** `docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md` §6, §7.11, §8,
  C13/C14, critic #8 (resolved).
- **gamemd.exe (pre-accepted, not re-decompiled):** `FUN_007C8D20` — shared
  case-insensitive find-or-allocate name matcher (OR-0x20 fold).
- **Repo code:** `src/sim/intern.rs` (StringInterner, already CI);
  `src/rules/ruleset.rs:1300-1759` (RuleSet storage + accessors, `lookup_ci`,
  `intern_all_ids`, `resolve_bridge_warheads`); `src/sim/game_entity.rs:176`
  (`type_ref`); `src/sim/world/mod.rs:270-467,644` (Simulation owns interner, rules
  threaded); `src/app_init.rs:620-626` (init seam);
  `src/sim/components.rs:78-80` (`Owner`); `src/app_commands.rs:16-33` (dup helpers).
- **Prior commits:** `845ea12b` (CI lookup + insert-side merge — makes casing already
  correct); `81977280` (Slice 1b — EntityStore into ObjectSubstrate);
  `012d7926` (Slice 2 Presence); `47d78ef0` (Slice 6 Dying window).
- **INI keys:** none (internal boundary refactor).
