# Bridge Map-Load Facts and Bridgehead Transitions

Date: 2026-05-15

Scope: reinvestigate the two foundational bridge/pathfinding questions:

1. How `gamemd.exe` derives authoritative bridge cell facts at map load.
2. How `gamemd.exe` handles bridgehead / bridge deck transition cells for movement and A*.

This report also compares the verified binary behavior with the current Rust implementation in
`src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, and
`src/sim/movement/movement_bridge.rs`.

## Executive result

The current Rust implementation is directionally similar on the broad concept of "bridge deck is
ground level + 4" and "there are transition cells", but it is not doing this the same way as
`gamemd.exe`.

The highest-confidence divergence is that `gamemd.exe` does not build bridge cell facts with a
global resolved-terrain inference pass that expands side cells, flood-normalizes deck height, or
fills bridge gaps. In the binary, high bridge facts are primarily stamped by overlay object marking
through `OverlayClass::Mark` into `CellClass::SetBridgeDirection_*`, and bridge traversal is later
resolved dynamically by `UnitClass::Can_Enter_Cell` / `CheckBridgeTraversal` using bridge flags,
cell level, target path height, slope index, and separate ground/bridge occupancy state.

This is foundational. If these facts diverge, pathfinding and locomotion may still work on simple
maps, but edge cells, bridgeheads, broken bridges, object blocking, and route choice can differ from
YR.

## Verified binary entry points

Primary functions inspected in Ghidra:

- `OverlayClass__Mark`
- `ReadMapOverlayPacks`
- `CellClass__SetBridgeDirection_NESW`
- `CellClass__SetBridgeDirection_NWSE`
- `CellClass__RecalcAttributes`
- `CellClass__RecalcZoneType`
- `UnitClass__Can_Enter_Cell`
- `CheckBridgeTraversal`
- `AStar_main_loop`
- `AStar_create_node`
- `MapClass__IsBridgeRampTile`
- `MapClass__UpdateBridgeEdgeTiles_High`
- `MapClass__Resize`
- `DriveLocomotionClass__Set_Destination`
- `DriveLocomotionClass__ComputeBridgeZOffset`

Confidence: high for the relationships and predicates described below. The one important caveat is
that low bridge overlay procedural placement deserves a separate focused pass if low bridges are in
scope.

## Cell fields and flags used by the binary

Known cell fields used by the verified paths:

| Field | Meaning in verified paths |
|---|---|
| `cell + 0x2C` | bridge anchor pointer |
| `cell + 0x38` | `IsoTileTypeIndex` |
| `cell + 0x44` | `OverlayTypeIndex` |
| `cell + 0x54` | ground secondary object pointer / occupancy pointer |
| `cell + 0x58` | bridge secondary object pointer / occupancy pointer |
| `cell + 0xE4` | ground-layer first object list |
| `cell + 0xE8` | bridge-layer first object list |
| `cell + 0xEC` | land type |
| `cell + 0x11A` | tile height / subtile byte used by TMP and bridge-ramp tests |
| `cell + 0x11B` | signed cell level |
| `cell + 0x11C` | slope index |
| `cell + 0x11E` | bridge state byte |
| `cell + 0x124` | ground occupancy bitfield |
| `cell + 0x128` | bridge occupancy bitfield |

Bridge-relevant flags:

| Flag | Verified use |
|---|---|
| `0x80` | high-bridge marker stamped by `SetBridgeDirection` on the anchor-side cell and consumed by bridge edge/state walkers; separate from traversal legality |
| `0x100` | bridge structural cell used by traversal and path height logic |
| `0x200` | bridgehead / transition flag used by `CheckBridgeTraversal` |
| `0x400` | destroyed / absent bridge-state bit from `SetBridgeDirection` state argument |
| `0x800` | direction sentinel from `SetBridgeDirection` direction argument |
| `0x1000` | bridge state bit from `SetBridgeDirection` state argument |
| `0x10000` | bridge state bit from `SetBridgeDirection` state argument |

## Map-load bridge facts in gamemd.exe

### Verified map-load flow

`ReadMapOverlayPacks` parses `[OverlayPack]` and `[OverlayDataPack]`. For every non-`0xFF`
overlay byte, it constructs an `OverlayClass` for that overlay type at the map cell. The constructor
path reaches `OverlayClass::Mark`.

`OverlayClass::Mark` is the active high-bridge stamping path. For runtime overlay type IDs:

- `0x18` calls `CellClass::SetBridgeDirection_NESW(cell, dir = 0, state = 1)`.
- `0x19` calls `CellClass::SetBridgeDirection_NESW(cell, dir = 6, state = 1)`.
- `0xED` calls `CellClass::SetBridgeDirection_NWSE(cell, dir = 0, state = 1)`.
- `0xEE` calls `CellClass::SetBridgeDirection_NWSE(cell, dir = 6, state = 1)`.

After bridge stamping, the normal overlay mark path writes the overlay type and calls
`CellClass::RecalcAttributes`.

For bridge overlays, `ReadMapOverlayPacks` preserves/restores the bridge state byte around object
construction and then lets `[OverlayDataPack]` write the final state bytes. This makes the overlay
byte and overlay data byte jointly important.

### What SetBridgeDirection actually does

`CellClass::SetBridgeDirection_NESW` and `CellClass::SetBridgeDirection_NWSE` decompile to the same
structure with different direction offset data.

The function does not scan a bridge component. It writes a fixed group of neighboring cells relative
to the anchor and direction:

- The anchor cell receives the anchor pointer, bridge state byte, and the bridge flags including
  `0x80`, `0x100`, `0x200`, `0x1000`, `0x10000`, and direction/state bits.
- The first and second bridge-side cells receive bridge structural flags and bridge anchor pointer,
  but not the same anchor-only `0x80` treatment.
- The opposite transition cell receives bridge structural / transition flags and anchor pointer.
- A further forward neighbor receives only a limited bridge-state flag update.
- For `dir == 6`, one extra cell receives anchor pointer and `0x10000`.
- Collapse/state-zero paths call `CellClass::BlowUpBridge` and clear the bridge anchor pointer.

The important parity point: this is deterministic neighbor stamping from the overlay anchor, not a
resolved-terrain bridge normalization algorithm.

### RecalcAttributes does not derive bridge structure

`CellClass::RecalcAttributes` derives terrain attributes from TMP/tile data:

- `cell + 0x11A`: height / subtile byte.
- `cell + 0x11B`: signed level, optionally overridden by a hidden parameter.
- `cell + 0x11C`: slope index from TMP slope type.
- land type and zone cache through `CellClass::RecalcZoneType`.

This function does not derive high bridge deck cells, bridgeheads, side-cell expansion, or connected
bridge deck height normalization.

### Bridge ramp recognition is tile-and-subtile specific

`MapClass::IsBridgeRampTile` does not simply ask whether the cell's tileset is a bridge set. It is
also not a bridge-fact stamping routine. It is a predicate that compares a bridge tile key against
theater bridge globals and also checks exact `cell + 0x11A` values.

Important argument convention: inspected callers do not pass a raw global `IsoTileTypeIndex` to this
predicate. `MapClass::UpdateBridgeEdgeTiles_High`, for example, prepares a one-based BridgeSet
relative key:

```c
bridge_key = (cell.IsoTileTypeIndex - BridgeSetStart) + 1;
```

Consumers should therefore avoid both raw global tile IDs and unproven zero-based relative IDs when
replicating this predicate.

The verified key/subtile matches are:

- two bridge globals require `0x11A == 0x0C`;
- one bridge-end table requires `0x11A == 0x04`;
- two bridge globals require `0x11A == 0x08`;
- one bridge-end table requires `0x11A == 0x02`.

So bridgehead/ramp recognition in the binary is much narrower than "tileset index is the concrete or
wood bridge set", and it must use the same BridgeSet-relative key convention as the verified caller.

### MapClass::Resize is not the normal origin

`MapClass::Resize` preserves bridge-related flags during map array reallocation and, during a later
refresh loop, calls `SetBridgeDirection` for cells that still have `0x80` but no bridge anchor
pointer. This is a repair/reassertion path after resize/editor/save-load style state changes, not the
normal retail skirmish origin of bridge flags.

## Bridgehead transition behavior in gamemd.exe

### A* carries path height

`AStar_main_loop` is not plain 2D cell search. It carries a path height:

- The goal height is normally the goal cell level, but if the goal cell has bridge flag `0x100` and
  the moving object is not the exempt object category observed in the binary, the goal height is
  `goal.Level + 4`.
- The start/current height is either ground level or `start.Level + 4` depending on the moving unit's
  current bridge-layer state, with additional bridge-height adjustment logic.
- Each candidate neighbor is tested through the unit's `Can_Enter_Cell` vtable call.

`AStar_create_node` then stores a per-node height. Bridge traversal is therefore not equivalent to
checking a boolean bridge-walkable flag on the destination.

### UnitClass::Can_Enter_Cell selects layer state around CheckBridgeTraversal

`UnitClass::Can_Enter_Cell` snapshots target cell occupancy before calling `CheckBridgeTraversal`.

Before traversal check:

- If the target cell has `0x100` and the incoming target height is unset or at least two levels away
  from the target cell ground level, it prepares to use the bridge layer.
- Otherwise it prepares to use ground-layer state.

Then it calls `CheckBridgeTraversal`.

After traversal check:

- If the incoming target height equals `target.Level + 4` on a `0x100` bridge cell, the occupancy
  snapshot is replaced with the bridge occupancy fields (`cell + 0x128`, `cell + 0x58`).
- Later object-list iteration chooses ground list `cell + 0xE4` or bridge list `cell + 0xE8` using a
  layer byte that can also be forced by `CheckBridgeTraversal`.

This means YR distinguishes bridge and ground object state at both occupancy-bit and object-list
levels. The two selections are related but not a single precomputed cell layer boolean.

### CheckBridgeTraversal's core rules

`CheckBridgeTraversal` returns allowed or blocked for bridge/height movement. It uses:

- source cell flags;
- destination cell flags;
- signed cell levels;
- incoming target/path height;
- slope index;
- the `0x100` bridge structural flag;
- the `0x200` bridgehead / transition flag.

Verified high-level behavior:

- If the destination has `0x100` and the target height is unset, the target height becomes
  `dst.Level + 4`; if the source lacks `0x200`, traversal is blocked.
- Equal-level movement is blocked when the carried target height disagrees with source ground height,
  except for the special case where source has `0x100`, source has `0x200`, and destination has
  `0x100`.
- One-level height differences require a slope index on the lower cell.
- Four-level differences are the bridge transition case:
  - going from deck height down to a lower bridge cell requires target height to equal source ground
    level and destination to have `0x100`;
  - going from ground to a cell four levels higher requires the source to have `0x100` and `0x200`;
    this path forces bridge-layer selection.
- Other height differences are blocked.

The exact bridgehead rule is therefore a dynamic interaction between `0x100`, `0x200`, source and
destination levels, and the carried path height.

## Current Rust comparison

### resolved_terrain.rs

`src/map/resolved_terrain.rs` currently turns map TMP tiles, overlays, INI terrain rules, heights,
ramps, water, roads, cliffs, bridge overlays, and bridgeheads into authoritative per-cell data.

The verified binary behavior differs in several important ways:

1. Rust's `classify_overlay_effects` treats a broad range of bridge overlay IDs as bridge deck
   cells. The binary's high bridge structural stamping path special-cases runtime overlay IDs
   `0x18`, `0x19`, `0xED`, and `0xEE` through `OverlayClass::Mark` and has separate procedural
   logic for other bridge overlay ranges. It does not run a simple "all bridge overlay IDs are deck
   cells" classification pass.

2. Rust derives side-cell expansion for high bridge decks. No matching global side-cell expansion
   pass was found in the verified binary path. `SetBridgeDirection` writes a fixed direction-relative
   set of cells from a bridge overlay anchor.

3. Rust flood-normalizes bridge deck heights across connected bridge components. The verified binary
   path stores signed cell level on each cell and applies the bridge deck offset dynamically as
   `Level + 4`. No connected-component max-height normalization pass was found.

4. Rust detects bridgehead transition cells with broad theater tileset checks such as bridge set or
   wood bridge set. The binary's `MapClass::IsBridgeRampTile` uses exact theater bridge keys, passed
   by inspected callers as one-based BridgeSet-relative values, and exact `cell + 0x11A` subtile
   values.

5. Rust gap-fills bridge deck cells between nearby detected deck cells. No matching gamemd map-load
   bridge gap-fill pass was found in the verified functions.

6. Rust stores simplified booleans like bridge walkable / transition and a precomputed deck level.
   The binary keeps multiple bridge flags and resolves transition legality later against path height,
   source cell, destination cell, slope index, and separate layer occupancy.

### pathfinding/core.rs

The Rust pathfinding cell model compresses the binary's bridge state into fewer facts. That can be
enough for simple cases, but it cannot exactly represent the verified binary behavior where:

- bridge structural cell (`0x100`) and bridgehead/transition (`0x200`) are distinct;
- high-bridge SetBridgeDirection / edge-walk marker (`0x80`) is separate again;
- bridge occupancy and object lists are split into ground and bridge layers;
- A* nodes carry height;
- `Can_Enter_Cell` may change which layer is checked while evaluating a candidate step.

The result is a likely parity risk at bridgeheads, damaged bridges, mixed ground/deck object blocking,
and routes that approach a bridge from unusual angles or heights.

### movement_bridge.rs

The Rust movement transition predicates resemble part of the binary's four-level bridge transition
rules, but they are simplified around Rust's resolved terrain fields. The binary decision depends on
source and destination flags, source and destination signed levels, target/path height, and side
effects into the layer-selection byte.

So the current Rust movement logic may match common bridge entry/exit cases while still diverging
from YR on edge cases.

## Practical severity

This does not mean "nothing works". The current Rust model includes the most visible invariant:
bridge deck height is ground level plus four. It also models transition cells.

But this is still foundational. If resolved terrain invents bridge cells, normalizes heights, or
marks transition cells differently from the binary, then every downstream system receives the wrong
input:

- A* may accept cells YR rejects.
- A* may reject cells YR accepts.
- Unit blocking may happen on the wrong layer.
- Bridgeheads may connect from the wrong side or at the wrong height.
- Broken bridge behavior can diverge.
- Render height and locomotion Z can disagree with gameplay.

## Recommended follow-up targets

1. Replace broad bridgehead detection with a binary-shaped bridge ramp table:
   one-based BridgeSet-relative bridge tile key plus exact `0x11A` subtile values, backed by theater
   bridge globals.

2. Split Rust's bridge cell representation so it can separately model at least:
   `0x80`, `0x100`, `0x200`, bridge state byte, signed level, slope index, and bridge anchor relation.

3. Rework high bridge map-load derivation around overlay-object stamping semantics:
   bridge overlay anchor IDs should call a `SetBridgeDirection`-equivalent routine instead of relying
   on global side expansion / flood normalization / gap fill.

4. Compare low bridge overlay IDs separately. The binary has procedural branches for low bridge
   overlay ranges that are not fully covered by this high-bridge-focused report.

5. Build a small map-cell dump harness for one known retail/YR map bridge. Compare per-cell Rust
   facts against binary-derived expected flags before changing A*.

## Bottom line

The answer to "are we doing this the same way as gamemd.exe?" is no.

The most important issue is not one isolated bug. It is that Rust currently treats resolved terrain
as the authoritative place to infer bridge decks and bridgeheads, while `gamemd.exe` stamps bridge
state through overlay marking and then resolves bridge traversal dynamically during movement/pathing.
That difference is large enough to investigate and correct before relying on bridge pathfinding
results as parity evidence.
