# TechnoClass::DrawExtras Building Bracket Block - Ghidra Research Report

**Address(es):** `0x006F5190` primary; helpers `0x006F5EF0`, `0x006DBB60`, `0x006D8DB0`, `0x006F60C0`, `0x00459EC0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** front building selection bracket block inside `TechnoClass::DrawExtras`: gates, direct calls, direct line segment set, color/Z-write flag arguments, order boundaries, and standard YR reachability.  
**Non-Scope:** `DrawBehind` back bracket geometry except as context, full health-bar math, full veterancy/pip systems, radial sensor rings, garrison pip scan, band-box selection.  
**Confidence:** High for control-flow gates, call order, helper semantics, standard YR reachability, and current Rust status. Medium for semantic corner names on the three direct line calls because this pass used the existing bracket topology report plus fresh `DrawExtras` decompile rather than clean symbolic stack names.  
**Active in YR:** Yes. `Tactical_ObjectRenderingLoop @ 0x006D8DB0` calls object `vtable+0x110` for drawn objects in the second pass; `BuildingClass::WhatAmI @ 0x00459EC0` returns `6`; no TS-only scenario or fog flag gates the bracket block itself.

## 1. Overview

The scoped block is the front half of selected-building wireframe brackets. It runs inside `TechnoClass::DrawExtras` after Ivan/wrench/veterancy overlays and before the selected health bar. It draws four full two-ended bracket-corner edges, then after a gated empty hook in stock YR, draws three single-ended direct `DrawLine3D` stubs that complete the front visible bracket topology.

Player-visible result: selected buildings get the front/right/top white wireframe bracket stubs above the building sprite. This block is not used for unit, aircraft, or infantry SHP pip bars.

## 2. Key Offsets And Gates

| Offset / slot | Purpose in this slice | Active in YR | Evidence |
|---|---|---|---|
| `TechnoClass+0x3CD` byte | Entry early-out: nonzero skips all `DrawExtras` overlays | Yes | `TechnoClass::DrawExtras @ 0x006F5190` first branch |
| `TechnoClass+0x83` byte | Selected gate for the bracket block and selected health bar | Yes | `0x006F5190` selected branch |
| vtable `+0x2C` | `WhatAmI`; must return `6` to enter building line-bracket block | Yes | `0x006F5190`; `BuildingClass::WhatAmI @ 0x00459EC0` returns `6` |
| vtable `+0x1C8` | Called before building bracket geometry; return selects bracket palette in prior reports | Yes | `0x006F5190`; matching `DrawBehind @ 0x006F60D0` color branch |
| vtable `+0x84`, then type vtable `+0x7C` | Get type, then `BuildingTypeClass::Dimension2`-style dimensions | Yes | `0x006F5190`; prior `SELECTION_BRACKETS_GHIDRA_REPORT.md` |
| vtable `+0x48` | Building coordinate origin used to build bracket points | Yes | `0x006F5190` |
| vtable `+0x448` | Gated hook between the four corner edges and direct-line edges; stock base is empty | Yes, no visible effect in stock YR | `0x006F5190`; `FUN_006F60C0` returns immediately |
| vtable `+0x44C` | Selected health bar call after bracket direct-line block | Yes | `0x006F5190`; `DrawHealthBar @ 0x006F64A0` |

Important negative gate: `GetVisualState() != 5` and `!IsDisguised()` are checked immediately before this block, but only for `DrawVeterancyPips` (`vtable+0x454`). The selected building bracket block itself is not gated by that visual-state test in the decompiled `0x006F5190`.

## 3. Core Logic

For a selected object whose `WhatAmI()` returns `6`, `DrawExtras`:

1. Calls vtable `+0x1C8` before drawing the bracket color path.
2. Gets type dimensions via vtable `+0x84` then type vtable `+0x7C`, halves width and height, and reads the object's vtable `+0x48` coordinate.
3. Checks `WhatAmI() != 0xF`; for real buildings this is always true because `BuildingClass::WhatAmI` returns `6`.
4. Calls `TechnoClass::DrawBracketCorner @ 0x006F5EF0` four times for the front/right edge set.
5. If `Strength > 0` and owner is allied with the local player or `RulesClass+0x17E6 EnemyHealth` is enabled, calls vtable `+0x448`. Stock base implementation `0x006F60C0` returns immediately.
6. Rechecks `WhatAmI()`. For buildings (`!= 0xF`) it takes the direct-line branch and issues three calls through `g_Tactical->vtable+0x60`, i.e. `Tactical::DrawLine3D @ 0x006DBB60`.
7. Calls selected `DrawHealthBar` via vtable `+0x44C`.

The four `DrawBracketCorner` calls are the front two-ended edges:

| Edge | Segment | Active in YR | Evidence |
|---|---|---|---|
| 1 | front ground, `FL_ground -> FR_ground` | Yes | `0x006F5190`; prior topology report |
| 2 | right ground, `BR_ground -> FR_ground` | Yes | `0x006F5190`; prior topology report |
| 3 | front-left vertical, `FL_roof -> FL_ground` | Yes | `0x006F5190`; prior topology report |
| 4 | back-right vertical, `BR_roof -> BR_ground` | Yes | `0x006F5190`; prior topology report |

The three direct `DrawLine3D` calls are single stubs using the same 25% quarter-point formula, but computed inline with vector adds and divide-by-4:

| Direct call | Stub | Active in YR | Evidence |
|---|---|---|---|
| 1 | `FL_roof` 25% toward hidden `FR_roof` | Yes | `0x006F5190` direct `g_Tactical+0x60` call; `SELECTION_BRACKETS_GHIDRA_REPORT.md` |
| 2 | `BR_roof` 25% toward hidden `FR_roof` | Yes | `0x006F5190` direct `g_Tactical+0x60` call; `SELECTION_BRACKETS_GHIDRA_REPORT.md` |
| 3 | `FR_ground` 25% toward `FR_roof` | Yes | `0x006F5190` direct `g_Tactical+0x60` call; `SELECTION_BRACKETS_GHIDRA_REPORT.md` |

`TechnoClass::DrawBracketCorner @ 0x006F5EF0` computes two quarter points: `(3*A+B)/4` and `(A+3*B)/4`, with signed divide-by-4 rounding toward zero after adding the sign correction. It then calls `Tactical::DrawLine3D` twice, choosing endpoint order by Z comparison before each line draw. Active in YR: Yes, directly called by both `DrawExtras` and `DrawBehind`.

`Tactical::DrawLine3D @ 0x006DBB60` projects both world endpoints through `WorldToScreenSub`, subtracts `AdjustForZ(z)` from screen Y, subtracts tactical viewport scroll fields `+0xB0/+0xB4`, computes endpoint depths internally from each endpoint's Z, then calls the primary surface line drawer at surface vtable `+0x34`. In this bracket block, callers pass the fifth argument as `0`; follow-up evidence identifies that argument as the surface Z-write flag, not the line depth. Active in YR: Yes, used by this standard draw path.

## 4. Color And Z-Write Arguments

Color path: building bracket lines normally use the palette-converted color for index `0x0F` (white). Matching `DrawBehind @ 0x006F60D0` shows the same `vtable+0x1C8` return compared against `< -4`, selecting `0x0C` instead of `0x0F` only for that abnormal negative-height path. Prior `SELECTION_BRACKETS_GHIDRA_REPORT.md` notes that the dim `0x0C` path is not reachable for standard BuildingClass content because the relevant return is not below `-4`.

Active in YR: Yes for the normal `0x0F` color; conditional for `0x0C` with evidence from `0x006F60D0` and the matching `0x006F5190` pre-color call.

Z/depth path: all bracket helper and direct line calls pass final flag `0` to `Tactical::DrawLine3D`. Follow-up report `TACTICAL_DRAWLINE3D_BRACKET_DEPTH_ARGUMENTS_GHIDRA_REPORT.md` verifies that `DrawLine3D` computes endpoint depths internally from endpoint Z and forwards the caller's final argument as the surface Z-write flag. Selected building bracket lines therefore Z-test but do not write Z. Active in YR: Yes.

## 5. Integration And Order Boundaries

`Tactical_ObjectRenderingLoop @ 0x006D8DB0` is the standard YR reachability proof. It first draws visible objects and marks `object+0x99`, then loops all display layers again and calls `vtable+0x110` for drawn objects. That invokes `TechnoClass::DrawExtras @ 0x006F5190` for base TechnoClass-derived objects unless overridden.

Inside `DrawExtras`, scoped order is:

1. Ivan bomb clock, if present.
2. Deploy-ready wrench, if building and flag `+0x6E8` set.
3. Veterancy pips, if not disguised and visual state is not `5`.
4. Selected building bracket block: four `DrawBracketCorner` calls.
5. Gated `vtable+0x448` hook; stock base returns immediately.
6. Selected building bracket block continued: three direct `DrawLine3D` single stubs.
7. Selected `DrawHealthBar` (`vtable+0x44C`).
8. Hover health bar branch for non-selected hover.
9. Talk bubble.

The important refinement versus older summaries is step 5: the gated `+0x448` hook sits between the four helper edges and the three direct single-stub edges in the decompiled `0x006F5190`. In stock YR this has no visible bracket effect because `0x006F60C0` is empty.

## 6. INI Keys

No INI key directly gates the bracket block. The block consumes runtime type dimensions that ultimately come from building art data:

| Key | File / path | Effect | Active in YR | Evidence |
|---|---|---|---|---|
| `Foundation` | `ini/artmd.ini`, e.g. `[GACNST] Foundation=4x4` at line 1601 | Width/height for bracket box through type dimensions | Yes | `0x006F5190` type `+0x7C`; prior `Dimension2` report |
| `Height` | `ini/artmd.ini`, e.g. `[GACNST] Height=4` at line 1602 | Z extent for bracket roof | Yes | `0x006F5190` type `+0x7C`; prior `Dimension2` report |
| `EnemyHealth` | `ini/rulesmd.ini:755`, default `yes` | Allows the `+0x448` hook for enemy selected/visible objects when strength > 0; stock hook has no visible effect | Conditional | `0x006F5190`; `FUN_006F60C0` empty |

## 7. Current Rust Implementation Status

Rust has a bracket builder with the same 12-edge topology in `src/app_selection_brackets.rs`: helper stubs at lines 168-172, front `DrawExtras` helper edges at lines 267-275, and direct single-stub equivalents at lines 277-281. It also documents white color and flat overlay depth at lines 37-41.

However, `src/app_render/build_instances.rs:446-449` currently disables bracket instance generation and returns an empty bracket vector. Active parity status: the researched front bracket block is not currently visible in Rust despite a dormant builder existing.

Known mismatch for this exact slice if re-enabled as-is: the Rust builder emits both `DrawBehind` and `DrawExtras` edges into one overlay collection, while gamemd separates back edges before the sprite and front edges in `DrawExtras`. That affects the selected-building silhouette because original back edges can be occluded by the building sprite.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::DrawExtras @ 0x006F5190` selected building bracket block | verified | Fresh Ghidra decompile | none for scoped block |
| Entry selected/building gates | verified | `0x006F5190`, `0x00459EC0` | none |
| Visual-state gate boundary | verified | `0x006F5190` calls `+0x454` before selected bracket branch | none |
| Four `DrawBracketCorner` calls | verified | `0x006F5190`; `0x006F5EF0` | exact stack variable names remain messy but topology is covered by prior report |
| `+0x448` hook between helper and direct edges | verified | `0x006F5190`; `0x006F60C0` empty | none for stock YR |
| Three direct `DrawLine3D` calls | verified | `0x006F5190`; `0x006DBB60`; prior topology report | none for scoped topology |
| Color/Z-write arguments | verified | `0x006F60D0`, `0x006F5EF0`, `0x006DBB60`, `TACTICAL_DRAWLINE3D_BRACKET_DEPTH_ARGUMENTS_GHIDRA_REPORT.md` | dim-color live content not found in this slot |
| Standard YR reachability | verified | `0x006D8DB0`; `0x00459EC0` | none |
| Full health-bar internals | touched-not-exhausted | `0x006F64A0` decompiled only as order boundary | out of scope |
| Full veterancy internals | touched-not-exhausted | `0x006F5190` order boundary; prior report | out of scope |

## 9. Open Questions - Final State

[RESOLVED] OQ-TDE-BB-001 - Is the block active in standard YR? Yes: object rendering loop calls `vtable+0x110` for drawn objects; buildings return `WhatAmI()==6`; no TS-only gate wraps the selected bracket block. Evidence: `0x006D8DB0`, `0x00459EC0`, `0x006F5190`.

[RESOLVED] OQ-TDE-BB-002 - Does the `GetVisualState()!=5` gate suppress building brackets? No; that gate applies to the preceding veterancy call, not the selected bracket branch. Evidence: `0x006F5190`.

[RESOLVED] OQ-TDE-BB-003 - Are the three single-stub edges direct `DrawLine3D` calls rather than `DrawBracketCorner` calls? Yes; the building non-infantry continuation calls `g_Tactical->vtable+0x60` three times after inline quarter-point construction. Evidence: `0x006F5190`, `0x006DBB60`.

[RESOLVED] OQ-TDE-BB-004 - Does anything draw between the four helper edges and three direct edges? Yes, a strength/alliance/EnemyHealth-gated `vtable+0x448` hook, but stock base `0x006F60C0` returns immediately. Evidence: `0x006F5190`, `0x006F60C0`.

[RESOLVED] OQ-TDE-BB-005 - What comes immediately after the direct bracket lines? The selected health bar call through vtable `+0x44C`. Evidence: `0x006F5190`; `0x006F64A0`.

[DEFERRED] OQ-TDE-BB-006 - Do any stock subclasses override `vtable+0x448`? Deferred as out-of-scope because the slot target for base TechnoClass is confirmed empty and this slot requested the building bracket block, not all subclass vtable audits. Next step: targeted vtable override scan if an implementer wants to model the hook.

## Sources

- Ghidra decompiled: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompiled: `TechnoClass::DrawBracketCorner @ 0x006F5EF0`
- Ghidra decompiled: `Tactical::DrawLine3D @ 0x006DBB60`
- Ghidra decompiled: `Tactical_ObjectRenderingLoop @ 0x006D8DB0`
- Ghidra decompiled: `TechnoClass::DrawHealthBar @ 0x006F64A0`
- Ghidra decompiled: `TechnoClass::DrawBehind @ 0x006F60D0`
- Ghidra decompiled: empty hook `0x006F60C0`
- Ghidra decompiled: `BuildingClass::WhatAmI @ 0x00459EC0`
- Prior docs: `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`, `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini:755`, `ini/artmd.ini:1599-1602`
- Rust checked: `src/app_selection_brackets.rs`, `src/app_render/build_instances.rs`
