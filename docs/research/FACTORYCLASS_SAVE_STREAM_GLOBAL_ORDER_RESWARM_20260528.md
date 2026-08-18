# FactoryClass Save Stream Global Order - Reswarm 2026-05-28

**Address(es):** `FUN_0067D300`, `FUN_0067E730`, `FUN_0067F7E0`, `FUN_0067F9C0`, `0x0067CACF..0x0067CAF5`, `FactoryClass::Load @ 0x004CA270`, `FactoryClass::Save @ 0x004CA3C0`, `FactoryClass::Constructor @ 0x004C98B0`, Factory COM class-factory thunk `0x006C523C..0x006C526A`, House factory-pointer load fixup area `0x00503294..0x00503320`
**Investigation Mode:** exhaustive-slice downgraded to partial for the exact top-level FactoryClass OLE record emitter
**Claimed Scope:** resolve what static binary evidence proves, and does not prove, about `FactoryClass` save/load stream order as the source of post-load `g_FactoryClass_Array` order.
**Non-Scope:** production step math, sidebar cameo order, AI build choice, unit exit placement, and ordinary runtime factory creation/removal already covered by `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`.
**Confidence:** High for negative facts around the direct `g_FactoryClass_Array` loop and FactoryClass constructor append during persistence; Medium for the final save/load handoff because the exact OLE record emitter for FactoryClass remains unresolved in static Ghidra.
**Active in YR:** Conditional - active for standard savegame save/load; unrelated to normal no-load runtime play.

## 1. Overview

The direct `g_FactoryClass_Array` loop found near `0x0067CACF..0x0067CAF5` is not the top-level OLE save-content enumerator. It calls virtual slot `+0x34`, which resolves in the `FactoryClass` vtable to `0x004CA430`, the debug/CRC-style dump surface, not `FactoryClass::Save @ 0x004CA3C0`.

For savegame restore, `FactoryClass` objects still append to `g_FactoryClass_Array` at constructor time through the COM class-factory thunk. Therefore restored same-frame factory completion order follows FactoryClass persistence object creation order. Static evidence in this pass did not prove that the FactoryClass OLE record emitter walks `g_FactoryClass_Array`; Rust must not rebuild post-load factory order from owner/category maps or assume the array order is automatically preserved unless it explicitly persists/restores a native factory-list order.

## 2. Class Layout / Key Offsets

| Offset / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `0x00A83E34` | `g_FactoryClass_Array` data pointer | constructor append and runtime tick report; checksum loop at `0x0067CADA` | Yes |
| `0x00A83E40` | `g_FactoryClass_Array_Count` | constructor append; checksum loop reads/reloads count | Yes |
| `FactoryClass+0x44` | inner queued-object pointer vector data | `FactoryClass::Save`/`Load` assembly from prior report | Conditional save/load |
| `FactoryClass+0x50` | inner queue count | `0x004CA3DB`, `0x004CA2F2` | Conditional save/load |
| `FactoryClass+0x58` | current production object pointer | load fixup registered by `FactoryClass::Load`; debug dump reads it | Conditional save/load |
| `FactoryClass+0x6C` | owner `HouseClass*` | load fixup registered by `FactoryClass::Load`; debug dump fetches house ID | Conditional save/load |
| `HouseClass+0x53AC` | infantry FactoryClass pointer | House load fixup registers slot at `0x005032B6..0x005032C2` | Conditional save/load |
| `HouseClass+0x53B0` | aircraft FactoryClass pointer | House load fixup registers slot at `0x00503294..0x005032A0` | Conditional save/load |
| `HouseClass+0x53B4` | building FactoryClass pointer | House load fixup registers slot at `0x005032A5..0x005032B1` | Conditional save/load |
| `HouseClass+0x53B8` | naval/building-alt FactoryClass pointer | House load fixup registers slot at `0x005032C7..0x005032D3` | Conditional save/load |
| `HouseClass+0x53BC` | vehicle FactoryClass pointer | House load fixup registers slot at `0x005032D8..0x005032E4` | Conditional save/load |
| `HouseClass+0x53CC` | naval vehicle FactoryClass pointer | House load fixup area continues through `0x005033xx`; exact line was touched, not fully decoded | Conditional save/load |

## 3. Core Logic

### 3.1 The direct `g_FactoryClass_Array` loop is not IPersist Save

The only direct static reference to `0x00A83E34/0x00A83E40` in the save/load-owner neighborhood is the loop at `0x0067CACF..0x0067CAF5`:

```text
0067cacf: MOV EAX,[0x00a83e40]
0067cad4: XOR ESI,ESI
0067cad6: TEST EAX,EAX
0067cad8: JLE 0x0067caf7
0067cada: MOV ECX,dword ptr [0x00a83e34]
0067cae0: LEA EAX,[ESP + 0x8]
0067cae4: PUSH EAX
0067cae5: MOV ECX,dword ptr [ECX + ESI*0x4]
0067cae8: MOV EDX,dword ptr [ECX]
0067caea: CALL dword ptr [EDX + 0x34]
0067caed: MOV EAX,[0x00a83e40]
0067caf2: INC ESI
0067caf3: CMP ESI,EAX
0067caf5: JL 0x0067cada
```

For `FactoryClass`, vtable `0x007E88D0 + 0x34` contains `0x004CA430`, not `0x004CA3C0`. `0x004CA430` decompiles as a debug/CRC-style routine: it prints/logs `Frame`, `QueuedObjects.Count`, object RTTI/HeapID, `IsSuspended`, `Balance`, `SpecialItem`, owner fetch ID, then calls `AbstractClass__ComputeCRC` and `FUN_004A1C*` value accumulators. It does not call `AbstractClass::Save`, does not write through an `IStream`, and does not emit an OLE object record.

Material finding: this loop is useful negative evidence. It proves there is a native forward walk of `g_FactoryClass_Array`, but it is not the save-content stream whose load side constructs `FactoryClass` objects. Active in YR: Yes for checksum/debug/save-validation style paths, but not as the IPersist stream body.

### 3.2 FactoryClass IPersist Save/Load still preserves inner queue order

The prior report remains valid for the per-factory object body:

- `FactoryClass::Save @ 0x004CA3C0` calls `AbstractClass::Save @ 0x00410320`, writes `Factory+0x50` queue count, then writes queue pointer slots from `Factory+0x44` in ascending index order.
- `FactoryClass::Load @ 0x004CA270` calls `AbstractClass::Load @ 0x00410380`, reads the saved queue count, appends placeholders in ascending index order, and registers each queue slot plus `Factory+0x6C` and `Factory+0x58` with `FUN_006CF240`.

Assembly rechecked:

```text
004ca3c0: MOV EAX,dword ptr [ESP + 0xc]
004ca3d2: CALL 0x00410320
004ca3db: MOV ECX,dword ptr [EDI + 0x50]
...
004ca3ff: MOV EDX,dword ptr [EDI + 0x44]
004ca408: LEA EAX,[EDX + EBX*0x4]
004ca40d: CALL dword ptr [ECX + 0x10]
```

```text
004ca270: PUSH ECX
004ca27f: CALL 0x00410380
...
004ca37a: MOV EDX,dword ptr [EDI + 0x44]
004ca37d: LEA EAX,[EDX + ESI*0x4]
004ca386: CALL 0x006cf240
```

Material finding: inner queue order is not the unresolved part. The unresolved part is the order in which all FactoryClass OLE records are emitted/loaded.

### 3.3 Restore append occurs at COM FactoryClass construction time

The FactoryClass COM class-factory thunk allocates a `0x74` object and calls the normal `FactoryClass::Constructor`:

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

The constructor appends the new object to `g_FactoryClass_Array` before continuing field setup:

```text
004c9974: MOV ECX,dword ptr [0x00a83e40]
004c997a: MOV EAX,ECX
004c997c: INC ECX
004c997d: MOV dword ptr [0x00a83e40],ECX
004c9983: MOV ECX,dword ptr [0x00a83e34]
004c9989: MOV dword ptr [ECX + EAX*0x4],ESI
```

Material finding: on load, every FactoryClass object appends to the runtime global factory array in the order the persistence layer creates FactoryClass objects. That order is the load-side object-record order, not a later House/category sort.

### 3.4 HouseClass load swizzles factory pointer fields, but this pass did not prove it creates factories

HouseClass load fixup code registers factory pointer slots with the generic swizzle table:

```text
00503294: LEA EAX,[ESI + 0x53b0]
0050329a: PUSH EAX
0050329b: PUSH 0xb0c110
005032a0: CALL 0x006cf240
005032a5: LEA ECX,[ESI + 0x53b4]
005032ab: PUSH ECX
005032ac: PUSH 0xb0c110
005032b1: CALL 0x006cf240
005032b6: LEA EDX,[ESI + 0x53ac]
005032bc: PUSH EDX
005032bd: PUSH 0xb0c110
005032c2: CALL 0x006cf240
005032c7: LEA EAX,[ESI + 0x53b8]
005032cd: PUSH EAX
005032ce: PUSH 0xb0c110
005032d3: CALL 0x006cf240
005032d8: LEA ECX,[ESI + 0x53bc]
005032de: PUSH ECX
005032df: PUSH 0xb0c110
005032e4: CALL 0x006cf240
```

This proves House factory fields are pointer slots in the save/load swizzle system. It does not, by itself, prove a House/category creation order for FactoryClass objects. The objects must exist in the old-to-new swizzle map by the time `FUN_006CF350` patches slots, and the constructor evidence shows those objects append when created.

Material finding: "House has factory pointers" is not enough to claim post-load order is House order. A direct FactoryClass OLE record emitter or a verified nested `OleLoadFromStream` in the House load body is still needed to prove that stronger claim.

### 3.5 Top-level CONTENTS save/load owner

`FUN_0067D300` is the standard content save owner. It saves many explicit global class arrays via `OleSaveToStream` and calls `FUN_00551B20` for the active vector. `FUN_0067E730` mirrors with `OleLoadFromStream` loops and calls `FUN_00551B90` for the active vector.

Static search for immediate references to `0x00A83E34` and `0x00A83E40` found the constructor/destructor/tick/checksum families but did not find a direct `FUN_0067D300` OLE-save loop over `g_FactoryClass_Array`. The nearby direct loop at `0x0067CACF` was resolved above as `+0x34` checksum/debug, not IPersist Save.

Material finding: this pass did not prove that standard save/load preserves `g_FactoryClass_Array` order as an OLE object stream. The safe mechanism statement is narrower: restored global array order follows FactoryClass persistence object creation order; exact emitter order remains the remaining uncertainty.

## 4. INI Keys

No INI key directly controls FactoryClass save stream order or `g_FactoryClass_Array` serialization.

| INI key / data source | Effect for this target | Evidence | Active in YR |
|---|---|---|---|
| `[General] MaximumQueuedObjects` | affects per-factory queue capacity, not top-level factory stream order | prior factory reports, `rules.ini:335` | Yes |
| `Factory=...` on buildings | can cause runtime factory existence by enabling production facilities, but does not sort save/load FactoryClass objects | `rulesmd.ini` / `rules.ini`; production reports | Yes |
| Savegame OLE stream | data source for restored FactoryClass object creation order | `FUN_0067D300`, `FUN_0067E730`, class factory thunk | Conditional save/load |

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FactoryClass::Constructor @ 0x004C98B0` | appends to global factory array at old count | `0x004C9974..0x004C9989` | Yes |
| Factory COM class-factory thunk | creates loaded FactoryClass objects through normal constructor | `0x006C523C..0x006C526A` | Conditional save/load |
| `FactoryClass::Save @ 0x004CA3C0` | saves one factory object body and inner queue order | `0x004CA3C0..0x004CA41F` | Conditional save/load |
| `FactoryClass::Load @ 0x004CA270` | loads one factory object body and swizzle-registers pointer fields | `0x004CA270..0x004CA3B5` | Conditional save/load |
| `0x0067CACF..0x0067CAF5` | walks `g_FactoryClass_Array`, but calls virtual `+0x34` checksum/debug | assembly plus vtable entry `0x007E8904 -> 0x004CA430` | Yes, not IPersist |
| `LogicClass::PerTickUpdate` | ticks factories in current `g_FactoryClass_Array` order after load | prior report `0x0055B66A..0x0055B68B` | Yes |

## 6. Current Rust Implementation Status

| Rust surface | Current shape | Delta for this target |
|---|---|---|
| `src/sim/production/production_types.rs:197` | `ProductionState` stores `queues_by_owner: BTreeMap<owner, BTreeMap<ProductionCategory, VecDeque<BuildQueueItem>>>`. | No native global FactoryClass object list or persistence creation order. |
| `src/sim/production/production_queue.rs:442..638` | `tick_production_with_overlay_registry` collects `(owner, category)` from nested maps, advances, then may spawn/deliver. | Same-frame completion order is owner/category key order, not native `g_FactoryClass_Array` order. |
| `src/sim/production/production_types.rs:22` | `BuildQueueItem` is `Serialize`/`Deserialize`; queue maps serialize structurally. | Rust snapshot order follows map/category structure unless a native-order list is added. |
| `src/sim/world/mod.rs:269` | `Simulation` serializes `production: ProductionState`. | Snapshot restore does not model FactoryClass COM object creation append order. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| direct `g_FactoryClass_Array` loop near save/load owner | verified | `0x0067CACF..0x0067CAF5` | none |
| virtual target of that direct loop | verified | `0x007E88D0 + 0x34 = 0x004CA430`; decompile `0x004CA430` | none |
| `0x004CA430` role | verified | decompile shows debug strings and CRC/value accumulator calls | none |
| FactoryClass IPersist Save body | verified by prior report and spot-check | `0x004CA3C0` assembly context | none for inner queue order |
| FactoryClass IPersist Load body | verified by prior report and spot-check | `0x004CA270` assembly context | none for inner queue order |
| COM class-factory load construction | verified | `0x006C523C..0x006C526A` | none |
| constructor append during persistence creation | verified | `0x004C9974..0x004C9989` | none |
| House factory pointer load fixup slots | touched-not-exhausted | `0x00503294..0x00503320` | exact full field order through `+0x53CC` and whether any nested object-load call exists nearby |
| `FUN_0067D300` direct content stream walk of `g_FactoryClass_Array` | verified negative within static immediate-reference search | no `0x00A83E34/0x00A83E40` references in `FUN_0067D300`; only nearby direct loop is `+0x34` checksum/debug | a full manual re-functioning of missed Ghidra boundaries would strengthen the negative |
| exact FactoryClass OLE record emitter order | deferred | class-factory thunk proves creation, not upstream emitter | requires draining the missed-boundary OLE object emitter or runtime breakpoint on FactoryClass class-factory calls during load |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] FSS-001 - Is this target broad or narrow? -> Narrow save/load ordering slice; investigated as exhaustive-slice but downgraded to partial because one static emitter boundary remains unresolved.` (evidence: user scope and final ledger)
- `[RESOLVED] FSS-002 - Does `FactoryClass::Save` preserve inner queue order? -> Yes, one factory writes queue count then queue slots in ascending index order.` (evidence: `0x004CA3C0`, prior report)
- `[RESOLVED] FSS-003 - Does `FactoryClass::Load` preserve inner queue order? -> Yes, it reads count, appends placeholders, then swizzle-registers slots in ascending index order.` (evidence: `0x004CA270`, prior report)
- `[RESOLVED] FSS-004 - Does the direct `g_FactoryClass_Array` loop at `0x0067CACF` call IPersist Save? -> No; it calls virtual `+0x34`, not `+0x18`.` (evidence: `0x0067CAEA`; vtable `0x007E8904`)
- `[RESOLVED] FSS-005 - What is FactoryClass virtual `+0x34`? -> `0x004CA430`, a debug/CRC-style routine using log strings and `AbstractClass__ComputeCRC`.` (evidence: `0x004CA430` decompile)
- `[RESOLVED] FSS-006 - Does persistence construction append to `g_FactoryClass_Array`? -> Yes; class-factory thunk calls the normal constructor, which appends at old count.` (evidence: `0x006C523C..0x006C526A`, `0x004C9974..0x004C9989`)
- `[RESOLVED] FSS-007 - Does load append order come from House/category sorting after load? -> No evidence for a post-load sort; append occurs when each FactoryClass object is created.` (evidence: constructor append and class-factory thunk)
- `[RESOLVED] FSS-008 - Do House factory fields participate in save/load swizzle? -> Yes, House load registers factory pointer slots around `+0x53AC..+0x53BC` and continuing through the same block.` (evidence: `0x00503294..0x005032E4`)
- `[RESOLVED] FSS-009 - Is a House factory pointer slot enough to prove House/category object creation order? -> No; swizzle slot registration only proves pointer patching, not object creation.` (evidence: `FUN_006CF240` role from active-vector report and House load context)
- `[RESOLVED] FSS-010 - Is there an INI key that sorts the factory stream? -> No direct key found; INI affects factory existence, not persistence stream ordering.` (evidence: INI grep and prior reports)
- `[RESOLVED] FSS-011 - What is the same-frame completion implication after load? -> The next factory tick uses whatever order `g_FactoryClass_Array` has after FactoryClass persistence construction; Rust must preserve that explicit order, not owner/category map order.` (evidence: `0x0055B66A..0x0055B68B` prior report plus constructor append)
- `[DEFERRED] FSS-012 - What exact function emits each FactoryClass OLE object record into the save stream?` (category: `requires-different-system-context`; reason: Ghidra function boundaries around the relevant OLE object emitter are missed/ambiguous, and the direct array loop resolved to checksum/debug rather than IPersist Save; next-step-if-pursued: runtime breakpoint on `0x004CA3C0` and `0x006C523C` during save/load, or read-only manual refunctioning in a separate Ghidra audit.)
- `[DEFERRED] FSS-013 - Does the exact emitter order equal HouseClass stream order plus factory field order?` (category: `requires-different-system-context`; reason: House factory slots are swizzled, but no verified nested FactoryClass `OleSaveToStream`/`OleLoadFromStream` call was proven in this pass; next-step-if-pursued: drain HouseClass IPersist Save/Load around factory fields with confirmed function boundaries.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Loaded FactoryClass objects append to `g_FactoryClass_Array` when the persistence layer creates each FactoryClass object. | `0x006C523C..0x006C526A`; `0x004C9974..0x004C9989` | missing native FactoryClass object list | `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs` | Add/preserve an explicit native-order factory list if implementing parity save/load or same-frame multi-factory completion. | Save two active factories whose owner/category sort differs from creation order; after load, the next completion/tick visits them in saved native factory-list order. | Do not recreate factory order by iterating `BTreeMap<owner, category>`. |
| The direct `g_FactoryClass_Array` loop at `0x0067CACF` is not the IPersist Save stream. | `0x0067CAEA` calls vtable `+0x34`; `0x007E8904 -> 0x004CA430`; `0x004CA430` decompile | Rust has no analogous distinction between checksum/debug surfaces and snapshot state | snapshot/native-save importer surfaces | Treat checksum/debug/global-audit walks separately from content persistence. | A native-save implementation must not cite the `+0x34` loop as proof of FactoryClass stream order. | Do not implement save stream enumeration from the `0x0067CACF` loop. |
| FactoryClass Save/Load preserves inner queue slots in ascending queue index. | `0x004CA3C0`; `0x004CA270` | Rust queue order inside `VecDeque` is directionally aligned, but top-level factory order is absent | `BuildQueueItem`, `ProductionState` | Keep per-factory queue order separate from global factory object order. | One factory with queued A then B round-trips with A before B. | Do not let a correct inner queue order hide a wrong all-factory order. |
| House factory pointer fields are swizzle slots, not order proof by themselves. | `0x00503294..0x005032E4`; `FUN_006CF240` role | Rust stores active producer by owner/category maps | `active_producer_by_owner`, future native factory references | When modeling native save/load, maintain pointer relationships and factory-object order as separate state. | After load, House pointer fields resolve to the same factory objects while global factory tick order remains the persisted factory-list order. | Do not infer global order from House field offset order without proving the object-record emitter. |
| Same-frame completion after load uses current `g_FactoryClass_Array` order. | prior report `0x0055B66A..0x0055B68B`; this report constructor append | Rust ticks collected owner/category pairs | `tick_production_with_overlay_registry` | Completion order must be driven by a native-order factory list before delivery/ready side effects. | `factory_snapshot_roundtrip_same_frame_completion_uses_saved_factory_order`: two factories complete on first tick after load and side effects occur in saved list order. | Do not sort by owner, category, stable id, sidebar order, or HouseClass pointer offset. |

## 10. Negative Facts / Do Not Do

- Do not use the `0x0067CACF..0x0067CAF5` forward loop as the save-content stream order. Active in YR: the loop exists, but it calls virtual `+0x34`, which is `FactoryClass__vtable_13 @ 0x004CA430`, not IPersist Save.
- Do not claim the current evidence proves standard save/load preserves `g_FactoryClass_Array` order directly. The constructor append is proven; the exact FactoryClass OLE record emitter remains deferred.
- Do not rebuild post-load factory order from `HouseClass+0x53AC..0x53CC` offsets unless a future report proves House/field-order object creation. This pass only proves swizzle slots.
- Do not treat Rust's `BTreeMap<owner, BTreeMap<category, queue>>` serialization order as a native save/load order. It is deterministic Rust order, not proven gamemd order.
- Do not collapse inner queue order and all-factory order into one concept. The inner queue is verified; the all-factory stream order is the unresolved parity risk.

## 11. Remaining Uncertainty

The exact function that emits `FactoryClass` OLE object records into the save stream remains unresolved. The direct global-array loop was resolved, but it is the wrong surface. The next high-confidence method is runtime breakpointing on `FactoryClass::Save @ 0x004CA3C0` during a save and `0x006C523C` during a load, recording caller stack and object order, or a separate Ghidra pass that repairs only the missed function boundaries around the OLE object emitter without mutating the project.

This uncertainty does not reopen the Rust-facing warning: post-load factory tick order must be an explicit native factory-list order, not owner/category map order.

## Sources

- Ghidra read-only decompile/assembly: `FUN_0067D300`, `FUN_0067E730`, `FUN_0067F7E0`, `FUN_0067F9C0`, `0x0067CACF..0x0067CAF5`, `FactoryClass::Load @ 0x004CA270`, `FactoryClass::Save @ 0x004CA3C0`, `FactoryClass__vtable_13 @ 0x004CA430`, `FactoryClass::Constructor @ 0x004C98B0`, Factory COM class-factory thunk `0x006C523C..0x006C526A`, House load swizzle area `0x00503294..0x00503320`.
- Prior research: `FACTORYCLASS_GLOBAL_ARRAY_INSERTION_REBUILD_ORDER_RESWARM_20260528.md`, `FACTORYCLASS_AND_CAMEOENTRY_STRUCT_LAYOUT.md`, `OBJECT_ACTIVE_VECTOR_SAVE_LOAD_REBUILD_OWNER_RESWARM_20260528.md`, `BUILDING_SYSTEMS_GHIDRA_REPORT.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`.
- Rust static scan: `src/sim/production/production_types.rs`, `src/sim/production/production_queue.rs`, `src/sim/world/mod.rs`.

**Status:** PARTIAL for the exact FactoryClass OLE record emitter; complete for the negative finding that the direct `g_FactoryClass_Array` loop is not the save-content stream and for the handoff that Rust must preserve explicit native factory-list order across snapshots/save-load.
