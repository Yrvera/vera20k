# Building First-Pass Display Path 0043DA80 - Ghidra Report

**Target:** `BuildingClass` vtable `+0x104(flag=1)` dispatch through `FUN_0043CEA0` to `FUN_0043DA80` during the first tactical object pass.
**Scope:** identify player-visible art drawn before the later second `DrawExtras` pass, and bracket occlusion/interleaving impact.
**Mode:** read-only Ghidra/live decompilation. No Ghidra mutation, Rust edit, INI edit, or in-repo doc edit.
**Status:** COMPLETE.

## Conclusion

`FUN_0043DA80 @ 0x0043DA80` is the first-pass building VXL/extras path, not the normal SHP body path.

In the standard first tactical object pass, selected buildings have already run `DrawBehind` and first-pass `DrawExtras`; then `+0x104(flag=1)` reaches `FUN_0043DA80` if `building+0x6E7 == 0`. This path can draw conditional construction/gate SHP overlay art and VXL building body/turret art. It does not draw the ordinary SHP building body, the damaged bib SHP, normal ActiveAnim/ProductionAnim/PowerUp/TurretAnim slots, or the `g_BUILDNGZ_SHA` body z-shape used by the ordinary SHP body path.

Bracket consequence: for ordinary SHP buildings, the first-pass visible building body was not drawn by this slot. First-pass selected brackets are therefore followed by no ordinary SHP body from `FUN_0043DA80`; their final visible ordering is corrected later by the second `DrawExtras` pass. For VXL-bodied/turreted buildings, `FUN_0043DA80` can draw VXL art after first-pass brackets and before the second `DrawExtras`, so the second pass is the one that reasserts front bracket visibility above VXL/extras and later first-pass object art.

## Verified Binary Evidence

### 1. `+0x104(flag=1)` reaches `FUN_0043DA80`, not `DrawBody`

**Active in YR:** Yes.

`Tactical_ObjectRenderingLoop @ 0x006D8DB0` marks visible Techno objects at `object+0x99`, calls `vtable+0x10C`, calls `vtable+0x110`, then calls `vtable+0x104` with the nonzero first-pass flag for non-foot Techno objects. `FUN_0043CEA0 @ 0x0043CEA0` dispatches the nonzero flag case to `vtable+0x4E4` when `building+0x6E7 == 0`; BuildingClass vtable `+0x4E4` resolves to `FUN_0043DA80`. The zero-flag case calls `vtable+0x114`, which is `BuildingClass_DrawBody @ 0x0043D290`.

### 2. Ordinary SHP body and damaged bib are not drawn by `FUN_0043DA80`

**Active in YR:** Yes.

Current Ghidra decompile of `BuildingClass_DrawBody @ 0x0043D290` shows the ordinary body `TechnoClass_DrawSHP` call using the selected body SHP and `g_BUILDNGZ_SHA`, followed by the damaged bib guard `Type+0x1518 != 0 && building+0x534 != 0`. Current Ghidra decompile of `FUN_0043DA80 @ 0x0043DA80` does not contain that `Type+0x1518` damaged-bib branch and does not reference `g_BUILDNGZ_SHA`; its SHP draw site is the separate construction/gate overlay path at `Type+0x150C`.

### 3. `FUN_0043DA80` can draw a construction/gate SHP overlay

**Active in YR:** Conditional.

Inside `FUN_0043DA80 @ 0x0043DA80`, when `vtable+0x184` returns `0x10`, the type has `Type+0x150C != 0`, and the gate/construction timing helpers (`FUN_004A5110`, `FUN_004A5130`, `FUN_004A51B0`, `FUN_004A51D0`) report an active state, the function computes a frame index from `Type+0xF00`, applies damaged-half selection when health ratio is at or below `Rules+0x1700` and `Type+0x1700` is set, then calls `TechnoClass_DrawSHP(Type+0x150C, frame, ..., y=-5-AdjustForZ, layer args 0/0, cell level)`.

This is player-visible overlay art in the first-pass slot, but it is not the ordinary building body or normal attached AnimClass slots.

### 4. `FUN_0043DA80` can draw VXL building body/turret art

**Active in YR:** Conditional.

The decompile gates the VXL branch through building type bytes around `Type+0x16C5` and `Type+0x16C6`. In the non-hidden states, it builds matrices with `BuildVXLTurretMatrix @ 0x00458810`, `FUN_00754BE0`, and `Locomotion_Matrix`, then calls `vtable+0x444`, which BuildingClass inherits as `TechnoClass__Draw @ 0x00706640`.

Verified calls include body/turret VXL pointers and cache fields:

- body-ish call using `Type+0xB8`, frame/facing component from `building+0x5xx`, cache at `Type+0x244`;
- turret-ish call using `Type+0xC0`, facing fields, cache at `Type+0x280`;
- optional repeated turret/body call when return-state gates request it.

`TechnoClass__Draw @ 0x00706640` either blits a cached VXL image through `VXL_CacheBlit @ 0x00707480` or rasterizes through `TechnoClass__Render @ 0x00706ED0`, so this is real player-visible voxel art.

### 5. Normal building animation slots and voxel shadows are not directly drawn here

**Active in YR:** Yes for the negative classification; Conditional for separate systems.

`FUN_0043DA80 @ 0x0043DA80` does not iterate the 21 building `Anims[]` slots and does not call AnimClass draw. Normal PowerUp, ActiveAnim, ProductionAnim, TurretAnim, LowPower, and damage-fire visuals remain separate layer-sorted AnimClass work, not this first-pass `+0x104(flag=1)` body/extras slot.

The current `FUN_0043DA80` decompile also does not call `ObjectClass__DrawVoxelShadow @ 0x005F5B90`. `TechnoClass__Draw @ 0x00706640` and `TechnoClass__Render @ 0x00706ED0` handle cached/rasterized VXL body pixels, but the verified shadow helper is separate. Therefore this slot should not be treated as the source of voxel shadow rendering unless a separate caller trace proves a shadow call immediately around it.

## Inference From Evidence

- Ordinary SHP buildings: first-pass `FUN_0043DA80` is usually visually empty except for conditional construction/gate overlay cases. It does not cover the body art that would occlude first-pass brackets in a simple per-object model.
- VXL/turreted buildings: this first-pass slot can visibly overdraw first-pass brackets with VXL body/turret pixels. The later second `DrawExtras` pass is necessary to put selected front bracket work back above all first-pass object drawing.
- For bracket parity, the key ordering is still `DrawBehind -> first DrawExtras -> FUN_0043DA80 -> ...all first-pass objects... -> second DrawExtras`; the contents of `FUN_0043DA80` are conditional but not ignorable for VXL buildings.

## Open Questions

- Which stock YR buildings set the `Type+0x16C5/0x16C6` VXL gates and therefore visibly use this first-pass VXL path?
- Which art parser keys map exactly to `Type+0x150C`, `Type+0x16C5`, and `Type+0x16C6` in current naming?
- Is there a nearby standard tactical shadow call for VXL buildings in the same frame phase, outside this function?

## Sources

- Ghidra decompile: `Tactical_ObjectRenderingLoop @ 0x006D8DB0`
- Ghidra decompile: `FUN_0043CEA0 @ 0x0043CEA0`
- Ghidra decompile: `BuildingClass_DrawBody @ 0x0043D290`
- Ghidra decompile: `FUN_0043DA80 @ 0x0043DA80`
- Ghidra decompile: `TechnoClass__Draw @ 0x00706640`
- Ghidra decompile: `TechnoClass__Render @ 0x00706ED0`
- Ghidra decompile: `VXL_CacheBlit @ 0x00707480`
- Ghidra decompile: `ObjectClass__DrawVoxelShadow @ 0x005F5B90`
