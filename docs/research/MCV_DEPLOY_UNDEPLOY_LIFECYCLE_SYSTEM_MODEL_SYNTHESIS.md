# MCV Deploy / Undeploy Lifecycle - System Model Synthesis

**Date:** 2026-05-27  
**Scope:** stock YR `AMCV/SMCV/PCV -> ConYard`, focused stock `GACNST -> AMCV` reverse conversion where narrow reports exist, and adjacent deploy failure, command gate, placement, facing, origin, health/refund, selection, sound, and link-transfer facts.  
**Non-scope:** full Slave Miner economy lifecycle, generic simple deployers, full sell/garrison survivor flow, full UnitClass/BuildingClass unlimbo internals, and unmodeled persistent audio/loop-handle implementation.  
**Output type:** model-synthesis with conflict/unknown map.  
**Overall safety:** partially implementation-safe. The core AMCV deploy failure, origin, facing source, placement blocker categories, ConYard redeploy gates, forward health/modeled-veterancy transfer, successful `GACNST -> AMCV` health transfer, and failed reverse refund facts are implementation-safe. Reverse veterancy/experience transfer and non-Allied ConYard reverse parity need a focused check before being treated as implementation-safe. The broad lifecycle side-effect set is not yet one implementation-safe patch because several fields are verified as copied/rebound but not fully named/modeled in Rust.

## Current Model

The lifecycle is not one simple spawn/despawn operation in `gamemd.exe`; it is two conversion pipelines with different gates and delayed side effects.

Forward deploy (`AMCV -> GACNST`) enters `UnitClass::Deploy @ 0x007393C0`. The unit must pass generic deploy readiness, then the target building type validates the footprint through `BuildingTypeClass` vtable `+0xA8 -> 0x00716150`, which walks the target foundation and calls `Cell_passability_building_placement @ 0x0047C620`. Placement failure can play `EVA_CannotDeployHere` for a human owner and does not consume the MCV. Success creates the building only after facing, origin, and placement gates pass, then transfers source state and destroys the unit.

Reverse undeploy (`GACNST -> AMCV`) is a ConYard-only command-gated conversion through `BuildingClass__CanUndeployMCV @ 0x00449BC0`, `BuildingClass__ShouldShowDeployButton @ 0x0044F5C0`, and the `BuildingClass__Sell @ 0x00449C30` state-2 completion path. The stock ConYard command requires `UndeploysInto`, `ConstructionYard=yes`, MP/skirmish-style mode, human/player control, `MCVRedeploys`, and no blocking link field. UI visibility also hides the action while production-busy. On reverse buildup completion, the building is detached before AMCV unlimbo. If AMCV placement succeeds, health ratio, selected state, several techno fields, destination/order, sound-event fields, optional powerup manager state, and linked techno targets are transferred. If AMCV unlimbo fails, no AMCV appears, the ConYard is removed, and the owner receives the source building refund value. The older broad MCV report indicates reverse conversion transfers veterancy/experience, but the focused GACNST success report used here does not clearly identify that field copy; treat reverse veterancy as `NEEDS_REINVESTIGATE` until the exact copy site is checked.

At the source scan used for this synthesis, current Rust was closer than the original mismatch: the scanned sources showed forward health/modeled-veterancy transfer plumbing, reverse health transfer plumbing, failed reverse refund plumbing, and blocked-deploy EVA plumbing. However, Rust still models a simplified `BuildingUp` / `BuildingDown` lifecycle and does not yet have a full gamemd-equivalent mission/state-machine, link-retarget, persistent sound-loop, powerup-manager, deploy-facing delay, or ConYard redeploy command-gate model.

## Implementation-Safe Facts

| Claim | Best evidence | Status | Active in YR | Safe? |
|---|---|---|---|---|
| AMCV deploy footprint failure plays `EVA_CannotDeployHere` for human stock AMCV contexts and leaves AMCV intact. | `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`; trace `AMCV_DEPLOY_BLOCKED_EVA_TRACE_2026-05-27.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| AMCV target footprint validation is target `BuildingType +0xA8` over base foundation cells, not only UnitClass `+0x314`. | `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Stock GACNST deploy rejects occupied cells, normal overlays, slope, nonbuildable LandTypes, bridge `0x100`, and bridge inactive/fallback `0x400`; mixed clear height alone is accepted. | `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Unit-to-building deploy origin is `unit_cell + (-1,-1)` for foundations larger than 2x2, otherwise `unit_cell`. | `BUILDING_MCV_DEPLOY_ORIGIN_BLAST_RADIUS_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| AMCV deploy-facing target comes from the `DeploysInto` building type `DeployFacing` raw byte; stock GACNST default is `0x80`. | `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| ConYard redeploy is gated by ConYard-specific MP/human/MCVRedeploys/link checks; `MCVRedeploys` is not a universal `UndeploysInto` gate. | `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md` | confirmed, one caller-feedback question remains | conditional | IMPLEMENTATION_SAFE for gates |
| Successful `GACNST -> AMCV` health uses saved source health ratio times new unit strength, floor, min 1. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Reverse transfer reads state at state-2 completion after reverse animation gate, not command issue time. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md`; `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Failed AMCV unlimbo refunds the source building `vtable+0x2BC` integer refund; it is not health ratio or AMCV cost. | `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Successful reverse path transfers selected state without selection voice and retargets collected technos whose link field pointed at the source building. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md` | confirmed for operations, field names partly provisional | conditional | IMPLEMENTATION_SAFE for operation ordering; names need caution |
| Reverse `GACNST -> AMCV` veterancy/experience copy. | broad `MCV_DEPLOY_GHIDRA_REPORT.md`; not clearly named in focused GACNST success report | touched but not narrowed | likely yes, unchecked | NEEDS_REINVESTIGATE |
| `NACNST -> SMCV` and other non-Allied ConYard reverse transfer/refund parity. | broad MCV overview plus generic `UndeploysInto` mechanism; no focused reverse-transfer report in this synthesis | plausible generic mechanism, not spot-checked | likely yes, unchecked | NEEDS_REINVESTIGATE |

## Doc-Patch-Ready Facts

- Older broad MCV docs that say production queue uses `entity[0x141]` should be refined to the sharper UI visibility evidence: `BuildingClass+0x504 > 0` in `ShouldShowDeployButton`.
- Older wording that implies UnitClass `+0x314` is the full GACNST footprint validator should be replaced with the split: generic readiness in `+0x314`, target footprint in building type `+0xA8`.
- Any doc implying same-height GACNST foundations are required is stale. Slope is a blocker; mixed clear cell height is not.
- Any doc implying failed reverse unlimbo uses a high dword of the health-ratio double is stale. The refund is a separate integer slot from source `vtable+0x2BC`.
- Any doc implying AMCV deploy uses `[AMCV] DeployFacing` or `[General] DeployDir` is stale. The source is the target building type's `DeployFacing`.
- Any implementation note that treats reverse Rust `veterancy` copy as fully proven by the focused GACNST success report should be softened until the exact reverse veterancy/experience field copy is located.

## Stale Or Superseded Claims

| Old claim | Replacement | Evidence |
|---|---|---|
| Rust MCV deploy should center the building by subtracting half the foundation. | gamemd applies only `(-1,-1)` for foundations larger than 2x2; 2x2 keeps unit cell. | `BUILDING_MCV_DEPLOY_ORIGIN_BLAST_RADIUS_GHIDRA_REPORT.md` |
| ConYard redeploy is available whenever `UndeploysInto` exists. | ConYards require the special MP/human/MCVRedeploys/link gate; non-ConYards bypass it. | `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md` |
| Broad lifecycle can be represented as only 30-tick `BuildingDown` then default spawn. | gamemd state-2 completion does refund/health/destination/sound/selection/link transfer and source cleanup ordering. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md`; `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` |
| Failed blocked deploy only returns false/logs. | Stock AMCV placement failure also plays `EVA_CannotDeployHere` for human player contexts. | `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md` |

## Cross-Doc Conflicts

- The broad `MCV_DEPLOY_GHIDRA_REPORT.md` remains useful as an overview, but several narrow reports supersede details: production-busy field, exact placement gate split, deploy-facing source, failed-refund provenance, and blocker taxonomy.
- Slave Miner/YAREFN reports show a related `UndeploysInto` lifecycle but are not a drop-in MCV rule. YAREFN is not gated like a ConYard, and SlaveManager transfer details must not be generalized to GACNST unless a GACNST report proves the same field path.
- The current Rust state-transfer plan is derivative. Use the underlying Ghidra reports as authority when the plan and code disagree.

## Needs Re-Investigation

1. `/re-investigate MCV deploy command dispatcher stale ConYard redeploy rejection feedback`
   - Reason: reports prove false/no conversion, but exact user feedback if a stale/networked command reaches `0x00449BC0` is not freshly traced.
2. `/re-investigate Techno link fields copied during GACNST redeploy +0x214 +0x150 +0x2B4 +0x34`
   - Reason: copy/rebind operations are verified, but exact field names and all readers are not.
3. `/re-investigate GACNST redeploy veterancy experience field copy`
   - Reason: broad overview says reverse conversion transfers veterancy/experience, but the focused GACNST success report proves health and several copied fields without clearly naming veterancy/experience offsets.
4. `/re-investigate NACNST SMCV reverse redeploy transfer parity`
   - Reason: GACNST/AMCV reverse facts are likely generic through `UndeploysInto`, but exact-mechanism parity needs side-specific confirmation before implementation-safe generalization.
5. `/re-investigate MCV deploy persistent sound event and loop handle lifecycle`
   - Reason: sound-event field copy/clear is verified, but Rust lacks equivalent persistent sound handle modeling.
6. `/re-investigate AMCV deploy nonstandard deploy-target flag type+0x5EC EVA suppression`
   - Reason: stock AMCV appears safe, but mod/nonstandard deployer behavior should not inherit EVA blindly.
7. `/trace-swarm MCV lifecycle blockers overlay slope nonbuildable bridge-marker deploy plus ConYard redeploy gate`
   - Reason: existing trace passes the structure-blocked EVA case; remaining blocker classes and redeploy UI/runtime gates need concrete end-to-end traces.

## Do-Not-Implement Notes

- Do not make one broad "spawn replacement with defaults" helper the lifecycle model.
- Do not copy movement queues as a substitute for gamemd's verified destination/link retargeting.
- Do not globally apply `MCVRedeploys` to all `UndeploysInto` buildings.
- Do not reintroduce a mixed-height foundation rejection.
- Do not copy AMCV body facing into the ConYard as a parity rule.
- Do not treat unmodeled persistent sound loop transfer as solved by one queued sound event.
- Do not claim broad lifecycle parity until deploy-facing delay, ConYard command gates, target/link rebinding, and persistent sound/powerup side effects are either modeled or explicitly proven irrelevant for byte/pixel/audio-visible state.
- Do not generalize GACNST-focused reverse transfer/refund facts to every ConYard side without a focused check or a proven generic branch argument.

## Rust Touchpoints

- `src/sim/world/world_spawn.rs::deploy_mcv`: deploy origin, placement checks, blocked-deploy EVA event, forward health/veterancy transfer, future deploy-facing delay.
- `src/sim/world/world_spawn.rs::undeploy_building`: starts delayed `BuildingDown`; should remain metadata-only for live completion-time transfer.
- `src/sim/world/mod.rs::tick_building_down`: reverse completion ordering, source-state read, occupancy cleanup, spawn success transfer, failed-spawn refund.
- `src/app_context_order.rs` and `src/sim/world/world_commands.rs`: ConYard command exposure and runtime acceptance gates.
- `src/app_sim_tick.rs`, `src/audio/events.rs`, `src/app_building_anim.rs`: deploy-failure EVA routing.
- `src/sim/production/production_sell.rs`: source building refund helper equivalent to gamemd `GetRefundValue`.

## Source Ledger

- `MCV_DEPLOY_GHIDRA_REPORT.md`
- `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`
- `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`
- `BUILDING_MCV_DEPLOY_ORIGIN_BLAST_RADIUS_GHIDRA_REPORT.md`
- `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`
- `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`
- `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md`
- `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`
- `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`
- `SLAVE_MINER_DEPLOY_SMIN_YAREFN_PATH_GHIDRA_REPORT.md`
- `SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`
- Trace: `traces/AMCV_DEPLOY_BLOCKED_EVA_TRACE_2026-05-27.md`
- Plan reference: `docs/plans/2026-05-27-mcv-state-transfer-plan.md`
- INI anchors: `ini/rulesmd.ini` `[MultiplayerDialogSettings]`, `[AMCV]`, `[GACNST]`; `ini/artmd.ini` `[GACNST]`; `ini/evamd.ini` `[EVA_CannotDeployHere]`
