# PathfinderClass__InvalidateZoneEdge — Decode Doc
**Address:** `0x0042CF80`
**Proposed Ghidra label:** `PathfinderClass__InvalidateZoneEdge`
**Active in YR:** Yes — reached via `FootClass__Run_AStar → AStar_pathfind_search → UpdateHierarchicalEdges` when hierarchical cell A* fails and flood-fill returns nonzero. No TS-only option gate in this caller chain.

## Summary

Marks a zone edge as "dirty" (adds it to the per-level exclusion vector) when the hierarchical A* retry loop determines that a path zone is unreachable. Called after `ZoneMap__FloodFillReachableZones` returns nonzero to indicate that two zones are no longer reachable from each other.

The function performs two exclusion appends (in order):
1. **Direct path edge**: the zone edge immediately adjacent to the failing zone in the stored `Zone_precheck` path.
2. **Common-neighbor edges**: for every zone that is a common neighbor of the direct edge's two endpoints, appends the `(earlier_endpoint, common_neighbor)` edge as an additional exclusion.

No duplicate suppression: if the same packed edge is appended multiple times across retries, all copies survive until the next `AStar_pathfind_search` entry clears the exclusion vectors.

---

## Signature

```c
void __thiscall PathfinderClass__InvalidateZoneEdge(
    int param_1,    // PathfinderClass* this
    uint param_2,   // zone ID to find in stored path
    int param_3     // hierarchy level / layer index
)
```

Verified via `decompile_function 0x0042CF80`.

---

## Decompilation Excerpt

```c
iVar4 = *(int *)(param_1 + 0xc74 + param_3 * 4);  // zone path count for level
if (iVar4 < 2) {
    *(undefined1 *)(param_1 + 0x38) = 0;  // clear hierarchy-valid flag
    return;
}
// scan stored zone path for param_2 (zone ID)
puVar11 = (ushort *)(param_1 + 0xbc + param_3 * 1000);
while (*puVar11 != param_2) {
    iVar8 = iVar8 + 1;
    puVar11 = puVar11 + 1;
    if (iVar4 <= iVar8) {
        *(undefined1 *)(param_1 + 0x38) = 0;
        return;
    }
}
// select direct edge endpoints
if (iVar8 == iVar4 + -1) {
    // current zone is last in path: use path[i-1] as early endpoint
    uVar2 = *(ushort *)(param_1 + 0xbc + iVar8 * 2);   // late = path[i]
    uVar3 = *(ushort *)(param_1 + 0xba + iVar8 * 2);   // early = path[i-1]
} else {
    // current zone is not last: use path[i+1] as late endpoint
    uVar3 = *(ushort *)(param_1 + 0xbc + iVar8 * 2);   // early = path[i]
    uVar2 = *(ushort *)(param_1 + 0xbe + iVar8 * 2);   // late = path[i+1]
}
// canonicalize: min<<16|max, then append direct edge
if (uVar6 != uVar9) {
    param_2 = uVar10 << 0x10 | param_2;
    FUN_0042d830(&param_2);  // append direct edge to exclusion vector
}
// common-neighbor scan
iVar4 = *(int *)(&DAT_0087f878 + iVar13) + uVar9 * 0x24;  // late endpoint record
param_2 = *(int *)(&DAT_0087f878 + iVar13) + uVar6 * 0x24; // early endpoint record
// outer loop: backward over late endpoint adjacency
iVar8 = *(int *)(iVar4 + 0x10);
while (iVar8 = iVar8 + -1, -1 < iVar8) {
    uVar2 = *(ushort *)(*(int *)(iVar4 + 4) + iVar8 * 8);  // candidate neighbor
    if (uVar2 != early_endpoint) {
        // inner loop: backward over early endpoint adjacency
        iVar14 = *(int *)(param_2 + 0x10) + -1;
        // if candidate found in early adjacency: append (early, candidate) edge
    }
}
```

Verified via `decompile_function 0x0042CF80`.

---

## Behavioral Analysis

### Step 1 — Guard: path length < 2

Reads `PathfinderClass+0xC74 + level*4` (zone path count for this level).
If count < 2, sets `PathfinderClass+0x38 = 0` (clears hierarchy-valid flag) and returns immediately without appending anything.

**Observable effect:** search retry is abandoned for this level; caller disables hierarchy for subsequent retries.

### Step 2 — Zone scan: find current zone in stored path

Scans the ushort array at `PathfinderClass+0xBC + level*1000` from index 0 upward.
If `param_2` (zone ID) is not found before exhausting the path length, sets `+0x38 = 0` and returns.

The `iVar8 != -1` guard also clears and returns; in practice the normal scan either finds the zone or hits the length guard first.

### Step 3 — Direct edge endpoint selection

Two cases based on whether the found index `i` equals `path_length - 1`:

- **i == last index**: direct edge = `(path[i-1], path[i])`. `early = path[i-1]`, `late = path[i]`.
- **i < last index**: direct edge = `(path[i], path[i+1])`. `early = path[i]`, `late = path[i+1]`.

Endpoint IDs are then canonicalized: `min(early, late) << 16 | max(early, late)`.

If the two endpoints are the same zone (self-edge guard), direct append is skipped.

### Step 4 — Direct edge append (first append)

Calls `FUN_0042D830` (vector push helper) with the canonicalized edge key.
This append happens BEFORE any graph adjacency reads. Verified via decompile: `DAT_0087f878` is not loaded until after the `FUN_0042D830` call.

`FUN_0042D830` appends unconditionally — no duplicate scan. If the same packed key is already in the vector, a duplicate record is added.
Verified via `decompile_function 0x0042D830`.

### Step 5 — Common-neighbor scan (subsequent appends)

Loads the zone graph at `DAT_0087F878 + level*0x18`:
- `late_record = graph + late_endpoint * 0x24`
- `early_record = graph + early_endpoint * 0x24`

**Outer loop**: iterates `late_endpoint` adjacency backward from `(count-1)` to `0`.
Each entry `candidate = late_record.adjacency[outer_idx]`.

Skip condition: if `candidate == early_endpoint`, skip entirely (the direct edge itself is not a common neighbor).

**Inner loop**: iterates `early_endpoint` adjacency backward from `(count-1)` to `0`.
If `early_record.adjacency[inner_idx] == candidate` AND `candidate != early_endpoint` (extra self-edge guard):
- Canonicalize `(early_endpoint, candidate)`: `min << 16 | max`
- Append to level exclusion vector (inlined push at `0x0042D0FA..0x0042D13C`, same capacity/grow logic as `FUN_0042D830`)

Crucially: **only `(early_endpoint, candidate)` is appended, not `(late_endpoint, candidate)`**.

### Step 6 — No duplicate suppression

Both the direct append and the common-neighbor appends use list-append semantics with no scan of existing entries. Duplicate packed keys are permitted — and are possible if the same zone fails across multiple retries within one search call.

### Exclusion lifetime

Exclusion vectors are at `PathfinderClass+0x74 + level*0x18` (and `+0x8C`, `+0xA4` for levels 1 and 2).
They are cleared at `AStar_pathfind_search` entry (loop at `0x0042C912..0x0042C925`).
`PathfinderClass__Reset` (called on each retry at `0x0042CC80`) does NOT clear the exclusion vectors.
Therefore exclusions accumulate across retries within one search call and persist until the next search entry.

Verified via `decompile_function 0x0042C900` and `decompile_function 0x0042A5B0`.

---

## Struct Field Accesses

| Field | Expression | Meaning |
|---|---|---|
| `PathfinderClass+0x38` | `*(param_1 + 0x38) = 0` | Hierarchy-valid flag; cleared when no path edge can be selected |
| `PathfinderClass+0xBC + level*1000` | ushort array scan | Stored Zone_precheck path (ushort zone IDs, start→destination order) |
| `PathfinderClass+0xC74 + level*4` | int read | Zone path count for this level |
| `PathfinderClass+0x74 + level*0x18` | vector object | Per-level exclusion vector (data ptr +4, count +8/+0x10, capacity +0xC/+0x14) |
| `DAT_0087F878 + level*0x18` | global read | Zone edge graph base pointer for this level |
| zone record `+0x04` | ptr read | Adjacency array pointer (ushort zone IDs, stride 8 bytes per entry) |
| zone record `+0x10` | int read | Adjacency entry count |

All field accesses verified via `decompile_function 0x0042CF80`.

---

## Callers

| Caller | Address | Notes |
|---|---|---|
| `PathfinderClass__UpdateHierarchicalEdges` | `0x0042CCD0` | Sole caller; calls this function per level when flood-fill returns nonzero |

Verified via `get_function_callers 0x0042CF80`.

---

## Callees

| Callee | Address | Role |
|---|---|---|
| `FUN_0042D830` | `0x0042D830` | Vector push helper with capacity growth; no duplicate scan |

Common-neighbor append is inlined (same grow/write logic as `FUN_0042D830`, not a separate call).

Verified via `get_function_callees 0x0042CF80`.

---

## Globals / INI Keys

| Symbol | Address | Role | Active in YR |
|---|---|---|---|
| `DAT_0087F878` | `0x0087F878` | Zone edge graph; stride `0x18` per level, `0x24` per zone record | Yes |

No INI keys are read by this function or its direct callee.

---

## Out-of-Scope References

| Symbol | Reason excluded |
|---|---|
| `ZoneMap__FloodFillReachableZones` | zone-system, separately documented |
| `Zone_precheck` | consumer of exclusion vectors, separately documented |
| `PathfinderClass__UpdateHierarchicalEdges` | caller, documented separately in Task #11 |

---

## Tiberian Sun Filter

No TS-only option gate appears in the `FootClass__Run_AStar → AStar_pathfind_search → UpdateHierarchicalEdges → InvalidateZoneEdge` call chain. The only dynamic gates are: hierarchy must be active (`Pathfinder+0x38 == 1`) and cell A* must fail while flood-fill returns nonzero. Both conditions occur in standard YR foot unit pathfinding.

---

## YELLOW (Unverified / Needs Follow-up)

| Item | Why unverified | How to verify |
|---|---|---|
| Zone record adjacency stride (8 bytes per entry) | Assumed from the `iVar8 * 8` load offset pattern; the zone record layout is read-only confirmed here but not fully mapped | `get_struct_layout` on ZoneRecord type or `analyze_data_region 0x0087F878` |
| `iVar8 != -1` guard at `0x0042CFE0` | This secondary guard fires if the scan somehow sets iVar8 to -1 (which the linear scan from 0 cannot do in practice); the exact guard condition is clear but the triggerable path is unclear | Trace callers to see if param_2 could ever be 0xFFFF |

---

## Self-Proof (3 Claims Verified This Session)

1. **Sole caller is `PathfinderClass__UpdateHierarchicalEdges @ 0x0042CCD0`** — confirmed via `get_function_callers 0x0042CF80`. No other callers exist.

2. **Zone path is at `PathfinderClass+0xBC + level*1000` (ushort array, stride 2)**  — confirmed in decompile: `puVar11 = (ushort *)(param_1 + 0xbc + param_3 * 1000)` and `*(ushort *)(param_1 + 0xbc + iVar8 * 2)`. The multiplier 1000 (decimal) = `0x3E8` gives the zone path array base; then each ushort is at +2 per element. Verified via `decompile_function 0x0042CF80`.

3. **`FUN_0042D830` is the sole named callee; common-neighbor append is inlined** — confirmed via `get_function_callees 0x0042CF80` returning only `FUN_0042D830 @ 0042d830`. The common-neighbor append uses the same capacity/grow/write pattern but is inlined at `0x0042D0FA..0x0042D13C` rather than a second call. Verified by cross-referencing `get_function_callees` output against the decompile showing no second call to `FUN_0042D830` in the inner loop.

---

## Cross-References

- `PATHFINDER_INVALIDATEZONEEDGE_COMMON_NEIGHBORS_GHIDRA_REPORT.md` — exhaustive prior analysis; this decode doc is consistent with all findings in sections 3.1–3.7 and 5 of that report.
- Prior doc cross-check: Section 3.3 of prior doc states direct edge is `(path[i-1], path[i])` for last-element case vs `(path[i], path[i+1])` otherwise — confirmed by this session's decompile.
- Prior doc cross-check: Section 3.5 states only `(early_endpoint, candidate)` is appended, not `(late_endpoint, candidate)` — confirmed by this session's decompile at the inner canonicalization block.
