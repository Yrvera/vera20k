# Path_Reroute_Straight_Line Slope/Marker/Retry -- Ghidra Research Report

**Address(es):** `Path_Reroute_Straight_Line @ 0x0042BE20`; slope-factor call inside at `0x0042BEC3`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact `Path_Reroute_Straight_Line` validation algorithm, its two call sites from `Path_optimize_straight_segments`, and the active YR pathfinding caller chain.
**Non-Scope:** full semantics of every subclass implementation behind virtual passability slot `+0x1AC`; full `MapClass__Get_Slope_Cost_At_Cell` table generation; broader A* retry/corridor behavior.
**Confidence:** High for the reroute helper, constants, caller arguments, and active path; Medium for subclass-specific passability meanings behind the virtual call.
**Active in YR:** Yes. On successful A*, `AStar_main_loop @ 0x00429A90` unconditionally calls `Path_smooth_corners` then `Path_optimize_straight_segments`, and that caller is the sole caller of `Path_Reroute_Straight_Line`.

## Summary

`Path_Reroute_Straight_Line` tries to replace a curved direction-array window with a straight Chebyshev decomposition. It validates every candidate step with the unit's virtual passability predicate, bridge/path marker rejection, and a counted steep-slope threshold, then writes replacement direction slots only after a full candidate ordering passes.

The helper is not a boolean walkable-line test. It is direction-aware, height-aware, marker-aware, slope-aware, and it retries once with the cardinal and diagonal order swapped.

## Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| Function entry is `0x0042BE20`; the often-cited `0x0042BEC3` is the internal call to `FootClass__Get_Slope_Speed_Factor`. | `get_function_by_address 0x0042BEC3` returned body `0x0042BE20..0x0042C1B3`; disassembly `0x0042BEC3 CALL 0x004DC760`. | Yes |
| Sole direct caller is `Path_optimize_straight_segments @ 0x0042B7F0`, with two call sites. | `get_function_callers 0x0042BE20`; xrefs from `0x0042BA96` and `0x0042BC2E`. | Yes |
| Caller uses `param_7=0` for mid-window reroutes and `param_7=1` for the end-of-window sweep. | Disassembly: `0x0042BA6B PUSH 0x0 ... CALL 0x0042BE20`; `0x0042BBFD PUSH 0x1 ... CALL 0x0042BE20`. | Yes |
| Direction decomposition is `diag_count = min(abs(dx), abs(dy))`; `card_count = max(abs(dx), abs(dy)) - diag_count`. | Decompile and assembly `0x0042BE93..0x0042BEBF`. | Yes |
| First attempt uses diagonal direction first when `diag_count > 0`; if no diagonal steps exist, the first empty attempt swaps into a cardinal-only validation pass. | Decompile loop around `0x0042BEED`, skip at `0x0042BEF9..0x0042C121`, swap at `0x0042C121..0x0042C149`. | Yes |
| Diagonal direction is selected from the signs of `dx` and `dy`; cardinal direction is selected from the longer axis and the signed sum branch. | Disassembly `0x0042BE31..0x0042BE8F`. | Yes |
| Slope checks are gated by `FootClass__Get_Slope_Speed_Factor() > 1e-5`. | Call/compare `0x0042BEC3..0x0042BEE8`; `read_memory 0x007E3810` decoded to `1e-05`. | Yes |
| Slope table base is `FootClass+0x21C` (`param_5[0x87]`) and is passed to `MapClass__Get_Slope_Cost_At_Cell`. | `0x0042BED2 MOV EAX,[EBX+0x21C]`; calls at `0x0042BF85` and `0x0042C089`. | Yes |
| A cell is counted steep when `slope_cost * slope_factor >= 0.01`; the counter increments before passability rejection is finalized. | First loop `0x0042BF76..0x0042BFA3`; second loop `0x0042C07A..0x0042C0A7`; `read_memory 0x007E3808` decoded to `0.01`. | Yes |
| The per-cell passability call is virtual slot `+0x1AC` on the Foot object with arguments `(cell, dir, running_height, 0, 1)`. Return value `0` is the only accepted value. | First loop `0x0042BFA7..0x0042BFBE`; second loop `0x0042C0AB..0x0042C0C2`. | Yes |
| `CellClass+0x140 & 0x40000` rejects the candidate cell even if the virtual passability call returns `0`. | First loop `0x0042BFC0..0x0042BFCA`; second loop `0x0042C0C4..0x0042C0CE`. | Yes |
| Lenient mode accepts at most three steep cells; a fourth steep cell rejects. Strict mode (`param_7 == 0`) rejects after the first steep cell. | `CMP EAX,0x3; JG reject` and `if param_7 == 0 && steep_count > 0 reject` at `0x0042BFCC..0x0042BFE5`; mirrored at `0x0042C0D0..0x0042C0E9`. | Yes |
| Running height updates after each accepted candidate cell from signed `CellClass+0x11B`, with bridge adjustment to `cell_height + 4` only when previous height minus cell height is exactly `4` and `CellClass+0x140 & 0x100` is set. | First loop `0x0042BFE7..0x0042C006`; second loop `0x0042C0EB..0x0042C10D`. | Yes |
| On first ordering failure, the helper swaps direction/count pairs and retries once. After two failed orderings it returns `0`. | Swap/counter `0x0042C121..0x0042C149`; failure return `0x0042C14F..0x0042C158`. | Yes |
| On success, output is `local_34` copies of first direction, then `local_30` copies of second direction, then `0xFFFFFFFE` padding through the original segment length. | Success write loops `0x0042C15B..0x0042C1A8`; return `AL=1` at `0x0042C1AB`. | Yes |
| `Path_optimize_straight_segments` compacts `0xFFFFFFFE` entries after all reroute attempts, then updates path length. | Decompile/disassembly `0x0042BC3A..0x0042BC90`. | Yes |
| Active caller chain is `FootClass__Find_Path @ 0x004D3920` -> `FootClass__Run_AStar @ 0x004CBBA0` -> `AStar_pathfind_search @ 0x0042C900` -> `AStar_main_loop @ 0x00429A90` -> `Path_optimize_straight_segments @ 0x0042B7F0` -> `Path_Reroute_Straight_Line @ 0x0042BE20`. | Caller/callee Ghidra queries and decompiles listed in Sources. | Yes |

## Active YR Status

This path is live in standard Yuri's Revenge pathfinding. No TS-only or `SpecialFlags` gate was found in this slice. `AStar_main_loop` calls the smoothing pipeline whenever a successful path has more than one node, before returning the reconstructed path to `FootClass__Find_Path`.

The slope coefficient comes from the already-verified `FootClass+0x530` `ThreatAvoidanceCoefficient` copy, not from a per-cell SlopeIndex speed-factor table. This report preserves the newer `SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md` correction.

## Rust Delta

Current Rust `src/sim/pathfinding/path_smooth.rs::reroute_segment` is structurally different:

| Binary behavior | Current Rust status |
|---|---|
| Direction-array replacement with `0xFFFFFFFE` padding then caller compaction | Rust splices coordinate vectors directly. Output may be equivalent only after exact validation/order rules match. |
| Validates through virtual passability with direction and running height | Rust calls `walkable(nx, ny)` only. No direction, height, or virtual passability code is available to this helper. |
| Rejects `CellClass+0x140 & 0x40000` cells | Missing in `reroute_segment`. |
| Counts steep cells using `MapClass__Get_Slope_Cost_At_Cell` and `FootClass+0x530` | Missing in `reroute_segment`. |
| Strict mode for mid-window (`param_7=0`) rejects any steep cell; end sweep (`param_7=1`) allows up to three | Missing; Rust has no strictness flag. |
| Tries first ordering, then swaps direction/count pairs and retries once | Missing; Rust uses one fixed Bresenham-style order. |
| Maintains bridge-aware running height from `CellClass+0x11B` and `+0x140 & 0x100` | Missing in `reroute_segment`. |

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Candidate step acceptance requires `Can_Enter_Cell(cell, dir, running_height, 0, 1) == 0` and no `0x40000` marker. | `0x0042BFA7..0x0042BFCA`, `0x0042C0AB..0x0042C0CE` | mismatch | `src/sim/pathfinding/path_smooth.rs::reroute_segment` and caller-supplied validation API | Replace boolean walkable-only validation with a richer per-step validator carrying direction, layer/height, and marker state. | A reroute candidate whose target cell is walkable but marker-flagged `0x40000` must fail and fall through to swapped-order retry or no optimization. | Do not treat marker as extra cost here; this helper rejects it outright. |
| Steep-cell count increments when `Get_Slope_Cost_At_Cell(pos, foot+0x21C) * slope_factor >= 0.01`, but only when `slope_factor > 1e-5`. | `0x0042BEC3..0x0042BFA3`, `0x0042C07A..0x0042C0A7`; constants read from `0x007E3810` and `0x007E3808` | missing | `path_smooth.rs`, plus slope-cost data/model surface | Add exact slope-cost lookup and pass the unit's `ThreatAvoidanceCoefficient`-derived slope factor into reroute validation. | With slope factor `0`, steep cells do not increment the counter; with factor above `1e-5`, a cell whose product is `0.01` is counted steep. | Do not use the old refuted SlopeIndex-to-speed-factor hypothesis. |
| `param_7=0` rejects one steep cell; `param_7=1` rejects only the fourth steep cell. | Caller pushes at `0x0042BA6B` and `0x0042BBFD`; branch at `0x0042BFCC..0x0042BFE5` | missing | `optimize_path`/reroute interface | Preserve the caller distinction: mid-window calls strict, end sweep lenient. | Same candidate with one steep cell fails in mid-window mode but succeeds in end-sweep mode if no other rejection fires. | Do not model this as a single global slope threshold. |
| Failed first ordering swaps direction/count pairs and retries once. | `0x0042C121..0x0042C149` | missing | `reroute_segment` | Try diagonal-first/cardinal-second, then cardinal-first/diagonal-second; for cardinal-only or diagonal-only displacements, the empty first pass still swaps into the real second pass. | Obstacle on diagonal-first first cell but open cardinal-first path should produce an optimized replacement in Rust. | Do not use Bresenham interleaving for this helper; binary writes grouped runs, not an interleaved line. |
| Running height is updated after each step from `CellClass+0x11B`, with bridge `+4` carry only under the exact `prev_height - cell_height == 4 && flags&0x100` condition. | `0x0042BFE7..0x0042C006`, `0x0042C0EB..0x0042C10D` | missing | path smoothing validation over `PathGrid`/terrain cell metadata | Feed the next step's passability call with the binary's running height, not a static start height. | Candidate over a bridge/ramp where the second cell's passability differs by incoming height must match gamemd acceptance/rejection. | Do not collapse this to layer-only walkability. |

## Acceptance Tests

- `reroute_rejects_search_marker_0x40000`: a candidate whose first ordering enters a marker cell must reject that ordering even if normal walkability says true.
- `reroute_strict_mode_rejects_first_steep_cell`: with slope factor above `1e-5` and product exactly `0.01`, `param_7=0` fails on the first steep cell.
- `reroute_lenient_mode_allows_three_steep_cells`: the same validation in `param_7=1` succeeds through three steep cells and fails on the fourth.
- `reroute_swaps_direction_order_once`: if diagonal-first hits a blocked/marker/steep rejection but cardinal-first does not, the helper writes the cardinal-first replacement instead of abandoning optimization.
- `reroute_uses_running_bridge_height`: construct two adjacent cells where `prev_height - cell_height == 4` and bridge flag `0x100` changes the next passability height to `cell_height+4`; verify the validator sees that value.
- `optimize_compacts_deleted_slots_after_reroute`: replacement padding behaves like native `0xFFFFFFFE` compaction, preserving final path length and order.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Path_Reroute_Straight_Line @ 0x0042BE20` | verified | decompile and disassembly `0x0042BE20..0x0042C1B3` | none for scoped algorithm |
| Two call sites from `Path_optimize_straight_segments` | verified | `0x0042BA96`, `0x0042BC2E`; pushes `0` and `1` | none |
| Active caller chain from `FootClass__Find_Path` | verified | caller/callee chain `0x004D3920 -> 0x004CBBA0 -> 0x0042C900 -> 0x00429A90 -> 0x0042B7F0 -> 0x0042BE20` | none |
| `MapClass__Get_Slope_Cost_At_Cell` internal formula | touched-not-exhausted | decompile `0x0056BCD0`; existing `fn-mapclass_get_slope_cost_at_cell.md` | table production and map population belong to slot 1 |
| Virtual passability slot `+0x1AC` subclass targets | deferred | dispatch sites `0x0042BFB6`, `0x0042C0BA` | separate per-locomotor/Foot subclass audit if needed |
| Direction offset table global | touched-not-exhausted | disassembly references `g_DirectionOffsets @ 0x0089F688`; existing pathfinding docs | exact table dump not needed for this slot because formulas and current Rust direction convention are already documented |

## Open Questions -- Final State

- `[RESOLVED] OQ-001 -- Is `0x42BEC3` the function entry or an internal instruction? -> Internal slope-factor call; function entry is `0x0042BE20`.` (evidence: `get_function_by_address 0x0042BEC3`; disassembly `0x0042BEC3`)
- `[RESOLVED] OQ-002 -- Does the helper reject `0x40000` marker cells or merely penalize them? -> Rejects them outright.` (evidence: `0x0042BFC0..0x0042BFCA`, `0x0042C0C4..0x0042C0CE`)
- `[RESOLVED] OQ-003 -- What passability arguments are used? -> `(cell, dir, running_height, 0, 1)` through `FootClass` vtable `+0x1AC`.` (evidence: `0x0042BFA7..0x0042BFB6`, `0x0042C0AB..0x0042C0BA`)
- `[RESOLVED] OQ-004 -- What are the slope constants? -> enable gate `1e-5` at `0x007E3810`; steep threshold `0.01` at `0x007E3808`.` (evidence: memory read `0x007E3808..0x007E3817`; comparisons at `0x0042BECC`, `0x0042BF96`, `0x0042C09A`)
- `[RESOLVED] OQ-005 -- What does `param_7` mean? -> `0` strict allows zero steep cells; nonzero lenient allows up to three.` (evidence: `0x0042BFCC..0x0042BFE5`, `0x0042C0D0..0x0042C0E9`)
- `[RESOLVED] OQ-006 -- Which caller passes strict versus lenient? -> mid-window call passes `0`; end sweep passes `1`.` (evidence: `0x0042BA6B`, `0x0042BBFD`)
- `[RESOLVED] OQ-007 -- Does retry interleave directions or swap grouped runs? -> Swaps direction/count pairs and retries grouped validation once.` (evidence: `0x0042C121..0x0042C149`)
- `[RESOLVED] OQ-008 -- What does success write? -> grouped first-direction run, grouped second-direction run, then `0xFFFFFFFE` padding.` (evidence: `0x0042C15B..0x0042C1A8`)
- `[RESOLVED] OQ-009 -- Is the helper active in standard YR? -> Yes, on successful A* via the unconditional smoothing pipeline.` (evidence: `AStar_main_loop @ 0x00429A90`, caller chain listed above)
- `[DEFERRED] OQ-010 -- Which exact subclass functions implement vtable `+0x1AC` for every Foot subtype?` (category: out-of-scope; reason: this slot needed the reroute call contract, not every passability implementation; next-step-if-pursued: per-Foot subclass passability audit)
- `[DEFERRED] OQ-011 -- How is the slope-cost table at `SpeedType+0x59F0` populated?` (category: out-of-scope; reason: assigned to the companion slope-cost slot; next-step-if-pursued: use slot 1 report/extend `MapClass__Get_Slope_Cost_At_Cell`)

## Remaining Uncertainty

The helper algorithm itself is closed for this slice. Remaining uncertainty is deliberately outside this slot: exact subclass implementations behind virtual slot `+0x1AC`, and the upstream construction/population of slope-cost tables. Neither changes the verified reroute contract: Rust must call an equivalent direction/height-aware passability predicate and exact slope-cost lookup rather than a boolean walkability closure.

## Sources

- Ghidra decompile/disassembly: `Path_Reroute_Straight_Line @ 0x0042BE20`
- Ghidra xrefs/callers: `Path_Reroute_Straight_Line` callers from `Path_optimize_straight_segments @ 0x0042B7F0`
- Ghidra decompile/disassembly: `Path_optimize_straight_segments @ 0x0042B7F0`
- Ghidra decompile: `AStar_main_loop @ 0x00429A90`
- Ghidra decompile: `AStar_pathfind_search @ 0x0042C900`
- Ghidra decompile: `FootClass__Run_AStar @ 0x004CBBA0`
- Ghidra decompile: `FootClass__Find_Path @ 0x004D3920`
- Ghidra decompile: `FootClass__Get_Slope_Speed_Factor @ 0x004DC760`
- Ghidra decompile: `MapClass__Get_Slope_Cost_At_Cell @ 0x0056BCD0`
- Ghidra memory read: `0x007E3808..0x007E3817` (`0.01`, `1e-5`)
- Existing docs checked: `docs/research/pathfinding/fn-path_reroute_straight_line.md`, `docs/research/pathfinding/fn-path_optimize_straight_segments.md`, `docs/research/pathfinding/_parity.md`, `docs/research/SLOPE_INDEX_SPEED_FACTOR_GHIDRA_REPORT.md`
- Current Rust scan: `src/sim/pathfinding/path_smooth.rs`
