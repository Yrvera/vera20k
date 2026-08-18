# Building Selection Brackets Research Index

This folder collects the verified `gamemd.exe` research for selected-building
wireframe brackets, related building health pips, and the small helper paths that
control bracket geometry, color, rasterization, and draw order.

## Current Source Of Truth

Read these first:

1. `SELECTION_BRACKETS_GHIDRA_REPORT.md` - overall 12-edge topology, dimension source, color rule, and phase order.
2. `TECHNO_DRAWBEHIND_BUILDING_BRACKET_EDGES_GHIDRA_REPORT.md` - the five back/left bracket edges drawn before the building body.
3. `TECHNO_DRAWEXTRAS_BUILDING_BRACKET_BLOCK_GHIDRA_REPORT.md` - the front/right bracket block, including the four two-ended edges and three direct single-stub edges.
4. `TECHNO_DRAWHEALTHBAR_BUILDING_BRANCH_GHIDRA_REPORT.md` - selected building health-pip branch after front brackets.
5. `BUILDING_BRACKET_MULTI_OBJECT_INTERLEAVING_GHIDRA_REPORT.md` - tactical object-loop order, including the later second `DrawExtras` pass.

Supporting helper reports:

- `BUILDINGTYPE_DIMENSION2_BRACKET_EXTENTS_GHIDRA_REPORT.md` - `Foundation` and `Height` to bracket extents.
- `FOUNDATION_PARSER_TABLE_BRACKET_EXTENTS_GHIDRA_REPORT.md` - fixed foundation-name table; no free-form `WxH` parser.
- `DRAWBRACKETCORNER_DRAWLINE3D_STUB_RASTER_GHIDRA_REPORT.md` - quarter-point stubs and `DrawLine3D`/surface-line behavior.
- `SURFACE_DRAW_LINE_BRACKET_RASTER_GHIDRA_REPORT.md` - exact `Surface::Draw_Line` clipping, integer stepping, and end-exclusive raster contract.
- `OBJECT_GETHEIGHT_BRACKET_DIM_COLOR_REACHABILITY_GHIDRA_REPORT.md` and `BUILDING_BRACKET_GETHEIGHT_DIM_COLOR_REACHABILITY_FOLLOWUP_GHIDRA_REPORT.md` - `GetHeight() < -4` dim-color branch and standard-building reachability.
- `BUILDING_GET_PIXEL_SELECTION_BRACKET_DELTA_GHIDRA_REPORT.md` - negative finding for stale `PixelSelectionBracketDelta` / `SelectBracketOffsetX/Y` claims.
- `TECHNO_DRAWEXTRAS_VTABLE_448_BUILDING_HOOK_OVERRIDES_GHIDRA_REPORT.md` - stock `BuildingClass` `vtable+0x448` resolves to an empty no-op.
- `BUILDING_HEALTH_PIP_VISUAL_ANCHOR_CASES_GHIDRA_REPORT.md` - concrete health-pip anchors for `GACNST`, `[TESLA] Image=NATSLA`, and `GAREFN`.
- `BUILDING_HEALTH_PIP_FINAL_FRAMEBUFFER_ANCHOR_GHIDRA_REPORT.md` - `PIPS.SHP` canvas-centering and final frame-rect top-left (`draw_point + (-5,-3)`).
- `TECHNO_HOVER_HEALTH_FLAG_0431_BUILDING_PIPS_GHIDRA_REPORT.md` - hover-health flag source and selected/hover-only building pip caller gate.
- `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md` - combined draw-order context for brackets and pips.
- `BUILDING_FIRST_PASS_DISPLAY_0043DA80_GHIDRA_REPORT.md` - first-pass building display helper contents; not the ordinary SHP body/bib/z-shape path.
- `BUILDING_BRACKET_ABUFFER_ZTEST_DEPTH_SEMANTICS_GHIDRA_REPORT.md` - selected bracket lines Z-test and A-buffer modulate/suppress, but do not Z-write.
- `BUILDING_BRACKET_DEPTH_DOMINANT_RASTER_REACHABILITY_GHIDRA_REPORT.md` - depth-dominant surface raster is real but unreachable from stock selected-building bracket segments.

## Cross-Cutting Docs Left At Root

These are intentionally not moved because they cover larger systems:

- `../BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`
- `../BUILDINGCLASS_VTABLE_COMPLETE.md`
- `../TECHNOCLASS_VTABLE_COMPLETE.md`
- `../ADDRESS_MAP.md`
- `../SELECTION_SYSTEM_GHIDRA_REPORT.md`
- `../TACTICAL_DRAWUNITACTIONVISUALS_SENSOR_RINGS_GHIDRA_REPORT.md`

## Retired Claims

- `0x004263C0` is not a building selection-bracket offset helper. It returns the global "No name" string pointer.
- No verified retail/local INI key named `SelectBracketOffsetX` or `SelectBracketOffsetY` was found for building line-bracket geometry.
- `vtable+0x1C8` in the bracket color path is `ObjectClass::GetHeight`, not `PixelSelectionBracketDelta`.
- `PixelSelectionBracketDelta` affects non-building pip/PIPBRD Y placement, not building line-bracket geometry.
- `[Map] LocalSize` is not a verified substitute for building bracket dimensions; brackets use type dimensions from `Foundation`/`Height`.

## Implementation Notes

Current Rust status after the bracket re-enable pass:

- Building bracket generation is active in `src/app_selection_brackets.rs`.
- Back, first-front, and final-front bracket instances are split in `src/app_render/build_instances.rs`.
- Back edges draw before object bodies in `src/app_render/draw_passes.rs`.
- The first front-edge submission draws before object bodies, and the final front-edge submission draws after bodies in `src/app_render/draw_passes.rs`. This mirrors the verified first-pass `DrawExtras` plus later second `DrawExtras` structure at a phase level, though not full per-object interleaving.
- Foundation dimensions use the fixed table in `src/rules/foundation.rs`.
- Psychic Sensor and Gap Generator radius rings are separate overlays, not bracket geometry.
- Building health pips use `PIPS.SHP` frame-rect placement, including the verified canvas/frame adjustment of `(-5,-3)` from the caller draw point.
- Building health pips are caller-gated by selected state or the current hover-health flag. Damaged-only non-selected/non-hover buildings should not draw building health pips.

Known remaining parity risks:

- Rust line rasterization now uses integer start-inclusive/end-exclusive stepping, but it still does not model A-buffer modulation or the primary-surface Z-test/no-Z-write behavior. The depth-dominant surface-line path remains real, but stock selected-building bracket segments do not reach it.
- The rare `GetHeight() < -4` dim-color branch is not implemented for normal overlay rendering. Follow-up evidence says standard selected buildings do not reach palette `0x0C` in normal YR states.
- Full per-object `DrawBehind -> DrawExtras -> +0x104(flag=1)` interleaving is still approximated as phase buffers, so unusual multi-building overlap can still differ.
- Visual parity has not yet been screenshot-compared against gamemd for representative buildings.

## Recommended Next Trace

Run a focused visual trace for selected `GACNST`, `[TESLA] Image=NATSLA`, and `GAREFN`:

- `GACNST`: 4x4 foundation, `Height=4`, baseline bracket geometry.
- `[TESLA] Image=NATSLA`: 1x1 foundation, `Height=5`, catches odd-foundation pip-anchor rounding and roof/Z projection drift.
- `GAREFN` or `YAREFN`: refinery footprint and health-pip placement.

Check back-edge occlusion, front-corner placement, health-pip placement, and multi-building overlap.
