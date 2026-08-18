# g_DirectionOffsets 0x0089F688 Bridge Marker Path - Ghidra Research Report

**Address(es):** `0x0089F688` table, `0x0049F2F0` initializer, `0x0042ACF0` consumer  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** runtime values/source/population of `g_DirectionOffsets @ 0x0089F688` specifically as consumed by `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0`, including direction `8` tube handling through `CellClass+0x116` and `g_TubeArray[idx]+0x28`, plus Rust fixture implications for temporary pathgrid cost markers.  
**Non-Scope:** full A* behavior, full TubeClass lifecycle, walk/drive tube traversal timing, `CellClass+0x140` bits other than `0x40000`, `PathfinderClass+0x3C` caller value provenance, and object eligibility field `+0x678` semantics.  
**Confidence:** High for table values/source, constructor reachability, `0x0042ACF0` direction/tube semantics, and marker fixture implications; live debugger memory dump was unavailable.  
**Active in YR:** Yes. The initializer is reached before `WinMain`; `0x0042ACF0` is called by live A* under `PathfinderClass+0x3C != 0`.

## Target question

What exact runtime values populate `g_DirectionOffsets @ 0x0089F688`, where do they come from, and how does `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` consume them when replaying peer path queues for temporary `CellClass+0x140 & 0x40000` A* cost markers, including direction `8` tube jumps?

## Non-goals

- Do not re-investigate all bridge A* costs or cleanup tails.
- Do not implement Rust changes.
- Do not prove all possible TubeClass producer invariants.
- Do not rename or mutate Ghidra symbols.
- Do not resolve the original symbolic name of `CellClass+0x140 & 0x40000`.

## Evidence needed to mark COMPLETE

- `g_DirectionOffsets` initialized table values verified from binary code.
- Startup path proving the initializer runs before normal YR gameplay.
- `0x0042ACF0` instruction-level evidence for direction `0..7` and direction `8`.
- Bounds/edge behavior for `dir == 8`, including `Cell+0x116 == -1`.
- Cost-marker fixture implications stated against current Rust surfaces.

## Stop conditions

- Stop after resolving the scoped table source/value and the `0x0042ACF0` consumer.
- Stop if live debugger is required for a stronger post-startup overwrite proof; record it as remaining uncertainty.
- Stop before touching Rust, INI, in-repo docs, or unrelated reports.

## 1. Overview

`g_DirectionOffsets @ 0x0089F688` is a runtime-populated eight-neighbor table of signed `(dx, dy)` cell offsets. It is not map-, theater-, INI-, or pathfinder-derived. It is written by `Foundation_direction_table_init @ 0x0049F2F0`, reached from the CRT constructor table before `WinMain`.

`PathfinderClass__UpdateBridgePassability @ 0x0042ACF0` uses this table only for peer path entries `0..7`. Peer path entry `8` is not an adjacent direction; it is a TubeClass jump. In that branch, the function reads the current cell's `CellClass+0x116` tube index, reads `g_TubeArray[index]` from pointer-table base `0x008B413C`, loads `TubeClass+0x28`, and uses that packed cell coordinate as the next marker coordinate.

## 2. Key Values / Offsets

| Item | Value / offset | Meaning in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `0x0089F688` | dword `0xFFFF0000` | dir `0` = `(dx=0, dy=-1)` | Yes | `0x0049F2F0`, write `0x0049F305` |
| `0x0089F68C` | dword `0xFFFF0001` | dir `1` = `(dx=+1, dy=-1)` | Yes | `0x0049F2F0`, write `0x0049F322` |
| `0x0089F690` | dword `0x00000001` | dir `2` = `(dx=+1, dy=0)` | Yes | `0x0049F2F0`, write `0x0049F336` |
| `0x0089F694` | dword `0x00010001` | dir `3` = `(dx=+1, dy=+1)` | Yes | `0x0049F2F0`, write `0x0049F34A` |
| `0x0089F698` | dword `0x00010000` | dir `4` = `(dx=0, dy=+1)` | Yes | `0x0049F2F0`, write `0x0049F388` |
| `0x0089F69C` | dword `0x0001FFFF` | dir `5` = `(dx=-1, dy=+1)` | Yes | `0x0049F2F0`, write `0x0049F375` |
| `0x0089F6A0` | dword `0x0000FFFF` | dir `6` = `(dx=-1, dy=0)` | Yes | `0x0049F2F0`, write `0x0049F38E` |
| `0x0089F6A4` | dword `0xFFFFFFFF` | dir `7` = `(dx=-1, dy=-1)` | Yes | `0x0049F2F0`, write `0x0049F394` |
| `CellClass+0x116` | signed short | Tube index; `-1` is the only missing-tube guard in `0x0042ACF0` | Conditional on peer path entry `8` | `0x0042AF15..0x0042AF1F` |
| `0x008B413C` | pointer-table base | `g_TubeArray` pointer table used by `0x0042ACF0` | Conditional on valid tube index | `0x0042AF21..0x0042AF2A` |
| `TubeClass+0x28` | packed cell coord | Direction-8 next marker coordinate | Conditional on valid tube index | `0x0042AF2A` |
| `CellClass+0x140 & 0x40000` | bit `0x40000` | Temporary A* cost marker toggled on replayed destination cells | Yes | `0x0042AF8D..0x0042AFAE`; consumer `0x00429830` |

## 3. Initialization / Population

Verified binary finding: `Foundation_direction_table_init @ 0x0049F2F0` writes the eight table entries above. The function constructs dwords from signed 16-bit low/high words and writes them into BSS/runtime globals.

Evidence:

- `decompile_function 0x0049F2F0` shows direct writes to `g_DirectionOffsets`, `_DAT_0089f68c`, `DAT_0089f690`, `DAT_0089f694`, `DAT_0089f698`, `_DAT_0089f69c`, `g_refinery_unload_adjacent_lookup_dx`/`0x0089F6A0`, and `DAT_0089f6a4`.
- Assembly context:
  - `0x0049F305`: `MOV [0x0089f688], EAX` after stack dword `(0, -1)`.
  - `0x0049F322`: `MOV dword ptr [0x0089f68c], ESI`.
  - `0x0049F336`: `MOV dword ptr [0x0089f690], ESI`.
  - `0x0049F34A`: `MOV dword ptr [0x0089f694], ESI`.
  - `0x0049F375`: `MOV [0x0089f69c], EAX`.
  - `0x0049F388`: `MOV dword ptr [0x0089f698], ESI`.
  - `0x0049F38E`: `MOV dword ptr [0x0089f6a0], EDX`.
  - `0x0049F394`: `MOV [0x0089f6a4], EAX`.

Verified liveness: the initializer is registered in the constructor table and runs before `WinMain`.

Evidence:

- `get_function_xrefs 0x0049F2F0` returns data xref `0x00812BAC`.
- `inspect_memory_content 0x00812BAC length 4` reads bytes `F0 F2 49 00`, the little-endian pointer `0x0049F2F0`.
- `FUN_007CBDAF @ 0x007CBDAF` calls `FUN_007CBED3(&DAT_00812000, &DAT_00815DA4)`.
- `FUN_007CBED3 @ 0x007CBED3` iterates function pointers and calls each non-null entry.
- `entry @ 0x007CD80F` calls `FUN_007CBDAF()` before `WinMain(...)`.

Important nuance: direct static `read_memory 0x0089F688` in the Ghidra PE image returned zeros because this is runtime-populated storage. The verified runtime value is the constructor-written value above; live debugger memory could not be dumped because the debugger server was not running.

## 4. Consumer Logic in `0x0042ACF0`

### Direction `0..7`

Active in YR: Yes, when a scanned peer path queue contains direction entries `0..7`.

For each replayed peer path entry that is not `8`, `0x0042ACF0` treats the entry as an index into `g_DirectionOffsets`:

- `0x0042AF41`: read X short from `[EAX*4 + 0x89F688]`.
- `0x0042AF49`: read Y short from `[EAX*4 + 0x89F68A]`.
- `0x0042AF51..0x0042AF65`: add those signed shorts to the current packed cell coord and store the new packed coord.
- `0x0042AF69..0x0042AFAE`: resolve the new coord to `CellClass` and toggle `0x40000`.

The table order is exactly: N, NE, E, SE, S, SW, W, NW.

### Direction `8`

Active in YR: Conditional; only when a peer path queue entry is exactly `8` and the current path coordinate resolves to a cell with tube metadata.

Instruction evidence:

- `0x0042AEF6`: `CMP EBX,0x18`; replay stops at 24 entries.
- `0x0042AF01`: `CMP EAX,0x8`; direction `8` enters the tube branch.
- `0x0042AF10`: calls `MapClass::Get_CellClass` for the current replay coordinate.
- `0x0042AF15`: `MOVSX EAX, word ptr [EAX + 0x116]`; reads signed tube index.
- `0x0042AF1C..0x0042AF1F`: compares only against `-1`; if equal, branch to zero-coordinate fallback.
- `0x0042AF21`: loads pointer-table base from `[0x008B413C]`.
- `0x0042AF27`: loads `g_TubeArray[index]`.
- `0x0042AF2A`: loads packed coord from `TubeClass+0x28`.
- `0x0042AF2F..0x0042AF3B`: if tube index is `-1`, writes packed coord `(0,0)`.
- `0x0042AF69..0x0042AFAE`: toggles `0x40000` on the resolved destination cell.

Bounds fact: this consumer has no upper-bound compare against the tube count before indexing `g_TubeArray`. It only checks `Cell+0x116 == -1`. A safe Rust implementation may validate data earlier, but the fixture for this consumer should not claim gamemd treats out-of-range positive tube indices as a clean no-op.

Coordinate fact: direction `8` marks `TubeClass+0x28`, not `TubeClass+0x24`, not the next path-buffer step, and not `current + g_DirectionOffsets[8]`. If the current cell has `Cell+0x116 == -1`, the marker coordinate becomes `(0,0)`.

### Marker write

Active in YR: Yes.

After each replayed coordinate update, the function toggles the destination cell's `0x40000` bit. The binary calls `MapClass::Get_CellClass` twice for the same updated coord, then applies a masked XOR:

`dest.flags = dest.flags ^ ((~dest.flags ^ dest.flags) & 0x40000)`, which degenerates to `dest.flags ^= 0x40000` for this bit.

Evidence: `0x0042AF77..0x0042AFAE`, especially `0x0042AF8D`, `0x0042AF93`, `0x0042AFA5`, `0x0042AFAB`, `0x0042AFAE`.

The cost consumer is `AStar_compute_edge_cost @ 0x00429830`: if destination `CellClass+0x140 & 0x40000` is set, cost is multiplied by the float at `0x007E37BC` (`4.0`). Active in YR: Yes.

## 5. Current Rust Implementation Status

Read-only scan only.

| Rust surface | Current shape | Delta for this slice |
|---|---|---|
| `src/sim/pathfinding/core.rs:382` `explicit_tube_edge` | Normal A* expansion can jump explicit tubes to `tube.exit`; filters non-explicit/zero-step/(0,0) exits | Not the same as `0x0042ACF0` peer-marker replay. Marker replay uses current cell `tube_index`, `Tube+0x28`, and toggles cost markers rather than adding a normal path edge. |
| `src/sim/pathfinding/core.rs:891` direction-8 A* expansion | Adds a direction-8 edge only outside bridge layer; cost is `STEP_COST * path_len + TUBE_DIR_TIEBREAK` | Fixture for bridge marker should not use this cost path as proxy. Marker replay costs nothing directly; it marks cells so later edge-cost reads multiply by 4. |
| `src/sim/pathfinding/core.rs:992` `PathCell` | Carries `tube_index`, `low_bridge_tube_cell`, and bridge metadata | No per-search temporary `0x40000` marker overlay is visible in the scanned surface. |
| `src/sim/pathfinding/core.rs:1347` grid builder | Copies `tube_index` and low-bridge predicate from resolved terrain into `PathGrid` | Correct storage surface for tube metadata, but `0x40000` should remain search-scoped, not a persistent `PathGrid` bit. |
| `src/map/tube_facts.rs:30` `TubeFact` | Stores `entry`, `exit`, `direction`, `path_steps`, source | `TubeFact.exit` is the Rust analog for `TubeClass+0x28` in fixture setup. |
| `src/sim/movement/tube_movement.rs:57` | Runtime movement begins from coordinate-shape heuristic, not a direction-8 path-entry byte | Movement behavior is separate from temporary marker replay; do not conflate the two in tests. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `g_DirectionOffsets` values | verified | `0x0049F2F0`; writes `0x0049F305..0x0049F394` | none |
| Constructor reachability | verified | xref `0x00812BAC`; `0x007CBDAF`; `0x007CBED3`; `entry @ 0x007CD80F` | none |
| Static PE memory zeros | verified nuance | `read_memory/inspect_memory_content 0x0089F688` returned zeros before runtime ctor | live debugger dump unavailable |
| `0x0042ACF0` direction `0..7` consumer | verified | `0x0042AF41..0x0042AF65` | none |
| `0x0042ACF0` direction `8` tube branch | verified | `0x0042AF01..0x0042AF3B` | none for scoped branch |
| Direction-8 upper-bound behavior | verified | no compare after `0x0042AF15` except `CMP EAX,-1`; direct index at `0x0042AF27` | wider producer invariants out of scope |
| Marker bit write | verified | `0x0042AF77..0x0042AFAE` | none |
| Cost consumer | verified | `AStar_compute_edge_cost @ 0x00429830` | none for `0x40000` |
| Current Rust scan | touched-not-exhausted | codegraph; `rg`; targeted file reads | implementation details for future patch |
| Full live-memory overwrite proof | deferred | debugger endpoint unavailable | optional runtime watchpoint |

## 7. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - What are the exact table values? -> Dir order is N, NE, E, SE, S, SW, W, NW with signed pairs (0,-1), (1,-1), (1,0), (1,1), (0,1), (-1,1), (-1,0), (-1,-1).` (evidence: `0x0049F2F0`, writes `0x0049F305..0x0049F394`; Active in YR: Yes)
- `[RESOLVED] OQ-2 - Is the table static PE data or runtime-populated? -> Runtime-populated BSS/global storage; raw static memory is zero before the constructor writes it.` (evidence: `read_memory 0x0089F688`; `0x0049F2F0`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - Does the initializer run before gameplay? -> Yes, via constructor table entry `0x00812BAC`, iterated before `WinMain`.` (evidence: `0x00812BAC`, `0x007CBDAF`, `0x007CBED3`, `0x007CD80F`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - How does `0x0042ACF0` consume directions `0..7`? -> Index*4 into `0x0089F688` for X and `0x0089F68A` for Y, add signed shorts to current replay coord.` (evidence: `0x0042AF41..0x0042AF65`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - How does `0x0042ACF0` consume direction `8`? -> It reads current `Cell+0x116`; `-1` maps coord to `(0,0)`, otherwise `g_TubeArray[idx]+0x28` supplies the next coord.` (evidence: `0x0042AF01..0x0042AF3B`; Active in YR: Conditional)
- `[RESOLVED] OQ-6 - Is there an upper-bound check for tube index in this consumer? -> No scoped upper-bound compare exists; only `idx == -1` is checked before direct pointer-table indexing.` (evidence: `0x0042AF15..0x0042AF27`; Active in YR: Conditional)
- `[RESOLVED] OQ-7 - What does each replayed step write? -> The resolved destination cell's `0x40000` bit is toggled.` (evidence: `0x0042AF77..0x0042AFAE`; Active in YR: Yes)
- `[RESOLVED] OQ-8 - What reads the marker? -> `AStar_compute_edge_cost @ 0x00429830` multiplies destination cost by 4.0 when `0x40000` is set.` (evidence: `0x00429830`; Active in YR: Yes)
- `[RESOLVED] OQ-9 - What is the replay length bound? -> At most 24 processed entries; stops when counter reaches `0x18` or the next path entry is `-1`.` (evidence: `0x0042AEF6`, `0x0042AFB2`; Active in YR: Yes)
- `[RESOLVED] OQ-10 - Does Rust currently have a persistent PathGrid cell for this marker? -> No obvious per-search `0x40000` overlay or persistent marker field in targeted scan.` (evidence: `src/sim/pathfinding/core.rs:990..1010`, `1347..1425`; Active in YR: Rust status only)
- `[DEFERRED] OQ-11 - Can live runtime memory prove no post-constructor overwrite?` (category: `needs-runtime-debugger`; reason: debugger server was unavailable; next-step-if-pursued: set a read/write watchpoint on `0x0089F688..0x0089F6A7` through startup)
- `[DEFERRED] OQ-12 - Which producers guarantee `Cell+0x116` is not a positive out-of-range index before `0x0042ACF0`?` (category: `out-of-scope`; reason: this slice is the consumer; next-step-if-pursued: TubeClass producer/lifecycle audit)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Direction entries `0..7` replay as signed adjacent offsets in N, NE, E, SE, S, SW, W, NW order | `0x0049F2F0`; `0x0042AF41..0x0042AF65` | missing for temporary marker overlay | `src/sim/pathfinding/core.rs` future per-search bridge-marker fixture/helper | Replaying peer path `[0,1,2,3,4,5,6,7]` from a start coord must mark the exact cumulative destination cells produced by these offsets | Proposed test: `astar_bridge_marker_peer_path_uses_gamemd_direction_offset_order` | Do not use a rotated, keypad, or Rust-facing enum order without proving it maps to this table |
| Direction `8` in marker replay jumps to `TubeClass+0x28` from the current cell's `Cell+0x116` | `0x0042AF01..0x0042AF2A` | missing; current `explicit_tube_edge` is normal A* expansion, not marker replay | `src/sim/pathfinding/core.rs`; `src/map/tube_facts.rs` fixture data | A peer path containing `8` on a cell with `tube_index = idx` must mark the tube exit coord (`TubeFact.exit`) as a temporary 4x-cost destination | Proposed test: `astar_bridge_marker_direction8_marks_tube_exit_coord` | Do not add an adjacent offset for `8`; do not mark `Tube+0x24`/entry; do not mark every tube path buffer step |
| Direction `8` with `Cell+0x116 == -1` resets replay coord to `(0,0)` and toggles that cell | `0x0042AF1C..0x0042AF3B`; marker write `0x0042AF69..0x0042AFAE` | missing | Future temporary marker overlay; fixture map with origin in bounds | A peer path `[8]` from a no-tube cell marks `(0,0)` rather than doing nothing | Proposed test: `astar_bridge_marker_direction8_without_tube_marks_origin` | Do not silently skip missing-tube direction `8` if the goal is consumer parity |
| Positive tube indices are not upper-bound checked inside `0x0042ACF0` | `0x0042AF15..0x0042AF27` | safe Rust likely validates earlier | `ResolvedTerrainGrid`/`PathGrid` tube metadata producers and tests | Keep fixture data valid for normal parity; if Rust guards invalid indices for safety, document it as an input-sanitization boundary, not gamemd consumer behavior | Proposed test: `pathgrid_tube_indices_for_bridge_marker_fixtures_are_validated_before_replay` | Do not claim gamemd's consumer treats out-of-range positive indices as no-op |
| `0x40000` is a search-scoped cost marker; edge cost reads multiply destination cost by 4.0 | `0x0042AF8D..0x0042AFAE`; `0x00429830` | missing; no per-search overlay seen | `src/sim/pathfinding/core.rs`; possible temporary marker overlay passed into A* | Marker fixture should assert affected cells cost 4x during the search and base `PathGrid` remains unchanged afterward | Proposed test: `astar_bridge_marker_overlay_is_search_scoped_and_multiplies_cost` | Do not persist this in `PathGrid`, bridge runtime state, save state, or terrain costs |

## Negative Facts / Do Not Do

- Do not treat `0x0089F688` as static initialized PE data; Ghidra static memory is zero and `0x0049F2F0` populates it before `WinMain`. Active in YR: Yes.
- Do not use `dir == 8` as a ninth adjacent offset. Active in YR: Conditional; evidence `0x0042AF01..0x0042AF2A`.
- Do not model direction-8 marker replay as Rust's current normal tube A* expansion cost (`STEP_COST * path_len + TUBE_DIR_TIEBREAK`). The binary marker replay just chooses a coordinate and toggles `0x40000`. Active in YR: Conditional.
- Do not mark tube entry, intermediate path steps, or all tube cells for this fixture; the scoped consumer reads `TubeClass+0x28` only. Active in YR: Conditional.
- Do not bake `0x40000` into persistent `PathGrid` or terrain cost data. The bit is toggled around A* and read as a temporary destination cost multiplier. Active in YR: Yes.
- Do not state that the consumer bounds-checks `Cell+0x116` against tube count; this function does not. Active in YR: Conditional.

## Remaining Uncertainty

- Live runtime watchpoint proof of no post-constructor overwrite was not possible because the debugger endpoint was unavailable. Static evidence proves the startup value and the constructor path.
- Wider TubeClass producer invariants for positive out-of-range `Cell+0x116` values were not investigated in this slot.
- Original Westwood symbolic names for the table and the `0x40000` bit remain unknown; names in this report are behavior-derived.

## Stale Docs / Follow-up Docs

No stale-doc wording is required for the specific scoped question. The prior `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md` explicitly deferred `g_DirectionOffsets`; this report fills that gap.

## Sources

- Ghidra decompile: `Foundation_direction_table_init @ 0x0049F2F0`
- Ghidra assembly contexts: `0x0049F305`, `0x0049F322`, `0x0049F336`, `0x0049F34A`, `0x0049F375`, `0x0049F388`, `0x0049F38E`, `0x0049F394`
- Ghidra xref: `0x0049F2F0` data xref `0x00812BAC`
- Ghidra memory inspect: `0x00812BAC` contains pointer `0x0049F2F0`
- Ghidra decompile: `entry @ 0x007CD80F`, `FUN_007CBDAF @ 0x007CBDAF`, `FUN_007CBED3 @ 0x007CBED3`
- Ghidra decompile and assembly contexts: `PathfinderClass__UpdateBridgePassability @ 0x0042ACF0`, especially `0x0042AEF6`, `0x0042AF01`, `0x0042AF15`, `0x0042AF21`, `0x0042AF27`, `0x0042AF2A`, `0x0042AF41`, `0x0042AF49`, `0x0042AF69..0x0042AFAE`
- Ghidra decompile: `AStar_compute_edge_cost @ 0x00429830`
- Existing docs: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `BIB_ADJACENT_CELL_DIRECTION_SOURCE_GHIDRA_REPORT.md`, `traces/IMPLEMENTATION_LOW_BRIDGE_TUBECLASS_HEIGHT_LAYER_TRACE.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/map/tube_facts.rs`, `src/sim/movement/tube_movement.rs`
