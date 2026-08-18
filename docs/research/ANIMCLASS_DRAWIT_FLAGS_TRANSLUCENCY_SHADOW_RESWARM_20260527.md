# AnimClass DrawIt Flags / Translucency / Shadow - Re-swarm Report

**Address(es):** `AnimClass::DrawIt @ 0x00422CA0`, `AnimTypeClass::Constructor @ 0x00427530`, `AnimTypeClass::ReadINI @ 0x00427D00`, `TechnoClass::Fire_At @ 0x006FDD50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** material-relevant `AnimClass::DrawIt` behavior for standard non-flat/non-tiled garrison `OccupantAnim` plus stock UC art defaults and shadow contrast.  
**Non-Scope:** global `AnimClass` traversal/order, exact `Tactical__AdjustForZ` internals, RING1 warp-ring pixel shader path, full `CC_Draw_Shape`/blitter table semantics, and every non-UC anim.  
**Confidence:** High for the claimed slice.  
**Active in YR:** Yes for stock occupied-building shots using `OccupantAnim=UCFLASH/UCCONS/UCINIT`; conditional for modded `Shadow=yes`, `Flat=yes`, or `Tiled=yes` occupant anims.

## Working Notes

**Target question:** Does standard garrison `OccupantAnim` require a full app-side `AnimClass` object pool for draw flags/translucency/shadow material parity, or can an embedded app `AnimRuntime` plus richer sprite material/depth payload match this slice?

**Non-goals:** Do not settle global anim object ordering, draw-list insertion, `Tactical__AdjustForZ` internals, or the whole generic animation system.

**Evidence needed to mark COMPLETE:** direct Ghidra evidence for constructor flags, `DrawIt` low-bit/`0x800`/`0x2000` behavior, translucency flag handling, shadow branch behavior, INI/default evidence for stock UC `Flat/Tiled/Shadow/PingPong/Translucent`, and current Rust deltas.

**Stop conditions:** no unresolved scoped open questions after one zero-add pass over `DrawIt`, parser/defaults, stock UC INI, and current Rust garrison flash material path.

## 1. Overview

Occupied building shots construct a normal `AnimClass` with draw flags `0x600`; `AnimClass::DrawIt` derives the final draw flags from instance flags, translucency/detail gates, a low-bit remap guard, and a final `| 0x2000` before `CC_Draw_Shape`. Stock UC anims are standard non-flat/non-tiled/non-shadow anims with `Translucent=yes`, so the garrison flash renderer needs native-equivalent material flags and depth, but this scoped flag branch does not by itself require a global `AnimClass` pool.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Evidence |
|---|---:|---|---|
| `AnimClass` | `+0x190` | stored constructor draw flags | `0x00422CA0`, `0x006FDD50` |
| `AnimClass` | `+0xAC` | current frame; drawn frame is `Start + CurrentFrame` | `0x00422CA0` |
| `AnimClass` | `+0x100` | instance `ZAdjust`, overwritten to `-200` for occupied building shots | `0x006FDD50`, `0x00422CA0` |
| `AnimClass` | `+0x119` | transient flag that forces translucency bits `0x4` or `0x6` depending on type byte `+0x368` | `0x00422CA0` |
| `AnimClass` | `+0x178` | frame/state byte used by translucency gate; values `>= 0x0F` return before draw in scoped paths | `0x00422CA0` |
| `AnimTypeClass` | `+0x2B4` | `Start` | `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x2C0` | `End`, used for progressive `Translucent=yes` thresholds | `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x2D8` | `TranslucencyDetailLevel`; branch only runs if `<= g_ExtraAnimationsEnabled` | `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x2EC` | fixed `Translucency` value; `0x19`, `0x32`, `0x4B` map to draw bits `0x2`, `0x4`, `0x6` | `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x344` | `YDrawOffset` | `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x35B` | `Tiled` | `0x00427530`, `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x369` | `Flat` | `0x00427530`, `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x36A` | `Translucent` | `0x00427530`, `0x00427D00`, `0x00422CA0` |
| `AnimTypeClass` | `+0x372` | `Shadow` | `0x00427530`, `0x00427D00`, `0x00422CA0` |

## 3. Core Logic

### 3.1 Constructor flags from garrison fire

`TechnoClass::Fire_At @ 0x006FDD50` selects `WeaponType+0x110` after the occupied virtual returns true. When the selected anim pointer is non-null, it allocates `0x1C8` bytes and calls `AnimClass::Constructor(type, coords, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`. For an occupied building (`WhatAmI == 6`) it later writes `AnimClass+0x100 = 0xFFFFFF38` (`-200`) when occupant count is positive.

Tiny detail: the constructor `zAdjust=0` is not final draw depth. For occupied buildings the post-constructor write is the active instance value.

### 3.2 Draw flag assembly

`AnimClass::DrawIt @ 0x00422CA0` loads base flags from `AnimClass+0x190` (`MOV EBX,[ESI+0x190]`). Scoped assembly evidence:

- `0x00423075`: `OR EBX,0x6` when instance byte `+0x119` is set and type byte `+0x368` is true.
- `0x0042307A`: `OR EBX,0x4` when instance byte `+0x119` is set and type byte `+0x368` is false.
- `0x004230FB`: `OR EBX,0x2` for the low progressive/fixed translucency case.
- `0x004230FE..0x00423103`: `TEST BL,0x1`; if bit `0x1` is absent, `OR BH,0x8`, which is `flags |= 0x800`.
- Standard non-flat/non-tiled draw uses `uVar15 | 0x2000` in decompile; assembly at `0x00423806` (`OR AH,0x20`) applies `0x2000` before the normal `CC_Draw_Shape` call.
- Shadow branch clears bits and forces shadow flags: `0x00423872 AND EBX,0xfffffff9`; `0x00423876 OR EBX,0x601`.

For the stock garrison constructor base `0x600`, bit `0x1` is not set, so the normal path adds `0x800`. Stock UC also has `Translucent=yes`, so the progressive translucency branch can add one of `0x2`, `0x4`, or `0x6` before the final `| 0x2000`. A naive "draw with 0x600" material is therefore incomplete.

### 3.3 Translucent=yes handling

`AnimTypeClass::ReadINI @ 0x00427D00` reads `Translucent` into type byte `+0x36A`. `DrawIt @ 0x00422CA0` only runs translucency selection when `AnimType+0x2D8 <= g_ExtraAnimationsEnabled`. If `Translucent=no`, fixed `Translucency` at `+0x2EC` may still set draw bits for values:

- `0x19` -> `flags |= 0x2`
- `0x32` -> `flags |= 0x4`
- `0x4B` -> `flags |= 0x6`

If `Translucent=yes`, `DrawIt` requires `AnimClass+0x178 < 0x0F` or returns before drawing. It compares `CurrentFrame` (`AnimClass+0xAC`) against fractions of `End` (`AnimType+0x2C0`) and selects `0x2`, `0x4`, or `0x6` through the same flag bits. For stock UC, `End` defaults to `0`; the branch still exists and must be modeled carefully by the lifecycle slot because the visible draw window is bounded by the first-AI guard/end behavior.

### 3.4 Standard non-flat/non-tiled draw

For standard stock UC (`Tiled=false`, `Flat=false`) the branch:

1. Computes frame as `AnimType.Start + AnimClass.CurrentFrame`.
2. Uses screen position `x = param_2.x`, `y = param_2.y + YDrawOffset`.
3. Calls vtable `+0x1D0` with layer/depth bucket `2`.
4. Calls `Tactical__AdjustForZ()`.
5. Calls `CC_Draw_Shape(..., flags | 0x2000, ..., depth = YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ() - 2, bucket = 2, ...)`.
6. If `Shadow=false`, returns.

### 3.5 Shadow branch contrast

`Shadow=yes` is not active for stock UC, but `DrawIt` has an active conditional branch for modded anims. After the normal draw, if `AnimType+0x372` is true:

- reads the SHP frame count/rect value from the shape header and offsets the drawn frame by half of that count (`normal frame + frame_count / 2`) for the shadow half;
- calls vtable `+0x1D0` with neutral-like args;
- computes shadow depth as `-2 - Tactical__AdjustForZ()`, not `YDrawOffset + ZAdjust - Tactical__AdjustForZ() - 2`;
- clears translucency bits with `flags &= 0xfffffff9` and forces `flags |= 0x601`;
- calls `CC_Draw_Shape` a second time.

Implementation implication: an embedded runtime must be able to request a second shadow draw for modded `Shadow=yes` occupant anims. Stock UC does not require that extra draw.

## 4. INI Keys

| Key | Native storage/default | Stock UC value | Effect |
|---|---|---|---|
| `Layer` | default `3`; read by `CCINIClass__ReadLayer` | `ground` | layer/traversal input; exact ordering is slot 3 scope |
| `Translucent` | default false at `+0x36A`; read bool | `yes` for `UCFLASH`, `UCCONS`, `UCINIT`, `UCELEC`, `UCBLOOD` in `artmd.ini` | progressive draw-flag translucency path |
| `Translucency` | default `0`; read int at `+0x2EC` | omitted | fixed 25/50/75 draw bits when present |
| `TranslucencyDetailLevel` | default `0`; read int at `+0x2D8` | omitted | gates translucency work by extra animation setting |
| `Flat` | default false at `+0x369`; read bool | omitted -> false | stock UC uses standard, not flat branch |
| `Tiled` | default false at `+0x35B`; read bool | omitted -> false | stock UC uses standard, not tiled branch |
| `Shadow` | default false at `+0x372`; read bool | omitted -> false | stock UC skips second shadow draw |
| `PingPong` | default false at `+0x370`; read bool | omitted -> false | lifecycle only; no direct draw-material payload |
| `YDrawOffset` | default `0` at `+0x344`; read int | omitted -> `0` | screen Y and depth term |
| `ZAdjust` | default `0` at `+0x348`; read int | omitted -> `0`, then instance set to `-200` by `Fire_At` | depth term |

Stock UC evidence: `ini/artmd.ini:16131..16141` has `[UCFLASH]`, `[UCCONS]`, `[UCINIT]` with only `Layer=ground` and `Translucent=yes`; `ini/art.ini:11583..11585` has base `[UCFLASH]` fallback with `Layer=ground`, `Translucent=yes`.

## 5. Integration Points

`TechnoClass::Fire_At` constructs the `AnimClass` in the fire action path. `AnimClass::DrawIt` is the per-object visual draw function. This report does not claim which global traversal pass invokes it or how ties against buildings/walls are resolved; those are separate swarm slots. Material-relevant conclusion: the draw call payload has more state than current `GarrisonMuzzleFlash` carries.

## 6. Current Rust Implementation Status

Current Rust garrison flash surfaces:

- `src/sim/components.rs:675..702` `GarrisonMuzzleFlash` stores position, frame, total SHP frames, rate delay, and `z_adjust`; it has no native draw flags, translucent/progressive state, fixed `Translucency`, `Shadow`, `Flat`, `Tiled`, or second-draw payload.
- `src/app_building_anim.rs:739..754` spawns garrison flashes with `z_adjust=-200`, `frame=0`, `total_frames`, and rate fields only.
- `src/app_building_anim.rs:767..781` advances by raw frame count and clamps rate zero to at least one tick, so it cannot drive the native `Translucent=yes`/`End=0` frame window correctly yet.
- `src/app_instances/overlays.rs:518..527` emits a `SpriteInstance` with `alpha: 1.0`, tint lighting, and no draw-flag-equivalent material payload for stock UC translucency.
- `src/app_instances/overlays.rs:531..546` maps `z_adjust` into a float bias, not the verified integer depth expression.
- `src/rules/art_data.rs:391..394` parses native rate logic frames, but the registry does not expose generic anim material metadata needed by this slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Garrison `AnimClass` constructor draw flags `0x600` | verified | `0x006FDD50` decompile | none |
| Occupied building `ZAdjust=-200` | verified | `0x006FDD50` decompile | none |
| Base flags load from `AnimClass+0x190` | verified | `0x00422CA0`, asm `0x0042304B` | none |
| `0x119` forced translucency bits | verified | asm `0x00423061..0x0042307A` | exact semantic name of `+0x119/+0x368` outside scope |
| Stock `Translucent=yes` progressive flag path | verified | `0x00427D00`, `0x00422CA0`, `ini/artmd.ini` | lifecycle slot must settle visible frame/end timing |
| Fixed `Translucency` values | verified | `0x00427D00`, `0x00422CA0` | none for values `0x19/0x32/0x4B` |
| Low bit guard adding `0x800` | verified | asm `0x004230FE..0x00423103` | exact blitter meaning deferred to CC_Draw_Shape table |
| Final `| 0x2000` | verified | `0x00422CA0`, asm `0x00423806`, `0x0042379B` | exact blitter meaning deferred |
| Standard non-flat/non-tiled normal draw | verified | `0x00422CA0` | global traversal/order separate |
| Shadow second draw branch | verified | `0x00422CA0`, asm `0x00423832..0x0042389E` | pixel result for modded shadow anim not runtime-captured |
| Stock UC Flat/Tiled/Shadow/PingPong defaults | verified | `0x00427530`, `0x00427D00`, `ini/artmd.ini` | none for scoped UC sections |
| Current Rust material payload | verified | `src/sim/components.rs`, `src/app_instances/overlays.rs` | implementation fix |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Are garrison OccupantAnim constructor flags really 0x600? -> Yes, `Fire_At` passes `0x600` to `AnimClass__Constructor`.` (evidence: `0x006FDD50`)
- `[RESOLVED] OQ-02 - Does `DrawIt` pass only stored 0x600 to `CC_Draw_Shape`? -> No; it mutates flags, adds `0x800` when bit 0 is clear, and ORs `0x2000` before draw.` (evidence: `0x00422CA0`, asm `0x004230FE..0x00423103`, `0x00423806`)
- `[RESOLVED] OQ-03 - Does stock UC set Translucent? -> Yes, `artmd.ini` stock UC sections set `Translucent=yes`.` (evidence: `ini/artmd.ini:16131..16141`)
- `[RESOLVED] OQ-04 - Do stock UC sections set Flat/Tiled/Shadow/PingPong? -> No in scoped stock sections; constructor defaults are false and ReadINI only changes them if keys exist.` (evidence: `0x00427530`, `0x00427D00`, `ini/artmd.ini`)
- `[RESOLVED] OQ-05 - Is Shadow=yes a separate draw payload? -> Yes; after normal draw it issues a second `CC_Draw_Shape` with shadow frame offset and `flags = (flags & ~0x6) | 0x601`.` (evidence: `0x00422CA0`, asm `0x00423832..0x0042389E`)
- `[RESOLVED] OQ-06 - Does standard stock UC require the flat or tiled branch? -> No; stock `Flat` and `Tiled` are omitted/default false.` (evidence: `0x00427530`, `0x00427D00`, `ini/artmd.ini`)
- `[RESOLVED] OQ-07 - Does this scoped flag branch force a full object pool? -> No scoped evidence says material flags require object identity; they require richer per-anim runtime/material fields. Object-pool need depends on traversal/order, handled by slot 3.` (evidence: this report coverage + non-scope)
- `[DEFERRED] OQ-08 - What exactly do final `0x800` and `0x2000` do inside all blitter combinations?` (category: requires-different-system-context; reason: this slot proves flags are set but not the full `CC_Draw_Shape`/blitter table; next-step-if-pursued: focused `CC_Draw_Shape` flag table verification)
- `[DEFERRED] OQ-09 - Does global traversal/tie order require a true `g_AnimClass_Array` object pool?` (category: requires-different-system-context; reason: slot 3/slot 1 own traversal and object registration; next-step-if-pursued: reconcile swarm slot reports before implementation)

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `AnimClass::DrawIt @ 0x00422CA0` normal standard branch | `Tiled=false`, `Flat=false`; flags include base `0x600`, possible translucency bits, low-bit `0x800`, final `0x2000` | `UCFLASH/UCCONS/UCINIT`, frame `Start+CurrentFrame` | `param_2.x`, `param_2.y + YDrawOffset`; depth `YDrawOffset+ZAdjust-AdjustForZ-2` | remap/light args from cell/object path; exact blitter table deferred | yes for stock UC | garrison flash normal sprite |
| 2 | `AnimClass::DrawIt @ 0x00422CA0` shadow branch | only if `AnimType+0x372 Shadow=yes` | second half of SHP frames | same anchor family; depth `-2-AdjustForZ` | flags forced to `0x601` after clearing translucency bits | no for stock UC; conditional for modded UC | optional shadow overlay |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `UCFLASH` | yes | yes for GI stock occupied shots | yes | no | no | garrison shot flash | no | no | `rulesmd.ini OccupantAnim=UCFLASH`; `artmd.ini:16131..16133`; `0x006FDD50` |
| `UCCONS` | yes | yes for Conscript stock occupied shots | yes | no | no | garrison shot flash | no | no | `rulesmd.ini OccupantAnim=UCCONS`; `artmd.ini:16135..16137`; `0x006FDD50` |
| `UCINIT` | yes | yes for Initiate stock occupied shots | yes | no | no | garrison shot flash | no | no | `rulesmd.ini OccupantAnim=UCINIT`; `artmd.ini:16139..16141`; `0x006FDD50` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Garrison `OccupantAnim` starts with constructor flags `0x600`, but `DrawIt` adds translucency bits, `0x800` when bit 0 is clear, and final `0x2000`. | `0x006FDD50`; `0x00422CA0`; asm `0x004230FE..0x00423103`, `0x00423806` | missing draw-flag-equivalent payload; current sprite alpha is always `1.0` | `src/sim/components.rs`, `src/app_instances/overlays.rs`, needed generic anim material metadata | Carry native-equivalent material flags/translucency state from anim metadata/runtime into sprite emission. | Stock `UCFLASH` over a visible cell renders with the same translucency stage and remap/material treatment as native for each visible frame. | Do not draw stock UC flashes as opaque ordinary sprites. |
| Stock `UCFLASH/UCCONS/UCINIT` define only `Layer=ground` and `Translucent=yes` among scoped material keys; `Flat/Tiled/Shadow/PingPong` remain default false. | `ini/artmd.ini:16131..16141`; `0x00427530`; `0x00427D00` | current rules registry does not expose generic anim booleans for garrison flashes | `src/rules/art_data.rs` or a new app-layer anim metadata type | Add metadata for `Translucent`, `Translucency`, `TranslucencyDetailLevel`, `Flat`, `Tiled`, `Shadow`, `PingPong`, `YDrawOffset`, `ZAdjust` as needed by runtime/render. | Parser test for UC sections: `Translucent=true`, `Flat=false`, `Tiled=false`, `Shadow=false`, `PingPong=false`. | Do not infer omitted `Shadow` from SHP frame counts or generic comments. |
| `Shadow=yes` requires a second draw using the shadow frame half and forced `0x601` flags; stock UC skips it. | `0x00422CA0`; asm `0x00423832..0x0042389E`; `ini/artmd.ini` | no second draw support on `GarrisonMuzzleFlash` | `src/app_instances/overlays.rs`, generic anim render payload | Support optional second shadow sprite for modded occupant anims, but keep stock UC one-draw. | Modded test anim with `Shadow=yes` emits two sprite payloads; stock `UCFLASH` emits one. | Do not bake shadow behavior into all UC flashes. |
| Standard stock UC depth uses integer native expression and not screen displacement. | `0x00422CA0`; `0x006FDD50` | current Rust uses float neutral-delta bias | `src/app_instances/overlays.rs`, render depth model | Preserve unshifted position and implement/prove integer-equivalent depth ordering. | Flash overlapping a building/wall sorts identically with `ZAdjust=-200`. | Do not use arbitrary float nudges without a proof against native ordering. |

Proposed Rust test names:

- `anim_metadata_parses_stock_uc_material_defaults`
- `garrison_occupant_anim_emits_translucent_material_flags`
- `garrison_occupant_anim_shadow_yes_emits_second_shadow_sprite`
- `garrison_occupant_anim_stock_uc_no_shadow_payload`
- `garrison_occupant_anim_low_bit_absent_adds_remap_flag_equivalent`

### Negative Facts / Do Not Do

- Do not treat constructor `0x600` as the complete draw flags; `DrawIt` mutates it.
- Do not treat stock UC `Translucent=yes` as optional or opaque.
- Do not emit a shadow draw for stock UC; `Shadow` defaults false and is omitted in scoped stock sections.
- Do not shift the garrison flash screen position by `ZAdjust`; it is depth input.
- Do not claim this slot proves global object-pool/traversal parity; that is outside this report.

### Stale Docs / Follow-up Docs

- `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md` should replace any broad wording like "`Translucent=yes` affects draw flags/alpha-style path" with: "`DrawIt` starts from `AnimClass+0x190`, can OR translucency bits `0x2/0x4/0x6`, ORs `0x800` when bit `0x1` is clear, and ORs `0x2000` before the standard `CC_Draw_Shape` call. Stock UC sections set `Translucent=yes`, so an opaque sprite is not parity."
- Any implementation note that says "draw flags are 0x600" should be narrowed to: "`Fire_At` constructor flags are `0x600`; final `DrawIt` flags are derived and include at least low-bit/remap and `0x2000` behavior on the standard stock UC path."

## Sources

- Ghidra decompiled: `AnimClass::DrawIt @ 0x00422CA0`.
- Ghidra assembly context: `0x0042304B`, `0x00423075`, `0x0042307A`, `0x004230FB`, `0x004230FE..0x00423103`, `0x00423806`, `0x00423832..0x0042389E`.
- Ghidra decompiled: `AnimTypeClass::Constructor @ 0x00427530`.
- Ghidra decompiled: `AnimTypeClass::ReadINI @ 0x00427D00`.
- Ghidra decompiled: `TechnoClass::Fire_At @ 0x006FDD50`.
- INI checked: `ini/artmd.ini`, `ini/art.ini`, `ini/rulesmd.ini`.
- Rust scanned: `src/sim/components.rs`, `src/app_building_anim.rs`, `src/app_instances/overlays.rs`, `src/rules/art_data.rs`.
