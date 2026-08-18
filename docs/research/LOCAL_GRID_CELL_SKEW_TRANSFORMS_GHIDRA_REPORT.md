# Local Grid Cell Skew Transforms - Ghidra Research Report

**Address(es):** `0x005654A0`, `0x00565520`, `0x00565660`; caller context `0x004AA440`, `0x0050DB00`  
**Investigation Mode:** coverage-map, constrained follow-up  
**Claimed Scope:** local-grid <-> cell skew helpers, known YR caller contexts, Rust-facing parity implications  
**Non-Scope:** tactical screen/cursor inverse math, render world-to-screen math, broad pathfinding validation  
**Confidence:** Medium. Formula and xref claims are inherited from a high-confidence prior Ghidra report; this session had no Ghidra MCP tools exposed, so deeper caller-parameter verification is partial.  
**Active in YR:** Yes / Conditional. `0x004AA440` is active in YR paradrop edge-cell selection; `HouseClass::DetermineEdge` is active where house waypoint edge is computed. `0x00565660` is active through `FlyLocomotionClass::Process`, but that caller was not expanded in this slot.

## 1. Overview

The three helpers at `0x005654A0`, `0x00565520`, and `0x00565660` are integer local-grid/cell-diamond skew transforms. They are not tactical screen, client-pixel, lepton, or mouse-pick transforms.

The player-visible risk is not cursor math. It is edge-cell selection: paradrop carrier spawn/exit cells, and any house edge logic that derives a player's map edge from an anchor, must use the same LocalSize-relative skewed playfield model or aircraft can enter/leave from visibly wrong cells on maps with non-zero `LocalSize` origins.

## 2. Class Layout / Key Offsets

| Field | Offset | Unit | Purpose | Active in YR |
|---|---:|---|---|---|
| Map full width | `MapClass+0xF4` | cells | parity/base width term `W` in local-grid skew formulas | Yes; evidence `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`, lines 40-60 |
| LocalSize left | `MapClass+0xFC` | local-grid cells | origin offset `L` added/subtracted by transforms | Yes; evidence `0x004AA440` prior Ghidra report and map parser `src/map/map_file.rs:340-360` |
| LocalSize top | `MapClass+0x100` | local-grid cells | origin offset `T` added/subtracted by transforms | Yes; evidence `0x004AA440` prior Ghidra report and map parser `src/map/map_file.rs:340-360` |
| LocalSize width | `MapClass+0x104` | local-grid cells | caller-side playable rectangle width for edge/search bounds | Yes; evidence `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:348-352` |
| LocalSize height | `MapClass+0x108` | local-grid cells | caller-side playable rectangle height for edge/search bounds | Yes; evidence `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:348-352` |

## 3. Core Logic

All formulas are integer cell-space transforms. `W = MapClass+0xF4`, `L = MapClass+0xFC`, `T = MapClass+0x100`.

`0x005654A0` local index -> cell:

```text
t1 = local_x + L
t2 = local_y + T
cell_x = ((t2 + 1) >> 1) + t1
cell_y = W + (t2 >> 1) - t1
```

Active in YR: Yes. Evidence: prior high-confidence Ghidra re-read in `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:43-51`; xrefs include `0x004AA440` and `HouseClass::DetermineEdge`.

`0x00565520` cell -> local index:

```text
parity = W & 1
diff = cell_x - cell_y + parity
local_x = (diff >> 1) + (W / 2) - L
local_y = cell_x + cell_y - W - T
```

Active in YR: Yes. Evidence: prior high-confidence Ghidra re-read in `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:52-59`; xref is `0x004AA440`.

`0x00565660` packed cell -> packed local index:

```text
same inverse as 0x00565520, returned as CONCAT22(short_y, short_x)
```

Active in YR: Yes, but caller scope only partially expanded here. Evidence: prior xref scan says the only xref is `FlyLocomotionClass::Process`, `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:28,60-61`.

Important tiny details:

- The inverse includes `W & 1`; odd-width maps differ by one before the signed shift. Active in YR: Yes; evidence `0x00565520` formula in prior report.
- `local_y` is not halved; it is the raw sum `cell_x + cell_y - W - T`. Active in YR: Yes; evidence `0x00565520` formula.
- `0x005654A0` uses `(t2 + 1) >> 1` for `cell_x` but `t2 >> 1` for `cell_y`; the half-cell parity asymmetry is intentional. Active in YR: Yes; evidence `0x005654A0` formula.
- `LocalSize` left/top are transform origins; LocalSize width/height are not part of the helper arithmetic but bound the callers' local index loops. Active in YR: Yes; evidence transform formulas plus `0x004AA440` LocalSize field list.

## 4. INI Keys

| INI source | Key | Type | Default / source | Effect | Active in YR |
|---|---|---|---|---|---|
| Map INI `[Map]` | `Size=left,top,width,height` | int tuple | map-specific | Rust stores width/height as `MapHeader.width/height`; gamemd helper uses full width `W` | Yes; evidence `src/map/map_file.rs:327-356` |
| Map INI `[Map]` | `LocalSize=left,top,width,height` | int tuple | map-specific | Defines playable local rectangle; left/top enter transforms and width/height bound edge search | Yes; evidence `src/map/map_file.rs:340-360`, `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:348-352` |

No `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` key drives these formulas directly.

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| `0x004AA440` edge/placement search | Uses LocalSize playfield fields and local-grid <-> cell skew while finding a cell on a requested edge; mode `0/1/2/3 = N/E/S/W`, `-1` maps to `0` | Yes. Evidence: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:678-686`; paradrop callers at `0x0065E660`, `0x004155F0`, `0x004157C0` |
| South mode in `0x004AA440` | Builds up to 10 valid candidates, then chooses random when alternate cell is sentinel or closest to alternate when not sentinel | Yes. Evidence: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:688-693`; Rust analog partially mirrors this at `src/sim/world/edge_cell.rs:84-104` |
| `HouseClass::DetermineEdge` `0x0050DB00` | Chooses a house edge from anchor/object position against four map reference points and stores `House+0x577C` | Yes / Conditional on house setup. Evidence: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:627-635` |
| `0x00565660` `FlyLocomotionClass::Process` xref | Packed-cell inverse is used by fly locomotion process, but this slot did not expand exact branch/context | Yes, caller exists; exact condition deferred. Evidence: `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:28` |

## 6. Current Rust Implementation Status

Rust parses the map fields needed to build an equivalent helper:

- `src/map/map_file.rs:105-119` stores full map width/height and LocalSize left/top/width/height.
- `src/map/map_file.rs:340-360` parses `[Map] LocalSize=left,top,width,height`.

Rust does not appear to have a local-grid skew helper matching `0x005654A0`, `0x00565520`, or `0x00565660`.

Current related Rust behavior:

- `src/sim/world/edge_cell.rs:43-55` exposes `find_passable_at_edge`, used by paradrop spawn/exit.
- `src/sim/world/edge_cell.rs:65-80` scans north/east/west as simple full-grid rectangle edges and then chooses closest to target.
- `src/sim/world/edge_cell.rs:84-104` implements the south candidate-list closest-to-target branch, but still uses full rectangular `map_width/map_height`, not a LocalSize skew helper.
- `src/sim/superweapon/paradrop.rs:68-88` uses `sim.fog.width/height` for launch edge-cell selection.
- `src/sim/aircraft/paradrop_mission.rs:205-231` uses `sim.fog.width/height` and adds a Rust-only fallback to South/corner after opposite-edge failure.
- `src/sim/house_state.rs:128-153` implements a closest-edge approximation using map width/height reference points, not the LocalSize skew helpers.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005654A0` formula | verified-from-prior-report | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:43-51` | Fresh Ghidra re-read unavailable in this slot |
| `0x00565520` formula | verified-from-prior-report | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:52-59` | Fresh Ghidra re-read unavailable in this slot |
| `0x00565660` packed inverse | touched-not-exhausted | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:28,60-61` | Exact `FlyLocomotionClass::Process` branch/context |
| `0x004AA440` paradrop caller matrix | touched-not-exhausted | `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:205,257,278,678-686` | Exact current/alternate/fallback cell parameter matrix needs fresh decompile |
| `HouseClass::DetermineEdge` | touched-not-exhausted | `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:627-635` | Whether it calls `0x005654A0` in all four edge-distance probes needs fresh decompile |
| Rust local-grid skew helper | verified missing by text scan | `rg local_idx/Cell_to_LocalIndex/local_to_cell src`; no matching helper found | Future implementation location |
| Rust edge-cell search | verified partial analog | `src/sim/world/edge_cell.rs:43-104` | Needs LocalSize skew parity review |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Are these tactical screen/cursor transforms? -> No; they are local-grid/cell integer skews.` (evidence: `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:18-36`)
- `[RESOLVED] OQ-2 - What formulas should Rust preserve? -> The formulas in Section 3, including odd-width parity and asymmetric half-shift.` (evidence: `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:43-61`)
- `[RESOLVED] OQ-3 - Which map fields are involved? -> full width `+0xF4`, LocalSize left/top `+0xFC/+0x100`, and caller bounds LocalSize width/height `+0x104/+0x108`.` (evidence: `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:40-41`, `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:348-352`)
- `[RESOLVED] OQ-4 - Is `0x004AA440` active in stock YR? -> Yes, paradrop launch and overfly call it for carrier edge cells.` (evidence: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md:205,257,278,409-418`)
- `[RESOLVED] OQ-5 - Does Rust have matching parsed inputs? -> Yes, `MapHeader` stores Size and LocalSize fields.` (evidence: `src/map/map_file.rs:105-119,340-360`)
- `[RESOLVED] OQ-6 - Does Rust have a matching skew helper? -> No matching local-index/cell helper was found by text scan.` (evidence: `rg local_idx|Cell_to_LocalIndex|local_to_cell src`)
- `[DEFERRED] OQ-7 - Exact `0x004AA440` parameter matrix for all callers` (category: `needs-runtime-debugger`; reason: no Ghidra MCP/decompiler tools exposed in this slot; next-step-if-pursued: re-decompile `0x004AA440` and its active paradrop call sites)
- `[DEFERRED] OQ-8 - Exact condition for `0x00565660` inside `FlyLocomotionClass::Process`` (category: `needs-runtime-debugger`; reason: only xref identity available from prior report; next-step-if-pursued: decompile `FlyLocomotionClass::Process` around the call)
- `[DEFERRED] OQ-9 - Whether `HouseClass::DetermineEdge` matters for AI beyond paradrop edge selection` (category: `requires-different-system-context`; reason: this slot scoped only transform and edge helper usage; next-step-if-pursued: trace all reads of `House+0x577C`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Local-index -> cell uses LocalSize left/top and full map width with asymmetric half-shifts | `0x005654A0`, prior report lines 43-51 | missing helper | `src/map` or `src/sim/world/edge_cell.rs` support surface | Provide a deterministic integer helper for LocalSize-relative edge iteration without render dependencies | On a map with `Size=0,0,100,100` and `LocalSize=2,8,65,62`, converting local indices on each edge returns gamemd-equivalent cells | Do not use `terrain::iso_to_screen`; proposed test `test_local_index_to_cell_matches_gamemd_local_size_parity` |
| Cell -> local-index inverse includes `W & 1` parity before halving | `0x00565520`, prior report lines 52-59 | missing helper/tests | `src/map` or `src/sim/world/edge_cell.rs` support surface | Round-trip cells through skew inverse for odd and even full map widths | Odd-width map edge cells round-trip to the expected local index rather than drifting one column | Ignoring odd-width parity causes off-by-one edge choice; proposed test `test_cell_to_local_index_odd_width_parity_matches_gamemd` |
| Active paradrop edge search should operate on the LocalSize skewed playfield, while Rust currently scans `sim.fog.width/height` rectangular edges | `0x004AA440` prior caller docs; `src/sim/world/edge_cell.rs:65-104`, `src/sim/superweapon/paradrop.rs:80-87` | mismatch/unchecked parity | `src/sim/world/edge_cell.rs`, `src/sim/superweapon/paradrop.rs`, `src/sim/aircraft/paradrop_mission.rs` | Future edge search should consume parsed map LocalSize and gamemd's local-grid skew before passability checks | Paradrop on a map with non-zero LocalSize left/top spawns/exits at the same skewed playable edge cell as gamemd, not the full fog-grid border | Do not substitute full rectangular map borders; proposed test `test_paradrop_edge_search_uses_local_size_skewed_playfield` |

### Negative Facts / Do Not Do

- Do not use these helpers for tactical screen-pixel or cursor-to-cell conversion. Evidence: prior xrefs exclude tactical render/pick code, `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:25-36`.
- Do not drop the `W & 1` term in the inverse. Evidence: `0x00565520` formula in prior report includes full-width parity.
- Do not treat LocalSize width/height as terms inside the skew formula. Evidence: `0x005654A0/0x00565520` formulas use `W`, `L`, and `T`; width/height bound caller loops.
- Do not scan full rectangular map borders as a proven equivalent to `0x004AA440`. Evidence: gamemd uses LocalSize fields at `MapClass+0xFC/+0x100/+0x104/+0x108`; Rust edge scan currently uses `map_width/map_height`.
- Do not assume `0x00565660` is unused because placement search uses `0x00565520`. Evidence: prior xref scan found `FlyLocomotionClass::Process`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`: replace `Active in YR: Yes - these transforms run on every tick, frame, click, and cursor move.` with `Active in YR: Mixed by transform family. Tactical world/screen/cell transforms run in render/input paths; `0x005654A0`, `0x00565520`, and `0x00565660` are active local-grid/cell helpers used by edge/search/fly-locomotion contexts, not every tick/frame/click/cursor path.`
- `C:/Users/enok/Documents/ra2-rust-game-docs/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`: replace `they are an internal helper for edge iteration` with `they are internal LocalSize/local-grid helpers for edge/search contexts; `0x00565660` also has a `FlyLocomotionClass::Process` caller and must not be described as edge-only.`

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/map/map_file.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/world/edge_cell.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/superweapon/paradrop.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/aircraft/paradrop_mission.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/sim/house_state.rs`
