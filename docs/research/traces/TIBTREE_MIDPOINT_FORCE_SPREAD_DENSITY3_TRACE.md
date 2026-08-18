# TIBTRE Midpoint Force Spread Density 3 Trace

Date: 2026-05-24

Scenario: A stock standard-YR `TIBTRE01`, `TIBTRE02`, or `TIBTRE03` terrain object exists on a map. Trace one idle animation-probability hit through midpoint ore spawning to `CellClass::SpreadTiberium(1)` and `CellClass::PlaceTiberium(type, 3)`, then compare current Rust.

Scope is limited to TIBTRE terrain midpoint ore spawning. Natural ore growth/spread, harvest, refinery behavior, terrain lighting, and editor/save edge cases are adjacent only.

## Evidence

- Ghidra read-only spot-check: `TerrainClass__AI @ 0071C730`.
- Ghidra read-only spot-check: `CellClass__SpreadTiberium @ 00483780`.
- Ghidra read-only spot-check: `CellClass__CanPlaceTiberium @ 004838E0`.
- Ghidra read-only spot-check: `CellClass__PlaceTiberium @ 00487190`.
- INI: `ini/rulesmd.ini` stock `TIBTRE01..03` have `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, and `Immune=yes`.
- INI/art evidence from prior report: all retail stock standard-theater `TIBTRE01..03` SHPs have 22 frames; art sections are theater-specific and live in YR.
- Theater INIs contain `AllowTiberium = true` on ore-permitting tilesets; binary reads this into the tile type and `CanPlaceTiberium` consults it.
- Rust: `src/sim/terrain_spawn.rs`, `src/sim/world/mod.rs`, `src/app_init.rs`, `src/sim/production/production_queue.rs`.

## Pipeline

`TerrainClass::AI` idle tick -> raw probability roll -> active terrain animation timer -> frame midpoint -> source cell lookup -> `SpreadTiberium(force=1)` -> random adjacent direction scan -> `CanPlaceTiberium` target gate -> `PlaceTiberium(type, 3)` -> overlay/data/write queues/dirty state -> visible ore cell.

## Stage Verdicts

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Standard YR liveness | Stock `TIBTRE01..03` are live TerrainClass map objects and their AI path is gated by `SpawnsTiberium && IsAnimated`, both true. | `seed_terrain_spawners` seeds terrain objects only when both parsed booleans are true. | PASS |
| Probability roll value shape | `Random::Next`, signed absolute, `% 1_000_000`, `* 1e-6`, strict `< AnimationProbability`; stock `.003` means threshold `3000 / 1_000_000`. | `TerrainSpawnProbability` samples raw `rng.next_u32()` with signed absolute, `% 1_000_000`, `* 1e-6`, strict `<`; stock parser stores `3000` micros. Exact RNG stream equality to gamemd was not computed. | UNCHECKED |
| Spawn timing | Probability-hit tick `H` sets frame `0` and arms a rate-3 timer. 22 frames gives midpoint `11`; spawn call occurs on the 11th expiry, `H+33`. | With `frame_count=22` and rate `3`, `TerrainSpawnerState` starts active on hit and emits `SpawnDue` after 11 timer expiries. | PASS |
| Same-tick spawn | No ore is placed on hit tick `H`; placement RNG is delayed until midpoint. | `tick()` returns `AnimationStarted`; `try_spawn_ore` is only called for `SpawnDue`. | PASS |
| Active animation RNG | While active, gamemd does not reroll `AnimationProbability`; it only checks timer expiry. | Active branch does not call `roll_succeeds`; probability RNG is suppressed. | PASS |
| Force flag meaning | `SpreadTiberium(1)` means `force=true`; it bypasses source spread gates including `TiberiumSpreads`, source density, source slope, and source object-list checks. | TIBTRE tick does not read `OreGrowthConfig.spreads` and runs separately from natural `ore_growth`. | PASS |
| Default/source type | If forced source has no tiberium overlay, gamemd defaults type index to `0` (`Riparius`). If a recognized source overlay survives, it maps that overlay to a tiberium type. Standard terrain `Unlimbo` normally clears same-cell tiberium overlays first. | Rust has only `ResourceType::Ore/Gem` plus a fixed `default_ore_overlay_id`; it does not model source-cell overlay-to-type mapping or the `Unlimbo` source clear. For the no-source stock case this is likely visually close, but exact type/overlay equality was not computed. | UNCHECKED |
| Neighbor selection order | `RandomRanged(0,7)`, then visits `(start + i) & 7` for `i=0..7`. | Rust uses `rng.next_range_u32(8)` and the same wrapped eight-slot pattern, but exact direction enum-to-cell-offset equality and RNG stream equality were not computed. | UNCHECKED |
| Target validation | Every candidate must pass full `CanPlaceTiberium`: playfield, no bridge/rail flags, blocking building exceptions, no spawning-terrain object, land Buildable table, no overlay, flat slope, tile `AllowTiberium`. | Rust rejects resource nodes, overlays, spawner cells, non-`AllowTiberium`, non-flat, base-build-blocked, and selected bridge flags, but it lacks the full live object-list/building exception model and exact land-table semantics. | FAIL |
| Existing ore/gem target | Existing ore/gem has an overlay and fails `CanPlaceTiberium` before `PlaceTiberium(type, 3)` is called. | Current `can_accept_tiberium` rejects existing `resource_nodes` and any overlay, so TIBTRE no longer grows existing adjacent ore in this path. | PASS |
| Placement density | TIBTRE calls `PlaceTiberium(type, 3)`; empty-cell branch writes `OverlayData = 3`. | `place_tiberium_empty` writes `overlay_data = 3` and creates `remaining = 120 * 3 = 360`. The overlay byte matches; the economic stock model remains a Rust abstraction. | PASS |
| Overlay art variant | Empty flat placement chooses a random overlay variant with `RandomRanged(0, 11)` from the tiberium type image range. | Rust picks one `default_ore_overlay_id` by first overlay name starting with `TIB`; no random variant or tiberium type image-range lookup. | FAIL |
| Queue/dirty side effects | Empty placement adds the cell to the TiberiumClass growth queue, sets `OverlayData=3`, dirties tactical screen, and marks radar terrain dirty. It does not add this empty placement to spread queue. | Rust mutates `resource_nodes` and `OverlayGrid`; `OverlayGrid` has dirty cells, but there is no TiberiumClass growth queue model or exact radar/tactical dirty equivalence for this call. | FAIL |
| Source-cell load lifecycle | Standard map load places overlays before terrain; `TerrainClass::Unlimbo` clears same-cell tiberium overlay/data for flagged overlays, so stock TIBTRE source type normally defaults to type 0. | `app_init` seeds resource nodes from overlays before `seed_terrain_spawners`; no source-cell overlay/resource clear is performed for TIBTRE terrain objects. | FAIL |

## Player-Visible Findings

1. Target validation is still not fully gamemd-equivalent. Some cells with buildings, special object-list contents, or land-table edge cases can accept or reject ore differently, changing where ore appears around a TIBTRE.
2. New ore uses a fixed default overlay instead of the binary's random flat variant from the type image range, so repeated spawns can look visually patterned.
3. Same-cell source overlays under a TIBTRE are not cleared during Rust terrain seeding, so a map that gamemd normalizes before spawning may retain extra ore/resource state in Rust.
4. Placement side effects are incomplete: no TiberiumClass growth queue and no exact radar/tactical dirty model, so later growth/minimap/update timing can diverge.

## TS/YR Boundary

This path is active in standard YR. The verified code path is `TerrainClass::AI -> SpreadTiberium(1) -> CanPlaceTiberium -> PlaceTiberium(type, 3)` for stock `TIBTRE01..03`; it is not a TS-only weed/Weeder path. The confusing names `Tiberium`, `TiberiumClass`, and `Riparius` are inherited engine terminology still used by YR for ore.

## Adjacent Findings

- The old "TIBTRE grows existing adjacent ore by 3" interpretation is stale for this caller. Direct `PlaceTiberium` has a grow-existing branch, but TIBTRE's `SpreadTiberium(force=1)` prefilters through `CanPlaceTiberium`, which requires no overlay.
- Exact gamemd RNG stream parity was not proven here; only the probability sample formula and delayed consumption shape were checked.
- Exact harvest value for a newly spawned `OverlayData=3` cell belongs to a harvest trace, not this TIBTRE spawn trace.

## Verdict Tally

PASS: 7 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0

Status: COMPLETE
