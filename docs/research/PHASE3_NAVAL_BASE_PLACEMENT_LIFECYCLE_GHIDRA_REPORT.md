# Phase 3 Naval Base Placement Lifecycle Ghidra Report

**Binary:** active retail Yuri's Revenge `gamemd.exe` 1.001
**Date verified:** 2026-08-26
**Mode:** read-only native verification
**Verdict:** **ACTIVE, VERIFIED.** This report closes the remaining default, coordinate, distance,
and owned-construction-yard ordering questions left after section 8 of
`PHASE3_HOUSECLASS_ORDINARY_BASE_PLACEMENT_005060B0_GHIDRA_REPORT.md`. It does not research or
close ordinary non-naval base planning.

## 1. Exact adjacency rule and default

`Rules+0xE0C` is the signed `[General] AINavalYardAdjacency=` field. The live parser binding is
at `RulesClass::ReadGeneral 0x006701D9..0x006701FE`. `RulesClass::Constructor` writes literal
`0x14` at `0x00666922`, so the constructor default is exactly **20 cells**. The value remains
signed and the naval consumer converts it to leptons by a signed left shift of eight.

This supersedes the stale `MaxBaseDistance` name in older documents. Both stock `rules.ini` and
`rulesmd.ini` use the constructor/default behavior for this field.

## 2. Exact result-to-yard distance comparison

The naval tail is at `HouseClass__AI_FindBasePlacement 0x005061BE..0x00506213`:

1. Read the first pointer from the House's owned `BuildConst` DynamicVector at
   `House+0x50/+0x54/+0x60`.
2. If it is null, return the valid FNPC result without applying an adjacency cap.
3. Call the first Building object's virtual coordinate getter at vtable `+0x48`, which resolves
   to `BuildingClass::GetCoords @ 0x00447AC0`.
4. Resolve the returned FNPC cell through `MapClass::Get_CellClass @ 0x005657A0`, then call that
   CellClass's virtual coordinate getter at vtable `+0x48`.
5. `FUN_00437160` subtracts the yard coordinate from the cell coordinate component-by-component
   using wrapped 32-bit integer arithmetic.
6. `CoordStruct__Distance3D @ 0x0041C380` computes the distance. Compare it to signed
   `(Rules+0xE0C) << 8`; the `JLE` accepts equality, so only a distance **strictly greater** than
   the threshold is rejected to packed `(0,0)`.

`CellClass__Get_Center_Coords @ 0x00480A30` returns signed-short cell X/Y multiplied by 256 plus
128. Its Z is the CellClass terrain surface computed at subcell `(128,128)`. Active-runtime
capture in `PHASE3_CELL_GROUND_HEIGHT_104_DOMAIN_CONSUMER_CENSUS_GHIDRA_REPORT.md` corrects the
earlier domain split: Cell owns a separately initialized scalar, but its active value is the same
104 leptons as the other captured level-height globals. `BuildingClass::GetCoords` starts from
the Building's stored north-west anchor
coordinate, preserves its object Z, and adds `(foundation_width - 1) * 128` to X and
`(foundation_height - 1) * 128` to Y. The same verified formula already appears in Rust's naval
factory exit, radar, animation-owner, lifecycle, and combat coordinate consumers.
`CoordStruct__Distance3D` uses the shared native approximate-square-root/f32/`ftol` pipeline
already represented by `util::native_x87::distance_3d_leptons`.

Therefore a cell-distance shortcut, 2D range, stored north-west-yard anchor, non-slope-aware
CellClass Z, or `>=` comparison is wrong.

## 3. What the first construction-yard pointer means

The vector at `House+0x50` is not the generic owned-object order. Its membership predicate is
type identity in source-ordered `[AI] BuildConst=` (`Rules+0x8B0/+0x8BC`), and its order is live
insertion/acquisition order. Corrected binding evidence: `RulesClass__ReadAI @ 0x00672AE0`, block
`0x00672B14..0x00672C01` (key push `0x00672B23`, BuildingType resolver call `0x00672B6A`);
there is no `[General]` fallback.

### 3.1 Successful Unlimbo insertion

`BuildingClass::Unlimbo 0x004411B6..0x00441223` scans the Rules `BuildConst` vector. When the
newly committed Building type matches, it appends the Building pointer at the House vector tail.
Failed Unlimbo does not create a live vector entry.

### 3.2 Owner transfer

`BuildingClass::ChangeOwner @ 0x00448260` performs two distinct list mutations around the
delegated Techno owner swap:

- before the owner swap, scan `BuildConst` and stable-remove the Building from the old House's
  construction-yard vector by shifting the tail left;
- after the owner swap, scan `BuildConst` again and append the Building to the new House's vector
  tail.

Consequently a captured Construction Yard becomes the newest entry for the new House. Sorting
live entity IDs or consulting a global logic vector cannot reproduce the first pointer after
capture.

### 3.3 Limbo and expiry removal

The House pointer-expiry handlers reached during Building Limbo/destruction (including the
matching Building branch around `0x004FBA1D`) stable-remove the expired Building pointer from the
construction-yard vector when the removal flag is nonzero. A later successful re-entry appends
the Building again at the tail. Already-limbo/no-op paths do not mutate the vector.

The minimum equivalent Rust authority is therefore a per-House ordered vector of stable IDs,
updated only after successful structure Reveal, on the successful Conceal/Limbo transition, and
on the authoritative owner-transfer sequence (old stable-remove before swap, new append after
swap). The immutable entity membership bit must be stamped from resolved `BuildConst` identity at
construction time so rule-less lifecycle callers do not guess later.

## 4. First-buildable selector defaults completed

`HouseClass__FirstBuildableFromArray @ 0x005051E0` is described in the parent report section
8.1. One remaining constructor fact is exact: `TechnoTypeClass__Constructor @ 0x00710FF0`
initializes `TechnoType+0x6D0` (`AIBasePlanningSide`) to signed `-1` (`param_1[0x1B4] =
0xFFFFFFFF`). The reader around `0x007149FB` applies the INI override.

The primary-superweapon tail follows a House `SuperClass` instance back to its registered
`SuperWeaponType` and reads only `DisableableFromShell`. It does not inspect charge, active,
ready, grant, or suspension state. A Rust direct lookup of the same registered primary
superweapon type is therefore an exact representation; consulting runtime charge/grant state
would add a nonexistent gate. `SuperWeapon2` remains ignored.

## 5. Retail activation and exclusions

- `rulesmd.ini` supplies `[General] Shipyard=GAYARD,NAYARD,YAYARD` and
  `[AI] BuildConst=GACNST,NACNST,YACNST`; base RA2 supplies the first two of each.
- Every stock YR shipyard foundation is `4x4`, producing the native `6x6` FNPC footprint, but
  the side-dependent source-order selector is still active.
- Stock playable Houses always select a non-null shipyard. The native caller immediately uses
  both independently returned pointers, so a malformed custom ruleset with no passing shipyard
  falls outside active-retail behavior and would enter native invalid-memory/crash territory.
  Rust must fail the placement deterministically without inventing a fallback type; emulating a
  host-memory fault is excluded.
- Stock Houses normally own a BuildConst yard by the time this branch is useful, but the null
  first-pointer branch is explicit and supported: it skips only the adjacency cap.
- This evidence does not authorize replacing the first ordered yard with nearest yard, oldest
  stable ID, any `ConstructionYard=yes` type, or the House primary/alternate base cell.

## 6. Rust handoff

The existing exact prerequisites are:

- `find_nearby_passable_cell` including independently re-run native bridge projection;
- `HouseState.base_center` (`+0x5490`) and `alternate_base_center` (`+0x5494`);
- `cell_kernel::cell_center` plus the common 104-authority `ground_height_leptons` for CellClass
  coordinates;
- `native_x87::distance_3d_leptons` for the exact rounded distance.

The missing state is the Rules lists/scalar, `AIBasePlanningSide`, immutable per-entity BuildConst
membership, and the snapshot/hash-covered per-House insertion-order vector. These are required
inputs, not optional ordinary-base-planning work.
