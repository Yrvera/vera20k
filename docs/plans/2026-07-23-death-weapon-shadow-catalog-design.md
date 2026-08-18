# Death-Weapon Shadow Catalog Design

**Date:** 2026-07-23  
**Status:** approved  
**Authority:** diagnostic-only; no live rules or combat authority

## Goal

Resolve explicit type `DeathWeapon` and `[CombatDamage] DeathWeapon` definitions
for the staged death-producer shadow without expanding the live weapon registry
or changing legacy death-AoE behavior.

## Architecture Context

`RuleSet::from_ini` receives the merged base-plus-YR rules view. It first parses
object types, then builds its live weapon registry from the ordinary object
weapon references returned by `collect_weapon_refs`. That collector currently
includes `Primary`, `Secondary`, elite weapons, and occupy weapons, but not
`DeathWeapon` or the Rules default death weapon.

The omission means `RuleSet::weapon` cannot resolve a weapon referenced only by
death-special state. Adding those names to `collect_weapon_refs` would not be
observation-only: current `death_weapon_aoe` uses `RuleSet::weapon`, so expanding
the live registry could activate legacy damage behavior before the coordinated
authority flip.

The approved death-producer shadow already keeps native-intent facts separate
from the legacy operation. Its combat adapter currently uses the live lookup for
both paths, causing valid death-only weapon definitions to appear as
`SelectedWeaponMissing`. Rules-side diagnostic resolution is therefore the next
prerequisite.

## Impact Analysis

- `src/rules/death_weapon_shadow.rs`
  - owns a diagnostic-only catalog of parsed death-weapon definitions;
  - collects explicit object and Rules-default references;
  - provides case-insensitive lookup without exposing mutable state.
- `src/rules/mod.rs`
  - declares the new rules module.
- `src/rules/ruleset.rs`
  - builds the catalog from the merged `IniFile`;
  - stores it separately from the live `weapons`, `warheads`, and `projectiles`
    maps;
  - exposes a crate-visible diagnostic lookup.
- `src/sim/combat/mod.rs`
  - resolves native-intent explicit/default weapon facts through the shadow
    catalog;
  - continues resolving the legacy death AoE through `RuleSet::weapon`.
- focused rules and combat tests.

The main risk is authority leakage. The catalog must not be returned by
`RuleSet::weapon`, included in live warhead/projectile discovery, intern names,
or become a fallback for gameplay callers.

The catalog is immutable rules data. It is not simulation state, is not
serialized into snapshots, does not participate in world hashes, and consumes
no RNG.

## Chosen Approach

Add a dedicated `DeathWeaponShadowCatalog` in `rules/`.

At `RuleSet` construction time, collect names from:

1. every parsed `ObjectType::death_weapon`;
2. `DamageRules::default_death_weapon`, when non-null.

For each unique case-insensitive reference, look up the section in the merged
`IniFile` and parse it through the existing `WeaponType::from_ini_section`.
Store it under the section's canonical header name. Missing sections are not
fabricated; the combat adapter retains `SelectedWeaponMissing`.

The catalog exposes lookup only to crate-internal diagnostic code. The existing
live registry and its lookup remain unchanged.

## Tiny-Detail Ledger

- Native selection order remains explicit type weapon, virtual fallback, then
  Rules default. Catalog availability must not bypass the unresolved virtual
  fallback.  
  [doc: `DEATH_WEAPON_WRITER_CLASSIFICATION_AND_SHADOW_BOUNDARY_GHIDRA_REPORT.md`
  §5.1]
- An explicit `DeathWeapon` is selection state, not a Techno invocation gate.
  Catalog resolution must not make it a gate.  
  [same doc §§3.1,8.2]
- `[CombatDamage] DeathWeapon` has native null constructor state. Absent or empty
  input adds no catalog reference.  
  [same doc §4]
- The catalog reads the already-merged base-plus-`rulesmd.ini` view; YR overrides
  remain authoritative over base values.  
  [AGENTS.md Sources Of Truth]
- Section lookup is case-insensitive and the stored identity uses the canonical
  section header, matching the existing rules-registry convention.  
  [Rust: `IniFile::section`, `RuleSet::from_ini` weapon parsing]
- Explicit/virtual damage uses the selected weapon's signed `Damage` and the
  dying type's f32 modifier. Catalog parsing must preserve the existing
  `WeaponType` signed integer value.  
  [same research doc §5.2]
- Rules-default damage ignores the default weapon's `Damage` and remains
  `Math__ftol(Strength * 0.5)`. Catalog resolution provides identity,
  projectile, and warhead facts only for that branch.  
  [same research doc §§4,5.2]
- Missing weapon sections remain `SelectedWeaponMissing`; no guessed or default
  `WeaponType` is created. This is a conservative Rust shadow classification,
  not a claim about unverified native loader allocation behavior.  
  [current shadow contract; native missing-section behavior UNCHECKED]
- Missing `Projectile` and `Warhead` fields retain their existing named
  diagnostic reasons. The catalog does not promote their registries or schedule
  projectiles.  
  [same research doc §5.3; approved producer-shadow design]
- A native-intent candidate with no current legacy producer records
  `legacy: None`, `legacy_sequence: None`, and `LegacyProducerAbsent`; absence
  is never represented by a fabricated zero-damage legacy operation.  
  [approved shadow-catalog boundary]
- The native virtual fallback at vslot `+0x3F4` remains missing. A catalog entry
  must never be treated as proof that `Primary` is this fallback.  
  [same research doc §§5.1,11 OQ19]
- Live `RuleSet::weapon`, live warhead/projectile maps, legacy
  `death_weapon_aoe`, target ordering, HP writes, lifecycle, RNG, snapshots, and
  hashes remain byte-for-byte on their current path.  
  [approved producer-shadow design]
- Catalog construction is deterministic and lookup-only. Hash-map iteration
  order must not enter diagnostic order; producer diagnostics retain the
  existing ordered dead-entity traversal.  
  [Rust: `handle_entity_deaths`; AGENTS.md determinism rule]
- Diagnostic-only weapon identities stay owned strings and do not mutate the
  simulation interner.  
  [approved producer-shadow design]
- Active standard-YR scope remains the Techno fatal and Fly crash routes; the
  dormant Tunnel route gains no implementation surface.  
  [same research doc §3]

## Design

### Components

#### `DeathWeaponShadowCatalog`

The rules-side catalog owns parsed `WeaponType` values in a private
case-insensitive map. It does not own warhead or projectile definitions because
the current pure producer boundary needs only their selected identities and
field-null state.

Its constructor accepts:

- the merged `IniFile`;
- parsed object types;
- the unresolved Rules default death-weapon name.

Its lookup accepts an arbitrary-cased weapon ID and returns an immutable
`WeaponType` reference when the referenced section existed.

#### `RuleSet`

`RuleSet` stores the catalog in a field distinct from `weapons`. A crate-visible
method such as `death_weapon_shadow(&self, id)` is deliberately named to prevent
accidental use as a general gameplay fallback.

Construction occurs after objects and `DamageRules` are available. It does not
insert catalog weapon warheads into the live `warheads` set or catalog
projectiles into the live `projectiles` set.

#### Combat adapter

The native-intent `resolve_death_weapon` helper uses
`RuleSet::death_weapon_shadow`. The existing `death_weapon_aoe` continues using
`RuleSet::weapon`.

This deliberately permits the same name to be:

- resolvable for diagnostic native facts; and
- unresolved for current legacy gameplay.

That asymmetry is the purpose of the staged migration.

For a dead type with explicit death state or an already represented Explodes
gate, the adapter may record a native-intent row even when the live lookup
produces no legacy AoE. Such a row has no pending AoE entry and therefore cannot
reach live target collection or HP writes.

### Interfaces / Contracts

```text
DeathWeaponShadowCatalog::from_rules(
    ini,
    objects,
    default_death_weapon,
) -> DeathWeaponShadowCatalog

DeathWeaponShadowCatalog::weapon(id) -> Option<&WeaponType>

RuleSet::death_weapon_shadow(id) -> Option<&WeaponType>
```

All interfaces are read-only after rules construction. No public API is added
outside the crate.

### Data Flow

1. Merge base rules and YR overrides before `RuleSet::from_ini`.
2. Parse object types and `DamageRules`.
3. Collect explicit and global-default death-weapon names.
4. Deduplicate names case-insensitively.
5. Resolve each section through `IniFile::section`.
6. Parse existing sections with `WeaponType::from_ini_section`.
7. Store canonical names in the separate catalog.
8. During a death observation, resolve native explicit/default facts through
   the catalog.
9. Resolve and execute the legacy candidate through the unchanged live
   registry.
10. Record any difference only in transient diagnostics.

### Error Handling

- Missing or empty references are skipped.
- A referenced missing section is absent from the catalog and becomes
  `SelectedWeaponMissing` in the producer evaluation.
- A present weapon section with null projectile or warhead retains those null
  fields so the producer emits the existing typed reason.
- No production unwraps.
- No fallback to `Primary`, a guessed virtual weapon, or a fabricated default
  definition.

### Testing Strategy

Rules tests:

- a weapon referenced only by object `DeathWeapon` resolves through the shadow
  catalog but not through `RuleSet::weapon`;
- the `[CombatDamage]` default resolves through the catalog but not the live
  registry;
- case variants collapse to the canonical section;
- missing/empty/default-missing references remain absent;
- stock `DefaultDeathWeapon` and representative stock explicit death-only
  weapons resolve from the merged rules view.

Combat tests:

- an explicit death-only definition supplies a native-intent candidate while
  the legacy lookup remains unchanged;
- a missing definition retains `SelectedWeaponMissing`;
- non-explicit legacy `Primary` fallback still reports
  `MissingVirtualFallback`;
- diagnostics retain existing source and target order;
- live HP and lifecycle outputs remain unchanged.

Validation:

- focused catalog and producer tests;
- `cargo check -q`;
- `git diff --check`;
- inspect that no new call site uses the catalog outside diagnostic resolution.

## Architectural Decisions

The design follows the existing rule that `rules/` parses immutable game data
and `sim/` consumes it. It avoids retaining the full raw INI in `RuleSet` and
avoids runtime parsing or simulation allocations.

The intentional deviation is duplicate parsing for a small subset of weapon
sections already present in the live registry. This duplication creates a hard
authority boundary and is preferable during shadow migration. It can be removed
only at the coordinated authority flip, when the live registry is proven to
contain the exact native reference set and consumers have migrated.

No permanent gameplay architecture is inferred from the native pointer
registry. Rust owns immutable parsed values; the shadow preserves only the
verified reference and selection semantics.

## Alternatives Considered

### Retain the merged `IniFile` and resolve on demand

Rejected because it retains broad parser state, introduces runtime parsing and
allocation concerns, and couples simulation diagnostics to raw INI storage.

### Expand the live registry and filter consumers

Rejected for this stage because `RuleSet::weapon` already feeds legacy gameplay.
One missed filter could activate death damage before the authority flip. A
shared superset plus visibility classes also adds more migration machinery than
the small diagnostic catalog.

### Add death references directly to `collect_weapon_refs`

Rejected because it immediately changes the live lookup surface and therefore
is not shadow-only.
