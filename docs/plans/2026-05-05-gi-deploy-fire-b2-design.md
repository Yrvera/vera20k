# GI Deploy-Fire — Slice B2 (UI integration) Design

## Goal

Wire the player input paths to B1's `Command::ToggleInfantryDeploy`:
extend the D-key handler to recognize deploy-fire infantry, add a
left-click-on-already-selected-own-GI toggle (mirroring the
garrison-unload pattern from commit `65f0b1d`), and translate
`SimSoundEvent::EntityDeployed` / `EntityUndeployed` into `AudioEvent`s
for playback.

Depends on B1 (sim core) being merged. B2 is pure wiring on top — no
new sim logic.

## Architecture Context

**B1 delivered** (prerequisite):
- `Command::ToggleInfantryDeploy { entity_id }`
- `ObjectType.deploy_fire: bool`
- Sim-side toggle handler with INI gate, phase advancement, sound emission
- Faithful Set_Destination gate
- Hash inclusion + tests

**Existing UI patterns to follow:**
- D-key: [src/app_input.rs:658-698](../../src/app_input.rs)
  `queue_deploy_undeploy_for_selected()` already handles MCV→ConYard
  and ConYard→MCV via `Command::DeployMcv` / `Command::UndeployBuilding`.
  The function builds a list of commands per selected entity, then
  schedules them.
- Left-click on already-selected own structure: commit `65f0b1d`
  added "left-click on selected garrisoned building unloads occupants"
  in [src/app_context_order.rs](../../src/app_context_order.rs). The
  pattern: check `selected_ids.contains(&target.stable_id)`, check
  category + capability, push command, return `true` to consume the
  click.
- AudioEvent translation: [src/app_sim_tick.rs:295+](../../src/app_sim_tick.rs)
  drains `sim.sound_events` each frame and translates each
  `SimSoundEvent` variant into the corresponding `AudioEvent` (the
  CrushSound implementation is the canonical example).

## Impact Analysis

**Files touched:**

| File | Change |
|------|--------|
| `src/app_input.rs` | extend `queue_deploy_undeploy_for_selected()` — +1 match arm for `EntityCategory::Infantry` |
| `src/app_context_order.rs` | +1 left-click branch for selected own deploy_fire infantry |
| `src/app_sim_tick.rs` | replace B1's stub `tracing::trace!` arms with real `AudioEvent` translation |
| `src/audio/events.rs` | +2 `AudioEvent` variants (`EntityDeployed`, `EntityUndeployed`) |
| `src/audio/` consumer (audio backend or wherever AudioEvents are matched) | +2 arms playing the sound |
| `src/app_context_order.rs` tests | +1 unit test (synthetic left-click) |

**Estimated diff:** ~150-250 LOC, mostly wiring.

**What depends on the changed code:**
- All exhaustive matches on `AudioEvent` (the audio backend dispatcher) gain two arms.
- No new abstractions, no new files.

**Determinism concerns:** none. UI/audio is non-sim — does not affect the state hash.

**Blast radius:** trivially small. All new arms are additive. The only behavior-affecting changes are:
- D-key with infantry selected now does something (was no-op before B2).
- Left-click on selected own deploy-fire GI now toggles deploy (was no-op before B2).
Neither breaks existing behavior on non-deploy-fire units.

## Chosen Approach

Three sites, three additive edits:

1. **D-key** — extend `queue_deploy_undeploy_for_selected` with an `Infantry` arm that checks `obj.deploy_fire`.
2. **Left-click on selected own GI** — sibling check next to the existing garrison-unload-on-left-click in `app_context_order.rs`.
3. **AudioEvent translation** — replace B1's stub arms in `app_sim_tick.rs` with real translation, mirroring the CrushSound pattern.

No alternative shape is meaningfully different — the existing patterns dictate the structure. (See Alternatives Considered for the cursor-customization sub-question.)

## Design

### Components

**1. D-key handler extension** in [src/app_input.rs:658-698](../../src/app_input.rs):

Inside the existing for-loop, add the Infantry arm:

```rust
match entity.category {
    EntityCategory::Structure => {
        // EXISTING — garrisoned building unload OR ConYard → MCV
        if obj.map_or(false, |o| o.can_be_occupied)
            && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
        {
            commands.push(Command::UnloadPassengers { transport_id: entity_id });
        } else if obj.map_or(false, |o| o.undeploys_into.is_some()) {
            commands.push(Command::UndeployBuilding { entity_id });
        }
    }
    EntityCategory::Infantry => {                                    // NEW arm
        if obj.map_or(false, |o| o.deploy_fire) {
            commands.push(Command::ToggleInfantryDeploy { entity_id });
        }
    }
    _ => {
        // EXISTING — vehicle MCV → ConYard
        if obj.map_or(false, |o| o.deploys_into.is_some()) {
            commands.push(Command::DeployMcv { entity_id });
        }
    }
}
```

**Multi-selection behavior:** D-key with mixed selection (e.g., 3 GIs + 1 MCV + 1 ConYard) emits one command per entity — three `ToggleInfantryDeploy`, one `DeployMcv`, one `UndeployBuilding`. Each fires independently in the same tick.

**Mid-transition GIs:** Already a no-op at the B1 sim handler, so spamming D doesn't queue a phase storm. The UI doesn't need to filter — sim absorbs the redundant commands.

**2. Left-click handler** in [src/app_context_order.rs](../../src/app_context_order.rs):

Find the existing branch handling left-click on selected garrisoned building and add a sibling for infantry. Order matters — this branch must be evaluated **before** the unit-vs-cell Move fallthrough so the click is consumed:

```rust
// In the click resolution chain, after the structure-self-click branches
// but BEFORE the move/attack fallthrough:
if selected_ids.contains(&target.stable_id) {
    if let Some(target_entity) = sim.entities.get(target.stable_id) {
        if let Some(obj) = rules
            .and_then(|r| r.object(sim.interner.resolve(target_entity.type_ref)))
        {
            if matches!(target_entity.category, EntityCategory::Infantry)
                && obj.deploy_fire
            {
                schedule_command(state, &owner, Command::ToggleInfantryDeploy {
                    entity_id: target.stable_id,
                });
                return true;  // consume click — no Move/Attack fallthrough
            }
        }
    }
}
```

**Per-entity granularity:** clicking one GI in a 5-GI selection toggles only that one. Matches existing left-click-on-self semantics. Bulk toggling stays on the D-key path.

**3. AudioEvent variants** in [src/audio/events.rs](../../src/audio/events.rs):

```rust
// Mirror EntityDestroyed:
EntityDeployed   { sound_id: String, screen_pos: Option<(f32, f32)> },
EntityUndeployed { sound_id: String, screen_pos: Option<(f32, f32)> },
```

**4. Translation** in [src/app_sim_tick.rs:295+](../../src/app_sim_tick.rs):

Replace B1's stub arms:

```rust
SimSoundEvent::EntityDeployed { deploy_sound_id, rx, ry } => {
    audio_events.push(AudioEvent::EntityDeployed {
        sound_id: sim.interner.resolve(deploy_sound_id).to_string(),
        screen_pos: tactical.world_to_screen(rx, ry),
    });
}
SimSoundEvent::EntityUndeployed { undeploy_sound_id, rx, ry } => {
    audio_events.push(AudioEvent::EntityUndeployed {
        sound_id: sim.interner.resolve(undeploy_sound_id).to_string(),
        screen_pos: tactical.world_to_screen(rx, ry),
    });
}
```

**5. Audio backend consumer** — wherever `AudioEvent` is matched to play sounds (likely `src/audio/mod.rs` or `src/audio/dispatcher.rs`), add two arms calling the existing sound-play helper. Same pattern as `EntityDestroyed`.

### Interfaces / Contracts

All edits additive:
- `AudioEvent` gains 2 variants.
- `queue_deploy_undeploy_for_selected` gains an `EntityCategory::Infantry` match arm.
- `app_context_order` left-click resolution gains a deploy-fire-infantry branch.
- `app_sim_tick` exhaustive `SimSoundEvent` match gains real arms (replacing B1 stubs).

No public API changes beyond the additive enum variants.

### Data Flow

```
[Player presses D with 3 GIs selected]
  ↓
app_input KeyCode::KeyD → queue_deploy_undeploy_for_selected(state)
  ↓
For each GI: schedule_command(Command::ToggleInfantryDeploy { entity_id })
  ↓
[B1 sim handler: 3× state mutations, 3× SimSoundEvent::EntityDeployed pushed]
  ↓
[App tick drains sim.sound_events]
  ↓
3× AudioEvent::EntityDeployed { sound_id: "GIDeploy", screen_pos }
  ↓
audio backend plays 3× GIDeploy.wav (positional)
```

```
[Player left-clicks already-selected own GI]
  ↓
app_input mouse event → app_context_order::try_queue_context_order_at_screen_point
  ↓
target == selected_self && category == Infantry && obj.deploy_fire
  ↓
schedule_command(Command::ToggleInfantryDeploy { entity_id })
  ↓
return true (click consumed)
  ↓
[B1 sim handler: state mutation, SimSoundEvent::EntityDeployed]
  ↓
audio backend plays GIDeploy.wav
```

### Error Handling

- **D-key with empty selection:** existing function early-returns; no change.
- **D-key with non-deploy-fire infantry selected:** UI emits no `ToggleInfantryDeploy` command (the `obj.deploy_fire` check filters). No sim-side no-op needed.
- **Left-click on selected own non-deploy-fire infantry:** branch's `obj.deploy_fire` check fails, falls through to default Move handler. Click is *not* consumed by the deploy branch.
- **Left-click on enemy unit even if it would be a deploy-fire type:** the existing `selected_ids.contains(&target.stable_id)` check fails (target isn't selected — it's an enemy), branch is skipped, falls through to attack handler. Correct.
- **Audio backend missing the sound file:** existing fallback (drop + warn log) — same as DieSound / CrushSound.

### Testing Strategy

Most B2 testing is manual (UI integration is hard to unit-test from scratch). Coverage:

**Manual smoke tests** (run in skirmish):
1. Spawn a GI, press D → deploy animation plays, GIDeploy.wav plays, transitions to deployed-idle posture.
2. Press D again → undeploy animation plays, GIUndeploy.wav, returns to upright Stand.
3. Click an empty cell while deployed → **no movement** (B1 gate). Visual feedback: hovering shows... whatever the existing cursor system does. (Cursor differentiation is out of scope — see Alternatives.)
4. Press D → undeploys. Click cell → moves normally.
5. Multi-select 3 GIs, press D → all three deploy in sync.
6. Mixed selection (2 GIs + 1 MCV) + D → both GIs toggle, MCV deploys to ConYard.
7. Select GI, **left-click on the same GI** → toggles deploy. Mirrors the garrison-building unload UX.
8. Select 5 GIs, left-click on one of them → only that one toggles (per-entity granularity).
9. Force-attack with Ctrl-click on enemy tank while deployed → attack proceeds (force-attack is `ForceAttack` not `AttackMove`; the gate only blocks movement-bearing commands). Confirm faithful behavior.
10. Issue Move on a deployed GI via the AI/script harness → silently ignored. (Manual via debug overlay or scripted scenario.)

**Unit tests:**

| Test | File | Setup | Assertion |
|------|------|-------|-----------|
| `left_click_on_selected_own_gi_emits_toggle` | `app_context_order_tests.rs` | Sim with 1 GI (deploy_fire=true), selected, synthesize left-click on its cell | `ToggleInfantryDeploy { entity_id: gi_id }` is in scheduled commands |
| `left_click_on_selected_own_rifle_no_toggle` | same | Sim with rifle infantry (deploy_fire=false), selected, synthesize click | No `ToggleInfantryDeploy` emitted |
| `left_click_on_unselected_own_gi_no_toggle` | same | Selection empty, synthesize left-click on a friendly GI | No `ToggleInfantryDeploy` (falls to selection logic, not deploy) |
| `left_click_on_enemy_gi_no_toggle` | same | Friendly tank selected, click enemy GI | Falls to attack handler; no `ToggleInfantryDeploy` |
| `dkey_with_mixed_selection_emits_per_entity` | `app_input_tests.rs` (or equivalent) | Sim with 2 GIs + 1 MCV + 1 ConYard, D-key | Exactly 4 commands scheduled: 2× `ToggleInfantryDeploy`, 1× `DeployMcv`, 1× `UndeployBuilding` |
| `dkey_with_non_deployfire_infantry_no_command` | same | Sim with 1 rifle infantry (deploy_fire=false) selected, D-key | No commands emitted |

### Architectural Decisions

- **Pattern followed**: D-key extension mirrors the existing `EntityCategory` match in `queue_deploy_undeploy_for_selected`; left-click handler mirrors the `65f0b1d` garrison-unload pattern; AudioEvent translation mirrors the CrushSound pattern. No new abstractions.
- **No new files**: edits only.
- **No cursor-system changes**: cursor differentiation for "hovering own deployed GI" is treated as out-of-scope. Existing selection-driven cursor behavior covers the common case (player sees the unit is selected). If a deploy-specific cursor variant is needed later, it's a separate slice.
- **Per-entity left-click granularity**: matches stock YR (clicking one infantry in a group selects-only-that-one in some game modes). Matches the `65f0b1d` pattern. Bulk operations stay on the D-key.
- **Force-attack passes through gate unchanged**: `ForceAttack` doesn't set a movement target, so it bypasses the B1 gate. Confirms faithful: gamemd deployed GI can still force-fire when player Ctrl-clicks an enemy.

**Tech debt introduced:** none.

## Alternatives Considered

- **Right-click on own GI to toggle**: rejected by user — the codebase uses left-click-on-selected-self for this kind of toggle (per `65f0b1d`). Right-click is reserved for movement/attack target.
- **Hotkey other than D**: rejected — D-key is the standard YR keybinding for deploy. Reusing the existing D-key handler is the natural fit.
- **Bulk toggle on left-click (one click toggles all selected GIs)**: rejected — the `65f0b1d` precedent is per-entity. Forcing bulk-via-left-click would diverge from that pattern. D-key remains the bulk path.
- **Distinct cursor for "click to undeploy" hover**: deferred. Existing cursor behavior is acceptable; if explicit cursor differentiation is desired, it's an additive cosmetic slice.
- **Eager validation in UI** (block the command from even being sent if mid-transition): rejected — the sim handler already absorbs redundant commands as no-ops (B1 §"mid-transition toggle ignored"). Filtering in UI duplicates that logic and complicates multi-selection bookkeeping.
- **Single combined `EntityDeployToggled` AudioEvent variant**: rejected — distinct sounds (DeploySound vs UndeploySound) need distinct events; combining them and dispatching by phase introduces UI knowledge of sim phase, which isn't necessary.

## Out of Scope

Same as B1's "Out of scope" plus:
- Cursor differentiation for hover-own-deployed-GI.
- "Press D to cancel a queued deploy mid-anim" UX (B1 already says mid-transition toggle is ignored — same semantic).
- Undeploy-confirmation prompt (RTS UX adds friction — not how stock YR works).
