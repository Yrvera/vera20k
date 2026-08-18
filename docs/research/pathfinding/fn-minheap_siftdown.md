# MinHeap__SiftDown — Decode Doc
**Proposed Ghidra label:** MinHeap__SiftDown

## Summary

`MinHeap__SiftDown` at `0x0042DCA0` implements a standard binary min-heap sift-down
operation. It is used by `Zone_precheck` to restore heap order after extracting the
minimum element from the zone-level open-list heap.

The function takes a starting index (`param_2`) and sifts the element down toward
the leaves, swapping with the smaller child until the heap property is restored:
every parent is ≤ both its children (by the float at `node + 0x08`, the f-cost).

The heap uses **1-based indexing**: element at index `i` has children at `2*i`
and `2*i+1`. Index 0 is unused (or stores `count`). The node-pointer array is
stride-4 (pointer-sized entries).

## Active in YR

**Yes.** Called by `Zone_precheck @ 0x0042C290` (task #16, completed), which is on
the live `AStar_pathfind_search → Zone_precheck` call chain (verified via
`get_function_callers 0x0042DCA0`).

## Callers

Verified via `get_function_callers 0x0042DCA0`:

| Caller | Address | Role |
|--------|---------|------|
| `Zone_precheck` | `0x0042C290` | Zone-level A* heap pop; calls SiftDown to restore order |

## Callees

Verified via `get_function_callees 0x0042DCA0`: no callees. All operations inline.

## Decompilation analysis

Source: `decompile_function 0x0042DCA0`.

### Signature

```c
void __thiscall MinHeap__SiftDown(int *param_1,   // heap struct ptr (this)
                                   int  param_2)   // starting index to sift from
```

### Heap struct layout (inferred from decompile)

`param_1` points to a heap struct with at least 3 words:

| Word offset | Byte offset | Type | Name | Notes |
|-------------|-------------|------|------|-------|
| `[0]` | `+0x00` | `int` | `count` | Current number of elements in heap |
| `[1]` | `+0x04` | `int` | `capacity` | (not accessed here) |
| `[2]` | `+0x08` | `int*` | `array` | Pointer to node-pointer array (1-indexed, stride 4) |

This matches the heap struct observed in `PathfinderClass__Reset` (primary heap at
`Pathfinder+0x14`, secondary heap at `Pathfinder+0x68`): `heap[0]` = count,
`heap[2]` = array pointer.

### Comparison key

```c
*(float *)(*(int *)(param_1[2] + i * 4) + 8)
```

The array at `param_1[2]` holds pointers to zone nodes. Each zone node has a float
at offset `+0x08`. This is the **f-cost** (priority value). Smaller f-cost = higher
heap priority (min-heap by f).

Consistent with the cell-level A* node struct where `node[2]` = f-cost at byte `+0x08`.

### Algorithm (1-based indexing, sift-down)

```c
void MinHeap__SiftDown(heap, start):
    // Find smallest among start, left=2*start, right=2*start+1
    left  = start * 2;
    right = start * 2 + 1;

    // Initial: compare start vs left child
    if (heap.count < left ||
        f(heap.array[start]) <= f(heap.array[left])):
        smallest = start    // start is smaller or left doesn't exist
    else:
        smallest = left     // left is smaller

    // Compare smallest vs right child
    if (right <= heap.count &&
        f(heap.array[right]) < f(heap.array[smallest])):
        smallest = right

    // If smallest is not start, swap and continue down
    if (smallest != start):
        do:
            child = smallest
            swap(heap.array[start], heap.array[child])
            start = child

            // Recompute children of new position
            left  = child * 2;
            right = child * 2 + 1;

            smallest = child
            if (child * 2 <= heap.count &&
                f(heap.array[child*2]) < f(heap.array[child])):
                smallest = child * 2
            if (right <= heap.count &&
                f(heap.array[right]) < f(heap.array[smallest])):
                smallest = right
        while (smallest != child)
```

**Direction**: top-down (root toward leaves). This is sift-DOWN, confirmed by:
- Starting at `param_2` (the newly-placed element at the root after pop)
- Moving to the smaller child (`2*i` or `2*i+1`)
- Continuing until no child is smaller

**Not** sift-up (which would compare to parent at `i/2` and move toward root).

### Indexing confirmation

From the decompile:
```c
iVar1 = param_2 * 2 + 1;      // right child = 2i+1
iVar4 = param_2 * 2;          // left child  = 2i
```

Left child is `2*i`, right child is `2*i+1`. Array access is `array[i*4]` (stride 4),
confirming 1-based: index 0 is the sentinel/unused slot (array[0] = 0 cleared on reset),
element 1 is the heap root.

### Comparison operators

The initial comparison uses `<=` (not `<`) for parent vs left:

```c
*(float *)(node[start] + 8) <= *(float *)(node[left] + 8)
```

This means: if parent.f ≤ left.f, parent wins (tie-break favors parent, preserving
stability for equal-cost nodes). The right-child comparison uses strict `<`:

```c
*(float *)(node[right] + 8) < *(float *)(node[smallest] + 8)
```

So right child only wins on strict improvement. Overall: left child is slightly
preferred over right on exact ties.

## Self-proof (3 claims re-verified)

**Claim 1:** Sole caller is `Zone_precheck @ 0x0042C290`.
Verified via `get_function_callers 0x0042DCA0` → `Zone_precheck @ 0042c290` only.

**Claim 2:** No callees — all operations inline.
Verified via `get_function_callees 0x0042DCA0` → "No callees found."

**Claim 3:** Heap struct `[2]` is the array pointer; comparison key is float at
`node + 0x08` (f-cost). Confirmed directly from decompile:
`*(float *)(*(int *)(param_1[2] + i * 4) + 8)` — `param_1[2]` = array pointer,
`i * 4` = 1-indexed stride, `+ 8` = node f-cost field.

## Heap struct fields used

| Expression | Meaning |
|-----------|---------|
| `*param_1` = `param_1[0]` | `heap.count` — number of live elements |
| `param_1[2]` | `heap.array` — pointer to node-pointer array (1-indexed) |
| `heap.array[i * 4]` | node pointer at 1-based index `i` |
| `node + 8` | float f-cost (comparison key) |

## Control flow summary

```
MinHeap__SiftDown(heap, i)
├── left = 2*i, right = 2*i+1
├── smallest = (left > count || f[i] <= f[left]) ? i : left
├── if right <= count && f[right] < f[smallest]: smallest = right
├── if smallest == i: return (heap already valid)
└── do:
    ├── swap heap.array[i] ↔ heap.array[smallest]
    ├── i = smallest
    ├── left = 2*i, right = 2*i+1
    ├── smallest = i
    ├── if left <= count && f[left] < f[i]: smallest = left
    ├── if right <= count && f[right] < f[smallest]: smallest = right
    └── while smallest != i
```

## Out-of-scope refs

| Symbol | Address | Reason out-of-scope |
|--------|---------|---------------------|
| `Zone_precheck` | `0x0042C290` | task #16 (completed) |

## YELLOW — Unverified

- `param_1[1]` (capacity at `+0x04`): the decompile does not access `param_1[1]`
  (it only reads `param_1[0]` = count and `param_1[2]` = array). The capacity field
  is inferred from the heap insert logic observed in `Zone_precheck` and
  `PathfinderClass__Reset`, not from SiftDown itself.
- Zone-node f-cost at `+0x08`: the comparison reads `node + 8` as a float. For zone
  nodes this is the zone-level g+h estimate, consistent with cell-level node `[2]` at
  `+0x08`. The zone node struct layout was not separately verified via
  `decompile_function` on the allocation site in `Zone_precheck`.
