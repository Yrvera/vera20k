# FUN_0042DD60 — Decode Doc
**Proposed Ghidra label:** `U16Vec__Constructor`
**Address:** `0x0042DD60`

## Summary

Initializes a small dynamic `u16` vector object in-place. The vector struct layout
(as revealed by this constructor) is:

| Field offset (×4 from base) | Byte offset | Type | Meaning |
|-----------------------------|-------------|------|---------|
| `[0]` | +0x00 | ptr | vtable pointer (`PTR_FUN_007e3824`) |
| `[1]` | +0x04 | ptr | data buffer pointer (NULL or allocated) |
| `[2]` | +0x08 | int | capacity (max elements) |
| `[3]` high byte (byte +0x0c) | +0x0C | byte | always 1 (alignment/flags?) |
| byte at `+0x0D` | +0x0D | byte | owns-flag: 1 = owns heap buffer, 0 = external |

The constructor:
1. Zeros `param_1[1]` (data ptr = NULL).
2. Sets capacity = `param_2`.
3. Sets byte at `+0x0C = 1`.
4. Sets vtable to `PTR_FUN_007e3824`.
5. If `param_2 == 0`: returns immediately (no allocation needed).
6. If `param_3 != 0` (pre-allocated buffer provided): stores `param_3` as data ptr,
   **does not set owns-flag**, returns.
7. If `param_3 == 0` (no pre-allocated buffer): calls `operator_new(param_2 * 2)`
   to allocate `capacity * sizeof(u16)` bytes, stores pointer, sets owns-flag = 1.

**Active in YR: Yes.** Called by `PathfinderClass__UpdateHierarchicalEdges @
0x0042CCD0` (in-scope, task #11) to initialize the per-level temp `u16` vector and
by `ZoneMap__FloodFillReachableZones @ 0x005840C0` (verified via `get_function_callers
0x0042DD60`).

---

## Decompilation excerpt

Source: `decompile_function 0x0042DD60`

```c
undefined4 * __thiscall FUN_0042dd60(undefined4 *param_1,  // vec* (this)
                                      int param_2,          // capacity
                                      int param_3)          // pre-alloc buffer ptr (or 0)
{
    param_1[1] = 0;                          // data = NULL
    param_1[2] = param_2;                    // capacity
    *(byte*)((int)param_1 + 0xc) = 1;       // byte at +0xC = 1
    *(byte*)((int)param_1 + 0xd) = 0;       // owns-flag = 0 initially
    *param_1 = &PTR_FUN_007e3824;            // vtable

    if (param_2 != 0) {
        if (param_3 != 0) {
            param_1[1] = param_3;            // use pre-alloc buffer
            return param_1;
        }
        void *buf = operator_new(param_2 * 2);  // alloc capacity*sizeof(u16)
        param_1[1] = buf;
        *(byte*)((int)param_1 + 0xd) = 1;   // owns-flag = 1
    }
    return param_1;
}
```

---

## Behavioral analysis

### Vector struct layout

The struct is 4 ints (0x10 bytes) + 2 status bytes:
- `[0]` = vtable ptr → `PTR_FUN_007e3824` (the destructor/reset vtable; verified from
  `PathfinderClass__UpdateHierarchicalEdges` decompile which sets `local_18 =
  &PTR_FUN_007e3824` for the temp vector)
- `[1]` = data buffer ptr (pointer to `u16[]`)
- `[2]` = capacity (number of `u16` elements that fit)
- byte `+0x0C` = always 1 (set before vtable; purpose unclear — possibly the
  `local_b` flag seen in `UpdateHierarchicalEdges` decompile as the "initialized" flag)
- byte `+0x0D` = owns-flag: 1 if this struct must `free()` the data buffer on
  destruction, 0 if the buffer is externally owned

### Two initialization modes

**Mode A — owned allocation** (`param_3 == 0`): the constructor calls `operator_new`
and takes ownership. The teardown code (called with vtable `PTR_FUN_007e3824` or
`PTR_FUN_007e3844`) calls `FUN_007c8b3d` (a `free()` wrapper) when owns-flag is set.
This is the mode used by `UpdateHierarchicalEdges` for its stack-local temp vector with
capacity 10.

**Mode B — external buffer** (`param_3 != 0`): the constructor stores the provided
buffer pointer without allocating or setting owns-flag. The external caller is
responsible for the buffer lifetime.

### Allocation size

`operator_new(param_2 * 2)` — each element is `u16` (2 bytes), so total allocation =
`capacity * 2` bytes. Confirmed by the `× 2` multiplier in the decompile.

---

## Struct field accesses

| Offset | Type | Meaning | Verified |
|--------|------|---------|---------|
| `+0x00` | ptr | vtable (`PTR_FUN_007e3824`) | decompile `*param_1 = &PTR_FUN_007e3824` |
| `+0x04` | ptr | data buffer | decompile `param_1[1] = pvVar1` |
| `+0x08` | int | capacity | decompile `param_1[2] = param_2` |
| `+0x0C` | byte | flag (always 1 here) | decompile `*(byte*)(param_1+3) = 1` → byte offset = `int* + 3 words... wait, see note` |
| `+0x0D` | byte | owns-flag | decompile `*(byte*)((int)param_1+0xd) = 0/1` |

> Note: `param_1` is `undefined4*` (4-byte pointer). `param_1 + 3` = byte offset `0x0C`
> (i.e., `[3]*4=0xC`). `(int)param_1 + 0xd` = byte offset `0x0D`. Both confirmed by
> decompile `*(undefined1 *)(param_1 + 3) = 1` and `*(undefined1 *)((int)param_1 + 0xd) = 0`.

---

## Globals

| Symbol | Address | Role |
|--------|---------|------|
| `PTR_FUN_007e3824` | `0x007e3824` | Destructor/reset vtable for this vector type (verified in `fn-pathfinder_update_hier_edges.md` via `read_memory 0x007e3824`) |

---

## Callers

Verified via `get_function_callers 0x0042DD60` — 3 callers:

| Caller | Address | Notes |
|--------|---------|-------|
| `PathfinderClass__UpdateHierarchicalEdges` | `0x0042CCD0` | Initializes temp u16 vector with capacity 10 for flood-fill neighbor collection |
| `ZoneMap__FloodFillReachableZones` | `0x005840C0` | Initializes temp vectors for zone flood-fill tracking |
| `FUN_00584550` | `0x00584550` | Unknown caller |

---

## Callees

| Function | Address | Role | Scope |
|----------|---------|------|-------|
| `operator_new` | (runtime) | C++ heap allocation | Out-of-scope — runtime |

---

## YELLOW — Unverified

- Byte at `+0x0C` (set to 1 always): its exact semantics are unclear. In
  `UpdateHierarchicalEdges` the `local_b` variable (which corresponds to this byte
  based on layout) is checked before calling `FUN_007c8b3d` for teardown. It may be
  an "is initialized" flag or a "can grow" flag. Not independently traced to a reader.
- Whether Mode B (external buffer) is used by any of the 3 known callers: all three
  known callers pass the stack-local struct pointer as `param_1`; the `param_3`
  (pre-alloc) argument would need caller decompile to confirm. Not traced here.
