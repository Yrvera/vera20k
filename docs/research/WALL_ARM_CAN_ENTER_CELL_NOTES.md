# Wall arm of `Can_Enter_Cell` — research notes

**Status: research notes, not a report.** Facts below were read from `gamemd.exe`
(SHA-256 `1cdd1180…4298c`) on 2026-09-15 by disassembly and decompile; addresses
cite the read site. Nothing here is a parity claim. Consumed by ledger rows I9a
(crusher route) and I9b (open: codes 4/5, A* wall cost, wall-attack
Override) in `docs/plans/2026-09-15-movement-retail-acceptance.md`.

## Native facts gathered 2026-09-15

## UnitClass::Can_Enter_Cell 0x0073F0A0, wall arm 0x0073F3D0..F4F9 (disassembly + decompile)
- cell.OverlayTypeIndex (+0x44) == -1 -> skip arm.
- ot = OverlayTypes[idx]; ot+0x2AA nonzero && !HouseClass::IsControlledByHuman(owner) && g_GameMode == 0 -> return 7 (0x0073F3EC..F41D).
- ot.Wall (+0x2A8) zero -> skip arm (0x0073F428).
- Crusher route (0x0073F42E..F46C): ot.Crushable (+0x22D) && (type.Crusher +0xD28 || TechnoClass::HasWeaponAbility(0x11) 0x0070D0D0),
  OR (ot.Wall && type.MovementZone (+0x5B4) == 0xC CrusherAll)
  -> HouseClass::Is_Ally_ByIndex(cell wall owner +0x50) 0x004F9A10: ally -> code = max(code,4) (0x0073F4EB); not ally -> code unchanged (free entry).
- Weapon route (0x0073F483..F4E9): vtable +0x2AC (0x00701120: primary weapon slot +0x3F4 non-null) false -> return 7;
  Weapon(0) (+0x3F8) -> Warhead (+0xAC): Wall (+0x144) or (Wood (+0x147) && ot.Armor (+0x9C) == 6) else return 7;
  Is_Ally_ByIndex(wall owner): ally -> code = max(code,4); enemy -> code = max(code,5) (0x0073F50E per decompile).

## InfantryClass::Can_Enter_Cell 0x0051BF90, wall arm (decompile)
- ot+0x2AA (`Crate=`, written by `OverlayTypeClass::ReadINI 0x005FE770`; see `native_mark_overlay_data` in `src/rules/overlay_types.rs`) && !human -> 7.
- ot.Wall && (cell+0x11E >> 4) != ot+0x2A0 (`DamageLevels=` from the art section, parsed as `damage_levels`): the wall arm is skipped on a fully destroyed wall stage:
  +0x2AC false -> 7; Weapon(0) -> FUN_00772AC0 (warhead +0x144 Wall) false -> 7; code = 5 - Is_Ally_ByIndex(owner).
- No crusher route for infantry.

## A* consumption
- AStar_compute_edge_cost 0x00429830 indexes 0x0081870C [1,1000,1,1,60,20,8,10000]; codes 4/5 expand at 60x/20x.
- VERA: core.rs `apply_search_cost_class_multiplier` has the table; `search_cost_classifier` hook exists but no production site sets it;
  entity_block_map carries 2/5/6 per cell. Neighbour passability is a bool from `is_cell_passable_for_mover_with_speed`.

## Crossing / blocked response
- DriveLocomotionClass::Process_Movement 0x004B2630: code dispatch 0x004B36F4 (6 -> scatter arm), 0x004B3944 (1 -> re-enter), else shared entry
  0x004B3607 (null Head_To); 0x004B364D `CMP code,2` -> code-2 wait; else 0x004B3A97: code 5 or 4 -> 0x004B3AD3:
  [ESP+0x64] set -> drop path (+0x5E0=-1), start +0x640 timer, return; else 0x004B3B03 -> cell of the refused coord
  (0x005657A0 -> 0x0047C5A0) then 0x004F9A90(owner house, cell) ... (continues; read 0x004B3B5E on).
- Walk/Hover: Override arm wall case (movement_occupancy.rs notes 0x00515C9C for Hover); cell target needs a Restore path.

## VERA state today
- Crushable walls reduce to zone class CRUSHABLE (overlay_reduced_zone_type); path grid `overlay_blocks` only for WALL/IMPASSABLE;
  class arm tests `zone_type == WALL`; so sandbags/fences admit every mover with a passable speed row (documented in cell_entry.rs header, I8).
- Non-crushable walls: HardBlocked unless Destroyer/AmphibiousDestroyer/InfantryDestroyer/CrusherAll (cell_entry.rs class arm; cell_rect.rs).
- Inputs available: OverlayTypeFlags {wall, crushable, armor_is_wood}; OverlayCell {overlay_id, overlay_data, wall_owner}; ResolvedCell.overlay_id/overlay_zone_type;
  WarheadType {wall, wood}; GameEntity.regular_crusher; combat_weapon::primary_for_tier(obj, veterancy).
- Identified: ot+0x2AA = `Crate=`, ot+0x2A0 = `DamageLevels=`. Unmodelled: mover ability 0x11 (no stock grant).
- Stock wall-capable non-crushers (primary warhead `Wall=yes`): `[FV]` (`HoverMissile` -> `HE`), `[BRUTE]` (`Punch` -> `Battering`).

## Drive code-4/5 arm, continued (0x004B3B03..3BEF, disassembly)
- 0x004B3B03: cell = Get_CellClass(refused coord) (0x005657A0); CellClass::Find_Blocking_Object 0x0047C5A0(cell):
  - found: HouseClass::Is_Ally_ByObject 0x004F9A90(owner house, object): ally -> nothing (0x004B3BEF); not ally -> owner vtable +0x1F4 (1, object)
    = mission override with Attack (1) on the blocking object.
  - none (0x004B3B94): cell.OverlayTypeIndex != -1 && OverlayTypes[idx].Wall (+0x2A8) -> owner vtable +0x1F4 (1, cell) = override Attack on the wall cell.
- So a Drive mover refused with code 4/5 attacks the wall cell (or the enemy body) through the same Override slot Walk/Hover use; VERA's
  Override arm (movement_occupancy.rs) exists for Walk/Hover objects only and has no cell-target Restore path.
