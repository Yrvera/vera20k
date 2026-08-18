# Minimap GadgetClass Click Provenance - Ghidra Research Report

**Address(es):** `0x006539D0`, `0x007F0320`, `0x007F0338`, `0x00653EA0`, `0x00639DA0`, `0x004AB9B0`, `0x006930A0`, `0x0063A5A0`, `0x0063A8E0`, `0x0063AB60`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** ordinary live in-game sidebar radar/minimap mouse event ownership, the button-down/up/drag path that pans the camera, and whether selected-unit minimap clicks dispatch movement/attack orders.  
**Non-Scope:** radar inverse math, camera clamp formulas, tactical order internals beyond proving absence/presence from radar clicks, radar rendering, terrain/object dots, sidebar chrome, shell/dialog radar assets, spy satellite, gap generators, and command-bar gadgets.  
**Confidence:** High for static event ownership and selected-unit branch behavior; Medium for runtime user gesture logging because no live debugger server was available.  
**Active in YR:** Yes. Evidence: `RadarClass__constructor @ 0x00652960` installs `vtable_RadarClass`; vtable data at `0x007F0338` points to the unnamed input handler `0x006539D0`; `GScreenClass__Input @ 0x004F4320` dispatches through the same RadarClass vtable family, and the Win32 mouse message wrapper reaches `FUN_006930A0 @ 0x006930A0`.

## Required Investigation Notes

- Target question: Which live input owner invokes the radar viewport/camera path for ordinary minimap clicks, and do selected units cause minimap clicks to issue commands?
- Non-goals: Do not redo radar-space inverse math, camera clamp formulas, tactical `BandBox_LeftUp` internals, or rendering.
- Evidence needed to mark COMPLETE: vtable/caller proof for the ordinary radar input handler; branch context for button press/release; proof that selected-unit radar clicks do or do not reach `DisplayClass__BandBox_LeftUp`; Rust-facing handoff.
- Stop conditions: stop after ordinary live in-game minimap event provenance is proven; do not mutate Ghidra; write only this report and the shared claims file.

## 1. Overview

The remaining uncertainty in `MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM_GHIDRA_REPORT.md` is resolved: ordinary in-game radar clicks are owned by `RadarClass`'s vtable input handler at `0x006539D0`, not by a separate base `GadgetClass` button object. The handler uses radar-surface bounds globals, calls `RadarClass__GetObjectAtRadarPixel`, computes/updates the camera through `FUN_006D6070`, and contains a selected-object classifier that prevents normal selected units from falling through to the tactical `BandBox_LeftUp` command dispatch.

There is a `BandBox_LeftUp` call inside the same unnamed handler, but it is not the selected-unit minimap command path. For ordinary selected mobile/building objects, `FUN_00639DA0` returns a non-`-1` code and the handler jumps to the radar camera/cursor path before the release-order branch.

## 2. Class / Vtable Evidence

| Item | Finding | Evidence | Active in YR |
|---|---|---|---|
| `vtable_RadarClass` | Starts at `0x007F0320` in the memory dump used here; constructor writes it into the object | `RadarClass__constructor @ 0x00652960`; `read_memory 0x007F0320` | Yes |
| `vtable_RadarClass+0x18` | Points to unnamed radar input handler `0x006539D0` | `read_memory 0x007F0320`: dword at `0x007F0338` = `0x006539D0`; `get_bulk_xrefs` reports `0x007F0338 -> 0x006539D0` | Yes |
| `vtable_RadarClass+0x48` | Points to `GScreenClass__Input @ 0x004F4320` | `read_memory 0x007F0320`: dword at `0x007F0368` = `0x004F4320`; `get_bulk_xrefs 0x004F4320` includes `0x007F0368` | Yes |
| `vtable_RadarClass+0x4C` | Points to `Minimap_Chat_Dispatch @ 0x00653850` | `read_memory 0x007F0320`: dword at `0x007F036C` = `0x00653850`; direct call from `PowerClass__AnimationTick @ 0x00640391` | Yes |
| `vtable_RadarClass+0xF4` | Points to `FUN_00653F70`, a one-call wrapper around `FUN_006D6070` | `get_bulk_xrefs 0x00653F70`: data xref `0x007F0414`; decompile `0x00653F70` | Yes |

## 3. Core Logic

### 3.1 Ordinary radar click owner is RadarClass, not base GadgetClass

Active in YR: Yes.

The live object path is `RadarClass`, which inherits the screen/display vtable chain. `RadarClass__constructor @ 0x00652960` writes `&vtable_RadarClass`; memory at `0x007F0338` contains `0x006539D0`, the unnamed radar input handler. This closes the old "GadgetClass button-event provenance" gap by correcting the premise: ordinary in-game radar input is not proven through a separate sidebar `GadgetClass` button at all.

Base `GadgetClass::Input @ 0x004E1640` and `GadgetClass::Clicked_On @ 0x004E13F0` remain active for many UI controls, but this minimap click path is owned by the `RadarClass` vtable entry and the Win32/display mouse handling path.

Evidence: `RadarClass__constructor @ 0x00652960`; `read_memory 0x007F0320`; `get_bulk_xrefs 0x006539D0`; `GScreenClass__Input @ 0x004F4320`.

### 3.2 Win32 mouse handling also suppresses tactical selection around radar helpers

Active in YR: Yes.

The Win32-style handler `FUN_006930A0` receives mouse messages and calls radar/selection guards before tactical command handling:

- `WM_LBUTTONDOWN` case `0x201` calls `FUN_0063A5A0(rel_x, rel_y)` at `0x00693187`. If that helper returns nonzero, the tactical press path does not start band-box selection.
- `WM_LBUTTONUP` case `0x202` calls `FUN_0063A8E0(rel_x, rel_y)` at `0x00693225`. If that helper returns nonzero, the tactical `DisplayClass__BandBox_LeftUp` call in `FUN_006930A0` is skipped.
- Right button press/release call `FUN_0063AAC0` / `FUN_0063AB00`, which set and clear radar suppression state `DAT_00AC4CB0` and handle a no-selection small-delta selection-clear case.

Evidence: decompile `FUN_006930A0 @ 0x006930A0`; decompile `FUN_0063A5A0`, `FUN_0063A8E0`, `FUN_0063AAC0`, `FUN_0063AB00`.

### 3.3 RadarClass handler bounds clicks against radar surface origin/size

Active in YR: Yes.

The unnamed handler at `0x006539D0` tests input against the radar surface rectangle:

- origin x: `DAT_00880C84`
- origin y: `DAT_00880C88`
- width: `DAT_00880C8C`
- height: `DAT_00880C90`

These are the same radar-origin globals used by radar events. The handler compares x/y with lower-inclusive and upper-exclusive bounds before calling `RadarClass__GetObjectAtRadarPixel @ 0x00656750`.

Evidence: assembly context `0x00653A4D..0x00653A83`; xrefs to `DAT_00880C84/88/8C/90`; `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`.

### 3.4 Selected-unit classifier prevents normal selected objects from reaching the order branch

Active in YR: Yes.

After the handler determines a radar cell/action, it checks radar availability through `FUN_00637AA0` and then calls `FUN_00639DA0`. That classifier reads `g_CurrentObjects_Count` and `g_CurrentObjects_Data`. For normal selected objects it returns a concrete code (`0`, `1`, or `2` in the decompiled body, depending on type/virtual checks) rather than `-1`.

The critical branch is:

```text
0x00653CC0 call FUN_00637AA0
0x00653CC5 test al, al
0x00653CC7 jz  0x00653CD7
0x00653CC9 call FUN_00639DA0
0x00653CCE cmp eax, -1
0x00653CD1 jnz 0x00653D62
```

`0x00653D62` is the radar camera/cursor branch. The `DisplayClass__BandBox_LeftUp` release-order call is below the fallthrough path at `0x00653D58`, so normal selected-object minimap clicks bypass selected-unit command dispatch.

Evidence: assembly context `0x00653CC0..0x00653D62`; decompile `FUN_00639DA0 @ 0x00639DA0`; selection globals verified in `SELECTION_SYSTEM_GHIDRA_REPORT.md`.

### 3.5 The order dispatch call exists, but not for normal selected radar clicks

Active in YR: Yes, conditional.

The same unnamed radar handler contains a `DisplayClass__BandBox_LeftUp @ 0x004AB9B0` call at `0x00653D58`. It is guarded by the fallthrough path where the selected-object classifier did not route to the camera branch and by a left-release bit check:

```text
0x00653CDF mov bl, [esp+0x58]
0x00653CE3 test bl, 0x8
...
0x00653D04 test bl, 0x4
0x00653D07 jz 0x00653EC3
...
0x00653D58 call DisplayClass__BandBox_LeftUp
```

`DisplayClass__BandBox_LeftUp` is the real selected/tactical command dispatcher: it can call `Selection__DispatchMultiUnitOrder`, enqueue command packets, start action-line timers, and handle selection. Therefore this call must not be ignored, but it must not be modeled as "selected units get minimap move orders" because the normal selected-object branch has already jumped away before it.

Evidence: assembly context `0x00653CDF..0x00653D58`; decompile `DisplayClass__BandBox_LeftUp @ 0x004AB9B0`.

### 3.6 Camera update occurs inside the RadarClass handler, not only through the tiny wrapper

Active in YR: Yes.

The previous partial report identified `FUN_00653F70 -> FUN_006D6070` as a camera setter wrapper. This remains true, but ordinary radar input also calls `FUN_006D6070` directly from the unnamed handler:

```text
0x00653E66 lea ecx, [esp+0x48]
0x00653E6A lea edx, [esp+0x18]
0x00653E75 call 0x005657A0
0x00653E7C call 0x00480A30
0x00653EA0 call FUN_006D6070
0x00653EAC call FUN_004F42F0(1)
```

This happens after the handler clamps/adjusts the radar-derived cell against map limits. Exact clamp formulas remain delegated to the prior inverse-transform report; this report only proves ownership and ordering.

Evidence: assembly context `0x00653D62..0x00653EA0`; decompile `FUN_00653F70 @ 0x00653F70`.

## 4. INI Keys

No INI key directly controls this input provenance. Selection state, active radar state, and special targeting state are runtime globals.

## 5. Integration Points

| Function / address | Role | Evidence | Active in YR |
|---|---|---|---|
| `RadarClass__constructor @ 0x00652960` | installs `vtable_RadarClass` | decompile | Yes |
| unnamed `RadarClass` handler `0x006539D0` | ordinary radar input owner; bounds radar clicks, classifies selected objects, routes to camera/order branches | vtable data at `0x007F0338`; assembly context | Yes |
| `GScreenClass__Input @ 0x004F4320` | vtable-dispatched screen input | decompile; vtable data at `0x007F0368` | Yes |
| `FUN_006930A0` | Win32 mouse-message tactical/radar guard path | xref from `0x00777640`; decompile | Yes |
| `FUN_0063A5A0` / `FUN_0063A8E0` | left down/up radar/planning suppression helpers for tactical path | decompile and xrefs from `FUN_006930A0` | Yes |
| `FUN_00639DA0` | selected-object classifier that gates selected radar clicks away from order dispatch | decompile; call at `0x00653CC9` | Yes |
| `DisplayClass__BandBox_LeftUp @ 0x004AB9B0` | real command/selection dispatcher; present in handler but not on normal selected-radar branch | decompile; call at `0x00653D58` | Conditional |
| `FUN_006D6070` | immediate camera viewport setter | prior report; direct call at `0x00653EA0` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta |
|---|---|---|
| `src/app_input.rs::handle_mouse_input` | on left press, calls `handle_sidebar_mouse_input`, then `try_begin_minimap_drag` before tactical selection | broad shape is plausible, but does not model native RadarClass branch order |
| `src/app_sidebar_render.rs::try_begin_minimap_drag` | checks minimap hit, then calls `minimap_move_order_if_selected` first | mismatch: native selected-object radar path jumps away from `BandBox_LeftUp` command dispatch |
| `src/app_sidebar_render.rs::minimap_move_order_if_selected` | selected non-structure entities receive Move or AttackMove from minimap click | mismatch for ordinary live minimap selected-unit clicks |
| `src/app_sidebar_render.rs::update_camera_from_minimap_cursor` | sets Rust camera through current app camera clamp | partial/mismatch risk versus native `FUN_006D6070` and `FUN_006D8640` |
| `src/render/minimap.rs::camera_top_left_for_screen_point_in_rect` | uses Rust minimap texture-space mapping | mismatch versus native radar surface origin/size globals and radar inverse |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Radar input vtable owner | verified | `RadarClass__constructor @ 0x00652960`; `read_memory 0x007F0320`; `0x007F0338 -> 0x006539D0` | none |
| Base `GadgetClass` as owner | verified-negative for ordinary radar click | no `GadgetClass` callback needed; RadarClass vtable entry owns the path | other sidebar controls remain GadgetClass-owned |
| Win32 left down/up suppression | verified | `FUN_006930A0`, `FUN_0063A5A0`, `FUN_0063A8E0` | none for this slice |
| Radar bounds globals | verified | `0x00653A4D..0x00653A83`; xrefs to `DAT_00880C84/88/8C/90` | exact write sites delegated to rendering reports |
| Selected-object classifier | verified | `FUN_00639DA0`; call/branch at `0x00653CC9..0x00653D62` | semantic name of vtable `+0x4BC` remains unresolved, but branch result is enough for normal selected-object bypass |
| `BandBox_LeftUp` order path in radar handler | verified | `0x00653D58`; decompile `0x004AB9B0` | exact no-selection/special-mode fallthrough cases are out of scope |
| Camera update in handler | verified | `0x00653EA0`; prior `FUN_006D6070` report | formulas delegated to prior report |
| Runtime breakpoint log | deferred | debugger server unavailable | optional confirmation only |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is ordinary radar click owned by a base GadgetClass object? -> No for this path; the live ordinary radar input owner is an unnamed RadarClass vtable handler at 0x006539D0.` (evidence: `RadarClass__constructor @ 0x00652960`; `0x007F0338 -> 0x006539D0`)
- `[RESOLVED] OQ-02 - Is the handler active in YR? -> Yes; RadarClass is constructed by PowerClass and installs the vtable, and GScreen/Display dispatches through this family.` (evidence: `0x00652960`; `0x0063F6B4`; `0x004F4320`)
- `[RESOLVED] OQ-03 - Does the handler use radar-surface bounds? -> Yes; it reads `DAT_00880C84/88/8C/90` before `RadarClass__GetObjectAtRadarPixel`.` (evidence: `0x00653A4D..0x00653AB7`)
- `[RESOLVED] OQ-04 - Does the handler call the radar pixel/object reverse mapper? -> Yes.` (evidence: `0x00653AB7 -> 0x00656750`)
- `[RESOLVED] OQ-05 - Does the handler contain a tactical order dispatch call? -> Yes, `DisplayClass__BandBox_LeftUp` is called at `0x00653D58`.` (evidence: `0x00653D58`; `0x004AB9B0`)
- `[RESOLVED] OQ-06 - Do normal selected objects reach that order call from radar click? -> No; with radar active, non-`-1` `FUN_00639DA0` result jumps to `0x00653D62`, before the order fallthrough.` (evidence: `0x00653CC0..0x00653D62`; `0x00639DA0`)
- `[RESOLVED] OQ-07 - Does selected-unit minimap click pan/set camera? -> Yes for the verified selected-object path; camera update is reached through the `0x00653D62` branch and direct `0x00653EA0 -> 0x006D6070` call.` (evidence: `0x00653D62..0x00653EA0`)
- `[RESOLVED] OQ-08 - Is `FUN_00653F70` still relevant? -> Yes, but it is a wrapper/vtable entry, not the only ordinary click camera route.` (evidence: decompile `0x00653F70`; data xref `0x007F0414`)
- `[RESOLVED] OQ-09 - Does the Win32 left-up tactical path also skip orders when radar helper consumes the event? -> Yes; `FUN_006930A0` only calls `BandBox_LeftUp` when `FUN_0063A8E0` returns zero and capture state is active.` (evidence: `0x006930A0`)
- `[RESOLVED] OQ-10 - Is current Rust selected minimap move-order precedence native? -> No; Rust calls `minimap_move_order_if_selected` before starting camera drag.` (evidence: `src/app_sidebar_render.rs`)
- `[DEFERRED] OQ-11 - What exactly is the virtual `+0x4BC` predicate in `FUN_00639DA0`?` (category: bounded-cost-too-high; reason: not needed to prove the selected-object bypass branch; next-step-if-pursued: inspect Unit/Infantry/Building vtables at `+0x4BC` and name the predicate)
- `[DEFERRED] OQ-12 - Runtime breakpoint log for one selected-unit minimap click.` (category: needs-runtime-debugger; reason: debugger server was unavailable; next-step-if-pursued: set breakpoints on `0x00653D58` and `0x00653EA0` with one selected Grizzly and click radar)

## 9. Visual/UI Composition Ledger

This is an input provenance report. It has no new paint composition claim.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| input-1 | unnamed `RadarClass` handler `0x006539D0` | RadarClass vtable entry `0x007F0338` | none | radar globals `DAT_00880C84/88/8C/90` | none | Yes | input owner |
| input-2 | `RadarClass__GetObjectAtRadarPixel @ 0x00656750` | called at `0x00653AB7` after bounds pass | none | radar-surface pixel | none | Yes | object/cell lookup |
| input-3 | `FUN_006D6070 @ 0x006D6070` | direct call at `0x00653EA0` | none | tactical viewport | none | Yes | camera set |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Ordinary minimap click with normal selected objects routes to radar camera/cursor branch, not selected-unit command dispatch | `0x00653CC0..0x00653D62`; `0x00639DA0`; `0x00653EA0`; `0x004AB9B0` | mismatch: Rust calls `minimap_move_order_if_selected` first | `src/app_sidebar_render.rs::try_begin_minimap_drag`, `minimap_move_order_if_selected` | Remove ordinary selected-unit Move/AttackMove precedence from minimap clicks; treat selected-unit minimap left click/drag as camera control | `minimap_left_click_with_selection_pans_camera_not_move_order`: select one mobile unit, left-click minimap, assert camera changes and no Move/AttackMove is queued | Do not infer from the `0x00653D58` call that selected minimap clicks issue orders; the selected-object branch jumps before it |
| Radar input ownership is `RadarClass` vtable handler, not a base `GadgetClass` button | `0x00652960`; `read_memory 0x007F0320`; `0x007F0338 -> 0x006539D0` | Rust has bespoke egui/sidebar hit handling | future native input routing around `src/app_input.rs` and sidebar/minimap hit tests | Model ordinary minimap input as a radar surface owned by display/radar input routing, not as a sidebar command button | `minimap_input_consumes_tactical_click_before_selection`: click inside minimap while over tactical selectable area, assert tactical selection path does not run | Do not use base GadgetClass hit-test/list order as proof for ordinary minimap content clicks |
| Camera update from radar handler calls `FUN_006D6070` directly after selected-object bypass and local cell adjustment | `0x00653D62..0x00653EA0`; prior clamp report | partial: Rust camera mapping/clamp is texture/app based | `src/app_sidebar_render.rs::update_camera_from_minimap_cursor`, `src/render/minimap.rs::camera_top_left_for_screen_point_in_rect` | Use native radar surface origin/size and native camera setter/clamp semantics for click/drag | `minimap_selected_click_uses_native_radar_camera_setter`: selected unit click at radar corner saturates to native camera min/max with no order | Do not animate through edge scroll or use generic playable-area clamp as parity without proof |

### Negative Facts / Do Not Do

- Do not describe ordinary in-game minimap clicks as base `GadgetClass` button clicks. The live owner is `RadarClass` vtable entry `0x007F0338 -> 0x006539D0`.
- Do not keep Rust's selected-unit minimap Move/AttackMove precedence as native parity. The verified selected-object branch bypasses `DisplayClass__BandBox_LeftUp`.
- Do not delete/ignore the `0x00653D58` `BandBox_LeftUp` call entirely. It exists, but it is not reached by the normal selected-object radar branch.
- Do not claim `FUN_00653F70` is the only radar camera route. Ordinary radar input directly calls `FUN_006D6070` at `0x00653EA0`.
- Do not use base `GadgetClass` half-open hit-test/list ordering as the minimap content click proof; use the radar bounds globals and `RadarClass__GetObjectAtRadarPixel`.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game/docs/research/MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM_GHIDRA_REPORT.md`: replace "Status: PARTIAL, because exact `GadgetClass` button-event provenance remains unresolved" with "Status: COMPLETE for ordinary in-game minimap input provenance: the path is an unnamed `RadarClass` vtable handler at `0x006539D0` (`vtable_RadarClass+0x18` via data entry `0x007F0338`), not a separate base `GadgetClass` button. The selected-object branch jumps through `FUN_00639DA0` to the camera path before the `DisplayClass__BandBox_LeftUp` order call."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/MouseClass_research.md`: replace "slot +0x074 `0x00654490` Radar_ClickHandler" with "`0x00654490` is `RadarClass__ComputeRadarMapBounds`; ordinary in-game radar click/input ownership is the unnamed `RadarClass` vtable handler at `0x006539D0`, with radar cell/object lookup at `0x00656750` and camera set through direct `0x00653EA0 -> 0x006D6070`."
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/ScrollClass_research.md`: replace wording that implies the remaining click owner is unresolved with "Ordinary radar clicks are handled by `RadarClass` vtable entry `0x007F0338 -> 0x006539D0`; selected-object clicks route to camera/cursor handling through `FUN_00639DA0` before the `BandBox_LeftUp` order branch."

## Sources

- Ghidra read-only decompile/assembly: `0x00652960`, `0x006539D0`, `0x00653CC0..0x00653EA0`, `0x00639DA0`, `0x004AB9B0`, `0x006930A0`, `0x0063A5A0`, `0x0063A8E0`, `0x0063AAC0`, `0x0063AB00`, `0x00637AA0`, `0x00653F70`, `0x006D6070`, `0x004F4320`.
- Ghidra read-only memory: `read_memory 0x007F0320`, `get_bulk_xrefs 0x006539D0`, `get_bulk_xrefs 0x00653F70`, `get_bulk_xrefs 0x004AB9B0`.
- Prior docs: `MINIMAP_CLICK_DRAG_INVERSE_TRANSFORM_GHIDRA_REPORT.md`, `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`, `RADAR_EVENT_CLASS_GHIDRA_REPORT.md`, `SELECTION_SYSTEM_GHIDRA_REPORT.md`, `MouseClass_research.md`, `ScrollClass_research.md`.
- Current Rust scan: `src/app_input.rs`, `src/app_sidebar_render.rs`, `src/render/minimap.rs`.

## Status

COMPLETE. Static vtable and branch evidence closes the prior provenance gap for ordinary in-game minimap input. Runtime breakpoint logging would be a useful audit, but it is not required for the Rust-facing conclusion: normal selected-unit minimap clicks should pan/set the camera and must not enqueue Move/AttackMove orders.
