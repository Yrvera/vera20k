# PathfinderClass__Constructor — Decode Doc
**Proposed Ghidra label:** PathfinderClass__Constructor (already labeled)

## Summary

Initializes the global `PathfinderClass` singleton. Called once at game startup from `0x0040AFA5`
(verified via `get_xrefs_to 0x0042A6D0`). Sets all scalar fields, allocates four large heap
buffers for the open list, closed-path list, closed list, and node pool, then initializes three
sub-structs and three data arrays. This function is the primary source of truth for the
`PathfinderClass` struct layout.

**Active in YR: Yes.** Reachable from game init path; no TS-legacy gate. Single xref from
`0x0040AFA5` (verified via `get_xrefs_to 0x0042A6D0`).

---

## Decompilation excerpt

Source: `decompile_function 0x0042A6D0`

```c
undefined1 * __fastcall PathfinderClass__Constructor(undefined1 *this)
{
    // --- Phase 1: scalar field init ---
    this[0x00] = 0;
    this[0x01] = 0;    // bridge_flank_enable (cleared; set non-zero by external setter)
    this[0x02] = 0;
    this[0x03] = 1;
    *(float*)(this + 0x04) = 1.0f;    // cost_multiplier
    this[0x08] = 1;
    *(u32*)(this + 0x18) = 0;
    *(u32*)(this + 0x1c) = 0;
    *(u32*)(this + 0x20) = 0;
    *(u32*)(this + 0x24) = 0;
    *(u32*)(this + 0x28) = 0xFFFFFFFF;
    *(u32*)(this + 0x2c) = 0xFFFFFFFF;
    this[0x38] = 1;
    *(u32*)(this + 0x3c) = 0;    // urgency

    *(u32*)(this + 0x6c) = 0xFFFFFFFF;
    *(u16*)(this + 0x70) = 0;
    *(u16*)(this + 0x72) = 0;

    // --- Phase 2: 3 sub-structs at +0x74, +0x8c, +0xa4 (stride 0x18) ---
    for (i = 0; i < 3; i++) {
        sub = FUN_0042dc50(this + 0x74 + i*0x18, /*capacity=*/0, /*buf=*/0);
        sub->vtable = PTR_FUN_007e37cc;   // sub-struct vtable
        sub->capacity = 10;               // sub[0x14] = 10
        sub->count    = 0;                // sub[0x10] = 0
    }

    // --- Phase 3: closed list header at +0x14 ---
    closed_hdr = operator_new(0x14);   // 20-byte header
    closed_hdr->count    = 0;          // +0x00
    closed_hdr->capacity = 0x10000;    // +0x04 = 65536
    closed_hdr->buf      = operator_new(0x40004);  // +0x08: 262148 bytes → (65537 slots)
    closed_hdr->unk_0c   = 0;          // +0x0c
    closed_hdr->unk_10   = 0xFFFFFFFF; // +0x10
    // zero all capacity+1 slots
    this[0x14] = closed_hdr;

    // --- Phase 4: node pool at +0x68 ---
    pool_hdr = operator_new(0x14);     // 20-byte header
    pool_hdr->count    = 0;            // +0x00
    pool_hdr->capacity = 10000;        // +0x04
    pool_hdr->buf      = operator_new(0x9c44);  // +0x08: 40004 bytes → (10001 slots)
    pool_hdr->unk_0c   = 0;
    pool_hdr->unk_10   = 0xFFFFFFFF;
    // zero all slots
    this[0x68] = pool_hdr;

    // --- Phase 5: open list at +0x10 ---
    open_buf = operator_new(0x100004); // 1MB+4 bytes
    // init 0x10000 (65536) blocks of 3×u32 each:
    for (j = 0; j < 0x10000; j++) {
        open_buf[j*4 - 1] = 0;   // word before block
        open_buf[j*4 + 0] = 0;
        open_buf[j*4 + 1] = 0;
    }
    open_buf[0x100000/4] = 0;    // terminator at byte +0x100000
    this[0x10] = open_buf;

    // --- Phase 6: closed-path buffer at +0x0c ---
    path_buf = operator_new(0x180004); // 1.5MB+4 bytes
    path_buf[0x180000/4] = 0;         // terminator
    this[0x0c] = path_buf;

    // --- Phase 7: re-clear closed list and node pool ---
    // (redundant clear of buf[] and count for +0x14 and +0x68 headers)

    // --- Phase 8: 3 path-record arrays (stride 0x3E8 bytes = 250 dwords) ---
    // starting at +0xbc, +0x4a4, +0x88c
    for (k = 0; k < 3; k++) {
        // zero 3 dwords at +0x40+k*4, +0x4c+k*4, +0x58+k*4
        this[0x40 + k*4] = 0;
        this[0x4c + k*4] = 0;
        this[0x58 + k*4] = 0;   // approx: 0xc offsets from EBP
        // call sub-struct[k].vtable[0x0c]() — some reset method
        // zero 0xFA (250) dwords = 0x3E8 bytes at this+0xbc + k*0x3E8
        memset(this + 0xbc + k*0x3e8, 0, 0x3e8);
        this[0xc74 + k*4] = 0;   // sentinel/count per array
    }

    // --- Phase 9: secondary buffer at +0x64 ---
    this[0x64] = operator_new(0x27100);  // 160000 bytes

    return this;
}
```

---

## Behavioral analysis

### Caller context

The single caller xref is at `0x0040AFA5` (DATA context inside `FUN_0040AFD0`, a generic
initialization iterator). This places the constructor on the game-startup init path. The
PathfinderClass singleton is allocated before gameplay begins and persists for the session.
Verified via `get_xrefs_to 0x0042A6D0` and `disassemble_function 0x0042A6D0`.

### Sub-struct format (FUN_0042DC50)

`FUN_0042DC50 @ 0x0042DC50` (verified via `decompile_function 0x0042DC50`):
```c
undefined4 * __thiscall FUN_0042dc50(undefined4 *sub, int capacity, int existing_buf) {
    sub[1] = 0;          // +0x04 data ptr / count
    sub[2] = capacity;   // +0x08 capacity
    *(char*)(sub + 3) = 1;          // +0x0d flag
    sub[0] = &PTR_FUN_007e37ec;     // vtable (then overwritten to PTR_FUN_007e37cc)
    if (capacity != 0) {
        if (existing_buf != 0) {
            sub[1] = existing_buf;
        } else {
            sub[1] = operator_new(capacity * 4);
            *(char*)(sub+3+1) = 1;  // owns-buffer flag
        }
    }
    return sub;
}
```
Called with `(0, 0)` so sub-struct is empty/no-buffer, then vtable is overwritten to
`PTR_FUN_007e37cc`. The 3 sub-structs sit at `+0x74`, `+0x8c`, `+0xa4` (stride = 0x18 bytes).

Each sub-struct layout (0x18 bytes):
- `+0x00` (ptr): vtable = `PTR_FUN_007e37cc @ 0x007E37CC`
- `+0x04` (ptr): data buf = 0 (none allocated with capacity 0)
- `+0x08` (dword): capacity arg = 0
- `+0x0d` (byte): flag = 1 (set by FUN_0042DC50)
- `+0x10` (dword): count = 0 (written after FUN_0042DC50 returns)
- `+0x14` (dword): capacity = 10 (hard-coded in constructor)

### Heap buffer inventory

| Field offset | Alloc size | Semantic | Slots |
|-------------|-----------|---------|-------|
| `+0x0c` (ptr) | 0x180004 = 1,572,868 bytes | Closed-path buffer (path reconstruction) | Large (1.5MB) |
| `+0x10` (ptr) | 0x100004 = 1,048,580 bytes | Open list (priority queue nodes), 12-byte entries | 0x10000 = 65536 nodes |
| `+0x14` (ptr→header) | header: 0x14 bytes; buf: 0x40004 = 262,148 bytes | Closed list (visited cells) | capacity=65536 dwords |
| `+0x64` (ptr) | 0x27100 = 160,000 bytes | Secondary buffer (unknown role) | 40000 dwords |
| `+0x68` (ptr→header) | header: 0x14 bytes; buf: 0x9c44 = 40,004 bytes | Node pool | capacity=10000 dwords |

Header struct layout (20 bytes, used for `+0x14` and `+0x68`):
- `+0x00` (i32): count (zeroed twice: initial alloc + Phase 7 re-clear)
- `+0x04` (i32): capacity
- `+0x08` (ptr): buffer pointer
- `+0x0c` (i32): = 0
- `+0x10` (i32): = 0xFFFFFFFF (sentinel/-1)

### Path-record arrays at +0xbc..+0xc74

Three arrays of 0xFA (250) dwords = 0x3E8 bytes each:
- Array 0: `+0xbc` – `+0x4a3`
- Array 1: `+0x4a4` – `+0x88b`
- Array 2: `+0x88c` – `+0xc73`

Each has a corresponding 3-dword group at the low offsets zeroed by the second loop
(approx `+0x40–0x60`) and a sentinel at `+0xc74`, `+0xc78`, `+0xc7c`.

---

## Struct field accesses (frame-annotated)

All offsets are **direct byte offsets** from the `PathfinderClass` instance pointer (`this`).
`param_1` type in decompile is `undefined1*` (byte pointer), so all offsets are literal.

| Offset | Type | Init value | Known semantic |
|--------|------|-----------|----------------|
| `+0x00` | byte | 0 | unknown (maybe SpeedType selector byte 0) |
| `+0x01` | byte | 0 | `bridge_flank_enable` (read by `AStar_compute_edge_cost`, written by external setter; verified via BRIDGE_COSTS doc) |
| `+0x02` | byte | 0 | unknown |
| `+0x03` | byte | 1 | unknown |
| `+0x04` | float | 1.0f | `cost_multiplier` — multiplied with helper return in `AStar_main_loop @ 0x00429F8F` |
| `+0x08` | byte | 1 | unknown |
| `+0x0c` | ptr | heap | closed-path reconstruction buffer (1.5MB) |
| `+0x10` | ptr | heap | open list priority queue (1MB) |
| `+0x14` | ptr | heap | closed list header struct |
| `+0x18` | dword | 0 | unknown |
| `+0x1c` | dword | 0 | unknown |
| `+0x20` | dword | 0 | unknown |
| `+0x24` | dword | 0 | unknown |
| `+0x28` | dword | 0xFFFFFFFF | sentinel |
| `+0x2c` | dword | 0xFFFFFFFF | sentinel |
| `+0x38` | byte | 1 | unknown |
| `+0x3c` | dword | 0 | `urgency` — read by `AStar_compute_edge_cost` code-2 branch; written by `AStar_pathfind_search` |
| `+0x40..0x60` | 9× dword | 0 | path-record count/state (zeroed by second loop, 3 per path array) |
| `+0x64` | ptr | heap | secondary buffer (160000 bytes) |
| `+0x68` | ptr | heap | node pool header struct |
| `+0x6c` | dword | 0xFFFFFFFF | sentinel |
| `+0x70` | uint16 | 0 | unknown |
| `+0x72` | uint16 | 0 | unknown |
| `+0x74` | 24-byte sub | vtable+data | sub-struct 0: path list / pending list |
| `+0x8c` | 24-byte sub | vtable+data | sub-struct 1 |
| `+0xa4` | 24-byte sub | vtable+data | sub-struct 2 |
| `+0xbc..0xc73` | 3× 0x3E8 bytes | zeroed | path direction / data arrays |
| `+0xc74..0xc7c` | 3× dword | 0 | per-array sentinels |

---

## Globals / Enums / INI

| Symbol | Address | Value | Role |
|--------|---------|-------|------|
| `g_PathfinderClass_Singleton` | `0x0087e8b8` | ptr | Runtime-allocated singleton; zero at static init (verified via `read_memory 0x0087e8b8`) |
| `PTR_FUN_007e37cc` | `0x007E37CC` | vtable | Sub-struct vtable used in Phase 2 |
| `PTR_FUN_007e37ec` | `0x007E37EC` | vtable | Initial vtable set by FUN_0042DC50 (overwritten) |

No INI keys read. No TS-gated flags.

---

## Callees

| Function | Address | Role | Out-of-scope? |
|----------|---------|------|--------------|
| `FUN_0042DC50` | `0x0042DC50` | Sub-struct allocator/constructor | In-scope (task #22) |
| `FUN_0042D540` | `0x0042D540` | Sub-struct reset/cleanup (called in Phase 8 via vtable `+0x0c`) | In-scope (task #21) |
| `operator_new` | `0x007C8E17` | Heap allocation | Out-of-scope (runtime, manifest excluded) |

---

## Out-of-scope refs

- `operator_new @ 0x007C8E17` — runtime heap allocator; manifest excluded.
- `FUN_007C8B3D` (called by FUN_0042D540 and FUN_0042DC50 for free) — runtime utility; manifest
  excluded per phase-0 drop entry.

---

## YELLOW — Unverified

- The exact semantic of fields `+0x00`, `+0x02`, `+0x03`, `+0x08`, `+0x38` is unknown — they are
  set to 0 or 1 in the constructor but their callers/readers are not identified in this session.
- The exact semantic of the 3 sub-structs at `+0x74/+0x8c/+0xa4` (called path lists here) is
  inferred from their use pattern (capacity=10, vtable). Actual role requires decode of
  `FUN_0042DC50` (task #22) and callers that use `(*piVar3 + 0x0c)`.
- The secondary buffer at `+0x64` (160000 bytes) has no identified semantic beyond its allocation
  size. It may be a direction buffer or cost cache.
- The fields `+0x18..+0x24` (4 dwords zeroed) and sentinels `+0x28`, `+0x2c` have no identified
  semantic in this decode session.
- The path-record arrays at `+0xbc..+0xc74`: stride is 0x3E8 = 250 dwords per array. Zeroed by
  constructor and by Phase 8 second loop. Semantic (directions, costs, or cell sequences) is
  unverified.
- The `+0x40..+0x60` group (9 dwords zeroed in Phase 8): exact correspondence to `[EBP-0xc]`,
  `[EBP+0]`, `[EBP+0xc]` is derived from the disassembly; specific field meanings not confirmed.
