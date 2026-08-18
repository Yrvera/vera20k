# BuildingClass::ReleaseDockedHarvester (0x004595C0) — Ghidra Decompilation Report

**Target:** `BuildingClass::ReleaseDockedHarvester` at 0x004595C0
**Date:** 2026-05-19 (initial), 2026-05-20 (disputed-claim audit + resolution)
**Confidence:** HIGH — body, anim slots, VOC, Force_Track, dock teardown, and
the queue-anchor formula all verified directly in the binary. The visible
"exit-east-of-pad" behaviour is downstream of this function (Mission_Harvest
case 0 SCAN clears and re-targets the destination), not within it.
**Caller:** `UnitClass::Mission_Deploy_Building` at 0x0073D630, on the
nonzero reciprocal-link branch. Later branch work shows this is not the normal
stock zero-link `CMIN/HARV -> GAREFN/NAREFN` DockUnload completion path.

---

> **Correction 2026-05-21 - reachability refinement**
>
> The body analysis below remains valid for `ReleaseDockedHarvester` when it is
> reached. `MISSION_DEPLOY_BUILDING_DOCKED_VS_UNDOCKED_BRANCH_GHIDRA_REPORT.md`
> and `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md`
> supersede this report's earlier "fires every ore delivery" framing. Stock
> refinery DockUnload normally keeps `unit+0x2E4 == 0` and exits through
> `Mission_Deploy_Building` state 4; the `ReleaseDockedHarvester` body is a
> conditional reciprocal-link release helper, not the standard stock
> dump-complete exit.

## 0. ✅ RESOLVED — Queue anchor formula reconciliation (Step 10)

**Status: 🟢 VERIFIED 2026-05-20** — the binary formula stands as decompiled.
The player-visible "exit through queue cell" behaviour is produced downstream
by Mission_Harvest case 0 SCAN, not by Step 10 itself.

### What the binary does (verified)

For a 4×3 GAREFN at cell (10, 10), pad (13, 11):

1. **vtable_BuildingClass + 0x1B8 = `ObjectClass__Get_Cell_Packed` at
   `0x0041BEA0`.** Verified via `read_memory` at vtable base
   `0x007E3EBC` + 0x1B8 = `0x007E4074` → bytes `a0be4100`.
2. **`BuildingClass::Location` (+0x9C / +0xA0) stores the NW-corner cell
   origin in leptons.** Verified via `BuildingClass::GetCoords`
   (`vtable+0x48` → `0x00447AC0`) which computes
   `coord.x = Location.x + (width-1)*128` /
   `coord.y = Location.y + (height-1)*128`. For a 4×3 GAREFN, that gives
   the geometric center at leptons `(NW.x + 3*128, NW.y + 2*128)` —
   which only matches the foundation center if `Location.x/256 = NW.x`.
3. **Step 10 anchor is literally `(NW.x - 1, NW.y + 1)`.** Verified via
   `decompile_function 0x004595C0`:
   ```
   psVar6 = vtable+0x1B8(building);      // Get_Cell_Packed → NW cell
   uStack_40 = CONCAT22(psVar6[1] + 1,   // anchor.y = NW.y + 1
                        *psVar6 + -1);   // anchor.x = NW.x - 1
   ```
   For GAREFN at (10, 10): anchor = **(9, 11)**, one cell west of the
   foundation. `FootClass::Find_Nearby_Passable_Cell` spirals from here
   and picks a cell west/south of the foundation.

### Why the player still sees an east-of-pad exit

After Step 10 writes `unit.NavCom = passable_cell_near_(9, 11)` and
Step 12 sets `unit.mission = MOVE`, the next mission cycle re-enters
`Mission_Harvest`. **Case 0 SCAN immediately overrides Step 10's
destination**:

```c
case 0:
  ...
  if (param_1[0x86] != 0) {                          // last harvest cell present
    (**(code **)(*param_1 + 0x480))(param_1[0x86], 1);  // Set_Destination(last_target, 1)
    ...
  }
  ...
  FootClass__Search_For_Tiberium_And_Move(...);       // re-target to ore
```

Verified via `decompile_function 0x0073E5E0` (UnitClass::Mission_Harvest).
The cell from Step 10 is **never a visible waypoint** — the unit's NavCom
is overwritten before any frame is drawn, and what the player sees is the
miner driving from pad (13, 11) toward an ore cell. Since (14, 11) is the
ONLY passable cell adjacent to the pad (foundation occupies (10..14,
10..13) with `RemoveOccupy` removing only the pad cell itself), every
visible exit passes through the queue cell.

### Other paths checked

- `BuildingClass::Receive_Radio` case 0x0E (refinery handshake at
  `0x0043C2D0`) computes a destination at `(NW.x + 3, NW.y + 1)` — the
  pad cell, used for INBOUND queueing. Not invoked on the outbound exit
  path. Verified via `decompile_function 0x0043C2D0`.
- Force_Track 0x47 from Step 8 lays a sub-cell ESE drive-track curve
  centred at building center − (0x80, −0x80) leptons. Its endpoint lies
  near the foundation interior, not east of the pad, so it does NOT
  produce the east-of-pad exit on its own.

### Implication for the Rust port

The parity bar is on observable output. The Rust port collapses Step 10
+ Mission_Harvest case 0 into a single behaviour: drive from pad directly
to the queue cell, then `SearchOre` takes over. The binary's `(9, 11)`
anchor never appears as a waypoint, so reproducing it would only delay
the visible motion without changing what the player sees. See
`refinery_exit_cell` in [src/sim/miner/miner_dock_sequence.rs](../ra2-rust-game/src/sim/miner/miner_dock_sequence.rs)
— its anchor is the queue cell, and the spiral ring-1 fallback handles
the case where the queue cell is occupied by another waiting miner.

Per the user's "exit-east-of-pad" observation, the doc's prior
"south-west exit" framing was the wrong rock to look under, because it
treated Step 10's anchor as the visible exit destination rather than as
a vestigial placeholder overwritten by Mission_Harvest case 0.

---

## 1. Role and Caller Context

> **Correction 2026-05-21:** This section's original "normal post-unload"
> framing is superseded. `ReleaseDockedHarvester` is reached from the nonzero
> reciprocal-link branch, not from the normal stock zero-link GAREFN/NAREFN
> DockUnload completion path.

`ReleaseDockedHarvester` is a **conditional reciprocal-link release helper** for a refinery. It fires when
the nonzero reciprocal-link branch reaches it. It is not the standard stock
high-frequency path — every ore delivery triggers it.

It is NOT called on sell, destruction, or chrono-wipe. Those paths use
`BuildingClass::UndockUnit` (0x4593A0), documented separately in
`BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`.

**Caller:** `UnitClass::Mission_Deploy_Building` (0x0073D630) calls this function when
`param_1[0xb9] != 0` (= harvester is docked at building) and `iVar3 != 0` (building found in cell),
then jumps to label `LAB_0073d672` for the standard post-exit path. Verified via
`decompile_function 0x0073D630`.

---

## 2. Verified Step-by-Step Execution

**Signature:** `void __fastcall BuildingClass__ReleaseDockedHarvester(BuildingClass *param_1)`
- `param_1` is `BuildingClass*` (direct struct pointer, NOT int-array)
- `param_1->field_0x2e4` = docked-unit pointer (building side of dock-link)
- `piVar1` = the docked unit pointer read from `param_1->field_0x2e4`

### Step 1: Clear anim slots 0xA and 0xB (unloading anim teardown)
```
BuildingClass__ClearAnimSlot(param_1, slot=0xA)  // returns address 0x4595d0
BuildingClass__ClearAnimSlot(param_1, slot=0xB)  // returns address 0x4595d9
```
- Calls `BuildingClass__ClearAnimSlot` (0x00451E40) twice — slots 10 and 11.
- These are the active unloading-pipe/active-dock animation slots.
- `ClearAnimSlot`: if the slot's AnimClass* is non-null, zeroes it and calls
  `AnimClass::vtable+0x20(1)` (destructor/kill with force=1).
- Verified via `decompile_function 0x00451E40`.

### Step 2: Play VOC at building location (departure sound)
```c
if (*(int *)(g_RulesClass_Instance + 0x244) != -1) {
    VocClass__PlayAt(rules+0x244, building.Location, 0);
}
```
- **VOC key: `BunkerWallsDownSound`** at `RulesClass+0x244` (byte offset).
- Verified: `RulesClass::ReadAudioVisual` (0x00669758) writes `param_1[0x91]` = byte offset
  `0x91 × 4 = 0x244` via the string key `s_BunkerWallsDownSound_0083a810`. In `rulesmd.ini`:
  `BunkerWallsDownSound= TankBunkerDown`. Verified via `decompile_function 0x00669758` +
  `search_strings BunkerWallsDown` + INI grep.
- Guard: -1 = "no VOC configured"; if so, no sound plays. Standard for optional VOC fields.
- `VocClass__PlayAt` (0x007509E0): takes (voc_id, coord_ptr, loop_flag). Verified via
  `decompile_function 0x007509E0`.

### Step 3: Create anim slot 0xC (departure anim, undamaged side)
```c
health_ratio = ObjectClass__GetHealthRatio(param_1)
if (health_ratio > rules.ConditionYellow) {
    anim_name_ptr = &param_1->Type->field_0x127C;   // slot 12 healthy name (+0x00)
} else {
    anim_name_ptr = &param_1->Type->field_0x128C;   // slot 12 damaged name (+0x10)
}
if (anim_name_ptr != null && *anim_name_ptr != '\0') {
    BuildingClass__CreateAnimForSlot(param_1, slot=0xC, anim_name_ptr, is_damaged)
}
```
- **BuildingTypeClass anim slot 12** (base 0x127C) = `SpecialAnimThree` INI key prefix.
  - `+0x00` (offset 0x127C) = healthy anim name (`SpecialAnimThree=`)
  - `+0x10` (offset 0x128C) = damaged anim name (`SpecialAnimThreeDamaged=`)
- Verified via `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` slot index table (idx 12 → base 0x127C).
- `ConditionYellow` = `RulesClass+0x1700` (double). Verified from `ReadAudioVisual` which reads
  `s_ConditionYellow_0083a370` into `param_1[0x5c0/0x5c1]` = 8-byte double at 0x1700.

### Step 4: Create anim slot 0xD (departure anim, second slot)
```c
health_ratio = ObjectClass__GetHealthRatio(param_1)   // re-checked
if (health_ratio > rules.ConditionYellow) {
    anim_name_ptr = &param_1->Type->field_0x12C0;   // slot 13 healthy name
} else {
    anim_name_ptr = &param_1->Type->field_0x12D0;   // slot 13 damaged name (+0x10)
}
if (anim_name_ptr != null && *anim_name_ptr != '\0') {
    BuildingClass__CreateAnimForSlot(param_1, slot=0xD, anim_name_ptr, is_damaged)
}
```
- **BuildingTypeClass anim slot 13** (base 0x12C0) = `SpecialAnimFour` INI key prefix.
  - `+0x00` (offset 0x12C0) = healthy anim name (`SpecialAnimFour=`)
  - `+0x10` (offset 0x12D0) = damaged anim name (`SpecialAnimFourDamaged=`)
- Verified via `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` (idx 13 → base 0x12C0).
- Health ratio is re-evaluated independently (two separate `GetHealthRatio` calls in binary).
- `BuildingClass__CreateAnimForSlot` (0x00451890): takes building*, slot_index, anim_name_ptr,
  is_damaged. Allocates `AnimClass` via `operator_new(0x1c8)` and stores in
  `building->Anims_0[slot]`. Verified via `decompile_function 0x00451890`.

### Step 5: Early-exit guard — check docked unit pointer
```c
piVar1 = *(int **)&param_1->field_0x2e4;
if (piVar1 == null) {
    param_1->field_0x718 = 0;          // clear building's +0x718 field
    (*building_vtable + 0x1e8)(5, 0);  // SetMission(5) on building
    return;
}
```
- If `field_0x2e4` is null at this point (no docked unit), the function clears `+0x718`
  (a state/counter field on BuildingClass), sets the building's mission to 5, and returns
  without any locomotion commands.
- Mission 5 = SLEEP or GUARD (building-side). Building returns to idle mission.

### Step 6: Locomotion type guard
```c
iVar4 = (*piVar1->vtable + 0x2C)();   // loco type query on unit
if (iVar4 != 1) return;               // only proceed for DriveLocomotion (type=1)
```
- Guard identical to `UndockUnit`. Proceeds only if active loco is DriveLocomotionClass.
- For chrono miner: TeleportLoco is piggybacked under DriveLoco; DriveLoco is active, returns 1.

### Step 7: Power_On the locomotor
```c
assert(piVar1[0x19d] != 0)   // unit->active_loco (byte offset 0x674) non-null assert
(*loco_vtable + 0x58)(loco)  // ILocomotion::Power_On
```
- vtable slot +0x58 = `ILocomotion::Power_On`. Identical to `UndockUnit`.
- Verified same slot usage in `BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md`.

### Step 8: Head_To with exit track index and offset coordinates
```c
piVar5 = (*building_vtable + 0x48)(building);  // GetCoords → {x, y, z}
(*loco_vtable + 0x70)(loco, 0x47, x - 0x80, y + 0x80, z);  // Force_Track
```
- Track index **0x47** (decimal 71) = hardcoded literal. Same as `UndockUnit`.
- Offsets: **−0x80 leptons in X** (−0.5 cells west), **+0x80 leptons in Y** (+0.5 cells south).
- These are hardcoded constants. The Ghidra plate comment on the function states:
  "For a 4×3 refinery anchored at cell (rx, ry), this resolves to the south bib row."
- Verified from decompile: `pcStack_3c = (char *)(iVar4 + -0x80)` and `iStack_38 = iVar2 + 0x80`.

### Step 9: SetSpeedMultiplier(1.0)
```c
(*unit_vtable + 0x544)(0, 0x3FF00000)  // speed = 1.0 (IEEE 754 double 1.0)
```
- Identical to `UndockUnit`. Restores full speed.

### Step 10: Compute queue anchor cell and find passable cell
```c
psVar6 = (*building_vtable + 0x1b8)(building);  // GetCellLocation → (cell_x, cell_y) as shorts
anchor.x = psVar6[0] - 1;   // one cell west of building's NW corner cell
anchor.y = psVar6[1] + 1;   // one cell south

// Pass the unit's locomotor pointer to Find_Nearby_Passable_Cell for passability check
iVar4 = (*unit_vtable + 0x84)();   // get loco
dest = FootClass__Find_Nearby_Passable_Cell(
    &piVar1_coord_buf, &anchor,
    *(undefined4 *)(iVar4 + 0x67c),   // unit's WaterBound flag
    0xFFFFFFFF,   // max_radius (unlimited)
    0, 0, 1, 1, 0, 0, 0, 1,
    &anchor, 0, 0
)
dest_cell_obj = MapClass__Get_CellClass(dest)
```
- Uses `FootClass::Find_Nearby_Passable_Cell` (0x0056DC20) — the same function used in
  `UnitClass::Mission_Deploy_Building` state 0 for initial approach. Verified via
  `decompile_function 0x0056DC20` and confirmed label in `get_function_callees`.
- ✅ **Queue anchor formula resolved — see §0.** Anchor is literally
  `(NW.x - 1, NW.y + 1)` = `(9, 11)` for GAREFN at (10, 10). The cell the
  spiral selects from this anchor is overwritten by `Mission_Harvest` case 0
  SCAN before any frame draws, so it is never the visible exit destination
  the player sees.
- vtable slot +0x1b8 = `ObjectClass__Get_Cell_Packed` at 0x0041BEA0.
  Verified via `read_memory` at vtable_BuildingClass base
  `0x007E3EBC` + 0x1B8 = `0x007E4074` → `a0be4100`. The function divides
  `Location` (+0x9C / +0xA0) by 256 with arithmetic-shift semantics.
  Confirmed (see §0) that `Location` stores the NW-corner cell origin in
  leptons by cross-checking `BuildingClass::GetCoords` (+0x48 → `0x00447AC0`)
  which adds `(width-1)*128` / `(height-1)*128` to recover the geometric
  centre.
- `iVar4 + 0x67c` = `TechnoTypeClass::WaterBound` bool (verified from `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`
  which notes WaterBound at offset 0x67C).

### Step 11: Set_Destination to found passable cell
```c
(*unit_vtable + 0x480)(dest_cell_obj, 1)  // FootClass::Set_Destination(cell, 1)
```
- vtable slot +0x480 = `FootClass::Set_Destination`.
- The Ghidra plate comment on the function notes an important side-effect:
  For the chrono miner (TeleportLoco piggybacked): `Set_Destination` attempts to unpiggyback
  DriveLoco→TeleportLoco via `IPiggyback::Is_Ok_To_End`, but fails because `Force_Track` in
  step 8 left `Is_Moving=true`. Falls back to `loco->Stop_Moving` (slot +0x48) + `unit->Stop_Moving`
  + `SetMission(ENTER=7)` + writes two unit flags. Then calls `loco->Set_Destination` (slot +0x44)
  with the dest_cell coord.

### Step 12: SetMission(MOVE=2) on unit
```c
(*unit_vtable + 0x1e8)(2, 0)  // SetMission(MOVE=2)
```
- vtable slot +0x1e8 = `SetMission`. Mission 2 = MOVE.
- Overrides any fallback SetMission(ENTER=7) written by Set_Destination's fallback path.
- Final unit mission: **MOVE** with NavCom = queue passable cell.

### Step 13: Dock teardown — clear both sides
```c
*(undefined4 *)&param_1->field_0x2e4 = 0;   // building side: clear building's dock-link
*(undefined4 *)&param_1->field_0x718 = 0;   // building's +0x718 state field
(*building_vtable + 0x1e8)(5, 0);           // building SetMission(5) = back to idle
(*building_vtable + 0x274)(3);              // RadioCommand(CLEAR=3)
```
- Note: `piVar1[0xb9] = 0` (unit side of dock-link at unit+0x2E4) is cleared at the start
  of the locomotion block (step 6), BEFORE the locomotion commands.
  The building side (`param_1->field_0x2e4`) is cleared HERE at step 13, after SetMission(MOVE).
  This asymmetry differs from `UndockUnit` where both sides are cleared together.
- `RadioCommand(CLEAR=3)`: notifies production system that this dock is free.
  vtable +0x274 = RadioCommand. Arg 3 = CLEAR.

---

## 3. Key Differences vs. UndockUnit (0x4593A0)

| Aspect | ReleaseDockedHarvester | UndockUnit |
|--------|----------------------|------------|
| Trigger | Conditional nonzero reciprocal-link release | Destruction/Sell/Chrono-wipe only |
| Anim slots cleared | Slots 0xA and 0xB (teardown) | None |
| VOC played | `BunkerWallsDownSound` (RulesClass+0x244) | None |
| Anims created | Slots 0xC and 0xD (SpecialAnimThree/Four) | None |
| Passable-cell search | Yes — anchor (NW.x-1, NW.y+1); overwritten by Mission_Harvest — see §0 | No |
| Set_Destination | Yes (FootClass::Set_Destination slot +0x480) | No |
| SetMission on unit | MOVE=2 | None |
| Building mission reset | SetMission(5) + RadioCommand(CLEAR=3) | RadioCommand(CLEAR=3) only |
| Unit dock-link clear | piVar1[0xb9]=0 at step 6 (before loco cmds) | piVar1[0xb9]=0 at step 9 (after) |

---

## 4. Chrono Miner vs. Regular Harvester

**Zero Teleporter branches.** Verified: no `unit->type->Teleporter`, no `+0xCD4`, no chrono-miner-
specific conditional in the entire function body. The function is identical for both vehicle types.
This matches the existing `MISSION_DEPLOY_BUILDING_0x73D630_STATE_MACHINE_GHIDRA_REPORT.md`
finding of zero Teleporter branches in the unload path.

The TeleportLoco swap-back (DriveLoco → TeleportLoco piggyback) is handled upstream in
`FootClass::AI` (0x4DA530) after the miner departs, not here.

---

## 5. Caller Post-Call Work (UnitClass::Mission_Deploy_Building)

After `ReleaseDockedHarvester` returns, `Mission_Deploy_Building` jumps to `LAB_0073d672` which:
1. Reads the building's `BuildingTypeClass` for `UnitTypeClass` fields at `+0xe0e`/`+0xe0f`/`+0xe13`.
2. Checks `+0x404` (likely OreStorage or capacity). If zero, branches through a deploy-type check.
3. In the normal path (ore refinery), calls `thunk_FUN_005b2ef0` and returns a tick delay.

The tick delay computation uses `MissionClass__GetMissionTimerEntry` + `Math__ftol` + `Random__RandomRanged(0,2)`, giving a 1–3 tick random jitter on the next Mission_Deploy_Building call.

---

## 6. Five Load-Bearing Verified Facts

1. **Anim slots 0xA+0xB cleared, 0xC+0xD created**: `BuildingClass__ClearAnimSlot` (0x00451E40)
   twice then `BuildingClass__CreateAnimForSlot` (0x00451890) up to twice. Slot 0xC =
   `SpecialAnimThree` (BuildingTypeClass+0x127C), slot 0xD = `SpecialAnimFour`
   (BuildingTypeClass+0x12C0). Healthy vs. damaged variant selected by `ConditionYellow` threshold.
   Verified via `decompile_function 0x004595C0` + `decompile_function 0x00451890` +
   `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`.

2. **Departure VOC = `BunkerWallsDownSound` (RulesClass+0x244)**: `param_1[0x91]` in
   `RulesClass__ReadAudioVisual` (0x00669758) writes `BunkerWallsDownSound` to byte offset 0x244.
   In `rulesmd.ini`: `BunkerWallsDownSound= TankBunkerDown`. Played via `VocClass__PlayAt`
   (0x007509E0) at building location with loop_flag=0.
   Verified via `decompile_function 0x00669758` + INI grep.

3. **Queue anchor formula — ✅ VERIFIED, see §0.** The decompile body calls
   `FootClass::Find_Nearby_Passable_Cell` (0x0056DC20) with anchor
   `(NW.x - 1, NW.y + 1)`. For GAREFN at (10, 10): anchor (9, 11).
   `BuildingClass::Location` stores NW-corner cell origin in leptons (verified
   via `BuildingClass::GetCoords` at `0x00447AC0` which adds `(width-1)*128` /
   `(height-1)*128` to recover the foundation centre — only consistent with
   `Location/256 = NW`). The anchor-derived destination is overwritten by
   `Mission_Harvest` case 0 SCAN before becoming visible, so the observable
   exit-through-queue-cell behaviour is consistent with the binary even though
   Step 10 itself targets a SW cell. Verified via `decompile_function 0x004595C0`,
   `decompile_function 0x00447AC0`, `decompile_function 0x0073E5E0`, and
   `read_memory 0x007E4074`.

4. **SetMission(MOVE=2) overrides Set_Destination's fallback SetMission(ENTER=7)**:
   `(*unit_vtable + 0x480)(dest, 1)` then `(*unit_vtable + 0x1e8)(2, 0)`. The explicit
   MOVE=2 follows immediately after Set_Destination, ensuring the unit departs rather than
   re-entering the building. Verified from decompile.

5. **No Teleporter branch — identical path for chrono miner and regular harvester**:
   The entire function body contains zero checks of `unit->type->Teleporter` or `unit+0xCD4`.
   Verified by full decompile read. Consistent with `MISSION_DEPLOY_BUILDING_0x73D630`
   finding of zero Teleporter branches in the unload path.

---

## 7. Open Items / Unverified

- **`param_1->field_0x718` semantics**: cleared in two places (early-exit guard and step 13).
  Likely a dock-state counter or "unloading-in-progress" flag on BuildingClass. Offset 0x718
  confirmed but field semantic not verified against a named field in existing docs.
- **`unit[0xb9] = 0` timing**: the Ghidra decompile shows this at `piVar1[0xb9] = 0` inside
  the locomotion-type guard (step 6), before `Power_On`. This differs slightly from `UndockUnit`
  where it appears after `Head_To`. Not a behavioral difference (both happen before the unit
  moves) but worth noting for Rust port accuracy.
- **`vtable + 0x84` identity**: called to retrieve the locomotor for `WaterBound` extraction.
  Label not verified against a known vtable slot name in this session.

---

**Status: COMPLETE**
