# Bridge Parity Gap System Model Synthesis

**Date:** 2026-05-23  
**Scope:** current Rust bridge parity gaps across bridge damage/collapse, repair side effects, movement/pathing, target/click selection, superweapons, radar/minimap, and rendering.  
**Non-scope:** new implementation, full retail route capture, campaign trigger payloads.  
**Output type:** implementation gap map.  
**Status:** IMPLEMENTATION_SAFE for the listed deltas where marked; investigation-blocked only for exact stock route/visual-table capture cases.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| High bridgehead slot `+3` direct damage collapses; Rust `bridgehead_advance_state` never collapses. | `HIGH_BRIDGE_COLLAPSE_STATE_MACHINE_GHIDRA_REPORT.md`; current `src/sim/bridge_state/mod.rs` scan | confirmed gap | high | yes | IMPLEMENTATION_SAFE |
| Current Rust zone precheck is still a corridor/Dijkstra approximation, not full three-level `Zone_precheck`. | `BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md`; current `zone_search.rs` header/body | confirmed gap | high | yes | IMPLEMENTATION_SAFE for next patch shape |
| Exact post-collapse route choices, especially low bridge detours, are not yet proven. | `STOCK_LOW_BRIDGE_COLLAPSE_ROUTE_TRACE_GHIDRA_REPORT.md`; dual-layer synthesis | unknown | medium | yes | NEEDS_RUNTIME_TRACE |
| ParaDrop/AmerParaDrop bridge-click target replacement is missing. | `PARADROP_BRIDGE_TARGET_VALIDATION_GHIDRA_REPORT.md`; `src/sim/superweapon/paradrop.rs` comment | confirmed gap | high | yes | IMPLEMENTATION_SAFE |
| Passive/opportunity-fire target acquisition lacks the verified bridge `OnBridge` candidate filter and threat-score ranking. | `GRIZZLY_*BRIDGE_FILTERS*`; current targeting docs/source scan | confirmed gap | high | yes | IMPLEMENTATION_SAFE |
| Bridge body shadows are still not rendered. | `BRIDGE_RENDERING_REMAINING_CASES_GHIDRA_REPORT.md`; `src/app_render/draw_passes.rs` | confirmed gap | high | yes | IMPLEMENTATION_SAFE |
| Bridge railings are drawn after objects/cliff merge, while binary railings are in the terrain bridge bundle. | same rendering report; current draw pass order | confirmed mismatch | medium | yes | NEEDS_VISUAL_DIFF before changing |
| Low bridge destroyed/healthy visuals are not fully mask/tile-mutation driven. | rendering report; low-bridge selector docs; current overlay/runtime split | confirmed gap | medium | yes | IMPLEMENTATION_SAFE for system design |
| Tactical pixel-to-cell inverse remains imperfect for off-map sentinel behavior. | live spot-check `0x006D6590`; current `world_point_to_cell` clamp | confirmed narrow gap | medium | yes | IMPLEMENTATION_SAFE |
| DestroyableBridges ownership, bridge repair SFX/EVA/radar event, superweapon AoE bridge Z for Lightning/Genetic, bridge debris RNG, low-zone records, and bridge A* marker/flank cost have current Rust coverage. | current source scan; May 22 docs | not current blockers | high | yes | DO_NOT_REOPEN without fresh regression |

## Current Gap Map

### 1. Bridge damage/collapse

The highest-impact remaining gameplay miss is high bridgehead direct damage. Binary `ProcessBridgeDamageStateMachine_High` has a bridgehead class slot `+3` branch that runs collapse and returns success. Current Rust `BridgeRuntimeState::bridgehead_advance_state` explicitly never returns `Collapsed`. This means sustained/direct damage on already-most-damaged high bridgehead pieces can fail to collapse where YR would visibly destroy bridge cells.

Normal body-cell collapse, CABHUT C4 bounded collapse, destroyability gate ownership, bridge-strength RNG, Ion bypass, and debris RNG appear to have current Rust coverage. Do not use older reports that still say Rust has no destroyability gate, no repair flow, no debris, or wrong debris RNG without rechecking current source.

### 2. Movement/pathing/zones

Rust now models many bridge-specific path pieces: dual layers, bridge records, low records for all-active zone adjacency, high-only bridge redirect, search-scoped `0x40000` marker, bridge flank costs, low tube identity, and explicit-tube bypasses.

The remaining mismatch is the route-planning system around those pieces. `zone_search.rs` still documents itself as a reduced corridor approximation. It has fixed the old ZoneId tie issue, but it is not full gamemd `Zone_precheck`: no full three-level hierarchy, parent-chain gating, exact zone edge byte cost, or exact retry-local exclusion model. Player-visible effect: after bridge collapse or blockage, units can choose a different detour, fail differently, or retry differently.

Exact stock low-bridge post-collapse route parity remains runtime-trace blocked; the Carville fixture is identified but not fully logged.

### 3. Targeting/input/superweapons

The tactical click inverse now uses the 180-step vertical scan shape and cardinal bridge checks, confirmed against a live spot-check of `0x006D6590`. The narrow mismatch is that app-level `world_point_to_cell` clamps fallback/off-map results to `(0,0)`-style `u16` cells instead of preserving the binary sentinel behavior.

ParaDrop bridge clicks are still wrong. Binary does not abort bridge targets; it attempts a nearby passable replacement and only uses the replacement if it is valid and non-bridge-surface. Current Rust always keeps the clicked target.

Combat/passive targeting remains bridge-incomplete. For Grizzly-style `OpportunityFire=yes`, binary candidate filtering rejects an `OnBridge` mismatch only when both attacker and target cells are structural bridge cells, and selection is threat-score based. Current Rust targeting is still nearest-first in relevant paths and does not apply that scoped bridge candidate filter.

Lightning Storm and Genetic Converter now pass bridge-adjusted AoE impact Z and have bridge-layer tests, so those are not current bridge blockers from this scan.

### 4. Rendering/presentation

Bridge body rendering has caught up on the major state-byte Y offsets and the railing lookup table is no longer the old all-zero placeholder. Remaining visible issues:

- Body shadows are built but the draw pass is disabled because the SHP shadow/darken blitter is missing.
- Railing draw order differs from gamemd: Rust draws railings after object/cliff merge; binary emits railings inside the terrain bridge bundle before object rendering. This needs screenshot diffing with units on/under a bridge before changing because Z-buffer/object internals may interact.
- Low bridge visuals are still too overlay-like. Binary healthy/destroyed low bridge visuals are selected through mask/tile rewrite helpers, not just ordinary overlay rendering.
- Under-bridge object occlusion should be validated with a tank/unit/ship under a high bridge after any draw-order changes.

Radar/minimap repair event and dirty-cell channels appear to have current Rust coverage (`RadarEventType::BridgeRepaired`, dirty generation, minimap bridge dirty application), so older “missing bridge repaired radar event” notes are stale.

## Ranked Next Fixes

1. **High bridgehead slot `+3` collapse**  
   File surface: `src/sim/bridge_state/mod.rs`, `src/sim/bridge_specs.rs`, `src/sim/world/bridge_orchestrator.rs`.  
   Acceptance: an already-AboutToFall high bridgehead hit collapses and produces the three axial `BlowUpBridge` cells plus ramp collapse side effects.

2. **ParaDrop bridge target replacement**  
   File surface: `src/sim/superweapon/paradrop.rs` plus a bridge-surface/nearby-cell helper.  
   Acceptance: bridge click tries replacement; valid non-bridge replacement is used; failed/bridge replacement keeps original target.

3. **Bridge-aware passive targeting**  
   File surface: `src/sim/combat/combat_targeting.rs`, passive scan scheduling.  
   Acceptance: moving MTNK does not acquire an on-bridge-mismatched target when both cells are structural bridge cells; higher threat beats nearer low threat.

4. **Zone precheck fidelity after bridge collapse**  
   File surface: `src/sim/pathfinding/zone_search.rs`, `zone_map.rs`, `zone_build.rs`.  
   Acceptance: equal-cost order remains stable and a runtime-traced low-bridge post-collapse route matches stock.

5. **Bridge rendering shadows and low-bridge tile mutation**  
   File surface: `src/app_render/draw_passes.rs`, bridge/overlay/minimap render paths, low-bridge state mutation.  
   Acceptance: bridge body shadows draw with a shadow blitter equivalent; low bridge destroyed visuals use selector/tile mutation; screenshot diff covers on-deck and under-bridge units.

## Do-Not-Implement Notes

- Do not reintroduce `[CombatDamage] DestroyableBridges` as the active source for skirmish/multiplayer; current ownership is `SpecialFlags`/session option.
- Do not make ParaDrop abort on bridge clicks.
- Do not make bridge target filtering a generic Z-level mismatch. The verified passive-acquire bridge rejection is scoped to both cells being structural bridge cells plus `OnBridge` mismatch.
- Do not replace the bridge debris RNG with small integer probability gates; current normalized draw shape appears fixed.
- Do not implement CABHUT collapse as full-span flood fill; bounded collapse is the verified model.

## Source Ledger

- Live Ghidra spot-check: `FUN_006D6590` tactical inverse confirms 180-attempt vertical scan, bridge structural branch, strict `>15` edge tests, and fallback path.
- Live Ghidra spot-check: `FUN_006B8AE0` confirms default bit reset includes `0x8000` for bridge destroyability.
- `HIGH_BRIDGE_COLLAPSE_STATE_MACHINE_GHIDRA_REPORT.md`
- `BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md`
- `BRIDGE_COLLAPSE_SYSTEM_MODEL_SYNTHESIS.md`
- `PARADROP_BRIDGE_TARGET_VALIDATION_GHIDRA_REPORT.md`
- `GRIZZLY_PASSIVE_TARGET_SCANNER_VTABLE_39C_GHIDRA_REPORT.md`
- `GRIZZLY_OPPORTUNITYFIRE_VISIBILITY_CLOAK_BRIDGE_FILTERS_GHIDRA_REPORT.md`
- `BRIDGE_RENDERING_REMAINING_CASES_GHIDRA_REPORT.md`
- `BRIDGE_PRESENTATION_RADAR_DIRTY_GHIDRA_REPORT.md` checked against current source; its missing-radar-event Rust status is stale.
- Current Rust scan: `src/sim/bridge_state/mod.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/pathfinding/zone_search.rs`, `src/sim/superweapon/paradrop.rs`, `src/sim/superweapon/lightning_storm.rs`, `src/sim/superweapon/genetic_converter.rs`, `src/app_render/draw_passes.rs`, `src/render/bridge_railing_atlas.rs`, `src/render/minimap.rs`, `src/sim/radar.rs`.
