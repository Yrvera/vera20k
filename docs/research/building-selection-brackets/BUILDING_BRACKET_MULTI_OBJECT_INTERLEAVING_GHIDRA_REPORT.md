# Building Bracket Multi-Object Interleaving - Ghidra Research Report

**Address(es):** `0x006D8DB0` primary; `0x006D3D10`, `0x006F60D0`, `0x006F5190`, `0x0043CEA0`, `0x0043D290`, `0x0043DA80`, `0x00459EC0`, `0x006F2B40`, `0x004D34D0`, `0x005F3920`
**Investigation Mode:** exhaustive-slice for selected-building bracket dispatch order around `Tactical_ObjectRenderingLoop`
**Claimed Scope:** order of selected building `DrawBehind`, `DrawExtras`, and building draw dispatch across multiple display-layer objects; standard YR reachability for that order.
**Non-Scope:** exact bracket edge topology, final `Surface::Draw_Line` raster/depth semantics, health-pip anchor details, and exact layer sort insertion policy.
**Confidence:** High for dispatcher order and building flag path; Medium for final overlap visual wording because this report is static Ghidra evidence, not a runtime pixel capture.
**Active in YR:** Yes. Standard tactical drawing calls `Tactical_ObjectRenderingLoop`, standard buildings set `AbstractFlags` bit `0x01` and do not set bit `0x04`, and selected building brackets are gated by `WhatAmI()==6` plus selected byte `+0x83`.

## 1. Result

The selected-building bracket path is not a simple per-object sequence of:

`DrawBehind -> building body -> DrawExtras`

For standard buildings, the binary uses a hybrid order:

1. First object pass, per object in display-layer order:
   `vtable+0x10C` (`TechnoClass::DrawBehind`) -> `vtable+0x110` (`TechnoClass::DrawExtras`) -> `vtable+0x104` (`BuildingClass` draw dispatcher, called with flag `1`).
2. After all five first-pass display layers finish, a second pass iterates display layers again and calls `vtable+0x110` (`TechnoClass::DrawExtras`) for every visible techno object marked at `object+0x99`.

Therefore multi-building overlap cases are effectively phase-batched for the final selected-building front bracket pass: every first-pass object draw has already happened before the second `DrawExtras` pass starts. The first-pass `DrawExtras` still exists and is per-object, but the later `DrawExtras` call re-emits selected-building front bracket work after all first-pass objects.

## 2. Verified Binary Evidence

### 2.1 Tactical draw reaches the object rendering loop

`TacticalClass_Draw @ 0x006D3D10` calls `Tactical_ObjectRenderingLoop @ 0x006D8DB0` in the normal tactical draw path after placement overlays and before laser/electric/beam/action visual passes.

**Active in YR:** Yes. This is the standard tactical draw pipeline, not a TS-only branch.

### 2.2 Standard buildings take the non-foot Techno branch

`ObjectClass::Constructor @ 0x005F3920` sets `AbstractFlags` bit `0x02` at `object+0x14`.
`TechnoClass::Constructor @ 0x006F2B40` sets bit `0x01` at `object+0x14`.
`FootClass::Constructor @ 0x004D34D0` sets bit `0x04` at `object+0x14`.
`BuildingClass::Constructor @ 0x0043B680` calls the Techno constructor and installs `vtable_BuildingClass`, but it does not set bit `0x04`.

So a standard `BuildingClass` reaches the `(flags & 0x04)==0 && (flags & 0x01)!=0` branch in `0x006D8DB0`.

**Active in YR:** Yes. This is constructor-time class identity for normal buildings.

### 2.3 First pass order for buildings is DrawBehind, DrawExtras, then vtable+0x104

In `Tactical_ObjectRenderingLoop @ 0x006D8DB0`, the non-foot Techno branch at assembly `0x006D90A9..0x006D9172` does this after viewport acceptance:

1. sets `object+0x99 = 1` at `0x006D9111..0x006D9118`;
2. calls `vtable+0x10C` at `0x006D913C`;
3. calls `vtable+0x110` at `0x006D9153`;
4. pushes `1` and calls `vtable+0x104` at `0x006D915B..0x006D916C`.

For `BuildingClass`, vtable memory at `0x007E3FC0` resolves:

| Vtable offset | Function |
|---|---|
| `+0x104` | `0x0043CEA0` |
| `+0x10C` | `0x006F60D0` (`TechnoClass::DrawBehind`) |
| `+0x110` | `0x006F5190` (`TechnoClass::DrawExtras`) |
| `+0x114` | `0x0043D290` (`BuildingClass_DrawBody`) |

**Active in YR:** Yes for standard buildings accepted into the tactical display layer and viewport.

### 2.4 vtable+0x104 with flag 1 does not call DrawBody

`FUN_0043CEA0 @ 0x0043CEA0` dispatches based on its third argument:

- if the third argument is zero, it calls `vtable+0x114` at `0x0043CFFB..0x0043D005`, which resolves to `BuildingClass_DrawBody @ 0x0043D290`;
- if the third argument is nonzero and `building+0x6E7` is zero, it calls `vtable+0x4E4` at `0x0043CFCD..0x0043CFE9`, which resolves to `FUN_0043DA80`.

The standard non-foot Techno branch pushes `1` before calling `+0x104`, so this first-pass building call takes the `+0x4E4` path, not the `DrawBody @ +0x114` path.

**Active in YR:** Yes, conditioned on the standard building branch in `0x006D8DB0`.

### 2.5 Second pass calls DrawExtras again after all first-pass layers

After the first loop finishes all five display layers (`local_d4 < 5`), `0x006D95AF..0x006D97B5` iterates the display-layer arrays again. For each object with `object+0x99 != 0` and `AbstractFlags & 0x01`, it computes screen position and calls `vtable+0x110` at `0x006D9789`.

For `BuildingClass`, `+0x110` is `TechnoClass::DrawExtras @ 0x006F5190`.

**Active in YR:** Yes. The gate is `IsTechno` and the visibility mark written in the first pass.

## 3. Selected Building Bracket Gates

`TechnoClass::DrawBehind @ 0x006F60D0` enters the building bracket block only when `WhatAmI()==6` and selected byte `this+0x83` is nonzero.

`TechnoClass::DrawExtras @ 0x006F5190` has the same selected-building gate for front bracket work. `BuildingClass::WhatAmI @ 0x00459EC0` returns constant `6`.

**Active in YR:** Yes. No TS-only fog or scenario flag is required for the selected-building bracket blocks themselves.

## 4. Multi-Building Interleaving Model

For two selected buildings `A` then `B` in the same display layer, the verified dispatcher order is:

1. `A.DrawBehind`
2. `A.DrawExtras`
3. `A.vtable+0x104(flag=1)`
4. `B.DrawBehind`
5. `B.DrawExtras`
6. `B.vtable+0x104(flag=1)`
7. later second pass: `A.DrawExtras`
8. later second pass: `B.DrawExtras`

For two selected buildings in different display layers, all first-pass layers complete before the second `DrawExtras` pass begins.

**Inference from verified order:** because the second `DrawExtras` pass is after all first-pass object work, the final selected-building front bracket submission is phase-batched across visible techno objects. This matters for overlap: an earlier building's second-pass front brackets can be emitted after a later building's first-pass drawing, so implementing only per-object `DrawBehind -> body -> DrawExtras` will not match overlap ordering.

## 5. Open Questions - Final State

- [RESOLVED] OQ-1 - Is the selected-building bracket dispatcher per-object `DrawBehind -> body -> DrawExtras`? No. Standard buildings take the non-foot Techno branch, which calls `DrawBehind`, `DrawExtras`, then `+0x104(flag=1)`, followed by a later second-pass `DrawExtras`. Evidence: `0x006D90A9..0x006D9172`, `0x006D95AF..0x006D97B5`.
- [RESOLVED] OQ-2 - Is the second `DrawExtras` phase active for standard buildings? Yes. Buildings are Techno objects (`object+0x14 & 0x01`) and are marked visible at `object+0x99` in the first pass. Evidence: constructors `0x006F2B40`, `0x0043B680`; dispatcher `0x006D9111`, `0x006D95CD..0x006D9789`.
- [RESOLVED] OQ-3 - Does standard building `+0x104` in this branch call `DrawBody @ +0x114`? No; the branch pushes `1`, and `0x0043CEA0` dispatches nonzero third argument to `+0x4E4`, not `+0x114`. Evidence: `0x006D915B..0x006D916C`, `0x0043CFCD..0x0043D005`.
- [DEFERRED] OQ-4 - Which exact visual elements in `FUN_0043DA80 @ 0x0043DA80` are player-visible for each building art kind? Category: out-of-scope. This slot only needed bracket interleaving relative to the main dispatcher.
- [DEFERRED] OQ-5 - Does double `DrawExtras` create any non-idempotent visual effect for solid line brackets, pips, or health bars in all palette/depth cases? Category: needs-runtime-debugger. Static evidence proves the calls; runtime pixel capture would confirm final overdraw behavior.

## Sources

- Ghidra decompile/disassembly: `TacticalClass_Draw @ 0x006D3D10`
- Ghidra decompile/disassembly: `Tactical_ObjectRenderingLoop @ 0x006D8DB0`
- Ghidra decompile: `TechnoClass::DrawBehind @ 0x006F60D0`
- Ghidra decompile: `TechnoClass::DrawExtras @ 0x006F5190`
- Ghidra decompile/disassembly: `FUN_0043CEA0 @ 0x0043CEA0`
- Ghidra decompile: `BuildingClass_DrawBody @ 0x0043D290`
- Ghidra decompile: `FUN_0043DA80 @ 0x0043DA80`
- Ghidra decompile/disassembly: `BuildingClass::WhatAmI @ 0x00459EC0`
- Ghidra decompile/disassembly: `TechnoClass::Constructor @ 0x006F2B40`
- Ghidra decompile: `FootClass::Constructor @ 0x004D34D0`
- Ghidra decompile: `ObjectClass::Constructor @ 0x005F3920`
- Ghidra read_memory: `vtable_BuildingClass +0x104..+0x120` at `0x007E3FC0`
