# Signed Object Vitality Shadow Transaction Design

**Date:** 2026-07-23
**Status:** approved for bounded implementation
**Authority:** shadow-only; live `Health`, lifecycle, snapshots, hashes, and RNG remain unchanged

## Goal and boundary

Add a pure representation of the verified `ObjectClass::ReceiveDamage` vitality
rows without wiring any production writer. The result preserves signed mutable
damage writeback and reports later callback/lifecycle work as intent only.

The implementation stops before Techno gates, synchronous trigger execution,
Cyborg rescue, kill-credit selection, wrapper-specific death behavior, Terrain
integration, or any authority flip.

## Architecture

- `sim/entity_state::VitalityState` remains the exact signed storage shape.
- `sim/combat/damage` owns the pure Object vitality behavior.
- A signed packet distinguishes a verified kernel-result writeback from the
  ignore-defenses path, where the requested value remains the normalized value.
- The outcome carries old/new vitality, normalized damage, numeric receiver
  result, and explicit callback/tail/lifecycle intents.
- The transaction does not reach into `EntityStore` and executes no intent.

## Ordered behavior

1. Snapshot signed entry health.
2. Return unchanged for entry health `<= 0`, requested damage `== 0`, or Object
   immunity with defenses active.
3. Select kernel writeback or the bypassed requested value.
4. Floor Building `CanC4=false` values to positive one.
5. Return unchanged for a remaining zero value.
6. Apply negative healing with a signed Strength cap; preserve normalized damage;
   request callback `+0x148(7)` only when health changed.
7. Start positive classification at result `1`.
8. Apply the strict integer-half Yellow crossing.
9. Inclusively cap overkill to entry health and write the cap back.
10. Apply the strict x87 `Strength * ConditionRed` crossing, overriding Yellow.
11. Commit signed health.
12. Convert exact zero to provisional fatal result `4` and report that the
    synchronous receiver tail plus concrete wrapper handoff remain required.

## Alternatives rejected

- Putting the transition in `entity_state`: that module represents storage and
  would become a second behavior/lifecycle authority.
- Rewriting the existing broad `receive_damage` pipeline now: its Techno/kernel
  assumptions are outside this bounded verified slice.
- Executing callbacks or UnInit from the pure helper: native callbacks can mutate
  state synchronously, while fatal result `4` has class-specific continuations.

## Validation

Focused unit tests cover early gates, capped and unchanged healing, the Building
negative/zero floor, inclusive exact/overkill writeback, odd Yellow boundaries,
strict Red equality/crossing with x87 inputs, exact-zero fatal intent, distinct
result `5` representation, and absence of executed lifecycle state.

