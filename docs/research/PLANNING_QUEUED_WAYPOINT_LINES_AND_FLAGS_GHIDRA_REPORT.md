# Planning / Queued Waypoint Lines and Flags - Ghidra Research Report

**Address(es):** `0x006DAD60` (waypoint path overlay), `0x00763980` / `0x00763BA0` (WaypointPath point helpers), `0x0073CEC0` / `0x0073D3E7` (UnitClass `FLAGFLY.SHP` draw site), `0x006DBB60` (`Tactical::DrawLine3D`, checked as non-caller for this slice)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Planning-mode and queued waypoint line rendering, whether all queued points are connected, relation to `NavQueue`, planning/Shift queue gating, and waypoint/flag markers.  
**Non-Scope:** Factory rally line details, exact writers of every path field, and full command execution semantics.  
**Confidence:** High for render path and segment iteration; Medium for the exact user-facing names of several display globals.  
**Active in YR:** Conditional. Active in standard tactical draw when a non-empty `WaypointPathClass` exists; skipped in map editor and filtered by shroud/viewport gates.

## 1. Overview

The planning/queued waypoint overlay is not the selected-unit target line and is not drawn by `Tactical::DrawLine3D`. The active tactical renderer is `FUN_006DAD60`, called twice from `TacticalClass_Draw`, and it walks `WaypointPathClass` point arrays stored off the player/house object.

The most important result: queued/planning path lines are drawn **through every adjacent stored waypoint**, not only from the unit to the final endpoint. If the path has a loop index, the last point can connect back to that loop index.

## 2. Key Offsets

| Owner | Offset | Purpose | Active in YR |
|---|---:|---|---|
| House/player | `+0x20C` | Current waypoint path index, valid `0..11`, `-1` none | Yes, evidence `0x005090F0`, `0x004AC700` |
| House/player | `+0x210 + index*4` | `WaypointPathClass*` array, 12 possible paths | Conditional, evidence `0x006DAD60`, `0x00504740` |
| WaypointPathClass | `+0x24` | Loop/closure index, `-1` means no loop | Conditional, evidence `0x00763BA0` |
| WaypointPathClass | `+0x2C` | 12-byte `CoordStruct` point array | Yes, evidence `0x00763980` |
| WaypointPathClass | `+0x38` | Active point count | Yes, evidence `0x006DAD60`, `0x00763980` |
| FootClass | `+0x520` | Assigned waypoint path id for a following unit | Conditional, evidence `0x004DC840`, `0x004D8420` |
| FootClass | `+0x686` | Current waypoint node index for a following unit | Conditional, evidence `0x004DC840`, `0x004D8E25` |
| UnitClass | `+0x6CC` | Gate for `FLAGFLY.SHP` draw block; draws when not `-1` | Conditional, evidence `0x0073D395..0x0073D408` |
| DisplayClass | `+0x11B3` | Planning-mode display flag | Conditional, evidence `0x004AC700` |
| DisplayClass | `+0x11BC` | Live planning hover-preview `CoordStruct*` | Conditional, evidence `0x004AAF00` |

## 3. Core Logic

### `FUN_006DAD60` waypoint overlay

`TacticalClass_Draw @ 0x006D3D10` calls the overlay twice:

- `0x006D463F`, with pushed argument `0`, before object rendering.
- `0x006D46C6`, with pushed argument `1`, after unit-action visuals/bandbox and before radar overlays.

Active in YR: Conditional. The function returns early in map editor and only draws paths whose `WaypointPathClass.Count > 0`.

Inside `FUN_006DAD60`, the renderer:

1. Computes line phase `(0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`.
2. Loads `MOUSE.SHA` into `DAT_00B0E644` on demand (`0x006DAE02`, string `0x0082604C`).
3. Iterates path slots `0..11` from `g_PlayerPtr + 0x210 + slot*4`.
4. Iterates every node `0..Count-1`.
5. Projects each node with `Tactical__WorldToScreenSub` and `Tactical__AdjustForZ`.
6. Draws a node marker if viewport and shroud tests pass.
7. Calls `FUN_00763BA0(current_point)` to fetch the next point.
8. Draws the segment from current to next if the next point exists and the line clips into the viewport.

`FUN_00763980(path, index)` returns `path->Items + index * 0xC` only for `0 <= index < Count`; otherwise it returns null. Active in YR: Yes, evidence `0x00763980`.

`FUN_00763BA0(path, current_point)` finds the current point index, adds one, and when it reaches `Count` it wraps to `path+0x24` only if that loop index is not `-1`. Otherwise it returns null. Active in YR: Conditional, evidence `0x00763BA0`.

### Style and markers

The tactical planning path marker is **not `FLAGFLY.SHP`**. `FUN_006DAD60` loads `MOUSE.SHA` and uses mouse action-table index `0x3C`; `FUN_005BE970(0x3C)` / `FUN_005BE990(0x3C)` read start/count from the mouse cursor table. Memory at `0x0082D6B8` gives start frame `0x180`, count `1`, so this marker is static in the verified table.

Line dashing uses `DAT_00842940`, beginning `01 01 01 01 01 00 00 00 ...` (five on, three off repeated). The local-player path branch calls primary-surface vtable `+0x4C` with that pattern and phase; the non-local branch calls vtable `+0x50`. Active in YR: Conditional, evidence `0x006DB155`, memory `0x00842940`, surface vtable entries resolving to `0x004C0750` and `0x004C0E30`.

The two calls from `TacticalClass_Draw` split shroud behavior. With pushed `0`, the node must be unshrouded; with pushed `1`, the node must be shrouded. Both passes also apply broad viewport checks. Active in YR: Conditional, evidence `0x006D463F`, `0x006D46C6`, `0x006DAEE0..0x006DAF55`.

### Planning / queue predicate

`FUN_00731BF0` is the verified queue/planning predicate. It returns true immediately if `g_SelectionSubMode != 0`; otherwise it checks two key-pair groups through `FUN_0054F5C0(DAT_00A8EC00/04)` and `FUN_0054F5C0(DAT_00A8EC08/0C)`, and requires all selected objects with object bit `+0x14 & 1` to pass vtable `+0x4C0`. Active in YR: Conditional, evidence `0x00731BF0`, callers `0x006FFBEC`, `0x0070F0B3`, planning command `0x00731AF0`.

### Relation to `NavQueue`

This overlay does not use `FootClass::NavQueue`. Selected-unit target lines use `NavQueue.Items[Count - 1]` as the final movement endpoint in `TechnoClass::DrawActionLines`; planning paths use `WaypointPathClass` and draw all adjacent points. Active in YR: Conditional, evidence `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, `0x006DAD60`.

`Tactical::DrawLine3D @ 0x006DBB60` was checked and is not the verified queued/planning waypoint renderer. In this slice its xref is the tactical vtable data entry (`0x007F43A8`), while actual planning path calls are direct `FUN_006DAD60` calls from `TacticalClass_Draw`. Active in YR for queued/planning paths: No, evidence `get_bulk_xrefs(0x006DBB60)`, `0x006D463F`, `0x006D46C6`.

### `FLAGFLY.SHP`

`FLAGFLY.SHP` string `0x008458F8` has a verified draw site in `UnitClass::DrawExtras @ 0x0073CEC0`, gated by `UnitClass + 0x6CC != -1`. When active, it computes `g_CurrentFrameCounter % 0xE` and draws with CC flags `0xA00`. Active in YR: Conditional, evidence `0x0073D395..0x0073D408`.

This is separate from `FUN_006DAD60` planning path markers, which use `MOUSE.SHA`. The prior context grouping waypoint path markers under `FLAGFLY.SHP` was too broad.

## 4. INI Keys

| Section | Key | Default | Binary read/use | Active in YR |
|---|---|---:|---|---|
| `[General]` | `MaxWaypointPathLength` | `15` | Read into `RulesClass + 0x90`; `FUN_005090F0` allows adding only when `Count < Rules+0x90` and no loop is set | Yes, evidence `ini/rulesmd.ini:424`, `0x00671DBF`, `0x005090F0` |
| `[AudioVisual]` | `WaypointAnimationSpeed` | `10` | Read into `RulesClass + 0x50`; no direct use found in `FUN_006DAD60` or the verified `FLAGFLY` draw block | Read active; consumer deferred. Evidence `ini/rulesmd.ini:670`, `0x006692D4` |
| `[AudioVisual]` | `StartPlanningModeSound` | `PlanningModeStart` | Read into `RulesClass + 0x1D4`; not render logic | Read active, evidence `ini/rulesmd.ini:630`, `0x006696E0..0x00669737` |
| `[AudioVisual]` | `EndPlanningModeSound` | `PlanningModeEnd` | Read into `RulesClass + 0x1E0`; not render logic | Read active, evidence `ini/rulesmd.ini:631`, `0x00669716` |
| `[AudioVisual]` | `AddPlanningModeCommandSound` | `PlanningModeAdd` | Read into `RulesClass + 0x1D8`; not render logic | Read active, evidence `ini/rulesmd.ini:632`, `0x00669B31` |

## 5. Integration Points

| Integration | Status | Evidence | Active in YR |
|---|---|---|---|
| Tactical draw invokes waypoint overlay twice | verified | `0x006D463F`, `0x006D46C6` | Conditional |
| Path slot creation on demand | verified | `FUN_00504740`, constructor `0x00763810` | Conditional |
| Current path addability/max length | verified | `0x005090F0`, `Rules+0x90` | Conditional |
| Cursor recognizes existing waypoint nodes | verified | `0x006928A2`, `0x00692903` | Conditional |
| Cursor hover updates live preview coordinate | verified | `0x004AAF00` | Conditional |
| Units can follow a waypoint path id/node | verified | `0x004DC840`, `0x004D8420`, `0x004D8E25` | Conditional |

## 6. Rust Implementation Status

This slot did not modify Rust. Source scan found visible target-line work in `src/app_target_lines.rs`, but no equivalent `WaypointPathClass` overlay, all-segment planning path renderer, `MOUSE.SHA` waypoint marker renderer, or `FLAGFLY.SHP` unit marker path in the scanned Rust results.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006DAD60` path overlay | verified | `0x006DAD60`, callers `0x006D463F`, `0x006D46C6` | none for segment iteration |
| All-segments vs final endpoint | verified | `0x006DAE85`, `0x006DB02A`, `0x00763BA0` | none |
| `Tactical::DrawLine3D` relation | verified | `0x006DBB60`, data xref `0x007F43A8` | rally line callers are separate |
| `WaypointPathClass` point lookup | verified | `0x00763980` | none |
| Loop closure | verified | `0x00763BA0` | writer of loop index non-scope |
| `MaxWaypointPathLength` read/use | verified | `0x00671DBF`, `0x005090F0`, `ini/rulesmd.ini:424` | none |
| Planning predicate | verified | `0x00731BF0`, callers `0x006FFBEC`, `0x0070F0B3` | exact key names deferred |
| `MOUSE.SHA` path marker | verified | `0x006DAE02`, `0x0082604C`, `0x0082D6B8` | none |
| `FLAGFLY.SHP` draw site | verified | `0x0073D3E7`, gate `Unit+0x6CC != -1` | writer/source of `+0x6CC` deferred |
| `WaypointAnimationSpeed` visual consumer | touched-not-exhausted | read at `0x006692D4`; no direct use in verified draw blocks | needs Rules+0x50 consumer trace |

## 8. Open Questions - Final State

[RESOLVED] OQ-PQW-001 - Does `Tactical::DrawLine3D @ 0x006DBB60` draw planning/queued waypoint segments? No verified queued/planning caller uses it; `FUN_006DAD60` is the active path renderer. Evidence: `0x006D463F`, `0x006D46C6`, `0x006DAD60`, `get_bulk_xrefs(0x006DBB60)`. Active in YR: No for this purpose.

[RESOLVED] OQ-PQW-002 - Are lines drawn through all queued waypoints or only final endpoints? All adjacent stored path nodes are processed, with optional loop closure. Evidence: `0x006DAD60`, `0x00763980`, `0x00763BA0`. Active in YR: Conditional.

[RESOLVED] OQ-PQW-003 - Is this the same as FootClass `NavQueue` target-line rendering? No. Action lines use final `NavQueue` endpoint; planning overlay uses House/player `WaypointPathClass` objects. Evidence: `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, `0x006DAD60`. Active in YR: Conditional.

[RESOLVED] OQ-PQW-004 - Are tactical waypoint path markers `FLAGFLY.SHP`? No for `FUN_006DAD60`: it loads `MOUSE.SHA`; `FLAGFLY.SHP` is a separate `UnitClass::DrawExtras` block. Evidence: `0x006DAE02`, `0x0082604C`, `0x0073D3E7`. Active in YR: Conditional.

[RESOLVED] OQ-PQW-005 - What limits path length? `MaxWaypointPathLength`, default 15, read into `Rules+0x90`; addability requires `Count < Rules+0x90` and `LoopIndex == -1`. Evidence: `0x00671DBF`, `0x005090F0`, `ini/rulesmd.ini:424`. Active in YR: Yes.

[DEFERRED] OQ-PQW-006 - Which exact UI action sets `WaypointPathClass +0x24` loop index? Category: requires-different-system-context. Reason: loop use is verified, writer is outside this render-focused slot.

[DEFERRED] OQ-PQW-007 - Which exact writer sets `UnitClass +0x6CC` for the `FLAGFLY.SHP` block? Category: bounded-cost-too-high. Reason: draw output is verified; writer trace is separate unit-state work.

[DEFERRED] OQ-PQW-008 - Does `WaypointAnimationSpeed` affect a marker through an intermediate counter? Category: requires-different-system-context. Reason: read into `Rules+0x50` is verified, but neither verified draw block reads it directly.

## Sources

- Ghidra decompiled: `0x006DAD60`, `0x00763980`, `0x00763BA0`, `0x006DBB60`, `0x0073CEC0`, `0x00731BF0`, `0x004AAF00`, `0x004AC700`, `0x005090F0`, `0x004DC840`, `0x004D8420`, `0x004D8E25`, `0x005BE970`, `0x005BE990`
- Ghidra assembly/xrefs: `0x006D463F`, `0x006D46C6`, `0x006DAE02`, `0x006DB155`, `0x0073D3E7`, `0x00671DBF`, `0x006692D4`, `0x00669B31`, `0x00669716`
- Memory inspected: `0x0082604C` (`MOUSE.SHA`), `0x008458F8` (`FLAGFLY.SHP`), `0x00842940` dash pattern, `0x0082D6B8` mouse action `0x3C`, `0x0083B43C`, `0x0083AB88`
- INI checked: `ini/rulesmd.ini:424`, `:630`, `:631`, `:632`, `:670`; base `rules.ini` matching defaults
- Prior context checked: `docs/research/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`, `docs/research/TARGET_LINES_GHIDRA_REPORT.md`, `docs/research/TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`
