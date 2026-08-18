# Death Special-Producer Shadow Design

**Date:** 2026-07-23  
**Status:** proposed for approval  
**Authority:** shadow-only; live death AoE, HP, lifecycle, RNG, snapshots, and hashes remain unchanged

## Goal

Introduce a typed, transient death special-producer trace which can represent the
verified native gate, weapon-selection, signed-damage, provenance, and ordered
legacy target observation without turning the existing batched death-AoE shortcut
into authority.

## Architecture Context

Current combat has two distinct migration surfaces:

1. `sim/combat/damage/object_vitality.rs` models the signed Object vitality
   transaction after damage normalization.
2. `sim/combat/mod.rs::handle_entity_deaths` approximates native death weapons by
   selecting an explicit weapon or `Primary`, batching blasts after the dead-object
   loop, precomputing unsigned AoE damage, and subtracting HP directly.

`sim/entity_state` owns persistent-per-entity shadow representations and
legacy-versus-exact state-write diagnostics. A death weapon is not an entity-state
family or a separate native HP writer. It is a producer operation which eventually
enters normal target receivers. Producer diagnostics therefore belong beside the
damage behavior, not inside `EntityStateShadow`.

Rules already parse type `Explodes`, `DeathWeapon`, veterancy abilities, and weapon
`Suicide`. The exact type modifier and `[CombatDamage]` default death weapon are
missing. The exact native virtual fallback at vslot `+0x3F4` and current-weapon
number are not represented by current Rust and must remain explicit
`Uncomparable` facts.

## Impact Analysis

- `src/rules/object_type.rs`
  - add exact `DeathWeaponDamageModifier` storage;
  - default to native `1.0f`;
  - parse through the verified leading-f32 INI semantics and retain raw f32 bits.
- `src/rules/damage_rules.rs`
  - add unresolved `[CombatDamage] DeathWeapon`;
  - default to null when the section/key is absent;
  - remove `Copy` from `DamageRules` if required by owned string storage.
- `src/sim/combat/damage/death_producer.rs`
  - new pure producer value model and transition;
  - no `EntityStore`, lifecycle, RNG, or output-sink reach-in.
- `src/sim/combat/damage/mod.rs`
  - expose the new crate-private module.
- `src/sim/combat/mod.rs`
  - build one producer observation per legacy death-AoE candidate;
  - retain current source position and identity;
  - attach ordered legacy AoE target IDs after the existing query;
  - return transient producer diagnostics through `DeathEffects` and
    `CombatTickResult`;
  - do not replace `death_weapon_aoe`, `apply_aoe_damage`, or the direct HP write.
- focused rules, pure-service, and combat integration tests.

Risk is diagnostic misrepresentation, not live gameplay mutation. The principal
guard is conservative classification: any missing native input produces a named
`Uncomparable` reason and never a fabricated exact value.

The added diagnostic vectors remain transient. They are not serialized, hashed,
or consumed by gameplay.

## Chosen Approach

Use a dedicated typed producer model in `sim/combat/damage`, with a separate
`DeathProducerShadowDiagnostic` returned from combat.

The model accepts caller-built value facts. It can compute native selection and
signed damage only when every required input is present. The current adapter
supplies exact type/veterancy fields, explicit weapon facts, Rules default facts,
source identity, and raw Rust position. It supplies `Missing` for the native
current-weapon Suicide fact and native virtual fallback unless a future verified
adapter can provide them.

The diagnostic contains both:

- the legacy operation actually observed by current Rust; and
- the native-intent candidate or named reasons why comparison is impossible.

The legacy path remains untouched and authoritative.

## Tiny-Detail Ledger

- The Techno helper gate is type `Explodes`, active-tier `Explodes` ability, or
  current weapon `Suicide`; explicit `DeathWeapon` alone is not a gate.
  [doc: `DEATH_WEAPON_WRITER_CLASSIFICATION_AND_SHADOW_BOUNDARY_GHIDRA_REPORT.md`
  §§3.1,5.1]
- Fly zero-HP crash is a separate active-YR invocation route; dormant Tunnel
  locomotion is not an active-YR route.
  [same doc §3.2-3.3]
- Selection order is explicit type `DeathWeapon`, native virtual fallback
  `+0x3F4`, then Rules default; the virtual fallback's semantic Rust identity is
  unresolved and must not be guessed as `Primary`.
  [same doc §§5.1,11 OQ19]
- The type modifier is native f32 with constructor default exact `1.0f`.
  [same doc §4]
- Explicit/virtual damage is `Math__ftol(i32 weapon damage * f32 modifier)`.
  [same doc §5.2]
- Rules-default damage is `Math__ftol(i32 Strength * double 0.5)` and does not
  read `DefaultDeathWeapon.Damage`.
  [same doc §§4,5.2]
- The helper then adds a signed i32 addend; all active discovered callers pass
  zero.
  [same doc §5.2]
- Null selected weapon produces no bullet; allocation failure produces no
  detonation. The current shadow does not allocate a bullet.
  [same doc §5.3]
- The source object is the dying object; source house is its current owner at
  nested detonation.
  [same doc §6.1]
- Native detonation uses the dying object's exact vslot `+0x48` coordinate, not a
  synthesized cell center.
  [same doc §§5.3,6.1]
- Native area collection precedes receiver mutation, and target records execute
  synchronously in collection order.
  [same doc §§6.2-6.3]
- Every target receives fresh signed i32 storage. Zero exits area dispatch;
  negative nonzero values stay signed.
  [same doc §6.3]
- A fatal target's complete nested death chain finishes before the outer
  dispatcher advances to its next target.
  [same doc §6.4]
- Techno local order is passenger cleanup, death helper/nested chain, attached
  bomb, fatal postlude. Fly calls the helper before crash `UnInit`.
  [same doc §7]
- Current Rust's batched timing, unsigned target damage, cell-center query, and
  missing source receiver argument remain DRIFT and cannot be labelled equal by
  this shadow.
  [same doc §8]
- Producer diagnostics are observation-only: no RNG, bullet allocation,
  lifecycle, HP, snapshot, or hash mutation.
  [same doc §9]

## Design

### Components

#### Exact rule state

`ObjectType` gains:

```text
death_weapon_damage_modifier: NativeF32Bits
```

The parser uses the project's verified leading-f32 numeric path and stores the
result's exact `f32::to_bits()`. Missing/invalid values retain native exact
`1.0f` according to the typed INI accessor's verified default behavior.

`DamageRules` gains:

```text
default_death_weapon: Option<String>
```

It reads `[CombatDamage] DeathWeapon`, trims the value, treats absent/empty as
null, and does not resolve or hardcode `DefaultDeathWeapon`.

#### Pure producer service

`sim/combat/damage/death_producer.rs` owns value types for:

- invocation evidence;
- gate reason;
- exact-or-missing current-weapon Suicide fact;
- explicit/virtual/Rules weapon facts;
- selection path;
- modifier bits and Strength;
- signed helper addend;
- selected weapon/projectile/warhead identities;
- signed computed damage;
- unsupported/non-finite arithmetic.

It uses the existing software-x87 service for:

```text
ftol(load_i32(weapon_damage) * load_f32(modifier_bits))
ftol(load_i32(strength) * load_f64(0.5))
```

The result is `i32` only when the verified x87 result fits the native dword.
Any unsupported x87 input/result is `Uncomparable`, never a lossy cast.

The service has no Rust entity or rules lookup. Callers resolve names and provide
facts.

#### Producer diagnostics

A separate transient diagnostic records:

```text
tick
legacy_sequence
source_id
source_owner
raw Rust position (cell, height level, sub-cell fixed bits)
legacy selected weapon/warhead/damage
legacy ordered target IDs
native gate evidence
native selection/damage candidate when complete
uncomparable reason set
```

Named reasons include at minimum:

- `LegacyBatchedInvocationTiming`
- `MissingCurrentWeaponSuicide`
- `MissingVirtualFallback`
- `InvocationRouteUnknown`
- `CoordinateAdapterUnverified`
- `SelectedWeaponMissing`
- `SelectedProjectileMissing`
- `SelectedWarheadMissing`
- `NativeArithmeticUnsupported`
- `LegacyUnsignedTargetDamage`
- `LegacyBypassesNormalReceiver`
- `LegacyRecursionAbsent`

Reasons are diagnostic data, not logs. Multiple reasons may coexist.

#### Combat adapter

`handle_entity_deaths` creates the diagnostic adjacent to the existing
`death_weapon_aoe` lookup so the observation has the same deterministic
`dead_entities` order as the legacy producer.

When the existing `apply_aoe_damage` returns, the adapter attaches the returned
target IDs in their exact legacy vector order before current direct HP writes run.
It does not claim this is native order.

`DeathEffects` carries the producer diagnostic vector to `CombatTickResult`.
The world ignores it just as gameplay ignores current vitality shadow
diagnostics.

### Interfaces / Contracts

The pure producer transition follows these rules:

1. A proven Fly invocation bypasses the Techno gate.
2. A proven Techno invocation evaluates exact type/active-tier ability facts.
3. If those facts do not trigger and current-weapon Suicide is missing, the gate
   is `Uncomparable`, not false.
4. If the gate triggers, explicit weapon wins when present.
5. Without explicit weapon, missing virtual fallback prevents selection. The
   service does not jump directly to Rules default.
6. A proven-null virtual fallback permits Rules-default selection.
7. Null final weapon means no detonation candidate.
8. Explicit/virtual damage and Rules-default damage use their separate verified
   x87 formulas.
9. The signed addend is applied after `ftol` with native wrapping dword semantics
   only if that wrapping has been positively verified; otherwise overflow is
   `Uncomparable`.

The current combat adapter cannot prove native invocation timing, Fly versus
Techno origin, virtual fallback, current weapon, exact CoordStruct conversion,
normal receiver entry, or recursion. Those omissions are always visible in the
reason set.

### Data Flow

1. Parse exact modifier/default rules without changing consumers.
2. For each legacy dead entity, snapshot source identity, owner, type facts,
   veterancy tier, and raw position.
3. Resolve explicit weapon and Rules-default weapon facts.
4. Supply missing current-weapon and virtual-fallback facts.
5. Run the pure producer transition.
6. Run existing legacy `death_weapon_aoe` unchanged.
7. Record the legacy weapon/warhead/damage observation.
8. Run existing legacy AoE query unchanged.
9. Record returned legacy target IDs in order.
10. Run existing direct HP writes and all side effects unchanged.
11. Return transient diagnostics.

### Error Handling

- No production unwraps.
- Missing rules/type/weapon/projectile/warhead values produce named diagnostic
  reasons.
- Unsupported native-x87 values produce `NativeArithmeticUnsupported`.
- Missing entity rows produce no producer diagnostic because the legacy
  producer did not execute.
- Diagnostics never become a fallback behavior source.

### Testing Strategy

#### Parser tests

- modifier absent -> exact `1.0f` bits;
- `.1`, `.5`, `.01`, explicit `1.0`, and leading-f32 syntax retain expected bits;
- Rules default absent/empty -> null;
- Rules default present -> trimmed unresolved name;
- stock `rulesmd.ini` resolves `DefaultDeathWeapon` and known modifiers.

#### Pure-service tests

- explicit weapon wins and uses modifier/x87 truncation;
- explicit pointer does not independently satisfy Techno gate;
- veteran and elite `Explodes` gates;
- missing current-weapon fact prevents a false negative;
- missing virtual fallback prevents an unjustified Rules fallback;
- proven-null virtual fallback uses half Strength and ignores default weapon
  `Damage`;
- null Rules weapon produces no detonation candidate;
- signed negative and zero damage remain signed;
- x87 unsupported values are uncomparable;
- dormant Tunnel is not representable as active-YR invocation.

#### Integration tests

- live legacy death-AoE HP and lifecycle outputs remain unchanged;
- one transient producer diagnostic per existing legacy death-AoE candidate;
- source, owner, raw sub-cell bits, and legacy weapon facts are recorded;
- legacy target IDs retain `apply_aoe_damage` order;
- diagnostics contain batched timing, receiver-bypass, unsigned-damage, and
  recursion-missing reasons;
- explicit modifier candidate is computed but overall operation remains
  uncomparable because of current adapter timing;
- a non-explicit legacy fallback reports missing virtual fallback rather than
  claiming `Primary` parity;
- diagnostics are absent from snapshots and world hashes.

## Architectural Decisions

- Producer diagnostics are separate from `EntityStateShadow`; this preserves the
  existing distinction between behavior operations and state representations.
- The pure model follows the caller-built value-type pattern already used by
  `object_vitality`.
- Missing native facts are first-class types. `Option` is used for proved
  nullable state; a separate exact-or-missing enum is used where `None` and
  “not represented” have different meanings.
- Current live code is deliberately duplicated at the observation seam rather
  than refactored through the shadow model. Sharing it now could make incomplete
  shadow semantics authoritative.
- Raw Rust sub-cell bits are recorded without claiming the current `Position`
  shape is already an exact native CoordStruct adapter.

No persistent state, serializer version, snapshot field, hash input, RNG draw,
lifecycle call, or cross-layer dependency is introduced.

## Alternatives Considered

### Extend generic vitality diagnostics

Rejected because a producer decision is not an entity-state write. Combining the
two would make status rows unable to distinguish “wrong operation existed” from
“right operation wrote different vitality.”

### Share one enriched record between live and shadow paths

Rejected for this stage because unresolved current-weapon, virtual-fallback,
coordinate, receiver, and recursion facts would leak into live authority or force
the shared record to preserve known-wrong legacy semantics.

### Log-only instrumentation

Rejected because logs are difficult to assert, aggregate, and carry across staged
comparison work. Typed transient rows are deterministic and testable while
remaining outside gameplay state.
