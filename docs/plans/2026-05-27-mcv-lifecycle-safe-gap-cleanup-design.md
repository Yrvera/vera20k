# MCV Lifecycle Safe-Gap Cleanup Design

## Goal

Keep the current MCV lifecycle patch limited to the implementation-safe gaps the user requested: ConYard redeploy gates, AMCV target-building `DeployFacing`, and placement blocker acceptance coverage.

## Architecture Context

The MCV lifecycle surfaces are split across command exposure, deterministic command acceptance, and deploy placement execution.

- `rules/` owns INI-derived object facts such as `ConstructionYard=` and `DeployFacing=`.
- `sim/world_spawn.rs` owns `AMCV -> ConYard` conversion setup and `ConYard -> AMCV` command acceptance.
- `sim/world/mod.rs` owns shared terrain/build-blocked queries used by deploy placement.
- App-level input/cursor/context-order modules expose player commands, but should call sim predicates instead of duplicating ConYard gate logic.
- Tests in `sim/deploy_tests.rs` can exercise the safe behavior without broad lifecycle state-machine refactors.

The design preserves the project boundary that `sim/` does not depend on UI, audio, render, or app modules.

## Impact Analysis

In-scope files:

- `src/rules/object_type.rs`
- `src/sim/world/world_spawn.rs`
- `src/sim/world/mod.rs`
- `src/app_context_order.rs`
- `src/app_cursor.rs`
- `src/app_input.rs`
- `src/sim/deploy_tests.rs`
- Minimal literal updates in tests that construct `ObjectType` directly.

Out-of-scope for this cleanup:

- `src/sim/world/mod.rs::tick_building_down` reverse state transfer and failed-spawn refund behavior.
- `src/sim/world/world_tests.rs` state-transfer and refund tests.
- Reverse veterancy/experience transfer.
- NACNST/SMCV redeploy generalization.
- Full deploy/undeploy mission state-machine refactor.

Risk areas:

- `Simulation::effective_build_blocked` has multiple callers, so added blocker checks must match placement-safe categories and not accidentally alter unrelated pathing semantics.
- The current deploy-facing implementation can only approximate gamemd's rotate-and-return mission path because Rust does not yet model the full deploy mission loop.
- The working tree contains unrelated dirty files, so cleanup must not revert user or other-session changes outside the scoped MCV files.

## Chosen Approach

Use a strict cleanup pass.

Keep only the requested safe-gap implementation and tests. Remove state-transfer/refund behavior from this patch unless it is proven to be pre-existing unrelated work. Do not fold the state-transfer plan into this work.

This approach is preferred because it preserves the user's explicit ordering: implement the implementation-safe gate/facing/blocker gaps first, then investigate or design broader lifecycle transfer separately.

## Tiny-Detail Ledger

- ConYard redeploy is keyed by `ConstructionYard=yes`, not hardcoded `GACNST`. Source: `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`.
- `UndeploysInto` is necessary but not sufficient for ConYards. Source: `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`.
- Non-ConYard `UndeploysInto` buildings bypass `MCVRedeploys`. Source: `BuildingClass__CanUndeployMCV @ 0x00449BC0`, reported in `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`.
- UI visibility hides ConYard redeploy while building production is busy. Source: `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0`.
- Runtime acceptance repeats the core ConYard gate so stale/desynced commands do not start `BuildingDown`. Source: `BuildingClass__CanUndeployMCV @ 0x00449BC0`.
- AMCV deploy-facing source is the target `DeploysInto` building type's `DeployFacing`, not `[AMCV] DeployFacing` and not `[General] DeployDir`. Source: `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`.
- `DeployFacing` is parsed as `INI value << 5`; default raw facing is `0x80`. Source: parser site `0x00460C76`, constructor default in `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`.
- Facing mismatch must return before building creation. Source: `UnitClass__Deploy @ 0x007395EF..0x0073965F`.
- Stock GACNST placement rejects normal overlays. Source: `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`.
- Stock GACNST placement rejects nonzero slope. Source: `Cell_passability_building_placement @ 0x0047C620`.
- Stock GACNST placement rejects nonbuildable LandType fallback. Source: `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`.
- Stock GACNST placement rejects bridge structural `0x100` and pure bridge marker `0x400`. Source: `Cell+0x140` checks in `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`.
- Mixed clear height alone must remain accepted. Source: same placement report and mixed-height trace references in the synthesis.
- Reverse veterancy/experience transfer is `NEEDS_REINVESTIGATE`; do not include it in this patch. Source: `MCV_DEPLOY_UNDEPLOY_LIFECYCLE_SYSTEM_MODEL_SYNTHESIS.md`.

## Design

### Components

Rules parsing:

- Add `ObjectType::construction_yard: bool`.
- Add `ObjectType::deploy_facing: u8`.
- Parse `ConstructionYard=` with false default.
- Parse `DeployFacing=` as raw byte `value << 5` with `0x80` default.

ConYard command gates:

- Add a sim-owned UI predicate for command exposure.
- Add a sim-owned runtime predicate for deterministic command acceptance.
- UI predicate includes production-busy hiding for `ConstructionYard=yes`.
- Runtime predicate requires structure, not building up/down, valid `UndeploysInto`, and for ConYards: human/player owner, `mcv_redeploy`, and no modeled blocking link.
- Non-ConYard `UndeploysInto` returns true before the ConYard-only `MCVRedeploys` gate.

Deploy-facing gate:

- `deploy_mcv` resolves the `DeploysInto` target object.
- It reads the target building type's `deploy_facing`.
- If the current modeled `u8` facing differs, set the target facing and return before despawning the MCV or spawning the building.
- Mark this as a temporary approximation of gamemd's rotate-and-return branch until a full deploy mission loop exists.

Placement blockers:

- `effective_build_blocked` must return blocked for foundation cells with overlay blockers, terrain-object blockers, nonzero slope, bridge structural `0x100`, or pure `0x400`.
- Existing mixed clear height acceptance must remain intact.

Cleanup:

- Remove `transferred_health` helper from this patch.
- Remove forward AMCV health/veterancy transfer from this patch.
- Remove reverse health/veterancy/refund changes from `tick_building_down` from this patch.
- Remove state-transfer/refund tests from this patch.

### Interfaces / Contracts

- `Simulation::should_show_undeploy_building_command(stable_id, rules) -> bool`
  - App/UI command exposure only.
  - Includes production-busy UI hiding.

- `Simulation::can_undeploy_building_runtime(stable_id, rules) -> bool`
  - Deterministic command acceptance.
  - Excludes the production-busy UI-only gate unless later RE proves runtime also checks it.

- `ObjectType::deploy_facing`
  - Raw 8-bit facing byte.
  - Never interpreted as unshifted INI direction after parsing.

### Data Flow

1. INI parse fills `ObjectType`.
2. UI asks sim whether selected/hovered structure should expose undeploy.
3. Command execution asks sim whether stale `UndeployBuilding` is legal.
4. AMCV deploy computes placement footprint and validates blockers.
5. AMCV deploy compares modeled current facing against target building `DeployFacing`.
6. Only after placement and facing gates pass does conversion proceed.

### Error Handling

- Invalid or missing `UndeploysInto` target returns false; no conversion starts.
- Missing `DeploysInto` target returns false; no conversion starts.
- Blocked placement returns false and leaves AMCV intact.
- Facing mismatch returns true/in-progress and leaves AMCV intact.

### Testing Strategy

Focused tests in `src/sim/deploy_tests.rs`:

- Parse `ConstructionYard` and `DeployFacing`.
- AMCV facing mismatch does not create ConYard.
- `DeployFacing` override is read from target building.
- Overlay, slope, nonbuildable LandType, live bridge, and pure `0x400` reject placement and keep AMCV.
- ConYard runtime rejects when `mcv_redeploy` is disabled.
- ConYard runtime rejects non-human owner.
- ConYard UI hides while building production is busy.
- Non-ConYard `UndeploysInto` is not gated by `MCVRedeploys`.

Verification commands:

```powershell
cargo fmt -- --check
cargo test -q deploy_mcv_
cargo test -q conyard_redeploy
cargo test -q mcv_redeploy_option
cargo test -q parse_construction_yard_and_deploy_facing
cargo check -q
```

## Architectural Decisions

- Keep command gate logic in `sim` so UI and runtime share one source of truth.
- Keep app modules thin; they call sim predicates and emit commands.
- Keep placement blocker logic at the terrain/build-blocked boundary rather than hardcoding the blocker taxonomy inside each test.
- Treat immediate facing snap as known temporary drift, not final parity, because the full deploy mission rotation path is not modeled in this cleanup.
- Do not use this pass to normalize or generalize the full lifecycle state machine.

## Alternatives Considered

### Leave current mixed patch as-is

Rejected. It mixes the requested safe gaps with state-transfer/refund changes and reverse veterancy copying, which violates the user's explicit ordering.

### Fold state transfer into the same design

Rejected. Some forward/reverse health/refund facts are implementation-safe, but reverse veterancy/experience and NACNST/SMCV generalization are not. Combining them here makes the safe-gap patch harder to audit.

### Re-investigate before any cleanup

Rejected for the safe-gap cleanup. The requested cleanup is to remove unapproved scope, not to expand the lifecycle surface. Re-investigation belongs before any later reverse veterancy/generalization work.

