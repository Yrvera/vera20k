# Bridge CABHUT Low Axis Bounded Collapse Trace

**Date:** 2026-05-22  
**Scenario:** SEAL/C4 on a `BridgeRepairHut` adjacent to a low bridge with representative `0x4A` and `0x53` body-overlay families.  
**Scope:** CABHUT/C4 low-bridge hut-death path through `DestroyBridgeFromCell_Low`, physical sweep-axis mapping, four-step collapse bound, and current Rust behavior after the physical-axis fix.  
**Non-scope:** high bridge variants, terminal overlays except as adjacent evidence, campaign trigger payloads, broad low bridge tube/path tie behavior, and unrelated bridge repair.  
**Status:** COMPLETE

## Pipeline

`SEAL C4 plant -> C4 timer expiry -> BridgeRepairHut branch -> hut 5x5 low evidence -> DestroyBridge_Low_OnHutDeath -> DestroyBridgeFromCell_Low -> CollapseBridge_{EW,NS}_Low bounded sweep -> per-cell DestroyBridge_Low retries -> bridge fallout/render refresh`

## Stage Verdicts

| Stage | Verdict | Rust surface | gamemd evidence | Notes |
|---|---|---|---|---|
| C4 on CABHUT is live and hut branch skips normal building damage | PASS | `src/sim/world/world_orders.rs:512`, `:536`, `:720` | `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`; `BuildingClass::Update @ 0x0043FB20`; stock `[CABHUT] BridgeRepairHut=yes` | Both paths use elapsed `>= C4Delay` and preserve the hut on the bridge-hut branch. Focused low-overlay test passed. |
| Hut-local scan is a 5x5 `[-2..=+2]` low/high selector | PASS | `src/sim/bridge_state/mod.rs:1504`; `src/sim/world/bridge_orchestrator.rs:177`, `:360` | `BuildingClass::Update @ 0x0043FB20`; `DestroyBridge_Low_OnHutDeath @ 0x00574C20` | Both scan 25 interior cells. Low overlay evidence selects the low path. |
| Low overlay family `0x4A` maps to physical EW sweep | PASS | `src/sim/world/bridge_orchestrator.rs:623`, `:1531` | Read-only Ghidra decompile `MapClass__DestroyBridgeFromCell_Low @ 0x00574780`; `MapClass__CollapseBridge_EW_Low @ 0x00575220` | gamemd branch for `0x4A..=0x52 / 0x5C..=0x5F / 0x64` calls `CollapseBridge_EW_Low`, whose decompile steps X. Rust now returns `Axis::EW` for `0x4A`. |
| Low overlay family `0x53` maps to physical NS sweep | PASS | `src/sim/world/bridge_orchestrator.rs:623`, `:1539` | Read-only Ghidra decompile `MapClass__DestroyBridgeFromCell_Low @ 0x00574780`; `MapClass__CollapseBridge_NS_Low @ 0x00575540` | gamemd branch for `0x53..=0x5B / 0x60..=0x63 / 0x65` calls `CollapseBridge_NS_Low`, whose decompile steps Y. Rust now returns `Axis::NS` for `0x53`. |
| Collapse walker bound is exactly 4 axial iterations | PASS | `src/sim/world/bridge_orchestrator.rs:326`, `:703` | Read-only Ghidra decompile `0x00575220` and `0x00575540` | gamemd uses loop count `4`; Rust uses `MAX_HUT_SWEEP_STEPS = 4`. |
| Per-step destroy retry bound is exactly 3 | PASS | `src/sim/world/bridge_orchestrator.rs:327`, `:707` | Read-only Ghidra decompile `0x00575220` and `0x00575540` | gamemd retries `DestroyBridge_Low` up to 3 times; Rust loops `MAX_HUT_ATTEMPTS_PER_STEP = 3`. |
| Current Rust low C4 overlay path reaches low collapse and preserves hut | PASS | `src/sim/world/world_orders_bridge_repair_tests.rs:890` | Same C4 entry report plus low dispatcher decompile | `cargo test --lib c4_on_cabhut_low_overlay_collapses_low_bridge` passed. This is not a full footprint-equality test. |
| Exact destroyed-cell footprint for long `0x4A` and `0x53` low bridges | UNCHECKED | `src/sim/world/bridge_orchestrator.rs:679` | `0x00575220`, `0x00575540` | Axis and bounds are checked, but this run did not compute a full gamemd destroyed-cell set and matching Rust set for a long low bridge fixture. |
| Canonical seed adjustment before collapse walker | UNCHECKED | `src/sim/world/bridge_orchestrator.rs:235`; per-cell canonicalization in `src/sim/bridge_state/walker.rs:553`, `:574` | `DestroyBridgeFromCell_Low @ 0x00574780` | gamemd canonicalizes around the matched overlay before calling `CollapseBridge_*_Low`. Rust canonicalizes inside each per-cell `destroy_bridge_low` call, but this trace did not prove the same initial collapse seed for every 3-wide low bridge placement. |
| Exact BridgeExplosions timing/RNG/order during low collapse | NOT-IMPLEMENTED | `src/sim/world/bridge_orchestrator.rs:299` | `CollapseBridge_EW_Low @ 0x00575220`; `CollapseBridge_NS_Low @ 0x00575540` | gamemd spawns three `BridgeExplosions` before each per-step `DestroyBridge_Low` call. Rust aggregates destroyed cells and calls `spawn_bridge_debris` after state mutation, so visible timing/RNG ordering is not exact. |

## Evidence Notes

- Ghidra MCP use was read-only: decompile only, no labels/comments/renames/saves.
- The decompiled low dispatcher and collapse walkers are active in standard YR. Evidence: `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md` ties `BuildingClass::Update` hut calls to `0x00574C20`, then `0x00574780`, then `0x00575220`/`0x00575540`; stock YR has `CABHUT BridgeRepairHut=yes` and C4 infantry.
- Focused verification commands:
  - `cargo test --lib hut_destroy_overlay_seed_uses_physical_span_axis_not_walker_family`
  - `cargo test --lib c4_on_cabhut_low_overlay_collapses_low_bridge`

## Failures

No direct FAIL was proven for physical sweep axis or four-step bound after the recent fix.

## Not Implemented

- Exact per-iteration `BridgeExplosions` timing/RNG/order is not implemented in the Rust hut-collapse path. The player-visible symptom is that bridge explosion/debris timing can occur after aggregated state mutation rather than before each gamemd per-step destruction attempt.

## Adjacent Findings

- The current axis regression is good coverage for `0x4A` and `0x53`, but it is a mapping-level test. A stronger follow-up is a long low bridge fixture for both families that asserts far cells survive and compares exact destroyed-cell coordinates.
- Terminal low overlays such as `0x65` are not the concrete body-overlay scenario here. Existing Rust tests check that terminal overlay scan hits do not fall through to fallback, but exact gamemd tail refresh behavior for no-change terminal cases was not traced in this run.

## Verdict Tally

PASS: 7 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1
