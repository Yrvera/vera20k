# GI Deploy-Fire — Slice B1 (sim core) Design

## Goal

Land the sim-side state machine for GI deploy/undeploy: a hashed
`DeployPhase` field on `GameEntity`, a `ToggleInfantryDeploy` command,
the parity-faithful Set_Destination gate (player + deployed silently
ignores Move/AttackMove/Enter/etc.), and Deploy/Undeploy sound emission.
**No UI hooks** — this slice is testable end-to-end at the sim level
via direct `Command::ToggleInfantryDeploy`. UI integration lands in B2.

## Architecture Context

**Sequence machinery already exists.**
- [src/sim/animation.rs:56-123](../../src/sim/animation.rs) defines
  `SequenceKind` with `Deploy`, `Undeploy`, `Deployed`, `DeployedFire`,
  `DeployedIdle`, `SecondaryFire`, `SecondaryProne` variants.
- art.ini sequence parsing (`src/rules/infantry_sequence.rs`) populates
  these per `ObjectType` via `SequenceSet`.
- `tick_animations()` ([animation.rs:421-449](../../src/sim/animation.rs))
  auto-transitions `Deployed → DeployedFire` when an `attack_target`
  appears, and back via `LoopMode::TransitionTo`.

**Sound parsing already exists.**
- [src/rules/object_type.rs:227,229](../../src/rules/object_type.rs)
  parse `DeploySound` and `UndeploySound` from rulesmd.ini onto the
  `ObjectType` struct.

**Weapon selection already exists and is target-driven.**
- [src/sim/combat/combat_weapon.rs:96](../../src/sim/combat/combat_weapon.rs)
  `select_weapon_with_ifv()` picks Primary/Secondary based on Verses
  table — no deploy state read. Matches gamemd RE finding (D-T.4 of
  GI_GHIDRA_REPORT) — the deploy state controls the visual sequence,
  not the weapon picked.

**State hash exists but does not include animation.**
- [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs)
  hashes movement/locomotor/attack_target/passenger_role/etc. but not
  `Animation`. This is intentional — animation is visual-only.

**The faithful Set_Destination gate has no Rust equivalent yet.**
The gamemd analog (`InfantryClass::Set_Destination @ 0x0051AA40`)
early-returns when `IsPlayerControl(owner) && current_seq ∈ {0x1B–0x1E}`.
The current Rust GI would walk away if given a Move command while
displaying a deploy sequence.

**No Deploy/Undeploy command type exists.** [src/sim/command.rs](../../src/sim/command.rs)
has `DeployMcv` and `UndeployBuilding` for the MCV↔ConYard transformation;
infantry deploy is a different mechanic (no entity substitution — same
GameEntity, just changes state).

## Impact Analysis

**Files touched:**

| File | Change |
|------|--------|
| `src/rules/object_type.rs` | +2 fields (`deploy_fire: bool`, `deploy_fire_weapon: Option<i32>`), +2 ReadINI lines, all literal-init test sites get the new fields |
| `src/sim/game_entity.rs` | +1 field `deploy_state: Option<DeployPhase>` |
| `src/sim/deploy.rs` (NEW, ~150 LOC) | `DeployPhase` enum, `tick_deploy_state()`, helpers, internal docs |
| `src/sim/mod.rs` | `pub mod deploy;` |
| `src/sim/command.rs` | +1 variant `Command::ToggleInfantryDeploy { entity_id }` |
| `src/sim/world/mod.rs` | +2 `SimSoundEvent` variants; tick-order insertion of `tick_deploy_state` |
| `src/sim/world/world_command.rs` (or wherever Command dispatch is) | +1 handler for `ToggleInfantryDeploy` |
| Move/AttackMove/EnterTransport/HarvestCell/RepairAtDepot/MinerReturn/CaptureBuilding handlers | +1 early-return gate keyed on `entity.is_deployed()` |
| `src/sim/animation.rs:421-449` | `tick_animations()` reads `deploy_state` and chooses sequence |
| `src/sim/world/world_hash.rs` | hash `deploy_state` |
| Snapshot version in `src/sim/snapshot/` | bump version (new field on `GameEntity`) |
| Test helpers (`locomotor_tests.rs:49`, `teleport_movement.rs:284`, etc.) | add `deploy_fire: false`, `deploy_fire_weapon: None`, `deploy_state: None` defaults |
| `src/sim/deploy_tests.rs` (NEW) | unit tests for state machine, gate, sound, hash determinism |

**What depends on the changed code:**
- All literal-init `ObjectType { ... }` and `GameEntity { ... }` sites need new field defaults.
- All exhaustive `match` on `Command` (sim dispatch, possibly net serializer) gain one arm.
- All exhaustive `match` on `SimSoundEvent` (currently in `app_sim_tick.rs:295+`) gain two arms — but **B1 leaves these as TODO/unreachable since UI translation lands in B2**. Use a placeholder `tracing::debug!` arm or `#[allow(unreachable_patterns)]` to keep the build green; B2 replaces with real translation.

**Determinism concerns:**
- `deploy_state` is included in the state hash. Replay-safe.
- `tick_deploy_state` is inserted into `World::advance_tick` at a fixed point (between vision/power and turrets+combat) so phase transitions happen on a deterministic tick boundary.
- No floating-point math in the phase-advance logic — `ticks_remaining` is `u16`, decremented integerwise.

**Blast radius:** small.
- `deploy_state.is_some()` gate only fires when an entity is in the deploy state machine. Non-deploy-fire infantry (`obj.deploy_fire == false`) never enter the machine, so `is_deployed() == false` always — no behavior change for them.
- The Set_Destination gate is keyed on `is_deployed()`, so any current Move command targeting a non-deployed unit is unchanged.
- Snapshot version bump invalidates pre-B1 saves. Acceptable cost (project is pre-1.0).

## Chosen Approach

Sim-only slice. State lives on `GameEntity` (gameplay-authoritative), animation is visual reflection. Three-phase enum (`Deploying / Deployed / Undeploying`) — `DeployedFire` is *not* a sim phase, it's an animation auto-transition driven by `attack_target.is_some()` (existing logic at animation.rs:438-440 keeps working unchanged). Toggle command, faithful Set_Destination gate, sounds emitted on phase entry. Test coverage exhaustive at sim level.

## Design

### Components

**1. `DeployPhase` enum** in `src/sim/deploy.rs`:

```rust
//! Infantry deploy-fire state machine.
//!
//! Models the sim-authoritative phase: Deploying → Deployed → Undeploying → None.
//! The animation system reads `entity.deploy_state` and reflects the visual
//! sequence (Deploy / Deployed / DeployedFire / Undeploy). `DeployedFire` is
//! not a sim phase — it's a visual sub-state of `Deployed` driven by
//! `attack_target.is_some()`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DeployPhase {
    /// Deploy animation playing — sim ticks count down to phase advance.
    /// Move/AttackMove/Enter commands silently ignored.
    Deploying { ticks_remaining: u16 },
    /// Stationary in deployed stance. Visual flips to DeployedFire when
    /// `attack_target.is_some()` (existing tick_animations auto-transition).
    Deployed,
    /// Undeploy animation playing — sim ticks count down to None.
    /// Move/AttackMove/Enter commands silently ignored.
    Undeploying { ticks_remaining: u16 },
}
```

**2. `GameEntity.deploy_state`** in [src/sim/game_entity.rs](../../src/sim/game_entity.rs):

```rust
pub deploy_state: Option<DeployPhase>,  // None = upright
```

Predicate helpers on `GameEntity`:

```rust
pub fn is_deployed(&self) -> bool {
    self.deploy_state.is_some()
}
pub fn is_fully_deployed(&self) -> bool {
    matches!(self.deploy_state, Some(DeployPhase::Deployed))
}
```

**3. `tick_deploy_state()`** in `src/sim/deploy.rs`:

```rust
pub fn tick_deploy_state(entities: &mut EntityStore) {
    for id in entities.keys_sorted() {
        let Some(entity) = entities.get_mut(id) else { continue };
        match entity.deploy_state {
            Some(DeployPhase::Deploying { ticks_remaining }) => {
                if ticks_remaining > 1 {
                    entity.deploy_state = Some(DeployPhase::Deploying {
                        ticks_remaining: ticks_remaining - 1,
                    });
                } else {
                    entity.deploy_state = Some(DeployPhase::Deployed);
                }
            }
            Some(DeployPhase::Undeploying { ticks_remaining }) => {
                if ticks_remaining > 1 {
                    entity.deploy_state = Some(DeployPhase::Undeploying {
                        ticks_remaining: ticks_remaining - 1,
                    });
                } else {
                    entity.deploy_state = None;
                }
            }
            _ => {}
        }
    }
}
```

**4. `Command::ToggleInfantryDeploy`** dispatch handler:

```rust
Command::ToggleInfantryDeploy { entity_id } => {
    let Some(entity) = entities.get_mut(entity_id) else { return };
    let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else { return };
    if !obj.deploy_fire {
        return;  // INI gate — only deploy-fire types respond
    }

    match entity.deploy_state {
        None => {
            // Start deploying.
            let ticks = compute_anim_ticks(obj, sequences, SequenceKind::Deploy);
            entity.deploy_state = Some(DeployPhase::Deploying { ticks_remaining: ticks });
            if let Some(sound_id) = obj.deploy_sound.as_ref().map(|s| interner.intern(s)) {
                let (rx, ry) = entity.position_rxry();
                sound_events.push(SimSoundEvent::EntityDeployed { deploy_sound_id: sound_id, rx, ry });
            }
        }
        Some(DeployPhase::Deployed) => {
            // Start undeploying.
            let ticks = compute_anim_ticks(obj, sequences, SequenceKind::Undeploy);
            entity.deploy_state = Some(DeployPhase::Undeploying { ticks_remaining: ticks });
            entity.movement_target = None;  // belt-and-braces — clear any stale nav
            if let Some(sound_id) = obj.undeploy_sound.as_ref().map(|s| interner.intern(s)) {
                let (rx, ry) = entity.position_rxry();
                sound_events.push(SimSoundEvent::EntityUndeployed { undeploy_sound_id: sound_id, rx, ry });
            }
        }
        Some(DeployPhase::Deploying { .. }) | Some(DeployPhase::Undeploying { .. }) => {
            // Mid-transition. Toggle ignored. Matches gamemd Mission_Dispatch
            // case 0x10 entry path which short-circuits when current_seq is
            // already in the deploy-set transitionals.
        }
    }
}
```

`compute_anim_ticks(obj, sequences, kind)` resolves the anim duration: looks up the GI's `SequenceSet`, fetches `SequenceDef` for the kind, returns `ceil(frame_count × tick_ms / SIM_TICK_MS)`. Falls back to a sensible default (e.g., 4 ticks) if the sequence isn't defined.

**5. Set_Destination gate** in each command handler that sets a movement intent:

```rust
// Top of Move, AttackMove, EnterTransport, HarvestCell, RepairAtDepot,
// MinerReturn, CaptureBuilding handlers:
if let Some(entity) = entities.get(entity_id) {
    if entity.is_deployed() {
        return;  // faithful: matches FUN_0051AA40 player+deploy early-return.
                 // Player must explicitly toggle undeploy first.
    }
}
```

**6. Animation reflection** in [src/sim/animation.rs:421-449](../../src/sim/animation.rs):

Replace the existing `Stand ↔ Walk` and attack-driven branches with a deploy-state-first cascade:

```rust
// After the existing dying-entity early-return:
match entity.deploy_state {
    Some(DeployPhase::Deploying { .. }) => {
        anim.switch_to(SequenceKind::Deploy);
    }
    Some(DeployPhase::Undeploying { .. }) => {
        anim.switch_to(SequenceKind::Undeploy);
    }
    Some(DeployPhase::Deployed) if has_attack => {
        if let Some(set) = seq_set {
            if set.get(&SequenceKind::DeployedFire).is_some() {
                anim.switch_to(SequenceKind::DeployedFire);
            } else {
                anim.switch_to(SequenceKind::Deployed);
            }
        }
    }
    Some(DeployPhase::Deployed) => {
        anim.switch_to(SequenceKind::Deployed);
    }
    None => {
        // Fall through to existing Stand/Walk + Attack/FireProne logic
        // (unchanged from current animation.rs).
        ...
    }
}
```

**7. SimSoundEvent variants** in [src/sim/world/mod.rs:87-130](../../src/sim/world/mod.rs):

```rust
EntityDeployed   { deploy_sound_id: InternedId, rx: u16, ry: u16 },
EntityUndeployed { undeploy_sound_id: InternedId, rx: u16, ry: u16 },
```

**8. Hash inclusion** in [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs):

```rust
match entity.deploy_state {
    None => 0u8.hash(state),
    Some(DeployPhase::Deploying { ticks_remaining }) => {
        1u8.hash(state);
        ticks_remaining.hash(state);
    }
    Some(DeployPhase::Deployed) => {
        2u8.hash(state);
    }
    Some(DeployPhase::Undeploying { ticks_remaining }) => {
        3u8.hash(state);
        ticks_remaining.hash(state);
    }
}
```

**9. Tick order** in `World::advance_tick` (per [src/sim/mod.rs](../../src/sim/mod.rs) header):

```
commands → ground movement → air/special → vision → power
   ↓
   tick_deploy_state                  ← NEW
   ↓
turrets+combat → retaliation → ... → state hash
```

`tick_deploy_state` must run *after* command dispatch (so a same-tick `ToggleInfantryDeploy` is reflected) and *before* combat (so `attack_target` decisions see the up-to-date phase).

### Interfaces / Contracts

**Public:**
- `DeployPhase` enum, `pub` from `src/sim/deploy.rs`. Re-exported via `src/sim/mod.rs` if other modules need it.
- `tick_deploy_state(entities: &mut EntityStore)` — pub, called by `World::advance_tick`.
- `Command::ToggleInfantryDeploy { entity_id: u64 }` — pub, additive variant.
- `SimSoundEvent::EntityDeployed { ... }`, `SimSoundEvent::EntityUndeployed { ... }` — pub, additive.
- `GameEntity::is_deployed() -> bool`, `is_fully_deployed() -> bool` — pub on the existing struct.
- `ObjectType.deploy_fire: bool`, `ObjectType.deploy_fire_weapon: Option<i32>` — pub fields.

**Internal:**
- `compute_anim_ticks(obj, sequences, kind) -> u16` — `pub(crate)` helper in `deploy.rs`.

### Data Flow

```
INI [E1] DeployFire=yes; DeploySound=GIDeploy
  ↓
ObjectType { deploy_fire: true, deploy_sound: Some("GIDeploy"), ... }

[Player issues toggle via D-key or left-click in B2]
  ↓
Command::ToggleInfantryDeploy { entity_id }
  ↓
[World command dispatch]
  ↓
entity.deploy_state = Some(DeployPhase::Deploying { ticks_remaining: N })
sound_events.push(SimSoundEvent::EntityDeployed { ... })

[Each subsequent tick]
  ↓
tick_deploy_state: ticks_remaining-- ; on zero → Deployed
tick_animations:   Deploying → SequenceKind::Deploy → Deployed → SequenceKind::Deployed
                   Deployed + has_attack → SequenceKind::DeployedFire (visual only)

[Player issues another toggle while Deployed]
  ↓
entity.deploy_state = Some(DeployPhase::Undeploying { ticks_remaining: N })
sound_events.push(SimSoundEvent::EntityUndeployed { ... })
  ↓
[Tick countdown]
  ↓
entity.deploy_state = None

[Move command issued mid-deploy]
  ↓
Move handler: entity.is_deployed() → early-return → no movement_target set → no motion
```

### Error Handling

- `Command::ToggleInfantryDeploy` for a non-existent entity ID: silent no-op (matches existing command handler convention).
- Toggle on entity with `obj.deploy_fire == false`: silent no-op (the INI gate).
- Mid-transition toggle: silent no-op (matches gamemd's "in-deploy-seq" early-return).
- Missing `Deploy` sequence in art.ini for a `deploy_fire=true` type: `compute_anim_ticks` falls back to a default (4 ticks). Logged at `tracing::warn!` once per type.
- Missing `DeploySound` in INI: emit skipped — no sound plays. Matches current CrushSound pattern.
- Move command on deployed entity: silent no-op. **No error log** — this is intentional behavior, not a malfunction. Player UX feedback can be added later (e.g., voice cue on rejected move).

### Testing Strategy

New file `src/sim/deploy_tests.rs`:

| Test | Setup | Assertion |
|------|-------|-----------|
| `deploy_phase_advances_to_deployed` | Toggle on upright deploy_fire GI; advance N=anim ticks | `deploy_state == Some(Deployed)` |
| `undeploy_phase_clears_to_none` | Deployed GI, toggle, advance N=anim ticks | `deploy_state == None` |
| `mid_deploying_toggle_ignored` | Deploying { ticks: 3 }, toggle | Phase unchanged, no extra sound emitted |
| `mid_undeploying_toggle_ignored` | Undeploying { ticks: 3 }, toggle | Phase unchanged, no extra sound emitted |
| `move_silently_ignored_on_deployed` | Deployed GI, issue Move | `movement_target == None`, position unchanged after tick |
| `move_silently_ignored_on_deploying` | Deploying { ticks: 5 }, issue Move | Same |
| `move_silently_ignored_on_undeploying` | Undeploying { ticks: 5 }, issue Move | Same |
| `attack_move_silently_ignored_on_deployed` | Deployed, issue AttackMove | Same |
| `enter_transport_silently_ignored_on_deployed` | Deployed, issue EnterTransport | Same |
| `move_works_after_undeploy_completes` | Deployed → toggle → wait for None → issue Move | Movement proceeds |
| `deploy_sound_emitted_on_phase_entry` | Toggle (None → Deploying) | Exactly one `EntityDeployed` event with correct sound_id |
| `deploy_sound_suppressed_when_unset` | `deploy_sound = None`, toggle | No `EntityDeployed` event |
| `undeploy_sound_emitted_on_phase_entry` | Deployed → toggle | Exactly one `EntityUndeployed` event |
| `non_deploy_fire_infantry_no_op` | Rifle infantry without `deploy_fire`, toggle | Phase unchanged |
| `hash_deterministic_through_full_cycle` | Two sims, identical command stream toggling deploy/undeploy 3× | Hashes match every tick |
| `snapshot_round_trip_mid_deploying` | Sim with Deploying { ticks: 5 }, save → load | Phase preserved exactly |
| `combat_fires_during_deployed_attack` | Deployed GI, target in range | Animation switches to DeployedFire; combat picks weapon via existing select_weapon (no deploy-state read) |

**Negative tests:** confirm non-infantry deploy commands (`DeployMcv` / `UndeployBuilding`) still work unchanged.

## Architectural Decisions

- **Pattern followed**: gameplay state on `GameEntity` (matches `attack_target`, `passenger_role`, `dock_state`); animation is visual reflection. Hash includes the gameplay field, not the visual.
- **Three-phase enum, not four**: `DeployedFire` is a visual sub-state of `Deployed` — gamemd treats it the same gameplay-wise (the entity is "deployed and may fire"). Modeling DeployedFire as a separate sim phase would add a transition that has no gameplay distinction.
- **Toggle command, not separate Deploy/Undeploy**: matches gamemd mission 0x10 (the player path) which is a single toggle. Simpler than two commands and lets the sim decide the next phase based on current state.
- **Faithful Set_Destination gate**: per user choice, exactly mirrors `FUN_0051AA40` early-return. Player must explicitly undeploy. No auto-undeploy on Move.
- **`compute_anim_ticks` default fallback**: art.ini sometimes omits `Deploy=` for non-default infantry mods. Falling back to 4 ticks is graceful — gamemd just plays whatever's in the sequence array, including zero-frame defaults (which would advance immediately). Default of 4 prevents edge-case insta-deploy.
- **Snapshot version bump**: necessary because `GameEntity` gains a field. Acceptable pre-1.0.

**Tech debt introduced:** The `SimSoundEvent` exhaustive match in `app_sim_tick.rs` will need stub arms in B1 (the actual translation lands in B2). Use a `tracing::trace!` placeholder rather than `unimplemented!()` so the build stays green and the events are observable in logs during sim-only testing.

## Alternatives Considered

- **Animation-driven phase (Q2 option Y)**: read deploy state from `entity.animation.sequence`, hash `Animation`. Rejected — conflates visual and gameplay state, hashes 6 bytes per animated entity per tick across the entire game.
- **Skip determinism (Q2 option Z)**: don't hash deploy_state. Rejected — replay/lockstep would desync on deploy.
- **Player + AI auto-deploy (scope option B)**: include the AI Mission_Guard auto-deploy logic. Rejected per user choice — that's intricate (`FUN_00521320` gates on type fields + frame timer + Rules+0xE30) and depends on Mission_Guard plumbing that doesn't exist yet. Deferred to a future slice.
- **Auto-undeploy on Move (Q3 option friendly)**: more user-friendly but introduces parity drift. Rejected per user choice.
- **Four-phase enum including DeployedFire**: would model the visual sub-state as a sim phase. Rejected — no gameplay distinction from Deployed; would require sim-side bookkeeping of fire intent that the existing combat tick already handles via `attack_target`.
- **Separate `Deploy` and `Undeploy` commands**: more explicit, but loses gamemd parity (mission 0x10 is a single toggle). Toggle is also harder to misuse from UI — the sim figures out direction from current state.
