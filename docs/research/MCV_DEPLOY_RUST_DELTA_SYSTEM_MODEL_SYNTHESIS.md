# MCV Deploy / Redeploy Rust Delta - System Model Synthesis

**Date:** 2026-05-27  
**Scope:** stock YR MCV/start-MCV, `AMCV/SMCV/PCV -> ConYard`, and stock `GACNST -> AMCV` redeploy deltas where current research docs agree and current Rust still diverges.  
**Non-scope:** full Slave Miner economy, full sidebar shell parity, generic sell/repair, and unverified mod-only deployer branches.  
**Output type:** implementation delta map.  
**Overall safety:** implementation-safe for the listed stock AMCV/GACNST facts unless marked `needs reinvestigation`; selected-mode `MCVDeploy` startup auto-deploy is not a Rust bug because newer docs verify no selected-mode auto-deploy.

## Current Rust Shape

The current Rust path is still a simplified replacement conversion:

- `world_spawn.rs::deploy_mcv` resolves `DeploysInto`, validates the target footprint through Rust terrain/structure checks, handles a byte-facing mismatch by setting `facing` immediately, then despawns the unit and spawns the target building.
- Forward deploy currently transfers selection only and gives the building a fixed 30-tick `BuildingUp`.
- `world_spawn.rs::undeploy_building` records a `BuildingDown` component with target type, owner, cell, z, and selected state.
- `world/mod.rs::tick_building_down` despawns the source building first, tries to spawn the unit, and transfers only selection.
- ConYard redeploy gates are now partially modeled: `MCVRedeploys`, human owner, production-busy UI hide, `building_up/down`, and `radio_contacts` blocker are present. A distinct MP/skirmish-mode gate is not modeled.
- `[SpecialFlags] MCVDeploy` is parsed in `map/basic.rs`, but selected-mode skirmish startup correctly does not auto-deploy from it.

## Agreed Doc Facts That Still Diverge From Rust

| Priority | Agreed fact | Evidence | Current Rust mismatch | Status |
|---|---|---|---|---|
| P0 | Forward deploy is a mission/state-machine path; facing mismatch orders a turn and returns in-progress without creating the building. | `MCV_DEPLOY_GHIDRA_REPORT.md`; `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md` | Rust sets `facing_target` and `facing` to the target immediately, returns true, and has no modeled mission retry/continuation. | drift |
| P0 | Successful `AMCV -> GACNST` transfers source health ratio, veterancy, experience, upgrade/powerup-style links, and other state before destroying the AMCV. | `MCV_DEPLOY_GHIDRA_REPORT.md`; `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md` confirms analogous reverse transfer categories | Rust despawns the MCV, spawns a default-health building, and only copies selection. | drift |
| P0 | Successful `AMCV -> GACNST` retargets alive technos whose active combat target is the old AMCV to the new ConYard, except the verified ConYard+Doggie infantry clear case. | `MCV_DEPLOY_TARGET_REDIRECTION_EXCEPTIONS_GHIDRA_REPORT.md` | Rust does not rewrite `GameEntity.attack_target` from old MCV id to new building id. | drift |
| P0 | Successful `GACNST -> AMCV` computes AMCV health from saved source health ratio times AMCV strength, floor, min 1, and writes both health mirrors. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md` | Rust spawns the AMCV at default full health. | drift |
| P0 | `GACNST -> AMCV` success collects links/targets before source removal, moves destination/order, sound-event fields, optional powerup manager, copied techno fields, selection-without-voice, and retargets technos pointing at the source. | `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md` | Rust despawns first and therefore loses source data/link context; only `selected` survives. | drift |
| P0 | Failed final AMCV unlimbo after ConYard pack-up removes the GACNST, places no AMCV, and refunds the source building `vtable+0x2BC` sell-back integer. | `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` | Rust removes the building and pays no refund if final spawn fails. | drift |
| P1 | AI/non-player-control ConYard deploy updates house base center, base-plan state, AI flags, and can move eligible owned technos near the base. Player-control houses skip this branch. | `GACNST_ISDEPLOYABLE_SPECIAL_BRANCH_GHIDRA_REPORT.md` | Rust deploy does not run an AI-only ConYard setup branch; launch-time base centers are not equivalent to this deploy-side update. | drift |
| P1 | ConYard redeploy gate includes an MP/skirmish-style mode requirement in addition to human owner, `MCVRedeploys`, and link clear. | `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`; `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md` | Rust has no distinct game-mode field/gate, so campaign/SP-style ConYard redeploy cannot be rejected for the same reason. | partial drift |
| P1 | Forward deploy creates a building with mission `0x12` and runs construction-complete/GrandOpening style side effects later; `FreeUnit` is only consumed by `OnConstructionComplete`, not AMCV deploy itself. | `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md` | Rust gives a generic fixed `BuildingUp` timer and lacks a gamemd-equivalent construction mission/completion lifecycle for the deployed ConYard. It correctly does not spawn `CMIN`. | drift, with free-unit negative fact aligned |
| P2 | Opening/start MCV placement fallback in selected skirmish uses the native selected-mode callback and `FUN_00688ED0` radius/randomized compass/jitter behavior. | `skirmish-ui/SKIRMISH_MCV_NEARBY_PLACEMENT_FALLBACK_00688ED0_GHIDRA_REPORT.md`; skirmish start traces | Rust uses its own deterministic fallback directions and simplified launch placement. | drift outside deploy body |
| P2 | Null-mode startup generator's only direct `Force_MCV_Deploy` call is separate from selected-mode launch and is gated by low byte `0x10`, not parsed `[SpecialFlags] MCVDeploy` bit `0x0100`. | `skirmish-ui/SPECIALFLAGS_MCVDEPLOY_BIT_CONFLICT_RECHECK_GHIDRA_REPORT.md`; `SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md` | Rust has no null-mode generator. This is a future missing path, not a selected skirmish bug. | missing future path |

## Already Aligned Or Not A Current Bug

- Forward deploy origin for 4x4 ConYards is now aligned with the verified `unit_cell + (-1,-1)` rule in Rust's `deploy_origin_from_center`.
- Deploy footprint blocker coverage is much closer than older docs: current `effective_build_blocked` checks structural bridge facts, destroyed/ramp bridge facts, overlay blockers, terrain-object blockers, slope, dynamic bridge state, and `build_blocked`.
- Mixed-height clear foundations should remain accepted; Rust has coverage for this and docs agree.
- AMCV deploy should not spawn a free `CMIN`; current Rust's no-CMIN behavior is correct for stock `GACNST`.
- Selected-mode skirmish should not auto-deploy startup MCVs from `[SpecialFlags] MCVDeploy`; newer skirmish UI reports supersede older broad auto-deploy wording.
- `mcv_redeploy` is the `MCVRedeploys` / MCV Repacks option, not `[SpecialFlags] MCVDeploy`; current Rust keeps those concepts separate.

## Recommended Fix Order

1. Replace forward deploy's one-shot conversion with a minimal mission/deploy-in-progress state: preserve target type, desired deploy facing, retry/continue after facing reaches the target byte, and only create the building on the matched-facing call.
2. Implement forward deploy transfer after successful building placement: health ratio, veterancy/experience, modeled upgrade/powerup/link fields, target redirection, source cleanup ordering, and no AMCV consumption on failure.
3. Rework `BuildingDown` completion for ConYard redeploy: snapshot source health/refund/link/sound/destination/selection data before despawn, attempt the one computed AMCV spawn, branch success vs failure, and refund on failed spawn.
4. Add AI/non-player ConYard setup side effects behind `ConstructionYard=yes`, multiplayer/skirmish mode, and non-player-control owner.
5. Add a real mode/session gate so ConYard redeploy can distinguish MP/skirmish from campaign/SP-style contexts.
6. Keep selected skirmish MCV startup as no-auto-deploy; only implement the null-mode low-flag `Force_MCV_Deploy` path if/when the null-mode generator is modeled.

## Needs Re-Investigation Before Broad Generalization

- Reverse `GACNST -> AMCV` veterancy/experience copy exact offsets: broad docs say yes, focused success report does not fully name it.
- Non-Allied reverse `NACNST -> SMCV` / `YACNST -> PCV` transfer/refund parity: likely generic, but not proven with a side-specific focused report.
- Exact semantic names/readers for `Techno+0x214`, `+0x150`, `+0x2B4`, and source `+0x34`; copy/rebind operations are proven, names remain provisional.
- Persistent sound loop/event handle modeling: binary copies/clears sound-event fields, but Rust lacks equivalent persistent per-object sound handles.

## Source Ledger

- `MCV_DEPLOY_GHIDRA_REPORT.md`
- `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`
- `AMCV_DEPLOY_FACING_RULE_GHIDRA_REPORT.md`
- `BUILDING_MCV_DEPLOY_ORIGIN_BLAST_RADIUS_GHIDRA_REPORT.md`
- `GACNST_DEPLOY_PLACEMENT_BLOCKER_TAXONOMY_GHIDRA_REPORT.md`
- `GACNST_FREE_UNIT_AFTER_AMCV_DEPLOY_GHIDRA_REPORT.md`
- `GACNST_ISDEPLOYABLE_SPECIAL_BRANCH_GHIDRA_REPORT.md`
- `GACNST_SUCCESSFUL_REDEPLOY_TRANSFER_GHIDRA_REPORT.md`
- `GACNST_UNDEPLOY_UNLIMBO_FAIL_REFUND_GHIDRA_REPORT.md`
- `MCV_DEPLOY_TARGET_REDIRECTION_EXCEPTIONS_GHIDRA_REPORT.md`
- `MCV_REDEPLOY_UI_COMMAND_GATE_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_SELECTED_MODE_MCVDEPLOY_START_FLAG_GHIDRA_REPORT.md`
- `skirmish-ui/SPECIALFLAGS_MCVDEPLOY_BIT_CONFLICT_RECHECK_GHIDRA_REPORT.md`
- Rust scan: `src/sim/world/world_spawn.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_commands.rs`, `src/map/basic.rs`, `src/app_skirmish.rs`, `src/sim/game_options.rs`
