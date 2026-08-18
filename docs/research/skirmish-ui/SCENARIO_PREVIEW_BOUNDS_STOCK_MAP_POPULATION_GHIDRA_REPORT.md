# SCENARIO_PREVIEW_BOUNDS_STOCK_MAP_POPULATION Ghidra Report

Date: 2026-05-21  
Target: `ScenarioClass+0x112C..0x113C` preview source bounds/count population for stock maps lacking `[Header]` preview bounds  
Mode: `/re-investigate` exhaustive-slice  
Binary: `gamemd.exe`  
Scope: Writers/readers/default behavior for `+0x112C`, `+0x1130`, `+0x1134`, `+0x1138`, `+0x113C` only  
Non-scope: Full `ScenarioClass`, complete map-list metadata decoding, full preview image loader internals

## Summary

The standard YR stock-map population path found for these fields is `[Header]` parsing in `ScenarioClass__Read_INI_Basic @ 0x00689E90`. It reads `StartX`, `StartY`, `Width`, `Height`, `NumberStartingPoints`, and then `WaypointN` entries into `ScenarioClass+0x112C..+0x113C` and `+0x1140/+0x1144`.

I did not find a stock-map fallback that derives these fields from `[Map] LocalSize`. `LocalSize` is read during scenario/map initialization, but the verified consumers call `RadarClass__ComputeRadarMapBounds @ 0x00654490` and update radar/map bounds fields, not `ScenarioClass+0x112C..+0x113C`.

For maps missing `[Header]` preview keys, the verified parser default is "keep the current field value", not "use LocalSize". No constructor/reset writer that assigns fixed default values to `+0x112C..+0x113C` was found in the scoped decompilation. Therefore a truly missing `[Header]` path does not synthesize preview bounds in the verified stock-map load path.

Retail-data sampling did not support the premise that normal shipped multiplayer stock maps lack these `[Header]` fields: local stock map files and embedded map data contain `[Header]` with `StartX` and `NumberStartingPoints` entries. Campaign or non-skirmish maps can have different preview requirements, but that is outside this slot.

## Scoped Fields

| Offset | Verified role in scoped functions | Active in YR |
|---|---|---|
| `ScenarioClass+0x112C` | Preview/source `StartX` used to project start markers | Yes; read/write in standard scenario INI and preview draw paths |
| `ScenarioClass+0x1130` | Preview/source `StartY` used to project start markers | Yes; read/write in standard scenario INI and preview draw paths |
| `ScenarioClass+0x1134` | Preview/source `Width` divisor | Yes; read/write in standard scenario INI and preview draw paths |
| `ScenarioClass+0x1138` | Preview/source `Height` divisor | Yes; read/write in standard scenario INI and preview draw paths |
| `ScenarioClass+0x113C` | `NumberStartingPoints` count gate | Yes; read/write in standard scenario INI and preview draw paths |

## Writers

### `ScenarioClass__Read_INI_Basic @ 0x00689E90`

Active in YR: Yes. Evidence: called from `ScenarioClass__Full_Init @ 0x00686B20`, which is called by `ScenarioClass__Read_Scenario_INI @ 0x00686730` in the scenario load path.

Verified finding: this is the standard stock-map writer for the scoped fields. It reads:

- `[Header] StartX` into `+0x112C`
- `[Header] StartY` into `+0x1130`
- `[Header] Width` into `+0x1134`
- `[Header] Height` into `+0x1138`
- `[Header] NumberStartingPoints` into `+0x113C`
- `[Header] WaypointN` entries into the adjacent start-point pair array at `+0x1140/+0x1144`

Default behavior: each scalar read uses the current field value as the default. The waypoint read uses the current pair as the default. Thus missing keys preserve prior/current field contents; this function does not derive fallback preview bounds from `[Map] LocalSize`.

### `FUN_0058B820`

Active in YR: Conditional. Evidence: called from map-generation flow `FUN_00594B50`, after random-map generation and zone allocation.

Verified finding: this random-map path computes start-point globals and source extrema from generated map/playfield cells, then copies them into `ScenarioClass`:

- `+0x113C` from the generated start count global
- `+0x1140/+0x1144` from generated start coordinate globals
- `+0x112C/+0x1130/+0x1134/+0x1138` from generated preview/source bound globals

This is a real writer, but it is conditional random-map population, not a stock-map missing-`[Header]` fallback.

### `FUN_0058BB30`

Active in YR: Conditional. Evidence: xrefs from shell/setup flows including `FUN_006AE6E0` and `FUN_005ED5A0`.

Verified finding: this function copies already-existing global preview/start data into `ScenarioClass+0x112C..+0x113C` and `+0x1140/+0x1144`. It does not parse map INI data and does not compute bounds from `LocalSize`.

Interpretation: this supports selected-map or generated-map cached metadata transfer in shell setup. It is not verified as a stock-map fallback for absent `[Header]` preview bounds.

### `FUN_00596300`

Active in YR: Conditional. Evidence: random-map dialog/control path.

Verified finding: on the random-map generate path, this function zeroes the global preview/start cache and, when a `ScenarioClass` instance exists, zeroes `+0x113C` and `+0x112C/+0x1130/+0x1134/+0x1138` before later terrain preview generation.

This explains transient random-map defaults; it is not a stock-map loader default.

## Non-Writers Checked

### `[Map] LocalSize`

Active in YR: Yes, but not as a scoped-field writer. Evidence: string `LocalSize @ 0x00820164` has xrefs in `ScenarioClass__Full_Init @ 0x00686B20`, `Read_Map_Section_And_IsoMapPacks @ 0x004ACE70`, and map writing. The read paths feed `RadarClass__ComputeRadarMapBounds @ 0x00654490`.

Verified finding: `RadarClass__ComputeRadarMapBounds` writes radar/map bounds fields around the radar object, not `ScenarioClass+0x112C..+0x113C`. No verified xref connected `LocalSize` to these ScenarioClass preview fields.

### `ScenarioClass__Constructor @ 0x006832C0` / reset helper `FUN_00683610`

Active in YR: Yes, but not as scoped-field default writers. Evidence: constructor calls reset helper; decompilation shows adjacent field initialization such as `+0x11E0` and `+0x11E4`, but not `+0x112C..+0x113C`.

Verified finding: no explicit fixed default assignment to the scoped preview fields was found in these initialization functions.

## Readers

### `DrawStartPositions @ 0x00640710`

Active in YR: Yes, conditional on a preview object and start count. Evidence: called from shell preview painting path; direct decompilation reads the scoped fields.

Verified behavior:

- Reads `+0x113C` and draws only when count is `1..8`.
- For each start marker, subtracts `+0x112C/+0x1130` from the start coordinate pair.
- Divides by `+0x1134/+0x1138` to scale into the preview rectangle.
- Uses adjacent start coordinate pairs at `+0x1140/+0x1144`.

Player-visible effect: if the count is zero or outside the accepted range, start markers are skipped. If count is valid but bounds are invalid, projection would be invalid; standard stock maps avoid this by carrying `[Header]` bounds.

### `FUN_00640A40`

Active in YR: Conditional. Evidence: preview/thumbnail helper reads `ScenarioClass+0x1134/+0x1138` while scaling start-marker coordinates.

Verified finding: reader only. It does not populate the scoped fields.

### Setup/status formatting around `0x005E7EFC`

Active in YR: Conditional. Evidence: instruction scan shows reads of `+0x113C`, `+0x1130`, `+0x112C`, `+0x1138`, and `+0x1134` near a formatting path in setup code.

Verified finding: reader/logging or UI-status use only; no scoped-field population observed there.

## Retail Data Check

Active in YR: Yes as local installed retail data evidence. Evidence: read-only `rg -a` over `C:/Users/enok/Documents/Command and Conquer Red Alert II/` found stock map files and embedded map data containing `[Header]`, `StartX=`, and `NumberStartingPoints=`.

Finding: sampled shipped maps such as local `.yro` files and embedded `multimd.mix` map data carry `[Header]` preview metadata. This supports the binary finding that stock-map preview population normally comes from `[Header]`, not a missing-`[Header]` fallback.

## Default Behavior For Missing `[Header]` Preview Bounds

Active in YR: Conditional. Evidence: `ScenarioClass__Read_INI_Basic @ 0x00689E90` uses current field values as defaults for `StartX`, `StartY`, `Width`, `Height`, and `NumberStartingPoints`; constructor/reset scoped decompilation did not reveal fixed assignments to these offsets.

Verified default: missing scalar keys preserve the current field value. Missing waypoint keys preserve the current waypoint pair. No verified stock-map fallback computes these fields from `LocalSize`.

Inference, bounded by evidence: for a fresh zeroed scenario object, absent `[Header]` preview keys would leave zero count/bounds, causing `DrawStartPositions` to skip markers because `+0x113C` is zero. If the object is reused without clearing these fields, absent keys could preserve previous contents. I did not verify allocator zeroing or all object reuse paths in this slot.

## Implementation-Relevant Conclusion

Active in YR: Yes for stock-map `[Header]` population; No evidence for stock-map `LocalSize` fallback.

For VERA20k parity, the verified stock-map behavior is:

1. Populate preview source bounds and start count from `[Header]` keys when present.
2. Do not substitute `[Map] LocalSize` as a verified fallback for `ScenarioClass+0x112C..+0x113C`.
3. Treat random-map/global-cache writers as conditional shell/random-map paths, not as ordinary stock-map INI fallback.
4. If `[Header]` preview keys are absent, preserve existing field defaults rather than synthesizing bounds.

## Open Questions Kept Out Of Scope

- Exact full ownership and lifetime of the `DAT_00AB*` global preview cache outside the confirmed random-map and shell-copy functions.
- Whether any non-standard or modded map without `[Header]` relies on prior field contents through object reuse.
- Full campaign preview behavior when no skirmish start markers are expected.

