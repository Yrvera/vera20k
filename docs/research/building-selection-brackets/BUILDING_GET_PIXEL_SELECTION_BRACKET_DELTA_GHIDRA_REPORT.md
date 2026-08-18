# Building GetPixelSelectionBracketDelta Slot - Ghidra Research Report

**Address(es):** `0x004263C0` (requested), `0x00459ED0` (BuildingClass vtable slot `+0x90`), `0x006F60D0`, `0x006F5190`, `0x006F64A0`, `0x00464AF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact contract for the requested `0x004263C0` function, the BuildingClass vtable slot that prior docs named `GetPixelSelectionBracketDelta`, and whether either value is consumed by building selection brackets.  
**Non-Scope:** complete 12-edge bracket topology, full health bar behavior, full BuildingTypeClass field map, or runtime pixel screenshots.  
**Confidence:** High for the negative finding that `0x004263C0` / BuildingClass slot `+0x90` do not provide building bracket offsets; Medium for parser-side INI mapping because this report did not re-walk the entire `TechnoTypeClass::ReadINI` body.  
**Active in YR:** Conditional. The functions and vtable slots exist in standard YR, but the requested function/slot is not on the active building selection-bracket geometry path. The line bracket path is active for selected buildings.

## 1. Overview

The requested address `0x004263C0` is not a building selection-bracket offset function. It returns the global wide string `"No name"` at `0x008182E0`, has only vtable data references in the checked xrefs, and is not called by the building bracket code.

BuildingClass does not use `0x004263C0` in its own vtable slot `+0x90`; it uses `0x00459ED0`, which returns `*(this + 0x520) + 0x60`. That is consistent with a type-name/string field, not a pixel delta or bracket geometry field.

The actual selected-building line brackets are produced by `TechnoClass::DrawBehind @ 0x006F60D0` and `TechnoClass::DrawExtras @ 0x006F5190`. Those functions use `WhatAmI()==6`, selected state `this+0x83`, `GetType()->Dimension2()`, and `GetCoords()`; they do not read `BuildingTypeClass+0x1538`, `SelectBracketOffsetX/Y`, or `TechnoTypeClass+0x3E0` for line geometry.

## 2. Key Offsets And Slots

| Item | Offset/address | Verified behavior | Active in YR |
|---|---:|---|---|
| Requested function | `0x004263C0` | Returns wide string pointer `0x008182E0` (`"No name"`). Xrefs checked were vtable data refs only. | Conditional: present in standard vtables, not active in building bracket path. Evidence: decompile `0x004263C0`, xrefs from vtable data only. |
| BuildingClass vtable slot named by prior docs | vtable `+0x90` -> `0x00459ED0` | Returns `*(this + 0x520) + 0x60`; no geometry math. | Conditional: BuildingClass vtable slot exists, not active in bracket path. Evidence: decompile `0x00459ED0`; data xref `0x007E3F4C`. |
| Bracket color/height query | vtable `+0x1C8` -> `0x005F5F40` for Techno/Building vtables checked | `ObjectClass::GetHeight`: returns object Z minus ground height, bridge-adjusted. Used only for `< -4` palette index choice in selected-building line bracket code. | Yes for selected buildings. Evidence: `DrawBehind @ 0x006F60D0`, `DrawExtras @ 0x006F5190`, vtable reads at `0x007E8E5C` and `0x007E4084`. |
| Building dimensions | `BuildingTypeClass::Dimension2 @ 0x00464AF0` | Returns `{FoundationWidthTable[foundation]<<8, FoundationHeightTable[foundation]<<8, Height*g_HeightFactor}`. | Yes for selected building brackets and building health pips. Evidence: decompile `0x00464AF0`; calls from `0x006F60D0`, `0x006F5190`, `0x006F64A0`. |
| `PixelSelectionBracketDelta` | `TechnoTypeClass+0x3E0` | Read directly by `TechnoClass::DrawHealthBar @ 0x006F64A0` only on non-building paths for PIPBRD/pip Y offsets. | Yes for non-building pips/bracket sprites; no for building line geometry. Evidence: `0x006F64A0` reads `GetType()+0x3E0` in infantry/vehicle branches, not building branch. |
| Claimed `BuildingTypeClass+0x1538` | `+0x1538..+0x1547` | Prior constructor-defaults doc identifies this as a zeroed 16-byte orphan block, not `SelectBracketOffsetX/Y`. No `SelectBracketOffset` INI string was found in repo INI or retail path grep. | No evidence of bracket activity in this slice. Evidence: no reads in decompiled bracket functions; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`; local grep. |

## 3. Core Logic

For selected buildings, the active bracket path is:

1. `DrawBehind @ 0x006F60D0` gates on `WhatAmI()==6` and `this+0x83 != 0`.
2. It calls vtable `+0x1C8` and uses `< -4` only to choose palette index `0x0C`; otherwise it uses `0x0F`.
3. It calls `GetType()` then `Dimension2()` and `GetCoords()`.
4. It draws five back/left bracket edges through `DrawBracketCorner @ 0x006F5EF0`.
5. `DrawExtras @ 0x006F5190` repeats the selected-building gate, calls vtable `+0x1C8` as a side-effect/color-path query, then uses `GetType()->Dimension2()` plus `GetCoords()` for front/right bracket geometry.
6. `DrawExtras` then calls `DrawHealthBar @ 0x006F64A0` for the selected object; the building branch of `DrawHealthBar` again uses `Dimension2`, not `PixelSelectionBracketDelta`.

There is no call to vtable `+0x90` and no use of the value returned by `0x004263C0` or `0x00459ED0` in the checked building bracket functions.

## 4. INI Keys

| Key | File evidence | Binary effect in this slice | Active in YR |
|---|---|---|---|
| `PixelSelectionBracketDelta` | `ini/rulesmd.ini:3913`, `5150`, `5262`, `6170`, `6365`; string at `0x00843DC0` | Stored on TechnoType and read at `+0x3E0` by non-building branches in `DrawHealthBar @ 0x006F64A0`. It does not affect building line bracket geometry. | Yes for non-buildings; no for building line geometry. |
| `SelectBracketOffsetX/Y` | No matches in repo INI or retail path grep; not present near the `PixelSelectionBracketDelta` string block checked in `gamemd.exe`. | No binary read found in `0x004263C0`, `0x00459ED0`, `0x006F60D0`, `0x006F5190`, `0x006F64A0`, or `0x00464AF0`. | No evidence in this slice. |
| `Foundation` | art/rules merge in Rust; binary `Dimension2 @ 0x00464AF0` uses `BuildingTypeClass+0xEF0` as foundation index. | Drives bracket X/Y extent through foundation width/height tables. | Yes. |
| `Height` | prior selection docs and `Dimension2 @ 0x00464AF0` use `BuildingTypeClass+0xEF4`. | Drives bracket Z extent as `Height * g_HeightFactor`. | Yes. |

## 5. Integration Points

| Function | Role | Active in YR |
|---|---|---|
| `0x004263C0` | Default vtable slot function returning `"No name"`; not a bracket offset provider. | Conditional: present, not bracket-active. |
| `0x00459ED0` | BuildingClass vtable `+0x90`, returns `Type+0x60`; not bracket-active. | Conditional: present, not bracket-active. |
| `0x006F60D0` | Draws selected-building back bracket edges before/behind body rendering. | Yes, when `WhatAmI()==6` and selected. |
| `0x006F5190` | Draws selected-building front bracket edges, then selected health bar/pips. | Yes, when selected and not sinking. |
| `0x006F64A0` | Building branch draws diagonal health pips from `Dimension2`; non-building branches use `TechnoType+0x3E0`. | Yes, class-conditional. |
| `0x00464AF0` | Supplies foundation width, height, and Z extent. | Yes. |

## 6. Current Rust Implementation Status

Rust parses `PixelSelectionBracketDelta` into `ObjectType.pixel_selection_bracket_delta` at `src/rules/object_type.rs:231` and `src/rules/object_type.rs:844`.

Rust applies that field to non-building PIPBRD and pip Y offsets at `src/app_ui_overlays.rs:421` and `src/app_ui_overlays.rs:513`, matching the binary's non-building usage direction.

The building line bracket builder exists in `src/app_selection_brackets.rs:175`, but `src/app_render/build_instances.rs:446` currently disables it and emits `Vec::new()` at `src/app_render/build_instances.rs:449`.

## 7. Prior-Doc Conflict Resolution

| Prior claim | Resolution | Evidence | Active in YR |
|---|---|---|---|
| `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` says `BuildingClass::GetPixelSelectionBracketDelta (0x004263C0) -> uses Type.Foundation + Type+0x1538 SelectBracketOffsetX/Y`. | Stale/wrong for this binary. `0x004263C0` returns `"No name"` and is not BuildingClass's vtable entry. BuildingClass slot `+0x90` is `0x00459ED0`, returning `Type+0x60`. | Ghidra `0x004263C0`, `0x00459ED0`; xrefs. | No for bracket path. |
| `SELECTION_BRACKETS_GHIDRA_REPORT.md` says `PixelSelectionBracketDelta` is read via `vtable+0x1C8` and for buildings affects color selection. | Stale/wrong as worded. `vtable+0x1C8` is `ObjectClass::GetHeight`, not `PixelSelectionBracketDelta`. The `< -4` color choice is height-based. | Ghidra `0x005F5F40`, `0x006F60D0`, `0x006F5190`. | Yes for height color gate; no for PixelSelectionBracketDelta. |
| `HEALTH_BAR_POSITIONING.md` says buildings do not use `PixelSelectionBracketDelta`. | Confirmed for building line brackets and building health pips. | Ghidra `0x006F64A0` building branch does not read `+0x3E0`. | Yes. |

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x004263C0` requested function | verified | Ghidra decompile; read memory `0x008182E0` = `"No name"` | none for this slice |
| BuildingClass vtable `+0x90` | verified | Ghidra read at `0x007E3F4C`; decompile `0x00459ED0` | none for this slice |
| Techno/Building vtable `+0x1C8` | verified | Ghidra read at `0x007E8E5C` / `0x007E4084`; decompile `0x005F5F40` | none for this slice |
| `DrawBehind @ 0x006F60D0` consumption | verified | decompile `0x006F60D0` | full edge topology belongs to slot 3/5 |
| `DrawExtras @ 0x006F5190` consumption | verified | decompile `0x006F5190` | full edge topology belongs to slot 4/5 |
| `DrawHealthBar @ 0x006F64A0` building vs non-building usage | verified | decompile `0x006F64A0` | none for this slice |
| `BuildingTypeClass::Dimension2 @ 0x00464AF0` | verified | decompile `0x00464AF0` | foundation table enumeration deferred to slot 2 |
| `SelectBracketOffsetX/Y` INI/key claim | verified negative for this slice | no local INI/retail path grep hits; not read by checked Ghidra functions | a global string-xref audit could confirm absence beyond this slice |
| Rust status | touched-not-exhausted | `src/rules/object_type.rs`, `src/app_ui_overlays.rs`, `src/app_selection_brackets.rs`, `src/app_render/build_instances.rs` | parity assessment belongs to implementation/disparity slot |

## 9. Open Questions - Final State

- `[RESOLVED] OQ1 - Does 0x004263C0 use foundation or SelectBracketOffset fields? -> No; it returns global string pointer 0x008182E0 ("No name").` (evidence: Ghidra `0x004263C0`, memory `0x008182E0`)
- `[RESOLVED] OQ2 - Is 0x004263C0 the BuildingClass vtable +0x90 implementation? -> No; BuildingClass vtable +0x90 points to 0x00459ED0.` (evidence: data xref `0x007E3F4C`, decompile `0x00459ED0`)
- `[RESOLVED] OQ3 - Is vtable +0x1C8 PixelSelectionBracketDelta? -> No; checked Techno/Building vtables point to ObjectClass::GetHeight @ 0x005F5F40.` (evidence: data refs `0x007E8E5C`, `0x007E4084`, decompile `0x005F5F40`)
- `[RESOLVED] OQ4 - Where is PixelSelectionBracketDelta consumed? -> Non-building branches in DrawHealthBar read GetType()+0x3E0 for PIPBRD/pip Y offsets.` (evidence: Ghidra `0x006F64A0`)
- `[RESOLVED] OQ5 - Do building line brackets read TechnoType+0x3E0? -> No; DrawBehind/DrawExtras building branches use Dimension2 and GetCoords, not +0x3E0.` (evidence: Ghidra `0x006F60D0`, `0x006F5190`)
- `[RESOLVED] OQ6 - Do building health pips read TechnoType+0x3E0? -> No; DrawHealthBar building branch uses Dimension2-derived edge projection.` (evidence: Ghidra `0x006F64A0`)
- `[RESOLVED] OQ7 - Does BuildingTypeClass+0x1538 have verified SelectBracketOffset semantics here? -> No; prior ctor-defaults doc marks it as a zeroed orphan 16-byte block, and checked bracket functions do not read it.` (evidence: `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, Ghidra `0x006F60D0`, `0x006F5190`)
- `[RESOLVED] OQ8 - Is the selected-building bracket path active in YR? -> Yes when object RTTI is building (`WhatAmI()==6`) and selected (`this+0x83 != 0`).` (evidence: Ghidra `0x006F60D0`, `0x006F5190`)
- `[RESOLVED] OQ9 - Is the color threshold linked to PixelSelectionBracketDelta? -> No; the threshold uses `GetHeight() < -4` through vtable +0x1C8.` (evidence: Ghidra `0x005F5F40`, `0x006F60D0`, `0x006F5190`)
- `[RESOLVED] OQ10 - Is there a repo/retail INI key named SelectBracketOffsetX/Y? -> No matches in searched local INI and retail path.` (evidence: local grep)
- `[DEFERRED] OQ11 - Does any unrelated non-bracket system use BuildingTypeClass+0x1538?` (category: out-of-scope; reason: this slot only resolves building selection bracket offset contract; next-step-if-pursued: run a BuildingTypeClass field xref investigation)
- `[DEFERRED] OQ12 - Does runtime display ever hit the `GetHeight() < -4` dim-color path for standard selected buildings?` (category: needs-runtime-debugger; reason: binary path exists but this slot did not attach to runtime states such as underground/warped buildings; next-step-if-pursued: watch selected building DrawBehind/DrawExtras with negative height)

## Sources

- Ghidra decompiled: `0x004263C0`, `0x00459ED0`, `0x005F5F40`, `0x006F60D0`, `0x006F5190`, `0x006F64A0`, `0x00464AF0`, `0x006F5EF0`, `0x006DBB60`.
- Ghidra memory/xrefs: `0x008182E0`, `0x00843DC0`, `0x007E3F4C`, `0x007E4084`, `0x007E8D24`, `0x007E8E5C`.
- Prior docs checked: `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `HEALTH_BAR_POSITIONING.md`, `BUILDINGCLASS_VTABLE_COMPLETE.md`, `TECHNOCLASS_VTABLE_COMPLETE.md`, `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`.
- INI/source checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`; retail path grep under `C:/Users/enok/Documents/Command and Conquer Red Alert II/`; `src/rules/object_type.rs`, `src/app_ui_overlays.rs`, `src/app_selection_brackets.rs`, `src/app_render/build_instances.rs`.
