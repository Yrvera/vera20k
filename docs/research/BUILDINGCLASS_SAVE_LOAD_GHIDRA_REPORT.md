---
name: BuildingClass Save/Load Serialization Format
description: Vtable slots 5/6 full decomp — the COM IPersistStream-based raw memory dump + pointer-fixup table scheme used for .SAV files.
type: reference
---

# BuildingClass Save/Load — Serialization Format

**Load address:** `0x00453E20` (vtable slot 5, body 870 bytes at 0x00453E20..0x00454186)
**Save address:** `0x00454190` (vtable slot 6, body 187 bytes at 0x00454190..0x0045424B)
**CLSID (slot 3, `GetClassID`):** stored at `0x007E96A0` = `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}`
**Confidence:** HIGH (direct decompilation + disassembly of both functions; inheritance chain verified)

> **Slot-order note.** Task 8's prompt listed slot 5 as Save and slot 6 as Load.
> That is reversed vs. the binary. The vtable follows the **IPersistStream** method
> order (IsDirty @4, Load @5, Save @6, GetSizeMax @7), so slot 5 = **Load**,
> slot 6 = **Save**. `AbstractClass::Load` (real Load, reads stream) is at
> `0x00410380`; `AbstractClass::Save` (real Save, writes stream) is at
> `0x00410320`. Ghidra labels on these two functions are correct; several
> *caller*-side labels (e.g. `CaptureManagerClass__Save/Load`) are swapped — trust
> the code, not the label.

---

## 1. Overview

BuildingClass serialization is **not field-by-field**. It is the classic Westwood
OLE Structured Storage scheme used across the whole engine:

1. **IPersistStream** is the contract — every Abstract-descended class exposes
   `IsDirty`/`Load`/`Save`/`GetClassID`/`GetSizeMax` via a secondary vtable at
   `this+0x04`. The primary vtable re-uses slots 5/6 for Load/Save too.
2. **`AbstractClass::Save` writes the entire instance as a raw memory dump**
   (`stream.Write(this_ptr, 4)` followed by `stream.Write(this, GetSizeMax())`).
   It then chains up: `ObjectClass::Save` and each subclass's Save *only* write
   the nested dynamic-array contents that the raw dump cannot capture (because
   those arrays' `Items` pointer points to a separate heap allocation).
3. **`AbstractClass::Load` reads the raw memory** back into the freshly-allocated
   instance and registers `(old_this → new_this)` in a global pointer-fixup
   dictionary at `DAT_00b0c110`. Each subclass's Load then re-runs the
   constructor (to re-seat vtables and sub-object vtables), re-reads the nested
   arrays, and registers each embedded pointer slot via `FUN_006cf240` so a
   later fixup pass can rewrite old pointers to new ones.
4. After every object has been loaded, a **single global fixup pass** walks the
   pointer-slot list and, for each slot, looks up the stored old-pointer value
   in the old→new map and writes the new pointer back.

The container file is an **OLE Compound Document** (`StgCreateDocfile` /
`StgOpenStorage` + `OleSaveToStream` / `OleLoadFromStream` — all three are
imported and referenced as strings at `0x0081086A`, `0x008108B0`, `0x008108C4`).
Each game object is saved as a separate IStream within the docfile; the stream
name is the GUID produced by `StringFromCLSID` of the object's CLSID prefixed
with the unique `AbstractClass::ID` (stored at `this+0x0C`). BuildingClass's
own CLSID is `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}`.

Consequence for a Rust re-implementation: you do not need to mirror the raw
memory layout. You need a *semantically equivalent* save of every field the
raw-dump path captures, plus identical handling of the two DynamicVectors that
BuildingClass's own Save writes explicitly. See §6/§7.

---

## 2. TechnoClass / ObjectClass / AbstractClass Base Chain

**Save (write) call chain** (from BuildingClass downwards, `FUN_00454190`):

```
BuildingClass::Save  (0x00454190)
  -> TechnoClass::Save_IStream   (FUN_0070c250)   [passthrough, writes nothing]
       -> ObjectClass::Save_IStream   (FUN_0065ac40)
            -> AbstractClass::Save   (0x00410320) [raw dump]
            -> writes DynamicVector at +0xE0
                 (count at +0xE8, Items at +0xE4)
  -> writes DynamicVector at +0x66C (count at +0x67C, Items at +0x670)
  -> writes DynamicVector at +0x684 (count at +0x694, Items at +0x688)
```

**Load (read) call chain** (`FUN_00453E20`):

```
BuildingClass::Load  (0x00453E20)
  -> maintenance pass on a secondary global table at DAT_00b0e840
     (remove-and-shift if the old ID was registered there)
  -> TechnoClass::Load_IStream   (FUN_0070bf50)
       -> ObjectClass::Load_IStream   (FUN_0065ab80)  [corrected 2026-05-29: was FUN_005f5e80]
            -> FUN_005f5e80 (ObjectClass base fixups)
                 -> AbstractClass::Load   (0x00410380)
                      reads old this_ptr, registers it in DAT_00b0c110 as the
                      "old -> new" mapping, then reads GetSizeMax() bytes of raw
                      memory into the freshly-allocated instance
                 -> registers fixups for +0x30, +0x34, +0x38, +0x18, +0x88
                 -> calls VocHandle::Init × 2; zeros +0xA8
            -> restores embedded DynamicVector vtable at +0xE0
               (=PTR_FUN_007e180c), re-reads the vector's contents, and
               registers every element for pointer-fixup
  -> re-register (old_this, new_this) in DAT_00b0e840 (matches the entry
     removed in the first maintenance pass)
  -> if `this != NULL`: call BuildingClass::Constructor (re-seats all 4
     vtables and the embedded sub-object vtables)
  -> register single-pointer fixups for +0x520, +0x524, +0x540, +0x548,
     +0x54C, +0x600, +0x6F4
  -> VocHandle::Init at +0x6A0 and +0x6B4 (BuildingClass-side looping
     sound handles reset to empty)
  -> single fixup at +0x6F4
  -> re-read DynamicVector at +0x66C and register every pointer element
  -> re-read DynamicVector at +0x684 and register every pointer element
  -> register 21 Anims[] slots (+0x55C..+0x5AC) for fixup
  -> register 3 Upgrades[] slots (+0x5EC..+0x5F4) for fixup
  -> register 8 secondary anim/fire slots (+0x5C8..+0x5E4) for fixup
  -> explicitly zero LightSourceClass* at +0x614
```

### AbstractClass::Save (0x00410320)

```c
stream->Write(&this_ptr, 4);                             // 4-byte old_this
size_t n = this->vtable[12](0);                           // GetSizeMax payload size
stream->Write(this, n);                                   // raw memory dump
this[+0x20] = 0;                                          // clear IsDirty flag
```

`vtable[12]` at byte offset `+0x30` is `GetClassSize` (returns 6 for
BuildingClass — this is the *class enum*, not a byte count; the size
actually used for the dump is whatever the full Abstract-derived size is).
Open question: the actual byte count written needs confirmation — may be
overridden per class, or the literal return (0x06) may be multiplied
somewhere. See §8 edge case "dump size".

### AbstractClass::Load (0x00410380)

```c
stream->Read(&old_this, 4);                               // read saved this_ptr
swap_table_add(DAT_00b0c110, old_this, new_this);         // FUN_006cf2c0
saved_vtable_offset = new_this[+0x1C];                    // preserve [+0x1C]
size_t n = new_this->vtable[12](0);                       // GetClassSize
stream->Read(new_this, n);                                // raw memory load
new_this[+0x1C] = saved_vtable_offset;                    // restore [+0x1C]
```

`FUN_006cf2c0` at `0x006cf2c0` appends a `(old_this, new_this)` pair to a
secondary list inside `DAT_00b0c110` (offsets +0x20 Items, +0x2C Count).

### ObjectClass::Save (`FUN_0065ac40`) / Load (`FUN_0065ab80` + `FUN_005f5e80`)

Save writes the **+0xE0 DynamicVector** (Count at +0xE8, Items at +0xE4,
element size 4). Each element is written raw as 4 bytes. Load is split across
two functions: `FUN_0065ab80` (the full ObjectClass::Load_IStream — called by
TechnoClass::Load_IStream) calls `FUN_005f5e80` (base fixups) then handles
the DV at +0xE0. `FUN_0065ab80` reads the count, allocates the Items backing
array if needed, re-reads each element, and registers each element for
pointer-fixup via `FUN_006cf240`. It also restores the embedded DV vtable at
`+0xE0` to `&PTR_FUN_007e180c`. `FUN_005f5e80` registers fixups for +0x30,
+0x34, +0x38, +0x18, +0x88, calls `VocHandle::Init` **twice** (two SoundEvent
slots, not three), and zeros `+0xA8`. (corrected 2026-05-29: was "Load is FUN_005f5e80" and "reinstates the three primary SoundEvent VocHandles"; binary shows the DV work is in FUN_0065ab80, and FUN_005f5e80 calls VocHandle::Init only twice — via decompile_function 0x005f5e80 and 0x0065ab80; ROOT_CAUSE: RTTI_LABEL_DRIFT + OPERATOR_OR_ORDER_DRIFT)

### TechnoClass::Save_IStream (`FUN_0070c250`)

**Thin wrapper** that calls `ObjectClass::Save` then returns hardcoded `0`
unconditionally — it does **not** propagate ObjectClass::Save's HRESULT return
value. In practice this means a stream write failure inside ObjectClass::Save
would be silently swallowed here, and BuildingClass::Save would continue writing
the two DynamicVectors. All TechnoClass state goes through the raw-dump path.
(corrected 2026-05-29: was "Does nothing beyond calling ObjectClass::Save" implying passthrough; binary returns 0 unconditionally via decompile_function 0x0070c250 — ROOT_CAUSE: INFERENCE_HARDENED)

> Do NOT confuse this with `TechnoClass__Save` at `0x0070c270` (Ghidra label)
> — that symbol is the CRC/checksum-stream `Save` used for multiplayer state
> sync, not the IStream save. The two share a name but write to different
> destinations.

### TechnoClass::Load_IStream (`FUN_0070bf50`)

Post-raw-dump cleanup. Zeros `+0x514`. Registers the following TechnoClass
pointer slots for fixup (all byte offsets):
`+0x21C (Owner HouseClass*)`, `+0x304..+0x320` (8 pointers), `+0x14C`,
`+0x2E4`, `+0x218`, `+0x500`, `+0x2B4` (current target), `+0x2B8`, `+0x2BC`,
`+0x434`, `+0x2D0`, `+0x2D8`, `+0x2DC`, `+0x2C0`, `+0x2CC`, `+0x2E0`,
`+0x2D4`, `+0x518`, `+0x51C`, `+0x274`, `+0x278`, `+0x294`, `+0x1CC`,
`+0x1D0`, `+0x11C`, `+0x2A8`, `+0x2AC`, `+0x2B0`, `+0x428`, `+0x42C`,
`+0x118`, `+0x324`, `+0x130`, `+0x12C`, `+0x2C8`, `+0x1D4`.

Calls `VocHandle::Init` four times (the four TechnoClass/ObjectClass
SoundEvent slots). Explicitly initializes:

| Offset | Type | Reset value | Purpose |
|---|---|---|---|
| +0x49C | int | 1 | state flag |
| +0x4A0 | int | 0 | — |
| +0x4B8 | byte | 0 | — |
| +0x4BC | int | 0 | — |
| +0x4D4 | byte | 0 | — |
| +0x4D8 | int | 0 | — |
| +0x4F0 | int | -1 (`0xFFFFFFFF`) | **sound loop handle #1 invalidated** |
| +0x4F4 | int | -1 (`0xFFFFFFFF`) | **sound loop handle #2 invalidated** |

These are all runtime-cache fields that must NOT persist across save/load.

---

## 3. BuildingClass-Specific Save (field list, in order)

BuildingClass's own Save writes the bodies of two DynamicVectors and nothing
else — every primitive field is carried by the raw-memory dump that happens
at the AbstractClass level.

| Order | Offset | Field | Method | Element size | Notes |
|---|---|---|---|---|---|
| 1 | +0x67C | **Upgrade-iteration DV Count** | `stream.Write(&count, 4, 0)` | 4 | Header count for the vector at +0x66C |
| 2 | +0x670 | **Upgrade-iteration DV Items[]** | `stream.Write(&items[i], 4, 0)` × count | 4 each | Element pointers written raw; rehydrated on load |
| 3 | +0x694 | **Occupant DV Count** | `stream.Write(&count, 4, 0)` | 4 | Header count for the vector at +0x684 |
| 4 | +0x688 | **Occupant DV Items[]** | `stream.Write(&items[i], 4, 0)` × count | 4 each | InfantryClass* pointers; rehydrated on load |

Everything else — `Type`, `Factory`, `Anims[21]`, `Upgrades[3]`,
`BuildingLightClass*`, `LightSourceClass*`, all `HasPower`/`HasExtra*` bytes,
all CDTimer fields, `UpgradeLevel`, `OwnershipChanged`, `GarrisonFireIndex`,
`QueueingCell`, etc. — rides in the raw dump from `AbstractClass::Save`.
Pointer fields are written verbatim (as 32-bit pointer values) and rehydrated
on Load via the fixup dictionary.

### Return-value / error handling

Both DynamicVectors short-circuit on any `stream->Write` failure (returns
the `HRESULT` directly). On clean success returns `0` (success). On AbstractClass
failure the raw dump was never written and we return the failing HRESULT
without attempting the two vectors. So partial saves are not possible —
it's success or abort.

### What is **not** written explicitly

The raw dump path means these fields ARE saved (as part of the byte blob)
but are NEVER touched by the load-time fixup machinery. They will contain
stale values after the raw read and must be reset by constructor + init
logic on the Load side:

- +0x520 `Type` (pointer — fixed up)
- +0x524 `Factory` (pointer — fixed up)
- +0x540 / +0x548 / +0x54C (BuildingClass-specific pointers — fixed up)
- +0x55C..+0x5AC `Anims[21]` (21 pointers — fixed up)
- +0x5B0..+0x5C4 `AnimStates[21]` (byte flags — survive raw dump)
- +0x5C8..+0x5E4 secondary anim/fire pointers (8 pointers — fixed up)
- +0x5EC..+0x5F4 `Upgrades[3]` (pointers — fixed up)
- +0x600 `BuildingLightClass*` (pointer — fixed up)
- +0x614 `LightSourceClass*` (pointer — **zeroed**, not fixed up; see §5)
- +0x6A0 SoundEvent (VocHandle — **reinitialized**)
- +0x6B4 SoundEvent (VocHandle — **reinitialized**)
- +0x6F4 (pointer — fixed up)

---

## 4. BuildingClass-Specific Load (reconstruction, in order)

`BuildingClass::Load` runs in six phases.

### Phase 1 — pre-dump heap-map maintenance

`DAT_00b0e840` is a global `(ID → this-pointer)` directory updated whenever
an Abstract-derived object is constructed. Load temporarily removes any
entry that matches the *incoming* saved ID (obtained by calling the
secondary-vtable IRTTITypeInfo::GetID on `this+4`, which reads `this[+0x0C]`)
so that the new instance can be re-inserted in Phase 4 without colliding
with a stale entry. This is purely a registry compaction; no stream I/O.

### Phase 2 — TechnoClass::Load chain (raw dump + parent fixups)

Calls `FUN_0070bf50` (TechnoClass::Load_IStream) which performs the
AbstractClass→ObjectClass→TechnoClass load sequence described in §2. On
return the entire 0x720-byte instance has been overwritten with the raw
dump from the stream, and every TechnoClass-level pointer slot is queued
for fixup.

### Phase 3 — Constructor re-run (vtable re-seat)

```c
if (this != NULL)
    BuildingClass::Constructor(this, arg2_on_stack);
```

`BuildingClass::Constructor` (0x0043B680) resets the four vtable pointers
at `this[0..3]` and initializes the in-struct DynamicVector headers at
+0x66C and +0x684 (setting their vtable to `&PTR_FUN_007e43c8`, capacity
increment to 10, and count to 0). It also calls `TechnoClass::Constructor`
which re-seats the rest of the sub-object vtables.

**Why the raw dump can't be trusted for vtables:** the dumped values are
the pointers from the *saving* process's memory map. After image base
randomization or a rebuild they would be invalid. The Constructor re-run
is the load-time guarantee that every vtable pointer is correct for the
loading process.

### Phase 4 — re-register in DAT_00b0e840

Re-inserts `(saved_ID, this)` into the global directory at `DAT_00b0e840`
(paired with the Phase 1 removal). If the directory is at capacity the
constructor resizes it by 10 (with a fallback to capacity-1 if no prior
allocation exists). On resize-failure this phase is skipped silently
(`JZ 0x00453F34`) — the object is still loaded, just unreachable via the
ID directory.

### Phase 5 — register BuildingClass-specific pointer fixups

Calls `FUN_006cf240(&DAT_00b0c110, &this->slot)` for each of:

| Order | Offset | Field | Notes |
|---|---|---|---|
| 1 | +0x520 | Type (BuildingTypeClass*) | resolved via old→new map |
| 2 | +0x524 | Factory (FactoryClass*) | NULL if unused |
| 3 | +0x540 | (unidentified pointer) | §8 open question |
| 4 | +0x548 | (unidentified pointer) | §8 open question |
| 5 | +0x54C | (unidentified pointer) | §8 open question |
| 6 | +0x600 | BuildingLightClass* (spotlight) | NULL if no spotlight |
| 7 | +0x6F4 | (unidentified pointer) | §8 open question |

Each call captures the current value (just read from the raw dump), zeroes
the slot, and appends `(old_ptr_value, &slot)` to the fixup queue inside
`DAT_00b0c110` (Items at +0x08, Count at +0x14).

Then `VocHandle::Init` (`FUN_00405BE0`) is called with `ECX = this+0x6A0`
and again with `ECX = this+0x6B4` — the two BuildingClass-specific
SoundEvent structs (20 bytes each) are re-initialized to the empty state.

### Phase 6 — read the two DynamicVectors

For each of the two vectors (DV @ +0x66C, DV @ +0x684):

```c
stream->Read(&count, 4, 0);
for (i = 0; i < count; i++) {
    stream->Read(&value, 4, 0);
    // DV_push_back equivalent: respects capacity, IsAllocated flag, and
    // a virtual grow callback at DV.vtable[2]
    if (DV.count < DV.capacity ||
        ((DV.is_allocated || DV.capacity==0) && DV.cap_increment > 0
         && DV.vtable.Resize(DV.capacity + DV.cap_increment, 0))) {
        DV.items[DV.count++] = value;
    }
}
for (i = 0; i < count; i++) {
    fixup_register(&DV.items[i]);   // FUN_006cf240
}
```

After the vector body is loaded, each element slot is registered for
pointer fixup separately.

### Phase 7 — register fixed-array pointer fixups

```c
for (i = 0; i < 21; i++) fixup_register(&this->Anims[i]);             // +0x55C..+0x5AC
for (i = 0; i < 3;  i++) fixup_register(&this->Upgrades[i]);          // +0x5EC..+0x5F4
for (i = 0; i < 8;  i++) fixup_register(&this->SecondaryAnims[i]);    // +0x5C8..+0x5E4
```

### Phase 8 — explicit reset

```c
this->LightSourceClass_ptr = 0;   // +0x614 zeroed unconditionally
```

This is the **only** BuildingClass-side pointer that is deliberately NOT
fixed up. See §5 for why.

---

## 5. Pointer Rehydration Schemes

Westwood uses a single universal scheme via `DAT_00b0c110` with two sub-lists.

### The fixup dictionary (`DAT_00b0c110`, ~0x38 bytes)

| Offset | Field | Purpose |
|---|---|---|
| +0x00 | vtable ptr | allocator vtable #1 (for Items[] at +0x08) |
| +0x04 | vtable ptr | allocator vtable #2 (for Items[] at +0x20) |
| +0x08 | `pointer_slot_list.items` | array of `(saved_value, &slot)` pairs |
| +0x0C | `pointer_slot_list.capacity` | |
| +0x10 | `pointer_slot_list.is_allocated` (byte at +0x11) | |
| +0x14 | `pointer_slot_list.count` | |
| +0x18 | `pointer_slot_list.cap_increment` | |
| +0x1C | vtable ptr | for the old→new map |
| +0x20 | `swap_map.items` | array of `(old_this, new_this)` pairs, 8 bytes each |
| +0x24 | `swap_map.capacity` | |
| +0x28 | `swap_map.is_allocated` (byte at +0x29) | |
| +0x2C | `swap_map.count` | |
| +0x30 | `swap_map.cap_increment` | |

### `FUN_006cf240(&DAT_00b0c110, int* slot_addr)` — register pointer slot

```
if (slot_addr == NULL) return E_POINTER;
old_value = *slot_addr;
if (old_value == 0) return 0;            // null pointers aren't fixed up
append (old_value, slot_addr) to pointer_slot_list
*slot_addr = 0;                           // clear slot so the dangling
                                          // raw-dump pointer can't be deref'd
                                          // accidentally before fixup
```

Null slots are skipped entirely — on Load, any field that was NULL at save
time remains NULL. This matches "conditional save only if non-null" in spirit
without needing an explicit conditional.

### `FUN_006cf2c0(&DAT_00b0c110, old_this, new_this)` — register swap map entry

```
append (old_this, new_this) to swap_map
```

Called once per object inside `AbstractClass::Load`.

### The fixup pass (inferred — not in this function body)

Not executed inside `BuildingClass::Load`; runs at end-of-game-load
(presumably the outer `OleLoadFromStream` driver). Pseudocode:

```
for (slot_addr, old_value) in pointer_slot_list:
    for (old_this, new_this) in swap_map:
        if (old_value == old_this):
            *slot_addr = new_this
            break
```

Complexity is O(slots × objects). Because the scheme is purely address-based
and doesn't use type tags, it works uniformly for *any* Abstract-derived
pointer (HouseClass*, BuildingTypeClass*, AnimClass*, InfantryClass*,
FactoryClass*, BuildingLightClass*, …). Per-type ID lookup (e.g. for `Type`
fields like `BuildingTypeClass*`) is NOT used on this path — both the
saving and loading processes address the same CLSID-registered array, so
pointer equality via the swap map is enough.

### The `DAT_00b0e840` directory

Independent of the fixup dictionary. Keyed by the unique `AbstractClass::ID`
at `this[+0x0C]`, it tracks all living Abstract-derived objects by ID so
the `ID → this` lookup can be performed after load. Load removes and
re-inserts the entry to keep the directory consistent with whatever the
dump produced.

---

## 6. Fields Excluded from Save (Runtime Caches)

These are **not** excluded from the raw dump (they get serialized as bytes)
but are **reset on Load** and therefore effectively excluded from persistence.
Do NOT try to carry these across a save in a Rust re-implementation.

### TechnoClass / ObjectClass-inherited resets

| Offset | Type | Reset value | Reason |
|---|---|---|---|
| +0x4F0 | int | `-1` | SoundEvent loop handle #1 — OS-level audio handle, invalid across runs |
| +0x4F4 | int | `-1` | SoundEvent loop handle #2 |
| +0x49C | int | `1` | TechnoClass flag (re-inited) |
| +0x4A0 | int | `0` | — |
| +0x4B8 | byte | `0` | — |
| +0x4BC | int | `0` | — |
| +0x4D4 | byte | `0` | — |
| +0x4D8 | int | `0` | — |
| +0x514 | int | `0` | explicit zero at start of TechnoClass::Load |
| +0x0A8 | int | `0` | ObjectClass flag |
| +0x4DC, +0x4EC, +0x4FC, +0x50C | 4 SoundEvents | VocHandle::Init × 4 | reinits 20-byte sound state structs |

### BuildingClass-specific resets

| Offset | Type | Reset action | Reason |
|---|---|---|---|
| +0x6A0 | SoundEvent (20B) | VocHandle::Init | Building-specific looping sound |
| +0x6B4 | SoundEvent (20B) | VocHandle::Init | Building-specific looping sound |
| +0x614 | ptr | explicit `= 0` | **LightSourceClass* — recreated on-demand during the first post-load tick, not persisted.** Contrast with +0x600 (BuildingLightClass* / spotlight) which IS fixed up. |

### Fields that survive the raw dump unchanged

Every *value-type* BuildingClass field survives:
`DamagedState` (+0x534), `AnimStates[21]` (+0x5B0..+0x5C4),
`Cycling anim phase index` (+0x5FC), `Wall orientation` (+0x618),
`Timer accumulator` (+0x620), `CDTimer` (+0x628..+0x638),
`HasPower` (+0x660), `IsOverpowered` (+0x661), `HasExtraPowerBonus` (+0x668),
`HasExtraPowerDrain` (+0x669), `GarrisonFireIndex` (+0x69C),
`ProduceCashTimer` (+0x6D0..+0x6D8), `SellBuilding/NominalPower flag`
(+0x6DC), `ConstructionComplete flag` (+0x6DD), `ForceShield active`
(+0x6DF), `OwnershipChanged` (+0x6E3), `CloakGenerator direction/radius`
(+0x6EB..+0x6EC), `Gap generator visual stage` (+0x6ED), `Refinery ore
level` (+0x6F0), `UpgradeLevel` (+0x702), `Bunker docking sub-state`
(+0x718).

---

## 7. Save-File Container Format (High Level)

- **Disk format:** OLE 2 Compound Document (`.SAV`). Opened via
  `StgOpenStorage` and created via `StgCreateDocfile` (both in `ole32.dll`).
  The docfile root contains one IStream per persisted game object, plus
  any number of metadata streams (game metadata, mission context, etc.).
- **Per-object serialization:** each IPersistStream-implementing object is
  saved through `OleSaveToStream(pPersistStream, pIStream)`. This writes:
  1. The object's CLSID (16 bytes, via `WriteClassStm`).
  2. The payload produced by `pPersistStream->Save(pIStream, TRUE)`, which
     for BuildingClass is `AbstractClass::Save` (writes `old_this` + raw
     dump) → `ObjectClass::Save` (writes +0xE0 DV) → `BuildingClass::Save`
     (writes +0x66C DV + +0x684 DV).
- **Per-object load:** mirror. `OleLoadFromStream(pIStream, iid, ppvObj)`
  reads the CLSID, instantiates the matching class (via
  `CoRegisterClassObject` / `CoCreateInstance`-style factory — see imports
  `CoRegisterClassObject` `0x124`, `CoDisconnectObject` `0x128`), then
  calls the new instance's `IPersistStream::Load`.
- **Versioning:** handled at the CLSID level. The retail BuildingClass CLSID
  is `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}`. A format-breaking change
  would ship a new CLSID, and the docfile driver would dispatch to a
  different (or backward-compat) Load implementation. There is no
  in-stream version field inside BuildingClass::Save/Load.
- **Integrity / validity:** propagates `HRESULT`s from every `Read`/`Write`;
  any negative HRESULT aborts the current object's load/save. No checksum
  inside the BuildingClass portion. (Game-wide integrity is the docfile's
  own property — OLE detects truncation/corruption at the storage layer.)
- **Pointer fixup pass ordering:** the outer load driver must complete every
  IPersistStream::Load call before running the fixup pass, because a
  pointer whose target object hasn't been loaded yet would miss the
  swap-map entry. The two-table scheme (slot list + swap map) exists
  precisely to decouple in-object loads from pointer resolution.

---

## 8. Edge Cases

### Null pointers

`FUN_006cf240` exits early (`return 0`) if the raw-dump slot is already
zero. Save-time null → load-time null. No special-case conditional needed
on either side — the universal path handles it.

### Uninitialized fields (e.g. `QueueingCell.min`)

`QueueingCell=` at `BuildingTypeClass+0x1618` is a **TypeClass** field and
therefore not part of BuildingClass's save/load. The instance-side
QueueingCell cache (if any) rides in the raw dump. The Task 3 bug
(uninitialized `QueueingCell.min` when `QueueingCell=` is absent) is an
INI-parse-time issue that predates save: whatever uninitialized bytes
happen to be in the BuildingTypeClass instance at the time of save will
be committed verbatim, and will re-appear on load. Neither Save nor Load
normalizes them. **This matters**: the raw-dump discipline means any
in-memory uninitialized-field bug will round-trip through a save/load
cycle unchanged.

### Version handling

None at the BuildingClass level. See §7.

### Corruption recovery

None. First `Read`/`Write` that returns a negative HRESULT causes the
whole object load/save to abort with that HRESULT. BuildingClass::Load has
no "if corrupt, use default" branch; the freshly-allocated instance is
left in a partially-loaded state and the caller must discard it.

### The Phase 1 heap-map maintenance

If `FUN_0041c590` (the ID lookup) returns 0 or "not present", Phase 1
skips the remove — not an error. If a duplicate ID is present it is
*always* removed (rather than flagged as an error). A saved file that
produces two objects with the same ID will silently have only the last-
loaded one reachable via the ID directory. Both objects would still
exist in memory and still be pointer-fixup targets.

### Dump size / `GetClassSize` puzzle

`BuildingClass::GetClassID` (slot 3) is a standard GUID copy. But the
AbstractClass `GetClassSize` (`vtable[12]` at +0x30) for BuildingClass
appears to return `6` (the `AbstractType` enum, `ABS_BUILDING`). That is
suspicious for a byte-count argument to `IStream::Write`. Two possibilities:
1. `vtable[12]` is a class-tag, not a size, and the actual byte count
   is pulled from a separate vtable slot that AbstractClass::Save reads
   but the decompilation misses (look for a second `(**(code**)(...+?))`
   call — maybe at vtable[13] `GetClassSize` in the sense of bytes).
2. The raw dump writes only 6 bytes and the rest of the per-object state
   is reconstructable from the other fixup work. This seems extremely
   unlikely — the whole pattern only makes sense if the full 0x720-byte
   struct is written.

Empirically every other class uses the same `vtable[12]` → Write pattern,
and the game demonstrably can load a building out of a `.sav`, so
hypothesis #1 is far more plausible. **Flagged as open**: confirm the
actual byte count by tracing `vtable[12]` for BuildingClass and at least
one sibling class.

### Non-standard BuildingClass-specific pointers at +0x540 / +0x548 / +0x54C / +0x6F4

These four slots are fixed up on Load but have no §2 entry in the master
report. Either:
- uninvestigated fields (likely), or
- shared fields with TechnoClass that the v2 master report missed.

Behavioral evidence: they survive save/load with old→new pointer fixup,
so they're Abstract-derived object pointers, not raw data. **Flagged
as open** — enumerate these offsets against the struct layout before
implementing a Rust-side Save equivalent.

### Relationship to the multiplayer-CRC `TechnoClass::Save` at 0x0070C270

That symbol is unrelated. It writes state to a checksum-accumulator
(`FUN_004A1D50`, `FUN_004A1CA0`, …), not to an IStream. It is the
per-tick state-hash function for lockstep. Separate research target
(already documented as part of the multiplayer determinism chain).

---

## Sources

- Load: `0x00453E20` (decomp + disasm; vtable slot 5)
- Save: `0x00454190` (decomp + disasm; vtable slot 6)
- CLSID: `0x007E96A0` (= `{0E272DC6-9C0F-11D1-B709-00A024DDAFD1}`)
- TechnoClass::Load_IStream: `FUN_0070BF50`
- TechnoClass::Save_IStream: `FUN_0070C250` (thin forwarder)
- ObjectClass::Load_IStream: `FUN_0065AB80` (corrected 2026-05-29: was FUN_005F5E80; FUN_005F5E80 is the base fixup sub-function called by FUN_0065AB80 — via decompile_function 0x0065ab80)
- ObjectClass::Load_IStream (base fixups sub-function): `FUN_005F5E80`
- ObjectClass::Save_IStream: `FUN_0065AC40`
- AbstractClass::Load: `0x00410380`
- AbstractClass::Save: `0x00410320`
- Pointer-slot register: `FUN_006CF240`
- Swap-map register: `FUN_006CF2C0`
- Fixup dictionary global: `DAT_00B0C110`
- ID directory global: `DAT_00B0E840`
- OLE imports: `StgCreateDocfile` (`ole32!0x123`), `StgOpenStorage` (`0x12A`),
  `OleSaveToStream` / `OleLoadFromStream` (strings at `0x0081086A`,
  `0x00810880`), `StringFromCLSID` (`0x12C`)
- Cross-reference: `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md` §2 (instance layout)
- Cross-reference: `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md` (vtable address table)

## Open questions (for follow-up)

1. Confirm the actual payload byte count written by `AbstractClass::Save`
   — is `vtable[12]` truly `GetClassSize` returning the enum (6), or does
   the save really emit only 6 bytes and rely entirely on the DV bodies +
   fixup replay? Disassemble `AbstractClass::Save` at `0x00410320` under
   a debugger with a real saved BuildingClass and measure stream bytes.
2. Identify fields at +0x540, +0x548, +0x54C, +0x6F4 in BuildingClass — all
   four are fixed up as Abstract-derived pointers but are not in the
   published struct layout.
3. Verify the outer save/load driver (caller of `OleSaveToStream`) to
   document the per-stream naming convention and the order in which
   building / unit / house / anim classes are loaded — the fixup pass's
   correctness depends on every producer having registered its swap-map
   entry before any consumer's fixup queue is resolved.
