# WaterBridge TMP Terrain Bytes Reswarm 2

Date: 2026-05-27

Slot: second pathgrid re-swarm, slot 1

Scope: resolve the WaterBridge / pier-like TMP terrain-byte blocker by checking retail theater INI definitions, retail MIX TMP bytes, and current Rust classification.

Status: COMPLETE

## Executive Summary

WaterBridge `wbrdge01/02` tiles are not movement-water in the retail assets I could decode.

All available retail `wbrdge01/02` TMP files across Temperate, Snow, Urban, NewUrban, and Desert store TMP `terrain_type = 14` for every occupied sub-tile. In the Rust rules table and prior Ghidra-backed docs, TMP byte `14` maps to Rough, not Water, Beach, or Tunnel.

Therefore WaterBridge should be treated as ordinary ground/Rough movement terrain for cell-entry legality, with no low-bridge/tube semantics unless a separate map tube/bridge mechanism marks the cell. It should reject ordinary naval-only movement because it is not ZoneType Water. It is not a special "pier" movement class.

The player-visible pier/water pathing bug should not be fixed by treating `WaterBridge` as water. The stronger candidate remains the shared `PathGrid` / cell-entry legality drift where true water cells can be accepted by coarse walkability helpers.

## Verified Asset And File Evidence

### Theater INI definitions

Local INI files:

| Theater file | `[General] WaterBridge` | wbrdge tileset section | `SetName` | `FileName` | `TilesInSet` |
|---|---:|---|---|---|---:|
| `ini/temperatmd.ini` | `76` | `[TileSet0076]` line 1097 | `Water bridge` | `wbrdge` | `2` |
| `ini/temperat.ini` | `76` | `[TileSet0076]` line 1097 | `Water bridge` | `wbrdge` | `2` |
| `ini/snowmd.ini` | absent | `[TileSet0072]` line 1062 | `Water bridge` | `wbrdge` | `2` |
| `ini/snow.ini` | absent | `[TileSet0072]` line 1062 | `Water bridge` | `wbrdge` | `2` |
| `ini/urbanmd.ini` | absent | `[TileSet0090]` line 1246 | `Water bridge` | `wbrdge` | `2` |
| `ini/urban.ini` | absent | `[TileSet0090]` line 1243 | `Water bridge` | `wbrdge` | `2` |
| `ini/urbannmd.ini` | absent | `[TileSet0090]` line 1247 | `Water bridge` | `wbrdge` | `2` |
| `ini/desertmd.ini` | `76` | `[TileSet0076]` line 1110 | `Water bridge` | `wbrdge` | `2` |
| `ini/lunarmd.ini` | `76` | `[TileSet0076]` line 1141 | `Water bridge` | `wbrdge` | `0` |

Interpretation:

- Temperate and Desert expose `WaterBridge=76` in `[General]` and have two `wbrdge` tiles.
- Snow, Urban, and NewUrban ship `wbrdge` tilesets, but do not define `[General] WaterBridge`.
- Lunar declares `WaterBridge=76`, but the tileset has `TilesInSet=0`; no `wbrdge01/02.lun` assets were found.

### Retail TMP byte dump

Method:

- Read retail archives from `<ra2-install>/`.
- Parsed MIX indexes read-only with an inline script following the repo's `src/assets/mix_archive.rs` format notes.
- Parsed TMP headers using the repo's documented layout: `src/assets/tmp_decode.rs` reads `terrain_type` from per-tile header offset `+41`.

Archive names below are hash-resolved nested archive identities:

| Asset | Source archive | Bytes | Template | Terrain bytes across occupied cells | Ramp bytes |
|---|---|---:|---|---|---|
| `wbrdge01.tem` | `ra2.mix -> isotemp.mix` | `14864` | `2x4` | all `14` | all `0` |
| `wbrdge02.tem` | `ra2.mix -> isotemp.mix` | `14864` | `4x2` | all `14` | all `0` |
| `wbrdge01.sno` | `ra2.mix -> isosnow.mix` | `14864` | `2x4` | all `14` | all `0` |
| `wbrdge02.sno` | `ra2.mix -> isosnow.mix` | `15656` | `4x2` | all `14` | all `0` |
| `wbrdge01.urb` | `ra2.mix -> isourb.mix` | `14864` | `2x4` | all `14` | all `0` |
| `wbrdge02.urb` | `ra2.mix -> isourb.mix` | `14864` | `4x2` | all `14` | all `0` |
| `wbrdge01.ubn` | `ra2md.mix -> isoubn.mix` | `14864` | `2x4` | all `14` | all `0` |
| `wbrdge02.ubn` | `ra2md.mix -> isoubn.mix` | `14864` | `4x2` | all `14` | all `0` |
| `wbrdge01.des` | `ra2md.mix -> isodes.mix` | `14864` | `2x4` | all `14` | all `0` |
| `wbrdge02.des` | `ra2md.mix -> isodes.mix` | `14864` | `4x2` | all `14` | all `0` |
| `wbrdge01.lun` | not found | n/a | n/a | n/a | n/a |
| `wbrdge02.lun` | not found | n/a | n/a | n/a | n/a |

Per-cell detail: every found TMP has 8 occupied sub-tiles, each with `height = 0`, `terrain_type = 14`, and `ramp_type = 0`.

### Terrain byte meaning

Current Rust source:

- `src/assets/tmp_decode.rs:59` reads the TMP per-cell terrain byte from `offset + 41`.
- `src/sim/pathfinding/passability.rs:80-89` maps TMP byte `14` to local `LandType::Rough`.
- `src/rules/terrain_rules.rs:141-161` maps raw TMP byte `14` to the `[Rough]` land-type section.
- `ini/rulesmd.ini:30212-30220` defines `[Rough]` as `Foot=100%`, `Track=100%`, `Wheel=100%`, `Float=0%`, `Buildable=yes`.
- `ini/rulesmd.ini:30234-30242` defines `[Water]` as `Foot=0%`, `Track=0%`, `Wheel=0%`, `Float=100%`, `Buildable=no`.

Existing Ghidra-backed doc evidence:

- `docs/research/SEA_TILES_GHIDRA_REPORT.md` states that water cells come from TMP `terrain_type = 9`, which maps to binary `LandType = 2` Water, then `RecalcZoneType` maps that to ZoneType Water.
- The same report explicitly separates WaterBridge as a two-tile LAT/visual exemption from the normal sea-tile water classification.

## Current Rust Comparison

Current Rust has two relevant classification paths:

1. Fallback path without TMP bytes:
   - `src/map/resolved_terrain.rs:1340-1383` uses the tileset `SetName`.
   - Because `SetName = Water bridge` contains `"water"`, the fallback marks it as water and ground-blocked.

2. Normal asset-backed path:
   - `load_tile_metadata()` starts with the SetName fallback but then loads TMP bytes when `AssetManager` is available.
   - `merge_tmp_metadata()` stores raw byte `14`.
   - `apply_land_type_semantics()` changes classification to `[Rough]`, clearing `is_water` and `ground_blocked`.

So for normal retail asset-backed map loading, Rust should classify WaterBridge as Rough/ground, matching the decoded TMP evidence. If any test/tool path builds resolved terrain without an `AssetManager`, WaterBridge can be falsely classified as water from the SetName fallback.

This fallback mismatch is real but it does not explain "ground units drive on water." It would tend to block WaterBridge ground traversal in assetless paths, not make true water passable.

## Movement Classification Verdict

| Candidate classification | Verdict | Reason |
|---|---|---|
| Movement-water | REJECTED | TMP byte is `14`, not water byte `9`; rules classify `14` as Rough. |
| Beach | REJECTED | TMP byte is not `10`; no beach semantics. |
| Tunnel / low-bridge tube | REJECTED for TMP alone | TMP byte is not `5`; no tube/low-bridge semantics from the tile. |
| High bridge deck | REJECTED for TMP alone | No high-bridge overlay/deck flag comes from `wbrdge` TMP bytes. |
| Ordinary ground/Rough | VERIFIED | Retail TMP byte `14` plus rules `[Rough]` produce ground-passable, non-water terrain. |
| Blocked terrain | REJECTED for normal ground movement | `[Rough]` has nonzero Foot/Track/Wheel and `Buildable=yes`; water/naval-only movement should still reject it because it is not ZoneType Water. |

## Rust-Facing Handoff

Required:

- Do not special-case `WaterBridge` as water in the native cell-entry evaluator.
- Let decoded TMP byte `14` classify WaterBridge as Rough/Ground when TMP metadata is available.
- Keep water rejection focused on true water cells: TMP byte `9` / ZoneType Water.
- If exact tests construct theater metadata without assets, avoid using SetName `"Water bridge"` as final passability. A fixture helper should either provide TMP metadata or override WaterBridge fallback to non-water.

Useful acceptance tests:

- `waterbridge_tmp_tiles_decode_as_rough_not_water`:
  - Load retail `wbrdge01/02` for available theaters.
  - Assert raw terrain byte `14`.
  - Assert resolved `is_water == false`, `zone_type == GROUND`, and `ground_walk_blocked == false`.

- `ground_unit_may_enter_waterbridge_but_not_adjacent_true_water`:
  - Place a ground unit near one WaterBridge tile and one true water tile.
  - Assert native-shaped cell-entry accepts the WaterBridge cell and rejects the true water cell.

Do not do:

- Do not fix pier/water drift by making every tileset whose name contains `"Water"` become movement-water after TMP data exists.
- Do not treat `WaterBridge` as a low bridge/tube without separate tube facts.
- Do not treat WaterBridge as a high bridge deck without bridge overlay/facts.

## Remaining Uncertainty

No asset decoding blocker remains for WaterBridge TMP terrain bytes.

Two adjacent uncertainties remain outside this slot:

- Exact concrete bad-map repro cells still need classification. A visible "pier" cell might be true water, WaterBridge rough ground, high bridge overlay, low bridge/tube, beach, or another tile entirely.
- Fresh live `gamemd.exe` decompile was not used in this slot. The movement classification conclusion is based on retail asset bytes plus existing Ghidra-backed docs for the TMP terrain-byte table and ZoneType water path.

