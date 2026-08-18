# Lighting Draw Consumers Cell Fields - Ghidra Report

Target: `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS`  
Date: 2026-05-22  
Scope: draw/render consumers of `CellClass+0x34` and scalar lighting fields `CellClass+0x104..+0x114`, especially TMP terrain, overlays, terrain objects, `TechnoClass_DrawSHP`, `AnimClass`, and building body handoff.  
Status: COMPLETE for the scoped consumer map; palette-table internals remain out of scope.

## Target Question

Which active standard YR render paths consume `CellClass+0x34` and `CellClass+0x104..+0x114`, which fields feed palette/light conversion versus scalar brightness/Z-style draw arguments, and what renderer-facing Rust shape follows from those consumers?

## Non-goals

- Do not reverse the internals of `CC_Draw_Shape`, `TMP_TileBlitter`, `LightConvertClass__Constructor`, `ConvertClass`, or blitter tables.
- Do not revisit map `[Lighting]` formula, lamp falloff, dirty-cell scheduling, `ExtraLight=` parser ownership, or `LightConvert` cache construction except where draw consumers read the resulting fields.
- Do not edit Rust, INI data, Ghidra state, `.swarm-claims.md`, or existing reports.
- Do not classify TS-style fog behavior beyond the visible active/conditional draw paths reached in this slice.

## Evidence Needed To Mark COMPLETE

- Decompile `0x00483E30` to confirm the stored cell-field bundle.
- Decompile active draw consumers named by prior cache reports: `0x00480350`, `0x0047F6A0`, `0x0071C250`, `0x00705E00`, `0x00423200`, `0x004D1890`, and building body `0x0043D290`.
- Confirm for each material consumer whether it reads `+0x34`, `+0x10A`, `+0x10C`, `+0x10E`, or related fields.
- Separate direct `LightConvertClass*`/palette-convert use from scalar arguments passed into `TMP_TileBlitter`, `CC_Draw_Shape`, `Tactical__AdjustForZ`, or `TechnoClass_DrawSHP`.
- Provide Rust-facing handoff with at least one concrete test-name proposal.

## Stop Conditions

- Stop before low-level `CC_Draw_Shape`/blitter implementation details.
- Stop if a consumer expands into a whole render subsystem.
- Stop if evidence only restates prior reports without adding the consolidated draw-consumer map.

## Verified Findings

### 1. `0x00483E30` owns the render-facing cell light bundle

Evidence: `FUN_00483E30` decompile writes `CellClass+0x34` and fields `+0x104`, `+0x108`, `+0x10A`, `+0x10C`, `+0x10E`, `+0x110`, `+0x112`, `+0x114`. The sentinel/default branch writes neutral values: `+0x104=0x10000`, `+0x108=0`, and `+0x10A/+0x10C/+0x10E/+0x110/+0x112/+0x114=1000`.

Active in YR: Yes. Prior reports and this slot confirm draw paths lazily call `0x00483E30` when `cell+0x34` is null; this makes the bundle live in standard rendering.

Consumer meaning in this slice:

| Field | Verified role | Direct draw consumers in this slot | Active in YR |
|---|---|---|---|
| `Cell+0x34` | cached `LightConvertClass*` | Directly passed by TMP terrain tile draw; read by anim cell-palette branches; lazy-init guard in many paths | Yes |
| `Cell+0x104` | stored dword light scale/normalization metadata | No direct audited draw consumer found | Yes as stored state; direct draw use not found |
| `Cell+0x108` | stored word auxiliary light metadata | No direct audited draw consumer found | Yes as stored state; direct draw use not found |
| `Cell+0x10A` | alternate/top scalar light/depth input | overlays, terrain-object branch, anims, building body/Z handoff, queued/cached draws | Yes |
| `Cell+0x10C` | common scalar brightness/light input | TMP terrain, most overlays, terrain objects, Techno SHPs, anims, smudge/queued draws | Yes |
| `Cell+0x10E` | alternate/bottom scalar light input | overlay shadow/special branch | Yes |
| `Cell+0x110/+0x112/+0x114` | normalized RGB cache-key mirror | Not directly drawn here; used to match/create `LightConvertClass` profile via `+0x34` | Yes as cache key; direct draw use not found |

### 2. TMP terrain tile draw directly consumes `Cell+0x34` and `Cell+0x10C`

Evidence: `CellOverlay_TileDraw @ 0x00480350` checks `cell+0x34 == 0`, calls `0x00483E30` to initialize it, then calls `TMP_TileBlitter` with `*(cell+0x34)` and `(short)*(cell+0x10C)`. Assembly range `0x00480370..0x00480480` was disassembled in this slot as a read-only spot-check range for the lazy-init and blitter call region.

Active in YR: Yes. This is the standard map tile draw path for TMP terrain cells.

Renderer classification: `+0x34` is palette/light-convert input for tile blitting; `+0x10C` is a scalar light/brightness argument passed alongside it. A renderer needs both concepts for terrain parity.

### 3. Overlay body draw is scalar-field driven after lazy LightConvert init

Evidence: `CellClass__DrawOverlay_Body @ 0x0047F6A0` checks `cell+0x34 == 0` and calls `0x00483E30`. Its `CC_Draw_Shape` branches then feed scalar fields:

- `+0x10E` in the branch gated by `cell+0x140 & 0x80`.
- `+0x10A` for one overlay-type branch.
- `+0x10C` for ordinary overlay draw branches.
- Fixed `1000` for some tiberium growth/damaged overlay branches.

Assembly range `0x0047F720..0x0047FBD0` was disassembled in this slot as the read-only spot-check range for the lazy-init and shape-call region.

Active in YR: Yes. This is the standard overlay body drawing path for ore, walls, bridges/overlay bodies, and related overlay SHPs.

Renderer classification: overlay draw cannot use a single per-cell RGB tint only. It needs branch-selectable scalar fields, especially `+0x10C` for common overlays and `+0x10A/+0x10E` for special branches.

### 4. Terrain object draw uses `+0x10C` normally and `+0x10A` for a type-flag branch

Evidence: `TerrainClass__Draw_It @ 0x0071C250` gets its cell, lazy-inits `cell+0x34`, then chooses:

- `(short)(cell+0x10C)` for normal terrain object draw.
- `(short)(cell+0x10A)` when `TerrainTypeClass+0x2B1` is true, with an added Y offset.

The selected scalar is passed to `CC_Draw_Shape`. Assembly range `0x0071C300..0x0071C3F0` was disassembled as the checked draw-call region.

Active in YR: Yes. This path is standard for map terrain objects, including trees, rocks, and lamp-post object art.

Renderer classification: terrain-object sprites follow cell scalar lighting, not just terrain-tile tint. Do not leave terrain objects at `[1,1,1]` when the cell is lit.

### 5. `TechnoClass_DrawSHP` consumes cell scalar lighting for object sprites

Evidence: `TechnoClass_DrawSHP @ 0x00705E00` has multiple cell-light branches:

- Building-specific/type branch gets the object's render cell, lazy-inits `cell+0x34`, then replaces a draw parameter with `(short)(cell+0x10C)`.
- A non-visible/owner/shroud-related branch gets a cell through `vtable+0x1BC`, lazy-inits `cell+0x34`, then sets a draw parameter from `(short)(cell+0x10C)`.
- Later calls transform this scalar through `vtable+0x464(param_7)` before `CC_Draw_Shape`.

Assembly range `0x00705EF0..0x00706140` was disassembled as the checked lazy-init/scalar-read region.

Active in YR: Yes. `TechnoClass_DrawSHP` is the standard SHP draw helper for building/infantry/unit-like object sprites.

Renderer classification: Techno sprites receive cell scalar lighting. The current Rust model where entities merely look up an RGB tint is too collapsed for exact parity because gamemd passes scalar light fields through object draw hooks before shape drawing.

### 6. `AnimClass__DrawIt` has both LightConvert-pointer and scalar-light branches

Evidence: `AnimClass__DrawIt @ 0x00423200` uses cell lighting in several branches:

- For one cell-palette branch, if an animation instance has no explicit convert, it can fetch the current cell and use `(short)(cell+0x10A)`.
- In another branch it lazy-inits `cell+0x34`, stores that pointer as the active convert, and if the animation type allows cell lighting, uses `(short)(cell+0x10C)`.
- For player/house color or explicitly configured animation branches, it can use global color schemes or fixed `1000` instead of cell scalar lighting.

Assembly range `0x00423280..0x00423380` was disassembled as the checked cell-read region.

Active in YR: Conditional per animation type flags, but the code path is live in standard YR animation drawing.

Renderer classification: effects/animations need per-type control over whether they use cell palette conversion, cell scalar lighting, a fixed/global convert, or fixed brightness. Do not apply map lighting indiscriminately to every animation frame.

### 7. Building body uses `+0x10A` in the Z/depth handoff, not the lamp RGB cache

Evidence: `BuildingClass_DrawBody @ 0x0043D290` repeatedly fetches the building cell, reads `(short)(cell+0x10A)`, adds signed `(short)(BuildingTypeClass+0x1548)` for `ExtraLight=`, then passes the result through `vtable+0x1D0`, `Tactical__AdjustForZ`, and `TechnoClass_DrawSHP`. This matches the prior `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md` finding that `ExtraLight=` belongs to building draw-depth/Z-style handoff and not RGB ambience.

Active in YR: Yes. `BuildingClass_DrawBody` is the normal visible building body render path.

Renderer classification: building bodies need a separate depth/Z/light-scalar handoff using the cell `+0x10A` equivalent. Do not implement `ExtraLight=` by altering the map RGB lighting grid.

### 8. Queued/fogged/cached draw path repeats the same scalar-field pattern

Evidence: `FUN_004D1890` uses cell lighting in queued/cached draw cases:

- Building-like case reads `(short)(cell+0x10A)` and combines it with a signed type field before shape draw.
- Anim-like queued case can read `(short)(cell+0x10A)`.
- Smudge-like case reads `(short)(cell+0x10C)`.
- A later object/tile-like case lazy-inits `cell+0x34` and passes `(short)(cell+0x10C)` to `CC_Draw_Shape`.

Assembly range `0x004D1D80..0x004D23D0` was disassembled as the checked queued/cached consumer region.

Active in YR: Conditional. The function is live code, but some cases are cached/fogged/queued render behavior; standard YR does not enable TS-style fog by default. Treat ordinary visible rendering as covered by the direct consumers above.

Renderer classification: if Rust later implements cached/fogged object rendering, it should reuse the same cell-light-profile plus scalar-field model rather than inventing a separate tint path.

## Consolidated Consumer Map

| Consumer | `+0x34` use | `+0x10A` use | `+0x10C` use | `+0x10E` use | Active in YR |
|---|---|---:|---:|---:|---|
| `CellOverlay_TileDraw @ 0x00480350` | passed to `TMP_TileBlitter`; lazy init | no | yes | no | Yes |
| `CellClass__DrawOverlay_Body @ 0x0047F6A0` | lazy init only in audited branches | branch-specific | common | branch-specific | Yes |
| `TerrainClass__Draw_It @ 0x0071C250` | lazy init only | type-flag branch | common | no | Yes |
| `TechnoClass_DrawSHP @ 0x00705E00` | lazy init only in audited branches | not direct here | yes | no | Yes |
| `AnimClass__DrawIt @ 0x00423200` | selected as active convert in cell-palette branch | branch-specific | branch-specific | no | Conditional by anim flags |
| `BuildingClass_DrawBody @ 0x0043D290` | not the RGB ambience path in audited handoff | building Z/depth/light handoff | no direct ordinary body use found | no | Yes |
| `FUN_004D1890` queued/cached draw | lazy init in some cases | branch-specific | branch-specific | no | Conditional |

## Implementation Handoff

1. Verified behavior -> terrain/TMP draw consumes both a `LightConvert` profile pointer and `+0x10C` scalar -> Rust delta -> introduce a render-facing `CellLightProfile`/indexed profile plus scalar bundle instead of only `[f32;3]` tint -> affected surface -> `src/map/lighting.rs`, `src/app_init.rs`, `src/render/tile_atlas.rs`, terrain instance generation -> acceptance scenario -> one lit lamp changes the tile's palette profile and scalar while many cells can share one RGB profile -> proposed test name -> `map_lighting_tile_uses_profile_and_scalar_brightness` -> risk -> high screenshot visibility.

2. Verified behavior -> sprites, overlays, terrain objects, and animations consume branch-selectable scalar fields, especially `+0x10A/+0x10C/+0x10E` -> Rust delta -> store a cell lighting bundle with at least `top`, `common`, `bottom`, and RGB profile key, not one tint -> affected surface -> sprite/overlay/terrain-object/anim instance builders and `src/render/batch.rs` tint inputs -> acceptance scenario -> a test map with one lamp shows matching brightness on terrain tile, wall/ore overlay, tree/lamp object, infantry/building SHP, and anim branch using the same affected cell -> proposed test name -> `map_lighting_scalar_bundle_feeds_tile_overlay_techno_anim` -> risk -> high because object sprites currently can diverge from terrain.

3. Verified behavior -> building body uses `cell+0x10A + signed ExtraLight` in the depth/Z-style draw handoff -> Rust delta -> remove `ExtraLight` from RGB grid and route it to building draw ordering/depth adjustment -> affected surface -> `src/map/lighting.rs::apply_extra_light`, building render instance depth code, art data consumers -> acceptance scenario -> `GAARTY ExtraLight=350` changes draw ordering/depth but does not brighten its occupied cell or nearby sprites -> proposed test name -> `building_extralight_adjusts_depth_not_rgb_lighting` -> risk -> medium/high for deployed building art sorting.

4. Verified behavior -> `AnimClass__DrawIt` sometimes uses global/fixed convert or fixed `1000` instead of cell lighting -> Rust delta -> animation render metadata needs flags for cell-lighted versus fixed/global lighting -> affected surface -> anim SHP atlas/instance paths and art parser flags -> acceptance scenario -> a cell-lit anim type changes with lamps while a fixed/global branch remains at neutral brightness -> proposed test name -> `anim_lighting_respects_cell_light_flags` -> risk -> medium; depends on which stock anim flags are implemented.

## Negative Facts / Do Not Do

- Do not model map lighting as only per-cell `[f32;3]`; `Cell+0x34` and scalar fields are consumed separately. Active in YR: Yes. Evidence: `0x00480350` passes `+0x34` and `+0x10C` to `TMP_TileBlitter`.
- Do not use `+0x10C` for every branch. Active in YR: Yes. Evidence: `0x0047F6A0` and `0x0071C250` use `+0x10A`/`+0x10E` branches.
- Do not brighten/darken RGB map cells from `ExtraLight=`. Active in YR: Yes. Evidence: `0x0043D290` uses signed `BuildingType+0x1548` with `cell+0x10A` in the building body draw handoff; prior report found no LightConvert/RGB ambience consumer.
- Do not assume `cell+0x34` is always preinitialized. Active in YR: Yes. Evidence: audited draw paths defensively call `0x00483E30` when it is null.
- Do not apply cell lighting indiscriminately to every animation. Active in YR: Conditional by anim flags. Evidence: `0x00423200` has cell-light, global-convert, and fixed-brightness branches.

## Remaining Uncertainty

- Exact low-level semantics of `CC_Draw_Shape` arguments remain out of scope. This report classifies which cell fields are supplied to draw calls, not the final pixel loop math.
- Exact `LightConvertClass` palette table generation and blitter table meanings remain covered only by prior cache reports, not this consumer slice.
- Human-readable names for `Cell+0x104` and `Cell+0x108` remain unresolved; no direct audited draw consumer was found here.
- Some `FUN_004D1890` cases are conditional cached/fogged behavior. Standard YR path activity is conditional for those cases because TS-style fog is disabled by default.

## Stale-Doc / Rust Comment Replacement Wording

- `src/map/lighting.rs` comment "per-cell RGB tint" should be narrowed to: "map lighting produces a render-facing cell light bundle: normalized RGB profile key plus scalar draw fields; the current Rust `[f32;3]` tint is an approximation."
- `src/map/lighting.rs::apply_extra_light` comment should be replaced with: "`ExtraLight=` is not RGB ambience; binary uses it in building draw depth/Z-style handoff with cell `+0x10A`."
- Any doc wording that says terrain lighting only tints terrain should be replaced with: "terrain tiles, overlays, terrain objects, Techno SHPs, and some anim branches consume cell lighting scalars; TMP tiles additionally receive the `LightConvertClass*` profile."

## Source Material

- Ghidra decompile: `0x00483E30` (`CellClass` light bundle setter/cache maintainer).
- Ghidra decompile: `0x00480350` (`CellOverlay_TileDraw`).
- Ghidra decompile: `0x0047F6A0` (`CellClass__DrawOverlay_Body`).
- Ghidra decompile: `0x0071C250` (`TerrainClass__Draw_It`).
- Ghidra decompile: `0x00705E00` (`TechnoClass_DrawSHP`).
- Ghidra decompile: `0x00423200` (`AnimClass__DrawIt`).
- Ghidra decompile: `0x004D1890` (queued/cached draw dispatcher).
- Ghidra decompile: `0x0043D290` (`BuildingClass_DrawBody`).
- Read-only disassembly spot-check ranges: `0x00480370..0x00480480`, `0x0047F720..0x0047FBD0`, `0x0071C300..0x0071C3F0`, `0x00705EF0..0x00706140`, `0x00423280..0x00423380`, `0x004D1D80..0x004D23D0`.
- Prior corroborating reports: `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`, `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`.
