# TechnoClass::DrawHealthBar Building Branch - Ghidra Research Report

**Address(es):** `0x006F64A0` primary; related `0x006F5190`, `0x00464AF0`, `0x006D1F10`, `0x005F5C60`, `0x005F5D20`, `0x005F5CD0`, `0x004AED70`, `0x00709A90`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** building (`WhatAmI()==6`) branch of `TechnoClass::DrawHealthBar`: health pip anchor, count, filled/empty split, frame selection, draw order, selected/hovered integration, and the post-health pip-scale visibility gate in this function.  
**Non-Scope:** non-building health bars except branch separation, full `DrawPipScalePips` internals, garrison occupant-pip pass `0x00430AC0`, sensor/gap rings, and runtime screenshot matching.  
**Confidence:** High for the scoped building branch.  
**Active in YR:** Yes. Evidence: `TechnoClass::DrawExtras @ 0x006F5190` calls vtable `+0x44C`, which resolves to `0x006F64A0`; building path is gated only by `WhatAmI()==6` inside `0x006F64A0`, with no TS-only flag.

## 1. Overview

`TechnoClass::DrawHealthBar` draws selected or hover building health as individual `PIPS.SHP` sprites along the building's upper-left isometric foundation edge. Buildings do not use `PIPBRD.SHP`; they derive pip geometry from `BuildingTypeClass::Dimension2`, draw filled pips first, then draw empty pips for the remaining slots.

After the building health pips, the same branch optionally calls vtable `+0x450` (`DrawPipScalePips`) to draw cargo/occupant/mind-control style pips. That later call has alliance, detection, `CanBeOccupied`, and `PipsDrawForAll` gates; those gates do not suppress the health pips themselves.

## 2. Key Offsets And Globals

| Field / global | Offset / address | Use | Active in YR |
|---|---:|---|---|
| RTTI / `WhatAmI` vtable slot | `+0x2C` | selects building branch when return value is `6` | Yes - checked at `0x006F64AB..0x006F64B1` |
| Selected byte | `TechnoClass+0x83` | caller selected gate, not read in building branch itself | Yes - selected call site in `0x006F5190` |
| Hover byte | `TechnoClass+0x431` | caller hover-health gate for non-selected objects | Yes - hover call site in `0x006F5190` |
| Owner house | `TechnoClass+0x21C` | alliance check before optional `DrawPipScalePips` | Yes - read at `0x006F6782` |
| House detection bitmask | `TechnoClass+0x210` | local-player bit can allow optional `DrawPipScalePips` | Conditional - read at `0x006F67AA`; depends on object/player visibility state |
| Building type pointer | `BuildingClass+0x520` | reads `CanBeOccupied` at type `+0x157B` | Yes - `0x006F67D6..0x006F67E6` |
| `PipsDrawForAll` | `TechnoTypeClass+0x3D8` | bypasses optional pip-scale visibility gate | Conditional - read at `0x006F67F2`; standard YR has at least `[YAPSYT] PipsDrawForAll=yes` in `ini/rulesmd.ini:13714` |
| `Dimension2` type vtable slot | type vtable `+0x7C` | source of foundation width/height/Z | Yes - call at `0x006F64D4`; bound to `BuildingTypeClass::Dimension2 @ 0x00464AF0` |
| `PIPS.SHP` pointer | `0x00AC147C` | building health pip sprites | Yes - used by `CC_Draw_Shape` calls at `0x006F66A6` and `0x006F6722` |
| `ConditionYellow` / `ConditionRed` | Rules `+0x1700` / `+0x1708` | thresholds via `IsYellowHP` / `IsRedHP` | Yes - defaults `50%` and `25%` in `ini/rulesmd.ini:752-753` |

## 3. Core Building Logic

Building branch entry:

```text
if this.WhatAmI() != 6:
    use non-building branch
else:
    draw building health pips
```

The branch calls vtable `+0x1C8` at `0x006F64BB` before reading dimensions. In this function's building branch, the return value is not consumed by later pip geometry or color decisions. Active in YR: Yes for the call; no verified effect inside this branch.

Dimension source:

```text
dims = this.GetType().Dimension2()
width_leptons  = dims.x
height_leptons = dims.y
z_leptons      = dims.z
half = (width_leptons / 2, height_leptons / 2, z_leptons / 2)
```

The signed `/2` sequence uses `CDQ; SUB EAX,EDX; SAR 1` at `0x006F64E3..0x006F64FA`; dimensions are nonnegative in normal data, so this is ordinary integer half. Active in YR: Yes.

Three offsets are projected through `CoordsToClient @ 0x006D1F10`:

```text
A = project((-half.x, +half.y, dims.z))
B = project((-half.x, -half.y, dims.z))
C = project((-half.x, +half.y, 0))
```

`C` is projected but is not used by the final health pip loops; the health pip anchor uses `A`. Active in YR: Yes. Evidence: calls at `0x006F6536`, `0x006F656E`, `0x006F659E`, later loop reads the first projection's X/Y.

Pip count:

```text
total_pips = (A.y - B.y) / 2
```

For normal rectangular foundations, this reduces to `floor(foundation_height * 15 / 2)` because X and Z cancel and Y spans one full foundation height. There is no explicit `max(1)` on `total_pips`; if the projected span produces `0`, both draw loops are skipped. Active in YR: Yes. Evidence: `0x006F65BC..0x006F65CF`.

Filled count:

```text
raw_filled = Math__ftol(ObjectClass::GetHealthRatio(this) * total_pips)
filled = raw_filled
if filled < 2:
    filled = 1
if filled >= total_pips:
    filled = total_pips
```

For any positive `total_pips`, at least one pip is drawn as filled even when the ratio product rounds to zero. If `total_pips` is zero, the later clamp sets `filled` back to zero. Active in YR: Yes. Evidence: `GetHealthRatio @ 0x005F5C60`, `FIMUL [total_pips]` at `0x006F65DC`, `Math__ftol @ 0x007C5F00`, clamps at `0x006F65E7..0x006F6602`.

Health frame:

```text
frame = 1  // green
if IsYellowHP(this):
    frame = 2
else if IsRedHP(this):
    frame = 4
```

`IsYellowHP @ 0x005F5D20` is true when `ConditionRed < health_ratio <= ConditionYellow`. `IsRedHP @ 0x005F5CD0` is true when `health_ratio <= ConditionRed` and `Health > 0`. Therefore exactly-zero health does not select the red frame if the branch is reached; normally dead buildings should no longer render through this path. Active in YR: Yes / Conditional for zero-health edge. Evidence: `0x006F6604..0x006F6630`, helper decompiles, `ini/rulesmd.ini:752-753`.

Pip positions:

```text
for i in 0 .. filled-1:
    pos.x = location.x + A.x + 3 + total_pips * 4 - i * 4
    pos.y = location.y + A.y + 4 - total_pips * 2 + i * 2
    draw PIPS.SHP frame

for i in filled .. total_pips-1:
    same position formula
    draw PIPS.SHP frame 0
```

This walks the pip row left by 4 pixels and down by 2 pixels per slot. Active in YR: Yes. Evidence: filled loop at `0x006F6658..0x006F66D6`; empty loop at `0x006F66FF..0x006F677B`.

Shape draw:

```text
CC_Draw_Shape(PIPS.SHP, frame, pos, bounds, 0x600, ..., 1000, ...)
```

Both filled and empty pips use `CC_Draw_Shape @ 0x004AED70`, `PIPS.SHP` pointer `0x00AC147C`, flags `0x600`, and draw argument `1000`. Active in YR: Yes. Evidence: calls at `0x006F66BA` and `0x006F675F`.

## 4. Selected And Hover Integration

`DrawHealthBar` itself does not check selected or hover state in the building branch. Those gates are in `TechnoClass::DrawExtras @ 0x006F5190`:

| Caller path | Gate | Effect | Active in YR |
|---|---|---|---|
| Selected path | `TechnoClass+0x83 != 0` | calls `DrawHealthBar` after selected building bracket front/direct lines | Yes - `0x006F5399` selected branch and `0x006F5E43` vtable `+0x44C` call |
| Hover path | `TechnoClass+0x431 != 0 && TechnoClass+0x83 == 0` | calls `DrawHealthBar` for hovered non-selected object | Yes - `0x006F5E50..0x006F5E78` |
| Hover disguised suppression | if `IsDisguised` is true, base `vtable+0xD0` returns `0`, so the alternate hover call is skipped | Conditional - buildings normally are not disguised; evidence `IsDisguised_Getter @ 0x0041C020`, base `+0xD0 @ 0x0041BE70` |

Draw order for selected buildings:

1. `DrawExtras` draws front/right bracket edges.
2. `DrawExtras` runs the separate `vtable+0x448` hook if strength/alliance/`EnemyHealth` permits; stock base is empty.
3. `DrawExtras` draws the direct front bracket stubs.
4. `DrawExtras` calls `DrawHealthBar`.
5. Inside `DrawHealthBar`, building health pips draw before optional `DrawPipScalePips`.

Active in YR: Yes. Evidence: `0x006F5190` selected branch and `0x006F64A0` building branch. `EnemyHealth` default is `yes` at `ini/rulesmd.ini:755`, but that key gates the earlier `+0x448` hook, not health pip drawing inside this function.

## 5. Optional Pip-Scale Gate After Health Pips

After health pips, the building branch computes whether to call vtable `+0x450` (`DrawPipScalePips @ 0x00709A90`):

```text
allow_pip_scale = owner.IsAlliedWith(g_PlayerPtr)
               || (this.visibility_bitmask_0x210 has local-player bit)
               || (this.WhatAmI()==6 && this.GetType().CanBeOccupied)

if this.GetType().PipsDrawForAll || allow_pip_scale:
    DrawPipScalePips(anchor, original_location, bounds)
```

The anchor passed to `DrawPipScalePips` is:

```text
anchor.x = location.x + C.x
anchor.y = location.y + C.y
```

where `C = project((-half.x, +half.y, 0))`. Active in YR: Yes / Conditional. The call is live, but what it draws depends on the building's `PipScale`, passengers, spawns, or other pip-scale data. Evidence: `0x006F677D..0x006F682C`, `DrawPipScalePips @ 0x00709A90`.

Important separation: `PipsDrawForAll`, `CanBeOccupied`, alliance, and the detection bitmask do not decide whether the health pips draw. They only decide whether the later `DrawPipScalePips` overlay draws. Active in YR: Yes. Evidence: all health pip draw loops precede the gate (`0x006F6658..0x006F677B`; gate starts at `0x006F677D`).

## 6. INI Keys

| INI key | Default / example | Effect in scoped branch | Active in YR |
|---|---|---|---|
| `[AudioVisual] ConditionYellow` | `50%`, `ini/rulesmd.ini:753` | yellow frame threshold through `IsYellowHP` | Yes |
| `[AudioVisual] ConditionRed` | `25%`, `ini/rulesmd.ini:752` | red frame threshold through `IsRedHP` | Yes |
| `[AudioVisual] EnemyHealth` | `yes`, `ini/rulesmd.ini:755` | not read in `DrawHealthBar`; caller uses it for earlier `+0x448` hook | Conditional / no direct health-pip effect |
| `Foundation` | art key, consumed by `Dimension2` via type field `+0xEF0` | determines `total_pips` through foundation height table | Yes |
| `Height` | art key, consumed by `Dimension2` via type field `+0xEF4` | affects projected `A.y` and vertical anchor, not `total_pips` because Z cancels between `A` and `B` | Yes |
| `PipsDrawForAll` | default false; `[YAPSYT] yes` at `ini/rulesmd.ini:13714` | permits optional `DrawPipScalePips` after health pips | Conditional |
| `CanBeOccupied` | many civilian/garrison buildings | permits optional `DrawPipScalePips` after health pips | Conditional |

## 7. Current Rust Implementation Status

Rust has a building status overlay in `src/app_ui_overlays.rs:73`. It broadly matches the binary's building health pip structure: pips along a 4/-2 diagonal step, filled/empty split, and `PIPS.SHP` atlas frame order `[0,1,2,4]` in `src/render/selection_overlay.rs:700-706`.

Observed differences or risks:

- Rust draws building status for any damaged non-selected structure (`src/app_ui_overlays.rs:95`), while this binary slice only proves calls from selected and hover paths in `DrawExtras`; whether the hover flag is set for damaged objects is outside this slice.
- Rust clamps `num_pips` with `.max(1)` (`src/app_ui_overlays.rs:140`), while the binary has no explicit total-pip minimum and skips both loops if `total_pips == 0`.
- Rust's formula intentionally adjusts the start Y for its own sprite anchor (`src/app_ui_overlays.rs:153-155`); binary anchor is exactly `location + project((-half.x,+half.y,dims.z)) + (3 + N*4, 4 - 2*N)`.
- Rust correctly maps red variant `3` onto source frame `4` through the atlas (`src/app_ui_overlays.rs:775-783`, `src/render/selection_overlay.rs:700-706`).

No Rust files were changed in this investigation.

## 8. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::DrawHealthBar` building branch | verified | decompile/assembly `0x006F64A0` | none for scoped branch |
| Building-vs-non-building branch separation | verified | `0x006F64AB..0x006F64B1`, non-building jump `0x006F683C` | none |
| `Dimension2` relation | verified | call `0x006F64D4`, report `BUILDINGTYPE_DIMENSION2...` | none for this branch |
| Projection anchor | verified | `CoordsToClient @ 0x006D1F10` calls in `0x006F6536..0x006F659E` | runtime pixel screenshot not checked |
| Pip count and filled count | verified | `0x006F65BC..0x006F6602` | exact global FPU rounding mode is delegated to `Math__ftol`; no manual floor in this branch |
| Color/frame thresholds | verified | `0x006F6604..0x006F6630`, helpers `0x005F5D20`, `0x005F5CD0` | none |
| Filled/empty draw loops | verified | `0x006F6658..0x006F677B` | none |
| Selected caller gate | verified | `DrawExtras @ 0x006F5190` selected branch | none |
| Hover caller gate | verified | `DrawExtras @ 0x006F5190` hover branch | what sets `+0x431` is out of scope |
| Optional `DrawPipScalePips` gate | verified | `0x006F677D..0x006F682C` | full pip-scale internals out of scope |
| `EnemyHealth` | verified as not a direct health-pip gate | `DrawExtras @ 0x006F5190`; `ini/rulesmd.ini:755` | broader enemy hover/select policy out of scope |
| Current Rust comparison | touched-not-exhausted | `src/app_ui_overlays.rs`, `src/render/selection_overlay.rs` | no code audit beyond scoped overlay comparison |

## 9. Open Questions - Final State

[RESOLVED] OQ-DHB-BLD-001 - What anchors building health pips? `location + project((-half.x,+half.y,dims.z)) + (3 + N*4, 4 - 2*N)` for the first pip, with step `(-4,+2)`. Evidence: `0x006F6536..0x006F66D6`.

Follow-up `BUILDING_HEALTH_PIP_VISUAL_ANCHOR_CASES_GHIDRA_REPORT.md` verifies concrete
case anchors from the placed/NW object screen point:

- `GACNST` 4x4 `Height=4`: first pip `sx+3, sy-ZAdj(4)-11`, `N=30`.
- `[TESLA] Image=NATSLA` 1x1 `Height=5`: first pip `sx+1, sy-ZAdj(5)-10`, `N=7`.
- `GAREFN` 4x3 `Height=4`: first pip `sx+1, sy-ZAdj(4)-10`, `N=22`.

The simple `sx+3, sy-11-Height*15` shortcut is only exact for the even-height
foundation case represented by `GACNST`; odd foundation heights visibly shift the
anchor by two pixels in X and one pixel in Y.

[RESOLVED] OQ-DHB-BLD-002 - How many pips draw? `N = (A.y - B.y) / 2`, reducing to `floor(foundation_height * 15 / 2)` for normal rectangular foundations. Evidence: `0x006F65BC..0x006F65CF`, `Dimension2 @ 0x00464AF0`.

[RESOLVED] OQ-DHB-BLD-003 - How is filled count clamped? `Math__ftol(health_ratio * N)`, minimum one for positive `N`, maximum `N`. Evidence: `0x006F65D7..0x006F6602`.

[RESOLVED] OQ-DHB-BLD-004 - Which frames are health and empty? Filled frame is `1` green, `2` yellow, or `4` red; empty frame is `0`. Evidence: `0x006F6604..0x006F6630`, `0x006F66A6`, `0x006F6722`.

[RESOLVED] OQ-DHB-BLD-005 - Do `PipsDrawForAll`, alliance, detection, or `CanBeOccupied` gate health pips? No; they gate only the later vtable `+0x450` pip-scale call after health pips. Evidence: `0x006F677D..0x006F682C`.

[RESOLVED] OQ-DHB-BLD-006 - Does `EnemyHealth` gate this function's health pips? No direct read in `DrawHealthBar`; `DrawExtras` uses `EnemyHealth` for the earlier `+0x448` hook. Evidence: `0x006F5190`, `0x006F64A0`, `ini/rulesmd.ini:755`.

[RESOLVED] OQ-DHB-BLD-007 - Does the building branch use `PIPBRD.SHP`? No; `PIPBRD.SHP` is only in the non-building branch. Building branch uses `PIPS.SHP @ 0x00AC147C`. Evidence: `0x006F66A6`, `0x006F6722`, non-building branch after `0x006F683C`.

[DEFERRED] OQ-DHB-BLD-008 - What exact UI input/render state sets `TechnoClass+0x431` hover health? Deferred as out-of-scope; this slice only verifies the caller gate once the flag is set.

## Sources

- Ghidra decompile/assembly: `TechnoClass::DrawHealthBar @ 0x006F64A0`
- Ghidra decompile: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompile: `BuildingTypeClass::Dimension2 @ 0x00464AF0`
- Ghidra decompile: `ObjectClass::GetHealthRatio @ 0x005F5C60`, `ObjectClass::IsYellowHP @ 0x005F5D20`, `ObjectClass::IsRedHP @ 0x005F5CD0`
- Ghidra decompile/assembly: `CC_Draw_Shape @ 0x004AED70`, `Math__ftol @ 0x007C5F00`, `DrawPipScalePips @ 0x00709A90`
- Existing reports checked: `SELECTION_BRACKETS_GHIDRA_REPORT.md`, `SELECTION_BRACKETS_PIPS_DRAW_ORDER_GHIDRA_REPORT.md`, `BUILDINGTYPE_DIMENSION2_BRACKET_EXTENTS_GHIDRA_REPORT.md`, `TECHNO_DRAWEXTRAS_BUILDING_BRACKET_BLOCK_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini:752-755`, `ini/rulesmd.ini:13714`
- Rust checked: `src/app_ui_overlays.rs`, `src/render/selection_overlay.rs`
