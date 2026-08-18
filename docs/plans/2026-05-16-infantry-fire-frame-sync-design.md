# Infantry Fire-Frame Synchronization Design

## Scope

This change makes infantry combat discharge wait for the active fire animation frame instead of applying damage on the same tick that range, facing, and cooldown pass.

The implementation is generic for infantry. GI is the validation case because its primary/secondary, prone, and deployed fire visuals exercise the important branches.

## Source Rules

- `FireUp`, `FireProne`, `SecondaryFire`, and `SecondaryProne` are parsed from art data and merged onto infantry object types.
- Missing prone/secondary fields fall back through the same data path instead of using GI-specific constants.
- Weapon selection remains target-driven. A deployed GI can use the selected secondary weapon while the visual sequence is `DeployedFire`.

## Runtime Flow

Combat owns the fire latch through `AttackTarget::pending_infantry_fire`.

1. A normal combat tick validates target, weapon, range, cooldown, and facing.
2. For infantry with animation state, the tick switches to the selected fire sequence and records the expected fire frame.
3. No projectile or damage is spawned until a later combat tick observes that the same sequence reached that frame.
4. The fire-frame tick revalidates the target, weapon, range, cooldown, and facing before spawning damage/projectile effects.
5. The latch is cleared when the shot fires or when the shot is cancelled.

Vehicles, buildings, aircraft, and garrison fire keep their existing immediate discharge behavior.

## Sequence Selection

- Fully deployed infantry uses `DeployedFire` for the visual sequence.
- Primary standing infantry uses `Attack`.
- Primary prone infantry uses `FireProne`.
- Secondary standing/prone use the secondary fire sequences when distinct fire-frame data is present, otherwise they fall back to the primary fire visual.

## Cancellation

A pending infantry shot is cancelled without spawning stale damage when:

- the target dies, disappears, becomes friendly/hidden under fog, or no valid retarget exists;
- the target leaves range;
- cooldown or burst delay is no longer ready at the discharge frame;
- turret/facing alignment is no longer valid;
- the infantry starts moving;
- the current animation sequence no longer matches the recorded pending fire sequence.

Retargeting clears the pending latch before the new target is stored.

## Determinism

The pending infantry fire state is serialized and included in `Simulation::state_hash`, together with existing burst timing fields, so two worlds that differ only by an accepted delayed infantry shot hash differently.
