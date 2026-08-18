# Zone_precheck Production Hierarchy Builder Brainstorm

Date: 2026-05-24

## Goal

Build the production `Zone_precheck` hierarchy data that `gamemd.exe` keeps beside
the map, without yet changing public route selection. The immediate value is to
replace the current hand-authored hierarchy fixtures with a real map-derived
hierarchy so future bridge pathing work can test against the same data shape that
the binary consumes.

Public hierarchy-gated A* should remain guarded until blocker-neighbor counts and
the remaining bridge/tube record production details are verified. A built hierarchy
must not silently make cross-zone routing depend on incomplete data.

## Evidence

- Research source:
  `docs/research/ZONE_PRECHECK_HIERARCHY_FULL_BUILD_CONTRACT_GHIDRA_REPORT.md`
- Queue source:
  `docs/implementation-queue/2026-05-24-implementation-queue-bridge-zone-production-hierarchy.md`
- Current Rust surfaces:
  - `src/sim/pathfinding/zone_map.rs`
  - `src/sim/pathfinding/zone_hierarchy.rs`
  - `src/sim/pathfinding/zone_build.rs`
  - `src/sim/pathfinding/zone_search.rs`
  - `src/sim/world/mod.rs`

Verified binary constraints that matter to this design:

- Full hierarchy build scans levels `2 -> 1 -> 0`.
- Level block sizes are `8x8`, `4x4`, and `2x2`.
- Zone id `0` is the invalid sentinel; real ids start at `1`.
- Discovery is row-major. Cells with reduced type `7` are skipped.
- Flood grouping requires same reduced zone type and neighbor height delta `< 2`.
- Level 0 and 1 records store the parent id from the next coarser level. Level 2
  parent is `0`.
- Temp edge buckets are selected by `(from & 0xF) << 4 | (to & 0xF)`.
- Temp edge dedup is exact packed-pair dedup, not undirected canonicalization.
- Final edges are emitted bucket order first, then insertion order inside each
  bucket; each temp pair emits both directed edges, low halfword first.
- Bridge/tube temp edges are appended after scanline discovery and are zero-flagged.
- `Zone_precheck` consumes the hierarchy plus the movement-zone passability row.
  The graph itself is map-level data, not per-mover data.

## Current Fit

`World::rebuild_zone_grid` is the correct integration point. It already has:

- `PathGrid`
- `ResolvedTerrainGrid`
- bridge endpoint records
- map width and height

`ZoneGrid::build_with_terrain` currently builds per-`MovementZone` flat maps,
adjacency, and super-zone connectivity. It initializes `hierarchies` empty.

`zone_search` already keeps production hierarchy use behind
`blocker_neighbor_counts.is_some()` plus `hierarchy_for(mz)`. That means the
hierarchy can be built and stored now without changing normal pathfinding results.

## Alternatives

### A. Map-Level Production Builder, Dormant in Search

Add a new private builder module for the hierarchy and store one map-level
`ZoneHierarchy` on `ZoneGrid`. Keep `hierarchy_for(mz)` as the access method, but
make it return the same map-level hierarchy for all movement zones.

Pros:

- Matches binary ownership: one map hierarchy plus movement-zone passability rows.
- Avoids cloning a large hierarchy for every `MovementZone`.
- Keeps public pathfinding behavior unchanged because search is still blocker-count
  gated.
- Gives focused tests a real hierarchy builder to assert against.
- Keeps `zone_build.rs` from growing further past the project file-size guidance.

Cons:

- Requires a small internal `ZoneGrid` storage refactor.
- Current `set_hierarchy(mz, ...)` fixture helper needs replacement or adaptation.
- Bridge/tube injection still needs a boundary for the exact three-pair record
  producer, because current Rust bridge endpoint records may not fully represent
  every binary temp edge pair.

### B. Per-MovementZone Hierarchies

Build or clone a hierarchy into `ZoneGrid.hierarchies` for each movement zone.

Pros:

- Minimal shape change to existing `ZoneGrid`.
- Existing test helper APIs can stay mostly intact.

Cons:

- Does not match the binary data model.
- Wastes memory on identical graph copies.
- Makes future bugs more likely because graph identity and movement-zone
  passability become entangled.

### C. Wait for Blocker Counts and Bridge Record Batch 2

Do no production builder work until every remaining consumer detail is verified.

Pros:

- Lowest risk of over-claiming bridge parity.

Cons:

- Leaves tests dependent on fixtures.
- Delays validation of level/block/edge-order behavior that is already verified.
- Does not help close the known corridor-approximation gap.

## Recommended Design

Use alternative A.

Add `src/sim/pathfinding/zone_hierarchy_build.rs` as a private module with a
single public crate-level entry point:

```rust
pub(crate) fn build_zone_hierarchy(
    path_grid: &PathGrid,
    resolved_terrain: &ResolvedTerrainGrid,
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
    height: u16,
) -> ZoneHierarchy
```

`ZoneGrid::build_with_terrain` should call this only when `resolved_terrain` is
`Some`. `ZoneGrid::build`, which lacks resolved terrain, should continue to build
flat maps only and leave hierarchy absent.

The builder should derive a shared source cell array once:

- reduced zone type from the same Rust surface used by flat movement-class zone
  rebuilding
- height byte from resolved terrain, matching the binary source copy
- type `7` as the skipped/outside value

If the flat builder's reduced-type helper is currently private, move the helper
behind a small shared private function rather than duplicating the classification
logic.

## Data Shape

Keep using the existing hierarchy structs where possible:

- `ZoneHierarchy`
- `ZoneLevelGraph`
- `ZoneRecord`
- `ZoneEdgeRecord`

Add builder-private types only for construction:

- `HierarchySourceCell { zone_type: u8, height: u8 }`
- `TempEdge { low: ZoneId, high: ZoneId, flag: u8 }`
- `TempBuckets`, a 256-bucket vector/array preserving insertion order

The temp edge representation should keep the binary low/high halfword semantics
explicit so final emission can append the low-halfword directed edge first, then
the reverse edge.

## Build Algorithm

For each level in `2, 1, 0`:

1. Compute block size as `1 << (level + 1)`.
2. Initialize per-cell zone ids to `0`.
3. Scan cells row-major.
4. Skip source type `7` and already assigned cells.
5. Start a new zone id and flood only inside the current aligned block.
6. Flood cells with same reduced type and height delta `< 2`.
7. When a boundary neighbor is encountered, append a temp edge with exact
   packed-pair dedup.
8. Set the zone record's parent from the next coarser level for levels `0` and
   `1`; use parent `0` for level `2`.
9. After scanline discovery for that level, append bridge/tube temp links.
10. Emit final directed edges in bucket order and stored insertion order.

The flood helper should intentionally model the binary's scanline left/right
ordering closely enough to preserve edge insertion order. A generic BFS/DFS would
probably produce correct connectivity but can produce the wrong chosen route when
`Zone_precheck` sees equal-cost alternatives.

## Bridge/Tube Link Boundary

Do not bury bridge/tube temp-edge production inside the scanline fill.

Create a builder-private bridge link provider with a narrow output:

```rust
struct HierarchyBridgeLink {
    a: (u16, u16),
    b: (u16, u16),
}
```

The first implementation can use currently verified active endpoint records to
append zero-flagged links after scanline discovery. However, the design must keep
this as a separable function because the binary helper computes three connection
pairs per active bridge/tube record, and current Rust endpoint records may not yet
carry enough information to reproduce all of those pairs. Public bridge parity
should not be claimed until that producer is verified and wired through.

Bad bridge/tube coordinates should be clamped to the valid cell range when looking
up zone ids, matching the verified binary behavior.

## ZoneGrid Changes

Prefer replacing:

```rust
hierarchies: BTreeMap<MovementZone, ZoneHierarchy>
```

with:

```rust
hierarchy: Option<ZoneHierarchy>
```

Keep:

```rust
pub(crate) fn hierarchy_for(&self, mz: MovementZone) -> Option<&ZoneHierarchy>
```

but ignore `mz` internally. This preserves the current caller contract while
aligning storage with the binary.

Any mutation path that can invalidate zones should clear the map-level hierarchy:

- `map_mut`
- `adjacency_mut`
- `set_super_zone`
- incremental update fallback paths

Full rebuild repopulates it when resolved terrain is available.

## Search Behavior

Do not remove the existing guard in `zone_search`:

- `blocker_neighbor_counts.is_some()`
- `hierarchy_for(mz).is_some()`
- no explicit tube scenario

This keeps production hierarchy construction separate from production route
activation. The player-visible pathfinder should remain on the current compatibility
path until blocker counts and exact bridge/tube link production are both handled.

## Tests

Add focused builder tests before any search activation:

- all-open terrain splits by `8x8`, `4x4`, and `2x2` block boundaries
- ids start at `1` and are assigned in row-major first-discovery order
- type `7` cells remain zone `0`
- same type with height delta `1` joins; height delta `2` splits
- level 0 and 1 parent ids point to the next coarser level at the representative
  cell
- final edge order follows temp bucket order plus insertion order
- temp pair dedup is exact and keeps opposite packed pairs distinct
- bridge endpoint links append after scanline links and carry flag `0`
- `ZoneGrid::build_with_terrain` populates the hierarchy
- `ZoneGrid::build` leaves the hierarchy absent
- zone mutation APIs clear the map-level hierarchy

Do not add tests that assert public route changes from the new builder yet.

## Acceptance Criteria

- `ZoneGrid` can build a real `ZoneHierarchy` from resolved terrain.
- The builder is deterministic and uses only fixed/integer map data.
- Existing public pathfinding tests continue to pass without requiring blocker
  counts.
- New hierarchy-builder tests cover level block sizes, parent chains, zone `0`,
  height/type grouping, edge order, and bridge-link insertion position.
- Any remaining bridge/tube link incompleteness is explicitly isolated behind the
  link provider and not presented as final bridge pathing parity.

## Next Implementation Step

Implement the private builder and map-level `ZoneGrid` storage refactor first.
After that, run focused tests around `zone_hierarchy`, `zone_map`, and
`zone_search`. Then run a second research/implementation pass for the exact
three-pair bridge/tube link producer before enabling hierarchy-gated public A*.
