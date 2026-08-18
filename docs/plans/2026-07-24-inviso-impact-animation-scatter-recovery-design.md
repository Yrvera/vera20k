# Inviso Impact-Animation Scatter Recovery Design

## Goal

Replace the unique useful part of `stash@{0}` with an exact current-architecture
implementation of the active-YR `Inviso=yes` detonation-coordinate behavior.
Damage and terrain consequences remain at the original impact coordinate; only
the visible impact animation and its paired animation-smudge use the randomized
coordinate.

## Evidence

- `docs/research/INVISIBLELOW_DETONATION_COORDSTRUCT_GHIDRA_REPORT.md`
- `BulletClass::Detonate @ 0x004690B0`
- randomized CoordStruct helper `0x0049F420`
- sine/cosine lookup helpers `0x004CACB0` and `0x004CAD00`
- native integer conversion helper `0x007C5F00`
- live `read_memory` evidence for the angle constant, lookup scale, x87 control
  word, and the 10,240-entry float lookup table

The verified mechanism consumes one low byte from persistent
`ScenarioClass::Random` at `ScenarioClass+0x218`,
computes:

```text
x = ftol(base.x + cos_lookup(angle) * 32)
y = ftol(base.y - sin_lookup(angle) * 32)
z = base.z
```

and falls back to the original coordinate if either resulting cell coordinate is
outside the unsigned 512 by 512 map domain. The RNG draw occurs before warhead
animation selection, including when `AnimList` is empty.

## Chosen design

1. Add a small combat-owned exact CoordStruct helper.
2. Store the 256 binary-derived sine and cosine `f32` bit samples actually
   reached by every possible low-byte draw.
3. Evaluate the radius multiply, base-coordinate add/subtract, and `ftol` through
   `util::native_x87::X87Chop53`. This preserves the active process's 53-bit,
   truncate-toward-zero behavior without floating point in simulation code.
4. Thread an explicit mutable Scenario RNG reference through
   `tick_combat_with_fog` and `resolve_attacker_fire`.
5. Resolve `weapon.projectile` only on the ordinary weapon-fire path. If that
   projectile has `Inviso=yes`, consume one draw and derive an effect-only
   coordinate immediately before warhead animation emission.
6. Keep damage, AoE, wall/bridge, terrain, ore, and radiation events on the
   original target coordinate.
7. Feed the randomized coordinate to both `ExplosionEffect` and the paired
   `SmudgeSpawnRequest::Anim`.

## Known adjacent ordering gap

Rust still drains animation-smudge requests after the combat batch, whereas
gamemd constructs/starts each impact AnimClass inline before the next object
continues. This recovery preserves the verified Inviso scatter draw in
live-object order, but does not certify global Scenario-RNG interleaving for
other animation/debris mechanisms that Rust has not yet modeled inline.
Stock GI `PIFFPIFF` has neither `Scorch` nor `Crater`, so its paired smudge path
does not add a second RNG draw.

## Rejected alternatives

- Reapply the stash helper: it uses approximate facing math, the obsolete RNG
  surface, and suppresses the draw for an empty `AnimList`.
- Reuse `smudge_dispatch::random_offset_at_radius`: its runtime trigonometric
  Q16 table is explicitly not binary-exact.
- Precompute only integer offsets: the binary adds tiny cardinal table samples
  to the base coordinate before `ftol`, so the final integer can depend on the
  base coordinate.
- Seed a local test RNG inside combat: this would hide the persistent
  Scenario-RNG
  authority and ordering contract.

## Validation

- Exhaust all 256 draw bytes against the checked-in binary-derived sample
  oracle, including cardinal tiny values and boundary fallback.
- Verify byte 0 points north and byte 64 points east.
- Verify Inviso consumes exactly one Scenario-RNG draw; non-Inviso consumes none.
- Verify empty `AnimList` still consumes the draw.
- Verify damage/terrain/radiation coordinates stay unchanged while the effect
  and paired animation-smudge share the randomized coordinate.
- Verify two Inviso attackers consume consecutive bytes in live-object order.
- Run focused combat tests, then `cargo check -q -p vera20k`.

## Stop condition

The exact replacement passes focused validation, unrelated diffs remain
untouched, and `stash@{0}` remains available until the replacement is committed.
