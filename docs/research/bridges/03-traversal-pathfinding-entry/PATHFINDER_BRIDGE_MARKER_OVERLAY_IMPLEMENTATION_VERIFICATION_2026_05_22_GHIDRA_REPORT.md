# Pathfinder Bridge Marker Overlay Implementation Verification - Ghidra Research Report

**Address(es):** `0x0042ACF0` (`PathfinderClass::UpdateBridgePassability`), `0x0042B080` fallback lookup, `0x00429830` (`AStar_compute_edge_cost`), `0x00429A90` (`AStar_main_loop`), `0x0042C900` (`AStar_pathfind_search`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** implementation-critical verification for the temporary `CellClass+0x140 & 0x40000` A* bridge/crowd marker overlay: probe source, peer replay geometry, 5x5 fallback, cleanup tails, and edge-cost placement.
**Non-Scope:** full A* algorithm, full zone retry semantics, runtime exception/process-abort cleanup, exact semantic name of `TechnoType/Object+0x678`, and persistent bridge zone/pathgrid rebuild behavior.
**Confidence:** High for static binary behavior in the claimed slice.
**Active in YR:** Yes, conditional on normal A* path searches where `PathfinderClass+0x3C != 0` and `PathfinderClass+0x03 != 0`.

## 1. Overview

This pass verifies the exact details needed before implementing the search-scoped bridge marker overlay in Rust. The original engine temporarily XOR-toggles `CellClass+0x140 bit 0x40000` before A*, lets the A* cost helper treat marked destination cells as `4.0x` cost, then runs the same toggler again on normal success/failure cleanup tails.

The important implementation conclusion is that this is not static pathgrid state, not passability, not RNG, and not a generic cliff/ramp multiplier. It is a temporary A* cost overlay whose shape is derived from a timer-selected probe, peer path queues, and occupied cells around the probe.

## 2. Key Offsets And Constants

| Offset / constant | Verified meaning in this slice | Evidence |
|---|---|---|
| `PathfinderClass+0x03` | Master enable byte for `UpdateBridgePassability`; zero returns before writes | `0x0042ACF3..0x0042AD00` |
| `PathfinderClass+0x3C` | Search urgency/retry mode; gates pre/post A* toggler calls and internal early zeroing | `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A42D`, `0x0042A442..0x0042A44C`, `0x0042AEB9..0x0042AED5` |
| `FootClass+0x388` | RateTimer used to derive probe direction | `0x0042AD35..0x0042AD4D` |
| `FootClass+0x8C` | On-bridge byte used in probe layer/list choice | `0x0042ADB8..0x0042ADC0` |
| `Object+0x30` | Next object in selected cell object list | `0x0042AE9E`, `0x0042AFBF` |
| `Object+0x558` | Peer replay starting coordinate | `0x0042AE33..0x0042AE3B` |
| `Object+0x5E0` | Peer direction queue base | `0x0042AE88`, `0x0042AEF6..0x0042AFB5` |
| `Cell+0xE4` / `Cell+0xE8` | Ground / bridge object-list heads | `0x0042ADCC`, `0x0042ADC2`, `0x0042986A..0x00429872` |
| `Cell+0x116` | Tube index used when queued direction is `8`; `-1` resets coord to `(0,0)` in marker replay | `0x0042AF15..0x0042AF3F` |
| `Cell+0x124` | Occupation byte gating 5x5 candidate toggles | `0x0042B005..0x0042B00D` |
| `Cell+0x140 & 0x100` | Structural bridge-cell bit used for layer/list selection | `0x0042AD93..0x0042ADCC`, `0x0042B080` |
| `Cell+0x140 & 0x40000` | Temporary A* marker bit; writer and reader verified | writers `0x0042AF93..0x0042AFAE`, `0x0042B029..0x0042B03D`, `0x0042B04F..0x0042B063`; reader `0x004299AA..0x004299C2` |
| `0x18` | Max peer path replay entries, i.e. 24 | `0x0042AEF6..0x0042AEF9` |
| `4.0f` | Marker edge-cost multiplier | `0x004299B8..0x004299C2`, global `0x007E37BC` |

## 3. Core Logic

### 3.1 Probe Direction Is Timer-Derived, Not RNG

`UpdateBridgePassability` gets the searching unit's current cell through vtable `+0x1B8`, then calls `RateTimer__Current` on `FootClass+0x388`. The probe direction formula is:

```text
dir = (((RateTimerCurrent(Foot+0x388) >> 12) + 1) >> 1) & 7
probe = current_cell + g_DirectionOffsets[dir]
```

Evidence: `0x0042AD0F..0x0042AD35` obtains current cell and timer; `0x0042AD40..0x0042AD4D` applies `>> 12`, `+1`, `>> 1`, `& 7`; `0x0042AD50..0x0042AD80` adds `g_DirectionOffsets`.

Implementation impact: Rust must not use `SimRng`, movement facing, destination direction, or queue direction for this probe.

### 3.2 Probe Layer/List Selection Uses Bridge Bit, Signed Level Gap, And On-Bridge Byte

After selecting the probe cell, the function reads `Cell+0x140 & 0x100`. If that bridge bit is not set, it uses `E4`. If the bridge bit is set, it compares signed byte levels from current cell and probe cell. The bridge list `E8` is selected only when absolute level gap is greater than `3`, or when the searching foot's `+0x8C` byte is nonzero. Otherwise it uses `E4`.

Evidence: `0x0042AD93..0x0042ADCC`; the boundary is `CMP EAX,0x3; JG 0x0042ADC2`, so the bridge-list threshold is `> 3`, not `>= 3`.

Implementation impact: layer choice must be based on the probe cell plus the mover's on-bridge state. Do not infer it solely from the current Rust movement layer.

### 3.3 Null Probe List Falls Back To A Height-Aware 5x5 Object Lookup

If the chosen `E4/E8` list head is null, `UpdateBridgePassability` calls `FUN_0042B080(probe_cell + 0x24, probe_level + (bridge_list ? 4 : 0))`.

`FUN_0042B080` scans offsets `[-2..2]` in both axes around the original probe coordinate. For each candidate cell it chooses candidate `E4/E8` from bridge bit and `abs(candidate.level - requested_height) < 3`. It then walks object `+0x30`, requires object state bit `((object+0x14) >> 2) & 1`, asserts `object+0x674` non-null, and calls the subobject vtable `+0xA0` with the original probe center in leptons `(x*256+128, y*256+128, requested_height * DAT_0089C2D8)`.

Evidence: `0x0042ADD4..0x0042ADF6` call setup; `FUN_0042B080` decompile, especially loop bounds, layer choice, object state bit, `0x674`, `+0xA0`, and lepton center constants.

Implementation impact: this fallback is object discovery for peer replay, not the same thing as the later 5x5 marker fallback. The later marker fallback still uses the original probe cell.

### 3.4 Peer Path Replay Starts At `path[0]` And Caps At 24 Entries

Eligible peer objects must be object kind `1` or `0xF`. The function loads the peer replay start coordinate from `peer+0x558` and direction queue base `peer+0x5E0`.

Kind `1` requires `path[0] != -1` and `path[1] != -1`, then processes from `path[0]`. Kind `0xF` requires `path[0]`, `path[1]`, and `path[2]` all non-`-1`, then also processes from `path[0]`.

The loop stops before processing more than 24 entries and also stops when the next queue dword is `-1`.

Evidence: kind gate `0x0042AE15..0x0042AE2D`; replay start `0x0042AE33..0x0042AE3B`; queue base `0x0042AE88`; kind-1 prerequisites `0x0042AE90..0x0042AE9C`; kind-`0xF` prerequisites `0x0042AED8..0x0042AEEF`; 24-entry cap and terminator `0x0042AEF6..0x0042AFB5`.

Implementation impact: do not skip `path[0]`; do not collapse kind `1` and kind `0xF` prerequisites.

### 3.5 Direction `8` Is Tube Handling, Not Compass Direction

For peer replay direction values `0..7`, the function adds `g_DirectionOffsets[dir]`. For direction `8`, it gets the current replay cell, reads `Cell+0x116`, and either:

- jumps to `g_TubeArray[tube_index]+0x28` when the tube index is not `-1`, or
- resets replay coordinate to `(0,0)` when the tube index is `-1`.

Evidence: direction-8 branch `0x0042AF01..0x0042AF3F`; normal direction branch `0x0042AF41..0x0042AF69`.

Implementation impact: do not `& 7` direction `8` in peer marker replay.

### 3.6 Peer Replay Toggles The Destination Cell Bit

After each replay step, the function calls `MapClass::Get_CellClass` twice on the same updated coordinate, then writes:

```text
dest.flags ^= ((~same_dest.flags ^ dest.flags) & 0x40000)
```

Because both lookups are for the same updated coordinate in this sequence, the `0x40000` effect degenerates to a destination-cell XOR toggle.

Evidence: coordinate write `0x0042AF69`; duplicate cell lookups `0x0042AF6D..0x0042AF88`; masked write `0x0042AF8D..0x0042AFAE`.

Implementation impact: use XOR-like parity semantics for duplicate marks. Two visits to the same destination cancel for this marker bit.

### 3.7 Later 5x5 Marker Fallback Is Occupation-Gated And Center-Toggled

If at least one peer path was processed, execution reaches the 5x5 marker phase. If no peer path was processed and `Pathfinder+0x3C == 1`, the function writes `Pathfinder+0x3C = 0` and returns before marker writes. Otherwise it also reaches the 5x5 marker phase.

The marker phase is centered on the probe cell:

- iterate `dx=-2..2`, `dy=-2..2`;
- candidate is `probe + (dx, dy)`;
- only candidates with `Cell+0x124 != 0` are considered;
- the searching unit's original current cell is skipped;
- considered candidates XOR-toggle `0x40000`;
- after the loop, the probe-center cell XOR-toggles `0x40000` unconditionally.

Evidence: no-peer early zero `0x0042AEB1..0x0042AED5`; loop setup `0x0042AFCB..0x0042AFFC`; occupation/self tests `0x0042B005..0x0042B027`; candidate toggle `0x0042B029..0x0042B03D`; loop bounds `0x0042B043..0x0042B04D`; unconditional center toggle `0x0042B04F..0x0042B063`.

Implementation impact: do not mark all 25 cells. The center has special parity: an occupied center toggles once in the candidate loop and once after the loop, cancelling; an unoccupied center toggles once after the loop.

### 3.8 Normal A* Cleanup Tails Pair The Pre-Search Toggle

`AStar_main_loop` calls `UpdateBridgePassability` before the search only when start/destination/height are not already equal and `Pathfinder+0x3C != 0`. Normal success and failure/no-result tails both check `+0x3C` and call `UpdateBridgePassability` again before returning.

Evidence:

- pre-toggle gate `0x00429BF3..0x00429C1A`;
- same-cell/same-height zero return before toggle `0x00429BF3..0x00429C0A -> 0x0042A451`;
- success cleanup `0x0042A423..0x0042A43B`;
- failure cleanup `0x0042A43E..0x0042A45A`.

`AStar_pathfind_search` wraps `AStar_main_loop`; the ordinary post-loop retry and return paths consume an already-cleaned result. Early returns before `AStar_main_loop` do not toggle anything.

Evidence: `AStar_pathfind_search` decompile, especially main-loop call and retry path around `0x0042CC02..0x0042CCCB`.

Implementation impact: Rust can model this as a search-scoped overlay value constructed before A* and dropped after A*, rather than mutating persistent map cells.

### 3.9 Marker Cost Placement

`AStar_compute_edge_cost` loads the base cost table, handles code-2 friendly/moving blocker prediction and urgency override, then checks destination `Cell+0x140 & 0x40000`. If set, it multiplies the current edge accumulator by `4.0`. Bridge flank multipliers happen after the marker. Direction epsilon is not in this helper; the caller adds it after multiplying helper return by `Pathfinder+0x04`.

Evidence:

- base table load `0x00429845..0x00429854`;
- code-2 branch and urgency handling `0x0042985C..0x004299A6`;
- marker multiply `0x004299AA..0x004299C2`;
- bridge flank multiplier branch `0x004299C6..0x00429A79`;
- caller helper call / `+0x04` multiply / epsilon add `0x00429F8A..0x00429F9D`;
- direction-8 bypass of helper `0x00429F6B..0x00429FA3`.

Implementation impact: marker cost must be applied to normal compass edges after entity/code cost and before bridge flank cost, while leaving the final direction tiebreak additive. It must not apply to direction-8 tube edges.

## 4. INI Keys

No direct INI key controls `0x40000` marker geometry or multiplier in this slice. The behavior is pathfinder/runtime state driven. `BlockagePathDelay` and related urgency setup remain upstream of `Pathfinder+0x3C`, but this verification did not re-investigate their parsing.

## 5. Integration Points

| Function | Role | Active in YR |
|---|---|---|
| `AStar_pathfind_search @ 0x0042C900` | Search wrapper; sets `Pathfinder+0x3C`, calls `AStar_main_loop`, handles retry | Yes |
| `AStar_main_loop @ 0x00429A90` | Owns pre-search toggler call and normal cleanup tails | Yes |
| `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` | Generates and XOR-toggles temporary marker geometry | Yes, conditional |
| `FUN_0042B080 @ 0x0042B080` | Height-aware object fallback lookup when probe list is null | Yes, conditional |
| `AStar_compute_edge_cost @ 0x00429830` | Consumes marker by multiplying current edge accumulator by `4.0` | Yes |

## 6. Current Rust Implementation Status

| Rust area | Current status vs verified behavior | Evidence |
|---|---|---|
| `src/sim/pathfinding/core.rs` | Has `AStarOptions`, entity soft-block costs, bridge/layer logic, direction tiebreaks, and separate tube edge branch; no search-scoped `0x40000` marker overlay found | `rg`; `AStarOptions`, normal edge cost, `TUBE_DIR_TIEBREAK` |
| `src/sim/movement/bump_crush.rs` | Builds `LayeredEntityBlockMap` from entity positions and next movement target cell; not a 24-entry peer path replay overlay | `rg`; `build_entity_block_set` |
| `src/sim/movement/movement_path.rs` and callers | Pass entity block maps into zoned pathfinding; no marker overlay input found | `rg` |
| `src/sim/pathfinding/zone_search.rs` | Retry/corridor pathfinding exists, but this report's marker overlay is lower-level A* cost input, not a zone-grid feature | `rg` |
| `src/app_sim_tick.rs` / bridge state | Persistent grid rebuild surfaces should not store the marker | prior reports plus Rust scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Probe timer formula | verified | `0x0042AD35..0x0042AD4D` | none |
| Probe layer/list selection | verified | `0x0042AD93..0x0042ADCC` | none |
| Null probe-list fallback object lookup | verified | `0x0042ADD4..0x0042ADF6`, `FUN_0042B080` | identity of `object+0x674` subobject is out-of-scope |
| Peer kind gates and prerequisites | verified | `0x0042AE15..0x0042AE2D`, `0x0042AE90..0x0042AEEF` | exact human-readable names for kind `1` and `0xF` rely on existing object-class research |
| Peer replay coordinate start and queue base | verified | `0x0042AE33..0x0042AE3B`, `0x0042AE88` | none |
| Peer replay direction `8` | verified | `0x0042AF01..0x0042AF3F` | full TubeClass internals out-of-scope |
| Peer replay XOR destination toggle | verified | `0x0042AF69..0x0042AFAE` | none |
| 5x5 marker fallback | verified | `0x0042AEB1..0x0042B063` | none |
| A* normal cleanup tails | verified | `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A45A` | abnormal interruption cleanup remains out-of-scope |
| Marker cost placement | verified | `0x00429845..0x00429A79`, `0x00429F8A..0x00429F9D` | none |
| Direction-8 cost bypass | verified | `0x00429F6B..0x00429FA3` | none |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is fresh Ghidra MCP available for the previously caveated probe/replay facts? -> Yes; direct decompile/disassembly verified the facts in this report.` (evidence: `PathfinderClass__UpdateBridgePassability`, `FUN_0042B080`)
- `[RESOLVED] OQ-2 - Is the probe based on RNG or a timer? -> Timer; `RateTimer__Current(Foot+0x388)` with `(((v >> 12) + 1) >> 1) & 7`.` (evidence: `0x0042AD35..0x0042AD4D`)
- `[RESOLVED] OQ-3 - What is the probe layer threshold? -> Bridge list if structural bridge bit is set and absolute signed level gap is `> 3`, or mover `+0x8C` is nonzero.` (evidence: `0x0042AD93..0x0042ADCC`)
- `[RESOLVED] OQ-4 - Does peer replay start at `path[0]`? -> Yes, after kind-specific prerequisites.` (evidence: `0x0042AE88..0x0042AEF1`)
- `[RESOLVED] OQ-5 - What is the peer replay cap? -> 24 processed entries maximum, also terminates on next `-1`.` (evidence: `0x0042AEF6..0x0042AFB5`)
- `[RESOLVED] OQ-6 - Is direction `8` a compass direction? -> No; it is tube handling with tube-index lookup or `(0,0)` fallback.` (evidence: `0x0042AF01..0x0042AF3F`)
- `[RESOLVED] OQ-7 - Does peer replay toggle source/dest alternation? -> No for this bit; duplicate lookups use the same updated destination coordinate, producing destination XOR parity.` (evidence: `0x0042AF69..0x0042AFAE`)
- `[RESOLVED] OQ-8 - What exactly does the 5x5 marker fallback mark? -> Occupied non-own candidate cells in a probe-centered `[-2..2]` square, plus unconditional probe-center toggle after the loop.` (evidence: `0x0042AFCB..0x0042B063`)
- `[RESOLVED] OQ-9 - Are normal A* success/failure exits paired with cleanup? -> Yes, in `AStar_main_loop`.` (evidence: `0x0042A423..0x0042A45A`)
- `[RESOLVED] OQ-10 - Where does the marker cost stack? -> After base/code-2 handling, before bridge flank multipliers; caller epsilon added later.` (evidence: `0x00429845..0x00429A79`, `0x00429F8A..0x00429F9D`)
- `[RESOLVED] OQ-11 - Does the marker apply to direction-8 tube edges? -> No; direction `8` bypasses `AStar_compute_edge_cost`.` (evidence: `0x00429F6B..0x00429FA3`)
- `[DEFERRED] OQ-12 - Does abnormal runtime interruption always clean up real `CellClass` flags?` (category: `needs-runtime-debugger`; reason: static normal return paths are paired, but exceptions/process aborts are not ordinary control flow; next-step-if-pursued: runtime watchpoint on `Cell+0x140 & 0x40000` through interrupted pathfinding)
- `[DEFERRED] OQ-13 - What is the exact semantic name of `+0x678` used in peer priority gating?` (category: `out-of-scope`; reason: verified comparison is enough for marker overlay behavior; next-step-if-pursued: dedicated TechnoType/object field investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Probe direction is derived from `RateTimerCurrent(Foot+0x388)`, not RNG or movement heading | `0x0042AD35..0x0042AD4D` | missing | path request / movement state surface feeding `core.rs` options | Provide deterministic timer-derived probe direction for marker overlay generation | `astar_bridge_marker_probe_uses_ratetimer_bucket_not_rng` | Do not use `SimRng`, facing, destination direction, or path queue direction |
| Probe object-list layer uses probe structural bridge bit, signed level gap `> 3`, and mover `+0x8C` | `0x0042AD93..0x0042ADCC` | missing | overlay builder plus bridge/layer query surface | Select `E4` vs `E8` equivalent using the verified probe-cell rule | `astar_bridge_marker_probe_layer_selects_e4_e8_with_gap_gt3` | Do not use `>= 3`; do not infer only from current Rust path layer |
| Peer replay starts at `Object+0x558`, processes directions from `Object+0x5E0`, and starts with `path[0]` after kind-specific prerequisites | `0x0042AE33..0x0042AEF1` | missing; current entity block map only uses current/next movement target cells | movement snapshot / peer path snapshot surface, then `core.rs` overlay input | Build marked cell parity from peer queued paths, preserving kind `1` vs `0xF` prerequisites | `astar_bridge_peer_marker_kind_prerequisites_start_at_path0` | Do not skip `path[0]`; do not collapse prerequisites |
| Peer replay caps at 24 entries and XOR-toggles destination cells, so duplicates cancel | `0x0042AEF6..0x0042AFB5`, `0x0042AF69..0x0042AFAE` | missing | overlay storage should support parity/count cancellation | Use parity semantics for repeated marks and cap replay at 24 | `astar_bridge_peer_marker_replay_caps_at_24_and_xors_duplicates` | Do not model as hard occupancy or monotonic set unless duplicates are known impossible |
| Direction `8` in peer replay uses tube exit or `(0,0)` fallback | `0x0042AF01..0x0042AF3F` | unchecked/missing | tube path metadata exposure if bridge/tube paths can appear in Rust peer queues | Treat direction `8` specially in overlay replay | `astar_bridge_peer_marker_direction8_uses_tube_exit_or_origin` | Do not mask direction `8` with `& 7` |
| 5x5 marker fallback is probe-centered, occupation-gated, skips own current cell, and toggles center afterward | `0x0042AEB1..0x0042B063` | missing | overlay builder plus occupation query surface | Add the verified 5x5 fallback in the same no-peer/urgency conditions | `astar_bridge_marker_5x5_probe_center_and_own_cell_rules` | Do not mark all 25 cells unconditionally |
| Marker multiplies current normal-edge accumulator by `4.0` after code-2 handling and before bridge flank multiplier | `0x00429845..0x00429A79` | missing | `src/sim/pathfinding/core.rs` normal compass edge cost | Apply marker multiplier at the verified point in Rust integer cost scale | `astar_edge_cost_marker_stacks_after_code2_before_bridge_flank` | Do not implement as terrain/cliff height cost or passability rejection |
| Direction tiebreak/epsilon is added after helper return and is not multiplied by marker | `0x00429F8A..0x00429F9D` | must preserve when marker is added | `core.rs` final `DIR_TIEBREAK` addition | Keep marker inside step-cost multiplier path, with tiebreak still final additive term | `astar_marker_cost_does_not_scale_direction_tiebreak` | Do not fold `DIR_TIEBREAK` into a value later multiplied by marker |
| Direction-8 tube edge bypasses marker cost helper | `0x00429F6B..0x00429FA3` | current Rust tube branch separate; future marker must preserve this | `core.rs` tube edge branch | Restrict marker multiplier to normal compass edges | `astar_marker_overlay_does_not_apply_to_direction8_tube_edge` | Do not globally post-process all destination costs |
| Normal pre/post A* calls cancel marker writes | `0x00429C10..0x00429C1A`, `0x0042A423..0x0042A45A` | Rust should avoid persistent mutation entirely | `core.rs` and pathfinding call surfaces | Model as a per-search overlay value dropped after search | `astar_bridge_marker_overlay_is_search_scoped_and_does_not_mutate_pathgrid` | Do not store marker in `PathGrid`, bridge state, zone grid, or save state |

### Stale Docs / Follow-up Docs

- `docs/research/UPDATEBRIDGEPASSABILITY_PROBE_RNG_0042AD35_GHIDRA_REPORT.md`: strict-tool caveat is now resolved for the implementation-critical facts by this fresh Ghidra MCP verification. Keep any broader caveats that were outside this slice.
- `docs/research/UPDATEBRIDGEPASSABILITY_PEER_PATH_PROPAGATION_0042AEF6_GHIDRA_REPORT.md`: strict-tool caveat is now resolved for start/prerequisite/cap/direction-8/destination-XOR facts by this report.
- `docs/research/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md`, `PATHFINDING_ASTAR_GHIDRA_REPORT.md`, and `ADDRESS_MAP.md`: replace any `0x40000` cliff-ramp wording with: "`0x40000` is the temporary A* marker multiplier; destination marked cells multiply the current normal-edge accumulator by `4.0` after code-2 handling and before bridge flank multipliers."

## Sources

- Ghidra MCP decompile: `PathfinderClass__UpdateBridgePassability`
- Ghidra MCP decompile: `FUN_0042B080`
- Ghidra MCP decompile: `AStar_compute_edge_cost`
- Ghidra MCP decompile/disassembly: `AStar_main_loop`
- Ghidra MCP decompile: `AStar_pathfind_search`
- Ghidra MCP disassembly: `ram:0042ACF0`, `ram:00429830`, `ram:00429A90`
- Prior docs: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`, `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`, `ASTAR_COMPUTE_EDGE_COST_00429830_MARKER_STACKING_GHIDRA_REPORT.md`
- Rust scan: `src/sim/pathfinding/core.rs`, `src/sim/movement/bump_crush.rs`, `src/sim/movement/movement_path.rs`, `src/sim/pathfinding/zone_search.rs`
