# Low Bridge TubeClass Doc Verification

Date: 2026-05-16

Scope: live Ghidra verification of low-bridge / TubeClass claims in:

- `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md`
- `BRIDGE_SYSTEM.md`
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` sections 14.17, 15.5, and TubeClass / IsLowBridgeCell / GetTubeAtCell references

No Rust code was inspected or changed.

## Summary verdict

The binary confirms that TubeClass is live for standard YR low-bridge/tunnel pathing, but the current docs mix two different TubeClass creation paths and overstate the "one tube per bridge" conclusion.

The load-bearing correction is:

- `CellClass::RecalcAttributes` creates a TubeClass shell for a qualifying `LandType == 10` cell with entry and exit both equal to that cell.
- The `[Tubes]` INI parser creates fully initialized TubeClass records with separate entry, exit, direction, step buffer, and step count.
- `MapClass::ComputeBridgeZones` reads low-bridge tube data, including `tube+0x28`; it does not fill `tube+0x28`.

Therefore, the later claim in `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 15.5 that `ComputeBridgeZones` fills the exit coord is wrong. The unqualified claim "one tube per bridge, not one per cell" is not supported by the verified RecalcAttributes path. The constructor itself is same-cell / zero-length until some caller overwrites the fields.

Doc status for this topic: YELLOW overall, with a RED correction required for section 15.5 of `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`.

## Confirmed claims

1. `CellClass::IsLowBridgeCell @ 0x00484AB0` is correctly identified.

   Binary:

   ```c
   return 0 <= *(short *)(cell + 0x116)
       && *(short *)(cell + 0x116) < DAT_008b4148
       && *(int *)(cell + 0xec) == 10;
   ```

   It checks a valid signed 16-bit tube index and `LandType == 10`. It does not check overlay ID directly.

2. `CellClass::GetTubeAtCell @ 0x00484F20` is correctly identified.

   Binary:

   ```c
   idx = *(short *)(cell + 0x116);
   if (0 <= idx && idx < DAT_008b4148) {
       return *(void **)(g_TubeArray + idx * 4);
   }
   return 0;
   ```

   Important nuance: this function bounds-checks only the tube index. It does not check `LandType == 10`.

3. `TubeClass::Constructor @ 0x00727FD0` is correctly identified and initializes a same-cell shell.

   Binary writes:

   - `tube+0x24 = *(int*)entry_coord`
   - `tube+0x28 = *(int*)entry_coord`
   - `tube+0x2C = param_3`
   - `tube+0x30..+0x1BF = 100 dword entries of -1`
   - `tube+0x1C0 = 0`

   It appends the tube to `g_TubeArray`, then writes this tube index to `entry_cell+0x116` when the entry coord is not `(0,0)`.

4. `CellClass::RecalcAttributes @ 0x0047D2B0`, call site `0x0047D940`, constructs TubeClass shells for qualifying cells.

   The low/tunnel branch requires:

   - `LandType == 10`
   - `cell+0x116` invalid or out of range
   - `IsoTileTypeIndex` inside one of four exact 4-tile ranges:
     - `DAT_00aa1054..DAT_00aa1054+3`
     - `DAT_00abb108..DAT_00abb108+3`
     - `DAT_00aa10b4..DAT_00aa10b4+3`
     - `DAT_00abad2c..DAT_00abad2c+3`
   - allocation of `0x1C4` bytes succeeds

   Assembly at `0x0047D935` loads the direction from `DAT_0081CC20[index]`, then calls `TubeClass::Constructor` at `0x0047D940`. Live memory at `0x0081CC20` is `[2, 4, 6, 0]`.

5. The `[Tubes]` parser at `FUN_007283C0` is a separate complete initialization path.

   For each `[Tubes]` INI entry, it constructs a TubeClass, then overwrites:

   - entry X/Y at `+0x24/+0x26`
   - direction or Z/direction field at `+0x2C`
   - exit X/Y at `+0x28/+0x2A`
   - path directions at `+0x30...`
   - step count at `+0x1C0`

   It writes the tube index to the entry cell's `+0x116`.

6. `MapClass::ComputeBridgeZones @ 0x0056D6E0` creates both high and low bridge records.

   The low branch:

   - only runs when the current cell is not `IsBridge` and not `IsWoodBridge`
   - requires `IsLowBridgeCell(current)`
   - checks opposite low-bridge neighbors in direction pairs `2/6` or `4/0`
   - calls `GetTubeAtCell(current)`
   - reads `tube+0x28`
   - compares linear cell order through `FUN_0042B1C0`
   - writes a 16-byte BridgeRecord with `+0x0C = 1`

7. `MapClass::UpdateBridgeZonesHelper @ 0x0056C510` does not filter on bridge kind when adding intact bridge records to the zone graph.

   The record loop checks `record+0x08 != 0` and uses endpoint A/B. The decompiled loop does not read `record+0x0C`.

8. `MapClass::FindBridgeRecord @ 0x0056DA10` filters out low bridge records.

   The first meaningful record test is `if (record+0x0C == 0)`. Low records written by `ComputeBridgeZones` have `+0x0C = 1`, so `FindBridgeRecord`, `ValidateBridgeZones`, and `InvalidateBridgeZones` operate on high records only.

9. `UnitClass::TubeMovement @ 0x007359F0` is live.

   Its only direct caller is `UnitClass::AI` at `0x007363B0`, reached when signed byte `UnitClass+0x684` is non-negative. The function uses `g_TubeArray[tube_index]`, `Unit+0x685` as a step cursor, tube step entries at `tube+0x30 + cursor*4`, entry/exit coords at `+0x24/+0x28`, and `tube+0x1C0` to animate/advance the unit through a tube. At tube exit it places the unit at `tube+0x28`, clears the tube index to `0xFF`, updates movement state, and adjusts facing from `tube+0x2C` when a tube is present at the final cell.

10. The direction-8 pathfinding sentinel is real, but it is not implemented through `GetTubeAtCell`.

    `PathfinderClass::UpdateBridgePassability @ 0x0042ACF0` checks path direction `8`, reads the current cell's `+0x116`, and then directly reads `g_TubeArray[idx]+0x28`.

## Incorrect or stale claims

1. `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 15.5 says: "One tube per bridge, not one per cell."

   Binary shows a more limited fact. `FUN_007283C0` creates one TubeClass per `[Tubes]` INI entry. `CellClass::RecalcAttributes` creates a TubeClass shell for the current qualifying `LandType == 10` cell and immediately writes that index to the current cell's `+0x116`. The RecalcAttributes path is not one-per-bridge in the verified code.

   Impact: a Rust plan that assumes all low bridges collapse to one long TubeClass record will miss the shell path and may model `cell+0x116` incorrectly.

2. `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` section 15.5 says: "`ComputeBridgeZones` (runs next via `FUN_00684C30`) fills the exit coord via endpoint walk."

   Binary shows this is wrong. At `0x0056D7C2`, `ComputeBridgeZones` calls `GetTubeAtCell(current)` and immediately reads `EAX+0x28`; at `0x0056D7ED` it calls `GetTubeAtCell(current)` again and reads `EAX+0x28` for the BridgeRecord endpoint. There is no write to `tube+0x28` in this function.

   Impact: this is the core contradiction. `ComputeBridgeZones` is a consumer of tube exit data, not the producer.

3. `BRIDGE_LOW_AND_ZONE_RECORDS_GHIDRA_SUPPLEMENT.md` says low bridge tubes are "zero-length / same-cell tube records" as the bottom-line model.

   This is confirmed for constructor-created RecalcAttributes shells, but incomplete for the `[Tubes]` parser path, which overwrites `+0x28`, `+0x30...`, and `+0x1C0` from INI data.

   Impact: correct for the constructor initial state; misleading if treated as the only live standard-YR TubeClass shape.

4. `BRIDGE_SYSTEM.md` low bridge water passability section says: "When placed: water cells get `cell+0x116` (tube_index) set to valid ID, and `cell+0xEC` changed from 2 (Water) to 10 (Tunnel)" and "When destroyed: LandType reverts to Water, tube_index cleared."

   The verified functions here confirm the `IsLowBridgeCell` predicate and the RecalcAttributes construction path, but they do not prove the broad "water cells" placement/destroyed lifecycle statement. In the verified RecalcAttributes path, `LandType == 10` is a precondition before the tube shell is created. The checked binary evidence does not show RecalcAttributes changing Water to Tunnel for low bridges.

   Impact: treat this as incomplete until the low-bridge destruction/repair tile mutators are separately audited for `+0xEC` and `+0x116` writes.

## Contradictions resolved

The apparent contradiction is resolved by separating constructor state from fully initialized tube records.

- Same-cell / zero-length is true for `TubeClass::Constructor` output and for RecalcAttributes-created shells unless another caller later overwrites fields.
- Separate entry/exit behavior is true for TubeClass records initialized by `[Tubes]` INI data.
- The verified binary does not support the claim that `ComputeBridgeZones` turns a RecalcAttributes shell into a long bridge tube.

The safest statement is:

> TubeClass records can be same-cell shells or fully initialized multi-cell tubes, depending on the creation path. Low-bridge zone construction consumes `tube+0x28`; it does not populate it.

## New verified binary facts

1. Live xrefs to `IsLowBridgeCell @ 0x00484AB0`:

   - `MapClass__ComputeBridgeZones`: `0x0056D75B`, `0x0056D773`, `0x0056D787`, `0x0056D79B`, `0x0056D7B3`
   - `FUN_00704000`: `0x007040D2`, `0x007040E2`, `0x007040F2`, `0x00704217`, `0x00704222`
   - `FUN_00484AE0`: `0x00484D29`, `0x00484D34`
   - `InfantryClass__What_Action_OnCell`: `0x0051F9B9`
   - `UnitClass__What_Action_OnCell`: `0x007406DC`
   - `FUN_00728280`: `0x007282CA`, plus an unlabeled call at `0x00728233`

2. Live xrefs to `GetTubeAtCell @ 0x00484F20`:

   - `FUN_00582D70`: `0x00582DF8`, `0x00582E8D`, `0x00582ECA`
   - `MapClass__ComputeBridgeZones`: `0x0056D7C2`, `0x0056D7ED`
   - `UnitClass__TubeMovement`: `0x0073607A`
   - `FUN_0051BF90`: `0x0051BFF2`, `0x0051C079`
   - `UnitClass__Can_Enter_Cell`: `0x0073F10D`, `0x0073F291`
   - `FootClass__ClickedAction_Cell`: `0x004D81B6`, `0x004D81D5`

3. Live xref to `UnitClass::TubeMovement @ 0x007359F0`:

   - `UnitClass__AI`: `0x007363B0`

4. Live xrefs to `TubeClass::Constructor @ 0x00727FD0`:

   - `CellClass__RecalcAttributes`: `0x0047D940`
   - `FUN_007283C0` (`[Tubes]` parser): `0x0072844C`
   - unlabeled COM/loading caller at `0x006C0156`

5. `FUN_00684C30` post-load ordering is:

   - iterate cells and call `CellClass__RecalcAttributes`
   - call `MapClass__ComputeBridgeZones`
   - call `MapClass__UpdateBridgeZonesHelper`

   This confirms the zone build consumes whatever tube state exists after RecalcAttributes and `[Tubes]` parsing, but it does not imply `ComputeBridgeZones` mutates TubeClass.

6. `FUN_00582D70` / `MapClass__AddBridgeZoneEdges` consume low tube data when the starting endpoint is not a high/wood bridge tile. They call `GetTubeAtCell`, use `tube+0x2C`, and walk `tube+0x30` for `tube+0x1C0` steps through `Path_walk_directions_to_cell`.

## Remaining open questions

1. Which retail/YR map-load path guarantees that low bridges have fully initialized `[Tubes]` records versus RecalcAttributes shells only?

   The binary parser and shell creator are verified, but this audit did not inspect actual retail map files or map-editor save output.

2. Which exact low-bridge tile families map to `DAT_00aa1054`, `DAT_00abb108`, `DAT_00aa10b4`, and `DAT_00abad2c` for each theater?

   The four ranges are verified as code predicates, but their theater key names were not re-verified in this pass.

3. Do low-bridge destruction/repair paths clear or restore `cell+0x116` and `LandType` in all cases?

   The current evidence verifies the pathing predicates and zone consumers, not the full damage lifecycle for tube indices.

4. Is `FUN_00704000` a low-bridge edge/topology helper used by normal unit action selection, and does it affect player commands beyond cursor/action classification?

   Its xrefs and low-bridge predicates are verified, but its semantic name remains unresolved.

## Implementation implications for Rust

1. Model `cell.tube_index` and `LandType == 10` as the low-bridge pathing predicate. Overlay ID alone is not enough.

2. Do not assume `GetTubeAtCell` checks land type. Callers that need "low bridge cell" semantics must apply the `IsLowBridgeCell` predicate.

3. Represent TubeClass as data with entry, exit, direction, step buffer, and step count. A tube may be a same-cell shell or a fully initialized multi-cell path depending on source.

4. Do not implement `ComputeBridgeZones` as a tube initializer. It should read `tube.exit` and create low BridgeRecords only when the low-neighbor and linear-order tests pass.

5. Preserve `BridgeRecord.bridge_kind`. Initial zone graph construction consumes intact records without filtering kind, but `FindBridgeRecord`/Validate/Invalidate are high-only because they skip `bridge_kind != 0`.

6. `UnitClass::TubeMovement` is player-visible. It controls tube traversal position, facing, exit placement, and blocked-exit behavior. A pathfinding-only tube shortcut is insufficient for visible parity.
