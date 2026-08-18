# FUN_0042DC50 — Decode Doc
**Address:** `0x0042DC50`
**Proposed Ghidra label:** `PathfinderHeapVec__Init`
**Active in YR:** Yes — called by `PathfinderClass__Constructor @ 0x0042A6D0` to initialize per-level exclusion/heap vectors. Also called by `FUN_00770270` (excluded per manifest: "phase0-drop: called by CDFileClass__Constructor, not pathfinding").

## Summary

Constructor/initializer for the inline heap-vector struct used as per-level exclusion lists and open-set heaps in PathfinderClass. Sets the vtable pointer, zeros/sets the count/capacity/ownership fields, and optionally allocates a backing buffer.

Takes three arguments: the struct pointer (`param_1`), a desired capacity (`param_2`), and an optional external buffer (`param_3`). If `param_3` is provided and `param_2 > 0`, the struct uses the external buffer (no ownership). If `param_3 == 0` and `param_2 > 0`, allocates `capacity * 4` bytes via `operator_new` and takes ownership.

**Proposed label:** `PathfinderHeapVec__Init` (companion to `PathfinderHeapVec__Clear @ 0x0042D540`).

---

## Signature

```c
undefined4 * __thiscall FUN_0042dc50(
    undefined4 *param_1,    // PathfinderHeapVec* this
    int          param_2,   // capacity (number of int entries)
    int          param_3    // external buffer ptr (0 = allocate internally)
)
```

Verified via `decompile_function 0x0042DC50`.

---

## Decompilation

```c
undefined4 * __thiscall FUN_0042dc50(undefined4 *param_1, int param_2, int param_3)
{
    param_1[1] = 0;                                  // +0x04: data_ptr = null
    param_1[2] = param_2;                            // +0x08: capacity = param_2
    *(undefined1 *)(param_1 + 3) = 1;               // +0x0C: init_flag = 1
    *(undefined1 *)((int)param_1 + 0xd) = 0;        // +0x0D: ownership_flag = 0
    *param_1 = &PTR_FUN_007e37ec;                   // +0x00: vtable = &PTR_FUN_007e37ec

    if (param_2 != 0) {
        if (param_3 != 0) {
            param_1[1] = param_3;                   // +0x04: use external buffer (no alloc)
            return param_1;
        }
        pvVar1 = operator_new(param_2 << 2);        // allocate capacity*4 bytes
        param_1[1] = pvVar1;                        // +0x04: data_ptr = allocated buffer
        *(undefined1 *)((int)param_1 + 0xd) = 1;   // +0x0D: ownership_flag = 1 (owns buffer)
    }
    return param_1;
}
```

Verified via `decompile_function 0x0042DC50`.

---

## Behavioral Analysis

### PathfinderHeapVec struct layout (confirmed across this function and FUN_0042D540)

| Offset | Type | Field | Confirmed |
|---|---|---|---|
| `+0x00` | `void*` | vtable pointer | This function: `*param_1 = &PTR_FUN_007e37ec` |
| `+0x04` | `int*` | data buffer pointer | This function: `param_1[1]` |
| `+0x08` | `int` | capacity | This function: `param_1[2] = param_2` |
| `+0x0C` | `char` | init_flag | This function: `*(param_1+3) = 1` (byte at offset 12) |
| `+0x0D` | `char` | ownership_flag | This function + FUN_0042D540: set/cleared at `(int)param_1 + 0xD` |
| `+0x10` | `int` | element count | FUN_0042D540: `*(param_1+0x10) = 0`; FUN_0042D830: `param_1[4]` (count) |

### Initialization paths

1. **Zero capacity (`param_2 == 0`)**: vtable set, all fields zeroed/flagged, no alloc. Vector is empty with no buffer.

2. **External buffer (`param_2 > 0` AND `param_3 != 0`)**: vtable set, capacity set, data_ptr = param_3 (external), ownership_flag = 0. No allocation. Used when the caller provides a stack-allocated or static buffer.

3. **Internal allocation (`param_2 > 0` AND `param_3 == 0`)**: vtable set, capacity set, data_ptr = `operator_new(capacity * 4)`, ownership_flag = 1. Heap-allocated buffer owned by this struct.

### Buffer element size

`operator_new(param_2 << 2)` = `capacity * 4` bytes → each element is 4 bytes (one `uint32` packed edge key or one `int` node index). Consistent with `FUN_0042D830` which writes `uint` values at `data[count]` (4 bytes each).

### vtable pointer

`PTR_FUN_007e37ec` — points to a function pointer table. The vtable slot `+0x0C` on the vector is the clear function (called by `AStar_pathfind_search` to reset exclusion vectors). Cross-reference: `FUN_0042D540` is the likely `vtable+0x0C` target.

---

## Struct Field Accesses

All confirmed from `decompile_function 0x0042DC50`:

| Offset | Access | Operation |
|---|---|---|
| `param_1+0x00` | write | Sets vtable = `&PTR_FUN_007e37ec` |
| `param_1+0x04` | write | Sets data_ptr (null, external, or allocated) |
| `param_1+0x08` | write | Sets capacity |
| `param_1+0x0C` | write | Sets init_flag = 1 (byte) |
| `param_1+0x0D` | write | Sets ownership_flag (0 or 1) |

---

## Callers

| Caller | Address | Notes |
|---|---|---|
| `PathfinderClass__Constructor` | `0x0042A6D0` | Primary caller; initializes all heap/exclusion vector fields |
| `FUN_00770270` | `0x00770270` | CDFileClass__Constructor related; excluded per manifest |

Verified via `get_function_callers 0x0042DC50`.

---

## Callees

| Callee | Address | Role |
|---|---|---|
| `operator_new` | `0x007C8E17` | C++ heap allocator; excluded per manifest (runtime) |

Verified via `get_function_callees 0x0042DC50`.

---

## INI Keys

None. Pure memory management / struct initialization.

---

## Self-Proof (3 Claims Verified This Session)

1. **Buffer allocation: `operator_new(capacity * 4)` — one uint32 per element** — confirmed from decompile: `pvVar1 = operator_new(param_2 << 2)` where `param_2` is the capacity. Left-shift by 2 = multiply by 4. Verified via `decompile_function 0x0042DC50`.

2. **Callers: `PathfinderClass__Constructor @ 0x0042A6D0` and `FUN_00770270 @ 0x00770270`** — confirmed via `get_function_callers 0x0042DC50`. Sole in-scope caller is PathfinderClass__Constructor; `FUN_00770270` is excluded per manifest.

3. **Ownership flag at `+0x0D` set to 1 only when internal allocation is performed** — confirmed from decompile: `*(undefined1 *)((int)param_1 + 0xd) = 1` appears ONLY in the `param_3 == 0` branch after `operator_new`. In the external-buffer branch (`param_3 != 0`), ownership_flag remains 0 (set earlier by `*(undefined1 *)((int)param_1 + 0xd) = 0`). This matches `FUN_0042D540`'s conditional free: `if (data_ptr != null && ownership_flag != 0) free(data_ptr)`. Verified via `decompile_function 0x0042DC50`.

---

## YELLOW (Unverified)

| Item | Why unverified | How to verify |
|---|---|---|
| `PTR_FUN_007e37ec` vtable contents | The vtable pointer is set but its slot contents (which functions are at `+0x0C` etc.) were not read this session | `read_memory 0x007E37EC` (or wherever `PTR_FUN_007e37ec` resolves) to get slot addresses |
| `+0x0C` init_flag semantics | Set to 1 here but never read within this function; exact role is unknown | Trace all reads of `param_1+0x0C` across callers of both this function and FUN_0042D540 / FUN_0042D830 |
| `FUN_00770270` call context | CDFileClass__Constructor related (manifest exclusion); why it initializes a PathfinderHeapVec is unclear | Decompile FUN_00770270 — out of scope for this decode |

---

## Companion Docs

- `fn-fn_42d540.md` — `PathfinderHeapVec__Clear @ 0x0042D540`: companion destructor/clear
- `fn-fn_42d830.md` (Task #24) — `FUN_0042D830`: vector push helper, uses same struct layout (`+0x04`=data, `+0x08`=capacity, `+0x10`=count)
