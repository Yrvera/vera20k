# Tactical AdjustForZ OccupantAnim Depth Contract - Reswarm 2026-05-27

**Address(es):** `AnimClass::DrawIt @ 0x00422CA0`, `Tactical__AdjustForZ @ 0x006D20E0`, `Tactical__ComputeZMultiplier @ 0x006D1BA8`, `TechnoClass::Fire_At @ 0x006FDD50`, `AnimClass::GetZAdjust @ 0x00425630`, `Math__ftol @ 0x007C5F00`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the integer depth argument passed by `AnimClass::DrawIt` for standard non-flat, non-tiled occupied-building `OccupantAnim` shot flashes, and the exact contribution of `Tactical__AdjustForZ`.  
**Non-Scope:** generic `AnimClass` lifecycle, global display-layer traversal, full `CC_Draw_Shape` z-buffer internals, all flat/tiled/shadow/RING1 branches, pixel proof for every building/wall overlap.  
**Confidence:** High for the caller formula and `AdjustForZ` arithmetic; Medium for downstream building/wall acceptance wording because global comparator is intentionally out of scope.  
**Active in YR:** Yes. Stock YR occupied weapons define `OccupantAnim=UCFLASH/UCCONS/UCINIT`; those art sections are standard, non-flat, non-tiled, ground-layer anims.

## 0. Working Notes

**Target question:** Can VERA20k's current float `garrison_flash_depth_apply_z_adjust` be proven equivalent to the native integer `AnimClass::DrawIt` depth contract for ordinary occupied-building shot flashes, or is a native-integer helper/test fixture required?

**Non-goals:** no Rust edits; no lifecycle model; no pool-vs-embedded-runtime decision beyond depth implications; no exhaustive wall/building draw traversal.

**Evidence needed to mark COMPLETE:** live YR activation proof, `Fire_At` occupied-shot `ZAdjust=-200` proof, decompile plus instruction range for standard `DrawIt` branch, decompile plus instruction range for `Tactical__AdjustForZ`, current Rust comparison, and an implementation handoff.

**Stop conditions:** stop after the standard non-flat/non-tiled branch and helper arithmetic are verified, and after enough Rust comparison exists to decide equivalence. Defer global object/wall comparator if the caller contract already proves the current float bias is not exact.

## 1. Overview

Standard occupied-building shot flashes are normal `AnimClass` draws. `Fire_At` constructs the anim, then the occupied-building branch overwrites instance `AnimClass+0x100` with `-200`.

For the standard non-flat, non-tiled `DrawIt` branch, the integer depth argument passed to `CC_Draw_Shape` is:

`AnimType.YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ(AnimClass::GetZAdjust()) - 2`

For stock `UCFLASH`, `UCCONS`, and `UCINIT`, `YDrawOffset` is omitted and remains `0`, and the occupied-building override makes `ZAdjust=-200`. Therefore the stock occupied-shot depth integer is:

`-200 - Tactical__AdjustForZ(anim_z) - 2`

This is not a normalized float bias around `1000`, and the current Rust float helper cannot be proven equivalent from the verified native mechanism.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning in this slice | Evidence |
|---|---:|---|---|
| `AnimClass` | `+0xA4` | z coordinate used by `AnimClass::GetZAdjust` base return | `0x00425630` |
| `AnimClass` | `+0xCC` | optional owner object pointer whose `+0xA4` is added by `GetZAdjust` | `0x00425630` |
| `AnimClass` | `+0x100` | instance `ZAdjust`, overwritten to `-200` for occupied-building shot flashes | `0x006FF411..0x006FF420`, `0x004237EA..0x00423803` |
| `AnimClass` | `+0x190` | stored draw flags; standard branch ORs in `0x2000` before drawing | `0x00423801..0x0042380C` |
| `AnimClass` | `+0xC8` | `AnimTypeClass*` | `0x004237E4` |
| `AnimTypeClass` | `+0x344` | `YDrawOffset`; default `0`; stock UC sections omit it | `0x004237E4..0x004237F0`, `ini/artmd.ini:16131..16141` |
| `AnimTypeClass` | `+0x35B` | `Tiled`; stock UC sections omit it, so false | `0x00423630..0x0042363E`, `ini/artmd.ini:16131..16141` |
| `AnimTypeClass` | `+0x369` | `Flat`; stock UC sections omit it, so false | `0x00423728..0x00423730`, `ini/artmd.ini:16131..16141` |
| global | `0x00B0CD48` | runtime `g_AdjustForZ_Multiplier`, computed from camera/scale | `0x006D1BA8..0x006D1BE3` |
| global | `0x007E1738` | double `0.5`, added before `Math__ftol` | `0x006D210D`, memory bytes `000000000000e03f` |
| global | `0x00822D80` | x87 control word `0x0E7F`, used by `Math__ftol` for truncation mode | `0x007C5F13..0x007C5F32`, memory bytes `7f0e0000` |

## 3. Core Logic

### Occupied shot value entering depth

`TechnoClass::Fire_At @ 0x006FDD50` constructs the selected `OccupantAnim` with draw flags `0x600`, then checks whether the firing object is a building and has occupants. The relevant instruction range:

| Address range | Evidence |
|---|---|
| `0x006FF3AD..0x006FF3C2` | pushes `zAdjust=0`, `reverse=0`, `drawFlags=0x600`, `loopCount=1`, `delay=0`, coordinates, and type, then calls `AnimClass::Constructor @ 0x00421EA0` |
| `0x006FF3CD..0x006FF3D7` | calls `WhatAmI` and compares to `6` for building |
| `0x006FF411..0x006FF420` | calls vtable `+0x408`, tests occupant count, then the continued branch writes `AnimClass+0x100 = 0xFFFFFF38` (`-200`) for positive occupied-building count |

The earlier calculation at `0x006FF3E8..0x006FF40B` can write a non-positive origin-relative `ZAdjust`, but the positive-occupant building branch overwrites it with `-200`. Active in YR: Yes for occupied buildings with stock UC weapons.

### Standard DrawIt branch

`AnimClass::DrawIt @ 0x00422CA0` reaches the standard branch only after rejecting tiled and flat:

| Address range | Evidence |
|---|---|
| `0x00423630..0x0042363E` | reads `AnimType+0x35B` (`Tiled`); `je 0x00423728` enters non-tiled handling |
| `0x00423728..0x00423730` | reads `AnimType+0x369` (`Flat`); `je 0x004237AE` enters standard non-flat branch |
| `0x004237AE..0x004237BB` | computes screen Y as `param_2.y + AnimType.YDrawOffset`; this is position, not depth |
| `0x004237D7..0x004237DF` | calls vtable `+0x1D0` (`AnimClass::GetZAdjust`) and passes return in `ECX` to `Tactical__AdjustForZ` |
| `0x004237E4..0x00423803` | loads `AnimType+0x344`, loads `AnimClass+0x100`, adds them, subtracts `AdjustForZ`, subtracts `2` |
| `0x00423809..0x00423827` | pushes that integer as the `CC_Draw_Shape` depth/z argument and calls `0x004AED70` |

Pseudocode for this branch:

```text
screen_point.x = input_screen.x
screen_point.y = input_screen.y + anim_type.YDrawOffset

z_pixels = Tactical__AdjustForZ(anim.GetZAdjust())
shape_depth = anim_type.YDrawOffset + anim.ZAdjust - z_pixels - 2

draw_flags = stored_flags | 0x2000
CC_Draw_Shape(..., draw_flags, ..., shape_depth, ...)
```

For stock occupied `UC*` flashes:

```text
YDrawOffset = 0
ZAdjust = -200
shape_depth = -202 - Tactical__AdjustForZ(anim_z)
```

### Tactical__AdjustForZ arithmetic

`Tactical__AdjustForZ @ 0x006D20E0` is not a constant subtraction. It converts a signed integer z input into an integer screen-depth contribution using a runtime multiplier:

Instruction range:

| Address | Operation |
|---|---|
| `0x006D20E3` | compares `ECX` z input with `0x2D8` (`728`) |
| `0x006D20ED..0x006D20FF` | stores an integer addend: `0`, or `1` when `z >= 728` |
| `0x006D20FF` | `fild` signed z input |
| `0x006D2103` | multiply by `qword ptr [0x00B0CD48]` (`g_AdjustForZ_Multiplier`) |
| `0x006D2109` | `fiadd` the threshold addend |
| `0x006D210D` | add double `0.5` from `0x007E1738` |
| `0x006D2113` | call `Math__ftol @ 0x007C5F00` |

Exact formula:

```text
threshold_add = (z >= 728) ? 1 : 0
return ftol_truncate_toward_zero((z * g_AdjustForZ_Multiplier) + threshold_add + 0.5)
```

`Math__ftol @ 0x007C5F00` sets/uses x87 control word `0x0E7F` (`0x00822D80`), executes `fistp qword`, returns low/high integer registers, and therefore truncates according to that control word. For the normal positive-z case, the prior `+0.5` makes this nearest-integer behavior. For negative z, do not replace this with floor; truncation toward zero after adding `0.5` is the verified operation.

`Tactical__ComputeZMultiplier @ 0x006D1BA8` computes the runtime multiplier:

```text
scale_part = 60.0 / g_TacticalScaleLikeValue
g_AdjustForZ_Multiplier = cos(camera_angle_inputs) * scale_part
```

The static image has `0x00B0CD48` in BSS, so the multiplier is runtime state, not a compile-time literal.

## 4. INI Keys

| Section | Source | Relevant keys | Effect |
|---|---|---|---|
| `[UCFLASH]` | `ini/artmd.ini:16131..16133` | `Layer=ground`, `Translucent=yes` | Standard non-flat/non-tiled path; no `YDrawOffset` |
| `[UCCONS]` | `ini/artmd.ini:16135..16137` | `Layer=ground`, `Translucent=yes` | Standard non-flat/non-tiled path; no `YDrawOffset` |
| `[UCINIT]` | `ini/artmd.ini:16139..16141` | `Layer=ground`, `Translucent=yes` | Standard non-flat/non-tiled path; no `YDrawOffset` |
| UC weapon entries | `ini/rulesmd.ini` | `OccupantAnim=UCFLASH/UCCONS/UCINIT` | Active ordinary occupied-building shot path |

No stock UC section in the scoped set defines `YDrawOffset`, `ZAdjust`, `Flat`, `Tiled`, or `Shadow`.

## 5. Integration Points

1. `TechnoClass::Fire_At` constructs one `AnimClass` for an occupied shot and writes `ZAdjust=-200` for positive occupied-building count.
2. `DisplayClass::Submit_Object`/layer traversal is outside this slot, but slot-3 already verified stock UC `Layer=ground` uses display layer 2 and sorted layer insertion, not raw `g_AnimClass_Array` order.
3. `AnimClass::DrawIt` standard branch supplies screen position and integer depth to `CC_Draw_Shape`.
4. `Tactical__AdjustForZ` contributes a camera/scale-dependent rounded integer derived from `AnimClass::GetZAdjust()`.

## 6. Current Rust Implementation Status

Current Rust keeps garrison flash screen position independent from `z_adjust`, which matches the native separation of screen Y and depth:

- `src/app_instances/overlays.rs:508..517` uses `flash.screen_y + entry.offset_y` for position and passes `flash.z_adjust` only to depth.

The depth mechanism is still not native-equivalent:

- `src/app_instances/overlays.rs:531..547` calls `compute_sprite_depth_params(origin_y, world_height, screen_y, z)`, then adds `neutral_delta = 1000 - z_adjust` scaled by `0.000001`.
- `src/app_instances/helpers.rs:43..52` normalizes `screen_y + z * terrain::HEIGHT_STEP` into a float depth and clamps it.
- Native standard `AnimClass::DrawIt` instead passes the signed integer `YDrawOffset + ZAdjust - Tactical__AdjustForZ(anim_z) - 2` to `CC_Draw_Shape`.

Verdict: the current float bias is directionally related to `ZAdjust`, but it cannot be proven equivalent to the native integer contract. A native-integer helper/test fixture is needed.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Ordinary occupied-shot `OccupantAnim` activation | verified | `0x006FF3AD..0x006FF3C2`; `rulesmd.ini OccupantAnim=` | none for depth slice |
| Occupied-building `ZAdjust=-200` | verified | `0x006FF411..0x006FF420`; prior report | none |
| Standard non-tiled/non-flat branch selection | verified | `0x00423630..0x00423730`; `artmd.ini:16131..16141` | none for stock UC |
| Standard branch screen position | verified | `0x004237AE..0x004237BB` | none |
| Standard branch integer depth formula | verified | `0x004237D7..0x00423827` | none |
| `AnimClass::GetZAdjust` input to `AdjustForZ` | verified | `0x00425630`; call at `0x004237D7..0x004237DF` | exact fire-origin z value depends on `Fire_At` coordinate input, outside this depth arithmetic slice |
| `Tactical__AdjustForZ` arithmetic | verified | `0x006D20E0..0x006D211B`; `Math__ftol @ 0x007C5F00` | none for formula |
| Multiplier computation | verified | `0x006D1BA8..0x006D1BE3` | exact runtime value requires camera/scale fixture |
| Global draw comparator/building/wall overlap | deferred | slot-3 draw traversal report; this report's caller formula | concrete pixel fixture for one building/wall overlap |
| Current Rust depth helper | verified mismatch | `src/app_instances/overlays.rs:531..547`, `src/app_instances/helpers.rs:43..52` | implement/prove native integer helper |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path active for ordinary YR garrison shots? -> Yes; stock weapons use `OccupantAnim` and stock UC art is standard non-flat/non-tiled.` (evidence: `0x006FF3AD..0x006FF3C2`; `ini/artmd.ini:16131..16141`)
- `[RESOLVED] OQ-02 - What instance `ZAdjust` enters standard DrawIt for occupied buildings? -> `-200` after positive occupied-building count.` (evidence: `0x006FF411..0x006FF420`)
- `[RESOLVED] OQ-03 - Does `ZAdjust` shift screen position? -> No; screen point uses `param_2.y + YDrawOffset`, while `ZAdjust` appears in the separate integer depth argument.` (evidence: `0x004237AE..0x00423827`)
- `[RESOLVED] OQ-04 - What exact standard depth expression is passed to `CC_Draw_Shape`? -> `YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ(anim.GetZAdjust()) - 2`.` (evidence: `0x004237D7..0x00423827`)
- `[RESOLVED] OQ-05 - What does `Tactical__AdjustForZ` compute? -> `ftol_truncate_toward_zero(z * g_AdjustForZ_Multiplier + (z >= 728 ? 1 : 0) + 0.5)`.` (evidence: `0x006D20E0..0x006D211B`, `0x007C5F00..0x007C5F3C`)
- `[RESOLVED] OQ-06 - Is `g_AdjustForZ_Multiplier` a literal? -> No; it is computed at runtime by `Tactical__ComputeZMultiplier`.` (evidence: `0x006D1BA8..0x006D1BE3`)
- `[RESOLVED] OQ-07 - Can current Rust float bias be proven equivalent? -> No; Rust normalizes/clamps terrain-like float depth and adds arbitrary scale, while native passes signed integer depth to `CC_Draw_Shape`.` (evidence: `src/app_instances/overlays.rs:531..547`; `src/app_instances/helpers.rs:43..52`; `0x004237D7..0x00423827`)
- `[DEFERRED] OQ-08 - Exact pixel ordering against every building body and wall case` (category: requires-different-system-context; reason: this needs final draw-stack/comparator fixture beyond the caller integer formula; next-step-if-pursued: pick one occupied `UCFLASH` overlap case and compare building/wall `CC_Draw_Shape` integer depths and layer insertion order)
- `[DEFERRED] OQ-09 - Exact numeric `AdjustForZ` for every camera zoom/angle` (category: requires-different-system-context; reason: multiplier is runtime camera/scale state; next-step-if-pursued: capture or recreate runtime tactical scale settings and assert integer outputs for sample z values)

## 9. Visual Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `TechnoClass::Fire_At @ 0x006FDD50` | occupied shooter, non-null `WeaponType+0x110` | `UCFLASH`/`UCCONS`/`UCINIT` | fire origin coordinate | constructor draw flags `0x600` | Yes | anim creation |
| 2 | `Fire_At` occupied-building branch | `WhatAmI == 6`, occupant count > 0 | same anim | same coordinate | writes `ZAdjust=-200` | Yes | depth setup |
| 3 | `AnimClass::DrawIt @ 0x00422CA0` standard branch | `Tiled=false`, `Flat=false`; stock UC omits both keys | current anim frame | `screen.y + YDrawOffset` | `CC_Draw_Shape`, flags include `0x2000` | Yes | final shape draw |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `UCFLASH` | Yes when referenced by occupied weapon | Yes | Yes | No | No | shot flash | No | No | `rulesmd.ini OccupantAnim=UCFLASH`; `artmd.ini:16131..16133`; `0x006FDD50` |
| `UCCONS` | Yes when referenced by occupied weapon | Yes | Yes | No | No | shot flash | No | No | `rulesmd.ini OccupantAnim=UCCONS`; `artmd.ini:16135..16137`; `0x006FDD50` |
| `UCINIT` | Yes when referenced by occupied weapon | Yes | Yes | No | No | shot flash | No | No | `rulesmd.ini OccupantAnim=UCINIT`; `artmd.ini:16139..16141`; `0x006FDD50` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard occupied-shot `OccupantAnim` depth integer is `YDrawOffset + ZAdjust - Tactical__AdjustForZ(anim_z) - 2`; stock UC makes that `-202 - AdjustForZ(anim_z)`. | `0x004237D7..0x00423827`; `ini/artmd.ini:16131..16141`; `0x006FF411..0x006FF420` | mismatch: normalized/clamped float bias around `compute_sprite_depth_params` | `src/app_instances/overlays.rs`, shared render-depth helper surface | Add/prove a native-integer anim depth helper and map it into the renderer without changing screen position. | `garrison_occupant_anim_z_adjust_uses_native_draw_depth_formula`: stock `UCFLASH`, `YDrawOffset=0`, `ZAdjust=-200`, sampled `anim_z` values produce the exact native integer before any renderer normalization. | Do not encode `-200` as screen displacement or arbitrary layer override. |
| `Tactical__AdjustForZ` uses signed z, runtime multiplier, threshold addend at `z >= 728`, `+0.5`, then x87 `ftol` with control word `0x0E7F`. | `0x006D20E0..0x006D211B`; `0x007C5F00..0x007C5F3C`; `0x00822D80` bytes | missing: current Rust uses `z as f32 * terrain::HEIGHT_STEP` and `z_bias`, not native helper | render math helper/test fixture | Implement the exact integer helper or fixture it with injected multiplier. | `tactical_adjust_for_z_matches_native_rounding_threshold`: z `727`, `728`, positive high z, and one negative z sample match threshold/truncation semantics. | Do not use floor for negative z and do not drop the `z >= 728` addend. |
| The multiplier is runtime camera/scale state, computed by `Tactical__ComputeZMultiplier`, not a hardcoded constant. | `0x006D1BA8..0x006D1BE3`; static `0x00B0CD48` reads as BSS zero | missing/proven-unavailable: Rust depth helper has no equivalent multiplier input | app render tactical/camera state | Carry or derive the native-equivalent multiplier for depth tests; keep tests able to inject it. | `garrison_occupant_anim_depth_uses_runtime_adjust_for_z_multiplier`: changing the multiplier changes native integer depth in the same direction and magnitude as formula. | Do not hardcode a single multiplier unless tied to a verified fixed camera mode. |
| Screen Y and shape depth are separate: standard branch screen Y adds `YDrawOffset`; depth adds `YDrawOffset + ZAdjust - AdjustForZ - 2`. | `0x004237AE..0x00423827` | partially aligned: Rust no longer shifts screen Y by `z_adjust` | `src/app_instances/overlays.rs` | Preserve no-screen-shift behavior while replacing depth math. | `garrison_occupant_anim_z_adjust_does_not_shift_screen_position`: `z_adjust=-200` changes depth helper output but not sprite position. | Do not regress to the older screen-row shift model. |

Proposed Rust test names:

- `garrison_occupant_anim_z_adjust_uses_native_draw_depth_formula`
- `tactical_adjust_for_z_matches_native_rounding_threshold`
- `garrison_occupant_anim_depth_uses_runtime_adjust_for_z_multiplier`
- `garrison_occupant_anim_z_adjust_does_not_shift_screen_position`
- `garrison_occupant_anim_depth_fixture_orders_against_building_wall`

## 11. Negative Facts / Do Not Do

- Do not claim current Rust's float `neutral_delta * 0.000001` bias is parity-equivalent. Native passes a signed integer depth to `CC_Draw_Shape`, and the current scale has no binary evidence.
- Do not apply occupied-shot `ZAdjust=-200` to screen Y. Native screen Y uses `YDrawOffset`; `ZAdjust` is in the separate depth expression.
- Do not omit `Tactical__AdjustForZ`. For nonzero anim z, native subtracts the rounded helper output from depth.
- Do not hardcode `AdjustForZ` as `z * constant` without the `z >= 728` integer addend and `+0.5`/ftol truncation behavior.
- Do not rely on raw `g_AnimClass_Array` order for the depth decision. Slot-3 verified display-layer sorted traversal is the relevant draw-order context.

## 12. Remaining Uncertainty

- Exact final pixel ordering against every building body and wall case remains a follow-up fixture, not a blocker for this contract. This report proves the garrison anim's native integer depth input and proves the current float bias is not equivalent.
- Exact runtime `g_AdjustForZ_Multiplier` values require camera/scale runtime context. The formula and update function are verified.

## 13. Stale Docs / Follow-up Docs

- `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md:6` should replace "exact `Tactical__AdjustForZ` internals" non-scope wording with: "exact `Tactical__AdjustForZ` internals are covered by `TACTICAL_ADJUSTFORZ_OCCUPANTANIM_DEPTH_CONTRACT_RESWARM_20260527.md`: `ftol_truncate_toward_zero(z * g_AdjustForZ_Multiplier + (z >= 728 ? 1 : 0) + 0.5)`."
- `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md:247` should replace the deferred OQ-14 with: "`Tactical__AdjustForZ` helper arithmetic resolved by `TACTICAL_ADJUSTFORZ_OCCUPANTANIM_DEPTH_CONTRACT_RESWARM_20260527.md`; global building/wall comparator remains a separate fixture question."
- `docs/research/GARRISON_VISUAL_OCCUPANTANIM_RESWARM_20260527.md:168..170` should be refined from "native integer `CC_Draw_Shape` depth formula" to: "native standard garrison `OccupantAnim` depth is the signed integer `YDrawOffset + ZAdjust - Tactical__AdjustForZ(anim_z) - 2`; for stock UC occupied-building shots this is `-202 - Tactical__AdjustForZ(anim_z)`."

## Sources

- Ghidra decompiled/read-only: `AnimClass::DrawIt @ 0x00422CA0`, `Tactical__AdjustForZ @ 0x006D20E0`, `Tactical__ComputeZMultiplier @ 0x006D1BA8`, `TechnoClass::Fire_At @ 0x006FDD50`, `AnimClass::GetZAdjust @ 0x00425630`, `Math__ftol @ 0x007C5F00`.
- Instruction ranges checked: `0x00423630..0x00423827`, `0x006D20E0..0x006D211B`, `0x006D1BA8..0x006D1BE3`, `0x006FF3AD..0x006FF420`, `0x007C5F00..0x007C5F3C`.
- Memory constants checked: `0x007E1738` double `0.5`, `0x00822D80` control word `0x0E7F`, `0x00B0CD48` runtime multiplier storage.
- INI checked: `ini/artmd.ini:16131..16141`; `ini/rulesmd.ini` stock `OccupantAnim=` entries.
- Rust scanned: `src/app_instances/overlays.rs`, `src/app_instances/helpers.rs`.
- Prior docs checked: `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`, `docs/research/GARRISON_VISUAL_OCCUPANTANIM_RESWARM_20260527.md`.

## Status

COMPLETE for the scoped standard garrison `OccupantAnim` depth integer contract. PARTIAL only for final global building/wall pixel-order fixtures, which are explicitly outside this slot's scope.
