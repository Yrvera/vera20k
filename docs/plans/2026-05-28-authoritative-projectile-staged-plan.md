# Authoritative Projectile Damage Model — Staged Plan (Contract #6)

Date: 2026-05-28
Premise: PROVEN player-visible drift. Rust applies weapon damage instantly in the
fire tick (`combat/mod.rs` pushes damage_events at fire, applies them Phase 4 of the
same `tick_combat_with_fog`, no projectile-speed/flight consideration). gamemd fires
an authoritative `BulletClass` object (`BulletClass::Fire 0x00468670` → `ObjectClass::Reveal`
registers it as a live object; **no damage in Fire**) and applies damage only at
detonation inside `BulletClass::AI 0x004666E0` (`BulletClassBulletDetonationImpactDamage`),
1+ ticks later for slow/ranged shots. A bullet Reveal-registered at the tail can
detonate on its first AI the SAME tick (the `for_each_live_object` case) — e.g. AA/zero-range.

This is a multi-session, high-blast-radius build. Implement in stages; each stage must
keep the suite green (10-baseline) and the state hash deterministic before the next.

## Why this can NOT be parallel-swarmed

Lockstep determinism is the #1 invariant. The change touches combat damage, RNG draw
order, the entity stream, and `world_hash` — all shared, all order-sensitive. Concurrent
writers to these = desync. One controlled implementer only.

## Determinism gate (must hold every stage)

1. **RNG draw parity.** gamemd `BulletClass::Fire` draws RNG for inaccuracy scatter
   (verify exact draw count/order at `0x00468670` before coding). Adding/removing/reordering
   any RNG draw shifts the whole stream → desync. Mirror gamemd's draw count & order, or
   document a deliberate Rust-native RNG sequence and keep it stable.
2. **Bullets in `world_hash`.** New entities must be hashed deterministically (BTreeMap
   id order already gives this) — confirm projectile entities are included.
3. **Tick-slot placement.** Bullet AI runs in the existing air/special-movement stage
   (where `rocket_movement`/`homing_movement` already live), AFTER combat fires them, so
   `for_each_live_object` gives same-tick-first-AI for zero-flight bullets.
4. **No float in sim.** Flight integration stays fixed-point (reuse existing
   rocket/homing math, which is already fixed-point).

## Stages

### Stage 0 — Investigation (do first; `/plan-investigation` the closed loop)
Read fire → fly → detonate → damage → retaliation end-to-end. Pin down:
- exact RNG draws in `BulletClass::Fire` (inaccuracy) and `BulletClass::AI` (homing/retarget).
- gamemd mid-flight retarget/interception behavior (`BulletClass::AI` target re-resolve;
  what happens when the target dies/moves mid-flight — detonate on last coord? re-home?).
- which weapons are Inviso/`Speed=0` (gamemd also instant — these stay hitscan).

### Stage A — Bullet entity + spawn-from-combat (behind the existing instant path)
- Add a projectile spawn in the fire pass that creates an entity carrying a **deferred
  warhead payload** (damage, warhead id, firer owner, target snapshot, veterancy).
- Reuse `RocketState`/`HomingState`/straight-line for trajectory; add a `Hitscan` variant
  for `Speed=0`/Inviso that detonates first-AI same tick (preserves current correct cases).
- Register via `register_live_object` so the air/special stage processes it; `for_each_live_object`
  gives same-tick first AI when appropriate.
- KEEP instant damage OFF the new path initially is impossible (the point is to defer) — so
  Stage A and B land together for the migrated weapons; gate by a per-weapon flag so only
  a small set (start with one slow projectile, e.g. V3) routes through bullets while the
  rest stay instant. This keeps the blast radius small and the suite green.

### Stage B — Detonation owns damage
- Lift the Phase-3 warhead-emission block (AoE, wall/bridge/wood, ore reduce, AnimList,
  sound, smudge, fire events) out of combat into `detonate_bullet()` called by the movement
  system on the `Detonation` phase. Damage application moves there.
- `CombatTickResult`'s event vectors now emit from detonation for migrated weapons.
- Retaliation/last-attacker now fires at impact, not at shot — verify ordering.

### Stage C — Migrate remaining weapons + retire app-layer visual
- Flip remaining ranged/artillery weapons onto the bullet path.
- Replace `app_fire_effects.rs` `ProjectileVisual` (real-time interpolation,
  `MIN/MAX_PROJECTILE_VISUAL_MS`) with a render of the real bullet entity position.
- Muzzle flash + report stay at fire.

## Acceptance tests
- Slow projectile (V3): target at range takes damage N ticks after fire, N = distance/speed
  (fixed-point), not same tick.
- Target dies/moves mid-flight: bullet detonates on last-known coord / re-homes per Stage-0
  finding (NOT auto-hit).
- Zero-range/AA (AAHeatSeeker2): detonates first-AI same tick (matches current correct case).
- Determinism: state hash stable across runs; no NEW failures vs the 10-baseline at every stage.

## Biggest risks (ranked)
1. RNG-draw parity (desync) — gate everything on Stage 0 nailing this.
2. Mid-flight retarget = genuinely new behavior; needs its own parity verification.
3. Test churn: `combat_tests.rs` asserts same-tick damage/death; migrated-weapon tests need
   a flight-advance inserted. Mechanical but large.

## Recommendation
Greenlight as a focused multi-session effort. Do Stage 0 (investigation) + Stage A/B for ONE
weapon (V3) as the first shippable increment behind a per-weapon gate, prove the premise fix
in-game, then migrate the rest in Stage C.
