# NavCom/NavQueue Action-Line Endpoint Visibility - Ghidra Report

**Target question:** For normal selected mobile technos, what live state chooses the player-visible selected action-line endpoint: `ArchiveTarget`, `NavQueue` last endpoint, or `NavCom`; what timer/selection/option gates apply; how does movement bridge Z adjustment apply; and what must Rust do when `NavCom` is split from `MovementTarget`?

**Non-goals:** This report does not investigate DriveLocomotion movement execution, NavQueue writers, planning-mode multi-segment path lines, factory rally lines, Psychic Sensor enemy action lines, mind-control links, or final surface raster internals below `ActionLines__DrawLine`.

**Evidence needed to mark COMPLETE:** Decompile plus assembly/address context for `TechnoClass__DrawActionLines @ 0x004DC060`; call-site evidence for selected-human mobile-techno dispatch in `TacticalClass_Draw @ 0x006D3D10`; timer start/body evidence for the 25-frame gate; existing Rust surface scan for `src/app_target_lines.rs` and movement destination state.

**Stop conditions:** Stop if the path expands into all destination writers or waypoint queue construction, if Ghidra read-only evidence cannot confirm endpoint priority/order, or if the Rust implication requires modifying code. None triggered.

**Investigation mode:** targeted re-swarm slot

**Confidence:** High for selected-unit endpoint priority/order, gates, timer duration, and movement bridge-Z adjustment. Medium for future Rust destination-state design shape because this report did not investigate every `NavCom` writer.

**Active in YR:** Conditional. Active in standard Yuri's Revenge for human-owned selected mobile technos when the selected action-line option gate is enabled, the global action-line timer is live or the forced parameter is nonzero, and the object has `ArchiveTarget` or `NavCom`.

## Executive Finding

Selected action lines are live-state visuals, not command-recorded visuals. `TechnoClass__DrawActionLines @ 0x004DC060` first requires `ArchiveTarget != NULL || NavCom != NULL`. `ArchiveTarget` then has strict priority and returns after drawing the attack line. Only when `ArchiveTarget == NULL` does the movement branch run; that movement endpoint is `NavQueue.Items[NavQueue.Count - 1]` when the queue is nonempty, otherwise `NavCom`.

For Rust, `src/app_target_lines.rs` should not continue to treat `MovementTarget.final_goal` or `movement_target.path.last()` as the authoritative selected movement-line endpoint once `NavCom`/head-to state is split out. The selected movement line should read live destination state in this order: combat/archive target first, then last queued navigation endpoint, then current `NavCom`. `MovementTarget` may remain an adapter for movement execution, but it is the wrong owner for this player-visible endpoint.

## Verified Binary Facts

1. **The selected action-line method requires `ArchiveTarget` or `NavCom` before any timer or queue logic.**
   - Active in YR: Yes, conditional on selected-human dispatch.
   - Evidence: `TechnoClass__DrawActionLines @ 0x004DC060` reads `Techno+0x2B4` at `0x004DC069`, tests it, reads `+0x5A4` at `0x004DC073`, and returns when both are zero at `0x004DC07B -> 0x004DC328`.
   - Handoff importance: a nonempty `NavQueue` alone is not enough to draw a selected movement line if `NavCom` has already been cleared.

2. **The standard selected-human tactical call is option, selection, ownership, and radar-overlay gated.**
   - Active in YR: Conditional.
   - Evidence: `TacticalClass_Draw @ 0x006D3D10` selected-human branch checks selected byte `Techno+0x83` at `0x006D4735-0x006D473D`, reads `DAT_00843108` at `0x006D473F-0x006D4746`, pushes `0, 0`, then calls vtable `+0x438` at `0x006D4748-0x006D4750`. Existing option report verifies `DAT_00843108` mirrors `[Options] UnitActionLines`.
   - Handoff importance: Rust's `TargetLineState.unit_action_lines_enabled` is the correct kind of app-layer gate, but it must be wired to the real option value; selection and local-human ownership must also gate the builder.

3. **The timer gate is a draw-time global frame window, normally 25 frames.**
   - Active in YR: Yes for stock selected-unit calls because both vtable arguments are pushed as zero at `0x006D474A-0x006D474C`.
   - Evidence: `DrawActionLines` reads the low byte of its first parameter at `0x004DC081`; when zero, it reads `g_ActionLines_StartFrame 0x00B0EA80` and `g_ActionLines_Duration 0x00B0EA88` at `0x004DC089-0x004DC0AD`. `ActionLines__StartTimer @ 0x0070D150` writes current frame to `0x00B0EA80` and `0x19` to `0x00B0EA88` at `0x0070D150-0x0070D16B`.
   - Handoff importance: while the timer is active, endpoint position should still be resolved from current live target state each render, not from the original command click.

4. **`ArchiveTarget` wins over movement, including queued movement.**
   - Active in YR: Yes, conditional on active combat/archive target.
   - Evidence: `0x004DC0B3` tests `ArchiveTarget`; nonzero path calls the selected techno's vtable `+0x300` for source coords, calls `TechnoClass__Resolve_ArchiveTarget_Coords`, draws the line, and returns at `0x004DC1A0-0x004DC1A7`. The movement branch begins only at `0x004DC1AA`.
   - Handoff importance: `src/app_target_lines.rs` currently checks `entity.attack_target` before movement; keep that priority when adding `NavCom` and `NavQueue`.

5. **Movement endpoint priority is `NavQueue.Last` else `NavCom`, and endpoint coordinates come from the endpoint object's virtual `+0x48` coordinate getter.**
   - Active in YR: Yes for `NavCom` fallback; conditional for queued waypoints.
   - Evidence: movement source loads object coords at `0x004DC1AA-0x004DC1C4`; `NavQueue.Count` is read at `Foot+0x598` (`0x004DC1BC`). If zero, `NavCom` at `Foot+0x5A4` is loaded at `0x004DC1CA-0x004DC1D7`; if nonzero, `NavQueue.Items` at `Foot+0x58C` is loaded and indexed as `Items[Count - 1]` at `0x004DC1D9-0x004DC1E3`. Both paths call endpoint vtable `+0x48` at `0x004DC1EC`.
   - Handoff importance: Rust needs a navigation destination target reference, not just `(rx, ry)` final path state, because the binary endpoint source is an `AbstractClass*`.

6. **Movement bridge Z adjustment is applied after endpoint coordinate resolution and only to the movement endpoint.**
   - Active in YR: Conditional on in-bounds endpoint cell with bridge flag set.
   - Evidence: after endpoint `+0x48`, the movement branch converts endpoint X/Y to cell coords with signed correction around `0x004DC205-0x004DC229`, calls bounds check at `0x004DC23B`, fetches the cell and tests `Cell+0x140 & 0x100` at `0x004DC244-0x004DC25C`, then replaces endpoint Z with `CellClass__GetGroundHeight + DAT_008B3DF4` at `0x004DC25E-0x004DC276`.
   - Handoff importance: bridge/high-deck target lines must project the live endpoint with deck Z, not the ground cell Z.

7. **Selected buildings are not part of this selected-unit path.**
   - Active in YR: No for building action lines through `0x004DC060`; selected building rally lines are separate.
   - Evidence: existing selected-unit baseline and `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md` record mobile vtable entries to `0x004DC060`; building/base entries checked point to the empty stub `0x00459E60`.
   - Handoff importance: keep factory rally lines separate from selected mobile action lines.

## Current Rust Surface

Current Rust has an app-layer selected/factory line builder in `src/app_target_lines.rs`.

- `TargetLineState` stores a 25-tick timer and `unit_action_lines_enabled` gate (`src/app_target_lines.rs:61-83`).
- `record_command_lines` starts the timer from command payloads (`src/app_target_lines.rs:89-105`) and is called before queueing sim commands (`src/app_context_order.rs:720`).
- `selected_action_line_for_entity` currently returns attack target first, then uses `entity.movement_target.final_goal.or_else(path.last())` for movement (`src/app_target_lines.rs:185-201`).
- `GameEntity` currently has `movement_target`, `attack_target`, and `rally_target`, but no explicit `NavCom` or `NavQueue` field (`src/sim/game_entity.rs:110-124`).
- `MovementTarget` owns execution path and segmented final goal, and is removed automatically on arrival (`src/sim/components.rs:196-294`).
- `build_target_line_instances` currently receives `height_map` and `Simulation`; `project_cell_destination` can derive bridge deck height from `sim.resolved_terrain` for non-low bridge decks (`src/app_target_lines.rs:109-128`, `src/app_target_lines.rs:225-247`).

## Implementation Handoff

1. **Verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk**
   - `ArchiveTarget` wins and returns before movement -> preserve `attack_target` priority when adding destination state -> `src/app_target_lines.rs::selected_action_line_for_entity` -> selected unit with both attack target and live NavCom still draws to attack endpoint -> `test_selected_action_line_archive_target_wins_over_navcom_and_queue` -> High player visibility; affects every selected attacking mover.

2. **Verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk**
   - Movement endpoint is `NavQueue.Last` else `NavCom`, not movement path end -> add explicit sim-owned navigation destination state, e.g. `nav_com: Option<TargetRef>` and `nav_queue: Vec<TargetRef>` or equivalent, then make target-line builder consume it -> `src/sim/game_entity.rs`, movement destination lifecycle code, `src/app_target_lines.rs` -> selected mover with an active path segment to `(12,10)`, final queued waypoint `(30,30)`, and current `NavCom` `(20,20)` draws to `(30,30)` only when queue nonempty; otherwise draws to `(20,20)` -> `test_selected_action_line_uses_navqueue_last_else_navcom_not_path_last` -> High player visibility; prevents lines from pointing at the current path segment or disappearing when `MovementTarget` is transient.

3. **Verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk**
   - `DrawActionLines` requires `ArchiveTarget || NavCom`; a queue alone is not a draw owner -> do not let queued endpoint render if `NavCom` has been cleared; keep line visibility tied to live destination ownership -> movement destination state and `src/app_target_lines.rs` -> selected unit with `nav_queue` entries but `nav_com=None` and no attack target draws no selected movement line even while timer remains active -> `test_selected_action_line_navqueue_without_navcom_does_not_draw` -> Medium risk; important for arrival/stop teardown parity.

4. **Verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk**
   - Movement endpoint applies bridge Z adjustment after resolving endpoint object coords -> keep `project_cell_destination` using resolved bridge deck Z for `NavCom`/queued cell targets, and extend target refs so entity/object targets can project from live coords -> `src/app_target_lines.rs`, future destination target representation -> selected move to high bridge deck draws endpoint 60 px above same ground cell under standard height level 4 -> `test_selected_action_line_navcom_bridge_endpoint_uses_deck_z` -> Medium-high screenshot risk on bridge maps.

5. **Verified behavior -> Rust delta -> affected surface -> acceptance scenario -> proposed test name -> risk**
   - Timer is global draw-time gate, not endpoint storage -> keep `TargetLineState` as timer/option state only; do not store command endpoint in app state as authoritative endpoint -> `src/app_target_lines.rs`, `src/app_context_order.rs` -> after a command starts the timer, changing live `NavCom` before expiry changes the line endpoint on the next render -> `test_selected_action_line_resolves_live_navcom_each_frame` -> Medium risk; avoids stale target-line feedback.

## Negative Facts / Do Not Do

- Do not use `movement_target.path.last()` as the selected movement-line endpoint after `NavCom` exists. Binary movement endpoint source is `NavQueue.Items[Count - 1]` else `NavCom`, both via endpoint object `+0x48`, at `0x004DC1BC-0x004DC1EC`.
- Do not draw a queued movement line when `NavCom` is null and `ArchiveTarget` is null. The early target gate checks only `ArchiveTarget` and `NavCom` at `0x004DC069-0x004DC07B`.
- Do not store click-command endpoint in `TargetLineState` as authoritative destination. `DrawActionLines` reads live techno fields every draw while the timer permits it.
- Do not make `[Options] UnitActionLines` control factory rally lines, Psychic Sensor enemy lines, or mind-control links. Existing reports show this gate applies to the selected-human `DrawActionLines` branch at `0x006D473F-0x006D4750`; those other line families have separate callers.
- Do not collapse bridge endpoint projection to ground height. The movement branch conditionally replaces endpoint Z with ground height plus bridge offset at `0x004DC25E-0x004DC276`.

## Remaining Uncertainty

- The exact Rust shape for `NavCom`/`NavQueue` target references remains design work. Binary state uses `AbstractClass*`; Rust likely needs an enum that can represent entity, cell, and later building/dock targets.
- This report did not verify NavQueue writer semantics or queue mutation timing. It only verifies selected-line endpoint consumption.
- This report did not verify full stop/arrival clearing of `NavCom`; arrival lifecycle is covered by adjacent NavCom/DriveLocomotion slots and should own teardown semantics.
- Rust's existing `unit_action_lines_enabled` gate exists, but this slot did not verify it is wired to user options throughout the app.

## Stale-Doc Replacement Wording

- `docs/research/TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, section "Current Rust Implementation Status", currently says searches under `src/` returned no matching implementation points. Replace with: "Rust now has selected action-line support centered on `src/app_target_lines.rs`: `TargetLineState`, `record_command_lines`, `build_target_line_instances`, attack-target priority, movement endpoint projection, endpoint boxes, and a 25-tick option-gated timer. It still derives movement endpoints from `MovementTarget.final_goal` or `path.last()` instead of a verified `NavQueue.Last else NavCom` state source."

## Sources

- Ghidra decompile: `TechnoClass__DrawActionLines @ 0x004DC060`.
- Ghidra assembly contexts: `0x004DC069`, `0x004DC073`, `0x004DC081`, `0x004DC0B3`, `0x004DC1BC`, `0x004DC1CA`, `0x004DC1D9`, `0x004DC1EC`, `0x004DC205`, `0x004DC244`, `0x004DC25E`.
- Ghidra assembly contexts: `TacticalClass_Draw @ 0x006D4735`, `0x006D473F`, `0x006D4748`, `0x006D4750`.
- Ghidra assembly context: `ActionLines__StartTimer @ 0x0070D150`.
- Existing reports cross-checked: `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, `SELECTED_UNIT_ACTION_TARGET_LINE_BASELINE_GHIDRA_REPORT.md`, `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`, `ACTIONLINES_TIMER_START_CLEAR_XREFS_GHIDRA_REPORT.md`, `IMPLEMENTATION_HIGH_BRIDGE_TARGET_LINE_PROJECTION_TRACE.md`.
- Rust scan: `src/app_target_lines.rs`, `src/app_context_order.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs`, `src/sim/game_entity.rs`, `src/sim/components.rs`.

## Status

COMPLETE for the scoped consumer question: selected action-line endpoint priority/order, gates, timer, bridge-Z adjustment, and Rust handoff when `NavCom` is split from `MovementTarget`.
