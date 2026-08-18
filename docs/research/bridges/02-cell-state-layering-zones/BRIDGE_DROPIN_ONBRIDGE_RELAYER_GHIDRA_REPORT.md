# Bridge DropIn OnBridge Relayer - Ghidra Research Report

**Address(es):** `0x005F4160` (`ObjectClass::DropIn`), `0x0047DD70` (`CellClass::BlowUpBridge`), `0x004D3780` (`TechnoClass::DoCloak`), `0x005683C0` / `0x005687F0` (Techno enter/exit cell list helpers)
**Date:** 2026-05-14
**Confidence:** High for normal Techno deck occupants during bridge collapse; Medium for exotic non-Techno objects that may appear in `CellClass+0xE8`.
**Active in YR:** Yes, conditional on a bridge-collapse path that calls `BlowUpBridge`. Standard YR has `DestroyableBridges=yes`, so this is live in normal skirmish play when high/low bridges collapse.

## 1. Overview

`ObjectClass::DropIn` performs the same old-layer/remove, mutate, new-layer/add sequence that normal bridge movement uses, but within the same cell. For ordinary units/infantry on a bridge deck, `DropIn` calls the virtual mark slot with `0` while `ObjectClass+0x8C` is still true, clears `+0x8C`, then calls the virtual mark slot with `1`. The concrete Techno mark slot is `TechnoClass::DoCloak`, which removes from or adds to `CellClass` selected lists using `ObjectClass+0x8C` at call time.

Therefore a bridge-collapse DropIn is a real same-cell relayer: bridge-list entry is removed from `CellClass+0xE8` before `OnBridge` is cleared, and the same object is re-added to `CellClass+0xE4` after `OnBridge` is cleared.

## 2. Key Offsets

| Offset | Owner | Meaning | Evidence |
|---|---|---|---|
| `ObjectClass+0x30` | Object | `NextObject` linked-list pointer inside selected cell list | `CellClass::AddContent` / `RemoveContent` |
| `ObjectClass+0x8C` | Object | persistent `OnBridge` list selector | `DropIn`, Techno enter/exit helpers |
| `ObjectClass+0x8D` | Object | falling state byte set by `DropIn` | `0x005F416A` |
| `ObjectClass+0x8F` | Object | bomb/falling damage byte set by `DropIn` | `0x005F4171` |
| `CellClass+0xE4` | Cell | ground object list head | `AddContent`, `RemoveContent`, `BlowUpBridge` ground pass |
| `CellClass+0xE8` | Cell | bridge/alt object list head | `AddContent`, `RemoveContent`, `BlowUpBridge` deck pass |

## 3. Core Logic

### 3.1 `ObjectClass::DropIn @ 0x005F4160`

Verified instruction order:

```asm
005F416A  mov byte ptr [esi+0x8D],1
005F4171  mov byte ptr [esi+0x8F],1
005F4178  call dword ptr [eax+0x124]   ; Mark/DoCloak(0), arg pushed at 005F4166
005F4184  call 0x004A9770              ; DisplayClass::RemoveFromLayer
005F418F  mov byte ptr [esi+0x8C],0    ; clear OnBridge
005F4196  call 0x004A9720              ; DisplayClass::Submit_Object
005F419D  push 1
005F41A1  call dword ptr [edx+0x124]   ; Mark/DoCloak(1)
```

The decompiler shows the same order:

```text
IsFallingDown = 1
IsABomb = 1
vtable+0x124(0)
DisplayClass::RemoveFromLayer(this)
OnBridge = 0
DisplayClass::Submit_Object(this)
vtable+0x124(1)
```

Binary-verified answer to Q1/Q2:

- `Mark(0)` happens before `OnBridge` is cleared.
- `Mark(1)` happens after `OnBridge` is cleared.
- The display relayer (`RemoveFromLayer` / `Submit_Object`) surrounds the `OnBridge` clear, but cell occupancy relayering is driven by the two `vtable+0x124` calls.

### 3.2 Concrete Techno mark slot: `TechnoClass::DoCloak @ 0x004D3780`

For UnitClass, InfantryClass, FootClass, and other normal Techno-derived deck occupants, vtable slot `+0x124` resolves to `TechnoClass::DoCloak`. Data xrefs place this function in live vtables including `0x007F5D94` (UnitClass vtable `+0x124`) and `0x007EB17C` (InfantryClass-family vtable slot).

Relevant verified flow:

```asm
004D3789  cmp edi,2
004D3799  call 0x006F4A70              ; TechnoClass::ProcessCloakAndNotify(mode)
004D37A6  call dword ptr [eax+0x78]    ; GetMapLayer
004D37A9  cmp eax,2                    ; only updates cell lists on layer 2
004D37B7  call dword ptr [edx+0x1B8]   ; GetCellCoords
...
004D37D8  call 0x005683C0              ; mode 1 or 3: AddContent helper
004D37F5  call 0x005687F0              ; mode 0: RemoveContent helper
```

Important details:

- `mode == 2` returns `1` immediately and does not touch cell lists.
- For mode `0`, it calls `TechnoClass__ExitCell_RemoveFromMultiCells`.
- For mode `1` or `3`, it calls `TechnoClass__EnterCell_AddToMultiCells`.
- It only calls enter/exit helpers when `GetMapLayer()` returns `2`. This is the live ground/deck occupancy path used by normal units.

### 3.3 Enter/exit helpers read `OnBridge` at call time

`TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`:

```asm
005688E1  mov dl,byte ptr [edi+0x8C]
005688E9  push edx
005688EA  push edi
005688EB  call 0x0047EA90              ; CellClass::RemoveContent
```

`TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`:

```asm
005684B1  mov dl,byte ptr [edi+0x8C]
005684B9  push edx
005684BA  push edi
005684BB  call 0x0047E8A0              ; CellClass::AddContent
```

Combining this with `DropIn` order:

1. `DropIn` calls `DoCloak(0)` while `OnBridge==1`.
2. Exit helper reads `OnBridge==1`.
3. `RemoveContent` removes from `CellClass+0xE8`.
4. `DropIn` clears `OnBridge=0`.
5. `DropIn` calls `DoCloak(1)`.
6. Enter helper reads `OnBridge==0`.
7. `AddContent` inserts into `CellClass+0xE4`.

This is same-cell relayering, not just a state-byte clear.

## 4. `BlowUpBridge` Binding

`CellClass::BlowUpBridge @ 0x0047DD70` is the collapse-time caller for deck occupants.

Verified bridge-list loop:

```asm
0047DDBA  mov ecx,dword ptr [esi+0xE8] ; AltObject / bridge list head
0047DDC0  test ecx,ecx
0047DDC4  mov eax,dword ptr [ecx]
0047DDC6  mov edi,dword ptr [ecx+0x30] ; snapshot next before mutation
0047DDC9  call dword ptr [eax+0xEC]    ; ObjectClass::DropIn slot
0047DDCF  test edi,edi
0047DDD1  mov ecx,edi
0047DDD3  jnz 0x0047DDC4
```

Key details:

- `BlowUpBridge` starts the deck pass directly from `CellClass+0xE8`.
- It snapshots `NextObject` before calling `DropIn`, so the loop can continue even though `DropIn` removes/re-adds the current object and overwrites `ObjectClass+0x30`.
- There is no earlier removal of the bridge-list object in this function before the `DropIn` call.
- The ground pass over `CellClass+0xE4` runs first and applies `ReceiveDamage` with `RulesClass+C4Warhead`; it does not touch the deck list.

Verified answer to Q3/Q4:

- The path is live during standard YR bridge collapse: `SetBridgeDirection(..., state=0)` calls `BlowUpBridge` for visited destroyed cells, and `BlowUpBridge` calls `DropIn` for every bridge-list object.
- No earlier pass in `BlowUpBridge` removes the deck object before `DropIn`. For normal Techno deck occupants, `DropIn` itself performs the removal and re-add.

## 5. `SetBridgeDirection` Binding

`CellClass::SetBridgeDirection_NESW @ 0x0047E040` and `_NWSE @ 0x0047E470` call `BlowUpBridge` when the state byte is destroyed (`param_3` byte is zero).

Representative verified call sites inside `SetBridgeDirection_NESW`:

```asm
0047E10B  mov ecx,esi
0047E10D  mov byte ptr [esi+0x11E],0
0047E114  call 0x0047DD70

0047E1E6  mov ecx,esi
0047E1E8  mov byte ptr [esi+0x11E],0
0047E1EF  call 0x0047DD70
```

This ties the DropIn relayer to bridge-collapse state mutation, not an unused object method.

## 6. Duplicate / Stale List Edge Cases

`CellClass::RemoveContent @ 0x0047EA90` uses only the selected list from its second stack argument:

```asm
0047EAA3  mov cl,byte ptr [esp+0x1C]
0047EAA7  test cl,cl
0047EAAB  mov eax,[edi+0xE8]            ; selected bridge list
0047EAB3  mov eax,[edi+0xE4]            ; selected ground list
```

It then walks only that selected list, unlinks the target if found, and clears `object+0x30` only after a successful selected-list unlink. It does not scan the other layer.

`CellClass::AddContent @ 0x0047E8A0` also selects only one list from the passed bridge flag. For non-building objects it prepends to the selected list. The only duplicate guard observed is a narrow selected-head check (`head->NextObject == object`) before prepending; it is not a scan across both lists.

Implication for Q6:

- gamemd is not a repair-by-id-across-all-layers system.
- If an object is stale in the wrong list, a selected-list remove will not clean it up.
- Current Rust `OccupancyGrid::remove(rx, ry, entity_id)` removes by ID across all layers in the cell, which is more forgiving than gamemd and can mask stale-list bugs.
- For parity-sensitive relayering, the future Rust invariant should be explicit selected-layer removal and selected-layer insertion, with debug validation to catch stale duplicates instead of silently repairing them.

This edge case is mostly a corruption/recovery distinction. In clean gamemd flow, stale duplicates should not arise because every live transition removes using the old selected layer before mutating the layer selector.

## 7. Current Rust Status

As of the current repo state:

- `src/sim/world/bridge_orchestrator.rs::drop_in_bridge_deck_entities` clears `bridge_occupancy` and `on_bridge`, snaps Z to ground, resets locomotor layer/phase, and clears movement target.
- It does not remove the entity from bridge occupancy and re-add it to ground occupancy for the same cell.
- `src/sim/occupancy.rs::remove` removes an entity ID from all occupants in a cell, not from one selected layer.
- `src/sim/occupancy.rs::move_entity` accepts one layer for remove+add and cannot express old-layer/new-layer relayering.
- `src/sim/occupancy.rs::rebuild` derives layer from `locomotor.layer`, not directly from `GameEntity::on_bridge`, which is risky for ramp edge states where the two intentionally disagree.

No implementation changes were made during this investigation.

## 8. Future Rust Invariant

For a future implementation, preserve this output-determining invariant:

```text
same-cell or cross-cell layer change:
  1. Snapshot old selected occupancy layer from old on_bridge.
  2. Remove from the old cell using only that old selected layer.
  3. Mutate on_bridge / bridge_occupancy / height state.
  4. Snapshot new selected occupancy layer from new on_bridge.
  5. Add to the destination cell using only that new selected layer.
  6. Treat stale entries in other layers as a validation failure, not as normal repair.
```

For bridge-collapse `DropIn`, old cell and new cell are the same `(rx, ry)`, but the sequence is still bridge-list remove, `OnBridge=false`, ground-list add.

## 9. Binary-Verified Findings vs Inference

Binary-verified:

- `DropIn` calls `vtable+0x124(0)` before clearing `ObjectClass+0x8C`.
- `DropIn` clears `ObjectClass+0x8C` before calling `vtable+0x124(1)`.
- Unit/infantry Techno `vtable+0x124` resolves to `TechnoClass::DoCloak`, which calls the Techno enter/exit list helpers for mark modes `1`/`0`.
- Techno enter/exit helpers read `ObjectClass+0x8C` immediately before calling `CellClass::AddContent`/`RemoveContent`.
- `BlowUpBridge` iterates `CellClass+0xE8`, snapshots `NextObject`, and calls vtable slot `+0xEC` (`DropIn`) on each bridge-list object.
- `RemoveContent` operates only on the selected layer list; it does not scan all layers.

Inference:

- Ordinary bridge-deck occupants in standard YR are Techno-derived units/infantry using `TechnoClass::DoCloak` at slot `+0x124`; this is strongly supported by vtable data xrefs and normal gameplay class hierarchy, but exotic non-Techno entries in `CellClass+0xE8` were not exhaustively classified here.
- Rust should prefer selected-layer removal over remove-by-id-across-all-layers for parity-sensitive paths; this is an implementation invariant derived from the binary behavior, not a binary code requirement.

## 10. Research Questions Answered

1. **Does DropIn's `Mark(0)` remove from bridge/alt before `OnBridge` is cleared?** Yes for normal Techno deck occupants. `DropIn` calls `vtable+0x124(0)` first, and `TechnoClass::DoCloak(0)` removes using `OnBridge==1`.
2. **Does later `Mark(1)` re-add to ground after `OnBridge` is cleared?** Yes. `DropIn` clears `+0x8C`, then `DoCloak(1)` adds using `OnBridge==0`.
3. **Is this sequence live during `BlowUpBridge` in standard YR?** Yes. Collapse calls `BlowUpBridge`; `BlowUpBridge` iterates `AltObject` and calls `DropIn`.
4. **Does an earlier collapse pass remove the deck object before `DropIn`?** No evidence of that in `BlowUpBridge`. The bridge-list loop calls `DropIn` directly; `DropIn` performs the selected-list remove.
5. **What exact Rust invariant should future implementation preserve?** Remove using old `on_bridge`, mutate, then add using new `on_bridge`, even when old and new cell are identical.
6. **Are duplicate/stale-list edge cases different from Rust remove-across-all-layers?** Yes. gamemd selected-list removal does not repair stale entries in the other list; Rust's current removal does.

## Sources

- Live Ghidra decompilation/disassembly:
  - `ObjectClass::DropIn @ 0x005F4160`
  - `TechnoClass::DoCloak @ 0x004D3780`
  - `TechnoClass__EnterCell_AddToMultiCells @ 0x005683C0`
  - `TechnoClass__ExitCell_RemoveFromMultiCells @ 0x005687F0`
  - `CellClass::AddContent @ 0x0047E8A0`
  - `CellClass::RemoveContent @ 0x0047EA90`
  - `CellClass::BlowUpBridge @ 0x0047DD70`
  - `CellClass::SetBridgeDirection_NESW @ 0x0047E040`
- Existing research:
  - `BRIDGE_OBJECT_ONBRIDGE_FIELD_GHIDRA_REPORT.md`
  - `BRIDGE_OBJECT_ONBRIDGE_EXTRA_WRITERS_GHIDRA_REPORT.md`
  - `BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md`
  - `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`
  - `OBJECTCLASS_GHIDRA_REPORT.md`
  - `CELL_OCCUPANCY_ORDERING_FOLLOWUP_GHIDRA_REPORT.md`
- Rust files read only:
  - `src/sim/world/bridge_orchestrator.rs`
  - `src/sim/occupancy.rs`
  - `docs/fidelity-checks/bridge-occupancy-layer-timing.md`
