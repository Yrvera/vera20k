# AnimClass Draw Traversal / Layer Ordering - Re-Swarm Slot 3

**Date:** 2026-05-27  
**Target:** AnimClass draw traversal/layer ordering for garrison `OccupantAnim`.  
**Investigation mode:** exhaustive-slice for traversal/order dependency; draw flags and lifecycle are non-goals.  
**Active in YR:** Yes. `TechnoClass::Fire_At` creates ordinary `AnimClass` objects for stock occupied-building `OccupantAnim`, and `TacticalClass::Draw` traverses the same display-layer system in normal tactical rendering.

## Working Notes

**Target question:** Does garrison `OccupantAnim` draw order against buildings/walls depend on the global `g_AnimClass_Array` order, a native `AnimClass` object pool/list, DisplayLayer insertion order, object id/order, or only layer/depth/Y-sort?

**Non-goals:** Do not restudy `AnimClass::AI` lifecycle, `DrawIt` flags/translucency, `Tactical_AdjustForZ` internals, building body drawing, or wall blitter flags beyond what is required to identify traversal ownership.

**Evidence needed to mark COMPLETE:** Direct decompile of display submission, sorted insertion, object rendering loop, `AnimClass::GetLayer`, `AnimClass` Y-sort override, and tactical draw phase order; caller/callee confirmation that `Tactical_ObjectRenderingLoop` draws from display layers, not `g_AnimClass_Array`.

**Stop conditions:** Stop once the render traversal owner and ordering key are proven for non-flat stock UC `OccupantAnim`; leave exact pixel/blitter flag behavior to slot 4/5.

## Summary

Native tactical rendering does not traverse `g_AnimClass_Array` to draw garrison `OccupantAnim`. The global anim array is used for lifecycle/ownership scans, but visible object rendering walks `g_DisplayLayers[0..4]`. `DisplayClass::Submit_Object @ 0x004A9720` asks the object for its layer via vtable `+0x78`, then inserts it into the corresponding `g_DisplayLayers` vector. Only layer `2` is sorted on insert.

For stock garrison `UCFLASH`/`UCCONS`/`UCINIT`, `AnimClass::GetLayer @ 0x00424CB0` returns the `AnimTypeClass+0x364` layer when the anim is not attached to an owner. Stock UC art has `Layer=ground`, so the object enters the ground layer. The occupied-building branch also does not attach the anim to the shooter after writing `ZAdjust=-200`, so the type layer path is the relevant path.

Within ground layer, `DynamicVector__Insert @ 0x005519B0` calls `DynamicVector__SortedInsert @ 0x00551A90`. The comparator is `ObjectClass__YSortComparator @ 0x005F6220`, which calls each object's vtable `+0xB8`. For `AnimClass`, vtable `+0xB8` is `0x00422BC0`; decompile shows it returns `ObjectClass__GetYSort() + AnimClass+0x104`. Constructor `0x00421EA0` copies `AnimTypeClass+0x340` (`YSortAdjust`) into `AnimClass+0x104`. Stock UC sections omit `YSortAdjust`, so the adjustment is zero.

`ObjectClass__GetYSort @ 0x005F6BD0` returns render-coordinate `X + Y`; it does not include Z/depth. `ObjectClass__YSortComparator @ 0x005F6220` has no secondary tiebreaker. Equal Y-sort values therefore keep existing insertion order because sorted insertion continues while the comparator is false and inserts after equal existing entries.

`TacticalClass::Draw @ 0x006D3F50` draws terrain phase first, including overlays/walls, then object phase through `Tactical_ObjectRenderingLoop @ 0x006D8DB0`. `Tactical_ObjectRenderingLoop` iterates layer `0..4`, and for each layer iterates the layer vector by index, calling object draw virtuals. After layer `2`, it performs a separate `g_BuildingClass_Array` turret/garrison-fire update pass. That pass is not the `OccupantAnim` traversal path; stock shot `OccupantAnim` is already a normal layer object.

## Verified Facts

1. `DisplayClass::Submit_Object @ 0x004A9720` removes the object if `object+0x94 != -1`, calls vtable `+0x78` for layer, and inserts into the selected display layer; sorted flag is exactly `layer == 2`. Evidence: decompile `0x004A9720`; callees `DisplayClass__RemoveFromLayer @ 0x004A9770`, `DynamicVector__Insert @ 0x005519B0`.

2. `AnimClass::GetLayer @ 0x00424CB0` returns `2` if `AnimClass+0xCC != 0`; otherwise it returns `AnimTypeClass+0x364` when the type pointer at `+0xC8` is non-null; otherwise fallback `3`. Evidence: decompile `0x00424CB0`. For occupied-building shots, prior report verifies the building branch does not call `SetOwnerObject` after `ZAdjust=-200`, so type layer controls stock UC.

3. Ground-layer sorted insertion does not use object id or global anim-array index. `DynamicVector__Insert @ 0x005519B0` calls `DynamicVector__SortedInsert @ 0x00551A90` only when the sorted flag is set; sorted insert walks existing layer entries and calls `ObjectClass__YSortComparator @ 0x005F6220`. Evidence: decompile `0x005519B0`, `0x00551A90`, `0x005F6220`.

4. `ObjectClass__YSortComparator @ 0x005F6220` compares only vtable `+0xB8` results. `ObjectClass__GetYSort @ 0x005F6BD0` returns `GetRenderCoords().X + GetRenderCoords().Y`. `AnimClass` vtable `+0xB8` is `0x00422BC0`, which returns `ObjectClass__GetYSort() + AnimClass+0x104`. Constructor `0x00421EA0` copies `AnimTypeClass+0x340` into `AnimClass+0x104`. Evidence: decompile `0x005F6220`, `0x005F6BD0`, `0x00422BC0`, `0x00421EA0`.

5. `Tactical_ObjectRenderingLoop @ 0x006D8DB0` draws from `g_DisplayLayers` vectors, not `g_AnimClass_Array`. It iterates five layers, iterates each layer buffer by index, calls draw virtuals (`+0x104` main draw on visible objects), and after layer `2` separately iterates `g_BuildingClass_Array` for `BuildingClass__UpdateGarrisonFire`. Evidence: decompile `0x006D8DB0`; callee list contains no anim-array traversal and includes `BuildingClass__UpdateGarrisonFire @ 0x0043E7B0`.

## Decision Handoff

- Approach 2, a shared app-layer `AnimRuntime` embedded in garrison flashes, can be parity-correct for draw traversal if the produced sprite instances enter the same layer-2/Y-sort stream with native-equivalent `GetYSort + YSortAdjust` and stable equal-key insertion order. A full app-side `AnimClass` pool is not required solely for garrison flash draw ordering.

- Do not key draw order to `g_AnimClass_Array` order. Keep any future global app anim collection separate from render ordering unless it feeds DisplayLayer-style submission.

- Exact depth/pixel parity still needs the slot 4/5 contracts: stock UC `DrawIt` flags/translucency and the integer `YDrawOffset + ZAdjust - Tactical_AdjustForZ() - 2` depth must be represented/proven in the renderer.

## Negative Facts / Do Not Do

- Do not draw garrison `OccupantAnim` by iterating `g_AnimClass_Array` order; native object draw traversal does not do that.

- Do not use object unique id as a tiebreaker for ground-layer equal Y-sort; the verified comparator has no id tiebreaker.

- Do not treat `ZAdjust=-200` as the layer-order key. Layer insertion uses `GetYSort + YSortAdjust`; `ZAdjust` is consumed inside `AnimClass::DrawIt`/shape depth, not the DisplayLayer comparator.

- Do not put stock non-flat UC `OccupantAnim` into the flat terrain animation pass. Stock UC sections are not `Flat=yes`; they use `Layer=ground`.

- Do not model the post-layer-2 `BuildingClass__UpdateGarrisonFire` pass as the shot `OccupantAnim` path. The shot flash is a normal `AnimClass` layer object.

## Remaining Uncertainty

- Exact wall-pixel over/under behavior still depends on `AnimClass::DrawIt` flags/translucency and shape blitter z-mode, not on the traversal list. This slot proves ordering ownership, but slot 4/5 should finish the pixel contract.

## Proposed Rust Tests

- `garrison_occupant_anim_enters_ground_y_sort_stream`
- `garrison_occupant_anim_y_sort_uses_fire_origin_plus_y_sort_adjust`
- `garrison_occupant_anim_equal_y_sort_preserves_spawn_order`
- `garrison_occupant_anim_draw_order_not_anim_pool_index`
- `garrison_occupant_anim_z_adjust_does_not_change_layer_sort_key`

## Stale-Doc Replacement Wording

- `docs/research/OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`: replace "exact global render traversal and z-buffer comparator outside scope" with: "Global traversal is now verified in `ANIMCLASS_DRAW_TRAVERSAL_LAYER_ORDERING_RESWARM_20260527.md`: non-flat stock UC `OccupantAnim` enters `g_DisplayLayers` through `DisplayClass::Submit_Object`, uses layer 2 ground sorted insertion, and sorts by `ObjectClass::GetYSort + AnimClass.YSortAdjust`; draw order does not depend on `g_AnimClass_Array` order. Exact blitter flags/pixel z behavior remains separate."

- `docs/research/ANIM_CLASS_DEEP_DIVE.md`: vtable row `+0x0B8 / 0x00422BC0` should be labeled `AnimClass::GetYSort` or `AnimClass::GetYSortWithAdjust`, not `GetRenderColor`; decompile shows it returns `ObjectClass__GetYSort() + AnimClass+0x104`.

## Sources

- Ghidra decompiled: `TacticalClass::Draw @ 0x006D3F50`
- Ghidra decompiled: `Tactical_ObjectRenderingLoop @ 0x006D8DB0`
- Ghidra decompiled: `DisplayClass::Submit_Object @ 0x004A9720`
- Ghidra decompiled: `DisplayClass::RemoveFromLayer @ 0x004A9770`
- Ghidra decompiled: `DynamicVector__Insert @ 0x005519B0`
- Ghidra decompiled: `DynamicVector__SortedInsert @ 0x00551A90`
- Ghidra decompiled: `ObjectClass__YSortComparator @ 0x005F6220`
- Ghidra decompiled: `ObjectClass__GetYSort @ 0x005F6BD0`
- Ghidra decompiled: `AnimClass::GetLayer @ 0x00424CB0`
- Ghidra decompiled: `AnimClass::GetYSortWithAdjust @ 0x00422BC0`
- Ghidra decompiled: `AnimClass::Constructor @ 0x00421EA0`
- Prior docs checked: `OCCUPANTANIM_ANIMCLASS_LIFECYCLE_DRAWIT_DEPTH_GHIDRA_REPORT.md`, `ZBUFFER_DEPTH_SYSTEM.md`, `DRAW_ORDER_DEPTH_SYSTEM.md`, `DISPLAYCLASS_GHIDRA_REPORT.md`, `ANIM_CLASS_DEEP_DIVE.md`

**Status:** COMPLETE for traversal/order dependency; wall pixel/blitter flags intentionally deferred.
