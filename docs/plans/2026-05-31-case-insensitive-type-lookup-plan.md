# Case-Insensitive Type-Name Lookup — Parity Fix Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Small, contained to `rules/`.

**Goal:** Make `RuleSet` type-name lookups (`object`/`weapon`/`warhead`/`projectile`/`super_weapon`)
case-insensitive so they match gamemd, which resolves type names case-insensitively.

**Architecture:** `rules/` is a low-level data layer consumed one-way by `sim/`/`render/`. This
change is entirely inside `src/rules/ruleset.rs` — it alters lookup *behavior* of existing
accessors, not their signatures, so the ~77 call sites are untouched.

**Design Doc:** [docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md](../research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md) — C13 / Slice 8 (parity core only; the TypeHandleTable/dual-id refactor is explicitly out of scope).

---

## Grounding Summary

- **gamemd (verified-from-binary, this session):** type-name matching is **case-insensitive**.
  `FUN_007C8D20` (`decompile_function 0x007C8D20`) is a `_stricmp`-style compare — when two bytes
  differ it lowercases both (the `+0xBF / +0x20 / +0x41` fold) before deciding. `BuildingTypeClass::
  FindOrAllocate` (`decompile_function 0x004653C0`) loops `g_BuildingTypeClass_Array` comparing the
  requested name against each type's name field (`+0x24`) through that comparator; the same
  comparator is wired into `AircraftTypeClass`/`AnimTypeClass`/etc. `FindOrAllocate`
  (`get_function_callers 0x007C8D20`). **Verified-from-binary.**
- **Rust (verified, current tree):** `object`/`weapon`/`warhead`/`projectile`/`super_weapon` are
  plain `HashMap<String, T>::get(id)` — exact-key, **case-sensitive** (ruleset.rs:1670, 1689, 1694,
  1699, 1716). All five name maps are `HashMap<String, T>` (ruleset.rs:1302–1351).
- **Existing patterns to mirror:** `object_case_insensitive` (ruleset.rs:1675) does
  `.get(id).or_else(|| .iter().find_map(|(k,v)| k.eq_ignore_ascii_case(id).then_some(v)))`;
  `country_multiplay_passive` (1704) uses the same fold; `terrain_object_type_case_insensitive`
  (1684) instead normalizes keys to uppercase at load (`.get(&name.to_ascii_uppercase())`). The
  codebase is currently **inconsistent** — this fix standardizes the type accessors on the fold.
- **INI:** no keys involved — pure lookup-behavior change.
- **Unknown after grounding:** exact caller count of `object_case_insensitive` (Task 2 greps it).

## Key Technical Decisions

- **Approach: per-call case-fold, applied in-place to the primary accessors** (not normalize-keys-
  at-load). **Confidence:** high — **Source:** existing `object_case_insensitive`/`country_multiplay_passive`
  pattern (ruleset.rs:1675, 1704) + verified gamemd behavior.
  - *Why over normalize-at-load:* the fold is contained to the accessor bodies in `ruleset.rs` and
    leaves stored keys untouched — so nothing that iterates `objects.keys()`/derives `factory_map`
    from object keys (ruleset.rs:1483)/etc. can break. Normalize-at-load would force an audit of
    every key-iteration + every insert site (much larger blast radius) for an identical *observable*
    result. Both produce gamemd's output; the fold is lower-risk.
  - *Perf:* exact `.get(id)` stays O(1) and is the normal path (RA2 IDs are consistently cased), so
    hot-path combat/movement lookups are unaffected. The O(n) scan only runs on a case-*mismatch*
    miss — i.e. exactly the previously-broken case being fixed.
- **Determinism (hard requirement):** the fold's scan is deterministic **iff** no two stored keys are
  equal-ignoring-case. Valid RA2 INI guarantees this — gamemd's own case-insensitive `FindOrAllocate`
  merges case-duplicate names, so two type names differing only by case cannot exist. The existing
  `object_case_insensitive` (already sim-reachable) relies on the same invariant; this fix does not
  introduce a *new* determinism risk. **Confidence:** high — **Source:** gamemd `FindOrAllocate`
  semantics (verified). A `debug_assert`-level load guard makes the invariant explicit (Task 4).

## Open Questions

### Resolved During Planning
- *Fold vs normalize-keys?* → Fold (contained, no key-iteration risk). See Key Decisions.
- *Does it change call sites?* → No. Accessor signatures unchanged; ~77 callers untouched.
- *Insert-side parity (gamemd merges case-dup names on insert)?* → Out of scope: RA2 INI has no
  case-duplicate type names, so insert-merge is **unobservable**. Documented, not implemented.

### Deferred to Implementation
- Whether `object_case_insensitive` is removed or kept as an alias depends on its caller count
  (Task 2 measures it; ≤3 callers → remove + redirect to `object`, else keep as a thin alias).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | add `lookup_ci` helper; make 5 accessors case-insensitive; reconcile `object_case_insensitive`; add tests |

## Interface Changes

- **Behavior change (not signature):** `object`/`weapon`/`warhead`/`projectile`/`super_weapon` now
  resolve case-insensitively. Same `fn(&self, &str) -> Option<&T>` signatures → no caller edits.
- **Possible removal:** `object_case_insensitive` (becomes redundant once `object` folds). If kept,
  it's a deprecated alias. Callers (Task 2) either redirect to `object` or keep compiling via the alias.

## Sim Checklist

- [x] No new f32/f64 — pure lookup logic, no math.
- [x] State-hash impact: **expected none.** Hash changes only if a sim lookup that currently *misses*
      on case would now *hit* — that's the bug being fixed; if it happens, investigate the call site
      (don't paper over). Asserted in Task 5.
- [x] No deps on render/ui/sidebar/audio/net — change is in `rules/`.
- [x] Tick ordering: unaffected.
- [x] Determinism: fold is deterministic under the no-case-collision invariant (Key Decisions + Task 4 guard).

## Risk Areas

- **Determinism** is the only real risk — covered by the no-case-collision invariant + the Task 4
  debug guard. If the guard ever fires, that INI has duplicate-by-case names (a data error gamemd
  would have merged) and must be looked at, not silently tolerated.
- **Masked bugs:** if making a lookup case-insensitive flips a result somewhere, a test/hash will
  move — that surfaces a real prior case-mismatch bug (the point of the fix). Task 5 checks for it.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 1 | Case-insensitive type-name resolution | gamemd matches names via `_stricmp` (`FUN_007C8D20`); a case-mismatch lookup that gamemd resolves but we return `None` for = a missing unit/weapon/effect in-game | Verified vs `0x007C8D20` + `0x004653C0`; Task 3 regression test (`htnk`/`HTNK`/`Htnk` all resolve) |

---

## Tasks

### Task 1: Add the `lookup_ci` helper and route the 5 accessors through it

**Why:** One audited case-insensitive lookup primitive, reused by every type accessor — mirrors the
existing `object_case_insensitive` fold so behavior is consistent and gamemd-matching.

**Files:**
- Modify: `src/rules/ruleset.rs` (add helper near the accessors ~1670; edit accessors at 1670, 1689, 1694, 1699, 1716)

**Pattern:** Mirrors `object_case_insensitive` (ruleset.rs:1675) and `country_multiplay_passive` (1704).

**Step 1 — Add the helper** (associated fn on `impl RuleSet`, placed just above `object` at ~1670):
```rust
/// Case-insensitive type-name lookup matching gamemd's `FindOrAllocate` (stricmp-style).
///
/// Exact match first — O(1) and the normal path, since RA2 type IDs are consistently
/// cased. Only on a case-mismatch miss does it scan for the unique case-insensitive
/// match. Valid RA2 data never holds two names equal-ignoring-case (the original engine's
/// case-insensitive find-or-allocate merges them), so the scan yields at most one hit and
/// the result stays deterministic for lockstep.
fn lookup_ci<'a, T>(map: &'a std::collections::HashMap<String, T>, id: &str) -> Option<&'a T> {
    map.get(id).or_else(|| {
        map.iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case(id).then_some(value))
    })
}
```
*(If `HashMap` is already imported in `ruleset.rs`, use the bare `HashMap<String, T>` form to match
file style.)*

**Step 2 — Route the five accessors through it.** Replace each body:
```rust
pub fn object(&self, id: &str) -> Option<&ObjectType> {
    Self::lookup_ci(&self.objects, id)
}

pub fn weapon(&self, id: &str) -> Option<&WeaponType> {
    Self::lookup_ci(&self.weapons, id)
}

pub fn warhead(&self, id: &str) -> Option<&WarheadType> {
    Self::lookup_ci(&self.warheads, id)
}

pub fn projectile(&self, id: &str) -> Option<&ProjectileType> {
    Self::lookup_ci(&self.projectiles, id)
}

pub fn super_weapon(&self, id: &str) -> Option<&SuperWeaponType> {
    Self::lookup_ci(&self.super_weapons, id)
}
```

**Step 3 — Verify compile.** Run: `cargo check -p vera20k`. Expected: clean.

### Task 2: Reconcile the now-redundant `object_case_insensitive`

**Why:** With `object` folding, `object_case_insensitive` (ruleset.rs:1675) duplicates it. Avoid two
ways to do the same thing.

**Files:**
- Modify: `src/rules/ruleset.rs` (and any callers found)

**Step 1 — Count callers.** Run: `rg -n "object_case_insensitive" src` (exclude the definition).

**Step 2 — Decide and apply:**
- If **≤3 call sites:** delete `object_case_insensitive` (ruleset.rs:1675–1681) and replace each call
  with `.object(...)`.
- If **>3 call sites:** keep it but make it a thin alias to avoid churn:
  ```rust
  /// Deprecated: `object` is now case-insensitive. Retained as an alias.
  pub fn object_case_insensitive(&self, id: &str) -> Option<&ObjectType> {
      self.object(id)
  }
  ```

**Step 3 — Verify compile.** Run: `cargo check -p vera20k`. Expected: clean.

### Task 3: Regression tests

**Why:** Lock in the parity behavior and prove all five accessors resolve regardless of case.

**Files:**
- Modify: `src/rules/ruleset.rs` (extend or add `#[cfg(test)] mod tests`)

**Step 1 — Add a test** that builds a minimal `RuleSet` from an in-memory INI defining one of each
type with a known-cased name, then asserts lookups succeed in lower/UPPER/Mixed case. Mirror the
existing ruleset test setup (find it via `rg -n "fn .*ruleset.*test|RuleSet::from" src/rules/ruleset.rs`
and reuse its INI fixture builder). Example shape:
```rust
#[test]
fn type_lookups_are_case_insensitive() {
    let rules = /* build RuleSet from the smallest INI fixture used by existing tests
                   that defines at least one [object], [weapon], [warhead], [projectile],
                   and a superweapon — reuse the existing fixture helper */;

    // ObjectType keyed as "HTNK" in INI resolves under any casing.
    assert!(rules.object("HTNK").is_some());
    assert!(rules.object("htnk").is_some(), "lowercase must resolve (gamemd parity)");
    assert!(rules.object("Htnk").is_some(), "mixed case must resolve");
    assert_eq!(
        rules.object("htnk").map(|o| o as *const _),
        rules.object("HTNK").map(|o| o as *const _),
        "all casings resolve to the same object",
    );

    // Same property for the other four accessors, using whatever names the fixture defines.
    // (weapon / warhead / projectile / super_weapon)
}
```
*(Use the real type names the chosen fixture defines; if no single fixture covers all five, write one
focused test per accessor reusing that accessor's existing test fixture.)*

**Step 2 — Run:** `cargo test -p vera20k -- ruleset` (or the module path the tests live in).
Expected: PASS.

### Task 4: Add the no-case-collision determinism guard

**Why:** Make the lockstep-safety invariant explicit — the fold is deterministic only if no two stored
keys are equal-ignoring-case. Fires in debug builds if violated; release is unaffected.

**Files:**
- Modify: `src/rules/ruleset.rs` (in `RuleSet` construction, after the five maps are populated — near ruleset.rs:1535+ where building finishes)

**Step 1 — Add a debug-only check** after the maps are built (before `RuleSet { ... }` is returned):
```rust
#[cfg(debug_assertions)]
{
    // Lockstep invariant: lookup_ci's scan is deterministic only if names are unique
    // ignoring case. gamemd's case-insensitive FindOrAllocate guarantees this for valid
    // data; assert it so a malformed INI surfaces loudly instead of desyncing silently.
    let check_unique_ci = |label: &str, keys: Vec<&String>| {
        let mut lowered: Vec<String> = keys.iter().map(|k| k.to_ascii_lowercase()).collect();
        lowered.sort();
        for pair in lowered.windows(2) {
            debug_assert_ne!(
                pair[0], pair[1],
                "{label}: type names collide ignoring case ({:?}) — breaks deterministic lookup",
                pair[0]
            );
        }
    };
    check_unique_ci("objects", objects.keys().collect());
    check_unique_ci("weapons", weapons.keys().collect());
    check_unique_ci("warheads", warheads.keys().collect());
    check_unique_ci("projectiles", projectiles.keys().collect());
    check_unique_ci("super_weapons", super_weapons.keys().collect());
}
```
*(Place it where all five local maps are still in scope as locals, before they move into the struct.
If `super_weapons` is built later than the others, move the check after the last one.)*

**Step 2 — Run:** `cargo test -p vera20k -- ruleset` and a real map-load test if one exists
(`rg -n "fn .*load.*map|from_ini" src/rules`). Expected: PASS, guard does not fire on stock rules.

### Task 5: Full verification + commit

**Why:** Confirm no behavior drift beyond the intended fix, and that the state hash is stable.

**Step 1 — Targeted tests:** `cargo test -p vera20k -- ruleset` → PASS (read the literal `test result:` line).

**Step 2 — Full suite:** `cargo test -p vera20k`. Expected: same pass count as before this change
(baseline was **3388 passed / 0 failed**, plus the new case-insensitive tests). If any *previously
passing* test now fails or the count of a hash-sensitive test shifts, **stop and investigate** — it
means a real lookup was relying on the old case-sensitive miss (the bug). Do not paper over it.

**Step 3 — Confirm no stray case-sensitive type lookups remain.** Run:
`rg -n "\.(objects|weapons|warheads|projectiles|super_weapons)\.get\(" src` — every direct `.get(`
on these maps outside `lookup_ci` should be reviewed; route any name-by-`&str` ones through the
accessor. (Internal exact-key uses with an already-normalized key may stay — note them.)

**Step 4 — Commit.** `fix(rules): case-insensitive type-name lookups to match gamemd`

## Sources & References

- **Design doc:** docs/research/ABSTRACT_OBJECT_SUBSTRATE_SERVICE_DESIGN.md (C13, Slice 8, §10).
- **gamemd.exe (verified this session):** `FUN_007C8D20` (case-insensitive comparator,
  `decompile_function 0x007C8D20`); `BuildingTypeClass::FindOrAllocate 0x004653C0` (uses it on name
  field `+0x24`, `decompile_function 0x004653C0`); shared across TypeClass FindOrAllocate
  (`get_function_callers 0x007C8D20`).
- **Current Rust:** src/rules/ruleset.rs — accessors 1670/1689/1694/1699/1716; maps 1302–1351;
  existing fold `object_case_insensitive` 1675, `country_multiplay_passive` 1704; uppercase-key
  variant `terrain_object_type_case_insensitive` 1684; map construction ~1392–1535.
- **INI keys:** none.
