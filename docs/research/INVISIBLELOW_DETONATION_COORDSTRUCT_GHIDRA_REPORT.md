# InvisibleLow Detonation CoordStruct - Ghidra Research Report

**Address(es):** `0x00468670` (`BulletClass::Fire`), `0x004666E0` (`BulletClass::AI`), `0x005880A0` (Inviso line helper), `0x00468D80` (`BulletClass::BulletDetonation`), `0x004690B0` (`BulletClass::Detonate`), `0x0049F420` (randomized CoordStruct helper)
**Confidence:** High for normal `Inviso=yes` coordinate flow, visual randomization, wall/cliff predicate details, `AlliedWallTransparency` identity/default, final `BulletClass::Fire` fallback CoordStruct stack ordering, and normal `InvisibleLow` AI/BounceCheck ordering.
**Active in YR:** Yes. `[InvisibleLow]` in `rulesmd.ini` is `Inviso=yes`, `SubjectToCliffs=yes`, `SubjectToElevation=yes`, `SubjectToWalls=yes`; `[M60]` and `[Para]` use it.

## 1. Overview

`InvisibleLow` is an invisible BulletType, not a pure direct-damage shortcut. `TechnoClass::Fire_At` still allocates a `BulletClass`, initializes it with the weapon projectile/warhead/damage/firer/target, and calls `BulletClass::Fire`.

For normal GI small-arms shots, the important coordinate chain is:

1. `BulletClass::Fire` receives the firing/source CoordStruct and resolves target/body CoordStructs.
2. If `BulletType.Inviso` is set, `BulletClass::Fire` runs an Inviso line helper and then sets the bullet object's `Location` (`+0x9C/+0xA0/+0xA4`) to a CoordStruct.
3. `BulletClass::BulletDetonation` starts from that bullet `Location`, then may override the final detonation CoordStruct to the target object's own CoordStruct if distance/type gates pass.
4. `BulletClass::Detonate` consumes that CoordStruct for damage/effects, but if `BulletType.Inviso` is set it randomizes the visible impact anim coordinate by radius `0x20` leptons before constructing the AnimClass.

So the visible GI `PIFFPIFF` puff is not tile-centered, but it is also not always exactly the target CoordStruct. It is the final detonation CoordStruct plus a small random Inviso visual offset.

## 2. Class Layout / Key Offsets

### BulletClass instance fields

| Offset | Type | Purpose | Evidence |
|--------|------|---------|----------|
| `+0x6C` | int | Bullet damage/strength from weapon damage. | `BulletClass::Init @ 0x004664C0` writes param 5. |
| `+0x90` | bool | Bullet alive flag; detonation/cluster loop tests it. | `0x00468D80`, `0x004690B0`. |
| `+0x9C/+0xA0/+0xA4` | CoordStruct | Bullet object `Location`; initial source of detonation CoordStruct. | `0x00468D8A..0x00468DAC`, `ObjectClass::GetCoords @ 0x005F65A0`. |
| `+0xAC` | BulletTypeClass* | Projectile type. | `BulletClass::Init @ 0x004664C0`; consumers in `0x00468670`, `0x00468D80`, `0x004690B0`. |
| `+0xB0` | TechnoClass* | Firer/owner techno. | `BulletClass::Init`; `BulletClass::Fire` reads `+0x21C` house from it for Inviso helper. |
| `+0xE0` | bool | Bright flag copied from WeaponType. | `BulletClass::Init`. |
| `+0xE8..+0xFF` | three doubles | Velocity vector copied in `BulletClass::Fire`; may be normalized in fallback. | `0x00468691..0x004686A0`, `0x00468992..0x00468A39`. |
| `+0x10C` | AbstractClass* | Target object. | `BulletClass::Init`; target-coordinate overrides in `0x00468D80`. |
| `+0x110` | int | Speed/runtime parameter from init; overwritten to 0 in an Inviso fallback branch. | `BulletClass::Init`, `0x0046898C`. |
| `+0x128` | WarheadTypeClass* | Warhead. | `BulletClass::Init`; `BulletClass::Detonate` reads `param_1[0x4A]`. |
| `+0x134/+0x138/+0x13C` | CoordStruct | Fire input CoordStruct copied at launch. | `0x004686A5..0x004686BC`. |
| `+0x140/+0x144/+0x148` | CoordStruct | Target/body CoordStruct from target vtable `+0x58`. | `0x00468700..0x0046872B`. |
| `+0x14C` | packed cell XY | Cell derived from fire input CoordStruct. | `0x004686BF..0x004686EF`. |
| `+0x8C` | bool | OnBridge copied from target if `Inviso=yes` and target OnBridge. | `TechnoClass::Fire_At @ 0x006FF080..0x006FF0B0` in live decompile. |

### BulletTypeClass fields relevant here

| Offset | INI key | GI value | Meaning in this path | Evidence |
|--------|---------|----------|----------------------|----------|
| `+0x294` | `Airburst` | false | If true, bypasses normal cluster path; not GI. Also prevents some target-coordinate overrides in `BulletDetonation`. | `0x00468E67`, `0x00469000`, prior `BULLETTYPECLASS_GHIDRA_REPORT.md`. |
| `+0x296` | `SubjectToCliffs` | true | Enables cliff checks in fallback/shared collision helpers. | `0x004CC100`, `0x00468BB0`, INI. |
| `+0x297` | `SubjectToElevation` | true | Parsed flag; separately gates range/elevation behavior. It is not read directly in the normal Inviso line helper. | `BULLETTYPECLASS_GHIDRA_REPORT.md`; `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`. |
| `+0x298` | `SubjectToWalls` | true | Enables wall checks in fallback/shared collision helpers. | `0x004CC100`, `0x00468BB0`, INI. |
| `+0x29B` | `Arcing` | false | If true, changes `BulletDetonation` override path; not GI. | `0x00468ECF`. |
| `+0x29D` | `Level` / `Floater` naming varies in older docs | false for GI | Used in BounceCheck water/floater-style branch, not normal GI. | `0x00468BB0`. |
| `+0x29E` | `Inviso` | true | Enables invisible impact path in `Fire` and visual randomization in `Detonate`. | `0x004688AF`, `0x00469AA4`, INI. |
| `+0x2A2` | `Inaccurate` | false for GI | If true, suppresses several target-position corrections in `BulletDetonation`. | `0x00468DC7`, `0x00468ECF`. |
| `+0x2A3` | `FlakScatter` | false for GI | If true with Inviso, applies launch scatter before the Inviso path. | `0x00468734..0x004688A9`. |
| `+0x2A4` | `AA` | false/default for `InvisibleLow` | Used by BounceCheck proximity-to-air target branch. | `0x00468BB0`; INI comments. |
| `+0x2AC` | `Cluster` | 0 for GI | Non-Airburst cluster count; Inviso bypasses this loop. | `0x00469020`, `0x0046909A`. |
| `+0x2F0` | `Arm` | 0 for `InvisibleLow` | Proximity arming delay passed to `ProximityDetector::Set`. | `0x00468A57..0x00468A93`. |

### Warhead fields relevant here

| Offset | INI key/name | SA/SSA value | Meaning | Evidence |
|--------|--------------|--------------|---------|----------|
| `+0x154` | `EMEffect` | false | If true, `BulletDetonation` skips target-coordinate correction block. | `0x00468E9F`; `WARHEAD_DETONATE_GHIDRA_REPORT.md`. |
| `AnimList` | `AnimList=` | `PIFFPIFF,PIFFPIFF` | The visible small-arms impact animation selected in detonation. | `rulesmd.ini`, `0x004690B0` AnimClass construction. |
| `Bullets` | `Bullets=yes` | true | Warhead semantic flag, but not the CoordStruct producer. | `rulesmd.ini`; warhead docs. |

## 3. Core Logic

### 3.1 Fire_At creates a real BulletClass

`TechnoClass::Fire_At @ 0x006FDD50` allocates a bullet with `BulletClass::Allocate`, calls `BulletClass::Init`, computes launch/source/target vectors, and calls the bullet vtable slot `+0x1F0`, which resolves to `BulletClass::Fire @ 0x00468670`.

Relevant details:

- `BulletClass::Allocate @ 0x0046B050` uses `CoCreateInstance` with `DAT_007E96E0` / `DAT_007F7C90`, then calls `BulletClass::Init`.
- `BulletClass::Init @ 0x004664C0` writes target `+0x10C`, speed/runtime parameter `+0x110`, warhead `+0x128`, bright `+0xE0`, damage `+0x6C`, BulletType `+0xAC`, and firer `+0xB0`.
- After a successful `BulletClass::Fire`, `TechnoClass::Fire_At` copies target `OnBridge` into bullet byte `+0x8C` if `BulletType.Inviso != 0` and the target has `ObjectClass+0x8C != 0`. This is active in YR and bridge-relevant, but no decompiled CoordStruct consumer in this path reads `+0x8C` to change X/Y/Z.

**Confidence:** HIGH. Evidence: live decompilation `0x006FDD50`, `0x0046B050`, `0x004664C0`.
**Active in YR:** Yes.

### 3.2 BulletClass::Fire normal Inviso path

`BulletClass::Fire @ 0x00468670`:

1. Calls `ObjectClass::Reveal`; returns false if reveal fails.
2. Copies the input velocity into `bullet+0xE8..+0xFF`.
3. Copies the fire/source CoordStruct into `bullet+0x134..+0x13C` and derives packed cell XY at `+0x14C`.
4. Calls target vtable `+0x58` and copies that CoordStruct into `bullet+0x140..+0x148`.
5. If `FlakScatter && Inviso`, applies a random horizontal scatter before the Inviso line helper. GI `InvisibleLow` has no `FlakScatter`, so this branch is inactive for GI.
6. If `Inviso`, calls `FUN_005880A0(output, source_coord, target_coord, firer_house)`.
7. If that helper returns a non-sentinel CoordStruct, the code calls `CellClass::GetGroundHeight` for that X/Y, writes the returned Z into the CoordStruct, then calls the bullet vtable `+0x1B4` with that CoordStruct. This is the clear fire-time impact coordinate set.
8. If the helper returns the sentinel (`DAT_0089DE30/34/38` compared against the helper result), the code enters a fallback branch involving `FUN_004CC100`, target `GetCoords`, and velocity normalization. Stack reconstruction around `0x0046893C..0x00468986` shows that the optional blocker-cell `Set_Raw_Coords` at `0x00468966` is overwritten by a final `Set_Raw_Coords(&target_coord)` at `0x00468986`. Therefore the lasting fire-time bullet `Location` after this fallback is the target/body CoordStruct, not an ordinary wall/cliff blocker cell and not a tile center.
9. `ProximityDetector::Set @ 0x004E1130` is armed from current bullet location to the target CoordStruct. If the target object type is `2`, arm delay is forced to 0; otherwise it uses BulletType `Arm` (`+0x2F0`).
10. If the bullet is alive, it is submitted to the display/layer list even though `Image=none`.

**Important correction:** `FUN_005880A0` is not a general "always return exact target impact" raycast. In the no-blocker case it returns the sentinel and the fallback path handles the actual location/velocity. Existing shorthand that called it the whole Inviso raycast is incomplete.

**Confidence:** HIGH. Steps 1-7 and 9-10 are direct decompile findings; step 8 is direct assembly/stack reconstruction using verified callee cleanup for `FUN_005880A0`, `FUN_004CC100`, `ObjectClass::GetCoords`, and `ObjectClass::Set_Raw_Coords`.
**Active in YR:** Yes for all `Inviso=yes` projectiles; `FlakScatter+Inviso` branch is conditional and not GI.

### 3.3 Inviso helper `FUN_005880A0`

The helper walks cells from source CoordStruct to target CoordStruct and returns either:

- a blocking CoordStruct, or
- the sentinel `DAT_00ABDC10/14/18` / compared as `DAT_0089DE30/34/38` by caller.

Details verified from live decompilation:

- Exact same source and target CoordStruct returns sentinel immediately.
- Cell conversion is integer lepton-to-cell using `(coord + sign_adjust_0xFF) >> 8`. This is the same signed floor-ish pattern seen elsewhere, not floating projection.
- Same-X and same-Y paths use simple cell stepping by `+1` or `-1`.
- Diagonal/general path computes next cell-boundary crossings:
  - X step is `+0x100` or `-0x100`.
  - Y step is `+0x100` or `-0x100`.
  - It advances whichever normalized boundary fraction is smaller.
- Out-of-bounds/null cell writes the packed cell into global `DAT_00ABDC74`, but the helper continues; this is diagnostic/fallback state, not a returned CoordStruct by itself.
- Each visited cell calls `Look_up_building_in_cell`.
- The returned building only blocks if all of these are true:
  - building exists,
  - `building.Type+0x16C0 != 0`,
  - `building.Owner.HouseType+0x1FA != 0`,
  - `building.Owner.HouseType != firer_house`.
- Existing `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` identifies `Type+0x16C0` as `IsLaserFence=` and owner HouseType `+0x1FA` as a legal/active-target gate for powered laser fences. That means this helper is specifically looking for hostile active laser-fence buildings, not ordinary walls/buildings.
- Same-X/same-Y blocking returns the blocking cell center:
  - `x = cell_x * 0x100 + 0x80`
  - `y = cell_y * 0x100 + 0x80`
  - `z = 0` before caller ground-height adjustment
- Diagonal/general blocking returns the crossed cell-boundary coordinate:
  - `x = next_boundary_x`
  - `y = next_boundary_y`
  - `z = 0` before caller ground-height adjustment

**Confidence:** HIGH. Evidence: live decompile `0x005880A0`; `Look_up_building_in_cell @ 0x0047C520`; prior unit-cell report for field semantics.
**Active in YR:** Conditional. Active for `Inviso=yes`; only changes the CoordStruct when hostile active laser-fence building cells are crossed.

### 3.4 Wall/cliff fallback helpers

`FUN_004CC100` and `FUN_004CC360` are shared line/collision helpers for `SubjectToCliffs` and `SubjectToWalls`.

Verified details:

- `FUN_004CC100 @ 0x004CC100` returns 0 immediately unless `BulletType.SubjectToCliffs` (`+0x296`) or `SubjectToWalls` (`+0x298`) is true.
- It computes source/target cells using the same 256-lepton cell conversion.
- Step count is `max(abs(dx_cells), abs(dy_cells))`; if 0, step deltas are 0.
- It steps from source toward target with integer deltas:
  - `step_x = (target_x - source_x) / steps`
  - `step_y = (target_y - source_y) / steps`
  - `step_z = (target_z - source_z) / steps`
- At each step it calls `FUN_004CC360`.
- `FUN_004CC360 @ 0x004CC360` checks:
  - `SubjectToCliffs`: compares effective heights; a positive height jump greater than 3 levels can block.
  - `SubjectToWalls`: checks the cell overlay type's wall flag at overlay type `+0x2A8`.
  - Target/end cell is exempt in one branch (`if candidate_cell == target_cell return 0`).
  - Wall blocking can be ignored when `[WallModel] AlliedWallTransparency` (`RulesClass+0x1850`) is enabled and the wall owner's house is allied with the passed house. Standard YR sets this to `no`, so allied wall transparency is inactive by default.

For normal GI target shots with no wall/cliff/laser-fence blocker, these helpers should not change the final target CoordStruct. For ordinary wall/cliff blocker cases in the `BulletClass::Fire` sentinel fallback, the helper can return a blocker cell and that cell can be written transiently, but the final fire-time bullet `Location` is overwritten with the target CoordStruct before `TargetSpeed` is zeroed and proximity setup proceeds.

### 3.4a Follow-up verification: wall/cliff/elevation blockers and AI ordering

Targeted follow-up on 2026-05-17 rechecked `FUN_004CC100`, `FUN_004CC360`, `BulletClass::BounceCheck`, `CellClass::GetEffectiveHeight`, `CellClass::Get_Cell_At`, `MapClass::Get_CellClass`, and `[WallModel]` parsing.

Verified details:

- `FUN_004CC100 @ 0x004CC100` is gated only by `SubjectToCliffs` (`BulletType+0x296`) or `SubjectToWalls` (`BulletType+0x298`). `SubjectToElevation` (`+0x297`) is not read by this wall/cliff line helper.
- `FUN_004CC100` converts source and target lepton coords to cell coords with `(coord + (coord >> 31 & 0xFF)) >> 8`, then uses `max(abs(dx_cells), abs(dy_cells))` as the step count.
- If the step count is zero, all deltas are set to zero and the loop is skipped; the helper returns no blocker.
- Otherwise `step_x`, `step_y`, and `step_z` are integer divisions of total lepton delta by the cell-step count. There is no floating line interpolation in this helper.
- The loop condition is `step_index < step_count`, so the target endpoint is not checked as a normal intermediate blocker by this helper.
- Each sampled position calls `FUN_004CC360` with the current interpolated X/Y/Z, the BulletType, and the firing house.
- `FUN_004CC360` derives the sampled cell by `MapClass::Get_CellClass` from the sampled lepton X/Y. Out-of-map coordinates go through the standard dummy cell path and set `DAT_00ABDC74`.
- `CellClass::GetEffectiveHeight @ 0x00487D50` is exactly `cell.Level(+0x11B) + ((cell.Flags(+0x140) >> 7) & 1) * 4`. Bridge-flagged cells therefore count as four levels higher for this predicate.
- The cliff block is an upward-step test: when `SubjectToCliffs` is true, a positive effective-height jump greater than 3 levels can return the sampled cell as a blocker. The practical threshold is a 4-level or greater upward face.
- The wall block is an overlay test: when `SubjectToWalls` is true, the sampled cell must have an overlay index not equal to `-1`, and that overlay type must have byte `+0x2A8` set (`Wall=yes`).
- The sampled target/end cell is exempt from wall blocking in one branch (`candidate cell == destination cell` returns no block).
- The wall predicate compares effective heights (`CellClass+0x11B` bytes in the decompiler view) and does not block when the destination-side height is lower than the source-side height. This is the "not firing downhill over wall" condition recorded in `CLIFF_OBJECTS_GHIDRA_REPORT.md`.
- `RulesClass+0x1850` is verified as `[WallModel] AlliedWallTransparency`; `RulesClass::ReadWallModel @ 0x0066D1F0` reads `AlliedWallTransparency` into `+0x1850` and `WallPenetratorThreshold` into `+0x1858`.
- The constructor default for `RulesClass+0x1850` is `0`, and both `ini/rules.ini` and `ini/rulesmd.ini` set `AlliedWallTransparency=no`. Therefore standard YR GI shots do not ignore allied walls through this flag.
- If `AlliedWallTransparency=yes`, the helper looks up the wall owner from `CellClass+0x50`, resolves the house from `g_HouseClass_Array`, and skips the wall block only when that house is allied with the firing house passed into the helper.
- `BulletClass::BounceCheck @ 0x00468BB0` also calls `FUN_004CC360` when `SubjectToCliffs` or `SubjectToWalls` is set. It passes the bullet's current `Location` as the candidate CoordStruct and returns true immediately if `FUN_004CC360` returns a cell.
- For normal `InvisibleLow` GI target shots, `BulletClass::Fire` handles the first impact placement before AI detonation, and `BulletClass::AI` does not then use `BounceCheck` to choose the impact point. At `0x00467840..0x0046788B`, AI compares the current bullet cell against `bullet+0x140/+0x144` target cell; for the normal same-cell Inviso result, it sets the detonation flags at `0x00467879..0x00467886` and jumps to the detonation block.
- At `0x00467B7A..0x00467BAA`, AI first calls `Set_Raw_Coords` with the current candidate CoordStruct, then tests the detonation flag byte at `[ESP+0x18]`. When that flag is already set by the same-cell target branch, the `JNZ 0x00467BF0` skips the `BounceCheck @ 0x00468BB0` call entirely.
- `ProximityDetector::Check @ 0x004E11F0` is also not the normal GI trigger. AI only calls it at `0x00467C2A..0x00467C35` when `BulletType.ROT > 0` (`+0x2DC`) or `BulletType.Ranged` (`+0x2A0`) is true. Standard `[InvisibleLow]` has no `ROT=` and no `Ranged=`, so this path is skipped for GI small-arms shots.
- `BounceCheck` remains relevant as the shared per-tick path for non-instant / moving bullets and as confirmation that `FUN_004CC360` is the canonical cliff/wall collision predicate. It is not the normal GI impact-coordinate producer after `Fire`.
- `SubjectToElevation` is confirmed not to be a wall/cliff impact-ray flag. Its verified YR-active consumer is the InRange height-fire bonus in `TechnoClass` range checks, as documented in `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`.
- Follow-up stack reconstruction of `BulletClass::Fire` around `0x0046893C..0x00468986` resolves the previous final-CoordStruct ambiguity. `[ESP+0x44]` is the target CoordStruct local; `FUN_004CC100` is called with that target pointer and the source pointer. If `FUN_004CC100` returns a blocker cell, the code calls the cell's `GetCoords` and temporarily calls bullet `Set_Raw_Coords` with that CoordStruct at `0x00468966`. Both the blocker and no-blocker paths then fall through to `0x0046897D..0x00468986`, which calls bullet `Set_Raw_Coords` again with `[ESP+0x44]`, the target CoordStruct. `ObjectClass::Set_Raw_Coords @ 0x005F6940` directly overwrites `ObjectClass+0x9C/+0xA0/+0xA4`, so the final call wins.
- `ObjectClass::GetCoords @ 0x005F65A0` and `ObjectClass::Set_Raw_Coords @ 0x005F6940` both use single-argument callee cleanup (`ret 4`), `FUN_004CC100` returns with `ret 8`, and `FUN_005880A0` returns with `ret 16`. The stack offsets used by the final `Set_Raw_Coords` are therefore stable in this reconstruction.

Resolved 2026-05-17: the previous same-tick AI/BounceCheck caveat does not apply to normal `InvisibleLow` GI target hits. The normal path sets a detonation flag before the `BounceCheck` call site and skips the call. This does not prove every non-GI `Inviso=yes` projectile behaves the same way, because `Ranged`, `ROT`, `FlakScatter`, `Vertical`, `Airburst`, and target type can route through other AI branches.

**Confidence:** HIGH for all predicate/key/default facts above, final `Fire` fallback CoordStruct dominance, and normal `InvisibleLow` AI/BounceCheck ordering.
**Active in YR:** Yes for `InvisibleLow` because it has `SubjectToCliffs=yes` and `SubjectToWalls=yes`; `AlliedWallTransparency` bypass is inactive in standard YR data because the key is `no`.

### 3.5 BulletDetonation final CoordStruct selection

`BulletClass::BulletDetonation @ 0x00468D80` starts with:

```text
detonation = bullet.Location (+0x9C/+0xA0/+0xA4)
```

Then it may overwrite `detonation` before calling `BulletClass::Detonate @ 0x004690B0`.

Verified branches:

- If target exists and `target->vtable+0x54()` reports alive/valid, store it as `live_target`.
- If `BulletType.Inaccurate == false`:
  - If target exists, compute 3D distance from current detonation CoordStruct to target `GetCoords` (`vtable+0x48`).
  - If distance `< 0x20`, `BulletType.Airburst == false`, and `Inaccurate == false`, overwrite detonation with target `GetCoords`.
- If `Warhead.EMEffect == false` and `BulletType.Airburst == false`, more target overrides can run:
  - If `live_target` exists and `live_target->vtable+0x78() != 2`, call `FUN_005F6360(this_bullet, target)`. If result `< 0x80`, call target vtable `+0xA4` (`GetCoords_OutputParam`) and overwrite detonation.
  - Else, if target exists and `FUN_005F6360(this_bullet, target) < 0x2A`, call target vtable `+0x58`, overwrite detonation with that CoordStruct, and if target type is building (`vtable+0x2C == 6`) with any nonzero building-type fields at `+0xEBC/+0xEC0/+0xEC4`, call target vtable `+0xA4` and overwrite detonation again.
- If `BulletType.Airburst == false`:
  - If `Cluster > 0`, loop `Cluster` times and call `BulletClass::Detonate`; after each live detonation, randomize the detonation CoordStruct by radius `0x100..0x200` using `FUN_0049F420`.
  - If `Cluster <= 0`, the loop does not run in this branch.
- If `BulletType.Airburst == true`, call `BulletClass::Detonate` once.
- If `BulletType.Airburst != false` test at `0x0046909A` actually routes `Inviso`? No: the instruction at `0x00468FFA` tests `BulletType+0x294`, which prior BulletType docs identify as `Airburst`, not `Inviso`. `Inviso` is `+0x29E` and is tested later in `BulletClass::Detonate`.

The older wording in `BULLETCLASS_TRAJECTORY_AND_HOMING.md` that treated vtable `+0xA4` as direct `ReceiveDamage` is stale for this path. `0x0041BDD0` is a wrapper that calls vtable `+0x48` (`GetCoords`) and copies X/Y/Z to the caller's output pointer. It is a coordinate-output helper.

**Confidence:** HIGH. Evidence: live disassembly/decompile `0x00468D80`, `0x0041BDD0`, `0x005F6360`.
**Active in YR:** Yes. Branches are conditional on target state/type and projectile/warhead flags.

### 3.6 Visible impact anim coordinate in BulletClass::Detonate

`BulletClass::Detonate @ 0x004690B0` receives the detonation CoordStruct pointer from `BulletClass::BulletDetonation`.

Near `0x00469AA4`, it copies the incoming CoordStruct:

```text
local_anim_coord = *detonation_coord
```

Then it checks `BulletType.Inviso` at `bullet->Type + 0x29E`:

```text
if (bullet.Type.Inviso) {
    local_anim_coord = RandomizeCoords(base=detonation_coord, radius=0x20, snap_to_cell_center=false)
}
```

The randomized coordinate is established before `Warhead__SelectExplosionAnim` and
then used for the combat-light/smudge path and
`AnimClass__Constructor(anim, &local_anim_coord, 0, 1, 0x2600, z_adjust, 0)`.
The `Inviso` call to `FUN_0049F420` therefore consumes its RNG byte even when
animation selection later returns null. This ordering is explicit at
`0x00469AA4..0x00469AF0`; the selection call follows at
`0x00469BA2..0x00469BD4`.

`FUN_0049F420 @ 0x0049F420` exact behavior:

- Inputs by calling convention:
  - `ECX` = output CoordStruct pointer.
  - `EDX` = base CoordStruct pointer.
  - stack arg 1 = radius.
  - stack arg 2 = snap-to-cell-center bool.
- It loads the active ScenarioClass pointer from `0x00A8B230`, forms
  `ScenarioClass+0x218`, calls `Random::Next()` on that Scenario RNG, and uses
  the returned byte as an angle. This is not a `g_MainRng` draw. Direct
  assembly: `0x0049F423 MOV EAX,[0x00A8B230]`,
  `0x0049F432 LEA ECX,[EAX+0x218]`,
  `0x0049F43F CALL 0x0065C780`.
- Angle unit:
  - `angle = (((random_byte << 8) as i16) - 0x3FFF) * -0.00009587672516830327`
  - The multiplier is effectively `-2*pi/65536`.
- It computes:
  - `x = ftol(base.x + cos_lookup(angle) * radius)`
  - `y = ftol(base.y - sin_lookup(angle) * radius)`
  - `z = base.z`
- `cos_lookup` and `sin_lookup` here are the binary's float-table results, not
  host-runtime trigonometry. `0x004CACB0` indexes the sine table rooted at
  `0x0084F084`; `0x004CAD00` indexes the same table one quarter-turn later.
  `FUN_0049F420` calls the first result for Y and the quarter-turn result for X.
- The gameplay FPU control word at `0x00822D80` is `0x0E7F`, so both lookup-index
  conversion and final coordinate conversion use the binary's toward-zero
  `FISTP` behavior. For the radius-`0x20` call at a positive map-scale base,
  byte `0` produces `(dx,dy) = (0,-32)`, byte `64` produces `(32,0)`, byte
  `128` produces `(-1,32)`, and byte `192` produces `(-32,-1)`. The latter
  two one-lepton biases come from the table's tiny negative/positive
  near-zero samples combined with truncate-toward-zero x87 arithmetic; they
  are not exact mathematical zeroes.
- If the randomized X/Y cell coordinate is outside `[0, 0x1FF]`, it falls back to the original base CoordStruct.
- If snap bool is true, it replaces X/Y with cell center:
  - `x = (x & ~0xFF) + 0x80`
  - `y = (y & ~0xFF) + 0x80`
- For Inviso impact anims, the caller passes radius `0x20` and snap `0`, so GI puffs are scattered within about 32 leptons of the final detonation CoordStruct and are not snapped to cell center.

This is the most important new finding for visual parity: the visible `PIFFPIFF` is lepton-precise and slightly randomized. A Rust fix that only preserves target sub-cell coords is directionally correct but still misses this retail random offset.

**Confidence:** HIGH. Evidence: live `decompile_function` /
`disassemble_function` reads on 2026-07-24 for `0x004690B0`, `0x0049F420`,
`0x004CACB0`, `0x004CAD00`, and `0x007C5F00`; `read_memory` at
`0x007E2810`, `0x008223B0`, `0x00822D80`, and sine-table anchors
`0x0084F084`, `0x00851084`, `0x00853084`, `0x00855084`.
**Active in YR:** The RNG/scatter path executes for every `Inviso=yes`
detonation, even when no impact anim is selected. Its coordinate becomes visible
when the warhead selects an impact anim. GI `[M60]`/`[Para]` hit this path.

## 4. INI Keys

YR `rulesmd.ini` values override base `rules.ini`.

| Section | Key | YR value | Effect |
|---------|-----|----------|--------|
| `[M60]` | `Projectile` | `InvisibleLow` | Uses the Inviso projectile family. |
| `[M60]` | `Warhead` | `SA` | Small-arms warhead with `PIFFPIFF`. |
| `[M60]` | `Damage` | `15` | Drives damage and AnimList index selection. |
| `[M60]` | `Speed` | `100` | Passed through bullet setup; Inviso still has no visible travel. |
| `[M60]` | `Anim` | `MGUN-*` | Muzzle flash only; separate from impact anim. |
| `[Para]` | `Projectile` | `InvisibleLow` | Deployed GI also uses Inviso low. |
| `[Para]` | `Warhead` | `SSA` | Deployed GI warhead with `PIFFPIFF`. |
| `[Para]` | `Damage` | `25` in YR | Different from base RA2 `15`; relevant to damage and selected impact anim index. |
| `[InvisibleLow]` | `Inviso` | `yes` | Enables `BulletClass::Fire` Inviso path and `Detonate` visual randomization. |
| `[InvisibleLow]` | `Image` | `none` | No projectile sprite. |
| `[InvisibleLow]` | `SubjectToCliffs` | `yes` | Enables cliff checks in shared collision/fallback paths. |
| `[InvisibleLow]` | `SubjectToElevation` | `yes` | Height/range behavior; not the normal visual puff coordinate producer. |
| `[InvisibleLow]` | `SubjectToWalls` | `yes` | Enables wall checks in shared collision/fallback paths. |
| `[InvisibleMedium]` | `SubjectToWalls` | `no` | Same Inviso visual logic, but shoots over walls by data. |
| `[InvisibleHigh]` | `SubjectToElevation` | `yes`; no wall/cliff flags | Can fire over walls/cliffs per data comments. |
| `[InvisibleAll]` | `AA/AG` | `yes/yes`; no subject flags | Infinite-range all-purpose Inviso; do not apply GI wall assumptions to it. |
| `[FlakProj]` | `Inviso`/`FlakScatter`/`Ranged` | `yes/yes/yes` | Shares Inviso mechanics but has scatter/ranged branches outside GI scope. |
| `[SA]` | `AnimList` | `PIFFPIFF,PIFFPIFF` | Visible impact puff. |
| `[SA]` | `Bullets` | `yes` | Warhead semantic flag. |
| `[SA]` | `ProneDamage` | `70%` | Damage modifier, not coordinate behavior. |
| `[SSA]` | `AnimList` | `PIFFPIFF,PIFFPIFF` | Visible deployed-GI impact puff. |
| `[SSA]` | `Bullets` | `yes` | Warhead semantic flag. |
| `[SSA]` | `ProneDamage` | `80%` in YR | Damage modifier, not coordinate behavior. |

## 5. Integration Points

### Calls into the system

| Function | Role | Evidence |
|----------|------|----------|
| `TechnoClass::Fire_At @ 0x006FDD50` | Primary caller: selects weapon, creates bullet, calls `Fire`. | Live decompile. |
| `BulletClass::Allocate @ 0x0046B050` | COM bullet allocation. | Live decompile. |
| `BulletClass::Init @ 0x004664C0` | Writes BulletType, target, firer, damage, warhead. | Live decompile. |
| `BulletClass::Fire @ 0x00468670` | Sets bullet location/velocity and Inviso impact coordinate. | Live decompile/disassembly. |
| `BulletClass::AI @ 0x004666E0` | Per-tick bullet update; detonation calls `0x00468D80`. | Existing `BULLET_CLASS_AI_GHIDRA_REPORT.md`, spot source docs. |
| `BulletClass::BulletDetonation @ 0x00468D80` | Computes final detonation CoordStruct and calls detonate. | Live disassembly/decompile. |
| `BulletClass::Detonate @ 0x004690B0` | Applies warhead effects and constructs impact AnimClass. | Live decompile. |
| `RulesClass::ReadWallModel @ 0x0066D1F0` | Reads `[WallModel]` allied-wall transparency and wall-penetrator threshold. | Live decompile. |

### Key callees

| Function | Role | Notes |
|----------|------|-------|
| `FUN_005880A0` | Inviso hostile active laser-fence line helper. | Returns blocking CoordStruct or sentinel. |
| `FUN_004CC100` | Subject-to-cliffs/walls line helper. | Fallback/shared collision; final fire-time side effect is the main remaining uncertainty. |
| `FUN_004CC360` | Cliff/wall predicate. | Checks effective height jumps and wall overlays. |
| `CellClass::GetEffectiveHeight @ 0x00487D50` | Effective cell-height helper. | `Level + 4` when bridge overlay flag bit 7 is set. |
| `CellClass::GetGroundHeight @ 0x00578080` | Writes/returns ground height for X/Y. | Uses X/Y cell lookup; bridge deck not proven in this wrapper. |
| `Look_up_building_in_cell @ 0x0047C520` | Returns first building object in the cell's object list. | Only building type (`vtable+0x2C == 6`) matches. |
| `ObjectClass::GetCoords @ 0x005F65A0` | Basic CoordStruct getter. | Copies `ObjectClass+0x9C/+0xA0/+0xA4`. |
| `GetCoords_OutputParam @ 0x0041BDD0` | vtable `+0xA4` wrapper. | Calls `GetCoords` and copies X/Y/Z to caller output; not damage. |
| `FUN_005F6360` | Bullet-target distance helper. | Uses 3D distance; subtracts building foundation `(height+width)*0x40`, clamped to 0. |
| `FUN_0049F420` | Randomized CoordStruct helper. | Used by Inviso impact anims with radius `0x20`, and by cluster scatter with larger radii. |

## 6. Current Rust Implementation Status

Relevant current Rust surface:

| File | Status |
|------|--------|
| `src/rules/projectile_type.rs` | Parses `inviso`, `subject_to_cliffs`, `subject_to_elevation`, `subject_to_walls`, `aa`, `ag`, `flak_scatter`, `arm`, etc. |
| `src/rules/weapon_type.rs` | Parses `Projectile=`, `Warhead=`, `Speed=`, `Anim=`. |
| `src/rules/ruleset.rs` | Loads projectile/warhead references used by weapons. |
| `src/sim/combat/inviso_scatter.rs` | Implements the narrow `FUN_0049F420` radius-`0x20`, snap-false mechanism with the binary float-table samples, x87 53-bit/toward-zero evaluation, native cell-boundary fallback, and one low-byte Scenario-RNG draw. |
| `src/sim/combat/mod.rs` | Current combat is still instant-hit, but now applies the verified `Inviso=yes` scatter only to the visible warhead effect coordinate before AnimList selection. Damage, ore, walls, radiation, and other detonation consumers keep the original coordinate. The wider code still does not build a full BulletClass-equivalent Inviso impact resolver or simulate the `BulletDetonation` target-override thresholds. |
| `src/sim/components.rs` | `WorldEffect` can carry sub-cell offsets, so it can represent lepton-precise impact anims. |
| `src/sim/world/mod.rs` | Passes the persistent Scenario simulation RNG into combat and transfers explosion effect coordinates into world effects. |
| `src/app_instances/overlays.rs` | Renders world effects through lepton projection, so render is ready to consume exact game-space CoordStructs. |
| `src/sim/combat/in_range.rs` | Uses `subject_to_elevation` for range/height behavior. This is separate from impact anim placement. |
| `src/sim/combat/combat_weapon.rs` | Uses projectile `AA`/`AG` filtering. |

Implementation-facing findings, without proposing code structure:

- Normal GI shots should not snap to tile center. They should be based on target/body CoordStructs and final detonation overrides.
- The visible AnimList coordinate for `Inviso=yes` should include the `FUN_0049F420` radius-`0x20`, snap-false randomization. This randomization belongs to deterministic sim/effect generation, not app/render screen-space code.
- Rust still drains paired animation-smudge requests after the combat batch,
  while gamemd constructs/starts each impact AnimClass inline. The new scatter
  draw is in live-object order, but full Scenario-RNG interleaving with other
  animation/debris mechanisms remains UNCHECKED. Stock GI `PIFFPIFF` has
  neither `Scorch` nor `Crater`, so the common GI paired-smudge path adds no
  second RNG draw.
- Laser-fence interception from `FUN_005880A0` is a separate edge case from ordinary `SubjectToWalls`.
- `SubjectToWalls`/`SubjectToCliffs` behavior for Inviso fallback should not treat the ordinary helper blocker cell as the final `Fire` location; the final `Fire` fallback location is the target CoordStruct. For normal `InvisibleLow` GI target hits, the later AI path sets the detonation flag before `BounceCheck`, so `BounceCheck` does not alter the impact coordinate.
- The recent G1 FLH muzzle/report fix should stay unchanged; this report covers impact/detonation side only.

## 7. Findings

| Finding | Evidence | Confidence | Active in YR? |
|---------|----------|------------|---------------|
| GI `M60`/`Para` use `InvisibleLow`, not a visible projectile sprite. | `rulesmd.ini` `[M60]`, `[Para]`, `[InvisibleLow]`. | HIGH | Yes. |
| `InvisibleLow` has `Inviso=yes`, `SubjectToCliffs=yes`, `SubjectToElevation=yes`, `SubjectToWalls=yes`. | `rulesmd.ini` `[InvisibleLow]`. | HIGH | Yes. |
| `BulletClass::Fire` still runs for Inviso shots. | `TechnoClass::Fire_At @ 0x006FDD50` calls vtable `+0x1F0`; `BulletClass::Fire @ 0x00468670`. | HIGH | Yes. |
| `FUN_005880A0` blocks only on hostile active laser-fence buildings, not every ordinary building or wall. | Checks building type `+0x16C0`, owner HouseType `+0x1FA`, owner house != firer house. | HIGH | Conditional. |
| No-blocker Inviso helper result is a sentinel, not the target coordinate. | `FUN_005880A0` writes sentinel at `LAB_00588539`; caller compares against `DAT_0089DE30/34/38`. | HIGH | Yes. |
| Ordinary wall/cliff blocking is handled by `FUN_004CC100` / `FUN_004CC360`, not by `FUN_005880A0`. | Live decompile `0x004CC100`, `0x004CC360`, `0x005880A0`. | HIGH | Conditional on `SubjectToWalls` / `SubjectToCliffs`. |
| `SubjectToElevation` does not participate in `FUN_004CC100` / `FUN_004CC360`. | No reads of `BulletType+0x297` in the helpers; existing InRange report maps it to height-fire range bonus. | HIGH | Yes, but for range, not impact raycast. |
| `CellClass::GetEffectiveHeight` is `Level + 4` when flag bit 7 of `CellClass+0x140` is set. | `CellClass__GetEffectiveHeight @ 0x00487D50`. | HIGH | Yes. |
| Cliff blocking threshold is an upward effective-height jump greater than 3 levels. | `FUN_004CC360` calls `GetEffectiveHeight` and branches on `3 < delta`. | HIGH | Conditional on `SubjectToCliffs`. |
| Wall blocking requires overlay type byte `+0x2A8` (`Wall=yes`) and skips the destination cell. | `FUN_004CC360`, overlay index `CellClass+0x44`, overlay type array, compare candidate vs target cell. | HIGH | Conditional on `SubjectToWalls`. |
| Allied-wall pass-through is controlled by `[WallModel] AlliedWallTransparency`, default false and `no` in standard YR. | `RulesClass::ReadWallModel @ 0x0066D1F0`, constructor defaults, `ini/rulesmd.ini`. | HIGH | Conditional; inactive by default. |
| Non-sentinel Inviso helper result has Z forced to `CellClass::GetGroundHeight` before setting bullet coords. | `0x00468915..0x00468931`. | HIGH | Conditional on helper hit. |
| Sentinel fallback final fire-time location is the target CoordStruct; an optional ordinary wall/cliff blocker-cell SetCoords is overwritten. | Stack reconstruction of `0x0046893C..0x00468986`; verified `ObjectClass::Set_Raw_Coords @ 0x005F6940` overwrites `+0x9C/+0xA0/+0xA4`; verified callee cleanup for the involved calls. | HIGH | Conditional on sentinel fallback. |
| Normal `InvisibleLow` GI AI detonation skips `BounceCheck`. | `BulletClass::AI @ 0x004666E0`: same-cell target branch sets detonation flags at `0x00467879..0x00467886`; later `0x00467BA4..0x00467BAA` skips `BounceCheck` when the flag is set. | HIGH | Yes for normal GI small-arms target hits. |
| Normal `InvisibleLow` GI AI detonation skips `ProximityDetector::Check`. | `0x00467C0C..0x00467C35` only calls `ProximityDetector::Check` when `ROT > 0` or `Ranged=yes`; standard `[InvisibleLow]` has neither. | HIGH | Yes for standard GI `InvisibleLow`. |
| `BulletDetonation` starts from `bullet.Location`. | `0x00468D8A..0x00468DAC`. | HIGH | Yes. |
| Close target can override detonation to target `GetCoords` if distance `< 0x20`, not Airburst, not Inaccurate. | `0x00468DE3..0x00468E8B`. | HIGH | Conditional, common for target hits. |
| vtable `+0xA4` is `GetCoords_OutputParam`, not direct damage. | `0x0041BDD0` calls `vtable+0x48` then copies X/Y/Z to caller output. | HIGH | Yes. |
| `FUN_005F6360` subtracts building foundation `(height+width)*0x40` from 3D distance and clamps below 0. | Live decompile `0x005F6360`. | HIGH | Conditional on building target. |
| `BulletClass::Detonate` randomizes visible impact anim CoordStruct for `Inviso=yes` by radius `0x20`. | `0x00469AA4` tests `BulletType+0x29E`; calls `FUN_0049F420(0x20,0)`. | HIGH | Yes for GI. |
| The `Inviso` RNG draw occurs before explosion-animation selection, so an empty/null AnimList result does not suppress the draw. | `0x00469AA4..0x00469AF0` precedes `Warhead__SelectExplosionAnim` at `0x00469BA2..0x00469BD4`. | HIGH | Yes. |
| `FUN_0049F420` preserves Z and randomizes X/Y using one random byte, binary sine-table samples, and toward-zero `ftol`: `x += cos`, `y -= sin`. | Live disassembly `0x0049F420`, `0x004CACB0`, `0x004CAD00`, `0x007C5F00`; table/control-word memory reads above. | HIGH | Yes. |
| Inviso visual randomization does not snap to cell center. | Caller passes snap bool 0; snap branch is skipped. | HIGH | Yes. |
| The current recovery working tree implements the narrow visible radius-`0x20` scatter and its Scenario-RNG consumption; the wider BulletClass detonation-coordinate pipeline remains incomplete and UNCHECKED. | Current Rust scan + binary findings above; exhaustive 256-byte scatter oracle and combat integration tests. | HIGH | Yes. |

## 8. Open Questions

1. Resolved 2026-05-17: the exact final `BulletClass::Fire` fallback CoordStruct is the target CoordStruct. The ordinary `FUN_004CC100` wall/cliff blocker cell can be written transiently at `0x00468966`, but final `Set_Raw_Coords(&target_coord)` at `0x00468986` wins.
2. `CellClass::GetGroundHeight @ 0x00578080` clearly derives a cell from X/Y and calls `0x0047B3A0`, but the inner function is table-heavy and not fully decoded here. Bridge-deck vs ground-only Z remains best handled by the existing bridge reports plus a targeted bridge shot trace.
3. Resolved 2026-05-17: `RulesClass+0x1850` is `[WallModel] AlliedWallTransparency`, default false, `no` in `rules.ini` and `rulesmd.ini`.
4. Resolved 2026-05-17 for normal `InvisibleLow` GI target hits: `BulletClass::AI @ 0x004666E0` sets the detonation flag on the same-cell target branch before `BounceCheck`, then skips `BounceCheck`; `ProximityDetector::Check` is also skipped because `[InvisibleLow]` has `ROT=0` and `Ranged=no`.
5. Non-GI `Inviso=yes` projectiles such as `FlakProj`, `InvisibleMedium`, `InvisibleHigh`, `InvisibleAll`, `Psychic`, and Tesla/comet fragments share the visual randomization but differ in flags; do not blindly apply `InvisibleLow` wall/cliff assumptions to them.

## Sources

- Live Ghidra decompiled/disassembled:
  - `0x006FDD50` `TechnoClass::Fire_At`
  - `0x0046B050` `BulletClass::Allocate`
  - `0x004664C0` `BulletClass::Init`
  - `0x00468670` `BulletClass::Fire`
  - `0x00467879..0x0046788B` `BulletClass::AI` same-cell target detonation flag
  - `0x00467BA4..0x00467BAA` `BulletClass::AI` BounceCheck skip gate
  - `0x00467C0C..0x00467C35` `BulletClass::AI` ProximityDetector gate
  - `0x0046893C..0x00468986` `BulletClass::Fire` sentinel fallback SetCoords ordering
  - `0x005880A0` Inviso line helper
  - `0x00468D80` `BulletClass::BulletDetonation`
  - `0x004690B0` `BulletClass::Detonate`
  - `0x0041BDD0` `GetCoords_OutputParam`
  - `0x0049F420` randomized CoordStruct helper
  - `0x004CC100` subject-to-cliffs/walls line helper
  - `0x004CC360` cliff/wall predicate helper
  - `0x00487D50` `CellClass::GetEffectiveHeight`
  - `0x0066D1F0` `RulesClass::ReadWallModel`
  - `0x00468BB0` BounceCheck
  - `0x004E1130` `ProximityDetector::Set`
  - `0x004E11F0` `ProximityDetector::Check`
  - `0x005F6360` bullet-target distance helper
  - `0x005F65A0` `ObjectClass::GetCoords`
  - `0x005F6940` `ObjectClass::Set_Raw_Coords`
  - `0x00578080` `CellClass::GetGroundHeight`
  - `0x0047C520` `Look_up_building_in_cell`
  - `0x005657A0` `MapClass::Get_CellClass`
- Repo INI checked:
  - `ini/rulesmd.ini`
  - `ini/rules.ini`
  - `ini/artmd.ini`
  - `ini/art.ini`
- Existing reports referenced:
  - `BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md`
  - `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`
  - `BULLET_CLASS_AI_GHIDRA_REPORT.md`
  - `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md`
  - `WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md`
  - `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
  - `BRIDGE_OBJECT_ONBRIDGE_EXTRA_WRITERS_GHIDRA_REPORT.md`
  - `UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md`
  - `TECHNOCLASS_INRANGE_DISTANCE_GHIDRA_REPORT.md`
- In-repo docs referenced:
  - `docs/fidelity-checks/2026-05-17-gi-small-arms-warhead-impact-placement.md`
  - `docs/plans/2026-05-17-invisiblelow-detonation-coord-investigation-plan.md`
