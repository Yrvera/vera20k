# Bridge Hut Fallback Stock Usage - Ghidra Research Report

**Address(es):** `0x00574000`, `0x00574C20`, `0x0043FB20`, `0x00438720`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** whether standard YR can execute the CABHUT no-overlay hut fallback, and how often shipped loose/packed RA2/YR maps place CABHUTs in no-overlay/fallback-shaped layouts.  
**Non-Scope:** full hut destruction state machine, collapse walker internals, bridge sound, event `0x1F`, and runtime debugger validation of every scanned map.  
**Confidence:** High for binary liveness and branch shape; Medium-High for stock-map incidence from static retail map scan.  
**Active in YR:** Yes. The functions are live in standard YR; the fallback branch is topology-conditional and is exercised by shipped map placements when the inner hut-centered overlay scan finds no matching bridge overlay.

## 0. Target / Stop Conditions

- **Target question:** Are CABHUT no-overlay / pure-flag bridge-hut fallback branches stock-map relevant, or mostly custom-map risk?
- **Non-goals:** Do not re-investigate the full hut destruction state machine; do not patch Rust; do not mutate Ghidra.
- **Evidence needed to mark COMPLETE:** read-only Ghidra proof that `DestroyBridge_{High,Low}_OnHutDeath` is called in standard YR; map scan covering loose map files plus shipped map MIX archives; exact count of CABHUT placements that hit overlay-fast vs no-overlay fallback shape; Rust-facing test handoff.
- **Stop conditions:** stop after binary liveness and stock incidence are resolved, with any limits of static scanning explicitly stated.

## 1. Overview

The fallback is not just a mod-map edge case. A static scan of the local retail install found 1,091 CABHUT placements across 133 shipped maps. Of those placements, 573 have no matching inner 5x5 bridge overlay for the selected high/low hut branch; 542 of those also have a nearby resolved `flags & 0x500` starter and therefore match the active binary fallback starter condition.

The fallback is especially common on high-bridge CABHUT layouts where the hut is near bridge marker/stamped cells rather than a high bridge overlay ID in the inner overlay band.

## 2. Binary Findings

| Finding | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `BuildingClass__Update` calls `DestroyBridge_Low_OnHutDeath` or `DestroyBridge_High_OnHutDeath` when `BuildingTypeClass+0x16B6` (`BridgeRepairHut`) is true and the delayed C4 marker/timer expires. | `0x0043FB20` decompile: C4 timer branch, `Type+0x16B6` gate, low/high 5x5 classifier, then calls `0x00574C20` or `0x00574000`. | High | Yes |
| `BombClass__Detonate` also calls the same hut destruction entries for a target building whose type has `BridgeRepairHut`. | `0x00438720` decompile: target RTTI building + `Type+0x16B6`, same 5x5 classifier, then calls low/high hut destruction. | High | Yes |
| The inner hut destruction scan is overlay-only and short-circuits. Low accepts `0x4A..=0x65`; high accepts `0xCD..=0xE8`. | `0x00574C20`, `0x00574000` decompile. | High | Yes |
| If the inner overlay scan fails, the no-overlay fallback checks `CellClass+0x140 & 0x500` at the hut cell, then direction indices `0..7`, distances `1..3`; if still neither `0x100` nor `0x400` is present it returns. | `0x00574000`, `0x00574C20` decompile. | High | Conditional: requires no inner overlay match and nearby flags. |
| The fallback then runs the already-documented anchor/ramp walk and bounded `ApplyDamageToCell` retry groups; no-overlay/no-starter is a silent no-op. | `0x00574000`, `0x00574C20`; prior `BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`. | High | Conditional |

## 3. Stock Map Scan Method

Scanner was a temporary Rust crate outside the repo at `%TEMP%/bridge_hut_stock_scan`, linking to `.` read-only.

Inputs:

- Loose retail map files under `<ra2-install>/`: `.mmx`, `.yro`, `.map`, `.mpr`, `.yrm`.
- Packed retail archives: `MAPS01.MIX`, `MAPS02.MIX`, `mapsmd03.mix`, `MULTI.MIX`, `multimd.mix`, `expandmd01.mix`.
- XCC global mix database for packed entry name lookup.
- Repo-extracted INI data: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- `MapFile::load_from_path`, `MixArchive::load`, `theater::load_theater`, `ResolvedTerrainGrid::build`, and `OverlayTypeRegistry::from_ini`.

Classification per CABHUT:

1. Find map-placed `[Structures]` entries with type `CABHUT`.
2. Run the same outer low/high selection used by the binary: low if any hut-centered 5x5 cell has low bridge overlay `0x4A..=0x65` or the resolved wood bridge repair tile predicate.
3. For the selected branch, test the inner hut-centered 5x5 for matching overlay band: low `0x4A..=0x65`, high `0xCD..=0xE8`.
4. If no matching overlay exists, search the hut cell first, then directions `N, NE, E, SE, S, SW, W, NW`, distances `1..3`, for resolved `bridge_facts.raw_flags & 0x500`.

Scan limits:

- This is static map-load classification, not a runtime debugger trace.
- Packed map names depend on XCC hash lookup; unresolved `[Basic] Name` values remain `<unnamed>`, but archive filename and internal map filename are recorded.
- Counts rely on the Rust resolved-terrain bridge flag model for static `CellClass+0x140` approximation. The binary branch shape is verified independently in Ghidra.

## 4. Stock Incidence Summary

| Metric | Count |
|---|---:|
| Candidate map entries seen | 407 |
| Unique map entries parsed | 285 |
| Parse failures | 0 |
| Unique maps with CABHUT | 133 |
| CABHUT placements | 1,091 |
| Placements dispatched low by outer classifier | 559 |
| Placements dispatched high by outer classifier | 532 |
| Inner overlay fast-path placements | 518 |
| No-overlay placements with `flags & 0x500` starter | 542 |
| No-overlay placements with no starter (fallback enters, then returns) | 31 |
| Unique maps with any no-overlay placement | 80 |

No-overlay placements by branch:

| Branch | Starter found | No starter | Total no-overlay |
|---|---:|---:|---:|
| High | 458 | 28 | 486 |
| Low | 84 | 3 | 87 |
| Total | 542 | 31 | 573 |

Maps with any no-overlay CABHUT placement:

| Map | No-overlay huts | Low | High | No-starter no-op |
|---|---:|---:|---:|---:|
| loose:amazon.mmx | 8 | 8 | 0 | 0 |
| loose:Barrel.mmx | 6 | 0 | 6 | 0 |
| loose:BayOPigs.mmx | 4 | 0 | 4 | 0 |
| loose:Carville.mmx | 12 | 12 | 0 | 0 |
| loose:Deadman.mmx | 8 | 0 | 8 | 0 |
| loose:EB3.mmx | 4 | 4 | 0 | 0 |
| loose:GoldSt.mmx | 16 | 0 | 16 | 0 |
| loose:Grinder.mmx | 8 | 0 | 8 | 0 |
| loose:Hills.mmx | 6 | 6 | 0 | 0 |
| loose:Kaliforn.mmx | 4 | 0 | 4 | 0 |
| loose:Pacific.mmx | 8 | 0 | 8 | 0 |
| loose:Potomac.mmx | 4 | 0 | 4 | 0 |
| loose:Rockets.mmx | 12 | 0 | 12 | 0 |
| loose:SeaofIso.mmx | 2 | 0 | 2 | 0 |
| loose:Unrepent.yro | 4 | 0 | 4 | 0 |
| loose:YuriPlot.mmx | 6 | 0 | 6 | 0 |
| MAPS01.MIX:all01t.map:<unnamed> | 2 | 0 | 2 | 0 |
| MAPS01.MIX:all03u.map:<unnamed> | 2 | 0 | 2 | 0 |
| MAPS01.MIX:all04u.map:<unnamed> | 3 | 0 | 3 | 0 |
| MAPS01.MIX:all11t.map:<unnamed> | 4 | 0 | 4 | 0 |
| MAPS01.MIX:sov01t.map:<unnamed> | 3 | 1 | 2 | 0 |
| MAPS02.MIX:sov08u.map:<unnamed> | 2 | 0 | 2 | 0 |
| MAPS02.MIX:sov1u.map:<unnamed> | 4 | 0 | 4 | 0 |
| mapsmd03.mix:all02umd.map:<unnamed> | 12 | 0 | 12 | 0 |
| mapsmd03.mix:all03umd.map:<unnamed> | 2 | 0 | 2 | 0 |
| mapsmd03.mix:sov03umd.map:<unnamed> | 9 | 0 | 9 | 1 |
| MULTI.MIX:mp02t2.map:<unnamed> | 2 | 0 | 2 | 0 |
| MULTI.MIX:mp04t8.map:<unnamed> | 12 | 0 | 12 | 0 |
| MULTI.MIX:mp06mw.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:mp06t2.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:mp07t4.map:<unnamed> | 4 | 0 | 4 | 2 |
| MULTI.MIX:mp09du.map:<unnamed> | 3 | 0 | 3 | 0 |
| MULTI.MIX:mp09t3.map:<unnamed> | 3 | 0 | 3 | 0 |
| MULTI.MIX:mp14mw.map:<unnamed> | 8 | 0 | 8 | 0 |
| MULTI.MIX:mp14t2.map:<unnamed> | 8 | 0 | 8 | 0 |
| MULTI.MIX:mp18du.map:<unnamed> | 10 | 0 | 10 | 2 |
| MULTI.MIX:mp18s3.map:<unnamed> | 8 | 0 | 8 | 0 |
| MULTI.MIX:mp23mw.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:mp23t4.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:mp24du.map:<unnamed> | 16 | 0 | 16 | 0 |
| MULTI.MIX:mp24t2.map:<unnamed> | 16 | 0 | 16 | 16 |
| MULTI.MIX:mp27du.map:<unnamed> | 2 | 0 | 2 | 2 |
| MULTI.MIX:mp27mw.map:<unnamed> | 2 | 0 | 2 | 0 |
| MULTI.MIX:mp28u4.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:mp34u4.map:<unnamed> | 2 | 0 | 2 | 0 |
| MULTI.MIX:tn02mw.map:<unnamed> | 4 | 0 | 4 | 0 |
| MULTI.MIX:tn02s4.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:2peaks.map:<unnamed> | 2 | 0 | 2 | 0 |
| multimd.mix:austintx.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:bridgegap.map:<unnamed> | 10 | 0 | 10 | 0 |
| multimd.mix:c1a02md.map:<unnamed> | 12 | 0 | 12 | 0 |
| multimd.mix:c2s01md.map:<unnamed> | 2 | 0 | 2 | 1 |
| multimd.mix:c2s03md.map:<unnamed> | 2 | 0 | 2 | 0 |
| multimd.mix:c3y01md.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:c3y02md.map:<unnamed> | 4 | 4 | 0 | 0 |
| multimd.mix:c3y03md.map:<unnamed> | 14 | 0 | 14 | 0 |
| multimd.mix:c4w01md.map:<unnamed> | 12 | 0 | 12 | 0 |
| multimd.mix:downtown.map:<unnamed> | 32 | 0 | 32 | 0 |
| multimd.mix:eastvsbest.map:<unnamed> | 4 | 4 | 0 | 0 |
| multimd.mix:fight.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:frstbite.map:<unnamed> | 4 | 4 | 0 | 0 |
| multimd.mix:manhatta.map:<unnamed> | 12 | 0 | 12 | 0 |
| multimd.mix:rushhr.map:<unnamed> | 12 | 0 | 12 | 0 |
| multimd.mix:triplecrossed.map:<unnamed> | 18 | 0 | 18 | 0 |
| multimd.mix:turfwar.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:xbarrel.map:<unnamed> | 6 | 0 | 6 | 0 |
| multimd.mix:xcarville.map:<unnamed> | 12 | 12 | 0 | 0 |
| multimd.mix:xgoldst.map:<unnamed> | 16 | 0 | 16 | 0 |
| multimd.mix:xmp04t8.map:<unnamed> | 12 | 0 | 12 | 0 |
| multimd.mix:xmp07t4.map:<unnamed> | 4 | 0 | 4 | 2 |
| multimd.mix:xmp14mw.map:<unnamed> | 8 | 0 | 8 | 0 |
| multimd.mix:xmp14t2.map:<unnamed> | 8 | 0 | 8 | 0 |
| multimd.mix:xmp23t4.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:xmp24du.map:<unnamed> | 16 | 0 | 16 | 0 |
| multimd.mix:xmp27du.map:<unnamed> | 2 | 0 | 2 | 2 |
| multimd.mix:xnorest.map:<unnamed> | 32 | 32 | 0 | 3 |
| multimd.mix:xpacificmw.map:<unnamed> | 8 | 0 | 8 | 0 |
| multimd.mix:xpotomac.map:<unnamed> | 4 | 0 | 4 | 0 |
| multimd.mix:xseaofiso.map:<unnamed> | 2 | 0 | 2 | 0 |
| multimd.mix:xtn02s4.map:<unnamed> | 4 | 0 | 4 | 0 |

## 5. Current Rust Implementation Status

Current Rust already has a dedicated hut fallback plan rather than the older traced-list-only shape:

- `src/sim/world/bridge_orchestrator.rs:390` defines `HutFallbackPlan`.
- `src/sim/world/bridge_orchestrator.rs:441` builds the plan.
- `src/sim/world/bridge_orchestrator.rs:448` finds the first fallback starter.
- `src/sim/world/bridge_orchestrator.rs:478` resolves structural vs pure-bridgehead anchors.
- `src/sim/world/bridge_orchestrator.rs:512` implements pure bridgehead anchor resolution.
- `src/sim/world/bridge_orchestrator.rs:552` runs the ramp/endpoint walk.
- Tests exist at `src/sim/world/world_orders_bridge_repair_tests.rs:834` and `:858` for pure bridgehead opposite offset and rejection of `0x80/0x800` alone.

Rust-facing risk is now less "fallback missing entirely" and more "needs stock-map fixture coverage and no-op fallback coverage for actual shipped layouts."

## 6. Coverage Ledger

| Area / branch | Status | Evidence | What remains |
|---|---|---|---|
| High OnHutDeath liveness | verified | `0x0043FB20`, `0x00438720`, `0x00574000` | none |
| Low OnHutDeath liveness | verified | `0x0043FB20`, `0x00438720`, `0x00574C20` | none |
| Inner overlay scan bands | verified | `0x00574000`, `0x00574C20` | none |
| Fallback starter mask/order | verified | `0x00574000`, `0x00574C20`; prior fallback report | none |
| Retail loose map scan | verified | 50 top-level `.mmx/.yro/.map` candidates; parsed by `MapFile::load_from_path` | none |
| Retail packed map scan | verified | `MAPS01.MIX`, `MAPS02.MIX`, `mapsmd03.mix`, `MULTI.MIX`, `multimd.mix`, `expandmd01.mix`; 407 candidate entries seen | runtime debugger validation of every map not done |
| Static `CellClass+0x140` approximation | touched-not-exhausted | `ResolvedTerrainGrid::build` + bridge facts | exact binary cell flags per map would require runtime dump |
| Full hut state machine | deferred | explicitly non-scope | use parent reports |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is the fallback function live in standard YR? -> Yes, C4 timer and demo-truck paths call it when `BridgeRepairHut` is true.` (evidence: `0x0043FB20`, `0x00438720`)
- `[RESOLVED] OQ-02 - Does the branch require no matching inner 5x5 overlay? -> Yes, fallback is after the overlay-only scan returns no match.` (evidence: `0x00574000`, `0x00574C20`)
- `[RESOLVED] OQ-03 - What qualifies the fallback starter? -> `CellClass+0x140 & 0x500`, checked at hut cell then direction `0..7`, distance `1..3`.` (evidence: `0x00574000`, `0x00574C20`)
- `[RESOLVED] OQ-04 - Do shipped maps contain no-overlay CABHUT placements? -> Yes: 573 placements across 80 maps; 542 have a nearby starter.` (evidence: static retail scan output `%TEMP%/bridge_hut_stock_scan_output_named.txt`)
- `[RESOLVED] OQ-05 - Is stock incidence mostly low or high? -> Mostly high: 486 high no-overlay placements vs 87 low.` (evidence: static retail scan)
- `[RESOLVED] OQ-06 - Are there stock no-overlay no-op cases? -> Yes: 31 placements have no resolved `0x500` starter within the binary search radius.` (evidence: static retail scan)
- `[RESOLVED] OQ-07 - Is current Rust still obviously missing the whole fallback? -> No; it has starter/anchor/ramp plan functions and focused tests.` (evidence: `bridge_orchestrator.rs:390..552`, `world_orders_bridge_repair_tests.rs:834`, `:858`)
- `[DEFERRED] OQ-08 - Do all 542 starter-positive static placements collapse identically in a live gamemd runtime?` (category: needs-runtime-debugger; reason: static scan approximates `CellClass+0x140`; next-step-if-pursued: instrument one representative high and one low map in gamemd and log selected starter/anchor)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Shipped maps contain many high-bridge CABHUT placements where the inner high overlay scan finds no `0xCD..=0xE8` overlay but fallback finds a `0x500` starter. | Static scan: 458 high starter-positive no-overlay placements; binary fallback at `0x00574000`. | coverage gap | `src/sim/world/bridge_orchestrator.rs`, map fixture/integration tests | Add a stock-map-derived high fallback fixture that destroys a CABHUT at a scanned coordinate and asserts fallback dispatch collapses/updates bridge state. | Use one named stock map such as `loose:Barrel.mmx` or `multimd.mix:bridgegap.map`, target a recorded high no-overlay CABHUT coordinate, verify bridge state changes through fallback. | Do not assume overlay-fast coverage proves high bridge hut destruction parity. Proposed test: `c4_on_stock_high_cabhut_no_overlay_fallback_collapses_bridge`. |
| Shipped maps also contain low no-overlay fallback placements, though fewer than high. | Static scan: 84 low starter-positive no-overlay placements; binary fallback at `0x00574C20`. | coverage gap | `src/sim/world/bridge_orchestrator.rs`, low bridge/tube fixtures | Add one stock-map-derived low fallback fixture so low branch starter/ramp behavior is covered. | Use `loose:Carville.mmx`, `loose:Hills.mmx`, `multimd.mix:xcarville.map`, or another listed low fallback map and assert the selected CABHUT dispatch enters low fallback. | Do not treat low overlay-fast maps like `CrctBrd.yro` as fallback coverage. Proposed test: `c4_on_stock_low_cabhut_no_overlay_fallback_collapses_bridge`. |
| No-overlay/no-starter is a real stock shape and returns without adjacent dirty/tactical dirty from the fallback body. | Static scan: 31 no-starter placements; binary early return in `0x00574000`/`0x00574C20`. | likely untested | `run_hut_fallback_plan`, C4-on-CABHUT tests | Add a deterministic no-starter case that clears the C4 marker but leaves bridge state unchanged and emits no fallback dirty side effects. | Use `MULTI.MIX:mp24t2.map` or `multimd.mix:xnorest.map` no-starter coordinates as fixture seed. | Do not force-collapse just because a CABHUT exists near a bridge-like area. Proposed test: `c4_on_cabhut_no_overlay_no_starter_noops_bridge`. |

## 9. Negative Facts / Do Not Do

- Do not treat this as mod-only risk; shipped maps contain 542 starter-positive fallback placements.
- Do not search the whole map for an overlay after the inner 5x5 overlay scan fails; the binary switches to flags and bounded direction/distance search.
- Do not let `0x80` or `0x800` alone qualify a starter; the binary starter mask is `0x500`.
- Do not assume all CABHUTs with no matching overlay should collapse; 31 stock placements have no resolved starter and map to the early no-op branch.
- Do not claim static scan proves every live runtime starter/anchor coordinate byte-for-byte; exact per-map `CellClass+0x140` would require runtime logging.

## 10. Stale Docs / Follow-up Docs

- `BRIDGE_NEXT_FIX_PRIORITY_VERIFICATION_GHIDRA_REPORT.md`: replace "stock-map hut fallback incidence deferred / unknown" with "static retail scan found 573 no-overlay CABHUT placements across 80 shipped maps; 542 had a nearby `flags & 0x500` starter and 31 were no-starter no-ops."
- `BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`: replace "Medium for stock-map incidence" with "Medium-High; see `BRIDGE_HUT_FALLBACK_STOCK_USAGE_GHIDRA_REPORT.md` for scan method and exact map list."

## Sources

- Ghidra read-only decompile: `0x00574000` `MapClass__DestroyBridge_High_OnHutDeath`
- Ghidra read-only decompile: `0x00574C20` `MapClass__DestroyBridge_Low_OnHutDeath`
- Ghidra read-only decompile: `0x0043FB20` `BuildingClass__Update`
- Ghidra read-only decompile: `0x00438720` `BombClass__Detonate`
- Prior report: `docs/research/BRIDGE_HUT_FALLBACK_FLAGS_RAMP_ONLY_GHIDRA_REPORT.md`
- Prior report: `docs/research/BRIDGE_NEXT_FIX_PRIORITY_VERIFICATION_GHIDRA_REPORT.md`
- Retail assets/maps: `<ra2-install>/`
- Static scan output: `%TEMP%/bridge_hut_stock_scan_output_named.txt`
- Current Rust scan: `src/sim/world/bridge_orchestrator.rs`, `src/sim/world/world_orders_bridge_repair_tests.rs`
