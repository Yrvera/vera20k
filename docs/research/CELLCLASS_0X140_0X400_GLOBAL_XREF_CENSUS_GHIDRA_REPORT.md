# CellClass+0x140 Bit 0x400 Global Xref Census - Ghidra Research Report

**Address(es):** `0x0047C620`, `0x0047E040`, `0x0047E470`, `0x00570050`, `0x00571050`, `0x00573540`, `0x00574000`, `0x00574C20`, `0x00576770`, `0x00703CC0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** whole-binary read-only census for instruction forms that read or write `CellClass+0x140` bit `0x400`, excluding re-investigation of already documented bridge writer, building-placement reader, bridge-destruction fallback, Ion Cannon impact-Z reader, and checked A* `0x40000` functions except where needed to resolve this census.  
**Non-Scope:** full bridge state machine, full `TooBigToFitUnderBridge` movement/render system, full superweapon impact-Z threading, and any non-`CellClass+0x140` use of immediate `0x400`.  
**Confidence:** High for the `0x400`/`0x40000` distinction and live additional reader list; Medium-High for "no additional writer" because map-load flag-copy code is broad and was classified by existing bridge docs rather than re-drained here.  
**Active in YR:** Yes for the live readers/writers listed below; false-positive candidate instructions are not `CellClass` flag consumers.

## 0. Investigation Contract

### Target Question

Does retail `gamemd.exe` contain any hidden A*/movement/pathgrid reader of `CellClass+0x140 & 0x400`, or any additional live reader/writer beyond the documented bridge writer, placement reader, bridge-destruction fallback, Ion Cannon impact-Z reader, and checked A* `0x40000` code?

### Non-Goals

- Do not rename, label, comment, or otherwise mutate the Ghidra database.
- Do not implement Rust.
- Do not re-document all bridge damage behavior unless needed to classify a `0x400` xref.
- Do not conflate `0x400` with `0x40000`.

### Evidence Needed To Mark COMPLETE

- Byte-pattern census for direct memory tests and compiler register forms for `CellClass+0x140` bit `0x400`.
- Decompile plus assembly/listing for each live candidate that survived the census.
- Explicit negative check against `PathfinderClass__UpdateBridgePassability` / A* `0x40000` sites.
- Active in YR classification for each material finding.
- Rust-facing implementation handoff with concrete acceptance test names.

### Stop Conditions

- Stop after all `CellClass+0x140 & 0x400` candidates are classified as known, additional live reader, known writer, or false positive.
- Stop after one zero-add pass over the candidate families adds no new live `0x400` reader/writer.
- Stop without expanding into full bridge rendering/movement mechanics.

## 1. Overview

The global census found no hidden A*/movement/pathgrid reader of `CellClass+0x140 & 0x400`. The live A* bridge-cost machinery uses `0x40000`, not `0x400`, and `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` reads `0x100` for bridge layer selection while writing/toggling `0x40000`.

The only additional non-placement, non-bridge-destruction live reader found is `FUN_00703CC0`, an orientation-sensitive Techno/Unit bridge-edge predicate used by `FUN_00703E70` and Unit draw/height paths. It reads `0x400` on the current cell and four neighbor cells, paired with `0x800` direction checks, to decide whether an object is at/near a non-structural bridge marker for rendering/height-fudge logic. It is not A*, movement legality, or persistent pathgrid state.

## 2. Census Method

Read-only Ghidra searches used these machine-code forms:

| Pattern family | Purpose | Result |
|---|---|---|
| `F7 81 40 01 00 00 00 04 00 00` and sibling ModRM forms | direct `test dword ptr [reg+0x140],0x400` | two live direct memory tests, both bridge-destruction hut fallback twins at `0x005742E4`, `0x00574F00` |
| `8B ?? 40 01 00 00` followed by `TEST AH/BH/CH/DH,0x04` or `TEST reg,0x400` | compiler form: load `Cell+0x140` then test bit `0x400` | placement, bridge-destruction/update, and `0x00703CC0` render-edge predicate |
| `F6 C4/C5/C6/C7 04`, `F7 C? 00 04 00 00` | high-byte/register tests of a loaded flags dword | live `CellClass` users plus audited false positives in UI/trigger/streaming code |
| `Cell+0x140` writes (`MOV [reg+0x140],reg`) around known bridge writers and map-load clusters | writer census | no additional semantic writer of bit `0x400` beyond SetBridgeDirection family and already documented map-load copy/clear path |
| `0x40000` forms in `0x0042ACF0` / A* docs | distinguish A* cost marker from this bit | verified separate bit, not a `0x400` reader |

Representative rejected false positives: `FUN_004A5EB0` tests a text/blitter flag `param_8 & 0x400`; `FUN_005E3010` tests UI/settings flags; `FUN_007BEA80`, `FUN_007C2D00`, and `FUN_007C3750` test streaming/event flags. These do not load from `CellClass+0x140`.

## 3. Class Layout / Key Offsets

| Offset / bit | Meaning in this census | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x140 & 0x100` | structural bridge/on-bridge flag used by bridge predicates and pathfinder layer choice | `0x00703B10`, `0x0042AD93` | Yes |
| `CellClass+0x140 & 0x400` | bridge marker / placement blocker / bridge-edge marker; set on collapsed/destroyed bridge direction paths and read by placement, bridge fallback, and render-edge predicate | `0x0047C620`, `0x0047E040`, `0x00570050`, `0x00703CC0` | Yes |
| `CellClass+0x140 & 0x800` | bridge orientation/direction bit paired with `0x400` in fallback/render probes | `0x00570050`, `0x00703CC0` | Yes |
| `CellClass+0x140 & 0x40000` | transient A* bridge-approach cost marker, not this census target | `0x0042ACF0`, `AStar_compute_edge_cost @ 0x00429830` prior docs | Yes, search-scoped |

## 4. Live Reader / Writer Census

| Function / site | Read / write form | Semantics | Additional beyond parent context? | Active in YR |
|---|---|---|---|---|
| `CellClass__SetBridgeDirection_NESW @ 0x0047E040` | writes `Cell+0x140`; assembly stores at `0x0047E0F0`, `0x0047E1CB`, `0x0047E295`, `0x0047E3CC`, `0x0047E452` | `param_3==0` sets `0x400` and calls `BlowUpBridge`; `param_3!=0` clears/replaces it while setting structural bridge bits | No, documented writer | Yes, called by bridge construction/damage/repair paths |
| `CellClass__SetBridgeDirection_NWSE @ 0x0047E470` | byte-identical writer family | same as above; orientation name is caller convention | No, documented writer | Yes |
| `Cell_passability_building_placement @ 0x0047C620` | load `[EDI+0x140]`, `TEST AH,0x04` at `0x0047C984`, `0x0047C9EA` | rejects placement on cells carrying `0x400`; one branch also rejects `0x100` via shifted/tested flags | No, documented placement reader | Yes, ordinary building placement/deploy validation |
| `ProcessBridgeDestruction_Low @ 0x00570050` | decompile reads `(flags & 0x500)`, `(flags & 0x400)`, `(flags & 0x800)`; assembly candidate `0x00570264`, `0x00570304` | low-bridge destruction/repair fallback: if overlay scan misses, finds a `0x100|0x400` anchor nearby; in `0x400`-only branch, walks perpendicular through up to four marked cells to resolve anchor/direction | Same bridge-fallback family; exact live sibling if prior doc only named hut-death bodies | Yes, bridge damage/repair paths |
| `ProcessBridgeDestruction_High @ 0x00573540` | same read family; assembly candidate `0x0057374A`, `0x005737F2` | high-bridge twin of low fallback using high bridge tile base | Same bridge-fallback family | Yes |
| `MapClass__UpdateAdjacentBridges @ 0x00571050` | reads `(flags & 0x500)`, `(flags & 0x400)`, `(flags & 0x800)`; assembly candidates `0x005710EA`, `0x0057118F` | low bridge adjacent update uses the same anchor resolution grammar, then scans forward for edge tiles to update/dirty | Same bridge-fallback family; additional exact reader site | Yes |
| `MapClass__UpdateAdjacentBridges_High @ 0x00576770` | same read family; assembly candidates `0x00576808`, `0x005768AD` | high bridge adjacent update twin | Same bridge-fallback family; additional exact reader site | Yes |
| `MapClass__DestroyBridge_High_OnHutDeath @ 0x00574000` | direct `TEST dword ptr [ECX+0x140],0x400` at `0x005742E4`; register tests around `0x00574241` | bridge-hut destruction fallback; scans 5x5 overlay first, then fallback marker path | No, documented bridge destruction fallback | Yes, callers `BombClass::Detonate` and `BuildingClass::Update` per prior docs |
| `MapClass__DestroyBridge_Low_OnHutDeath @ 0x00574C20` | direct `TEST dword ptr [ECX+0x140],0x400` at `0x00574F00`; register tests around `0x00574E5D` | low bridge-hut destruction fallback twin | No, documented bridge destruction fallback | Yes |
| `FUN_00703CC0 @ 0x00703CC0` | current cell `MOV ECX,[ESI+0x140]; TEST CH,0x04`; neighbors use `TEST reg,0x400` and `TEST highbyte,0x04` at `0x00703DF2`, `0x00703E01`, `0x00703E1B`, `0x00703E2F`, `0x00703E43` | orientation-sensitive bridge-edge predicate for Techno/Unit rendering/height-fudge. Current cell with `0x400` returns true; selected neighbors require `0x400` plus `0x800` match/mismatch by side. | Yes, this is the new live reader outside the parent-known movement/placement/destruction set | Yes, called by `FUN_00703E70`; callers include `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`, `FUN_004DAFF0`, `FUN_0073CE0D` |
| `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` | no `0x400` read; reads `0x100` at `0x0042AD93`, writes/toggles `0x40000` at `0x0042AFA5`, `0x0042B035`, `0x0042B05B` | A* bridge approach cost overlay only | Negative proof site | Yes, but for `0x40000`, not `0x400` |

## 5. New Reader Semantics: `FUN_00703CC0`

`FUN_00703CC0` is structurally parallel to `TechnoClass__IsOnBridge_ForFiring @ 0x00703B10`, but substitutes bit `0x400` where `0x00703B10` checks `0x100`.

Binary behavior:

- It asks the object for its map coordinate through vtable `+0x1B8`, then gets the current cell and four direction-table neighbor cells.
- It returns false if the current cell lookup fails or if `Techno+0x8C` is nonzero.
- It returns true if the current cell has `Cell+0x140 & 0x400`.
- It returns true for selected neighboring cells only when `0x400` is paired with an orientation test on `0x800`:
  - neighbor from `DAT_0089F698`: `0x400 && 0x800 != 0`;
  - neighbor from `g_refinery_unload_adjacent_lookup_dx`: `0x400 && 0x800 == 0`;
  - neighbor from `DAT_0089F690`: `0x400 && 0x800 == 0`;
  - neighbor from `g_DirectionOffsets`: `0x400 && 0x800 != 0`.
- It returns true at `0x00703E53`, otherwise false at `0x00703E5D`.

Evidence: decompile and disassembly of `0x00703CC0`; caller `0x00703E70`; `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`.

Active in YR: Yes. `UnitClass__Draw_Sprite_With_BridgeFudge` is a Unit draw path. `TooBigToFitUnderBridge` is parsed on UnitType and many stock YR units set it, although this specific reader is render/height-fudge adjacency logic, not A* or movement legality.

## 6. A* / Movement Negative Proof

No hidden A*/movement/pathgrid consumer of `CellClass+0x140 & 0x400` was found.

The key checked path is `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0`:

- `0x0042AD93`: reads `Cell+0x140` and tests `AH,0x01`, i.e. bit `0x100`, to choose object-list/layer context.
- `0x0042AFA5`, `0x0042B035`, `0x0042B05B`: masks `0x40000`, not `0x400`, and writes back to `Cell+0x140`.
- Existing A* reports identify `AStar_compute_edge_cost @ 0x00429830` as the `0x40000` cost consumer.

Active in YR: Yes, but it is search-scoped `0x40000` behavior. It is not a reader of `0x400`.

## 7. INI Keys

No INI key directly controls `CellClass+0x140 & 0x400`. Related active gates:

| Key / section | Retail YR value / role | Effect on this census | Active in YR |
|---|---|---|---|
| `[General] DestroyableBridges` / map `BridgeDestruction` | enabled by default in YR rules/map behavior | determines whether bridge damage paths matter, but not a `0x400` reader/writer itself | Conditional; default enabled for standard destroyable bridge content |
| `TooBigToFitUnderBridge=` on UnitTypes | many stock units set true | makes draw/bridge-fudge paths using `0x00703E70` relevant for those units | Conditional by unit type |
| Building placement data (`Foundation`, `WaterBound`, `PlaceAnywhere`) | object-type placement inputs | reaches `Cell_passability_building_placement` where `0x400` rejects cells | Yes for standard placement |

## 8. Current Rust Implementation Status

| Rust surface | Status for this census | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs::PathGrid` | no persistent `0x400` A* marker should be added; existing path grid stores walkability/bridge metadata, not CellClass bit parity | Codegraph context; file scan |
| `src/app_sim_tick.rs::rebuild_dynamic_path_grid` | bridge state rebuild surface; should not treat `0x400` as A* cost bit | parent context and Rust scan |
| `src/sim/bridge_state` | correct home for bridge damage/repair state; exact `0x400` marker may only matter if state-to-placement/render parity needs it | parent context and Rust scan |
| `src/sim/production/production_placement.rs` | placement should reject destroyed/marker bridge cells equivalent to binary `0x0047C620`; exact parity unchecked here | file scan |
| render helpers / unit bridge visual surfaces | likely missing or unchecked for `0x00703CC0` non-structural bridge-edge predicate | codegraph and render helper scan |

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question / non-goals / evidence / stop conditions | verified | report section 0 | none |
| Direct memory `0x400` tests | verified | pattern hits `0x005742E4`, `0x00574F00`; both decompiled in bridge hut fallback twins | none |
| Register/high-byte `0x400` tests | verified | `F6 C4/C5/C6/C7 04`, `F7 C? 00 04 00 00` searches; decompile/disassembly of surviving candidates | none |
| Building placement reader | verified-known | `0x0047C620`, assembly around `0x0047C970..0x0047C9EA` | exact Rust placement parity remains implementation work |
| Bridge destruction/update fallback readers | verified | `0x00570050`, `0x00571050`, `0x00573540`, `0x00574000`, `0x00574C20`, `0x00576770` | no further semantic expansion in this report |
| `FUN_00703CC0` reader | verified | decompile/disassembly and callers | exact Rust render parity unchecked |
| `PathfinderClass__UpdateBridgePassability` distinction | verified | decompile/disassembly `0x0042ACF0`; existing A* docs | none |
| Writer census | verified for known writers; touched-not-exhausted for broad map-load copy | `0x0047E040`, `0x0047E470`; existing bridge docs for map-load copy/clear | a future map-load flag-copy audit could rename every upstream producer, but no hidden pathgrid writer is indicated |
| False-positive `0x400` users | verified by sample decompiles | `0x004A5EB0`, `0x005E3010`, `0x007BEA80`, `0x007C2D00`, `0x007C3750` | none material |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this an exhaustive slice or coverage map? -> Exhaustive slice for whole-binary `CellClass+0x140 & 0x400` read/write forms visible through byte patterns and Ghidra-decompiled candidates.` (evidence: census method table)
- `[RESOLVED] OQ-2 - Is there a hidden A*/movement/pathgrid reader of `0x400`? -> No. The checked pathfinder code reads `0x100` and toggles/consumes `0x40000`, not `0x400`.` (evidence: `0x0042ACF0`, `0x00429830` prior docs)
- `[RESOLVED] OQ-3 - Are the known SetBridgeDirection writers still the only semantic `0x400` writers? -> Yes for live semantic writers found by this census; map-load flag copy/clear remains the already documented loader path, not a new pathgrid writer.` (evidence: `0x0047E040`, `0x0047E470`, prior bridge docs)
- `[RESOLVED] OQ-4 - Is building placement still a live reader? -> Yes; `0x0047C620` rejects placement when loaded `Cell+0x140` has bit `0x400`.` (evidence: `0x0047C620`, `0x0047C984`, `0x0047C9EA`)
- `[RESOLVED] OQ-5 - Which exact bridge-destruction/update siblings read `0x400`? -> `0x00570050`, `0x00571050`, `0x00573540`, `0x00574000`, `0x00574C20`, `0x00576770`.` (evidence: decompile and assembly candidates listed above)
- `[RESOLVED] OQ-6 - Is there an additional live non-destruction/non-placement reader? -> Yes, `FUN_00703CC0`, called from `FUN_00703E70` and Unit draw/height paths.` (evidence: `0x00703CC0`, callers `0x0073B140`, `0x004DAFF0`, `0x0073CE0D`)
- `[RESOLVED] OQ-7 - Is `FUN_00703CC0` movement legality? -> No; caller evidence places it in rendering/height-fudge bridge-edge logic, not A* or pathgrid mutation.` (evidence: `UnitClass__Draw_Sprite_With_BridgeFudge @ 0x0073B140`)
- `[RESOLVED] OQ-8 - Does an INI key directly define `0x400`? -> No direct key found; related gates are bridge destruction, placement data, and `TooBigToFitUnderBridge` render path activity.` (evidence: INI grep for bridge/placement/TooBig)
- `[RESOLVED] OQ-9 - Null/out-of-bounds cell behavior for readers? -> Bridge fallback readers use `MapClass__Get_CellClass`/cell-array checks and fall back to `DAT_00ABDC50`; `FUN_00703CC0` null-checks neighbor cells before reading them except current cell after initial non-null test.` (evidence: `0x00570050`, `0x00703CC0`)
- `[RESOLVED] OQ-10 - Does paused/replay/save restore change this bit's meaning? -> No scoped evidence of alternate meaning; save/load/map-load broad bit-copy is out of semantic scope and not a reader.` (evidence: no candidate reader/writer outside listed live functions)
- `[DEFERRED] OQ-11 - Exact upstream map-file source of every initial `Cell+0x140` bit including `0x400`.` (category: out-of-scope; reason: this census is xref/use semantics, not a full map parser flag-origin audit; next-step-if-pursued: investigate `FUN_00565C10` upstream temporary buffer population)
- `[DEFERRED] OQ-12 - Full `TooBigToFitUnderBridge` movement-vs-render reconciliation.` (category: out-of-scope; reason: only the `0x00703CC0` reader was in scope; next-step-if-pursued: verify `TOO_BIG_TO_FIT_UNDER_BRIDGE_GHIDRA_REPORT.md` and locomotor reports as a separate doc audit)

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Cell+0x140 & 0x400` is not an A* cost/pathgrid bit; A* uses `0x40000` only. Active in YR: Yes for A* `0x40000`, No for `0x400` as A* input. | `0x0042ACF0` decompile/disassembly; `AStar_compute_edge_cost @ 0x00429830` prior docs | none observed; future `0x40000` overlay still belongs per-search | `src/sim/pathfinding/core.rs`, future cost-overlay surface | keep `0x400` out of persistent `PathGrid` cost logic; model `0x40000` separately if implemented | Destroyed/marker bridge cells do not gain a 4x A* cost unless the search-scoped `0x40000` overlay is active. Proposed test name: `astar_does_not_treat_bridge_0x400_marker_as_cost_overlay` | Do not conflate `0x400` with `0x40000`; do not persist transient A* marks in `PathGrid` |
| Building placement rejects cells carrying `Cell+0x140 & 0x400`. Active in YR: Yes. | `Cell_passability_building_placement @ 0x0047C620`, `TEST AH,0x04` at `0x0047C984` and `0x0047C9EA` | unchecked for exact marker taxonomy | `src/sim/production/production_placement.rs`, deploy placement surfaces | ensure bridge marker/destroyed bridge cells cannot be used for ordinary building foundation placement | Place a ready building over a cell represented as destroyed/marker bridge and expect placement preview plus commit to fail. Proposed test name: `building_placement_rejects_bridge_0x400_marker_cell` | Do not use generic unit walkability alone as building placement truth |
| `FUN_00703CC0` reads `0x400` in a render/height-fudge bridge-edge predicate, not movement. Active in YR: Conditional on draw/height path and unit type/path context. | `0x00703CC0` decompile/disassembly; callers `0x00703E70`, `0x0073B140`, `0x004DAFF0`, `0x0073CE0D` | missing/unchecked | render unit bridge helpers, e.g. `src/app_instances/helpers.rs` and unit draw ordering surfaces | reproduce orientation-sensitive bridge-edge detection for units rendered near `0x400` bridge markers if visual parity requires it | A TooBig unit adjacent to a collapsed/destroyed bridge marker chooses the same split/fudge branch as retail. Proposed test name: `unit_bridge_edge_fudge_uses_0x400_marker_orientation` | Do not feed this predicate into A* or movement legality |

## 12. Negative Facts / Do Not Do

- Do not implement `CellClass+0x140 & 0x400` as an A* cost marker. The A* cost marker is `0x40000`.
- Do not add a persistent `PathGrid` flag for `0x400` just because bridge destruction uses it; known pathgrid-cost behavior is separate.
- Do not treat `FUN_00703CC0` as movement/pathfinding. Its callers are render/height-fudge paths.
- Do not treat every immediate `0x400` in the binary as `CellClass`; many hits are UI, blitter, trigger, or streaming flags.
- Do not remove placement rejection for `0x400` bridge-marker cells.

## 13. Remaining Uncertainty

- The exact upstream map-load source of each initial `Cell+0x140` bit remains outside this xref census. Existing docs indicate a broad bit-copy/clear path; this report did not re-drain that parser path.
- The exact Rust feature gap for `FUN_00703CC0` depends on a separate render parity audit. The binary reader is live, but this report does not prove whether the current Rust visual output already approximates it through other bridge-height logic.

## 14. Stale Docs / Follow-up Docs

No mandatory stale-doc patch is required from this census. Optional wording for future bridge/pathfinding docs:

> `CellClass+0x140 & 0x400` is not a hidden A* or PathGrid cost bit. A whole-binary xref census found it in building placement, bridge destruction/update fallback, SetBridgeDirection writer paths, and a Techno/Unit render-height bridge-edge predicate at `0x00703CC0`. A* uses `0x40000`, not `0x400`.

## Sources

- Ghidra read-only decompiled/disassembled: `0x0042ACF0`, `0x0047C620`, `0x0047E040`, `0x0047E470`, `0x00570050`, `0x00571050`, `0x00573540`, `0x00574000`, `0x00574C20`, `0x00576770`, `0x00703B10`, `0x00703CC0`, `0x00703E70`, `0x0073B140`, `0x004DAFF0`, `0x0073CE0D`.
- Ghidra read-only byte-pattern searches: `F7 81 40 01 00 00 00 04 00 00`; `8B 87 40 01 00 00`; `F6 C4/C5/C6/C7 04`; `F7 C5/C7 00 04 00 00`; `Cell+0x140` write forms.
- Docs referenced: `AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md`, `AI_BRIDGE_INTERACTION_GHIDRA_REPORT.md`, `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`, `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`, `TOO_BIG_TO_FIT_UNDER_BRIDGE_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/sim/pathfinding/core.rs`, `src/sim/bridge_state`, `src/app_sim_tick.rs`, `src/sim/production/production_placement.rs`, render bridge helpers.
