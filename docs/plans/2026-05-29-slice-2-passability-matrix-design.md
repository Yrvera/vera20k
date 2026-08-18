# Slice 2 Passability Matrix Design

## Goal
Collapse Rust to one verified native `g_PassabilityMatrix` equivalent.

## Architecture Context
`src/sim/pathfinding/passability.rs` owns public passability helpers used by pathfinding, hierarchy prechecks, cell entry, and terrain-cost fallback. `src/sim/pathfinding/zone_build.rs` currently owns a private table for movement-class zone rebuilding. Research verifies the native table at `0x0082A594` as `MovementZone` row `0..12` by reduced `CellClass+0x4C ZoneType` column `0..7`; only value `1` passes.

## Impact Analysis
Touched modules are `passability.rs`, `zone_build.rs`, and comments that point to the old private table home. The behavioral risk is broad because pathing legality consumes these helpers, but the intended change is narrow: replace the stale public matrix with the verified native rows and make zone rebuilding import the same source.

## Chosen Approach
Use `passability.rs` as the single Rust source of truth with a `MOVEMENT_ZONE_PASSABILITY` constant. Update all matrix lookups and zone rebuilding to use that constant. Keep `SpeedType` compatibility helpers for existing fallback call sites, but document that native direct matrix readers are keyed by `MovementZone`, not `SpeedType`.

## Tiny-Detail Ledger
- Native table shape is `13 x 8 int32`, total 416 bytes at `0x0082A594`; Rust stores equivalent `u8` values because only values `1`, `2`, and `3` are read. Source: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
- Rows are `MovementZone` numeric rows `0..12`, not `SpeedType`. Source: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`; `MOVEMENTZONE_PARSER_NUMERIC_ROW_MAPPING_GHIDRA_REPORT.md`.
- Columns are reduced `CellClass+0x4C ZoneType` values, not raw TMP land type bytes. Source: `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`.
- Only matrix value `1` passes; values `2` and `3` both block. Source: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
- Fly row 9 exists and blocks outside/sentinel column 7 with value `3`. Source: `ZONE_PASSABILITY_MATRIX_READERS_GHIDRA_REPORT.md`.
- `SpeedType` remains speed/cost-domain unless a separate verified reader proves otherwise. Source: `CELLCLASS_MAPCLASS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` Slice 2.

## Design
### Components
`passability.rs` exposes constants and helper functions. `zone_build.rs` consumes the exported table.

### Interfaces / Contracts
`is_passable_for_zone` and `passability_value` read `MOVEMENT_ZONE_PASSABILITY`. `is_passable_for_speed_type` remains a compatibility fallback and is labeled as such.

### Data Flow
Resolved terrain writes reduced `zone_type`; zone building and A*/hierarchy prechecks pass that reduced column into the shared table.

### Error Handling
Out-of-range row/column and invalid movement zones return blocked/impassable, matching existing conservative behavior.

### Testing Strategy
Focused matrix tests should assert the verified 13x8 rows and exhaustive agreement between zone lookup and raw table values. Full pathfinding regression can run after the workspace is free.

## Architectural Decisions
This follows the existing sim/pathfinding boundary and avoids moving passability logic into map or rules. It intentionally does not implement Slice 3 `CellRect` validators or parser follow-ups.

## Alternatives Considered
Leaving the private `zone_build.rs` table in place would preserve duplicate authority. Expanding into Fly-map/parser/CellRect hygiene would exceed the Slice 2 boundary and overlap later slices.
