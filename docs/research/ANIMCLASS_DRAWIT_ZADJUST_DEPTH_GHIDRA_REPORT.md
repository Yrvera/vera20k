# AnimClass DrawIt ZAdjust Depth - Ghidra Report

**Address(es):** `AnimClass::DrawIt @ 0x00422CA0`, `AnimClass::Constructor @ 0x00421EA0`, `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`, `BuildingClass::Update @ 0x0043FB20`, `AnimClass::GetZAdjust @ 0x00425630`, `Tactical__AdjustForZ @ 0x006D20E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact active-YR draw-depth/order expression for normal `AnimClass::DrawIt` use of instance `ZAdjust`, type `YDrawOffset`, `Tactical__AdjustForZ`, and the constant branch bias; proof that building damage-fire anims are native `AnimClass` objects that reach this path; Rust-facing damage-fire/world-effect ordering implications.  
**Non-Scope:** full palette/blitter behavior, translucency flag table, all `AnimClass::AI`, full object draw traversal, every flat/tiled/RING1 visual branch, or pixel proof for every building/body overlap.  
**Confidence:** High for constructor row order, damage-fire path liveness, standard/flat/tiled depth formulas, signedness of the arithmetic, and current Rust mismatch. Medium for final per-pixel z-buffer comparison because `CC_Draw_Shape` internals and global layer traversal are intentionally out of this slice.

## 0. Working Notes

**Target question:** What exact active-YR integer expression does `AnimClass::DrawIt` pass as the shape depth/z argument when applying `ZAdjust`, and do building damage-fire `AnimClass` objects use that same path?  

**Non-goals:** Do not investigate all palette/blitter behavior; do not re-open all `AnimClass` spawn paths; do not modify Rust or INI files; do not decide the entire generic `AnimClass` runtime architecture beyond damage-fire/world-effect render-depth handoff.  

**Evidence needed to mark COMPLETE:** direct decompile plus assembly for `DrawIt` depth expression and `Tactical__AdjustForZ`; direct constructor argument-order proof for damage-fire anims; active caller evidence from `BuildingClass::Update`; constructor registration evidence proving native `AnimClass` lifecycle; current Rust surface comparison; at least one implementation handoff item.  

**Stop conditions:** Stop after the depth arithmetic, damage-fire liveness, and Rust handoff are proven; defer final `CC_Draw_Shape` per-pixel z-buffer internals and unrelated `AnimClass` materials.

## 1. Executive Summary

Active YR building damage fires are real `AnimClass` instances, not special building overlay records. `BuildingClass::Update @ 0x0043FB20` calls `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0` when the cached damage-fire state flips on. `CreateDamageFireAnims` allocates `0x1C8`, calls `AnimClass::Constructor(type, coords, delay=0, loopCount=1, drawFlags=0x600, zAdjust=0, reverse=0)`, stores the returned pointer in one of eight building slots at `BuildingClass+0x5C8..+0x5E4`, then overwrites `AnimClass+0x100` with the computed non-positive damage-fire `ZAdjust`.

`AnimClass::Constructor @ 0x00421EA0` installs `vtable__AnimClass`, stores draw flags at `+0x190`, stores or copies instance `ZAdjust` at `+0x100`, and registers the object in `g_AnimClass_Array`. Therefore building damage fires use the normal `AnimClass` draw lifecycle and reach `AnimClass::DrawIt`, subject to ordinary visibility/lifetime gates.

For the normal non-tiled/non-flat branch of `AnimClass::DrawIt`, the integer depth argument passed to `CC_Draw_Shape` is:

```text
AnimType.YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ(AnimClass::GetZAdjust()) - 2
```

For flat anims the final constant is `-3`; for tiled anims it is `-0x32` and then decremented across repeated tile draws. Stock `FIRE01`, `FIRE02`, and `FIRE03` in `artmd.ini` omit `Flat`, `Tiled`, `YDrawOffset`, and `ZAdjust`, so damage fires enter the standard non-flat/non-tiled expression with `YDrawOffset=0` and the per-instance computed `ZAdjust`.

Current Rust carries damage-fire `z_adjust` integers, but renders with `base_depth + (1000 - z_adjust) * 0.000001` through `garrison_flash_depth_apply_z_adjust`. That is not the native integer expression and the source comment/test wording calling it native-equivalent is stale.

## 2. Verified Binary Findings

### 2.1 Damage-fire liveness from BuildingClass::Update

**Active in YR: Yes.** Evidence: `BuildingClass::Update @ 0x0043FB20` is the normal building update body. At `0x0043FC39..0x0043FC84`, it reads `BuildingTypeClass+0x157B`, compares `ObjectClass::GetHealthRatio()` against `RulesClass+0x1700` or `RulesClass+0x1708`, and computes a damage-fire active byte. At `0x0043FC84..0x0043FC97`, if this byte differs from `BuildingClass+0x5E8` and is true, it calls `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`; at `0x0043FC99..0x0043FCBE`, false destroys existing slots via virtual `+0xF8` and clears them.

Material tiny details:

- Active in YR: Yes. `BuildingTypeClass+0x157B == 0` selects `RulesClass+0x1700`; nonzero selects `RulesClass+0x1708`. Evidence: decompile and assembly `0x0043FC39..0x0043FC84`.
- Active in YR: Yes. The cached byte at `BuildingClass+0x5E8` prevents recreating damage-fire anims every tick. Evidence: `CMP BL,byte ptr [ESI+0x5E8]` at `0x0043FC84`, call only on mismatch.
- Active in YR: Yes. Repair or removal above threshold destroys each non-null slot through virtual `+0xF8` before zeroing the slot. Evidence: `0x0043FC99..0x0043FCBE`.

### 2.2 Damage-fire constructor row and slot ownership

**Active in YR: Yes.** Evidence: `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`.

At `0x0043C0D3..0x0043C105`, the function reads `RulesClass+0x2B0` as the damage-fire anim count and returns immediately if zero. At `0x0043C0F0..0x0043C105`, it chooses an initial fire type index with `Random__RandomRanged(0, count - 1)`.

At `0x0043C105..0x0043C118`, it initializes:

- `EDI = BuildingClass+0x5C8`, the first of eight damage-fire `AnimClass*` slots.
- `EBP = 0x15D8`, the first damage-fire offset record in `BuildingTypeClass`.

The loop advances `EBP += 8` and `EDI += 4` until `EBP < 0x1618` fails, giving exactly eight candidate slots. Evidence: `0x0043C27A..0x0043C28E`.

Constructor argument order is proven by the push sequence at `0x0043C1B4..0x0043C1DC`:

```text
push reverse = 0
push zAdjust = 0
push drawFlags = 0x600
push loopCount = 1
push delay = 0
push &coords
push RulesClass.BuildingDamageFireAnims[start_index]
ECX = allocated 0x1C8 object
call AnimClass::Constructor @ 0x00421EA0
```

Material tiny details:

- Active in YR: Yes. If a damage-fire offset is the sentinel pair at globals `0x0089C848/0x0089C84C`, the whole creator returns, not just that slot. Evidence: `0x0043C118..0x0043C137`.
- Active in YR: Yes. If the destination slot pointer is non-null, the whole creator returns. Evidence: `CMP dword ptr [EDI],0` then `JNZ 0x0043C294` at `0x0043C13D..0x0043C140`.
- Active in YR: Yes. The constructor coordinate is `IsometricPixelToWorld(offset) + building.GetCoords()`, with z initialized to zero before adding building z. Evidence: `0x0043C146..0x0043C1A0`.
- Active in YR: Yes. The returned `AnimClass*` is stored into the building slot before `ZAdjust` and random start frame are written. Evidence: `MOV [EDI], EBX` at `0x0043C1EB`, followed by z math at `0x0043C1ED..0x0043C237`.
- Active in YR: Yes. Damage-fire `ZAdjust` is a signed 32-bit integer written to `AnimClass+0x100`; positive values are clamped to zero. Evidence: signed arithmetic uses `SAR ECX,1`, writes `[EBX+0x100]`, then `TEST EAX,EAX` / `JLE` else writes `0` at `0x0043C20C..0x0043C237`.
- Active in YR: Yes. If the chosen anim type `End` at `AnimType+0x2C0` is positive, a second RNG call selects initial `AnimClass+0xAC` in `[0, End-1]`. Evidence: `0x0043C237..0x0043C25B`.
- Active in YR: Yes. After each successful spawn attempt, the fire type index increments and wraps to zero when it reaches the count. Evidence: `0x0043C261..0x0043C272`.

### 2.3 Constructor registration and native object identity

**Active in YR: Yes.** Evidence: `AnimClass::Constructor @ 0x00421EA0`, called directly by `CreateDamageFireAnims`.

`AnimClass::Constructor` stores `vtable__AnimClass` at `AnimClass+0x0` and registers the instance in the global anim array:

- `0x0042201C..0x00422030`: writes `vtable__AnimClass` and secondary vtables.
- `0x00422058..0x004220AA`: grows or uses `g_AnimClass_Array`, increments `g_AnimClass_Array_Count`, and stores the `AnimClass*`.
- `0x00421FD2`: stores constructor draw flags at `AnimClass+0x190`.
- `0x0042219E..0x004221BF`: if constructor `zAdjust` is nonzero, writes it to `AnimClass+0x100`; otherwise copies `AnimTypeClass+0x348` to `AnimClass+0x100`.

Damage-fire callers pass constructor `zAdjust=0`, so constructor initially copies the type `ZAdjust`; `CreateDamageFireAnims` then overwrites `+0x100` with the computed damage-fire slot z-adjust. Active in YR: Yes, because `CreateDamageFireAnims` immediately writes the field after storing the returned pointer.

### 2.4 DrawIt normal branch expression

**Active in YR: Yes for stock damage fires.** Evidence: stock YR `ini/rulesmd.ini:519` lists `DamageFireTypes=FIRE01,FIRE02,FIRE03`; `ini/artmd.ini:16018..16035` defines these three sections with `Rate`, `LoopCount`, and `StartSound` only. They omit `Flat`, `Tiled`, `YDrawOffset`, and `ZAdjust`, so constructor defaults/type defaults select non-tiled, non-flat, `YDrawOffset=0`, type `ZAdjust=0`, then damage-fire code overwrites instance `ZAdjust`.

In `AnimClass::DrawIt @ 0x00422CA0`, the standard branch is:

- Active in YR: Yes. `0x00423630..0x0042363E` tests `AnimType+0x35B` (`Tiled`); false goes toward standard/flat handling.
- Active in YR: Yes. `0x00423728..0x00423730` tests `AnimType+0x369` (`Flat`); false enters standard non-flat handling at `0x004237AE`.
- Active in YR: Yes. `0x004237AE..0x004237BB` computes the screen point as `x = param_2.x`, `y = param_2.y + AnimType.YDrawOffset`. Instance `ZAdjust` is not applied to screen position.
- Active in YR: Yes. `0x004237D7..0x004237DF` calls vtable `+0x1D0`, which is `AnimClass::GetZAdjust @ 0x00425630`, then passes the returned integer in `ECX` to `Tactical__AdjustForZ @ 0x006D20E0`.
- Active in YR: Yes. `0x004237E4..0x00423803` loads `AnimType+0x344`, loads `AnimClass+0x100`, adds them, subtracts `AdjustForZ`, and subtracts constant `2`.
- Active in YR: Yes. `0x00423806..0x00423827` ORs `0x2000` into the draw flags and pushes the computed integer as the depth/z argument to `CC_Draw_Shape @ 0x004AED70`.

Exact standard expression:

```text
shape_depth = *(int *)(AnimType + 0x344)
            + *(int *)(AnimClass + 0x100)
            - Tactical__AdjustForZ(AnimClass::GetZAdjust(anim))
            - 2
```

Signedness:

- Active in YR: Yes. `YDrawOffset` and instance `ZAdjust` are loaded as full 32-bit dwords and combined with ordinary 32-bit add/sub. Evidence: `MOV EDX,[ECX+0x344]`, `MOV EBP,[ESI+0x100]`, `ADD EDX,EBP`, `SUB EDX,EAX` at `0x004237E4..0x00423803`.
- Active in YR: Yes. Damage-fire producer writes a signed non-positive 32-bit value to `AnimClass+0x100`, and `DrawIt` consumes the exact dword. Evidence: producer `0x0043C20C..0x0043C237`; consumer `0x004237EA..0x004237FD`.

### 2.5 Flat, tiled, shadow, and RING1 contrasts

**Active in YR: Conditional.** These branches exist and are active for anim types whose INI/default flags select them, but stock damage fires do not set those flags.

- Flat branch: `0x00423732..0x0042379E` computes `YDrawOffset + ZAdjust - AdjustForZ - 3` and ORs `0x2000`. Active in YR: Conditional on `AnimType+0x369`/`Flat=yes`; stock `FIRE01..03` omit it.
- Tiled branch: `0x00423644..0x0042371B` computes initial `YDrawOffset + ZAdjust - AdjustForZ - 0x32`, then decrements the depth as repeated tiles are drawn. Active in YR: Conditional on `AnimType+0x35B`/`Tiled=yes`; stock `FIRE01..03` omit it.
- Shadow branch: after standard draw, `0x0042383C..0x0042389E` uses shadow depth `-2 - AdjustForZ`, clears some bits, and forces `0x601`. Active in YR: Conditional on `AnimType+0x372`/`Shadow=yes`; stock `FIRE01..03` omit it.
- RING1 special branch: `0x00422CA0..0x00422FCC` has a warp-ring path that still uses `YDrawOffset + ZAdjust - AdjustForZ - 2` inside its custom rasterization setup. Active in YR: Conditional on globals and `RING1`; not relevant to building damage fires.

### 2.6 Tactical__AdjustForZ and AnimClass::GetZAdjust

**Active in YR: Yes.** Evidence: `DrawIt` standard/flat/tiled branches call `Tactical__AdjustForZ` after vtable `+0x1D0`.

`AnimClass::GetZAdjust @ 0x00425630`:

```text
z = *(int *)(AnimClass + 0xA4)
if (*(int *)(AnimClass + 0xCC) != 0) {
    z += *(int *)(OwnerObject + 0xA4)
}
return z
```

Evidence: decompile and assembly `0x00425630..0x0042566E`.

`Tactical__AdjustForZ @ 0x006D20E0`:

```text
threshold_add = (z >= 0x2D8) ? 1 : 0
return Math__ftol((double)z * *(double *)0x00B0CD48 + threshold_add + 0.5)
```

Evidence: `0x006D20E3` compares `ECX` with `0x2D8`; `0x006D20ED..0x006D20FF` stores `0` or `1`; `0x006D20FF` `FILD`s signed z; `0x006D2103` multiplies by global `0x00B0CD48`; `0x006D2109` adds threshold; `0x006D210D` adds double `0.5`; `0x006D2113` calls `Math__ftol`.

Active in YR: Yes for any drawn non-invisible `AnimClass`, including damage-fire `AnimClass` objects, because `DrawIt` directly calls it before `CC_Draw_Shape`.

## 3. Current Rust Surfaces

Current Rust damage-fire runtime:

- `src/app_building_anim.rs:74..79` says the app-side bridge tracks native damage-fire slot lifetime and RNG order "as far as the current overlay surface allows."
- `src/app_building_anim.rs:247..288` creates `DamageFireAnim` records, stores slot, type, frame, and `z_adjust`.
- `src/sim/components.rs:650..677` defines `DamageFireOverlays` / `DamageFireAnim`; this is app-side component storage, not a global registered `AnimClass`.
- `src/sim/components.rs:769..826` defines `AnimClassSpawnDescriptor` and `WorldEffect::anim_spawn`, but damage-fire overlays do not use this descriptor and are not `WorldEffect`s.

Current Rust rendering:

- `src/app_instances/overlays.rs:106..160` builds damage-fire sprite instances from `DamageFireOverlays`.
- `src/app_instances/overlays.rs:156` applies `garrison_flash_depth_apply_z_adjust(fire_depth, fire.z_adjust)`.
- `src/app_instances/overlays.rs:547..551` implements that as `base_depth + (1000 - z_adjust) * 0.000001`, clamped to `[0.001, 0.999]`.

Verdict: Rust preserves some producer-side slot/ZAdjust data, but the render depth mechanism is not active-YR native-equivalent. The native path carries a registered `AnimClass` into `DrawIt` and passes a signed integer expression to `CC_Draw_Shape`; Rust applies an ad-hoc normalized float bias to a building-overlay sprite.

## 4. Negative Facts / Do Not Do

- Do not treat `1000` as native neutral for `AnimClass::DrawIt` `ZAdjust`. Active in YR: No for this expression. `1000` appears as another draw parameter/default in `DrawIt`, but standard shape depth is `YDrawOffset + ZAdjust - AdjustForZ - 2`.
- Do not apply damage-fire `ZAdjust` to screen Y. Active in YR: No. Screen Y uses `param_2.y + YDrawOffset`; `ZAdjust` is consumed in the separate depth argument.
- Do not collapse building damage fires into generic `WorldEffect` defaults. Active in YR: No. The native path stores eight `AnimClass*` slots on the building and registers each constructed anim globally.
- Do not assume non-flat `-2` for every anim. Active in YR: Conditional. Flat uses `-3`; tiled uses `-0x32` plus per-tile decrements; stock damage fires use the standard `-2` path.
- Do not use floor or unchecked floating rounding for `Tactical__AdjustForZ`. Active in YR: No. The helper uses x87 `Math__ftol` after adding threshold and `0.5`.

## 5. Stale Docs / Source Wording

No tracked research doc in this slot was found claiming the current Rust float helper is native-equivalent for damage-fire. Two source-level comments/tests are stale and should be replaced during implementation:

- `src/app_instances/overlays.rs:548..549` currently says `CC_Draw_Shape treats z_adjust as a sort-depth bias: 1000 is neutral...`; replace with: `Temporary approximation: native AnimClass::DrawIt uses signed integer depth YDrawOffset + ZAdjust - AdjustForZ(anim_z) - 2 for standard non-flat anims; this float bias is not native-equivalent.`
- `src/app_instances/overlays.rs:846` test name `garrison_flash_depth_applies_native_z_adjust_as_depth_bias`; replace with: `garrison_flash_depth_approximation_preserves_no_screen_shift_until_native_depth_helper`.

Exact doc-path stale replacement wording: None found.

## 6. Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Damage-fire `AnimClass` objects use normal standard `DrawIt` depth: `YDrawOffset + AnimClass.ZAdjust - Tactical__AdjustForZ(anim.GetZAdjust()) - 2` for stock `FIRE01..03`. | Replace/prove current float `garrison_flash_depth_apply_z_adjust` approximation with a native integer anim-depth helper feeding renderer ordering. | `src/app_instances/overlays.rs`; shared render-depth helper; damage-fire render path. | GACNST slot using `FIRE01`, `YDrawOffset=0`, computed `ZAdjust=-192`, and `anim_z=0` produces native integer `-194` before renderer normalization, not `base + 0.001192`. | `damage_fire_anim_depth_uses_native_drawit_integer_formula` | High: current float bias can sort damage fires differently against building bodies/walls. |
| `CreateDamageFireAnims` constructs real `AnimClass` rows with `(delay=0, loop=1, drawFlags=0x600, zAdjust=0, reverse=0)`, stores eight building slots, then overwrites instance `+0x100`. | Model damage-fire rows as generic `AnimClass`-like runtime entries or preserve all constructor row fields plus post-constructor `ZAdjust` in the app bridge; do not hide them as anonymous overlays. | `src/app_building_anim.rs`; `src/sim/components.rs`; damage-fire component/runtime shape. | Damaging a building across threshold creates up to eight records with slot index, original constructor row, and post-constructor `z_adjust`; repair destroys the same slots. | `damage_fire_slots_preserve_constructor_row_and_post_zadjust` | High: missing native object identity blocks DrawIt/lifetime parity and future global anim ordering. |
| `Tactical__AdjustForZ` uses signed z, runtime multiplier, threshold addend at `z >= 728`, plus `0.5`, then `Math__ftol`. | Add an injectable native `AdjustForZ` helper/test fixture rather than deriving depth from `compute_sprite_depth_params` terrain float depth. | render math helper; `src/app_instances/overlays.rs`; possibly tactical camera state. | Samples at `z=727`, `z=728`, a high positive z, and one negative z match the binary formula for a supplied multiplier. | `tactical_adjust_for_z_matches_binary_threshold_and_rounding` | Medium: wrong threshold/rounding creates 1-pixel depth disparities that compound with `ZAdjust`. |

## 7. Remaining Uncertainty

- Exact downstream `CC_Draw_Shape` per-pixel z-buffer comparison for one concrete damage-fire-over-building overlap remains deferred; this slot proves the integer argument passed into it, not final pixel collisions.
- Exact runtime `g_AdjustForZ_Multiplier` values for each camera/zoom state were not captured; the helper formula is proven, but acceptance tests should inject multiplier or capture it from the app's native-equivalent tactical state.
- The final global traversal order for registered damage-fire anims relative to all other `ObjectClass` instances is covered by sibling ordering reports, not re-investigated here.

## 8. Evidence Log

- Live read-only Ghidra decompile/disassembly: `AnimClass::DrawIt @ 0x00422CA0`, `AnimClass::Constructor @ 0x00421EA0`, `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`, `BuildingClass::Update @ 0x0043FB20`, `AnimClass::GetZAdjust @ 0x00425630`, `Tactical__AdjustForZ @ 0x006D20E0`.
- Ghidra caller evidence: `BuildingClass::Update @ 0x0043FB20` is a caller of `BuildingClass::CreateDamageFireAnims @ 0x0043C0D0`.
- INI evidence: `ini/rulesmd.ini:519` `DamageFireTypes=FIRE01,FIRE02,FIRE03`; `ini/artmd.ini:16018..16035` stock damage-fire art definitions.
- Rust evidence: `src/app_building_anim.rs`, `src/sim/components.rs`, `src/app_instances/overlays.rs`.

## 9. Status

COMPLETE for the scoped damage-fire/`DrawIt` depth contract and Rust-facing handoff.
