# PathfinderClass__UpdateHierarchicalEdges — Decode Doc
**Proposed Ghidra label:** PathfinderClass__UpdateHierarchicalEdges (already labeled)

## Summary

Called by `AStar_pathfind_search` after a marker-gated `AStar_main_loop` fails while
hierarchical mode is active. Loops over zone hierarchy levels 0, 1, 2. For each level:

1. Reads the current zone id from `DAT_0087f858` keyed by the pathfinder's stored cell
   (`PathfinderClass+0x70`).
2. Builds a temporary `u16` vector (stack-allocated, vtable at `0x007e3844`, capacity 10).
3. Calls `ZoneMap__FloodFillReachableZones` to detect whether the current cell is locally
   reachable within this level's block.
4. **Zero-return branch** (cell is locally reachable — soft split): iterates the temp
   vector backwards; for each `neighbor_zone != current_zone` appends sorted undirected
   packed edge `min<<16|max` to the per-level exclusion list at `PathfinderClass+0x7c +
   level*0x18`.
5. **Nonzero-return branch** (cell is blocked — hard split): calls
   `PathfinderClass__InvalidateZoneEdge(this, current_zone, level)`, which selects one
   adjacent stored-path edge to invalidate; may also clear `PathfinderClass+0x38`
   (hierarchy-valid flag) if no actionable path exists.

After this returns, `AStar_pathfind_search` calls `PathfinderClass__Reset`, re-reads
`+0x38`, and runs `Zone_precheck` again with the updated exclusion vectors.

**Active in YR: Yes.** Single caller: `AStar_pathfind_search @ 0x0042C900` (verified
via `get_function_callers 0x0042CCD0`). Reachable through any hierarchical-mode path
request that fails the inner A* loop.

---

## Decompilation excerpt

Source: `decompile_function 0x0042CCD0`

```c
void __thiscall
PathfinderClass__UpdateHierarchicalEdges(int param_1,   // PathfinderClass* this
                                         undefined4 param_2) // FootClass* mover
{
    int iVar10;
    int *piVar11;       // walks param_1+0x7c, advancing +6 ints (=0x18 bytes) per level
    // ... locals for temp vector (stack-allocated) ...

    iVar5 = ZoneMap__CellToZoneIndex(param_1 + 0x70);
    iVar10 = 0;
    piVar11 = (int *)(param_1 + 0x7c);
    iVar5 = DAT_0087f858 + iVar5 * 10;     // per-cell zone record, stride 10 bytes

    do {
        uVar1 = *(ushort *)(iVar5 + iVar10 * 2);   // current_zone for this level

        // Build stack temp vector: vtable=PTR_FUN_007e3844, capacity=10
        FUN_0042dd60(0, 0);
        local_18 = &PTR_FUN_007e3844;
        local_4 = 10;
        local_8 = 0;

        // Call flood-fill split detector
        uVar6 = MapClass__Get_CellClass(param_1 + 0x70);
        cVar4 = ZoneMap__FloodFillReachableZones(uVar6, iVar10, pppuVar12, param_2);

        if (cVar4 == '\0') {
            // Zero-return: cell locally reachable — append collected neighbor edges
            iVar7 = local_8 + -1;
            if (-1 < iVar7) {
                uVar3 = (uint)uVar1;   // current_zone
                do {
                    uVar9 = (uint)*(ushort *)(local_14 + iVar7 * 2);
                    if (uVar9 != uVar3) {
                        // sort endpoints; pack as min<<16|max; append to piVar11 list
                        uVar8 = uVar3; if (uVar9 < uVar3) { uVar8=uVar9; uVar9=uVar3; }
                        // grow list if needed (capacity check via piVar11[-2+8])
                        *(uint *)(piVar11[-1] + iVar2 * 4) = uVar8 << 0x10 | uVar9;
                    }
                    iVar7 = iVar7 + -1;
                } while (-1 < iVar7);
            }
        }
        else {
            // Nonzero-return: cell is blocked — invalidate one stored-path edge
            iVar7 = ZoneMap__CellToZoneIndex(param_1 + 0x70);
            PathfinderClass__InvalidateZoneEdge(
                *(ushort *)(DAT_0087f858 + iVar7*10 + iVar10*2),   // current_zone
                iVar10);   // level
        }

        // Teardown temp vector (free if owns-flag set)
        // Advance to next level slot
        iVar10 = iVar10 + 1;
        piVar11 = piVar11 + 6;   // advance 6 ints = 0x18 bytes
    } while (iVar10 < 3);
}
```

---

## Behavioral analysis

### Step-by-step execution

1. **Cell → zone index** (`ZoneMap__CellToZoneIndex(param_1+0x70)`)
   - Converts the stored current-cell at `PathfinderClass+0x70` into a zone-table index.
   - `DAT_0087f858` is a global runtime array; each cell record is 10 bytes; 2 bytes per
     level: `record[level*2]` = zone id `u16` (verified: `read_memory 0x0087f858` returns
     zeros at startup; populated at map load by zone builder).

2. **Level loop** (`iVar10 = 0..2`)
   - Iterates the 3 hierarchy levels.
   - `piVar11 = param_1 + 0x7c` advances by `+6` ints = 0x18 bytes per iteration, indexing
     the per-level exclusion vector objects at `+0x7c`, `+0x94`, `+0xac`.

3. **Temp vector initialization**
   - Stack-local vector struct: vtable at `PTR_FUN_007e3844` (verified via `read_memory
     0x007e3844` → `00dc4200 b0d84200`); capacity 10 (`local_4 = 10`).
   - `FUN_0042dd60` is the sub-struct allocator (same one used by `PathfinderClass__Constructor`
     for the 3 path-record sub-structs).
   - Teardown vtable at `PTR_FUN_007e3824` (verified via `read_memory 0x007e3824` →
     `b0db4200 b0d84200`); calls `FUN_007c8b3d` to free if owns-flag is set.

4. **`ZoneMap__FloodFillReachableZones`** (out-of-scope, zone-system)
   - Returns `'\0'` (zero) if the current cell is **locally reachable** within the level's
     flood block; returns nonzero if the cell is **blocked/split** from neighbors.
   - Inputs: `CellClass*` for current cell, level index, temp vector pointer, mover pointer.
   - On zero return: fills `local_14` (vector data pointer) with `u16` neighbor zone ids
     that were reached during flood; `local_8` = count.

5. **Zero-return branch — soft-split exclusions**
   - Iterates collected neighbor zones backward.
   - For each `neighbor_zone != current_zone`: sorts `(lo, hi)` → packs as `lo<<16|hi`;
     appends into the per-level exclusion vector at `piVar11` (capacity-grows if needed).
   - Can append multiple exclusions for one level in a single call.

6. **Nonzero-return branch — hard-split via `InvalidateZoneEdge`**
   - Re-reads `current_zone` from `DAT_0087f858` (same formula; second read uses a fresh
     `CellToZoneIndex` call).
   - Calls `PathfinderClass__InvalidateZoneEdge(this, current_zone, level)`.
   - That function (see `fn-pathfinder_invalidate_zone_edge.md`) reads the stored
     `Zone_precheck` path for this level from `PathfinderClass+0xbc + level*1000`;
     selects the adjacent edge around the current zone; appends it and common-neighbor
     exclusions; may clear `PathfinderClass+0x38` if no valid path edge exists.

---

## Struct field accesses

| Owner | Offset | Meaning | Verified |
|-------|--------|---------|---------|
| `PathfinderClass` | `+0x38` | hierarchy-valid byte; cleared by `InvalidateZoneEdge` when no path edge | decompile `0x0042CF80` reads/clears `param_1+0x38` |
| `PathfinderClass` | `+0x70` | stored current cell (zone lookup input) | decompile `0x0042CCD0` `param_1+0x70` passed to `CellToZoneIndex` |
| `PathfinderClass` | `+0x7c` | per-level exclusion vector, level 0 | decompile `piVar11 = (int*)(param_1+0x7c)` |
| `PathfinderClass` | `+0x94` | per-level exclusion vector, level 1 | `piVar11+6` = `+0x7c+0x18` |
| `PathfinderClass` | `+0xac` | per-level exclusion vector, level 2 | `piVar11+12` = `+0x7c+0x30` |
| `PathfinderClass` | `+0xbc + level*1000` | stored `Zone_precheck` path for level | decompile `0x0042CF80` `param_1+0xbc+param_3*1000` |
| `PathfinderClass` | `+0xc74 + level*4` | stored path length for level | decompile `0x0042CF80` `param_1+0xc74+param_3*4` |
| `DAT_0087f858` | global | per-cell zone ids: `[cell_index*10 + level*2]` = `u16` zone id | `read_memory 0x0087f858` (zeros at startup; runtime-populated) |
| `DAT_0087f878` | global | hierarchy graph adjacency base; stride `level*0x18` | decompile `0x0042CF80` `&DAT_0087f878 + iVar13` |

> Frame note: `param_1` is `int` (not `int*`), so offsets are direct byte offsets.

---

## Globals / Enums / INI

| Symbol | Address | Role |
|--------|---------|------|
| `DAT_0087f858` | `0x0087f858` | Zone-map per-cell level zone-id table; 10 bytes per cell, 2 bytes per level (u16) |
| `DAT_0087f878` | `0x0087f878` | Zone hierarchy adjacency graph base; stride 0x18 per level; each record has neighbor list |
| `PTR_FUN_007e3844` | `0x007e3844` | Vtable for growing temp `u16` vector (`00dc4200` = `FUN_0042DC00`) |
| `PTR_FUN_007e3824` | `0x007e3824` | Vtable for resetting/freeing temp vector (`b0db4200` = `FUN_0042DBB0`) |

No INI keys are read directly by this function.

---

## Callees

| Function | Address | Role | Out-of-scope? |
|----------|---------|------|--------------|
| `ZoneMap__CellToZoneIndex` | (zone-system) | Convert cell to zone table index | Yes — zone-system |
| `MapClass__Get_CellClass` | (cell-system) | Get `CellClass*` for cell coord | Yes — cell-system |
| `ZoneMap__FloodFillReachableZones` | `0x005840C0` | Flood-fill local block; returns 0=reachable, nonzero=blocked | Yes — zone-system |
| `PathfinderClass__InvalidateZoneEdge` | `0x0042CF80` | Select and append one stored-path edge to exclusion list; clear hierarchy-valid flag if no path | In-scope (task #12) |
| `FUN_0042dd60` | `0x0042dd60` | Sub-struct allocator for temp vector | In-scope (task #23) |
| `FUN_007c8b3d` | (runtime) | Free heap buffer if owns-flag set | Out-of-scope — runtime utility |

---

## Relationship to A* retry loop

```
AStar_pathfind_search (0x0042C900)
  └─ if marker-gated AStar_main_loop returns 0 AND hierarchy enabled:
       1. CALL PathfinderClass__UpdateHierarchicalEdges  ← this function
       2. CALL PathfinderClass__Reset
       3. re-read PathfinderClass+0x38
       4. if +0x38 still set: re-run Zone_precheck with updated exclusions → retry A*
```

Key constraint: the handoff carries **no A* frontier context** — only `(this, mover)`.
The function recomputes exclusions entirely from stored pathfinder state + zone system.
This was verified by the call site `0x0042CC79: CALL 0x0042CCD0` which pushes only mover
and uses `ECX=PathfinderClass` (verified in existing doc, consistent with live decompile).

---

## YELLOW — Unverified

- The exact meaning of `local_14` in the zero-return branch: Ghidra shows it as a local
  variable reused as both a `char` flag and an `int` buffer pointer — Ghidra's decompile
  may not have reconstructed the temp vector data-pointer field correctly. The behavior
  (iterate neighbor zones from the vector) is clear but the exact field slot in the
  stack-local vector struct is not independently confirmed.
- Runtime frequency of zero-return vs nonzero-return branches across stock YR maps is not
  determinable from static analysis alone.
- `PathfinderClass+0x7c` exclusion vector layout (the 6-int stride): the sub-struct at
  `+0x7c` has fields `[0]=count, [1]=data_ptr, [2]=capacity, ...`. The exact field order
  matches `FUN_0042DD60`'s constructor pattern but is not independently audited here;
  see constructor doc (`fn-pathfinder_constructor.md`) for the definitive layout.
