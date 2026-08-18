# 0xFFFF Terrain ClearTile Presentation Design

## Goal

Render every in-bounds `IsoMapPack5` no-tile sentinel as the active theater's
`ClearTile` across tactical terrain, atlas preparation, and radar/minimap while
preserving the sentinel as authoritative resolved terrain state for simulation.

## Architecture Context

`MapFile` normalizes raw `0x0000FFFF` records to the semantic `NO_TILE` value
`-1`. `ResolvedTerrainGrid` retains that value and owns the map-load terrain
metadata. `TerrainGrid` is the presentation handoff consumed by atlas loading,
tactical instance construction, lighting/shroud filtering, and minimap terrain
generation.

The current production handoff drops every negative `final_tile_index` before
creating `TerrainGrid`. On `XMP29U2.MAP`, 125 sentinels are inside `LocalSize`;
dropping them leaves the frame clear color visible. The active YR draw,
variant-preparation, and radar paths instead substitute `g_ClearTile` and force
sub-tile zero for `IsoTileTypeIndex == 0xFFFF`.

## Impact Analysis

- `src/map/resolved_terrain.rs`
  - Retain a map-level theater `ClearTile` presentation authority.
  - Expose one pure sentinel-to-presentation mapping.
  - Use that mapping for presentation-only variant and radar metadata.
  - Preserve `final_tile_index`, passability, buildability, smudge, and
    `AllowTiberium` semantics.
- `src/map/terrain.rs`
  - Stop removing sentinel cells from the presentation grid.
  - Emit them with the resolved `ClearTile`, sub-tile zero, existing level,
    lighting coordinate, and radar metadata.
- Existing atlas, tactical renderer, shroud, and minimap consumers require no
  new fallback logic because they already consume `TerrainGrid`.

The change does not affect map serialization, simulation tick ordering,
deterministic state hashing, savegames, snapshots, shaders, or GPU resource
formats.

## Chosen Approach

`ResolvedTerrainGrid` owns one `clear_tile_id`. A presentation helper maps:

- sentinel (`-1` or legacy `0xFFFF`) -> `(clear_tile_id, 0)`
- ordinary tile -> `(final_tile_index as u16, final_sub_tile)`

The resolved cell retains its original `final_tile_index`. Presentation-only
metadata and variants use the helper, and `TerrainGrid` uses the same helper
when emitting cells. This avoids duplicating an effective tile field on every
cell and keeps render modules independent of map sentinel encoding.

## Player-Experience Detail Ledger

- `MILESTONE-BLOCKING` — Every sentinel inside `LocalSize` emits a terrain
  cell. `XMP29U2.MAP` must produce 6,160 presentation cells, including its 125
  in-bounds sentinels.
  [runtime observation; `src/map/terrain.rs::build_terrain_grid_from_resolved`]
- `MILESTONE-BLOCKING` — Presentation uses theater `ClearTile`, not hardcoded
  tile zero.
  [doc: `ISOMETRIC_TILE_TYPE_CLASS_GHIDRA_REPORT.md` section 17]
- `MILESTONE-BLOCKING` — Sentinel identity remains unchanged for simulation.
  [scope decision; `ResolvedTerrainCell::final_tile_index`]
- `MILESTONE-BLOCKING` — Sentinel presentation forces sub-tile zero while
  preserving cell level and screen position.
  [GHIDRA `CellOverlay_TileDraw @ 0x00480350`]
- `COMPOUNDING` — Variant and atlas selection use `ClearTile` before the atlas
  is built; no first-frame atlas miss is permitted.
  [GHIDRA variant pre-pick `0x00546DA0`; `src/app_init_helpers.rs::build_tile_atlas`]
- `COMPOUNDING` — Radar/minimap receives the restored cell and ClearTile radar
  metadata.
  [GHIDRA `CellClass::GetRadarColor @ 0x0047C060`]
- `COMPOUNDING` — Existing `LocalSize` clipping and shroud filtering remain
  authoritative. Out-of-bounds and unrevealed cells are not exposed.
- `EXACTIFICATION-RESIDUAL` — Rust's general terrain-variant selector is not
  exact gamemd parity; this slice applies the existing selector consistently
  to sentinel fallbacks.
- `EXACTIFICATION-RESIDUAL` — Broader radar/minimap composition drift is
  outside this slice.
- `UNKNOWN-RISK` — A non-stock theater without `ClearTile` keeps the existing
  tile-zero fallback.

## Design

### Components

- `ResolvedTerrainGrid`: owner of theater-wide presentation fallback identity.
- `ResolvedTerrainCell`: retains semantic terrain and existing presentation
  metadata fields.
- `TerrainGrid`: concrete, sentinel-free presentation cells consumed by render.

### Interfaces / Contracts

`ResolvedTerrainGrid` provides a read-only presentation mapping for a resolved
cell. Callers must not write the mapped tile back into
`ResolvedTerrainCell::final_tile_index`.

### Data Flow

1. Theater load resolves `[General] ClearTile` to a flat tile ID.
2. Resolved terrain stores that ID once at grid scope.
3. Sentinel cells retain `final_tile_index == -1`.
4. Presentation-only radar metadata and variant selection use ClearTile.
5. Terrain-grid construction emits the sentinel as ClearTile/sub-tile zero.
6. Atlas collection, tactical rendering, and minimap generation consume the
   ordinary `TerrainCell`.

### Error Handling

When theater data or `ClearTile` is unavailable, presentation falls back to
tile zero, preserving the current missing-theater behavior. Existing atlas
lookup failure behavior remains unchanged.

### Testing Strategy

- Synthetic nonzero-ClearTile test proving:
  - sentinel state remains `-1`;
  - presentation tile uses the theater value;
  - sub-tile becomes zero;
  - elevation is preserved.
- Terrain-grid test proving sentinel cells are emitted and feed atlas keys.
- Existing viewport/shroud and visible-instance tests remain regression guards.
- Retail ignored test for `XMP29U2.MAP` proving:
  - 164 raw sentinels remain in resolved state;
  - 125 are inside `LocalSize`;
  - the presentation grid contains all 6,160 in-bounds cells;
  - every in-bounds sentinel resolves to the theater ClearTile.
- Production visual check under full visibility: no black interior holes.

## Architectural Decisions

The design follows the existing map-to-render preparation boundary:
`map/` resolves presentation facts and `render/` consumes concrete values.
It does not introduce render dependencies into simulation or duplicate
presentation state per cell.

No new technical debt is introduced. Missing-theater tile-zero fallback and
the existing general variant/minimap parity residuals remain explicitly
outside this slice.

## Alternatives Considered

- Per-cell presentation tile fields were rejected because they duplicate state,
  expand fixtures, and can drift from mutable terrain identity.
- Late substitution only in `TerrainGrid` was rejected because variant and
  radar metadata would already have been derived from the wrong tile when
  `ClearTile` is nonzero.
- Replacing the sentinel with ClearTile during resolution was rejected because
  it changes simulation-visible terrain semantics.
