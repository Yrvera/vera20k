# Tactical Screen Pixel To Cell Inverse - Ghidra Research Report

**Address(es):** `0x006D6590` (active tactical screen/client-pixel -> cell inverse)
**Investigation Mode:** exhaustive-slice attempted; downgraded to partial coverage-map
**Claimed Scope:** tactical screen/client pixel to map-cell inverse behavior as it affects cursor pick, high-bridge target cells, building placement target cells, and app-layer consumers.
**Non-Scope:** generic rendering, object selection ordering beyond coordinate inversion, minimap preview generation, pathfinding after the target cell has already been chosen.
**Confidence:** High for active `0x006D6590` identity, height-loop cap, viewport-offset sign, and cardinal bridge branch after the 2026-05-22 verify-doc/recheck pass; High for current Rust surface scan.
**Active in YR:** Yes. The 2026-05-22 audit confirmed live YR callers including tactical pick and radar update paths.
**Status:** PATCHED 2026-05-22 after verify-doc-swarm YELLOW audit; Rust-side line citations re-patched 2026-07-18 after verify-doc-fix-swarm w1 slot 15 (all binary claims re-confirmed CONFIRMED via live decompile of `0x006D6590`; ~14 stale `src/` line ranges corrected for SRC_LINE_DRIFT after further file growth).

## 1. Overview

The tactical inverse is the player-visible path from a screen/client pixel to a target map cell. It drives where a right-click move lands, which cell a ready building preview/placement uses, which cell a superweapon targets, and which bridge deck cell receives cursor/action feedback.

Prior Ghidra documentation identified `0x006D6590` as the active client-pixel -> cell inverse. A 2026-05-22 verify-doc/recheck pass confirmed the live function identity, callers, viewport-offset sign, height-loop cap semantics, and cardinal bridge branch. This report keeps the Rust-facing handoff and updates the previously stale caveats.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `CellClass+0x11B` | terrain height level used by prior inverse report's height iteration | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:112`, `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md:184` | Yes - prior docs say height/elevation helpers are active in tactical/pathing code |
| `CellClass+0x140 bit 0x100` | structural high-bridge flag used by prior inverse report's bridge branch and by action/cursor height adjustment | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:115`, `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md:40-53` | Yes - high bridge body cells in standard YR |
| `CellClass+0x140 bit 0x80` | effective-height +4 contribution in `CellClass__GetEffectiveHeight` | `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md:184-198` | Yes - bridge/elevation helper active |
| `g_RadarViewportOffsetX/Y` | viewport offset subtracted inside `0x006D6590`; some callers may pre-add before calling | 2026-05-22 verify-doc audit of `0x006D6590` | Yes - active tactical/radar callers confirmed |
| camera fields `+0xB0/+0xB4` | camera offset applied by forward `CoordsToClient2` | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:82`, `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:100` | Yes - prior report says forward tactical transforms run on every frame/tick/click path |

## 3. Core Logic

### 3.1 Prior binary-documented inverse behavior

Prior report `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md` says the inverse at `0x006D6590`:

1. Applies a camera/matrix inverse to screen/client pixels plus camera terms, subtracting `g_RadarViewportOffsetX/Y` inside `0x006D6590` after tactical camera fields are applied. Some callers may pre-add the viewport offset before entering this function.
2. Converts the result to a candidate cell.
3. Iterates height correction using `CellClass+0x11B`.
4. Caps the loop by comparing the incremented counter against `0xB3` (i.e., `if (0xb3 < local_58) → exit`); the effective cap is 180 failed attempts because the counter runs 0–179 before the guard fires. (corrected 2026-05-28: was "compares against `0xB4`"; binary `if (0xb3 < local_58)` confirmed via `decompile_function 0x006D6590` — OPERATOR_OR_ORDER_DRIFT)
5. If the resolved cell has `CellClass+0x140 & 0x100`, checks cardinal bridge neighbors and may shift the selected cell using strict `> 0xF` / `> 15` pixel edge tests.

**Active in YR:** Yes. The 2026-05-22 audit confirmed the exact function and live tactical/radar caller paths.

### 3.2 Height iteration cap and termination

The load-bearing claim is that the inverse is not a three-pass heuristic: the assembly increments the scan counter and compares against `0xB3` (`if (0xb3 < local_58) → exit`), giving an effective cap of 180 failed attempts (counter 0–179). Early exit is by convergence or bridge-shift finalization. This matters only on pathological high/steep terrain, but it is part of the original target-cell contract. (corrected 2026-05-28: was "compares against `0xB4`"; binary shows `0xb3` as the comparison literal — OPERATOR_OR_ORDER_DRIFT)

**Active in YR:** Yes. Evidence: 2026-05-22 audit of live `0x006D6590`.

### 3.3 Bridge neighbor shift rules

Prior documentation confirms a directional/cardinal bridge branch, not a radial bridge search:

- condition starts from a bridge structural cell (`CellClass+0x140 & 0x100`);
- checks up to four cardinal neighbors;
- uses strict `> 15` pixel edge tests;
- may shift the picked cell to a bridge neighbor.

The 2026-05-22 audit confirmed cardinal neighbor calls using directions `2/4/0/6`, bridge flag `0x100`, orientation bit `0x800`, and strict `0xF` threshold comparisons. Some endpoint tie-break details should still be checked in focused tests before broad bridge-click rewrites.

**Active in YR:** Yes.

### 3.4 Camera/sidebar/radar viewport offsets

The 2026-05-22 audit corrected the older sign wording: `0x006D6590` subtracts `g_RadarViewportOffsetX/Y` internally. Some callers may pre-add the viewport offset before the call, so the end-to-end coordinate-space contract is caller-sensitive. Current Rust screen preprocessing is simpler:

- `screen_point_to_world_cell` calls `screen_point_to_world`, then `world_point_to_cell`;
- `world_point_to_cell` calls `terrain::screen_to_iso_with_height_and_bridges`;
- current Rust evidence: `src/app_sim_tick.rs:1482-1494` (`screen_point_to_world_cell`) and `src/app_sim_tick.rs:1448-1476` (`world_point_to_cell`). (corrected 2026-07-18: was `1277-1289` and `1243-1271`; file grew since the 2026-05-28 pass, shifting both functions further down — SRC_LINE_DRIFT, verified by direct read of src/app_sim_tick.rs)

The Rust screen-to-world path uses `screen / zoom + camera` and forwards the bridge height map. It does not show an explicit radar/sidebar viewport-offset term in the scanned surface.

**Active in YR:** Yes for the inverse path. A live viewport/camera pair is still useful to validate the caller-side coordinate-space contract against Rust full-window cursor coordinates.

### 3.5 Cursor pick/building placement implications

Current Rust consumers that depend directly on the inverse include:

- `update_building_placement_preview` uses `screen_point_to_world_cell` for the preview origin at `src/app_sim_tick.rs:1113-1160`. (corrected 2026-07-18: was `908-955`; function shifted further down the file since the 2026-05-28 pass — SRC_LINE_DRIFT, verified by direct read of src/app_sim_tick.rs)
- `place_ready_building_at_cursor` uses the stored preview cell when present, otherwise falls back to `screen_point_to_world_cell` at `src/app_commands.rs:203-273`. (corrected 2026-07-18: was `211-227`; full function range verified by direct read of src/app_commands.rs — SRC_LINE_DRIFT)
- `launch_super_weapon_at_cursor` rejects sidebar/minimap hits before using `screen_point_to_world_cell` at `src/app_commands.rs:285-317`. (corrected 2026-07-18: was `293-306`; full function range verified by direct read of src/app_commands.rs — SRC_LINE_DRIFT)
- `app_context_order` uses `screen_point_to_world_cell` and entity hover picking with `bridge_height_map` for right-click order cells (`rg` evidence: `src/app_context_order.rs:43`, `src/app_context_order.rs:94-103`, `src/app_context_order.rs:499-517`). (corrected 2026-07-18: middle citation was `91-99`; `hover_target_at_point` call verified at src/app_context_order.rs:94-103 by direct read — SRC_LINE_DRIFT)

**Active in YR:** Yes for the equivalent player actions: hover/right-click/building placement/superweapon cell targeting are normal YR input paths. Rust evidence is implementation scan only, not binary evidence.

## 4. INI Keys

No INI key directly controls the coordinate inverse itself. Bridge existence and placement validity are data-driven elsewhere by terrain/map/overlay/rules systems, but this slice found no inverse-specific INI key to trace.

## 5. Integration Points

| Integration point | Verified / current behavior | Evidence | Active in YR |
|---|---|---|---|
| Forward projection | `CoordsToClient` / `CoordsToClient2` project lepton coords to tactical pixels with Z lift | `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:80-83`, `COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md:23-62` | Yes |
| Inverse projection | `0x006D6590` maps client pixels back to cells with height and bridge corrections | 2026-05-22 verify-doc audit; `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:83`, `104-121` | Yes |
| High bridge action/cursor | bridge flag adds height offset but does not create a special high-bridge action code | `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md:40-68` | Yes |
| Rust screen cell owner | `screen_point_to_world_cell` centralizes app screen -> world -> cell conversion and passes `tactical_bridge_inverse_map` | `src/app_sim_tick.rs:1482-1494` (corrected 2026-07-18: was `1277-1289`; SRC_LINE_DRIFT) | Rust-only |
| Rust target lines | current `project_cell_destination` can use a passed bridge map or derive deck height from `Simulation.resolved_terrain` | `src/app_target_lines.rs:254-284` (corrected 2026-07-18: was `228-258`; `project_cell_destination` + `bridge_deck_height_for_cell` verified at src/app_target_lines.rs:254-284 by direct read — SRC_LINE_DRIFT) | Rust-only |

## 6. Current Rust Implementation Status

| Surface | Current status | Evidence | Delta against prior binary docs |
|---|---|---|---|
| `terrain::screen_to_cell_tactical_inverse` + `apply_tactical_bridge_inverse` | 180 height iterations (`TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS`), cardinal directional bridge neighbor shift matching gamemd's four-direction pattern | `src/map/terrain.rs:296-428` | (corrected 2026-05-28: was "3 iterations + 7x7 search"; current Rust already updated to 180-iteration scan + directional bridge logic — STALE row) |
| `app_sim_tick::screen_point_to_world_cell` | `screen / zoom + camera`, then height+bridge inverse | `src/app_sim_tick.rs:1482-1494` (corrected 2026-07-18: was `1277-1289`; SRC_LINE_DRIFT) | Unchecked: no explicit `g_RadarViewportOffsetX/Y` equivalent found in scanned surface |
| building placement preview | cursor cell is preview origin | `src/app_sim_tick.rs:1113-1160` (corrected 2026-07-18: was `908-955`; SRC_LINE_DRIFT) | Risk: any inverse pixel drift directly moves ghost/build cell |
| ready building placement click | uses preview cell if present | `src/app_commands.rs:203-273` (corrected 2026-07-18: was `211-227`; SRC_LINE_DRIFT) | Good stabilization: click uses the shown preview cell |
| superweapon target | rejects sidebar/minimap before inverse | `src/app_commands.rs:285-317` (corrected 2026-07-18: was `293-306`; SRC_LINE_DRIFT) | Good guard for sidebar/minimap; exact viewport offset still unchecked |
| target line cell projection | can resolve high bridge deck from sim terrain even without explicit bridge map | `src/app_target_lines.rs:254-284` (corrected 2026-07-18: was `222-258`; SRC_LINE_DRIFT) | Prior trace's line-endpoint failure is stale for current code; needs acceptance test |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x006D6590` identity | verified | 2026-05-22 verify-doc audit; live tactical/radar callers | none |
| 180 height iteration cap | verified | 2026-05-28 decompile: `if (0xb3 < local_58)` confirmed via `decompile_function 0x006D6590`; comparison value is `0xB3` (corrected from `0xB4`) | none for cap; add tests |
| bridge neighbor shift branch | verified for cardinal/orientation/strict threshold | 2026-05-22 audit: directions `2/4/0/6`, `0x100`, `0x800`, strict `> 0xF` | endpoint tie-break coverage tests |
| radar/sidebar viewport offset | verified for internal subtract; caller contract still needs runtime test | 2026-05-22 audit; `COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md:50` | compare Rust full-window cursor coordinates at tactical/sidebar boundary |
| high bridge cursor action | verified from prior report | `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md:40-68`, `221-229` | none for high-bridge action code; not a coordinate inverse detail |
| Rust inverse implementation | verified current scan | `src/map/terrain.rs:296-429` (corrected 2026-07-18: was `296-428`; `apply_tactical_bridge_inverse` closing brace confirmed at line 429 by direct read — off-by-one SRC_LINE_DRIFT) | Add targeted parity tests |
| Rust building placement consumers | verified current scan | `src/app_sim_tick.rs:1113-1160`, `src/app_commands.rs:203-273` (corrected 2026-07-18: was `908-955` and `211-227`; SRC_LINE_DRIFT) | Add bridge endpoint/sidebar-boundary placement tests |
| Rust target line endpoint projection | verified current scan | `src/app_target_lines.rs:254-284` (corrected 2026-07-18: was `222-258`; SRC_LINE_DRIFT) | Add regression test because prior trace is stale |
| generic object selection | deferred | scope excluded by parent | Follow separate selection-order investigation |
| minimap preview | deferred | scope excluded by parent | Follow minimap-specific report if needed |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ1 - What prior research exists? -> Relevant prior coordinate, bridge cursor, and high-bridge pick trace reports found.` (evidence: `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`, `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md`, `COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`)
- `[RESOLVED] OQ2 - Is `0x006D6590` still the exact inverse function identity/name? -> Yes, verified by the 2026-05-22 audit with live tactical/radar callers.`
- `[RESOLVED] OQ3 - Is the 180 iteration cap inclusive or exclusive, and what exact condition terminates the loop? -> The loop increments the counter and compares against `0xB3` (binary: `if (0xb3 < local_58) return`); effective cap is 180 failed attempts (counter 0–179). (corrected 2026-05-28: was `0xB4`; binary confirms `0xB3` via decompile_function 0x006D6590 — OPERATOR_OR_ORDER_DRIFT)`
- `[PARTIAL] OQ4 - What is the exact bridge neighbor shift order and direction rule? -> Cardinal directions `2/4/0/6`, bridge flag `0x100`, orientation bit `0x800`, and strict `>0xF` edge tests are verified; endpoint tie-breaks still need focused acceptance tests.`
- `[PARTIAL] OQ5 - Are `g_RadarViewportOffsetX/Y` always applied to tactical clicks, or only to specific UI/radar coordinate spaces? -> `0x006D6590` subtracts them internally; some callers may pre-add before calling. Runtime boundary tests should verify Rust's full-window coordinate contract.`
- `[RESOLVED] OQ6 - Does Rust still use a 7x7 bridge search? -> No (corrected 2026-05-28): current Rust uses `apply_tactical_bridge_inverse` with four cardinal direction probes matching gamemd's pattern, not a 7x7 radial search. Prior answer was stale — STALE` (evidence: `src/map/terrain.rs:354-429` (corrected 2026-07-18: was `354-428`; closing brace of `apply_tactical_bridge_inverse` confirmed at line 429 by direct read — off-by-one SRC_LINE_DRIFT))
- `[RESOLVED] OQ7 - Does Rust still use only 3 height iterations? -> No (corrected 2026-05-28): current Rust uses `for _ in 0..TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS` (180 iterations) matching gamemd. The `for _ in 0..3` loop no longer exists; function is now `screen_to_cell_tactical_inverse` at `src/map/terrain.rs:296`. Prior doc answer was stale — STALE`
- `[RESOLVED] OQ8 - Does building placement consume the inverse? -> Yes, preview uses `screen_point_to_world_cell`; placement click uses preview cell if present, else inverse fallback.` (evidence: `src/app_sim_tick.rs:1113-1160`, `src/app_commands.rs:203-273`; corrected 2026-07-18: was `908-955` and `211-227` — SRC_LINE_DRIFT)
- `[RESOLVED] OQ9 - Does superweapon targeting consume the inverse? -> Yes, after sidebar/minimap guard.` (evidence: `src/app_commands.rs:285-317`; corrected 2026-07-18: was `293-306` — SRC_LINE_DRIFT)
- `[RESOLVED] OQ10 - Is the old high-bridge target-line failure still current? -> Not as written; current projection can derive deck height from `Simulation.resolved_terrain`.` (evidence: `src/app_target_lines.rs:254-284`; corrected 2026-07-18: was `222-258` — SRC_LINE_DRIFT)
- `[RESOLVED] OQ11 - Are there inverse-specific INI keys? -> None found in this scoped scan.` (evidence: no relevant keys in prior coordinate reports; inverse constants are binary/math constants)
- `[DEFERRED] OQ12 - Null pointer / invalid cell / off-map behavior of gamemd inverse?` (category: needs-runtime-debugger; reason: exact edge branches in `0x006D6590` not reopened; next-step-if-pursued: inspect map-cell lookup fallback in inverse and MapClass cell accessor)
- `[DEFERRED] OQ13 - Pause/replay/save-restore effects?` (category: out-of-scope; reason: coordinate inverse is an input/render helper and no stateful tick behavior was investigated; next-step-if-pursued: trace input dispatch lifecycle separately)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Binary says screen-pixel inverse can iterate height correction for 180 failed attempts (`if (0xb3 < counter)` guard), not only 3 passes. | 2026-05-28 decompile `0x006D6590`; current Rust `src/map/terrain.rs:306` | resolved (corrected 2026-05-28: was "mismatch"; Rust already updated to 180-iteration loop — STALE row) | `src/map/terrain.rs::screen_to_cell_tactical_inverse` | Rust already matches; add tests confirming 180-iteration convergence. | Synthetic height-map where first three guesses do not converge but a later correction resolves the intended cell; proposed test name: `test_screen_to_cell_height_iteration_handles_more_than_three_steps` | Do not assume "3 is enough" as a parity rule. |
| Binary says high-bridge inverse uses cardinal/directional neighbor checks and strict `>15` edge tests, not radial closest search. | 2026-05-28 decompile `0x006D6590`; current Rust `src/map/terrain.rs:354-429` (corrected 2026-07-18: was `354-428`; closing brace of `apply_tactical_bridge_inverse` confirmed at line 429 by direct read — off-by-one SRC_LINE_DRIFT) | resolved (corrected 2026-05-28: was "mismatch"; Rust now uses `apply_tactical_bridge_inverse` with directional cardinal probes matching gamemd — STALE row) | `src/map/terrain.rs::apply_tactical_bridge_inverse`; app consumers in `app_context_order.rs`, `app_commands.rs`, `app_sim_tick.rs` | Bridge endpoint/boundary pixels must resolve to the same target cell sequence as gamemd. | High bridge endpoint click at the ramp/body boundary chooses the directional neighbor expected by gamemd; proposed test name: `test_screen_to_cell_bridge_endpoint_uses_directional_neighbor_shift` | Do not replace gamemd's branch with a larger radial search; that can pick plausible but wrong bridge cells near endpoints/curves. |
| Binary says viewport/radar offsets are subtracted inside `0x006D6590`; some callers may pre-add before calling. Current Rust scan shows only `screen / zoom + camera`. | 2026-05-28 decompile (re-confirmed 2026-07-18); Rust `src/app_sim_tick.rs:1482-1494`, `src/app_commands.rs:285-317` (corrected 2026-07-18: was `1277-1289` and `293-306` — SRC_LINE_DRIFT) | unchecked caller contract | `src/app_sim_tick.rs::screen_point_to_world_cell`; sidebar/minimap guards in `src/app_commands.rs` and `src/app_sidebar_render.rs` | Tactical clicks at the sidebar boundary must not shift one cell relative to gamemd; sidebar/minimap clicks must not arm/fire as tactical cells. | Move cursor from last tactical pixel into sidebar and back; cell under the last tactical pixel remains stable while sidebar pixel is ignored; proposed test name: `test_screen_to_cell_sidebar_boundary_applies_tactical_viewport_origin` | Do not treat full-window X/Y as tactical-client X/Y until the viewport origin contract is verified. |
| Current Rust placement click uses the preview cell, not a fresh inverse, when a preview exists. | `src/app_commands.rs:203-273`; preview source `src/app_sim_tick.rs:1113-1160` (corrected 2026-07-18: was `211-227` and `908-955` — SRC_LINE_DRIFT) | none observed for click/preview drift | `src/app_commands.rs::place_ready_building_at_cursor`; `src/app_sim_tick.rs::update_building_placement_preview` | Keep placement commit cell identical to the visible ghost cell even if cursor position changes between preview update and click handling. | Place a building on a bridge/height boundary after cursor movement between frames; committed `rx,ry` equals preview `rx,ry`; proposed test name: `test_ready_building_click_commits_preview_cell_after_inverse_boundary_jitter` | Do not recompute the placement cell on click when a preview exists. |
| High-bridge cursor action uses normal Move/Attack logic; the bridge flag supplies height context, not a special high-bridge cursor code. | `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md:40-68`, `221-229` | mostly independent of inverse; cursor target cell still depends on inverse | `src/app_cursor.rs`, `src/app_context_order.rs`, `src/app_entity_pick.rs` | Once inverse selects the bridge cell, cursor/action classification should remain normal move/attack, not introduce special high-bridge cursor semantics. | Hover/click intact and damaged high bridge cells: action category stays normal Move for traversable cells; proposed test name: `test_high_bridge_cell_pick_preserves_normal_move_cursor_after_inverse` | Do not add a high-bridge-specific cursor action as a workaround for inverse errors. |

### Negative Facts / Do Not Do

- Do not implement a radial 7x7 search as "gamemd behavior"; it differs from the verified cardinal/directional branch with strict `>15` edge tests. Note: current Rust (`src/map/terrain.rs:354-429` (corrected 2026-07-18: was `354-428`; closing brace of `apply_tactical_bridge_inverse` confirmed at line 429 by direct read — off-by-one SRC_LINE_DRIFT)) already uses the directional approach — this warning is for future refactors. Evidence: 2026-05-28 decompile of `0x006D6590`. Active in YR: Yes for bridge picking generally; endpoint tie-break details still need acceptance coverage.
- Do not assume three height iterations are a gamemd cap. Evidence: 2026-05-28 decompile confirms an effective 180-attempt scan via `if (0xb3 < local_58)` guard (corrected from `0xB4`). Current Rust uses `0..TACTICAL_INVERSE_MAX_SCAN_ATTEMPTS` (180) at `src/map/terrain.rs:306` and already matches. Active in YR: Yes.
- Do not add a special high-bridge cursor action to compensate for coordinate issues. Evidence: bridge flag is height-only in `WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md:40-68`. Active in YR: Yes.
- Do not fire superweapons/building placement from sidebar/minimap pixels as if they were tactical cells. Evidence: current Rust already guards superweapon release at `src/app_commands.rs:285-317` (corrected 2026-07-18: was `293-306` — SRC_LINE_DRIFT); prior inverse offset risk is documented at `COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md:236-248`. Active in YR: Conditional until exact viewport caller is rechecked.
- Do not cite `COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md` Stage 9 as current Rust truth without checking the code; target-line projection has since changed. Evidence: old trace lines 66-92 vs current `src/app_target_lines.rs:254-284` (corrected 2026-07-18: was `222-258` — SRC_LINE_DRIFT). Active in YR: N/A, Rust-doc staleness.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`: patched 2026-05-22 for viewport-offset sign, effective 180 attempts, strict `>15` bridge edge tests, and stale Rust guidance. Note: comparison literal is `0xB3` not `0xB4` (corrected 2026-05-28).
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`: replace the Stage 9 current-Rust failure wording with: "Older Rust projected target-line cell destinations with ground height only. Current Rust `project_cell_destination` can derive high-bridge deck height from `Simulation.resolved_terrain` even when no explicit bridge map is passed; keep a regression test for high-bridge move/rally target-line endpoints."

## 10. Remaining Uncertainty

- Exact endpoint tie-break behavior in the bridge shift rule: still needs acceptance-test coverage.
- Exact screen/camera/sidebar/radar caller coordinate-space contract: internal subtract is verified, but Rust full-window input mapping still needs boundary testing.
- Off-map/null-cell fallback behavior inside the inverse: unresolved.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/COORD_TRANSFORM_AUDIT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/COORDINATE_ELEVATION_LAYER_MODEL_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/WHAT_ACTION_BRIDGE_CELLS_CURSOR_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/COORD_HEIGHT_SCREEN_PICK_HIGH_BRIDGE_MOVE_TRACE.md`
- `C:/Users/enok/Documents/ra2-rust-game/src/map/terrain.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_sim_tick.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_commands.rs`
- `C:/Users/enok/Documents/ra2-rust-game/src/app_target_lines.rs`
