# High Bridge Edge-Lane Vehicle Traversal - Ghidra Reinvestigation Report

**Date:** 2026-05-17  
**Address(es):** `0x0047E040`, `0x0047E470`, `0x005FC570`, `0x004D9C60`, `0x0073F0A0`, `0x00429A90`, `0x0042A4A0`, `0x004802A0`, `0x00547230`, `0x0047F6A0`  
**Confidence:** High for traversal/stamping; Medium for final rendered vehicle-pixel diagnosis  
**Active in YR:** Yes. These are live YR map-load, A*, UnitClass entry, and tactical draw paths.

## 1. Overview

This pass reinvestigates the BayOPigs-style high-bridge symptom: Rust lets vehicles render on the top-left/min-X edge lane where the player-visible bridge railing/deck makes the vehicle appear outside the bridge.

The correction from this pass is important: the raw `BRIDGE2` anchor count is still not the movement footprint, but the previous conclusion "all stamped structural cells are valid vehicle lanes" was too broad. `SetBridgeDirection` stamps a Forward2 cell with `0x100` but explicitly without `0x200`, and `CheckBridgeTraversal` uses `0x200` in the level-walk/diff-0 gate. Rust currently skips that gate when both current and neighbor cells are structural bridge cells, which allows the suspect min-X Forward2 lane.

## 2. Key Offsets And Flags

| Offset / flag | Meaning in this question | Evidence | Active in YR? |
|---:|---|---|---|
| `CellClass+0x24/+0x26` | map cell x/y used by direction stepping | `SetBridgeDirection`, `AStar_main_loop` | Yes |
| `CellClass+0x38` | IsoTileType/tile index used by TMP and railing draw | `CellOverlay_TileDraw`, `FUN_004802A0` | Yes |
| `CellClass+0x44` | overlay type index; raw bridge anchor overlays live here | `OverlayClass::Mark`, `DrawOverlay_Body` | Yes |
| `CellClass+0x11A` | sub-tile/caller sub-index used by TMP and railing draw | `CellOverlay_TileDraw`, `FUN_004802A0` | Yes |
| `CellClass+0x11B` | signed terrain level used by A* and bridge traversal | `CheckBridgeTraversal`, `AStar_create_node` | Yes |
| `CellClass+0x11C` | slope/ramp passability byte for diff-1 moves | `CheckBridgeTraversal` | Yes |
| `CellClass+0x124` | ground occupancy bits | `UnitClass::Can_Enter_Cell` | Yes |
| `CellClass+0x128` | bridge occupancy bits | `UnitClass::Can_Enter_Cell` | Yes |
| `CellClass+0x140 & 0x80` | high-bridge body/anchor draw marker, not the normal lane predicate | `SetBridgeDirection`, `DrawOverlay_Body` | Yes |
| `CellClass+0x140 & 0x100` | structural/on-bridge cell bit | `SetBridgeDirection`, `CheckBridgeTraversal`, `Can_Enter_Cell` | Yes |
| `CellClass+0x140 & 0x200` | bridgehead/transition bit; required by specific traversal cases | `SetBridgeDirection`, `CheckBridgeTraversal` | Yes |
| `CellClass+0x140 & 0x1000` | forward-side marker; Forward3 only receives this among normal bits | `SetBridgeDirection` | Yes, but not a normal lane bit by itself |
| `CellClass+0x140 & 0x10000` | extra-side marker; ExtraDir6 only receives this among normal bits | `SetBridgeDirection` | Yes, but not a normal lane bit by itself |

## 3. SetBridgeDirection Stamping Facts

`OverlayClass::Mark @ 0x005FC570` calls the bridge stamping helper for high bridge overlay IDs:

| Overlay ID | Helper call |
|---:|---|
| `0x18` | `SetBridgeDirection_NESW(direction=0, state=1)` |
| `0x19` | `SetBridgeDirection_NESW(direction=6, state=1)` |
| `0xED` | `SetBridgeDirection_NWSE(direction=0, state=1)` |
| `0xEE` | `SetBridgeDirection_NWSE(direction=6, state=1)` |

The two helpers at `0x0047E040` and `0x0047E470` are byte-identical in behavior. For intact state (`state=1`) the relevant slots are:

| Slot | Relative cell | `0x100` | `0x200` | Consequence |
|---|---|---:|---:|---|
| Anchor | anchor | yes | yes | structural + transition |
| Forward1 | `anchor + direction` | yes | yes | structural + transition |
| Forward2 | `anchor + 2 * direction` | yes | **no** | structural but not transition |
| Forward3 | `anchor + 3 * direction` | no | no | forward-side marker only |
| Opposite | `anchor + opposite(direction)` | yes | yes | structural + transition |
| ExtraDir6 | extra east step after opposite when `direction == 6` | no | no | extra-side marker only |

For BayOPigs `BRIDGE2` direction 6 anchors at `x=112` and `x=160`, this maps the suspect min-X edge cells to Forward2:

| Component | Anchor column | Rust path columns | Suspect top-left/min-X column | Inferred slot |
|---:|---:|---|---:|---|
| 1 | `x=112` | `x=110..113` | `x=110` | Forward2 (`0x100`, no `0x200`) |
| 2 | `x=160` | `x=158..161` | `x=158` | Forward2 (`0x100`, no `0x200`) |

## 4. CheckBridgeTraversal Gate

`CheckBridgeTraversal @ 0x004D9C60` is the live Unit/Infantry/Foot bridge validator. It returns only `0` (OK) or `7` (blocked).

For the edge-lane question, the load-bearing branch is the diff-0 level-walk gate:

```text
parent_selected_height =
    parent.Level if parent.Flags & 0x100
    else path_height

diff = parent_selected_height - candidate.Level

if abs(diff) == 0:
    if (
        candidate lacks 0x100
        OR candidate lacks 0x200
        OR parent lacks 0x100
    ) AND path_height != -1 AND path_height != candidate.Level:
        return 7
    else:
        return 0
```

Tiny detail that matters: when a vehicle is on the bridge deck, A* carries the node height as `Level + 4`, not `Level`. Therefore a move from a bridge-deck node into a Forward2 candidate (`0x100` yes, `0x200` no) satisfies the blocking condition:

```text
candidate lacks 0x200 = true
path_height != -1 = true
path_height != candidate.Level = true  // deck height is Level + 4
=> return 7
```

That means `0x100` is necessary for structural bridge treatment, but it is not sufficient to allow every bridge-deck step. The `0x200` transition bit still gates level-walk deck traversal when the caller's path-height state is at deck height.

## 5. A* Height Context

`AStar_main_loop @ 0x00429A90` passes the current path height into `UnitClass::Can_Enter_Cell` through vtable slot `+0x1AC`.

`AStar_create_node @ 0x0042A4A0` stores the node height:

- start node uses `Pathfinder+0x30`;
- normal non-bridge candidate starts with `candidate.Level`;
- if the candidate has `0x100` and the parent cell also has `0x100`, and the parent node height equals `parent.Level + 4`, the candidate node height becomes `candidate.Level + 4`;
- if entering a bridge candidate from a compatible low-side parent, the candidate node height can also be promoted to `candidate.Level + 4`.

So once A* is on the bridge deck, the `CheckBridgeTraversal` diff-0 gate sees `path_height = Level + 4`. That is the state that blocks Forward2 candidates lacking `0x200`.

## 6. UnitClass Entry Context

`UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the live normal vehicle cell-entry function. It first chooses a ground-vs-bridge object-list byte from the candidate cell and incoming `path_height`, then calls `CheckBridgeTraversal` through vtable slot `+0x1B0`. If the bridge check returns `7`, `Can_Enter_Cell` immediately returns blocked.

The function may later re-read bridge occupancy bits from `cell+0x128` if the post-check path height equals `candidate.Level + 4`, but that happens after the hard traversal gate. It cannot rescue a Forward2 diff-0 rejection.

## 7. Render Stack Findings

The render-side suspicion is still real, but it is not enough to explain the edge-lane legality by itself.

`DrawOverlay_Body @ 0x0047F6A0` draws bridge body SHP only through the overlay body path and uses:

- `cell+0x44` overlay type;
- `cell+0x140 & 0x80`;
- `cell+0x11E` state byte;
- cell xy parity for the Latin-square frame offset;
- `cell+0x11B + (0x80 ? 4 : 0)` for bridge body draw height.

`CellOverlay_TileDraw @ 0x00480350`, `FUN_004802A0`, and `FUN_00547230` draw TMP/late railing using `cell+0x38` and `cell+0x11A`. The late railing path is tile-index/sub-tile driven, not raw overlay-byte driven.

This means Rust still has known render gaps around railing/late overlay emission, but the top-left vehicle-outside symptom now has a direct traversal explanation too: Rust is allowing the Forward2 min-X cells as bridge vehicle destinations when the binary's diff-0 gate would reject them for deck-height movement.

## 8. Current Rust Implementation Status

Rust correctly implements the raw stamp slots and the fact that Forward2 is structural but not transition:

- `src/map/bridge_facts.rs`: `Forward2` sets `BRIDGE_FLAG_STRUCTURAL` and clears/omits `BRIDGE_FLAG_TRANSITION`.
- `src/map/resolved_terrain.rs`: any `facts.has_structural_bridge()` sets `has_bridge_deck=true` and `bridge_walkable=true` if not terrain/overlay blocked.

The mismatch is in A* expansion:

- `src/sim/pathfinding/core.rs` computes `needs_bridge_traversal` as:

```text
neighbor.transition || !neighbor.structural || !current.structural
```

For a bridge-deck move from Forward1/anchor/opposite into Forward2, both cells are structural and the neighbor is not transition, so Rust skips `check_bridge_traversal`. It then allows the neighbor because bridge-layer walkability is true. This bypasses the binary diff-0 `0x200`/path-height consistency gate.

This exactly matches the BayOPigs diagnostic data:

- component 1 min-X edge `x=110` has `path_bridge_walkable=true` and `path_transition=false`;
- component 2 min-X edge `x=158` has `path_bridge_walkable=true` and `path_transition=false`;
- those are the cells where the proxy unit center appears outside or barely on the visible bridge pixels.

## 9. Reconciled Diagnosis

The old raw-anchor heuristic was still wrong: movement should not be reduced to only raw `BRIDGE2` overlay anchor cells.

The corrected movement diagnosis is narrower:

```text
Raw anchor only: too narrow.
All 0x100 structural cells as freely bridge-walkable: too wide.
Binary behavior: structural cells plus CheckBridgeTraversal path-height/0x200 gates.
```

For the BayOPigs top-left/min-X edge, the suspect cells are Forward2 structural cells with no transition bit. The binary gate rejects those cells for bridge-deck diff-0 traversal when the path height is `Level + 4`. Rust currently permits them because it skips traversal validation for structural-to-structural body moves.

## 10. Open Questions

1. Retail visual capture should still be used to confirm the player-visible result with a specific normal vehicle on BayOPigs, but the binary evidence is sufficient to explain why Rust can be too permissive on the top-left edge.
2. The late railing path still needs a separate render correction: Rust's current overlay-byte gate does not match `FUN_004802A0` / `FUN_00547230`.
3. This report does not investigate locomotor probe calls outside A* pathfinding. The A* and `UnitClass::Can_Enter_Cell` path is the normal route-planning path for vehicles.

## Sources

- Ghidra decompiled: `OverlayClass__Mark @ 0x005FC570`
- Ghidra decompiled: `CellClass__SetBridgeDirection_NESW @ 0x0047E040`
- Ghidra decompiled: `CellClass__SetBridgeDirection_NWSE @ 0x0047E470`
- Ghidra decompiled: `CheckBridgeTraversal @ 0x004D9C60`
- Ghidra decompiled: `UnitClass__Can_Enter_Cell @ 0x0073F0A0`
- Ghidra decompiled: `AStar_main_loop @ 0x00429A90`
- Ghidra decompiled: `AStar_create_node @ 0x0042A4A0`
- Ghidra decompiled: `CellClass__DrawOverlay_Body @ 0x0047F6A0`
- Ghidra decompiled: `CellOverlay_TileDraw @ 0x00480350`
- Ghidra decompiled: `FUN_004802A0`
- Ghidra decompiled: `FUN_00547230`
- Prior docs checked:
  - `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`
  - `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
  - `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
  - `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
- Rust files read:
  - `src/map/bridge_facts.rs`
  - `src/map/resolved_terrain.rs`
  - `src/sim/pathfinding/core.rs`
  - `src/app_instances/bridges.rs`
- Visual artifacts checked:
  - `docs/visual-checks/bridge-terrain-overlay-mismatch/bayopigs-mmx-summary.md`
  - `docs/visual-checks/bridge-unit-edge-footprint/investigation.md`
  - `docs/visual-checks/bridge-unit-edge-footprint/bayopigs-mmx-edge-unit-footprint.csv`
  - `docs/visual-checks/bridge-render-footprint/bayopigs-mmx-render-footprint.md`
