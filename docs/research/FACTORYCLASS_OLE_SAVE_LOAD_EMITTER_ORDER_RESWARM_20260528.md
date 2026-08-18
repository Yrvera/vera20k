# FactoryClass OLE Save/Load Emitter Order - Reswarm 2026-05-28

**Address(es):** `FUN_0067D300`, `FUN_0067E730`, `FUN_0067FDF0`, generic load loop area `0x0067EEA1..0x0067EEDF`, `FactoryClass::Constructor @ 0x004C98B0`, Factory class-factory thunk around `0x006C523C..0x006C527F`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact owner/order that emits `FactoryClass` OLE records into the standard save stream, and the load-side reconstruction order that repopulates `g_FactoryClass_Array`.
**Non-Scope:** production math, sidebar cameo state, factory AI body, House production category semantics, and runtime factory append/remove already covered by sibling reports.
**Confidence:** High
**Active in YR:** Conditional - active for standard savegame save/load; not reached during ordinary no-load runtime play.

## 0. Working Notes

Target question: Resolve whether `FactoryClass` OLE object records are emitted by global factory array order, HouseClass stream/order plus per-house factory fields, or a separate object-table order.

Non-goals: Do not rediscover runtime constructor append/destructor compacting/per-tick order except where needed for save/load reconstruction; do not patch Rust or stale docs.

Evidence needed to mark COMPLETE: decompile plus assembly proving the save emitter owner, its vector parameter, inclusive/exclusive loop bounds, load counterpart order, and constructor append during load.

Stop conditions: every material open question resolved or explicitly deferred, zero-add pass over save/load owner and helper, report written only to the requested research path.

## 1. Overview

The `FactoryClass` OLE record emitter is not the earlier checksum/debug loop at `0x0067CACF`. It is the generic save helper `FUN_0067FDF0`, called by the standard contents save owner `FUN_0067D300` with `EDX=0x00A83E30`, the dynamic-vector object whose data/count fields are `g_FactoryClass_Array` and `g_FactoryClass_Array_Count`.

`FUN_0067FDF0` writes the count at vector base `+0x10`, then walks vector data at base `+0x4` from index `0` while `index < count`, querying `IPersistStream`, calling `OleSaveToStream`, and releasing each object. For FactoryClass, that means standard saves emit FactoryClass OLE records in current `g_FactoryClass_Array` ascending index order. On load, the corresponding positional loop in `FUN_0067E730` calls `OleLoadFromStream` the saved count times; each FactoryClass record constructs through the FactoryClass class-factory thunk, which calls the normal constructor and therefore appends to `g_FactoryClass_Array` in saved stream order.

## 2. Key Offsets / Globals

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `0x00A83E30` | dynamic-vector object base for FactoryClass global vector | save owner passes it in `EDX` at `0x0067DF08`; constructor grow call uses it at `0x004C9968` | Yes / Conditional save |
| `0x00A83E34` | `g_FactoryClass_Array` data pointer (`base+0x4`) | `FUN_0067FDF0` reads `*(param_2+4)`; constructor writes array slot at `0x004C9983..0x004C9989` | Yes |
| `0x00A83E40` | `g_FactoryClass_Array_Count` (`base+0x10`) | `FUN_0067FDF0` writes this count; constructor increments at `0x004C9974..0x004C997D` | Yes |
| `FactoryClass+0x44/+0x50` | inner queued-object vector data/count | `FactoryClass::Save @ 0x004CA3C0`; prior report verified ascending slot save | Conditional save/load |
| `FactoryClass+0x6C/+0x58` | owner pointer/current object pointer swizzle slots | `FactoryClass::Load @ 0x004CA270`; prior report | Conditional load |

## 3. Core Logic

### 3.1 Save owner and helper call

`FUN_0067D300` is the standard contents save owner. After the large inline class-array OLE sections, it enters a helper sequence. The first helper call in that sequence is:

```text
0067df08: MOV EDX,0xa83e30
0067df0d: MOV ECX,ESI          ; IStream*
0067df0f: CALL 0x0067fdf0
0067df14: TEST EAX,EAX
0067df16: JGE 0x0067df22
```

Material finding: this is the FactoryClass OLE emitter owner. Active in YR: Conditional - reached in standard savegame content save via `FUN_0067CEF0 -> FUN_0067D300` xref `0x0067D1AF`.

### 3.2 Generic OLE emitter semantics

`FUN_0067FDF0(IStream*, vector_base)` is a generic dynamic-vector OLE save helper:

```text
0067fdfa: PUSH 0x0
0067fdfc: MOV EAX,dword ptr [EDI + 0x10]  ; count
0067fe09: MOV dword ptr [ESP + 0x20],EAX
0067fe0d: CALL dword ptr [ECX + 0x10]     ; IStream::Write(count, 4)
0067fe1e: MOV EAX,dword ptr [EDI + 0x4]   ; data pointer
0067fe2d: MOV EAX,dword ptr [EAX + ESI*0x4]
0067fe31: PUSH 0x7f7c80                   ; IPersistStream IID
0067fe39: CALL dword ptr [ECX]            ; QueryInterface
0067fe45: CALL dword ptr [0x007e15f4]     ; OleSaveToStream
0067fe56: CALL dword ptr [ECX + 0x8]      ; Release
0067fe61: INC ESI
0067fe64: JL 0x0067fe1e                   ; index < count
```

The loop is exclusive upper-bound (`index < saved_count`), starts at `0`, emits no records for count `0`, and does not sort or filter entries. If any `Write`, `QueryInterface`, `OleSaveToStream`, or `Release` returns negative HRESULT, the helper returns failure immediately. Active in YR: Conditional - generic helper is active in the standard save path; with `EDX=0x00A83E30` it is specifically FactoryClass.

### 3.3 Load-side reconstruction

`FUN_0067E730` mirrors the stream order. After the corresponding prior explicit array loads, the FactoryClass helper slot is the generic count/OleLoad loop at:

```text
0067eea1: MOV EDX,dword ptr [ESI]
0067eea3: PUSH EBX
0067eea8: PUSH 0x4
0067eeaa: PUSH EAX
0067eeab: PUSH ESI
0067eeac: CALL dword ptr [EDX + 0xc]      ; IStream::Read(count, 4)
0067eebd: CMP EAX,EBX
0067eebf: JLE 0x0067eedf
0067eec1: LEA ECX,[ESP + 0x14]
0067eec6: PUSH 0x7f7c90
0067eecb: PUSH ESI
0067eecc: CALL EDI                       ; OleLoadFromStream
0067eeda: INC EBP
0067eedd: JL 0x0067eec1                  ; index < count
```

This load-side loop also uses an exclusive upper bound and no sorting. The class identity comes from each OLE record in the stream; for FactoryClass records, the COM class-factory thunk allocates `0x74`, calls `FactoryClass::Constructor @ 0x004C98B0`, then queries/returns the requested interface:

```text
006c523c: PUSH 0x74
006c523e: CALL 0x007c8e17
006c524a: MOV ECX,EAX
006c524c: CALL 0x004c98b0
006c5261: MOV ECX,dword ptr [ESP + 0x14]
006c5265: MOV EAX,dword ptr [ESI]
006c5267: PUSH EDI
006c5268: PUSH ECX
006c5269: PUSH ESI
006c526a: CALL dword ptr [EAX]
```

`FactoryClass::Constructor` appends the new object to `g_FactoryClass_Array` at the old count:

```text
004c9974: MOV ECX,dword ptr [0x00a83e40]
004c997a: MOV EAX,ECX
004c997c: INC ECX
004c997d: MOV dword ptr [0x00a83e40],ECX
004c9983: MOV ECX,dword ptr [0x00a83e34]
004c9989: MOV dword ptr [ECX + EAX*0x4],ESI
```

Material finding: load reconstruction order for factories equals the saved FactoryClass OLE record order, and that saved record order equals `g_FactoryClass_Array[0..count)` at save time. Active in YR: Conditional - reached in standard load via `FUN_0067E440 -> FUN_0067E730` xref `0x0067E659`.

### 3.4 Relationship to HouseClass factory fields

HouseClass load swizzle registers factory pointer fields, but those fields are not the OLE object emitter. They patch references to FactoryClass objects that were emitted/loaded in the object stream. The FactoryClass OLE records come from the global vector helper, not from iterating HouseClass factory fields. Active in YR: Conditional save/load; evidence is the direct `FUN_0067D300 -> FUN_0067FDF0(EDX=0x00A83E30)` emitter plus House swizzle evidence from prior reports.

### 3.5 Relationship to the rejected checksum/debug loop

The earlier loop at `0x0067CACF..0x0067CAF5` walks `g_FactoryClass_Array`, but calls FactoryClass vtable `+0x34` (`0x004CA430`) rather than IPersist save. The OLE save-content emitter is the separate `FUN_0067FDF0` call at `0x0067DF0F`. Active in YR: Yes for the debug/checksum path, but not as the save-content stream.

## 4. INI Keys

No INI key controls FactoryClass OLE stream order.

| INI/data source | Relevance | Evidence | Active in YR |
|---|---|---|---|
| `Factory=...` on building types | causes factories to exist through production paths, but does not sort save records | prior production docs and Rust/static INI scan | Yes |
| `[General] MaximumQueuedObjects` | affects inner factory queue capacity, not top-level OLE order | prior FactoryClass layout/save reports | Yes |
| Savegame OLE stream | authoritative source for restored FactoryClass object creation order | `FUN_0067D300`, `FUN_0067E730`, `FUN_0067FDF0` | Conditional |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_0067CEF0 -> FUN_0067D300` | standard save wrapper reaches contents save | xref `0x0067D1AF` | Conditional save |
| `FUN_0067D300 -> FUN_0067FDF0` | emits FactoryClass records using vector base `0x00A83E30` | `0x0067DF08..0x0067DF0F` | Conditional save |
| `FUN_0067FDF0` | writes count then saves objects at `vector_base+4` ascending index | `0x0067FDFC..0x0067FE64` | Conditional save |
| `FUN_0067E440 -> FUN_0067E730` | standard load wrapper reaches contents load | xref `0x0067E659` | Conditional load |
| `FUN_0067E730` factory-position load loop | reads count then `OleLoadFromStream` records in stream order | `0x0067EEA1..0x0067EEDF` | Conditional load |
| Factory class-factory thunk | allocates and constructs loaded FactoryClass objects | xref `0x006C524C`, context `0x006C523C..0x006C527F` | Conditional load |
| `FactoryClass::Constructor` | appends loaded object to global factory array | `0x004C9974..0x004C9989` | Yes |

## 6. Current Rust Implementation Status

Static scan only; no Rust files were modified.

| Rust surface | Current shape | Delta for this target |
|---|---|---|
| `src/sim/production/production_types.rs:197` | `ProductionState` stores queues as nested `BTreeMap<owner, BTreeMap<category, VecDeque<...>>>`. | Missing explicit native FactoryClass object list and saved list order. |
| `src/sim/production/production_queue.rs:442..638` | production tick collects `(owner, category)` pairs from maps before mutation. | Iteration order is owner/category key order, not saved/restored `g_FactoryClass_Array` order. |
| `src/sim/world/mod.rs:269` and `src/sim/snapshot.rs` | whole `Simulation` state serializes structurally with `ProductionState`. | Snapshot roundtrip preserves Rust map state, not a native FactoryClass OLE/object order. |
| `src/sim/world/world_hash.rs:142..164` | hashes production queues and active producers by nested map iteration. | No explicit hash surface for native factory-object order. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| save owner for FactoryClass OLE records | verified | `0x0067DF08..0x0067DF0F` | none |
| emitter helper vector layout and loop bounds | verified | `FUN_0067FDF0`, `0x0067FDFC..0x0067FE64` | none |
| save order equals `g_FactoryClass_Array` ascending index | verified | `EDX=0x00A83E30`; helper reads `+0x10` count and `+0x4` data | none |
| load counterpart order | verified | `0x0067EEA1..0x0067EEDF`; stream-position mirror of save helper sequence | none |
| loaded FactoryClass appends through normal constructor | verified | xref `0x006C524C`; constructor append `0x004C9974..0x004C9989` | none |
| House factory fields as emitter source | verified negative | prior House swizzle report plus direct global-vector emitter | none |
| checksum/debug loop as emitter source | verified negative | `0x0067CACF..0x0067CAF5` calls vtable `+0x34`, prior report | none |
| Rust delta | verified static | `production_types.rs`, `production_queue.rs`, `world/mod.rs`, `snapshot.rs`, `world_hash.rs` | implementation/test pass |

## 8. Open Questions - Final State

- `[RESOLVED] FSO-001 - What emits FactoryClass OLE object records? -> `FUN_0067D300` calls `FUN_0067FDF0` with `EDX=0x00A83E30`.` (evidence: `0x0067DF08..0x0067DF0F`)
- `[RESOLVED] FSO-002 - What order does the save helper use? -> ascending vector index from `0` while `index < *(base+0x10)`, reading objects from `*(base+4)`.` (evidence: `0x0067FDFC..0x0067FE64`)
- `[RESOLVED] FSO-003 - Does the helper sort, filter, or use House fields? -> No; it only reads the vector count/data and calls QI/OleSave/Release per slot.` (evidence: `FUN_0067FDF0`)
- `[RESOLVED] FSO-004 - Is `0x00A83E30` the FactoryClass vector object? -> Yes; constructor grow uses base `0x00A83E30`, writes data at `0x00A83E34`, count at `0x00A83E40`; save passes the same base.` (evidence: `0x004C995E..0x004C9989`, `0x0067DF08`)
- `[RESOLVED] FSO-005 - What load order reconstructs factories? -> the matching stream-position load loop reads the count and calls `OleLoadFromStream` in ascending saved record order.` (evidence: `0x0067EEA1..0x0067EEDF`)
- `[RESOLVED] FSO-006 - Does load append to `g_FactoryClass_Array`? -> Yes; FactoryClass class-factory thunk calls normal constructor, which appends at old count.` (evidence: `0x006C523C..0x006C527F`, `0x004C9974..0x004C9989`)
- `[RESOLVED] FSO-007 - Does load reconstruction order equal the saved global array order? -> Yes: save emits global array order; load constructs records in stream order; constructor appends in that order.` (evidence: `0x0067DF08`, `0x0067FDF0`, `0x0067EEA1..0x0067EEDF`, `0x004C9974..0x004C9989`)
- `[RESOLVED] FSO-008 - Is HouseClass stream/field order the FactoryClass object emitter? -> No; House fields are pointer swizzle slots, not the object-record enumerator.` (evidence: direct global-vector emitter; prior House load swizzle block `0x00503294..0x00503320`)
- `[RESOLVED] FSO-009 - Is the previous `0x0067CACF` global-array loop the OLE save emitter? -> No; it calls vtable `+0x34` checksum/debug, while OLE save uses `FUN_0067FDF0`.` (evidence: prior report plus `0x0067DF08..0x0067DF0F`)
- `[RESOLVED] FSO-010 - Are zero-count factories possible in stream? -> If count is zero, helper writes zero and emits no OLE records; load reads zero and constructs none.` (evidence: `0x0067FE18..0x0067FE1C`, `0x0067EEBD..0x0067EEBF`)
- `[RESOLVED] FSO-011 - Are negative HRESULT failures ignored? -> No; save/load return failure on negative Write/QI/OleSave/Release/OleLoad results.` (evidence: `0x0067FE10..0x0067FE68`, `0x0067EEAF..0x0067EEDF`)
- `[RESOLVED] FSO-012 - Does Rust currently preserve this native order? -> No explicit native FactoryClass list was found; current state is nested owner/category maps.` (evidence: `src/sim/production/production_types.rs:197`, `src/sim/production/production_queue.rs:442`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard save emits FactoryClass OLE records by current `g_FactoryClass_Array[0..count)` ascending index. | `0x0067DF08..0x0067DF0F`; `FUN_0067FDF0 @ 0x0067FDFC..0x0067FE64`; Active in YR: Conditional save | Missing explicit native factory-object list. | `src/sim/production/production_types.rs`, `src/sim/snapshot.rs` | Persist a native-order factory list separate from owner/category queues. | Save two factories whose creation order differs from owner/category sort; saved list order remains creation/global-array order. | `factory_save_stream_emits_global_factory_array_order` | High - wrong order changes post-load same-frame completion side effects. |
| Standard load reconstructs FactoryClass global array in saved OLE record order because each loaded FactoryClass is constructed and appended. | `0x0067EEA1..0x0067EEDF`; `0x006C524C`; `0x004C9974..0x004C9989`; Active in YR: Conditional load | Snapshot restore rebuilds Rust maps, not native FactoryClass append order. | snapshot restore and production tick surfaces | Restore factory-object list in saved order before ticking production. | Load a snapshot where two factories complete on the first tick after load; completion/delivery order follows saved factory-list order. | `factory_load_reconstructs_global_array_from_saved_record_order` | High - owner/category map restore is deterministic but non-native. |
| House factory pointer fields are reference fixups, not the object emitter. | direct emitter `0x0067DF08`; House swizzle block from prior report `0x00503294..0x00503320`; Active in YR: Conditional load | Rust active producer maps combine references and order. | `active_producer_by_owner`, future native factory references | Keep pointer relationships and global factory order as separate pieces of state. | After load, House/category active producer references resolve correctly while global tick order remains saved factory-list order. | `factory_load_house_pointers_do_not_define_global_order` | Medium - tempting shortcut would pass simple queue tests and fail cross-house ordering. |

## 10. Negative Facts / Do Not Do

- Do not say the exact OLE emitter remains unresolved. Active in YR: Conditional save; evidence now resolves it as `FUN_0067D300 -> FUN_0067FDF0(EDX=0x00A83E30)` at `0x0067DF08..0x0067DF0F`.
- Do not use HouseClass factory field order as the FactoryClass object stream order. Active in YR: Conditional load; evidence shows a direct global-vector emitter, while House fields are swizzle references.
- Do not use Rust nested owner/category `BTreeMap` order as native save/load order. Active in YR: native save uses `g_FactoryClass_Array` count/data (`FUN_0067FDF0`), not owner/category keys.
- Do not cite the `0x0067CACF` global-array loop as the OLE save emitter. Active in YR: Yes for its own path, but prior evidence shows it calls vtable `+0x34` checksum/debug, not IPersist save.
- Do not collapse per-factory inner queue order with all-factory object order. Active in YR: Conditional save/load; `FactoryClass::Save` preserves inner slots, while `FUN_0067FDF0` owns the outer factory-object order.

## 11. Remaining Uncertainty

None for the scoped target. This report does not decode every neighboring helper after `0x0067DF22`; those are out-of-scope because the FactoryClass helper is pinned by the `0x00A83E30` vector base and constructor/global-vector xrefs.

## 12. Stale Docs / Replacement Wording

- `docs/research/FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`: replace "The exact function that emits `FactoryClass` OLE object records into the save stream remains unresolved" with "The exact emitter is `FUN_0067D300 -> FUN_0067FDF0` with `EDX=0x00A83E30`; it writes `g_FactoryClass_Array_Count` and emits `g_FactoryClass_Array` entries in ascending index order via `QueryInterface(IPersistStream)` and `OleSaveToStream`."
- `docs/research/FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`: replace "Save/load top-level factory stream order remains a bounded follow-up" with "Save/load top-level factory stream order is global FactoryClass array order: standard save calls `FUN_0067FDF0` with vector base `0x00A83E30`, and standard load constructs FactoryClass records in stream order, appending through the normal constructor."
- `docs/research/FACTORY_HOUSE_BULLET_ANIM_SAME_TICK_SYSTEM_MODEL_SYNTHESIS.md`: replace "Save/load top-level factory stream order remains a bounded follow-up" with "Save/load preserves the outer FactoryClass order by OLE stream: records are emitted from `g_FactoryClass_Array` ascending index and reload append reconstructs that order."
- `docs/contracts/2026-05-28-factory-house-tail-order-implementation-contract.md`: replace any blocker wording that treats FactoryClass save stream order as unknown with "Resolved: FactoryClass outer OLE order is `g_FactoryClass_Array` ascending index via `FUN_0067D300 -> FUN_0067FDF0(EDX=0x00A83E30)`; Rust must persist/restore a native factory-list order rather than rebuilding from owner/category maps."

## Sources

- Ghidra read-only decompile/assembly: `FUN_0067D300`, `FUN_0067E730`, `FUN_0067FDF0`, `FUN_0067F9C0`, `FUN_0067C690`, `FactoryClass::Constructor @ 0x004C98B0`, `FactoryClass::Save @ 0x004CA3C0`, Factory class-factory xref/context around `0x006C523C..0x006C527F`.
- Ghidra xrefs: `FUN_0067CEF0 -> FUN_0067D300 @ 0x0067D1AF`, `FUN_0067E440 -> FUN_0067E730 @ 0x0067E659`, `0x006C524C -> FactoryClass::Constructor`.
- Prior docs: `FACTORYCLASS_SAVE_STREAM_GLOBAL_ORDER_RESWARM_20260528.md`, `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`, `OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`, `HOUSECLASS_MPLAYER_DEFEATED_SCATTER_PRODUCTION_TAIL_RESWARM_20260528.md`.
- Rust static scan: `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs`, `src/sim/snapshot.rs`, `src/sim/world/mod.rs`, `src/sim/world/world_hash.rs`.

**Status:** COMPLETE
