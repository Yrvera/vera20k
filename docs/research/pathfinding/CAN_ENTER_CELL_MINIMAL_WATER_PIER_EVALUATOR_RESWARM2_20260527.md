# Can Enter Cell Minimal Water/Pier Evaluator Re-Swarm 2

Date: 2026-05-27  
Slot: 3 / second pathgrid re-swarm  
Scope: minimum active-YR `UnitClass::Can_Enter_Cell` / `CellClass::CheckCellPassability` evaluator slice needed to stop ordinary ground units from entering water, shore, pier-adjacent, bridge-adjacent, and tube/low-bridge cells without decoding the full occupant tree.

## Verdict

`PathGrid::is_walkable()` is not a valid substitute for native cell-entry legality.

The minimum native-shaped Rust evaluator for this bug class must include:

- target `CellClass::LandType` and mover `SpeedType`, checked through `SpeedType x LandType` zero-cost rejection;
- source/target bridge layer state: cell flags `0x100` and `0x200`, cell level byte, ramp/slope byte, and current requested bridge/elevation parameter;
- tube/low-bridge transition direction handling, especially direction `8`;
- chosen occupancy layer/list selector only enough to know whether the speed/land rejection is skipped for the alternate/high layer path;
- optional locomotor passability hook result when the native call-site requests it.

The full object-list tree can be deferred for a first terrain-legality slice, but exact `Can_Enter_Cell` result priority cannot be reproduced until it is implemented.

## Ghidra Evidence

### `UnitClass__Can_Enter_Cell @ 0x0073F0A0`

Decompile summary:

- The function returns native passability status codes, with `7` as hard impassable.
- Early terrain/bridge/tube checks can return `7` before any object-list traversal.
- The selected object list is traversed later through `CellClass + 0xe4` or `CellClass + 0xe8`.
- When the selected list is exhausted, a `SpeedType x LandType` table zero check can return `7`.

Assembly evidence for the post-object speed/land rejection:

```asm
0073fa8a: TEST ESI,ESI
0073fa8c: JNZ 0x0073f528
0073fa92: MOV AL,byte ptr [ESP + 0x13]
0073fa96: TEST AL,AL
0073fa98: JNZ 0x0073fc24
0073fa9e: MOV EAX,dword ptr [EDI + 0xec]
0073faa4: MOV EDX,dword ptr [EBX + 0x6c4]
0073faaa: LEA ECX,[EAX + EAX*0x8]
0073faad: MOV EAX,dword ptr [EDX + 0x67c]
0073fab3: ADD ECX,EAX
0073fab5: FLD float ptr [ECX*0x4 + 0x89ea40]
0073fabc: FCOMP float ptr [0x007e1748]
0073fac4: TEST AH,0x40
0073fac7: JZ 0x0073fc24
0073fad0: MOV EAX,0x7
0073fadc: RET 0x14
```

Interpretation:

- `CellClass + 0xec` is the target land type.
- `UnitTypeClass + 0x67c` is the mover speed type used as the second index.
- Indexing is `land_type * 9 + speed_type`.
- If the table value equals the zero constant at `0x007e1748`, `Can_Enter_Cell` returns `7`.
- This is the verified direct rejection that stops ordinary ground units from entering bare water when their speed row has zero movement on water land type.
- The check is skipped when `byte [ESP+0x13] != 0`, which is the alternate-layer/list path selected earlier in this function. Rust must not collapse all layers into a single `PathGrid` boolean.

### Terrain/bridge/tube gates before object traversal

`UnitClass__Can_Enter_Cell` performs bridge/tube checks before the object-list walk.

The call to the bridge/layer virtual happens before the list traversal:

```asm
0073f2d0: MOV EAX,dword ptr [EBX]
0073f2d2: LEA ECX,[ESP + 0x13]
0073f2d6: PUSH ESI
0073f2d7: LEA EDX,[ESP + 0xa0]
0073f2de: PUSH ECX
0073f2df: MOV ECX,dword ptr [ESP + 0x9c]
0073f2e6: PUSH EDX
0073f2e7: PUSH EDI
0073f2e8: PUSH ECX
0073f2e9: MOV ECX,EBX
0073f2eb: CALL dword ptr [EAX + 0x1b0]
0073f2f1: CMP EAX,0x7
0073f2f4: JNZ 0x0073f303
0073f300: RET 0x14
```

Direction `8` is special-cased before normal smoothing/path entry handling:

```asm
0073f218: CMP EDI,0x8
0073f21b: JNZ 0x0073f24b
0073f21d: TEST ESI,ESI
0073f21f: JZ 0x0073fcd0
0073f225: MOV ESI,dword ptr [ESI + 0x28]
0073f228: TEST SI,SI
0073f22f: JNZ 0x0073f23c
0073f231: CMP word ptr [ESP + 0x2a],SI
0073f236: JZ 0x0073fcd0
0073f23f: XOR EAX,EAX
0073f248: RET 0x14
```

Decompile interpretation:

- If direction is `8`, a tube must exist for the cell.
- The tube field at `+0x28` must not be an all-zero pair.
- Valid direction-`8` tube traversal returns `0` early.
- Missing or empty tube data returns hard block `7`.

For non-`8` directions, tube direction mismatch can hard-block before object traversal:

```asm
0073f24b: TEST ESI,ESI
0073f24d: JZ 0x0073f27c
0073f24f: MOV EDX,dword ptr [ESI + 0x2c]
0073f252: MOV EAX,EDI
0073f254: SUB EAX,EDX
0073f25b: CMP EAX,0x2
0073f25e: JLE 0x0073f27c
0073f260: CMP EAX,0x6
0073f263: JGE 0x0073f27c
0073f265: CMP EDI,-0x1
...
0073f2ba: MOV EAX,0x7
0073f2c6: RET 0x14
```

Minimum Rust impact: smoothing and goal acceptance must preserve tube/low-bridge direction semantics; direction `8` cannot be treated as an ordinary adjacent movement.

### Special land type 10 / tile-set branch

`UnitClass__Can_Enter_Cell` has a pre-object branch when `CellClass + 0xec == 10` and a unit-type field at `+0xdfc` is active:

```asm
0073f130: MOV EDI,dword ptr [ECX + 0xec]
0073f136: CMP EDI,0xa
0073f139: JNZ 0x0073f1ce
0073f141: MOV ECX,dword ptr [EDX + 0x38]
0073f14a: MOV ECX,dword ptr [EDX + ECX*0x4]
0073f14d: MOV EDX,dword ptr [ECX + 0x2e4]
0073f153: CMP EDX,0x5
0073f158: CMP dword ptr [ECX + 0x2e8],0x3
...
0073f176: CMP byte ptr [ECX + 0x11a],0x2
0073f182: MOV EAX,0x7
...
0073f191: CMP EDX,0x3
0073f196: CMP dword ptr [ECX + 0x2e8],0x4
...
0073f1b3: CMP byte ptr [ECX + 0x11a],0x6
0073f1bf: MOV EAX,0x7
```

This slot did not decode the semantic name of `CellClass + 0x11a`; it must not be called MovementZone from this evidence alone. For the water/pier bug, the important point is that this is a narrow land-type-10/tile-set branch and not the general water rejection path.

### `FootClass__LocomotorPassabilityCheck @ 0x004D9C10`

Decompile and assembly show this helper is a conditional locomotor hook:

```asm
004d9c11: MOV ESI,ECX
004d9c13: MOV EAX,dword ptr [ESI + 0x674]
004d9c19: TEST EAX,EAX
004d9c1b: JZ 0x004d9c53
004d9c1d: MOV AL,byte ptr [ESP + 0x18]
004d9c21: TEST AL,AL
004d9c23: JZ 0x004d9c53
004d9c25: MOV EAX,dword ptr [ESP + 0x8]
004d9c2a: MOV EDI,dword ptr [EAX + 0x24]
004d9c41: MOV ESI,dword ptr [ESI + 0x674]
004d9c47: PUSH EDI
004d9c48: PUSH ESI
004d9c49: MOV ECX,dword ptr [ESI]
004d9c4b: CALL dword ptr [ECX + 0x1c]
004d9c50: RET 0x14
004d9c53: XOR EAX,EAX
004d9c56: RET 0x14
```

Interpretation:

- If the unit has no locomotor pointer at `FootClass + 0x674`, this returns `0`.
- If the caller flag at stack `+0x18` is false, this returns `0`.
- Otherwise it calls locomotor virtual `+0x1c` with the target cell coordinate from `CellClass + 0x24`.
- If this helper returns `7` at the `Can_Enter_Cell` call site, `Can_Enter_Cell` hard-blocks before overlay and object-list logic.

Minimum Rust impact: keep an explicit locomotor passability hook/result in the evaluator input, but the whole locomotor implementation can be deferred if the immediate water/pier tests use ordinary ground locomotion where the hook returns `0`.

### `CheckBridgeTraversal @ 0x004D9C60`

Decompile summary:

- Returns `0` or `7`.
- If previous/source cell is not supplied, it derives the reverse-neighbor cell using `(direction - 4) & 7`.
- It updates/consults a bridge/elevation parameter.
- It uses `CellClass + 0x140` flags:
  - `0x100`: bridge/deck-style alternate height/list flag;
  - `0x200`: bridgehead/entry flag.
- It uses `CellClass + 0x11b` as level.
- It uses `CellClass + 0x11c` as the one-level ramp/slope permissive byte.

Important branch behavior:

- Same-height transition can still reject when bridge/elevation state does not match the requested layer.
- Absolute height difference `1` checks ramp/slope byte on the appropriate cell.
- Absolute height difference `4` is bridge entry/exit:
  - entering down from bridge requires target/deck flag and matching requested layer;
  - entering up onto bridge requires source `0x100` and `0x200`, then sets the out flag.
- Any other height delta returns `7`.

Minimum Rust impact: a water/pier fix cannot treat bridge decks as ordinary ground cells based only on `PathGrid::is_any_layer_walkable()`. It needs source cell, target cell, direction, requested/effective level, and bridge flags.

### `CellClass__CheckCellPassability @ 0x004834A0`

This is the nearby-passable / cell-rect style boolean validator, not the `Can_Enter_Cell` status-code function.

Decompile summary:

- Returns `1` for passable and `0` for not passable.
- Optional zone-id parameter rejects if `MapClass__GetZoneID(...)` does not match.
- Optional level parameter rejects mismatched ground/bridge layers.
- Selects normal occupation flags at `CellClass + 0x124` or alternate occupation flags at `CellClass + 0x128`.
- Applies caller-provided masks to ignore selected occupation flag bits.
- For passable wall/overlay cases, it may temporarily treat land type as `0`.
- Otherwise it checks `SpeedType x LandType` table and rejects zero-cost land unless the alternate bridge/list path is active.

Assembly evidence for speed/land rejection:

```asm
004835d5: MOV EDX,dword ptr [ESP + 0x14]
004835d9: LEA EDX,[EDX + EAX*0x8]
004835dc: ADD EAX,EDX
004835de: FLD float ptr [EAX*0x4 + 0x89ea40]
004835e5: FCOMP float ptr [0x007e1748]
004835ed: TEST AH,0x40
004835f0: JZ 0x004835ff
004835f2: TEST CL,CL
004835f4: JNZ 0x004835ff
004835f9: XOR AL,AL
004835fc: RET 0x1c
00483602: MOV AL,0x1
00483605: RET 0x1c
```

Interpretation:

- This function also rejects `SpeedType x LandType == 0`.
- The alternate bridge/list condition can allow the cell despite a zero ground land-type speed.
- The function does not call the full object-list tree; it only checks occupation bytes and caller masks.

## Requested Questions

### Water LandType / SpeedType Zero Rejection

Verified.

Both `UnitClass__Can_Enter_Cell` and `CellClass__CheckCellPassability` reject cells when the movement table entry at `g_SpeedType_LandType_Table[land_type * 9 + speed_type]` equals zero, except when the active alternate bridge/list condition bypasses that ground-land check.

For ordinary ground units, bare water should be rejected by this table. Rust must not let `PathGrid` mark water as final ground-passable unless the native speed/land table also permits the mover.

### MovementZone / ZoneType Involvement

No direct `MovementZone x ZoneType` matrix use was verified inside `UnitClass__Can_Enter_Cell @ 0x0073F0A0` in this slot.

The direct terrain rejection here is `SpeedType x LandType`, plus bridge/tube/level checks. `CellClass__CheckCellPassability` optionally calls `MapClass__GetZoneID` when a zone-id constraint is supplied, and one stack argument influences that zone lookup and overlay whitelist branches, but this slot did not prove a general MovementZone/ZoneType matrix inside these two evaluator functions.

Rust should therefore split the concerns:

- route planning may still need MovementZone/ZoneType reduced-zone reachability;
- final cell-entry legality for this water/pier bug must include the native speed/land and bridge/tube slice described here.

### High Bridge Layer Acceptance / Rejection

Verified at the bridge traversal helper level.

High/alternate layer legality depends on source/target level, `0x100`/`0x200` flags, ramp byte, and requested/effective bridge level. It is not equivalent to “any layer walkable”.

### Low Bridge / Tube Direction `8`

Verified.

Direction `8` is a special tube transition path. It requires tube data and returns early. Missing tube data or empty tube endpoint data hard-blocks. Direction `8` must not be smoothed or redirected as an ordinary movement direction.

### Shore / Beach Handling

No separate shore/beach special-case branch was verified in `UnitClass__Can_Enter_Cell` for ordinary terrain. Shore/beach legality flows through the same `SpeedType x LandType` table unless an overlay or bridge/list condition changes the effective land type/layer.

For Rust, shore/beach should not be inferred from `is_water` alone. The evaluator needs the actual land type and speed type table entry.

### Before Or After Object List

Verified order:

1. Initial bridge/tube/height/tile-set checks can return `7`.
2. Virtual bridge/layer check at vtable `+0x1b0` can return `7`.
3. Locomotor passability hook can return `7`.
4. Overlay/wall-style checks can return or raise status.
5. Object list at `CellClass + 0xe4` or `+0xe8` is traversed.
6. If the selected object list is exhausted, the `SpeedType x LandType == 0` rejection can return `7`.

So for exact status-code parity, Rust must not freely reorder terrain and occupant checks. For a first terrain-only helper, return an enum such as `TerrainHardBlocked`, `TerrainAllowedNeedsOccupancy`, or `DeferredObjectList`, rather than pretending the full `Can_Enter_Cell` result is known.

## Required Rust Evaluator Inputs

For the minimum water/pier/shore/bridge cell-entry slice, Rust needs:

- mover speed type from the unit type;
- target cell land type;
- source cell and target cell coordinates;
- movement direction, including explicit support for direction `8`;
- source and target cell level bytes;
- source and target bridge flags equivalent to `CellClass + 0x140` bits `0x100` and `0x200`;
- source and target ramp/slope byte equivalent to `CellClass + 0x11c`;
- requested/effective bridge level parameter, including `-1`;
- selected layer/list mode equivalent to normal vs alternate list (`e4` vs `e8`) because it controls speed/land bypass behavior;
- optional locomotor passability result or hook;
- optional zone-id/level/occupation-mask arguments for `CheckCellPassability`-style nearby-cell search.

## Deferrable For This Bug Class

These are not needed to stop ordinary ground units entering bare water through pathgrid drift, but remain required for full parity:

- full occupant linked-list semantics and return-code priority `1..6`;
- crushability, garrison, wall ownership, ally/enemy status, weapon ability, and special-object branches;
- exact `FootClass::Find_Path -> Find_Nearby_Passable_Cell` stack row;
- exact semantic names for `CellClass + 0x11a` and the land-type-10 tile-set branch;
- full locomotor virtual `+0x1c` behavior;
- all aircraft/infantry override variants of cell entry.

## Implementation Handoff

Do not patch this by making `PathGrid` water non-walkable globally; native allows different movers/layers to resolve differently.

Instead, introduce a native-shaped terrain/cell-entry legality layer above `PathGrid`:

1. Use `PathGrid` only as a coarse candidate graph.
2. For each candidate or shortcut cell, evaluate the mover-specific terrain slice:
   - tube/bridge direction and layer checks;
   - locomotor hard-block hook where requested;
   - speed type vs land type zero rejection;
   - `CheckCellPassability`-style zone/level/occupation masks for nearby-passable searches.
3. Feed this evaluator into:
   - A* neighbor legality;
   - move-goal redirection;
   - path smoothing/reroute;
   - scatter/staging/helper cell selection.
4. Keep object-list evaluation separate until the full `Can_Enter_Cell` occupant tree is decoded.

Acceptance tests should include:

- ordinary ground speed rejects bare water even if `PathGrid` says the cell is ground-walkable;
- shore/beach uses land-type table, not `is_water`;
- high bridge deck over water is accepted/rejected by layer flags and requested level, not by any-layer walkability;
- direction `8` tube transition requires tube data and is not smoothed as ordinary movement;
- smoothing cannot shortcut through water by using `PathGrid::is_walkable()` directly.

## Confidence

High for the minimum water/pier terrain-slice requirements above: the key claims are backed by decompile plus assembly ranges from the requested active functions.

Medium for exact high-bridge vtable binding from `UnitClass__Can_Enter_Cell` to `CheckBridgeTraversal`: this slot verified the virtual call site and decoded `CheckBridgeTraversal`, but did not independently dump the UnitClass vtable slot.

Unchecked in this slot: full object-list priority, exact land-type-10 tile-set semantics, WaterBridge TMP bytes, and the `Find_Path -> FNPC` argument row.
