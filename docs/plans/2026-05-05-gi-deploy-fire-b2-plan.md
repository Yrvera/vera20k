# GI Deploy-Fire — Slice B2 (UI integration) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire the player input paths to B1's `Command::ToggleInfantryDeploy` — extend the D-key handler to include deploy-fire infantry, extend the existing left-click-on-already-selected-self deploy branch to recognize deploy-fire infantry, and translate `SimSoundEvent::EntityDeployed` / `EntityUndeployed` into `GameSoundEvent` for spatial playback. Pure wiring on top of B1 — no new sim logic.

**Architecture:** Three-site additive edit. (1) `GameSoundEvent` gains two spatial variants (mirrors `EntityDestroyed` / `EntityCrushed`). (2) `app_sim_tick` replaces B1's stub `log::trace!` arms with real translation (mirrors the `iso_to_screen` pattern at lines 282–316). (3) `app_input::queue_deploy_undeploy_for_selected` gains an `Infantry` match arm (mirrors the existing `Structure` and default `_` arms). (4) `app_context_order`'s existing deploy-on-self-click branch (lines 371–415) extends its non-Structure arm with a deploy-fire-infantry case before the existing MCV-deployer case. The audio dispatch consumer in `app_building_anim::drain_sound_events` requires NO changes — its wildcard `_ =>` arm at line 462 catches new spatial variants automatically via the `sound_id()` / `screen_pos()` accessors.

**Design Doc:** [docs/plans/2026-05-05-gi-deploy-fire-b2-design.md](2026-05-05-gi-deploy-fire-b2-design.md)

**Sibling B1 (already merged):** [docs/plans/2026-05-05-gi-deploy-fire-b1-plan.md](2026-05-05-gi-deploy-fire-b1-plan.md). All B1 prerequisites are landed on `dev`: `Command::ToggleInfantryDeploy`, `ObjectType.deploy_fire`, sim handler with INI gate + sound emission, Set_Destination gate on 7 movement commands, hash inclusion + 17 unit tests, snapshot version bump 2 → 3.

---

## Grounding Summary

- **R1 — ra2-rust-game-docs:** B2 introduces no new RE; B1 already cited `GI_GHIDRA_REPORT.md §§ D-T.0–D-T.12`. Nothing to re-research.
- **R2 — Ghidra verification:** Not applicable. UI input wiring doesn't reverse-engineer anything new. The toggle command + Set_Destination gate were verified in B1.
- **R3 — Repo patterns mirrored:**
  - **D-key:** `queue_deploy_undeploy_for_selected` at [src/app_input.rs:658-698](../../src/app_input.rs) — already structured as `match entity.category { Structure => ..., _ => MCV-deployer }`. The new `Infantry` arm slots between them.
  - **Left-click on selected own structure:** commit `65f0b1d` ("garrison: left-click on selected garrisoned building unloads occupants") added a structure-only self-click branch at [src/app_context_order.rs:166-202](../../src/app_context_order.rs). The MORE general deploy-on-self-click branch is at [src/app_context_order.rs:371-415](../../src/app_context_order.rs) and already handles Structure (unload / undeploy) and non-Structure (MCV-deployer) — the new infantry case slots into the non-Structure arm.
  - **AudioEvent translation:** `SimSoundEvent::EntityCrushed` → `GameSoundEvent::EntityCrushed` at [src/app_sim_tick.rs:306-316](../../src/app_sim_tick.rs) is the canonical mirror. Uses `crate::map::terrain::iso_to_screen(rx, ry, 0)` to compute screen_pos.
  - **Spatial dispatch is wildcard-driven:** `app_building_anim::drain_sound_events` at [src/app_building_anim.rs:462](../../src/app_building_anim.rs) routes any non-voice/non-EVA/non-UI variant through the spatial dispatch path via `event.sound_id()` and `event.screen_pos()`. New spatial variants need only extend those two accessors.
- **R4 — INI keys:** All B2-relevant keys are already parsed by B1 — `DeployFire=` → `ObjectType.deploy_fire`, `DeploySound=` → `ObjectType.deploy_sound`, `UndeploySound=` → `ObjectType.undeploy_sound`. No new INI parsing.
- **Open after grounding:** Mixed structure+infantry selection, where the user clicks a deployed-fire-eligible infantry and `structure_owner` is set, currently falls through to rally-set + Move (the structure_owner branch's inner check at line 174 only handles `EntityCategory::Structure`). B2 leaves this edge case as-is per the design's "per-entity granularity" note — see Open Questions.

## Key Technical Decisions

- **Two new spatial `GameSoundEvent` variants, not one combined.** `EntityDeployed` and `EntityUndeployed` carry different `sound_id`s and convey different player-visible feedback. Combining them would push sim-phase knowledge into the audio layer, which the design explicitly rejects. **Confidence:** high — **Source:** design doc §"Architectural Decisions" + repo precedent (DeploySound vs UndeploySound are distinct keys at [src/rules/object_type.rs:227-231](../../src/rules/object_type.rs)).
- **Wildcard dispatch in `drain_sound_events` means no audio backend edit is needed.** Adding new `GameSoundEvent` variants automatically routes through the spatial path at [src/app_building_anim.rs:462-483](../../src/app_building_anim.rs) provided `sound_id()` and `screen_pos()` are extended. **Confidence:** high — **Source:** code inspection.
- **Self-click branch extends only the canonical at line 371, not the structure_owner branch at line 166.** The structure_owner branch fires only when the selection contains structures; clicking an infantry in a mixed-selection case is a niche edge that the design doesn't cover. **Confidence:** high — **Source:** design doc §"Per-entity granularity" + code inspection.
- **No automated unit tests for B2.** B1 covered the sim state machine exhaustively (17 tests). B2's edits are wiring — `cargo check` catches enum-arm-coverage and import errors; manual smoke tests cover runtime behavior. The design lists 6 manual smoke tests plus 6 proposed unit tests; the unit tests would each require constructing a synthetic `AppState` (camera, render context, asset manager, sound registry), which is heavy infrastructure for low marginal value. **Confidence:** medium — **Source:** design doc §"Testing Strategy" ("Most B2 testing is manual"). If runtime smoke tests reveal regressions, follow up with focused integration tests.
- **`tactical.world_to_screen` referenced in design is a name typo for the actual helper `crate::map::terrain::iso_to_screen`.** All four existing spatial-event translation arms in `app_sim_tick.rs` use `iso_to_screen(rx, ry, 0)`. **Confidence:** high — **Source:** [src/app_sim_tick.rs:289,300,311](../../src/app_sim_tick.rs).
- **`AudioEvent` referenced in design is a name typo for `GameSoundEvent`.** Repo type is named `GameSoundEvent` and lives in `src/audio/events.rs`. **Confidence:** high — **Source:** [src/audio/events.rs:22](../../src/audio/events.rs).

## Open Questions

### Resolved During Planning

- **Where is the audio backend that consumes `GameSoundEvent`?** `app_building_anim::drain_sound_events` at [src/app_building_anim.rs:413-486](../../src/app_building_anim.rs). Routes voice / EVA / UI events to dedicated paths and falls through to `_ =>` for spatial events using `event.sound_id()` / `event.screen_pos()`. New spatial variants need no edit there.
- **Does the existing self-click branch at line 371 fire for infantry?** Yes — it lives inside the bottom `else` of the click-resolution chain (`structure_owner` is None for unit-only selections). The bug-free path is: infantry GI selected + click on the same GI → falls through `clicked_friendly_refinery`, `clicked_ore`, `structure_owner`, garrison-entry, engineer-capture, then hits the self-click branch.
- **Is the design's `tactical.world_to_screen` a real helper?** No — the actual helper is `crate::map::terrain::iso_to_screen`. Plan corrects.
- **Is the design's `AudioEvent` the real type name?** No — actual type is `GameSoundEvent`. Plan corrects.
- **Does adding `GameSoundEvent` variants break anything?** It breaks the exhaustive `match self` blocks in `impl GameSoundEvent::sound_id()` and `screen_pos()`. Both use the unified `Self::A | Self::B { sound_id, .. } => sound_id` shape and are extended in Task 1.

### Deferred to Implementation

- **Mixed structure+infantry selection self-click on infantry.** When a user has selected at least one structure and clicks a selected deploy-fire infantry, the `structure_owner` branch at [src/app_context_order.rs:159](../../src/app_context_order.rs) fires first. Its inner self-click filter at line 174 (`entity.category == EntityCategory::Structure`) means the click falls through to rally-set + Move. The infantry doesn't deploy. Niche but inconsistent with single-selection behavior. Out of scope for B2 per the design's "per-entity granularity" framing; if a follow-up wants to fix it, extend the structure_owner self-click filter with the same Infantry arm (mirror of Task 4).
- **Cursor differentiation for hovering own deployed GI.** Out of scope per design §"Out of Scope". If desired later, becomes a separate cosmetic slice.
- **Whether AI-issued deploy commands should also be wired in B2.** Out of scope — B1's design covered AI auto-deploy as deferred. B2 is player-input-only.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/audio/events.rs](../../src/audio/events.rs) | +2 `GameSoundEvent` variants + extend `sound_id()` / `screen_pos()` match arms |
| Modify | [src/app_sim_tick.rs](../../src/app_sim_tick.rs) | Replace B1 stub `log::trace!` arms (lines 317-342) with real translation |
| Modify | [src/app_input.rs](../../src/app_input.rs) | Extend `queue_deploy_undeploy_for_selected` (line 674) with `Infantry` match arm |
| Modify | [src/app_context_order.rs](../../src/app_context_order.rs) | Extend deploy-on-self-click branch (line 396-405) with deploy-fire-infantry case |

## Interface Changes

**Public additions (additive — no existing callers break):**
- `GameSoundEvent::EntityDeployed { sound_id: String, screen_pos: Option<(f32, f32)> }`
- `GameSoundEvent::EntityUndeployed { sound_id: String, screen_pos: Option<(f32, f32)> }`

**Exhaustive matches that gain arms:**
- `match self { ... }` in `impl GameSoundEvent::sound_id()` at [src/audio/events.rs:108-123](../../src/audio/events.rs) — adds 2 patterns to the unified or-arm.
- `match self { ... }` in `impl GameSoundEvent::screen_pos()` at [src/audio/events.rs:126-134](../../src/audio/events.rs) — adds 2 explicit arms (these have spatial data, so they extract `screen_pos`).
- `match sim_event { ... }` in `app_sim_tick.rs` at line 283 — replaces 2 stub arms with real translation arms.
- `match entity.category { ... }` in `app_input.rs::queue_deploy_undeploy_for_selected` at line 674 — adds 1 arm (`Infantry`).

**No interface changes:**
- `Command` enum — already includes `ToggleInfantryDeploy` from B1.
- `ObjectType` — already includes `deploy_fire` from B1.
- `SimSoundEvent` — already includes `EntityDeployed` / `EntityUndeployed` from B1.
- The audio dispatch consumer (`drain_sound_events`) — wildcard dispatch picks up new spatial variants automatically.

## Sim Checklist

(Not applicable — B2 touches no `sim/` files. The B1 sim core handles all deterministic logic; B2 is pure UI/audio wiring above the sim boundary.)

## Risk Areas

- **Self-click branch ordering.** The existing branch at line 371 is reached only after garrison-entry (line 248) and engineer-capture (line 308) checks have failed. For an infantry GI selected + clicking the same GI: garrison-entry's `garrison_target` is `None` (the GI itself isn't a garrisonable building), engineer-capture's `capture_target` is `None` (it's not an enemy structure). So the self-click branch fires correctly. Verified by reading the chain.
- **D-key with mixed selection (e.g., 3 GIs + 1 MCV + 1 ConYard) emits one command per entity.** B1 sim handlers absorb redundant or no-op commands silently — the existing `Structure` arm of `queue_deploy_undeploy_for_selected` already issues `UnloadPassengers` or `UndeployBuilding`, the new `Infantry` arm issues `ToggleInfantryDeploy`, the default `_` arm issues `DeployMcv`. All three fire independently in the same tick.
- **Mid-transition spam (player presses D while a GI is mid-deploy).** B1's sim handler already returns `false` for mid-transition toggles (covered by tests `mid_deploying_toggle_ignored` / `mid_undeploying_toggle_ignored`). UI does not need to filter.
- **Force-attack on enemy unit while deployed.** `Command::ForceAttack` does not flow through any of the 7 gated movement commands. Force-fire works as expected with no B2 changes — Ctrl-click stays functional.
- **Re-rendering the static frame during deployment.** Animation cascade was rewritten in B1 to switch sequences based on `deploy_state`. The 29 existing animation tests pass after the rewrite (verified in B1 Task 11). No B2 risk here.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | DeploySound / UndeploySound plays at the GI's screen position with spatial volume scaling | Stock YR plays "GIDeploy" / "GIUndeploy" positionally — the player hears louder if camera is over the deploying GI, fades if off-screen. Fires every time a GI deploys/undeploys (frequent in normal play). | Manual smoke test in skirmish: select GI, press D, listen for spatial GIDeploy.wav at the GI's location. Compare to gamemd.exe behavior. |
| Task 3 | D-key with multi-select including GIs, MCVs, and ConYards in the same selection issues exactly one command per entity, all fire same tick | Stock YR semantic: D is bulk-deploy. Misclassifying or skipping any entity type would surprise the player. Fires on every group-deploy (common in macro play). | Manual smoke test 6: select 2 GIs + 1 MCV + 1 ConYard, press D, observe 4 simultaneous animations. |
| Task 4 | Single GI selected, left-click on the same GI, toggles deploy | Mirror of the structure-self-click pattern from `65f0b1d`. Stock YR doesn't natively support left-click-self-deploy for infantry (it does for ConYards), but the design adopts this UX deliberately to mirror the unload pattern. Player visibility: high (every time they want to deploy a single GI without using the keyboard). | Manual smoke test 7: select GI, click on its cell, observe deploy. |
| Task 4 | Multi-select 5 GIs, left-click on one of them, only that one toggles | Per-entity granularity matches the `65f0b1d` precedent. Bulk operations stay on the D-key (Task 3). Player visibility: medium-high — wrong behavior would confuse anyone who used the D-key path in parallel. | Manual smoke test 8. |
| Task 4 | Click on empty cell while deployed → no movement | B1 sim gate (Set_Destination) blocks Move at the sim layer. UI doesn't filter — the command flows through and the sim absorbs it as a `false` return. Player visibility: very high — happens any time a deployed GI is given a move order. | Manual smoke test 3 + 4 + 10. |

---

## Tasks

### Task 1: Add `EntityDeployed` and `EntityUndeployed` `GameSoundEvent` variants

**Why:** Audio-side data type for the two new sim events. Mirrors `EntityDestroyed` / `EntityCrushed` shape (sound_id + screen_pos). No audio backend edit is needed because `drain_sound_events` dispatches new spatial variants through its wildcard path automatically.

**Files:**
- Modify: [src/audio/events.rs](../../src/audio/events.rs) — add 2 enum variants (after `EntityCrushed` at line 64), extend `sound_id()` match (line 108-123), extend `screen_pos()` match (line 126-134).

**Pattern:** Mirrors `GameSoundEvent::EntityCrushed` exactly — same field shape (`sound_id: String, screen_pos: Option<(f32, f32)>`), same accessor pattern.

**Step 1: Add the two enum variants**

In [src/audio/events.rs](../../src/audio/events.rs), immediately after the `EntityCrushed { ... }` block ending around line 64, insert:

```rust
    /// An infantry entity entered the Deploying phase — play DeploySound.
    EntityDeployed {
        /// sound.ini ID from the entity's DeploySound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        screen_pos: Option<(f32, f32)>,
    },

    /// An infantry entity entered the Undeploying phase — play UndeploySound.
    EntityUndeployed {
        /// sound.ini ID from the entity's UndeploySound= field.
        sound_id: String,
        /// Screen position of the sound source (for spatial audio).
        screen_pos: Option<(f32, f32)>,
    },
```

**Step 2: Extend `sound_id()` accessor**

In `impl GameSoundEvent::sound_id`, locate the existing or-arm chain (lines 109-122) and extend it. The current chain ends with `| Self::BuildingGarrisonedSfx { sound_id, .. } => sound_id`. Add the two new variants to the chain:

```rust
    pub fn sound_id(&self) -> &str {
        match self {
            Self::WeaponFired { sound_id, .. }
            | Self::UnitSelected { sound_id }
            | Self::UnitMoveOrder { sound_id }
            | Self::UnitAttackOrder { sound_id }
            | Self::EntityDestroyed { sound_id, .. }
            | Self::EntityCrushed { sound_id, .. }
            | Self::EntityDeployed { sound_id, .. }
            | Self::EntityUndeployed { sound_id, .. }
            | Self::BuildingReady { sound_id }
            | Self::UnitReady { sound_id }
            | Self::UiSound { sound_id }
            | Self::StructureGarrisoned { sound_id }
            | Self::StructureAbandoned { sound_id }
            | Self::BuildingGarrisonedSfx { sound_id, .. } => sound_id,
        }
    }
```

**Step 3: Extend `screen_pos()` accessor**

In `impl GameSoundEvent::screen_pos`, the existing block lists explicit arms for spatial events and a `_ => None` fallback. Add the two new arms before the fallback:

```rust
    pub fn screen_pos(&self) -> Option<(f32, f32)> {
        match self {
            Self::WeaponFired { screen_pos, .. } => *screen_pos,
            Self::EntityDestroyed { screen_pos, .. } => *screen_pos,
            Self::EntityCrushed { screen_pos, .. } => *screen_pos,
            Self::EntityDeployed { screen_pos, .. } => *screen_pos,
            Self::EntityUndeployed { screen_pos, .. } => *screen_pos,
            Self::BuildingGarrisonedSfx { screen_pos, .. } => *screen_pos,
            _ => None,
        }
    }
```

**Step 4: Verify**

Run: `cargo check --lib`

Expected: clean. The new variants are unused by their producer (Task 2) but the enum compiles standalone.

**Step 5: Commit**

```
git add src/audio/events.rs
git commit -m "audio: add EntityDeployed and EntityUndeployed GameSoundEvent variants"
```

---

### Task 2: Replace B1 stub arms in `app_sim_tick.rs` with real translation

**Why:** Translates `SimSoundEvent::EntityDeployed` / `EntityUndeployed` (emitted by B1's toggle handler) into the matching `GameSoundEvent` variants for spatial playback. Removes B1's `log::trace!` placeholders.

**Files:**
- Modify: [src/app_sim_tick.rs:317-342](../../src/app_sim_tick.rs) — replace 2 stub arms.

**Pattern:** Mirrors `SimSoundEvent::EntityCrushed` → `GameSoundEvent::EntityCrushed` translation at [src/app_sim_tick.rs:306-316](../../src/app_sim_tick.rs).

**Step 1: Replace the stub arms**

In [src/app_sim_tick.rs](../../src/app_sim_tick.rs), locate the existing B1 stub arms (currently `log::trace!` + `continue;` for both `EntityDeployed` and `EntityUndeployed`, around lines 317-342):

```rust
                    SimSoundEvent::EntityDeployed {
                        deploy_sound_id,
                        rx,
                        ry,
                    } => {
                        log::trace!(
                            "B1 stub: EntityDeployed sound={} at ({}, {}) — B2 will translate",
                            sim.interner.resolve(deploy_sound_id),
                            rx,
                            ry,
                        );
                        continue;
                    }
                    SimSoundEvent::EntityUndeployed {
                        undeploy_sound_id,
                        rx,
                        ry,
                    } => {
                        log::trace!(
                            "B1 stub: EntityUndeployed sound={} at ({}, {}) — B2 will translate",
                            sim.interner.resolve(undeploy_sound_id),
                            rx,
                            ry,
                        );
                        continue;
                    }
```

Replace with real translation:

```rust
                    SimSoundEvent::EntityDeployed {
                        deploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityDeployed {
                            sound_id: sim.interner.resolve(deploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityUndeployed {
                        undeploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityUndeployed {
                            sound_id: sim.interner.resolve(undeploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
```

Note: the surrounding `match` arms produce `app_event: GameSoundEvent` which is later pushed onto `state.sound_events` — these arms now produce values rather than `continue`-ing, matching the EntityCrushed pattern.

**Step 2: Verify**

Run: `cargo check --lib`

Expected: clean. The translation now references the new variants from Task 1.

**Step 3: Commit**

```
git add src/app_sim_tick.rs
git commit -m "app: translate EntityDeployed/EntityUndeployed sim sounds to GameSoundEvent"
```

---

### Task 3: Extend D-key handler with `Infantry` match arm

**Why:** Wires D-key press to `Command::ToggleInfantryDeploy` for selected deploy-fire infantry. Existing handler already routes Structure (garrison/ConYard) and `_` (MCV); Infantry is the missing arm.

**Files:**
- Modify: [src/app_input.rs:674-693](../../src/app_input.rs) — extend the existing `match entity.category` block.

**Pattern:** Mirrors the existing `EntityCategory::Structure` arm in the same function — same shape (push a `Command` onto the local `commands` vec for later scheduling).

**Step 1: Add the Infantry arm**

In [src/app_input.rs](../../src/app_input.rs), locate the existing match block in `queue_deploy_undeploy_for_selected` (lines 674-692):

```rust
            match entity.category {
                crate::map::entities::EntityCategory::Structure => {
                    // Garrisoned building → evacuate occupants.
                    if obj.map_or(false, |o| o.can_be_occupied)
                        && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                    {
                        commands.push(Command::UnloadPassengers {
                            transport_id: entity_id,
                        });
                    } else if obj.map_or(false, |o| o.undeploys_into.is_some()) {
                        commands.push(Command::UndeployBuilding { entity_id });
                    }
                }
                _ => {
                    if obj.map_or(false, |o| o.deploys_into.is_some()) {
                        commands.push(Command::DeployMcv { entity_id });
                    }
                }
            }
```

Replace with:

```rust
            match entity.category {
                crate::map::entities::EntityCategory::Structure => {
                    // Garrisoned building → evacuate occupants.
                    if obj.map_or(false, |o| o.can_be_occupied)
                        && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                    {
                        commands.push(Command::UnloadPassengers {
                            transport_id: entity_id,
                        });
                    } else if obj.map_or(false, |o| o.undeploys_into.is_some()) {
                        commands.push(Command::UndeployBuilding { entity_id });
                    }
                }
                crate::map::entities::EntityCategory::Infantry => {
                    // Deploy-fire infantry (GI, GuardianGI, etc.) → toggle deploy.
                    if obj.map_or(false, |o| o.deploy_fire) {
                        commands.push(Command::ToggleInfantryDeploy { entity_id });
                    }
                }
                _ => {
                    if obj.map_or(false, |o| o.deploys_into.is_some()) {
                        commands.push(Command::DeployMcv { entity_id });
                    }
                }
            }
```

**Step 2: Verify**

Run: `cargo check --lib`

Expected: clean. `Command::ToggleInfantryDeploy` is already in scope via the existing `Command` import; `EntityCategory::Infantry` is reached via the same `crate::map::entities::EntityCategory` path used by the Structure arm.

**Step 3: Commit**

```
git add src/app_input.rs
git commit -m "app: D-key toggles deploy on selected DeployFire infantry"
```

---

### Task 4: Extend deploy-on-self-click with deploy-fire-infantry case

**Why:** Wires "left-click on already-selected own GI" to `Command::ToggleInfantryDeploy`, mirroring the structure-self-click UX from commit `65f0b1d`. Reuses the existing self-click branch — only its non-Structure arm needs an Infantry case before the existing MCV-deployer fallthrough.

**Files:**
- Modify: [src/app_context_order.rs:380-405](../../src/app_context_order.rs) — extend the cmd selection inside the existing self-click branch.

**Pattern:** Mirrors the existing self-click-Structure logic in the same branch, plus the D-key Infantry filter from Task 3 (`obj.map_or(false, |o| o.deploy_fire)`).

**Step 1: Extend the cmd selection block**

In [src/app_context_order.rs](../../src/app_context_order.rs), locate the existing deploy-on-self-click branch (lines 380-405). The current shape:

```rust
                            let cmd = if entity.category == EntityCategory::Structure {
                                // Garrisoned building → unload occupants.
                                if obj.map_or(false, |o| o.can_be_occupied)
                                    && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                                {
                                    Some(Command::UnloadPassengers {
                                        transport_id: target.stable_id,
                                    })
                                // ConYard → MCV
                                } else if obj.map_or(false, |o| o.undeploys_into.is_some()) {
                                    Some(Command::UndeployBuilding {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            } else {
                                // MCV → ConYard
                                if obj.map_or(false, |o| o.deploys_into.is_some() || o.deployer) {
                                    Some(Command::DeployMcv {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            };
```

Replace with:

```rust
                            let cmd = if entity.category == EntityCategory::Structure {
                                // Garrisoned building → unload occupants.
                                if obj.map_or(false, |o| o.can_be_occupied)
                                    && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                                {
                                    Some(Command::UnloadPassengers {
                                        transport_id: target.stable_id,
                                    })
                                // ConYard → MCV
                                } else if obj.map_or(false, |o| o.undeploys_into.is_some()) {
                                    Some(Command::UndeployBuilding {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            } else if entity.category == EntityCategory::Infantry
                                && obj.map_or(false, |o| o.deploy_fire)
                            {
                                // Deploy-fire infantry (GI, GGI, etc.) → toggle deploy.
                                Some(Command::ToggleInfantryDeploy {
                                    entity_id: target.stable_id,
                                })
                            } else {
                                // MCV → ConYard
                                if obj.map_or(false, |o| o.deploys_into.is_some() || o.deployer) {
                                    Some(Command::DeployMcv {
                                        entity_id: target.stable_id,
                                    })
                                } else {
                                    None
                                }
                            };
```

**Step 2: Verify**

Run: `cargo check --lib`

Expected: clean. `Command::ToggleInfantryDeploy` is already in scope via the existing `Command` import in this file (line 22).

**Step 3: Commit**

```
git add src/app_context_order.rs
git commit -m "app: left-click on selected own DeployFire infantry toggles deploy"
```

---

### Task 5: Full regression + manual smoke test

**Why:** Confirm B2 is integrated cleanly, no existing tests regressed, and runtime behavior matches expectations. B2 introduces no new automated unit tests (per design + Key Decisions); runtime validation is via the design's manual smoke test list.

**Files:** none (build/test/run only).

**Step 1: Full check**

Run: `cargo check --workspace --all-targets`

Expected: zero errors and no new warnings beyond the pre-existing ones in `app_selection_brackets.rs` and `bik_player_audio.rs` baseline noise.

**Step 2: Full test suite**

Run: `cargo test --lib`

Expected: 1483 passing tests (B1 baseline). No test should regress — B2 touches no `sim/` or `audio/events.rs` test files except by extending enum variants.

If `tests::test_sound_id_accessor` or other `audio/events.rs` tests added by an unrelated parallel session reference the new variants, they will pass automatically because both new variants share the same `sound_id` field shape as `EntityDestroyed`.

**Step 3: Manual smoke tests** — run in skirmish (use the existing dev launch flow). For each, observe the expected behavior:

1. **Single GI deploy via D-key:** Select a GI, press D. Expected: deploy animation plays (Deploy frames from artmd.ini `[GISequence]`), GIDeploy.wav plays positionally at the GI's location, GI ends in stationary `Deployed` posture.

2. **Single GI undeploy via D-key:** With the GI deployed, press D again. Expected: undeploy animation, GIUndeploy.wav, GI returns to upright Stand.

3. **Movement blocked while deployed:** Right-click an empty cell while deployed. Expected: no movement (B1 Set_Destination gate). The unit stays in place.

4. **Movement works after undeploy:** Press D to undeploy, wait for the undeploy animation to finish, then right-click an empty cell. Expected: GI moves normally.

5. **Multi-select GIs:** Select 3 GIs, press D. Expected: all three deploy in sync, three GIDeploy.wav instances play (all spatially positioned at each GI).

6. **Mixed selection + D-key:** Select 2 GIs + 1 MCV + 1 ConYard, press D. Expected: 2× ToggleInfantryDeploy (GIs deploy) + 1× DeployMcv (MCV → ConYard) + 1× UndeployBuilding (ConYard → MCV) all fire same tick.

7. **Single GI deploy via left-click on self:** Select a GI, left-click on that GI's cell. Expected: GI toggles deploy. Mirrors the structure-unload UX from `65f0b1d`.

8. **Per-entity left-click granularity:** Select 5 GIs, left-click on one of them. Expected: only that one GI toggles deploy, the other 4 stay in their current state. Bulk operations remain on the D-key (test 5).

9. **Force-attack passes through gate:** While a GI is deployed, Ctrl-click an enemy tank. Expected: GI fires at the tank (force-attack uses `Command::ForceAttack`, not movement-bearing commands, so the B1 gate doesn't fire). This is parity-faithful per gamemd.exe.

10. **Mid-transition spam:** Select a GI, press D rapidly. Expected: the deploy animation plays once cleanly. Subsequent D presses during the Deploying / Undeploying phase are silently no-op'd by the B1 sim handler (`mid_deploying_toggle_ignored` test). No phase storm, no extra sound events.

**Step 4: No commit** — verification only. B2 ships when manual tests all pass.

If any manual test fails, do NOT layer fixes on top — diagnose the root cause first (per CLAUDE.md "If a fix makes things worse, STOP and reassess"). Common failure modes:
- D-key fires no command for GI: Task 3's Infantry arm isn't reached. Check `entity.category` resolution.
- D-key fires the wrong command: `obj.deploy_fire` returns false. Check rules.ini parse and B1 Task 1 (`section.get_bool("DeployFire")`).
- No DeploySound plays: Either `obj.deploy_sound` is None (rules.ini missing the entry — check stock `[E1] DeploySound=GIDeploy`), or the SoundRegistry doesn't resolve "GIDeploy" → file. Check `sound.ini` lookup at [src/audio/sfx.rs:158-185](../../src/audio/sfx.rs).
- Sound plays but wrong volume / position: `iso_to_screen(rx, ry, 0)` z-coord might need to be the GI's actual z. Mirror the EntityCrushed translation if in doubt — both pass z=0.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-gi-deploy-fire-b2-design.md](2026-05-05-gi-deploy-fire-b2-design.md)
- **Sibling B1 plan:** [docs/plans/2026-05-05-gi-deploy-fire-b1-plan.md](2026-05-05-gi-deploy-fire-b1-plan.md)
- **Ghidra reports (B1 baseline):** [ra2-rust-game-docs/GI_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GI_GHIDRA_REPORT.md) §§ D-T.0–D-T.12. B2 itself adds no new RE.
- **gamemd.exe addresses (B1 baseline, kept here):**
  - `FUN_0051F6E0` — `InfantryClass::Mission_Deploy_Toggle` (mission 0x10 dispatch)
  - `FUN_0051AA40` — `InfantryClass::Set_Destination` with player+deploy early-return
- **Prior commits:**
  - `65f0b1d` — garrison: left-click on selected garrisoned building unloads occupants. The canonical pattern Task 4 mirrors.
  - `67d5fc5` — audio: emit CrushSound + DieSound when a vehicle crushes infantry. The canonical translation pattern Task 2 mirrors.
  - `a3302db` — app: stub EntityDeployed/EntityUndeployed sound translation for B1. The stubs Task 2 replaces.
  - B1 final commits on `dev` (Tasks 1–14, ending with `323a494 tests: 17-test suite for GI deploy-fire state machine (B1)`).
- **INI keys driving B2 behavior (already parsed by B1):**
  - rulesmd.ini `[E1] DeployFire=yes`, `DeploySound=GIDeploy`, `UndeploySound=GIUndeploy`.
- **Related code (read for plan):**
  - [src/audio/events.rs:22-104](../../src/audio/events.rs) — `GameSoundEvent` enum and accessors.
  - [src/audio/events.rs:106-135](../../src/audio/events.rs) — `sound_id()` and `screen_pos()` impls (extended in Task 1).
  - [src/app_sim_tick.rs:282-342](../../src/app_sim_tick.rs) — `SimSoundEvent` translation arms; lines 317-342 are B1 stubs replaced in Task 2.
  - [src/app_input.rs:658-698](../../src/app_input.rs) — `queue_deploy_undeploy_for_selected`; line 674 is the match site extended in Task 3.
  - [src/app_context_order.rs:371-415](../../src/app_context_order.rs) — deploy-on-self-click branch; lines 380-405 are the cmd-selection block extended in Task 4.
  - [src/app_building_anim.rs:413-486](../../src/app_building_anim.rs) — `drain_sound_events` audio dispatch; needs no edit (wildcard catches new spatial variants).
  - [src/audio/sfx.rs:155-260](../../src/audio/sfx.rs) — `SfxPlayer` lookup and playback (no edits).
- **Repo precedent commits referenced:** `65f0b1d`, `67d5fc5`, `a3302db`.
