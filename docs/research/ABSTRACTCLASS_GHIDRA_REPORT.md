# AbstractClass — Ghidra Research Report

**Primary addresses:** Constructor `0x00410170`, Destructor `0x004101F0`, Vtable `0x007E1F50`
**Confidence:** HIGH (decompiled ~20 functions, verified from binary)
**Active in YR:** Yes — this is the root base class for every game object in both TS and YR

## 1. Overview

AbstractClass is the root base class for every game object in gamemd.exe. It provides COM
interface compliance (IUnknown, IPersistStream), a notification system (INoticeSink,
INoticeSource), RTTI identification, unique instance tracking, CRC computation for
deterministic sync, and a polymorphic virtual method table with ~28 slots.

All concrete game objects (units, buildings, cells, bullets, anims, houses, etc.) inherit
from AbstractClass. The class uses MSVC multiple inheritance with 4 vtable pointers.

**Inheritance interfaces:**
```
AbstractClass : IPersistStream, IRTTITypeInfo, INoticeSink, INoticeSource
```

## 2. Class Layout (0x24 bytes = 36 bytes)

All offsets are BYTE offsets verified from the constructor assembly at `0x00410170`.

| Offset | Size | Type | Name | Init Value | Evidence |
|--------|------|------|------|-----------|----------|
| 0x00 | 4 | `void*` | vtable_primary | `0x007E1F50` | Constructor sets `[EAX]` |
| 0x04 | 4 | `void*` | vtable_IRTTITypeInfo | `0x007E1F34` | Constructor sets `[EAX+4]` |
| 0x08 | 4 | `void*` | vtable_INoticeSink | `0x007E1F2C` | Constructor sets `[EAX+8]` |
| 0x0C | 4 | `void*` | vtable_INoticeSource | `0x007E1F24` | Constructor sets `[EAX+C]` |
| 0x10 | 4 | `int` | UniqueID | `0xFFFFFFFF` | `MOV [EAX+0x10], 0xFFFFFFFF` |
| 0x14 | 1 | `byte` | AbstractFlags | `& 0xF8` (bits 0-2 cleared) | `AND CL, 0xF8` at `0x0041018E` |
| 0x15 | 3 | — | padding | — | Not initialized |
| 0x18 | 4 | `void*` | field_0x18 | `0` | `MOV [EAX+0x18], 0` |
| 0x1C | 4 | `int` | field_0x1C | `0` | `MOV [EAX+0x1C], 0` |
| 0x20 | 1 | `bool` | Dirty | `false` | `MOV byte [EAX+0x20], 0` |
| 0x21 | 3 | — | padding | — | Alignment to 0x24 |

### Constructor Variants

There are three "constructor" functions, each used in different contexts:

| Address | Name | What It Does | Used By |
|---------|------|-------------|---------|
| `0x00410170` | `AbstractClass__Constructor_Full` | Sets all 4 vtable pointers, initializes UniqueID to -1, clears AbstractFlags bits 0-2, zeroes field_0x18/0x1C/Dirty | **Real constructors** of derived classes (44+ callers) |
| `0x004101C0` | `AbstractClass__Constructor_VtablesOnly` | Only sets vtable pointers, no field initialization | Some re-initialization contexts |
| `0x004101F0` | `AbstractClass__Destructor_ResetVtables` | Only sets AbstractClass vtable pointers | **Destructors** of all derived classes |

**Critical labeling note:** In Ghidra, all three were previously labeled as "constructors"
or "INoticeSink__Constructor". The function at `0x004101F0` is called at the END of
destructors to reset vtable pointers to the base class — it is NOT a constructor despite
its old label. Similarly, derived class functions labeled `*__Constructor` at addresses
like `0x0047BB60` (CellClass), `0x005F3B80` (ObjectClass), `0x004109C0`
(AbstractTypeClass) are actually **destructors** — they reset vtables, clean up resources,
and call the base destructor. The real constructors are the ones found via callers of
`AbstractClass__AssignUniqueID` (`0x00410230`).

### Constructor Sequence (verified from CellClass, FactoryClass, HouseClass constructors)

```
1. Call AbstractClass__Constructor_Full(this)     // 0x00410170
   - Sets INoticeSink/INoticeSource vtables first (then overwritten by AbstractClass vtables)
   - UniqueID = 0xFFFFFFFF
   - AbstractFlags bits 0-2 = 0
   - field_0x18 = 0, field_0x1C = 0, Dirty = false

2. Initialize derived class fields
   - Derived class sets its own fields at offsets >= 0x24

3. Set derived class vtable pointers
   - Overwrites [0x00], [0x04], [0x08], [0x0C] with derived vtable addresses

4. Call AbstractClass__AssignUniqueID(this + 4)   // 0x00410230
   - Passes the IRTTITypeInfo vtable pointer (this + 4)
   - Reads global Heap pointer at 0x00A8B230
   - If Heap exists: calls Heap::GetNextID() at 0x0068BCB0
     → Increments counter at Heap+0x214, returns new ID in EAX
     → Stores at [param_1 + 0x0C] = this+0x10 = UniqueID
   - If no Heap: sets UniqueID = 0

5. Register in class-specific global array (DynamicVectorClass)
```

### Destructor Sequence

```
1. Set own vtable pointers (so virtual calls during destruction resolve correctly)
2. Remove self from global arrays (DynamicVectorClass removal loops)
3. Clean up owned resources (release references, clear pointers)
4. Call AbstractClass__Destructor_ResetVtables(this)  // 0x004101F0
   - Resets vtable pointers to AbstractClass base vtable
```

## 3. UniqueID System

**Global object pointer:** `0x00A8B230` — pointer to the object that owns the UniqueID counter.
*(corrected 2026-05-28: doc previously named this "AbstractClass Heap pointer" / "Heap object"; Ghidra labels the
global `g_ScenarioClass_Instance`, not a Heap. The behavior description below is accurate; the "Heap" naming
was MISLEADING — ROOT_CAUSE: RTTI_LABEL_DRIFT. Verified via `decompile_function 0x00410230`.)*

**ID assignment function:** `AbstractClass__AssignUniqueID` at `0x00410230`.

Assembly at 0x00410230:
```asm
MOV ECX, [0x00A8B230]    ; Load global object pointer (labeled g_ScenarioClass_Instance in Ghidra)
TEST ECX, ECX
JNZ  assign               ; If object exists, get next ID
; else: set UniqueID = 0
MOV ECX, [ESP+4]          ; param_1 (this+4)
XOR EAX, EAX
MOV [ECX+0xC], EAX        ; UniqueID = 0
RET 4

assign:
CALL 0x0068BCB0           ; FUN_0068bcb0 — increments [ECX+0x214], returns new value in EAX
MOV EDX, [ESP+4]
MOV [EDX+0xC], EAX        ; UniqueID = returned ID
RET 4
```

**FUN_0068bcb0** (ID counter increment) at `0x0068BCB0`:
```asm
MOV EAX, [ECX+0x214]     ; Load current counter (from object at ECX, offset +0x214)
INC EAX                    ; Increment
MOV [ECX+0x214], EAX     ; Store back
RET                        ; Return new value in EAX
```
*(Note: Ghidra decompiles this as void-return due to incorrect return type; raw bytes confirm EAX is loaded and
returned. Verified via read_memory at 0x0068BCB0.)*

UniqueIDs are monotonically increasing integers allocated from a global counter at offset +0x214 of the object
pointed to by `[0x00A8B230]`. Each object gets a unique ID at construction time. The
sentinel value `0xFFFFFFFF` indicates an uninitialized/unassigned ID.

## 4. AbstractFlags (Offset 0x14)

The AbstractFlags byte contains at least 3 bit flags in the low 3 bits. The constructor
clears bits 0-2 via `AND CL, 0xF8`.

| Bit | Mask | Name | Meaning | Confidence |
|-----|------|------|---------|-----------|
| 0 | 0x01 | **IsTechno** | Set when the object is a TechnoClass (or any Foot/Building derivative). Gates access to TechnoTypeClass fields. | HIGH — set by `TechnoClass::Constructor` at `0x006F322F` (`OR AL, 0x1`); checked by `ObjectClass::Select`, `ObjectClass::UnInit`, and NavTarget is-techno probes |
| 1 | 0x02 | **IsObject** | "This object has been through the ObjectClass ctor." Set immediately after ObjectClass fields are zeroed, while `InLimbo` is still true — so it is NOT "IsOnMap". | HIGH — set by `ObjectClass::Constructor` at `0x005F3B37` (`OR byte [ESI+0x14], 2`) |
| 2 | 0x04 | **IsFoot** | True for FootClass-derived (Infantry / Unit / Aircraft), false for Building. Gates Foot-only logic such as NavTarget, PathFinder, OccupyableCell checks. | HIGH — set by `FootClass::Constructor` at `0x004D34DD` (`OR DL, 0x4`) |
| 3-7 | — | Unknown | Not set by any constructor in the chain | — |

All 3 low bits are cleared in the base constructor via `AND CL, 0xF8`. Derived
constructors then OR in their own bit: ObjectClass sets bit 1, TechnoClass sets bit 0,
FootClass sets bit 2. BuildingClass inherits bit 0 from TechnoClass but does NOT set
bit 2 — that's the fast path for the "non-building TechnoClass" check (`(byte @+0x14) & 0x04`).

## 5. Primary Virtual Table (28 entries at 0x007E1F50)

`param_1` in all base implementations is `undefined4 *` (4-byte stride). Offsets in the
struct access comments use BYTE offsets.

### COM Interface Methods (IUnknown + IPersistStream)

| Slot | VT Offset | Address | Name | Implementation |
|------|-----------|---------|------|---------------|
| 0 | 0x00 | `0x00410260` | QueryInterface | Checks 4 GUIDs: IUnknown (`{00000000-…-46}`), IPersistStream (`{00000109-…-46}`), IPersist (`{0000010C-…-46}`) return primary `this`; custom GUID (`{170DAC82-12E4-11D2-8175-006008055BB5}`) returns `this+4` (IRTTITypeInfo interface) |
| 1 | 0x04 | `0x00410300` | AddRef | Always returns 1 (no real refcount) |
| 2 | 0x08 | `0x00410310` | Release | Always returns 1 (no real refcount) |
| 3 | 0x0C | `0x004C9150` | GetClassID (pure virtual, IPersistStream) | Derived classes implement GetClassID for COM IPersist. **NOT WhatAmI** — WhatAmI is at +0x2C (slot 11). |
| 4 | 0x10 | `0x00410450` | IsDirty | `return !(byte at this+0x20)` — S_OK (0) if dirty, S_FALSE (1) if clean |
| 5 | 0x14 | `0x004C9150` | Load (IPersistStream) | Stub returns 0. ObjectClass override at `0x005F5E80` reads from stream + registers pointers for swizzle |
| 6 | 0x18 | `0x004C9150` | Save (IPersistStream) | Stub returns 0. ObjectClass override serializes state |
| 7 | 0x1C | `0x004103E0` | GetSizeMax | Calls `vtable[0x30]` (GetSize) to compute serialized size, writes to ULARGE_INTEGER output |

### Game Virtual Methods

| Slot | VT Offset | Address | Name | Implementation | ObjectClass Override |
|------|-----------|---------|------|---------------|---------------------|
| 8 | 0x20 | `0x004105A0` | ScalarDeletingDestructor | Resets vtables to AbstractClass; if bit 0 of param set, calls `operator delete` at `0x007C8B3D` | `0x005F6DC0` |
| 9 | 0x24 | `0x00410470` | Init | No-op (called after Load to finalize initialization) | Same (no-op) |
| 10 | 0x28 | `0x00410480` | PointerExpiredNotification | No-op. Params: `(void* expired_ptr, bool was_removed)` | `0x005F5230` — clears pointers at +0x34, +0x30, +0x88 if they match |
| 11 | 0x2C | `0x004C9150` | **WhatAmI / IRTTITypeInfo::Process** (pure virtual) | Every derived class overrides this to return its RTTI enum (Unit=1, Aircraft=4, Building=6, Infantry=0xF, Overlay=0x14, Terrain=0x24). Callers: `ObjectClass::Mark`, `ObjectClass::ReceiveDamage`, `ObjectClass::AI`, `ObjectClass::UnInit`, and most `Filter_AbstractType_InMap`-style dispatch. Also exposed via IRTTITypeInfo secondary vtable thunk at `0x00410210` (`this-4` adjustor). | Pure (each derived class overrides). |
| 12 | 0x30 | `0x004C9150` | GetSize | Returns 0. Used by GetSizeMax for serialized size computation | Same (ret 0) |
| 13 | 0x34 | `0x00410410` | ComputeCRC | Feeds `UniqueID` (+0x10, 4 bytes) and `Dirty` (+0x20, 1 byte) to CRC engine via `CRCEngine__AddData` | `0x005F6250` — calls base, then adds ObjectClass fields: coords (+0x9C/A0/A4), state flags (+0x74, +0x80, etc.) |
| 14 | 0x38 | `0x00410490` | GetOwningHouseIndex | Returns -1 (`OR EAX, 0xFFFFFFFF`). No owning house by default | Same (ret -1) |
| 15 | 0x3C | `0x004104A0` | Unknown_0x3C | Returns 0 (false) | Same |
| 16 | 0x40 | `0x004104B0` | Unknown_0x40 | Returns 0 (false) | Same |
| 17 | 0x44 | `0x00410440` | IsAlive | Returns true (`MOV AL, 1`). Base assumes alive | `0x005F6690` — `return *(byte*)(this+0x90) == 0` (checks ObjectClass death flag) |
| 18 | 0x48 | `0x004104C0` | GetCoords | Returns `{0, 0, 0}` from global at `0x00887680` (12-byte CoordStruct) | `0x005F65A0` — copies from ObjectClass +0x9C/+0xA0/+0xA4 |
| 19 | 0x4C | `0x004104F0` | GetCenterCoords | Calls `this->vtable[0x48]` (GetCoords) — delegates | Same (delegates) |
| 20 | 0x50 | `0x00410520` | IsFallingDown_Early | Returns false. ObjectClass override checks falling flag + health threshold | `0x005F6B60` |
| 21 | 0x54 | `0x00410530` | IsFallingDown_Late | Returns false. ObjectClass override is complement of slot 20 | `0x005F6B90` |
| 22 | 0x58 | `0x00410540` | GetFLH | Calls `this->vtable[0x48]` (GetCoords) — delegates to GetCoords in base | Same (delegates) |
| 23 | 0x5C | `0x00410570` | Update / AI | Just `RET` (no-op). Per-tick processing | `0x005F3E70` — complex: handles falling/sinking, health decay, sound, splash anims |
| 24 | 0x60 | `0x007FB160` | RemoveThis (pure virtual) | Data/purecall — not a real function in base | `0x005F6DA0` — clears pointer at +0x88 if it matches param |
| 25 | 0x64 | `0x004C9150` | Unknown_0x64 | Returns 0 | `0x00426390` |
| 26 | 0x68 | `0x007FAFC0` | Unknown_0x68 (pure virtual) | Data/purecall — not a real function in base | `0x004263A0` |
| 27 | 0x6C | `0x004C9150` | Unknown_0x6C | Returns 0 | `0x005F3E30` — dispatches through type object vtable |

## 6. Secondary Virtual Tables

### IRTTITypeInfo Interface (offset +0x04 in object, vtable at 0x007E1F34)

Custom game interface identified by GUID `{170DAC82-12E4-11D2-8175-006008055BB5}`.

| Slot | Address | Name | Implementation |
|------|---------|------|---------------|
| 0 | `0x004105E0` | QueryInterface (thunk) | `SUB [ESP+4], 4; JMP AbstractClass__QueryInterface` — adjusts this-4 to primary, forwards to QI |
| 1 | `0x004105F0` | AddRef (thunk) | `SUB [ESP+4], 4; JMP AbstractClass__AddRef` |
| 2 | `0x00410600` | Release (thunk) | Calls `AbstractClass__Release` |
| 3 | `0x00410210` | Process (thunk) | Adjusts `this-4` to primary, calls `primary_vtable[0x2C]` (slot 11 Process) |
| 4 | `0x00410220` | GetID | Returns `*(this+0x0C)` → since this = object+4, reads object+0x10 = UniqueID |
| 5 | `0x00410230` | AssignID | Assigns UniqueID from global Heap counter (see Section 3) |

### INoticeSink Interface (offset +0x08 in object, vtable at 0x007E1F2C)

| Slot | Address | Name | Implementation |
|------|---------|------|---------------|
| 0 | `0x00410580` | ReceiveNotice | Returns 0 (false). Stub — override in derived classes to handle notifications |

Pre-AbstractClass base vtable for INoticeSink at `0x007E1FBC` has 1 entry (same stub).

### INoticeSource Interface (offset +0x0C in object, vtable at 0x007E1F24)

| Slot | Address | Name | Implementation |
|------|---------|------|---------------|
| 0 | `0x00410590` | SendNotice | No-op (void return). Override in derived classes to dispatch notifications |

Pre-AbstractClass base vtable for INoticeSource at `0x007E1FB4` has 2 entries.

## 7. COM Interface Details

AbstractClass implements COM interfaces but with stub reference counting:
- `AddRef()` and `Release()` both always return 1
- Objects are NOT managed by COM reference counting — the game's Heap/allocator system
  handles object lifetime
- `QueryInterface` returns the correct interface pointer for 4 supported GUIDs

### Supported COM GUIDs

| GUID | Interface | Returns |
|------|-----------|---------|
| `{00000000-0000-0000-C000-000000000046}` | IUnknown | `this` (primary) |
| `{00000109-0000-0000-C000-000000000046}` | IPersistStream | `this` (primary — merged into primary vtable) |
| `{0000010C-0000-0000-C000-000000000046}` | IPersist | `this` (primary — base of IPersistStream) |
| `{170DAC82-12E4-11D2-8175-006008055BB5}` | IRTTITypeInfo (custom) | `this+4` (secondary_4 vtable pointer) |

## 8. CRC / Deterministic Sync

The `ComputeCRC` virtual method (slot 13) feeds object state to a CRC engine for
multiplayer sync verification.

**AbstractClass base** feeds only:
- `UniqueID` (4 bytes at +0x10) via `CRCEngine__AddData`
- `Dirty` flag (1 byte at +0x20) via `CRCEngine__AddByte`

**ObjectClass override** at `0x005F6250` calls the base, then adds:
- Pointer references at +0x30 and +0x34 (swizzled through IRTTITypeInfo::GetID)
- Coordinates at +0x9C, +0xA0, +0xA4
- Various state flags: +0x6C, +0x74, +0x80, +0x81, +0x83, +0x84, +0x8C, +0x8D, +0x8F, +0x90

**CRC engine** uses a CRC-32 lookup table at `0x0081F7B4`.

Fields NOT included in CRC: field_0x18, field_0x1C, AbstractFlags. This indicates these
are either transient runtime state or intentionally excluded from sync verification.

## 9. Inheritance Hierarchy

```
AbstractClass (0x24 bytes)
├── ObjectClass        (~0xAC bytes) — anything on the map with coords
│   ├── MissionClass   (~0xD8 bytes)
│   │   └── RadioClass (~0x110 bytes)
│   │       └── TechnoClass (~0x520 bytes)
│   │           ├── FootClass   (~0x6E0 bytes)
│   │           │   ├── InfantryClass
│   │           │   ├── UnitClass
│   │           │   └── AircraftClass
│   │           └── BuildingClass
│   ├── BulletClass
│   ├── AnimClass
│   ├── TerrainClass
│   ├── OverlayClass
│   ├── SmudgeClass
│   ├── VoxelAnimClass
│   └── ParticleClass
│
├── AbstractTypeClass — type definitions (INI sections)
│   ├── ObjectTypeClass
│   │   └── TechnoTypeClass → InfantryTypeClass, UnitTypeClass, etc.
│   ├── BulletTypeClass, WarheadTypeClass, WeaponTypeClass
│   └── AnimTypeClass, ParticleSystemTypeClass, etc.
│
├── CellClass          (~0x148 bytes) — per-cell map data
├── HouseClass         (~0x160B8 bytes) — per-player state
├── FactoryClass       — production queue entries
├── SuperClass         — superweapon instances
├── TeamClass          — AI team instances
├── TagClass           — trigger tags
├── TubeClass          — tunnel network segments
├── BombClass          — ivan bomb / crazy ivan
├── CaptureManagerClass — mind control
├── TemporalClass      — chrono eraser
├── SlaveManagerClass  — slave miner management
├── SpawnManagerClass  — aircraft carrier spawn management
├── RadSiteClass       — radiation site
├── EMPulseClass       — EMP effect
├── AirstrikeClass     — boris airstrike
├── ParasiteClass      — terror drone parasite
├── LightSourceClass   — dynamic lighting
├── DiskLaserClass     — disk laser beam
├── AlphaShapeClass    — fog ghost shapes
├── FoggedObjectClass  — fog of war cached objects
├── WaypointPathClass  — mission waypoint paths
└── Tactical           — tactical map renderer
```

All 44+ classes confirmed as direct or indirect inheritors via xrefs to the constructor
at `0x00410170`.

## 10. Current Rust Implementation Status

The Rust engine uses a fundamentally different architecture: a **monomorphic `GameEntity`
struct** instead of C++ polymorphic inheritance.

**Mapping:**

| AbstractClass Feature | Rust Equivalent | File |
|----------------------|-----------------|------|
| UniqueID (offset 0x10) | `stable_id: u64` | `src/sim/game_entity.rs` |
| AbstractFlags (offset 0x14) | No direct equivalent (state encoded in optional fields) | — |
| Dirty flag (offset 0x20) | Not implemented | — |
| GetCoords (vtable 0x48) | `position: Position` struct | `src/sim/components.rs` |
| IsAlive (vtable 0x44) | `is_alive()` method (checks `health.current > 0`) | `src/sim/game_entity.rs` |
| GetOwningHouseIndex (vtable 0x38) | `owner: InternedId` | `src/sim/game_entity.rs` |
| ComputeCRC (vtable 0x34) | State hashing in `World::advance_tick` | `src/sim/world/mod.rs` |
| Class hierarchy | `EntityCategory` enum + optional components | `src/map/entities.rs` |
| EntityStore (BTreeMap) | `EntityStore` with `BTreeMap<u64, GameEntity>` | `src/sim/entity_store.rs` |
| WhatAmI / RTTI | `category: EntityCategory` | `src/sim/game_entity.rs` |
| PointerExpiredNotification | Weak refs via `Option<u64>` stable_id | Throughout sim |
| COM interfaces | Not applicable (no COM in Rust engine) | — |

**Not implemented in Rust:**
- AbstractFlags (IsActive, IsOnMap, IsNetPlayer) — no direct equivalent
- Dirty flag — not needed (deterministic sim doesn't use dirty tracking)
- COM interface hierarchy — not applicable
- INoticeSink/INoticeSource notification pattern — not implemented
- field_0x18 and field_0x1C — unknown purpose, not implemented

## 11. Open Questions

1. **field_0x18 purpose (LOW confidence):** Initialized to 0, registered for pointer
   swizzle in ObjectClass::Load (so it holds a pointer at runtime). Not included in CRC.
   Could be a linked-list "next" pointer or a back-reference. Needs more investigation
   by tracing writes to this offset across the codebase.

2. **field_0x1C purpose (LOW confidence):** Initialized to 0, NOT swizzled in Load, NOT
   included in CRC. Documented as "RefCount" in some prior reports but AddRef/Release are
   stubs that never modify it. May be unused or serve a different purpose than reference
   counting.

3. **Primary vtable slots 15, 16, 25, 26 (LOW confidence):** These slots are not
   overridden in ObjectClass and their purpose in derived classes needs investigation:
   - Slot 15 (0x3C): Returns false — possibly `IsDiscoveredByCurrentPlayer()`
   - Slot 16 (0x40): Returns false — possibly `IsInLimbo()`
   - Slot 25 (0x64): Returns 0 — overridden in ObjectClass to 0x00426390
   - Slot 26 (0x68): Pure virtual — overridden in ObjectClass to 0x004263A0

4. **Primary vtable slots 24 and 26 (MEDIUM confidence):** These point to data addresses
   (`0x007FB160`, `0x007FAFC0`) in AbstractClass's vtable, which decompile as garbage.
   They are likely `_purecall` entries or RTTI metadata. ObjectClass provides real
   implementations, confirming they are pure virtual in the base class.

5. **AbstractFlags bits 3-7:** Only bits 0-2 are documented. The AND mask `0xF8` in the
   constructor clears only bits 0-2, preserving bits 3-7. Are bits 3-7 ever used? Needs
   a comprehensive search for bit operations on this field.

## Sources

### Ghidra Functions Decompiled
- `0x00410170` — AbstractClass__Constructor_Full (the real constructor)
- `0x004101C0` — AbstractClass__Constructor_VtablesOnly
- `0x004101F0` — AbstractClass__Destructor_ResetVtables
- `0x00410210` — AbstractClass__IRTTITypeInfo_Process_thunk
- `0x00410220` — AbstractClass__IRTTITypeInfo_GetID
- `0x00410230` — AbstractClass__AssignUniqueID
- `0x00410260` — AbstractClass__QueryInterface
- `0x00410300` — AbstractClass__AddRef
- `0x00410310` — AbstractClass__Release
- `0x00410380` — AbstractClass__Load
- `0x004103E0` — AbstractClass__GetSizeMax
- `0x00410410` — AbstractClass__ComputeCRC
- `0x00410450` — AbstractClass__IsDirty
- `0x004104C0` — AbstractClass__GetCoords
- `0x004104F0` — AbstractClass__GetCenterCoords (delegates to GetCoords)
- `0x00410540` — AbstractClass__GetFLH (delegates to GetCoords)
- `0x004105A0` — AbstractClass__ScalarDeletingDestructor
- `0x004105E0` — IRTTITypeInfo QI adjustor thunk
- `0x004105F0` — IRTTITypeInfo AddRef adjustor thunk
- `0x00410600` — IRTTITypeInfo Release adjustor thunk
- `0x0068BCB0` — Heap::GetNextID (UniqueID counter increment)
- `0x005F5E80` — ObjectClass::Load
- `0x005F6250` — ObjectClass::ComputeCRC
- `0x005F6690` — ObjectClass::IsAlive
- `0x005F65A0` — ObjectClass::GetCoords
- `0x005F5230` — ObjectClass::PointerExpiredNotification
- `0x005F3E70` — ObjectClass::Update
- `0x005F6DA0` — ObjectClass vtable slot 24
- `0x005F6B60` — ObjectClass::IsFallingDown_Early
- `0x005F6B90` — ObjectClass::IsFallingDown_Late
- `0x0047BBF0` — CellClass::Constructor (real constructor)
- `0x004C98B0` — FactoryClass::Constructor (real constructor)
- `0x004C9150` — Stub__ReturnZero (shared stub returning 0)

### Vtable Memory Inspected
- `0x007E1F50` — AbstractClass primary vtable (28 entries)
- `0x007E1F34` — AbstractClass IRTTITypeInfo vtable (6-7 entries)
- `0x007E1F2C` — AbstractClass INoticeSink vtable (1-2 entries)
- `0x007E1F24` — AbstractClass INoticeSource vtable (1-2 entries)
- `0x007EF060` — ObjectClass primary vtable (for override comparison)
- `0x007E1FBC` — INoticeSink base vtable
- `0x007E1FB4` — INoticeSource base vtable

### COM GUIDs Inspected
- `0x007F7C90` — IUnknown GUID
- `0x007F7C80` — IPersistStream GUID
- `0x007F7C70` — IPersist GUID
- `0x007E9AE0` — IRTTITypeInfo custom GUID

### Global Data
- `0x00A8B230` — AbstractClass Heap pointer (used for UniqueID allocation)
- `0x00887680` — Default {0,0,0} coordinate struct (12 bytes, all zeroes)
- `0x0081F7B4` — CRC-32 lookup table

### Documentation Files Referenced
- `ra2-rust-game-docs/BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` — AbstractFlags bit definitions
- `ra2-rust-game-docs/ALPHA_SHAPE_CLASS_LIFECYCLE.md` — AbstractClass layout
- `ra2-rust-game-docs/GAMEMD_ARCHITECTURE.md` — Class hierarchy
- `ra2-rust-game-docs/HOUSECLASS_CONSTRUCTOR_DETAILED.md` — Constructor patterns
- `ra2-rust-game-docs/CELLCLASS_STRUCT_GHIDRA_REPORT.md` — CellClass inheritance
- `ra2-rust-game-docs/TECHNOCLASS_STRUCT_LAYOUT.md` — TechnoClass inheritance
