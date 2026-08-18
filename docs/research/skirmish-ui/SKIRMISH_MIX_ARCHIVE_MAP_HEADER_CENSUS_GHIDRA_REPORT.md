# Skirmish MIX Archive Map Header Census - Ghidra Research Report

**Date:** 2026-05-21  
**Address(es):** `0x00699980`, `0x005D63E0`, `0x0069AE10`, `0x00689D30`, `0x00640710`, `0x00641B00`  
**Investigation Mode:** exhaustive-slice scoped to archive/PKT census  
**Claimed Scope:** MIX-contained or patch-contained stock map records visible through the offline Skirmish Choose Map source path, classified for `[PreviewPack]`, `[Header] NumberStartingPoints`, `Waypoint1..8`, and live `STARTBUT.SHP` overlay eligibility.  
**Non-Scope:** full preview-pixel decoding, exact baked red marker count, exact Choose Map display sorting beyond prior binary order report, campaign-only `MAPS01/02/mapsmd03` payload behavior, custom loose user maps, and runtime screenshot validation.  
**Confidence:** High for the live YR source path and local retail archive census; Medium for treating unresolved legacy `MISSIONS.PKT` cooperative entries as absent rather than intentionally hidden, because the local asset lookup cannot resolve 50 of those RA2 cooperative names.
**Active in YR:** Yes for the `MISSIONSMD.PKT` / `MISSIONS.PKT` source path and preview/live-overlay gates; conditional by selected mode/filter and by whether a listed map payload resolves in the local install.

## 1. Overview

The prior root-file census was correct for the 54 loose local map files, but it is not the full stock map universe used by gamemd's offline Skirmish chooser. The live list builder first consumes `MISSIONSMD.PKT` / `MISSIONS.PKT` records, and those records resolve to `.map` payloads inside `multimd.mix`, `expandmd01.mix`, and `multi.mix`.

The archive-contained YR stock path changes the live overlay picture substantially: all 161 locally resolved `MISSIONSMD.PKT` maps have `[PreviewPack]` and eligible `[Header] NumberStartingPoints`, so they draw live `STARTBUT.SHP` overlays. The base RA2 `MISSIONS.PKT` maps that resolve locally through `multi.mix` have `[PreviewPack]` but no `[Header]`, so they remain baked-preview-only.

## 2. Binary Gates Reused By This Census

| Finding | Active in YR | Evidence |
|---|---|---|
| Choose Map source records are seeded from `MISSIONSMD.PKT`, loose `*.PKT`, `*.YRO`/embedded PKT, and loose `*.YRM`; the modal consumes `DAT_00A8B8CC`, not a fresh filesystem scan. | Yes | `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`; `0x00699980`; strings `MISSIONSMD.PKT`, `MultiMaps`, `*.PKT`, `*.YRO`, `*.YRM`. |
| Map records with no parsed `GameModes` list match selected mode string `standard`. | Yes | `0x005D63E0 -> 0x0069AE10`; zero-count branch compares to string `standard` at `0x0083F668`. |
| `[Header]` preview metadata defaults to `-1` for `NumberStartingPoints` and zeroes eight waypoint pairs before reads. | Yes | `FUN_00689D30 @ 0x00689D30`. |
| Live start overlays draw only when `0 < ScenarioClass+0x113C < 9`. | Yes | `DrawStartPositions @ 0x00640710`. |
| `[PreviewPack]` drives the selected-map preview surface; this report only checked presence, not full RGB decode for every archive map. | Yes | preview load/decode path `0x00641B00`; root decode details remain in `SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md`. |

## 3. Census Method

Inputs:

- `MISSIONSMD.PKT` resolved from `langmd.mix`.
- `MISSIONS.PKT` resolved from `ra2.mix -> local.mix`.
- Map payloads resolved by basename plus `.map`, `.yro`, `.yrm`, `.mmx`, `.mpr` through the same local asset manager priority stack used by the Rust tooling: `expandmd01.mix`, `multimd.mix`, `multi.mix`, and base archives.
- Candidate campaign archives `MAPS01.MIX`, `MAPS02.MIX`, and `mapsmd03.mix` were raw-scanned only enough to exclude them from this offline Skirmish stock-map claim; they contain campaign/script map payloads, not the PKT-driven Skirmish list surface.

Classification:

- `PreviewPack`: section exists.
- `HeaderN`: `[Header] NumberStartingPoints` parsed as integer, or `none`.
- `Live`: `HeaderN` is in `1..8`, matching `DrawStartPositions`.
- `WaypointN count`: number of `[Header] Waypoint1..Waypoint8` keys present.
- `standard`: no `GameModes` list or explicit `standard`; all resolved PKT maps here have empty mode lists and therefore match `standard` through `0x0069AE10`.

## 4. Results

| Source list | PKT source | Map payload sources | PKT entries | Resolved maps | PreviewPack | Header maps | Live eligible | Active in YR |
|---|---|---|---:|---:|---:|---:|---:|---|
| `MISSIONSMD.PKT` | `langmd.mix` | `multimd.mix` 149, `expandmd01.mix` 12 | 161 | 161 | 161 | 161 | 161 | Yes |
| `MISSIONS.PKT` | `ra2.mix -> local.mix` | `multi.mix` 87 | 137 | 87 | 87 | 0 | 0 | Conditional: base RA2 stock records visible only when the YR list includes/uses base `MISSIONS.PKT`; local unresolved cooperative names remain absent. |
| Loose root maps from prior report | install root | `.mmx`, `.yro`, `.map` | n/a | 54 | 54 | 9 | 9 | Yes for loose local files, but not the full stock list. |

YR archive live-overlay count distribution from `MISSIONSMD.PKT`:

| `NumberStartingPoints` | Count | Active in YR |
|---:|---:|---|
| 2 | 34 | Yes |
| 3 | 7 | Yes |
| 4 | 81 | Yes |
| 5 | 1 | Yes |
| 6 | 24 | Yes |
| 8 | 14 | Yes |

Base RA2 archive records from `MISSIONS.PKT`:

- 87 locally resolved `multi.mix` maps all have `[PreviewPack]`.
- 87/87 have no `[Header] NumberStartingPoints`, so `FUN_00689D30` leaves `ScenarioClass+0x113C = -1`.
- 0/87 are live-overlay eligible; they are baked-preview-only for this gate.
- 50 `MISSIONS.PKT` names, mostly cooperative campaign-style records, did not resolve to a local `.map/.yro/.yrm/.mmx/.mpr` payload through the local install stack. This report does not classify missing payloads.

## 5. Corrected Picture Versus Root-File Census

The root-file report remains accurate for its claimed 54 loose files. The broader stock-map conclusion must change:

- Loose root stock files: 54 total, 9 live-overlay eligible.
- PKT/MIX stock YR records: 161 total, 161 live-overlay eligible.
- Resolved base PKT/MIX records: 87 total, 0 live-overlay eligible.

Player-visible implication: relying only on loose root maps would make most stock YR Skirmish maps look like the Dustbowl baked-only path. Retail YR's archive-backed `MISSIONSMD.PKT` maps normally exercise live `STARTBUT.SHP` overlays.

## 6. Current Rust Implementation Status

Active in YR: Not applicable; this is implementation comparison.

Current Rust `src/app_list_maps.rs` scans only the RA2 directory for loose `.mmx/.yro/.map/.mpr/.yrm` files and sorts by display name. It does not load `MISSIONSMD.PKT`, `MISSIONS.PKT`, or archive-contained `.map` payloads, and it does not reproduce the binary's `DAT_00A8B8CC` source order/filtering path.

Rust-facing delta:

- Missing stock-map source: `MISSIONSMD.PKT` records from `langmd.mix` and map payloads from `multimd.mix` / `expandmd01.mix`.
- Missing base stock source if desired: resolved `MISSIONS.PKT` records from `multi.mix`.
- Current root-only census under-represents live overlay fixtures; tests should include archive-backed names such as `XMP29U2.map`, `XMP03T4.map`, `XMP25T6.map`, and `XDeath.map`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Live Choose Map source path | verified | `0x00699980`; prior list-population report | none for this slice |
| `MISSIONSMD.PKT` local census | verified | `langmd.mix`; 161/161 map payloads resolved | exact display labels/order outside this slot |
| `MISSIONS.PKT` local census | touched-not-exhausted | `ra2.mix -> local.mix`; 87 resolved, 50 missing | missing cooperative names need separate asset/source audit if base RA2 co-op list matters |
| Archive raw map payloads not PKT-referenced | deferred | raw scan found extra campaign/map payloads in `MAPS01/02/mapsmd03`, `multimd`, `expandmd01` | out of scope; not claimed as visible Skirmish list entries |
| `[PreviewPack]` archive presence | verified | local archive payload parse | full RGB decode/baked red components deferred |
| `[Header]` / live overlay eligibility | verified | local archive payload parse plus `0x00689D30` / `0x00640710` | runtime screenshot validation deferred |
| Rust source parity | verified gap | `src/app_list_maps.rs` loose-file scan; prior Rust status reports | implementation plan/code change |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Do archive-contained stock maps change the live overlay picture beyond 54 root files? -> Yes. `MISSIONSMD.PKT` resolves 161 local archive maps and all 161 are live-overlay eligible.` (evidence: local PKT/MIX census; `0x00699980`; `0x00640710`)
- `[RESOLVED] OQ-2 - Are archive YR stock maps PreviewPack-backed? -> Yes for this local source list: 161/161 resolved `MISSIONSMD.PKT` maps have `[PreviewPack]`.` (evidence: local `langmd.mix` PKT and `multimd.mix` / `expandmd01.mix` payload parse)
- `[RESOLVED] OQ-3 - Do archive YR stock maps carry `[Header] NumberStartingPoints`? -> Yes: 161/161 resolved `MISSIONSMD.PKT` maps have eligible values in `1..8`.` (evidence: local payload parse; `0x00689D30`)
- `[RESOLVED] OQ-4 - Do base RA2 archive maps behave like loose RA2 `.mmx` files for live overlays? -> Yes for 87 locally resolved `MISSIONS.PKT` maps: all have `[PreviewPack]`, none have `[Header]`, so overlays skip.` (evidence: local `multi.mix` payload parse; `0x00640710`)
- `[RESOLVED] OQ-5 - Are raw campaign archives relevant to this offline Skirmish stock-map claim? -> No. `MAPS01.MIX`, `MAPS02.MIX`, and `mapsmd03.mix` contain campaign/script map payloads and are not the PKT-driven Skirmish list surface traced at `0x00699980`.` (evidence: source path report; local raw scan)
- `[DEFERRED] OQ-6 - Why do 50 `MISSIONS.PKT` cooperative-style names not resolve locally?` (category: out-of-scope; reason: base RA2 cooperative payload completeness is not needed to answer YR Skirmish header/live-overlay delta; next-step-if-pursued: dedicated base RA2 PKT asset-resolution audit)
- `[DEFERRED] OQ-7 - Do all 161 archive previews contain baked red `4x4` pixels?` (category: bounded-cost-too-high; reason: user scope asked header/live-overlay census and said not to re-decode every preview unless needed; next-step-if-pursued: run the root-report PreviewPack red-component decoder across `MISSIONSMD.PKT`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Offline Skirmish map records are PKT/MIX-backed, not just loose root files. | `0x00699980`; `MISSIONSMD.PKT` 161 resolved maps | missing | `src/app_list_maps.rs`, future map-record loader | Load `MISSIONSMD.PKT` `MultiMaps` records and resolve `.map` payloads from `multimd.mix` / `expandmd01.mix` before/with loose maps. | Choose Map shows stock YR maps that are not loose root files, e.g. `XMP29U2.map`. | Do not treat the 54 loose files as the full retail stock list. |
| Empty map `GameModes` accepts selected mode string `standard`. | `0x0069AE10` | missing | map-list filtering | Records with no `GameModes` must be visible in Battle/standard mode. | Standard Battle list includes resolved `MISSIONSMD.PKT` maps with empty GameModes. | Do not reject empty `GameModes` as "no modes". |
| Archive YR stock maps use live STARTBUT overlays. | `0x00689D30`; `0x00640710`; 161/161 HeaderN in `1..8` | likely missing until archive maps and `[Header]` source bounds are wired | preview metadata / shell preview renderer | Parse `[Header] NumberStartingPoints` and `Waypoint1..8` from selected archive map payloads and gate overlays on `1..8`. | Selecting `XMP03T4.map` draws four live start markers/labels in addition to baked preview pixels. | Do not infer live overlays from `[Waypoints]`; use `[Header]`. |
| Base RA2 archive maps are baked-only for this gate. | 87 `MISSIONS.PKT` maps in `multi.mix`; no `[Header]`; `0x00640710` gate | unchecked | optional base stock-map support | If base `MISSIONS.PKT` is included, leave live overlays off for those no-header maps. | Selecting `MP02T2.map` shows decoded preview but no live STARTBUT overlays. | Do not synthesize overlays from gameplay waypoints. |

## Stale Docs / Follow-up Docs

- `SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md` should keep its root-file claim but replace the broad caveat with: "The 54 loose root files are not the full stock Skirmish map universe. `MISSIONSMD.PKT` adds 161 archive-backed YR map records; all 161 have eligible `[Header] NumberStartingPoints` and exercise live `STARTBUT.SHP` overlays."
- Any implementation plan based on `src/app_list_maps.rs` root scanning should be updated to add PKT/MIX source loading before using the stock-map census as parity coverage.

## Sources

- Ghidra read-only decompile: `DrawStartPositions @ 0x00640710`.
- Ghidra read-only decompile: `[Header]` loader `FUN_00689D30 @ 0x00689D30`.
- Ghidra-backed prior report: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md` for `0x00699980`, `0x005D63E0`, `0x0069AE10`, and source strings.
- Ghidra-backed prior report: `SKIRMISH_RETAIL_STOCK_MAP_PREVIEW_CENSUS_GHIDRA_REPORT.md` for root-file baseline and PreviewPack gate `0x00641B00`.
- Local retail data: `<ra2-install>/langmd.mix`, `multimd.mix`, `expandmd01.mix`, `ra2.mix`, `MULTI.MIX`.
- Rust contrast: `src/app_list_maps.rs`.
