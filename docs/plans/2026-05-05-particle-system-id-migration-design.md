# Particle-System Reference Typing Migration — Design

## Goal

Replace every `Option<String>` / `Vec<String>` field in `rules/` that points at a
`ParticleSystemType` with a typed `Option<ParticleSystemTypeId>` /
`Vec<ParticleSystemTypeId>`, resolved at parse time inside `RuleSet::from_ini`. After
this lands, no consumer holds an unresolved particle-system name string.

## Architecture Context

The `[Particles]` and `[ParticleSystems]` sections are already loaded by a working
2-pass resolver in [`src/rules/ruleset.rs`](../../src/rules/ruleset.rs):

- Pass 1 calls `from_ini_section_pending` on each registered section, producing a
  `PendingParticleType` / `PendingParticleSystemType` that captures the type with
  its cross-reference left as `None` plus the unresolved reference string.
- Pass 2 builds an uppercase `HashMap<String, *Id>` and walks each pending entry,
  setting the `*Id` slot via map lookup or warning + leaving `None` on miss.

Outputs are stored as `Vec<ParticleType>` / `Vec<ParticleSystemType>` (index =
`*Id.0`) plus the corresponding `*_by_name` lookup map. `RuleSet::p_type_id_by_name`
and `RuleSet::ps_type_id_by_name` provide case-insensitive lookups.

The fields targeted by this migration are all consumer-side references that today
live as raw strings:

- `WeaponType::attached_particle_system: Option<String>`
- `ObjectType::natural_particle_system: Option<String>`
- `ObjectType::refinery_smoke_particle_system: Option<String>`
- `ObjectType::damage_particle_systems: Vec<String>`
- `ObjectType::destroy_particle_systems: Vec<String>`
- `GeneralRules::barrel_particle: Option<String>`
- `CombatDamageDefaults::default_*_system` (9 fields)

Current parse order in `RuleSet::from_ini`: `GeneralRules` → object loop → weapons
→ warheads → projectiles → super_weapons → `CombatDamage` → particles →
particle_systems. Every consumer parses *before* the particle registry exists,
which is why these fields were left as deferred strings in commits 0101a64,
b92d107, and ac3faae.

**Live consumers of any of the 14 in-scope fields: zero.** Every read site is in
`#[cfg(test)]` code. This migration is a data-layer reshape ahead of future
consumers, not a behavior change.

## Impact Analysis

**New type, inlined in `src/rules/ruleset.rs`:**

```rust
pub struct RuleParseCtx<'a> {
    pub ps_by_name: &'a HashMap<String, ParticleSystemTypeId>,
}
```

A `RuleParseCtx::empty()` constructor backed by a `LazyLock<HashMap<…>>` returns a
context that resolves every name to `None` with no allocation per call. Future
migrations (warheads, weapons, projectiles by ID) will add fields to this struct
without churning every call site again.

Resolution policy on the context:

- `resolve_ps(name)` returns `Some(id)` on case-insensitive hit.
- On miss with a non-empty `ps_by_name`: log a warning at `warn!` level identifying
  the consumer (`owner` + `key`) and the unresolved name, return `None`. Matches
  the existing `parse_particle_types` / `parse_particle_system_types` policy.
- On miss with `ps_by_name.is_empty()` (i.e., `RuleParseCtx::empty()`): silent
  `None`. Tests that pass an empty context don't generate log noise.

**Parse-order reshuffle in `RuleSet::from_ini`:** move the particle 2-pass to run
before any consumer parser. New order:

1. `production`, `terrain_rules`, `bridge_rules`, `garrison_rules`, `radar_event_config`
   (none reference particles — keep where they are).
2. **Particles** (`parse_particle_types`).
3. **Particle systems** (`parse_particle_system_types`).
4. Build `RuleParseCtx { ps_by_name: &particle_system_types_by_name }`.
5. `general` (now takes `&ctx`).
6. Object registry loop (now passes `&ctx`).
7. Weapons, warheads, projectiles, super_weapons (weapons take `&ctx`; warheads,
   projectiles, super_weapons unchanged).
8. `CombatDamage` (now takes `&ctx`).

**Field-type changes** (struct definitions):

| File | Field | Old | New |
|---|---|---|---|
| `src/rules/weapon_type.rs` | `attached_particle_system` | `Option<String>` | `Option<ParticleSystemTypeId>` |
| `src/rules/object_type.rs` | `natural_particle_system` | `Option<String>` | `Option<ParticleSystemTypeId>` |
| `src/rules/object_type.rs` | `refinery_smoke_particle_system` | `Option<String>` | `Option<ParticleSystemTypeId>` |
| `src/rules/object_type.rs` | `damage_particle_systems` | `Vec<String>` | `Vec<ParticleSystemTypeId>` |
| `src/rules/object_type.rs` | `destroy_particle_systems` | `Vec<String>` | `Vec<ParticleSystemTypeId>` |
| `src/rules/ruleset.rs` (GeneralRules) | `barrel_particle` | `Option<String>` | `Option<ParticleSystemTypeId>` |
| `src/rules/combat_damage.rs` | 9 `default_*_system` fields | `Option<String>` | `Option<ParticleSystemTypeId>` |

**Signature changes** (one new `&RuleParseCtx<'_>` argument each):

- `ObjectType::from_ini_section(id, section, category, ctx)`
- `WeaponType::from_ini_section(id, section, ctx)`
- `GeneralRules::from_ini(ini, ctx)`
- `CombatDamageDefaults::from_ini_section(section, ctx)`

**Call-site fan-out:**

| Constructor | Production | Tests | Total |
|---|---|---|---|
| `ObjectType::from_ini_section` | 1 | ~21 | ~22 |
| `WeaponType::from_ini_section` | 1 | 6 | 7 |
| `GeneralRules::from_ini` | 1 | 8 | 9 |
| `CombatDamageDefaults::from_ini_section` | 1 | 4 | 5 |
| **Total** | **4** | **~39** | **~43** |

Test sites that don't care about resolution add one line:
`let ctx = RuleParseCtx::empty();` and pass `&ctx`.

**Two `ObjectType` literal builders** are affected only by struct-shape change
(no signature impact):

- `src/sim/movement/locomotor_tests.rs:12` — set the four ObjectType particle
  fields to `None` / `Vec::new()` (now typed).
- `src/sim/movement/teleport_movement.rs:247` — same.

**Tests that change semantics** (must rewrite assertions, not just plumbing):

- `techno_type_parses_damage_particle_systems_csv` (`object_type.rs:1324`) —
  rewrite to build a small `RuleSet` containing `BigGreySSys` and `SmallGreySSys`
  in `[ParticleSystems]`, parse end-to-end, then assert the resolved IDs match
  what `rs.ps_type_id_by_name(...)` returns.
- `barrel_particle_parsed_from_general` (`ruleset.rs:1931`) — same shape change.
- `parses_full_combat_damage_section` (`combat_damage.rs:76`) — same shape change.
- `combat_damage_defaults_load_from_ini` (`ruleset.rs:2087`) — already integration
  test; assertions become ID-based.
- "default / None" tests (`empty_section_yields_all_none`, `barrel_particle_default_none`,
  `whitespace_only_value_treated_as_none`, etc.) stay structurally the same — the
  result is still `None`.

**Risk areas:**

1. **Missed call sites in unscanned files.** Mitigation: rely on the compiler.
   Once the signature changes, every miss is a build error.
2. **Determinism.** Parse-time `HashMap` lookup is deterministic for fixed inputs.
   No `HashMap` iteration is introduced in any sim path. Lockstep contract
   unaffected.
3. **INI ordering corner cases.** Already handled by the existing 2-pass within
   particles. Consumers run *after* particles in the new order, so they cannot
   miss a forward-declared system.

**Architectural fit:** clean. `RuleParseCtx` lives entirely in `rules/`. No new
cross-module dependencies. Follows the same "resolve at parse time, consumers
hold IDs" pattern already proven by the in-place 2-pass.

**Parity fit:** zero risk to observable behavior. No live runtime consumer reads
any of these fields today. The migration sets up the data layer for future
consumers (refinery smoke, damage smoke spawning, gap-generator wiring) to
look up by ID instead of by string.

## Chosen Approach

Reorder `RuleSet::from_ini` so the particle 2-pass runs first, then thread a
`RuleParseCtx<'a>` (variant **A2** from the brainstorm) into the four affected
`from_ini_section` / `from_ini` constructors. Each consumer resolves names to
`ParticleSystemTypeId` at parse time using the context, with a **B3** warning
policy (warn on miss when the registry is non-empty; silent on miss when the
registry is empty, to keep test logs clean).

**Why this approach over the alternatives:**

- Versus a third-pass post-resolution sweep (Option B from the brainstorm): the
  consumer structs would need to hold either a `Pending*` wrapper or a companion
  `pending_*: Vec<String>` shadow field, doubling the data layer. The Pending
  pattern in `rules/` exists because particles cross-reference each *other* —
  applying it to consumers is over-applying.
- Versus a bare `&HashMap<String, ParticleSystemTypeId>` argument (variant A1):
  same call-site cost today, but absorbs the next migration (e.g., warheads → `WarheadTypeId`)
  by adding a field to `RuleParseCtx` instead of churning every call site again.

## Design

### Components

**`RuleParseCtx<'a>`** — new struct in `src/rules/ruleset.rs`:

```rust
use std::collections::HashMap;
use std::sync::LazyLock;

static EMPTY_PS_MAP: LazyLock<HashMap<String, ParticleSystemTypeId>> =
    LazyLock::new(HashMap::new);

pub struct RuleParseCtx<'a> {
    pub ps_by_name: &'a HashMap<String, ParticleSystemTypeId>,
}

impl<'a> RuleParseCtx<'a> {
    pub fn empty() -> RuleParseCtx<'static> {
        RuleParseCtx { ps_by_name: &EMPTY_PS_MAP }
    }

    pub fn resolve_ps(&self, name: &str, owner: &str, key: &str)
        -> Option<ParticleSystemTypeId>
    {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            return None;
        }
        let upper = trimmed.to_ascii_uppercase();
        match self.ps_by_name.get(&upper) {
            Some(&id) => Some(id),
            None => {
                if !self.ps_by_name.is_empty() {
                    log::warn!(
                        "{}.{}: '{}' references unknown particle system, leaving unresolved",
                        owner, key, trimmed
                    );
                }
                None
            }
        }
    }

    pub fn resolve_ps_csv(&self, raw: Option<&str>, owner: &str, key: &str)
        -> Vec<ParticleSystemTypeId>
    {
        let Some(s) = raw else { return Vec::new() };
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("none"))
            .filter_map(|t| self.resolve_ps(t, owner, key))
            .collect()
    }
}
```

Notes:
- `resolve_ps_csv` drops unresolved entries from the resulting `Vec` (consistent
  with the warn-and-skip policy). Order of resolved entries is preserved.
- `owner` / `key` are `&str` (cheap) and feed only the warn message; they're not
  stored. Consumers pass static string literals like `"GAREFN"` and
  `"DamageParticleSystems"`.

### Data Flow

```
[Particles]               [ParticleSystems]
     ↓                          ↓
parse_particle_types →   parse_particle_system_types
     ↓                          ↓
  by_name map P            by_name map PS
                              ↓
                         RuleParseCtx { ps_by_name: &PS }
                              ↓
                       ┌──────┼──────┬────────┐
                       ↓      ↓      ↓        ↓
                  GeneralRules  Object   Weapon  CombatDamage
                                  ↓        ↓        ↓
                               typed IDs in struct fields
```

### Interfaces / Contracts

After this lands, every consumer field that previously held a particle-system
name string holds either `Option<ParticleSystemTypeId>` or
`Vec<ParticleSystemTypeId>`. Lookups against `RuleSet::particle_system_type(id)`
return the corresponding `&ParticleSystemType`. Future runtime consumers (combat
damage, refinery, gap generator) follow the established pattern: store `*Id`,
look up the type via `RuleSet`.

`RuleSet::ps_type_id_by_name` and `RuleSet::p_type_id_by_name` stay public —
they remain useful for tests, ad-hoc lookups, and future code paths that
resolve a name discovered at runtime (e.g., from a save game that stores the
old name format).

### Error Handling

- Empty / `none` / whitespace-only values: `None` / dropped from `Vec`. No log.
- Non-empty value with missing registry entry, non-empty registry: `warn!` with
  consumer + key + name, then `None` / dropped. Matches existing 2-pass policy.
- Empty registry (test-only `RuleParseCtx::empty()`): silent `None` /
  dropped. No log noise in unit tests.

No panics. No `Result`. The contract is that `from_ini_section` always succeeds
even with a partially populated INI — same as today.

### Testing Strategy

**Compiler-level coverage:** the four signature changes force every call site
to be visited. After the migration, the codebase compiles only when each call
site is updated. This is the strongest guarantee that nothing is missed.

**Unit-test rewrites:**

- `techno_type_parses_damage_particle_systems_csv`: rewrite to feed a tiny
  ruleset (`[ParticleSystems]\nBigGreySSys\nSmallGreySSys\n` plus minimal
  sections), parse full `RuleSet::from_ini`, assert
  `obj.damage_particle_systems == vec![big_id, small_id]` where
  `big_id = rs.ps_type_id_by_name("BigGreySSys").unwrap()`.
- `barrel_particle_parsed_from_general`: rewrite analogously — full `RuleSet::from_ini`
  with `[ParticleSystems]` containing `SmallGreySSys`, assert
  `general.barrel_particle == rs.ps_type_id_by_name("SmallGreySSys")`.
- `parses_full_combat_damage_section`: rewrite to call through
  `RuleSet::from_ini` (since `RuleParseCtx` requires registry context anyway).
  Or keep as a per-section test using `RuleParseCtx::empty()` — assertions all
  become `is_none()` since the empty context resolves nothing. Splitting one
  test into "shape via empty ctx" and "resolution via full ruleset" is also fine.

**New test — empty-ctx silence:** add a test that calls `from_ini_section` with
`RuleParseCtx::empty()` on a section with `BarrelParticle=Foo` and asserts the
field is `None`. Then capture log output (via a test logger or `log::set_logger`
hook) and assert no `WARN` lines fire. Optional but cheap.

**New test — warn-on-miss with non-empty ctx:** parse a `RuleSet` whose
`[General]` section has `BarrelParticle=NonExistent` but `[ParticleSystems]`
contains other entries; assert `general.barrel_particle == None` and the test
logger captured a warning matching `BarrelParticle.*NonExistent`.

**Integration check against retail INI:** add a test that loads
`ini/rulesmd.ini` through `RuleSet::from_ini` and asserts that no `WARN` lines
about unresolved particle-system references fire. This catches typos in the
parser keys (`BarrelParticle` vs `BarrelParticles`) and any retail particle
system the registry forgot. If a similar integration test already exists, fold
the assertion into it; otherwise add a small `#[ignore]`-able one.

### Determinism Considerations

`HashMap<String, ParticleSystemTypeId>` lookup is deterministic for fixed
inputs. The new `RuleParseCtx` is built once per `RuleSet::from_ini` call and
discarded. No `HashMap` iteration occurs anywhere in `sim/`. State hashing is
unaffected — it observes the post-parse `RuleSet`, which now contains
`ParticleSystemTypeId` values (a `u32`, fully deterministic) rather than name
strings (also deterministic, but less compact).

## Architectural Decisions

**Patterns followed:**

- "Resolve at parse time, consumers hold IDs" — same pattern as the existing
  `ParticleType.next_particle: Option<ParticleTypeId>` and
  `ParticleSystemType.holds_what: Option<ParticleTypeId>`.
- Case-insensitive name resolution via uppercase keys in the `_by_name` map —
  same as `RuleSet::ps_type_id_by_name`.
- Warn-and-leave-unresolved on miss — same as `parse_particle_types`.

**Patterns deviated from:** none.

**Tech debt introduced:** none. The `RuleParseCtx` struct is a single-field
holder today; it's designed to grow into the multi-field "context bag" pattern
as future migrations land (warheads → `WarheadTypeId`, projectiles →
`ProjectileTypeId`, etc.). Each future migration extends `RuleParseCtx` and
swaps the relevant string fields without churning call sites again.

## Alternatives Considered

**Bare `&HashMap<String, ParticleSystemTypeId>` argument (A1):** identical
diff size today, but every future cross-ref migration adds another argument to
every constructor. Rejected to avoid that recurring churn.

**Tiny `ParticleSystemResolver` trait + `NoOp` impl (A3):** solves the test
ergonomics nicely, but adds an abstraction layer for a problem `RuleParseCtx::empty()`
solves with no abstraction. Rejected as overkill.

**Third resolution pass (Option B):** keep `from_ini_section` signatures as-is,
add a sweep after the particle 2-pass that resolves stored strings into IDs.
Forces consumer structs to hold either `Pending*` wrappers (data-layer
duplication) or shadow `Vec<String>` companion fields (struct pollution).
Rejected because consumers don't have the chicken-and-egg cycle that justified
the Pending pattern for particles themselves — they're just downstream.

**Inline particle parse at the top of `RuleSet::from_ini` without a context
struct, then mutate consumer structs in place after parsing:** smaller signature
change, but means consumer structs need a mutable post-parse step *and* must
hold the unresolved name somewhere temporarily. Strictly worse than A2.
