# MapClass::Get_CellClass Fallback Dummy Cell - Ghidra Research Report

**Address(es):** `MapClass::Get_CellClass @ 0x005657A0`  
**Investigation Mode:** exhaustive-slice downgraded to partial because this session has no exposed Ghidra MCP.  
**Claimed Scope:** Prior-verified behavior and Rust-facing contract for `MapClass::Get_CellClass` in-bounds lookup and out-of-bounds/null-cell dummy fallback.  
**Non-Scope:** Full caller census, CellClass full layout, pathfinding formulas, bridge zone rebuilds, and AAHeatSeeker2 retargeting behavior already covered by sibling reports.  
**Confidence:** Medium overall. The central fallback claim is high in prior binary reports, but this slot did not freshly decompile `0x005657A0`.  
**Active in YR:** Yes for the central helper and its standard callers, by prior binary reports citing live YR call paths.

## 1. Overview

`MapClass::Get_CellClass @ 0x005657A0` is the standard engine helper that converts a packed cell coordinate into a `CellClass*`. Prior Ghidra reports agree on the important parity contract: invalid cell lookup does not return null to most consumers; it returns a dummy CellClass-compatible object rooted at `DAT_00ABDC50`, while storing the requested coordinate at `DAT_00ABDC74`.

This matters because callers often immediately read fields or dispatch CellClass methods after lookup. A Rust API that models every invalid lookup as `None` can skip side effects, skip fallback checks, or diverge from callers that are supposed to receive a non-null cell-like object.

## 2. Class Layout / Key Offsets

| Field / global | Offset / value | Purpose in this slice | Evidence | Active in YR |
|---|---:|---|---|---|
| Cell array index formula | `y * 0x200 + x` | Fixed 512-wide map-cell indexing, independent of loaded-map playfield width | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61` | Yes, used by live helper paths in cited reports |
| Valid linear index range | `0..0x3FFFF` | Any computed index outside this range falls back to the dummy cell | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61` | Yes |
| `DAT_00ABDC50` | global dummy cell base | CellClass-compatible fallback returned for OOB/null cell pointer | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7 | Yes |
| `DAT_00ABDC74` | `DAT_00ABDC50 + 0x24` | Stores the requested packed cell coordinate when fallback is used | `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md:177`; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7 | Yes |
| Dummy bridge flags | zero/clear in prior bridge-zone report | Reads of `CellClass+0x140 & 0x100` on the dummy cell do not enter bridge logic | `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7 | Yes for bridge-zone consumers |
| Dummy-cell identity check | `this == DAT_00ABDC50` | Some CellClass routines explicitly early-out on the fallback cell | `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md:38,97,119` | Yes |

## 3. Core Logic

Prior reports describe the helper contract as:

1. Interpret the input as a packed cell coordinate with signed low/high 16-bit `x`/`y` in consumers that unpack the coordinate.
2. Compute the fixed-width linear index as `y * 0x200 + x`.
3. If the index is outside `[0, 0x3FFFF]`, or if the indexed cell pointer is null, write the original requested coordinate to `DAT_00ABDC74` and return `DAT_00ABDC50`.
4. Otherwise return the real `CellClass*`.

Material finding: the fallback is not a Rust-style absence value. It is a non-null, CellClass-compatible object. Active in YR: Yes. Evidence: prior binary report `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`, plus independent OOB dummy-cell notes in `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93` and `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7.

Material finding: the fixed 512-wide index and `0x3FFFF` upper bound are not the playable map rectangle. Some callers still perform later playfield checks, but `Get_CellClass` itself supplies a dummy object first. Active in YR: Yes. Evidence: `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93`, where one wrapper does not call `MapClass::IsRectInPlayfield` and another does so after the cell loop.

Material finding: fallback writes the requested coordinate into the dummy-cell coordinate storage, so consumers reading the cell's coordinate field can still observe the probed coordinate rather than a constant `(0,0)` dummy. Active in YR: Yes. Evidence: `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md:177`; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7.

Material finding: at least one CellClass routine recognizes the dummy cell by address and returns immediately. Active in YR: Yes. Evidence: `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md:38,97,119`.

## 4. INI Keys

No INI keys control `MapClass::Get_CellClass` fallback behavior in the reports checked. The behavior is an engine helper contract, not rules/art data. Active in YR: Yes as engine code; no INI gate found in scoped docs.

## 5. Integration Points

Verified or prior-verified caller categories that rely on receiving a CellClass-compatible object:

| Caller / category | Contract used | Evidence | Active in YR |
|---|---|---|---|
| Bullet target invalidation retarget | Removed ground target can be replaced with `MapClass::Get_CellClass(last_target_cell)` and stored as a non-null target pointer | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:33,35`; `BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md:48,58,90` | Yes for standard destroyed-target path |
| CellRect passability and occupancy validators | OOB/null cells use dummy cell before optional/final playfield checks | `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93` | Yes |
| Bridge zone and bridge probing helpers | OOB probes use dummy cell with bridge flags clear; bridge logic is skipped for dummy | `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7 | Yes |
| IsoMapPack5 map decoding | OOB/null cells redirect to dummy, payload is still consumed instead of aborting decode | `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md:82-97,216` | Yes for map-load path |
| Cell attribute recalculation | Dummy cell identity can early-out instead of mutating real map state | `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md:38,97,119` | Yes |

Tick-cycle integration was not freshly traced in this slot. The helper is used by both map-load and runtime systems; exact tick ordering belongs to the individual caller reports.

## 6. Current Rust Implementation Status

Rust currently has multiple cell-access contracts rather than one gamemd-style `CellClass*` surface:

| Rust surface | Current behavior | Rust delta against this helper contract |
|---|---|---|
| `src/map/resolved_terrain.rs:264-272` | `ResolvedTerrainGrid::index/cell` returns `None` for `rx >= width || ry >= height` | Missing a documented gamemd-style sentinel/dummy access mode for callers that need non-null fallback semantics |
| `src/sim/pathfinding/core.rs:1165-1171` | `PathGrid::cell` returns `None` for OOB; selected search internals use `DEFAULT_BLOCKED_CELL` via `unwrap_or` at several call sites | Partially similar but not equivalent: default blocked cell is pathfinding-local, not a CellClass-compatible dummy with requested coordinate storage |
| `src/sim/bridge_state/mod.rs:760-770` | Bridge runtime cell access returns `None` for OOB or non-bridge cell | Not equivalent for bridge probes that should read a dummy cell with flags clear before later caller-specific checks |
| `src/sim/overlay_grid.rs:77-78,520` | Overlay reads return a static default for OOB | Closest existing Rust pattern, but only overlay data; it does not solve full CellClass-compatible fallback |

This report does not recommend replacing all `Option` APIs. It identifies that Rust needs an explicit parity contract for call sites that model `MapClass::Get_CellClass`, separate from strict checked access used by internal Rust systems.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MapClass::Get_CellClass @ 0x005657A0` central fallback contract | touched-not-exhausted | Prior binary reports listed above | Fresh direct Ghidra MCP recheck of the function body |
| Fixed `y * 0x200 + x` indexing and `[0,0x3FFFF]` bounds | touched-not-exhausted | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61` | Fresh instruction-level spot check |
| Null indexed-cell pointer fallback | touched-not-exhausted | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61` | Fresh instruction-level spot check |
| `DAT_00ABDC74` coord write | touched-not-exhausted | `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md:177`; `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7 | Fresh instruction-level spot check |
| `DAT_00ABDC50` dummy CellClass-compatible object | touched-not-exhausted | `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md:216` | Dump full initialized field values if needed |
| Full dummy-cell field table | deferred | `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md:356-357` already deferred this | Separate CellClass dummy-layout investigation |
| Complete caller census | deferred | Many reports cite callers; no Ghidra MCP here | Dedicated xref census from `0x005657A0` |
| Rust cell API scan | verified | CodeGraph and focused `rg` over `src/`; files listed in section 6 | Future implementation scan should decide exact ownership |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] Q1 - Is this target narrow enough for exhaustive-slice? -> Yes in scope, but downgraded because Ghidra MCP was not available in this session.` (evidence: session tool list; no Ghidra namespace exposed)
- `[RESOLVED] Q2 - Does prior research already state the fallback behavior? -> Yes; the AAHeatSeeker2 CellClass retarget report states fixed indexing, OOB/null fallback, `DAT_00ABDC74`, and `DAT_00ABDC50`.` (evidence: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`)
- `[RESOLVED] Q3 - Is the fallback active in standard YR? -> Prior reports mark it active in live YR caller paths including bullet invalidation, CellRect validators, and bridge helpers.` (evidence: cited reports in sections 2 and 5)
- `[RESOLVED] Q4 - Does the helper return null for invalid cells? -> No in prior reports; it returns the dummy cell and stores the probed coordinate.` (evidence: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md:177`)
- `[RESOLVED] Q5 - Is the dummy simply equivalent to rejecting out-of-play coordinates? -> No; some callers read the dummy before later playfield checks, and some do not perform the same playfield check.` (evidence: `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93`)
- `[RESOLVED] Q6 - Does the dummy preserve the requested coordinate? -> Prior reports say the requested coordinate is written at `DAT_00ABDC74`, equivalent to dummy + `0x24`.` (evidence: `BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md` section 6.7)
- `[RESOLVED] Q7 - Do any routines special-case the dummy by identity? -> Yes; `CellClass::RecalcAttributes @ 0x0047D2B0` has a sentinel early-out per prior report.` (evidence: `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md:38,97,119`)
- `[RESOLVED] Q8 - Are INI keys involved? -> None found in scoped docs; this is engine helper behavior.` (evidence: focused doc/INI relevance scan; no target-specific key)
- `[RESOLVED] Q9 - Does current Rust have one equivalent map-cell access contract? -> No; terrain/path/bridge APIs generally return `Option`, while overlay has a default OOB read.` (evidence: `src/map/resolved_terrain.rs:264-272`; `src/sim/pathfinding/core.rs:1165-1171`; `src/sim/overlay_grid.rs:77-78`)
- `[DEFERRED] Q10 - What are all initialized field values inside `DAT_00ABDC50`?` (category: `requires-different-system-context`; reason: no Ghidra MCP or binary memory dump in this session; next-step-if-pursued: dump dummy bytes and constructor/init xrefs)
- `[DEFERRED] Q11 - Which exact callers require dummy semantics versus strict rejection?` (category: `bounded-cost-too-high`; reason: complete xref census is outside this slot without Ghidra; next-step-if-pursued: xref `0x005657A0` and classify each caller)
- `[DEFERRED] Q12 - Does every dummy-cell method dispatch remain safe beyond coordinate/flag reads?` (category: `requires-different-system-context`; reason: requires CellClass vtable/method sweep, not this helper slice; next-step-if-pursued: enumerate CellClass methods invoked on `DAT_00ABDC50`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Invalid `MapClass::Get_CellClass` lookups return a non-null dummy cell and store the requested coordinate, instead of returning null | Prior binary reports: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md:177` | Missing/unchecked; core terrain/path APIs currently expose `Option` for OOB | `src/map/resolved_terrain.rs`; future shared map-cell access layer | Add an explicit gamemd-style access contract for callers that need dummy-cell semantics, preserving requested coord and dummy identity | Deterministic unit test: requesting `(-1,0)`/large unsigned equivalent through the parity access API returns a dummy-like cell whose coord is the requested coord and whose bridge flags are clear. Proposed test name: `test_get_cellclass_oob_returns_dummy_with_requested_coord` | Do not globally replace checked `Option` access; strict Rust internals still need absence for logic that is not modeling gamemd helper calls |
| Fixed lookup uses `y * 0x200 + x` and valid linear range `[0,0x3FFFF]`, not loaded-map width/height | Prior binary reports: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`; `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61` | Mismatch risk; Rust grid indexing uses current grid width in `ResolvedTerrainGrid::index` and `PathGrid::cell` | `src/map/resolved_terrain.rs`; `src/sim/pathfinding/core.rs`; any future helper mirroring `MapClass::Get_CellClass` | Keep existing width-based APIs, but when implementing `Get_CellClass` parity semantics use the binary's 512-wide index/range rules | Deterministic unit test: coordinate whose loaded-map width check would fail but binary linear range would be in range must follow the parity helper's documented 512-wide rule. Proposed test name: `test_get_cellclass_uses_512_wide_linear_index_contract` | Do not use playable-map rectangle checks as a substitute for the helper's own fallback predicate |
| Callers may continue after dummy substitution and perform their own later checks | `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93`; `ISOMAPPACK5_DECODER_GHIDRA_REPORT.md:82-97` | Unchecked; many Rust call sites short-circuit on `None` | Pathfinding/cell validators, bridge probing, map-load decoders | Audit callers that model gamemd helper flows and decide whether OOB should use dummy/default data first, then caller-specific rejection | Acceptance scenario: a CellRect-like validator probing an OOB cell receives dummy data, then only fails if its caller-specific playfield check requires it. Proposed test name: `test_cellrect_validator_applies_dummy_before_playfield_reject` | Do not collapse dummy-cell behavior into an immediate OOB false for all callers |

### Negative Facts / Do Not Do

- Do not model `MapClass::Get_CellClass` as `Option<Cell>` at parity call sites. Prior reports say invalid lookup returns `DAT_00ABDC50`, not null. Active in YR: Yes. Evidence: `AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md:35`.
- Do not replace the helper's fallback predicate with playable-map bounds. Active in YR: Yes. Evidence: `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md:61,93`.
- Do not use a dummy cell with constant `(0,0)` coordinates. Active in YR: Yes. Evidence: prior docs state the requested coordinate is written to `DAT_00ABDC74`/dummy+`0x24`.
- Do not infer that dummy-cell reads always mean passable. Active in YR: Conditional by caller. Evidence: bridge reports note flags clear and bridge logic skipped, while CellRect validators perform additional checks.
- Do not hardcode this contract into unrelated overlay/smudge default reads without checking their gamemd caller. Active in YR: Conditional. Evidence: Rust overlay OOB default exists, but this report only covers `MapClass::Get_CellClass`.

### Remaining Uncertainty

- Fresh direct decompilation of `0x005657A0` was not possible in this session because no Ghidra MCP tools were exposed.
- Full initialized contents of `DAT_00ABDC50` remain unresolved here; prior reports verify key coordinate/flag behavior but not a complete field table.
- Complete xref/caller classification remains unresolved; this report lists representative live callers from existing reports.
- Whether Rust should centralize this as a shared `CellClass`-like facade or narrower per-caller helpers is an implementation design choice, not settled by this research.

## Sources

- `docs/research/AAHEATSEEKER2_CELLCLASS_RETARGET_AI_BEHAVIOR_GHIDRA_REPORT.md`
- `docs/research/BULLETCLASS_TARGET_INVALIDATION_AAHEATSEEKER2_GHIDRA_REPORT.md`
- `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_ZONE_LIFECYCLE_GHIDRA_REPORT.md`
- `docs/research/DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md`
- `docs/research/ISOMAPPACK5_DECODER_GHIDRA_REPORT.md`
- `docs/research/FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`
- Rust scan: `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, `src/sim/bridge_state/mod.rs`, `src/sim/overlay_grid.rs`
