# MapClass::UpdateBridgeZonesHelper — Decode Doc

Address: `0x0056C510`  
Scope: Full function.

## Summary

`MapClass::UpdateBridgeZonesHelper` rebuilds the entire map zone graph after any bridge
state change (collapse, repair, or initial construction). It is the terminal call in every
bridge collapse walker and repair walker, and is also called from bridge damage state
machines, cell zone helpers, and map initialization paths.

The function performs five phases:

1. **Clear all bridge-crossing edge data** from the zone graph's bucket list.
2. **Zone flood-fill** — assigns zone IDs to all cells by flood-filling with
   `MapClass::ZoneFloodFillScanLine`, yielding `param_1+0x4C` total zones.
3. **Bridge-crossing edge insertion** — for each bridge segment (`param_1+0x54` array),
   if the segment is active (`+8 offset flag != 0`), looks up the zone IDs of the two
   endpoints and inserts an edge into the per-bucket adjacency list in the zone graph.
4. **Per-zone adjacency array allocation** — allocates neighbour arrays for each zone.
5. **Passability matrix computation** — for each movement/terrain type (iterating
   `g_PassabilityMatrix` in 8-byte steps), BFS-floods zones that share a passability class,
   writing final reachability entries into `param_1+0x18` passability arrays.

This produces the complete zone-connectivity data that pathfinding consults when determining
whether a unit can reach a destination. When a bridge collapses, zones on either side
become disconnected; when repaired, they reconnect.

## Active in YR

**Yes.** Called from all 4 collapse walkers, all 4 repair walkers, both bridge-damage state
machines, both bridge-destruction state machines, and map-initialization paths (verified via
`get_function_callers 0x0056C510` — 28 distinct callers including all collapse, repair, and
damage paths).

## Decompilation Excerpt

From `decompile_function 0x0056C510`. Function is ~200 lines; key phases excerpted:

```c
uint __fastcall MapClass__UpdateBridgeZonesHelper(int param_1)
{
    // param_1 = MapClass this pointer

    local_48 = 0xffffffff;  // tracks largest zone ID found
    local_28 = (ushort*)0xffffffff;

    // ---- Phase 1: Clear existing bridge-crossing edge data ----
    piVar19 = *(int**)(param_1 + 0x14);   // zone graph bucket list
    // (a) Call zone->vtable+0xC (destructor/clear) for each bucket entry:
    if (0 < piVar19[2]) {
        iVar20 = 0;
        do {
            (**(code**)(*(int*)(*piVar19 + iVar20) + 0xC))();
            iVar17++;
            iVar20 += 0x18;
        } while (iVar17 < piVar19[2]);
    }
    // (b) Free 13 passability arrays at param_1+0x18..+0x48:
    piVar19 = (int*)(param_1 + 0x18);
    iVar17 = 0xD;
    do {
        if (*piVar19 != 0) { FUN_007c8b3d(*piVar19); *piVar19 = 0; }
        piVar19++;
        iVar17--;
    } while (iVar17 != 0);

    // (c) Zero the zone ID array (param_1+0x68 base, param_1+0x6C length):
    for (pbVar5 = *(byte**)(param_1+0x68); pbVar5 < pbVar1; pbVar5 += 4) {
        pbVar5[2] = 0;
        pbVar5[3] = 0;
    }

    // ---- Phase 2: Zone flood-fill ----
    uVar21 = 1;  // zone IDs start at 1
    pbVar5 = *(byte**)(param_1 + 0x68);
    while (pbVar5 < pbVar1) {
        local_28 = (ushort*)(uint)*pbVar5;
        if ((local_28 == (ushort*)0x7) || (*(short*)(pbVar5+2) != 0)) {
            pbVar5 += 4;  // skip already-zoned or sentinel cells
        } else {
            MapClass__ZoneFloodFillScanLine(pbVar5, uVar21, &iStack_34);
            // Track largest zone
            if ((int)puVar14 < (int)puVar6) {
                local_48 = uVar21 & 0xffff;
                puVar14 = puVar6;
            }
            uVar21++;
            pbVar5 += iStack_34 * 4;
        }
    }
    *(uint*)(param_1 + 0x4C) = uVar21 & 0xffff;  // total zone count

    // ---- Phase 3: Bridge-crossing edge insertion ----
    puStack_3c = *(undefined4**)(param_1 + 0x60);  // bridge segment count
    if (-1 < (int)puStack_3c - 1) {
        iVar17 = ((int)puStack_3c - 1) * 0x10;
        do {
            psVar2 = (short*)(*(int*)(param_1 + 0x54) + iVar17);  // bridge segment entry
            if (*(char*)(*(int*)(param_1+0x54) + 8 + iVar17) != '\0') {  // segment active?
                // Look up zone IDs at both bridge endpoint cells:
                iVar7 = *(int*)(param_1+0xF8) + 1 + *(int*)(param_1+0xF4);
                iVar20 = psVar2[1] * iVar7 + (int)*psVar2;   // endpoint 1 zone index
                uVar21 = (uint)*(ushort*)(*(int*)(param_1+0x68) + 2 + iVar20*4);
                iVar20 = psVar2[3] * iVar7 + (int)psVar2[2]; // endpoint 2 zone index
                uVar10 = (uint)*(ushort*)(*(int*)(param_1+0x68) + 2 + iVar20*4);
                if (uVar10 != uVar21) {
                    // Pack the two zone IDs and insert into adjacency bucket:
                    uVar18 = min_zone<<16 | max_zone;
                    uVar21 = (min_zone & 0xF)<<4 | max_zone & 0xF;  // bucket key
                    // Insert edge into bucket piVar19 at [uVar21 * 0x18]
                    ...
                }
            }
            iVar17 -= 0x10;
        } while (puStack_3c-- != 0);
    }

    // ---- Phase 4: Per-zone adjacency array allocation ----
    // Count zone neighbours, allocate arrays, populate them

    // ---- Phase 5: Passability matrix computation ----
    puStack_3c = &g_PassabilityMatrix;
    puStack_40 = (undefined4*)(param_1 + 0x18);  // destination array slot
    do {
        puVar12 = operator_new(*(int*)(param_1+0x4C) << 1);  // zone_count * 2 bytes
        *puStack_40 = puVar12;
        // Mark impassable zones:
        for (iVar17 = 0; iVar17 < zone_count; iVar17++)
            puVar12[iVar17] = (puStack_3c[pvVar9[iVar17]] != 1) ? 1 : 0;
        // BFS flood by passability class:
        for each unvisited zone:
            if (zone_passability_class matches):
                BFS flood all reachable zones with same class
                assign group ID uVar4; uVar4++;
        *puVar12 = 0xFFFF;  // zone 0 always inaccessible
        puStack_3c += 8;    // next passability matrix row
        puStack_40++;       // next destination slot
    } while (puStack_3c < 0x82A734);   // g_PassabilityMatrix end
    ...
    return local_48;  // largest zone ID (or 0xFFFFFFFF if none)
}
```

## Behavioral Analysis

### Phase 1 — Clear

The zone graph is stored at `param_1+0x14` (a pointer to a bucket-list structure). Before
rebuild, each bucket entry gets its clear method called (`vtable+0xC`), and the 13
passability arrays at `param_1+0x18` through `param_1+0x4C` (13 × 4 bytes = 0x34 bytes)
are freed (`FUN_007c8b3d` = `operator delete`). The raw zone cell array at
`param_1+0x68` (array pointer) of length `param_1+0x6C` is zeroed (bytes 2 and 3 of each
4-byte entry, which store the zone ID).

### Phase 2 — Flood-fill

Zone IDs start at 1. Each unvisited cell is flood-filled with `ZoneFloodFillScanLine`,
which assigns contiguous zone IDs to connected regions. The total number of zones ends up
in `param_1+0x4C`. The largest zone's ID is tracked in `local_48` (returned).
`ZoneFloodFillScanLine` also calls itself recursively (verified via
`get_xrefs_to 0x0056CB90`).

### Phase 3 — Bridge-crossing edges

Bridge segments are stored in `param_1+0x54` (array pointer), `param_1+0x60` (count).
Each segment entry is 0x10 bytes (16): endpoint-1 at offsets 0/2, endpoint-2 at offsets
4/6, active flag at offset 8. For each active segment, the function reads the zone IDs
at both endpoint cells, packs them into a 32-bit value `(zone_min<<16 | zone_max)`, and
inserts the pair into the zone graph's adjacency bucket using a bucket key
`(zone_min & 0xF)<<4 | zone_max & 0xF`. This bucket-hash structure avoids O(N²) scanning
of all zone pairs — a departure from the typical gamemd pattern.

### Phase 4 — Per-zone adjacency arrays

After inserting all bridge edges, the function allocates per-zone neighbour arrays by:
1. Counting degree of each zone (iterating all bucket entries).
2. `operator_new`ing an array of ushorts per zone.
3. Populating the arrays with the zone IDs that each zone connects to.

### Phase 5 — Passability matrix

`g_PassabilityMatrix` is an array of passability type descriptors (8 bytes each, ending
before `0x82A734`). For each entry, the function allocates a `zone_count`-length array
of ushorts at the next slot in `param_1+0x18`. It marks impassable zones (where the zone's
terrain type doesn't match the passability class), then BFS-floods all zones reachable by
that movement type, assigning group IDs (2, 3, ...). Zone 0 is always marked `0xFFFF`
(inaccessible sentinel). These arrays are what pathfinding reads to determine zone
reachability — when a bridge is down, zones on opposite sides have different group IDs for
ground-movement passability, so pathfinders do not attempt to route across them.

**Parity note:** The exact passability matrix layout (`param_1+0x18` through `+0x4C`, 13
slots) and the zone cell array format (`param_1+0x68`, `param_1+0x6C`) are
pathfinding-critical. Any difference between gamemd's output here and the Rust port will
manifest as units refusing to cross intact bridges or routing through collapsed ones.

## Struct Field Accesses

`param_1` is `int` — MapClass `this`; direct byte offsets.

| Offset | Field | Role |
|---|---|---|
| `+0x14` | Zone graph bucket list ptr | Adjacency bucket structure |
| `+0x18` .. `+0x48` | Passability arrays (13 slots) | Per-movement-type zone reachability arrays |
| `+0x4C` | Total zone count | Set at end of flood-fill phase |
| `+0x54` | Bridge segment array ptr | Array of 0x10-byte bridge segment entries |
| `+0x60` | Bridge segment count | Number of entries in `+0x54` array |
| `+0x68` | Zone cell array ptr | Per-cell zone ID storage (bytes 2-3 of each 4-byte entry) |
| `+0x6C` | Zone cell array length | Count of entries in `+0x68` array |
| `+0xF4` | Map width minus 1 | Used to compute zone cell index from (X, Y) |
| `+0xF8` | Map stride (width+1) | `*(param_1+0xF8) + 1 + *(param_1+0xF4)` = columns per row |

Bridge segment entry layout (0x10 bytes at `param_1+0x54 + i*0x10`):

| Byte offset | Content |
|---|---|
| 0+1 | Endpoint 1 X (short) |
| 2+3 | Endpoint 1 Y (short) |
| 4+5 | Endpoint 2 X (short) |
| 6+7 | Endpoint 2 Y (short) |
| 8 | Active flag (non-zero = active bridge crossing) |

## Globals Referenced

| Global | Role |
|---|---|
| `g_PassabilityMatrix` | Passability type descriptor table; iterated in 8-byte steps, end at `0x82A734` |
| `MapClass__ZoneFloodFillScanLine` (callee) | Does the actual flood-fill per connected region |
| `FUN_007C8B3D` (callee) | `operator delete` — frees heap allocations |

## Out-of-scope Refs

- `MapClass::ZoneFloodFillScanLine` @ `0x0056CB90` — the per-region flood filler
- `g_PassabilityMatrix` structure — 13 movement type entries, 8 bytes each
- All collapse/repair walkers calling this function (28 total verified callers)
- `MapClass::AssignOrphanedCellZone` @ `0x0056D460` — also calls this helper
- `CCINIClass::Constructor` @ `0x00599650` — calls this at map load; initial zone setup

## Unverified Claims (YELLOW)

- The "13 passability arrays" count is inferred from the `iVar17 = 0xD` loop clearing
  `param_1+0x18` through `+0x48`. The exact movement types those 13 slots correspond to
  are not decoded here — the `g_PassabilityMatrix` entries determine them.
- `param_1+0x54` is identified as a bridge segment array (each entry 0x10 bytes) from the
  stride `iVar17 -= 0x10` and the 4 short fields read from it. The array format may differ
  from this description; `decode-struct-CellClass_BridgeFields` (task #21) may clarify.
- `g_PassabilityMatrix` address range ending near `0x82A734` is inferred from the
  `while (puStack_3c < 0x82A734)` loop condition in the decompilation. The actual static
  address of `g_PassabilityMatrix` was not independently read via `read_memory` in this
  decode — it is identified only by the decompiler's named reference.
- The return value `local_48` (largest zone ID) is not consumed by any caller in the
  bridge-destruction path (all callers are `void` context). Its use case is unclear.
