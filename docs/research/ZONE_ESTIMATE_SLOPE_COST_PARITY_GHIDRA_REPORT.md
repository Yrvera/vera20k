# Zone_Estimate_Slope_Cost Parity -- Ghidra Research Report

**Address(es):** `0x00585F40` (`Zone_Estimate_Slope_Cost`), caller `0x0042C290` (`Zone_precheck`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact contribution of `Zone_Estimate_Slope_Cost` as consumed by `Zone_precheck`: call contract, inputs, output/rounding, mover gate, YR activity, and whether flat synthetic Rust precheck tests may defer it.  
**Non-Scope:** cell-level A* slope movement, full path smoothing, full construction/writer lifecycle of the `FootClass+0x21C` slope context, and runtime incidence on stock maps.  
**Confidence:** High for caller/helper consumption and Rust-facing deferral boundary; Medium for writer-side slope-context semantics because that lifecycle remains out of scope.  
**Active in YR:** Yes. The helper is reached from standard YR foot pathfinding through `FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck` when the mover's pathfinder coefficient is above the binary threshold.

## 0. Working Notes Required By Swarm Slot

Target question: What exact slope-cost contribution does `Zone_Estimate_Slope_Cost` add to `Zone_precheck`, and can the first Rust zone-precheck patch defer it for synthetic flat tests?

Non-goals: Do not re-investigate cell-level A* slope movement, smoothing/reroute beyond call-contract comparison, full slope-context construction, bridge zone edge flags, or zone hierarchy writer order.

Evidence needed to mark COMPLETE: decompile plus assembly-context evidence for the helper body, `Zone_precheck` gate/scale/add path, `FootClass__Get_Slope_Speed_Factor`, live YR caller chain, and current Rust absence of zone-level slope estimation.

Stop conditions: Stop after the read-side helper contract and direct `Zone_precheck` consumption are verified; defer writer lifecycle or runtime incidence questions rather than expanding scope.

## 1. Overview

`Zone_Estimate_Slope_Cost` returns an integer estimate for one hierarchical zone edge. `Zone_precheck` calls it only if the object argument is non-null and `FootClass__Get_Slope_Speed_Factor(object) > 0x007E3810` (`~1e-5` from prior constant dump). The result is multiplied by that mover-specific factor, converted through `Math__ftol`, and added into the candidate edge cost alongside target zone-type base cost and optional edge-flag `0.001`.

Active in YR: Yes. Evidence: `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900`; `AStar_pathfind_search` calls `Zone_precheck` at `0x0042CB46..0x0042CB58` and retry `0x0042CCA1..0x0042CCB8`; `Zone_precheck` calls the helper at `0x0042C580`.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Type / shape | Purpose | Active in YR |
|---|---:|---|---|---|
| Foot object passed to `Zone_precheck` | `+0x21C` | pointer | Slope-cost context passed as helper `param_1`. | Yes; read at `0x0042C2BF`. |
| Foot object | `+0x530` | double | Mover pathfinder cost coefficient returned by `FootClass__Get_Slope_Speed_Factor` unless exempt. | Yes; read at `0x004DC77E`. |
| Foot object | `+0x5D4 -> +0x24 -> +0xF2` | linked object/type flag | Exemption path returns constant `1.0` instead of `+0x530`. | Conditional; active only when the linked object/type flag is set. |
| TechnoTypeClass | `+0x2F0` | double | Source copied to `Foot+0x530` in `FootClass__Unlimbo`. | Yes; `0x004D72EA..0x004D72F4`. |
| Slope context | `+0x57E4` | `i32[]` | Level-1 direct lookup by neighbor representative index. | Yes; read at `0x005862A2`. |
| Slope context | `+0x59F0` | `i32[]`, 130-wide grid | Level-2 coarse sample grid. | Yes; read throughout `0x0058611B..0x0058627F`. |
| Level-1 zone graph | `DAT_0087F890` | stride `0x24` records | Provides neighbor representative `+0x20` for level 1. | Yes. |
| Level-2 zone graph | `DAT_0087F8A8` | stride `0x24` records | Provides endpoint representatives `+0x20` for level 2. | Yes. |
| Corner offset table | `0x00ABD460` | four `(i16 dx, i16 dy)` pairs | Corner offsets initialized immediately before helper body. | Yes; writes immediately before `0x00585F40`, reads by level 2. |
| Direction corner tables | `0x0082A984`, `0x0082A9C4` | two corner indices per direction | Source and destination two-corner selections for level 2. | Yes; read at `0x0058611B`, `0x005861D5`. |

## 3. Core Logic

### 3.1 Caller Gate And Scaling

`Zone_precheck` first checks the object pointer. If null, it clears slope context and disables slope contribution. If non-null, it calls `FootClass__Get_Slope_Speed_Factor`, reads `object+0x21C`, and enables the helper only if the returned floating value is strictly greater than the threshold at `0x007E3810`.

For each candidate edge while enabled, `Zone_precheck` calls:

```text
Zone_Estimate_Slope_Cost(object+0x21C, level, current_zone, neighbor_zone)
slope_i = Math__ftol(helper_result * slope_factor)
candidate_cost = current_cost + ZoneBaseCost[target_zone_type] + slope_i + optional_0_001
```

Evidence: decompile `0x0042C290`; assembly-context `0x0042C2BA` shows `CALL 0x004DC760`, `MOV EAX,[ESI+0x21C]`, `FCOMP [0x007E3810]`; assembly-context `0x0042C56B` shows pushes into `CALL 0x00585F40`, then `FILD`, `FMUL [ESP+0x60]`, `CALL 0x007C5F00`. Active in YR: Yes.

### 3.2 `Math__ftol` Rounding Contract

The conversion happens after multiplying the integer helper result by the slope factor. `Math__ftol @ 0x007C5F00` stores ST0 through `FISTP qword ptr [EAX]`; prior constant evidence for its control word is `0x0E7F`, so this path is truncation toward zero. For non-negative slope costs, that is equivalent to floor.

Evidence: assembly-context `0x007C5F00` shows `FNSTCW`, optional `FLDCW [0x00822D80]`, `FISTP qword ptr [EAX]`; prior reports `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md` and `SKIRMISH_SPEED_AND_PARTICLE_NORMALIZED_GHIDRA_REPORT.md` identify `0x00822D80 = 0x0E7F`. Active in YR: Yes.

### 3.3 Helper Level Dispatch

The helper dispatches only on the `level` argument:

| Level | Behavior | Evidence | Active in YR |
|---:|---|---|---|
| `0` | return `0`; no slope context read. | `0x00585F47..0x00585F4B`, default return `0x005862AC..0x005862B2` | Yes; `Zone_precheck` runs level 0. |
| `1` | ignore current zone; read neighbor representative from level-1 graph, then return `*(i32 *)(ctx + 0x57E4 + rep*4)`. | `0x0058628A..0x005862A9` | Yes. |
| `2` | convert current and neighbor representatives to adjusted 4-cell coarse coordinates, sample the 130-wide `ctx+0x59F0` grid, and return an arithmetic half-sum. | `0x00585F63..0x00586287` | Yes. |
| other | return `0`. | dispatch at `0x00585F47..0x00585F59`, return `0x005862AC` | Defensive; caller uses only `2,1,0`. |

### 3.4 Level 1 Formula

Level 1 is asymmetric: the current zone argument is ignored. The cost is entirely a property of the neighbor zone representative in the level-1 graph.

```text
rep = Level1Graph[neighbor_zone].representative_plus_0x20
return slope_context[0x57E4/4 + rep]
```

Evidence: decompile `0x00585F40`; assembly-context `0x0058628A` shows `MOV EAX,[ESP+0x20]`, `LEA EDX,[EAX+EAX*8]`, `MOV EAX,[0x0087F890]`, `MOV ECX,[EAX+EDX*4+0x20]`, then `MOV EAX,[EDX+ECX*4+0x57E4]`. Active in YR: Yes.

### 3.5 Level 2 Formula

Level 2 reads both endpoint representatives from `DAT_0087F8A8 + zone*0x24 + 0x20`, converts each representative into a 130-wide coarse grid coordinate, subtracts one cell from odd 4-cell blocks, and compares the adjusted endpoints.

If adjusted source and destination match, it returns:

```text
(grid[src_rep_coarse] + grid[dst_rep_coarse]) >> 1
```

If they differ, it determines one of eight directions from source to destination, samples two source-side corners using `0x0082A984`, samples two destination-side corners using `0x0082A9C4`, takes the min of the two samples at each endpoint, and returns:

```text
(min(source_corner_a, source_corner_b) + min(dest_corner_a, dest_corner_b)) >> 1
```

The final shift is arithmetic (`SAR EAX,1`). Ordinary slope samples are expected non-negative, but the helper itself does not clamp.

Evidence: decompile `0x00585F40`; assembly context at `0x00585F40` confirms dispatch, `0x00586285` confirms `SAR EAX,1`, and direct decompile shows table reads from `0x0082A984`, `0x0082A9C4`, and grid reads through `ctx+0x59F0`. Active in YR: Yes.

### 3.6 Mover Factor Source And YR Defaults

`FootClass__Get_Slope_Speed_Factor @ 0x004DC760` is a label misnomer: it returns either `1.0` through the linked-object exemption or the double at `Foot+0x530`. The `Foot+0x530` field is copied in `FootClass__Unlimbo` from `TechnoTypeClass+0x2F0`, the `ThreatAvoidanceCoefficient` INI field per existing `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`.

Evidence: decompile and assembly-context `0x004DC760..0x004DC784`; decompile and assembly-context `0x004D72E0..0x004D72F4` showing `FLD [EAX+0x2F0]` then `FSTP [ESI+0x530]`. Active in YR: Yes for every Foot object; the linked-object exemption is conditional and not normal stock-YR ground-unit behavior.

## 4. INI Keys

| Key / data | Effect in this slice | Active in YR |
|---|---|---|
| `ThreatAvoidanceCoefficient` | Parsed onto `TechnoTypeClass+0x2F0`, copied to `Foot+0x530`, gates/scales zone slope cost. Stock harvester variants set `1` or `.65`; default constructor value can be `0` for units not overriding it. | Yes. Evidence: prior `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`; writer copy `0x004D72EA..0x004D72F4`. |
| `MovementZone=` | Supplies the passability row to `Zone_precheck`; does not directly affect slope helper formula. | Yes. Evidence: `Zone_precheck` matrix check in decompile `0x0042C290`. |
| `SlopeClimb` / `SlopeDescend` and related runtime cliff multipliers | Not read by this helper or its `Zone_precheck` call contract. They belong to movement execution / locomotor speed docs, not this zone-precheck cost input. | Active elsewhere, not in this slice. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Normal path request | `FootClass__Run_AStar` calls `AStar_pathfind_search`. | `0x004CBBA0` decompile. | Yes. |
| Initial hierarchy precheck | `AStar_pathfind_search` calls `Zone_precheck` on the live hierarchy path. | `0x0042CB46..0x0042CB58`. | Yes. |
| Retry precheck | failed hierarchical A* can rerun `Zone_precheck`. | `0x0042CCA1..0x0042CCB8`. | Yes. |
| Candidate edge expansion | `Zone_precheck` adds the slope contribution before passability/exclusion acceptance of the candidate. | `0x0042C56B..0x0042C5D2`. | Yes. |
| Flat/level-0 stage | Level 0 never adds slope because helper returns zero. | `0x00585F47..0x00585F4B`. | Yes. |

## 6. Current Rust Implementation Status

| Rust surface | Status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Missing exact slope contribution; current corridor Dijkstra still uses centroid/Manhattan distance and has no mover coefficient or slope-context input. |
| `src/sim/pathfinding/zone_map.rs` / `zone_build.rs` | Current Rust does not model the three binary hierarchy levels with representative `+0x20` indices, parent gates, edge flag byte, or the slope context arrays needed for exact helper parity. |
| `src/sim/pathfinding/terrain_speed.rs` | Has runtime per-cell slope speed modifiers (`SlopeClimb`/`SlopeDescend`) for movement execution; this is not the `Zone_precheck` slope-cost pipeline. |
| `src/rules/ruleset.rs` and `src/rules/object_type.rs` | Current Rust parses several movement fields; this slot did not verify a Rust `ThreatAvoidanceCoefficient` field or per-instance pathfinder coefficient surface. |

No Rust files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_Estimate_Slope_Cost @ 0x00585F40` read-side formula | verified | decompile `0x00585F40`; assembly-context `0x00585F40`, `0x0058628A` | none for caller-consumed formula |
| `Zone_precheck` slope gate and scaling | verified | decompile `0x0042C290`; assembly-context `0x0042C2BA`, `0x0042C56B` | none |
| `Math__ftol` conversion site | verified | `0x0042C589..0x0042C591`; `0x007C5F00` assembly context | no new constant dump this slot |
| Mover coefficient reader | verified | decompile/assembly-context `0x004DC760` | linked-object `+0xF2` semantic name remains outside scope |
| `Foot+0x530` source copy | verified | decompile/assembly-context `0x004D72E0..0x004D72F4`; prior `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md` | no full INI parser re-audit |
| Active YR path | verified | `0x004CBBA0`, `0x0042C900`, `0x0042C290` | runtime frequency by unit/map not measured |
| Rust zone-level slope status | verified for scan | `zone_search.rs`, `zone_map.rs`, `terrain_speed.rs` reads | no code changes/tests |
| Full `Foot+0x21C` slope-context writer lifecycle | deferred | read-side formula sufficient for this slot | follow-up needed before exact implementation |

## 8. Open Questions -- Final State Of The Investigation Log

- `[RESOLVED] OQ-1 -- What exact slice is claimed? -> Helper read formula and direct Zone_precheck contribution, not slope-context construction or A* movement.` (evidence: slot scope; report section 0; Active in YR: Yes)
- `[RESOLVED] OQ-2 -- Is the helper live in standard YR? -> Yes when a normal path request reaches Zone_precheck with a mover coefficient above threshold.` (evidence: `0x004CBBA0`, `0x0042C900`, `0x0042C290`; Active in YR: Yes)
- `[RESOLVED] OQ-3 -- What enables the helper call? -> non-null object plus `FootClass__Get_Slope_Speed_Factor > 0x007E3810`.` (evidence: `0x0042C2BA..0x0042C2FB`; Active in YR: Yes)
- `[RESOLVED] OQ-4 -- What is the output type and rounding? -> helper returns `int`; caller multiplies by double factor and `Math__ftol` truncates before adding as integer-derived cost.` (evidence: `0x0042C589..0x0042C591`, `0x007C5F00`; Active in YR: Yes)
- `[RESOLVED] OQ-5 -- Does level 0 contribute slope? -> No, helper returns 0.` (evidence: `0x00585F47..0x00585F4B`; Active in YR: Yes)
- `[RESOLVED] OQ-6 -- Does level 1 depend on current zone? -> No, it reads only neighbor representative and `ctx+0x57E4`.` (evidence: `0x0058628A..0x005862A9`; Active in YR: Yes)
- `[RESOLVED] OQ-7 -- What is level 2's formula? -> direction-specific two-corner min at each endpoint from the 130-wide grid, then arithmetic half-sum.` (evidence: `0x00585F63..0x00586287`; Active in YR: Yes)
- `[RESOLVED] OQ-8 -- Is slope factor tied to `SlopeClimb`/`SlopeDescend`? -> No evidence in this call contract; the factor is `Foot+0x530`/`ThreatAvoidanceCoefficient`, while cliff speed multipliers are separate movement execution data.` (evidence: `0x004DC760`, `0x004D72E0..0x004D72F4`; Active in YR: Yes for this path)
- `[RESOLVED] OQ-9 -- Can flat synthetic Zone_precheck tests defer slope? -> Yes if they exercise level-0 or use mover coefficient `<= 1e-5`/no mover; no helper call or no contribution is binary-correct there.` (evidence: `0x0042C2BA..0x0042C2FB`, `0x00585F47`; Active in YR: Yes)
- `[RESOLVED] OQ-10 -- What current Rust gap matters? -> Rust has runtime terrain slope speed, but no binary zone-level slope estimate in `zone_search.rs`.` (evidence: source scan; Active in YR: N/A for Rust)
- `[DEFERRED] OQ-11 -- Who constructs every `Foot+0x21C + 0x57E4/+0x59F0` value?` (category: out-of-scope; reason: exact writer lifecycle is not needed to decide first flat-test deferral; next-step-if-pursued: trace slope-context constructor/fill functions)
- `[DEFERRED] OQ-12 -- Which stock bridge-collapse routes are changed by slope contribution?` (category: needs-runtime-debugger; reason: requires live zone graph plus route capture on sloped maps; next-step-if-pursued: instrument `Zone_precheck` chosen chain and helper return values)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Zone_precheck` slope contribution is gated by non-null mover and `Foot+0x530`/exemption factor greater than `~1e-5`, then adds `ftol(helper_result * factor)`. | `0x0042C2BA..0x0042C2FB`, `0x0042C56B..0x0042C591`, `0x004DC760` | missing in `find_zone_corridor`; no mover coefficient or helper input. | `src/sim/pathfinding/zone_search.rs`; future mover/pathfinder coefficient surface. | Exact precheck implementation should add slope only under the binary gate; flat/no-mover tests may keep slope zero. | `zone_precheck_applies_slope_cost_only_above_factor_threshold`. | Do not add unconditional slope penalties or reuse runtime `SlopeClimb`/`SlopeDescend` here. |
| Level 0 always returns zero, and level 1 uses only the neighbor representative to index `ctx+0x57E4`. | `0x00585F47..0x00585F4B`, `0x0058628A..0x005862A9` | Rust lacks level-specific zone records and representative indices. | `src/sim/pathfinding/zone_map.rs`, `src/sim/pathfinding/zone_build.rs`, `src/sim/pathfinding/zone_search.rs`. | Three-level precheck should be able to defer slope in level-0 synthetic tests but must preserve level-1 neighbor-only asymmetry when slope parity is implemented. | `zone_slope_level1_uses_neighbor_representative_not_source_zone`. | Do not replace level-1 lookup with centroid distance or source/destination averaging. |
| Level 2 uses endpoint representatives, a 130-wide slope grid, direction-specific two-corner samples, min at each endpoint, and arithmetic half-sum before caller scaling. | `0x00585F63..0x00586287`, table reads `0x0082A984`, `0x0082A9C4` | no 130-wide zone slope grid or corner-table sampling in Rust. | future zone hierarchy/slope-cost data surface; `zone_search.rs`. | Sloped synthetic graph with equal base zone costs should choose the route whose binary sampled/min-averaged slope contribution is lower. | `zone_slope_level2_uses_directional_corner_min_average`. | Do not use all-four-corner average, raw cell height delta, or zone center height as a substitute. |

### Negative Facts / Do Not Do

- Do not block the first Zone_precheck parity patch on slope if its tests are flat, level-0, no-mover, or explicitly use factor `<= 1e-5`; binary contribution is zero in those cases. Active in YR: Yes; evidence `0x0042C2BA..0x0042C2FB`, `0x00585F47`.
- Do not use `SlopeClimb`/`SlopeDescend` or current Rust `terrain_speed.rs` runtime speed logic as the zone-precheck slope contribution. Active in YR: Yes for separate movement execution, but not this helper; evidence `0x004DC760`, `0x004D72E0..0x004D72F4`.
- Do not treat `FootClass__Get_Slope_Speed_Factor` as a per-cell SlopeIndex lookup. Active in YR: Yes; evidence prior `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md` plus `0x004DC760`.
- Do not apply slope at level 0. Active in YR: Yes; evidence `0x00585F47..0x00585F4B`.
- Do not use current zone in the level-1 helper formula. Active in YR: Yes; evidence `0x0058628A..0x005862A9`.

### Remaining Uncertainty

- Full construction/writer lifecycle of the `Foot+0x21C` slope context remains deferred.
- Runtime incidence of slope changing low-bridge post-collapse detours on stock maps remains unmeasured.
- Exact semantic name of the linked-object `+0xF2` exemption flag remains outside this slot.

### Stale Docs / Follow-up Docs

- `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace: "`FootClass.+0x21C` is passed to `Zone_Estimate_Slope_Cost` — likely the locomotor speed type or a slope-modifier index. Needs follow-up."
  - With: "`FootClass+0x21C` is the slope-cost context pointer consumed by `Zone_Estimate_Slope_Cost`; level 1 reads `ctx+0x57E4` by neighbor representative, level 2 reads the 130-wide `ctx+0x59F0` grid by direction-specific corner samples. The mover scalar/gate is separate: `Foot+0x530`, copied from `TechnoTypeClass+0x2F0` (`ThreatAvoidanceCoefficient`)."
- `docs/research/BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`
  - Replace: "Slope cost estimation in precheck | partial | `Zone_Estimate_Slope_Cost` not separately ported; Rust uses inline slope multipliers."
  - With: "Slope cost estimation in precheck | verified-missing in Rust | `Zone_precheck` adds `ftol(Zone_Estimate_Slope_Cost(ctx, level, cur, next) * Foot+0x530_or_exempt_1_0)` only when the factor is `> ~1e-5`; Rust has runtime terrain slope speed but no zone-level slope estimate."
- `docs/research/ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
  - Replace: "Full `Zone_Estimate_Slope_Cost` internals were not re-documented here; only the call/added-cost contract was verified."
  - With: "Full helper internals are documented in `ZONE_ESTIMATE_SLOPE_COST_PARITY_GHIDRA_REPORT.md`: level 0 returns zero; level 1 uses the neighbor representative and `ctx+0x57E4`; level 2 uses level-2 representatives, a 130-wide `ctx+0x59F0` grid, direction-specific two-corner mins, and arithmetic half-sum before caller `ftol(result * factor)`."

## Sources

- Ghidra decompiled/rechecked: `0x00585F40`, `0x0042C290`, `0x004DC760`, `0x0042C900`, `0x004CBBA0`, `0x004D7170`, `0x007C5F00`.
- Ghidra assembly contexts: `0x0042C2BA`, `0x0042C56B`, `0x00585F40`, `0x0058628A`, `0x004DC760`, `0x004D72E0`, `0x007C5F00`.
- Prior reports referenced: `ZONE_ESTIMATE_SLOPE_COST_GHIDRA_REPORT.md`, `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`, `CLIFF_RAMP_TRAVERSAL_GHIDRA_REPORT.md`, `ADD_TIBERIUM_CREDITS_PURIFIER_VIRTUAL_PURIFIERS_GHIDRA_REPORT.md`, `SKIRMISH_SPEED_AND_PARTICLE_NORMALIZED_GHIDRA_REPORT.md`.
- Rust files scanned: `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/terrain_speed.rs`, `src/rules/ruleset.rs`, `src/rules/object_type.rs`.

