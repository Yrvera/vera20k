# GI Deploy-Fire — Slice B1 (sim core) Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Land the sim-side state machine for GI deploy/undeploy: a hashed `DeployPhase` field on `GameEntity`, a `Command::ToggleInfantryDeploy`, the parity-faithful `Set_Destination` gate (player + deployed silently ignores Move/AttackMove/Enter/etc.), and Deploy/Undeploy sound emission. End-to-end testable via direct `Command::ToggleInfantryDeploy` — no UI integration. UI wiring is B2.

**Architecture:** Three-phase enum (`Deploying / Deployed / Undeploying`) on `GameEntity`. `DeployedFire` stays a visual sub-state of `Deployed`, driven by the existing `attack_target` auto-transition in `tick_animations`. Toggle command flows through normal command dispatch. Set_Destination gate is keyed on `is_deployed()` and replicated across the 7 movement-bearing handlers. Sound emission via existing `SimSoundEvent` channel, with `log::trace!` stubs in `app_sim_tick` (B2 replaces with real audio translation). State hashed for replay/lockstep parity. Snapshot version bumped 2 → 3.

**Design Doc:** [docs/plans/2026-05-05-gi-deploy-fire-b1-design.md](2026-05-05-gi-deploy-fire-b1-design.md)

**Sibling Slice (NOT in scope):** [docs/plans/2026-05-05-gi-deploy-fire-b2-design.md](2026-05-05-gi-deploy-fire-b2-design.md)

---

## Grounding Summary

- **Existing RE research:** [GI_GHIDRA_REPORT.md §§ D-T.0–D-T.12](../../../ra2-rust-game-docs/GI_GHIDRA_REPORT.md) — structurally complete for the deploy mechanic. HIGH-confidence findings load-bearing for B1: D-T.1 (player path = action 4 → mission 0x10 → `FUN_0051F6E0` toggle); D-T.2 (no auto-undeploy on Move; `InfantryClass::Set_Destination @ 0x0051AA40` early-returns when `IsPlayerControl(owner) && current_seq ∈ {0x1B-0x1E}`); D-T.4 (weapon swap is target-driven — `select_weapon_with_ifv` reads no deploy state, confirmed in [combat_weapon.rs:96](../../src/sim/combat/combat_weapon.rs)); D-T.5 (no cell-validity gate — bridge / sub-cell / water are permissive).
- **Existing animation infra in repo:** `SequenceKind::{Deploy, Undeploy, Deployed, DeployedFire, DeployedIdle}` already exist at [src/sim/animation.rs:93-102](../../src/sim/animation.rs); art.ini parsing populates them at [src/rules/infantry_sequence.rs:190-194](../../src/rules/infantry_sequence.rs); `tick_animations()` already auto-transitions `Deployed → DeployedFire` when `attack_target.is_some()` at [animation.rs:438-440](../../src/sim/animation.rs) (this stays unchanged for B1 — we add a higher-priority cascade above it).
- **Existing INI parsing:** `DeploySound` / `UndeploySound` already on `ObjectType` at [src/rules/object_type.rs:227-231,723-724](../../src/rules/object_type.rs). `DeployFire` and `DeployFireWeapon` are NOT yet parsed — gap closed in Task 1.
- **Repo pattern mirrored for sound emission:** `SimSoundEvent::EntityCrushed { crush_sound_id: InternedId, rx, ry }` ([world/mod.rs:104](../../src/sim/world/mod.rs)) + matching translation arm in [app_sim_tick.rs:306-316](../../src/app_sim_tick.rs). The two new variants and their stub arms mirror this exactly.
- **Repo pattern mirrored for entity state:** `attack_target: Option<AttackTarget>`, `dock_state: Option<DockState>`, `passenger_role: PassengerRole` — all show the "Option<subsystem state> on `GameEntity`, `#[serde(default)]` for forward compat, helper predicates on `impl GameEntity`" shape.
- **Tick order in `advance_tick`:** [world/mod.rs:995-1387](../../src/sim/world/mod.rs). Phase 4.5 (superweapons) ends at line 1168; Phase 5 (turret rotation + combat) starts at line 1170. `tick_deploy_state` slots between them at the new "Phase 4.6", inside the `if let Some(rules) = rules` block (line 1152), so command-issued toggles materialize the same tick they're issued and combat reads the up-to-date `deploy_state` later in Phase 5.
- **State hash in [world_hash.rs:273-390](../../src/sim/world/world_hash.rs)** already iterates entities in stable_id order via BTreeMap. New `deploy_state` hash block joins the existing per-entity hash sequence.
- **Snapshot version constant** at [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs): `SNAPSHOT_VERSION: u32 = 2`. Bumped to 3 in Task 13. Existing tests `round_trip_preserves_state_hash` and `version_mismatch_is_rejected` validate the bump.
- **INI keys driving B1 behavior:** rulesmd.ini `[E1] DeployFire=yes`, `DeploySound=GIDeploy`, `UndeploySound=GIUndeploy`, `IFVMode=2`. `DeployFireWeapon=` is unset on E1 (defaults to -1 / `None`). artmd.ini `[GISequence] Deploy=300,15,0`, `Deployed=292,1,1`, `DeployedFire=315,6,6`, `Undeploy=276,2,2`. Stock GI Deploy = 15 frames × DEFAULT_TRANSITION_TICK_MS (80ms) = 1200ms (≈55 sim ticks at SIM_TICK_MS=22) — `DEPLOY_DEFAULT_TICKS = 55` sized to match.
- **Logging crate:** the codebase uses `log` (not `tracing`) — verified via Grep. Stub arms use `log::trace!`, not `tracing::trace!` as the design doc casually suggested.
- **Open after grounding:** `compute_anim_ticks` cannot reach the `BTreeMap<String, SequenceSet>` from `apply_command` (sequences live in `AppState`, not `Simulation`). B1 uses a constant fallback sized for stock GI (55 ticks ≈ 1.21s, matching `Deploy=300,15,0` × DEFAULT_TRANSITION_TICK_MS at SIM_TICK_MS=22). See Open Questions for the deferred follow-up.

## Key Technical Decisions

- **Three-phase enum (`Deploying { ticks_remaining } / Deployed / Undeploying { ticks_remaining }`)**: matches design §"Components". `DeployedFire` stays visual-only. **Confidence:** high — **Source:** design doc + verified against [animation.rs:438-440](../../src/sim/animation.rs).
- **Toggle command, not separate Deploy/Undeploy**: matches gamemd mission 0x10. **Confidence:** high — **Source:** GI_GHIDRA_REPORT §D-T.1.
- **Inline Set_Destination gate replicated in 7 handlers** (no shared helper): each handler has handler-specific cleanup before the gate would run, so a centralized gate would still need per-handler bookkeeping. **Confidence:** high — **Source:** review of [world_commands.rs:109,333,492,509,593,692,712](../../src/sim/world/world_commands.rs).
- **Constant fallback `DEPLOY_DEFAULT_TICKS: u16 = 55` for `compute_anim_ticks`**: `apply_command` has no access to `BTreeMap<String, SequenceSet>` (it lives in `AppState`, not `Simulation`). Threading sequences through `advance_tick` would touch `app_sim_tick.rs`, `ai.rs:979,1055`, `replay.rs:86`, `snapshot.rs:136`. The constant is sized so the sim `Deploying → Deployed` transition lands when the visual Deploy sequence completes (15 frames × DEFAULT_TRANSITION_TICK_MS=80ms = 1200ms ≈ 55 sim ticks at SIM_TICK_MS=22). Sized for the GI; non-GI deploy-fire types may drift, but they share the same `Deploy=300,15,0` sequence in stock YR. Precise per-type lookup is deferred — see Open Questions. **Confidence:** medium — **Source:** artmd.ini `[GISequence] Deploy=300,15,0`, [src/rules/infantry_sequence.rs:237-239](../../src/rules/infantry_sequence.rs) for `DEFAULT_TRANSITION_TICK_MS=80ms`, util/fixed_math.rs:51 for `SIM_TICK_HZ=45`.
- **`log::trace!` not `tracing::trace!` for stub arms**: codebase uses `log`. **Confidence:** high — **Source:** repo grep.
- **Direct `entity.position.rx / .ry` access** for sound event coords (no `position_rxry()` accessor exists; matches existing pattern at [combat/mod.rs:1380](../../src/sim/combat/mod.rs) and [bump_crush.rs:424](../../src/sim/movement/bump_crush.rs)). **Confidence:** high — **Source:** code review.
- **Snapshot bump 2 → 3**: `GameEntity` gains a serialized field. **Confidence:** high — **Source:** design doc.
- **`#[serde(default)]` on the new field**: forward-compat insurance; matches existing pattern on `on_bridge`, `invulnerability`. **Confidence:** high — **Source:** [game_entity.rs:110-136](../../src/sim/game_entity.rs).

## Open Questions

### Resolved During Planning

- **Where do animation sequences live, can `compute_anim_ticks(obj, sequences, kind)` look them up?** Sequences live in `AppState.animation_sequences`, not in `Simulation`. B1 uses `DEPLOY_DEFAULT_TICKS` constant; precise per-type lookup deferred.
- **How many `ObjectType { ... }` literal-init sites exist?** Two: [locomotor_tests.rs:12](../../src/sim/movement/locomotor_tests.rs) and [teleport_movement.rs:247](../../src/sim/movement/teleport_movement.rs). `make_obj` and `make_drive_obj` helpers respectively.
- **How many `GameEntity { ... }` literal-init sites exist?** Zero — every callsite goes through `GameEntity::new()`. Grep for `^\s*GameEntity\s*\{$` returns no matches; the seemingly-matching hits in [bump_crush.rs:583](../../src/sim/movement/bump_crush.rs) etc. are function signatures (`fn infantry(...) -> GameEntity {`), not literal-init.
- **Which logging macro does the codebase use?** `log::*` everywhere; no `tracing` imports.
- **Does `select_weapon_with_ifv` read deploy state?** No — verified at [combat_weapon.rs:96-150](../../src/sim/combat/combat_weapon.rs). Confirms D-T.4 finding that weapon pick is target-driven, independent of deploy state. **No combat code changes needed in B1.**

### Deferred to Implementation

- **Precise per-type Deploy/Undeploy frame-count lookup.** B1 ships with `DEPLOY_DEFAULT_TICKS = 55`, sized for stock GI's `Deploy=300,15,0`. Other deploy-fire types share the same 15-frame Deploy sequence in stock YR, so the constant is approximately right for the full B1 surface. For mod compatibility / non-GI types with different sequences, two follow-up options: (a) thread `&BTreeMap<String, SequenceSet>` through `advance_tick`/`apply_command` (touches every `advance_tick` call site); or (b) merge per-type `(deploy_ticks, undeploy_ticks)` onto `ObjectType` at art.ini load time, mirroring how `queueing_cell` and `docking_offset` are merged from art.ini. Option (b) is closer to existing repo precedent. Should land before B2 ships if the visual ever drifts noticeably.
- **Set_Destination gate behavior on `Stop` and `Guard`.** Design lists 7 movement-bearing commands; `Stop` and `Guard` are deliberately excluded. `Stop` is a no-op for a deployed unit (no movement target to clear); `Guard` is the post-deploy default mission per gamemd `FUN_0051F6E0` line `vtable+0x1F0(5)`. No follow-up needed unless behavior surprises in playtest.
- **Behavior when `obj.deploy_fire == false` but `entity.deploy_state.is_some()` somehow.** This shouldn't happen in B1 (only `ToggleInfantryDeploy` writes `deploy_state`, and the toggle handler INI-gates on `obj.deploy_fire`). If a snapshot mismatch ever produced this, the gate would still fire correctly — the unit would behave as deployed but couldn't be undeployed. Defensive cleanup deferred.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/rules/object_type.rs](../../src/rules/object_type.rs) | +2 fields (`deploy_fire`, `deploy_fire_weapon`) + 2 ReadINI lines |
| Modify | [src/sim/movement/locomotor_tests.rs](../../src/sim/movement/locomotor_tests.rs) | Default the 2 new fields in `make_obj` literal-init |
| Modify | [src/sim/movement/teleport_movement.rs](../../src/sim/movement/teleport_movement.rs) | Default the 2 new fields in `make_drive_obj` literal-init |
| Create | `src/sim/deploy.rs` (~80 LOC) | `DeployPhase` enum, `tick_deploy_state`, `compute_anim_ticks` |
| Modify | [src/sim/mod.rs](../../src/sim/mod.rs) | `pub mod deploy;` + `#[cfg(test)] mod deploy_tests;` |
| Modify | [src/sim/game_entity.rs](../../src/sim/game_entity.rs) | +1 field `deploy_state` + `is_deployed()` / `is_fully_deployed()` helpers + `new()` default |
| Modify | [src/sim/command.rs](../../src/sim/command.rs) | +1 variant `Command::ToggleInfantryDeploy { entity_id }` |
| Modify | [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | +2 `SimSoundEvent` variants + insert `tick_deploy_state` into `advance_tick` |
| Modify | [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) | +1 handler for `ToggleInfantryDeploy` + Set_Destination gate in 7 movement handlers |
| Modify | [src/sim/animation.rs](../../src/sim/animation.rs) | `tick_animations` reads `deploy_state` and reflects sequence |
| Modify | [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) | Hash `deploy_state` |
| Modify | [src/sim/snapshot.rs](../../src/sim/snapshot.rs) | Bump `SNAPSHOT_VERSION` 2 → 3 |
| Modify | [src/app_sim_tick.rs](../../src/app_sim_tick.rs) | Stub `log::trace!` arms for the 2 new SimSoundEvent variants |
| Create | `src/sim/deploy_tests.rs` (~400 LOC) | 17 tests covering state machine, gate, sounds, hash, snapshot |

## Interface Changes

**Public additions (additive — no existing breaks):**
- `pub enum DeployPhase` in `src/sim/deploy.rs`, surfaced via `pub mod deploy;` in [src/sim/mod.rs](../../src/sim/mod.rs).
- `pub fn tick_deploy_state(entities: &mut EntityStore)` in `src/sim/deploy.rs`.
- `Command::ToggleInfantryDeploy { entity_id: u64 }`.
- `SimSoundEvent::EntityDeployed { deploy_sound_id, rx, ry }` and `SimSoundEvent::EntityUndeployed { undeploy_sound_id, rx, ry }`.
- `GameEntity::is_deployed(&self) -> bool` and `GameEntity::is_fully_deployed(&self) -> bool`.
- `GameEntity.deploy_state: Option<DeployPhase>` field.
- `ObjectType.deploy_fire: bool` and `ObjectType.deploy_fire_weapon: Option<i32>` fields.

**Exhaustive matches that gain arms:**
- `match cmd { ... }` in [world_commands.rs:102](../../src/sim/world/world_commands.rs) — adds 1 arm.
- `match sim_event { ... }` in [app_sim_tick.rs:283](../../src/app_sim_tick.rs) — adds 2 arms (stub `log::trace!` + `continue`).

## Sim Checklist

- [x] All math uses `fixed`-point — no f32/f64. **Verified:** `ticks_remaining: u16` is integer; no fixed-point math needed for B1.
- [x] New state included in deterministic state hash. **Verified:** `deploy_state` hashed in Task 12.
- [x] No dependencies on render/ui/sidebar/audio/net. **Verified:** `src/sim/deploy.rs` imports only from `sim/`.
- [x] Tick ordering impact noted: `tick_deploy_state` runs after Phase 4.5 (superweapons) and before Phase 5 (turret rotation), inside `if let Some(rules)` block — command-dispatched toggles materialize the same tick they're issued and Phase 5 combat reads the up-to-date `deploy_state`.
- [x] BTreeMap iteration order considered: `tick_deploy_state` walks `entities.keys_sorted()` (deterministic, matches existing sim-loop convention).

## Risk Areas

- **Snapshot version bump invalidates pre-B1 saves.** Acceptable per design (project is pre-1.0); no test fixtures are pre-version-3 saves, so no test fallout expected. Existing test `version_mismatch_is_rejected` continues to validate the rejection path.
- **Set_Destination gate may swallow legitimate edge cases** (e.g., AI-issued movement on a deployed unit). Mitigation: `is_deployed()` is `false` for any unit that never entered the deploy state machine; AI never auto-deploys GIs in B1 (out of scope per design); so AI-issued Move on a deployed GI is unreachable without prior player toggle. If a future slice adds AI auto-deploy, AI move issuers should call `is_deployed` themselves to avoid dispatching dead-letter commands.
- **Animation cascade rewrite** in `tick_animations` (Task 11) replaces 30+ existing lines. Risk: subtly breaking the existing Stand/Walk/Attack auto-transitions for non-deploy-fire entities. Mitigation: the `None` branch of the new cascade contains the exact prior logic verbatim (Walk/Stand/Attack/FireProne/WetAttack/FireFly), and the design's matrix covers regression via test `combat_fires_during_deployed_attack`. Visually verify in any playtest that rifle infantry still auto-walk and auto-fire normally.
- **17 tests in one file (~400 LOC).** Within ~600 LOC guideline. If grown further, split into a submodule directory.
- **Mid-transition toggle silent no-op:** matches gamemd. Tests `mid_deploying_toggle_ignored` and `mid_undeploying_toggle_ignored` cover this.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 7 | `ToggleInfantryDeploy` handler INI gate (`obj.deploy_fire`) | Stock YR ignores deploy on non-deploy-fire infantry (Tanya, Engineer, etc.). Toggling them must be silently no-op. | Test `non_deploy_fire_infantry_no_op` |
| Task 7 | Mid-transition toggle is silent no-op | Matches gamemd `FUN_0051F6E0` early-return when `current_seq ∈ {0x1B-0x1E}` and `type+0x6C4 < 0`. Player spamming D mid-deploy must not produce a phase storm. | Tests `mid_deploying_toggle_ignored` / `mid_undeploying_toggle_ignored` |
| Task 9 | `tick_deploy_state` insertion point in `advance_tick` (Phase 4.6, between superweapons and turret rotation) | Phase order matters for replay determinism: command-issued toggle on tick N must be reflected before combat reads `deploy_state`. | Tests `deploy_phase_advances_to_deployed`, `hash_deterministic_through_full_cycle` |
| Task 10 | Set_Destination gate on all 7 movement-bearing commands (Move, AttackMove, EnterTransport, HarvestCell, RepairAtDepot, MinerReturn, CaptureBuilding) | Stock YR: deployed GI silently ignores movement orders. Player must explicitly toggle undeploy first. Missing any of the 7 is a parity drift. | Tests `move_silently_ignored_on_deployed`, `attack_move_silently_ignored_on_deployed`, `enter_transport_silently_ignored_on_deployed`, `move_silently_ignored_on_deploying`, `move_silently_ignored_on_undeploying`, `move_works_after_undeploy_completes` |
| Task 11 | Animation cascade: `Deploying → Deploy`, `Deployed (no attack) → Deployed`, `Deployed (with attack) → DeployedFire`, `Undeploying → Undeploy`, `None → existing Stand/Walk/Attack` | Visual stance must flip on the same tick `deploy_state` does. The `DeployedFire` auto-transition must keep working when `deploy_state == Deployed`. | Test `combat_fires_during_deployed_attack` |
| Task 12 | `deploy_state` included in state hash | Replay/lockstep would desync on deploy. | Test `hash_deterministic_through_full_cycle` |
| Task 13 | Snapshot version bump 2 → 3 | Pre-bump saves loaded into post-bump engine must reject cleanly. | Existing test `version_mismatch_is_rejected` ([snapshot.rs:197](../../src/sim/snapshot.rs)) — verify still passes |

---

## Tasks

### Task 1: Add `deploy_fire` and `deploy_fire_weapon` fields to `ObjectType`

**Why:** INI gate for the toggle handler — only `DeployFire=yes` types respond. `DeployFireWeapon=` is parsed for future AI auto-deploy use; B1 doesn't read it but completes the parse contract.

**Files:**
- Modify: [src/rules/object_type.rs](../../src/rules/object_type.rs) — struct definition near line 459 (after `ifv_mode`); parse site near line 838 (after `ifv_mode` parse).

**Pattern:** Mirrors existing bool / Option<i32> fields like `gunner` and `ammo`.

**Step 1: Add field declarations**

In [src/rules/object_type.rs](../../src/rules/object_type.rs), immediately after the `pub ifv_mode: u32,` declaration (line 459), insert:

```rust
    /// Whether this infantry can toggle into a deploy-fire stance (DeployFire=yes
    /// in rules.ini). Only deploy-fire types respond to `Command::ToggleInfantryDeploy`.
    /// Stock YR sets this on GI (E1), GuardianGI (GGI), and a handful of others.
    pub deploy_fire: bool,

    /// Index of the weapon (0=primary, 1=secondary) that the AI auto-deploy planner
    /// considers when deciding "should I deploy here?". Parsed from `DeployFireWeapon=N`
    /// in rules.ini. Default `None`. Not consulted in B1 (no AI auto-deploy);
    /// fire-time weapon pick is target-driven via `select_weapon_with_ifv`.
    pub deploy_fire_weapon: Option<i32>,
```

**Step 2: Add parse calls**

In [src/rules/object_type.rs](../../src/rules/object_type.rs), immediately after the `ifv_mode: section.get_i32("IFVMode")...` line (line 838), insert:

```rust
            deploy_fire: section.get_bool("DeployFire").unwrap_or(false),
            deploy_fire_weapon: section.get_i32("DeployFireWeapon"),
```

**Step 3: Verify**

Run: `cargo check --lib`
Expected: errors only at the two literal-init sites in `locomotor_tests.rs` and `teleport_movement.rs` (fixed in Tasks 2-3).

**Step 4: Commit**

```
git add src/rules/object_type.rs
git commit -m "rules: parse DeployFire and DeployFireWeapon onto ObjectType"
```

---

### Task 2: Default new fields in `locomotor_tests.rs` literal-init

**Why:** `make_obj()` constructs `ObjectType` literally — the two new fields need defaults at this site.

**Files:**
- Modify: [src/sim/movement/locomotor_tests.rs:128](../../src/sim/movement/locomotor_tests.rs)

**Step 1: Insert defaults**

Find the line `ifv_mode: 0,` (around line 128) inside the `ObjectType { ... }` initializer. Insert immediately after:

```rust
        deploy_fire: false,
        deploy_fire_weapon: None,
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: errors now point only at `teleport_movement.rs`.

**Step 3: Commit**

```
git add src/sim/movement/locomotor_tests.rs
git commit -m "tests: default deploy_fire fields on locomotor_tests ObjectType helper"
```

---

### Task 3: Default new fields in `teleport_movement.rs` literal-init

**Why:** Second of two literal-init sites.

**Files:**
- Modify: [src/sim/movement/teleport_movement.rs:363](../../src/sim/movement/teleport_movement.rs)

**Step 1: Insert defaults**

Find the line `ifv_mode: 0,` (around line 363) inside the `ObjectType { ... }` initializer of `make_drive_obj`. Insert immediately after (note the surrounding indentation is 12 spaces):

```rust
            deploy_fire: false,
            deploy_fire_weapon: None,
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: clean compile.

**Step 3: Commit**

```
git add src/sim/movement/teleport_movement.rs
git commit -m "tests: default deploy_fire fields on teleport_movement ObjectType helper"
```

---

### Task 4: Create `src/sim/deploy.rs`

**Why:** The state-machine module — defines `DeployPhase`, `tick_deploy_state`, and `compute_anim_ticks`.

**Files:**
- Create: `src/sim/deploy.rs` (~80 LOC)
- Modify: [src/sim/mod.rs](../../src/sim/mod.rs) — `pub mod deploy;`

**Pattern:** Mirrors small focused sim modules like [src/sim/ore_growth.rs](../../src/sim/ore_growth.rs) and [src/sim/power_system.rs](../../src/sim/power_system.rs) — top-level functions operating on `EntityStore`.

**Step 1: Write the module**

Create `src/sim/deploy.rs`:

```rust
//! Infantry deploy-fire state machine.
//!
//! Models the sim-authoritative phase: Deploying → Deployed → Undeploying → None.
//! The animation system reads `entity.deploy_state` and reflects the visual
//! sequence (Deploy / Deployed / DeployedFire / Undeploy). `DeployedFire` is
//! not a sim phase — it's a visual sub-state of `Deployed` driven by
//! `attack_target.is_some()` (existing tick_animations auto-transition).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/entity_store, sim/game_entity.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::entity_store::EntityStore;

/// Default deploy/undeploy duration in sim ticks when the per-type art.ini
/// frame count cannot be resolved from this scope.
///
/// At SIM_TICK_MS=22, 55 ticks ≈ 1210ms. Sized to roughly match stock GI
/// Deploy (15 frames × ~80ms/frame ≈ 1200ms), so when the sim phase advances
/// to Deployed and the animation cascade switches the sequence, the visual
/// Deploy animation has just about completed. Without this sizing, sim phase
/// transitions ahead of the art.ini-driven visual and `tick_animations`
/// truncates the Deploy sequence mid-playback. Per-type precise lookup is
/// deferred — see plan Open Questions.
pub(crate) const DEPLOY_DEFAULT_TICKS: u16 = 55;

/// Sim-authoritative deploy phase for an entity.
///
/// `None` on `GameEntity.deploy_state` means upright (default). Any `Some(_)`
/// variant gates the Set_Destination early-return — deployed units silently
/// ignore Move/AttackMove/Enter/etc. until explicitly undeployed.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum DeployPhase {
    /// Deploy animation playing — sim ticks count down to Deployed.
    Deploying { ticks_remaining: u16 },
    /// Stationary in deployed stance. Visual flips to DeployedFire when
    /// `attack_target.is_some()` (existing tick_animations auto-transition).
    Deployed,
    /// Undeploy animation playing — sim ticks count down to None.
    Undeploying { ticks_remaining: u16 },
}

/// Resolve the number of sim ticks the deploy or undeploy phase should run.
///
/// B1 returns the constant fallback regardless of input — see the doc on
/// `DEPLOY_DEFAULT_TICKS` for the rationale and the deferred follow-up.
pub(crate) fn compute_anim_ticks() -> u16 {
    DEPLOY_DEFAULT_TICKS
}

/// Advance every entity's `deploy_state` by one tick.
///
/// `Deploying { N }` → `Deploying { N-1 }` until N == 1, then promotes to
/// `Deployed`. `Undeploying { N }` follows the same shape, ending at `None`.
pub fn tick_deploy_state(entities: &mut EntityStore) {
    let keys = entities.keys_sorted();
    for id in keys {
        let Some(entity) = entities.get_mut(id) else {
            continue;
        };
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
            Some(DeployPhase::Deployed) | None => {}
        }
    }
}
```

**Step 2: Register the module**

In [src/sim/mod.rs](../../src/sim/mod.rs), add this line near the other animation/AI modules (e.g., after `pub mod animation;` line 54 or alphabetically appropriate):

```rust
pub mod deploy;
```

**Step 3: Verify**

Run: `cargo check --lib`
Expected: clean (the new module compiles standalone — no callers yet).

**Step 4: Commit**

```
git add src/sim/deploy.rs src/sim/mod.rs
git commit -m "sim: add DeployPhase enum + tick_deploy_state state machine"
```

---

### Task 5: Add `deploy_state` field + helpers to `GameEntity`

**Why:** Single source of truth for deploy phase. Hashable for replay; `#[serde(default)]` for forward compat.

**Files:**
- Modify: [src/sim/game_entity.rs](../../src/sim/game_entity.rs)

**Pattern:** Mirrors `Option<T>` subsystem fields like `attack_target` and `dock_state`.

**Step 1: Add the import**

In [src/sim/game_entity.rs](../../src/sim/game_entity.rs), after the existing `use crate::sim::docking::...` line (around line 28), add:

```rust
use crate::sim::deploy::DeployPhase;
```

**Step 2: Add the field**

In the `GameEntity` struct definition, immediately before `pub debug_log: ...` (around line 200), insert:

```rust
    /// Active deploy-fire phase. `None` = upright (default). `Some(Deploying)` /
    /// `Some(Deployed)` / `Some(Undeploying)` for the three machine states.
    /// Hashed for lockstep determinism. Set by `Command::ToggleInfantryDeploy`,
    /// advanced by `tick_deploy_state`. Animation reflects this; combat does not
    /// read it (weapon pick is target-driven).
    #[serde(default)]
    pub deploy_state: Option<DeployPhase>,
```

**Step 3: Default the field in `GameEntity::new`**

In `GameEntity::new()` body, immediately before `debug_log: None,` (around line 301), insert:

```rust
            deploy_state: None,
```

**Step 4: Add the predicate helpers**

In `impl GameEntity`, immediately after `is_alive()` (around line 359), insert:

```rust
    /// Whether this entity is in any deploy phase (Deploying, Deployed, or Undeploying).
    /// Used by the 7 movement-command handlers to silently ignore movement orders.
    pub fn is_deployed(&self) -> bool {
        self.deploy_state.is_some()
    }

    /// Whether this entity has finished deploying and is in the stationary
    /// Deployed phase (not transitioning).
    pub fn is_fully_deployed(&self) -> bool {
        matches!(self.deploy_state, Some(DeployPhase::Deployed))
    }
```

**Step 5: Verify**

Run: `cargo check --lib`
Expected: clean. (No literal-init sites for `GameEntity` — all callers use `GameEntity::new()`.)

**Step 6: Commit**

```
git add src/sim/game_entity.rs
git commit -m "sim: add deploy_state field + is_deployed helpers to GameEntity"
```

---

### Task 6: Add `Command::ToggleInfantryDeploy` variant

**Why:** The user-issued command. Routed through normal command dispatch — replay-safe.

**Files:**
- Modify: [src/sim/command.rs](../../src/sim/command.rs)

**Pattern:** Mirrors `Stop { entity_id }` and `MinerReturn { entity_id }` shapes.

**Step 1: Add the variant**

In [src/sim/command.rs](../../src/sim/command.rs), inside the `pub enum Command { ... }` block, immediately before the closing `}` (around line 134), insert:

```rust
    /// Toggle an infantry unit's deploy-fire state.
    ///
    /// Three transitions:
    /// - `None → Deploying` (start deploy animation)
    /// - `Deployed → Undeploying` (start undeploy animation)
    /// - mid-transition (Deploying / Undeploying) → no-op (matches gamemd)
    ///
    /// Silently no-op if the entity's type is not `DeployFire=yes`.
    ToggleInfantryDeploy { entity_id: u64 },
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: error in [world_commands.rs:102](../../src/sim/world/world_commands.rs) — `match cmd` is now non-exhaustive. Fixed in Task 7.

**Step 3: Commit**

```
git add src/sim/command.rs
git commit -m "sim: add Command::ToggleInfantryDeploy variant"
```

---

### Task 7: Add 2 `SimSoundEvent` variants + `ToggleInfantryDeploy` handler

**Why:** Wires the command to its dispatch behavior. Adds the sound-event variants emitted on phase entry.

**Files:**
- Modify: [src/sim/world/mod.rs:88-136](../../src/sim/world/mod.rs) — `SimSoundEvent` enum
- Modify: [src/sim/world/world_commands.rs:102](../../src/sim/world/world_commands.rs) — `apply_command` match

**Pattern:** Sound variants mirror `EntityCrushed { crush_sound_id, rx, ry }`. Handler mirrors `Command::Stop` (small mutator) + `Command::MinerReturn` (entity-mutating per-id command).

**Step 1: Add `EntityDeployed` and `EntityUndeployed` to `SimSoundEvent`**

In [src/sim/world/mod.rs](../../src/sim/world/mod.rs), inside `pub enum SimSoundEvent { ... }`, immediately after the existing `EntityCrushed { ... }` block (lines 104-108), insert:

```rust
    /// An infantry entity entered the Deploying phase — play its DeploySound=.
    EntityDeployed {
        deploy_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
    /// An infantry entity entered the Undeploying phase — play its UndeploySound=.
    EntityUndeployed {
        undeploy_sound_id: InternedId,
        rx: u16,
        ry: u16,
    },
```

**Step 2: Add the handler arm**

In [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs), inside the `match cmd { ... }` block in `apply_command`, immediately after `Command::UndeployBuilding { entity_id } => { ... }` (around line 433), insert:

```rust
            Command::ToggleInfantryDeploy { entity_id } => {
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                let Some(rules) = rules else { return false };
                // INI gate: only DeployFire=yes types respond.
                let type_str = match self.entities.get(*entity_id) {
                    Some(e) => self.interner.resolve(e.type_ref).to_string(),
                    None => return false,
                };
                let Some(obj) = rules.object(&type_str) else {
                    return false;
                };
                if !obj.deploy_fire {
                    return false;
                }
                let deploy_sound = obj.deploy_sound.clone();
                let undeploy_sound = obj.undeploy_sound.clone();

                let Some(entity) = self.entities.get_mut(*entity_id) else {
                    return false;
                };
                let (rx, ry) = (entity.position.rx, entity.position.ry);
                let new_phase: Option<crate::sim::deploy::DeployPhase>;
                let mut emit_deploy_sound = false;
                let mut emit_undeploy_sound = false;
                match entity.deploy_state {
                    None => {
                        new_phase = Some(crate::sim::deploy::DeployPhase::Deploying {
                            ticks_remaining: crate::sim::deploy::compute_anim_ticks(),
                        });
                        emit_deploy_sound = true;
                    }
                    Some(crate::sim::deploy::DeployPhase::Deployed) => {
                        new_phase = Some(crate::sim::deploy::DeployPhase::Undeploying {
                            ticks_remaining: crate::sim::deploy::compute_anim_ticks(),
                        });
                        emit_undeploy_sound = true;
                        // Belt-and-braces: clear any stale movement target.
                        entity.movement_target = None;
                    }
                    Some(crate::sim::deploy::DeployPhase::Deploying { .. })
                    | Some(crate::sim::deploy::DeployPhase::Undeploying { .. }) => {
                        return false;
                    }
                }
                entity.deploy_state = new_phase;

                if emit_deploy_sound {
                    if let Some(sound_name) = deploy_sound {
                        let sound_id = self.interner.intern(&sound_name);
                        self.sound_events
                            .push(crate::sim::world::SimSoundEvent::EntityDeployed {
                                deploy_sound_id: sound_id,
                                rx,
                                ry,
                            });
                    }
                }
                if emit_undeploy_sound {
                    if let Some(sound_name) = undeploy_sound {
                        let sound_id = self.interner.intern(&sound_name);
                        self.sound_events
                            .push(crate::sim::world::SimSoundEvent::EntityUndeployed {
                                undeploy_sound_id: sound_id,
                                rx,
                                ry,
                            });
                    }
                }
                true
            }
```

**Step 3: Verify**

Run: `cargo check --lib`
Expected: error in [app_sim_tick.rs:283](../../src/app_sim_tick.rs) — `match sim_event` is now non-exhaustive. Fixed in Task 8.

**Step 4: Commit**

```
git add src/sim/world/mod.rs src/sim/world/world_commands.rs
git commit -m "sim: handle ToggleInfantryDeploy + emit Deploy/Undeploy sound events"
```

---

### Task 8: Stub `log::trace!` arms for the 2 new `SimSoundEvent` variants in `app_sim_tick`

**Why:** Restore exhaustive-match on `SimSoundEvent` without doing real audio translation. B2 replaces these stubs with real `GameSoundEvent::EntityDeployed` / `EntityUndeployed` translation.

**Files:**
- Modify: [src/app_sim_tick.rs:283-436](../../src/app_sim_tick.rs)

**Pattern:** Existing `SimSoundEvent::DockDeploy { .. } => { ...; continue; }` arm at line 317-321 — translate-not-implemented events `continue;` past the assignment. Mirror that exactly, plus a `log::trace!` for visibility during sim-only testing.

**Step 1: Add the stub arms**

In [src/app_sim_tick.rs](../../src/app_sim_tick.rs), inside the `match sim_event { ... }` block (around line 283-436), insert two arms immediately after the existing `EntityCrushed { ... }` block ending around line 316:

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

**Step 2: Verify**

Run: `cargo check`
Expected: clean compile across `--lib` and `--bin` targets.

**Step 3: Commit**

```
git add src/app_sim_tick.rs
git commit -m "app: stub EntityDeployed/EntityUndeployed sound translation for B1"
```

---

### Task 9: Wire `tick_deploy_state` into `World::advance_tick`

**Why:** Make the state machine advance every tick. Insertion point matters for replay determinism.

**Files:**
- Modify: [src/sim/world/mod.rs](../../src/sim/world/mod.rs) — inside `advance_tick`, between Phase 4.5 (line 1168) and Phase 5 (line 1170).

**Pattern:** Single function call between two existing phase blocks.

**Step 1: Insert the call**

In [src/sim/world/mod.rs](../../src/sim/world/mod.rs), inside the `if let Some(rules) = rules { ... }` block, locate this exact existing code:

```rust
            // --- Phase 4.5: Superweapons ---
            // DEPENDS ON: power state (suspend/resume gating).
            // PRODUCES: world_effects (bolt anims), damage to entities, sound_events.
            if self.game_options.super_weapons {
                crate::sim::superweapon::tick_superweapons(self, rules);
            }

            // --- Phase 5: Turrets + Combat ---
```

Insert this block between the two phases:

```rust
            // --- Phase 4.6: Deploy/Undeploy state machine ---
            // DEPENDS ON: command dispatch (ToggleInfantryDeploy may have set
            //   Deploying/Undeploying this tick).
            // PRODUCES: phase advances (Deploying→Deployed, Undeploying→None)
            //   that combat (Phase 5) and animation (post-tick) read this tick.
            crate::sim::deploy::tick_deploy_state(&mut self.entities);

```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: clean.

**Step 3: Commit**

```
git add src/sim/world/mod.rs
git commit -m "sim: insert tick_deploy_state into advance_tick (Phase 4.6)"
```

---

### Task 10: Apply Set_Destination gate to all 7 movement-bearing command handlers

**Why:** Faithful parity. Deployed unit silently ignores movement orders — the player must explicitly toggle undeploy first. Mirrors `InfantryClass::Set_Destination @ 0x0051AA40`.

**Files:**
- Modify: [src/sim/world/world_commands.rs](../../src/sim/world/world_commands.rs) — 7 handlers.

**Pattern:** Inline `if entity.is_deployed() { return false; }` immediately after the ownership check at the top of each handler.

**Step 1: Gate `Command::Move` (line 109)**

Locate:

```rust
                if !self.entity_owned_by_id(command_owner, *entity_id) {
                    return false;
                }
                // Cancel any dock state when given a new move order.
```

Insert immediately after the closing `}` of the ownership check:

```rust
                if self
                    .entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
```

**Step 2: Gate `Command::AttackMove` (line 333)**

After the ownership check at line 339-341, insert the same block (using `*entity_id`).

**Step 3: Gate `Command::MinerReturn` (line 492)**

After the ownership check at line 493-495, insert the same block (using `*entity_id`).

**Step 4: Gate `Command::RepairAtDepot` (line 509)**

After the ownership check at line 514-516, insert the same block (using `*entity_id`).

**Step 5: Gate `Command::EnterTransport` (line 593)**

After the ownership check at line 598-600, insert (using `*passenger_id`):

```rust
                if self
                    .entities
                    .get(*passenger_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
```

**Step 6: Gate `Command::HarvestCell` (line 692)**

After the ownership check at line 697-699, insert (using `*entity_id`):

```rust
                if self
                    .entities
                    .get(*entity_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
```

**Step 7: Gate `Command::CaptureBuilding` (line 712)**

After the ownership check at line 717-719, insert (using `*engineer_id`):

```rust
                if self
                    .entities
                    .get(*engineer_id)
                    .is_some_and(|e| e.is_deployed())
                {
                    return false;
                }
```

**Step 8: Verify**

Run: `cargo check --lib`
Expected: clean.

**Step 9: Commit**

```
git add src/sim/world/world_commands.rs
git commit -m "sim: gate 7 movement commands on is_deployed (faithful Set_Destination)"
```

---

### Task 11: Animation reflection in `tick_animations`

**Why:** Visual stance flips to match `deploy_state`. The existing `Deployed → DeployedFire` auto-transition (when `attack_target.is_some()`) keeps working unchanged for the `Deployed` phase.

**Files:**
- Modify: [src/sim/animation.rs:414-450](../../src/sim/animation.rs)

**Pattern:** Branch on `deploy_state` first; the `None` branch falls through to the existing logic, preserved verbatim.

**Step 1: Replace the auto-transition block**

In [src/sim/animation.rs](../../src/sim/animation.rs), find the existing block between the `dying` early-return and the frame-advance loop. Locate this exact code (lines 414-450):

```rust
        let has_movement: bool = entity.movement_target.is_some();
        let has_attack: bool = entity.attack_target.is_some();

        // Look up this type's sequence definitions for transition checks.
        let seq_set: Option<&SequenceSet> = sequences.get(interner.resolve(entity.type_ref));

        // Auto-transition: stand ↔ walk based on MovementTarget presence.
        if has_movement && anim.sequence == SequenceKind::Stand {
            anim.switch_to(SequenceKind::Walk);
        } else if !has_movement && anim.sequence == SequenceKind::Walk {
            anim.switch_to(SequenceKind::Stand);
        }

        // Attack animation: when entity is stationary with an attack target,
        // switch to the appropriate fire sequence based on current stance.
        if has_attack && !has_movement {
            if let Some(set) = seq_set {
                match anim.sequence {
                    SequenceKind::Stand if set.get(&SequenceKind::Attack).is_some() => {
                        anim.switch_to(SequenceKind::Attack);
                    }
                    SequenceKind::Prone if set.get(&SequenceKind::FireProne).is_some() => {
                        anim.switch_to(SequenceKind::FireProne);
                    }
                    SequenceKind::Deployed if set.get(&SequenceKind::DeployedFire).is_some() => {
                        anim.switch_to(SequenceKind::DeployedFire);
                    }
                    SequenceKind::Swim if set.get(&SequenceKind::WetAttack).is_some() => {
                        anim.switch_to(SequenceKind::WetAttack);
                    }
                    SequenceKind::Fly if set.get(&SequenceKind::FireFly).is_some() => {
                        anim.switch_to(SequenceKind::FireFly);
                    }
                    _ => {}
                }
            }
        }
```

Replace it with:

```rust
        let has_movement: bool = entity.movement_target.is_some();
        let has_attack: bool = entity.attack_target.is_some();

        // Look up this type's sequence definitions for transition checks.
        let seq_set: Option<&SequenceSet> = sequences.get(interner.resolve(entity.type_ref));

        // Deploy state takes priority over the standard Stand/Walk/Attack cascade.
        // The visual reflects the sim phase; DeployedFire is the auto-transition
        // when a Deployed unit gains an attack target (visual-only, matches stock YR).
        match entity.deploy_state {
            Some(crate::sim::deploy::DeployPhase::Deploying { .. }) => {
                anim.switch_to(SequenceKind::Deploy);
            }
            Some(crate::sim::deploy::DeployPhase::Undeploying { .. }) => {
                anim.switch_to(SequenceKind::Undeploy);
            }
            Some(crate::sim::deploy::DeployPhase::Deployed) => {
                if has_attack {
                    if let Some(set) = seq_set {
                        if set.get(&SequenceKind::DeployedFire).is_some() {
                            anim.switch_to(SequenceKind::DeployedFire);
                        } else {
                            anim.switch_to(SequenceKind::Deployed);
                        }
                    } else {
                        anim.switch_to(SequenceKind::Deployed);
                    }
                } else {
                    anim.switch_to(SequenceKind::Deployed);
                }
            }
            None => {
                // Standard cascade for upright entities — preserved verbatim from prior logic.
                if has_movement && anim.sequence == SequenceKind::Stand {
                    anim.switch_to(SequenceKind::Walk);
                } else if !has_movement && anim.sequence == SequenceKind::Walk {
                    anim.switch_to(SequenceKind::Stand);
                }
                if has_attack && !has_movement {
                    if let Some(set) = seq_set {
                        match anim.sequence {
                            SequenceKind::Stand if set.get(&SequenceKind::Attack).is_some() => {
                                anim.switch_to(SequenceKind::Attack);
                            }
                            SequenceKind::Prone if set.get(&SequenceKind::FireProne).is_some() => {
                                anim.switch_to(SequenceKind::FireProne);
                            }
                            SequenceKind::Swim if set.get(&SequenceKind::WetAttack).is_some() => {
                                anim.switch_to(SequenceKind::WetAttack);
                            }
                            SequenceKind::Fly if set.get(&SequenceKind::FireFly).is_some() => {
                                anim.switch_to(SequenceKind::FireFly);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
```

(The `Deployed if set.get(&SequenceKind::DeployedFire).is_some()` arm from the original cascade is folded into the `Some(Deployed) + has_attack` branch.)

**Step 2: Verify**

Run: `cargo check --lib`
Expected: clean.

Run: `cargo test --lib animation`
Expected: existing animation tests pass — the `None` branch behavior is unchanged for entities without `deploy_state`.

**Step 3: Commit**

```
git add src/sim/animation.rs
git commit -m "sim: animation reflects deploy_state (Deploy/Deployed/DeployedFire/Undeploy)"
```

---

### Task 12: Hash `deploy_state` in `world_hash::hash_entities`

**Why:** Replay/lockstep determinism. Without hashing, two sims diverge on deploy.

**Files:**
- Modify: [src/sim/world/world_hash.rs:273-390](../../src/sim/world/world_hash.rs)

**Pattern:** Mirrors the existing `attack_target` Option hash block at lines 331-337 (kind discriminant byte + variant payload).

**Step 1: Add the hash block**

In [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs), inside `fn hash_entities`, immediately after `entity.capture_target.hash(hasher);` (line 339), insert:

```rust
            match entity.deploy_state {
                None => 0u8.hash(hasher),
                Some(crate::sim::deploy::DeployPhase::Deploying { ticks_remaining }) => {
                    1u8.hash(hasher);
                    ticks_remaining.hash(hasher);
                }
                Some(crate::sim::deploy::DeployPhase::Deployed) => {
                    2u8.hash(hasher);
                }
                Some(crate::sim::deploy::DeployPhase::Undeploying { ticks_remaining }) => {
                    3u8.hash(hasher);
                    ticks_remaining.hash(hasher);
                }
            }
```

**Step 2: Verify**

Run: `cargo check --lib`
Expected: clean.

**Step 3: Commit**

```
git add src/sim/world/world_hash.rs
git commit -m "sim: hash deploy_state for replay determinism"
```

---

### Task 13: Bump snapshot version 2 → 3

**Why:** New serialized field on `GameEntity` invalidates pre-B1 saves.

**Files:**
- Modify: [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs)

**Step 1: Edit the constant**

In [src/sim/snapshot.rs](../../src/sim/snapshot.rs), change:

```rust
const SNAPSHOT_VERSION: u32 = 2;
```

to:

```rust
const SNAPSHOT_VERSION: u32 = 3;
```

**Step 2: Verify existing snapshot tests still pass**

Run: `cargo test --lib snapshot`
Expected: `round_trip_preserves_state_hash` and `version_mismatch_is_rejected` both pass.

**Step 3: Commit**

```
git add src/sim/snapshot.rs
git commit -m "sim: bump snapshot version 2 → 3 for deploy_state field"
```

---

### Task 14: Add 17 unit tests in `src/sim/deploy_tests.rs`

**Why:** Exhaustive sim-level coverage of state machine, gate, sounds, hash, and snapshot — per design's testing strategy table.

**Files:**
- Create: `src/sim/deploy_tests.rs` (~400 LOC)
- Modify: [src/sim/mod.rs](../../src/sim/mod.rs) — add `#[cfg(test)] #[path = "deploy_tests.rs"] mod deploy_tests;`

**Pattern:** Mirrors [src/sim/animation_tests.rs](../../src/sim/animation_tests.rs) — top-level `#[cfg(test)]`, helper `make_*` functions, one `#[test]` per row of the design's test table. Uses the canonical `IniFile::from_str(...) → RuleSet::from_ini(&ini)` test fixture pattern from [ruleset.rs:1696-1699](../../src/rules/ruleset.rs).

**Step 1: Write the test module**

Create `src/sim/deploy_tests.rs`:

```rust
//! Unit tests for the GI deploy-fire state machine (Slice B1).

#![cfg(test)]

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::AttackTarget;
use crate::sim::command::{Command, CommandEnvelope};
use crate::sim::components::Health;
use crate::sim::deploy::{compute_anim_ticks, DeployPhase};
use crate::sim::game_entity::GameEntity;
use crate::sim::world::{SimSoundEvent, Simulation};

/// Test ruleset with E1 (DeployFire=yes, GIDeploy/GIUndeploy sounds) and E2
/// (no DeployFire). Mirrors the [InfantryTypes] / [General] / weapon section
/// scaffolding from the canonical fixture in `ruleset.rs::make_test_rules`.
fn make_rules_with_deploy() -> RuleSet {
    let text = "\
[InfantryTypes]
0=E1
1=E2

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]

[AircraftTypes]

[BuildingTypes]

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Speed=4
Primary=M60
DeployFire=yes
DeploySound=GIDeploy
UndeploySound=GIUndeploy
IFVMode=2

[E2]
Name=Conscript
Cost=100
Strength=100
Armor=none
Speed=4
Primary=INTL

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[INTL]
Damage=20
ROF=20
Range=5
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let ini: IniFile = IniFile::from_str(text);
    RuleSet::from_ini(&ini).expect("test ruleset parse")
}

/// Test ruleset where E1 has DeployFire=yes but no DeploySound/UndeploySound.
fn make_rules_no_sounds() -> RuleSet {
    let text = "\
[InfantryTypes]
0=E1

[General]
BuildSpeed=0.75
MultipleFactory=0.7
LowPowerPenaltyModifier=1.25
MinLowPowerProductionSpeed=0.4
MaxLowPowerProductionSpeed=0.85

[VehicleTypes]

[AircraftTypes]

[BuildingTypes]

[E1]
Name=GI
Cost=200
Strength=125
Armor=none
Speed=4
Primary=M60
DeployFire=yes

[M60]
Damage=25
ROF=20
Range=5
Warhead=SA

[SA]
Verses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%
CellSpread=0
";
    let ini: IniFile = IniFile::from_str(text);
    RuleSet::from_ini(&ini).expect("test ruleset parse")
}

fn spawn_infantry(sim: &mut Simulation, type_str: &str, owner: &str, rx: u16, ry: u16) -> u64 {
    let owner_id = sim.interner.intern(owner);
    let type_id = sim.interner.intern(type_str);
    let id = sim.next_stable_entity_id;
    sim.next_stable_entity_id += 1;
    let e = GameEntity::new(
        id,
        rx,
        ry,
        0,
        0,
        owner_id,
        Health { current: 125, max: 125 },
        type_id,
        EntityCategory::Infantry,
        0,
        5,
        false, // is_voxel = false (SHP infantry)
    );
    sim.entities.insert(e);
    id
}

/// Schedule one command for tick N+1 and run a single advance_tick.
fn dispatch(sim: &mut Simulation, owner: &str, cmd: Command, rules: &RuleSet) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    let owner_id = sim.interner.intern(owner);
    let cmds = vec![CommandEnvelope::new(owner_id, sim.tick + 1, cmd)];
    sim.advance_tick(&cmds, Some(rules), &height_map, None, 22);
}

/// Sentinel attack_target used to detect whether a movement command's
/// pre-path-grid mutation block ran (gate absent or gate passed) or not
/// (gate fired). Move/AttackMove handlers clear `attack_target` BEFORE
/// the path_grid check, so this is a reliable signal independent of
/// path_grid availability.
fn sentinel_attack_target() -> AttackTarget {
    AttackTarget {
        target: 9999,
        cooldown_ticks: 5,
        burst_remaining: 0,
        burst_delay_ticks: 0,
    }
}

fn tick_n(sim: &mut Simulation, rules: &RuleSet, n: u32) {
    let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..n {
        sim.advance_tick(&[], Some(rules), &height_map, None, 22);
    }
}

#[test]
fn deploy_phase_advances_to_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. })
    ));

    let n = compute_anim_ticks() as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(sim.entities.get(gi).unwrap().deploy_state, Some(DeployPhase::Deployed));
}

#[test]
fn undeploy_phase_clears_to_none() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { .. })
    ));

    let n = compute_anim_ticks() as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(sim.entities.get(gi).unwrap().deploy_state, None);
}

#[test]
fn mid_deploying_toggle_ignored() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 3 });

    let sounds_before = sim.sound_events.len();
    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    // Tick advance still runs; Deploying decremented from 3 → 2 (or to Deployed if already 1).
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. }) | Some(DeployPhase::Deployed)
    ));
    let new_deploy_undeploy_sounds = sim
        .sound_events
        .iter()
        .skip(sounds_before)
        .filter(|e| matches!(
            e,
            SimSoundEvent::EntityDeployed { .. } | SimSoundEvent::EntityUndeployed { .. }
        ))
        .count();
    assert_eq!(new_deploy_undeploy_sounds, 0);
}

#[test]
fn mid_undeploying_toggle_ignored() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Undeploying { ticks_remaining: 3 });

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Undeploying { .. }) | None
    ));
}

#[test]
fn move_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);
    sim.entities.get_mut(gi).unwrap().attack_target = Some(sentinel_attack_target());
    let pos = (sim.entities.get(gi).unwrap().position.rx, sim.entities.get(gi).unwrap().position.ry);

    dispatch(
        &mut sim,
        "Americans",
        Command::Move { entity_id: gi, target_rx: 30, target_ry: 30, queue: false, group_id: None },
        &rules,
    );
    let entity = sim.entities.get(gi).unwrap();
    // Strong gate signal: Move handler clears attack_target BEFORE the path_grid
    // check, so a passing gate preserves the sentinel; an absent gate clears it.
    assert!(entity.attack_target.is_some(), "gate should preserve attack_target");
    assert!(entity.movement_target.is_none());
    assert_eq!((entity.position.rx, entity.position.ry), pos);
}

#[test]
fn move_silently_ignored_on_deploying() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 5 });
    sim.entities.get_mut(gi).unwrap().attack_target = Some(sentinel_attack_target());

    dispatch(
        &mut sim,
        "Americans",
        Command::Move { entity_id: gi, target_rx: 30, target_ry: 30, queue: false, group_id: None },
        &rules,
    );
    let entity = sim.entities.get(gi).unwrap();
    assert!(entity.attack_target.is_some(), "gate should preserve attack_target");
    assert!(entity.movement_target.is_none());
}

#[test]
fn move_silently_ignored_on_undeploying() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Undeploying { ticks_remaining: 5 });
    sim.entities.get_mut(gi).unwrap().attack_target = Some(sentinel_attack_target());

    dispatch(
        &mut sim,
        "Americans",
        Command::Move { entity_id: gi, target_rx: 30, target_ry: 30, queue: false, group_id: None },
        &rules,
    );
    let entity = sim.entities.get(gi).unwrap();
    assert!(entity.attack_target.is_some(), "gate should preserve attack_target");
    assert!(entity.movement_target.is_none());
}

#[test]
fn attack_move_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);
    sim.entities.get_mut(gi).unwrap().attack_target = Some(sentinel_attack_target());

    dispatch(
        &mut sim,
        "Americans",
        Command::AttackMove { entity_id: gi, target_rx: 30, target_ry: 30, queue: false },
        &rules,
    );
    let entity = sim.entities.get(gi).unwrap();
    // AttackMove also clears attack_target before path_grid check; sentinel
    // surviving = gate fired.
    assert!(entity.attack_target.is_some(), "gate should preserve attack_target");
    assert!(entity.movement_target.is_none());
    assert!(entity.order_intent.is_none());
}

#[test]
fn enter_transport_silently_ignored_on_deployed() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(
        &mut sim,
        "Americans",
        Command::EnterTransport { passenger_id: gi, transport_id: 9999 },
        &rules,
    );
    assert!(matches!(
        sim.entities.get(gi).unwrap().passenger_role,
        crate::sim::passenger::PassengerRole::None
    ));
}

#[test]
fn move_works_after_undeploy_completes() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    let n = compute_anim_ticks() as u32;
    tick_n(&mut sim, &rules, n);
    assert_eq!(sim.entities.get(gi).unwrap().deploy_state, None);

    // Gate should NOT fire now that deploy_state == None. Use the inverse
    // sentinel check: if Move runs past the gate, it clears attack_target
    // before the path_grid check; if the gate (incorrectly) fired, the
    // sentinel would survive.
    sim.entities.get_mut(gi).unwrap().attack_target = Some(sentinel_attack_target());
    dispatch(
        &mut sim,
        "Americans",
        Command::Move { entity_id: gi, target_rx: 12, target_ry: 12, queue: false, group_id: None },
        &rules,
    );
    assert!(
        sim.entities.get(gi).unwrap().attack_target.is_none(),
        "Move handler past the gate must clear attack_target after undeploy completes"
    );
}

#[test]
fn deploy_sound_emitted_on_phase_entry() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    let evs: Vec<_> = sim
        .sound_events
        .iter()
        .filter_map(|e| match e {
            SimSoundEvent::EntityDeployed { deploy_sound_id, rx, ry } => {
                Some((*deploy_sound_id, *rx, *ry))
            }
            _ => None,
        })
        .collect();
    assert_eq!(evs.len(), 1);
    let (id, rx, ry) = evs[0];
    assert_eq!(sim.interner.resolve(id), "GIDeploy");
    assert_eq!((rx, ry), (25, 30));
}

#[test]
fn deploy_sound_suppressed_when_unset() {
    let rules = make_rules_no_sounds();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    let count = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::EntityDeployed { .. }))
        .count();
    assert_eq!(count, 0);
    assert!(matches!(
        sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { .. })
    ));
}

#[test]
fn undeploy_sound_emitted_on_phase_entry() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 25, 30);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);

    dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
    let count = sim
        .sound_events
        .iter()
        .filter(|e| matches!(e, SimSoundEvent::EntityUndeployed { .. }))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn non_deploy_fire_infantry_no_op() {
    let rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let conscript = spawn_infantry(&mut sim, "E2", "Soviets", 10, 10);

    dispatch(
        &mut sim,
        "Soviets",
        Command::ToggleInfantryDeploy { entity_id: conscript },
        &rules,
    );
    assert!(sim.entities.get(conscript).unwrap().deploy_state.is_none());
}

#[test]
fn hash_deterministic_through_full_cycle() {
    let rules = make_rules_with_deploy();
    let mut sim_a = Simulation::new();
    let mut sim_b = Simulation::new();
    let gi_a = spawn_infantry(&mut sim_a, "E1", "Americans", 10, 10);
    let gi_b = spawn_infantry(&mut sim_b, "E1", "Americans", 10, 10);
    assert_eq!(gi_a, gi_b);

    for _ in 0..3 {
        dispatch(&mut sim_a, "Americans", Command::ToggleInfantryDeploy { entity_id: gi_a }, &rules);
        dispatch(&mut sim_b, "Americans", Command::ToggleInfantryDeploy { entity_id: gi_b }, &rules);
        let n = compute_anim_ticks() as u32;
        for _ in 0..n {
            tick_n(&mut sim_a, &rules, 1);
            tick_n(&mut sim_b, &rules, 1);
            assert_eq!(sim_a.state_hash(), sim_b.state_hash());
        }
    }
}

#[test]
fn snapshot_round_trip_mid_deploying() {
    use crate::sim::snapshot::GameSnapshot;
    let _rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state =
        Some(DeployPhase::Deploying { ticks_remaining: 5 });

    let bytes = GameSnapshot::save(&sim, 0, 0, "test_map");
    let snap = GameSnapshot::load(&bytes).expect("load");
    assert_eq!(
        snap.sim.entities.get(gi).unwrap().deploy_state,
        Some(DeployPhase::Deploying { ticks_remaining: 5 })
    );
}

#[test]
fn combat_fires_during_deployed_attack() {
    use crate::sim::animation::{
        Animation, LoopMode, SequenceDef, SequenceKind, SequenceSet, tick_animations,
    };

    let _rules = make_rules_with_deploy();
    let mut sim = Simulation::new();
    let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
    sim.entities.get_mut(gi).unwrap().deploy_state = Some(DeployPhase::Deployed);
    sim.entities.get_mut(gi).unwrap().attack_target = Some(AttackTarget {
        target: 9999,
        cooldown_ticks: 10,
        burst_remaining: 0,
        burst_delay_ticks: 0,
    });
    sim.entities.get_mut(gi).unwrap().animation = Some(Animation::new(SequenceKind::Deployed));

    let mut sequences: BTreeMap<String, SequenceSet> = BTreeMap::new();
    let mut set = SequenceSet::new();
    set.insert(
        SequenceKind::Deployed,
        SequenceDef {
            start_frame: 0,
            frame_count: 1,
            facings: 8,
            facing_multiplier: 1,
            tick_ms: 200,
            loop_mode: LoopMode::Loop,
            clockwise_facings: false,
        },
    );
    set.insert(
        SequenceKind::DeployedFire,
        SequenceDef {
            start_frame: 8,
            frame_count: 6,
            facings: 8,
            facing_multiplier: 6,
            tick_ms: 80,
            loop_mode: LoopMode::TransitionTo(SequenceKind::Deployed),
            clockwise_facings: false,
        },
    );
    sequences.insert("E1".to_string(), set);

    let _ = tick_animations(&mut sim.entities, &sequences, 22, &sim.interner);
    assert_eq!(
        sim.entities.get(gi).unwrap().animation.as_ref().unwrap().sequence,
        SequenceKind::DeployedFire
    );
}
```

**Step 2: Register the test module**

In [src/sim/mod.rs](../../src/sim/mod.rs), add at the end of the file (after `pub mod particles;`, around line 90):

```rust
#[cfg(test)]
#[path = "deploy_tests.rs"]
mod deploy_tests;
```

**Step 3: Verify helper signatures**

If the build fails on any helper signature mismatch, adjust:
- `IniFile::from_str(text)` is the canonical fixture pattern at [ruleset.rs:1697](../../src/rules/ruleset.rs). If it returns a `Result`, add `.expect("ini parse")`.
- `Simulation::new()` is verified to exist via [snapshot.rs:148](../../src/sim/snapshot.rs).
- `sim.entities.insert(e)` and `next_stable_entity_id` are verified pub fields/methods on `EntityStore` and `Simulation`.
- `Health` import is `crate::sim::components::Health` per [game_entity.rs:24](../../src/sim/game_entity.rs).
- `AttackTarget`'s field names (`target`, `cooldown_ticks`) match its definition in `src/sim/combat`. If different, adjust the literal-init in `combat_fires_during_deployed_attack`.

**Step 4: Verify**

Run: `cargo test --lib deploy_tests`
Expected: all 17 tests pass.

Failure-mode hints:
- `deploy_phase_advances_to_deployed` fails → check Task 9 (`tick_deploy_state` wired into `advance_tick`).
- `move_silently_ignored_*` fails → check Task 10 gates that specific command.
- `hash_deterministic_through_full_cycle` fails → check Task 12 (`deploy_state` hashed).
- `snapshot_round_trip_mid_deploying` fails → check Task 13 (version bump) and `#[serde(default)]` on the new field.

**Step 5: Commit**

```
git add src/sim/deploy_tests.rs src/sim/mod.rs
git commit -m "tests: 17-test suite for GI deploy-fire state machine (B1)"
```

---

### Task 15: Full regression — `cargo check` + `cargo test`

**Why:** Confirm B1 is integrated cleanly and no existing tests regressed.

**Files:** none (build/test only).

**Step 1: Full check**

Run: `cargo check --workspace --all-targets`
Expected: zero errors and no new warnings.

**Step 2: Full test suite**

Run: `cargo test --lib`
Expected: all tests pass, including:
- 17 new tests in `deploy_tests`
- Existing snapshot tests (`round_trip_preserves_state_hash`, `version_mismatch_is_rejected`)
- Existing animation tests — should pass without modification because the new deploy match arm in `tick_animations` only fires when `deploy_state.is_some()`, which is `None` for all existing test entities

**Step 3: Manual sim-only verification**

Optional — exercise a full deploy/undeploy cycle from a scratch test or a `cargo run --example` harness:

```rust
let rules = make_rules_with_deploy();
let mut sim = Simulation::new();
let gi = spawn_infantry(&mut sim, "E1", "Americans", 10, 10);
println!("Initial: {:?}", sim.entities.get(gi).unwrap().deploy_state);
dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
println!("After toggle: {:?}", sim.entities.get(gi).unwrap().deploy_state);
tick_n(&mut sim, &rules, 30);
println!("After 30 ticks: {:?}", sim.entities.get(gi).unwrap().deploy_state);
dispatch(&mut sim, "Americans", Command::ToggleInfantryDeploy { entity_id: gi }, &rules);
println!("After 2nd toggle: {:?}", sim.entities.get(gi).unwrap().deploy_state);
tick_n(&mut sim, &rules, 30);
println!("After 30 more ticks: {:?}", sim.entities.get(gi).unwrap().deploy_state);
```

Expected output (with `tick_n` set to e.g. 60 instead of 30 to clear the 55-tick deploy phase):
```
Initial: None
After toggle: Some(Deploying { ticks_remaining: 54 })  // already decremented once by Phase 4.6
After 60 ticks: Some(Deployed)
After 2nd toggle: Some(Undeploying { ticks_remaining: 54 })
After 60 more ticks: None
```

**Step 4: No commit** — verification only. B2 picks up here for UI integration.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-gi-deploy-fire-b1-design.md](2026-05-05-gi-deploy-fire-b1-design.md)
- **Sibling B2 design:** [docs/plans/2026-05-05-gi-deploy-fire-b2-design.md](2026-05-05-gi-deploy-fire-b2-design.md)
- **Ghidra reports:** [ra2-rust-game-docs/GI_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GI_GHIDRA_REPORT.md) §§ D-T.0–D-T.12 — deploy-trigger investigation, structurally complete.
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `FUN_0051F6E0 @ 0x0051F6E0` — `InfantryClass::Mission_Deploy_Toggle` (mission 0x10 dispatch)
  - `FUN_0051AA40 @ 0x0051AA40` — `InfantryClass::Set_Destination` with player+deploy early-return
  - `FUN_0051F660 @ 0x0051F660` — `InfantryClass::Mission_Move` override
  - `TechnoTypeClass+0xEC8` — `CanDeployFire` bool (= INI `DeployFire=`)
  - `TechnoTypeClass+0x6A8` — `DeployFireWeapon` index (= INI `DeployFireWeapon=`)
  - Active-sequence offset: `entity+0x6C4` — deploy-set = `{0x1B (Deploy), 0x1C (Deployed), 0x1D (DeployedFire), 0x1E (DeployedIdle)}`
- **INI keys driving B1:** rulesmd.ini `[E1] DeployFire=yes`, `DeploySound=GIDeploy`, `UndeploySound=GIUndeploy`, `IFVMode=2`. artmd.ini `[GISequence] Deploy=300,15,0`, `Deployed=292,1,1`, `DeployedFire=315,6,6`, `Undeploy=276,2,2`.
- **Related code (read for plan):**
  - [src/rules/object_type.rs:227-231,723-724,838](../../src/rules/object_type.rs) — existing DeploySound/UndeploySound parse, IFVMode anchor
  - [src/sim/animation.rs:56-123,375-466](../../src/sim/animation.rs) — `SequenceKind` + `tick_animations`
  - [src/sim/world/world_commands.rs:109,333,492,509,593,692,712](../../src/sim/world/world_commands.rs) — 7 movement-bearing handlers
  - [src/sim/world/mod.rs:88-136,995-1387](../../src/sim/world/mod.rs) — `SimSoundEvent` + `advance_tick` ordering
  - [src/sim/world/world_hash.rs:273-390](../../src/sim/world/world_hash.rs) — entity hashing
  - [src/sim/snapshot.rs:16](../../src/sim/snapshot.rs) — version constant
  - [src/sim/combat/combat_weapon.rs:96-150](../../src/sim/combat/combat_weapon.rs) — `select_weapon_with_ifv` (verified: no deploy-state read, no combat changes needed)
  - [src/app_sim_tick.rs:282-436](../../src/app_sim_tick.rs) — `SimSoundEvent` translation site
  - [src/rules/ruleset.rs:1696-1699](../../src/rules/ruleset.rs) — canonical test fixture pattern (`IniFile::from_str` → `RuleSet::from_ini`)
- **Prior commits / PRs:** none — B1 is the first slice.
