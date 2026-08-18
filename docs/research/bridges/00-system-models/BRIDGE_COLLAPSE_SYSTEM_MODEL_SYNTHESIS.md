# Bridge Collapse System Model Synthesis

**Date:** 2026-05-22  
**Scope:** C4/BridgeRepairHut bridge-collapse entry, `DestroyBridgeFromCell_*`, bounded `CollapseBridge_*_*` sweep, fallout ordering, and current Rust implementation risk.  
**Non-scope:** engineer repair mutation, generic weapon/AoE bridge damage except where it shares fallout, campaign trigger action semantics beyond event `0x1F` classification.  
**Output type:** conflict-map plus implementation-safe model.  
**Status:** IMPLEMENTATION_SAFE for bounded collapse scope and CABHUT entry; NEEDS_REINVESTIGATE only for campaign trigger payloads and exact Rust RNG/audio parity.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| CABHUT C4 uses BridgeRepairHut branch before normal building damage; hut survives. | `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`; trace `CABHUT_C4_TIMER_EXPIRY_BRIDGE_BRANCH_TRACE.md`; `BuildingClass::Update @ 0x0043FB20` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Hut entry scans for bridge overlays and dispatches through `DestroyBridgeFromCell_Low/High`. | `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`; `0x00574000`, `0x00574C20` reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `DestroyBridgeFromCell_High` routes high subranges to canonical `CollapseBridge_*_High` walkers. | live Ghidra spot-check `0x005749C0` on 2026-05-22 | confirmed | high | yes | IMPLEMENTATION_SAFE |
| CABHUT collapse is full-span. | contradicted by `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`; live Ghidra spot-check `0x00575BA0` | contradicted | high | yes | DOC_PATCH_READY |
| CABHUT collapse uses a bounded four-iteration sweep. | live Ghidra `0x00575BA0`: `local_2c = 4`; report decompiles all four walker twins | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Each sweep iteration spawns three `BridgeExplosions` before calling `DestroyBridge_*` up to three retries. | live Ghidra `0x00575BA0`; `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE for ordering; RNG exactness separately audited |
| `0xCD..0xD5 / 0xDF..0xE2 / 0xE7` are physical EW high bridge ranges, despite Ghidra label naming. | `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 7; live `0x005749C0` calls `CollapseBridge_EW_High` for this branch | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Trigger event `0x1F` performs bridge mutation or recursive damage. | `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`; fallout report | contradicted | high | conditional on authored tags | DOC_PATCH_READY |
| Current Rust already uses bounded hut collapse. | `src/sim/world/bridge_orchestrator.rs` scan: `run_hut_collapse_bounded`, `MAX_HUT_SWEEP_STEPS = 4` | confirmed | high | n/a | IMPLEMENTATION_REVIEW_SAFE |
| Current Rust has stale full-span wording/tests. | `world_orders_bridge_repair_tests.rs` assertion text; `bridge_orchestrator.rs` doc comment line says "full-span flood" | confirmed | high | n/a | PATCH_READY |
| Current Rust physical-axis mapping is certainly correct. | Rust scan conflicts with binary axis semantics around `0xCD` branch; no long-span regression found | disputed | medium | n/a | NEEDS_FOCUSED_FIX_OR_TEST |

## Current Model

For C4 on a BridgeRepairHut, gamemd does not damage the hut. The timer-expiry branch in `BuildingClass::Update` detects `BridgeRepairHut`, dispatches bridge destruction from the hut coordinate, then clears the C4 marker. Stock YR data makes this path live for CABHUT-style bridge huts.

The hut bridge entry searches for nearby bridge overlay evidence, chooses low or high, and calls `DestroyBridgeFromCell_*`. That function canonicalizes the seed cell by probing one or two cells back along the bridge axis, then calls one of four `CollapseBridge_*_*` walkers.

The collapse walker is bounded. It measures bridge extent in both axial directions, chooses a start biased by `(back - fwd) / 2` with signed division, picks the direction toward the longer side, and runs a hard maximum of four axial iterations. Each iteration may spawn three `BridgeExplosions` on the perpendicular row/column, calls `DestroyBridge_*` up to three retries, steps one cell, and breaks if the next overlay leaves the bridge family band. The resulting high-bridge footprint is roughly 3 cells wide by 6 cells long when overlap and perpendicular destruction are included, not the entire span of a long bridge.

`DestroyBridge_*` and `DestroyBridgeWalker_*` names are dangerous. The high overlay range `0xCD..0xD5 / 0xDF..0xE2 / 0xE7` is a physical EW bridge range according to `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`, and the live `DestroyBridgeFromCell_High @ 0x005749C0` branch calls `CollapseBridge_EW_High` for it. Rust should separate "walker family / 3-cell write primitive" from "physical span axis used by the bounded sweep."

## Implementation-Safe Facts

- Full-span collapse is wrong for CABHUT/C4 bridge collapse. Long bridges should retain cells outside the bounded footprint after one event.
- The bound is exactly four axial iterations, with up to three `DestroyBridge_*` retries per iteration.
- The walker tail always updates bridge zones and sets the tactical full-redraw/path flag in gamemd.
- CABHUT survives because the bridge-hut branch skips normal building damage, not because `Immune=yes` blocks damage.
- Event `0x1F` is a trigger broadcast path, not a bridge mutation or recursive collapse mechanism.
- Stock YR has `DestroyableBridges=yes`, `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`, `MetallicDebris=...`, `BridgeStrength=1500`, `C4Warhead=Super`, and `IonCannonWarhead=IonCannonWH` in `rulesmd.ini` with RA2 fallback matching the relevant keys.

## Doc-Patch-Ready Facts

- `traces/CABHUT_PER_CELL_DESTRUCTION_CASCADE_TRACE.md` is stale where it says gamemd drives a span-completing/full-span collapse. Newer binary evidence says bounded four-step.
- Any Rust comments or tests saying "entire bridge span" for one CABHUT C4 event should be rewritten to "bounded collapse footprint."
- `BRIDGE_SYSTEM.md` older overview wording should not be used over `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` and `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md` for collapse scope.

## Stale Or Superseded Claims

- **Superseded:** "CollapseBridge walks the whole span."  
  **Replacement:** `CollapseBridge_*_*` measures both extents but only walks at most four axial cells.

- **Superseded:** "Rust should collapse every cell in the span from one CABHUT event."  
  **Replacement:** Rust should preserve far cells on long bridges and only mutate the bounded gamemd footprint.

- **Misleading:** "Axis::NS for high overlay `0xCD` means physical north-south bridge."  
  **Replacement:** this label tracks a walker-family naming convention; physical axis must follow the collapse dispatcher / overlay table.

## Cross-Doc Conflicts

- `CABHUT_PER_CELL_DESTRUCTION_CASCADE_TRACE.md` Stage 2 conflicts with `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md` and `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`. The newer reports plus live Ghidra spot-check win for collapse scope.
- `BRIDGE_SYSTEM.md` is useful as a broad reference but has verified amendment docs and older generalized statements. Use targeted Ghidra reports for implementation.

## Needs Re-Investigation

- Campaign/map trigger effects after event `0x1F`: not needed for skirmish bridge-collapse parity, but needed before campaign scripting support.
- Exact parity of current Rust bridge-collapse visual RNG and audio `Report=` behavior: fallout ordering is verified, but current Rust approximates some anim/RNG/audio surfaces.
- Current Rust physical-axis mapping for bounded hut collapse should be tested or fixed with a long EW and long NS bridge fixture. This is implementation validation, not a research blocker.

## Do-Not-Implement Notes

- Do not implement full-span BFS/flood-fill collapse for CABHUT/C4.
- Do not use `Immune=yes` to decide whether bridge-hut C4 collapse fires.
- Do not collapse event `0x1F` trigger broadcast into bridge mutation or zone refresh.
- Do not trust Ghidra function labels alone for bridge axis. Confirm against overlay ranges and `CollapseBridge_*` caller selection.

## Rust Handoff

Current Rust already has the right broad shape in `src/sim/world/bridge_orchestrator.rs`: `run_hut_collapse_bounded`, `MAX_HUT_SWEEP_STEPS = 4`, extent measurement, signed bias, and three attempts per step.

Required next patch should be narrow:

1. Replace stale "full-span" wording in `bridge_orchestrator.rs` and `world_orders_bridge_repair_tests.rs`.
2. Add a long-span regression test proving one CABHUT C4 event does not destroy the far end of a long bridge.
3. Add physical-axis tests for high ranges:
   - `0xCD` family should sweep along physical EW (`CollapseBridge_EW_High`) while still using the correct 3-cell write primitive.
   - `0xD6` family should sweep along physical NS (`CollapseBridge_NS_High`) mirror.
4. If tests expose mismatch, split Rust helpers into `physical_span_axis_for_destroy_overlay` and `walker_family_for_destroy_overlay`.

## Source Ledger

- Live Ghidra spot-check, 2026-05-22: `MapClass::CollapseBridge_NS_High @ 0x00575BA0` confirms `local_2c = 4`, extent measurement, three anim spawns, three retry calls, tail zone/full-redraw flag.
- Live Ghidra spot-check, 2026-05-22: `MapClass::DestroyBridgeFromCell_High @ 0x005749C0` confirms high overlay subrange dispatch to `CollapseBridge_EW_High` vs `CollapseBridge_NS_High`.
- `BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`: primary correction of full-span misconception; chain/no-recursion/trigger findings.
- `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`: C4 marker, hut branch, overlay scan, canonical start, bounded walker.
- `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`: `BlowUpBridge`, deck drop-in, anim/debris, zone/radar/full-redraw, event `0x1F` separation.
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`: overlay tables, axis-label caveat, high bridge state machine.
- `BRIDGE_SYSTEM_VERIFY_DOC_AMENDMENTS.md`: confirms older overview needs corrections in bridge-field/flag prose.
- `traces/CABHUT_C4_TIMER_EXPIRY_BRIDGE_BRANCH_TRACE.md`: Rust vs gamemd timer/hut branch trace.
- `traces/CABHUT_PER_CELL_DESTRUCTION_CASCADE_TRACE.md`: stale trace; useful as historical bug report but superseded for full-span scope.
- `ini/rulesmd.ini`, `ini/rules.ini`: bridge-related defaults.
- Rust scan: `src/sim/world/bridge_orchestrator.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/world/world_orders_bridge_repair_tests.rs`.
