# BulletClass Target Invalidation for AAHeatSeeker2 - Ghidra Research Report

**Date:** 2026-05-20  
**Primary addresses:** `0x004684E0` (`BulletClass` pointer-expired handler), `0x00468430` (`BulletClass::UpdateTarget`), `0x004666E0` (`BulletClass::AI`)  
**Investigation mode:** exhaustive-slice for `BulletClass+0x10C` invalidation affecting ROT>0 `AAHeatSeeker2`/`DRAGON` homing bullets  
**Claimed scope:** target death/removal notification, chrono-warp update, null/sentinel handling in `BulletClass::AI`, and stock deployed Guardian GI `MissileLauncher` applicability  
**Non-scope:** full homing math, arming/proximity internals, DRAGON rendering, warhead damage math, and non-Guardian-GI weapons that also use `AAHeatSeeker2`  
**Confidence:** HIGH for the field writers and AI null/sentinel behavior; MEDIUM for every possible non-destruction limbo caller because non-destroying limbo paths were sampled, not exhaustively enumerated  
**Active in YR:** Yes. The path is inherited by `BulletClass`, which is created by stock weapon fire and registered through `ObjectClass::Constructor`; `MissileLauncher` uses `Projectile=AAHeatSeeker2`, `Image=DRAGON`, `ROT=60` in `rulesmd.ini`.

## 1. Overview

`BulletClass+0x10C` is the live target pointer used by the ROT>0 homing branch. It is not polled for visibility and it is not repaired by `BulletClass::AI` itself when the pointer remains non-null. The live invalidation writer for normal target destruction/removal is `BulletClass` vtable slot `+0x28` at `0x004684E0`; the separate `BulletClass::UpdateTarget @ 0x00468430` is chrono-warp-specific and has only one caller.

When the expired target can be converted to a valid ground cell, `+0x10C` is changed from the object pointer to a `CellClass*`. When it cannot, `+0x10C` is cleared to null. Once AI sees null, it substitutes the null coordinate sentinel for homing; if a target coordinate is sentinel and the bullet is at or above `Rules.FlightLevel`, AI forces detonation.

## 2. Key Offsets and Globals

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `BulletClass+0x10C` | Current homing target pointer; may be object, `CellClass*`, or null | `BulletClass::Init @ 0x004664C0`; AI reads `param_1[0x43]`; pointer-expired writes `[ESI+0x10C]` | Yes - standard bullet runtime field |
| `BulletClass+0xB0` | Owner/firer pointer; pointer-expired clears it if the owner expires | `0x004684F7..0x00468503` assembly context | Yes - same handler |
| `BulletClass+0xAC` | `BulletTypeClass*`; cleared if the type pointer expires | `0x004685A2..0x004685AA` assembly context | Yes, but type expiry is not normal match play |
| `BulletClass+0x130` | `WeaponTypeClass*`; cleared if weapon type expires | `0x004685B0..0x004685B8` assembly context | Yes, but type expiry is not normal match play |
| `BulletClass+0x154` | Bounce/impact anim pointer; cleared if that anim expires | `0x004685BE..0x004685C6` assembly context | Conditional - only when bullet is waiting on an anim |
| `DAT_0089DDF0/2` | off-map cell-coordinate sentinel used before `MapClass::Get_CellClass` retargeting | comparisons in `0x00468430` and `0x004684E0` bodies | Yes |
| `DAT_0089DE30/34/38` | null target-coordinate sentinel used by `BulletClass::AI` after null target | `BulletClass::AI @ 0x004666E0` decompile assigns/compares this coord triple in the ROT>0 branch | Yes |
| `RulesClass+0x5A0` | `FlightLevel`; lost/sentinel target detonation threshold | `BulletClass::AI @ 0x004666E0` decompile compares `GetHeight()` to this field after the sentinel-coordinate check | Yes |

## 3. Normal Death / Removal Path

Material finding: target destruction/removal invalidates `+0x10C` through the object removal notification system, not through `BulletClass::AI`.

1. `ObjectClass::UnInit @ 0x005F6620` calls `Detach_From_All_Lists @ 0x007258D0` before conceal/limbo and before clearing `ObjectClass+0x90` alive state.
   - Active in YR: Yes. This is the generic object destruction path used by units and bullets.
2. `Detach_From_All_Lists @ 0x007258D0` iterates registered objects and calls each listener's virtual slot `+0x28` with the expiring object and the caller's removal flag.
   - Active in YR: Yes. Object classes with `AbstractFlags` bit 1 set take this path; Unit/Infantry/Aircraft RTTI values also reach the object cleanup branch.
3. `ObjectClass::Constructor @ 0x005F3900`, inherited by `BulletClass::Constructor @ 0x00466380`, registers every object in `DAT_00B0F724` and sets `AbstractFlags` bit 1 at `+0x14`.
   - Active in YR: Yes. `BulletClass` calls `ObjectClass::Constructor`, then registers in `g_BulletClass_Array`.
4. `BulletClass` vtable slot `+0x28` is bound at data xref `0x007E470C` to the function body starting `0x004684E0`.
   - Active in YR: Yes. This is the concrete handler called for bullets during removal notification.

The `0x004684E0` handler first delegates to inherited `ObjectClass` pointer-expiry cleanup (`0x005F5230`), then handles Bullet-specific fields:

| Condition | Write | Evidence | Active in YR |
|---|---|---|---|
| expired pointer equals `BulletClass+0xB0` | owner becomes null | `0x004684F7..0x00468503` | Yes |
| expired pointer equals `BulletClass+0x10C`, map-editor mode is off, target is not high-flying, and target cell is not the off-map sentinel | target becomes `MapClass::Get_CellClass(last_target_cell)` | `0x00468509..0x00468594`; `MapClass::Get_CellClass @ 0x005657A0` | Yes |
| expired pointer equals `BulletClass+0x10C`, but map-editor mode is on, target is high-flying, or target cell is sentinel | target becomes null | `0x00468551..0x0046859C` | Yes |
| expired pointer equals `+0xAC`, `+0x130`, or `+0x154` | matching pointer field becomes null | `0x004685A2..0x004685C6` | Conditional; type/weapon expiry is not normal skirmish flow |

Important correction to older docs: the virtual `+0x54` predicate used by both `0x004684E0` and `0x00468430` is not proven to be "IsOnMap" in this slice. For `ObjectClass`, the slot resolves through vtable data including `0x007E4738` to `ObjectClass::IsHighFlying @ 0x005F6B90`, which checks an object flag plus `GetHeight() >= 2 * DAT_00AC13C8`. Therefore the verified branch is "not high-flying" versus "high-flying", not "on-map" versus "off-map".

## 4. Chrono-Warp UpdateTarget Path

`BulletClass::UpdateTarget @ 0x00468430` is real, but its xrefs show only one caller: `TeleportLocomotionClass::StateMachineTick @ 0x007193EE`.

Its logic mirrors the target branch of `0x004684E0`: read current target coordinates via vtable `+0x48`; if map-editor mode is off, target is not high-flying, and the cell coordinate is not `DAT_0089DDF0/2`, replace `+0x10C` with `MapClass::Get_CellClass`; otherwise clear `+0x10C`.

Active in YR: Conditional. It is live for chrono/teleport locomotion target transitions. It is not the standard "target died" path and is not called from `BulletClass::AI`.

## 5. `BulletClass::AI` After Null or Sentinel Target

For `AAHeatSeeker2`, `rulesmd.ini [AAHeatSeeker2] ROT=60`, so `BulletClass::AI` takes the ROT>0 homing branch.

Verified AI behavior:

| Case | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `+0x10C` is non-null | AI calls target vtable `+0x58` for current target coordinate; if target has `AbstractFlags` bit 1, it then calls vtable `+0xA4` as an alternate/center coordinate | `BulletClass::AI @ 0x004666E0` decompile, ROT>0 target-coordinate branch | Yes |
| `+0x10C` is null | AI uses the null coordinate sentinel at `DAT_0089DE30` instead of dereferencing a target | `BulletClass::AI` branch assigning `pdVar7 = &DAT_0089DE30` | Yes |
| target coordinate is sentinel | AI still calls `BulletClass::HomingTrack @ 0x005B20F0`; homing handles sentinel by turning from current velocity rather than steering to a valid target | `HomingTrack @ 0x005B20F0` sentinel branch compares against `DAT_00ABEF10/14/18` | Yes |
| target coordinate is sentinel and bullet height is at least `Rules.FlightLevel` | AI sets detonation flags | `BulletClass::AI` compares target coord to `DAT_0089DE30/34/38` and `GetHeight()` to `RulesClass+0x5A0` | Yes |
| target exists and missile stops closing | approach accumulator can force detonation after the sample window | `BulletClass::AI` fields `+0x118/+0x120`, constants `60`, `0.983`, `60.0` | Yes |

No visibility/cloak/invisibility test was found in the traced `+0x10C` writer path or the ROT>0 AI coordinate-read path. Active in YR: Yes, by absence in the decompiled live branches; invisibility alone does not clear `+0x10C` in this slice.

## 6. Deployed Guardian GI Context

Stock deployed Guardian GI uses `rulesmd.ini [GGI] Secondary=MissileLauncher`; `[MissileLauncher] Projectile=AAHeatSeeker2`; `[AAHeatSeeker2] Image=DRAGON`, `ROT=60`, `AA=yes`, `AG=yes`, `Arm=2`, `Proximity=no`, `Ranged=yes`.

Active in YR: Yes. The parent report verified `InfantryClass::Fire_At_Target -> TechnoClass::Fire_At -> BulletClass::Allocate/Init/Fire -> BulletClass::AI`; this report only adds the target-invalidation slice.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BulletClass+0x10C` initialization | verified | `BulletClass::Init @ 0x004664C0` writes target param to `+0x10C` | none |
| normal destroyed/removed target invalidation | verified | `ObjectClass::UnInit @ 0x005F6620`; `Detach_From_All_Lists @ 0x007258D0`; `BulletClass` vtable `0x007E470C -> 0x004684E0` | none |
| pointer-expired retarget-to-cell branch | verified | assembly context `0x00468509..0x00468594`; `MapClass::Get_CellClass @ 0x005657A0` | none |
| pointer-expired clear-to-null branch | verified | assembly context `0x00468551..0x0046859C` | none |
| `BulletClass::UpdateTarget` caller set | verified | `get_function_xrefs 0x00468430` -> sole caller `0x007193EE` | none |
| `BulletClass::AI` null target behavior | verified | `BulletClass::AI @ 0x004666E0` null branch to `DAT_0089DE30` | none |
| sentinel target detonation at `FlightLevel` | verified | `BulletClass::AI @ 0x004666E0`; `RulesClass+0x5A0` compare | none |
| non-destroying limbo without `UnInit` | touched-not-exhausted | `ObjectClass::Conceal @ 0x005F4D30` does not call `Detach_From_All_Lists` | enumerate every transport/garrison/limbo caller if needed |
| invisibility/cloak invalidation | touched-not-exhausted | no visibility test in traced writer/AI branches | separate cloak/visibility systems could affect target acquisition, not in-flight bullet pointer |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What clears or updates `BulletClass+0x10C` when the target is destroyed? Answer: `BulletClass` pointer-expired handler at `0x004684E0`, reached through `ObjectClass::UnInit -> Detach_From_All_Lists -> vtable+0x28`. Evidence: `0x005F6620`, `0x007258D0`, data xref `0x007E470C`.

[RESOLVED] OQ-2 - Does `BulletClass::UpdateTarget @ 0x00468430` handle normal target death? Answer: no; its sole xref is `TeleportLocomotionClass::StateMachineTick @ 0x007193EE`. Evidence: Ghidra `get_function_xrefs`.

[RESOLVED] OQ-3 - Does target death always null the pointer? Answer: no. If the expired target coordinate maps to a non-sentinel cell and the target is not high-flying, `+0x10C` becomes that `CellClass*`; otherwise it becomes null. Evidence: `0x00468509..0x0046859C`.

[RESOLVED] OQ-4 - What does AI do when `+0x10C` is null? Answer: substitutes the null coordinate sentinel, runs homing movement, and detonates once sentinel target plus `GetHeight() >= Rules.FlightLevel` is true. Evidence: `BulletClass::AI @ 0x004666E0`.

[RESOLVED] OQ-5 - Is invisibility itself a target invalidator for in-flight AAHeatSeeker2? Answer: no verified in-flight invalidation check was found in `0x004684E0`, `0x00468430`, or the ROT>0 coordinate-read branch of `BulletClass::AI`. Evidence: decompiled functions listed above.

[DEFERRED] OQ-6 - Do all non-destroying limbo transitions preserve in-flight target pointers? Category: bounded-cost-too-high. `ObjectClass::Conceal @ 0x005F4D30` alone does not dispatch pointer-expired notifications, but a full census of every limbo caller is outside this slot.

## Sources

- Live Ghidra decompilation/assembly context:
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::UpdateTarget @ 0x00468430`
  - `BulletClass` pointer-expired handler body at `0x004684E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `ObjectClass::Constructor @ 0x005F3900`
  - `ObjectClass::Conceal @ 0x005F4D30`
  - `ObjectClass::UnInit @ 0x005F6620`
  - `ObjectClass::IsHighFlying @ 0x005F6B90`
  - `Detach_From_All_Lists @ 0x007258D0`
  - `MapClass::Get_CellClass @ 0x005657A0`
  - `get_xrefs_to 0x004684E0` -> data xref `0x007E470C`
  - `get_function_xrefs 0x00468430` -> `0x007193EE`
- Parent report: `C:/Users/enok/Documents/ra2-rust-game-docs/GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
- INI evidence:
  - `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini [GGI]`, `[MissileLauncher]`, `[AAHeatSeeker2]`
  - `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini [DRAGON]`
