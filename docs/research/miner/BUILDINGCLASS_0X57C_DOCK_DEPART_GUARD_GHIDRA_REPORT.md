# BUILDINGCLASS_0X57C_DOCK_DEPART_GUARD

**Date:** 2026-05-20  
**Mode:** exhaustive-slice  
**Target:** exact semantic of `BuildingClass+0x57C` as used by the refinery dock state-4 departure guard  
**Status:** COMPLETE  
**Primary conclusion:** `BuildingClass+0x57C` is `BuildingClass::Anims_0[8]`, the live `AnimClass*` slot for `ProductionAnim`. The state-4 guard is an animation-slot occupancy guard, not a locomotor readiness field, not a building readiness flag, and not an unload/depart latch.

## Scope Boundary

Started from OQ-1 in `RADIO_LINK_REFINERY_DOCK_STATE_MACHINE_GHIDRA_REPORT.md`. Checked the requested `0x0044B000`, the actual `BuildingClass::UpdateAnimation`, the slot helper trio, and the read/write sites needed to identify the field. Did not investigate unrelated slot keys or non-refinery production systems beyond confirming they are outside the standard chrono miner departure path.

## Verified Findings

### 1. Requested address check

`0x0044B000` is not `BuildingClass::UpdateAnimation`; Ghidra decompiles it as `BuildingClass::Mission_Attack`. The actual `BuildingClass::UpdateAnimation` body is at `0x004509D0`, and `BuildingClass::Update @ 0x0043FB20` calls it at `0x0043FE22`.

**Active in YR:** Yes for both functions as building logic, but `0x0044B000` is not active evidence for this target.  
**Evidence:** `decompile_function 0x0044B000`; `decompile_function 0x004509D0`; `get_function_xrefs 0x004509D0` -> `0x0043FE22`.

### 2. Field identity

`BuildingClass+0x57C` is the slot-8 member of the building animation pointer array. The array base is `building+0x55C`; slot `8` gives `0x55C + 8*4 = 0x57C`. The corresponding `BuildingTypeClass` art table slot starts at `Type+0xF4C + 8*0x44 = Type+0x116C`, which prior slot mapping identifies as `ProductionAnim`.

**Active in YR:** Yes as a live BuildingClass field and generic animation slot; Conditional in standard chrono miner dock because stock GAREFN/NAREFN do not define an active `ProductionAnim`.  
**Evidence:** `decompile_function 0x00451890` iterates/stores through `Anims_0`; `decompile_function 0x00451E40` clears `(&this->Anims_0)[slot]`; `REFINERY_DOCK_CELL_AND_ANIM_HELPERS_GHIDRA_REPORT.md` slot table; `ini/artmd.ini:1749`, `ini/artmd.ini:1763`, `ini/artmd.ini:1787`.

### 3. Write sites are animation-slot helper writes

No special dock latch write to `+0x57C` was found in the checked path. Creation goes through `BuildingClass::SetAnimSlotImage @ 0x00451750` -> `BuildingClass::CreateAnimForSlot @ 0x00451890`, which writes the new `AnimClass*` into `(&building->Anims_0)[slot]`. Clearing goes through `BuildingClass::ClearAnimSlot @ 0x00451E40`, which nulls the selected slot before destroying the prior anim.

**Active in YR:** Yes. These helpers are used by stock YR building animation logic. For standard chrono miner departure, the slot-8 create call is issued only if the refinery has a non-empty slot-8 art name; stock GAREFN/NAREFN do not.  
**Evidence:** `decompile_function 0x00451750`; `decompile_function 0x00451890`; `decompile_function 0x00451E40`; `get_function_xrefs` for all three helpers.

### 4. State-4 departure guard

In `UnitClass::Mission_Deploy_Building @ 0x0073D630`, harvester state 4 looks up the adjacent building, verifies `building->Type+0x16BB` (`Refinery=yes`), then compares `building+0x57C` against zero. If non-zero, it returns `1` immediately and does not clear the unit's dock-deploy byte or depart that tick.

This is exactly "wait while slot-8 ProductionAnim object exists." It is not checking locomotor state, building mission readiness, or an unload/depart latch.

**Active in YR:** Conditional. The code path is active for harvester unload against `Refinery=yes`, but the wait branch only fires if slot 8 is non-null. Standard GAREFN/NAREFN do not populate it from stock artmd.ini.  
**Evidence:** `decompile_function 0x0073D630`; assembly context `0x0073E1CF` to `0x0073E1EA` shows `Type+0x16BB` gate and `CMP [EAX+0x57C], 0`; `ini/rulesmd.ini:11727`, `ini/rulesmd.ini:12520`.

### 5. Slot-8 creation in the unload FSM

At the end of the deposit loop, `UnitClass::Mission_Deploy_Building` calls `BuildingClass::SetAnimSlotImage` with slot `8`, then sets the unit substate to `4` and clears slot `10` (`SpecialAnim`) if occupied. The same slot-8 creation sequence exists on the forced completion branch.

**Active in YR:** Conditional. The call site is active for stock chrono miner unload, but `SetAnimSlotImage` creates nothing when the selected art name is empty/commented.  
**Evidence:** assembly contexts at `0x0073E517` and `0x0073E58F` push slot `8` before `CALL 0x00451750`; contexts at `0x0073E534` and `0x0073E5AC` clear slot `0xA`.

### 6. UpdateAnimation handling of slot 8

`BuildingClass::UpdateAnimation @ 0x004509D0` reads `building+0x57C` and sibling `building+0x588`; if the type flag at `Type+0x16A9` is set, current mission is not `0x14`, either slot is non-null, and `Type+0xCCE` is false, it may create slot `12` and then clears slots `8` and `11`. This confirms `+0x57C` participates in the building animation-slot lifecycle, not locomotor state.

**Active in YR:** Conditional. The code is live in `BuildingClass::Update`, but stock GAREFN/NAREFN chrono-miner unload normally has `+0x57C == 0` because the slot-8 art key is not active.  
**Evidence:** `decompile_function 0x004509D0`; assembly context `0x00450AAD` reads `[ESI+0x57C]`; context `0x00450B22` clears slot `8`.

### 7. Standard YR chrono miner/refinery result

Standard `CMIN` docks only with `NAREFN,GAREFN` in `rulesmd.ini`. Both stock refinery building types have `DockUnload=yes` and `Refinery=yes`, so the state-4 guard code is reached. However, `artmd.ini` has no active `ProductionAnim` for `GAREFN`, and the `NAREFN` `ProductionAnim=NAREFN_AR` line is commented out. Therefore, in standard YR chrono miner unload, slot `8` is normally never populated at dump completion and the departure guard does not delay the miner.

**Active in YR:** Yes for the code path; No for an actual stock delay from `+0x57C` in standard chrono miner -> GAREFN/NAREFN departure. Conditional for mods or nonstandard rules/art that define refinery `ProductionAnim`.  
**Evidence:** `ini/rulesmd.ini:7351`, `ini/rulesmd.ini:7361`, `ini/rulesmd.ini:11726`, `ini/rulesmd.ini:11727`, `ini/rulesmd.ini:12519`, `ini/rulesmd.ini:12520`, `ini/artmd.ini:1749`, `ini/artmd.ini:1763`, `ini/artmd.ini:1787`.

## Answer to OQ-1

`BuildingClass+0x57C` is an animation-in-progress proxy only in the narrow sense that it is non-null while slot-8 `ProductionAnim` has an `AnimClass` object. Its exact semantic is **animation slot 8 pointer (`ProductionAnim`)**. The refinery departure guard uses pointer occupancy as a wait condition. It is not a locomotor readiness flag, not building readiness, and not an unload/depart latch.

## Open Questions Kept Out of Scope

- Exact INI-key identity for `BuildingTypeClass+0x16A9` and `+0xCCE` was not traced; it is not needed to identify `+0x57C`.
- Whether a modded refinery `ProductionAnim` is visible for one tick or for a longer animation window depends on the full slot-8 lifecycle and AnimClass destruction/update ordering; stock chrono miner departure does not exercise it.

