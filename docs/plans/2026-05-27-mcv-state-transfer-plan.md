# MCV State Transfer Implementation Plan

> Execute this plan task-by-task. Keep commits optional unless the user asks.

**Goal:** Make MCV deploy and ConYard undeploy carry gamemd-equivalent health and modeled veterancy state instead of respawning full-health rookie replacements, and add the verified failed-undeploy refund behavior.

**Architecture:** Keep conversion ownership in `Simulation` world code. For forward deploy, capture source state immediately before replacing the MCV. For delayed ConYard undeploy, read the live source building state at `BuildingDown` completion, matching gamemd's state-2 transfer point. Reuse the existing object spawn helper, then overwrite only the fields that gamemd transfers after successful placement. Do not copy Rust movement paths.

---

## Grounding Summary

**Forward deploy:** `UnitClass::Deploy @ 0x007393C0` transfers source health by ratio, clamps replacement health to at least 1, and copies veterancy/experience state. Source: `docs/research/units/allied/AMCV.md:432`, `docs/research/MCV_DEPLOY_GHIDRA_REPORT.md:166`.

**Reverse undeploy:** `BuildingClass__Sell @ 0x00449C30` saves the source building health ratio in state 2 after the reverse animation completion gate, tries one AMCV unlimbo, and on success applies `floor(saved_ratio * UnitType.Strength)` with minimum 1. Source: `docs/research/GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:45`, `:71`.

**Failed reverse unlimbo:** If the AMCV placement/unlimbo fails, gamemd removes/uninitializes the GACNST, spawns no visible AMCV, and credits the source building's sell-back refund from vtable `+0x2BC`. Source: `docs/research/GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:50`, `:133`.

**Current Rust delta:** `spawn_object_at_height` initializes replacements from target `Strength=` and veterancy `0`; `deploy_mcv`, `undeploy_building`, and `tick_building_down` currently preserve only owner/selection/spawn fields. Source: `src/sim/world/world_spawn.rs:292`, `:621`, `:716`; `src/sim/world/mod.rs:1051`.

## Non-Goals

- Do not copy `movement_target`. The verified gamemd target-redirection loop is not equivalent to a unit movement queue.
- Do not implement full target/link retargeting in this patch. Track it as a separate parity task.
- Do not change ConYard redeploy gates in this plan; those are covered by the MCV redeploy gate report.
- Do not change deploy/undeploy coordinate math unless tests reveal it blocks the state-transfer work.

## Tiny-Detail Ledger

- Health transfer uses source `current / max` ratio against the replacement type's max health, not raw HP copy. Source: `AMCV.md:432`; `MCV_DEPLOY_GHIDRA_REPORT.md:174`.
- Rounding is integer floor after multiplying by replacement max. Source: `MCV_DEPLOY_GHIDRA_REPORT.md:175`; `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:83`.
- Successful transfer clamps nonzero source health to at least 1 HP. Source: `MCV_DEPLOY_GHIDRA_REPORT.md:177`; `AMCV.md:432`.
- Replacement max health remains the target object's `Strength=`, not the source max. Source: same health-ratio transfer docs.
- Rust-modeled `veterancy` must copy from source to replacement. Source: `AMCV.md:435`; `MCV_DEPLOY_GHIDRA_REPORT.md:171`.
- Forward deploy applies state only after placement succeeds; blocked deploy must leave the MCV unchanged. Source: `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md:154`.
- Reverse undeploy reads source health/refund at pack-up completion, not command start. Source: gamemd state-2 completion source `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:45`, `:46`.
- Failed reverse spawn does not run successful health/veterancy transfer. Source: `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:120`.
- Failed reverse spawn removes the building and spawns no visible AMCV. Source: `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:122`.
- Failed reverse spawn refunds source building sell-back value, not AMCV cost, not health-ratio high bits. Source: `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:119`, `:120`.
- Credits must be added to the source owner house. Source: `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md:124`.
- No random numbers, floating point, or nondeterministic iteration are needed; use integer `u32/u64` math.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Leave unchanged unless needed | `src/sim/components.rs` | Do not add source health/veterancy/refund fields; live source state is read at completion. |
| Modify | `src/sim/world/world_spawn.rs` | Capture source transfer data in `deploy_mcv` and `undeploy_building`; add helper for proportional health. |
| Modify | `src/sim/world/mod.rs` | Apply delayed transfer in `tick_building_down`; add failed-spawn refund. |
| Modify | `src/sim/production/production_sell.rs` or `src/sim/production/mod.rs` | Add/expose a verified `GetRefundValue`-style helper if needed; do not reuse the current health-scaled sell helper blindly. |
| Modify | `src/sim/world/world_tests.rs` | Add focused tests for forward transfer, reverse transfer, and failed reverse refund. |

## Interface Changes

- `BuildingDown` should remain a delayed spawn descriptor. Source health, source veterancy, and refund are read from the live source entity at completion.
- Optional helper:
  - `transferred_health(source: Health, target_max: u16) -> Health`
  - `refund_value_for_object(obj: &ObjectType) -> i32` or equivalent verified `GetRefundValue` helper. This must not use source health unless a later report proves that for the targeted path.

## Implementation Tasks

### Task 1: Add proportional health helper

Add a private helper in `world_spawn.rs` or a small nearby module:

```rust
fn transferred_health(source: Health, target_max: u16) -> Health
```

Rules:
- If `source.current == 0`, return `current = 0`.
- If `source.max == 0`, treat this as an unreachable malformed-state fallback. Use a deterministic result and document it as defensive Rust behavior, not parity evidence.
- Otherwise compute `(source.current * target_max) / source.max` using widened integer math.
- Clamp nonzero results to at least 1 and at most `target_max`.
- Preserve `max = target_max`.

Add unit-level coverage if there is an existing local helper-test pattern; otherwise cover through conversion tests.

### Task 2: Forward AMCV -> GACNST state transfer

In `deploy_mcv`:

1. Capture `entity.health` and `entity.veterancy` in `deploy_data`.
2. Leave all placement checks before `despawn_entity`.
3. After `spawn_object_at_height` succeeds, read the spawned entity's generated `health.max`.
4. Overwrite:
   - `ge.health = transferred_health(source_health, ge.health.max)`
   - `ge.veterancy = source_veterancy`
5. Keep existing `selected` and `building_up` behavior.

Do not alter `movement_target`, `attack_target`, or locomotor fields.

### Task 3: Keep reverse transfer state live until completion

Do not extend `BuildingDown` with source health/veterancy/refund fields for this work.

In `undeploy_building`:

1. Continue to validate the source building and record only delayed spawn metadata.
2. Do not snapshot health, veterancy, or refund here.
3. Preserve the existing delayed animation behavior.

Refund helper requirement:
- Add or expose a verified `GetRefundValue`-style helper for the source object.
- Do not reuse current `production_sell::sell_refund_for_building` as-is; it is health-scaled, while the focused GACNST refund report keeps the refund slot separate from the saved health ratio.
- If the existing sell path is later fixed to the verified non-health-scaled behavior, reuse that corrected helper.

### Task 4: Apply reverse transfer or failed-spawn refund

In `tick_building_down`:

1. Extract the existing spawn fields and, from the still-live source entity, read:
   - `source_health`
   - `source_veterancy`
   - source type object for refund calculation
   - source owner for the credit recipient
2. Keep the current full-foundation occupancy cleanup before `despawn_entity`.
3. Compute `refund_on_spawn_failure` from the source type using the verified refund helper.
4. Despawn the building.
5. Try `spawn_object_at_height`.
6. If spawn succeeds:
   - Apply proportional health against spawned unit max.
   - Copy veterancy.
   - Preserve selection.
7. If spawn fails:
   - Add `refund_on_spawn_failure` to the source owner credits if positive.
   - Leave the building gone and no AMCV spawned.

Use the existing `production::credits_entry_for_owner` path for credits if accessible.

### Task 5: Tests

Add focused tests in `world_tests.rs`:

1. `deploy_mcv_transfers_health_ratio_and_veterancy`
   - Spawn AMCV.
   - Set health to a non-full ratio and veterancy nonzero.
   - Deploy.
   - Assert GACNST exists, AMCV gone, GACNST max is target strength, current matches floor ratio clamp, veterancy matches.

2. `undeploy_conyard_transfers_health_ratio_and_veterancy`
   - Spawn/prepare GACNST.
   - Set health and veterancy.
   - Start undeploy and tick through completion.
   - Assert spawned AMCV current/max/veterancy.

3. `undeploy_conyard_failed_spawn_refunds_and_does_not_restore_building`
   - Arrange final AMCV spawn to fail. Prefer a deterministic blocker or invalid rules/type fixture over test-only hooks.
   - Record owner credits.
   - Complete `BuildingDown`.
   - Assert GACNST gone, AMCV absent, credits increased by source building sell refund.

4. Regression: keep `test_undeploy_conyard_spawns_mcv` passing, including the move-out occupancy assertion.

### Task 6: Verification

Run focused tests first:

```powershell
cargo test --lib deploy_mcv_transfers_health_ratio_and_veterancy -- --nocapture
cargo test --lib undeploy_conyard_transfers_health_ratio_and_veterancy -- --nocapture
cargo test --lib undeploy_conyard_failed_spawn_refunds_and_does_not_restore_building -- --nocapture
cargo test --lib test_undeploy_conyard_spawns_mcv -- --nocapture
```

Then run one final check if no other cargo process owns the workspace:

```powershell
cargo check -q
```

## Risk Areas

- Because reverse transfer state is read at completion, changes to building health during pack-up will affect the spawned AMCV. This is intentional and matches the verified state-2 timing.
- Refund formula currently lives in `production_sell.rs` as a private health-scaled helper, but the targeted failed-undeploy refund needs a verified `GetRefundValue`-style value. Do not couple this path to the current health-scaled helper without correcting that helper first.
- Failed spawn may be hard to trigger through current `spawn_object_at_height`, because it does not do full unit unlimbo collision validation. If there is no natural failure path today, add the refund plumbing and mark the test as pending until unit unlimbo validation exists, or build a rules-missing target fixture that makes `spawn_object_at_height` return `None`.
- Rust currently models `veterancy` as a tier value, while gamemd copies a broader 5-dword experience block. Copying the existing field closes the modeled state gap but not the unmodeled experience-block gap.

## Acceptance Criteria

- Damaged AMCV deploys into a proportionally damaged ConYard.
- Veteran/elite AMCV deploys into a ConYard preserving Rust-modeled veterancy.
- Damaged ConYard undeploys into a proportionally damaged AMCV.
- ConYard undeploy preserves Rust-modeled veterancy on the AMCV.
- Failed final AMCV spawn removes the ConYard, spawns no AMCV, and refunds source building sell-back value.
- No conversion path copies `movement_target`.
- Focused tests pass, plus `cargo check -q` if the workspace is otherwise healthy.
