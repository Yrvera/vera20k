# Building Bracket GetHeight Dim Color Reachability Follow-up - Ghidra Report

Target: GetHeight() < -4 bracket dim-color reachability for standard selected Yuri's Revenge buildings.

Report path: `C:/Users/enok/Documents/ra2-rust-game-docs/BUILDING_BRACKET_GETHEIGHT_DIM_COLOR_REACHABILITY_FOLLOWUP_GHIDRA_REPORT.md`

Status: COMPLETE

## Answer

Active in YR: No for standard selected buildings reaching the dim palette branch.

The selected-building bracket path is active in standard YR, and the dim-color branch exists in that path. However, fresh Ghidra reads show the dim branch is reached only when the building's vtable `+0x1C8` height result is below `-4`. For `BuildingClass`, that vtable entry resolves to the generic `ObjectClass::GetHeight`, whose result is `Location.Z - CellClass::GetGroundHeight(Location) - (OnBridge ? DAT_00AC13BC : 0)`.

For a normal placed/revealed selected building, the checked binary paths preserve raw `Location.Z` as the building's world Z and do not introduce a negative height source in the bracket path. Therefore the standard selected building case stays on palette index `0x0F`, not `0x0C`.

Conditional reachability remains only for forced/nonstandard object states, such as a selected drawn building with `OnBridge` set while its raw Z was not bridge-raised, or another external corruption/mod/script path that makes raw Z more than four leptons below ground. That is not verified as reachable for standard selected YR buildings.

## Verified Binary Evidence

1. Active in YR: Yes for the bracket path. `Tactical_ObjectRenderingLoop @ 0x006D8DB0` calls object vtable `+0x110` for drawn display-layer objects in the later extras pass; `TechnoClass::DrawExtras @ 0x006F5190` then checks selected byte `+0x83` and `WhatAmI()==6` before the selected building bracket block. `BuildingClass::WhatAmI @ 0x00459EC0` returns `6`.

2. Active in YR: Conditional for dim color, Yes for the branch existing. `TechnoClass::DrawExtras @ 0x006F5190` initializes the bracket color index to `0x0F`, calls vtable `+0x1C8`, compares the result with `-4`, and writes `0x0C` only when the result is below `-4` (`0x006F53AD..0x006F53C0`).

3. Active in YR: Conditional for dim color, Yes for the matching back-edge branch existing. `TechnoClass::DrawBehind @ 0x006F60D0` performs the same selected-building `WhatAmI()==6` and selected-byte gate, initializes color index `0x0F`, calls vtable `+0x1C8`, and switches to `0x0C` only on `< -4` (`0x006F6109..0x006F6119`).

4. Active in YR: Yes for standard buildings binding to the generic height getter. Ghidra xrefs show `ObjectClass::GetHeight @ 0x005F5F40` bound through data entries including Building vtable entry `0x007E4084`; the function computes height from `Location.Z`, `CellClass::GetGroundHeight`, and `OnBridge`/`DAT_00AC13BC`.

5. Active in YR: No for normal standard selected buildings reaching dim color. `ObjectClass::Set_Raw_Coords @ 0x005F6940` writes raw `Location.X/Y/Z` directly, `BuildingClass::GetCoords @ 0x00447AC0` preserves raw `Location.Z` when returning the building coordinate used by drawing, and `ObjectClass::Reveal @ 0x005F4EC0` calls vtable `+0x1B4` to apply reveal coordinates before display submission. These checked standard object-placement/display paths do not create `GetHeight() < -4`.

## Inference

The "no standard selected building" conclusion is an inference from the verified branch condition and the verified normal placement/display coordinate path. The binary evidence proves the dim branch requires an abnormal negative above-ground height; the checked standard building paths do not produce that state.

This report does not claim that the branch is dead code globally. It is live code with conditional reachability for forced/nonstandard states.

## Open Questions

- Exact runtime value and initializer context for `DAT_00AC13BC` remain a bridge-system detail, not needed for the standard-building verdict because any normal nonnegative height fails `< -4`.
- Whether a corrupt map, save, debugger write, or custom extension can force a selected building into the mismatch state remains outside standard YR reachability.

## Ghidra Reads

- Decompiled: `0x006D8DB0`, `0x006F5190`, `0x006F60D0`, `0x005F5F40`, `0x005F5FA0`, `0x005F5940`, `0x005F6940`, `0x00447AC0`, `0x005F4EC0`, `0x005F4D30`, `0x005F4160`, `0x00459EC0`.
- Assembly context: `0x006F53BB`, `0x006F6114`, `0x005F3880`.
- Xrefs: `0x005F5F40`, `0x005F5FA0`, `0x005F6940`, `0x005F5940`, `0x00AC13BC`.
