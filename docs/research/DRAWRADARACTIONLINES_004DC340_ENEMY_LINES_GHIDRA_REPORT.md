# DrawRadarActionLines 0x004DC340 Enemy Lines - Ghidra Research Report

**Address(es):** `TechnoClass::DrawRadarActionLines @ 0x004DC340`; caller `TacticalClass_Draw @ 0x006D3D10`; eligibility helper `FUN_0043B150`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** Enemy/non-human action-line draw path from the standard tactical render loop through line/dot rendering.
**Non-Scope:** Full Psychic Sensor cell-counter implementation, all cloak visibility systems, and low-level surface rasterization internals beyond the calls made here.
**Confidence:** High for call gates, target selection, color source, dots, dash timing, bridge adjustment, and `UnitActionLines` non-involvement.
**Active in YR:** Conditional. The code is in the standard YR tactical draw loop, but only runs for non-human FootClass technos that pass the psychic-detection helper and object-state gates.

## 1. Overview

`DrawRadarActionLines` draws a dashed tactical-screen line plus 3x3 endpoint dots for a non-human FootClass techno whose current target or movement destination is inside the local player's psychic-detection coverage. Despite the name, this is not the minimap renderer: it projects world coordinates through `TacticalClass::CoordsToClient2` and draws to `DAT_0088731C` using the radar/tactical viewport rectangle globals.

Material correction versus `TARGET_LINES_GHIDRA_REPORT.md`: the call is not simply "visible enemy + not cloaked" and not a `UnitActionLines` option path. It is gated by `FUN_0043B150`, which walks the local player's `HouseClass` list at `+0x12C/+0x138` for active psychic-detection-capable buildings and compares the enemy's ArchiveTarget/NavCom endpoint to `BuildingTypeClass+0x170C` (`PsychicDetectionRadius`).

## 2. Key Offsets / Globals

| Field / global | Purpose | Active in YR |
|---|---|---|
| Techno/Object `+0x14` bit `0x04` | FootClass-derived object gate; buildings do not draw this path | Yes, `0x006D4773..0x006D477C`; `ABSTRACTCLASS_GHIDRA_REPORT.md:139-146` |
| Object `+0x81` | Must be `0`; existing docs identify this as the placed/discovered state byte, not cloak state | Yes, `0x006D4782..0x006D478A`; `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:1077` |
| Techno `+0x21C` | Owner `HouseClass*`; used for human/allied gate and house-color read | Yes, `0x004DC4D4`, `0x0043B150` |
| Techno `+0x2B4` | `ArchiveTarget`; attack target has priority over movement | Yes, `0x004DC34F..0x004DC384` |
| Foot `+0x58C/+0x598` | NavQueue items/count; last queue entry wins over current NavCom | Yes, `0x004DC3E1..0x004DC410` |
| Foot `+0x5A4` | `NavCom`; fallback movement endpoint | Yes, `0x004DC372..0x004DC37A`, `0x004DC3F2` |
| House `+0x56F9..+0x56FB` | 3-byte RGB house color used for endpoint dots and line-color work buffer | Yes, `0x004DC4D4..0x004DC504` |
| `DAT_00822540` | 8-byte dash pattern, bytes `01 01 01 01 01 00 00 00` | Yes, read at `0x004DC703`; Ghidra memory `0x00822540` |
| `DAT_008B3DF4` | Added to ground height for bridge endpoint Z adjustment | Yes, NavCom branch only, `0x004DC483..0x004DC498` |
| `DAT_00843108` | Selected-human `DrawActionLines` option mirror; not read by enemy branch | No for this function; compare `0x006D4735..0x006D4750` vs `0x006D4764..0x006D478E` |

## 3. Core Logic

1. Standard call site: `TacticalClass_Draw @ 0x006D3D10` iterates `g_TechnoClass_Array` late in pass 2 after radar overlays and before capture-manager link drawing. Active in YR: Yes, standard tactical rendering path.
2. Human-player owned objects skip this function. The selected-human branch separately checks object `+0x83` selected and `DAT_00843108` before virtual `+0x438` `DrawActionLines`. Active in YR: Yes, `0x006D4735..0x006D4750`.
3. Non-human objects call `FUN_0043B150`; only if it returns nonzero, object pointer is non-null, `+0x14 & 0x04` is true, and `+0x81 == 0`, the caller invokes `0x004DC340`. Active in YR: Conditional, `0x006D4764..0x006D478E`.
4. `FUN_0043B150` first rejects allied houses, then iterates the local player's psychic-detection building list (`g_PlayerPtr+0x12C`, count `+0x138`). Each listed building must pass its vtable `+0x350` active/operational check. Active in YR: Conditional; retail `[NAPSIS]` has `PsychicDetectionRadius=15` in `rulesmd.ini:13353`.
5. The helper tests either the enemy's `ArchiveTarget` or, for FootClass objects, its final nav destination (`NavQueue[last]` if count > 0, else `NavCom`) against the detecting building's `BuildingTypeClass+0x170C` radius in leptons (`radius * 0x100`). Active in YR: Conditional, `FUN_0043B150`; parser evidence `0x00460C39..0x00460C46`.
6. Inside `DrawRadarActionLines`, `ArchiveTarget` wins. If `ArchiveTarget != 0`, the start coordinate is virtual `+0x300` fire coords and the endpoint is `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0`. The function does not also draw NavCom. Active in YR: Yes, `0x004DC384..0x004DC3C5`.
7. If there is no `ArchiveTarget`, `NavCom == 0` returns immediately; otherwise start is object `+0x9C/+0xA0/+0xA4`, and endpoint is last queued NavQueue item if count is nonzero, else current `NavCom`. Active in YR: Yes, `0x004DC372..0x004DC410`.
8. NavCom endpoints get a bridge Z correction: endpoint X/Y are reduced to cell coordinates by signed add-255 then shift-right-8; if in bounds and `CellClass+0x140 & 0x100`, endpoint Z becomes `CellClass::GetGroundHeight(coord) + DAT_008B3DF4`. Active in YR: Yes, `0x004DC42A..0x004DC498`.
9. Both endpoints are projected to tactical client pixels with `TacticalClass::CoordsToClient2 @ 0x006D2140`; both Y values receive `g_RadarViewportOffsetY`. Active in YR: Yes, `0x004DC49C..0x004DC4E8`.
10. The function draws a 3x3 rectangle at each projected endpoint, offset by `(-2, -2)` before clipping through `AlphaShapeClass::ClipRect @ 0x00421B60`, then uses surface vtable `+0x14`. Active in YR: Yes, `0x004DC4EC..0x004DC65E`.
11. The main segment is clipped to `{g_RadarViewportOffsetX, g_RadarViewportOffsetY, g_RadarViewportWidth, g_RadarViewportHeight}` by `FUN_007BC2B0`. If clipping rejects the segment, dots may already have drawn but the dashed line is skipped. Active in YR: Yes, `0x004DC661..0x004DC69B`.
12. Dashed-line timing uses wall clock, not game frames: `timeGetTime()` is masked with `0x3FF` for the intensity phase and `(-time >> 5) & 0xF` for dash phase, then calls surface vtable `+0x4C` with pattern `DAT_00822540` and final parameter `0`. Active in YR: Yes, `0x004DC6A1..0x004DC747`.
13. The intensity modulation path calls `FUN_006612C0`, a byte-wise RGB blend helper. The branch only blends during the low `0x200` half of the `timeGetTime() & 0x3FF` cycle; the `0x100` bit reverses the 0..255 amount with XOR `0xFF`. Active in YR: Yes, `0x004DC6A9..0x004DC6D1`; helper `0x006612C0`.
14. There is no timer check and no `g_ActionLines_StartFrame` / duration read in `0x004DC340`. Active in YR: Yes by absence in decompiled function and instruction range `0x004DC340..0x004DC752`.

## 4. INI Keys

| INI key | Location / default | Effect in this slice | Active in YR |
|---|---|---|---|
| `PsychicDetectionRadius=` | BuildingTypeClass `+0x170C`, default 0; read at `0x00460C39..0x00460C46` | Radius used by `FUN_0043B150` to decide whether enemy target/nav endpoints get radar action lines | Yes; `[NAPSIS] PsychicDetectionRadius=15` at `ini/rulesmd.ini:13353` |
| `SensorArray=` | BuildingTypeClass `+0x16C8` | Related Psychic Sensor capability/list ownership; not directly read inside `0x004DC340` | Yes; `[NAPSIS] SensorArray=yes` at `ini/rulesmd.ini:13375` |
| `SensorsSight=` | TechnoTypeClass `+0x5F0` | Not the radius read by `FUN_0043B150`; psychic action lines use `+0x170C` | Conditional/non-effect for this slice; `[NAPSIS] SensorsSight=15` at `ini/rulesmd.ini:13376` |
| `[Options] UnitActionLines=` | Option mirror `DAT_00843108` | Gates only selected-human `DrawActionLines`; not the enemy/psychic-detection branch | No for `DrawRadarActionLines`; call-site compare at `0x006D473F` vs enemy call at `0x006D478E` |

## 5. Integration Points

`DrawRadarActionLines` is called only from `TacticalClass_Draw @ 0x006D3D10` in this database (`get_function_callers` result). The call is in the same late TechnoClass overlay loop as selected action lines, capture-manager links, and service/tether lines.

Rust currently has selected-command target lines in `src/app_target_lines.rs`, including a 25-tick timer and selected-source filter. It does not model this psychic-detection enemy line path, house-color endpoint dots, wall-clock dash modulation, or `PsychicDetectionRadius`-based eligibility. Evidence: `src/app_target_lines.rs:1`, `src/app_target_lines.rs:19`, `src/app_target_lines.rs:138`; repo search found no `DrawRadarActionLines`/radar action-line implementation.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::DrawRadarActionLines @ 0x004DC340` | verified | Decompiled and disassembled `0x004DC340..0x004DC752` | none for this slice |
| Standard call site | verified | `TacticalClass_Draw @ 0x006D3D10`; xref from `0x006D478E` | none for this slice |
| Eligibility helper `FUN_0043B150` | verified for line eligibility | Decompiled `0x0043B150`; caller at `0x006D4764` | Full HouseClass list maintenance is out-of-scope |
| Target-vs-nav priority | verified | `0x004DC34F..0x004DC410` | none |
| House-color/dot/dash path | verified | `0x004DC4D4..0x004DC747`; `DAT_00822540` memory read | exact display-pixel RGB depends on runtime display format |
| Bridge adjustment | verified | `0x004DC42A..0x004DC498` | none for NavCom branch |
| `UnitActionLines` option effect | verified negative | Selected-human branch `0x006D4735..0x006D4750`; enemy branch `0x006D4764..0x006D478E` | none |

## 7. Open Questions - Final State

[RESOLVED] OQ-DRAL-001 - Is the path active in standard YR? Yes, conditionally; standard `TacticalClass_Draw` calls it at `0x006D478E`, and retail `[NAPSIS]` sets `PsychicDetectionRadius=15` (`rulesmd.ini:13353`).

[RESOLVED] OQ-DRAL-002 - Does `UnitActionLines` affect it? No; `DAT_00843108` is checked only in the selected-human branch before virtual `+0x438`, not before `DrawRadarActionLines`.

[RESOLVED] OQ-DRAL-003 - Does ArchiveTarget or NavCom win? ArchiveTarget wins and returns; NavCom is used only when ArchiveTarget is null.

[RESOLVED] OQ-DRAL-004 - Are dots drawn? Yes, 3x3 clipped rectangles at both endpoints offset by `(-2,-2)`.

[RESOLVED] OQ-DRAL-005 - Is there an explicit cloak-state gate? No direct cloak-state field gate was found in the caller or `0x004DC340`; eligibility is through psychic-detection helper plus object state `+0x81 == 0`.

[DEFERRED] OQ-DRAL-006 - Exact runtime RGB after display-format conversion and blend on all render modes. Category: requires-runtime-debugger. Reason: binary shows source bytes and shift/loss globals, but final pixel value depends on active display format globals.

## Sources

- Ghidra: `TechnoClass::DrawRadarActionLines @ 0x004DC340`
- Ghidra: `TacticalClass_Draw @ 0x006D3D10`, xref call `0x006D478E`
- Ghidra: `FUN_0043B150`, `FUN_0045A130`, `AlphaShapeClass::ClipRect @ 0x00421B60`, `FUN_007BC2B0`, `FUN_006612C0`, `TacticalClass::CoordsToClient2 @ 0x006D2140`, `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0`
- Ghidra memory: `0x00822540` bytes `01 01 01 01 01 00 00 00`
- INI: `ini/rulesmd.ini:13342`, `:13353`, `:13375`, `:13376`
- Docs checked: `TARGET_LINES_GHIDRA_REPORT.md`, `building-selection-brackets/SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `BUILDINGCLASS_CLOAK_SENSOR_GHIDRA_REPORT.md`, `ABSTRACTCLASS_GHIDRA_REPORT.md`, `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`
