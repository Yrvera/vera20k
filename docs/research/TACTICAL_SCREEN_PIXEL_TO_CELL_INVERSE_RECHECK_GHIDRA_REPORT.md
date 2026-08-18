# Tactical Screen Pixel To Cell Inverse Recheck - Ghidra Research Report

**Address(es):** `0x006D6590` primary inverse; callers at `0x004A91B0`, `0x00653760`, `0x00692300`, `0x00656EC0`, `0x006DA380`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** screen/client pixel to map-cell inverse at `0x006D6590`, exact height scan loop, bridge neighbor rule, viewport/radar offset handling visible at direct callers, off-map/null-cell behavior, and Rust-facing implications for cursor clicks, placement, superweapon targeting, and bridge endpoint picks.
**Non-Scope:** object selection ordering after the cell is chosen, minimap rendering, pathfinding after command creation, low-bridge tube internals, building foundation anchor parity.
**Confidence:** High for `0x006D6590` and immediate callers/helpers decompiled here; Medium for UI naming of caller wrappers whose Ghidra labels are still generic.
**Active in YR:** Yes. The function is called by live tactical picking (`0x006DA380`), radar update (`0x00656EC0`), cursor/cell wrapper paths (`0x00653760`, `0x00692300`), and display/mouse update path (`0x004A91B0`).

## Required Investigation Notes

- Target question: What exact behavior does gamemd use to convert a tactical screen/client pixel into a map cell, especially around height, bridges, viewport offsets, and off-map cells?
- Non-goals: Do not implement Rust, do not investigate general coordinate system architecture, object selection tie-breaks, pathfinding, or building foundation anchoring beyond this inverse's consumers.
- Evidence needed to mark COMPLETE: decompile `0x006D6590`; confirm assembly for loop bound and branch constants; trace direct callers; decompile helper/cell accessor; compare current Rust surfaces.
- Stop conditions: stop at this function's direct callers/helpers once the inverse algorithm and Rust handoff are clear; defer deeper UI event provenance or object-selection semantics.

## 1. Overview

`0x006D6590` converts a client/screen pixel into a packed cell coordinate. It subtracts the global radar/tactical viewport origin, applies the tactical camera offsets and matrix inverse, then scans vertical candidate pixels until the candidate cell's terrain/bridge-adjusted projected height is at or above the input pixel.

The Rust approximation is materially different: current `src/map/terrain.rs::screen_to_iso_with_height_and_bridges` does three iterative solves and a 7x7 closest-bridge search. The binary uses a 180-pixel vertical scan, `MapClass__Get_CellClass` sentinel fallback, a structural-bridge bit branch, cardinal neighbors only, and a hard 15-pixel bridge-edge threshold.

## 2. Key Offsets, Flags, and Globals

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Tactical `+0xB0/+0xB4` | camera/client scroll offsets added before inverse transform | `0x006D6599..0x006D65B9`, `0x006D66AA..0x006D66C0` | Yes |
| `g_RadarViewportOffsetX/Y` | global viewport origin subtracted inside `0x006D6590` | decompile `0x006D6590`; assembly reads `0x00886FA0/0x00886FA4` at `0x006D6599/0x006D65A5` | Yes |
| `CellClass+0x24/+0x26` | cell X/Y words used to project bridge-cell reference point | `0x006D6895..0x006D68CE` | Yes |
| `CellClass+0x11B` | signed terrain height level; each level is 15 screen pixels | `0x006D6751..0x006D6768` | Yes |
| `CellClass+0x140 bit 0x100` | structural high-bridge/body flag; enables bridge branch and extra 60-pixel adjustment | `0x006D6760..0x006D6771`, `0x006D6948..0x006D6960`, `0x006D69A3..0x006D69BC` | Yes |
| `CellClass+0x140 bit 0x800` | bridge orientation/side selector for which cardinal neighbors are considered | `0x006D6793..0x006D688E` | Yes on bridge cells |
| `g_DirectionOffsets` | cardinal neighbor helper table; directions `0,2,4,6` are used here | `Pathfinding_update_continued` `0x00481810`; xrefs to `0x0089F688` | Yes |
| `MapClass__Get_CellClass` sentinel | out-of-range/null cell lookup returns `DAT_00ABDC50`, not null, and records requested cell at `DAT_00ABDC74` | `0x005657A0` | Yes |

## 3. Core Logic

### 3.1 Input coordinate space

`0x006D6590` takes `param_3 = [x,y]` and subtracts `g_RadarViewportOffsetX/Y` internally. It also adds tactical camera offsets `this+0xB0/+0xB4` before calling `Matrix3x4_TransformPoint`.

Evidence:

- `0x006D6599..0x006D65B9`: reads viewport offsets and camera fields.
- `0x006D66AA..0x006D66C0`: per-scan candidate uses `camera_x + (input_x - offset_x)` and `camera_y + scan_y`.
- `0x00692379..0x0069238E` and `0x006DA4CB..0x006DA4E1`: callers add `g_RadarViewportOffsetX/Y` to local tactical points before calling the inverse.
- `0x00656F34..0x00656F4E`: radar update calls with viewport center `offset + width/2`, `offset + height/2`.

Implication: the inverse itself owns viewport-origin subtraction. Rust should not treat full-window pixels, tactical-local pixels, and minimap-local pixels as interchangeable. A wrapper may add offsets before the call, but `0x006D6590` still subtracts them.

### 3.2 Fallback initial cell

Before the scan loop, the function does a one-shot inverse at the input pixel and stores that packed cell in `local_54`. If the scan exceeds the loop cap, it returns this fallback packed cell.

Evidence:

- `0x006D65F1..0x006D6663`: first matrix transform, two `Math__ftol` calls, signed divide-by-256 cell packing.
- `0x006D69D8..0x006D69F9`: if the scan counter reaches 180, writes fallback `local_54` to output.

### 3.3 Height scan loop

The loop is a vertical pixel scan, not a small fixed-point convergence loop.

Algorithm from decompile and assembly:

1. Set `scan_y = (input_y - viewport_offset_y) + 0xB4`.
2. For each iteration, inverse-transform `(input_x - viewport_offset_x + camera_x, scan_y + camera_y)` to a candidate cell.
3. Fetch `CellClass` for that candidate.
4. Subtract `cell_height * 15` from `scan_y`.
5. Apply bridge adjustment if the bridge branch says to.
6. If adjusted `scan_y <= input_y - viewport_offset_y`, return the candidate cell.
7. Otherwise set `scan_y = previous_scan_y - 1`, increment counter, and loop while counter `< 0xB4`.
8. If counter reaches `0xB4` (180), return the fallback initial cell.

Evidence:

- Start at `+0xB4`: `0x006D668A..0x006D6698`.
- Height level times 15: `0x006D6751..0x006D6768` (`height*3*5`, subtract from scan).
- Return condition: `0x006D69C4..0x006D6A77`.
- Loop bound: `0x006D69D6..0x006D69E5` decrements scan, increments counter, `CMP EAX,0xB4; JL 0x006D66A6`.

Tiny details:

- The maximum is 180 attempted scan iterations. The prior "`0xB3` cap" wording should be read as "fallback when counter is greater than 179"; the assembly compare is against `0xB4`.
- Scan decrements by exactly one screen pixel from the previous unadjusted scan Y after a failed test.
- Signed cell conversion uses the `value + (sign & 0xFF) >> 8` pattern before packing shorts.

### 3.4 Bridge branch and exact neighbor rule

The bridge branch only runs when current cell `flags & 0x100` is set. It checks cardinal neighbors through `Pathfinding_update_continued` with directions `2`, `4`, and conditionally `0`/`6`. The helper uses `g_DirectionOffsets[(dir & 7)]` and returns the neighbor `CellClass`.

Verified neighbor order and predicates:

1. Always fetch direction `2` first, then direction `4`.
2. If `flags & 0x800` is set, fetch direction `0`; mark a "dir0 open edge" only if that neighbor is not `flags & 0x100`.
3. If `flags & 0x800` is clear, fetch direction `6`; mark a "dir6 open edge" only if that neighbor is not `flags & 0x100`.
4. If `flags & 0x800` is set and direction `4` is not bridge, mark a direct `+Y` return candidate.
5. If `flags & 0x800` is clear and direction `2` is not bridge, mark a direct `+X` return candidate.
6. If `flags & 0x800` is set, direction `2` is not bridge, and `abs(current_height - dir2_height) <= 1`, mark a height-compatible `+X` return candidate.
7. If `flags & 0x800` is clear, direction `4` is not bridge, and `abs(current_height - dir4_height) <= 1`, mark a height-compatible `+Y` return candidate.

Evidence:

- Direction helper: `Pathfinding_update_continued` `0x00481810`, with `param_2 < 8` and `g_DirectionOffsets + (dir&7)*4`.
- Fetch dir2/dir4: decompile `0x006D6590`; call sites before `0x006D6793`.
- Dir0 edge predicate: `0x006D6793..0x006D67B9`.
- Dir6 edge predicate: `0x006D67C1..0x006D67E4`.
- Direct dir4/dir2 predicates: `0x006D67E9..0x006D6827`.
- Height-compatible dir2 predicate: `0x006D6827..0x006D685F`.
- Height-compatible dir4 predicate: `0x006D685F..0x006D6895`.

Return/adjust behavior:

- After projecting the current bridge cell reference point, if projected bridge Y is at or above the input Y and a direct/height-compatible dir4 candidate exists, return `(cell_x, cell_y + 1)`.
- Else if projected bridge Y is at or above the input Y and a direct/height-compatible dir2 candidate exists, return `(cell_x + 1, cell_y)`.
- Else, dir0/dir6 open-edge flags do not directly return a neighbor. They gate whether the extra bridge-height subtraction (`0x3C`, 60 pixels = 4 height levels * 15) is applied.
- The edge threshold is strict `> 0xF`, not `>= 0xF`.
- For the dir0 edge, extra bridge adjustment is applied when `(input_y_delta - input_x_delta/2) > 15`.
- For the dir6 edge, extra bridge adjustment is applied when `(input_y_delta + input_x_delta/2) > 15`.
- If neither dir0 nor dir6 open-edge flag is active, a bridge cell applies the 60-pixel adjustment unconditionally.

Evidence:

- `+Y` return path: `0x006D6905..0x006D6A36`.
- `+X` return path: `0x006D691D..0x006D6A02`.
- Dir0/dir6 threshold: `0x006D6935..0x006D69C0`.
- Strict threshold compare: `0x006D6986..0x006D698B` and `0x006D699C..0x006D69A1`.
- Unconditional bridge adjustment when no dir0/dir6 edge: `0x006D6948..0x006D6964`.

This refines the older docs: gamemd is not just "check four cardinal neighbors and shift by 15 px." It checks a fixed subset/order controlled by `0x800`, returns only +X/+Y neighbor cells directly, and uses dir0/dir6 only to decide bridge-height application near an open edge.

### 3.5 Off-map/null behavior

`0x006D6590` does not null-check the returned `CellClass`. `MapClass__Get_CellClass` handles invalid packed cells.

`MapClass__Get_CellClass` computes `index = y * 0x200 + x`. If `index < 0`, `index > 0x3FFFF`, or the cell pointer array entry is null, it writes the requested packed cell to `DAT_00ABDC74` and returns sentinel cell `DAT_00ABDC50`.

Evidence: `0x005657A0` decompile.

Input-negative handling is caller-specific, not in `0x006D6590`: `0x00692300` returns 0 immediately if either input screen coordinate is negative. Other direct callers do not show that same guard in the decompiled slice.

## 4. INI Keys

No INI key directly controls `0x006D6590`. Search of `rules.ini`, `rulesmd.ini`, `art.ini`, and `artmd.ini` found bridge/cursor keys, but the inverse constants here are binary math constants and map/cell data (`CellClass` flags and height).

Relevant but not inverse-specific defaults:

| Key | YR/default role | Evidence | Active in YR |
|---|---|---|---|
| `DestroyableBridges=yes` | bridge state can change, which can clear or alter bridge cell flags consumed by the inverse | `ini/rulesmd.ini:804` | Yes |
| `BridgeStrength=1500` | bridge damage durability; not read by inverse | `ini/rulesmd.ini:816` | Yes |

## 5. Integration Points

| Caller/helper | Finding | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00653760` | wrapper converts screen point to cell through `0x006D6590`; map editor branch still calls same inverse | `0x006537C8`, `0x006537E6` | Yes |
| `FUN_00692300` | rejects negative input; adds viewport offsets before inverse; then computes world coords and shroud/object info | `0x00692300..0x006925E6` | Yes |
| `Tactical__PickObjectAtScreenPoint` | object-pick fallback adds viewport offsets, calls inverse, gets `CellClass+0xE4` object list | `0x006DA4CB..0x006DA501` | Yes |
| `RadarClass__Update` | samples inverse at viewport center to update radar view rectangle | `0x00656F34..0x00656F5E` | Yes |
| `FUN_004A91B0` | display/mouse path uses inverse when not in radar-viewport branch; radar branch uses `FUN_00653760` | `0x004A91B0..0x004A94xx` | Yes |
| Raw call sites `0x00537740`, `0x00537800`, `0x005378C0`, `0x00537980` | small call thunks store resulting cells into globals/fields `+0x348E/+0x3492/+0x3496/+0x349A` | assembly context at each call | Touched, likely UI/corner state |
| Raw call sites `0x005FDC21`, `0x005FDC45` | validate/convert inverse result through `Cell_in_bounds_check` style path | assembly context and `0x00568300` | Touched |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Evidence | Delta |
|---|---|---|---|
| `src/map/terrain.rs::screen_to_iso_with_height_and_bridges` | three height iterations, then 7x7 bridge search; closest candidate wins with `dist < 0.7` | `src/map/terrain.rs:275..330` | mismatch |
| `src/app_sim_tick.rs::screen_point_to_world` | `screen / zoom + camera`; no explicit tactical viewport origin term | `src/app_sim_tick.rs:1164..1169` | unchecked/mismatch risk |
| `src/app_sim_tick.rs::screen_point_to_world_cell` | central app cursor-cell wrapper with bridge map | `src/app_sim_tick.rs:1198..1209` | affected |
| building placement preview | uses `screen_point_to_world_cell` | `src/app_sim_tick.rs:841..878` | affected |
| ready building placement click | commits preview cell if present; fallback recomputes inverse | `src/app_commands.rs:211..227` | preview-click stabilization present |
| superweapon target | guards minimap/sidebar before inverse | `src/app_commands.rs:293..306` | guard present; viewport contract still risk |

Codegraph was incomplete for cross-file Rust callers of these pub(crate) functions, so the file scan above is the reliable current Rust surface evidence.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x006D6590` identity and core loop | verified | decompile `0x006D6590`; assembly `0x006D6590..0x006D6A77` | none for this slice |
| 180-iteration loop bound | verified | `0x006D69D6..0x006D69E5` | none |
| viewport/camera arithmetic inside inverse | verified | `0x006D6599..0x006D65B9`, `0x006D66AA..0x006D66C0` | deeper UI event provenance not claimed |
| bridge neighbor order/predicates | verified | `0x006D6793..0x006D6895`, `0x006D6905..0x006D69C0` | semantic name of bit `0x800` remains inferred as orientation/side selector |
| off-map/null fallback | verified | `MapClass__Get_CellClass` `0x005657A0` | none |
| direct live callers | verified | xrefs/callers and decompiles listed in Section 5 | raw thunk owner names unresolved |
| current Rust inverse implementation | verified | source scan lines in Section 6 | tests not run; no Rust edits made |
| minimap/object selection after cell pick | deferred | non-scope | separate picker/selection investigation |
| exact UI event coordinate provenance for all wrappers | deferred | direct caller evidence only | runtime/debugger trace of mouse event path |

## 8. Open Questions - Final State

- `[RESOLVED] OQ1 - Is 0x006D6590 the active inverse? -> Yes; it is called by tactical picking, radar update, and wrapper paths.` (evidence: `get_function_callers 0x006D6590`; `0x006DA4E1`, `0x00656F4E`, `0x006537C8/0x006537E6`)
- `[RESOLVED] OQ2 - What coordinate space does it accept? -> It subtracts global viewport offsets internally and adds tactical camera fields before matrix inverse; several callers pass offset-added local points.` (evidence: `0x006D6599..0x006D66C0`, `0x00692379..0x0069238E`, `0x006DA4CB..0x006DA4E1`)
- `[RESOLVED] OQ3 - Is the height loop 3 iterations, 179, 180, or other? -> It attempts up to 180 scan iterations, looping while counter < 0xB4 after each failed candidate.` (evidence: `0x006D69D6..0x006D69E5`)
- `[RESOLVED] OQ4 - What is the scan step? -> Failed candidates reset to previous unadjusted scan Y minus exactly 1 pixel.` (evidence: `0x006D69CE..0x006D69DD`)
- `[RESOLVED] OQ5 - What happens on scan exhaustion? -> It returns the one-shot fallback cell computed before the loop.` (evidence: `0x006D65F1..0x006D6663`, `0x006D69EB..0x006D69F9`)
- `[RESOLVED] OQ6 - What bridge flag gates the branch? -> `CellClass+0x140 & 0x100`.` (evidence: `0x006D6760..0x006D6771`)
- `[RESOLVED] OQ7 - Which bridge neighbors are checked and in what order? -> dir2, dir4, then conditional dir0/dir6; predicates are controlled by `0x800`.` (evidence: `0x006D6793..0x006D6895`, `0x00481810`)
- `[RESOLVED] OQ8 - Is bridge correction radial/closest? -> No; it is cardinal, orientation-gated, and threshold-gated.` (evidence: `0x006D6793..0x006D69C0`)
- `[RESOLVED] OQ9 - Is threshold inclusive? -> No; assembly uses strict greater-than after compare to `0xF`.` (evidence: `0x006D6986..0x006D698B`, `0x006D699C..0x006D69A1`)
- `[RESOLVED] OQ10 - Does the inverse return null off-map? -> No; cell lookup returns sentinel `DAT_00ABDC50`.` (evidence: `0x005657A0`)
- `[RESOLVED] OQ11 - Are there inverse-specific INI keys? -> None found; bridge map state can affect flags, but constants are binary/math data.` (evidence: `rg` across `ini/*.ini`)
- `[RESOLVED] OQ12 - Does Rust match the loop? -> No; current Rust uses three iterations.` (evidence: `src/map/terrain.rs:281..297`)
- `[RESOLVED] OQ13 - Does Rust match bridge picking? -> No; current Rust uses 7x7 closest-candidate search.` (evidence: `src/map/terrain.rs:299..330`)
- `[RESOLVED] OQ14 - Which app surfaces consume this? -> cursor cell, placement preview/click, superweapon target, context orders, object hover/pick surfaces.` (evidence: `src/app_sim_tick.rs:1198..1209`, `src/app_commands.rs:211..227`, `src/app_commands.rs:293..306`, `src/app_context_order.rs` scan)
- `[DEFERRED] OQ15 - What user-facing names/owners correspond to raw thunks at 0x00537740..0x00537980 and 0x005FDC21/45?` (category: requires-different-system-context; reason: Ghidra has no function boundary/name for those call-site thunks; next-step-if-pursued: inspect surrounding vtable or constructor tables read-only in a UI-specific investigation)
- `[DEFERRED] OQ16 - What exact mouse event path feeds every wrapper in normal gameplay?` (category: needs-runtime-debugger; reason: direct callers are enough for this inverse contract, but full event provenance requires runtime state/call stack; next-step-if-pursued: break on `0x006D6590` during tactical click, sidebar click, radar click)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Binary uses a vertical scan starting at `input_y - viewport_y + 180`, subtracts `height*15`, scans down one pixel per failed attempt, and falls back after 180 attempts. | `0x006D668A..0x006D6768`, `0x006D69C4..0x006D69F9` | mismatch: Rust uses `for _ in 0..3` iterative correction | `src/map/terrain.rs::screen_to_iso_with_height_and_bridges`; `src/app_sim_tick.rs::world_point_to_cell` | Cell under cursor on steep/elevated terrain must match gamemd's scan, including fallback behavior. | Synthetic height map requiring >3 corrections still resolves to the same cell as the scan; proposed test `screen_to_cell_uses_180_pixel_height_scan_not_three_iterations` | Do not tune iteration count heuristically; the binary algorithm is a pixel scan with a fallback cell. |
| Bridge branch is cardinal/orientation-gated, not radial: dir2/dir4 first, conditional dir0/dir6, strict 15-pixel edge tests, direct returns only to +X/+Y neighbors. | `0x006D6793..0x006D69C0`, helper `0x00481810` | mismatch: Rust searches a 7x7 neighborhood and picks closest bridge candidate | `src/map/terrain.rs::screen_to_iso_with_height_and_bridges`; consumers in `app_context_order.rs`, `app_commands.rs`, `app_sim_tick.rs` | Bridge endpoint/ramp/body boundary pixels must choose the same cell as gamemd, especially ties near open bridge edges. | Click pixel exactly 15 and 16 pixels across both bridge-edge tests; proposed test `screen_to_cell_bridge_edge_threshold_is_strict_and_cardinal` | Do not replace with larger search radius or "closest deck" logic; plausible cells can be wrong at endpoints. |
| `0x006D6590` subtracts global viewport offsets internally; several callers add offsets to tactical-local points before calling. | `0x006D6599..0x006D66C0`, `0x00692379..0x0069238E`, `0x006DA4CB..0x006DA4E1` | unchecked/mismatch risk: Rust `screen_point_to_world` uses full `screen/zoom + camera` with no explicit viewport origin | `src/app_sim_tick.rs::screen_point_to_world`, `screen_point_to_world_cell`; sidebar/minimap guards | Last tactical pixel before sidebar and first sidebar pixel must follow gamemd's viewport contract: tactical pixel picks a cell, sidebar pixel is ignored by tactical targeting. | Move cursor over tactical/sidebar boundary and launch/place; proposed test `screen_to_cell_respects_tactical_viewport_origin_at_sidebar_boundary` | Do not feed sidebar/minimap/full-window pixels into the map inverse without a viewport-space decision. |
| Off-map lookup returns a sentinel `CellClass`, not null; one caller rejects negative screen input before inverse, but inverse itself does not. | `0x005657A0`; `0x00692300` negative guard | mismatch risk: Rust clamps negative float cell coords to zero via `.round().max(0.0) as u16` | `src/app_sim_tick.rs::world_point_to_cell`; command guards | Off-map clicks should not silently become `(0,0)` unless the relevant caller path in gamemd does so. | Negative and far-outside tactical points preserve caller-specific reject/fallback semantics; proposed test `screen_to_cell_offmap_does_not_unconditionally_clamp_to_origin` | Do not use origin clamp as a universal replacement for gamemd's sentinel/fallback behavior. |
| Building placement click uses the preview cell when present in current Rust. | Rust `src/app_commands.rs:211..227` | none observed for preview/click drift | `src/app_commands.rs::place_ready_building_at_cursor`; `src/app_sim_tick.rs::update_building_placement_preview` | Preserve ghost/click agreement while fixing inverse beneath preview. | Cursor jitter at bridge/height boundary commits preview `rx,ry`; proposed test `ready_building_click_commits_preview_cell_after_inverse_boundary_jitter` | Do not recompute placement on click if preview exists. |

### Negative Facts / Do Not Do

- Do not describe the binary loop as a 3-pass convergence solve. It is a one-pixel vertical scan with a 180-attempt cap (`0x006D69D6..0x006D69E5`).
- Do not implement bridge picking as a square/radial closest-candidate search and call it parity. Binary bridge picking is cardinal, `0x800`-gated, and has strict `> 15` edge tests (`0x006D6793..0x006D69C0`).
- Do not assume off-map/null cells crash or return null. `MapClass__Get_CellClass` returns sentinel `DAT_00ABDC50` (`0x005657A0`).
- Do not add special high-bridge cursor actions to cover coordinate errors. Prior verified cursor report shows high bridge `0x100` is height context, not a special action-code source.
- Do not treat full-window, tactical-local, radar-local, sidebar, and minimap pixels as the same input space. `0x006D6590` subtracts viewport offsets internally, while some callers add them first.

### Stale Docs / Follow-up Docs

- `docs/research/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`: replace "max 180 iterations (`0xB3`)" with "maximum 180 failed scan attempts: assembly increments the counter and loops while counter `< 0xB4`; equivalently fallback starts when the counter is greater than `0xB3`."
- `docs/research/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`: replace "checks up to 4 cardinal neighbors on the bridge, and may shift the pick one cell along the bridge direction based on which neighbor is also a bridge cell and 15 pixels distance threshold" with "when the current cell has `CellClass+0x140 & 0x100`, gamemd checks dir2 and dir4 first, conditionally checks dir0 or dir6 based on `flags & 0x800`, directly returns only `(x,y+1)` or `(x+1,y)` for qualifying open dir4/dir2 cases, and uses dir0/dir6 only to gate the extra 60-pixel bridge-height adjustment with strict `> 15` edge tests."
- `docs/research/TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_GHIDRA_REPORT.md`: replace the PARTIAL status with "Superseded by `TACTICAL_SCREEN_PIXEL_TO_CELL_INVERSE_RECHECK_GHIDRA_REPORT.md`, which re-opened `0x006D6590` in live Ghidra and verified loop bound, bridge branch, callers, and off-map sentinel behavior."
- `docs/research/traces/COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`: replace "Gamemd uses directional/cardinal bridge-neighbor tests with a `15` pixel threshold" with "Gamemd's bridge inverse is cardinal and orientation-gated: dir2/dir4 are checked first, dir0 or dir6 is conditional on `flags & 0x800`, and the open-edge tests use strict `> 15` thresholds before applying the extra bridge-height adjustment."

## 10. Remaining Uncertainty

- Exact semantic name of `CellClass+0x140 bit 0x800`: verified as a live bridge branch selector here, inferred as bridge orientation/side selector, but this report does not name the upstream writer.
- Exact gameplay ownership of raw thunk call sites at `0x00537740`, `0x00537800`, `0x005378C0`, `0x00537980`, `0x005FDC21`, `0x005FDC45`.
- Full runtime mouse-event provenance for every wrapper path; direct caller arithmetic is verified, but a debugger trace would be needed to label each UI input source perfectly.

## Sources

- Ghidra decompile: `0x006D6590`, `0x004A91B0`, `0x00653760`, `0x00692300`, `0x00656EC0`, `0x006DA380`, `0x00481810`, `0x005657A0`, `0x006D1F10`, `0x006D2280`, `0x00568300`, `0x00568350`.
- Ghidra xrefs/callers: `get_function_callers 0x006D6590`, `get_function_xrefs 0x006D6590`, assembly context for all direct call sites.
- Prior docs: `docs/research/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`, `docs/research/WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md`, `docs/research/traces/COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`.
- Rust scan: `src/map/terrain.rs`, `src/app_sim_tick.rs`, `src/app_commands.rs`, `src/app_context_order.rs`, `src/app_target_lines.rs`.
- INI scan: `ini/rules.ini`, `rulesmd.ini`, `art.ini`, `artmd.ini`.
