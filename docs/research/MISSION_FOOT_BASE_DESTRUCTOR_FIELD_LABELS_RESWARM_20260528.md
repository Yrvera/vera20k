# Mission / Foot Base Destructor Field Labels - Reswarm 2026-05-28

**Address(es):** `FootClass` base destructor body `0x004D3590`, shared Techno/Radio/Mission teardown body `MissionClass__Destructor @ 0x006F4500`, `ObjectClass__Destructor @ 0x005F3B80`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** field-level side effects in shared Foot and Techno/Radio/Mission base destructors needed for a future pending-delete/finalizer contract: locomotor COM release, navigation/enter queue buffer frees, radio contacts buffer free, Techno manager releases, owner pointer clear, object pointer uninit calls, sound/voc handles, cell special pointer clear, and listener/registry removals.  
**Non-Scope:** leaf Unit/Infantry/Aircraft/Building/Bullet/Anim destructor census, full semantics of every Techno manager helper, runtime debugger watchpoints, and Rust implementation.  
**Confidence:** High for verified offsets/order and cleanup side effects; Medium for semantic names of a few Techno helper pointers where the destructor proves ownership but broader manager behavior belongs to adjacent systems.  
**Active in YR:** Yes. These destructors are reached from active pending-delete scalar-deleting destructor chains for Foot-derived objects and Building/Techno objects. Conditional side effects depend on non-null fields.

## 1. Overview

The base destructor chain is not generic storage removal. `FootClass` first removes team/listener membership, destroys a Foot-owned helper pointer, clears a cell special pointer, detaches a movement sound handle, releases the active locomotor COM interface, frees EnterQueue and NavQueue buffers, then calls the shared Techno/Radio/Mission teardown. The shared teardown releases Techno manager objects, stops/detaches four voc handles, clears owner/local-control state, removes Techno/House listener entries, uninitializes two object pointers, frees three embedded dynamic vectors including `RadioClass::Contacts`, and finally calls `ObjectClass__Destructor`.

Ghidra still names `0x004D3590` as a `FootClass__Constructor` duplicate, but the body is destructor-shaped and is called from Unit/Infantry/Aircraft leaf destructors. Likewise `0x006F4500` is labeled `MissionClass__Destructor`, but the body starts with Techno vtables, then tears down Techno fields, Radio fields, Mission vtables, and calls `ObjectClass__Destructor`.

## 2. Class Layout / Key Offsets

### FootClass destructor fields

| Offset | Field label / semantics | Destructor action | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x5D4` | `TeamClass*` membership pointer | If non-null and game active, calls `FUN_006EA870(this, -1, 0)` before registry removal. | decompile `0x004D3590`; team-remove helper decompile `0x006EA870` | Conditional |
| `+0x69C` | Foot-owned helper pointer; prior docs disagree on exact name, destructor proves owned scalar object | If non-null, calls vtable `+0x20(1)` and clears it. | decompile `0x004D3590` | Conditional |
| `+0x564/+0x566` | last/special cell coordinate pair | If not null-cell sentinel, gets `CellClass`; if `Cell+0xE0 == this`, writes `Cell+0xE0 = 0`. | decompile `0x004D3590`; assembly `0x004D3656..0x004D3668` | Conditional |
| `+0x544` | `LoopingSoundHandle` / Foot movement-loop sound handle | `SoundEvent__Release`, then `VocHandle__Detach`. | decompile `0x004D3590`; assembly `0x004D366F..0x004D367E` | Conditional on active handle |
| `+0x674` | `ILocomotion*` active locomotor COM pointer | If non-null, calls vtable `+0x08(this_loco)`; no explicit null write visible before vector frees. | decompile `0x004D3590`; assembly `0x004D3683..0x004D3690` | Conditional |
| `+0x5AC..+0x5C0` | `EnterQueue` dynamic vector | Resets vtable, frees `+0x5B0` only if owned flag `+0x5B9` is set, clears flag and count-ish field. | decompile `0x004D3590`; assembly around `0x004D3693..0x004D36AA` | Conditional |
| `+0x588..+0x59C` | `NavQueue` dynamic vector | Resets vtable, frees `+0x58C` only if owned flag `+0x595` is set, clears flag and count-ish field. | decompile `0x004D3590`; assembly `0x004D36E2..0x004D3701` | Conditional |

### Shared Techno / Radio / Mission teardown fields at `0x006F4500`

| Offset | Field label / semantics | Destructor action | Evidence | Active in YR |
|---:|---|---|---|---|
| `+0x514` | Techno helper/list bundle, exact name unresolved | If non-null, calls `FUN_00636310(this+0x514)` then clears pointer. | decompile `0x006F4500`; helper decompile `0x00636310` | Conditional |
| `+0x2BC` | `CaptureManagerClass*` | If non-null, calls vtable `+0x20(1)` and clears. | `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`; decompile `0x006F4500` | Conditional |
| `+0x2D0` | `SpawnManagerClass*` | If non-null, calls vtable `+0x20(1)` and clears. | struct layout doc; decompile `0x006F4500` | Conditional |
| `+0x2D8` | `SlaveManagerClass*` | Calls cleanup helper `FUN_006B0AE0(0,0)`, then vtable `+0x20(1)`, then clears. | decompile `0x006F4500`; helper decompile `0x006B0AE0` | Conditional, stock YR for slave miner paths |
| `+0x274` | `TemporalClass*` | If non-null, calls vtable `+0x20(1)` and clears. | struct layout doc; decompile `0x006F4500` | Conditional |
| `+0x294` | `AirstrikeClass*` | Releases only if `Airstrike+0x4C == this`, then clears. | `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`; decompile/assembly `0x006F45B1..0x006F45BE` | Conditional |
| `+0x488`, `+0x4A4`, `+0x4C0`, `+0x4DC` | four Techno voc/sound handles | First three: `VocHandle__Stop` then `VocHandle__Detach`; fourth: detach only. | assembly `0x006F45C5..0x006F4607` | Conditional on handles |
| `+0x21C` | `Owner HouseClass*` | Writes null. | decompile and assembly `0x006F4610` | Yes |
| `+0x41A` | local/player-control byte | Writes `0`. | decompile and assembly `0x006F4616` | Yes |
| global `0x00A8EC78` | `g_TechnoClass_Array` dynamic vector | Searches for `this`; if found, removes/compacts via `FUN_0063F000`. | decompile and assembly `0x006F461C..0x006F4636`; constructor add in `0x006F2B40` | Yes |
| global `0x00B0F6C8` | House remove-listener vector | Searches/removes via `FUN_0045ADD0`. | decompile and assembly `0x006F465D..0x006F4662`; constructor add in `0x006F2B40` | Yes |
| `+0x12C`, `+0x130` | object pointers owned by Techno/Mission teardown | If non-null, call vtable `+0xF8()` and clear. | decompile `0x006F4500`; assembly `0x006F4667..0x006F4697` | Conditional |
| `+0x470..+0x47D` | embedded dynamic vector | Resets vector vtable and frees owned data buffer through generic dvec destructor shape. | decompile `0x006F4500`; constructor `0x006F2B40`; helper `FUN_004E0410` | Conditional |
| `+0x458..+0x465` | embedded dynamic vector | Resets vector vtable and frees `+0x45C` if owned flag `+0x465` set. | decompile `0x006F4500`; helper `FUN_004E0410` | Conditional |
| `+0x440..+0x44D` | embedded dynamic vector | Resets vector vtable and frees `+0x444` if owned flag `+0x44D` set. | decompile `0x006F4500`; helper `FUN_004E0410` | Conditional |
| `+0xE0..+0xED` | `RadioClass::Contacts` dynamic vector | Sets Radio vtables, resets contact-vector vtable, frees `+0xE4` only if owned flag `+0xED` set, clears flag/count. | `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`; decompile `0x006F4500`; assembly `0x006F4735..0x006F4775` | Yes |

## 3. Core Logic

### FootClass base destructor order (`0x004D3590`)

1. Reinstalls FootClass vtables.
2. If `Foot+0x5D4 != 0` and `g_GameActive != 0`, removes the foot object from its team via `FUN_006EA870(this, -1, 0)`.
3. Removes `this` from the Foot global/class array (`DAT_008B3DC0` vector) and from the team-remove-listener vector (`DAT_00B0F5D8`) if present.
4. Destroys the owned helper at `Foot+0x69C` through virtual `+0x20(1)` and clears `+0x69C`.
5. Checks the coordinate pair at `Foot+0x564/+0x566` against null-cell globals. If not sentinel, gets a `CellClass`; if `Cell+0xE0` points at this object, clears it.
6. Releases and detaches `LoopingSoundHandle` at `Foot+0x544`.
7. Reads the locomotor COM pointer at `Foot+0x674`; if non-null, calls vtable slot `+0x08` on that interface.
8. Frees `EnterQueue` buffer at `+0x5B0` only when owned flag `+0x5B9` is set; clears flag and related count field.
9. Frees `NavQueue` buffer at `+0x58C` only when owned flag `+0x595` is set; clears flag and related count field.
10. Calls shared teardown `0x006F4500`.

Important tiny details:

- The cell clear is conditional on both coordinate pair and pointer identity. It does not blindly clear the cell.
- The locomotor release happens after the Foot sound detach and before both queue frees.
- The queue buffer frees are ownership-flag gated. A future Rust finalizer should not assume every vector header owns heap memory.
- The function does not clear `Foot+0x674` in the visible decompile before chaining to the shared destructor.

### Shared Techno / Radio / Mission teardown order (`0x006F4500`)

1. Reinstalls TechnoClass vtables.
2. Tears down Techno-owned managers in this order: `+0x514` helper/list bundle, `CaptureManager +0x2BC`, `SpawnManager +0x2D0`, `SlaveManager +0x2D8`, `TemporalClass +0x274`, owner-validated `AirstrikeClass +0x294`.
3. Stops/detaches three Techno voc handles and detaches a fourth handle.
4. Clears owner pointer `+0x21C` and byte `+0x41A`.
5. Removes from `g_TechnoClass_Array`, then from `g_HouseClass_RemoveListeners`.
6. If `+0x12C` and `+0x130` are non-null, calls each object's vtable `+0xF8()` and clears the field.
7. Tears down embedded dynamic vectors at `+0x470`, `+0x458`, and `+0x440`, with owned-buffer checks.
8. Switches to RadioClass vtables, tears down `RadioClass::Contacts` at `+0xE0..+0xED`.
9. Switches to MissionClass vtables and calls `ObjectClass__Destructor @ 0x005F3B80`.

Important tiny details:

- `AirstrikeClass* +0x294` is only destructed when its back-pointer `+0x4C` equals `this`; stale/foreign pointers are not destroyed by this path.
- `SlaveManager +0x2D8` gets a pre-destruction helper call before scalar deletion, unlike the simpler manager pointers.
- Owner pointer clear happens before Techno class-array and House listener removal.
- Radio contacts are not compacted here; the whole contacts buffer is freed when the object itself is destroyed.

## 4. INI Keys

No INI key directly gates these destructor mechanisms. Fields become non-null through normal object/type features:

| Feature / key family | Field affected | Active status |
|---|---|---|
| `MindControl=`-style weapons | `Techno+0x2BC CaptureManager` | Conditional |
| `Spawns=` / carrier missile systems | `Techno+0x2D0 SpawnManager` | Conditional |
| `Enslaves=` | `Techno+0x2D8 SlaveManager` | Stock-live for slave miner systems |
| `Temporal=` weapons | `Techno+0x274 TemporalClass` | Conditional |
| `AirstrikeTeam=` | `Techno+0x294 AirstrikeClass` | Stock `[BORIS]` uses this on InfantryClass; stock aircraft usually do not |
| Radio docking/boarding/repair protocols | `RadioClass+0xE0..+0xED Contacts` | Active for all Techno objects, capacity varies |

## 5. Integration Points

| Caller / integration | Verified fact | Evidence |
|---|---|---|
| Unit destructor | Calls `0x004D3590` after Unit leaf cleanup. | xref `0x00735970`, `0x007359E3`; prior destructor census |
| Infantry destructor | Calls `0x004D3590` after Infantry leaf cleanup. | xref `0x00517F89`; prior destructor census |
| Aircraft destructor | Calls `0x004D3590` after Aircraft leaf cleanup. | xrefs `0x00414203`, `0x00414276`; prior destructor census |
| Building destructor | Calls shared teardown `0x006F4500` directly after Building leaf cleanup. | xref `0x0043C0BF` |
| Foot destructor to shared base | `0x004D3590` calls `0x006F4500` after queue frees. | assembly `0x004D36F3..0x004D3701` |
| Shared base to ObjectClass | `0x006F4500` calls `ObjectClass__Destructor`. | assembly `0x006F4773..0x006F4790` |

This makes the active runtime order for Foot-derived pending-delete finalization:

leaf class destructor -> FootClass base destructor -> shared Techno/Radio/Mission teardown -> ObjectClass destructor -> AbstractClass destructor.

## 6. Current Rust Implementation Status

| Rust surface | Current behavior | Delta against verified destructor base behavior |
|---|---|---|
| `src/sim/world/mod.rs:675` `despawn_entity` | Removes origin-cell occupancy, clears radio contacts referencing the stable id, removes entity, unregisters live object. | No class-aware finalizer, no ordered Foot/Techno/Radio/Mission cleanup ladder, no pending-delete drain integration. |
| `src/sim/game_entity.rs:171..187` | Entity owns optional `locomotor`, `movement_target`, `navigation`, `attack_target`, `radio_contacts`. | Storage fields exist, but deletion is generic and does not model COM-release/order-sensitive cleanup. |
| `src/sim/components.rs:298..310` | `NavigationState` models `nav_com_aux`, `nav_com`, `suspended_nav_com`, `nav_queue`. | No destructor-specific freeing/clearing order or pointer-expiry retention integrated into finalization. |
| `src/sim/components.rs:384..410` | Drive runtime has destination/head_to/path/tube state. | Native locomotor COM interface is released before queues/base teardown; Rust does not have a finalizer boundary for locomotor-owned buffers. |
| `src/app_building_anim.rs` / sound event surfaces | App drains queued sound events and visual anims. | Native destructor stops/detaches object-owned voc handles during finalization; app event queues are not equivalent to object-owned handles. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FootClass` base destructor `0x004D3590` | verified | decompile `0x004D3590`; callees; assembly contexts `0x004D3656..0x004D3701` | exact semantic name for `Foot+0x69C` beyond owned helper pointer remains doc-conflicted |
| Foot team removal | verified | `0x004D3590`; `FUN_006EA870` decompile | broader TeamClass semantics out-of-scope |
| Foot cell `Cell+0xE0` clear | verified | decompile `0x004D3590`; assembly `0x004D3656..0x004D3668` | exact system name for `Cell+0xE0` out-of-scope |
| Foot locomotor COM release | verified | decompile `0x004D3590`; assembly `0x004D3683..0x004D3690`; locomotor docs | slot `+0x08` canonical COM method name not relabeled here |
| Foot EnterQueue/NavQueue buffer frees | verified | decompile `0x004D3590`; constructor `0x004D31E0`; NavCom docs | none for destructor order |
| Shared Techno manager releases | verified | decompile `0x006F4500`; struct-layout docs | full manager behavior out-of-scope |
| Shared voc handle cleanup | verified | decompile `0x006F4500`; assembly `0x006F45C5..0x006F4607` | exact gameplay role for each handle not fully named |
| Owner clear and listener removals | verified | decompile `0x006F4500`; assembly `0x006F4610..0x006F4662` | none for destructor order |
| Radio contacts buffer teardown | verified | decompile `0x006F4500`; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md` | none for destructor order |
| Rust finalizer parity | touched-not-exhausted | source scan listed in Section 6 | future implementation contract/design |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which base destructor body owns Foot-specific cleanup? -> `0x004D3590`; Ghidra labels it a duplicate constructor, but leaf destructors call it and it chains to `0x006F4500`.` (evidence: xrefs `0x00735970`, `0x00517F89`, `0x00414203`; decompile `0x004D3590`)
- `[RESOLVED] OQ-02 - Does Foot destructor release the locomotor COM pointer? -> Yes, it reads `+0x674` and if non-null calls vtable `+0x08` on the interface.` (evidence: decompile `0x004D3590`; assembly `0x004D3683..0x004D3690`)
- `[RESOLVED] OQ-03 - Does Foot destructor clear cell special pointer state? -> Yes, it conditionally clears `CellClass+0xE0` only if that cell field equals this object.` (evidence: `0x004D3656..0x004D3668`)
- `[RESOLVED] OQ-04 - Which Foot dynamic buffers are freed? -> EnterQueue `+0x5AC` and NavQueue `+0x588`, ownership-flag gated.` (evidence: decompile `0x004D3590`; constructor `0x004D31E0`)
- `[RESOLVED] OQ-05 - Does the shared destructor clear owner before listener removals? -> Yes, `+0x21C=0` and `+0x41A=0` precede Techno and House listener removals.` (evidence: `0x006F4610..0x006F4662`)
- `[RESOLVED] OQ-06 - Are Techno manager pointers released generically? -> Mostly vtable `+0x20(1)`, but `SlaveManager +0x2D8` has a pre-cleanup helper and `Airstrike +0x294` is owner-backpointer guarded.` (evidence: decompile `0x006F4500`; `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-07 - Are radio contacts per-object storage freed in this chain? -> Yes, after switching to RadioClass vtables the contacts vector at `+0xE0..+0xED` frees owned data and clears flags/count.` (evidence: decompile `0x006F4500`; `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-08 - Does shared teardown destroy object pointers with `+0xF8`? -> Yes, non-null `+0x12C` and `+0x130` are dispatched through vtable `+0xF8()` and cleared.` (evidence: decompile `0x006F4500`)
- `[RESOLVED] OQ-09 - Is ordinary pause a destructor skip gate? -> Out of this function, but prior slot proved pending-delete drain skips only session-end flags, not ordinary pause.` (evidence: `MAIN_TICK_PENDING_DELETE_SKIP_FLAGS_RESWARM_20260528.md`)
- `[RESOLVED] OQ-10 - What Rust surfaces carry analogous state? -> `GameEntity::locomotor`, `movement_target`, `navigation`, `radio_contacts`, and generic `despawn_entity`.` (evidence: `src/sim/game_entity.rs:171`, `src/sim/components.rs:298`, `src/sim/world/mod.rs:675`)
- `[DEFERRED] OQ-11 - What is the exact canonical name for `Techno+0x514`?` (category: requires-different-system-context; reason: destructor proves helper/list-bundle cleanup but full writer/consumer lifecycle is not needed for this finalizer contract; next-step-if-pursued: trace all `+0x514` writes and `FUN_00636310` callers)
- `[DEFERRED] OQ-12 - What is the exact canonical name for `CellClass+0xE0`?` (category: requires-different-system-context; reason: destructor side effect is proven, but cell field identity belongs to a CellClass layout audit; next-step-if-pursued: trace all `Cell+0xE0` writers/readers)
- `[DEFERRED] OQ-13 - What exact gameplay role maps to each Techno voc handle at `+0x488/+0x4A4/+0x4C0/+0x4DC`?` (category: bounded-cost-too-high; reason: destructor cleanup order is proven; per-handle audio producer mapping is a separate audio lifecycle slice; next-step-if-pursued: xref each `VocHandle__Init/Stop/Detach` owner offset)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Foot finalization removes team/listener membership before helper/cell/loco/vector cleanup. | `0x004D3590`; `FUN_006EA870`; xrefs from Unit/Infantry/Aircraft destructors | missing: generic `despawn_entity` | `src/sim/world/mod.rs:675`; future pending-delete finalizer | Add a Foot-class finalizer step before storage removal that handles team/listener state in native order. | Destroy a team member unit; team intrusive membership and listener arrays are clear before entity storage disappears. | Do not collapse Foot teardown into `EntityStore::remove`. |
| Foot destructor clears `CellClass+0xE0` only when the stored cell pointer equals `this`. | `0x004D3656..0x004D3668` | unchecked/missing cell special pointer model | occupancy/cell state surfaces | Model any future cell-special pointer as identity-guarded clear, not blanket cell reset. | Destroy a foot object whose last special cell points to a different object; that other object remains in the cell field. | Do not clear cell metadata by coordinate alone. |
| Active locomotor COM pointer is released after sound detach and before queue buffer frees. | `0x004D3677..0x004D3690` | missing: Rust locomotor is plain state with no finalizer boundary | `src/sim/game_entity.rs:171`; `src/sim/movement/*` | Locomotor-owned runtime buffers and piggyback state need a destructor-equivalent release point before Foot queues/base teardown. | Destroy a chrono miner during piggyback/drive state; locomotor state is finalized once before navigation queues/base fields are removed. | Do not rely on dropping the whole entity row to express locomotor release order. |
| Foot destructor frees EnterQueue and NavQueue buffers with owned-flag checks. | decompile `0x004D3590`; `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` | Rust `Vec` drops with entity, but no native distinction between `MovementTarget`, NavQueue, and EnterQueue finalization | `src/sim/components.rs:298..310`, movement command surfaces | Preserve separate Foot navigation/enter queues and clear/finalize them at the Foot finalizer point. | Entity with NavCom plus synthetic loaded NavQueue is destroyed; finalizer clears queue after locomotor release and before shared base cleanup. | Do not treat `MovementTarget.path` and `NavQueue` as the same native field. |
| Shared teardown releases Techno managers in fixed order and with special gates for SlaveManager and Airstrike. | decompile `0x006F4500`; struct docs | missing manager finalizer model | future Techno manager components: capture, spawn, slave, temporal, airstrike | Add manager-specific cleanup hooks rather than a single generic optional-pointer drop. | Destroy slave miner / mind-control unit / spawn carrier; relevant manager state is finalized before owner/listener removal. | Do not delete `AirstrikeClass*` unless its owner back-pointer matches. |
| Owner pointer and local-control byte clear before Techno and House listener removal. | `0x006F4610..0x006F4662` | Rust owner is stable until entity removal; owner indexes are rebuilt/generic | `src/sim/entity_store.rs`; `src/sim/world/mod.rs` owner/index handling | Future finalizer must model owner-clear timing if any hook reads owner during removal. | Instrument finalizer order: manager cleanup can still see old owner, listener removal sees owner already cleared. | Do not assume owner remains readable throughout destruction. |
| Radio contacts buffer is destroyed as part of shared base teardown, after Techno vectors and before ObjectClass destructor. | `0x006F4735..0x006F4790`; radio protocol doc | partial: `despawn_entity` clears other entities' contacts for an id immediately | `src/sim/game_entity.rs:182`; `src/sim/world/mod.rs:695` | Separate pointer-expiry/contact invalidation from the object's own RadioClass contacts storage teardown. | Destroy a dock-linked harvester; other contacts receive native pointer-expiry/BREAK behavior before the dead object's own contacts vector disappears. | Do not make `clear_radio_contacts_for` the only RadioClass destruction behavior. |
| Shared teardown dispatches `+0xF8` on object pointers `+0x12C/+0x130` before dynamic-vector and Radio cleanup. | decompile `0x006F4500` | missing unknown owned-object finalizers | future attached-object/anim/helper components | Owned object pointers must be uninitialized recursively before embedded vector frees. | Destroy techno with attached helper object at one of these offsets; helper finalizes exactly once before base object destructor. | Do not free parent storage first and leave child-object cleanup to app retain lists. |

### Stale Docs / Follow-up Docs

- `OBJECT_DERIVED_DESTRUCTOR_SIDE_EFFECTS_CENSUS_RESWARM_20260528.md` can replace its deferred base-label note with: "Resolved by `MISSION_FOOT_BASE_DESTRUCTOR_FIELD_LABELS_RESWARM_20260528.md`: `FootClass` destructor releases team membership, owned helper `+0x69C`, `CellClass+0xE0`, `LoopingSoundHandle +0x544`, locomotor COM `+0x674`, EnterQueue `+0x5AC`, and NavQueue `+0x588` before chaining to shared Techno/Radio/Mission teardown. The shared teardown releases Techno managers, voc handles, owner/listener state, two `+0xF8` object pointers, three Techno dynamic vectors, Radio contacts, then ObjectClass."
- `FOOTCLASS_NON_MOVEMENT_FIELDS.md` should not rely on the older "NavQueue shift-click waypoint queue" wording as a runtime producer claim; `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` already narrowed runtime producers to save/load/unknown legacy state.

## Sources

- Ghidra decompiled read-only: `FootClass` destructor body `0x004D3590`, `MissionClass__Destructor @ 0x006F4500`, `ObjectClass__Destructor @ 0x005F3B80`, `FootClass__Constructor @ 0x004D31E0`, `TechnoClass__Constructor @ 0x006F2B40`, `FootClass__PointerExpired @ 0x004D9960`, helpers `FUN_004E0410`, `FUN_004E0ED0`, `FUN_0045ADD0`, `FUN_0063F000`, `FUN_006EA870`, `FUN_00636310`, `FUN_006B0AE0`.
- Ghidra xrefs/callers: `0x004D3590` from Unit/Infantry/Aircraft leaves; `0x006F4500` from Foot/Building; `0x005F3B80` from shared teardown.
- Ghidra assembly contexts: `0x004D3656..0x004D3701`, `0x006F45B1..0x006F4790`, `0x005F3D1D..0x005F3D65`.
- Prior docs: `OBJECT_DERIVED_DESTRUCTOR_SIDE_EFFECTS_CENSUS_RESWARM_20260528.md`, `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`, `FOOTCLASS_STRUCT_LAYOUT.md`, `FOOTCLASS_NON_MOVEMENT_FIELDS.md`, `RADIO_CLASS_PROTOCOL_GHIDRA_REPORT.md`, `AIRCRAFTCLASS_0XA5_RADIO_GATE_WRITERS_GHIDRA_REPORT.md`, `NAVCOM_POINTEREXPIRED_RETENTION_BRANCHES_GHIDRA_REPORT.md`, `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md`.
- Rust scan: `src/sim/world/mod.rs`, `src/sim/game_entity.rs`, `src/sim/components.rs`, `src/sim/movement/*`, `src/app_building_anim.rs`, `src/app_fire_effects.rs`.

Status: COMPLETE for the requested base-destructor field-label slice. Remaining deferred labels are exact names for adjacent systems, not blockers for a lifecycle/finalizer implementation contract.
