# Force-Attack Entity Parity Design

## Goal

Preserve player force-attack intent for entity targets so Ctrl-clicked units and buildings fire like gamemd.exe, while still applying receiver-side friendly-fire protection.

## Architecture Context

Force-fire starts in the app layer and is serialized as `Command::ForceAttack`.
`Simulation::apply_command` currently dispatches both `Command::Attack` and
`Command::ForceAttack` through `combat::issue_attack_command`, which stores
`AttackTarget::new(target_id)`. After this point combat cannot distinguish a
normal entity attack from a manual force-attack.

Empty-cell force-fire already preserves intent via `TargetKind::Cell(rx, ry)`.
Entity force-fire does not have an equivalent marker, so combat later applies
normal entity-target filters such as friendly-target retarget/removal.

Relevant components:

- `src/app_context_order.rs`: detects Ctrl-only force-fire and queues commands.
- `src/sim/command.rs`: deterministic command payloads and replay surface.
- `src/sim/world/world_commands.rs`: validates ownership and applies commands.
- `src/sim/combat/mod.rs`: `AttackTarget`, weapon selection, fire timing, damage events.
- `src/sim/combat/combat_aoe.rs`: splash target collection and damage calculation.
- `src/rules/warhead_type.rs`: parsed warhead data, currently missing `AffectsAllies`.
- `src/sim/snapshot.rs`: snapshot version and round-trip tests.
- `src/sim/world/world_hash.rs`: deterministic state hash for lockstep/debug.

## Impact Analysis

This changes authoritative sim state because `AttackTarget` must remember whether
an entity target was force-attacked. That affects snapshot serialization and world
hashing. It also changes combat behavior for forced entity targets: friendly and
normally invalid targets must keep firing instead of being retargeted or cleared.

The damage path must also gain `AffectsAllies` handling. Without that, fixing
force-targeting would make ordinary AP, SA, HE, and similar warheads damage allies,
which is the opposite of gamemd.exe behavior.

Risk areas:

- Snapshot version compatibility: adding a field to `AttackTarget` is a binary
  format change.
- Determinism: the force flag must be hashed and serialized.
- Retargeting: normal attack and passive acquisition must still reject friendlies.
- Damage: friendly targets should still receive visual/sound events, but HP must
  remain unchanged for `AffectsAllies=no`.
- Weapon selection: force-fire with `Verses=0` should still produce a shot and
  zero damage, not fail selection.

## Chosen Approach

Add explicit force intent to `AttackTarget`, e.g. `force_fire: bool` or a small
`AttackIntent` enum. `Command::Attack` creates a normal entity attack;
`Command::ForceAttack` creates a forced entity attack. `Command::ForceAttackCell`
continues to use `TargetKind::Cell`, and may also set the same force intent for
consistency.

Combat then uses this intent to distinguish two concepts:

- Targeting eligibility: force-fire may keep firing at manual targets that normal
  target selection would reject.
- Damage eligibility: receiver-side warhead rules still decide whether HP changes.

This keeps the app layer as an input translator and leaves gameplay semantics in
`sim/`, matching the existing architecture.

## Tiny-Detail Ledger

- Force-fire can target friendly entities; target legality is bypassed upstream. Source: `combat/systems/can_target_gates.md` section 8.
- Force-fire does not bypass cooldown, ammo, fire-busy, or cloak-style commit gates. Source: `combat/systems/can_target_gates.md` section 8.
- Forced shots still produce projectile/muzzle/impact visuals and report sounds even when damage resolves to zero. Source: `combat/systems/friendly_fire.md` sections 3 and 5.
- `AffectsAllies=no` zeroes damage at receiver side, not target selection side. Source: `combat/systems/friendly_fire.md` section 3.
- Splash includes friendlies in the target list and applies `AffectsAllies` per target. Source: `combat/systems/friendly_fire.md` section 4.
- Force-fire does not bypass `AffectsAllies`. Source: `combat/systems/friendly_fire.md` section 5.
- Force-fire with `Verses=0` should fire and deal zero damage, consuming fire cadence and producing effects. Source: `combat/systems/verses_armor_matrix.md` section 6 and `combat/systems/can_target_gates.md` section 8.
- Exact gamemd mechanism for force-fire suppressing the Verses-zero target gate remains an open research follow-up. Source: `combat/systems/can_target_gates.md` section 11.
- Normal attack/passive acquisition still must not choose friendly targets. Source: `combat/systems/friendly_fire.md` section 6 and `combat/systems/target_acquisition.md`.
- `AffectsAllies` defaults to false in gamemd, so omitted INI keys protect allies. Source: `combat/systems/friendly_fire.md` section 2.

## Design

### Components

`AttackTarget`

- Add explicit force intent to the authoritative attack component.
- Keep `TargetKind` as the target identity: entity or cell.
- Prefer `AttackIntent` over a bare bool if it improves readability, but avoid
  adding more states than needed.

Command application

- `Command::Attack` calls the normal attack issuer.
- `Command::ForceAttack` calls a new force-aware issuer or passes an intent argument.
- `Command::ForceAttackCell` remains force-fire-on-cell and should produce the same
  attack component shape consistently.

Combat targeting and retargeting

- For normal entity targets, preserve current friendly/visibility retarget behavior.
- For forced entity targets, skip friendly retarget/removal caused solely by alliance.
- For forced entity targets, preserve the selected manual target unless it is gone,
  dead, or blocked by a commit-time gate that gamemd also enforces.
- Passive target acquisition must never create `force_fire` attacks.

Weapon selection

- Keep normal weapon selection behavior for normal attacks.
- Add a force-fire selection path that can return a weapon even when `Verses=0`,
  so the shot can fire and then resolve to zero damage.
- Do not bypass projectile class gates unless research confirms gamemd does so for
  that target kind. For the first implementation, only explicitly cover the
  researched force-fire/Verses and friendly-target behavior.

Damage application

- Parse `AffectsAllies` into `WarheadType`, defaulting to false.
- Before subtracting HP in direct damage, check whether attacker owner and target
  owner are allied. If allied and `AffectsAllies=false`, record the attack side
  effects but apply zero HP damage.
- Apply the same rule per target in AoE damage. The AoE collector should not skip
  friendlies up front; it should include them and let damage resolve to zero.
- Preserve non-HP cell effects such as bridge/wall/overlay routing according to
  their existing warhead gates; do not ally-gate those unless a source says so.

### Interfaces / Contracts

- `AttackTarget::new(target_id)` remains normal attack for compatibility inside code.
- Add `AttackTarget::forced_entity(target_id)` or `AttackTarget::new_with_intent`.
- Add or update tests to prove normal and forced entity attacks serialize differently.
- Update `state_hash` so two otherwise-identical sims diverge if one has a forced
  attack target and the other has a normal attack target.

### Data Flow

1. App detects Ctrl-click on entity.
2. App queues `Command::ForceAttack`.
3. World command validation confirms attacker ownership and target existence, but
   does not run the normal friendship rejection.
4. Combat stores an `AttackTarget` with force intent.
5. Combat tick resolves target and weapon.
6. Force intent suppresses normal retarget/target-validity rejection.
7. Fire cadence, animation sync, range, ammo, and commit gates still apply.
8. Fire event, sound, and impact effects are emitted.
9. Damage resolves through Verses and `AffectsAllies`; HP changes only when allowed.

### Error Handling

Invalid attacker or missing target still no-ops exactly as current commands do.
If a forced target disappears or is dead, clear or retarget according to the same
target-loss rules normal attacks use; the manual target no longer exists to fire at.

Unknown warhead or weapon data should keep existing behavior: fail selection or skip
the shot rather than inventing fallback constants.

### Testing Strategy

Focused unit/integration tests:

- `Command::Attack` against friendly target is rejected as today.
- `Command::ForceAttack` against friendly entity is accepted and stores forced intent.
- Forced friendly entity target is not removed by the combat friendly-retarget branch.
- Forced friendly target with `AffectsAllies=no` fires and emits fire effects, but HP
  does not change.
- Forced friendly target with `AffectsAllies=yes` takes damage.
- Forced entity target with `Verses=0` fires and emits effects, but HP does not change.
- Normal attack/passive acquisition still refuses friendlies.
- Snapshot round-trip preserves forced entity attack.
- State hash differs between normal and forced entity attacks.
- Existing force-fire-cell tests still pass.

## Architectural Decisions

- Force intent belongs in `sim::combat::AttackTarget`, not the app layer, because
  combat is where retargeting, weapon selection, and damage decisions occur.
- `AffectsAllies` belongs in `rules::warhead_type` and combat damage application,
  mirroring gamemd's receiver-side damage gate.
- This design intentionally avoids a broad rewrite of `TargetKind`; it adds the
  smallest authoritative state needed to preserve player intent.

Tech debt:

- The exact binary mechanism for force-fire suppressing the Verses-zero target gate
  is still marked as an open follow-up in the research docs. The implementation
  should match the documented observable behavior and keep the code isolated enough
  to refine once that mechanism is traced.

## Alternatives Considered

### `TargetKind::ForcedEntity(u64)`

This makes the target enum carry both identity and command intent. It would solve
the immediate bug but mixes target shape with targeting semantics and becomes less
flexible if more manual-fire flags are needed later.

### Dispatch-only special case

Keeping `AttackTarget` unchanged and trying to special-case only
`Command::ForceAttack` does not work reliably because the combat tick later needs
the force intent for retargeting, weapon gates, snapshots, and replay.

### GI-only patch

Rejected because the bug is not GI-specific. Any force-attack-on-entity can hit the
same loss of manual intent, and fixing only GI would leave visible parity holes.
