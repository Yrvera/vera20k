# Layered Bridge Path Entry Foundation Scope — Ghidra Research Report

**Address(es):** `AStar_pathfind_search @ 0x0042C900`, `AStar_main_loop @ 0x00429A90`, `MapClass__GetZoneID @ 0x0056D230`, `MapClass__ResolvePathCoord_BridgeAware @ 0x00583180`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Whether a flat/no-slope `Zone_precheck` foundation can be implemented before layered bridge path integration, and what it may claim.  
**Non-Scope:** Full cell edge costs, full smoothing, full bridge collapse state machines, exact stock-map route oracle, exact slope helper internals.  
**Confidence:** High for path entry and bridge-aware zone/coordinate handoff; Medium for current Rust player-visible coverage because call-site wiring can change.  
**Active in YR:** Yes.

## Target Question

Can Rust implement the flat ground `Zone_precheck` foundation first, or must the layered bridge pathing entry be rewritten in the same first patch?

## Answer

Foundation First is valid only as a scoped foundation/library step: flat/no-slope synthetic `Zone_precheck` and data-model tests can be implemented first if the work explicitly does not claim player-visible high-bridge route parity.

Player-visible bridge-capable route parity still requires layered integration later. `gamemd` does not have Rust's split between "flat zoned path" and "layered path"; normal foot path entry reaches one `AStar_pathfind_search` path that performs bridge-aware start/destination zone lookup, bridge-aware coordinate resolution, `Zone_precheck`, and bridge/ground marker-gated cell A* in one flow.

## Non-Goals

- No claim about exact Carville or other stock-map detour selection.
- No claim about full retry-edge producer parity beyond the path-entry dependency.
- No claim about diagonal bridge edge cost parity.
- No claim about low-bridge collapse state machines.

## Evidence Needed To Mark COMPLETE

- Verify normal foot pathing reaches `AStar_pathfind_search` in standard YR.
- Verify `AStar_pathfind_search` performs bridge-aware zone/coordinate resolution before `Zone_precheck`.
- Verify `AStar_main_loop` consumes hierarchy/zone markers together with bridge/ground closed lists.
- Verify high-vs-low bridge layer implications for zone resolution.
- Compare current Rust flat and layered path entry behavior.

## Stop Conditions

- Stop at the first confirmed answer to the scope question.
- Do not chase full `Can_Enter_Cell`, smoothing, or cell cost tables unless needed to answer whether layered integration is mandatory in the first patch.
- Do not modify Rust or INI files.

## Core Findings

### 1. Normal YR foot pathing reaches one bridge-aware `AStar_pathfind_search`

`FootClass__Run_AStar @ 0x004CBBA0` calls `AStar_pathfind_search @ 0x0042C900` after walking the current path queue to the current start cell. The call passes the foot object, movement-zone override `-1`, and the hierarchy flag argument.

**Active in YR:** Yes. Evidence: `FootClass__Run_AStar @ 0x004CBBA0` decompile calls `AStar_pathfind_search`; prior doc `ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md` also records this as live through standard `FootClass__Run_AStar`.

### 2. `AStar_pathfind_search` performs bridge-aware zone and coordinate resolution before `Zone_precheck`

The path entry resolves:

- source zone with `MapClass__GetZoneID(start, movement_zone, foot+0x8c on_bridge byte)`;
- destination zone with `MapClass__GetZoneID(dest, movement_zone, (dest_cell+0x140 >> 8) & 0x...01)`;
- source logical coordinate with `MapClass__ResolvePathCoord_BridgeAware(start_cell, foot_on_bridge)`;
- destination logical coordinate with `MapClass__ResolvePathCoord_BridgeAware(dest_cell, dest_bridge_flag_bit)`;
- then calls `Zone_precheck` when hierarchy is enabled.

**Active in YR:** Yes. Evidence: `AStar_pathfind_search @ 0x0042C900`, decompile around the two `MapClass__GetZoneID` calls and two `MapClass__ResolvePathCoord_BridgeAware` calls before `Zone_precheck @ 0x0042CB58`; assembly context confirms the call at `0x0042CB58`.

### 3. High bridge cells use bridge-aware redirect; low bridge records are not used by `FindBridgeRecord`

`MapClass__GetZoneID @ 0x0056D230` only enters bridge redirect when the caller passes the bridge-layer bool and the cell has flag `0x100`. It calls `MapClass__FindBridgeRecord(coord, 1, 0)`. `MapClass__FindBridgeRecord @ 0x0056DA10` skips records where bridge-kind `+0x0C != 0`, so the redirect path is high-bridge only. Low-bridge records exist from `MapClass__ComputeBridgeZones @ 0x0056D6E0` as kind `1`, but are not selected by this high-bridge redirect helper.

**Active in YR:** Yes. Evidence: `MapClass__GetZoneID @ 0x0056D230`; `MapClass__FindBridgeRecord @ 0x0056DA10`; `MapClass__ComputeBridgeZones @ 0x0056D6E0`.

### 4. Cell A* marker consumption is intertwined with bridge/ground closed lists

`AStar_main_loop @ 0x00429A90` reads the level-0 zone marker array from `Zone_precheck`, then branches into bridge/ground marker/closed-list handling. When the candidate's level-0 zone was marked by precheck, it can proceed directly. Otherwise, the hierarchy flag plus `cell+0x122` gate decides whether the candidate is rejected before the per-layer closed/g-cost arrays are consulted.

The same loop uses separate arrays at Pathfinder offsets `+0x18/+0x24` and `+0x1C/+0x20` for the two height/layer cases. That means marker handoff is not just a flat `BTreeSet<ZoneId>` corridor.

**Active in YR:** Yes. Evidence: `AStar_main_loop @ 0x00429A90` decompile around the `ZoneMap__CellToZoneIndex` marker test and the `LAB_00429EC7` / `LAB_00429F04` split; assembly context at `0x00429EC7` and `0x00429F04`.

### 5. Current Rust separates flat zoned search from layered bridge search

Current Rust has:

- `src/sim/pathfinding/zone_search.rs::find_path_zoned_marker`: flat ground only; uses reachability, corridor Dijkstra, `expand_corridor`, and corridor-restricted A*.
- `src/sim/pathfinding/zone_search.rs::find_layered_path_zoned_marker`: layered bridge-capable; currently only does a reachability precheck, then calls full layered A* without marker-path/corridor handoff.
- `src/sim/movement/movement_path.rs::find_move_path_with_marker`: bridge-capable Drive/Walk/Mech movers prefer the layered branch; if it fails, Rust falls back to flat A* unless the goal is bridge-only.
- `src/sim/movement/movement_commands.rs::issue_move_command_with_layered`: ordinary initial move commands currently pass `zone_grid: None`, so initial player move commands do not exercise the zoned foundation unless that wiring changes.

**Active in YR:** Not applicable to binary; current Rust evidence. Evidence: `src/sim/pathfinding/zone_search.rs`, `src/sim/movement/movement_path.rs`, `src/sim/movement/movement_commands.rs`.

## Current Rust Implementation Status

Foundation First can add binary-style hierarchy records and a flat/no-slope precheck surface without touching layered bridge A* immediately. That work is useful because the same data model is needed later.

It must be explicitly scoped as:

- synthetic/flat pathing foundation;
- no claim for high-bridge player route parity;
- no claim for the actual movement-command path until `zone_grid` is wired into command/repath call sites that need it;
- no claim that `find_layered_path_zoned_marker` has `gamemd` marker-path handoff.

## Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass__Run_AStar -> AStar_pathfind_search` | verified | `0x004CBBA0`, `0x0042C900` | none for entry activation |
| `AStar_pathfind_search` bridge-aware zone/coord resolution | verified | `0x0042C900`, `0x0056D230`, `0x00583180` | exact destination flag mask naming |
| `Zone_precheck` call placement | verified | `0x0042CB58`, `0x0042CCB3` | none for placement |
| `AStar_main_loop` marker/closed-list interaction | touched-not-exhausted | `0x00429A90`, `0x00429EC7`, `0x00429F04` | full cell expansion parity is separate |
| Low bridge path layer behavior | touched-not-exhausted | `0x0056D230`, `0x0056DA10`, `PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md` | exact stock low-bridge route remains runtime-trace gated |
| Current Rust initial move zoned wiring | verified | `movement_commands.rs` passes `zone_grid: None` | future integration decision |
| Current Rust layered path handoff | verified | `zone_search.rs::find_layered_path_zoned_marker` | binary marker-path integration missing |

## Open Questions — Final State

- `[RESOLVED] OQ-1 — Is normal pathing active through this entry? -> Yes, `FootClass__Run_AStar` reaches `AStar_pathfind_search`.` (evidence: `0x004CBBA0`, `0x0042C900`)
- `[RESOLVED] OQ-2 — Does path entry know whether the unit starts on bridge? -> Yes, source `GetZoneID` and source coordinate resolution receive `foot+0x8c`/`param_4[0x23]`.` (evidence: `0x0042C900`)
- `[RESOLVED] OQ-3 — Does destination bridge state affect precheck coordinates? -> Yes, destination uses `dest_cell+0x140` bridge bit-derived bool for zone and coordinate resolution.` (evidence: `0x0042C900`)
- `[RESOLVED] OQ-4 — Is low bridge redirected by `FindBridgeRecord`? -> No for this high-bridge redirect helper; `FindBridgeRecord` skips kind `1` low records.` (evidence: `0x0056DA10`, `0x0056D6E0`)
- `[RESOLVED] OQ-5 — Can flat Rust foundation be implemented without rewriting layered path immediately? -> Yes, as a scoped foundation, but it must not claim player-visible bridge route parity.` (evidence: current Rust split plus binary unified path entry)
- `[DEFERRED] OQ-6 — Exact retry-edge producer behavior for layered failures` (category: `out-of-scope`; reason: assigned to another reswarm slot; next-step-if-pursued: investigate `PathfinderClass__UpdateHierarchicalEdges` / `ZoneMap__FloodFillReachableZones`)
- `[DEFERRED] OQ-7 — Full cell edge cost parity on high bridge diagonal moves` (category: `out-of-scope`; reason: explicitly excluded; next-step-if-pursued: use bridge A* cost reports)
- `[DEFERRED] OQ-8 — Exact stock-map route after bridge collapse` (category: `needs-runtime-debugger`; reason: static evidence cannot determine rebuilt graph order/path cells; next-step-if-pursued: runtime route trace)

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Flat foundation can be built first only as foundation scope | `0x0042C900`; `zone_search.rs` | missing binary-style hierarchy/precheck data model | `src/sim/pathfinding/zone_map.rs`, future hierarchy/precheck module, `zone_search.rs` | Add ordered hierarchy/edge metadata and flat/no-slope synthetic precheck without claiming layered parity | `zone_precheck_flat_foundation_rejects_off_parent_child_path` | Do not advertise as fixing bridge-capable movement commands |
| Bridge-capable player route parity needs layered integration later | `0x0042C900`, `0x00429A90`, `0x00583180` | `find_layered_path_zoned_marker` only reachability-prechecks then runs full layered A* | `src/sim/pathfinding/zone_search.rs`, `src/sim/pathfinding/core.rs`, `src/sim/movement/movement_path.rs` | Feed binary-style precheck markers into layered A* and remove approximation/fallback claims for bridge paths | `layered_zone_precheck_markers_gate_high_bridge_route` | Do not rely on flat `find_path_zoned_marker` for high-bridge routes |
| Low bridge is mostly ground-layer for path-entry redirect, while high bridge uses bridge-aware redirect | `0x0056D230`, `0x0056DA10`, low-ramp trace | Rust bridge redirect currently uses high-active-only for `zone_at(Bridge)`; low bridge adjacency remains graph/tube data | `src/sim/pathfinding/zone_build.rs`, `zone_map.rs` | Preserve high-only bridge-layer redirect and low-bridge ground/tube connectivity distinction when adding hierarchy | `zone_get_high_bridge_redirect_ignores_low_bridge_records` | Do not make low bridge use high bridge redirect just because both are bridge records |

## Negative Facts / Do Not Do

- Do not claim Foundation First fixes player-visible high-bridge route parity.
- Do not route `find_layered_path_zoned_marker` through flat `find_path_zoned_marker`; high bridge route choice depends on bridge-aware start/destination layer resolution.
- Do not treat low bridge records as high-bridge `FindBridgeRecord` redirect records.
- Do not keep the flat fallback from layered failure as a parity claim; it is a Rust convenience path, not the verified `gamemd` contract.
- Do not write stock-map route assertions from this report.

## Proposed Tests

- `zone_precheck_flat_foundation_rejects_off_parent_child_path`
- `zone_get_high_bridge_redirect_ignores_low_bridge_records`
- `layered_zone_precheck_markers_gate_high_bridge_route`
- `initial_move_command_with_zone_grid_exercises_precheck_when_wired`
- `layered_failure_does_not_fallback_to_flat_for_bridge_only_goal`

## Stale Docs / Replacement Wording

If a doc says layered bridge pathing can be deferred while still fixing bridge route parity, replace with:

> Flat/no-slope `Zone_precheck` foundation may be implemented first as a data-model and synthetic-test step. It does not claim player-visible high-bridge route parity until `find_layered_path_zoned_marker` consumes binary-style precheck markers/retry output.

If a doc implies low bridge records should use the high-bridge redirect path, replace with:

> `MapClass__GetZoneID` bridge redirect calls `FindBridgeRecord`, which skips kind `1` low bridge records. Low bridge connectivity belongs to ground/tube/zone graph handling, not the high-bridge endpoint redirect.

## Sources

- Ghidra: `FootClass__Run_AStar @ 0x004CBBA0`
- Ghidra: `AStar_pathfind_search @ 0x0042C900`
- Ghidra: `Zone_precheck @ 0x0042C290`
- Ghidra: `AStar_main_loop @ 0x00429A90`
- Ghidra: `MapClass__GetZoneID @ 0x0056D230`
- Ghidra: `MapClass__ResolvePathCoord_BridgeAware @ 0x00583180`
- Ghidra: `MapClass__FindBridgeRecord @ 0x0056DA10`
- Ghidra: `MapClass__ComputeBridgeZones @ 0x0056D6E0`
- Rust: `src/sim/pathfinding/zone_search.rs`
- Rust: `src/sim/movement/movement_path.rs`
- Rust: `src/sim/movement/movement_commands.rs`
- Docs: `C:/Users/enok/Documents/ra2-rust-game-docs/ZONE_PRECHECK_0042C290_HIERARCHY_EXCLUSIONS_GHIDRA_REPORT.md`
- Docs: `C:/Users/enok/Documents/ra2-rust-game-docs/traces/PATHFIND_INFANTRY_LOW_BRIDGE_RAMP_TRACE.md`
