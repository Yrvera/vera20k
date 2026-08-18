# Low Bridge Collapse Tube Zones - Ghidra Research Report

**Address(es):** `0x00484AB0`, `0x00484F20`, `0x0057BAA0`, `0x0057BCF0`, `0x0057C2B0`, `0x00571490`, `0x00570050`, `0x0056C510`, `0x00582D70`, `0x00728280`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** low bridge collapse/destruction behavior that affects TubeClass identity, low bridge zone records, damaged/destroyed low bridge passability, damaged low bridge occupancy/zone overrides, and Rust-facing test deltas.  
**Non-Scope:** high bridge state machine internals, bridge sound/EVA/event fallout, C4/CABHUT entry timing, object drop-in/ground kill ordering, and exact rendering frame selection.  
**Confidence:** High for low bridge predicate, low overlay state transitions, three-cell recalc, destroyed-only zone rebuild, all-active low record zone inclusion, and no direct normal damage-path tube deletion found. Medium for global absence of exotic computed `CellClass+0x116` writes outside the prior direct-write audit.  
**Active in YR:** Yes. These functions are called by standard YR map recalc, bridge damage/destruction, zone rebuild, and pathing graph construction; no TS-only scenario gate was found in this slice.

## 0. Investigation Contract

Target question: When a low bridge is damaged or collapsed, what happens to low bridge overlay state, TubeClass records, bridge-zone connectivity, and passability?

Non-goals: Do not redo the high bridge state machine except as contrast; do not investigate C4 timer entry, audio/EVA, trigger event `0x1F`, or unit fallout ordering.

Evidence needed to mark COMPLETE: decompile and assembly/context evidence for low bridge destroy walker zone timing; decompile evidence for low bridge predicate and tube lookup; prior direct write audit for `CellClass+0x116`; Rust scan of bridge/tube/zone surfaces; implementation handoff with acceptance scenarios and proposed test names.

Stop conditions: stop after verifying low bridge collapse/tube/zone behavior and Rust-facing deltas; defer any broader locomotor, renderer, high bridge, or map-corpus questions.

## 1. Overview

Low bridge collapse in `gamemd.exe` is not "delete the tube and make terrain water." The live model keeps low bridge tube identity separate from active bridge-zone connectivity. Low bridge cells are TubeClass-backed tunnel cells (`CellClass+0x116` valid and `LandType == 10`), while damage/collapse changes overlay/state and invalidates bridge-zone connectivity only at the destroyed-anchor/full-collapse transition.

The player-visible effect is that the first low bridge hit damages the bridge art and affected cell attributes but does not sever the low bridge zone connection. The second hit, or a hit on an already damaged low bridge main cell, transitions the three-cell strip to destroyed anchors, calls low-bridge fallout helpers, invalidates bridge zones, and rebuilds the zone graph if the invalidate call reports a change.

## 2. Key Offsets And Records

| Structure | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `CellClass` | `+0x44` | overlay/type state used by low bridge destroy walkers | `DestroyBridge_Low @ 0x0057BAA0`, walkers `0x0057BCF0`, `0x0057C2B0` | Yes |
| `CellClass` | `+0xEC` | final `LandType`; low bridge predicate requires `10` | `CellClass__IsLowBridgeCell @ 0x00484AB0` | Yes |
| `CellClass` | `+0x116` | signed tube index into `g_TubeArray` | `0x00484AB0`, `0x00484F20`; direct-write audit in `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md` | Yes |
| `CellClass` | `+0x11A` | low bridge surface/tile sub-state used by damage state machine | `ProcessBridgeDamageStateMachine_Low @ 0x00571490` | Yes |
| `CellClass` | `+0x11B` | level/height byte adjusted by low destruction helper paths | `ProcessBridgeDestruction_Low @ 0x00570050` | Yes |
| `CellClass` | `+0x11E` | low bridge damage byte for state-machine branch | `0x00571490` | Yes |
| `CellClass` | `+0x140` | bridge flags; low/high and direction-related dispatch gates | `0x00571490`, `0x00570050` | Yes |
| `TubeClass` | `+0x24/+0x28/+0x2C/+0x30/+0x1C0` | entry, exit, direction, path steps, path length | `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`, `FUN_00582D70 @ 0x00582D70` | Yes |
| `BridgeRecord` | `+0x08` | active/intact byte used by all-active zone rebuild | `UpdateBridgeZonesHelper @ 0x0056C510` | Yes |
| `BridgeRecord` | `+0x0C` | kind: `0 = high`, `1 = low/tube` | prior `ComputeBridgeZones @ 0x0056D6E0`, all-active contrast `0x0056C510` | Yes |

## 3. Core Logic

### 3.1 Low bridge cells are TubeClass-backed tunnel cells

`CellClass__IsLowBridgeCell @ 0x00484AB0` returns true only when:

- `*(i16 *)(cell + 0x116) >= 0`;
- the signed tube index is less than `DAT_008B4148` tube count;
- `*(i32 *)(cell + 0xEC) == 10`.

`CellClass__GetTubeAtCell @ 0x00484F20` only bounds-checks `+0x116` and returns `g_TubeArray[index]`; it does not re-check `LandType == 10`.

Active in YR: Yes. These predicates are consumed by standard map/zone/tube paths, including low bridge zone construction. Evidence: decompile `0x00484AB0`, `0x00484F20`; prior caller evidence in `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`.

### 3.2 Low destroy dispatch accepts only the low overlay family

`DestroyBridge_Low @ 0x0057BAA0` reads `CellClass+0x44` and dispatches only low overlay bands:

- NS low family: `0x4A..0x52`, `0x5C..0x5F`, or `0x64`;
- EW low family: `0x53..0x5B`, `0x60..0x63`, or `0x65`;
- anything outside those low bands returns no dispatch.

The dispatcher anchor-adjusts by checking one and two cells along the relevant axis, then calls `DestroyBridgeWalker_NS_Low @ 0x0057BCF0` or `DestroyBridgeWalker_EW_Low @ 0x0057C2B0`.

Active in YR: Yes. Evidence: decompile `0x0057BAA0`; direct callers are the low collapse/destruction paths documented in existing bridge reports.

### 3.3 Low walker state transitions are two-hit for main low bridge body cells

Verified low overlay transitions:

| Axis/family | First hit | Full destroy hit | Bridgehead hit | Evidence | Active in YR |
|---|---|---|---|---|---|
| NS main | `0x4A..0x4F -> 0x50` on the 3-cell strip | `0x50..0x52 -> 0x64` on the 3-cell strip | `0x5C -> 0x5D`, `0x5E -> 0x5F` | `0x0057BCF0` | Yes |
| EW main | `0x53..0x58 -> 0x59` on the 3-cell strip | `0x59..0x5B -> 0x65` on the 3-cell strip | `0x60 -> 0x61`, `0x62 -> 0x63` | `0x0057C2B0` | Yes |

Tiny details:

- Both low walkers write the same transition overlay to three cells: hit cell plus two perpendicular neighbors.
- First-hit main transitions call the low `ApplyBridgeDestruction_*_Low` helpers but keep the local full-destroy flag clear.
- Destroyed-anchor transitions mark radar dirty for all three cells, call `FindBridgeEndpoints_*_Low`, set the full-destroy flag, set a 3x3 object-notification rect, and return collapse success.
- Already beyond the handled damaged range returns early with no state change.

Active in YR: Yes. Evidence: decompile `0x0057BCF0`, `0x0057C2B0`.

### 3.4 Recalc always happens for three affected cells; bridge-zone rebuild is destroyed-only

Both low walkers call `CellClass__RecalcAttributes @ 0x0047D2B0` on the hit cell and its two strip neighbors after writing overlay state.

Assembly/context spot-check for the NS low walker:

- `0x0057C229: CALL 0x0047D2B0`
- `0x0057C234: CALL 0x0047D2B0`
- `0x0057C241: CALL 0x0047D2B0`

Immediately after those three recalc calls, the walker checks the full-destroy flag:

- `0x0057C268: TEST BL,BL`
- `0x0057C26A: JZ 0x0057C275`
- `0x0057C270: CALL 0x0056C510`

Therefore first-hit damage recalculates the three cells but does not call `UpdateBridgeZonesHelper`; destroyed-anchor/full-collapse calls it.

Active in YR: Yes. Evidence: decompile `0x0057BCF0`, `0x0057C2B0`; assembly/context at `0x0057C229..0x0057C270`.

### 3.5 State-machine low collapse also invalidates zones only after final collapse

`ProcessBridgeDamageStateMachine_Low @ 0x00571490` has low tile-family paths and `Cell+0x11E` state paths. On final collapse it calls `CellClass__SetBridgeDirection_NWSE`, clears the damage byte, writes overlay `-1`, calls `MapClass__UpdateAdjacentBridges`, then invalidates bridge zones and conditionally updates bridge zones.

Assembly/context spot-check:

- `0x005721B3: CALL 0x0047E470`
- `0x005721B8: MOV byte ptr [ESI + 0x11E],0x0`
- `0x005721C2: MOV dword ptr [ESI + 0x44],0xffffffff`
- `0x005721C9: CALL 0x00571050`
- `0x005721D1: CALL 0x0056DAE0`
- `0x005721D6: TEST AL,AL`
- `0x005721D8: JZ 0x005721E1`
- `0x005721DC: CALL 0x0056C510`

Active in YR: Yes. Evidence: decompile `0x00571490`; assembly/context at `0x005721B1..0x005721DC`.

### 3.6 TubeClass records are not normally deleted by low damage/collapse

No direct `CellClass+0x116` clear or TubeClass deletion appears in the decompiled low damage/collapse functions checked here (`0x0057BAA0`, `0x0057BCF0`, `0x0057C2B0`, `0x00571490`, `0x00570050`). Their observable mutation is overlay/state, three-cell recalc, adjacent bridge updates, invalidate/validate bridge zones, and optional full zone rebuild.

The prior direct write audit found `CellClass+0x116` clears in tube save/compaction and tube removal/destructor-side cleanup, not in the normal low bridge damage/repair helper family:

- `0x0072824A`, `0x007282E1`: clear entry cell tube index when tube no longer qualifies during save/compaction;
- `0x00728776`: removal/destructor-side clear if the entry cell still points at the removed tube;
- `0x007280B7`, `0x00728519`: constructor/parser writes tube index.

Active in YR: Conditional. Tube save/compaction and tube removal are live engine paths, but normal low bridge damage/collapse does not use them in the checked evidence. Evidence: decompile of low damage/collapse functions; direct-write audit in `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`.

### 3.7 Full zone rebuild includes active low records

`MapClass__UpdateBridgeZonesHelper @ 0x0056C510` iterates active bridge records and tests only `record+0x08 != 0` before folding endpoint cluster pairs into movement-zone connectivity. It does not test `BridgeRecord+0x0C` in this all-active loop.

`FUN_00582D70 @ 0x00582D70`, used by hierarchical zone building, has a non-high/wood branch that calls `GetTubeAtCell`, reads `Tube+0x2C`, checks adjacent tube cells, walks each adjacent tube path, and inserts three temporary graph connection pairs with flag low byte zero.

Active in YR: Yes. Evidence: decompile `0x0056C510`, `0x00582D70`; corroborated by `LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`.

### 3.8 Occupancy and damaged low bridge zone overrides

`CellClass__RecalcZoneType @ 0x00483C80`, as summarized and spot-checked by prior reports, assigns reduced zone type from overlay/object/building checks and final land speed rows. Unobstructed `LandType == 10` low bridge cells fall to ground-like zone type `0`; object/building occupancy overrides come from the normal ground object/building checks.

The deck-list/high-bridge occupant list is not a separate low bridge zone-type source in `RecalcZoneType`. Damaged low bridge overlay writes can change cell attributes because the walkers call `RecalcAttributes` on the three affected cells, but the bridge-zone graph connection remains until the destroyed-anchor transition.

Active in YR: Yes. Evidence: `RecalcZoneType @ 0x00483C80`; collapse recalc evidence at `0x0057C229..0x0057C241`; `BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`.

## 4. INI Keys And Defaults

| Source | Data | Effect for this slice | Active in YR |
|---|---|---|---|
| `ini/rules.ini`, `ini/rulesmd.ini` | `DestroyableBridges=yes` in `[General]` | Stock bridge destruction is enabled; low bridge collapse paths are reachable. | Yes |
| `ini/rules.ini`, `ini/rulesmd.ini` | low overlay families `LOBRDG*`, `LOBRDGE*`, `LOBRDB*`, `LOBRDGB*` | Visible low bridge overlay families and damaged/destroyed overlay state. | Yes |
| `ini/rules.ini`, `ini/rulesmd.ini` | low bridge overlay entries mostly use `Land=Road` / `NoUseTileLandType=true` | Do not use this as the movement predicate; binary low bridge predicate is tube index plus final `LandType == 10`. | Yes |
| map `[Tubes]` | explicit tube records | Explicit records are supported, but stock retail scan found none in 385 scanned map payloads; stock low bridge behavior relies on auto shell/predicate data. | Conditional on map data |

## 5. Integration Points

Cold/load path:

1. `CellClass__RecalcAttributes @ 0x0047D2B0` computes final land/zone/tube facts.
2. Auto low bridge/tunnel tube shells are created for qualifying `LandType == 10` cells with invalid tube index.
3. `ComputeBridgeZones @ 0x0056D6E0` creates low `BridgeRecordKind=1` records from `IsLowBridgeCell`.
4. `UpdateBridgeZonesHelper @ 0x0056C510` includes every active bridge record, including low records.
5. Hierarchical zone building uses `FUN_00582D70` low/tube branch for non-high records.

Damage/collapse path:

1. Weapon/CABHUT/collapse entry reaches the low destroy/state-machine paths when the input cell is in the low family.
2. First low main hit writes damaged overlays and recalculates three cells.
3. Destroyed-anchor hit writes `0x64` or `0x65`, marks radar dirty, finds bridge endpoints, recalculates the same three-cell strip, invalidates bridge zones, and calls `UpdateBridgeZonesHelper` if needed.
4. Tube records remain separate; normal low damage/collapse invalidates zone connectivity rather than deleting `TubeClass`.

## 6. Current Rust Implementation Status

Rust already contains several matching concepts:

| Surface | Observed status |
|---|---|
| `src/map/tube_facts.rs` | `TubeFact`, `TubeId`, `TubeSource::AutoLowBridge`, and explicit map tube shape exist. |
| `src/map/resolved_terrain.rs` | `tube_index`, `tube_facts`, `tube_at_cell`, `is_low_bridge_tube_cell`, auto low bridge tube construction, and explicit `[Tubes]` seeding exist. |
| `src/sim/bridge_state/mod.rs` | `BridgeRecordKind::{High, Low}`, low tube endpoint computation, endpoint record activity, and zone dirty outcomes exist. |
| `src/sim/bridge_state/walker.rs` | `destroy_bridge_low` and low walkers implement first-hit damage vs second-hit collapse, with tests such as `low_direct_first_hit_damages_without_deactivating_zone_record_then_second_hit_collapses`. |
| `src/sim/pathfinding/zone_build.rs` | `BridgeRecordFilter` distinguishes all-active zone insertion from high-only redirect. |
| `src/sim/pathfinding/core.rs` | direction-8 tube edge exists but intentionally accepts explicit nonzero map tubes only; auto low bridge shell tubes are predicate/zone facts. |
| `src/sim/pathfinding/zone_map_tests.rs` | `stock_low_bridge_auto_shell_zone_grid_uses_low_records_without_explicit_tubes` covers stock auto-shell low record connectivity. |

Current Rust delta for this slot is mostly guardrail and end-to-end collapse coverage:

- Keep the separation between tube identity and active low bridge zone connectivity.
- Preserve first-hit damaged overlays/recalc without low zone-record deactivation.
- Ensure full collapse deactivates low bridge zone records or makes all-active zone rebuild stop connecting the low bridge.
- Add/keep an end-to-end low bridge collapse test through the world/orchestrator path, not just isolated bridge-state walker tests.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Low bridge predicate | verified | `0x00484AB0` | none |
| Tube lookup | verified | `0x00484F20` | none |
| Low overlay destroy dispatch | verified | `0x0057BAA0` | none |
| NS low walker transitions | verified | `0x0057BCF0`, asm/context `0x0057C229..0x0057C270` | none for tube/zone timing |
| EW low walker transitions | verified | `0x0057C2B0`; mirrored structure to NS | none for tube/zone timing |
| Low state-machine final invalidation | verified | `0x00571490`, asm/context `0x005721B1..0x005721DC` | exact high contrast remains non-scope |
| ProcessBridgeDestruction_Low broader scan | touched-not-exhausted | `0x00570050` | visual pavement/ramp details outside this slot |
| Tube direct-write lifecycle | verified by prior audit, spot-checked against low functions | `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`; low function decompiles here | exotic computed writes outside pattern audit not fully impossible |
| All-active zone rebuild includes low records | verified | `0x0056C510`, `0x00582D70` | exact path tiebreaking outside this slot |
| Occupancy zone overrides | verified shape | `0x00483C80`; `BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md` | exact object flag semantics outside this slot |
| Rust low bridge tube model | verified by source scan | `src/map/tube_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs` | no code edits in this slot |
| Rust end-to-end low collapse coverage | touched-not-exhausted | `src/sim/world/world_tests.rs`, `src/sim/bridge_state/walker.rs` | add/verify low-specific world/orchestrator collapse test if absent |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Does low bridge passability use low overlay alone? -> No; `IsLowBridgeCell` requires valid tube index and final `LandType == 10`.` (evidence: `0x00484AB0`)
- `[RESOLVED] OQ2 - Does `GetTubeAtCell` re-check `LandType == 10`? -> No; it only bounds-checks `CellClass+0x116`.` (evidence: `0x00484F20`)
- `[RESOLVED] OQ3 - Which low overlay ranges enter the low destroy walker? -> `0x4A..0x65` split into NS/EW low families; outside values return no dispatch.` (evidence: `0x0057BAA0`)
- `[RESOLVED] OQ4 - Is first-hit low main damage a full collapse? -> No; first hit writes damaged overlays `0x50` or `0x59` and keeps full-destroy flag clear.` (evidence: `0x0057BCF0`, `0x0057C2B0`)
- `[RESOLVED] OQ5 - When does the low walker call `UpdateBridgeZonesHelper`? -> Only when the destroyed-anchor/full-destroy flag is set.` (evidence: assembly/context `0x0057C268..0x0057C270`)
- `[RESOLVED] OQ6 - Are affected low cells recalculated on first-hit damage? -> Yes; three `CellClass__RecalcAttributes` calls occur before the full-destroy zone update test.` (evidence: assembly/context `0x0057C229`, `0x0057C234`, `0x0057C241`)
- `[RESOLVED] OQ7 - Does the low state-machine final collapse also conditionally invalidate/rebuild zones? -> Yes; it calls invalidate, tests AL, and calls `0x0056C510` only if nonzero.` (evidence: `0x005721D1..0x005721DC`)
- `[RESOLVED] OQ8 - Does normal low damage/collapse delete TubeClass records? -> No direct tube index clear/delete was found in checked low damage/collapse functions; tube clears are in save/compaction/removal paths.` (evidence: checked decompiles plus direct-write audit report)
- `[RESOLVED] OQ9 - Are low records included in full zone graph rebuild? -> Yes; all-active loops check record active byte, not record kind.` (evidence: `0x0056C510`, `0x00582D70`)
- `[RESOLVED] OQ10 - Are damaged/occupied low bridge zone overrides from deck occupants? -> No separate deck-list low-zone source was found; normal RecalcZoneType object/building checks apply.` (evidence: `0x00483C80`; follow-up report)
- `[RESOLVED] OQ11 - Does Rust have TubeClass-shaped map facts? -> Yes; `TubeFact`, `TubeId`, auto low shells, explicit map tubes, and low predicate are present.` (evidence: `src/map/tube_facts.rs`, `src/map/resolved_terrain.rs`)
- `[RESOLVED] OQ12 - Does Rust distinguish all-active zone insertion from high-only lookup? -> Yes; `BridgeRecordFilter` and low record zone tests are present.` (evidence: `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map_tests.rs`)
- `[DEFERRED] OQ13 - Exact player-visible path tie order after low collapse zone rebuild.` (category: `needs-runtime-debugger`; reason: this slot verified connectivity invalidation, not equal-cost path choice; next-step-if-pursued: trace a concrete unit path before/after low bridge collapse in gamemd)
- `[DEFERRED] OQ14 - Full visual pavement/ramp art sequence for every low helper branch.` (category: `out-of-scope`; reason: rendering frame parity belongs to bridge rendering/surface reports; next-step-if-pursued: targeted low bridge visual trace)
- `[DEFERRED] OQ15 - Global proof against every possible computed `CellClass+0x116` writer.` (category: `bounded-cost-too-high`; reason: direct pattern audit plus checked low functions are sufficient for the normal low collapse handoff; next-step-if-pursued: full dataflow/write-reference pass on `CellClass+0x116`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Low bridge tube identity is separate from active zone connectivity; normal low collapse does not delete TubeClass records. | `0x00484AB0`, `0x00484F20`; low function decompiles; direct-write audit in lifecycle report | mostly present; guard with tests | `src/map/tube_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs` | Keep `tube_index`/`TubeFact` stable while bridge-state active records decide whether zones connect. | Collapse a low bridge and assert terrain tube facts/tube indices remain present while low endpoint record is inactive or omitted from zone adjacency. | Do not clear `tube_index` or remove auto low shell tubes as the collapse mechanism. Proposed test: `low_bridge_collapse_preserves_tube_facts_but_deactivates_zone_record`. |
| First-hit low bridge damage writes damaged overlays/recalculates affected cells but does not rebuild/deactivate bridge-zone connectivity. | `0x0057BCF0`, `0x0057C2B0`; asm/context `0x0057C229..0x0057C270` | appears present in walker tests; needs end-to-end guard if not already world-covered | `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/pathfinding/zone_build.rs` | First low hit should return absorbed/no zone rebuild; all-active low record should still connect zones. | Hit a healthy low bridge main cell once; assert damaged state, `zones_dirty=false`, and low bridge zone reachability still succeeds. | Do not one-shot healthy low bridge body cells into destroyed connectivity. Proposed test: `low_bridge_first_hit_recalc_keeps_zone_connectivity`. |
| Destroyed-anchor/full collapse writes `0x64`/`0x65`, recalculates the three-cell strip, invalidates bridge zones, and conditionally rebuilds zone graph. | `0x0057BCF0`, `0x0057C2B0`; asm/context `0x0057C268..0x0057C270`; state-machine context `0x005721D1..0x005721DC` | bridge-state tests exist; add low-specific world/orchestrator refresh guard if absent | `src/sim/bridge_state/walker.rs`, `src/sim/world/bridge_orchestrator.rs`, `src/sim/pathfinding/zone_map_tests.rs` | Second hit on damaged low bridge should set destroyed state, `zones_dirty=true`, and rebuilt zones should no longer connect across the bridge. | Damage a low bridge twice through orchestrator/world path; assert `state_changed=true`, low bridge record inactive/no longer connected, and units cannot path across via low record. | Do not rebuild bridge zones on every damaged overlay write; rebuild on destroyed transition. Proposed test: `low_bridge_second_hit_rebuilds_zones_and_blocks_crossing`. |
| All-active zone graph build includes active low bridge records; high-only lookup remains high-only. | `0x0056C510`, `0x00582D70`; contrast prior `FindBridgeRecord @ 0x0056DA10` | present; preserve filter separation | `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map_tests.rs` | Keep low records in all-active adjacency and out of high-only redirect/lookup. | Low record appears in all-active zone graph but not in redirect lookup. | Do not replace all-active bridge record handling with high-only `FindBridgeRecord` semantics. Proposed test: `low_bridge_record_all_active_not_high_redirect`. |
| Damaged/occupied low bridge zone type is recomputed by normal cell attributes; deck occupants are not a separate low-zone override. | `0x00483C80`; three recalc calls at `0x0057C229..0x0057C241` | partially present; needs focused damaged/occupied low bridge guard if future occupancy code changes | `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, occupancy/pathgrid builders | Recompute cell zone/class from terrain/overlay/normal objects, not from high-deck occupant list. | Place a ground blocker/occupant on low bridge and assert blocking is handled by occupancy/can-enter, while deck-list occupants do not rewrite low bridge zone class. | Do not use high bridge deck occupant list to classify low bridge/tunnel zone type. Proposed test: `low_bridge_zone_type_ignores_deck_occupants`. |

### Negative Facts / Do Not Do

- Do not model low bridge collapse by deleting `TubeClass` records or clearing `CellClass+0x116`. Evidence: checked low damage/collapse decompiles plus direct-write audit showing clears in save/compaction/removal, not normal collapse.
- Do not one-shot a healthy low main bridge tile to destroyed connectivity. Evidence: first hit writes `0x50` or `0x59`; destroyed anchors are `0x64`/`0x65` on the damaged transition (`0x0057BCF0`, `0x0057C2B0`).
- Do not call full bridge-zone rebuild on every low damage overlay write. Evidence: `TEST BL,BL` then conditional `CALL 0x0056C510` at `0x0057C268..0x0057C270`.
- Do not filter low records out of all-active zone graph construction. Evidence: `UpdateBridgeZonesHelper @ 0x0056C510` uses active byte only; `FUN_00582D70 @ 0x00582D70` has an explicit non-high/tube branch.
- Do not treat low overlay `Land=Road` as the movement truth. Evidence: `IsLowBridgeCell @ 0x00484AB0` requires valid tube index and `LandType == 10`.

### Remaining Uncertainty

- Full visual/ramp frame parity for every low helper branch remains outside this slot.
- A runtime debugger trace would still be useful for equal-cost path tie behavior after low bridge collapse.
- A total proof against every exotic computed `CellClass+0x116` write was not attempted; the direct pattern audit and checked low functions are strong enough for the normal collapse handoff.

### Stale Docs / Follow-up Docs

- No direct stale-doc replacement is required for the named low bridge TubeClass and zone reports; they already contain the critical separation between tube identity and active zone connectivity.
- Suggested guard wording for any implementation note that still says "destroyed low bridge deletes tubes": replace with `Destroyed low bridge transitions invalidate/deactivate low bridge zone connectivity; normal low damage/collapse paths do not delete TubeClass records or clear per-cell tube indices in the checked binary evidence.`

## Sources

- Ghidra decompiled/rechecked: `CellClass__IsLowBridgeCell @ 0x00484AB0`
- Ghidra decompiled/rechecked: `CellClass__GetTubeAtCell @ 0x00484F20`
- Ghidra decompiled/rechecked: `DestroyBridge_Low @ 0x0057BAA0`
- Ghidra decompiled/rechecked: `MapClass__DestroyBridgeWalker_NS_Low @ 0x0057BCF0`
- Ghidra assembly/context: `0x0057C229`, `0x0057C234`, `0x0057C241`, `0x0057C268..0x0057C270`
- Ghidra decompiled/rechecked: `MapClass__DestroyBridgeWalker_EW_Low @ 0x0057C2B0`
- Ghidra decompiled/rechecked: `ProcessBridgeDamageStateMachine_Low @ 0x00571490`
- Ghidra assembly/context: `0x005721B1..0x005721DC`
- Ghidra decompiled/touched: `ProcessBridgeDestruction_Low @ 0x00570050`
- Ghidra decompiled/rechecked: `MapClass__UpdateBridgeZonesHelper @ 0x0056C510`
- Ghidra decompiled/rechecked: `FUN_00582D70 @ 0x00582D70`
- Prior docs consulted: `LOW_BRIDGE_TUBECLASS_GHIDRA_REPORT.md`, `LOW_BRIDGE_TUBECLASS_PRODUCERS_AND_LIFECYCLE_GHIDRA_REPORT.md`, `LOW_BRIDGE_ZONE_PRECHECK_LANDTYPE10_CONNECTIVITY_GHIDRA_REPORT.md`, `BRIDGE_REMAINING_GAPS_FOLLOWUP_GHIDRA_REPORT.md`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md`, `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- Rust surfaces scanned: `src/map/tube_facts.rs`, `src/map/tubes.rs`, `src/map/resolved_terrain.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/bridge_state/walker.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_map_tests.rs`, `src/sim/pathfinding/core.rs`, `src/sim/world/world_tests.rs`
