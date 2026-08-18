# Selected Unit Action Target Line Baseline - Ghidra Research Report

**Address(es):** `0x004DC060` (`TechnoClass__DrawActionLines`), `0x007049C0` (`ActionLines__DrawLine`), `0x006D3D10` (`TacticalClass_Draw`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** selected human/mobile click-command action/target lines only: state source, timer/option gates, endpoints, pixel style, surface, and draw order baseline for comparison with other battlefield line families.
**Non-Scope:** factory rally lines, planning/waypoint overlay lines, Psychic Sensor enemy lines, CaptureManager link geometry, service/tether lines, and surface vtable raster internals below the submitted draw calls.
**Confidence:** High for the selected-unit path and helper call shape; Medium for exact final surface pixel raster below `DAT_0088731c` vtable methods because this slot did not decompile those surface implementations.
**Active in YR:** Conditional. Active in standard YR for human-owned selected mobile technos when `UnitActionLines`/`DAT_00843108` is enabled, the global 25-frame timer is still live or forced param is nonzero, and the techno has `ArchiveTarget` or `NavCom`.

## 1. Overview

The selected-unit action target line is the short-lived battlefield line drawn after player command feedback, such as selected unit to clicked move destination or selected unit to attack target. It is produced by the selected-human branch in `TacticalClass_Draw`, implemented by mobile-techno vtable slot `+0x438`, and rendered by `ActionLines__DrawLine`.

This baseline is separate from rally, planning, Psychic Sensor, and mind-control lines. The player-visible selected-unit output is one line per selected mobile unit at most: `ArchiveTarget` has priority; otherwise movement points to the final queued waypoint when `NavQueue.Count != 0`, or `NavCom` when no queue exists.

## 2. Key Offsets And Globals

| Field / global | Purpose | Active in YR | Evidence |
|---|---|---|---|
| `Techno+0x83` | selected-state byte checked before dispatch | Conditional: selected objects only | `0x006D4735-0x006D473D` |
| `DAT_00843108` | selected action-line option mirror | Conditional: mirrors `UnitActionLines` | read at `0x006D473F`, writer `0x0070D180` |
| `Techno+0x2B4` | `ArchiveTarget`; attack/combat line source selector | Yes, conditional on active target | read at `0x004DC069`, branch at `0x004DC0B3` |
| `Foot+0x5A4` | `NavCom`; movement fallback endpoint | Yes, conditional on move/nav destination | read at `0x004DC073`, `0x004DC1CA` |
| `Foot+0x598` | `NavQueue.Count`; selects final queued endpoint when nonzero | Conditional: queued waypoints | read at `0x004DC1BC` |
| `Foot+0x58C` | `NavQueue.Items`; endpoint uses `Items[Count-1]` | Conditional: queued waypoints | read at `0x004DC1D9-0x004DC1DF` |
| `Object+0x9C/+0xA0/+0xA4` | movement line source coords | Yes, movement branch | read at `0x004DC1AA-0x004DC1C4` |
| `g_ActionLines_StartFrame 0x00B0EA80` | timer start frame | Yes | read at `0x004DC089`, written by `0x0070D150` |
| `g_ActionLines_Duration 0x00B0EA88` | duration; `0x19` frames from `StartTimer` | Yes | read at `0x004DC08F`, written by `0x0070D150` |
| `DAT_0088731c` | composition surface used by endpoint boxes and final line | Yes | calls at `0x00704D11`, `0x00704DA2`, `0x00704E30` |

## 3. Core Logic

### 3.1 Tactical dispatch gate

`TacticalClass_Draw` reaches the selected-unit line path in the pass where `param_3 == 2` or `3`. Inside the `g_TechnoClass_Array` loop, it first checks `HouseClass__IsHumanPlayer`; non-human objects go to the separate radar/Psychic Sensor branch.

For the selected-unit branch, stock YR then checks:

1. radar overlay branch flag `cVar4 == 0`;
2. `Techno+0x83 != 0`;
3. `DAT_00843108 != 0`;
4. vtable `+0x438` with both stack arguments pushed as zero.

Active in YR: Conditional. Evidence: `0x006D471A-0x006D4750`; assembly context shows `MOV AL,[0x00843108]`, `PUSH 0x0`, `PUSH 0x0`, then `CALL dword ptr [EAX + 0x438]`.

### 3.2 Timer and target existence gate

`TechnoClass__DrawActionLines` returns immediately unless either `ArchiveTarget` or `NavCom` is non-null. If the low byte of its parameter is zero, it applies the global timer:

- if `g_ActionLines_StartFrame != -1`, elapsed is `g_CurrentFrameCounter - g_ActionLines_StartFrame`;
- if `elapsed >= g_ActionLines_Duration`, return;
- otherwise use `duration - elapsed`;
- return if remaining is less than `1`.

Active in YR: Yes for stock selected-unit calls because the caller passes zero. Evidence: no-target gate `0x004DC069-0x004DC07B`; timer gate `0x004DC081-0x004DC0AD`; `ActionLines__StartTimer @ 0x0070D150` writes current frame and `0x19`.

### 3.3 Attack/combat endpoint path

If `ArchiveTarget` is non-null, it wins over movement and the function returns after drawing. The start point comes from the selected techno vtable slot `+0x300` with argument `0`, not from raw `+0x9C/+0xA0/+0xA4` location. The endpoint comes from `TechnoClass__Resolve_ArchiveTarget_Coords @ 0x0070BCB0`, which begins with target vtable `+0x58` center coords and has a conditional building/locomotor-moving correction.

The attack color is converted from `ConvertClass+0x174` palette-table index `8`: byte offset `+8` in 8-bit mode or word offset `+0x10` in 16-bit mode, then shifted through display channel globals before being passed as RGB-like bytes to the helper.

Active in YR: Yes, conditional on `ArchiveTarget != 0`. Evidence: branch and return `0x004DC0B3-0x004DC1A7`; source call `0x004DC0BB-0x004DC0C6`; helper call `0x004DC0CE-0x004DC0D6`; color reads around `0x004DC100-0x004DC11D`; helper flags pushed as zero at `0x004DC12F-0x004DC131`.

### 3.4 Movement endpoint path

If `ArchiveTarget` is null, the movement branch uses source coords from `+0x9C/+0xA0/+0xA4`. Endpoint selection is:

- if `NavQueue.Count == 0`, use `NavCom`;
- otherwise use `NavQueue.Items[NavQueue.Count - 1]`;
- call selected endpoint object's vtable `+0x48` for endpoint coords.

For movement endpoints only, it checks the endpoint cell and replaces Z with `CellClass__GetGroundHeight(endpoint) + DAT_008B3DF4` when the endpoint cell is in bounds and `Cell+0x140 & 0x100` is nonzero.

The movement color is converted from palette-table index `3`: byte offset `+3` in 8-bit mode or word offset `+6` in 16-bit mode.

Active in YR: Conditional. Evidence: source reads `0x004DC1AA-0x004DC1C4`; queue/fallback branch `0x004DC1BC-0x004DC1EC`; bridge-Z branch `0x004DC205-0x004DC276`; movement color reads around `0x004DC291-0x004DC2A4`; helper call at `0x004DC323`.

### 3.5 Pixel style and target surface

`ActionLines__DrawLine` projects both 3D endpoints through `TacticalClass__CoordsToClient2 @ 0x006D2140`, then adds `g_RadarViewportOffsetY` to both projected Y values. `CoordsToClient2` uses `0x3c` and `0x1e` iso constants, signed `/256` style conversion, Z adjustment, and tactical scroll subtraction.

For stock selected-unit output, the helper draws:

1. one clipped `3x3` endpoint box at each endpoint, offset by `(-2,-2)`;
2. one final clipped solid line.

It draws both endpoint boxes and the final line on `DAT_0088731c`. Endpoint boxes call surface vtable `+0x14`; final solid lines call vtable `+0x30`. The dashed branch exists and uses `(0x7fffffff - g_CurrentFrameCounter) % 0xf` plus pattern pointer `DAT_00843128`, but stock selected-unit dispatch pushes zero arguments and the attack branch also pushes zero into the helper, so stock selected-unit click-command lines do not use dashed mode.

Active in YR: Yes for endpoint boxes plus solid line; Conditional but not reached by stock selected-unit dispatch for dashed mode. Evidence: projection calls `0x007049CD-0x007049F4`; endpoint box setup/calls `0x00704C8B-0x00704D11` and `0x00704D14-0x00704DA2`; dashed phase `0x00704DB0-0x00704DC5`; solid call `0x00704E1D-0x00704E30`; caller zero args `0x006D474A-0x006D4750`, `0x004DC12F-0x004DC131`.

## 4. INI Keys

| Key | Default / source | Effect | Active in YR | Evidence |
|---|---|---|---|---|
| `[Options] UnitActionLines` | default enabled by `OptionsClass__SetDefaults`; user INI can override | controls selected-unit dispatch through `DAT_00843108` | Yes | default write at `Options+0x1E` in `0x005FA350`; read string `s_UnitActionLines_008331c8` in `0x005FA620`; sync writer `0x0070D180` |

No rules/art INI key controls endpoint size, selected-unit line thickness, dash pattern, or selected-unit line duration in this slice. The endpoint `3x3`, `-2` offset, `0x19` timer duration, palette indices `8` and `3`, and dash modulus `0xf` are binary constants.

## 5. Integration Points And Draw Order

In `TacticalClass_Draw`, selected-unit action target lines draw after the broader tactical object/effect/UI overlay calls such as `Tactical_ObjectRenderingLoop`, `LaserDrawClass__DrawAll`, `EBoltMgr__UpdateAndDrawAll`, `LineTrail__UpdateAndDrawAll`, `RadBeam__DrawAndTickAll`, `Tactical__DrawUnitActionVisuals`, bandbox, placement/rally-family overlay calls, and radar overlay setup.

Within the per-techno loop, selected-unit `DrawActionLines` precedes CaptureManager link drawing and service/tether line drawing for the same techno.

Active in YR: Conditional on pass and object state. Evidence: parent pass order in `TacticalClass_Draw @ 0x006D3D10`; selected action-line call `0x006D4750` precedes CaptureManager branch beginning `0x006D479F` and service/tether branch ending at `0x006D48F1`.

## 6. Current Rust Implementation Status

The repo already has an app-layer implementation in `src/app_target_lines.rs`: 25-tick timer, command-triggered records, selected non-structure filtering, hardcoded move/attack colors, and a float-stepped `1x1` line emitter. It currently differs from this baseline in several visible ways:

- no `UnitActionLines` option gate observed in the scanned Rust path;
- hardcoded approximate RGB rather than palette/ConvertClass indices `8` and `3`;
- attack source uses entity screen position rather than fire coords/vtable `+0x300` equivalent;
- movement endpoint comes from recorded command cell, not verified `NavQueue.Last else NavCom` runtime state;
- no selected-unit endpoint `3x3` boxes;
- line uses float DDA `1x1` sprite instances rather than `ActionLines__DrawLine` clipping and surface calls;
- current render placement is app UI step 10 before selection brackets, not the original same per-techno ordering before CaptureManager/service lines.

Evidence: `src/app_target_lines.rs:18-24`, `src/app_target_lines.rs:74-128`, `src/app_target_lines.rs:148-186`, `src/app_target_lines.rs:225-258`, `src/app_context_order.rs:731-733`, `src/app_render/draw_passes.rs:298-306`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass__DrawActionLines @ 0x004DC060` | verified | decompile plus assembly contexts | none for selected-unit baseline |
| selected-human tactical dispatch | verified | `0x006D471A-0x006D4750` | none |
| option default/read/sync | verified | `0x005FA350`, `0x005FA620`, `0x0070D180` | dialog checkbox details out of scope |
| timer read in draw function | verified | `0x004DC081-0x004DC0AD`, `0x0070D150` | full timer xref inventory belongs to timer report |
| attack `ArchiveTarget` branch | verified | `0x004DC0B3-0x004DC1A7`, `0x0070BCB0` | deeper chrono/building correction math out of scope |
| movement `NavCom/NavQueue` branch | verified | `0x004DC1BC-0x004DC1EC` | queue writer semantics out of scope |
| movement bridge-Z endpoint adjustment | verified | `0x004DC205-0x004DC276` | none for branch predicate/effect |
| `ActionLines__DrawLine @ 0x007049C0` selected stock path | verified | decompile plus assembly contexts | surface vtable raster internals deferred |
| projection helper `0x006D2140` | verified | decompile | exact runtime value of Z multiplier global not needed for baseline |
| line clip helper `0x007BC2B0` | touched-not-exhausted | decompile | final pixel rules inside surface methods out of scope |
| Rust implementation comparison | verified from local source | `src/app_target_lines.rs`, `src/app_render/draw_passes.rs` | no code changes in this slot |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Is the selected click-command line active in standard YR? Yes, conditionally for human selected technos with option enabled and live target/timer state. Evidence: `0x006D4735-0x006D4750`, `0x004DC069-0x004DC0AD`.

[RESOLVED] OQ-2 - Which state wins when both combat and movement state exist? `ArchiveTarget` wins and returns before movement. Evidence: `0x004DC0B3-0x004DC1A7`.

[RESOLVED] OQ-3 - Does movement use immediate `NavCom` or queued waypoints? It uses `NavQueue.Items[Count-1]` when count is nonzero, otherwise `NavCom`. Evidence: `0x004DC1BC-0x004DC1EC`.

[RESOLVED] OQ-4 - Does the stock selected-unit path draw dashed lines? No. The helper supports dashed mode, but stock selected-unit dispatch passes zero and the attack branch pushes zero. Evidence: `0x006D474A-0x006D4750`, `0x004DC12F-0x004DC131`, `0x00704DA5-0x00704E30`.

[RESOLVED] OQ-5 - What is the selected-unit pixel style? Two clipped `3x3` endpoint boxes offset `(-2,-2)` plus one clipped solid line on `DAT_0088731c`. Evidence: `0x00704C8B-0x00704D11`, `0x00704D14-0x00704DA2`, `0x00704E1D-0x00704E30`.

[RESOLVED] OQ-6 - Is `UnitActionLines` a rules/art key? No evidence in repo rules/art scan; binary reads `[Options] UnitActionLines` from user options and defaults it enabled. Evidence: `0x005FA350`, `0x005FA620`; `rg UnitActionLines ini/rulesmd.ini ini/rules.ini ini/artmd.ini ini/art.ini` returned no rows.

[DEFERRED] OQ-7 - Exact raster internals of `DAT_0088731c` vtable `+0x14/+0x30/+0x4c`. Category: out-of-scope. Reason: this baseline verifies submitted geometry/style and surface; surface raster internals are shared renderer substrate work.

## Sources

- Ghidra decompiled: `TechnoClass__DrawActionLines @ 0x004DC060`
- Ghidra decompiled: `ActionLines__DrawLine @ 0x007049C0`
- Ghidra decompiled: `TacticalClass_Draw @ 0x006D3D10`
- Ghidra decompiled: `ActionLines__StartTimer @ 0x0070D150`
- Ghidra decompiled: `TechnoClass__Resolve_ArchiveTarget_Coords @ 0x0070BCB0`
- Ghidra decompiled: `OptionsClass__SetDefaults @ 0x005FA350`, `OptionsClass__ReadFromINI @ 0x005FA620`, `TechnoClass__SetDrawHealthBarsFlag @ 0x0070D180`
- Ghidra decompiled: `TacticalClass__CoordsToClient2 @ 0x006D2140`, `FUN_007BC2B0 @ 0x007BC2B0`
- Assembly contexts checked: `0x006D4735`, `0x006D473F`, `0x006D4750`, `0x004DC069`, `0x004DC081`, `0x004DC0B3`, `0x004DC1BC`, `0x004DC2B6`, `0x004DC323`, `0x00704C8B`, `0x00704D0A`, `0x00704DB0`, `0x00704E1D`
- Prior reports used as starting context only: `TARGET_LINES_GHIDRA_REPORT.md`, `TECHNOCLASS_DRAWACTIONLINES_004DC060_GHIDRA_REPORT.md`, `ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE_GHIDRA_REPORT.md`, `ACTIONLINES_TIMER_START_CLEAR_XREFS_GHIDRA_REPORT.md`, `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`
- Rust/source scans: `src/app_target_lines.rs`, `src/app_context_order.rs`, `src/app_render/draw_passes.rs`
