# Psychic Sensor Enemy Action Lines Recheck - Ghidra Research Report

**Address(es):** `TechnoClass__DrawRadarActionLines @ 0x004DC340`, eligibility helper `FUN_0043B150`, caller `TacticalClass_Draw @ 0x006D3D10`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Re-check the standard tactical render path that draws non-human FootClass action lines when Psychic Detection eligibility passes.
**Non-Scope:** Psychic Sensor building-list maintenance, full cloak/sensor visibility model, minimap rendering, selected-unit action-line internals beyond negative comparisons, and mind-control/service line geometry.
**Confidence:** High for entry/caller, eligibility gates, endpoint priority, color/dots/dash, option/timer non-involvement, and draw order.
**Active in YR:** Conditional. The call is in standard `TacticalClass_Draw`, but only non-human FootClass technos that pass `FUN_0043B150` plus object-state gates draw; standard YR content enables the feature through `[NAPSIS] PsychicDetectionRadius=15`.

## 1. Overview

`DrawRadarActionLines` is tactical-screen output despite its name. It projects world coordinates with `TacticalClass__CoordsToClient2 @ 0x006D2140`, offsets by tactical/radar viewport globals, clips against the tactical viewport rectangle, and draws to `DAT_0088731C`, the same render surface used by other tactical overlays.

The player-visible effect is a dashed line and two 3x3 endpoint dots exposing an enemy/non-human FootClass object's attack target or final movement destination when that endpoint is inside the local player's Psychic Detection coverage. It is not a minimap line, not generic enemy intent, not selected-unit command feedback, and not controlled by `[Options] UnitActionLines`.

## 2. Key Offsets / Globals

| Field / global | Meaning in this slice | Active in YR |
|---|---|---|
| `Techno+0x21C` | Owner `HouseClass*`; helper rejects allies and renderer reads owner RGB from this house | Yes, `FUN_0043B150`; `0x004DC4D4..0x004DC4DE` |
| `Object+0x14` bit `0x04` | FootClass-derived gate in caller and movement-endpoint branch in helper | Conditional, `0x006D4773..0x006D4780`, `FUN_0043B150` |
| `Object+0x81` | Must be zero at caller before drawing | Conditional, `0x006D4782..0x006D478A` |
| `Techno+0x2B4` | `ArchiveTarget`; takes priority over movement endpoint | Yes, `0x004DC34F..0x004DC3C5` |
| `Foot+0x58C/+0x598` | NavQueue pointer/count; last queued item wins when count > 0 | Conditional, `0x004DC3E1..0x004DC410` |
| `Foot+0x5A4` | `NavCom`; fallback movement target; null returns without drawing | Conditional, `0x004DC372..0x004DC37A`, `0x004DC3F2` |
| `BuildingType+0x170C` | Parsed `PsychicDetectionRadius`, compared after multiplying by `0x100` | Conditional, `0x00460C39..0x00460C46`, `FUN_0043B150` |
| `House+0x56F9..0x56FB` | Owner house RGB source for dots/line color work buffer | Yes, `0x004DC4D4..0x004DC504` |
| `DAT_00822540` | Dash pattern bytes `01 01 01 01 01 00 00 00` | Yes, pushed at `0x004DC703`; memory read confirms repeated pattern |
| `DAT_00843108` | `UnitActionLines` option mirror | No for this path; only xrefs are write `0x0070D180` and read `0x006D473F` |
| `g_ActionLines_StartFrame`, `g_ActionLines_Duration` | Selected-unit action-line timer globals | No for this path; xrefs are selected-line/timer/save-load code, not `0x004DC340` |

## 3. Core Logic

### 3.1 Tactical entry and draw order

`TacticalClass_Draw @ 0x006D3D10` reaches this overlay only in the pass-2/pass-3 tactical overlay section after object rendering, lasers, electric bolts, line trails, rad beams, `Tactical__DrawUnitActionVisuals`, bandbox drawing, placement overlays, and optional radar-overlay setup. Active in YR: Yes, decompile shows the line-family loop after those calls.

Inside the per-techno loop:

1. `HouseClass__IsHumanPlayer` splits human-player objects from non-human objects. Active in YR: Yes, decompile around the loop and assembly `0x006D4720..0x006D4764`.
2. Human-player selected-unit lines require selected byte `+0x83` and `DAT_00843108`, then call vtable `+0x438` with two zero args. Active in YR: Conditional; evidence `0x006D4735..0x006D4750`.
3. Non-human objects call `FUN_0043B150`; if true, caller also requires non-null pointer, `Object+0x14` bit `0x04`, and `Object+0x81 == 0`, then calls `TechnoClass__DrawRadarActionLines`. Active in YR: Conditional; evidence `0x006D4764..0x006D478E`.
4. After this branch, if the radar-overlay gate local is false, the common link/tether block runs: `CaptureManagerClass__DrawLinks @ 0x00472160` at `0x006D47BF` / `0x006D47F6`, then service/tether helper `0x00705860` at `0x006D48F1`. Active in YR: Conditional; draw order evidence `0x006D478E` before `0x006D47BF`, `0x006D47F6`, `0x006D48F1`.

### 3.2 Psychic Detection eligibility helper

`FUN_0043B150` first rejects allied houses by calling `HouseClass__IsAlliedWith(g_PlayerPtr, target_owner)`. Active in YR: Yes; evidence `FUN_0043B150` and caller `0x006D4766`.

For enemies, it iterates the local player's list at `g_PlayerPtr+0x12C` with count `g_PlayerPtr+0x138`. Each listed building must pass vtable `+0x350` before it can detect. Active in YR: Conditional; evidence `FUN_0043B150`.

The helper tests two possible endpoint families:

- `ArchiveTarget` path: if `Techno+0x2B4` is non-null, object targets whose type virtual `+0x2C` returns `0x0B` are compared from the detecting building coordinate to the target cell center; coordinate-bearing targets with `target+0x14 bit 0` set are compared to the target's virtual `+0x48` coordinate. Active in YR: Conditional; evidence `FUN_0043B150` branch around ArchiveTarget.
- Movement path: if the candidate is FootClass-like (`Object+0x14 bit 0x04`), use `NavQueue[Count - 1]` when `Foot+0x598 > 0`, otherwise `NavCom` at `Foot+0x5A4`; null endpoint fails. Object/cell endpoint cases are tested similarly. Active in YR: Conditional; evidence `FUN_0043B150` movement branch and renderer `0x004DC3E1..0x004DC410`.

In all successful cases, distance must be `<= BuildingType+0x170C * 0x100`. Active in YR: Conditional; evidence helper comparison and parser xref `PsychicDetectionRadius` string `0x0081A960` read at `0x00460C39`, stored to `BuildingType+0x170C` at `0x00460C46`.

### 3.3 Endpoint choice in renderer

`DrawRadarActionLines` repeats the visible endpoint priority: `ArchiveTarget` wins. If `Techno+0x2B4` is non-null, start coords come from vtable `+0x300` with argument `0`, endpoint comes from `TechnoClass__Resolve_ArchiveTarget_Coords @ 0x0070BCB0`, and the movement branch is skipped. Active in YR: Yes/Conditional; evidence `0x004DC34F..0x004DC3C5`.

If `ArchiveTarget` is null, `NavCom == 0` returns immediately. Otherwise the start is the object stored coordinate at `+0x9C/+0xA0/+0xA4`, and endpoint uses the last queued NavQueue item when count is nonzero, else `NavCom`. Active in YR: Conditional; evidence `0x004DC372..0x004DC410`.

Movement endpoints apply bridge Z correction: endpoint X/Y are converted to cells by signed add-255 then `SAR 8`; if the cell is in bounds and `Cell+0x140 & 0x100`, endpoint Z becomes `CellClass__GetGroundHeight + DAT_008B3DF4`. Active in YR: Conditional; evidence `0x004DC42A..0x004DC498`.

### 3.4 Pixel style, color, and dash

Both endpoints are projected through `TacticalClass__CoordsToClient2 @ 0x006D2140`, which uses the 60/30 isometric constants, signed `/256` truncation, Z adjustment, and tactical scroll subtraction. `DrawRadarActionLines` then adds `g_RadarViewportOffsetY` to both projected Y values. Active in YR: Yes; evidence `0x004DC49C..0x004DC4D4`, `0x006D2140`.

The renderer draws two endpoint dots as 3x3 rectangles offset by `(-2, -2)` from the projected endpoints. Each rectangle is clipped by `AlphaShapeClass__ClipRect @ 0x00421B60` before surface vtable `+0x14` fills it. Active in YR: Yes; evidence `0x004DC4EC..0x004DC65E`, `0x0045A130`, `0x00421B60`.

The main segment is clipped by `FUN_007BC2B0` against `{g_RadarViewportOffsetX, g_RadarViewportOffsetY, g_RadarViewportWidth, g_RadarViewportHeight}`. If clipping rejects the segment, the function returns after dots may already have drawn. Active in YR: Yes; evidence `0x004DC661..0x004DC69B`, `0x007BC2B0`.

Line animation uses `timeGetTime()`, not `g_CurrentFrameCounter` and not the selected action-line timer. The low 10 bits drive color/intensity modulation; `(-time >> 5) & 0xF` drives dash phase; dash pattern pointer is `DAT_00822540`. Active in YR: Yes; evidence `0x004DC6A1..0x004DC747`, memory `0x00822540 = 01 01 01 01 01 00 00 00 ...`.

The color source is the object's owner house RGB at `House+0x56F9..0x56FB`, converted through display-format globals; a byte-wise blend helper `FUN_006612C0` can modulate the RGB work buffer during part of the `timeGetTime() & 0x3FF` cycle. Active in YR: Yes; evidence `0x004DC4D4..0x004DC504`, `0x004DC6A9..0x004DC6D1`, `0x006612C0`.

## 4. INI Keys

| INI key | Location / value | Effect in this slice | Active in YR |
|---|---|---|---|
| `PsychicDetectionRadius=` | `[NAPSIS] PsychicDetectionRadius=15` in `ini/rulesmd.ini:13353`; base `rules.ini:10220` also 15 | Parsed to `BuildingType+0x170C`; helper compares endpoint distance to `radius * 0x100` | Yes, standard YR NAPSIS enables this conditional path |
| `SensorArray=` | `[NAPSIS] SensorArray=yes` in `ini/rulesmd.ini:13375` | Related to building sensor capability/list membership, but not read by `0x004DC340` itself | Conditional; list construction out-of-scope |
| `SensorsSight=` | `[NAPSIS] SensorsSight=15` in `ini/rulesmd.ini:13376` | Not the action-line eligibility radius used in this helper; `PsychicDetectionRadius` is the compared field | No direct effect in this slice |
| `[Options] UnitActionLines=` | String xrefs in `OptionsClass__ReadFromINI @ 0x005FA80E` / `WriteToINI @ 0x005FAE08`; mirror `DAT_00843108` | Gates selected-human `DrawActionLines`, not Psychic Sensor enemy lines | No for this path |

## 5. Integration Points

`get_function_xrefs(0x004DC340)` returned only `TacticalClass_Draw @ 0x006D478E`. `get_function_xrefs(0x0043B150)` returned only `TacticalClass_Draw @ 0x006D4766`. Active in YR: Yes, both are reached from the standard tactical draw loop, conditionally per object.

`DAT_00843108` has only a write at `0x0070D180` and a read at `0x006D473F`; no read exists in `DrawRadarActionLines` or the non-human branch. The selected action-line timer globals `0x00B0EA80/0x00B0EA84` have xrefs from selected-line timer/save-load code, not from `0x004DC340`. Active in YR: negative finding for this path.

Current Rust status: Codegraph and `rg` find selected command line support centered on `src/app_target_lines.rs` (`TargetLineState`, `record_command_lines`, `build_target_line_instances`) and no analogue for `DrawRadarActionLines`/Psychic Sensor enemy action lines. That scan is implementation context only, not a behavioral source.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass__DrawRadarActionLines @ 0x004DC340` | verified | decompile plus assembly contexts `0x004DC34F..0x004DC747` | none for requested slice |
| `FUN_0043B150` eligibility helper | verified for line eligibility | decompile `0x0043B150`; caller `0x006D4766` | building sensor-list maintenance out-of-scope |
| Tactical-screen vs minimap classification | verified | `0x004DC49C..0x004DC4D4`, `0x006D2140`, surface `DAT_0088731C`, viewport globals `0x00886FA0..0x00886FAC` | none |
| Caller gates | verified | `0x006D4764..0x006D478E` | exact semantic name of `Object+0x81` outside scope |
| Endpoint priority | verified | `0x004DC34F..0x004DC410` | none |
| Dots/color/dash | verified | `0x004DC4D4..0x004DC747`; memory `0x00822540` | final display-format pixel values require runtime mode |
| Timer and `UnitActionLines` non-involvement | verified | xrefs `DAT_00843108`, `0x00B0EA80`, `0x00B0EA84`; branch compare `0x006D473F` vs call `0x006D478E` | none |
| Relative draw order | verified | `TacticalClass_Draw`; assembly order `0x006D478E` before `0x006D47BF`, `0x006D47F6`, `0x006D48F1` | geometry of later lines out-of-scope |
| Current Rust comparison | touched-not-exhausted | Codegraph context; `src/app_target_lines.rs` search | no Rust parity judgment beyond missing analogue |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is this tactical-screen output or minimap output? -> Tactical-screen output; it uses tactical projection, tactical viewport globals, and surface `DAT_0088731C`.` (evidence: `0x004DC49C..0x004DC747`, `0x006D2140`)
- `[RESOLVED] OQ-2 - What live path calls it in standard YR? -> Only `TacticalClass_Draw` in this database, from the pass-2/pass-3 tactical overlay loop.` (evidence: `get_function_xrefs(0x004DC340)`, `0x006D478E`)
- `[RESOLVED] OQ-3 - What makes a non-human object eligible? -> Non-allied to local player, at least one local psychic-detection building passes vtable `+0x350`, endpoint within `PsychicDetectionRadius * 0x100`, and caller gates pointer non-null, `Object+0x14 bit 0x04`, `Object+0x81 == 0`.` (evidence: `FUN_0043B150`, `0x006D4764..0x006D478E`, `0x00460C39..0x00460C46`)
- `[RESOLVED] OQ-4 - Which endpoint wins when both attack target and movement exist? -> `ArchiveTarget` wins; movement is considered only when `ArchiveTarget` is null.` (evidence: `0x004DC34F..0x004DC3C5`)
- `[RESOLVED] OQ-5 - Does NavQueue affect the drawn endpoint? -> Yes, final queued nav item `Items[Count - 1]` wins when count is nonzero; otherwise `NavCom` is used.` (evidence: `0x004DC3E1..0x004DC410`)
- `[RESOLVED] OQ-6 - Does bridge height affect movement endpoint Z? -> Yes, only in movement branch when endpoint cell has `Cell+0x140 & 0x100`.` (evidence: `0x004DC42A..0x004DC498`)
- `[RESOLVED] OQ-7 - What dot style is drawn? -> Two 3x3 clipped rectangles offset by `(-2, -2)` from projected endpoints.` (evidence: `0x004DC4EC..0x004DC65E`)
- `[RESOLVED] OQ-8 - What dash pattern and phase source are used? -> Pattern `01 01 01 01 01 00 00 00`, phase `(-timeGetTime() >> 5) & 0xF`.` (evidence: `0x004DC6A1..0x004DC747`, memory `0x00822540`)
- `[RESOLVED] OQ-9 - Does `[Options] UnitActionLines` gate this path? -> No; it gates the selected-human branch only.` (evidence: `0x006D473F`, `0x006D478E`, xrefs to `0x00843108`)
- `[RESOLVED] OQ-10 - Does the selected action-line 25-frame timer gate this path? -> No; timer globals are not referenced by `0x004DC340`.` (evidence: xrefs to `0x00B0EA80/0x00B0EA84`)
- `[RESOLVED] OQ-11 - Where does it draw relative to mind-control/service lines? -> Before CaptureManager links and service/tether helper for the same per-techno overlay loop when the common block is reached.` (evidence: `0x006D478E`, `0x006D47BF`, `0x006D47F6`, `0x006D48F1`)
- `[DEFERRED] OQ-12 - What is the exact runtime semantic name of `Object+0x81`?` (category: `requires-different-system-context`; reason: caller uses it as a draw suppressor, but naming it confidently requires the broader visibility/cloak state audit; next-step-if-pursued: dedicated Object/Techno visibility flag xref slice)
- `[DEFERRED] OQ-13 - What are exact final RGB pixel values under every display format?` (category: `needs-runtime-debugger`; reason: binary shows source RGB and conversion globals but final mode values are runtime-dependent; next-step-if-pursued: debugger capture of `g_DD_*` globals and surface writes)

## Sources

- Ghidra decompile/assembly: `TechnoClass__DrawRadarActionLines @ 0x004DC340`
- Ghidra decompile/assembly: `FUN_0043B150`
- Ghidra decompile/assembly: `TacticalClass_Draw @ 0x006D3D10`
- Ghidra helpers checked: `TechnoClass__Resolve_ArchiveTarget_Coords @ 0x0070BCB0`, `TacticalClass__CoordsToClient2 @ 0x006D2140`, `FUN_006612C0`, `FUN_0045A130`, `AlphaShapeClass__ClipRect @ 0x00421B60`, `FUN_007BC2B0`
- Ghidra xrefs/memory: `0x004DC340`, `0x0043B150`, `0x00843108`, `0x00B0EA80`, `0x00B0EA84`, `0x00822540`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`
- Prior context checked, not used as ground truth: `DRAWRADARACTIONLINES_004DC340_ENEMY_LINES_GHIDRA_REPORT.md`, `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`, selected action-line reports
