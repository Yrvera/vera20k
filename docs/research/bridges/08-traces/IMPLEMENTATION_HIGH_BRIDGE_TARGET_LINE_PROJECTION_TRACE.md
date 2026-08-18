# Implementation Trace: High Bridge Target-Line Projection

Scenario: a selected ground unit receives a move order to high bridge deck cell `(10,10)` with ground level `0` and deck level `4`.

Scope: click/command target cell, movement target-line endpoint projection, and current Rust endpoint pixels versus gamemd's active selected-unit action-line bridge-Z projection. This trace does not cover line raster style, endpoint boxes, option UI, movement path execution, bridge boundary pixels, or low-bridge behavior except as adjacent findings.

## Verdict Summary

PASS: 4 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Pipeline

Player clicks bridge deck center -> Rust resolves `(10,10)` -> Rust records `Command::Move` target-line cell destination -> target-line render resolves effective cell Z -> Rust projects endpoint pixels -> gamemd selected-unit `DrawActionLines` resolves `NavCom` endpoint, applies bridge Z, and projects with `CoordsToClient2`.

## Concrete Coordinate Model

Rust terrain projection:

```text
iso_to_screen(rx, ry, z):
  sx = (rx - ry) * 30 - 30
  sy = (rx + ry) * 15 + 15 - z * 15

project_cell_destination:
  endpoint = iso_to_screen(rx, ry, z) + (30, 15)
```

For `(10,10)`, ground `z=0`, bridge deck `z=4`:

```text
ground endpoint = (0, 330)
deck endpoint   = (0, 270)
delta           = -60 px Y
```

gamemd selected-unit movement line path:

```text
TechnoClass::DrawActionLines movement branch:
  endpoint = NavQueue.last else NavCom
  endpoint coords = target->vtable+0x48
  if endpoint cell in bounds and Cell.Flags & 0x100:
      endpoint.Z = CellClass::GetGroundHeight(endpoint) + DAT_008B3DF4
  ActionLines__DrawLine(...)

ActionLines__DrawLine:
  calls TacticalClass__CoordsToClient2 for both 3D endpoints

CoordsToClient2:
  screen_y = signed_trunc(iso_y_raw / 256) - AdjustForZ(Z) - tactical_scroll_y
```

The active standard-YR chain is confirmed by `TechnoClass__DrawActionLines @ 0x004DC060`, `ActionLines__DrawLine @ 0x007049C0`, and `TacticalClass__CoordsToClient2 @ 0x006D2140`. The selected-unit path is conditional on the normal `UnitActionLines` option/default gate, selected human-owned mobile techno, live action-line timer, and a non-null `NavCom`/queued endpoint.

## Stage Results

### Stage 1 - Click resolves command target cell

Rust output: for the bridge deck center `(0,270)`, `screen_to_iso_with_height_and_bridges()` resolves `(10,10)` after adding bridge height `4 * 15 = 60` to the inverse-projection Y input. Existing trace evidence: `COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`.

gamemd output: prior binary trace of the active click/action path shows the standard-YR bridge-cell path checks `CellClass+0x140 & 0x100` and applies bridge height correction for high bridge cells. For the center-click case, the target cell remains `(10,10)`.

Verdict: PASS. Both sides produce command target cell `(10,10)` for the concrete center-click scenario.

### Stage 2 - Rust target-line cell Z selection

Rust output: `build_target_line_instances()` handles `LineDest::Cell { rx, ry }` by calling `project_cell_destination(rx, ry, height_map, None, Some(sim))` at `src/app_target_lines.rs:170-172`. `project_cell_destination()` calls `bridge_deck_height_for_cell()` before falling back to `height_map` at `src/app_target_lines.rs:199-201`. For an intact non-low bridge deck cell with `bridge_deck_level = 4`, `bridge_deck_height_for_cell()` returns `Some(4)` at `src/app_target_lines.rs:217-222`.

gamemd output: `TechnoClass__DrawActionLines @ 0x004DC060` movement endpoint bridge adjustment replaces endpoint Z with `CellClass__GetGroundHeight + DAT_008B3DF4` when the endpoint cell has `Cell+0x140 & 0x100`. The selected-unit movement branch is active in standard YR when `ArchiveTarget == 0` and `NavCom != 0`.

Verdict: PASS for the requested bridge-deck semantic. Both sides choose the bridge/deck height path rather than the ground-only path.

### Stage 3 - Rust endpoint pixels

Rust output:

```text
rx=10, ry=10, z=4
sx = (10 - 10) * 30 - 30 = -30
sy = (10 + 10) * 15 + 15 - 4 * 15 = 255
endpoint = (-30 + 30, 255 + 15) = (0,270)
```

The old ground-only output would have been `(0,330)`, so the implemented code lifts the visible move feedback endpoint by exactly `60` px.

gamemd output: not needed for Rust-only computation.

Verdict: PASS. Current Rust endpoint for this concrete input is exactly `(0,270)`.

### Stage 4 - gamemd bridge Z projection delta

gamemd output: active `CoordsToClient2 @ 0x006D2140` subtracts `AdjustForZ(Z)` from projected screen Y. `Tactical__AdjustForZ @ 0x006D20E0` uses multiplier `15/256` at default standard projection. Existing bridge/elevation docs verify a high bridge deck is `+4` height levels above ground, where one height level corresponds to `15` screen pixels in the normal isometric projection. Therefore the bridge deck target projects `60` px above the same ground cell target.

Rust output: Stage 3 also lifts the endpoint by exactly `4 * terrain::HEIGHT_STEP = 4 * 15 = 60` px.

Verdict: PASS for the bridge-Z projection delta. Both sides use a `-60` px screen-Y lift for this requested `ground=0`, `deck=4` bridge endpoint.

### Stage 5 - Absolute gamemd endpoint pixel

Rust output: `(0,270)` in the repo's unscrolled tactical coordinate model.

gamemd output: the active binary formula includes endpoint CoordStruct X/Y convention, `tactical+0xB0`, `tactical+0xB4`, and the later action-line viewport-Y offset `0x00886FA4`. I did not run a live gamemd frame with matched tactical scroll/viewport globals for this exact map cell.

Verdict: UNCHECKED. The bridge-Z delta matches exactly, but absolute gamemd client pixel equality was not computed for a live viewport state.

### Stage 6 - Final line raster pixels

Rust output: `emit_colored_line()` emits rounded float-DDA `1x1` `SpriteInstance` pixels.

gamemd output: `ActionLines__DrawLine @ 0x007049C0` draws clipped `3x3` endpoint boxes and one clipped solid line on `DAT_0088731C`.

Verdict: UNCHECKED for this slot. The requested implementation fix concerns endpoint projection; full raster style was already documented elsewhere and was not re-traced here.

## Failures

None for the implemented high-bridge endpoint projection fix in this concrete scenario.

## Not Implemented

None for this concrete endpoint projection scenario.

## Adjacent Findings

- Rust still does not reproduce gamemd selected-unit action-line pixel style: endpoint boxes, surface clipping, palette/convert color, and exact draw order remain broader target-line parity work.
- Absolute pixel equality against gamemd needs a live matched viewport/camera capture or a fully specified tactical scroll state. This trace verifies the corrected bridge-Z endpoint selection and the `60` px lift, not every viewport-translated final pixel.

## Sources

- Rust: `src/app_target_lines.rs:170-222`, `src/map/terrain.rs:196-203`.
- Ghidra read-only decompile: `TechnoClass__DrawActionLines @ 0x004DC060`.
- Ghidra read-only disassembly/decompile: `TacticalClass__CoordsToClient2 @ 0x006D2140`.
- Existing docs: `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, `ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE_GHIDRA_REPORT.md`, `TACTICAL_ADJUSTFORZ_MULTIPLIER_GHIDRA_REPORT.md`, `GETEFFECTIVEHEIGHT_PLUS4_UNIT_GHIDRA_REPORT.md`, `COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`.

## Status

COMPLETE
