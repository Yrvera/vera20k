# TechnoClass::DrawBehind Building Bracket Back Edges - Ghidra Research Report

**Address(es):** `0x006F60D0` primary, `0x006F5EF0` bracket edge helper, `0x006DBB60` 3D line drawer, `0x006D8DB0` object render loop, `0x0043CEA0` BuildingClass draw dispatcher, `0x0043D290` BuildingClass body draw, `0x00464AF0` BuildingTypeClass::Dimension2  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** building-only `TechnoClass::DrawBehind` path that draws the back/hidden selection-bracket edges before the building sprite.  
**Non-Scope:** front edges in `DrawExtras`, health bars, pips, hover overlays, target/action lines, and runtime screenshot matching.  
**Confidence:** High for entry gates, call order, BuildingClass reachability, helper/depth/color call arguments, and five-edge topology. Medium for naming of individual decompiler temporaries because Ghidra aliases stack locals in `0x006F60D0`.  
**Active in YR:** Yes. The path is reached from the standard object render loop for visible selected buildings; no TS-only gate is present inside `0x006F60D0`.

## 1. Overview

`TechnoClass::DrawBehind` is the pre-sprite half of the building selection bracket renderer. It only does useful work when the object reports `WhatAmI() == 6` and `this+0x83` is nonzero, then emits five `DrawBracketCorner` edges that belong behind the building artwork.

The visible player effect is that the rear/left bracket stubs are submitted before `BuildingClass::DrawIt`, so the building sprite occludes the ground/back portions and only the appropriate roof-side stubs remain visible around the selected structure.

## 2. Key Offsets And Inputs

| Field / source | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `vtable+0x2C` | class kind / `WhatAmI`; DrawBehind requires `6`, then rejects `0xF` | Ghidra `0x006F60D0` first two virtual calls | Yes - selected buildings report `6`; infantry `0xF` is explicitly excluded |
| `this+0x83` | selected flag | Ghidra `0x006F60D0` checks byte before any geometry | Yes - standard selection state gate |
| `vtable+0x1C8` | height-like value used only for dim color threshold | Ghidra `0x006F60D0`, compare result `< -4` | Conditional - active when value is below `-4`; normal buildings do not take dim branch per prior bracket docs |
| `vtable+0x84`, then type `vtable+0x7C` | fetch type and call `Dimension2` | Ghidra `0x006F60D0`; `0x00464AF0` decompile | Yes - building geometry is type/art driven |
| `vtable+0x48` | get center/render coordinate used as bracket origin | Ghidra `0x006F60D0` before edge construction | Yes - standard render coordinate path |
| `g_PaletteData + 0x174` | converts palette index to surface color, byte or ushort by surface mode | Ghidra `0x006F60D0` format branch | Yes - standard surface color conversion |

## 3. Entry Gates

| Gate | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `WhatAmI() == 6` | Non-buildings skip the entire function body | Ghidra `0x006F60D0` | Yes - building-only bracket path |
| `this+0x83 != 0` | Unselected buildings skip all back edges | Ghidra `0x006F60D0` | Yes - selected-state visual |
| second `WhatAmI() != 0xF` | Infantry branch is rejected before geometry | Ghidra `0x006F60D0` | Yes, but redundant after `== 6`; it prevents shared Techno code from using building geometry on infantry |
| `GetHeight() < -4` | palette index becomes `0x0C`; otherwise `0x0F` | Ghidra `0x006F60D0` | Conditional - normal selected buildings use `0x0F`; below-threshold objects use dim color if reachable |

No `SpecialFlags`, fog-of-war mode flag, scenario TS-legacy flag, or INI option gates this function directly. Visibility and viewport culling happen in the caller before dispatch.

## 4. Back Edge Set

The five calls to `TechnoClass::DrawBracketCorner` in `0x006F60D0` correspond to the back/left portion of the building 3D box. Names below use the existing selection-bracket convention: `FL/FR/BL/BR` on ground and roof, with `BL` being the rear/top-screen corner.

| Edge | Endpoints | Visual role | Evidence | Active in YR |
|---|---|---|---|---|
| 1 | `BL ground -> BL roof` | back-left vertical | Five sequential `DrawBracketCorner` calls in Ghidra `0x006F60D0`; topology cross-check `SELECTION_BRACKETS_GHIDRA_REPORT.md` | Yes - selected buildings |
| 2 | `BR ground -> BL ground` | back ground edge | same | Yes - selected buildings |
| 3 | `BL ground -> FL ground` | left ground edge | same | Yes - selected buildings |
| 4 | `FL roof -> BL roof` | left roof edge | same | Yes - selected buildings |
| 5 | `BR roof -> BL roof` | back roof edge | same | Yes - selected buildings |

`DrawBehind` does not draw the front ground, right ground, front-left vertical, back-right vertical, front roof, right roof, or front-right vertical stubs. Those belong to `DrawExtras` and are out of this slot's scope.

## 5. Palette And Line Arguments

`DrawBehind` selects a palette index before constructing edges:

| Case | Palette index | Evidence | Active in YR |
|---|---:|---|---|
| normal | `0x0F` | Ghidra `0x006F60D0` initializes color index before `GetHeight` compare | Yes |
| dim / below threshold | `0x0C` | Ghidra `0x006F60D0`, branch when `vtable+0x1C8` result `< -4` | Conditional |

The palette index is converted through `g_PaletteData+0x174`: byte read when the palette/surface mode field at `g_PaletteData+4` equals `1`, otherwise ushort read at `index * 2`. The converted value is the color argument passed to `DrawBracketCorner`.

`DrawBracketCorner @ 0x006F5EF0` computes quarter points and calls `g_Tactical->vtable+0x60` twice per edge. The fourth argument is the converted color; the fifth argument is `0`. `Tactical::DrawLine3D @ 0x006DBB60` projects both 3D endpoints, applies `Tactical::AdjustForZ` to screen Y, computes endpoint depths internally from the endpoint Z values, and forwards the color to `g_PrimarySurface->vtable+0x34` (`Draw_Line`). The zero fifth argument is the surface Z-write flag, so selected building bracket lines Z-test but do not write Z.

**Active in YR:** Yes - these helpers are reached from selected building `DrawBehind` and share the standard tactical surface line path.

## 6. Call Order And Reachability

`Tactical_ObjectRenderingLoop @ 0x006D8DB0` performs the standard object pass. For visible objects in the sprite loop it marks `this+0x99 = 1`, then calls:

1. `vtable+0x10C`
2. `vtable+0x104`

For BuildingClass, the vtable contains:

| Slot | Function | Evidence | Active in YR |
|---|---|---|---|
| `+0x104` | `0x0043CEA0` BuildingClass draw dispatcher | `BUILDINGCLASS_VTABLE_COMPLETE.md`; Ghidra `BuildingClass__Constructor @ 0x0043B710` installs `vtable_BuildingClass` | Yes |
| `+0x10C` | `0x006F60D0` inherited `TechnoClass::DrawBehind` | `BUILDINGCLASS_VTABLE_COMPLETE.md`; Ghidra caller dispatch at `0x006D8DB0` | Yes |
| `+0x110` | `0x006F5190` inherited `TechnoClass::DrawExtras` | `BUILDINGCLASS_VTABLE_COMPLETE.md`; Ghidra second loop dispatch at `0x006D8DB0` | Yes |
| `+0x114` | `0x0043D290` `BuildingClass::DrawBody` | `BUILDINGCLASS_VTABLE_COMPLETE.md`; Ghidra `0x0043CEA0` dispatches this on pass flag `0` | Yes |

Thus the standard selected-building order is:

1. `TechnoClass::DrawBehind @ 0x006F60D0` - five back bracket edges.
2. `BuildingClass::DrawIt / dispatcher @ 0x0043CEA0`.
3. `BuildingClass::DrawBody @ 0x0043D290` when the dispatcher takes the SHP body pass.
4. Later, the object loop's second pass calls `vtable+0x110` (`DrawExtras`) for overlays/front bracket pieces.

**Active in YR:** Yes - no TS-only branch is required for this order. The caller has optional scenario visibility checks for other object categories, but the direct visible-object path dispatches `+0x10C` before `+0x104`.

## 7. Current Rust Implementation Status

| Area | Status | Evidence | Active in YR target |
|---|---|---|---|
| 12-edge building bracket geometry exists | implemented but disabled | `src/app_selection_brackets.rs` builds all 12 edges; `src/app_render/build_instances.rs` currently returns `Vec::new()` for brackets | Yes target, not currently active in Rust |
| Back edges split behind body | not implemented as a separate render phase | `src/app_selection_brackets.rs` emits back and front edges into one overlay list; `src/app_render/draw_passes.rs` draws `selection_brackets` in UI no-depth pass | Yes target |
| Palette dim threshold | not implemented | `src/app_selection_brackets.rs` uses fixed white RGBA | Conditional target |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::DrawBehind @ 0x006F60D0` entry gates | verified | Ghidra decompile | none |
| `TechnoClass::DrawBehind @ 0x006F60D0` five back edges | verified | Ghidra decompile plus prior topology report cross-check | exact stack-temporary names are decompiler-aliased, but endpoint topology is resolved |
| `DrawBracketCorner @ 0x006F5EF0` argument behavior | verified | Ghidra decompile | none |
| `Tactical::DrawLine3D @ 0x006DBB60` projection and surface call | verified | Ghidra decompile plus `TACTICAL_DRAWLINE3D_BRACKET_DEPTH_ARGUMENTS_GHIDRA_REPORT.md` | endpoint depths are computed internally; bracket path passes final Z-write flag `0` |
| `Tactical_ObjectRenderingLoop @ 0x006D8DB0` order | verified | Ghidra decompile | none |
| `BuildingClass::DrawIt @ 0x0043CEA0` to `DrawBody @ 0x0043D290` relation | verified | Ghidra decompile | none |
| TS legacy gate check | verified | absence of direct TS/SpecialFlags checks in `0x006F60D0`; standard caller path in `0x006D8DB0` | none |

## 9. Open Questions - Final State

[RESOLVED] OQ-TDB-001 - Is `DrawBehind` building-only? Yes: `0x006F60D0` requires `WhatAmI()==6` and selected byte `+0x83`. Active in YR: Yes.  
[RESOLVED] OQ-TDB-002 - Does `DrawBehind` run before the building body? Yes: `0x006D8DB0` calls `vtable+0x10C` before `vtable+0x104`; BuildingClass `+0x104` dispatches `+0x114` `DrawBody`. Active in YR: Yes.  
[RESOLVED] OQ-TDB-003 - Which edges are in this function? Five back/left box edges listed in section 4. Active in YR: Yes.  
[RESOLVED] OQ-TDB-004 - What color is used? Palette index `0x0F`, or `0x0C` when `GetHeight()< -4`, converted through `g_PaletteData+0x174`. Active in YR: Yes / Conditional.  
[RESOLVED] OQ-TDB-005 - Is this TS-only or optional legacy? No direct TS-only gate was found in `0x006F60D0`, and the standard object render loop reaches it for visible selected buildings. Active in YR: Yes.  
[RESOLVED] OQ-TDB-006 - Exact final `Surface::Draw_Line` depth parameter semantics after `Tactical::DrawLine3D`'s internal `ftol` expression. Follow-up report `TACTICAL_DRAWLINE3D_BRACKET_DEPTH_ARGUMENTS_GHIDRA_REPORT.md` verifies that endpoint depths are computed internally from endpoint Z, while the caller's final argument is the surface Z-write flag. Selected building bracket callers pass `0`, so they Z-test but do not write Z.

## Sources

- Ghidra decompile: `TechnoClass::DrawBehind @ 0x006F60D0`
- Ghidra decompile: `TechnoClass::DrawBracketCorner @ 0x006F5EF0`
- Ghidra decompile: `Tactical::DrawLine3D @ 0x006DBB60`
- Ghidra decompile: `Tactical_ObjectRenderingLoop @ 0x006D8DB0`
- Ghidra decompile: `BuildingClass::DrawIt dispatcher @ 0x0043CEA0`
- Ghidra decompile: `BuildingClass::DrawBody @ 0x0043D290`
- Ghidra decompile: `BuildingTypeClass::Dimension2 @ 0x00464AF0`
- Prior docs checked: `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`, `BUILDINGCLASS_VTABLE_COMPLETE.md`
- INI checked for related defaults only: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`
