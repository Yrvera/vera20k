# TechnoClass::DrawActionLines 0x004DC060 - Ghidra Research Report

**Address(es):** `0x004DC060` primary, `0x006D4735-0x006D4750` active caller, `0x0070BCB0` archive-target endpoint helper  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Selected-unit action line gates, ArchiveTarget/NavCom/NavQueue priority, endpoint coordinate sources, movement endpoint bridge Z adjustment, and standard YR activity for `TechnoClass::DrawActionLines`.  
**Non-Scope:** Low-level pixel raster style in `ActionLines__DrawLine`, enemy/AI `DrawRadarActionLines`, all timer start/clear call sites beyond the primary gate, and UI checkbox plumbing beyond default/read/sync evidence needed for active-path status.  
**Confidence:** High  
**Active in YR:** Conditional - active in standard YR when `[Options] UnitActionLines` / `DAT_00843108` is true, the techno belongs to the human player, is selected, and has either `ArchiveTarget` or `NavCom`.

## 1. Overview

`TechnoClass::DrawActionLines` is the selected-unit target-line producer used by mobile technos in standard YR. It draws at most one action line per selected human-owned unit: combat target lines win over movement lines, and movement lines point to the final queued destination when `NavQueue.Count > 0`.

The player-visible effect is the line shown after issuing move/attack-style commands. The function itself is not a renderer; it chooses the source point, endpoint, color index, and timer/force parameters, then calls `ActionLines__DrawLine`.

## 2. Key Offsets And Globals

| Field / global | Evidence | Purpose | Active in YR |
|---|---:|---|---|
| `Techno+0x2B4` | `0x004DC069`, `0x004DC0B3` | `ArchiveTarget`; combat branch gate and priority selector | Yes - live branch in selected-human call path |
| `Foot/Techno+0x5A4` | `0x004DC073`, `0x004DC1CA` | `NavCom`; movement branch fallback endpoint | Yes - live branch when `ArchiveTarget == 0` |
| `Foot+0x598` | `0x004DC1BC` | `NavQueue.Count`; nonzero selects last queued waypoint | Yes - live branch when queued waypoints exist |
| `Foot+0x58C` | `0x004DC1D9-0x004DC1E3` | `NavQueue.Items`; endpoint uses `Items[Count-1]` | Yes - live branch when queue count is nonzero |
| `Object/Techno+0x9C/+0xA0/+0xA4` | `0x004DC1AA-0x004DC1C4` | movement line source world coordinates | Yes - movement branch |
| `g_ActionLines_StartFrame` `0x00B0EA80` | `0x004DC089`, `0x0070D150` | timer start frame, `-1` means no start-frame subtraction | Yes - live timer gate |
| `g_ActionLines_Duration` `0x00B0EA88` | `0x004DC08F`, `0x0070D150` | duration; `StartTimer` writes `0x19` frames | Yes - live timer gate |
| `DAT_00843108` | `0x006D473F-0x006D4746`, `0x0070D180` | call-site option flag mirror | Yes - standard default is true, player can disable |

## 3. Core Logic

1. The function first requires `ArchiveTarget != 0 || NavCom != 0`.
   Evidence: `0x004DC069-0x004DC07B`.  
   Active in YR: Yes - this is the first branch in the live vtable method.

2. If the first stack parameter's low byte is zero, the timer gate is applied. With `g_ActionLines_StartFrame == -1`, the function uses the full current duration; otherwise it computes `elapsed = g_CurrentFrameCounter - g_ActionLines_StartFrame`, returns when `elapsed >= g_ActionLines_Duration`, and otherwise keeps `remaining = duration - elapsed`. It also returns when `remaining <= 0`.
   Evidence: `0x004DC081-0x004DC0AD`. `ActionLines__StartTimer` at `0x0070D150` writes current frame and `0x19`.  
   Active in YR: Yes - the standard caller pushes parameter zero at `0x006D474A` and `0x006D474C`.

3. `ArchiveTarget` has strict priority. Once `ArchiveTarget` is nonzero, the combat branch calls `ActionLines__DrawLine` and returns; it never falls through to `NavCom` or `NavQueue`.
   Evidence: branch at `0x004DC0B3-0x004DC19B`, return epilogue at `0x004DC1A0-0x004DC1A7`; movement branch starts only at `0x004DC1AA`.  
   Active in YR: Yes - live branch for selected mobile technos with an active archive/combat target.

4. Combat line source uses virtual slot `+0x300` with argument `0`, not the object's raw location. The result is copied as the line start.
   Evidence: `0x004DC0BB-0x004DC0D6` calls `[vtable+0x300]`, pushes `0`, then copies returned X/Y/Z.  
   Active in YR: Yes - live `ArchiveTarget` branch.

5. Combat line endpoint comes from `TechnoClass__Resolve_ArchiveTarget_Coords` at `0x0070BCB0`. That helper starts from `ArchiveTarget->[vtable+0x58]` center coordinates and has a conditional building/locomotor-moving adjustment path.
   Evidence: caller `0x004DC0D6-0x004DC0E4`; helper `0x0070BCB0` decompile.  
   Active in YR: Yes for the basic center-coordinate path; Conditional for the building moving-locomotor correction because it requires `ArchiveTarget.WhatAmI() == 1`, non-null locomotor at `+0x674`, locomotor slot `+0x10` true, and non-null/nonzero turret type from caller slot `+0x3F4`.

6. Movement line source is the unit/object location at `+0x9C/+0xA0/+0xA4`.
   Evidence: `0x004DC1AA-0x004DC1C4` loads X into `EBP`, Y into `EBX`, and Z into stack before endpoint selection.  
   Active in YR: Yes - live movement branch when `ArchiveTarget == 0` and `NavCom != 0`.

7. Movement endpoint priority is `NavQueue.Items[NavQueue.Count - 1]` when `NavQueue.Count != 0`; otherwise it uses `NavCom`. The selected endpoint object's virtual slot `+0x48` supplies endpoint coordinates.
   Evidence: zero-count path `0x004DC1BC-0x004DC1D7`; queued path `0x004DC1D9-0x004DC1E3`; common `Get_Coords` call at `0x004DC1EC`.  
   Active in YR: Yes - `NavCom` fallback is normal movement; queued endpoint is conditional on nonempty queue.

8. Movement endpoint bridge adjustment uses endpoint X/Y converted to cell coordinates by adding `0xFF` only for negative values, then arithmetic shifting by 8. If the map bounds check succeeds and `Cell+0x140` has bit `0x100`, endpoint Z is replaced with `CellClass__GetGroundHeight(endpoint) + DAT_008B3DF4`.
   Evidence: conversion `0x004DC205-0x004DC229`, bounds call `0x004DC23B`, cell lookup/flag test `0x004DC244-0x004DC25C`, height plus bridge offset `0x004DC25E-0x004DC276`.  
   Active in YR: Conditional - only for in-bounds movement endpoints on cells whose flags include `0x100`.

9. Movement color uses palette/convert table index `3`; combat color uses index `8`. Both are converted through display shift/loss globals before `ActionLines__DrawLine`.
   Evidence: combat table reads `+8` or `+0x10` at `0x004DC100-0x004DC11D`; movement reads `+3` or `+6` at `0x004DC280-0x004DC2A4`.  
   Active in YR: Yes - both live branches.

## 4. INI And Option Evidence

| Key / default | Evidence | Effect | Active in YR |
|---|---|---|---|
| `[Options] UnitActionLines` | string `0x008331C8`; read by `OptionsClass__ReadFromINI` at `0x005FA808-0x005FA81D` into `Options+0x1E` | user option for action lines | Yes |
| default `UnitActionLines = true` | `OptionsClass__SetDefaults` writes byte `1` at `Options+0x1E` (`0x005FA37D`) | standard default enables call gate | Yes |
| option mirror to `DAT_00843108` | `OptionsClass__ReadFromINI` calls `TechnoClass__SetDrawHealthBarsFlag` with `Options+0x1E` at `0x005FACFA-0x005FACFD`; setter writes `DAT_00843108` at `0x0070D180` | selected-unit call-site gate | Yes |

Repo INI files did not contain an overriding `UnitActionLines` row in the checked `ini/` tree, so the binary default and player settings path are the relevant standard-YR evidence for this slot.

## 5. Integration Points

The active standard-YR call is in `TacticalClass_Draw` pass 2's techno iteration. The relevant branch:

- verifies the object owner is the human player through `HouseClass__IsHumanPlayer` before entering the selected-unit action-line path (`0x006D471A-0x006D4727`);
- skips if radar overlay mode flag `cVar4` is nonzero (`0x006D4729-0x006D472F`);
- requires selected byte `Techno+0x83` nonzero (`0x006D4735-0x006D473D`);
- requires `DAT_00843108` nonzero (`0x006D473F-0x006D4746`);
- pushes both DrawActionLines stack arguments as `0` and calls virtual `+0x438` (`0x006D4748-0x006D4750`).

Vtable data xrefs show `0x004DC060` is reached virtually, not by direct code calls:

| Vtable entry address | Value read | Active in YR |
|---:|---:|---|
| `0x007E26DC` | `0x004DC060` | Yes - aircraft/action-line-capable mobile techno vtable slot |
| `0x007E90CC` | `0x004DC060` | Yes - foot/mobile techno vtable slot |
| `0x007EB490` | `0x004DC060` | Yes - infantry/mobile techno vtable slot |
| `0x007F60A8` | `0x004DC060` | Yes - unit/mobile techno vtable slot |

Building/base entries checked in this slice point at the empty stub `0x00459E60`, whose body immediately returns. Active in YR: No for selected building action lines through this function, because selected buildings do not use `0x004DC060` at vtable `+0x438`.

## 6. Current Rust Implementation Status

Searches for `DrawActionLines`, `ActionLines`, `UnitActionLines`, `NavQueue`, and `ArchiveTarget` under `src/` returned no matching implementation points. This slot did not inspect broader movement/attack fields because the requested target is binary verification, not implementation planning.

Active in YR: Not applicable to Rust status.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass__DrawActionLines` `0x004DC060` | verified | full decompile and assembly listing | none for claimed slice |
| early no-target gate | verified | `0x004DC069-0x004DC07B` | none |
| timer gate in primary function | verified | `0x004DC081-0x004DC0AD`; `0x0070D150` | full timer xref semantics are slot 3 |
| `ArchiveTarget` priority over movement | verified | `0x004DC0B3-0x004DC1A7` before movement branch | none |
| combat source coordinate source | verified | vtable `+0x300` call at `0x004DC0BB-0x004DC0D6` | exact concrete method behavior out of scope |
| combat endpoint helper | touched-not-exhausted | `0x0070BCB0` decompiled | helper's chrono/building correction internals are only summarized |
| movement source coordinate source | verified | `0x004DC1AA-0x004DC1C4` | none |
| `NavCom` fallback endpoint | verified | `0x004DC1CA-0x004DC1D7` | none |
| `NavQueue` last-element endpoint | verified | `0x004DC1D9-0x004DC1E3` | queue writers are out of scope |
| movement bridge Z adjustment | verified | `0x004DC205-0x004DC276` | none for branch predicate/effect |
| selected human call-site activity | verified | `0x006D471A-0x006D4750` | broader render pass ordering is slot 5 |
| option default/read/sync | verified | `0x005FA37D`, `0x005FA808-0x005FA81D`, `0x005FACFA-0x005FACFD`, `0x0070D180` | UI checkbox dialog details are slot 5 |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Is `0x004DC060` active in standard YR? Yes, conditionally: `TacticalClass_Draw` calls vtable `+0x438` for human selected technos when `DAT_00843108 != 0` and the vtables for mobile techno classes point at `0x004DC060`. Evidence: `0x006D4735-0x006D4750`, vtable data reads at `0x007E26DC`, `0x007E90CC`, `0x007EB490`, `0x007F60A8`.  
[RESOLVED] OQ-2 - Does `ArchiveTarget` override movement lines? Yes; the combat branch returns after drawing and never reaches movement selection. Evidence: `0x004DC0B3-0x004DC1A7`.  
[RESOLVED] OQ-3 - Does the movement line use current `NavCom` or queued waypoints? It uses `NavQueue.Items[Count-1]` when count is nonzero, otherwise `NavCom`. Evidence: `0x004DC1BC-0x004DC1EC`.  
[RESOLVED] OQ-4 - What coordinates are used for source endpoints? Combat uses vtable `+0x300` with argument `0`; movement uses `+0x9C/+0xA0/+0xA4`. Evidence: `0x004DC0BB-0x004DC0D6`, `0x004DC1AA-0x004DC1C4`.  
[RESOLVED] OQ-5 - Does bridge adjustment alter target-line endpoint Z? Yes for movement endpoints only when the endpoint cell is in bounds and `Cell+0x140 & 0x100` is set. Evidence: `0x004DC205-0x004DC276`.  
[DEFERRED] OQ-6 - What exact pixels does `ActionLines__DrawLine` emit? category: out-of-scope. Reason: assigned to slot 2. Next step: use `ACTIONLINES_DRAWLINE_007049C0_PIXEL_STYLE`.  
[DEFERRED] OQ-7 - Which exact player actions start/clear the 25-frame timer? category: out-of-scope. Reason: assigned to slot 3. Next step: use `ACTIONLINES_TIMER_START_CLEAR_XREFS`.

## Sources

- Ghidra decompiled/listed: `TechnoClass__DrawActionLines` `0x004DC060`.
- Ghidra decompiled/listed: `TacticalClass_Draw` `0x006D3D10`, call-site region `0x006D4735-0x006D4750`.
- Ghidra decompiled: `TechnoClass__Resolve_ArchiveTarget_Coords` `0x0070BCB0`.
- Ghidra decompiled: `ActionLines__StartTimer` `0x0070D150`.
- Ghidra decompiled/listed: `OptionsClass__SetDefaults` `0x005FA350`; `OptionsClass__ReadFromINI` `0x005FA620`; `TechnoClass__SetDrawHealthBarsFlag` `0x0070D180`.
- Ghidra memory reads: vtable entries `0x007E26DC`, `0x007E90CC`, `0x007EB490`, `0x007F60A8`.
- Starting reference only: `docs/research/TARGET_LINES_GHIDRA_REPORT.md`.
