# Bridge Deep Slot 2 - CABHUT C4 Collapse Walker Trace

Scope: C4 timer expires on a stock YR CABHUT next to a standard high or low bridge. This trace covers only the hut collapse walker and per-cell destruction pipeline: 5x5 scan order, low/high family choice, seed coordinate, bounded 4-step walker, axis/family mapping, pre-destroy BridgeExplosions, per-cell BlowUpBridge ordering, and RNG draw order. Engineer repair is out of scope.

Hard constraints honored: Ghidra was used read-only only; no Rust/INI files were edited. This is the only file written by this slot.

## Summary

Rust is not fully on gamemd parity for this concrete CABHUT C4 collapse path.

The broad gameplay gate is right: C4 timer expiry on a `BridgeRepairHut=yes` CABHUT routes to bridge collapse and leaves the hut alive. Rust also has the verified bounded 4-step shape instead of the old full-span flood-fill.

The hard mismatches are inside the collapse walker and `BlowUpBridge` fallout:

- Rust runs the bounded hut walker from the first 5x5 overlay hit. gamemd first calls `DestroyBridgeFromCell_Low/High`, which may shift the seed by +1, 0, or -1 along the perpendicular/body axis before calling `CollapseBridge_*`. This can shift the collapsed footprint by one cell.
- gamemd's `CollapseBridge_*` walker spawns three `BridgeExplosions` anims before each per-step `DestroyBridge_*` retry block. Rust does not implement this pre-destroy walker animation stage.
- Rust runs deck DropIn and debris over the aggregate `destroyed_set`; gamemd's `BlowUpBridge` fallout is per actual `BlowUpBridge` cell.
- Rust debris RNG still uses small ranges and `BridgeVoxelMax`; gamemd uses normalized `RandomRanged(0, 0x7FFFFFFE)` probability/jitter draws and does not use `BridgeVoxelMax` for standard YR `BlowUpBridge`.
- Rust has no persistent collapsed-cell queue equivalent from `CellClass::BlowUpBridge`.

## Evidence Sources

- Live Ghidra, read-only: `BuildingClass::Update @ 0x0043FB20`.
- Live Ghidra, read-only: `MapClass::DestroyBridge_High_OnHutDeath @ 0x00574000`.
- Live Ghidra, read-only: `MapClass::DestroyBridge_Low_OnHutDeath @ 0x00574C20`.
- Live Ghidra, read-only: `MapClass::DestroyBridgeFromCell_High @ 0x005749C0`.
- Live Ghidra, read-only: `MapClass::DestroyBridgeFromCell_Low @ 0x00574780`.
- Live Ghidra, read-only: `MapClass::CollapseBridge_NS_High @ 0x00575BA0`.
- Live Ghidra, read-only: `CellClass::BlowUpBridge @ 0x0047DD70`.
- Existing reports: `CABHUT_C4_COLLAPSE_ENTRY_GHIDRA_REPORT.md`, `BRIDGE_COLLAPSE_FALLOUT_ORDERING_GHIDRA_REPORT.md`, `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`, `BRIDGE_COLLAPSE_SOUND_SOURCE_GHIDRA_REPORT.md`.
- INI: `ini/rulesmd.ini` has `BridgeExplosions=TWLT026,TWLT036,TWLT050,TWLT070`, `MetallicDebris=...`, `C4Delay=.03`, and `[CABHUT] BridgeRepairHut=yes`, `Immune=yes`.

## Pipeline Diagram

C4 marker already pending -> `BuildingClass::Update` timer reaches `elapsed >= C4Delay` -> `BridgeRepairHut` branch skips building damage -> low/high family pre-scan in hut 5x5 -> `DestroyBridge_Low/High_OnHutDeath` overlay 5x5 scan -> `DestroyBridgeFromCell_Low/High` canonicalizes seed -> `CollapseBridge_*_*` bounded four-step walker -> three pre-destroy `BridgeExplosions` per walker step -> up to three `DestroyBridge_*` calls per step -> per-cell `BlowUpBridge` fallout -> zone/full redraw update.

## Stage Results

### Stage 1 - Timer expiry and hut branch

Input: stock CABHUT has pending C4 and the timer reaches the delay.

gamemd: `BuildingClass::Update @ 0x0043FB20` checks `field_0x6DF`, compares elapsed frames against `field_0x530`, then if `Type+0x16B6 BridgeRepairHut` is set, skips vtable damage and calls low/high bridge hut destruction, then clears `+0x6DF` and `+0x540`.

Rust: `src/sim/world/world_orders.rs:512` uses `tick - plant_start_tick >= rules.c4_delay_ticks`; `src/sim/world/world_orders.rs:734` detects `bridge_repair_hut`, calls `dispatch_bridge_collapse_from_hut`, and returns `killed_building=false`.

Verdict: PASS for this scenario. Numeric branch condition is `elapsed >= delay` in both, and the hut survives.

### Stage 2 - Low/high family choice

gamemd: `BuildingClass::Update @ 0x0043FB20` scans the hut-local 5x5. Low is selected if any scanned cell has a low wood bridge tile index in `[WoodBridgeSet, WoodBridgeSet+0x10)` or low overlay `0x4A..=0x65`; otherwise high is selected.

Rust: `choose_hut_bridge_family` at `src/sim/world/bridge_orchestrator.rs:376` selects low if any scan cell has low destroy overlay or `is_wood_bridge_repair_tile`, else high.

Verdict: PASS for standard high/low overlay-present cases. The low tile-index path is UNCHECKED for literal tile-range equality because Rust uses resolved terrain metadata rather than a direct tile-index comparison.

### Stage 3 - Hut entry 5x5 overlay scan

gamemd: `DestroyBridge_High_OnHutDeath @ 0x00574000` and `DestroyBridge_Low_OnHutDeath @ 0x00574C20` scan offsets `x=-2..=2` outer, `y=-2..=2` inner for the matching overlay family; first overlay match calls `DestroyBridgeFromCell_*` and returns. For hut `(10,10)`, the first six entry-scan cells are `(8,8),(8,9),(8,10),(8,11),(8,12),(9,8)`.

Rust: `hut_destroy_5x5_scan` at `src/sim/world/bridge_orchestrator.rs:228` emits exactly that order, and `find_destroy_overlay_seed` at `src/sim/world/bridge_orchestrator.rs:256` uses first match.

Verdict: PASS for the entry overlay scan order.

### Stage 4 - Overlay family and physical axis mapping

gamemd high family:

- NS subrange: `0xCD..=0xD5`, `0xDF..=0xE2`, `0xE7` -> calls `CollapseBridge_EW_High`.
- EW subrange: `0xD6..=0xDE`, `0xE3..=0xE6`, `0xE8` -> calls `CollapseBridge_NS_High`.

Low family:

- NS subrange: `0x4A..=0x52`, `0x5C..=0x5F`, `0x64` -> calls `CollapseBridge_EW_Low`.
- EW subrange: `0x53..=0x5B`, `0x60..=0x63`, `0x65` -> calls `CollapseBridge_NS_Low`.

Rust: `src/sim/bridge_state/walker.rs:597` through `:618` uses the same subranges; `physical_span_axis_for_destroy_overlay` at `src/sim/world/bridge_orchestrator.rs:639` flips walker axis to physical span axis.

Verdict: PASS for subrange classification and physical axis family.

### Stage 5 - Canonical seed coordinate before `CollapseBridge_*`

gamemd: `DestroyBridgeFromCell_High @ 0x005749C0` and `DestroyBridgeFromCell_Low @ 0x00574780` do not pass the first overlay-hit cell directly to `CollapseBridge_*`. They probe one and two cells behind along the relevant body axis. Concrete high example: first hit overlay `0xCD` at `(x,y)` with `(x,y-1)` off-band calls `CollapseBridge_EW_High` at `(x,y+1)`, not `(x,y)`. Concrete EW-subrange example: first hit overlay `0xD6` at `(x,y)` with `(x-1,y)` off-band calls `CollapseBridge_NS_High` at `(x+1,y)`, not `(x,y)`.

Rust: `find_destroy_overlay_seed` at `src/sim/world/bridge_orchestrator.rs:256` returns the first matching cell directly, and `dispatch_bridge_collapse_from_hut` passes it to `run_hut_collapse_bounded` at `src/sim/world/bridge_orchestrator.rs:205`.

Verdict: FAIL. For standard edge-first 5x5 hits, Rust's bounded walker can start one cell off from gamemd, shifting the collapsed footprint and all pre-destroy/debris coordinates.

### Stage 6 - Bounded four-step walker and retry counts

gamemd: `CollapseBridge_NS_High @ 0x00575BA0` measures backward and forward extents, chooses `step = -1` if forward count is less than backward count else `+1`, computes `start = seed - (back - forward) / 2` using signed integer division, loops exactly 4 axial iterations, and calls `DestroyBridge_*` up to 3 times per step.

Rust: `run_hut_collapse_bounded` at `src/sim/world/bridge_orchestrator.rs:695` does extent measurement, uses the same `step` condition at `:710`, integer bias at `:711`, loops `MAX_HUT_SWEEP_STEPS = 4` at `:719`, and retries `MAX_HUT_ATTEMPTS_PER_STEP = 3` at `:723`.

Verdict: PASS for loop constants and formula shape, but final cell equality is blocked by Stage 5's seed-coordinate failure.

### Stage 7 - Pre-destroy walker `BridgeExplosions`

gamemd: inside each `CollapseBridge_*` iteration, before the `DestroyBridge_*` retry loop, the walker spawns three `BridgeExplosions` anims on perpendicular cells unless the center cell is the terminal destroyed cap. `CollapseBridge_NS_High @ 0x00575BA0` shows three anim iterations, each consuming two normalized jitter draws, `RandomRanged(1,5)` delay, and `RandomRanged(0, BridgeExplosions.Count-1)` anim index before `DestroyBridge_High` is called.

Rust: `run_hut_collapse_bounded` at `src/sim/world/bridge_orchestrator.rs:719` through `:737` goes straight into `call_destroy_per_family`; no pre-destroy anim stage exists. Rust only calls `spawn_bridge_debris` after aggregating outcomes at `src/sim/world/bridge_orchestrator.rs:315`.

Verdict: NOT-IMPLEMENTED. Player sees fewer/misordered explosion visuals and the shared RNG stream advances differently before each cell destruction.

### Stage 8 - Per-cell `BlowUpBridge` fallout scope/order

gamemd: `CellClass::BlowUpBridge @ 0x0047DD70` performs, per cell: ground-list force kill, deck-list `DropIn`, collapsed-cell queue append, then optional debris/explosion block. This is scoped to each actual `BlowUpBridge` call.

Rust: `apply_hut_bridge_execution` at `src/sim/world/bridge_orchestrator.rs:302` kills ground occupants only for `blow_up_cells`, but `drop_in_bridge_deck_entities` runs over all `destroyed_set` at `:312`, and `spawn_bridge_debris` runs over all `destroyed_set` at `:315`.

Verdict: FAIL. Rust can DropIn deck units and spawn `BlowUpBridge` debris on cells that gamemd only destroyed/flagged but did not route through `BlowUpBridge`.

### Stage 9 - `BlowUpBridge` debris RNG order/ranges

gamemd `CellClass::BlowUpBridge @ 0x0047DD70` order per actual BlowUpBridge cell:

1. gate on `BridgeExplosions.ActiveCount > 0`;
2. outer 95 percent gate: `RandomRanged(0, 0x7FFFFFFE)`;
3. two jitter draws: `RandomRanged(0, 0x7FFFFFFE)` twice;
4. metallic 50 percent gate: `RandomRanged(0, 0x7FFFFFFE)`;
5. optional metallic slot: `RandomRanged(0, MetallicDebris.Count - 1)`;
6. bridge explosion delay: `RandomRanged(1,5)`;
7. bridge explosion slot: `RandomRanged(0, BridgeExplosions.Count - 1)`.

Rust: `spawn_bridge_debris` uses `next_range_u32(20)` at `src/sim/world/bridge_orchestrator.rs:1078`, `next_range_u32(0xFFFF)` twice at `:1084-1085`, `next_range_u32(2)` at `:1095`, and then delay/slot at `:1121-1122`.

Verdict: FAIL. The output visuals, chosen anims, and RNG stream are not literally equal.

### Stage 10 - `BridgeVoxelMax` participation

gamemd: standard YR `BlowUpBridge` does not read `BridgeVoxelMax` for this path. The live gate is `BridgeExplosions.ActiveCount > 0`; metallic debris uses `MetallicDebris.ActiveCount` only if the metallic probability gate passes.

Rust: `spawn_bridge_debris` reads `rules.bridge_rules.voxel_max` at `src/sim/world/bridge_orchestrator.rs:1070` and requires `voxel_max > 0` for metallic debris at `:1099`.

Verdict: FAIL. Modded or test data with `BridgeVoxelMax=0` suppresses metallic debris in Rust but not in gamemd's standard YR path.

### Stage 11 - Jittered visual coordinates

gamemd: both walker pre-destroy `BridgeExplosions` and per-cell `BlowUpBridge` explosion/debris use normalized jitter draws to offset visuals inside the cell.

Rust: `spawn_bridge_debris` consumes placeholder jitter draws but places effects at `CELL_CENTER_LEPTON` for both `sub_x` and `sub_y` at `src/sim/world/bridge_orchestrator.rs:1107-1108` and `:1129-1130`.

Verdict: FAIL. Visual positions are centered instead of jittered.

### Stage 12 - Collapsed-cell queue

gamemd: `CellClass::BlowUpBridge @ 0x0047DD70` appends the cell coordinate to a global collapsed-cell queue after occupant fallout and before debris RNG.

Rust: this trace found only local `destroyed_set`/`blow_up_cells` aggregation inside `apply_hut_bridge_execution`; no persistent collapsed-cell queue equivalent was found for the per-cell `BlowUpBridge` pipeline.

Verdict: NOT-IMPLEMENTED for the queue side effect. Player-visible impact depends on downstream consumers, so exact presentation impact remains UNCHECKED.

### Stage 13 - Zone/full redraw/radar invalidation

gamemd: `CollapseBridge_*` tails call `UpdateBridgeZonesHelper()` and set `g_Tactical+0xD7C = 1`. `SetBridgeDirection_*` also marks radar terrain dirty after `BlowUpBridge` for state-0 cells.

Rust: `apply_hut_bridge_execution` calls `update_adjacent_bridges` at `src/sim/world/bridge_orchestrator.rs:316` and `refresh_bridge_zones_if_dirty` at `:318`. App path-grid rebuild is signaled by the returned `bridge_state_changed`.

Verdict: UNCHECKED. The broad zone refresh exists, but this slot did not compute literal equality for tactical full-redraw, radar dirty cells, or per-cell timing.

## Verdict Tally

PASS: 5 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 2

PASS stages: timer/hut branch, overlay-present low/high family choice, hut-entry 5x5 scan, overlay family/axis mapping, bounded loop/retry constants.

FAIL stages: canonical seed coordinate, fallout scope/order, debris RNG ranges, BridgeVoxelMax participation, jittered visual coordinates.

UNCHECKED stages: low tile-index family equivalence; zone/full-redraw/radar exactness.

NOT-IMPLEMENTED stages: pre-destroy walker `BridgeExplosions`; persistent collapsed-cell queue equivalent.

## Top Player-Visible Findings

1. Stage 5 FAIL - Rust can collapse the wrong footprint row/column because it skips `DestroyBridgeFromCell_*` canonical seed adjustment; our `src/sim/world/bridge_orchestrator.rs:202` / `:205`; gamemd evidence `MapClass::DestroyBridgeFromCell_High @ 0x005749C0`, `MapClass::DestroyBridgeFromCell_Low @ 0x00574780`.
2. Stage 7 NOT-IMPLEMENTED - Rust lacks the three pre-destroy `BridgeExplosions` per bounded walker step, so CABHUT collapse visuals and RNG order are wrong; our `src/sim/world/bridge_orchestrator.rs:719`; gamemd evidence `MapClass::CollapseBridge_NS_High @ 0x00575BA0`.
3. Stage 8 FAIL - Rust applies DropIn/debris to aggregate `destroyed_set`, not only actual `BlowUpBridge` cells; our `src/sim/world/bridge_orchestrator.rs:312` / `:315`; gamemd evidence `CellClass::BlowUpBridge @ 0x0047DD70`.
4. Stage 9 FAIL - Rust debris RNG uses small ranges instead of normalized `RandomRanged(0, 0x7FFFFFFE)` gates/jitter; our `src/sim/world/bridge_orchestrator.rs:1078` / `:1084` / `:1095`; gamemd evidence `CellClass::BlowUpBridge @ 0x0047DD70`, `BRIDGE_RNG_CALL_ORDER_CLASSIFICATION_GHIDRA_REPORT.md`.
5. Stage 11 FAIL - Rust centers debris/explosion effects instead of applying gamemd jittered visual coordinates; our `src/sim/world/bridge_orchestrator.rs:1107` / `:1129`; gamemd evidence `CellClass::BlowUpBridge @ 0x0047DD70`, `MapClass::CollapseBridge_NS_High @ 0x00575BA0`.

## Adjacent Findings

- The `BuildingClass::Update` low/high pre-scan and the `DestroyBridge_*_OnHutDeath` overlay-entry scan are not the same consumer. Family choice only needs a boolean low-vs-high result; overlay entry selection needs first-match order. Current Rust reuses one scan list, which is safe for the boolean family decision in standard overlay-present cases but should not be cited as proof of the BuildingClass pre-scan's exact iteration order.
- Collapse audio remains missing through the same mechanism identified by the prior presentation trace: standard YR gets sound from the selected `BridgeExplosions` anim `Report=`/`StartSound`, not from a hardcoded bridge sound.
- True downstream impact of the collapsed-cell queue needs a separate trace. This slot only verified that the queue append exists in gamemd's per-cell `BlowUpBridge` and did not find a persistent Rust equivalent.

## Status

COMPLETE.
