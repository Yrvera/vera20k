# Infantry Fear/Prone/Crawls Runtime - Design

## Goal

Implement full infantry fear/prone parity for normal play:

- infantry under fire gains fear;
- fearful infantry drops prone and later recovers;
- prone infantry uses the `Crawls=` speed branch;
- prone damage uses a real sim stance bit instead of animation inference;
- prone, crawl, down, up, and fire-prone animation choices follow sim state.

The immediate player-visible target is GI parity, but the system must be generic
for all infantry because the original code is `InfantryClass`/`InfantryTypeClass`
behavior, not a GI special case.

## Grounding

Verified binary findings from the GI investigation:

- `InfantryTypeClass::ReadINI` parses `Crawls=` from art metadata into
  InfantryType offset `+0xEBD`. GI has `Crawls=yes` in `artmd.ini [GI]`.
- `Fearless=` is InfantryType offset `+0xEBC`; `Fraidycat=` is `+0xEBF`.
- `InfantryClass::Panic_SetFear300` sets fear to 300 unless the type is
  `Fearless=yes` or the unit has the veteran FEARLESS ability.
- `InfantryClass::SetFear` has two paths:
  - first hit: fraidycat immediately reaches 300; normal non-fearless infantry
    reaches 100;
  - later hits when fear is already above 99, or when no damager is supplied:
    add 50, 25, or 12 based on health and `ConditionRed`/`ConditionYellow`,
    clamped to 300.
- `InfantryClass::Fear_Decay_Handler` decrements fear first, then triggers
  Down when post-decay fear is at least 50, and triggers Up when post-decay
  fear is below 50.
- `InfantryClass::GetMovementSpeed` applies the prone speed branch only when the
  runtime prone bit is set:
  - `Crawls=yes`: speed becomes ceiling two thirds;
  - `Crawls=no`: speed becomes `speed + speed / 2`.

Repo facts:

- `src/sim/game_entity.rs` has no infantry fear/prone runtime state today.
- `src/sim/combat/mod.rs` and `src/sim/combat/combat_aoe.rs` currently use
  `animation_is_prone` as a temporary proxy for prone damage.
- `src/sim/animation.rs` already has `Prone`, `Crawl`, `Down`, `Up`,
  `FireProne`, `Panic`, and `SecondaryProne` sequence kinds.
- `src/sim/movement/movement_tick.rs` already computes deterministic current
  speed and effective cell speed; this is the right insertion point for the
  prone speed multiplier.
- `src/rules/object_type.rs` parses rule-side infantry flags but does not yet
  expose `Fearless`, `Fraidycat`, or `Crawls`.
- `src/rules/art_data.rs` does not yet parse `Crawls=`, and
  `RuleSet::merge_art_data` currently merges art data only for buildings.

## Architecture Context

This feature belongs entirely below render/audio/UI:

- `rules/` parses and merges data:
  - `ObjectType::fearless`
  - `ObjectType::fraidycat`
  - `ObjectType::crawls`
  - `ArtEntry::crawls`
- `sim/` owns runtime state:
  - `InfantryRuntime { fear_level, is_prone }`
  - fear application on damage
  - fear decay and prone transitions during the tick
  - prone speed and prone damage predicates
- `animation` reflects sim state and keeps transition sequences from being
  overwritten too early.
- `movement` reads a deterministic multiplier from the infantry runtime.
- `combat` writes fear when damage actually lands and reads the sim prone bit
  when scaling damage.

No render/audio/UI dependency is needed.

## Chosen Approach

Approach A: add a sim-owned infantry runtime and make combat, movement, and
animation consume it.

This matches the original structure most closely: gamemd has a fear value and a
separate prone byte on `InfantryClass`; animation is not the source of truth.

Rejected approaches:

- GI-only behavior: faster, but it would immediately fail for Conscripts,
  civilians, Guardian GIs, and modded infantry that use the same original
  system.
- Animation-driven stance: this is the current temporary hook. It cannot match
  gamemd because damage and speed use the runtime prone bit immediately after
  `Do_Action(Down)`/`Do_Action(Up)`, not after visual animation inference.

## Design

### Rules Data

Add these fields to `ObjectType`:

```rust
pub fearless: bool,
pub fraidycat: bool,
pub crawls: bool,
```

Parse `Fearless=` and `Fraidycat=` from rules INI in
`ObjectType::from_ini_section`.

Add `crawls: bool` to `ArtEntry`, parsed from `Crawls=`. Extend
`RuleSet::merge_art_data` so it handles infantry as well as buildings:

- resolve art metadata using the same `Image=`/object-id convention;
- for infantry, copy `entry.crawls` into `obj.crawls`;
- preserve existing building-only foundation/dock merge behavior.

### Runtime State

Add a small sim-owned infantry runtime to `GameEntity`:

```rust
pub struct InfantryRuntime {
    pub fear_level: u16,
    pub is_prone: bool,
}
```

Only infantry entities should have it initialized. Non-infantry entities keep
`None`. The field needs serde defaults consistent with the existing entity
state pattern.

`is_prone` is intentionally separate from animation. In gamemd, Down sets the
prone byte and Up clears it as part of the action request; speed and damage read
that byte directly.

### Infantry Module

Add a focused `src/sim/infantry.rs` or `src/sim/infantry/mod.rs` module with
pure deterministic helpers:

- `apply_fear_from_damage`
- `apply_panic_force`
- `tick_fear_decay_and_prone`
- `is_prone_for_damage`
- `prone_speed_multiplier`

Constants:

- max fear: 300
- first normal hit: 100
- first fraidycat hit: 300
- repeated-hit base add: 50
- decay order: decrement first, then evaluate transitions
- Down threshold: post-decay `fear_level >= 50`
- Up threshold: post-decay `fear_level < 50`

Fear application must check both type `fearless` and veteran FEARLESS ability.
If the current veteran ability model lacks FEARLESS, add a narrow query/helper
that returns false until that ability is implemented rather than hardcoding a
gameplay shortcut into the fear code.

Fear decay is a separate gate. Gamemd's decay handler checks only type
`Fearless=yes` for the decrement; it does not call the veteran FEARLESS ability
query there. Keep application and decay helpers separate so existing fear is
handled like the binary.

### Tick Order

`Simulation::advance_tick` currently runs:

1. commands
2. ground/special movement
3. vision/power/superweapons
4. deploy state
5. combat
6. post-combat effects and animation

Add infantry fear decay/prone transitions after deploy state and before combat.
That is the closest available slot to gamemd's AI/update sequence: prone state
is stable before combat reads it, and deployed infantry can block automatic
stand/prone changes before firing.

Damage-caused fear is applied in combat when damage is actually applied to the
target. That means a unit damaged this tick can drop prone on the next fear tick,
matching the existing staged combat pipeline without requiring a separate
mid-combat mutation pass.

### Combat Integration

Replace `animation_is_prone` use in both direct-fire and AOE damage with
`infantry::is_prone_for_damage(entity)`.

When Phase 4 in `combat::tick_combat_with_fog` subtracts health and records
`last_attacker_id`, also call `apply_fear_from_damage` for live infantry that
actually took nonzero damage. Invulnerability-nullified damage should still set
`last_attacker_id`, but should not increase fear because no damage lands.

### Movement Integration

After current speed and terrain speed are combined in
`movement_tick.rs`, multiply by the prone speed factor when:

- entity category is infantry;
- `InfantryRuntime::is_prone` is true;
- the `ObjectType` can be resolved.

Use integer/fixed-point math only:

- `Crawls=yes`: ceiling two-thirds, equivalent to `ceil(speed * 2 / 3)` for
  positive integer speed values;
- `Crawls=no`: `speed + speed / 2`, matching the binary's integer truncation.

### Animation Integration

Animation should consume runtime state:

- `Down` starts when the fear tick flips `is_prone` true.
- `Up` starts when the fear tick flips `is_prone` false.
- moving while prone uses `Crawl`.
- idle while prone uses `Prone`.
- attacking while prone uses `FireProne` or `SecondaryProne` as appropriate.

Existing transition sequences must be protected so normal stand/walk/attack
selection does not immediately overwrite `Down` or `Up` before they complete.
Deployed/deploying/undeploying sequences remain higher priority.

### Fraidycat Flee

`Fraidycat=yes` is less important for stock GI but is part of full infantry
parity. Implement it as part of the generic module if the existing garrison
entry/building search APIs are sufficient:

- when fear is high and the unit is not deployed, sleeping, or in an active
  locomotor transition, pick a valid nearby enterable building;
- issue the same movement/enter intent used by normal garrison entry.

If the existing APIs cannot issue that order without a larger mission-system
change, land the core fear/prone/Crawls behavior first and leave fraidycat flee
as the only separately documented follow-up in the implementation plan.

## Impact Analysis

Likely files:

- `src/rules/art_data.rs`
- `src/rules/ruleset.rs`
- `src/rules/object_type.rs`
- `src/sim/game_entity.rs`
- `src/sim/mod.rs`
- `src/sim/infantry.rs` or `src/sim/infantry/mod.rs`
- `src/sim/world/mod.rs`
- `src/sim/combat/mod.rs`
- `src/sim/combat/combat_aoe.rs`
- `src/sim/movement/movement_tick.rs`
- `src/sim/animation.rs`
- `src/sim/world/world_hash.rs`
- targeted tests under `src/rules`, `src/sim`, `src/sim/combat`, and
  `src/sim/movement`

Main risks:

- tick order: prone state must be visible to combat after fear decay, while
  damage-induced fear should not mutate stance in the middle of a damage batch;
- off-by-one thresholds: binary behavior is Down at post-decay 50 and Up below
  post-decay 50;
- deployed infantry: panic should not visually or logically undeploy GI;
- current temporary combat tests that force prone via animation need to move to
  explicit runtime state;
- art merge expansion must not alter building foundation/dock behavior;
- new `InfantryRuntime` fields must be included in deterministic state hashing,
  or lockstep/replay divergence can be hidden until later combat or movement
  differs.

## Verification

Minimum verification:

- rules parser tests for `Fearless`, `Fraidycat`, and art `Crawls`;
- pure fear math tests for first hit, repeated hits, clamping, fearless blocking,
  fraidycat, panic force, decay, and Down/Up threshold edges;
- combat tests proving prone damage uses `InfantryRuntime::is_prone`, not
  animation;
- AOE prone damage equivalent test;
- movement test proving `Crawls=yes` uses ceiling two-thirds speed and
  `Crawls=no` uses `speed + speed / 2` while prone;
- state-hash test proving fear level and prone bit affect `Simulation::state_hash`;
- animation tests for Down, Up, Prone, Crawl, and FireProne selection from sim
  state;
- a GI integration test: damaged GI gains fear, drops prone, crawls slower,
  later recovers, and never uses the old animation proxy as source of truth.
