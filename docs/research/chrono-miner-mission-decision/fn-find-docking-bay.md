# FootClass::Find_Docking_Bay — Decode

**Proposed Ghidra label:** `FootClass__Find_Docking_Bay` (already labeled)

---

## Summary

`FootClass__Find_Docking_Bay` at `0x004DF040` scans the caller's dock-type list (e.g. `UnitTypeClass.Dock[]`) and returns the **closest eligible building instance** of any listed dock type that the miner can enter. Per-candidate eligibility is delegated to the virtual method at vtable slot `0x52c` (`FUN_004dee80`), which iterates the owner's live building instances to find one with a matching TypeClass, passes zone reachability and radio clearance checks, and returns the nearest acceptable candidate with its squared distance. The outer loop selects the minimum-distance result, with a Veteran-dock override: a Veteran building wins regardless of distance.

Called twice from `UnitClass__Mission_Harvest` state 2 (RETURN): first with `arg3=0` (normal), then with `arg3=1` (editor/fallback — `g_MapEditorMode` is incremented by the caller before this second call and decremented after).

---

## Active in YR

**Yes.** Called from `UnitClass__Mission_Harvest` case 2 (state 2 = RETURN). This is the live harvest-return path for all harvesting units including the chrono miner. Confirmed via `get_function_callers 0x004DF040` and `decompile_function 0x0073E5E0`.

Also in vtable slot `0x528` of `vtable__UnitClass` (`0x007F5C70 + 0x528 = 0x007F6198`), confirmed via `read_memory 0x007F6198` returning `0x004DF040`. The same vtable slot entry appears in two additional vtables at `0x007E91BC` and `0x007EB580` (verified `read_memory` on both).

`AircraftClass__FindBuildingToDock` also calls this function, but with itself (an aircraft) as `this`. That path is out of scope for this chrono-miner system.

---

## Decompilation excerpt

Verified via `decompile_function 0x004DF040`:

```c
int __thiscall
FootClass__Find_Docking_Bay(int *param_1, int param_2, undefined4 param_3, undefined4 param_4)
{
  // param_1  = this (FootClass / miner instance)
  // param_2  = TypeClass* with dock list: +0x10 = count, +4 = array-of-TypeClass*-ptr
  // param_3  = editor flag: 0 = normal check, 1 = editor/fallback (bypass zone check)
  // param_4  = passed through to eligibility checker

  iVar1 = param_2;   // save TypeClass ptr
  iVar3 = 0;         // best building found (0 = none)
  iVar4 = 0;         // loop index
  local_4 = -1;      // best distance so far (-1 = sentinel "none yet")

  if (0 < *(int *)(param_2 + 0x10)) {   // dock list count > 0
    do {
      param_2 = -1;   // reset per-candidate distance output
      // Virtual call at vtable slot 0x52c on THIS (miner):
      // — passes dock-type entry from list (a BuildingTypeClass*)
      // — passes editor flag, param_4, and &distance_out
      // — returns the eligible building instance ptr, or 0 if none
      iVar2 = (**(code **)(*param_1 + 0x52c))(
                  *(undefined4 *)(*(int *)(iVar1 + 4) + iVar4 * 4),  // dock type[iVar4]
                  param_3, param_4, &param_2);                        // editor, p4, &dist

      if ((iVar2 != 0) &&
         ((((iVar3 == 0 || (param_2 < local_4)) || (local_4 == -1)) ||
          (*(char *)(iVar2 + 0x3d3) != '\0')))) {
        local_4 = param_2;   // update best distance
        iVar3 = iVar2;       // update best building
      }
      iVar4 = iVar4 + 1;
    } while (iVar4 < *(int *)(iVar1 + 0x10));
  }
  return iVar3;   // best eligible building, or 0 if none found
}
```

---

## Behavioral analysis

### Dock list source
`param_2` is the miner's `UnitTypeClass*` (or similar TypeClass), called from `Mission_Harvest` as `param_1[0x1b1] + 1000` — i.e. byte offset `+0x3E8` from the miner's TypeClass pointer:
- `TypeClass+0x10` (byte) = dock list count (direct `int` offset)
- `TypeClass+0x04` (byte) = pointer to array of `BuildingTypeClass*` entries

These are the INI `Dock=NAREFN,GAREFN` entries resolved to TypeClass pointers at load time.

### Per-candidate eligibility: vtable slot `0x52c` = `FUN_004dee80`
The virtual method at slot `0x52c` (`vtable__UnitClass + 0x52c = 0x007F619C`, verified via `read_memory 0x007F619C` returning `0x004DEE80`) is not a simple building-eligibility predicate — it is itself an **owner-building scanner**. Given a BuildingTypeClass `param_2`, it:
1. Counts owned building instances of that type via `HouseClass__CountOwnedInstances(TypeClass.HouseOwner)` — if 0, returns null immediately.
2. Iterates `HouseClass.BuildingInstances[]` (at `HouseClass+0x78` count, `HouseClass+0x6C` array).
3. For each building: checks alive (`Building+0x81 == 0`), correct TypeClass (`Building+0x520 == param_2`).
4. Zone reachability: unless `param_4 == 1` (editor flag), calls `FUN_0065adf0` (zone-available check on miner) and `MapClass__Can_Reach_Zone` to confirm the miner's movement zone can reach the building's zone. If `GetMission() == 2`, skips the zone check.
5. Radio clearance: `Receive_Radio(0x0F, building)` — checks if the building will accept a docking request. Returns `1` on success.
6. Distance scoring: calls `FUN_005f6500(miner, building)` which computes squared Euclidean distance (leptons) between miner and building centers via their `GetCoords` vtable calls. Uses this to update `*param_5` (the outer `param_2` distance output).
7. Returns the closest building passing all checks, or null.

Confirmed via `create_function 0x004DEE80` + `decompile_function 0x004DEE80`.

### Selection logic: minimum distance + Veteran override
Back in `Find_Docking_Bay`, the outer loop selects the best result across all dock types:
- Accept if: no best yet (`iVar3 == 0`), first valid distance (`local_4 == -1`), new candidate is closer (`param_2 < local_4`), OR the candidate building has Veteran status (`Building+0x3d3 != 0`).
- The Veteran condition unconditionally overrides distance: a Veteran refinery always wins.

### Vtable slot disambiguation
- Slot `0x528` (`0x007F6198`): `FootClass__Find_Docking_Bay` itself (the outer scanner)
- Slot `0x52c` (`0x007F619C`): `FUN_004dee80` (the per-TypeClass building-instance scanner / eligibility checker, called from within `Find_Docking_Bay`)
- Slot `0x530` (`0x007F61A0`): `FUN_004dee50` (thin wrapper that calls slot `0x52c` and discards the distance output)

### Editor/fallback mode (`arg3=1`)
When `Mission_Harvest` case 2 calls this with `arg3=1`, the caller first increments `g_MapEditorMode` and decrements it after. Inside `FUN_004dee80`, the editor flag (`param_4 == 1`) bypasses the zone-reachability check (`FUN_0065adf0` / `MapClass__Can_Reach_Zone`). This is the "any dock in editor mode" fallback that allows the miner to find a refinery even when normal zone routing would reject it.

---

## Struct field accesses

### FootClass (param_1, this = miner)
All offsets as direct byte offsets (param_1 is `int *`, pointer arithmetic × 4 applies where indexed):

| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `*param_1` | `+0x000` | vtable ptr | Used to dispatch slot `0x52c` |
| `param_1[0x1b1]` | `+0x6C4` | UnitTypeClass* | Miner's type — provides the dock list |
| `param_1[0x87]` | `+0x21C` | HouseClass* | Owner house (used inside eligibility checker) |

### TypeClass (iVar1 = param_2 input = miner's UnitTypeClass)
| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `*(int *)(iVar1 + 4)` | `+0x004` | dock array ptr | Pointer to `BuildingTypeClass*[]` |
| `*(int *)(iVar1 + 0x10)` | `+0x010` | dock count | Number of dock type entries |

Note: offset from `param_1[0x1b1] + 1000` = `UnitTypeClass + 0x3E8` — the dock list is embedded at byte offset `+0x3E8` within the UnitTypeClass, with count at `+0x3F8` (= `+0x3E8 + 0x10`) and array ptr at `+0x3EC` (= `+0x3E8 + 0x04`). Verified by cross-referencing `Mission_Harvest` decompile: `*(int *)(param_1[0x1b1] + 0x3f8)` is the dock count used in a parallel loop there.

### BuildingClass (iVar2, piVar1 inside eligibility checker)
| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `*(char *)((int)piVar1 + 0x81)` | `+0x081` | dead/destroyed flag | `0` = alive |
| `piVar1[0x148]` | `+0x520` | BuildingTypeClass* | Building's type back-pointer |
| `*(char *)(iVar2 + 0x3d3)` | `+0x3D3` | Veteran status byte | Non-zero = Veteran; triggers dock preference override |

from `AbstractClass` frame (direct byte offsets, param is `int *`): `piVar1[0x148]` = `+0x148*4 = +0x520`.

### HouseClass (inside eligibility checker)
| Expression | Byte offset | Field | Notes |
|---|---|---|---|
| `*(int *)(param_1[0x87] + 0x78)` | HouseClass`+0x78` | building instance count | |
| `*(int *)(param_1[0x87] + 0x6c)` | HouseClass`+0x6C` | building instance array ptr | `BuildingClass*[]` |

---

## Globals / enums / INI

| Symbol | Address/offset | Role |
|---|---|---|
| `g_MapEditorMode` | (global, incremented by caller) | Non-zero = editor mode; bypasses zone check in eligibility checker |

**INI keys:** `Dock=` on the miner's unit type entry (e.g. `[CMIN] Dock=NAREFN,GAREFN`). These resolve to the BuildingTypeClass ptr array at `UnitTypeClass+0x3EC`. Verified via `ini/rulesmd.ini` `[CMIN]` section.

**Distance function `FUN_005f6500` (`0x005F6500`):** computes squared Euclidean distance between two objects' `GetCoords` results, returned as integer (lepton² units). Smaller = closer. Confirmed via `create_function` + `decompile_function 0x005F6500`.

---

## Out-of-scope refs

- `AircraftClass__FindBuildingToDock` (`0x0041BBD0`): calls `Find_Docking_Bay` on behalf of aircraft. Out of scope for chrono-miner system; the call passes `param_2` (not `this`) as the first arg due to different calling convention in that wrapper.
- `FUN_0065adf0` (`0x0065ADF0`): zone-available check used inside eligibility checker. Not decoded in this document; it iterates a list at `param_1+0xE4/0xE8` for zone matching.
- `MapClass__Can_Reach_Zone`: zone reachability check. Out of scope.
- `FootClass__Find_Nearest_Dock` (`0x004DFCB0`): separate dock-finding function in FootClass; not called from this path but shares the dock-list iteration pattern.

---

## Unverified

YELLOW — not verified in this session:

- The exact byte offset for `g_MapEditorMode` (used by the caller in `Mission_Harvest` case 2 around the second call). The increment/decrement pattern is clear from the `Mission_Harvest` decompile; the global's address was not read.
- Whether `FUN_0065adf0` is the same as any named zone-check function in the broader docs archive. Its role (zone-available predicate on `param_1+0xE4/E8`) is inferred from the decompile but not cross-referenced.
- `Building+0x3D3` Veteran byte: inferred from context (Veteran tiebreaker logic in selection). Not confirmed against a known TechnoClass/BuildingClass struct layout doc.
- The exact vtable slots for the two sibling functions at `0x004DEE50` (slot `0x530`) and `FUN_004dee80` (slot `0x52c`) in the InfantryClass and AircraftClass vtables (only verified for UnitClass vtable).
