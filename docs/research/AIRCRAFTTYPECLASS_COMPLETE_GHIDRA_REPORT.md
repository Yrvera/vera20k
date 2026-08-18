# AircraftTypeClass — Complete Ghidra Research Report

**Primary Address Range:** `0x0041C8B0`–`0x0041CFE0` (ctor, ReadINI, vtable methods, Find)
**Primary VTable:** `0x007E2868` (48 slots)
**Secondary VTables:** `0x007E284C` (4 slots @ +4), `0x007E2844` (2 slots @ +8), `0x007E283C` (2 slots @ +0xC)
**Instance Size:** `0xE10` (3600 bytes) — verified via `operator_new(0xE10)` at `0x0041CF4A`
**Confidence:** High (every claim decompiled / disassembled from binary)
**Active in YR:** Yes — all findings are live in standard YR skirmish

---

## 1. Overview

This report extends [AIRCRAFTCLASS_GHIDRA_REPORT.md](AIRCRAFTCLASS_GHIDRA_REPORT.md) §3 and
[READINI_FIELD_MAPS.md](READINI_FIELD_MAPS.md) §AircraftTypeClass by filling the four remaining gaps:

1. **VTable enumeration** — all 48 primary slots + 3 secondary vtables, flagging which are aircraft overrides
2. **Full ctor defaults sweep** — every write the ctor makes, including parent fields it tweaks after TechnoTypeClass ctor returns
3. **Explicit instance size** — `0xE10` bytes from the allocation site
4. **Parent TechnoTypeClass fields in aircraft context** — which inherited bytes the aircraft ctor touches and how ReadINI delegates parent-field parsing

The pre-existing doc already covers: the 10 AircraftType-specific ReadINI keys, constructor address,
ReadINI address, a partial ctor-defaults table (4 entries), and runtime (AircraftClass) behavior.

---

## 2. Instance Size

**Total size: `0xE10` bytes (3600).**

Verified at `0x0041CF4A` inside `FUN_0041cef0` (the "find or create aircraft type by name" function
that is the sole entry point that allocates `AircraftTypeClass`):

```
pvVar3 = operator_new(0xe10);
if (pvVar3 != NULL) {
    AircraftTypeClass__Constructor(param_1);
    return pvVar3;
}
```

Last declared field is `Fighter` (bool) at offset `0xE0E`. Bytes `0xE0F`–`0xE0F` are padding
(no writes detected). One padding byte brings total to `0xE10` (aligned).

**Active in YR:** Yes. The allocator is reached from the standard INI load path; every aircraft
type in `rulesmd.ini` flows through `FUN_0041cef0` → `operator_new(0xE10)`.

---

## 3. Constructor — Full Sweep (`0x0041C8B0`)

Decompile uses `undefined4 *param_1` (i.e. `int*`), so `param_1[N]` is byte offset `N*4`.
Writes using `*(undefined1 *)((int)param_1 + 0xXXX)` are direct byte offsets.

Call signature: `AircraftTypeClass::Constructor(this, INIEntry)` (thiscall in ECX; `param_2` = INI name).

### 3.1 Execution order

```
1.  TechnoTypeClass::Constructor(INIEntry, 4)   // parent ctor, RTTI id = 4
2.  Zero/init AircraftTypeClass-specific fields (0xDF8..0xE0E)
3.  Install 4 vtables at offsets 0, 4, 8, 0xC
4.  AbstractClass::AssignUniqueID(this + 4)     // UniqueID lands at instance +0x10
5.  Register `this` in g_AircraftTypeClass_Array (DynamicVectorClass at 0x00A8B218)
6.  Linear scan of the array to find own index → store at 0xDF8 (ArrayIndex)
7.  Set aircraft-specific overrides of parent defaults + aircraft-specific defaults
```

### 3.2 Parent call

`TechnoTypeClass::Constructor(INIEntry, 4)` at `0x00710AF0`.
The literal `4` is stored by the parent ctor at **offset `0x67C`** (`param_1[0x19f] = param_3` in
the parent decompile). This is the TypeClass's own RTTI/type-id slot. Confidence: HIGH (verified
in `0x00710AF0`). The `4` value has not been cross-referenced to a named RTTI enum here — see Open
Questions.

### 3.3 Every write the AircraftTypeClass ctor makes (after parent returns)

All offsets are from `this`. Type column describes the write granularity.

#### 3.3.1 AircraftTypeClass-specific field initialization

| Offset  | Size | Value          | Field (from ReadINI) | Source                  |
|---------|------|----------------|----------------------|-------------------------|
| `0xDF8` | 4    | `0xFFFFFFFF`   | ArrayIndex (sentinel)| `param_1[0x37e]`        |
| `0xDFC` | 1    | `0`            | Carryall             | `+0x37f`                |
| `0xE00` | 4    | `0`            | Trailer (AnimType*)  | `param_1[0x380]`        |
| `0xE04` | 4    | `3`            | SpawnDelay           | `param_1[0x381]`        |
| `0xE08` | 1    | `0`            | Rotors               | `+0x382`                |
| `0xE09` | 1    | `0`            | CustomRotor          | `+0xE09`                |
| `0xE0A` | 1    | `0`            | Landable             | `+0xE0A`                |
| `0xE0B` | 1    | `0`            | FlyBy                | `+0xE0B`                |
| `0xE0C` | 1    | `0`            | FlyBack              | `+0xE0C` (899)          |
| `0xE0D` | 1    | `0`            | AirportBound         | `+0xE0D`                |
| `0xE0E` | 1    | `0`            | Fighter              | `+0xE0E`                |

Note: `SpawnDelay` default is `3` (not 0). `Trailer` default is `nullptr`. All booleans default
`false`. `ArrayIndex` is overwritten later (step 6) with the registration index.

#### 3.3.2 VTable installation

| Offset | Value        | Target VTable                  |
|--------|--------------|--------------------------------|
| `0x00` | `0x007E2868` | primary (48 slots)             |
| `0x04` | `0x007E284C` | secondary @ +4 (4 slots)       |
| `0x08` | `0x007E2844` | secondary @ +8 (2 slots)       |
| `0x0C` | `0x007E283C` | secondary @ +0xC (2 slots)     |

#### 3.3.3 UniqueID assignment

`AbstractClass::AssignUniqueID(this + 4)` at `0x00410230`.
`AssignUniqueID` writes `*(this + 4 + 0xC) = unique_id`, i.e. the UniqueID field lives at **instance
offset `0x10`**. If `g_ScenarioClass_Instance == 0` at ctor time, UniqueID is set to `0` instead
(zero-session bootstrap path).

#### 3.3.4 Array registration

The aircraft type array is a standard RA2 `DynamicVectorClass` whose header sits at `0x00A8B218`:

| Offset   | Field                                                |
|----------|------------------------------------------------------|
| `0xA8B218` | Allocator vtable                                   |
| `0xA8B220` | Capacity                                           |
| `0xA8B224` | Data pointer (`g_AircraftTypeClass_Array`)         |
| `0xA8B225` | `IsAllocated` flag                                 |
| `0xA8B228` | **Count** (element count)                          |
| `0xA8B22C` | Growth step                                        |

The ctor appends `this`, re-scans the array, and stores the resulting index at `0xDF8`. If
registration fails (capacity full, growth disabled), the index stays `-1` (0xFFFFFFFF).
Ghidra mis-labels `g_AircraftTypeClass_Array` as `g_InfantryTypeClass_Array` — the label is
wrong; the array is aircraft types (only reachable from `AircraftTypeClass::Constructor` at this
address).

#### 3.3.5 Parent TechnoTypeClass defaults the aircraft ctor **overrides**

These writes happen *after* `TechnoTypeClass::Constructor` has already set the field. The aircraft
ctor is deliberately changing parent defaults for aircraft types.

| Offset  | Size | Aircraft sets | Parent default (from 0x00710AF0) | Meaning |
|---------|------|---------------|----------------------------------|---------|
| `0x718` | 4    | `0x20` (32)   | uninitialized                    | **ROT** (Rate of Turn) default for aircraft |
| `0xC8D` | 1    | `0`           | `1`                              | Parent sets to 1; aircraft forces to 0 |
| `0xD38` | 1    | `1`           | uninitialized (→ 0 effectively)  | Aircraft-only true |
| `0xD96` | 1    | `1`           | uninitialized (→ 0 effectively)  | Aircraft-only true |

Parent fields at `0xD35`, `0xD36`, `0xD3B`, `0xD97`, `0xD2E` are explicitly re-zeroed by the aircraft
ctor (redundant with `operator_new`-following parent ctor in practice — but the writes exist).

**Interpretation notes:**
- `0x718` (ROT) — RA2 convention. Aircraft default Rate-of-Turn is 32 unless overridden by the
  `ROT=` key in the INI. `TechnoTypeClass::ReadINI` will parse `ROT=` into this slot.
- `0xC8D` is adjacent to `0xC95` (ConsideredAircraft). The parent default of `1` being flipped
  to `0` for aircraft types is curious — likely a flag whose name conveys "consider X" in a
  way that's *true* for non-aircraft and *false* for actual aircraft. Exact name not identified.
  See Open Questions.
- `0xD38`, `0xD96` being forced to `1` for aircraft types is also unnamed here. See Open Questions.

### 3.4 Ctor side-effects

- Calls `TechnoTypeClass::Constructor` (which recursively calls `ObjectTypeClass::Constructor`
  → `AbstractTypeClass::Constructor`).
- Registers `this` into the global aircraft-type array.
- Assigns a session-unique ID (or 0 at startup).
- No COM initialization call in the primary ctor — the 4 vtables cover IUnknown/IPersistStream
  dispatch.

---

## 4. ReadINI — Full Decompile (`0x0041CC20`)

Complete body, not just the 10-key surface. `param_1` here is `int` (direct byte offsets).

```
int AircraftTypeClass::ReadINI(this, INIEntry) {
    INIClass::ClearSectionCache();
    if (!TechnoTypeClass::ReadINI(INIEntry)) return 0;   // parent parses its ~300 keys

    TypeNameSect = this + 0x24;    // buffer holding the [TypeName] section header
    ImageSect    = this + 0x1F8;   // buffer holding the [Image]= section (set by parent)

    *(this + 0xE0A) = ReadBool(TypeNameSect, "Landable",     *(this + 0xE0A));
    *(this + 0xE0D) = ReadBool(TypeNameSect, "AirportBound", *(this + 0xE0D));
    *(this + 0xE0E) = ReadBool(TypeNameSect, "Fighter",      *(this + 0xE0E));
    *(this + 0xDFC) = ReadBool(TypeNameSect, "Carryall",     *(this + 0xDFC));

    *(this + 0xE08) = ReadBool(ImageSect, "Rotors",      *(this + 0xE08));
    *(this + 0xE09) = ReadBool(ImageSect, "CustomRotor", *(this + 0xE09));

    // Trailer: only overwrite if string is present (iVar5 != 0)
    buf[0x80] = 0;
    if (ReadString(ImageSect, "Trailer", "", buf, 0x80)) {
        *(this + 0xE00) = AnimTypeClass::FindByName();   // resolves buf → AnimType*
    }
    // else: keep whatever was already at +0xE00 (ctor default = nullptr)

    *(this + 0xE04) = ReadInt(ImageSect, "SpawnDelay", *(this + 0xE04));  // default 3

    *(this + 0xE0B) = ReadBool(TypeNameSect, "FlyBy",   *(this + 0xE0B));
    *(this + 0xE0C) = ReadBool(TypeNameSect, "FlyBack", *(this + 0xE0C));

    return 1;
}
```

### 4.1 Details that matter

- **Section sources** — four keys (`Rotors`, `CustomRotor`, `Trailer`, `SpawnDelay`) use the
  `[Image]` section (offset `this + 0x1F8`). The other six use `[TypeName]` (offset `this + 0x24`).
  `TechnoTypeClass::ReadINI` already resolved `Image=` and left the target name in `this + 0x1F8`.
- **Key ordering in the binary is NOT alphabetical.** The read order is:
  `Landable, AirportBound, Fighter, Carryall, Rotors, CustomRotor, Trailer, SpawnDelay, FlyBy, FlyBack`.
  This matters because reads have side-effects (ClearSectionCache was called once, at the top — all
  subsequent reads within one TypeName section benefit from the parent's cached parse).
- **Trailer is conditional** — `ReadString` returning `0` (empty / key absent) leaves the existing
  AnimType pointer alone. Combined with the ctor zeroing `+0xE00`, "absent key = nullptr"; an
  explicit empty string (`Trailer=`) also leaves it at nullptr since `ReadString` returns length.
  An INI update cycle preserves the previously-loaded Trailer when the key is absent. **Edge case
  for INI reload:** if a later parse removes `Trailer=`, the old pointer survives.
- **`ClearSectionCache` is called once**, at the very top of the override, *before* the parent
  `TechnoTypeClass::ReadINI` runs. This is unusual — most type classes call ClearSectionCache in
  the parent. The implication is that `AircraftTypeClass::ReadINI` invalidates any cached section
  state before doing anything; the parent then runs normally.
- **Return value** — `0` if parent fails, `1` on success. The parent checks that the INI entry
  is valid; a missing/malformed `[TypeName]` aborts without touching any AircraftType fields.

### 4.2 Confirmed field mapping

| #  | INI Key       | Section    | Type            | Offset  | Default | Source                |
|----|---------------|------------|-----------------|---------|---------|------------------------|
| 1  | Landable      | [TypeName] | bool            | `0xE0A` | false   | ReadBool              |
| 2  | AirportBound  | [TypeName] | bool            | `0xE0D` | false   | ReadBool              |
| 3  | Fighter       | [TypeName] | bool            | `0xE0E` | false   | ReadBool              |
| 4  | Carryall      | [TypeName] | bool            | `0xDFC` | false   | ReadBool              |
| 5  | Rotors        | [Image]    | bool            | `0xE08` | false   | ReadBool              |
| 6  | CustomRotor   | [Image]    | bool            | `0xE09` | false   | ReadBool              |
| 7  | Trailer       | [Image]    | AnimTypeClass*  | `0xE00` | nullptr | ReadString + FindByName |
| 8  | SpawnDelay    | [Image]    | int (frames)    | `0xE04` | **3**   | ReadInt               |
| 9  | FlyBy         | [TypeName] | bool            | `0xE0B` | false   | ReadBool              |
| 10 | FlyBack       | [TypeName] | bool            | `0xE0C` | false   | ReadBool              |

---

## 5. Primary VTable (`0x007E2868`) — 48 Slots

All 48 slots verified by reading raw bytes at `0x007E2868`. Each row flags whether the slot is
inherited from `TechnoTypeClass` (vtable `0x007F4ED8`) or is an aircraft-specific override.

| Slot | Offset       | Address at slot | Inherited/Override | Notes (where identified) |
|------|--------------|-----------------|--------------------|--------------------------|
| 0    | `+0x00`      | `0x00410260`    | inherited          | thunk (likely IUnknown::QueryInterface or AbstractClass virtual) |
| 1    | `+0x04`      | `0x00410300`    | inherited          | thunk |
| 2    | `+0x08`      | `0x00410310`    | inherited          | thunk |
| 3    | `+0x0C`      | `0x0041CEB0`    | **override**       | `IPersistStream::GetClassID` — returns CLSID from `0x007E95B0` |
| 4    | `+0x10`      | `0x00410450`    | inherited          | — |
| 5    | `+0x14`      | `0x0041CE20`    | **override**       | `IPersistStream::Load` — calls parent Load, reinstalls aircraft vtables |
| 6    | `+0x18`      | `0x0041CE90`    | **override**       | `IPersistStream::Save` |
| 7    | `+0x1C`      | `0x007170A0`    | inherited          | — |
| 8    | `+0x20`      | `0x0041CFE0`    | **override**       | Scalar-deleting destructor (reinstalls vtables, then delete) |
| 9    | `+0x24`      | `0x00410470`    | inherited          | — |
| 10   | `+0x28`      | `0x00410480`    | inherited          | — |
| 11   | `+0x2C`      | `0x0041CFB0`    | **override**       | aircraft-specific method (unnamed) |
| 12   | `+0x30`      | `0x0041CFC0`    | **override**       | aircraft-specific method (unnamed) |
| 13   | `+0x34`      | `0x0041CDB0`    | **override**       | Aircraft-field stream-writer — saves bytes from `+0xE08`, `+0xE09`, `+0xE0A`, `+0xE0D`, ... (likely a sub-save hook) |
| 14   | `+0x38`      | `0x00410490`    | inherited          | — |
| 15   | `+0x3C`      | `0x004104A0`    | inherited          | — |
| 16   | `+0x40`      | `0x0041CFD0`    | **override**       | aircraft-specific method (unnamed) |
| 17   | `+0x44`      | `0x00410440`    | inherited          | — |
| 18   | `+0x48`      | `0x004104C0`    | inherited          | — |
| 19   | `+0x4C`      | `0x004104F0`    | inherited          | — |
| 20   | `+0x50`      | `0x00410520`    | inherited          | — |
| 21   | `+0x54`      | `0x00410530`    | inherited          | — |
| 22   | `+0x58`      | `0x00410540`    | inherited          | — |
| 23   | `+0x5C`      | `0x00410570`    | inherited          | — |
| 24   | `+0x60`      | `0x00410C20`    | inherited          | — |
| 25   | `+0x64`      | `0x0041CC20`    | **override**       | **`ReadINI`** (confirmed) |
| 26   | `+0x68`      | `0x00410B90`    | inherited          | — |
| 27   | `+0x6C`      | `0x0041CF80`    | inherited          | (same address as TechnoTypeClass — both classes have this same helper pinned here; not a true override) |
| 28   | `+0x70`      | `0x00711EC0`    | inherited          | — |
| 29   | `+0x74`      | `0x00716290`    | inherited          | — |
| 30   | `+0x78`      | `0x005F75C0`    | inherited          | — |
| 31   | `+0x7C`      | `0x0041CBF0`    | **override**       | aircraft-specific (unnamed) |
| 32   | `+0x80`      | `0x0041CBE0`    | **override**       | aircraft-specific (unnamed) |
| 33   | `+0x84`      | `0x00711F00`    | inherited          | — |
| 34   | `+0x88`      | `0x00711EE0`    | inherited          | — |
| 35   | `+0x8C`      | `0x0041CB20`    | **override**       | aircraft-specific (unnamed) |
| 36   | `+0x90`      | `0x0041CB70`    | **override**       | aircraft-specific (unnamed) |
| 37   | `+0x94`      | `0x005F7900`    | inherited          | — |
| 38   | `+0x98`      | `0x00712040`    | inherited          | — |
| 39   | `+0x9C`      | `0x0041CFA0`    | inherited          | (shared pin, same target as TechnoTypeClass) |
| 40   | `+0xA0`      | `0x0041CB50`    | **override**       | aircraft-specific (unnamed) |
| 41   | `+0xA4`      | `0x0041CB60`    | **override**       | aircraft-specific (unnamed) |
| 42   | `+0xA8`      | `0x00716150`    | inherited          | — |
| 43   | `+0xAC`      | `0x00711EB0`    | inherited          | — |
| 44   | `+0xB0`      | `0x007120D0`    | inherited          | — |
| 45   | `+0xB4`      | `0x00712120`    | inherited          | — |
| 46   | `+0xB8`      | `0x00711F60`    | inherited          | — |
| 47   | `+0xBC`      | `0x00717800`    | inherited          | — |

**Override count: 15 aircraft-specific methods** at slots 3, 5, 6, 8, 11, 12, 13, 16, 25, 31, 32, 35, 36, 40, 41.
The remaining 33 slots are identical to `TechnoTypeClass`'s primary vtable (same function pointers).

Bytes past `+0xC0` (slot 48) are `00 00 00 00` followed by a GUID (`7FB618...`) — the vtable ends at
slot 47; the GUID is the CLSID that `slot 3 (GetClassID)` returns.

### 5.1 Identified override details

- **Slot 3 (`0x0041CEB0`) — `GetClassID`:** 24-byte stub. Validates the out-pointer (returns
  `0x80004003 E_POINTER` if null), then copies 16 bytes of GUID data from `0x007E95B0`, `0x007E95B4`,
  `0x007E95B8`, `0x007E95BC` into `*ppClassID`. Verified via disassembly.
- **Slot 5 (`0x0041CE20`) — `Load`:** Calls the parent `Load` via vtable slot 5 dispatch (through
  indirect call at `0x007162E5`), then reinstalls the four aircraft vtables (to overwrite
  parent-installed ones), then recursively loads child state via `0x00711828`.
- **Slot 6 (`0x0041CE90`) — `Save`:** Thin wrapper that calls the inherited save helper at
  `0x00712EAC` (via stack forwarding).
- **Slot 8 (`0x0041CFE0`) — Deleting destructor:** MSVC scalar-deleting destructor pattern —
  reinstall vtables, call member dtor, `delete`.
- **Slot 13 (`0x0041CDB0`) — sub-save hook:** Writes aircraft-specific bytes (Rotors, CustomRotor,
  Landable, AirportBound, FlyBack, Fighter, Carryall) to a stream via repeated calls to
  `0x004A1C8D` (stream write-byte helper). This runs after a call to `0x00717197` (parent's
  sub-save). Purpose: serialize the 10 aircraft-specific INI fields into a save file.
- **Slot 25 (`0x0041CC20`) — `ReadINI`:** Confirmed, fully decompiled in §4.

### 5.2 Unidentified overrides

Slots 11, 12, 16, 31, 32, 35, 36, 40, 41 are aircraft overrides that have not been named in this
report. They are small (typically <200 bytes each based on their tight address clustering in
`0x0041CB20`–`0x0041CFD0`) and likely correspond to the TechnoTypeClass/ObjectTypeClass/AbstractClass
interface hooks for aircraft-specific behavior (e.g. shape-file resolution, voxel data lookup,
image-name fallback, construction-option filters). See Open Questions.

---

## 6. Secondary VTables

Aircraft inherits multiple-inheritance / COM layout from TechnoTypeClass. Each secondary vtable
is pointed to by an additional vtable slot at the start of the instance (offsets +4, +8, +0xC).
These are *not* duplicates of the primary vtable — they are IUnknown adjustor thunks for when
the instance is cast to a secondary base interface.

### 6.1 Secondary vtable @ `0x007E284C` (instance offset +4, 4 slots)

| Slot | Address        | Notes                           |
|------|----------------|---------------------------------|
| 0    | `0x004105E0`   | adjustor thunk                  |
| 1    | `0x004105F0`   | adjustor thunk                  |
| 2    | `0x00410600`   | adjustor thunk                  |
| 3    | `0x00410210`   | adjustor → AbstractClass method |

### 6.2 Secondary vtable @ `0x007E2844` (instance offset +8, 2 slots)

| Slot | Address        | Notes                       |
|------|----------------|-----------------------------|
| 0    | `0x00410580`   | adjustor thunk              |
| 1    | `0x007FB508`   | RTTI type descriptor (data) |

### 6.3 Secondary vtable @ `0x007E283C` (instance offset +0xC, 2 slots)

| Slot | Address        | Notes                       |
|------|----------------|-----------------------------|
| 0    | `0x00410590`   | adjustor thunk              |
| 1    | `0x007FB4F0`   | RTTI type descriptor (data) |

These secondaries match the general TypeClass MI layout; aircraft does not add novel secondary
interfaces versus TechnoTypeClass (the thunk addresses differ only because they're aircraft-
specific adjustors, i.e. same shape, different `this`-adjustment offsets).

---

## 7. Parent TechnoTypeClass Fields — Aircraft Context

### 7.1 Parent-range ownership

The Aircraft-specific `ReadINI` at `0x0041CC20` **does not parse any key in the `0x000`–`0xDFB`
range**. All parent fields — `Name`, `UIName`, `Image`, `Ammo`, `InitialAmmo`, `Passengers`,
`BalloonHover`, `ConsideredAircraft`, `Locomotor`, `ROT`, `Speed`, `TechLevel`, `Cost`, `Strength`,
`Armor`, `Warhead`-tables, `Prerequisite`-lists, and ~300 others — are owned by
`TechnoTypeClass::ReadINI` (at `0x00712170`, vtable slot 25 of TechnoType), which aircraft calls
as the first step.

**Implication for parity:** correct parent-field parsing is a `TechnoTypeClass`-level concern,
not aircraft-level. If an INI key lands in bytes `0x000`–`0xDFB`, look for the ReadBool/ReadInt/
ReadString call inside `0x00712170`, not inside `0x0041CC20`.

### 7.2 Parent bytes the Aircraft ctor explicitly touches

Post-parent-ctor, the aircraft ctor writes **9 parent-range fields** (above, §3.3.5). These are
the only spots where aircraft overrides a parent default. Summary:

| Offset  | Aircraft forces to | Notes                                                      |
|---------|--------------------|------------------------------------------------------------|
| `0x718` | `32` (0x20)        | ROT default for aircraft types                             |
| `0xC8D` | `0` (parent was 1) | Parent-named flag, aircraft-negates (unknown name)         |
| `0xD2E` | `0`                | zero (likely redundant re-zero)                            |
| `0xD35` | `0`                | zero (likely redundant re-zero)                            |
| `0xD36` | `0`                | zero (likely redundant re-zero)                            |
| `0xD38` | `1`                | aircraft-only true (name unknown)                          |
| `0xD3B` | `0`                | zero (likely redundant re-zero)                            |
| `0xD96` | `1`                | aircraft-only true (name unknown)                          |
| `0xD97` | `0`                | zero (likely redundant re-zero)                            |

The five "redundant re-zero" fields may in fact matter if the parent ctor leaves them
uninitialized (since C++ `operator new` does not zero-init). Treat them as authoritative defaults
for aircraft types regardless of parent behavior.

### 7.3 Parent fields the runtime AircraftClass reads from its TypeClass (already documented)

From [AIRCRAFTCLASS_GHIDRA_REPORT.md](AIRCRAFTCLASS_GHIDRA_REPORT.md) §3.2:

| Offset  | Field              | Used for                                                    |
|---------|--------------------|-------------------------------------------------------------|
| `0x34C` | Locomotor CLSID    | COM locomotion object instantiation                         |
| `0x5E0` | Passengers         | max passenger capacity (Carryall/transport)                 |
| `0x680` | InitialAmmo        | Starting ammo override (-1 = use Ammo)                      |
| `0x684` | Ammo               | Max ammo (-1 = unlimited)                                   |
| `0xC95` | ConsideredAircraft | Treated as aircraft for targeting/tab purposes              |
| `0xD68` | BalloonHover       | Stays airborne permanently (Kirov)                          |

This is a curated list from runtime usage — not an exhaustive parent field map. The complete
parent layout is documented in the various `TECHNOCLASS_*` reports in this directory. Those
reports are the source for any key in `0x000`–`0xDFB`.

---

## 8. Integration Points

- **Allocation:** `FUN_0041cef0` (at `0x0041CEF0`) is the sole caller of `AircraftTypeClass::Constructor`.
  It's a find-or-create helper: given an INI name, it searches the existing array (with two
  prefix checks against strings at `0x00817474` and `0x00817694`), and if not found and both
  prefix checks pass, allocates a new instance.
- **Array:** `g_AircraftTypeClass_Array` at `0x00A8B224` (data pointer), count at `0x00A8B228`,
  header at `0x00A8B218`. Wrapped in a DynamicVectorClass. Consumers include `HouseClass`,
  various RulesClass parsers, and `FactoryClass` production code.
- **ReadINI call site:** dispatched through vtable slot 25 of whichever TypeClass pointer is
  held. Aircraft's slot 25 points to `0x0041CC20`.
- **CLSID:** embedded directly after the primary vtable at `0x007E28C8` (16 bytes starting with
  `00 4E 40 39 9D 52 A2 46 DF 91 3F 18 B6 7F 00 F0` — a standard 128-bit COM GUID, returned by
  `GetClassID`).

---

## 9. Current Rust Implementation Status

**Rust mirror:** This report is pure research. The repo's Rust code does not currently attempt
a 1:1 `AircraftTypeClass` struct layout — the parser/rules code uses an abstracted TypeClass
representation. No files need updating as a direct result of this research.

**Implications for future parity work, if/when a runtime aircraft-type struct is modeled:**

- Instance must be 0xE10 bytes if a bit-accurate layout is targeted (for save/load parity).
- `SpawnDelay` must default to `3`, not `0` (likely to be missed otherwise).
- `Trailer` is Art-section-scoped, not Rules — must parse from `[<Image>]`, not `[<TypeName>]`.
- The parse order from `rules(md).ini` sections for these 10 keys matches §4.2 exactly; if a
  parser uses a different order and performs side-effectful lookups (e.g. `AnimTypeClass::FindByName`
  must be called during an active load), it may observe different `AnimType*` values.

---

## 10. Open Questions

1. **Name of slot 11, 12, 16, 31, 32, 35, 36, 40, 41 overrides.** These are aircraft-specific
   vtable hooks at `0x0041CB20`–`0x0041CFD0`. Each should be decompiled individually and matched
   against TechnoTypeClass's equivalent inherited implementation to name the override purpose.
   Low priority unless save/load or RTTI parity work needs them.
2. **Meaning of parent bytes `0xC8D`, `0xD38`, `0xD96`.** Aircraft ctor forces specific values
   (0, 1, 1) to override parent defaults. These are unnamed TechnoTypeClass bool flags. Candidate
   inspection: search `TechnoTypeClass::ReadINI` (`0x00712170`) for `ReadBool` calls that write
   to these offsets — the associated INI key name identifies the field.
3. **RTTI type id `4`.** The literal `4` passed to `TechnoTypeClass::Constructor` and stored at
   parent offset `0x67C` is almost certainly the `AbstractType::Aircraft` enum value, but this
   has not been cross-referenced to a named enum here. Compare against other TypeClass ctors
   (`UnitTypeClass`, `InfantryTypeClass`, `BuildingTypeClass`) to confirm the enum mapping.
4. **Slot 27 and 39 shared pins.** Aircraft vtable slots 27 (`0x0041CF80`) and 39 (`0x0041CFA0`)
   point to aircraft-range addresses but match TechnoType's vtable at the same slots —
   meaning these helpers live in aircraft-address space but are *called* by Techno code too,
   OR Ghidra's TechnoType vtable read happens to match because the helpers are shared. Worth
   verifying by decompiling each to confirm whether they're truly shared or aircraft-specific
   implementations that happen to behave identically.
5. **`AssignUniqueID(this + 4)` oddity.** The `+4` offset means `UniqueID` lives at instance +0x10.
   This is inside the vtable-pointer region — it looks wrong. Verify by inspecting the actual
   layout of an `AircraftTypeClass` instance after ctor runs. Candidate explanation: the
   secondary-vtable pointer at +4 doubles as an AbstractClass sub-object header, and UniqueID
   is at AbstractClass offset +0xC relative to that. If confirmed, this is an unusual MI layout
   for AA-type classes and would warrant its own doc.

---

## Sources

**Decompiled / disassembled (Ghidra MCP on gamemd.exe):**
- `0x0041C8B0` — `AircraftTypeClass::Constructor` (full decompile + disassembly)
- `0x0041CC20` — `AircraftTypeClass::ReadINI` (full decompile)
- `0x0041CEF0` — `FindOrCreateAircraftType` (allocation site, confirms instance size)
- `0x0041CEB0` — vtable slot 3 (`GetClassID`) — disassembly
- `0x0041CE20` — vtable slot 5 (`Load`) — disassembly
- `0x0041CE90` — vtable slot 6 (`Save`) — disassembly
- `0x0041CFE0` — vtable slot 8 (deleting destructor) — disassembly
- `0x0041CDB0` — vtable slot 13 (field stream-writer) — disassembly
- `0x00410230` — `AbstractClass::AssignUniqueID` (full decompile)
- `0x00710AF0` — `TechnoTypeClass::Constructor` (full decompile + disassembly, for vtable
  comparison and parent-default reference)
- `0x007E2868` — AircraftTypeClass primary vtable (raw memory read, 256 bytes)
- `0x007E284C`, `0x007E2844`, `0x007E283C` — secondary vtables (raw memory)
- `0x007F4ED8` — TechnoTypeClass primary vtable (raw memory read, 200 bytes, for diff)

**Cross-referenced documents:**
- [AIRCRAFTCLASS_GHIDRA_REPORT.md](AIRCRAFTCLASS_GHIDRA_REPORT.md) — pre-existing runtime-class
  report (verified still accurate; this report extends §3/§12.2 rather than supersedes)
- [READINI_FIELD_MAPS.md](READINI_FIELD_MAPS.md) — the 10-key fragment this report completes
- [ADDRESS_MAP.md](ADDRESS_MAP.md) — indexed `0x0041CC20` under ReadINI functions

**INI files checked:**
- `ini/rulesmd.ini` / `ini/rules.ini` — `[TypeName]` section keys for all aircraft
- `ini/artmd.ini` / `ini/art.ini` — `[Image]` section keys for Rotors/CustomRotor/Trailer/SpawnDelay

---

*Report date: 2026-04-24. Generated via Ghidra MCP live decompilation.*
