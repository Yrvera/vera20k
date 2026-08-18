# Pathfinder UpdateBridgePassability 0x0042ACF0 - Ghidra Research Report

**Address(es):** `0x0042ACF0` primary; callers/callees `0x00429A90`, `0x00429830`, `0x0042B080`, `0x0042A6D0`, `0x0042C900`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, exact `CellClass+0x140 & 0x40000` bridge-approach A* cost marker lifecycle/geometry, 5x5 update behavior, and immediate callers/callees.  
**Non-Scope:** `CellClass+0x140 & 0x400` bridge inactive/fallback marker semantics, bridge rendering, full A* pathfinding, full movement-zone rebuilds, and all CellClass flag bits.  
**Confidence:** High for the scoped binary behavior; Medium for the human-readable `bridge-approach` name because no original symbol name was available.  
**Active in YR:** Yes. The function is constructed enabled (`PathfinderClass+0x03 = 1`) and is called by live A* (`AStar_main_loop @ 0x00429A90`) whenever per-search urgency `PathfinderClass+0x3C` is nonzero.

## 1. Overview

`PathfinderClass::UpdateBridgePassability` temporarily toggles `CellClass+0x140 bit 0x40000` during A* searches. `AStar_main_loop` calls it before the search and again after success or failure, so the normal lifecycle is "toggle on, search while marked, toggle back off."

`0x40000` is the A* bridge-approach cost marker consumed by `AStar_compute_edge_cost @ 0x00429830`, which multiplies the destination cell cost by `4.0`. It is explicitly distinct from `CellClass+0x140 & 0x400`, which prior work verified as a bridge inactive/fallback marker and is not this A* cost bit.

## 2. Class Layout / Key Offsets

| Offset / bit | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `PathfinderClass+0x03` | Master enable byte for `UpdateBridgePassability`; constructor sets it to `1`; function returns immediately if zero | Yes | `0x0042A6D0` writes `param_1[3] = 1`; `0x0042ACF3` reads `[ECX+0x3]` |
| `PathfinderClass+0x3C` | Per-search urgency / retry mode; gates calls from A* and changes peer-path eligibility | Conditional: active when nonzero | `0x00429C10`, `0x0042A423`, `0x0042A43E` call only if nonzero; `0x0042C900` writes caller param (corrected 2026-07-12: second address was `0x0042A3FE`, which is actually inside the `AStar_reconstruct_path @ 0x0042AA90` call setup, not a `+0x3C` gate check; the real second gate-check read is `MOV EAX,[ESI+0x3C]` at `0x0042A423` — verified via disassemble_function 0x00429A90 — GHIDRA_ADDRESS_SHIFT) |
| `FootClass+0x8C` | Searching unit on-bridge byte used in layer/list choice | Yes | `0x0042ADB8` |
| `Techno/Object+0x30` | Next object pointer in cell object list | Yes | `0x0042AFBF`; helper `0x0042B080` walks `+0x30` |
| `CellClass+0x24` | Packed cell coordinate | Yes | `0x0042AFD5`, `0x0042B00F` |
| `CellClass+0xE4` | Ground object-list head | Yes | `0x0042ADCC`; helper `0x0042B080` |
| `CellClass+0xE8` | Bridge/alternate object-list head | Yes | `0x0042ADC2`; helper `0x0042B080` |
| `CellClass+0x116` | Tube index; `-1` means no tube | Conditional: only direction `8` path queue entries | `0x0042AF15` |
| `CellClass+0x11B` | Signed cell level | Yes | `0x0042AD9E`, `0x0042ADD8` |
| `CellClass+0x124` | Occupation byte tested by the 5x5 phase | Yes | `0x0042B005` |
| `CellClass+0x140 & 0x100` | Structural bridge cell bit used for layer/list choice | Yes | `0x0042AD93`, `0x0042B080` |
| `CellClass+0x140 & 0x40000` | Temporary A* bridge-approach cost marker, XOR-toggled here and read by edge cost | Yes | Writers `0x0042AF93`, `0x0042B029`, `0x0042B04F`; reader `0x00429830` |
| `CellClass+0x140 & 0x400` | Separate bridge inactive/fallback marker; not this report's A* marker | Yes, but out of scope | `CELLCLASS_0X140_BIT_0X400_PATHGRID_SEMANTIC_GHIDRA_REPORT.md`; negative boundary from `0x00429830` |

## 3. Core Logic

### 3.1 Entry and Call Lifecycle

Active in YR: Yes, conditional on `PathfinderClass+0x3C != 0` for each search.

`AStar_main_loop @ 0x00429A90` calls `UpdateBridgePassability` in three places:

- Before seeding the initial closed-list, when start/destination cell or height differs and `PathfinderClass+0x3C != 0` (`0x00429C10..0x00429C1A`).
- After successful path reconstruction/smoothing, again gated by `+0x3C != 0` (`0x0042A423..0x0042A42D` in the success tail; corrected 2026-07-12: was `0x0042A3FE..0x0042A406` — that range is the `CALL 0x0042AA90` (`AStar_reconstruct_path`) setup two calls earlier, not the `UpdateBridgePassability` gate/call. `AStar_reconstruct_path` (`0x0042A406`), `Path_smooth_corners` (`CALL 0x0042B210` at `0x0042A415`), and `Path_optimize_straight_segments` (`CALL 0x0042B7F0` at `0x0042A41E`) run first; the actual `+0x3C` re-check is `MOV EAX,[ESI+0x3C]` at `0x0042A423` and the call itself is at `0x0042A42D` — verified via disassemble_function 0x00429A90 and get_xrefs_to 0x0042ACF0 (callers: `0x00429C1A`, `0x0042A42D`, `0x0042A44C`) — GHIDRA_ADDRESS_SHIFT).
- On failure/no-result exit, again gated by `+0x3C != 0` (`0x0042A43E` region in the decompile).

Because the writer uses XOR, the second call is the normal cleanup. A port must not persist these bits in the static path grid.

### 3.2 Initial Probe Cell and Layer Selection

Active in YR: Yes.

The function:

1. Gets the searching unit's current cell coordinate via vtable `+0x1B8`, then `MapClass::Get_CellClass`.
2. Reads a pseudo-random direction from `RateTimer__Current`: `dir = (((timer >> 12) + 1) >> 1) & 7`.
3. Adds `g_DirectionOffsets[dir]` to the current cell coordinate; this probe is not the ordered movement direction.
4. Chooses which object list on the probe cell to scan:
   - Ground list `Cell+0xE4` when the probe is not structural bridge (`!(flags & 0x100)`), or when `abs(current_cell.Level - probe.Level) <= 3` and the searching unit is not on a bridge.
   - Bridge list `Cell+0xE8` when the probe is a bridge cell and either the level gap is at least 4 or the searching unit is already on a bridge.

Assembly evidence: `0x0042AD35..0x0042AD8D` for timer/probe coordinate, `0x0042AD93..0x0042ADD4` for list choice. Note the exact comparison: assembly uses `CMP EAX,0x3; JG bridge-list`, so `abs > 3` selects bridge, not `>= 3`.

If the chosen list is null, it calls helper `0x0042B080(probe_coord, probe.Level + (bridge ? 4 : 0))` to find a nearby object at the selected height.

### 3.3 Helper `0x0042B080` Fallback Lookup

Active in YR: Yes, conditional on the selected `E4/E8` list being null.

`FUN_0042B080` scans a 5x5 square centered on the probe coordinate, with offsets `[-2..2]` in both axes. For each candidate cell, it chooses that cell's `E4` or `E8` object list by bridge bit and height:

- Ground list if `!(flags & 0x100)` or `abs(candidate.Level - requested_height) < 3`.
- Bridge list otherwise.

It then walks the list using object `+0x30`. It only considers objects with state bit `((object+0x14) >> 2) & 1` set. For those objects, it asserts `object+0x674` is non-null, calls vtable `+0xA0` on that subobject with the original center cell's lepton center `(x*256+128, y*256+128, requested_height * DAT_0089C2D8)`, and returns the first object whose subobject says yes. If none pass, it returns null.

Evidence: decompile `0x0042B080`; Active in YR: Yes through `0x0042ADF1`.

### 3.4 Object Eligibility for Path-Queue Propagation

Active in YR: Yes.

For each object found in the selected list, the function only handles vtable `+0x2C` kinds `1` and `0xF`. It skips all other object kinds.

Additional gate before reading the object's path queue:

- If `PathfinderClass+0x3C == 2`, it bypasses the owner-priority/playfield gate and can inspect the peer path.
- Otherwise, it skips the object when it is the same TechnoType as the searching unit, when the searching unit's `TechnoType+0x678` value is not greater than the peer object's `TechnoType+0x678` value, or when `MapClass::Is_Cell_In_Playfield` on the peer path start fails.

Evidence: `0x0042AE15..0x0042AE7D`; especially `0x0042AE58..0x0042AE66` (`CMP searching_type+0x678, peer_type+0x678; JLE skip`). Active in YR: Yes. The exact semantic name of `+0x678` is not proven in this slice; the ordering comparison itself is verified.

### 3.5 Path Queue Toggle Geometry

Active in YR: Yes when a scanned peer has a non-empty path queue.

Path queue base is `object + 0x5E0`, corresponding to object int-index `0x178` in the decompiler. It stores direction codes with `-1` terminator and a maximum of 24 entries.

The function treats kind `1` and kind `0xF` differently before entering the shared loop:

- Kind `1`: requires `path[0] != -1` and `path[1] != -1`, then enters the loop with `EDI = object + 0x5E0`, so the first processed entry is `path[0]`. `path[1]` is a prerequisite, not the first processed entry.
- Kind `0xF`: requires `path[0] != -1`, `path[1] != -1`, and `path[2] != -1`, then also enters the loop with `EDI = object + 0x5E0`, so the first processed entry is `path[0]`.

The shared loop:

- Stops before processing more than 24 entries (`CMP EBX,0x18; JGE ...` at `0x0042AEF6`).
- Direction `0..7`: adds `g_DirectionOffsets[dir]` to the current path coordinate.
- Direction `8`: reads the current cell's `Cell+0x116`; if `-1`, it resets the path coordinate to `(0,0)`, otherwise it reads `g_TubeArray[tube_index]+0x28` as the next coordinate.
- For each step, it XOR-toggles the destination cell's `0x40000` bit. The binary calls `Get_CellClass` twice on the same updated coordinate, so the apparent source/destination masked write degenerates to a destination-cell toggle for this bit.

Assembly evidence for the masked write: `0x0042AF77..0x0042AFAE`, especially:

`MOV EDX,[prev+0x140]; MOV ESI,[dest+0x140]; NOT EDX; XOR EDX,ESI; AND EDX,0x40000; XOR ESI,EDX; MOV [dest+0x140],ESI`.

Both `prev` and `dest` resolve from the same current path coordinate in this sequence. Therefore, for the `0x40000` bit, this is equivalent to `dest.flags ^= 0x40000`, not an alternating inverse-of-source pattern.

### 3.6 Fallback / 5x5 Occupation Toggle

Active in YR: Yes whenever at least one peer path was processed, or when no peer path was processed and `PathfinderClass+0x3C != 1`. Conditional no-op: if no peer path was processed and `+0x3C == 1`, the function writes `+0x3C = 0` and returns before this 5x5 phase.

The 5x5 phase is centered on the probe cell chosen in section 3.2. It loops `dx=-2..2` and `dy=-2..2`:

- Computes `candidate = probe.coord + (dx, dy)`.
- Reads `candidate.Cell+0x124`.
- If that byte is zero, it does nothing for that candidate.
- If nonzero, it compares `candidate.Cell+0x24` to the searching unit's original current coordinate.
- If candidate equals the searching unit's current coordinate, it skips it.
- Otherwise it XOR-toggles `candidate.Cell+0x140 & 0x40000`.

After all 25 candidates, it unconditionally XOR-toggles the probe-center cell's `0x40000`.

Assembly evidence: loop setup `0x0042AFCB..0x0042AFFC`; occupation/self tests `0x0042B005..0x0042B027`; XOR toggle `0x0042B029..0x0042B03D`; loop bounds `0x0042B043..0x0042B04D`; unconditional center toggle `0x0042B04F..0x0042B063`.

Tiny parity detail: an occupied probe-center cell is toggled once by the candidate loop and once by the unconditional final write, so its net state is unchanged. An unoccupied probe-center cell is toggled once by the final write. Occupied non-center cells in the 5x5 square are toggled once, except the searching unit's own current cell is skipped.

### 3.7 Cost Consumer

Active in YR: Yes.

`AStar_compute_edge_cost @ 0x00429830` reads `dest_cell+0x140 & 0x40000`; if set, it multiplies the edge cost by `4.0` from `0x007E37BC`. The same function does not use `0x400` for this multiplier. `0x400` and `0x40000` are separate bits with separate lifecycles.

Evidence: decompile `0x00429830`; prior `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md` context; `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`.

## 4. INI Keys

No INI key directly sets `CellClass+0x140 & 0x40000` or directly configures `UpdateBridgePassability`.

| Input | Relationship | Active in YR | Evidence |
|---|---|---|---|
| Unit movement/path orders | Reach `FootClass::Find_Path -> Run_AStar -> AStar_pathfind_search -> AStar_main_loop` | Yes | prior A* spine reports; `0x00429A90` direct caller |
| `[General] PathDelay` / caller throttles | Can affect when path searches are requested, not the bit geometry | Conditional | `PATHFINDERCLASS_GHIDRA_REPORT.md` |
| MovementZone / SpeedType | Affect A* and list/cost context, but do not write `0x40000` | Yes | `0x0042C900`, `0x00429A90` |

## 5. Integration Points

| Integration point | Role | Active in YR | Evidence |
|---|---|---|---|
| `AStar_main_loop @ 0x00429A90` | Sole verified caller; calls before and after A* under `+0x3C != 0` | Yes / Conditional | `0x00429C10`, success/failure tails |
| `PathfinderClass constructor @ 0x0042A6D0` | Initializes `+0x03 = 1`, so the function body is enabled | Yes | decompile |
| `AStar_pathfind_search @ 0x0042C900` | Writes `+0x3C = caller param`, which gates calls and behavior | Yes | decompile |
| `FUN_0042B080` | Fallback object finder when chosen object list is null | Conditional | `0x0042ADF1` |
| `AStar_compute_edge_cost @ 0x00429830` | Reads `0x40000` and multiplies by 4.0 | Yes | decompile |
| `CellClass::RecalcAttributes @ 0x0047D2B0` | Not a writer for `0x40000` in this slice; prior docs saying otherwise are stale | Yes for function, No for this write | `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`; target writes found only in Pathfinder path |

## 6. Current Rust Implementation Status

Read-only scan only:

| Rust area | Current status vs scoped finding | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs`, `src/sim/movement/path_markers.rs`, `src/sim/movement/movement_commands.rs` | STALE (corrected 2026-07-12): a search-scoped `SearchMarkerOverlay` (XOR-toggle `BTreeSet`, `core.rs:216-238`) and a `SEARCH_MARKER_COST_MULTIPLIER = 4` (`core.rs:139`) applied via `apply_search_marker_cost` (`core.rs:2134`) now exist and are wired into live pathfinding, not just tests. `build_peer_search_marker_overlay` (`path_markers.rs:13`) replays up to 24 remaining path-queue destinations per nearby peer with XOR-parity, called from `movement_commands.rs:364-366` and threaded into `find_move_path`/`find_layered_path_zoned_marker`/`find_path_zoned_marker`. This covers the per-search overlay (Handoff row 1) and peer-path propagation (row 2) in spirit, though the peer-selection mechanism differs from the binary's probe-cell/priority-ordered scan (see Rust delta note below). The pseudo-random-probe 5x5 fallback (row 3) is still absent — no `.toggle()` caller outside `core_tests.rs`/`movement_path.rs` tests builds a probe-centered 5x5 overlay. | `Grep` for `SearchMarkerOverlay`, `build_peer_search_marker_overlay`, `.toggle(`; `Read` `src/sim/pathfinding/core.rs` lines 120-238; `src/sim/movement/path_markers.rs` (full file); `src/sim/movement/movement_commands.rs` lines 360-460 |
| `src/sim/pathfinding/core.rs` height cost | Current code multiplies cost when effective path height changes; binary `0x40000` multiplier is cell-flag-driven instead | `src/sim/pathfinding/core.rs` lines around cliff/height multiplier |
| `src/sim/pathfinding/terrain_cost.rs` | Terrain costs are static per speed/terrain and cannot represent transient per-search peer-path flags | file scan |
| `src/app_sim_tick.rs::rebuild_dynamic_path_grid` | Rebuilds static/dynamic bridge walkability from terrain and bridge runtime state; this should not persist `0x40000` because gamemd toggles it around individual A* calls | `src/app_sim_tick.rs` |
| `src/sim/bridge_state/` | Models bridge runtime walkability/damage, not this transient A* congestion marker | file scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | verified | decompile + assembly contexts | none for scoped writer geometry |
| Early enable `PathfinderClass+0x03` | verified | `0x0042ACF3`; constructor `0x0042A6D0` | no other setters searched exhaustively |
| A* caller lifecycle | verified | `0x00429A90` decompile/assembly | exact higher-level caller urgency values remain wider A* scope |
| Initial random probe and list choice | verified | `0x0042AD35..0x0042ADD4` | no runtime distribution sampling |
| `FUN_0042B080` fallback lookup | verified | decompile `0x0042B080` | exact identity of subobject vtable `+0xA0` out of scope |
| Path-queue marker propagation | verified | `0x0042AE80..0x0042AFB5`; assembly `0x0042AF93` | exact semantic of object kind names remains prior-doc dependent |
| Direction `8` tube handling | verified | `0x0042AF01..0x0042AF3F` | TubeClass internals out of scope |
| 5x5 occupation toggle | verified | `0x0042AFCB..0x0042B063` | none for geometry |
| Cost consumer `0x00429830` | verified | decompile | none for `0x40000` cost multiplier |
| `0x400` distinction | verified negative | `0x00429830`; `CELLCLASS_0X140_BIT_0X400...` | no re-documentation of `0x400` beyond boundary |
| INI writers | verified negative for direct key | doc/code search; target decompile | none |
| Rust current status | touched-not-exhausted | codegraph + `rg` | implementation details for future patch |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-1 - Is this function live in YR? -> Yes; constructor enables `+0x03`, and live A* calls it under `+0x3C != 0`.` (evidence: `0x0042A6D0`, `0x00429A90`; Active in YR: Yes / Conditional)
- `[RESOLVED] OQ-2 - Who calls it? -> Verified caller is `AStar_main_loop`; it calls before search and after success/failure.` (evidence: `0x00429A90`; Active in YR: Yes)
- `[RESOLVED] OQ-3 - What reads the bit? -> `AStar_compute_edge_cost` multiplies destination cost by 4.0 when `0x40000` is set.` (evidence: `0x00429830`; Active in YR: Yes)
- `[RESOLVED] OQ-4 - Is `0x40000` the same as `0x400`? -> No; `0x400` is a separate inactive/fallback bridge marker and is not the A* cost bit.` (evidence: `0x00429830`; `CELLCLASS_0X140_BIT_0X400...`; Active in YR: Yes)
- `[RESOLVED] OQ-5 - Does RecalcAttributes own this bit? -> No scoped writer found there; current verified writer is Pathfinder's per-search toggler.` (evidence: `0x0042ACF0`; `FULL_PASSABILITY_RECALC_0047D2B0...`; Active in YR: No for Recalc write)
- `[RESOLVED] OQ-6 - What is the 5x5 geometry? -> Centered on the pseudo-random probe cell, offsets -2..2 both axes; occupation byte gates candidate toggles; own current cell skipped; center toggled unconditionally afterward.` (evidence: `0x0042AFCB..0x0042B063`; Active in YR: Yes)
- `[RESOLVED] OQ-7 - What happens if no peer path is found? -> If no peer path and `+0x3C == 1`, set `+0x3C = 0` and return; otherwise run the 5x5 phase.` (evidence: `0x0042AEB9..0x0042AECB`; Active in YR: Conditional)
- `[RESOLVED] OQ-8 - What is the max peer path length? -> 24 entries; loop stops when counter reaches `0x18` or terminator `-1`.` (evidence: `0x0042AEF6`, `0x0042AFB2`; Active in YR: Yes)
- `[RESOLVED] OQ-9 - How are tubes handled? -> Direction `8` reads `Cell+0x116`; `-1` resets coord to `(0,0)`, otherwise jumps to `g_TubeArray[idx]+0x28`.` (evidence: `0x0042AF01..0x0042AF3F`; Active in YR: Conditional)
- `[RESOLVED] OQ-10 - What if selected object list is null? -> Helper `0x0042B080` scans a 5x5 area and uses a height-aware object test to find an object.` (evidence: `0x0042ADF1`, `0x0042B080`; Active in YR: Conditional)
- `[RESOLVED] OQ-11 - Is there a null-pointer edge? -> Null selected/fallback object list skips peer path processing and may fall through to urgency/5x5 behavior; helper returns 0 cleanly.` (evidence: `0x0042AE09`, `0x0042B080`; Active in YR: Yes)
- `[RESOLVED] OQ-12 - Does the bit persist after normal A*? -> Normal pre/post XOR lifecycle cancels; persistence would require interrupted/asymmetric call flow not found in scoped caller.` (evidence: paired calls in `0x00429A90`; Active in YR: Yes)
- `[DEFERRED] OQ-13 - Exact semantic name of object `+0x678` owner/rank comparison.` (category: out-of-scope; reason: only the gating comparison matters for this marker lifecycle; next-step-if-pursued: dedicated TechnoType/object identity field pass)
- `[DEFERRED] OQ-14 - Exact runtime values/population of `g_DirectionOffsets @ 0x0089F688`.` (category: requires-different-system-context; reason: table values are a general coordinate-system artifact; next-step-if-pursued: table initialization/value dump)
- `[DEFERRED] OQ-15 - Whether every possible abnormal A* interruption always runs cleanup.` (category: needs-runtime-debugger; reason: scoped static caller has cleanup tails, but process abort/exception scenarios cannot be proven statically; next-step-if-pursued: runtime break/trace paired calls)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x40000` is toggled per A* search and cleaned by a second call; it is not static terrain/passability | `0x00429A90`, `0x0042ACF0`, `0x00429830` | implemented (corrected 2026-07-12, was `missing`): `SearchMarkerOverlay` (`src/sim/pathfinding/core.rs:216-238`) is a search-scoped XOR-toggle `BTreeSet`, never persisted into `PathGrid`; `SEARCH_MARKER_COST_MULTIPLIER = 4` (`core.rs:139`) is applied via `apply_search_marker_cost` (`core.rs:2134`) | `src/sim/pathfinding/core.rs` | Add a per-search temporary bridge-approach/crowd marker consulted by A* cost, scoped to one search and restored afterward | Two consecutive searches over the same map leave the base `PathGrid` unchanged, while the first search can see temporary 4x cells; proposed test: `astar_bridge_approach_markers_are_search_scoped_and_restored` (verify against existing `core_tests.rs`/`movement_path.rs` overlay tests, which cover this scenario) | Do not bake `0x40000` into `PathGrid` rebuilds or bridge runtime state |
| Peer path propagation XOR-toggles `0x40000` on destination cells along scanned peer object path queues, with max 24 entries and direction-8 tube handling | `0x0042AEF6..0x0042AFB5`; corrected by 2026-05-22 verify-doc audit, parent spot-check `disassemble_function 0x0042ACF0` | partially implemented (corrected 2026-07-12, was `missing`): `build_peer_search_marker_overlay` (`src/sim/movement/path_markers.rs:13`) replays up to `PEER_MARKER_REPLAY_LIMIT = 24` remaining path-queue cells per nearby peer with XOR-parity (`overlay.toggle(cell)`), wired live via `movement_commands.rs:364-366`. Remaining delta: peer selection is a flat `PEER_MARKER_LOCAL_RADIUS = 3`-cell scan of all live entities, not the binary's probe-cell object-list scan with kind `1`/`0xF` gating, `+0x678` priority ordering, or `Is_Cell_In_Playfield`; no direction-8 tube-jump handling in the replay | `src/sim/movement/path_markers.rs`; `src/sim/movement/movement_commands.rs` | When computing a path near other moving units, derive temporary cost marks from nearby peers' queued directions, including tube jump semantics if tube paths are in play; start at `path[0]` for both kind `1` and kind `0xF` after their respective prerequisites | Two moving units with queued bridge-approach paths cause a third unit's A* to prefer a non-marked alternative because marked destination cells cost 4x; proposed test: `astar_bridge_approach_peer_path_queue_marks_raise_costs` (verify against existing `path_markers.rs` tests, which cover replay-cap and XOR-parity but not probe-cell/priority gating or tube jumps) | Do not model this as simple occupancy blocking, walkability changes, an own-path walk, or an alternating inverse-of-source pattern |
| 5x5 fallback toggles occupied cells around a pseudo-random adjacent probe, skips the searching unit's own cell, then toggles the probe center unconditionally | `0x0042AD35..0x0042ADD4`, `0x0042AFCB..0x0042B063` | missing | `src/sim/pathfinding/core.rs`; deterministic RNG/timer compatibility surface | Add the 5x5 cost-marker fallback only in the same urgency/no-peer conditions, using the exact occupation/self/center rules | In a fixture with occupied cells in the 5x5 square, own cell is not marked, non-center occupied cells are marked, occupied center nets unchanged, unoccupied center is marked; proposed test: `astar_bridge_approach_5x5_toggle_matches_probe_center_rules` | Do not implement this as "mark all cells in a 5x5"; the center and occupation gates change results |

### Negative Facts / Do Not Do

- Do not conflate `CellClass+0x140 & 0x40000` with `CellClass+0x140 & 0x400`. Evidence: A* cost reads `0x40000` at `0x00429830`; the separate `0x400` report ties `0x400` to inactive/fallback bridge semantics. Active in YR: Yes.
- Do not label `0x40000` as a permanent cliff-ramp/RecalcAttributes flag. Evidence: scoped writers are `PathfinderClass::UpdateBridgePassability` pre/post A* toggles; `FULL_PASSABILITY_RECALC_0047D2B0` identifies RecalcAttributes as single-cell zone/attribute work, not this temporary writer. Active in YR: Yes for A*, No for Recalc writer.
- Do not persist the marker in Rust `PathGrid`, zone grid, save state, or bridge runtime state. Evidence: `AStar_main_loop` calls the toggler before and after search; normal net effect is cleanup. Active in YR: Yes.
- Do not use walkability/blocking to represent `0x40000`. Evidence: `AStar_compute_edge_cost` only multiplies cost by 4.0 when the destination cell has the bit; no passability rejection is tied to this bit in the scoped consumer. Active in YR: Yes.
- Do not simplify the 5x5 behavior to unconditional square marking. Evidence: `Cell+0x124` gates candidate toggles, the searching unit's current coordinate is skipped, and the probe center is toggled unconditionally after the loop. Active in YR: Yes.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/PATHFINDING_STANDALONE_FUNCTIONS_GHIDRA_REPORT.md` replacement wording:
  "`CellClass+0x140 bit 0x40000` is the temporary Pathfinder bridge-approach A* cost marker. `AStar_compute_edge_cost @ 0x00429830` multiplies destination-cell cost by 4.0 when it is set. The verified writer/clearer is `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`, called before and after A* under `PathfinderClass+0x3C != 0`; do not describe this as a permanent cliff-ramp flag set by `CellClass::RecalcAttributes`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/fidelity-checks/bridge-cost-penalty.md` replacement wording:
  "`0x40000` is not a terrain cliff-ramp bit. It is a transient Pathfinder bridge-approach cost marker toggled around individual A* searches by `0x0042ACF0`; it should be modeled as a per-search cost overlay, not as static terrain or walkability."
- `C:/Users/enok/Documents/ra2-rust-game-docs/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` replacement wording:
  "Where this report calls `0x40000` a cliff/impassable-edge flag, replace with: `temporary Pathfinder bridge-approach cost marker; reader cost multiplier is verified at `0x00429830`, writer/cleanup lifecycle at `0x0042ACF0`."

## 10. Remaining Uncertainty

- Exact original symbolic name for the marker is unknown; `bridge-approach A* cost marker` is behavior-derived.
- Exact semantic name of the object/Techno field `+0x678` used in peer eligibility is not resolved here.
- Runtime proof of paired cleanup under abnormal interruption was not performed; static caller tails are paired for normal success/failure.
- Exact initialized contents of `g_DirectionOffsets @ 0x0089F688` were not dumped; use existing direction-table research if implementation needs byte-for-byte coordinate deltas.

## Sources

- Ghidra decompile: `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`
- Ghidra assembly contexts: `0x0042ACF3`, `0x0042AD93`, `0x0042AE15`, `0x0042AE4E`, `0x0042AEF6`, `0x0042AF93`, `0x0042AFCB`, `0x0042B000`, `0x0042B043`
- Ghidra decompile: `FUN_0042B080`
- Ghidra decompile: `AStar_main_loop @ 0x00429A90`
- Ghidra assembly contexts: `0x00429C10`, `0x0042A3DE`
- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`
- Ghidra decompile: `PathfinderClass::Constructor @ 0x0042A6D0`
- Ghidra decompile: `AStar_compute_edge_cost @ 0x00429830`
- Existing docs: `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md`, `BRIDGE_ASTAR_COSTS_AND_ZONE_PRECHECK_GHIDRA_REPORT.md`, `CELLCLASS_0X140_BIT_0X400_PATHGRID_SEMANTIC_GHIDRA_REPORT.md`, `FULL_PASSABILITY_RECALC_0047D2B0_GHIDRA_REPORT.md`, `ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/app_sim_tick.rs`, `src/sim/bridge_state/`
