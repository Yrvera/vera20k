# Factory Rally Point Line Caller / Color / Gate - Ghidra Research Report

**Address(es):** `0x006DA9D0` (verified selected-building rally-line renderer), `0x006DBB60` (checked DrawLine3D premise), `0x006D3D10` (tactical draw integration), `0x00455DA0` (BuildingClass rally-visual eligibility), `0x00443860` / `0x004FBF60` (rally-point set/store context)

**Investigation Mode:** exhaustive-slice

**Claimed Scope:** selected factory/building rally point line rendering: caller sites, visibility gates, coordinate sources, color source, dash phase, and draw order relative to brackets/action lines.

**Non-Scope:** planning-mode waypoint paths, `FUN_006DAD60` marker/link family beyond its draw-order adjacency, factory unit-exit gameplay, passability snapping internals beyond the already-known rally storage path.

**Confidence:** High for the corrected selected-building rally-line path; Medium for exact surface-slot pixel raster internals because this slot did not decompile the primary-surface vtable target.

**Active in YR:** Conditional. The path is live in standard YR when a local-player selected building is a rally-capable producer/repair/cloning building and has `TechnoClass+0x218` set to a rally target.

## 1. Overview

The starting premise was stale: the selected factory/building rally point line does **not** call `Tactical::DrawLine3D @ 0x006DBB60`. The verified live selected-building rally line renderer is `FUN_006DA9D0`, called twice from `TacticalClass_Draw`; it projects the selected building and its rally target to tactical client pixels and draws directly through `g_PrimarySurface` vtable slot `+0x4C`.

This means prior wording that grouped rally lines under `Tactical::DrawLine3D` should be treated as a coarse family-level shortcut, not a verified implementation fact for selected factory rally lines.

## 2. Key Offsets / Fields

| Owner | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| BuildingClass / TechnoClass | `+0x218` (`param_1[0x86]`) | `AbstractClass*` | rally target pointer consumed by `FUN_006DA9D0`; written by command handling helper context | Conditional; evidence `0x006DA9D0`, `0x0070C610` |
| BuildingClass / TechnoClass | `+0x21C` (`param_1[0x87]`) | `HouseClass*` | owner compared to `g_PlayerPtr` before drawing | Yes; evidence `0x006DA9D0` |
| ObjectClass/TechnoClass | `+0x83` | byte | selected flag gate | Conditional; evidence `0x006DA9D0` |
| ObjectClass/TechnoClass | `+0x90` low byte (`param_1[0x24]`) | byte | draw/visible-list gate from current objects list | Conditional; evidence `0x006DA9D0` |
| BuildingTypeClass | `+0xEB8` | int enum | factory category gate: `0x28` UnitType or `0x10` InfantryType returns true in `0x00455DA0` | Yes for standard WF/barracks; evidence `0x00455DA0`, `rulesmd.ini` |
| BuildingTypeClass | `+0x16AC` | byte | `Cloning=` also permits rally-line visual eligibility | Conditional; evidence `0x00455DA0`, `rulesmd.ini:13541` |
| BuildingTypeClass | `+0x16A9` | byte | `UnitRepair=` also permits rally-line visual eligibility | Conditional; evidence `0x00455DA0`, `rulesmd.ini:11877` etc. |
| HouseClass | `+0x56F9/+0x56FA/+0x56FB` | RGB bytes | owner house line color, packed through DD shifts/loss masks | Yes; evidence `0x006DA9D0` |

## 3. Core Logic

### 3.1 Premise correction: not `Tactical::DrawLine3D`

`get_function_xrefs(0x006DBB60)` returned only the Tactical vtable entry at `0x007F43A8`; no direct code xref exists. More importantly, the verified selected-building rally-line caller, `FUN_006DA9D0`, contains no call through the Tactical vtable `+0x60` entry. It draws with:

```text
g_PrimarySurface->vtable[0x4C](&start_client_point, &end_client_point, color, &DAT_00842930, phase, param_2)
```

Active in YR: Yes/Conditional. `0x006DA9D0` is called unconditionally from the live tactical renderer twice, but each object draw is gated.

### 3.2 Caller sites and draw order

`TacticalClass_Draw @ 0x006D3D10` calls `FUN_006DA9D0` twice:

| Call site | Argument | Order role | Active in YR |
|---:|---:|---|---|
| `0x006D4648` | `0` | first overlay pass before `BuildingPlacement_OverlayRenderer`, `FUN_0053D850`, and `Tactical_ObjectRenderingLoop` | Yes |
| `0x006D46CF` | `1` | second overlay pass after `Tactical__DrawUnitActionVisuals` and bandbox, before radar overlays and before selected action lines | Conditional on pass-2 draw path |

Relative ordering:

- The first rally pass is before object rendering and before object extras/brackets.
- The second rally pass is after object rendering / extras and after `Tactical__DrawUnitActionVisuals` / bandbox.
- Selected-unit action target lines draw later in the techno loop at `0x006D473F..0x006D4750`.
- Mind-control link draws from `CaptureManagerClass::DrawLinks` occur still later at `0x006D47B0..0x006D47F6`.

Active in YR: Yes. Evidence: `TacticalClass_Draw @ 0x006D463F..0x006D46D8` and `0x006D473F..0x006D47F6`.

### 3.3 Visibility / eligibility gates

Inside `FUN_006DA9D0`, the loop walks `g_CurrentObjects_Data` backwards from `g_CurrentObjects_Count - 1` to `0`. A line is drawn only when all of these gates pass:

1. Object RTTI via vtable `+0x2C` returns `6` (`BuildingClass::WhatAmI @ 0x00459EC0`).
2. Low byte at `object+0x90` is nonzero.
3. Selected byte at `object+0x83` is nonzero.
4. Owner pointer at `object+0x21C` equals `g_PlayerPtr`.
5. Building vtable `+0x284` returns nonzero.
6. Rally target pointer at `object+0x218` is non-null.

Active in YR: Conditional. Standard selected local-player war factories and barracks can satisfy this after rally is set; non-selected buildings, enemy buildings, no-rally buildings, and ineligible buildings do not draw.

### 3.4 `BuildingClass` vtable `+0x284` eligibility

`BuildingClass` vtable base `0x007E3EBC`; `+0x284` entry at `0x007E4140` points to `0x00455DA0`. This function returns true when any of these are true:

- `BuildingTypeClass+0xEB8 == 0x28` (`Factory=UnitType`)
- `BuildingTypeClass+0xEB8 == 0x10` (`Factory=InfantryType`)
- `BuildingTypeClass+0x16AC != 0` (`Cloning=yes`)
- `BuildingTypeClass+0x16A9 != 0` (`UnitRepair=yes`)

Otherwise it returns false. Active in YR: Yes/Conditional. Evidence: decompile `0x00455DA0`; standard YR INI has `Factory=InfantryType` on barracks and `Factory=UnitType` on war factories, plus `UnitRepair=yes` and `Cloning=yes` on standard structures.

### 3.5 Coordinate sources

Start coordinate:

- `FUN_006DA9D0` calls selected building vtable `+0x48` to fetch world coordinates.
- It converts with `CoordsToClient`, subtracts tactical viewport scroll (`this+0xB0/+0xB4`), and adds radar/tactical viewport offsets (`g_RadarViewportOffsetX/Y`) for the surface-line point.

End coordinate:

- It reads the rally target pointer at building `+0x218`.
- It calls target vtable `+0x48` to fetch target coordinates.
- It overwrites Z with `CellClass__GetGroundHeight`.
- If target cell flags at `CellClass+0x140` include `0x100`, it adds `DAT_00B0CEB4` as a bridge/high-ground correction.
- It projects through `Tactical__WorldToScreenSub` + `Tactical__AdjustForZ`, subtracts tactical scroll, adds `g_RadarViewportOffsetX/Y`, and applies small diagonal offsets before the three line submits.

Active in YR: Yes/Conditional. Evidence: `0x006DA9D0`.

### 3.6 Color, pulse, and line submits

The renderer computes:

- `phase = (0x7FFFFFFF - g_CurrentFrameCounter) % 0xF`
- one background/default-colored line using `DAT_00A83CDA`, `g_DefaultRadarBgColor`, and the DirectDraw channel loss/shift globals
- two owner-colored lines using `HouseClass+0x56F9/+0x56FA/+0x56FB`, packed through `g_DD_RLoss/GLoss/BLoss` and `g_DD_RShift/GShift/BShift`
- the shared pattern pointer `&DAT_00842930`

The three calls are diagonally stepped: the endpoint point is initially offset by `+2,+2`, then decremented by `1,1` before each owner-color submit. This produces a small multi-pixel/outlined line family rather than the selected-unit action line's endpoint-box + single-line style.

Active in YR: Yes/Conditional. Evidence: `0x006DA9D0`; owner RGB offsets also match existing house-color usage in other line reports.

## 4. INI Keys / Standard Content Checks

| INI evidence | Effect here | Active in YR |
|---|---|---|
| `rulesmd.ini:11695 [GAPILE] Factory=InfantryType` | passes `+0xEB8 == 0x10` | Yes |
| `rulesmd.ini:12497 [NAHAND] Factory=InfantryType` | passes `+0xEB8 == 0x10` | Yes |
| `rulesmd.ini:13175 [YABRCK] Factory=InfantryType` | passes `+0xEB8 == 0x10` | Yes |
| `rulesmd.ini:11777 [GAWEAP] Factory=UnitType` | passes `+0xEB8 == 0x28` | Yes |
| `rulesmd.ini:12567 [NAWEAP] Factory=UnitType` | passes `+0xEB8 == 0x28` | Yes |
| `rulesmd.ini:13311 [YAWEAP] Factory=UnitType` | passes `+0xEB8 == 0x28` | Yes |
| `rulesmd.ini:13541 Cloning=yes` | passes `+0x16AC` when set | Conditional |
| `rulesmd.ini:11877` and sibling `UnitRepair=yes` entries | passes `+0x16A9` when set | Conditional |

No dedicated `RallyLineColor` or `RallyLineStyle` INI key was found in the scoped scan. The draw color is runtime house RGB plus display-format packing.

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `TacticalClass_Draw @ 0x006D3D10` | owns both `FUN_006DA9D0` calls and relative render order | `0x006D4648`, `0x006D46CF` | Yes |
| `FUN_006DA9D0` | selected local building rally-line renderer | full decompile | Conditional |
| `BuildingClass` vtable `+0x284 -> 0x00455DA0` | "can draw/use rally visual" predicate | vtable byte read + decompile | Conditional |
| `BuildingClass::SetRallyPoint @ 0x00443860` | player command setup/snapping context; not the draw caller | decompile | Conditional |
| `HouseClass::Set_Rally_Point_Cell @ 0x004FBF60` | house-level rally-cell storage; not this selected-building line's direct target source | decompile | Conditional |
| `TechnoClass__SetGhostCell @ 0x0070C610` | simple writer to `TechnoClass+0x218`; relevant storage helper context | decompile | Conditional |

## 6. Current Rust Implementation Status

Rust currently stores house-level rally points and uses them for production movement, but this slot found no rendered factory rally line:

| Rust file | Current state |
|---|---|
| `src/sim/production/production_queue.rs:25` | stores owner rally point as `(rx, ry)` |
| `src/sim/world/world_commands.rs:579` | handles `Command::SetRally` |
| `src/app_context_order.rs:217..235` | context-order branch emits `SetRally` for structures |
| `src/ui/in_game_hud.rs:57` | HUD text displays rally coordinate |

Missing relative to verified gamemd output: selected-building rally line render, eligibility gates matching `0x00455DA0`, owner RGB line color, phase formula `(0x7FFFFFFF - frame) % 0xF`, three stepped line submits, and two-pass draw order around object/bracket rendering.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Tactical::DrawLine3D @ 0x006DBB60` as factory rally renderer | verified-refuted | `get_function_xrefs -> 0x007F43A8 [DATA]`; `FUN_006DA9D0` decompile has no Tactical vtable `+0x60` call | none for selected factory rally lines |
| `FUN_006DA9D0` caller sites | verified | `0x006D4648`, `0x006D46CF` | none |
| `FUN_006DA9D0` gates | verified | `0x006DA9D0` decompile | exact meaning of low byte `+0x90` outside visible-list context deferred |
| `BuildingClass` vtable `+0x284` | verified | vtable `0x007E3EBC+0x284=0x00455DA0`; decompile | none for eligibility predicate |
| Start/end coordinate source | verified | `0x006DA9D0` vtable `+0x48`, target `+0x218`, `CellClass__GetGroundHeight` | exact stack-local point pair naming is decompiler-noisy; behavioral source is clear |
| Color and phase | verified | `0x006DA9D0`; `House+0x56F9..+0x56FB`; `(0x7FFFFFFF-frame)%0xF` | primary-surface slot `+0x4C` raster internals deferred |
| Draw order vs brackets/action lines | verified | `TacticalClass_Draw @ 0x006D4648`, `0x006D46CF`, `0x006D473F..0x006D47F6` | none for scoped relative order |
| `FUN_006DAD60` adjacent family | touched-not-exhausted | decompile checked for scope separation | planning/marker family belongs to slot 3 / separate report |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does selected factory rally line call Tactical::DrawLine3D? -> No for the verified selected-building rally path; it calls g_PrimarySurface vtable +0x4C from FUN_006DA9D0.` (evidence: `0x006DA9D0`; `get_function_xrefs(0x006DBB60) -> 0x007F43A8 [DATA]`)
- `[RESOLVED] OQ-2 - What calls the selected-building rally renderer? -> TacticalClass_Draw calls FUN_006DA9D0 twice, at 0x006D4648 and 0x006D46CF.` (evidence: `0x006D3D10`)
- `[RESOLVED] OQ-3 - Is this active in standard YR? -> Conditional; standard WF/barracks meet vtable +0x284 eligibility and draw if selected/local/rally target exists.` (evidence: `0x00455DA0`; `rulesmd.ini:11695`, `11777`, `12497`, `12567`, `13175`, `13311`)
- `[RESOLVED] OQ-4 - What gates visibility? -> current-object entry, Building RTTI, selected byte, owner==g_PlayerPtr, +0x284 eligibility, and non-null +0x218 target.` (evidence: `0x006DA9D0`)
- `[RESOLVED] OQ-5 - What is the target coordinate source? -> target pointer at building +0x218, target vtable +0x48, ground-height Z replacement, bridge flag +0x100 Z add.` (evidence: `0x006DA9D0`)
- `[RESOLVED] OQ-6 - What is the source coordinate? -> selected building vtable +0x48, converted by CoordsToClient and viewport scroll/offset adjustment.` (evidence: `0x006DA9D0`)
- `[RESOLVED] OQ-7 - What is the color source? -> first default/background packed color, then owner RGB from House +0x56F9..+0x56FB packed to DD format.` (evidence: `0x006DA9D0`)
- `[RESOLVED] OQ-8 - What is the pulse/dash parameter? -> (0x7FFFFFFF - g_CurrentFrameCounter) % 0xF, passed with &DAT_00842930 to surface slot +0x4C.` (evidence: `0x006DA9D0`)
- `[RESOLVED] OQ-9 - Where does it draw relative to brackets and action lines? -> first pass before object rendering/brackets; second pass after unit-action visuals/bandbox and before selected action lines/capture links.` (evidence: `0x006D4648..0x006D47F6`)
- `[RESOLVED] OQ-10 - Is UnitActionLines required? -> No evidence in this path; `DAT_00843108` is read later for selected-unit action lines, not in FUN_006DA9D0.` (evidence: `0x006DA9D0`; `0x006D473F`)
- `[DEFERRED] OQ-11 - Exact pixel raster semantics of g_PrimarySurface vtable +0x4C` (category: `out-of-scope`; reason: this slot only needed surface/depth behavior enough to distinguish the caller from DrawLine3D; next-step-if-pursued: dedicated Surface +0x4C dashed-line raster investigation)
- `[DEFERRED] OQ-12 - Full role of FUN_006DAD60 marker/link family` (category: `out-of-scope`; reason: adjacent draw-order function, but not the selected-building factory rally-line renderer; next-step-if-pursued: planning/queued waypoint slot should own it)

## Sources

- Ghidra decompiled: `0x006D3D10`, `0x006DA9D0`, `0x006DAD60` (scope separation), `0x006DBB60`, `0x00455DA0`, `0x00443860`, `0x004FBF60`, `0x00459EC0`, `0x0070C610`
- Ghidra xrefs: `get_function_xrefs(0x006DBB60)`, `get_function_xrefs(0x006DA9D0)`
- PE/vtable byte check: `gamemd.exe` VA `0x007E4140` -> dword `0x00455DA0`; VA `0x007F43A8` -> dword `0x006DBB60`
- INI checked: `ini/rulesmd.ini` factory/repair/cloning keys listed in section 4
- Rust scanned: `src/app_context_order.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/world_commands.rs`, `src/ui/in_game_hud.rs`
- Prior context, not ground truth: `docs/research/PLACEMENT_RALLY_WAYPOINT_VISUALS_GHIDRA_REPORT.md`, `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`, `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`
