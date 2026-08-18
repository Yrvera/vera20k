# UnregisterBridgeRepairHut + "Hut Registry" — Ghidra Research Report (2026-05-18)

Closes the residual Phase-2 item from
`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §6 row "Hut registry at
MapClass+0x1160 (DAT_008B41A8) — Not started" by fully decompiling
`MapClass::UnregisterBridgeRepairHut` (`0x00577920`, vtable slot 25)
and pinning down the two DynVecs it touches.

**TL;DR:** The function and the global are both **misnamed**.

- `MapClass::UnregisterBridgeRepairHut` is a **TagClass-removal helper**,
  not a hut-destruction handler. It only does work when the abstract
  being detached is a TagClass (RTTI `0x2C`).
- `DAT_008B41A8` is **`g_DestroyedEventTagList`** — a global
  `DynamicVectorClass<TagClass*>` that holds Tags whose category bit
  `0x04` (destroyed event) is set. Not a building registry.
- The per-cell "cells-with-attached-object" DynVec lives on the
  **MapClass singleton at byte offset `+0x115C`** (not `+0x1160`).
  `+0x1160` is the **data_ptr** field of that DynVec, not its base.

**Confidence:** HIGH for the function body, the registry layout, the
caller, and the RTTI gate — all freshly re-decompiled (`0x00577920`,
`0x007258d0`) plus disassembled at both call sites
(`0x00725ae5`, `0x00725b57`) in this session.

**Active in YR:** Yes, but **dormant on standard skirmish maps**. Code
path is exercised whenever any TagClass is destroyed; standard YR
skirmish maps have no `[Tags]` / `[Triggers]` / `[Events]` sections, so
both DynVecs are empty and every loop is a no-op zero-iteration. Active
on campaign and scripted multiplayer maps.

---

## 1. Function body — `MapClass::UnregisterBridgeRepairHut` (`0x00577920`)

Signature (Ghidra): `void __thiscall (MapClass *this, AbstractClass *target)`

Reproduced behaviour (paraphrased, not raw decomp):

```
phase A — per-cell registry (MapClass+0x115C DynVec):
    if target->vtable[0x2C]() != 0x2C:          // RTTI gate: TagClass only
        goto phase B                            // skip phase A entirely
    for i in 0 .. this->count_at_0x116C:
        coord = this->data_at_0x1160[i]         // packed (short x, short y)
        idx   = coord.y * 0x200 + coord.x
        cell  = (idx in 0..0x3FFFF)
                  ? this->cell_grid_at_0x13C[idx]
                  : &g_DefaultCell_at_0xABDC50  // sentinel; coord cached in DAT_00ABDC74
        if cell != NULL and cell->attached_object_at_0x3C == target:
            FUN_00485250(cell, 0)               // detach object from cell (clears cell+0x3C)
            removed = this->vtable_at_0x115C[4](cell + 0x24)   // DynVec::Find(coord)
            if removed != -1 and removed < count:
                count -= 1
                shift entries [removed+1 .. count]  left by one
            i -= 1                              // re-test the slot now occupied by next entry
    FUN_00485130(this, target)                  // wholesale per-cell detach by-object

phase B — global g_DestroyedEventTagList at DAT_008B41A8:
    if target->vtable[0x2C]() != 0x2C:          // RTTI gate again
        return
    idx = (*(DAT_008B41A8 + 0x10))(&target)     // DynVec::Find(target)
    if idx == -1 or idx >= DAT_008B41B8 (count): return
    DAT_008B41B8 -= 1                           // count decrement
    shift entries [idx+1 .. count] left by one in the data_ptr at DAT_008B41AC
```

Key facts (every offset verified in the live decomp of `0x00577920`):

- The function is gated **twice** on `target->WhatAmI() == 0x2C`
  (TagClass). Both gates are necessary because phase B can also be
  entered when phase A is skipped.
- `MapClass+0x116C` is the **count** of the per-cell registry, NOT the
  capacity. Capacity is at `+0x1164`. The loop walks `0 .. count`.
- `MapClass+0x1160` is the **data_ptr** of the per-cell registry. Each
  entry is a packed `short x; short y` (4 bytes).
- The 0x200 stride at `coord.y * 0x200 + coord.x` is the gamemd
  fixed-stride cell grid (0x200 = 512, the map_grid pitch used by
  `MapClass+0x13C`).
- `FUN_00485250(cell, 0)` is `CellClass::AttachObject(NULL)` — assigning
  NULL detaches whatever was on the cell, decrements its refcount, and
  removes the cell from the per-cell DynVec via vtable[4] (Find).
- `FUN_00485130(this, target)` is `CellClass::DetachObject(target)` (the
  same function, called by MapClass-this-pointer = the cell, with the
  abstract as argument) — symmetric cleanup that handles the case where
  the target was attached to a cell not in the registry.

**No EVA voice, no sound, no animation, no zone rebuild fires from this
function.** It is purely list-bookkeeping.

---

## 2. The two DynVecs

### 2.1 Per-cell "attached-object" registry — `MapClass+0x115C`

Field layout (verified earlier in
`MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §5; re-confirmed via
`get_field_access_context` on the singleton aliases at `0x00880944`):

| MapClass offset | Singleton alias | Field | Type |
|---|---|---|---|
| `+0x115C` | `DAT_00880944` | vtable (`&PTR_FUN_007E3890`) | `int*` |
| `+0x1160` | `DAT_00880948` | data_ptr | `CellStruct*` |
| `+0x1164` | `DAT_0088094C` | capacity | `int` |
| `+0x1168` | `DAT_00880950` | owns_memory | `bool` |
| `+0x1169` | `DAT_00880951` | flag | `bool` |
| `+0x116C` | `DAT_00880954` | count | `int` |
| `+0x1170` | `DAT_00880958` | grow_step | `int` |

Each entry is a 4-byte packed `(short x, short y)` cell coord. The
DynVec is element-typed for the cell-coord struct (vtable[8] = grow,
vtable[0x10] = Find-by-coord).

Producers / consumers (from §5 of the revisit report, confirmed):
- Producer: `FUN_00485250` (CellClass attach-object) pushes the cell's
  `+0x24` coord when a non-null object is attached.
- Consumer: `FUN_00485130` (CellClass detach-object) calls vtable[0x10]
  Find-by-coord and shifts to remove.
- Consumer: `MapClass::UnregisterBridgeRepairHut` (this report) — runs
  for the TagClass case.

### 2.2 Global destroyed-event tag list — `DAT_008B41A8`

Field layout (verified in
`MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §3 and
`MAPCLASS_COMPLETE_DECODE.md` §A):

| Address | Field | Type |
|---|---|---|
| `0x008B41A8` | vtable | `int*` |
| `0x008B41AC` | data_ptr | `TagClass**` |
| `0x008B41B0` | capacity | `int` |
| `0x008B41B4` | owns_memory | `bool` |
| `0x008B41B5` | flag | `bool` |
| `0x008B41B8` | count | `int` |
| `0x008B41BC` | grow_step | `int` |

Each entry is a `TagClass*`. Filtered membership rule:
```
for each TagClass tag in g_TagTypeClass_Array:
    if FUN_006E61F0(tag) & 0x04:          // has destroyed-type event
        push tag into DAT_008B41A8 DynVec
```

The "0x04" bit comes from the trigger-event category decoder — event
codes `8` and `0x18` (Destroyed) set bit `0x04`. (See
`MAPCLASS_COMPLETE_DECODE.md` §A for the full event-code → bit table.)

Producers:
- `FUN_00684C30` (`0x00684C30`, scenario post-init) — populates from
  `[Tags]` map section.
- `FUN_0067F9C0` (`0x0067F9C0`, savegame loader) — pushes from stream.

Consumers:
- `MapClass::UnregisterBridgeRepairHut` (`0x00577920`) — removes a
  TagClass from the list when that tag is destroyed.

Note: the report previously hypothesised this was the "bridge repair
hut" list. That hypothesis is wrong (corrected in
`MAPCLASS_COMPLETE_DECODE.md` §A and §16 of the bridge-repair report).
It is a destroyed-event-trigger registry that includes huts only
incidentally — and only if the map explicitly tags the hut with a
destroyed-event trigger.

---

## 3. Callers — who triggers the unregister?

`get_function_callers(0x00577920)` returns exactly one caller:

- `Detach_From_All_Lists` at `0x007258D0` (Ghidra annotation:
  "RTTI-keyed removal notification dispatch"; canonical-name candidate
  `Detach_From_All_Lists` / `Notify_Observers_Of_Removal`).

Two call sites within `Detach_From_All_Lists`:

| Call site | Dispatch case | RTTI | Effect |
|---|---|---|---|
| `0x00725AE5` | `case 0x0C` | object being detached has `WhatAmI()==0x0C` | enters `UnregisterBridgeRepairHut` body, **fails inner RTTI gate** (≠ 0x2C), returns no-op |
| `0x00725B57` | `case 0x2C` | TagClass | enters body, **passes inner gate**, runs both phases |

Both call sites use the same `__thiscall` calling convention:
- ECX = MapClass singleton (`g_Map` at `0x0087F7E8`)
- arg 0 (ESI) = the abstract being detached
- arg 1 (constant `1`) = unused by the body (the function signature
  reads only `param_2` for the abstract; the `1` is a stale third
  arg held over from an earlier helper signature)

The case-0x0C call is **effectively dead code**: it always returns
early because no class with `WhatAmI()==0x0C` is also a TagClass. The
RTTI gate inside `UnregisterBridgeRepairHut` makes the call a no-op for
that branch. (RTTI 0x0C in gamemd is FactoryClass — its tear-down
dispatches to this helper redundantly, which is harmless but wasteful.)

So in practice the function does meaningful work in exactly one
scenario: **a TagClass is being destroyed**.

### When does a TagClass get destroyed?

- Scenario unload (end of mission, return to shell).
- TagClass destruction triggered by scripted trigger actions
  (`Destroy Tag`, etc.) on campaign/scripted maps.
- Save/load reset between scenarios.

`Detach_From_All_Lists` is invoked from each AbstractClass destructor
prologue (the canonical Westwood "remove me from every list that knows
me" pattern). It is not called on building death directly.

---

## 4. Cleanup cascade — what fires when this runs

For a TagClass being destroyed:

1. **Phase A** — walks every cell in `MapClass+0x115C` registry,
   detaches the dying tag from any cell whose `+0x3C` points at it,
   shifts the cell-coord out of the registry.
2. `FUN_00485130(MapClass, tag)` — fallback per-cell detach that
   handles any cells the registry walk missed (e.g. cells where the
   tag is attached but the registry never recorded the coord).
3. **Phase B** — removes the tag's pointer from
   `DAT_008B41A8 g_DestroyedEventTagList` if present.

**Side effects that do NOT fire from this function:**

- No cell flags are cleared. The `0x200` bridgehead flag (slot-4's
  domain) is untouched.
- No `0x1F` (BridgeDestroyed) or `0x30` (BridgeRepaired) TriggerEvent
  is fired here. Those events fire on **bridge** destruction/repair
  inside the bridge-segment paths (see
  `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md`), not on
  TagClass destruction.
- No EVA voice, no anim, no sound — pure bookkeeping.
- The hut-building's destruction does NOT call this function directly.
  Buildings flow through their own teardown (BuildingClass destructor →
  `Detach_From_All_Lists` → case 0x06 for BuildingClass, which does
  NOT dispatch into `UnregisterBridgeRepairHut`). The
  building→hut→bridge cleanup happens elsewhere (the bridge-segment
  damage path, separately documented).

---

## 5. Why the misnomer?

Ghidra's name for `0x00577920` ("UnregisterBridgeRepairHut") was
inherited from a prior labelling pass that saw the function iterate
`DAT_008B41A8` and assumed (because the DynVec was thought to be a hut
registry) that the function was the hut unregister path. Both
assumptions are wrong:
- The DynVec holds `TagClass*`, not building pointers.
- The function's RTTI gate is `0x2C` (TagClass), not `0x06`
  (BuildingClass).

`BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §16 already proposed the
correct name **`MapClass::UnregisterTagFromCellAndGlobalList`** (Phase 2
rename candidate). This investigation confirms that proposal. No rename
performed in this session (read-only constraint).

---

## 6. Rust parity implications

- **No port required for hut destruction.** Removing a bridge repair
  hut in Rust does NOT need to interact with either DynVec.
- **If/when triggers land in Rust** (campaign / scripted MP support),
  the equivalent of `MapClass::UnregisterTagFromCellAndGlobalList`
  becomes part of TagClass teardown, not bridge teardown.
- The previously-quoted Phase-2 task "decompile UnregisterBridgeRepairHut
  to learn the hut registry" is **dissolved** — the registry doesn't
  exist. Hut state in Rust can be modelled with the building entity
  alone.

---

## 7. Open follow-ups (out of scope for this report)

- The `case 0x0C` dispatch in `Detach_From_All_Lists` calling a
  TagClass-gated helper is suspicious — likely a TS-era artifact where
  FactoryClass used to share teardown with Tags. Confirming this needs
  a TS-vs-YR diff; documented here for future cleanup.
- `Detach_From_All_Lists`'s 8-case dispatch table (RTTI 0x04, 0x0C,
  0x0D, 0x18, 0x22, 0x26, 0x2C, 0x2F/0x30, 0x33, 0x3C) has only
  partial canonical-name coverage. Slot-4 and slot-5 of the swarm may
  surface the rest.

---

## Sources (this session)

### Decompiled
- `0x00577920` `MapClass::UnregisterBridgeRepairHut` — full body
- `0x007258D0` `Detach_From_All_Lists` — full body (caller)
- `0x00485250` `CellClass::AttachObject` (FUN_00485250) — Phase A helper
- `0x00485130` `CellClass::DetachObject` (FUN_00485130) — Phase A fallback
- `0x00684C30` `FUN_00684C30` (scenario post-init, producer)

### Disassembly (calling-convention verification)
- `0x007258D0` full listing — both call sites at `0x00725AE5` and
  `0x00725B57` verified `__thiscall` with ECX=MapClass singleton

### Xrefs
- `get_xrefs_to(0x00577920)` → 2 code refs (both inside
  `Detach_From_All_Lists`), 8 data refs (vtables)
- `get_function_callers(0x00577920)` → `Detach_From_All_Lists` only
- `get_field_access_context(0x008B41A8)` → producers `FUN_00684C30`
  and `FUN_0067F9C0`, consumer `MapClass::UnregisterBridgeRepairHut`
- `get_field_access_context(0x00880944)` → producer `FUN_00485250`,
  consumer `FUN_00485130` (per-cell DynVec singleton alias)

### Memory
- `read_memory(0x008B41A8, 32)` → all zeros (static init; populated at
  runtime by scenario post-init)
- `read_memory(0x00880944, 40)` → all zeros (same — runtime-populated)

### Referenced docs (already verified by prior sessions)
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §6 (Phase 2 stub),
  §16 (misnomer analysis), §17 (rename proposals)
- `MAPCLASS_ZONES_RAMPS_HUT_REGISTRY_GHIDRA_REPORT.md` §3 (registry
  layout), §5 (correction)
- `MAPCLASS_GHIDRA_REPORT_REVISIT_2026_04_24.md` §5 (per-cell DynVec
  field layout)
- `MAPCLASS_COMPLETE_DECODE.md` §A (event-category bit mapping,
  destroyed-event = bit 0x04)
- `TECHNOCLASS_PROCESSCELLACTION_0x1F_0x30_GHIDRA_REPORT.md`
  (BridgeDestroyed / BridgeRepaired trigger-event IDs; not fired
  from this path)
