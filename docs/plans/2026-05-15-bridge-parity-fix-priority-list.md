# Bridge Parity Fix Priority List

Date: 2026-05-15

Research sources:

- `docs/research/BRIDGE_MAP_LOAD_AND_BRIDGEHEAD_TRANSITIONS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`

Purpose: preserve the recommended implementation order for bridge/pathfinding
parity work. This is a fix-priority list, not an implementation plan.

## Priority 1: Correct Authoritative Bridge Cell Facts

Primary area: `src/map/resolved_terrain.rs`

Fix this first because every downstream system consumes resolved terrain. If this
layer invents deck cells, bridgeheads, or deck heights differently from
`gamemd.exe`, pathfinding and locomotion will debug against fake input.

Target behavior:

- High bridge map-load facts should be produced by a `SetBridgeDirection`-equivalent
  stamp from high bridge overlay anchor IDs.
- Rust should separately represent the binary bridge facts instead of flattening
  them into only `bridge_walkable` / `bridge_transition`:
  - bridge body/render marker equivalent to `0x80`;
  - bridge structural cell equivalent to `0x100`;
  - bridgehead/transition equivalent to `0x200`;
  - bridge state byte;
  - bridge anchor relation and direction.
- Current side-cell expansion, connected-component deck-height normalization, and
  gap-fill behavior should be removed, disabled, or kept behind proof from a map
  dump until verified against `gamemd.exe`.

Parity risk if skipped:

- A* may accept cells YR rejects or reject cells YR accepts.
- Render height and sim height may disagree.
- Damage/repair code may mutate the wrong cells.

## Priority 2: Replace Broad Bridgehead/Ramp Detection

Primary area: `src/map/resolved_terrain.rs`

Current broad checks such as `tileset_index == BridgeSet/WoodBridgeSet` are too
coarse for parity.

Target behavior:

- Detect high bridge ramp/bridgehead tiles using exact binary-style
  `IsoTileTypeIndex` plus `cell + 0x11A` subtile/height byte checks.
- Preserve the distinction between bridge ramp tile recognition and bridge
  traversal flags.

Parity risk if skipped:

- Units may fail to enter a bridge.
- Units may enter from a cell that should not be a bridgehead.
- Plateau-to-bridge and chasm-side bridgehead cases remain unstable.

## Priority 3: Rework High-Bridge Traversal Gate

Primary areas:

- `src/sim/pathfinding/core.rs`
- `src/sim/movement/movement_bridge.rs`

Do this after bridge cell facts are corrected. Pathfinding parity is only
meaningful once the cell facts match the binary.

Target behavior:

- Keep A* node height explicit.
- Mirror `CheckBridgeTraversal` rules using source cell, destination cell,
  carried target/path height, signed levels, slope index, and separate bridge
  flags.
- Preserve the distinction between:
  - bridge structural flag (`0x100`);
  - bridgehead/transition flag (`0x200`);
  - bridge body/render marker (`0x80`).
- Match binary layer selection closely enough that ground occupancy/list and bridge
  occupancy/list do not collapse into one boolean.

Parity risk if skipped:

- Bridge entry/exit may work in one direction but fail in another.
- Units can route around valid bridges or try to cross invalid ones.
- Blocking on bridge deck vs ground under bridge can diverge.

## Priority 4: Add Bridge Kind To Zone Records

Primary areas:

- `src/sim/bridge_state/mod.rs`
- `src/sim/pathfinding/zone_build.rs`
- `src/sim/pathfinding/zone_map.rs`

The binary `BridgeRecord` is a 16-byte record with a kind field:

- `bridge_kind = 0`: high bridge.
- `bridge_kind = 1`: low bridge.

`MapClass::FindBridgeRecord` skips low bridge records.

Target behavior:

- Add a high/low bridge kind to Rust endpoint records.
- Audit every consumer of `BridgeEndpointRecord` and decide whether it should use:
  - high only;
  - low only;
  - both.
- Do not inject low bridge records into high-bridge zone/redirect behavior that
  mirrors `FindBridgeRecord`.

Parity risk if skipped:

- Low bridge records can affect high-bridge zone lookup.
- Destroyed bridge redirects can choose records the binary would ignore.
- Cross-zone path checks can report connectivity that YR would not.

## Priority 5: Implement Low Bridge Tube Semantics

Primary areas:

- terrain resolution / cell facts;
- low bridge pathing;
- zone records;
- movement through tubes.

Do this after high bridge basics unless the currently failing map uses low
bridges.

Target behavior:

- Represent a tube index equivalent to `cell + 0x116`.
- Match `IsLowBridgeCell`: valid tube index and numeric `LandType == 10`.
- Construct low bridge tube records from exact terrain/tile identity during
  terrain attribute resolution.
- Treat low bridge traversal as tube-backed movement, not as high bridge deck
  traversal with a lower height.

Parity risk if skipped:

- Low bridges may draw correctly while pathing incorrectly.
- Units may treat low bridges as ordinary road, water, or high bridge cells.
- Any map depending on units crossing under/through low bridges will diverge.

## Working Rule

Do not chase A* symptoms before fixing bridge cell facts. The intended order is:

1. Cell facts.
2. Bridgehead/ramp detection.
3. High-bridge traversal.
4. Bridge records/zones.
5. Low bridge tubes.

This order minimizes rework because each later layer depends on the earlier one.
