# LayerClass — z-Sorted Draw List Service (Ghidra Report)

**Address(es):** `0x004a862a` (constructor), `0x008a0360` (global array `g_DisplayLayers`)
**Confidence:** HIGH — all offsets, algorithm, and integration points verified
via direct Ghidra decompilation and assembly inspection.
**Active in YR:** **Yes** — this is the core per-frame object draw dispatch in
Yuri's Revenge. Not gated behind a flag.

This report consolidates and verifies what was previously scattered across
[DRAW_ORDER_DEPTH_SYSTEM.md](DRAW_ORDER_DEPTH_SYSTEM.md),
[TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md](TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md),
and [GAMEMD_ARCHITECTURE.md](GAMEMD_ARCHITECTURE.md). It adds: the exact
`LayerClass` struct layout, the sorted-insert algorithm (asm-verified), the
per-position-update re-insertion pattern in locomotors, the complete set of
`Submit_Object` call sites, and the grow policy.

---

## 1. Overview

`LayerClass` is a derived `DynamicVectorClass<ObjectClass*>` that tracks all
game objects belonging to one of five z-depth bands. There are exactly five
instances, laid out contiguously at `g_DisplayLayers` (`0x008a0360`), and
together they form the draw list that `Tactical_ObjectRenderingLoop`
(`0x006d8db0`) walks once per frame.

Only **Layer 2 (Ground)** maintains sorted order — by lepton `X + Y` in world
coordinates, via insertion sort. The other four layers append unsorted and
render in insertion order.

Objects re-register into their layer (via `DisplayClass::Submit_Object`,
`0x004a9720`) in two situations:
1. Their layer identity changes (e.g. aircraft takes off → Air, or falls into
   a tunnel → Underground). Detected by `ObjectClass::AI` comparing the
   result of `vtable+0x78` (`InWhichLayer`) before vs after AI processing.
2. **Their position changes**. Locomotors (verified in
   `FlyLocomotionClass::Process` at `0x004cd792`) explicitly call
   `Remove_From_Layer` + `Submit_Object` each tick they move the object.
   For Layer 2 objects this triggers a fresh insertion-sort walk.

This "re-submit on move" pattern means layer 2 is **kept sorted incrementally**
across ticks, not rebuilt from scratch each frame. The rendering loop just
iterates the vector in index order and assumes it is already correct.

---

## 2. LayerClass Struct Layout

`LayerClass` inherits from `DynamicVectorClass<ObjectClass*>` which
inherits from `VectorClass<ObjectClass*>`. Total size **0x18** bytes
(verified by constructor stride: `piVar1 + 6` ints per iter, 5 iterations,
spanning `0x008a0360` to `0x008a03d8`).

| Offset | Size | Field | Initial value | Verified at |
|--------|------|-------|---------------|-------------|
| `0x00` | 4 | `vtable` (→ `vtable__LayerClass` at `0x007e6060`) | `&vtable__LayerClass` | asm `004a8660` |
| `0x04` | 4 | `Items` (`ObjectClass**` buffer) | `NULL` | asm `004a863c` |
| `0x08` | 4 | `Capacity` (VectorMax) | `0` | asm `004a863f` |
| `0x0C` | 1 | `IsInitialized` | `1` | asm `004a8641` (corrected 2026-05-29: was `IsAllocated`; binary shows offset 0x0D — not 0x0C — is the heap-free guard: `Clear` checks `*(char*)(this+0x0D)` before freeing Items, and `Resize` sets `*(bool*)(this+0x0D) = (buffer == NULL)`; offset 0x0C is only set temporarily to 0/1 during `Resize` as a reentrancy guard, consistent with IsInitialized semantics — RTTI_LABEL_DRIFT; verified via `decompile_function 0x0040cc70` and `decompile_function 0x0040ce50`) |
| `0x0D` | 1 | `IsAllocated` | `0` | asm `004a8645` (corrected 2026-05-29: was `IsInitialized`; see 0x0C correction above — 0x0D is the actual heap-ownership flag — RTTI_LABEL_DRIFT) |
| `0x0E` | 2 | padding | — | — |
| `0x10` | 4 | `ActiveCount` (Count) | `0` | asm `004a865d` |
| `0x14` | 4 | `CapacityIncrement` | **`10`** (0xA) | asm `004a8656` |

Verified via `LayerClass__Constructor` at `0x004a862a`:

```c
piVar1 = &g_DisplayLayers.count;  // Ghidra's label is misleading — this
                                   // actually points at offset 0x08 (Capacity)
iVar2 = 5;
do {
    piVar1[-1] = 0;                       // 0x04: Items = NULL
    *piVar1 = 0;                          // 0x08: Capacity = 0
    *(byte*)(piVar1 + 1) = 1;             // 0x0C: IsAllocated = 1
    *(byte*)((int)piVar1 + 5) = 0;        // 0x0D: IsInitialized = 0
    entry->vtable = &PTR_FUN_007e192c;    // (overwritten twice below)
    entry->vtable = &PTR_FUN_007e4f64;
    piVar1[3] = 10;                       // 0x14: CapacityIncrement = 10
    piVar1[2] = 0;                        // 0x10: Count = 0
    entry->vtable = &vtable__LayerClass;  // final vtable
    piVar1 = piVar1 + 6;                  // +0x18 bytes
} while (--iVar2 != 0);
```

The vtable is assigned three times during construction: first `VectorClass`'s
vtable, then `DynamicVectorClass`'s, then finally `LayerClass`'s — standard
multi-inheritance constructor chain.

### Per-object state

Each `ObjectClass` carries its own layer bookkeeping:

| Object offset | Size | Field | Meaning |
|---------------|------|-------|---------|
| `0x94` | 4 | `LayerIndex` | Which layer this object is in (0..4), or `-1` if not submitted |
| `0x99` | 1 | `wasDrawn` | Cleared at start of each render; set if the object was actually drawn (in viewport) |

`LayerIndex == -1` is the sentinel for "not currently registered in any
layer" (e.g. freshly constructed, hidden, or destroyed).

---

## 3. The Five Layers

`g_DisplayLayers[5]` at `0x008a0360`:

| Index | Address | Name | Sorted? | Typical contents |
|-------|---------|------|---------|------------------|
| 0 | `0x008a0360` | Underground | No | Subterranean units (tunnel locomotion), subterranean APCs |
| 1 | `0x008a0378` | Surface | No | Flat-below-ground effects (`Layer=surface` in art.ini, "ships go under it") |
| 2 | `0x008a0390` | **Ground** | **Yes (Y-sort)** | Buildings, infantry, ground vehicles, non-flat anims, ground-layer animations |
| 3 | `0x008a03a8` | Air | No | Airborne aircraft, airborne projectiles/bullets |
| 4 | `0x008a03c0` | Top | No | Top-most effects (`Layer=top` in art.ini) |

End-of-array sentinel used by loops: `0x008a03d8` (and `0x008a03e8` for
`capacity`-field-based walks, which end at `entry[4].Count + 0x18`).

### Layer-name enum — VERIFIED via `Layer_To_Name` (`0x0048e090`)

The canonical layer-index → name mapping is stored in `g_LayerNameTable`
at `0x0081da78` (five string pointers). Resolved via memory inspection:

| Index | String | Pointer |
|-------|--------|---------|
| 0 | "Underground" | `0x0081db94` |
| 1 | "Surface" | `0x0081db8c` |
| 2 | "Ground" | `0x0081db84` |
| 3 | "Air" | `0x0081758c` |
| 4 | "Top" | `0x0081db80` |

`Layer_From_Name` (`0x0048e050`) scans this table and returns the matching
0-based index, or `-1` on miss. `CCINIClass::ReadLayer` (`0x00477050`)
uses this to parse `Layer=` INI keys — so the VALUE stored in
`AnimTypeClass+0x364` is **already the LayerClass 0-based index**, not a
separate enum.

**Important:** `ANIM_CLASS_DEEP_DIVE.md` documents this field with a
1-based enum ("1=Underground … 3=Ground"). That documentation is
**incorrect** — `Layer=ground` in art.ini parses to `2` (not 3), via
`Layer_From_Name("ground")` matching the index-2 entry of the table. When
`AnimClass::GetLayer` returns this value directly, it feeds into
`g_DisplayLayers[val]` which is 0-indexed. The two must agree (and they
do, once the doc is corrected).

### InWhichLayer virtual (vtable offset `0x78`) — per-class breakdown

On `ObjectClass` the layer-lookup virtual is at **vtable+0x78**. For foot
units it delegates to the locomotor's `In_Which_Layer` at **locomotor
vtable+0x74** (slot 29). Verified return values:

| Class / Locomotor | Return value | Verified at | Notes |
|-------------------|--------------|-------------|-------|
| **DriveLocomotionClass** | `2` (Ground) always | `0x004b4820` | Every tank, harvester, wheeled/tracked vehicle |
| **ShipLocomotionClass** | `2` (Ground) always | `0x006a3e50` | Destroyers, Dreadnoughts, Aegis, etc. |
| **FlyLocomotionClass** | `4` (Top) if `owner->GetHeight() > 0`, else `2` (Ground) | `0x004cfcf0` | **Airborne aircraft go to TOP, not Air.** See asm breakdown below |
| **JumpjetLocomotionClass** | Altitude-threshold based; reads `RulesClass+0x140` | `0x0054b8d0` | Rocketeer, Siege Chopper, Harrier. Not fully decompiled — same Top/Ground pattern likely |
| **AnimClass** (override at vtable+0x78) | `2` if `owner != NULL`, else `animtype->Layer` field at `+0x364`, else `3` (Air) | `0x00424cb0` | `Layer=` INI key is parsed 0-based; `3` default for ownerless type-less anims |

The `ObjectClass::AI` re-layer path at `0x005f400e` calls
`vtable+0x78` twice per tick (before and after AI) and only re-submits if
the returned value differs, so stable-layer objects skip the O(n) resort.

**Tiberian Sun confirmation:** these locomotors are all active in YR.
`JumpjetLocomotion` is used by 9 units, `FlyLocomotion` by 13 units (per
Ghidra's constructor comments on their CLSIDs). No TS-only gating.

### FlyLocomotionClass::In_Which_Layer — assembly-verified

Disassembled at `0x004cfcf0` (no function boundary existed; bytes:
`8B 44 24 04 8B 48 08 8B 11 FF 92 C8 01 00 00  33 C9 85 C0 0F 9F C1 49 83 E1 FE 83 C1 04 8B C1 C2 04 00`):

```
MOV  EAX, [ESP+4]          ; EAX = this (FlyLocomotionClass*)
MOV  ECX, [EAX+8]          ; ECX = this->Owner (FootClass*)
MOV  EDX, [ECX]            ; EDX = owner->vtable
CALL [EDX+0x1C8]           ; altitude = owner->GetHeight()  (vtable+0x1C8)
XOR  ECX, ECX
TEST EAX, EAX
SETG CL                    ; CL = (altitude > 0) ? 1 : 0
DEC  ECX                   ; CL=1 → ECX=0;  CL=0 → ECX=-1
AND  ECX, 0xFFFFFFFE       ; mask low bit
ADD  ECX, 4
MOV  EAX, ECX
RET  4
```

Arithmetic (signed, 32-bit wrap):
- Airborne (`altitude > 0`): `0 - 1 = -1`; `AND` → `0`; `+ 4` → **`4` (Top)**
- Landed/docked (`altitude ≤ 0`): `0 - 1 = 0xFFFFFFFF`; `AND` →
  `0xFFFFFFFE`; `+ 4` → **`2` (Ground)**

`ObjectClass::GetHeight` at vtable+0x1C8 (concrete: `0x005F5F40`) returns
the altitude above ground level. Any positive altitude → aircraft is on
the Top draw layer, rendering above all ground-layer objects and above
everything in Air/Surface.

**This contradicts prior project documentation** which claimed aircraft
use layer 3 (Air). Layer 3 (Air) is actually used for ownerless
airborne anims (the `AnimClass::GetLayer` default), not for aircraft. The
conceptual grouping is:
- **Top (4)** = flying aircraft drawn last-on-top
- **Air (3)** = ownerless / untyped airborne anims, debris
- **Ground (2)** = everything that participates in Y-sort (buildings,
  units, ground anims, landed aircraft, ships)
- **Surface (1)** = below-ground flat effects (`Layer=surface` anims;
  comment in artmd.ini: "Lower than ground — go under ships")
- **Underground (0)** = tunnel-locomotion units

### AnimClass::GetLayer — decompiled

```c
int AnimClass__GetLayer(AnimClass* this) {  // vtable+0x78
    if (this->field_0xCC != 0) return 2;    // attached to owner → Ground
    if (this->AnimType != NULL)             // at +0xC8
        return this->AnimType->Layer;       // at AnimType+0x364 (0-based index)
    return 3;                                // default: Air
}
```

- `field_0xCC` is the "attached owner" field. When an anim is attached to
  a building/unit (Tesla Coil charge effect, garrison muzzle flash,
  on-fire anim), it forcibly renders in the Ground layer next to its
  owner so Y-sort keeps them together.
- Ownerless + typed: reads `Layer=` from art.ini (0..4).
- Ownerless + typeless: falls through to layer 3 (Air). This default
  affects freshly-constructed anims before a type is assigned.

---

## 4. Core Algorithm — Submit_Object and Sorted Insert

### DisplayClass::Submit_Object (`0x004a9720`)

The unified add/move path. Called whenever an object needs to (re-)join its layer.

```c
void DisplayClass__Submit_Object(ObjectClass* obj) {
    if (obj == NULL) return;
    if (obj->LayerIndex != -1)           // already in a layer?
        Remove_From_Layer(obj);          // 0x004a9770 — strip from old
    int new_layer = obj->vtable->InWhichLayer();  // vtable+0x78
    if (new_layer != -1) {
        bool sorted = (new_layer == 2);
        if (DynamicVector_Insert(&g_DisplayLayers[new_layer], obj, sorted))
            obj->LayerIndex = new_layer;
    }
}
```

**Key behavior:** a `Submit_Object` call where the object's layer has not
changed still performs Remove-then-Insert. For Layer 2 this means every
re-submit does a fresh insertion-sort walk. This is the critical cost path.

### Remove_From_Layer (`0x004a9770`)

Strips the object from its current layer's vector.

```c
void Remove_From_Layer(ObjectClass* obj) {
    if (obj == NULL || obj->LayerIndex == -1) return;
    LayerClass* layer = &g_DisplayLayers[obj->LayerIndex];
    int pos = layer->vtable[0x10 / 4](layer, &obj);  // InWhichPosition
    if (pos != -1 && pos < layer->Count) {
        // shift-left from (pos+1) to end
        layer->Count -= 1;
        for (int i = pos; i < layer->Count; i++)
            layer->Items[i] = layer->Items[i+1];
        obj->LayerIndex = -1;
    }

    // Safety fallback: if LayerIndex was stale and the object wasn't found
    // in the expected layer, sweep all 5 layers and remove from any that
    // contain it.
    if (obj->LayerIndex != -1) {
        for (LayerClass* L = g_DisplayLayers; L < &g_DisplayLayers[5]; L++) {
            int p;
            while ((p = L->vtable[0x10/4](L, &obj)) != -1 && p < L->Count) {
                L->Count -= 1;
                for (int i = p; i < L->Count; i++)
                    L->Items[i] = L->Items[i+1];
            }
        }
        obj->LayerIndex = -1;
    }
}
```

**`vtable+0x10`** is `InWhichPosition(obj)` — linear search returning the
index of `obj` in the layer's `Items` array, or -1 if absent.

### DynamicVector__SortedInsert (`0x00551a90`)

**Verified from assembly.** The sorted-insert routine uses a forward linear
scan + shift-right. Counter-intuitive detail: the comparator is called
`__thiscall` with the **existing element as `this`** and the **new element
as the stack argument**.

```c
bool SortedInsert(LayerClass* this, ObjectClass* new_obj) {
    // Grow if needed (via vtable+0x08 = VectorClass::Resize)
    if (this->Count >= this->Capacity) {
        if (!this->IsInitialized && this->Count != 0) return false;
        if (this->CapacityIncrement < 1) return false;
        if (!this->vtable->Resize(
                this->Count + this->CapacityIncrement, NULL))
            return false;
    }

    // Linear scan: find the first existing element whose GetYSort()
    // is GREATER than new_obj's. Insert before it.
    int pos = 0;
    while (pos < this->Count) {
        bool existing_greater =
            ObjectClass__YSortComparator(this->Items[pos], new_obj);
        if (existing_greater) break;
        pos++;
    }

    // Shift elements [pos..Count-1] right by one.
    for (int i = this->Count - 1; i >= pos; i--)
        this->Items[i+1] = this->Items[i];

    this->Items[pos] = new_obj;
    this->Count += 1;
    return true;
}
```

**Complexity:** O(n) scan + O(n) shift = O(n) per insert. For n = layer-2
object count. Typical RA2 skirmish has 100–400 layer-2 objects.

**Grow policy (verified by decompiling `VectorClass::Resize` at `0x0040ce50`):**

```c
uint VectorClass__Resize(this, int new_capacity, void* new_buffer) {
    if (new_capacity == 0) {
        this->vtable->Clear();    // vtable+0xC — frees buffer
        return 1;
    }
    this->IsAllocated = 0;
    void* buf = new_buffer ?: operator_new(new_capacity * 4);
    this->IsAllocated = 1;
    if (!buf) return 0;

    if (this->Items != NULL) {
        int copy_count = min(new_capacity, this->Capacity);
        memcpy(buf, this->Items, copy_count * 4);
        if (this->IsAllocated)            // note: ALWAYS true by this
            operator_delete(this->Items); //       point (we just set it)
    }
    this->Items = buf;
    this->Capacity = new_capacity;
    this->IsAllocated = (new_buffer == NULL);

    if (this->Capacity < this->Count)    // truncate Count if shrinking
        this->Count = this->Capacity;
    return 1;
}
```

**Exact allocation, no doubling.** `SortedInsert` passes
`Capacity + CapacityIncrement` as the new capacity — so the buffer grows
**by exactly 10 each time**. For 400 layer-2 objects loaded at scenario
start, this is ~40 `operator_new` + `memcpy` + `operator_delete` cycles,
each one larger than the last (O(n²) allocator pressure). Steady-state
cost after load is zero because removals free no memory — the buffer
stays high-water-mark sized until `Clear` is called.

**`DynamicVectorClass::Clear` (at `0x0040cc70`):**

```c
void DynamicVectorClass__Clear(this) {  // vtable+0xC
    this->Count = 0;
    if (this->Items != NULL && this->IsAllocated) {
        operator_delete(this->Items);
        this->Items = NULL;
    }
    this->IsAllocated = 0;
    this->Capacity = 0;
}
```

**Clear frees the buffer AND zeros capacity** — not just a count reset.
After `Clear`, the next insert triggers a fresh grow from 0 to 10.
This is called per-layer from `DisplayClass::Init_Clear` at scenario start.

**Insert-failure handling:** if `Resize` returns 0 (allocation failure),
`SortedInsert` returns `false`. `Submit_Object` silently skips storing the
layer index — the object ends up with `LayerIndex == -1` and is
**invisible to the render loop**. Under memory pressure this is a silent
correctness issue, but in practice it never fires on modern hosts.

### ObjectClass::YSortComparator (`0x005f6220`) — VERIFIED FROM ASSEMBLY

```c
bool YSortComparator(ObjectClass* existing, ObjectClass* new_obj) {
    // NOTE: __thiscall — `existing` is ECX (this), `new_obj` is stack arg
    int existing_key = new_obj->vtable->GetYSort();   // vtable+0xB8
    int new_key      = existing->vtable->GetYSort();  // vtable+0xB8
    return existing_key < new_key;
    // (Ghidra decompiles the two vtable calls in swapped order from the
    //  variable names, but assembly confirms the actual behavior: returns
    //  true iff existing.GetYSort() > new_obj.GetYSort().)
}
```

Re-reading the decompiled output carefully with the correct this-pointer
assignment: `iVar1 = (*param_2->vtable[0xb8/4])()` uses `param_2` (new_obj)
to get `iVar1`, then `iVar2 = (*param_1->vtable[0xb8/4])()` uses `param_1`
(existing) to get `iVar2`, returns `iVar1 < iVar2` → i.e. returns true when
`new_obj.GetYSort() < existing.GetYSort()` → **existing's sort key is
greater than new's**.

The `SortedInsert` loop stops at the first `pos` where this comparator
returns true, i.e. the first element with a **greater** GetYSort. So
`new_obj` is inserted just before it. Result: the layer is kept sorted
**ascending** by GetYSort. Objects at the top-left of the map (lower X+Y)
are at lower indices and drawn first; objects at the bottom-right (higher
X+Y) are at higher indices and drawn last (on top).

### ObjectClass::GetYSort (`0x005f6bd0`)

```c
int ObjectClass__GetYSort(ObjectClass* this) {
    CoordStruct buf1, buf2;
    this->vtable->GetRenderCoords(&buf1);  // vtable+0xAC
    this->vtable->GetRenderCoords(&buf2);  // vtable+0xAC (called twice!)
    return buf1.Y + buf2.X;                // lepton Y + lepton X
}
```

**Sort key = `X + Y` in lepton coordinates** (world coords, not screen
pixels). This is the isometric depth key: "further down-right in the iso
grid" = larger sum = drawn later = on top.

**No tiebreaker.** Objects with identical `X+Y` have no defined relative
order — they end up in whatever order `Submit_Object` was called. The
elevation Z coordinate does **NOT** participate in the sort.

**Odd detail:** `GetRenderCoords` is called twice with two different output
buffers, though only one set of fields is read from each. This is either a
micro-optimization artifact or compiler-generated redundancy; functionally
it is equivalent to a single call. `GetRenderCoords` is `vtable+0xAC`, which
may be overridden by `BuildingClass` to return foundation center (affecting
building sort position relative to units).

---

## 5. When Submit_Object is Called

Xrefs to `DisplayClass__Submit_Object` (`0x004a9720`) — 13 call sites.
All verified via direct xref query. Also listing `Remove_From_Layer`
(`0x004a9770`) xrefs (14 call sites — it's called independently as well).

### Submit_Object call sites (13)

| Caller | Address | When |
|--------|---------|------|
| `ObjectClass::AI` | `0x005f400e` | End of per-tick AI, **only if `vtable+0x78` return changed** since start of AI |
| `ObjectClass::Reveal` | `0x005f4fe2` | Object transitions from hidden → visible |
| `ObjectClass::DropIn` | `0x005f4196` | Object parachutes/paradrops into the world |
| `BulletClass::Fire` | `0x00468b6d` | Projectile spawned and begins flight |
| `FlyLocomotionClass::Process` | `0x004cd792` | **Every aircraft movement tick** (pre-Remove + post-Insert pattern) |
| `FUN_004cd2a0` (locomotor helper) | `0x004cd4e7` | Drive locomotor position update |
| `FUN_006622c0` | `0x0066242c`, `0x0066290d` | Teleport/Chrono locomotion reposition |
| `FUN_0075f8b0` | `0x0075f952` | Wave/ripple effect class |
| `AnimClass::SetOwnerObject` | `0x00424c00`, `0x00424c7c` | Anim attached/detached from owner (invalidates the `field_0xCC != 0 → Ground` condition in `GetLayer`) |
| `FUN_0054ca90` | `0x0054cc0d` | Temporal warp / displacement effect |
| unidentified | `0x0054b18e` | Voxel effect / jumpjet helper (uncreated function boundary) |

### Remove_From_Layer independent call sites (ones NOT from Submit_Object)

| Caller | Address | When |
|--------|---------|------|
| `AnimClass::Constructor` | `0x004229ca` | Safety strip during construction |
| `AnimClass::SetOwnerObject` | `0x00424b74`, `0x00424c2b` | First call clears old layer before re-submit |
| `AnimClass::Detach` | `0x0042517a` | Anim being detached from owner's lifecycle |
| `AnimClass::Limbo` | `0x0042559b` | Anim placed into limbo (hidden, paused) |
| `ObjectClass::AI` | `0x005f414c` | When object's `byte_0x81` flag is set (dead/hidden state) |
| `ObjectClass::Conceal` | `0x005f4d79` | Object becomes hidden (cloak, enter garrison) |
| `BulletClass::Fire` | `0x004686f5` | Strip bullet from any prior layer before re-submit |
| `FlyLocomotionClass::Process` | `0x004cd75a` | Pre-move strip (paired with Submit_Object at `0x004cd792`) |
| `FUN_004cd2a0` | `0x004cd333` | Drive locomotor pre-move strip |
| `FUN_0054ca90` | `0x0054cbd4` | Temporal warp pre-move strip |
| unidentified | `0x0075f9b2` | Wave/effect system |

### Key pattern — move-triggered re-sort (FlyLocomotionClass::Process)

Verified sequence in `FlyLocomotionClass__Process` at `~0x4cd75f`:

```c
// (after computing new world-space position)
Remove_From_Layer(obj);                      // strip from current layer
obj->vtable->vtable_0x124(0);                // unlink from cell list
obj->vtable->SetCoords(&new_coords, ...);    // vtable+0x1B4
obj->vtable->vtable_0x124(1);                // relink to cell list
DisplayClass__Submit_Object(obj);            // re-insert (sorted in layer 2)
```

This Remove → Set-coords → Submit sequence is what **keeps layer 2 sorted
incrementally**. Each moved object pays O(n) per tick, where n is the
layer's current size. For 100 moving ground units in a 400-object Layer 2
this is ~40,000 comparisons per tick, but insertion sort is near-optimal
for mostly-sorted data and amortized per-object cost stays low.

### Re-layer path (ObjectClass::AI at 0x005f400e)

```c
int old_layer = obj->vtable->InWhichLayer();  // vtable+0x78
// ... do per-tick AI: z-coordinate update, cell-based effects, etc.
int new_layer = obj->vtable->InWhichLayer();  // vtable+0x78
if (old_layer != new_layer)
    DisplayClass__Submit_Object(obj);         // re-register into new layer
```

This is the **only** layer-transition trigger in `ObjectClass::AI` itself.
In-layer position changes are handled by the locomotor as shown above, not
by `AI`.

---

## 5a. vtable__LayerClass layout (`0x007e6060`) — VERIFIED

Read directly from memory. `LayerClass` only overrides **two** virtuals
vs. its `DynamicVectorClass` base; the rest are inherited:

| Slot | Offset | Address | Function | Override? |
|------|--------|---------|----------|-----------|
| 0 | `+0x00` | `0x004aeb50` | `~LayerClass` destructor | **LayerClass override** |
| 1 | `+0x04` | `0x0040ccd0` | (inherited VectorClass method) | — |
| 2 | `+0x08` | `0x0040ce50` | `VectorClass::Resize` | inherited from DynamicVectorClass |
| 3 | `+0x0C` | `0x0040cc70` | `DynamicVectorClass::Clear` | inherited |
| 4 | `+0x10` | `0x0040cf00` | `DynamicVectorClass::InWhichPosition` (linear search, returns index) | inherited |
| 5 | `+0x14` | `0x0040cca0` | (inherited) | — |
| 6 | `+0x18` | `0x0040ccc0` | (inherited) | — |
| 7 | `+0x1C` | **`0x005519b0`** | `DynamicVector__Insert` (wrapper: sorted vs unsorted dispatch) | **LayerClass override** |
| 8 | `+0x20` | `0x0040cc00` | (inherited) | — |
| 9 | `+0x24` | `0x0040cc10` | (inherited) | — |

So `LayerClass` = `DynamicVectorClass<ObjectClass*>` + custom destructor +
custom Insert wrapper. The sorted-insert routine itself
(`DynamicVector__SortedInsert` at `0x00551a90`) is **not** in the vtable —
it's called directly by the vtable+0x1C wrapper when `sorted == true`.

The LayerClass destructor (`0x004aeb50`):
```c
LayerClass* ~LayerClass(LayerClass* this, byte delete_flag) {
    this->vtable = &VectorClass_vtable;  // chain to base destructors
    if (this->Items != NULL && this->IsAllocated) {
        operator_delete(this->Items);
        this->Items = NULL;
    }
    this->IsAllocated = 0;
    this->Capacity = 0;
    if (delete_flag & 1)
        operator_delete(this);   // complete-object destructor
    return this;
}
```

## 5b. Save/Load — `DisplayClass::Save` / `DisplayClass::Load`

LayerClass state is **persisted to save games**. Decompiled:

```c
void DisplayClass__Load(IStream* stream) {        // at 0x004ae6f0
    for (int i = 0; i < 5; i++)
        VectorClass__Load(&g_DisplayLayers[i], stream);  // 0x00551b90
}

void DisplayClass__Save(IStream* stream) {        // at 0x004ae720
    for (int i = 0; i < 5; i++)
        if (VectorClass__Save(&g_DisplayLayers[i], stream) < 0)  // 0x00551b20
            return;  // abort on I/O error
}
```

Called from `RadarClass::Save` (`0x00656aca`) and `RadarClass::Load`
(`0x006568b2`) — i.e. the full `DisplayClass` subtree is serialized as
part of `RadarClass` which is itself part of the `TacticalClass` save
tree.

`VectorClass::Load` (at `0x00551b90`) reads:
1. `IStream->Read(&count, 4)` — how many elements were saved
2. For each element: `IStream->Read(&pointer, 4)` — reads the pointer
   value as it was when saved (pre-swizzle)
3. Grows via Resize if needed, then stores via the insertion wrapper
4. After all reads: for each element, calls `FUN_006cf240` — this is
   the **pointer swizzle** step that converts the saved raw addresses
   back into live pointers via a relocation table at `DAT_00b0c110`.

This means the save file stores absolute pointers (as written at save
time), and a separate swizzle table maps old→new after load. Rust
implementation note: our `GameSnapshot` uses `bincode` and entity IDs,
not raw pointer swizzling — no direct parallel needed, but the fact
that layer ORDER is preserved across saves matters (the sorted state of
Layer 2 carries through saves, so loaded games don't need to re-sort).

## 5c. DisplayClass hierarchy — which class owns what

The draw-list state lives on `DisplayClass`, which is part of a multi-level
class hierarchy walked via `Init_Clear` chain:

```
MapClass                                    (FUN_005659f0 / MapClass::Init_Clear)
  └─ DisplayClass                           (FUN_004a88c0 / DisplayClass::Init_Clear)
       └─ RadarClass                        (FUN_00652de0 / RadarClass::Init_Clear)
            └─ SidebarClass (presumed)
                 └─ TacticalClass           (TacticalClass_Draw / input dispatch)
```

Verified: `DisplayClass::Init_Clear` at `0x004a88c0` calls
`MapClass::Init_Clear` as its first operation (inherited chain);
`RadarClass::Init_Clear` at `0x00652de0` calls `DisplayClass::Init_Clear`
first. Each level's `Init_Clear` resets its own additional fields then
chains up.

`g_DisplayLayers` at `0x008a0360` is a **global static** inside
`DisplayClass`, not a member field — the 5 LayerClass instances are
sized `0x18` each and laid out contiguously. This means they can be
iterated as a flat array via pointer arithmetic (`pDVar1 + 0x18`) which
is the iteration pattern used throughout the codebase.

The `DisplayClass` vtable is at `0x007e6130`. Slot 0 is
`DisplayClass::Init_Clear` (`0x004a88c0`), confirming the class hierarchy
follows Westwood's convention of putting `Init_Clear` as vtable slot 0.

## 6. DisplayClass::Init_Clear (`0x004a88c0`)

Called once per scenario boot to reset the draw-list state:

```c
void FUN_004a88c0(DisplayClass* this) {
    FUN_005659f0();  // some global effect-system reset
    this->field_0x11A4 = 0;
    this->field_0x11A8 = 0;
    this->field_0x11AC = -1;
    this->field_0x117C = 0;
    this->field_0x11B8 = -1;
    this->byte_0x11B0 = 0;
    this->byte_0x11CF = 0;
    this->byte_0x11D0 = 0;
    this->byte_0x11B1 = 0;
    this->byte_0x11B2 = 0;
    // Clear each of the 5 layers (vtable+0xC = DynamicVectorClass::Clear)
    for (LayerClass* L = g_DisplayLayers; L < &g_DisplayLayers[5]; L++)
        L->vtable->Clear();
}
```

The `vtable+0xC` virtual on each LayerClass zeros `Count` (and possibly frees
Items, depending on the `Clear` implementation). After this, all objects in
the game world have `LayerIndex` pointing to layers that are now empty —
they will re-register via `Submit_Object` as they are spawned during
scenario load.

---

## 7. Render Loop Consumption (Tactical_ObjectRenderingLoop at 0x006d8db0)

Covered in detail in
[DRAW_ORDER_DEPTH_SYSTEM.md §4](DRAW_ORDER_DEPTH_SYSTEM.md) and
[TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md](TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md).
Summary in LayerClass terms:

**First pass** — iterate layers 0→4, for each object in index order:
1. Clear `obj->wasDrawn` (byte at offset `0x99`).
2. Compute screen coords via vtable dispatch; clip to viewport.
3. If visible: set `wasDrawn = 1`, call `SetDrawCoords` (`vtable+0x10C`),
   then `DrawShadow` (`vtable+0x110`) for alive non-techno objects, then
   `DrawAs` (`vtable+0x104`).
4. **After layer 2 only:** iterate `g_BuildingClass_Array` and call
   `BuildingClass::UpdateGarrisonFire` for each drawn building. Turrets and
   garrison muzzle flashes thus render on top of everything in layer 2,
   regardless of Y-sort.

**Second pass** — iterate layers 0→4 again, only for objects with
`wasDrawn == 1`, calling `DrawExtras` (`vtable+0x110`) — selection brackets,
health bars, veterancy pips, etc.

The end-of-loop sentinel address `< 0x8a03db` in the second pass (vs
`< 0x8a03d8` elsewhere) is a `capacity`-field-based loop — it iterates
while pointing at the `Capacity` field of the entry. `0x8a03db` = start of
entry 4 (`0x8a03c0`) + offset of a field at `+0x1B`. This is consistent with
a loop that terminates by pointer comparison against the byte after entry 4.

---

## 8. Grow Policy and Memory Layout

- `CapacityIncrement = 10` — when `Count` reaches `Capacity`, buffer grows
  by 10 slots at a time (via `VectorClass::Resize`, `vtable+0x08`).
- `IsAllocated = 1` at construction — the LayerClass owns its `Items`
  buffer.
- No per-layer initial capacity — all 5 layers start at `Capacity=0,
  Items=NULL`. First Submit into each layer triggers the first Resize.

For a typical skirmish with 300 layer-2 objects, the layer undergoes ~30
resize operations during load (100×3 insertions into an initially empty
layer). After scenario is built, steady-state growth is rare because removals
match insertions.

---

## 9. Integration Points Summary

**Called BY LayerClass:**
- `VectorClass::Resize` — `vtable+0x08` on the layer itself
- `VectorClass::Clear` — `vtable+0x0C` (during `Init_Clear`)
- `DynamicVectorClass::InWhichPosition` — `vtable+0x10` (during Remove)
- `ObjectClass::InWhichLayer` — `vtable+0x78` on each object
- `ObjectClass::GetYSort` — `vtable+0xB8` on each object
- `ObjectClass::GetRenderCoords` — `vtable+0xAC` (called by GetYSort)

**Called FROM LayerClass by the render loop:**
- `vtable+0x99` (wasDrawn byte) read/write — not a virtual, direct field
- `vtable+0x104` (DrawAs), `vtable+0x10C` (SetDrawCoords),
  `vtable+0x110` (DrawShadow / DrawExtras) — called from render loop on
  objects read out of the layer

**Invoked at scenario lifecycle:**
- `LayerClass__Constructor` (`0x004a862a`) — one-time at module load/init
- `DisplayClass::Init_Clear` (`0x004a88c0`) — once per scenario start

---

## 10. Current Rust Implementation Status

**We do not have a LayerClass analogue.** Our render-layer approach is
simpler but structurally different:

- [src/app_render/build_instances.rs:181-206](../../ra2-rust-game/src/app_render/build_instances.rs#L181-L206)
  rebuilds flat `Vec<SpriteInstance>` lists **every frame** from scratch
  (`state.cached_unit_instances` is taken, cleared, and refilled).
- Each list is then sorted with `sort_by_depth_desc`
  ([src/app_render/build_instances.rs:607](../../ra2-rust-game/src/app_render/build_instances.rs#L607))
  — Rust's stable sort, O(n log n) per frame.
- Lists are partitioned by asset type (overlay, bridge_detail, bridge_body,
  wall, unit, bridge_unit, shp_paged, bridge_shp_paged, building_turret) —
  not by z-depth band. There is no 5-layer enum.
- Building turrets are already drawn in a separate pass AFTER layer-2
  objects — this matches the binary's turret pass
  ([src/app_render/build_instances.rs:188-195](../../ra2-rust-game/src/app_render/build_instances.rs#L188-L195)).
- Depth = iso screen-space depth key computed per sprite
  (not lepton X+Y from world coords). Functionally equivalent for
  ground-plane objects since screen-space iso depth is a monotonic
  transform of world X+Y, but **not** equivalent for objects at different
  z-elevations (aircraft, airborne bullets, anims with Z offset).

**Gaps vs. the binary:**
1. **No persistent draw list** — we re-sort from scratch every frame instead
   of maintaining sorted state incrementally. At 100+ moving units this is
   pure waste: the list is mostly sorted frame-to-frame.
2. **No 5-layer partition** — we partition by asset type, which gets
   close-enough for RA2's aesthetic (because ground sprites, VXL, SHP are
   all rendered by the same ground-layer rules) but cannot correctly express
   the `Underground`/`Air`/`Top` bands. Airborne bullets and tunnel-diving
   subterranean APCs are not handled by a layer system.
3. **No layer-change detection on entities** — our renderer re-derives what
   goes where each frame from entity components, rather than having entities
   announce "I am now Air" via a submit call.

**Current performance:** per frame we do one `sort_by` call per SpriteInstance
list. With `N` total drawable entities this is O(N log N) per frame. At 60 fps
and 500 entities this is 4500×60 = 270k comparisons/sec. Binary's pattern
amortizes to O(movers × layer_size) ≈ 100×400 = 40k/tick (30 tps =
1.2M/sec), so **our current pattern is actually cheaper at current scale** —
but scales worse as entity counts grow because we rebuild from scratch.

No changes are urgent. This report exists as a reference for future
rendering-perf work.

---

## 10a. Rendering-correctness gaps in current Rust

The preceding section covers perf differences. These are **correctness
gaps** — places where our renderer produces a different (worse) visual
result than gamemd would, directly due to missing LayerClass logic.

Severity: **low-to-medium visual fidelity**. None block correctness of
the simulation. Ranked by how often the player will notice.

### 1. `Layer=` art.ini key is not parsed at all

- **Grep result:** zero occurrences of `Layer=` parsing in `src/rules/`.
- **INI evidence:** `ini/artmd.ini` sets `Layer=ground`, `Layer=surface`,
  `Layer=top` on ~100+ anims.
- **gamemd behavior:** stored at `AnimTypeClass+0x364` as 0-based layer
  index, returned by `AnimClass::GetLayer` (`0x00424cb0`) when the anim
  has no owner.
- **Our behavior:** every anim routes to `shp_paged` in
  [src/app_render/build_instances.rs:175](../../ra2-rust-game/src/app_render/build_instances.rs#L175)
  and gets the same depth treatment as unit sprites.
- **Consequence:**
  - `Layer=surface` anims (commented in artmd.ini: "Lower than ground —
    go under ships") render at the same depth as ground sprites instead
    of beneath them.
  - `Layer=top` anims (some superweapon FX, nuke cloud) don't forcibly
    render above everything.
  - `Layer=ground` (the common default) lands correctly by accident.
- **Fix scope:** parse `Layer=` in `src/rules/art_data.rs`, store 0..4
  on the anim type, partition the `shp_paged` list by layer band.

### 2. Airborne aircraft not guaranteed above ground layer

- **gamemd behavior:** `FlyLocomotionClass::In_Which_Layer` (`0x004cfcf0`,
  §4 above) returns **4 (Top)** when `owner->GetHeight() > 0`. Top is a
  strictly separate bucket drawn after all layer-2 content.
- **Our behavior:** aircraft apply a screen-Y offset
  (`altitude * 0.06 px`) at
  [src/app_instances/units.rs:84-91](../../ra2-rust-game/src/app_instances/units.rs#L84-L91)
  that feeds into `depth_y → compute_sprite_depth` with a
  `z * 0.0001` bias at
  [src/app_instances/helpers.rs:51](../../ra2-rust-game/src/app_instances/helpers.rs#L51).
  They remain in the same sort space as buildings.
- **Consequence:** an iso-row-forward building can draw **above** a
  low-flying aircraft, because 2–3 cells of iso-row produce more depth
  delta than `600 altitude × 0.0001 = 0.06` z-bias. Visible
  depth-flicker when aircraft fly over Construction Yard / War Factory.
- **Fix scope:** emit airborne aircraft into a separate draw list
  (gated on `locomotor.altitude > 0`) drawn strictly after unit /
  building / SHP lists; OR boost the depth bias large enough to
  dominate all possible iso-row deltas within the viewport.

### 3. No attached-anim Y-sort coupling to owner — INVESTIGATED

- **gamemd behavior:** `AnimClass::GetLayer` forces layer 2 (Ground) when
  `field_0xCC != 0` (owner attached). The Y-sort coupling works via a
  subtle mechanism (verified from ANIM_CLASS_DEEP_DIVE.md §SetOwnerObject
  and vtable analysis):
  - `AnimClass` does **NOT** override `GetRenderCoords` (vtable+0xAC) —
    it inherits `ObjectClass::GetRenderCoords` at `0x0041be00`, which
    returns the anim's own stored `Location` field.
  - `AnimClass` DOES override `GetCoords` (vtable+0x48, at `0x00422be0`)
    which returns `ownerCoords + myOffset` when attached. But
    `GetYSort` calls `GetRenderCoords`, NOT `GetCoords`.
  - **The trick:** `SetOwnerObject` (`0x00424b50`) calls
    `vtable->SetCoords(ownerCoords)` at attachment time — it **copies
    the owner's position into the anim's Location**. So when
    `GetRenderCoords` returns the anim's stored Location, it IS the
    owner's position. Both have identical X+Y → identical Y-sort key.
  - For moving owners, `AnimClass::AI` calls `GetCoords` (which adds
    owner offset) and writes back to Location each tick, keeping the
    anim synced.
  Confidence: **HIGH** — vtable slot 43 at offset 0xAC confirmed as
  inherited `ObjectClass::GetRenderCoords`. AnimClass vtable slot 18
  at offset 0x48 confirmed as overridden `AnimClass::GetCoords`.
- **Our behavior:** `build_damage_fire_instances` and
  `build_garrison_muzzle_flash_instances`
  ([build_instances.rs:198-200](../../ra2-rust-game/src/app_render/build_instances.rs#L198-L200))
  emit to `shp_paged` with the anim's own position. If the sim doesn't
  sync the anim's position to the owner's position each tick, the
  depth keys diverge and the damage fire separates from its parent.
- **Consequence:** damage fires on a moving vehicle (e.g. War Miner on
  fire driving to refinery) could drift behind the vehicle visually.
  Static buildings are unaffected (they don't move). Garrison muzzle
  flash uses fire-port position which is already in the building's
  cell, so that's likely fine.
- **Fix scope:** the fix is a **sim-level position sync**, not a render
  change. Ensure that attached-anim position tracks the owner's
  position each tick. The render depth computation can stay as-is —
  if the positions match, the depth keys match automatically. Low
  urgency since most attached anims are on buildings (which don't move).

### 4. Shadow rendering — INVESTIGATED: we don't draw shadows at all

- **gamemd behavior:** inside the layer-2 loop, for each object:
  `DrawShadow` (vtable+0x110) → `DrawAs` (vtable+0x104) called
  sequentially. Shadow frames are stored in the second half of SHP
  files (e.g. a 120-frame SHP has 60 real + 60 shadow frames). Shadows
  are flat, untinted, drawn at a shadow offset before the main sprite.
- **Our behavior (verified from code audit of `src/render/`,
  `src/app_render/`, `src/app_instances/`):**
  - **SHP sprites:** the sprite atlas loader
    (`src/render/sprite_atlas.rs`) recognizes the second-half shadow
    frames with the comment *"RA2 anim SHPs have shadow frames in
    second half"* but **discards them** — only real frames go into the
    atlas.
  - **VXL units:** the voxel rasterizer (`src/render/vxl_raster.rs`)
    bakes lighting into the RGBA buffer. No shadow pass exists.
  - **No shadow instances, buffers, or draw calls** anywhere in the
    pipeline. The merge pass
    (`src/app_render/merge_passes.rs`) has pool keys `"unit"`,
    `"shp_p0"`…`"shp_p3"`, `"overlay_wall"` — no shadow key.
  - **Bottom line: shadows are completely absent**, not just misordered.
- **Consequence:** units and buildings have no ground shadows. In the
  original game, every unit casts a flat shadow decal on the terrain.
  This is noticeable, especially for infantry grouped together.
- **Fix scope:** this is a **separate feature** (load second-half SHP
  frames into a shadow sub-atlas, emit shadow SpriteInstances at shadow
  offset, draw before main sprites). Not a LayerClass ordering concern.
  The shadow-interleaving question (per-object shadow→sprite ordering
  vs batched shadow pass) becomes relevant only after shadows exist.
  For now, **moot**.

### 5. DrawExtras second-pass ordering

- **gamemd behavior:** after all 5 layers draw their sprites, a **second
  loop across all 5 layers** calls `vtable+0x110` (DrawExtras) on every
  object that was drawn — selection brackets, health bars, veterancy
  pips, capture links. This ensures UI overlays render atop every game
  sprite.
- **Our behavior:** likely OK. Selection brackets are drawn in their
  own late pass via
  [src/app_selection_brackets.rs](../../ra2-rust-game/src/app_selection_brackets.rs).
  Not verified this session.
- **Consequence:** if any health-bar / pip draw happens BEFORE the main
  sprite list, units could overdraw their own health bars.
- **Fix scope:** audit the final-pass ordering in `app_render`; likely
  fine. Low priority.

### Priority summary (updated 2026-04-06 after investigation)

| Gap | Visible? | Fix size | Priority | Status |
|-----|----------|----------|----------|--------|
| 1. `Layer=` unparsed | Rare (14 non-default anims total; 1 surface, 13 top) | M | **Low** — too few anims to justify 5-bucket refactor | Open |
| 2. Aircraft depth-flicker | Yes (common, near buildings) | S | **High** — simple partition by altitude > 0 | Open |
| 3. Attached-anim Y-sort | Moving vehicles only; static buildings unaffected | S | **Low** — sim-level position sync, not render | Investigated — mechanism understood |
| 4. Shadow rendering | Yes (no shadows at all) | L (new feature) | **Medium** — separate feature, not LayerClass | Investigated — moot for ordering |
| 5. DrawExtras ordering | Unlikely | S (audit only) | Low | Open |

**Recommended single fix:** gap #2 (aircraft depth-flicker). Emit airborne
aircraft into a separate draw list gated on `locomotor.altitude > 0`,
drawn strictly after all Ground-layer content. Matches gamemd's
`FlyLocomotionClass::In_Which_Layer` returning 4 (Top). Small scope,
high visibility.

None of these affect simulation correctness or multiplayer determinism.
All are deferred visual-fidelity polish items.

---

## 11. Open Questions

1. ~~**`VectorClass::Resize` virtual**~~ — **RESOLVED.** Verified at
   `0x0040ce50`: exact-allocate (no doubling), capacity grows by
   `CapacityIncrement` (10) per resize. Copy-old + free-old pattern.
2. ~~**`DynamicVectorClass::Clear`**~~ — **RESOLVED.** Verified at
   `0x0040cc70`: frees the Items buffer, sets `IsAllocated=0`,
   `Capacity=0`, `Count=0`. Next insert triggers a fresh grow from 0.
3. ~~**Specific `InWhichLayer` return values**~~ — **RESOLVED.** Verified:
   Drive→2, Ship→2, Fly→2 or 4 (altitude-based), AnimClass via owner
   check + AnimType.Layer field. Surprise finding: airborne aircraft go
   to layer **4 (Top)**, not 3 (Air) as prior docs claimed.
4. **Render coords dual-call in GetYSort** — why is `GetRenderCoords`
   (vtable+0xAC) called twice with two separate stack buffers when only
   one X and one Y are read? Probable: compiler artifact from inlining
   two separate CoordStruct locals. No behavioral impact. Low priority.
5. **JumpjetLocomotionClass::In_Which_Layer full logic** (`0x0054b8d0`) —
   partial decompile shows it reads `owner->GetHeight()` and a
   `RulesClass+0x140` threshold, then branches. Not fully decompiled;
   same Top/Ground pattern likely, potentially with hysteresis.
   Follow-up research (2026-04-06): INI keys `JumpjetHeight=` (per-type,
   500–750 leptons), `JumpjetClimb=`, `JumpjetCrash=`, and global
   `FlightLevel=1500` found. No documented hysteresis. **For
   implementation purposes, using `altitude > 0` (same as FlyLoco)
   is safe** — the exact threshold refinement can come later via Ghidra.
6. **BuildingClass / InfantryClass / UnitClass `InWhichLayer` overrides** —
   not individually decompiled. Render-loop behavior confirms they all
   return 2 (Ground), but direct vtable+0x78 inspection would seal the
   claim. Low priority — the turret pass at `layer == 2` only triggers
   for classes that actually end up there.
7. **Overflow safety for `InWhichLayer` returning invalid index** — if a
   virtual returns 5, 6, or 100, `&g_DisplayLayers[N]` would index past
   the 5-entry array. No range check in Submit_Object. Relies on
   virtuals always returning values in `{-1, 0, 1, 2, 3, 4}`. Not
   observed to fire; trusted invariant.
8. **Insert-failure behavior under allocator pressure** — if Resize
   fails (returns 0), the object ends up with `LayerIndex == -1` and is
   invisible to render. No retry, no warning. In practice doesn't fire
   on modern hosts, but noted as a theoretical correctness edge.

---

## Sources

**Ghidra decompilations (verified, 24 functions):**

Core LayerClass / DisplayClass:
- `0x004a862a` — `LayerClass__Constructor` (+ assembly)
- `0x004a88c0` — `DisplayClass::Init_Clear`
- `0x004a9700` — `DisplayClass` vtable+0xC tick function
- `0x004a9720` — `DisplayClass::Submit_Object`
- `0x004a9770` — `Remove_From_Layer` (includes 5-layer safety sweep)
- `0x004ae6f0` — `DisplayClass::Load`
- `0x004ae720` — `DisplayClass::Save`
- `0x004aeb50` — `LayerClass` destructor

VectorClass hierarchy:
- `0x0040cc70` — `DynamicVectorClass::Clear` (buffer-free behavior)
- `0x0040ce50` — `VectorClass::Resize` (exact-allocate confirmed)
- `0x0040cf00` — `VectorClass::InWhichPosition` (created function boundary)
- `0x00551a90` — `DynamicVector__SortedInsert` (+ assembly `00551a90–00551b19`)
- `0x005519b0` — `DynamicVector_Insert` wrapper (vtable+0x1C)
- `0x00551b20` — `VectorClass::Save`
- `0x00551b90` — `VectorClass::Load`

Object AI / layer assignment:
- `0x005f400e` — `ObjectClass::AI` (re-layer path)
- `0x005f4196` — `ObjectClass::DropIn` (call site)
- `0x005f4fe2` — `ObjectClass::Reveal` (call site)
- `0x005f6220` — `ObjectClass::YSortComparator`
- `0x005f6bd0` — `ObjectClass::GetYSort`
- `0x00424cb0` — `AnimClass::GetLayer` (vtable+0x78 override)
- `0x00424b50` — `AnimClass::SetOwnerObject`
- `0x004cd792` — `FlyLocomotionClass::Process` (move-triggered re-sort)

Locomotor InWhichLayer (vtable slot 29, offset 0x74):
- `0x004b4820` — `DriveLocomotionClass::In_Which_Layer` → `2`
- `0x006a3e50` — `ShipLocomotionClass::In_Which_Layer` → `2`
- `0x004cfcf0` — `FlyLocomotionClass::In_Which_Layer` (altitude-switch, created)
- `0x0054b8d0` — `JumpjetLocomotionClass::In_Which_Layer` (partial)

INI layer name resolution:
- `0x00477050` — `CCINIClass::ReadLayer`
- `0x0048e050` — `Layer_From_Name`
- `0x0048e090` — `Layer_To_Name`

**Memory inspections:**
- `0x008a0360` — `g_DisplayLayers` global (5×0x18, 120 bytes)
- `0x007e6060` — `vtable__LayerClass` (10 slots verified)
- `0x0081da78` — `g_LayerNameTable` (5 string pointers)
- `0x0081db80–0x0081758c` — string data ("Underground","Surface","Ground","Air","Top")
- `0x007e89f4 + 0x74` — FlyLocomotion ILocomotion vtable slot 29
- `0x004cfcf0` — FlyLoco::In_Which_Layer raw instruction bytes (36 bytes)

**Renames applied this investigation:**
- `FUN_004a88c0` → `DisplayClass__Init_Clear`
- `FUN_005519b0` → `DynamicVector__Insert`
- `FUN_0040cf00` → `VectorClass__InWhichPosition` (new function boundary)
- `FUN_004cfcf0` → `FlyLocomotionClass__In_Which_Layer` (new function boundary)
- Program saved.

**Memory inspection:**
- `0x008a0360` — `g_DisplayLayers` global (120 bytes verified zero at rest;
  populated at runtime)

**Xrefs:** `get_xrefs_to 0x004a9720` (Submit_Object) — 13 call sites;
`get_xrefs_to 0x008a0360` (g_DisplayLayers) — 11 references across
constructor, Submit/Remove, Init_Clear, and FUN_004ae6f0/720.

**Prior docs referenced (content verified and extended):**
- `DRAW_ORDER_DEPTH_SYSTEM.md` — covered layer count, Y-sort, render loop
  structure; did NOT cover LayerClass struct layout or per-position re-submit.
- `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` — covered the full three-pass
  frame architecture; LayerClass is the object-source for Pass 2 Step 8.
- `GAMEMD_ARCHITECTURE.md` line 462 — identified `DAT_008a0360 LayerClass[5]`
  with the five-name enumeration.

**INI:**
- `ini/artmd.ini` — `Layer=ground|surface|top` key values observed.
