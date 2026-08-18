# Zone_Estimate_Slope_Cost -- Ghidra Research Report

**Address(es):** `0x00585F40` (`Zone_Estimate_Slope_Cost`), caller `0x0042C290` (`Zone_precheck`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact helper formula, inputs, bounds, and call path when used by `Zone_precheck @ 0x0042C290`
**Non-Scope:** full cell A*, full zone adjacency emission order, full writer lifecycle of `FootClass+0x21C`, and runtime frequency measurement
**Confidence:** High for formula/caller behavior; Medium for Rust handoff metadata inventory because no Rust changes or tests were run
**Active in YR:** Yes. The helper is called only from live `Zone_precheck`, which is reached from standard foot pathfinding through `FootClass__Run_AStar -> AStar_pathfind_search -> Zone_precheck`.

## Target Question

What exact formula, inputs, bounds, and call path does `Zone_Estimate_Slope_Cost @ 0x00585F40` use when `Zone_precheck @ 0x0042C290` adds optional slope cost to hierarchical zone edges?

## Non-Goals

- Do not re-document the whole `Zone_precheck` graph search beyond the helper call contract.
- Do not implement Rust changes.
- Do not audit all `FootClass+0x21C` writers or all slope-map construction internals.
- Do not reinterpret `edge+4` flag semantics; prior reports already verified it is only a `0.001` tiebreak input here.

## Evidence Needed To Mark COMPLETE

- Decompile of `Zone_Estimate_Slope_Cost @ 0x00585F40`.
- Assembly evidence for level dispatch, representative index reads, corner table reads, and return arithmetic.
- Caller evidence from `Zone_precheck @ 0x0042C290` showing how the helper result is scaled and added.
- Live path evidence from `FootClass__Run_AStar` / `AStar_pathfind_search`.
- Rust handoff scan for current `zone_search`, `zone_map`, and slope metadata gaps.

## Stop Conditions

This slice stops after the helper's read-side formula and direct caller integration are exhausted. It does not chase the complete construction of the `FootClass+0x21C` slope-cost object unless the helper formula cannot be explained without it. It does not modify code.

## 1. Overview

`Zone_Estimate_Slope_Cost` returns an integer terrain-slope estimate for one hierarchical zone edge. `Zone_precheck` calls it only when a moving object exists and `FootClass__Get_Slope_Speed_Factor(object) > 1e-5`; it then multiplies the returned integer by that slope factor, converts through `Math__ftol`, and adds it to the accumulated float edge cost.

Active in YR: Yes. Evidence: `0x004CBC31 -> 0x0042C900`, `0x0042CB58/0x0042CCB3 -> 0x0042C290`, `0x0042C580 -> 0x00585F40`.

## 2. Class Layout / Key Offsets

| Owner | Offset / address | Type / shape | Purpose | Active in YR |
|---|---:|---|---|---|
| Foot/Object passed to `Zone_precheck` | `+0x21C` | pointer | slope-cost context passed as helper `param_1`. | Yes; read at `0x0042C2BF` before slope calls. |
| Foot/Object passed to `Zone_precheck` | `+0x530` | double | fallback slope speed factor returned by `FootClass__Get_Slope_Speed_Factor`. | Yes; `0x004DC760`. |
| Foot/Object passed to `Zone_precheck` | `+0x5D4 -> +0x24 -> +0xF2` | object/type flag | if nonzero, slope speed factor is forced to `1.0`. | Yes; `0x004DC760`. |
| Slope context | `+0x57E4` | `i32[]` | level-1 direct slope-cost lookup by zone representative cell index. | Yes; `0x0058628A..0x005862A9`. |
| Slope context | `+0x59F0` | `i32[]`, 130-wide indexed grid | level-2 coarse slope-cost grid. | Yes; `0x00586169`, `0x005861BE`, `0x0058622A`, `0x0058626D`. |
| Zone graph level 1 | `DAT_0087F890` | zone records, stride `0x24` | level-1 record source for `record[zone].+0x20`. | Yes; `0x00586294..0x005862A2`. |
| Zone graph level 2 | `DAT_0087F8A8` | zone records, stride `0x24` | level-2 record source for `record[zone].+0x20`. | Yes; `0x00585F63..0x00585F71`. |
| Zone record | `+0x20` | representative cell/index | level-specific representative used by slope estimation. | Yes; read in both helper branches. |
| Local corner offsets | `0x00ABD460` | four `(i16 dx, i16 dy)` pairs | initialized immediately before helper to `(0,0),(4,0),(0,4),(4,4)`. | Yes; writes at `0x00585EF6..0x00585F31`, reads at `0x00586125`, `0x005861E0`, `0x00586231`. |
| Source corner table | `0x0082A984` | `u32[8][2]` | two candidate source corners for each direction. | Yes; read at `0x0058611B`. |
| Destination corner table | `0x0082A9C4` | `u32[8][2]` | two candidate destination corners for each direction. | Yes; reads at `0x005861D9`, `0x00586223`. |
| Slope enable threshold | `0x007E3810` | double `1e-5` | slope helper disabled unless factor is greater than this. | Yes; `0x0042C2E9..0x0042C2FB`. |

## 3. Core Logic

### Caller gate and scaling

`Zone_precheck` does not always call the helper:

1. If its object argument is null, slope factor and slope context are set to zero and no helper call happens.
2. Otherwise it calls `FootClass__Get_Slope_Speed_Factor` with the object in `ECX`, stores `object+0x21C`, and sets the slope-enabled byte only if the factor is strictly greater than `1e-5`.
3. For each candidate zone edge while enabled, it calls `Zone_Estimate_Slope_Cost(slope_context, level, current_zone, neighbor_zone)`.
4. It converts the returned integer to x87 float with `FILD`, multiplies by the stored double slope factor, calls `Math__ftol @ 0x007C5F00`, then adds that integer with `FIADD` into the candidate edge cost.

Evidence: `0x0042C2BA..0x0042C2FB`, `0x0042C56B..0x0042C591`, `0x0042C5BB..0x0042C5D2`. Active in YR: Yes.

### Helper level dispatch

The helper has three material level cases:

- `level == 0`: return `0`.
- `level == 1`: return `slope_context[0x57E4/4 + level1_zone_record[neighbor].representative]`.
- `level == 2`: compute a two-endpoint average from the 130-wide grid at `slope_context+0x59F0`.
- Any other level value: return `0`.

Evidence: `0x00585F40..0x00585F59`, level-1 return `0x0058628A..0x005862A9`, default return `0x005862AC..0x005862B2`. Active in YR: Yes for levels `2,1,0` from `Zone_precheck`; invalid levels are defensive only.

### Level-1 formula

For level `1`, the source/current zone is ignored. The helper reads only the neighbor zone:

```text
rep = Level1Graph[neighbor_zone].representative_at_plus_0x20
return *(i32 *)(slope_context + 0x57E4 + rep * 4)
```

Evidence: assembly `0x0058628A..0x005862A9` computes `neighbor*9`, reads `DAT_0087F890 + neighbor*0x24 + 0x20`, then loads `slope_context + 0x57E4 + rep*4`. Active in YR: Yes.

### Level-2 representative coordinate conversion

For each endpoint zone at level `2`, the helper converts `rep = Level2Graph[zone].+0x20` into coarse cell coordinates using a hardcoded row stride `0x82` (`130`):

```text
x4 = ((rep - 1) % 130) * 4
y4 = (((rep - ((rep - 1) % 130)) / 130) * 4) - 4
adjusted_x = x4 - (((x4 / 4) & 1) != 0 ? 1 : 0)
adjusted_y = y4 - (((y4 / 4) & 1) != 0 ? 1 : 0)
```

The decompiler shows signed modulo/division, but normal representative values are non-negative. The intermediate adjusted values are held in 16-bit registers/locals before corner offsets are added.

Evidence: current-zone conversion begins at `0x00585F63..0x00585F97`; parity adjustment at `0x00585F9E..0x00585FDF`; neighbor-zone conversion repeats at `0x00585FE1..0x00586055`. Active in YR: Yes.

### Level-2 same adjusted block

If source and destination adjusted coordinates match exactly, the helper returns the arithmetic half-sum of the two representative samples:

```text
src_sample = grid130[src_x4 / 4, src_y4 / 4]
dst_sample = grid130[dst_x4 / 4, dst_y4 / 4]
return (src_sample + dst_sample) >> 1
```

The final shift is arithmetic (`SAR EAX,1`), but slope samples are expected non-negative in ordinary use.

Evidence: equality branch in decompile and return sequence `0x00586274..0x00586287`; reads from `slope_context+0x59F0`. Active in YR: Yes.

### Level-2 directional branch

If adjusted coordinates differ, the helper determines one of the standard 8 directions from source to destination:

| Direction code | Condition from source to destination |
|---:|---|
| `0` | same x, destination north / lower y |
| `1` | east and north |
| `2` | east |
| `3` | east and south |
| `4` | same x, destination south / higher y |
| `5` | west and south |
| `6` | west |
| `7` | west and north |

Evidence: branch ladder `0x005860C0..0x00586117`. Active in YR: Yes.

It then samples two source-side corners and two destination-side corners. Corner index `0..3` maps to `(0,0),(4,0),(0,4),(4,4)` from `0x00ABD460`.

Source corner table at `0x0082A984`: dir `0..7` uses `0,1`; `1,1`; `1,3`; `3,3`; `2,3`; `2,2`; `0,2`; `0,0`.

Destination corner table at `0x0082A9C4`: dir `0..7` uses `2,3`; `2,2`; `0,2`; `0,0`; `0,1`; `1,1`; `1,3`; `3,3`.

For each sampled corner:

```text
sample_index = floor_div4(adjusted_x + corner_dx)
             + floor_div4(adjusted_y + corner_dy) * 130
sample = *(i32 *)(slope_context + 0x59F0 + sample_index * 4)
```

Then:

```text
src_min = min(sample(source_corner_a), sample(source_corner_b))
dst_min = min(sample(dest_corner_a), sample(dest_corner_b))
return (src_min + dst_min) >> 1
```

Evidence: source table read and first source sample `0x0058611B..0x00586170`, second source sample and min `0x00586174..0x005861D1`, destination table samples/min `0x005861D5..0x0058627F`, final `SAR EAX,1` at `0x00586285`. Active in YR: Yes.

### Bounds and edge cases

| Case | Binary behavior | Evidence | Active in YR |
|---|---|---|---|
| `level == 0` | returns `0`; no slope data read. | `0x00585F47..0x00585F4B`, `0x005862AC..0x005862B2` | Yes, because `Zone_precheck` runs level 0. |
| `level == 1` | reads only neighbor representative and `slope_context+0x57E4`; no source-zone use. | `0x0058628A..0x005862A9` | Yes. |
| `level == 2` | reads both endpoint representatives and `slope_context+0x59F0`; no explicit zone-id or grid clamp. | `0x00585F63..0x00586287` | Yes. |
| invalid level | returns `0`. | dispatch `DEC/JNZ` at `0x00585F51..0x00585F59`, default return `0x005862AC` | Defensive; not used by verified caller. |
| null slope context | no guard inside helper; caller must avoid or binary would dereference it on level 1/2. | helper reads `[slope_context+...]` directly | Conditional; caller only calls after non-null object and factor gate, but `+0x21C` itself is not checked here. |
| out-of-range zone id | no guard inside helper; zone record indexing is direct. | `zone*0x24 + 0x20` reads | Conditional; graph search supplies valid zone ids. |
| out-of-range representative/grid index | no clamp in helper; it trusts zone record `+0x20` and 130-wide slope arrays. | direct grid loads through `+0x57E4/+0x59F0` | Conditional; build pipeline is expected to bound it. |

## 4. INI Keys

`Zone_Estimate_Slope_Cost` reads no INI key directly.

| Key / data | Binary field / data | Effect in this slice | Active in YR |
|---|---|---|---|
| `MovementZone=` | caller-supplied row to `Zone_precheck`, not read by helper | controls whether candidate zone edge is passable before/after slope cost is considered. | Yes; `AStar_pathfind_search` passes row into `Zone_precheck`. |
| Speed/slope factor data | object runtime fields, not an INI read here | gates and scales helper output. | Yes; `FootClass__Get_Slope_Speed_Factor @ 0x004DC760`. |

## 5. Integration Points

| Point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Normal foot path | `FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900`. | call at `0x004CBC31` | Yes. |
| Initial `Zone_precheck` | `AStar_pathfind_search` calls `Zone_precheck` when hierarchy is enabled and start/destination zones match at the caller's map-zone level. | `0x0042CB46..0x0042CB58` | Yes. |
| Retry `Zone_precheck` | after failed hierarchical cell A*, retry can call `Zone_precheck` again. | `0x0042CCA1..0x0042CCB8` | Yes. |
| Blocked-destination helper | `FUN_0042D170` calls `Zone_precheck`; helper can run if slope gate passes. | `0x0042D222`; caller at `0x004D3C9C` | Yes, but runtime frequency not measured. |
| Slope contribution | helper integer result is scaled by slope factor and inserted into candidate edge cost before passability/visited filters finish. | `0x0042C580..0x0042C5D2` | Yes. |

## 6. Current Rust Implementation Status

| Surface | Status vs binary slice |
|---|---|
| `src/sim/pathfinding/zone_search.rs::find_zone_corridor` | Uses centroid Manhattan distance plus `g+h` ordering; no slope cost input. |
| `src/sim/pathfinding/zone_search.rs::find_path_zoned` | Single-level corridor approximation; binary slope helper is level-aware and only meaningful inside the three-level precheck. |
| `src/sim/pathfinding/zone_map.rs::ZoneInfo` | Stores centroid/cell count, not zone `+0x20` representative index, reduced zone type, edge flag, parent, or per-level hierarchy. |
| `src/sim/pathfinding/zone_build.rs` / `zone_map.rs` | Current graph is per movement zone; binary uses global zone graph plus movement-zone passability row. |
| Slope metadata | No scanned surface carries the `Foot+0x21C` slope-cost arrays or 130-wide representative-grid samples needed for exact helper parity. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Zone_Estimate_Slope_Cost @ 0x00585F40` | verified | decompile + assembly contexts `0x00585F40..0x005862B2` | none for read-side formula |
| level dispatch | verified | `0x00585F47..0x00585F59`, `0x0058628A`, `0x005862AC` | none |
| level-1 formula | verified | `0x0058628A..0x005862A9` | writer-side meaning of `+0x57E4` array deferred |
| level-2 coordinate conversion | verified | `0x00585F63..0x00586055` | exact writer lifecycle of reps out-of-scope |
| level-2 direction and corner tables | verified | `0x005860C0..0x0058627F`; data dumps `0x0082A984`, `0x0082A9C4` | none |
| caller slope gate/scale | verified | `0x0042C2BA..0x0042C2FB`, `0x0042C56B..0x0042C5D2` | none |
| active YR path | verified | `0x004CBC31`, `0x0042CB58`, `0x0042CCB3` | runtime frequency of alternate caller only |
| Rust surface scan | verified for scan | Codegraph and file reads for `zone_search.rs`, `zone_map.rs` | implementation not changed |
| full `Foot+0x21C` construction | deferred | helper/caller evidence sufficient for formula | follow-up if implementing exact slope map |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-1 -- What is the exact scope? -> exhaustive slice for helper formula and direct `Zone_precheck` use.` (evidence: user scope)
- `[RESOLVED] OQ-2 -- Is the helper live in standard YR? -> Yes, through normal foot pathfinding when slope factor is greater than `1e-5`.` (evidence: `0x004CBC31`, `0x0042CB58/0x0042CCB3`, `0x0042C580`)
- `[RESOLVED] OQ-3 -- What enables helper calls? -> non-null object plus `FootClass__Get_Slope_Speed_Factor(object) > 1e-5`.` (evidence: `0x0042C2BA..0x0042C2FB`)
- `[RESOLVED] OQ-4 -- What is the scaling? -> helper integer is multiplied by slope factor and converted through `Math__ftol` before `FIADD` into edge cost.` (evidence: `0x0042C585..0x0042C5D2`)
- `[RESOLVED] OQ-5 -- What does level 0 return? -> `0`.` (evidence: `0x00585F47..0x00585F4B`, `0x005862AC`)
- `[RESOLVED] OQ-6 -- What does level 1 use? -> neighbor zone representative only, indexing `slope_context+0x57E4`.` (evidence: `0x0058628A..0x005862A9`)
- `[RESOLVED] OQ-7 -- What does level 2 use? -> endpoint representatives, parity-adjusted 4-cell corner samples, min at each endpoint, half-sum.` (evidence: `0x00585F63..0x00586287`)
- `[RESOLVED] OQ-8 -- Are there explicit helper bounds checks? -> no; invalid level returns zero but pointers/zone ids/grid indices are trusted.` (evidence: direct dereferences in `0x00585F63..0x005862A9`)
- `[RESOLVED] OQ-9 -- What are corner offsets? -> `(0,0),(4,0),(0,4),(4,4)` initialized at `0x00585EF6..0x00585F31`.` (evidence: assembly context)
- `[RESOLVED] OQ-10 -- What Rust delta follows? -> current Rust lacks level hierarchy, representative slope samples, edge flags, and binary cost formula.` (evidence: Codegraph/file scan)
- `[DEFERRED] OQ-11 -- Who constructs every byte of `Foot+0x21C` slope context?` (category: out-of-scope; reason: not needed to prove helper formula; next-step-if-pursued: trace constructors/writers of `+0x57E4/+0x59F0`)
- `[DEFERRED] OQ-12 -- How often does `FUN_0042D170` slope path execute in ordinary skirmish?` (category: needs-runtime-debugger; reason: static reachability proven but frequency requires instrumentation; next-step-if-pursued: breakpoint `0x0042D222` during blocked-destination commands)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Slope cost in `Zone_precheck` is gated by `slope_factor > 1e-5`, then `ftol(Zone_Estimate_Slope_Cost(...) * slope_factor)` is added to the edge cost. | `0x0042C2BA..0x0042C2FB`, `0x0042C580..0x0042C5D2` | missing: no slope-cost contribution in zone corridor. | `src/sim/pathfinding/zone_search.rs::find_zone_corridor`; future mover slope-factor input. | Add slope only when exact binary gate passes; keep no-slope movers at zero. | Graph with two equal base-cost routes but one high-slope edge should pick the flat route only when slope factor is above threshold. Proposed test name: `zone_precheck_applies_slope_cost_only_above_factor_threshold`. | Do not add unconditional slope penalties for all movers. |
| Level 1 ignores current zone and returns `slope_context+0x57E4` at the neighbor zone representative index. | `0x0058628A..0x005862A9` | missing: Rust zone records do not store binary representative index or level-specific slope arrays. | `src/sim/pathfinding/zone_map.rs`, `zone_build.rs`, `zone_search.rs`. | Preserve per-level representative metadata if exact hierarchy is implemented. | Two level-1 edges into the same neighbor from different current zones should receive identical slope contribution. Proposed test name: `zone_slope_level1_depends_on_neighbor_representative_only`. | Do not use centroid-to-centroid distance as a substitute for this lookup. |
| Level 2 samples two source corners and two destination corners from a 130-wide slope grid, takes `min` at each endpoint, then returns `(src_min + dst_min) >> 1`. | `0x00585F63..0x00586287`, tables `0x0082A984`, `0x0082A9C4` | missing: no 130-wide slope grid/corner sampling metadata. | future zone hierarchy/slope-cost data surface; `src/sim/pathfinding/zone_search.rs`. | Implement direction-specific corner sampling before claiming slope parity. | Direction east from source to neighbor should use source corners `1,3` and destination corners `0,2`, picking mins before averaging. Proposed test name: `zone_slope_level2_uses_directional_corner_min_average`. | Do not average zone centroids, all four corners, or raw cell heights. |

### Negative Facts / Do Not Do

- Do not apply slope cost at level `0`; the helper returns `0`. Active in YR: Yes; evidence `0x00585F47..0x00585F4B`.
- Do not use source zone for level-1 slope cost; only neighbor zone representative is read. Active in YR: Yes; evidence `0x0058628A..0x005862A9`.
- Do not use centroid Manhattan distance or center-to-center height delta for this helper. Active in YR: Yes; evidence level-2 corner-table sampling at `0x0058611B..0x0058627F`.
- Do not clamp helper indices unless separately matching writer-side constraints; the helper itself performs no zone-id/grid bounds checks. Active in YR: Yes; evidence direct loads through graph and slope arrays.
- Do not treat slope cost as independent of the mover; `Zone_precheck` uses the moving object's slope factor and `+0x21C` context. Active in YR: Yes; evidence `0x0042C2BA..0x0042C2C9`.

### Remaining Uncertainty

- The exact writer lifecycle and allocation shape for the `Foot+0x21C` slope context was not traced; this report verifies the read contract needed by `Zone_precheck`.
- Runtime frequency of the alternate `FUN_0042D170` caller was not measured.
- The semantic source of the integer samples in `+0x57E4/+0x59F0` is inferred as slope cost from function usage and names; the read formula is verified, but construction remains a follow-up.

### Stale Docs / Follow-up Docs

- `docs/research/ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
  - Replace: "Full `Zone_Estimate_Slope_Cost` internals were not re-documented here; only the call/added-cost contract was verified."
  - With: "Full helper internals are documented in `ZONE_ESTIMATE_SLOPE_COST_GHIDRA_REPORT.md`: level 0 returns 0; level 1 reads the neighbor representative from the level-1 graph and indexes `Foot+0x21C + 0x57E4`; level 2 uses level-2 representatives, 130-wide `+0x59F0` corner samples, direction-specific two-corner mins, and returns their arithmetic half-sum. `Zone_precheck` adds `ftol(helper_result * slope_factor)` only when slope factor is greater than `1e-5`."

## Sources

- Ghidra decompiled: `0x00585F40`, `0x0042C290`, `0x004DC760`, `0x0042C900`, `0x004CBBA0`, `0x0042D170`, `0x004D3920`, `0x00581F90`, `0x0056BCD0`.
- Ghidra assembly/data contexts: `0x0042C2BA..0x0042C2FB`, `0x0042C56B..0x0042C5D2`, `0x00585EF6..0x00585F31`, `0x00585F40..0x005862B2`, `0x0082A984`, `0x0082A9C4`, `0x007E3810`, `0x007E3818`.
- Prior reports referenced: `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`, `ZONE_MAP_BUILD_LEVEL_GHIDRA_REPORT.md`, `ASTAR_PATHFIND_SEARCH_0042C900_RETRY_SEMANTICS_GHIDRA_REPORT.md`.
- Rust scan: Codegraph context and file reads for `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/zone_map.rs`.
