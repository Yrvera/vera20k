# Bridge Low-Bridge and Zone-Record Supplement

Date: 2026-05-15

Parent report: `BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`

Scope: continue the bridge reinvestigation into the parts left open by the parent
report:

- low bridge map-load / terrain construction;
- bridge zone records and high-vs-low filtering;
- zone redirect behavior for destroyed high bridges;
- current Rust differences that follow from those details.

No Rust code was changed.

## Executive result

The deeper pass found a second foundational mismatch.

High bridges and low bridges are not just two overlay families in `gamemd.exe`.
High bridge traversal is mostly driven by bridge flags and `SetBridgeDirection`.
Low bridge traversal is tied to land type, theater tile identity, and `TubeClass`
construction from `CellClass::RecalcAttributes`.

Current Rust still treats low bridge overlays as bridge overlays in the same broad
overlay-index classifier used by high bridges. That is not the same shape as the
binary. It may reproduce some visible low bridge artwork, but it does not match
the binary's low-bridge pathing foundation.

The bridge-zone record model also differs: the binary's `BridgeRecord` has a
`bridge_kind` field where `0 = high` and `1 = low`, and `FindBridgeRecord` skips
low records. Rust's runtime endpoint record has no bridge-kind field and injects
all active records uniformly.

## Verified binary functions

Primary functions inspected:

- `OverlayClass::Mark` @ `0x005FC570`
- `CellClass::RecalcAttributes` @ `0x0047D2B0`
- `CellClass::IsLowBridgeCell` @ `0x00484AB0`
- `CellClass::GetTubeAtCell` @ `0x00484F20`
- `CellClass::IsOnBridgeSurface` @ `0x00485060`
- `CellClass::IsBridge` @ `0x00486750`
- `CellClass::IsWoodBridge` @ `0x00486770`
- `TubeClass::Constructor` @ `0x00727FD0`
- `UnitClass::TubeMovement` @ `0x007359F0`
- `MapClass::ComputeBridgeZones` @ `0x0056D6E0`
- `MapClass::FindBridgeRecord` @ `0x0056DA10`
- `MapClass::InvalidateBridgeZones` @ `0x0056DAE0`
- `MapClass::ValidateBridgeZones` @ `0x0056DB70`
- `MapClass::UpdateBridgeZonesHelper` @ `0x0056C510`
- `MapClass::GetZoneID` @ `0x0056D230`
- `MapClass::IsLowBridgeEndpointTile` @ `0x00574600`
- `MapClass::PlaceBridgeRamp_Low` @ `0x00579010`
- `MapClass::UpdateBridgeTile_Low` @ `0x0057A430`
- `MapClass::ComputeBridgeAdjacencyMask_Low` @ `0x00579B70`
- `MapClass::ComputeBridgeSurfaceMask` @ `0x0057B210`
- `PathfinderClass::UpdateBridgePassability` @ `0x0042ACF0`

Active in YR: yes for map-load/post-load bridge construction, zone construction,
bridge damage/repair zone updates, and unit pathing. Map editor branches are
present but are called out where relevant.

## Finding 1: low bridge overlays are not enough to create low-bridge pathing

Evidence: `CellClass::IsLowBridgeCell` @ `0x00484AB0`.

The function returns true only when both conditions hold:

- `cell + 0x116` is a valid tube index: `0 <= tube_index < g_TubeArray.count`.
- `cell + 0xEC` land type equals `10`.

It does not check overlay ID directly.

Why it matters: low bridge pathing is not simply "overlay ID is in the low bridge
range". A cell can draw a low bridge overlay and still fail the low-bridge-cell
predicate if the tube index was not constructed.

## Finding 2: RecalcAttributes constructs low-bridge tubes

Evidence: `CellClass::RecalcAttributes` @ `0x0047D2B0`, xref to
`TubeClass::Constructor` @ `0x0047D940`.

The low-bridge tube construction branch fires only if:

- `this->LandType == 10`;
- `cell + 0x116` is invalid or outside the current tube array count;
- `IsoTileTypeIndex` falls inside one of four exact 4-tile ranges:
  - `DAT_00AA1054 .. DAT_00AA1054 + 3`;
  - `DAT_00ABB108 .. DAT_00ABB108 + 3`;
  - `DAT_00AA10B4 .. DAT_00AA10B4 + 3`;
  - `DAT_00ABAD2C .. DAT_00ABAD2C + 3`;
- the offset from the matched range is not `-1`;
- allocation of `0x1C4` bytes succeeds.

Then the binary calls:

```text
TubeClass::Constructor(cell_coord, DAT_0081CC20[tile_index - range_base])
```

`TubeClass::Constructor` stores the same coord into tube fields `+0x24` and
`+0x28`, stores the direction parameter at `+0x2C`, initializes 100 path slots to
`-1`, appends the tube to `g_TubeArray`, and writes the tube index back to
`cell + 0x116`.

Tiny detail: the constructor only writes `cell + 0x116` when the coord is not
`(0,0)`. That guard is live in the constructor.

Why it matters: the `RecalcAttributes` path backs qualifying low/tunnel cells
with zero-length / same-cell tube shell records. This is not the only live
TubeClass shape: the `[Tubes]` parser can create fully initialized entry/exit/
step tubes. The observable result is not derived from overlay index alone.

## Finding 3: low bridge INI overlays say Road, but binary pathing still keys on land type 10

Evidence: `rulesmd.ini` low bridge sections and `IsLowBridgeCell`.

Representative YR low bridge overlay sections such as `[LOBRDG01]`,
`[LOBRDGE1]`, `[LOBRDB01]`, and `[LOBRDGB1]` use:

```ini
Land=Road
NoUseTileLandType=true
```

The binary predicate is still numeric: `LandType == 10`.

Why it matters: a Rust implementation should be careful not to infer the low
bridge traversal model from the display name or from a broad overlay family alone.
The actual binary check is the resolved numeric land type plus tube index.

## Finding 4: low bridge endpoint overlays procedurally stamp multiple cells

Evidence: `OverlayClass::Mark` @ `0x005FC570`.

Low bridge endpoint overlay ranges have special branches:

- `0x7A .. 0x7D` enter the first low-bridge endpoint branch.
- `0xE9 .. 0xEC` enter the second low-bridge endpoint branch.

For each branch, the binary:

1. Initializes small static offset tables the first time the branch is used.
2. Checks a 3-cell local pattern and aborts if any target already has an overlay.
3. Writes three overlay cells with state bytes `0`, `1`, `2`.
4. Searches forward along a direction until it finds the matching opposite
   endpoint with state byte `1`.
5. Computes the fill length as the maximum of absolute X/Y distance back toward
   the originating endpoint.
6. For each fill step, writes a 3-cell row.
7. Picks bridge body overlay variants with `Random::Next() & 3` plus a family
   base overlay ID.
8. Calls `CellClass::RecalcAttributes` for every written cell.

Why it matters: the low bridge endpoint branch is procedural and randomized for
some body art variants. It is not equivalent to marking every low bridge overlay
ID as an independently authoritative bridge deck.

## Finding 5: low bridge surface/ramp update uses bit masks from neighbor height + surface state

Evidence:

- `MapClass::ComputeBridgeAdjacencyMask_Low` @ `0x00579B70`
- `MapClass::PlaceBridgeRamp_Low` @ `0x00579010`
- `MapClass::UpdateBridgeTile_Low` @ `0x0057A430`
- `MapClass::ComputeBridgeSurfaceMask` @ `0x0057B210`

`ComputeBridgeAdjacencyMask_Low` starts from the current cell level and tests
eight neighbor positions. A neighbor contributes to the mask only when:

- the neighbor is in playfield;
- neighbor signed level equals current level `+ 4`;
- an additional surface/tile helper check returns false.

The mask bits observed include `0x01`, `0x02`, `0x04`, `0x08`, `0x10`, `0x20`,
`0x40`, and `0x80`.

`PlaceBridgeRamp_Low` and `UpdateBridgeTile_Low` then use exact bit patterns,
including:

- `(mask & 0xA0) == 0xA0`;
- `(mask & 0x11) == 0x11`;
- `(mask & 0x44) == 0x44`;
- `(mask & 0x88) == 0x88`;
- `(mask & 0x22) == 0x22`;
- several asymmetric mixed masks such as `0x2C == 0x24`, `0xA1 == 0x21`,
  `0x1A == 0x12`, `0xC2 == 0x42`, `0x0B == 0x09`, `0x68 == 0x48`,
  `0x86 == 0x84`, and `0xB0 == 0x90`.

When a low bridge surface cell is accepted, `UpdateBridgeTile_Low` writes:

- `cell + 0x11A = 0`;
- `IsoTileTypeIndex = DAT_00AA0738`;
- a bridge/tube assignment through `FUN_005A0090`;
- recursive updates over all eight neighbors.

Why it matters: low bridge ramp/surface maintenance is a local mask-driven tile
rewrite system, not a simple overlay family classifier.

## Finding 6: IsLowBridgeEndpointTile is direction-specific

Evidence: `MapClass::IsLowBridgeEndpointTile` @ `0x00574600`.

For direction `2`, accepted endpoint tiles require `cell + 0x11A == 0x04` and
match either:

- `DAT_00ABC1E8`;
- `DAT_00AA0E38`;
- one of `DAT_00ABAD30 .. DAT_00ABAD30 + 3`.

For direction `4`, accepted endpoint tiles require `cell + 0x11A == 0x02` and
match either:

- `DAT_00ABC1D0`;
- `DAT_00AA1540`;
- one of `DAT_00AA1028 .. DAT_00AA1028 + 3`.

Tiny detail: the EW/direction-4 constants are not the same as the NS/direction-2
constants. Existing docs already warned about this kind of axis copy-paste trap;
the binary confirms the split here.

## Finding 7: ComputeBridgeZones creates both high and low BridgeRecords

Evidence: `MapClass::ComputeBridgeZones` @ `0x0056D6E0`.

The bridge record table is a dynamic vector at `MapClass + 0x50`, with data
pointer at `+0x54` and count at `+0x60`. Each record is 16 bytes:

| Offset | Meaning |
|---|---|
| `+0x00` | endpoint A cell coord |
| `+0x04` | endpoint B cell coord |
| `+0x08` | intact byte, `1 = intact`, `0 = destroyed` |
| `+0x09..0x0B` | unused/padding in observed paths |
| `+0x0C` | bridge kind, `0 = high`, `1 = low` |

High bridge record creation:

- scans all cells;
- requires `CellClass::IsBridge` or `CellClass::IsWoodBridge`;
- chooses concrete vs wood base tile global;
- checks `DAT_0082A734[index] == cell.Height`;
- walks perpendicular using `DAT_0082A774[index]`;
- detects the opposite end using `DAT_0082A7B4`;
- sets the intact byte based on whether intervening cells keep bridge flag
  `0x100`;
- writes `bridge_kind = 0`.

Low bridge record creation:

- only considered when the cell is not a high/wood bridge tile;
- requires `CellClass::IsLowBridgeCell`;
- checks opposite low-bridge neighbors in direction pairs `2/6` and `4/0`;
- reads tube data through `CellClass::GetTubeAtCell`;
- consumes `tube+0x28` as the exit endpoint; it does not populate or repair
  that field;
- compares linearized cell order through helper `FUN_0042B1C0`;
- writes `bridge_kind = 1`.

Why it matters: low records exist in the same table, but not every consumer treats
them like high bridge records.

## Finding 8: FindBridgeRecord skips low bridge records

Evidence: `MapClass::FindBridgeRecord` @ `0x0056DA10`.

The first meaningful test inside the record scan is:

```text
if (record + 0x0C == 0) { ... consider this record ... }
```

Records with `bridge_kind != 0` are skipped. Since `ComputeBridgeZones` writes
`1` for low bridges, `FindBridgeRecord` is high-bridge-only.

The match itself is axis-aligned:

- if endpoint X coordinates are equal, the query Y must be between endpoint Y
  values and `abs(query.X - endpoint.X) <= dist`;
- otherwise query X must be between endpoint X values and
  `abs(query.Y - endpointA.Y) <= dist`.

Why it matters: any Rust helper claiming to mirror `FindBridgeRecord` must have
the high-only filter. A generic endpoint-record search over high and low bridges
is not the same function.

## Finding 9: Invalidate/Validate can rebuild bridge records if lookup misses

Evidence:

- `MapClass::InvalidateBridgeZones` @ `0x0056DAE0`
- `MapClass::ValidateBridgeZones` @ `0x0056DB70`

Both functions first call `FindBridgeRecord(coord, dist = 3, start = 0)`.

If the lookup returns `-1`, both call `MapClass::ComputeBridgeZones()` and retry.
If the retry still misses, they return `0`.

`InvalidateBridgeZones` loops over all matching records, calls
`RemoveBridgeZoneEdges`, then writes record `+0x08 = 0`.

`ValidateBridgeZones` loops over all matching records, writes record `+0x08 = 1`,
calls `AddBridgeZoneEdges`, then calls `Can_Reach_Zone` between the endpoints. It
returns true only when this changed reachability state.

Why it matters: bridge records are not just a one-time static artifact. The binary
has a self-healing path that recomputes records when damage/repair lookup misses.

## Finding 10: map post-load recomputes attributes, bridge records, then zone ids

Evidence: post-load function `FUN_00684C30`.

Relevant order:

1. Iterate all cells and call `CellClass::RecalcAttributes`.
2. Call `MapClass::ComputeBridgeZones`.
3. Call `MapClass::UpdateBridgeZonesHelper`.

Why it matters: low bridge tubes are created during the RecalcAttributes pass
before bridge records are computed. That ordering is necessary because
`ComputeBridgeZones` tests `IsLowBridgeCell`, which needs a valid tube index.

## Finding 11: GetZoneID uses high bridge records only and remaps destroyed high bridges

Evidence: `MapClass::GetZoneID` @ `0x0056D230`.

When the caller passes the bridge-aware flag and the queried cell has flag
`0x100`, the function:

1. Calls `FindBridgeRecord(coord, dist = 1, start = 0)`.
2. Returns `0xFFFFFFFF` if no high bridge record matches.
3. Reads the matching record.
4. If record `+0x08` intact byte is zero, walks from the queried bridge cell in a
   direction derived from endpoint orientation until it reaches a non-`0x100`
   cell.
5. If that landed cell is a bridge/wood-bridge tile and its land type is not `3`,
   it uses the opposite endpoint coord for zone lookup.
6. Finally returns the zone ID from `MapClass + 0x18 + movement_zone * 4` indexed
   by the cluster id in zone cell data at `MapClass + 0x68`.

Tiny detail: the destroyed-bridge walk direction is:

```text
same endpoint X  -> direction 4
different X      -> direction 2
```

The binary expression is `(-(endpoint_a.x != endpoint_b.x) & 0xFFFFFFFE) + 4`.

Why it matters: Rust's nearest-endpoint bridge redirect is not the same as this
destroyed-high-bridge zone fallback.

## Finding 12: UpdateBridgeZonesHelper builds 13 movement-zone maps

Evidence: `MapClass::UpdateBridgeZonesHelper` @ `0x0056C510`.

Relevant verified details:

- Per-cell zone data at `MapClass + 0x68` is 4 bytes per cell:
  - byte 0: zone type;
  - byte 1: height;
  - bytes 2-3: cluster id.
- It clears bytes 2-3 before flood filling.
- It seeds a sentinel zone type `7`.
- It flood-fills clusters and writes cluster count to `MapClass + 0x4C`.
- It adds bridge edges from intact bridge records before building per-movement
  zone-id arrays.
- It creates 13 arrays at `MapClass + 0x18 .. +0x4B`, one per movement zone.
- It uses the passability matrix to decide whether each cluster receives a
  movement-zone id.

Why it matters: Rust's single ground-zone adjacency model is materially simpler
than the binary's 13 movement-zone zone maps plus hierarchical graph structures.

## Finding 13: PathfinderClass::UpdateBridgePassability toggles flag 0x40000

Evidence: `PathfinderClass::UpdateBridgePassability` @ `0x0042ACF0`.

This function picks a neighboring cell based on `RateTimer::Current`, decides
whether to inspect the ground object list (`cell + 0xE4`) or bridge object list
(`cell + 0xE8`) using bridge flag `0x100`, height difference, and the unit's
bridge-layer byte. It then walks unit/factory path data and toggles cell flag
`0x40000` along affected cells.

It also has a fallback 5x5 scan around the selected neighbor, skipping the exact
unit source cell, toggling `0x40000` on occupied cells, and finally toggling
`0x40000` on the neighbor itself.

Why it matters: there is another transient pathing/passability bit involved in
bridge-adjacent movement. Rust currently models static path grid walkability and
runtime occupancy differently, so this is a future parity target if units still
choose strange bridge routes after cell facts are corrected.

## Current Rust comparison

### `src/map/overlay_types.rs`

Rust marks these as bridge overlays:

- high concrete: `24 | 25`;
- high wood: `237 | 238`;
- low wood body: `74..=101`;
- low wood ends: `122..=125`;
- low urban body: `205..=232`;
- low urban ends: `233..=236`.

This is useful for rendering and broad classification, but it is not the binary's
low-bridge pathing predicate. In the binary, low bridge pathing requires the
`IsLowBridgeCell` predicate: valid tube index plus land type `10`.

### `src/sim/bridge_state/mod.rs`

`BridgeEndpointRecord` stores:

- `endpoint_a`;
- `endpoint_b`;
- `group_id`;
- `active`.

It does not store the binary `bridge_kind` field. Therefore the runtime cannot
express the binary distinction where low bridge records exist but
`FindBridgeRecord` skips them.

`compute_bridge_endpoints` also differs from the binary. Rust collects all
cardinal ground neighbors around a bridge component and picks the pair with
maximum Manhattan distance. The binary `ComputeBridgeZones` uses bridge tile
orientation tables, bridge/wood base globals, exact height table matches, low
bridge tube data, and axis-specific scanning.

### `src/sim/pathfinding/zone_build.rs`

Rust `inject_bridge_adjacency` injects every active endpoint record into one
ground-zone adjacency graph. The binary uses 13 movement-zone maps and
high/low-aware bridge records. `FindBridgeRecord` specifically ignores low
records.

Rust `build_bridge_redirect` chooses the nearest active endpoint by Manhattan
distance for every bridge-layer-walkable cell. The binary `GetZoneID` only enters
the bridge remap path for `flag 0x100` with the bridge-aware caller flag set,
uses `FindBridgeRecord` high-only, and has a destroyed-bridge directional walk
based on endpoint orientation.

### `src/sim/bridge_specs.rs`

Rust already contains pure helper logic for:

- low bridge overlay damage families;
- zone connection record decoding;
- high-vs-low bridge-zone policy decisions.

Those helpers show the project has some of the right binary facts available, but
they are not fully wired into the live bridge runtime / zone runtime model.

## New parity risks identified

1. Low bridge body/endpoint overlay IDs may be treated as bridge decks even when
   the binary would require tube construction first.

2. Low bridge records may affect Rust's zone adjacency as if they were high
   bridge records. In the binary, low records are present but skipped by
   `FindBridgeRecord`.

3. Rust's endpoint pair selection can choose endpoints that the binary would
   never create because binary endpoint creation is orientation-table driven.

4. Rust's destroyed-bridge redirect is nearest-endpoint based. The binary
   destroyed-high-bridge zone fallback walks a fixed direction derived from the
   high bridge record orientation.

5. Rust cannot currently represent a bridge record that is low-kind but still
   present in the table. That distinction matters because some code iterates all
   records while `FindBridgeRecord` filters to high only.

6. Low bridge pathing through tubes is not represented as a first-class runtime
   system. The binary's low bridge cells are tied to `TubeClass`, `cell + 0x116`,
   `GetTubeAtCell`, and `UnitClass::TubeMovement`. The implementation must
   distinguish same-cell RecalcAttributes tube shells from fully initialized
   `[Tubes]` records; checked Drive/Walk direction-8 producers divide by
   `TubeClass+0x1C0`, so zero-step shells are not valid visible traversal inputs.

## Recommended next research targets

1. Trace low bridge unit entry end-to-end:
   `Can_Enter_Cell` / path node -> `IsLowBridgeCell` -> tube selection ->
   `UnitClass::TubeMovement`.

2. Dump the four low-bridge tile base globals and `DAT_0081CC20` direction table
   from the binary, then map them to each theater's TMP tile names.

3. Verify whether Rust's current `Land=Road` handling maps low bridge overlays to
   the numeric land type that the binary stores as `10`.

4. Audit all live consumers of Rust `BridgeEndpointRecord` and classify whether
   each should consume high only, low only, or both.

5. Build one retail low-bridge cell dump: overlay id, tile index, height byte,
   land type, tube index, bridge record kind, zone id before/after damage.

## Bottom line

The deeper answer is still no: we are not yet doing it the same way as
`gamemd.exe`.

The new important point is that low bridges are their own pathing model. They are
not just low-altitude versions of high bridges. The binary treats low bridge
cells as tube-backed cells, and bridge zone records carry a high-vs-low kind that
some consumers filter on. Rust's current bridge overlay and endpoint models flatten
those distinctions.
