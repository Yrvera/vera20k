# Bridge Object OnBridge Field (`ObjectClass+0x8C`) - Ghidra Report

**Date:** 2026-05-14
**Scope:** Follow-up to the cell occupancy ordering work. This report covers the object-local bridge-list selector at `ObjectClass+0x8C`, especially writer timing relative to cell-list removal/addition.
**Confidence:** High for the drive/walk/ship locomotion timing and ObjectClass base behavior; medium for non-ground locomotor side paths that were byte-scanned but not fully behavior-ported here.
**Active in YR:** Yes. The field is live for normal YR units, infantry, ships, aircraft, buildings, animations, and teleport/warp flows. The occupancy-list consequence is most directly visible for Techno/Foot movement across bridge ramps and bridge bodies.

## Executive Findings

`ObjectClass+0x8C` is the persistent per-object `OnBridge` byte used by `CellClass::AddContent` and `CellClass::RemoveContent` to choose between the ground list (`CellClass+0xE4`) and the bridge/alt list (`CellClass+0xE8`).

The normal movement ordering is:

1. Remove/unmark the object from the old cell while `+0x8C` still contains the old layer state.
2. Update coordinates to the new cell.
3. Run the bridge-transition predicate and possibly write `+0x8C = 1` or `+0x8C = 0`.
4. Add/mark the object in the new cell using the updated `+0x8C`.

This matters because the old and new cells can legitimately use different lists on the same cell-boundary crossing. A correct Rust model should not update the entity's occupancy-list layer before removal from the old cell, and should not delay the update until after insertion into the new cell.

For drive, ship, and walk movement, the transition predicate is the same shape:

```c
// src = previous cell class, dst = new cell class
if ((int8)dst.Level == (int8)src.Level - 4) {
    if (dst.Flags & 0x100) {
        object->OnBridge = 1;
        goto after_clear_check;
    }
    if (src.Flags & 0x100) {
        object->OnBridge = 0;
    }
} else {
    if ((dst.Flags & 0x100) == 0) {
        if (src.Flags & 0x100) {
            object->OnBridge = 0;
        }
    }
}
after_clear_check:
```

Equivalent rule:

- Set `OnBridge=1` only when moving to a bridge-flagged cell whose `Level` is exactly source `Level - 4`.
- Clear `OnBridge=0` when the destination is not bridge-flagged and the source is bridge-flagged.
- If the destination is bridge-flagged but the exact `-4` level relation is not true, leave the current byte unchanged.
- The set branch has priority over the clear branch because the clear branch is skipped after the `OnBridge=1` write.

This is the same corrected predicate already represented in Rust's bridge movement code, but the occupancy-list update point is now pinned more precisely.

## Prior Work Used

Parent report: `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`

That report already verified:

- `CellClass::AddContent @ 0x0047E8A0` selects `FirstObject` (`+0xE4`) when the second stack argument is `0`, and `AltObject` (`+0xE8`) when nonzero.
- `CellClass::RemoveContent @ 0x0047EA90` uses the same second stack argument.
- `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0` passes `byte ptr [object+0x8C]` to `AddContent`.
- `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0` passes `byte ptr [object+0x8C]` to `RemoveContent`.

This report does not re-cover list insertion order. It only closes the deferred `+0x8C` writer/update-order gap.

## Verified Base-Class Behavior

### Constructor default

`ObjectClass::Constructor @ 0x005F3900` initializes `+0x8C` to `0`.

Evidence:

```asm
005f396f: MOV byte ptr [ESI + 0x8c],BL
```

At this point `BL` is zero in the constructor's zero-initialization sequence. Nearby stores initialize `+0x80`, `+0x82`, `+0x83`, `+0x84`, `+0x8D`, `+0x8E`, and `+0x8F` to zero as well.

Active in YR: Yes. This is the base constructor for ObjectClass-derived objects.

### Unlimbo can set OnBridge before marking

`ObjectClass::Unlimbo @ 0x005F5940` checks the destination cell before the object is placed.

Verified decompile shape:

```c
cell = CellClass::Get_Cell_At(coords);
if ((cell->Flags & 0x100) != 0) {
    this->OnBridge = 1;
    if ((cell->Flags & 0x200) == 0) {
        return 0;
    }
}
...
ok = this->Mark(coords, 0x80);
if (ok) {
    this->Set_Coords_With_Cloak(coords); // vtable+0x1B4
    ...
    return 1;
}
```

Assembly evidence for the writer:

```asm
005f597b: MOV EAX,dword ptr [EBP + 0x140]
005f5981: TEST AH,0x1
005f5984: JZ 0x005f599c
005f5986: MOV byte ptr [ESI + 0x8c],0x1
005f598d: MOV EAX,dword ptr [EBP + 0x140]
005f5993: TEST AH,0x2
005f5996: JZ 0x005f5b41
```

Tiny detail: the bridge-body rejection is after the `OnBridge=1` write. If the target cell has `Flags&0x100` but not `Flags&0x200`, Unlimbo returns failure with the byte already set. Because the object is still not successfully marked/placed, this is normally not an occupancy-list registration, but it is a real side effect.

Active in YR: Yes. Conditional on Unlimbo into a bridge-flagged cell.

### DropIn clears OnBridge before re-submitting

`ObjectClass::DropIn @ 0x005F4160` clears `+0x8C` while re-layering the falling object.

Evidence:

```asm
005f4184: CALL 0x004a9770
005f4189: PUSH ESI
005f418a: MOV ECX,0x87f7e8
005f418f: MOV byte ptr [ESI + 0x8c],0x0
005f4196: CALL 0x004a9720
005f419b: MOV EDX,dword ptr [ESI]
005f419d: PUSH 0x1
```

The decompile also shows the surrounding sequence:

```c
this->IsFallingDown = 1;
this->IsABomb = 1;
this->Mark(0);
DisplayClass::RemoveFromLayer(this);
this->OnBridge = 0;
DisplayClass::Submit_Object(this);
this->Mark(1);
```

Active in YR: Yes for falling/drop-in objects. This is not the normal vehicle ramp crossing path, but it prevents falling objects from retaining bridge-list membership.

### GetHeight subtracts bridge height when OnBridge is true

`ObjectClass::GetHeight @ 0x005F5F40` uses `+0x8C` as a semantic height correction, not just an occupancy-list flag.

Verified decompile:

```c
height = this->Location_Z - CellClass::GetGroundHeight(this->Location);
if (this->OnBridge != false) {
    height -= DAT_00ac13bc; // bridge Z offset
}
return height;
```

Active in YR: Yes. This confirms `+0x8C` is a persistent bridge-surface state, not a transient AddContent argument only.

### ShouldBeOnBridge is a local predictor, not the main crossing writer

`ObjectClass::ShouldBeOnBridge @ 0x005F6A70` reads the current `+0x8C`, compares old/new ground heights, and returns the prior or predicted value.

Verified decompile shape:

```c
old_on_bridge = this->OnBridge;
new_ground = CellClass::GetGroundHeight(new_coords);
old_ground = CellClass::GetGroundHeight(this->Location);

if (this->OnBridge == 0
    && BridgeHeight * 3 < old_ground - new_ground
    && (CellClass::Get_Cell_At(new_coords)->Flags & 0x100) != 0) {
    return 1;
}

if (this->OnBridge != 0 && BridgeHeight * 3 < new_ground - old_ground) {
    return 0;
}

return old_on_bridge;
```

`FootClass::ShouldBeOnBridge @ 0x004DDC40` adds one gate:

```c
if ((int8)this->TubeIndex >= 0) {
    return 0;
}
return ObjectClass::ShouldBeOnBridge(...);
```

Active in YR: Yes, but it is not the direct writer used in the drive/walk boundary crossing shown below.

## Add/Remove Layer Argument Source

The list-selection byte is read at the moment `AddContent` or `RemoveContent` is called.

`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`:

```asm
005684b1: MOV DL,byte ptr [EDI + 0x8c]
005684b7: MOV ECX,EAX
005684b9: PUSH EDX
005684ba: PUSH EDI
005684bb: CALL 0x0047e8a0       ; CellClass::AddContent
```

`TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`:

```asm
005688e1: MOV DL,byte ptr [EDI + 0x8c]
005688e7: MOV ECX,EAX
005688e9: PUSH EDX
005688ea: PUSH EDI
005688eb: CALL 0x0047ea90       ; CellClass::RemoveContent
```

Active in YR: Yes.

Implementation consequence: an entity move operation cannot use only one final layer value for both removal and insertion when crossing a bridge transition. It needs old-layer removal and new-layer insertion, or the caller must sequence the field/state updates exactly like gamemd.

## Normal Movement Crossing Writers

### Drive locomotor, first boundary block

`DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20` contains a bridge-state update immediately after it unmarks/moves the object and before it re-marks it.

Relevant decompile shape:

```c
object->Mark(0);                    // vtable+0x124, MARK_REMOVE
object->Set_Coords_With_Cloak(...); // vtable+0x1B4

src = MapClass::Get_CellClass(old_cell);
dst = MapClass::Get_CellClass(new_cell);

if ((int8)dst->Level == (int8)src->Level - 4) {
    if (dst->Flags & 0x100) {
        object->OnBridge = 1;
        goto after_clear_check;
    }
    if (src->Flags & 0x100) {
        object->OnBridge = 0;
    }
} else {
after_set_label:
    if ((dst->Flags & 0x100) == 0) {
        if (src->Flags & 0x100) {
            object->OnBridge = 0;
        }
    }
}

after_clear_check:
object->PerCellProcess(...);        // vtable+0x1CC in this caller
object->Mark(1);                    // MARK_PUT
```

Assembly evidence:

```asm
004b1825: TEST dword ptr [EBX + 0x140],EAX
004b182b: JZ 0x004b183f
004b182d: MOV EDX,dword ptr [EBP + 0xc]
004b1830: MOV byte ptr [EDX + 0x8c],0x1
004b1837: TEST dword ptr [EBX + 0x140],EAX
004b183d: JNZ 0x004b1851
004b183f: TEST dword ptr [ESI + 0x140],EAX
004b1845: JZ 0x004b1851
004b1847: MOV EAX,dword ptr [EBP + 0xc]
004b184a: MOV byte ptr [EAX + 0x8c],0x0
```

Here `EBX` is the destination cell and `ESI` is the source/previous cell in this decompile branch. `EAX` is loaded with `0x100` earlier, so both tests are bridge-flag tests.

Active in YR: Yes for drive locomotor vehicles.

### Drive locomotor, second boundary block

The same function has a second equivalent bridge-state block later in the drive-track flow.

Assembly evidence:

```asm
004b257b: TEST dword ptr [EAX + 0x140],EDX
004b2581: JZ 0x004b2595
004b2583: MOV ESI,dword ptr [EBP + 0xc]
004b2586: MOV byte ptr [ESI + 0x8c],0x1
004b258d: TEST dword ptr [EAX + 0x140],EDX
004b2593: JNZ 0x004b25a7
004b2595: TEST dword ptr [ECX + 0x140],EDX
004b259b: JZ 0x004b25a7
004b259d: MOV EAX,dword ptr [EBP + 0xc]
004b25a0: MOV byte ptr [EAX + 0x8c],0x0
004b25a7: MOV EBP,dword ptr [EBP + 0xc]
004b25aa: PUSH 0x1
```

This block occurs before the subsequent `Mark(1)` call.

Active in YR: Yes for another drive-track crossing path in the same function.

### Walk locomotor uses the same timing

`WalkLocomotionClass::ProcessMovement @ 0x0075AEC0` has the same remove -> coordinate update -> `OnBridge` write -> add sequence.

Relevant decompile shape:

```c
object->Mark(0);
object->Set_Coords_With_Cloak(...);
src = MapClass::Get_CellClass(...);
dst = MapClass::Get_CellClass(...);
// same Level/Flags predicate
object->PerCellProcess(...);
object->Mark(1);
```

Assembly evidence:

```asm
0075c16e: TEST dword ptr [EAX + 0x140],ECX
0075c174: JZ 0x0075c188
0075c176: MOV EDX,dword ptr [EBP + 0xc]
0075c179: MOV byte ptr [EDX + 0x8c],0x1
0075c180: TEST dword ptr [EAX + 0x140],ECX
0075c186: JNZ 0x0075c19a
0075c188: TEST dword ptr [ESI + 0x140],ECX
0075c18e: JZ 0x0075c19a
0075c190: MOV EAX,dword ptr [EBP + 0xc]
0075c193: MOV byte ptr [EAX + 0x8c],0x0
```

Active in YR: Yes for infantry/foot walking locomotion.

### Ship locomotor uses the same predicate

`ShipLocomotionClass::Process_Drive_Track @ 0x006A05F0` and `ShipLocomotionClass::Process_Movement @ 0x006A1C80` contain the same writer pattern.

Assembly evidence:

```asm
006a0eb1: TEST dword ptr [EBX + 0x140],EAX
006a0eb7: JZ 0x006a0ecb
006a0eb9: MOV EDX,dword ptr [EBP + 0xc]
006a0ebc: MOV byte ptr [EDX + 0x8c],0x1
006a0ec3: TEST dword ptr [EBX + 0x140],EAX
006a0ec9: JNZ 0x006a0edd
006a0ecb: TEST dword ptr [ESI + 0x140],EAX
006a0ed1: JZ 0x006a0edd
006a0ed3: MOV EAX,dword ptr [EBP + 0xc]
006a0ed6: MOV byte ptr [EAX + 0x8c],0x0
```

and:

```asm
006a1bcc: TEST dword ptr [EAX + 0x140],EDX
006a1bd2: JZ 0x006a1be6
006a1bd4: MOV ESI,dword ptr [EBP + 0xc]
006a1bd7: MOV byte ptr [ESI + 0x8c],0x1
006a1bde: TEST dword ptr [EAX + 0x140],EDX
006a1be4: JNZ 0x006a1bf8
006a1be6: TEST dword ptr [ECX + 0x140],EDX
006a1bec: JZ 0x006a1bf8
006a1bee: MOV EAX,dword ptr [EBP + 0xc]
006a1bf1: MOV byte ptr [EAX + 0x8c],0x0
```

Active in YR: Conditional. Ships normally travel water/ground layers; bridge underpass behavior uses the same bridge-flag field when relevant.

## Other Direct Writer Families Found

The byte-pattern scan found additional direct writes to `[object+0x8C]`. These are live but mostly outside the narrow ground/bridge occupancy-list crossing.

Verification note (2026-05-14): treat this table as scoped evidence, not an exhaustive inventory. The follow-up report `BRIDGE_OBJECT_ONBRIDGE_EXTRA_WRITERS_GHIDRA_REPORT.md` classifies the extra byte-scan hits from verify-doc. `0x0051A407` is a real `InfantryClass::Mission_Enter` clear before an enter/conceal path; `0x006FF0B0` is a real `BulletClass.OnBridge` set for `Inviso=yes` bullets fired at an on-bridge target. The other listed extras (`0x006DD711`, `0x006E3FEB`, `0x0051FDB2`, `0x00776EC9`, `0x00776F04`) are not runtime `ObjectClass+0x8C` bridge-state writes. None of those extra hits changes the normal drive/walk/ship movement-order finding.

| Address family | Function family | Observed effect | Notes |
|---|---|---|---|
| `0x00416B6E`, `0x00416B77` | `AircraftClass::Carryall_Pickup @ 0x00416AF0` | Sets carried object's `OnBridge` to `1` or `0` | Carryall pickup/drop logic preserves or clears carried-object bridge state. |
| `0x00417290` | `AircraftClass` mission flow | Clears `OnBridge` | Aircraft-specific path, not normal ground cell-list movement. |
| `0x00424ADF` | `AnimClass` flow | Sets `OnBridge=1` before `Mark(1)` | AnimClass also derives from ObjectClass and can register on bridge layer. |
| `0x004CD3C9`, `0x004CDEA6`, `0x004CDFA5`, `0x004CECFF` | `FlyLocomotionClass::Process @ 0x004CD600` | Clears or sets `OnBridge` during flight/crash/landing logic | Mostly aircraft/flying object height management. |
| `0x00514944`, `0x0051495F` | `HoverLocomotionClass::Move @ 0x00514310` | Same bridge-flag set/clear shape | Relevant for hover units when bridge flags are encountered. |
| `0x0054B214`, `0x0054B537`, `0x0054C8B7`, `0x0054D012`, `0x0054D438`, `0x0054D99C` | Jumpjet locomotion family | Sets/clears during jumpjet movement/landing | Jumpjets use air-ish movement with bridge-aware placement resets. |
| `0x005B16D9`, `0x005B16F3` | Mech locomotion family | Same set/clear shape | Function was not fully named in the current listing, but the surrounding function range belongs to mech locomotion code. |
| `0x0062ACE3`..`0x0062AEDC` | `WarpAttachClass` flow | Sets/clears target object's `OnBridge` around warp placement checks | Relevant to chrono/warp repositioning rather than normal per-cell movement. |
| `0x00718704`, `0x00718719`, `0x00719698`, `0x007196A4`, `0x007198FA` | `TeleportLocomotionClass` flow | Sets/clears during teleport update/post-warp/phase transitions | Teleport has its own bridge-state placement path. |
| `0x0073A297`, `0x0073A6F5` | `UnitClass` deploy/limbo-like flows | Clears `OnBridge` | Unit-specific state reset. |
| `0x0075C179`, `0x0075C193` | `WalkLocomotionClass::ProcessMovement` | Same set/clear shape | Covered above. |

Copy-style register writes also occur:

- `BuildingClass::SpawnSurvivors @ 0x00442D90` copies the source building's `+0x8C` to spawned survivors (`MOV CL,[source+0x8C]`; `MOV [new+0x8C],CL`).
- `BuildingClass` undeploy/sell-related flow near `0x0044A493` copies a building/source object's `+0x8C` to the produced object.
- `ObjectClass::Constructor @ 0x005F3900` zero-initializes the byte.
- Unit/Infantry copy or placement flows near `0x00738041`, `0x007382B7`, `0x007434F3` copy or recompute the byte during specialized state transitions.

These writer families reinforce that `OnBridge` is object-local persistent state. It is not recomputed from the destination cell every time a consumer wants a layer.

## Current Rust Status

Current Rust already has the right high-level field split:

- `src/sim/game_entity.rs` has `GameEntity::on_bridge: bool`.
- `GameEntity::movement_layer_or_ground()` prefers `on_bridge` over `locomotor.layer`.
- `src/sim/movement/movement_bridge.rs` models a persistent `on_bridge` state independent of path layer.
- `src/sim/movement/movement_bridge.rs::compute_bridge_transition` matches the verified set/clear predicate: enter on ramp-to-body `dst_h == src_h - 4 && dst.bridge_walkable`; exit on `!dst.bridge_walkable && src.bridge_walkable`; otherwise unchanged.
- `src/sim/world/world_spawn.rs` sets `on_bridge=true` and `bridge_occupancy=Some(...)` for map spawns with `High=yes` only when resolved bridge deck data exists.

Important current divergence risk:

Two normal movement paths currently disagree with the verified gamemd ordering:

- `src/sim/movement/movement_step.rs` calls `OccupancyGrid::move_entity(..., active_layer, ...)` before `resolve_cell_transition_bridge_state`, then applies `on_bridge` later through the caller. On bridge transitions, insertion can therefore use the pre-transition/path layer instead of the post-transition `OnBridge` list selector.
- `src/sim/movement/movement_tick.rs` has a drive-track cell-jump branch that calls `resolve_cell_transition_bridge_state` before occupancy movement, but still calls `OccupancyGrid::move_entity(..., active_layer, ...)` before applying `on_bridge`. This path can use A*'s `next_layer` for insertion rather than the post-transition `OnBridge` list selector.

In gamemd, old-cell removal observes the old `OnBridge`, then the new-cell add observes the post-transition `OnBridge`.

The same principle also applies to same-cell relayering. As of the 2026-05-14 Rust audit, `src/sim/world/bridge_orchestrator.rs::drop_in_bridge_deck_entities` clears `on_bridge` and `bridge_occupancy` during bridge collapse without relayering the persistent occupancy entry from bridge to ground.

Implementation implication:

- Removal should use the pre-transition `on_bridge`/old selected list.
- Insertion should use the post-transition selected list after the bridge update.
- A single `move_entity(old, new, layer)` call is only parity-safe for non-transition steps where old and new selected lists are the same.
- Both `movement_step.rs` and the drive-track branch in `movement_tick.rs` need the split old-layer/new-layer sequencing.

This is a narrower and stronger requirement than the earlier occupancy-ordering report. The earlier ordering work preserved list order once the correct layer was chosen; this report explains when the chosen layer changes within a cell-boundary transition.

## Implementation Guidance

Do not derive this from the requested A* layer alone. `locomotor.layer` and `OnBridge` intentionally disagree on bridge ramp edge ticks.

Recommended model:

1. Snapshot `old_layer_for_occupancy = entity.on_bridge ? Bridge : Ground` before any cell-crossing mutation.
2. Remove from the old cell using `old_layer_for_occupancy`.
3. Resolve the bridge transition and update `entity.on_bridge`/`bridge_occupancy`.
4. Compute `new_layer_for_occupancy = entity.on_bridge ? Bridge : Ground`.
5. Insert into the new cell using `new_layer_for_occupancy`.
6. Keep `locomotor.layer` as the path/A* layer; do not use it as the authoritative list selector at bridge ramps.

For same-cell layer changes, use the same principle: remove from the old selected list before mutating `on_bridge`, then add to the new selected list after mutation.

## Open Questions

1. The non-drive writer families should be verified individually before porting teleport, jumpjet, hover, carryall, or warp bridge placement parity. The byte scan proves direct writes exist, but this report only fully traces the normal ground/walk/ship movement ordering.
2. `ObjectClass::Unlimbo` sets `OnBridge=1` before rejecting non-bridgehead bridge cells. Future save/load or failed-placement parity work should check whether any live caller observes the side effect after a failed Unlimbo.
3. Some direct-write hits are copy/constructor flows for object families not yet central to Rust simulation. They should be scoped per feature instead of implemented as a broad refactor.

## Evidence Sources

Live Ghidra/decompilation and byte-pattern searches in `gamemd.exe`:

- `ObjectClass::Constructor @ 0x005F3900`
- `ObjectClass::DropIn @ 0x005F4160`
- `ObjectClass::Unlimbo @ 0x005F5940`
- `ObjectClass::GetHeight @ 0x005F5F40`
- `ObjectClass::ShouldBeOnBridge @ 0x005F6A70`
- `FootClass::ShouldBeOnBridge @ 0x004DDC40`
- `DriveLocomotionClass::Process_Drive_Track @ 0x004B0F20`
- `WalkLocomotionClass::ProcessMovement @ 0x0075AEC0`
- `ShipLocomotionClass::Process_Drive_Track @ 0x006A05F0`
- `ShipLocomotionClass::Process_Movement @ 0x006A1C80`
- `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`
- `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`
- `CellClass::AddContent @ 0x0047E8A0`
- `CellClass::RemoveContent @ 0x0047EA90`

Prior reports:

- `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`
- `OBJECTCLASS_GHIDRA_REPORT.md`
- `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
